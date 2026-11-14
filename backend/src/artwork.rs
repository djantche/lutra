//! Downloading and storing game artwork, whichever service it comes from.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024;

/// Hosts serving Steam's own artwork, tried in order.
const STEAM_HOSTS: [&str; 2] = [
    "https://shared.akamai.steamstatic.com/store_item_assets/steam",
    "https://cdn.cloudflare.steamstatic.com/steam",
];

#[derive(Debug)]
pub enum Error {
    /// A SteamGridDB API key was rejected.
    InvalidKey,
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidKey => write!(f, "SteamGridDB rejected this API key"),
            Error::Other(message) => write!(f, "{message}"),
        }
    }
}

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into()
}

fn extension_of(url: &str) -> &str {
    let name = url.split('?').next().unwrap_or(url);
    match name.rsplit('.').next() {
        Some(ext @ ("png" | "jpg" | "jpeg" | "webp")) => ext,
        _ => "png",
    }
}

/// Official portrait capsule of a Steam app.
pub fn steam_poster_urls(app_id: &str) -> Vec<String> {
    STEAM_HOSTS
        .iter()
        .map(|host| format!("{host}/apps/{app_id}/library_600x900.jpg"))
        .collect()
}

/// Official wide hero of a Steam app.
pub fn steam_hero_urls(app_id: &str) -> Vec<String> {
    STEAM_HOSTS
        .iter()
        .map(|host| format!("{host}/apps/{app_id}/library_hero.jpg"))
        .collect()
}

/// Official logo of a Steam app.
pub fn steam_logo_urls(app_id: &str) -> Vec<String> {
    STEAM_HOSTS
        .iter()
        .map(|host| format!("{host}/apps/{app_id}/logo.png"))
        .collect()
}

/// Saves the first URL that answers as `<dir>/<stem>.<ext>`.
/// Returns where the file went and which URL it came from; `Ok(None)` if none of them has the image.
/// Older files with the same stem are replaced.
pub fn download_first(
    agent: &ureq::Agent,
    urls: &[String],
    dir: &Path,
    stem: &str,
) -> Result<Option<(PathBuf, String)>, Error> {
    for url in urls {
        let mut response = match agent.get(url).call() {
            Ok(response) => response,
            Err(ureq::Error::StatusCode(404)) => continue,
            Err(e) => return Err(Error::Other(e.to_string())),
        };
        let bytes = response
            .body_mut()
            .with_config()
            .limit(MAX_IMAGE_BYTES)
            .read_to_vec()
            .map_err(|e| Error::Other(e.to_string()))?;

        fs::create_dir_all(dir).map_err(|e| Error::Other(e.to_string()))?;
        let path = dir.join(format!("{stem}.{}", extension_of(url)));
        fs::write(&path, bytes).map_err(|e| Error::Other(e.to_string()))?;
        remove_stale(dir, stem, &path);
        return Ok(Some((path, url.clone())));
    }
    Ok(None)
}

/// The extension for the image format `bytes` start with, read from its signature: a file's name
/// or a link's ending says nothing reliable about what it holds.
fn sniff_extension(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("jpg"),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some("webp"),
        _ => None,
    }
}

/// Writes an image of the user's as `<dir>/<stem>.<ext>`, replacing the previous one.
/// This copy is what Lutra shows from then on, so the original can move or disappear.
fn store_image(bytes: &[u8], dir: &Path, stem: &str) -> Result<PathBuf, Error> {
    let extension = sniff_extension(bytes)
        .ok_or_else(|| Error::Other("this is not a PNG, JPEG or WebP image".into()))?;
    fs::create_dir_all(dir).map_err(|e| Error::Other(e.to_string()))?;
    let path = dir.join(format!("{stem}.{extension}"));
    fs::write(&path, bytes).map_err(|e| Error::Other(e.to_string()))?;
    remove_stale(dir, stem, &path);
    Ok(path)
}

/// Copies an image file of the user's into `dir` as `<stem>.<ext>`.
pub fn import_file(source: &Path, dir: &Path, stem: &str) -> Result<PathBuf, Error> {
    let metadata =
        fs::metadata(source).map_err(|e| Error::Other(format!("{}: {e}", source.display())))?;
    if !metadata.is_file() {
        return Err(Error::Other(format!("{} is not a file", source.display())));
    }
    if metadata.len() > MAX_IMAGE_BYTES {
        return Err(Error::Other(
            "this image is too large (25 MB at most)".into(),
        ));
    }
    let bytes = fs::read(source).map_err(|e| Error::Other(e.to_string()))?;
    store_image(&bytes, dir, stem)
}

/// Downloads an image from a link the user gave and keeps it in `dir` as `<stem>.<ext>`.
pub fn import_url(
    agent: &ureq::Agent,
    url: &str,
    dir: &Path,
    stem: &str,
) -> Result<PathBuf, Error> {
    let mut response = agent
        .get(url)
        .call()
        .map_err(|e| Error::Other(e.to_string()))?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_IMAGE_BYTES)
        .read_to_vec()
        .map_err(|e| Error::Other(e.to_string()))?;
    store_image(&bytes, dir, stem)
}

/// A new image may have another extension than the previous one: do not leave both behind.
fn remove_stale(dir: &Path, stem: &str, keep: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let same_stem = path.file_stem().is_some_and(|name| name == stem);
        if same_stem && path != keep {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_a_safe_extension() {
        assert_eq!(extension_of("https://x/y/z.jpg?t=1"), "jpg");
        assert_eq!(extension_of("https://x/y/z.webp"), "webp");
        assert_eq!(extension_of("https://x/y/z"), "png");
        assert_eq!(extension_of("https://x/y/z.exe"), "png");
    }

    #[test]
    fn builds_official_steam_urls_for_every_host() {
        let posters = steam_poster_urls("291550");
        assert_eq!(posters.len(), 2);
        assert!(
            posters
                .iter()
                .all(|url| url.ends_with("/apps/291550/library_600x900.jpg"))
        );
        assert!(
            steam_hero_urls("1")
                .iter()
                .all(|url| url.ends_with("/apps/1/library_hero.jpg"))
        );
    }

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    #[test]
    fn recognizes_images_by_content_not_by_name() {
        assert_eq!(sniff_extension(PNG), Some("png"));
        assert_eq!(sniff_extension(&[0xFF, 0xD8, 0xFF, 0xE0, 0]), Some("jpg"));
        assert_eq!(sniff_extension(b"RIFF\x10\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(sniff_extension(b"<!DOCTYPE html>"), None);
        assert_eq!(sniff_extension(b""), None);
    }

    #[test]
    fn imports_a_file_under_the_stable_name_whatever_it_was_called() {
        let source = tempfile::tempdir().unwrap();
        let game = tempfile::tempdir().unwrap();
        let original = source.path().join("My Holiday Photo (final).jpeg");
        fs::write(&original, PNG).unwrap();
        fs::write(game.path().join("poster.jpg"), "old").unwrap();

        let saved = import_file(&original, game.path(), "poster").unwrap();

        // The extension follows the content (it is a PNG), and the old cover is gone.
        assert_eq!(saved, game.path().join("poster.png"));
        assert_eq!(fs::read(&saved).unwrap(), PNG);
        assert!(!game.path().join("poster.jpg").exists());
        // The user's file is only copied, never moved.
        assert!(original.exists());
    }

    #[test]
    fn refuses_what_is_not_an_image() {
        let source = tempfile::tempdir().unwrap();
        let game = tempfile::tempdir().unwrap();
        let text = source.path().join("cover.png");
        fs::write(&text, "not an image").unwrap();

        assert!(import_file(&text, game.path(), "poster").is_err());
        assert!(import_file(source.path(), game.path(), "poster").is_err());
        assert!(import_file(&source.path().join("missing.png"), game.path(), "poster").is_err());
        assert!(!game.path().join("poster.png").exists());
    }

    #[test]
    fn replacing_an_image_removes_the_old_one_with_another_extension() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("poster.png"), "old").unwrap();
        fs::write(dir.path().join("hero.png"), "other image").unwrap();

        let new = dir.path().join("poster.jpg");
        fs::write(&new, "new").unwrap();
        remove_stale(dir.path(), "poster", &new);

        assert!(!dir.path().join("poster.png").exists());
        assert!(new.exists());
        assert!(dir.path().join("hero.png").exists());
    }
}
