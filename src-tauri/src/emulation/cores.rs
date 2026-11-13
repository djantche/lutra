//! The cores Lutra can run: the ones it manages itself, and the ones the system provides.
//! Lutra's own copy always wins, so what runs is what Lutra downloaded and checked.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_uint, c_void};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::info::{self, CoreInfo};

const BUILDBOT: &str = "https://buildbot.libretro.com";
const SUFFIX: &str = "_libretro.so";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    /// Downloaded and verified by Lutra.
    Lutra,
    /// Found in the system's libretro folder.
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InstalledCore {
    pub info: CoreInfo,
    pub path: PathBuf,
    pub origin: Origin,
    /// CRC of the downloaded archive, to tell when a newer build exists.
    pub crc: Option<String>,
}

/// Where cores and their info files are looked for.
#[derive(Debug, Clone)]
pub struct Locations {
    /// `<data>/emulation/cores`: Lutra's own cores, `info/` inside it for their `.info` files.
    pub lutra: PathBuf,
    /// `/usr/lib/libretro`.
    pub system_cores: PathBuf,
    /// `/usr/share/libretro/info`.
    pub system_info: PathBuf,
}

impl Locations {
    pub fn system_default(lutra: PathBuf) -> Self {
        Self {
            lutra,
            system_cores: PathBuf::from("/usr/lib/libretro"),
            system_info: PathBuf::from("/usr/share/libretro/info"),
        }
    }

    fn lutra_info(&self) -> PathBuf {
        self.lutra.join("info")
    }
}

fn core_id(file_name: &str) -> Option<&str> {
    file_name.strip_suffix(SUFFIX)
}

fn read_info(id: &str, dirs: &[PathBuf]) -> CoreInfo {
    dirs.iter()
        .find_map(|dir| fs::read_to_string(dir.join(format!("{id}_libretro.info"))).ok())
        .map(|text| info::parse_core(id, &text))
        .unwrap_or_else(|| info::parse_core(id, ""))
}

fn cores_in(
    dir: &Path,
    origin: Origin,
    info_dirs: &[PathBuf],
    out: &mut HashMap<String, InstalledCore>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = core_id(&name) else { continue };
        let crc = fs::read_to_string(dir.join(format!("{name}.crc")))
            .ok()
            .map(|text| text.trim().to_string());
        // The first origin to claim an id keeps it.
        out.entry(id.to_string()).or_insert_with(|| InstalledCore {
            info: read_info(id, info_dirs),
            path: entry.path(),
            origin,
            crc,
        });
    }
}

/// Every runnable core, Lutra's own first, sorted by name.
pub fn inventory(locations: &Locations) -> Vec<InstalledCore> {
    let mut found = HashMap::new();
    let lutra_info = [locations.lutra_info(), locations.system_info.clone()];
    cores_in(&locations.lutra, Origin::Lutra, &lutra_info, &mut found);
    let system_info = [locations.system_info.clone(), locations.lutra_info()];
    cores_in(
        &locations.system_cores,
        Origin::System,
        &system_info,
        &mut found,
    );
    let mut cores: Vec<InstalledCore> = found.into_values().collect();
    cores.sort_by(|a, b| {
        a.info
            .display_name
            .to_lowercase()
            .cmp(&b.info.display_name.to_lowercase())
    });
    cores
}

// ---- checking that a file really is a libretro core ----

#[repr(C)]
struct SystemInfo {
    library_name: *const c_char,
    library_version: *const c_char,
    valid_extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}

/// What a core says about itself once loaded.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub api_version: u32,
    pub name: String,
    pub version: String,
    pub extensions: Vec<String>,
    pub needs_fullpath: bool,
}

fn text(pointer: *const c_char) -> String {
    if pointer.is_null() {
        return String::new();
    }
    // SAFETY: libretro guarantees these are NUL-terminated strings owned by the core.
    unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned()
}

fn last_error() -> String {
    // SAFETY: `dlerror` returns a thread-local NUL-terminated message or null.
    let message = unsafe { libc::dlerror() };
    if message.is_null() {
        "unknown error".into()
    } else {
        text(message)
    }
}

/// Loads a core and asks it for its API version and system info, without starting it. A file
/// that is not a core, a core for another architecture or with missing libraries, and a core
/// built for another API version are all refused here, with the loader's own message.
pub fn probe(path: &Path) -> Result<Probe, String> {
    let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
    // SAFETY: plain `dlopen` of a path we own; the handle is closed before returning.
    let handle = unsafe { libc::dlopen(c_path.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
    if handle.is_null() {
        return Err(last_error());
    }

    let symbol = |name: &str| -> Result<*mut c_void, String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;
        // SAFETY: `handle` is a live handle returned by `dlopen`.
        let found = unsafe { libc::dlsym(handle, c_name.as_ptr()) };
        if found.is_null() {
            Err(format!("not a libretro core: {name} is missing"))
        } else {
            Ok(found)
        }
    };

    let result = (|| {
        let api: unsafe extern "C" fn() -> c_uint =
            // SAFETY: `retro_api_version` has this signature in the libretro API.
            unsafe { std::mem::transmute(symbol("retro_api_version")?) };
        let get_info: unsafe extern "C" fn(*mut SystemInfo) =
            // SAFETY: `retro_get_system_info` has this signature in the libretro API.
            unsafe { std::mem::transmute(symbol("retro_get_system_info")?) };
        // SAFETY: both functions may be called before `retro_init`, per the libretro API.
        let (api_version, system) = unsafe {
            let mut system = std::mem::zeroed::<SystemInfo>();
            get_info(&mut system);
            (api(), system)
        };
        if api_version != 1 {
            return Err(format!("unsupported libretro API version {api_version}"));
        }
        Ok(Probe {
            api_version,
            name: text(system.library_name),
            version: text(system.library_version),
            extensions: text(system.valid_extensions)
                .split('|')
                .filter(|ext| !ext.is_empty())
                .map(str::to_lowercase)
                .collect(),
            needs_fullpath: system.need_fullpath,
        })
    })();

    // SAFETY: closes the handle opened above; nothing from the core is used afterwards.
    unsafe { libc::dlclose(handle) };
    result
}

// ---- the libretro build server ----

/// A core the build server offers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCore {
    pub id: String,
    /// Build date, `YYYY-MM-DD`.
    pub date: String,
    /// CRC32 of the archive.
    pub crc: String,
}

/// The server's index: one `date crc file.so.zip` line per core.
pub fn parse_index(text: &str) -> Vec<RemoteCore> {
    let mut cores: Vec<RemoteCore> = text
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (date, crc, file) = (parts.next()?, parts.next()?, parts.next()?);
            let id = file.strip_suffix(&format!("{SUFFIX}.zip"))?;
            Some(RemoteCore {
                id: id.to_string(),
                date: date.to_string(),
                crc: crc.to_lowercase(),
            })
        })
        .collect();
    cores.sort_by(|a, b| a.id.cmp(&b.id));
    cores
}

pub fn fetch_index(agent: &ureq::Agent) -> Result<Vec<RemoteCore>, String> {
    let body = agent
        .get(&format!(
            "{BUILDBOT}/nightly/linux/x86_64/latest/.index-extended"
        ))
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    Ok(parse_index(&body))
}

/// The first entry of a zip archive called `wanted`, written to `target` (atomically).
fn extract_entry(zip_bytes: &[u8], wanted: &str, target: &Path) -> Result<(), String> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(|e| e.to_string())?;
    let mut entry = archive
        .by_name(wanted)
        .map_err(|_| format!("{wanted} is not in the archive"))?;
    let partial = target.with_extension("part");
    {
        let mut file = fs::File::create(&partial).map_err(|e| e.to_string())?;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = entry.read(&mut buffer).map_err(|e| e.to_string())?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read]).map_err(|e| e.to_string())?;
        }
    }
    fs::rename(&partial, target).map_err(|e| e.to_string())
}

fn download(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, String> {
    agent
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(600 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

/// Downloads a core into Lutra's folder and checks it loads. A core that does not is removed.
pub fn install(
    agent: &ureq::Agent,
    locations: &Locations,
    remote: &RemoteCore,
) -> Result<Probe, String> {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(&locations.lutra).map_err(|e| e.to_string())?;
    let file = format!("{}{SUFFIX}", remote.id);
    let bytes = download(
        agent,
        &format!("{BUILDBOT}/nightly/linux/x86_64/latest/{file}.zip"),
    )?;
    let target = locations.lutra.join(&file);
    extract_entry(&bytes, &file, &target)?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;

    match probe(&target) {
        Ok(probe) => {
            let _ = fs::write(locations.lutra.join(format!("{file}.crc")), &remote.crc);
            Ok(probe)
        }
        Err(error) => {
            let _ = fs::remove_file(&target);
            Err(format!(
                "{} is not usable on this system: {error}",
                remote.id
            ))
        }
    }
}

/// Downloads the `.info` files, so cores Lutra installs are described even without the system's.
pub fn install_infos(agent: &ureq::Agent, locations: &Locations) -> Result<usize, String> {
    let bytes = download(agent, &format!("{BUILDBOT}/assets/frontend/info.zip"))?;
    let dir = locations.lutra_info();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut count = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        let Some(name) = entry
            .enclosed_name()
            .and_then(|path| path.file_name().map(|n| n.to_owned()))
        else {
            continue;
        };
        if !name.to_string_lossy().ends_with(".info") {
            continue;
        }
        let mut text = Vec::new();
        entry.read_to_end(&mut text).map_err(|e| e.to_string())?;
        fs::write(dir.join(name), text).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

/// Whether a newer build of an installed core exists on the server.
pub fn has_update(installed: &InstalledCore, remote: &[RemoteCore]) -> bool {
    installed.origin == Origin::Lutra
        && remote
            .iter()
            .find(|candidate| candidate.id == installed.info.id)
            .is_some_and(|candidate| installed.crc.as_deref() != Some(candidate.crc.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_core(dir: &Path, id: &str, info_dir: &Path, display: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::create_dir_all(info_dir).unwrap();
        fs::write(dir.join(format!("{id}_libretro.so")), b"not really").unwrap();
        fs::write(
            info_dir.join(format!("{id}_libretro.info")),
            format!("display_name = \"{display}\"\nsupported_extensions = \"nes\"\n"),
        )
        .unwrap();
    }

    #[test]
    fn lutra_s_own_core_wins_over_the_system_s_with_the_same_id() {
        let root = tempfile::tempdir().unwrap();
        let locations = Locations {
            lutra: root.path().join("lutra"),
            system_cores: root.path().join("sys"),
            system_info: root.path().join("sysinfo"),
        };
        write_core(
            &locations.lutra,
            "nestopia",
            &locations.lutra.join("info"),
            "Nestopia (Lutra)",
        );
        write_core(
            &locations.system_cores,
            "nestopia",
            &locations.system_info,
            "Nestopia (system)",
        );
        write_core(
            &locations.system_cores,
            "fceumm",
            &locations.system_info,
            "FCEUmm",
        );

        let cores = inventory(&locations);
        assert_eq!(cores.len(), 2);
        let nestopia = cores.iter().find(|c| c.info.id == "nestopia").unwrap();
        assert_eq!(nestopia.origin, Origin::Lutra);
        assert_eq!(nestopia.info.display_name, "Nestopia (Lutra)");
        assert_eq!(
            cores.iter().find(|c| c.info.id == "fceumm").unwrap().origin,
            Origin::System
        );
    }

    #[test]
    fn a_core_without_an_info_file_is_still_listed_with_its_id() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("odd_libretro.so"), b"x").unwrap();
        let locations = Locations {
            lutra: root.path().join("none"),
            system_cores: root.path().to_path_buf(),
            system_info: root.path().join("noinfo"),
        };
        let cores = inventory(&locations);
        assert_eq!(cores[0].info.display_name, "odd");
    }

    #[test]
    fn the_server_index_is_parsed_into_cores() {
        let index = "2026-11-10 e211e6cb snes9x_libretro.so.zip\n2026-11-11 1ae86b13 stella_libretro.so.zip\nbroken line\n2026-11-11 abc not_a_core.zip\n";
        let cores = parse_index(index);
        assert_eq!(cores.len(), 2);
        assert_eq!(
            cores[0],
            RemoteCore {
                id: "snes9x".into(),
                date: "2026-11-10".into(),
                crc: "e211e6cb".into()
            }
        );
        assert_eq!(cores[1].id, "stella");
    }

    #[test]
    fn an_update_is_offered_only_for_cores_lutra_installed_with_another_crc() {
        let remote = vec![RemoteCore {
            id: "snes9x".into(),
            date: "d".into(),
            crc: "bbbb".into(),
        }];
        let make = |origin, crc: Option<&str>| InstalledCore {
            info: info::parse_core("snes9x", ""),
            path: PathBuf::new(),
            origin,
            crc: crc.map(str::to_string),
        };
        assert!(has_update(&make(Origin::Lutra, Some("aaaa")), &remote));
        assert!(has_update(&make(Origin::Lutra, None), &remote));
        assert!(!has_update(&make(Origin::Lutra, Some("bbbb")), &remote));
        assert!(!has_update(&make(Origin::System, Some("aaaa")), &remote));
    }

    #[test]
    fn a_core_is_taken_out_of_its_archive() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
            writer
                .start_file("x_libretro.so", zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"core bytes").unwrap();
            writer.finish().unwrap();
        }
        let target = dir.path().join("x_libretro.so");
        extract_entry(&bytes, "x_libretro.so", &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"core bytes");
        assert!(extract_entry(&bytes, "missing.so", &dir.path().join("m")).is_err());
        assert!(!dir.path().join("x_libretro.part").exists());
    }

    #[test]
    fn files_that_are_not_cores_are_refused_with_a_reason() {
        let dir = tempfile::tempdir().unwrap();
        let bogus = dir.path().join("bogus_libretro.so");
        fs::write(&bogus, b"this is not an ELF file").unwrap();
        assert!(probe(&bogus).is_err());
        assert!(probe(&dir.path().join("missing.so")).is_err());
        // A real shared library that is not a core.
        let libc = ["/usr/lib/libc.so.6", "/lib/x86_64-linux-gnu/libc.so.6"]
            .into_iter()
            .map(PathBuf::from)
            .find(|path| path.exists());
        if let Some(libc) = libc {
            let error = probe(&libc).unwrap_err();
            assert!(error.contains("not a libretro core"), "{error}");
        }
    }

    /// Needs the system's cores: `cargo test -- --ignored emulation`.
    #[test]
    #[ignore]
    fn a_real_system_core_reports_itself() {
        let path = Path::new("/usr/lib/libretro/snes9x_libretro.so");
        if !path.exists() {
            return;
        }
        let probe = probe(path).unwrap();
        println!("{probe:?}");
        assert_eq!(probe.api_version, 1);
        assert!(probe.extensions.contains(&"sfc".to_string()));
    }

    /// Needs the network: downloads a small core, checks it, runs it through `probe`.
    #[test]
    #[ignore]
    fn a_core_can_be_downloaded_and_verified() {
        let dir = tempfile::tempdir().unwrap();
        let locations = Locations {
            lutra: dir.path().join("cores"),
            system_cores: dir.path().join("none"),
            system_info: dir.path().join("none"),
        };
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(120)))
            .build()
            .into();
        let index = fetch_index(&agent).unwrap();
        let remote = index
            .iter()
            .find(|core| core.id == "gambatte")
            .expect("gambatte is offered");
        let probe = install(&agent, &locations, remote).unwrap();
        println!("{probe:?}");
        assert!(probe.extensions.contains(&"gb".to_string()));
        let installed = inventory(&locations);
        assert_eq!(installed[0].origin, Origin::Lutra);
        assert_eq!(installed[0].crc.as_deref(), Some(remote.crc.as_str()));
        assert!(!has_update(&installed[0], &index));
        assert!(install_infos(&agent, &locations).unwrap() > 100);
    }
}
