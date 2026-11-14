//! Microsoft's own `d3dcompiler_47.dll`. The one Winetricks installs is a 2013 build, and with it
//! every vertex shader of some games (Tekken Tag Tournament 2's PC port) fails to compile, which
//! leaves their work to the processor and the game at a few frames per second. A current build
//! compiles them all.
//!
//! The DLL is not offered on its own anywhere small. Electron's Windows build ships it, so Lutra
//! reads just that one file out of the release archive (158 MB) with HTTP range requests, through
//! the zip's table of contents, and checks the result against a pinned size and SHA-256. Whatever
//! the server sends, only that exact file is ever used.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::wine;

const ELECTRON_VERSION: &str = "45.3.2";

/// The pinned file: Microsoft's 64-bit `d3dcompiler_47.dll`, version 10.0.28000.2270.
pub const D3DCOMPILER_SIZE: u64 = 4_753_864;
const D3DCOMPILER_SHA256: &str = "f5c95e919b4ffa1ac71049cd45b9f986d2965e6888395ca2806337985334fa56";
const D3DCOMPILER_NAME: &str = "d3dcompiler_47.dll";

/// How much of the archive's end is read first: the table of contents is far smaller than this.
const TAIL: u64 = 512 * 1024;

fn archive_url() -> String {
    format!(
        "https://github.com/electron/electron/releases/download/v{ELECTRON_VERSION}/electron-v{ELECTRON_VERSION}-win32-x64.zip"
    )
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent("Lutra")
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .build()
        .into()
}

/// A file of the archive, as its table of contents describes it.
#[derive(Debug, PartialEq)]
struct Entry {
    method: u16,
    compressed: u64,
    uncompressed: u64,
    /// Where its local header starts in the archive.
    offset: u64,
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Where the table of contents is: (offset, size), from the end-of-archive record.
fn table_of_contents(tail: &[u8]) -> Option<(u64, u64)> {
    let start = tail
        .windows(4)
        .rposition(|window| window == [0x50, 0x4b, 0x05, 0x06])?;
    let record = &tail[start..];
    Some((
        u64::from(u32_at(record, 16)?),
        u64::from(u32_at(record, 12)?),
    ))
}

/// Finds a file by name in the table of contents.
fn find_entry(table: &[u8], wanted: &str) -> Option<Entry> {
    let mut at = 0;
    while u32_at(table, at)? == 0x0201_4b50 {
        let name_len = usize::from(u16_at(table, at + 28)?);
        let extra_len = usize::from(u16_at(table, at + 30)?);
        let comment_len = usize::from(u16_at(table, at + 32)?);
        let name = table.get(at + 46..at + 46 + name_len)?;
        if name == wanted.as_bytes() {
            return Some(Entry {
                method: u16_at(table, at + 10)?,
                compressed: u64::from(u32_at(table, at + 20)?),
                uncompressed: u64::from(u32_at(table, at + 24)?),
                offset: u64::from(u32_at(table, at + 42)?),
            });
        }
        at += 46 + name_len + extra_len + comment_len;
    }
    None
}

/// The file's bytes, from what was read starting at its local header.
fn extract(from_header: &[u8], entry: &Entry) -> Result<Vec<u8>, String> {
    if u32_at(from_header, 0) != Some(0x0403_4b50) {
        return Err("the archive's file header is not where it should be".into());
    }
    let name_len = usize::from(u16_at(from_header, 26).ok_or("truncated header")?);
    let extra_len = usize::from(u16_at(from_header, 28).ok_or("truncated header")?);
    let start = 30 + name_len + extra_len;
    let data = from_header
        .get(start..start + entry.compressed as usize)
        .ok_or("the archive ended early")?;

    match entry.method {
        0 => Ok(data.to_vec()),
        8 => {
            let mut inflated = Vec::with_capacity(entry.uncompressed as usize);
            // Never inflate more than the table of contents promised.
            flate2::read::DeflateDecoder::new(data)
                .take(entry.uncompressed + 1)
                .read_to_end(&mut inflated)
                .map_err(|error| format!("could not unpack the file: {error}"))?;
            Ok(inflated)
        }
        other => Err(format!("unsupported compression ({other})")),
    }
}

/// Reads `length` bytes from `from`. Also returns the archive's total size. `report` gets the
/// share read so far, 0 to 100. Only explicit ranges: GitHub's CDN answers 501 to "the last N
/// bytes", so the size is learned from a one-byte read first.
fn read_range(
    url: &str,
    from: u64,
    length: u64,
    report: &mut dyn FnMut(u8),
) -> Result<(Vec<u8>, u64), String> {
    let range = format!("bytes={from}-{}", from + length - 1);
    let mut response = agent()
        .get(url)
        .header("Range", &range)
        .call()
        .map_err(|error| format!("download failed: {error}"))?;
    if response.status().as_u16() != 206 {
        return Err("the server does not support partial downloads".into());
    }
    // `Content-Range: bytes 100-199/5000`
    let total = response
        .headers()
        .get("content-range")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit('/').next())
        .and_then(|total| total.parse::<u64>().ok())
        .ok_or("the server did not say how big the archive is")?;

    let mut body = response.body_mut().as_reader();
    let mut bytes = Vec::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = body
            .read(&mut buffer)
            .map_err(|error| format!("download interrupted: {error}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        report(((bytes.len() as u64 * 100) / length.max(1)).min(100) as u8);
    }
    Ok((bytes, total))
}

fn sha256(file: &Path) -> Result<String, String> {
    let output = wine::host_env(&mut Command::new("sha256sum"))
        .arg(file)
        .output()
        .map_err(|error| format!("sha256sum is not available: {error}"))?;
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .map(str::to_string)
        .ok_or_else(|| "sha256sum gave no result".to_string())
}

/// Does this file have the pinned size and checksum?
fn is_the_pinned_file(file: &Path) -> bool {
    fs::metadata(file).is_ok_and(|meta| meta.len() == D3DCOMPILER_SIZE)
        && sha256(file).is_ok_and(|sum| sum == D3DCOMPILER_SHA256)
}

/// The DLL on disk, fetched into `cache` the first time. `report` gets the download's progress,
/// 0 to 100 (nothing when it was already there).
pub fn d3dcompiler(cache: &Path, report: &mut dyn FnMut(u8)) -> Result<PathBuf, String> {
    let file = cache.join(format!("d3dcompiler_47-{ELECTRON_VERSION}.dll"));
    if is_the_pinned_file(&file) {
        return Ok(file);
    }
    fs::create_dir_all(cache).map_err(|error| error.to_string())?;

    let url = archive_url();
    // The end of the archive holds its table of contents.
    let (_, total) = read_range(&url, 0, 1, &mut |_| {})?;
    let tail_start = total.saturating_sub(TAIL);
    let (tail, _) = read_range(&url, tail_start, total - tail_start, &mut |_| {})?;
    let (toc_offset, toc_size) =
        table_of_contents(&tail).ok_or("could not read the archive's table of contents")?;
    let table = if toc_offset >= tail_start {
        let from = (toc_offset - tail_start) as usize;
        tail.get(from..from + toc_size as usize)
            .ok_or("the table of contents is cut short")?
            .to_vec()
    } else {
        read_range(&url, toc_offset, toc_size, &mut |_| {})?.0
    };
    let entry = find_entry(&table, D3DCOMPILER_NAME)
        .ok_or_else(|| format!("{D3DCOMPILER_NAME} is not in the archive"))?;
    if entry.uncompressed != D3DCOMPILER_SIZE {
        return Err("the archive holds another build of the DLL than the one expected".into());
    }

    // The file's local header (a name and an extra field of unknown length), then its data.
    let length = (30 + 1024 + entry.compressed).min(total - entry.offset);
    let (bytes, _) = read_range(&url, entry.offset, length, report)?;
    let dll = extract(&bytes, &entry)?;

    let part = file.with_extension("part");
    fs::write(&part, dll).map_err(|error| error.to_string())?;
    if !is_the_pinned_file(&part) {
        let _ = fs::remove_file(&part);
        return Err("the downloaded DLL is not the expected file (checksum mismatch)".into());
    }
    fs::rename(&part, &file).map_err(|error| error.to_string())?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    /// A zip with the given files, built by hand: (name, bytes, deflate it).
    fn zip(files: &[(&str, &[u8], bool)]) -> Vec<u8> {
        let mut archive = Vec::new();
        let mut table = Vec::new();
        for (name, data, deflate) in files {
            let stored: Vec<u8> = if *deflate {
                let mut encoder =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(data).unwrap();
                encoder.finish().unwrap()
            } else {
                data.to_vec()
            };
            let method: u16 = if *deflate { 8 } else { 0 };
            let offset = archive.len() as u32;
            let extra = b"xx";

            archive.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            archive.extend_from_slice(&[20, 0, 0, 0]);
            archive.extend_from_slice(&method.to_le_bytes());
            archive.extend_from_slice(&[0; 8]);
            archive.extend_from_slice(&(stored.len() as u32).to_le_bytes());
            archive.extend_from_slice(&(data.len() as u32).to_le_bytes());
            archive.extend_from_slice(&(name.len() as u16).to_le_bytes());
            archive.extend_from_slice(&(extra.len() as u16).to_le_bytes());
            archive.extend_from_slice(name.as_bytes());
            archive.extend_from_slice(extra);
            archive.extend_from_slice(&stored);

            table.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            table.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
            table.extend_from_slice(&method.to_le_bytes());
            table.extend_from_slice(&[0; 8]);
            table.extend_from_slice(&(stored.len() as u32).to_le_bytes());
            table.extend_from_slice(&(data.len() as u32).to_le_bytes());
            table.extend_from_slice(&(name.len() as u16).to_le_bytes());
            table.extend_from_slice(&0u16.to_le_bytes());
            table.extend_from_slice(&3u16.to_le_bytes());
            table.extend_from_slice(&[0; 8]);
            table.extend_from_slice(&offset.to_le_bytes());
            table.extend_from_slice(name.as_bytes());
            table.extend_from_slice(b"hey");
        }
        let table_offset = archive.len() as u32;
        let table_size = table.len() as u32;
        archive.extend_from_slice(&table);
        archive.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        archive.extend_from_slice(&[0; 4]);
        archive.extend_from_slice(&(files.len() as u16).to_le_bytes());
        archive.extend_from_slice(&(files.len() as u16).to_le_bytes());
        archive.extend_from_slice(&table_size.to_le_bytes());
        archive.extend_from_slice(&table_offset.to_le_bytes());
        archive.extend_from_slice(&[0, 0]);
        archive
    }

    #[test]
    fn a_file_is_found_and_unpacked_from_the_end_of_the_archive() {
        let big = vec![7u8; 5000];
        let archive = zip(&[
            ("LICENSE", b"mit", false),
            ("d3dcompiler_47.dll", &big, true),
            ("electron.exe", b"exe", true),
        ]);

        let (offset, size) = table_of_contents(&archive).unwrap();
        let table = &archive[offset as usize..(offset + size) as usize];
        let entry = find_entry(table, "d3dcompiler_47.dll").unwrap();
        assert_eq!((entry.method, entry.uncompressed), (8, 5000));
        assert!(entry.compressed < 5000);

        // Only what starts at the local header is needed, like the range request gives.
        let unpacked = extract(&archive[entry.offset as usize..], &entry).unwrap();
        assert_eq!(unpacked, big);

        let stored = find_entry(table, "LICENSE").unwrap();
        assert_eq!(
            extract(&archive[stored.offset as usize..], &stored).unwrap(),
            b"mit"
        );
        assert_eq!(find_entry(table, "missing.dll"), None);
        // A name that only starts like the wanted one is not it.
        assert_eq!(find_entry(table, "d3dcompiler_47"), None);
    }

    #[test]
    fn damaged_archives_are_refused_not_trusted() {
        assert_eq!(table_of_contents(b"not a zip at all"), None);
        assert_eq!(find_entry(b"garbage", "x"), None);

        let archive = zip(&[("d3dcompiler_47.dll", b"data", false)]);
        let (offset, size) = table_of_contents(&archive).unwrap();
        let entry = find_entry(
            &archive[offset as usize..(offset + size) as usize],
            "d3dcompiler_47.dll",
        )
        .unwrap();
        // Not a local header there, and data that stops short.
        assert!(extract(&archive[1..], &entry).is_err());
        assert!(extract(&archive[..entry.offset as usize + 34], &entry).is_err());
        let unknown = Entry {
            method: 99,
            ..entry
        };
        assert!(extract(&archive, &unknown).is_err());
    }

    #[test]
    fn a_file_longer_than_promised_is_cut() {
        let big = vec![1u8; 4000];
        let archive = zip(&[("a.bin", &big, true)]);
        let (offset, size) = table_of_contents(&archive).unwrap();
        let mut entry =
            find_entry(&archive[offset as usize..(offset + size) as usize], "a.bin").unwrap();
        entry.uncompressed = 100;
        let unpacked = extract(&archive[entry.offset as usize..], &entry).unwrap();
        assert_eq!(unpacked.len(), 101);
    }

    /// Reads the real file from GitHub (about 2 MB over the network) and checks it is the pinned
    /// one: `cargo test -- --ignored fetches_the_real`.
    #[test]
    #[ignore]
    fn fetches_the_real_microsoft_dll() {
        let dir = tempfile::tempdir().unwrap();
        let mut last = 0;
        let file = d3dcompiler(dir.path(), &mut |percent| last = percent).unwrap();
        assert_eq!(fs::metadata(&file).unwrap().len(), D3DCOMPILER_SIZE);
        assert_eq!(last, 100);
        // The second time it comes from the cache, with no download.
        let mut again = 0;
        assert_eq!(
            d3dcompiler(dir.path(), &mut |percent| again = percent).unwrap(),
            file
        );
        assert_eq!(again, 0);
    }
}
