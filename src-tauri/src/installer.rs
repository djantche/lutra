//! Installing a game from its setup program: the installer runs in a Wine prefix, and afterwards
//! the programs that appeared in the prefix are the candidates for the game's own executable.
//! Nothing is guessed from the installer: the prefix is compared before and after.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// How deep under `drive_c` programs are looked for.
const MAX_DEPTH: usize = 7;

/// Folders that Wine (or Windows) fills in itself; nothing a user would launch lives there.
const NOISE_DIRS: &[&str] = &[
    "windows",
    "internet explorer",
    "windows nt",
    "windows media player",
    "windows defender",
    "windowspowershell",
    "common files",
    "package cache",
    "microsoft.net",
    "temp",
    "$recycle.bin",
    "wine",
    "_commonredist",
    "redist",
    "redistributables",
    "directx",
    "vcredist",
    "dotnet",
    "dotnetfx",
    "support",
];

/// Programs that come with a game but are not the game: uninstallers, crash reporters, runtimes.
const NOISE_NAMES: &[&str] = &[
    "unins",
    "uninst",
    "uninstall",
    "setup",
    "install",
    "vcredist",
    "vc_redist",
    "dxsetup",
    "dxwebsetup",
    "dotnet",
    "crashhandler",
    "crashreport",
    "crashpad",
    "unitycrashhandler",
    "unrealcefsubprocess",
    "notification_helper",
    "updater",
    "update",
    "launcherhelper",
    "cefprocess",
    "ue4prereqsetup",
    "oalinst",
    "physx",
    "7z",
    "unrar",
];

/// A program that appeared during the installation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Absolute path on the host.
    pub path: String,
    /// Path relative to `drive_c`, for display.
    pub shown: String,
    pub size: u64,
    pub score: i64,
}

fn compact(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_exe(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
}

fn walk(dir: &Path, depth: usize, found: &mut HashSet<PathBuf>) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Symlinks (the `dosdevices` drives) would lead out of the prefix.
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !NOISE_DIRS.contains(&name.as_str()) {
                walk(&path, depth + 1, found);
            }
        } else if is_exe(&path) {
            found.insert(path);
        }
    }
}

/// Every program under a prefix's `drive_c`, apart from what Wine puts there itself.
pub fn scan_programs(drive_c: &Path) -> HashSet<PathBuf> {
    let mut found = HashSet::new();
    walk(drive_c, 0, &mut found);
    found
}

fn is_noise(path: &Path) -> bool {
    let stem = compact(
        &path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    NOISE_NAMES
        .iter()
        .any(|noise| stem.starts_with(&compact(noise)))
}

/// The programs in `after` that were not in `before`, most likely game first.
///
/// A program scores higher the bigger it is (games are big, helpers small), when its name
/// resembles the game's, and the closer it sits to the install folder's top.
pub fn candidates(
    before: &HashSet<PathBuf>,
    after: &HashSet<PathBuf>,
    drive_c: &Path,
    title: &str,
) -> Vec<Candidate> {
    let wanted = compact(title);
    let mut found: Vec<Candidate> = after
        .difference(before)
        .filter(|path| !is_noise(path))
        .map(|path| {
            let size = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
            let relative = path.strip_prefix(drive_c).unwrap_or(path);
            let depth = relative.components().count() as i64;
            let stem = compact(
                &path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
            let mut score = (size / (1024 * 1024)).min(300) as i64;
            if !wanted.is_empty()
                && !stem.is_empty()
                && (wanted.contains(&stem) || stem.contains(&wanted))
            {
                score += 200;
            }
            score -= (depth - 4).max(0) * 15;
            Candidate {
                path: path.to_string_lossy().into_owned(),
                shown: relative.to_string_lossy().into_owned(),
                size,
                score,
            }
        })
        .collect();
    found.sort_by(|a, b| b.score.cmp(&a.score).then(a.shown.cmp(&b.shown)));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, bytes: usize) -> PathBuf {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, vec![0u8; bytes]).unwrap();
        path
    }

    #[test]
    fn wine_s_own_folders_are_not_scanned() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "windows/system32/notepad.exe", 10);
        write(
            root.path(),
            "Program Files/Internet Explorer/iexplore.exe",
            10,
        );
        let game = write(root.path(), "Program Files/Cool Game/cool.exe", 10);
        let found = scan_programs(root.path());
        assert_eq!(found, HashSet::from([game]));
    }

    #[test]
    fn only_what_the_installer_added_is_a_candidate() {
        let root = tempfile::tempdir().unwrap();
        let old = write(root.path(), "Program Files/Other/other.exe", 10);
        let before = scan_programs(root.path());
        let game = write(
            root.path(),
            "Program Files/Cool Game/cool.exe",
            5 * 1024 * 1024,
        );
        let after = scan_programs(root.path());
        assert!(after.contains(&old));

        let list = candidates(&before, &after, root.path(), "Cool Game");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, game.to_string_lossy());
        assert_eq!(list[0].shown, "Program Files/Cool Game/cool.exe");
    }

    #[test]
    fn uninstallers_crash_reporters_and_runtimes_are_left_out() {
        let root = tempfile::tempdir().unwrap();
        let before = HashSet::new();
        write(root.path(), "Games/Cool/unins000.exe", 100);
        write(root.path(), "Games/Cool/UnityCrashHandler64.exe", 100);
        write(root.path(), "Games/Cool/vc_redist.x64.exe", 100);
        write(root.path(), "Games/Cool/Setup.exe", 100);
        let game = write(root.path(), "Games/Cool/Cool.exe", 100);
        let after = scan_programs(root.path());
        let list = candidates(&before, &after, root.path(), "Cool");
        assert_eq!(
            list.iter().map(|c| c.path.clone()).collect::<Vec<_>>(),
            vec![game.to_string_lossy().into_owned()]
        );
    }

    #[test]
    fn the_likeliest_game_program_comes_first() {
        let root = tempfile::tempdir().unwrap();
        let before = HashSet::new();
        write(root.path(), "Games/Cool/tools/editor.exe", 80 * 1024 * 1024);
        write(root.path(), "Games/Cool/CoolGame.exe", 20 * 1024 * 1024);
        write(root.path(), "Games/Cool/launcher.exe", 2 * 1024 * 1024);
        let after = scan_programs(root.path());
        let list = candidates(&before, &after, root.path(), "Cool Game");
        assert_eq!(list[0].shown, "Games/Cool/CoolGame.exe");
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn symlinked_drives_do_not_lead_out_of_the_prefix() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        write(outside.path(), "elsewhere.exe", 10);
        fs::create_dir_all(root.path().join("drive_c")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("drive_c/link")).unwrap();
        assert!(scan_programs(&root.path().join("drive_c")).is_empty());
    }
}
