//! Wine and Proton builds that Lutra downloads and keeps itself, in `<data>/runners/`.
//!
//! The catalogue comes from GitHub releases (GE-Proton, and Kron4ek's Wine builds). An install is a
//! job in its own thread that reports real progress through `runner-install` events: bytes
//! received while downloading, then entries extracted. The archive's SHA-256 (published by GitHub
//! next to every asset) is checked before anything is unpacked, and the job can be cancelled at any
//! point, leaving nothing behind.
//!
//! Extraction and checksums go through the system's `tar` and `sha256sum`, like the rest of Lutra
//! relies on existing tools rather than reimplementing them.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::applog;
use crate::host::AppHandle;
use crate::wine::{self, RunnerKind};

/// How often progress is pushed to the interface while bytes or entries flow.
const REPORT_EVERY: Duration = Duration::from_millis(150);

/// What a runner can be downloaded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    ProtonGe,
    Wine,
    WineStaging,
}

impl Source {
    fn repo(self) -> &'static str {
        match self {
            Source::ProtonGe => "GloriousEggroll/proton-ge-custom",
            Source::Wine | Source::WineStaging => "Kron4ek/Wine-Builds",
        }
    }

    /// The archive of a release (`tag`) for this source.
    fn asset_name(self, tag: &str) -> String {
        match self {
            Source::ProtonGe => format!("{tag}-x86_64.tar.gz"),
            Source::Wine => format!("wine-{tag}-amd64-wow64.tar.xz"),
            Source::WineStaging => format!("wine-{tag}-staging-amd64-wow64.tar.xz"),
        }
    }

    /// The folder name an install gets: what the archive unpacks to.
    fn install_name(self, tag: &str) -> String {
        match self {
            Source::ProtonGe => tag.to_string(),
            Source::Wine => format!("wine-{tag}-amd64-wow64"),
            Source::WineStaging => format!("wine-{tag}-staging-amd64-wow64"),
        }
    }
}

/// One downloadable release.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Available {
    pub source: Source,
    pub tag: String,
    /// The folder it installs to; also the key of its install job.
    pub name: String,
    pub size: u64,
    pub published: String,
    pub installed: bool,
}

/// A runner kept by Lutra.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub name: String,
    pub kind: RunnerKind,
    /// The `proton` script or the `wine` binary: what a game stores as its runner.
    pub path: PathBuf,
    pub size: u64,
}

pub fn runners_dir(data: &Path) -> PathBuf {
    data.join("runners")
}

/// The file that runs Windows programs inside an unpacked build, and what kind it is.
pub fn entry_point(dir: &Path) -> Option<(RunnerKind, PathBuf)> {
    let proton = dir.join("proton");
    if proton.is_file() {
        return Some((RunnerKind::Proton, proton));
    }
    let wine = dir.join("bin/wine");
    wine.is_file().then_some((RunnerKind::Wine, wine))
}

/// Size of a folder, links not followed: a runner may point anywhere.
fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => dir_size(&entry.path()),
            Ok(kind) if kind.is_file() => entry.metadata().map_or(0, |meta| meta.len()),
            _ => 0,
        })
        .sum()
}

/// What is installed, newest name first.
pub fn installed(data: &Path) -> Vec<Installed> {
    let Ok(entries) = fs::read_dir(runners_dir(data)) else {
        return Vec::new();
    };
    let mut found: Vec<Installed> = entries
        .flatten()
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|entry| {
            let (kind, path) = entry_point(&entry.path())?;
            Some(Installed {
                name: entry.file_name().to_string_lossy().into_owned(),
                kind,
                path,
                size: dir_size(&entry.path()),
            })
        })
        .collect();
    found.sort_by_key(|runner| std::cmp::Reverse(runner.name.to_lowercase()));
    found
}

/// Deletes an install. Only a folder directly under `<data>/runners/` can go.
pub fn remove(data: &Path, name: &str) -> Result<(), String> {
    check_name(name)?;
    let dir = runners_dir(data).join(name);
    if !dir.is_dir() {
        return Err(format!("{name} is not installed"));
    }
    if is_installing(name) {
        return Err(format!("{name} is being installed"));
    }
    fs::remove_dir_all(&dir).map_err(|error| format!("cannot remove {name}: {error}"))?;
    applog::info(format!("Removed the runner {name}"));
    Ok(())
}

/// A release tag or folder name that cannot reach outside the runners folder.
fn check_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if valid {
        Ok(())
    } else {
        Err(format!("invalid runner name `{name}`"))
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent("Lutra")
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .build()
        .into()
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    size: u64,
    #[serde(default)]
    digest: Option<String>,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    published_at: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

fn releases(source: Source) -> Result<Vec<Release>, String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases?per_page=20",
        source.repo()
    );
    agent()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("cannot reach GitHub: {error}"))?
        .body_mut()
        .read_json::<Vec<Release>>()
        .map_err(|error| format!("unexpected answer from GitHub: {error}"))
}

/// Recent releases of a source, with what is already installed marked.
pub fn available(data: &Path, source: Source) -> Result<Vec<Available>, String> {
    let dir = runners_dir(data);
    Ok(releases(source)?
        .into_iter()
        .filter(|release| !release.prerelease)
        .filter_map(|release| {
            let wanted = source.asset_name(&release.tag_name);
            let asset = release.assets.iter().find(|asset| asset.name == wanted)?;
            let name = source.install_name(&release.tag_name);
            Some(Available {
                source,
                installed: dir.join(&name).is_dir(),
                size: asset.size,
                published: release.published_at.clone(),
                tag: release.tag_name,
                name,
            })
        })
        .collect())
}

/// Installs in progress, with the flag that cancels each.
fn jobs() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static JOBS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    JOBS.get_or_init(Mutex::default)
}

fn is_installing(name: &str) -> bool {
    jobs().lock().is_ok_and(|jobs| jobs.contains_key(name))
}

/// Names of the installs running now.
pub fn running() -> Vec<String> {
    jobs()
        .lock()
        .map(|jobs| jobs.keys().cloned().collect())
        .unwrap_or_default()
}

pub fn cancel(name: &str) {
    if let Ok(jobs) = jobs().lock()
        && let Some(flag) = jobs.get(name)
    {
        flag.store(true, Ordering::Relaxed);
    }
}

/// Why a job ended early.
enum Stop {
    Cancelled,
    Failed(String),
}

impl<E: std::fmt::Display> From<E> for Stop {
    fn from(error: E) -> Self {
        Stop::Failed(error.to_string())
    }
}

/// Pushes the job's progress to the interface and to Lutra's log.
struct Reporter<'a> {
    app: &'a AppHandle,
    name: &'a str,
    last: Instant,
}

impl Reporter<'_> {
    fn emit(&self, stage: &str, received: u64, total: u64, line: Option<&str>) {
        let _ = self.app.emit(
            "runner-install",
            json!({ "name": self.name, "stage": stage, "received": received, "total": total, "line": line }),
        );
    }

    /// A line for the install's log, and a new stage.
    fn stage(&mut self, stage: &str, total: u64, line: &str) {
        applog::info(format!("Runner {}: {line}", self.name));
        self.emit(stage, 0, total, Some(line));
        self.last = Instant::now();
    }

    /// Progress inside a stage, at most every `REPORT_EVERY`.
    fn progress(&mut self, stage: &str, received: u64, total: u64) {
        if self.last.elapsed() >= REPORT_EVERY {
            self.emit(stage, received, total, None);
            self.last = Instant::now();
        }
    }
}

/// Starts installing a release in the background and returns at once; follow it with the
/// `runner-install` events.
pub fn install(app: &AppHandle, data: PathBuf, source: Source, tag: String) -> Result<(), String> {
    check_name(&tag)?;
    let name = source.install_name(&tag);
    if runners_dir(&data).join(&name).exists() {
        return Err(format!("{name} is already installed"));
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let mut running = jobs().lock().map_err(|error| error.to_string())?;
        if running.contains_key(&name) {
            return Err(format!("{name} is already being installed"));
        }
        running.insert(name.clone(), Arc::clone(&cancelled));
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let mut report = Reporter {
            app: &app,
            name: &name,
            last: Instant::now(),
        };
        let result = run(&data, source, &tag, &name, &cancelled, &mut report);
        let work = runners_dir(&data).join(format!(".{name}.work"));
        let _ = fs::remove_dir_all(&work);
        match result {
            Ok(()) => {
                applog::info(format!("Runner {name} installed"));
                report.emit("done", 1, 1, Some("Installed"));
                // The list of runners changed.
                let _ = app.emit("runners-changed", ());
            }
            Err(Stop::Cancelled) => {
                applog::info(format!("Runner {name}: install cancelled"));
                report.emit("cancelled", 0, 0, Some("Cancelled"));
            }
            Err(Stop::Failed(message)) => {
                applog::error(format!("Runner {name}: {message}"));
                let _ = app.emit(
                    "runner-install",
                    json!({ "name": name, "stage": "failed", "received": 0, "total": 0, "line": message }),
                );
            }
        }
        if let Ok(mut running) = jobs().lock() {
            running.remove(&name);
        }
    });
    Ok(())
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), Stop> {
    if cancelled.load(Ordering::Relaxed) {
        Err(Stop::Cancelled)
    } else {
        Ok(())
    }
}

fn run(
    data: &Path,
    source: Source,
    tag: &str,
    name: &str,
    cancelled: &AtomicBool,
    report: &mut Reporter,
) -> Result<(), Stop> {
    let root = runners_dir(data);
    let work = root.join(format!(".{name}.work"));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work)?;

    report.stage(
        "prepare",
        0,
        &format!("Looking for {} on GitHub", source.asset_name(tag)),
    );
    let release = releases(source)
        .map_err(Stop::Failed)?
        .into_iter()
        .find(|release| release.tag_name == tag)
        .ok_or_else(|| Stop::Failed(format!("release {tag} not found")))?;
    let wanted = source.asset_name(tag);
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == wanted)
        .ok_or_else(|| Stop::Failed(format!("{wanted} is not part of release {tag}")))?;
    check_cancelled(cancelled)?;

    // Download.
    let archive = work.join(&asset.name);
    report.stage(
        "download",
        asset.size,
        &format!("Downloading {} ({})", asset.name, human(asset.size)),
    );
    download(&asset, &archive, cancelled, report)?;

    // Verification.
    check_cancelled(cancelled)?;
    match asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
    {
        Some(expected) => {
            report.stage("verify", 0, "Checking the SHA-256 of the archive");
            let actual = sha256(&archive)?;
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(Stop::Failed(format!(
                    "the download is corrupted (expected SHA-256 {expected}, got {actual})"
                )));
            }
            report.stage("verify", 0, "Checksum matches");
        }
        None => report.stage(
            "verify",
            0,
            "GitHub published no checksum for this file: not verified",
        ),
    }

    // Extraction.
    check_cancelled(cancelled)?;
    let unpacked = work.join("unpacked");
    fs::create_dir_all(&unpacked)?;
    extract(&archive, &unpacked, cancelled, report)?;
    let _ = fs::remove_file(&archive);

    let top = single_folder(&unpacked)?;
    let Some((kind, _)) = entry_point(&top) else {
        return Err(Stop::Failed(
            "the archive does not hold a Proton or Wine build".into(),
        ));
    };
    check_cancelled(cancelled)?;
    let target = root.join(name);
    fs::rename(&top, &target)?;
    report.stage(
        "extract",
        0,
        &format!(
            "Installed as a {} runner in {}",
            kind.as_str(),
            target.display()
        ),
    );
    Ok(())
}

fn download(
    asset: &ReleaseAsset,
    to: &Path,
    cancelled: &AtomicBool,
    report: &mut Reporter,
) -> Result<(), Stop> {
    let mut response = agent()
        .get(&asset.browser_download_url)
        .call()
        .map_err(|error| format!("download failed: {error}"))?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(asset.size);
    let mut body = response.body_mut().as_reader();
    let mut file = File::create(to)?;
    let mut buffer = vec![0u8; 256 * 1024];
    let mut received = 0u64;
    loop {
        check_cancelled(cancelled)?;
        let read = body
            .read(&mut buffer)
            .map_err(|error| format!("download interrupted: {error}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        received += read as u64;
        report.progress("download", received, total);
    }
    file.flush()?;
    if received != total {
        return Err(Stop::Failed(format!(
            "the download stopped at {} of {}",
            human(received),
            human(total)
        )));
    }
    report.emit("download", received, total, Some("Download finished"));
    Ok(())
}

fn sha256(file: &Path) -> Result<String, Stop> {
    let output = wine::host_env(&mut Command::new("sha256sum"))
        .arg(file)
        .output()
        .map_err(|error| format!("sha256sum is not available: {error}"))?;
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .map(str::to_string)
        .ok_or_else(|| Stop::Failed("sha256sum gave no result".into()))
}

/// Unpacks with `tar`, counting entries first so the progress is a real share of the work.
fn extract(
    archive: &Path,
    into: &Path,
    cancelled: &AtomicBool,
    report: &mut Reporter,
) -> Result<(), Stop> {
    report.stage("extract", 0, "Reading the archive");
    let listing = wine::host_env(&mut Command::new("tar"))
        .arg("-tf")
        .arg(archive)
        .stderr(Stdio::null())
        .output()
        .map_err(|error| format!("tar is not available: {error}"))?;
    let total = listing.stdout.iter().filter(|&&byte| byte == b'\n').count() as u64;
    check_cancelled(cancelled)?;

    report.stage("extract", total, &format!("Extracting {total} files"));
    let mut child = wine::host_env(&mut Command::new("tar"))
        .arg("-xvf")
        .arg(archive)
        .arg("-C")
        .arg(into)
        .arg("--no-same-owner")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start tar: {error}"))?;
    let stderr = child.stderr.take();
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut stderr) = stderr {
            let _ = stderr.read_to_string(&mut text);
        }
        text
    });
    let mut done = 0u64;
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = line;
            done += 1;
            if cancelled.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Stop::Cancelled);
            }
            report.progress("extract", done, total);
        }
    }
    let status = child.wait()?;
    let errors = errors.join().unwrap_or_default();
    if !status.success() {
        return Err(Stop::Failed(format!(
            "tar failed ({status}): {}",
            errors.lines().next().unwrap_or("no message")
        )));
    }
    report.emit("extract", total, total, Some("Extraction finished"));
    Ok(())
}

/// The one folder an archive unpacks to.
fn single_folder(dir: &Path) -> Result<PathBuf, Stop> {
    let mut folders: Vec<PathBuf> = fs::read_dir(dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    match folders.len() {
        1 => Ok(folders.remove(0)),
        count => Err(Stop::Failed(format!(
            "the archive should hold one folder, it holds {count}"
        ))),
    }
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_cannot_escape_the_runners_folder() {
        assert!(check_name("GE-Proton11-12").is_ok());
        assert!(check_name("wine-11.21-staging-amd64-wow64").is_ok());
        for bad in ["", ".", "..", "../x", "a/b", ".hidden", "a b"] {
            assert!(check_name(bad).is_err(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn archive_names_follow_each_source() {
        assert_eq!(
            Source::ProtonGe.asset_name("GE-Proton11-12"),
            "GE-Proton11-12-x86_64.tar.gz"
        );
        assert_eq!(
            Source::ProtonGe.install_name("GE-Proton11-12"),
            "GE-Proton11-12"
        );
        assert_eq!(
            Source::Wine.asset_name("11.21"),
            "wine-11.21-amd64-wow64.tar.xz"
        );
        assert_eq!(
            Source::WineStaging.install_name("11.21"),
            "wine-11.21-staging-amd64-wow64"
        );
    }

    #[test]
    fn sizes_read_like_a_download_manager() {
        assert_eq!(human(512), "512 B");
        assert_eq!(human(99_958_200), "100.0 MB");
        assert_eq!(human(566_859_228), "566.9 MB");
    }

    #[test]
    fn finds_proton_and_wine_entry_points() {
        let dir = std::env::temp_dir().join(format!("lutra-runners-{}", std::process::id()));
        let proton = dir.join("GE-Proton1");
        let wine = dir.join("wine-1/bin");
        fs::create_dir_all(&proton).unwrap();
        fs::create_dir_all(&wine).unwrap();
        fs::write(proton.join("proton"), "").unwrap();
        fs::write(wine.join("wine"), "").unwrap();
        assert_eq!(entry_point(&proton).unwrap().0, RunnerKind::Proton);
        assert_eq!(
            entry_point(&dir.join("wine-1")).unwrap().0,
            RunnerKind::Wine
        );
        assert!(entry_point(&dir).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
