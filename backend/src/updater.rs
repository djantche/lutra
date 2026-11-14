//! Updating Lutra from its GitHub releases.
//!
//! `check` asks GitHub for the latest release and compares it with the running version. An
//! AppImage can install the update itself: it downloads the release's AppImage next to the
//! running one, checks the SHA-256 GitHub published for it (no checksum, no install), keeps the
//! old file as `<name>.previous` and puts the new one in its place. The running process keeps its
//! own copy open, so replacing the file under it is safe; the interface then offers to restart.
//! Any other install (a deb package, a build from source) is sent to the release page instead:
//! replacing files a package manager owns is not Lutra's job.
//!
//! Progress arrives as `update-install` events.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::applog;
use crate::host::AppHandle;
use crate::wine;

const REPO: &str = "djantche/lutra";
/// The version this build is: the shell and the backend are released together.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

const REPORT_EVERY: Duration = Duration::from_millis(150);

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
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    published_at: String,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

/// What the interface needs to tell about an update.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    /// The release notes (Markdown).
    pub notes: String,
    pub page: String,
    pub published: String,
    /// Lutra can install it itself: it runs from an AppImage it may replace, and the release has
    /// a checked AppImage for this machine.
    pub can_install: bool,
    /// Size of the download when `can_install`.
    pub size: u64,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent("Lutra")
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .build()
        .into()
}

/// `0.6.0`, `v0.6.0` or `0.7.0-beta.1` as numbers; what comes after `-` or `+` is ignored.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let core = text
        .trim()
        .trim_start_matches('v')
        .split(['-', '+'])
        .next()?;
    let mut parts = core.split('.').map(|part| part.parse::<u64>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

/// The AppImage Lutra runs from, if it is one and the folder it sits in lets us replace it.
fn running_appimage() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("APPIMAGE")?);
    let folder = path.parent()?;
    let writable = fs::metadata(folder).is_ok_and(|meta| !meta.permissions().readonly())
        && fs::metadata(&path).is_ok();
    writable.then_some(path)
}

/// The release's AppImage for this machine's architecture.
fn appimage_asset(release: &Release) -> Option<&ReleaseAsset> {
    let words: &[&str] = match std::env::consts::ARCH {
        "aarch64" => &["aarch64", "arm64"],
        _ => &["x86_64", "amd64"],
    };
    release.assets.iter().find(|asset| {
        asset.name.ends_with(".AppImage") && words.iter().any(|word| asset.name.contains(word))
    })
}

fn sha256_of(asset: &ReleaseAsset) -> Option<&str> {
    asset.digest.as_deref()?.strip_prefix("sha256:")
}

fn latest_release() -> Result<Release, String> {
    agent()
        .get(&format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("cannot reach GitHub: {error}"))?
        .body_mut()
        .read_json::<Release>()
        .map_err(|error| format!("unexpected answer from GitHub: {error}"))
}

fn describe(release: &Release, appimage: bool) -> UpdateInfo {
    let asset = appimage_asset(release).filter(|asset| appimage && sha256_of(asset).is_some());
    UpdateInfo {
        current: CURRENT.to_string(),
        latest: release.tag_name.trim_start_matches('v').to_string(),
        available: is_newer(&release.tag_name, CURRENT),
        notes: release.body.clone().unwrap_or_default(),
        page: release.html_url.clone(),
        published: release.published_at.clone(),
        can_install: asset.is_some(),
        size: asset.map_or(0, |asset| asset.size),
    }
}

/// Asks GitHub whether a newer release exists.
pub fn check() -> Result<UpdateInfo, String> {
    let release = latest_release()?;
    let info = describe(&release, running_appimage().is_some());
    applog::info(format!(
        "Update check: running {}, latest {}{}",
        info.current,
        info.latest,
        if info.available { " (newer)" } else { "" }
    ));
    Ok(info)
}

static RUNNING: AtomicBool = AtomicBool::new(false);
static CANCELLED: AtomicBool = AtomicBool::new(false);

pub fn cancel() {
    CANCELLED.store(true, Ordering::SeqCst);
}

enum Stop {
    Cancelled,
    Failed(String),
}

impl<E: std::fmt::Display> From<E> for Stop {
    fn from(error: E) -> Self {
        Stop::Failed(error.to_string())
    }
}

/// Downloads, checks and puts in place the latest release's AppImage.
pub fn install(app: &AppHandle) -> Result<(), String> {
    let target = running_appimage().ok_or("not_appimage")?;
    // The release is read again here: what to install is not for the interface to say.
    let release = latest_release()?;
    if !is_newer(&release.tag_name, CURRENT) {
        return Err("up_to_date".into());
    }
    let asset = appimage_asset(&release).ok_or("no_asset")?;
    let expected = sha256_of(asset).ok_or("no_checksum")?.to_string();
    let (url, size, version) = (
        asset.browser_download_url.clone(),
        asset.size,
        release.tag_name.trim_start_matches('v').to_string(),
    );
    if !url.starts_with("https://github.com/") {
        return Err("unexpected download address".into());
    }
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    CANCELLED.store(false, Ordering::SeqCst);

    let app = app.clone();
    std::thread::spawn(move || {
        applog::info(format!("Updating to {version}: {url}"));
        let result = run(&app, &target, &url, size, &expected);
        RUNNING.store(false, Ordering::SeqCst);
        let (state, message) = match result {
            Ok(()) => ("done", version),
            Err(Stop::Cancelled) => ("cancelled", String::new()),
            Err(Stop::Failed(error)) => ("failed", error),
        };
        applog::info(format!("Update: {state} {message}"));
        let _ = app.emit(
            "update-install",
            json!({ "state": state, "message": message }),
        );
    });
    Ok(())
}

fn progress(app: &AppHandle, state: &str, received: u64, total: u64) {
    let _ = app.emit(
        "update-install",
        json!({ "state": state, "received": received, "total": total }),
    );
}

fn run(app: &AppHandle, target: &Path, url: &str, size: u64, expected: &str) -> Result<(), Stop> {
    let name = target.file_name().ok_or("no file name")?.to_string_lossy();
    // Beside the running file, so the final rename never crosses file systems.
    let partial = target.with_file_name(format!(".{name}.download"));
    let result = download(app, url, &partial, size)
        .and_then(|()| verify(app, &partial, expected))
        .and_then(|()| replace(app, target, &partial));
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

fn download(app: &AppHandle, url: &str, to: &Path, size: u64) -> Result<(), Stop> {
    let mut response = agent()
        .get(url)
        .call()
        .map_err(|error| format!("download failed: {error}"))?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(size);
    let mut body = response.body_mut().as_reader();
    let mut file = File::create(to)?;
    let mut buffer = vec![0u8; 256 * 1024];
    let (mut received, mut last) = (0u64, Instant::now());
    progress(app, "download", 0, total);
    loop {
        if CANCELLED.load(Ordering::Relaxed) {
            return Err(Stop::Cancelled);
        }
        let read = body
            .read(&mut buffer)
            .map_err(|error| format!("download interrupted: {error}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        received += read as u64;
        if last.elapsed() >= REPORT_EVERY {
            last = Instant::now();
            progress(app, "download", received, total);
        }
    }
    file.flush()?;
    if received != total {
        return Err(Stop::Failed(format!(
            "the download stopped at {received} of {total} bytes"
        )));
    }
    progress(app, "download", received, total);
    Ok(())
}

fn verify(app: &AppHandle, file: &Path, expected: &str) -> Result<(), Stop> {
    progress(app, "verify", 0, 0);
    let output = wine::host_env(&mut Command::new("sha256sum"))
        .arg(file)
        .output()
        .map_err(|error| format!("sha256sum is not available: {error}"))?;
    let actual = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .map(str::to_string)
        .ok_or("sha256sum gave no result")?;
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(Stop::Failed(format!(
            "the download is corrupted (expected SHA-256 {expected}, got {actual})"
        )));
    }
    // A last look at what is about to become the program: an ELF file.
    let mut magic = [0u8; 4];
    File::open(file)?.read_exact(&mut magic)?;
    if &magic != b"\x7fELF" {
        return Err(Stop::Failed("the download is not an AppImage".into()));
    }
    Ok(())
}

/// The old file stays as `<name>.previous` (one copy, the one before this update), so a bad
/// release can be undone by hand.
fn replace(app: &AppHandle, target: &Path, new: &Path) -> Result<(), Stop> {
    progress(app, "replace", 0, 0);
    replace_files(target, new)
}

fn replace_files(target: &Path, new: &Path) -> Result<(), Stop> {
    fs::set_permissions(new, fs::Permissions::from_mode(0o755))?;
    let name = target.file_name().ok_or("no file name")?.to_string_lossy();
    let previous = target.with_file_name(format!("{name}.previous"));
    let _ = fs::remove_file(&previous);
    fs::rename(target, &previous)?;
    if let Err(error) = fs::rename(new, target) {
        let _ = fs::rename(&previous, target);
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(assets: &[(&str, Option<&str>)]) -> Release {
        Release {
            tag_name: "v0.7.0".into(),
            body: Some("notes".into()),
            html_url: "https://github.com/djantche/lutra/releases/tag/v0.7.0".into(),
            published_at: "2026-11-20T10:00:00Z".into(),
            assets: assets
                .iter()
                .map(|(name, digest)| ReleaseAsset {
                    name: name.to_string(),
                    size: 10,
                    digest: digest.map(str::to_string),
                    browser_download_url: format!("https://github.com/x/{name}"),
                })
                .collect(),
        }
    }

    #[test]
    fn versions_are_compared_as_numbers() {
        assert!(is_newer("v0.7.0", "0.6.0"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.6.0", "0.6.0"));
        assert!(!is_newer("v0.5.2", "0.6.0"));
        // A pre-release tag counts as its number; garbage is never an update.
        assert!(is_newer("0.7.0-beta.1", "0.6.0"));
        assert!(!is_newer("nightly", "0.6.0"));
    }

    #[test]
    fn only_an_appimage_with_a_checksum_for_this_machine_can_be_installed() {
        let arch = if std::env::consts::ARCH == "aarch64" {
            "aarch64"
        } else {
            "x86_64"
        };
        let appimage = format!("Lutra_0.7.0_{arch}.AppImage");
        let good = release(&[
            ("Lutra_0.7.0_amd64.deb", Some("sha256:aa")),
            (appimage.as_str(), Some("sha256:bb")),
        ]);
        let info = describe(&good, true);
        assert!(info.available && info.can_install);
        assert_eq!(info.latest, "0.7.0");

        // Not running from an AppImage: the release page is the way.
        assert!(!describe(&good, false).can_install);
        // No checksum published: nothing to verify the download against.
        let unchecked = release(&[(appimage.as_str(), None)]);
        assert!(!describe(&unchecked, true).can_install);
        // No AppImage for this machine.
        assert!(
            !describe(
                &release(&[("Lutra_0.7.0_amd64.deb", Some("sha256:aa"))]),
                true
            )
            .can_install
        );
    }

    #[test]
    fn replacing_keeps_the_old_file_as_previous() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("Lutra.AppImage");
        let new = dir.path().join(".Lutra.AppImage.download");
        fs::write(&target, "old").unwrap();
        fs::write(&new, "new").unwrap();
        fs::write(dir.path().join("Lutra.AppImage.previous"), "older").unwrap();
        // The events go nowhere in a test: only the file moves are checked.
        let result = replace_files(&target, &new);
        assert!(result.is_ok());
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(
            fs::read_to_string(dir.path().join("Lutra.AppImage.previous")).unwrap(),
            "old"
        );
        assert!(!new.exists());
        assert_eq!(
            fs::metadata(&target).unwrap().permissions().mode() & 0o111,
            0o111
        );
    }

    /// `cargo test -- --ignored --nocapture reads_the_real_latest_release` (network).
    #[test]
    #[ignore = "needs the network"]
    fn reads_the_real_latest_release() {
        let release = latest_release().expect("GitHub answers");
        println!(
            "{} {:?}",
            release.tag_name,
            release
                .assets
                .iter()
                .map(|a| (&a.name, a.digest.is_some()))
                .collect::<Vec<_>>()
        );
        assert!(appimage_asset(&release).is_some_and(|asset| sha256_of(asset).is_some()));
        assert!(parse_version(&release.tag_name).is_some());
    }
}
