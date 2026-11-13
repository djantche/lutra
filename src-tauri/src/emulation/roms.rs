//! Finding games in the user's ROM folders, and telling which system each belongs to.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::systems;

const MAX_DEPTH: usize = 6;

/// Formats several systems share: the folder the file sits in says which system it is.
const SHARED_FORMATS: &[&str] = &[
    "iso", "cue", "chd", "bin", "img", "ccd", "mds", "zip", "7z", "m3u", "cdi", "gdi",
];
/// What is never a game even when it has a known extension (disc tracks, covers).
const SKIPPED_FORMATS: &[&str] = &[
    "sbi", "sub", "txt", "nfo", "png", "jpg", "xml", "dat", "sav", "srm", "state",
];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rom {
    pub path: PathBuf,
    /// libretro database name (`Nintendo - Game Boy Advance`).
    pub system: String,
    /// File name without extension, as the thumbnail server names games.
    pub stem: String,
    /// A tidy name to show.
    pub title: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct Scan {
    pub roms: Vec<Rom>,
    /// Disc images or archives found where nothing says which system they are for.
    pub unplaced: usize,
}

/// `Super Mario World (USA) [!]` -> `Super Mario World`.
pub fn clean_title(stem: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in stem.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(if c == '_' { ' ' } else { c }),
            _ => {}
        }
    }
    let title = out.split_whitespace().collect::<Vec<_>>().join(" ");
    // "Legend of Zelda, The" -> "The Legend of Zelda".
    match title.strip_suffix(", The") {
        Some(rest) => format!("The {rest}"),
        None if title.is_empty() => stem.to_string(),
        None => title,
    }
}

/// The system of a file: its extension when only one system uses it, otherwise the nearest
/// folder above it that is named after a system.
fn system_of(path: &Path, roots: &[PathBuf]) -> Option<&'static str> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    if let Some(system) = systems::system_for_extension(&extension) {
        return Some(system);
    }
    // Nearest folder first, stopping at the ROM folder the user configured.
    path.ancestors()
        .skip(1)
        .take_while(|dir| !roots.iter().any(|root| root == dir))
        .find_map(|dir| systems::system_for_folder(&dir.file_name()?.to_string_lossy()))
        .or_else(|| {
            // The configured folder itself may be named after the system.
            roots
                .iter()
                .find(|root| path.starts_with(root))
                .and_then(|root| systems::system_for_folder(&root.file_name()?.to_string_lossy()))
        })
}

fn walk(dir: &Path, depth: usize, roots: &[PathBuf], scan: &mut Scan) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files = Vec::new();
    let mut folders = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => folders.push(entry.path()),
            Ok(kind) if kind.is_file() => files.push(entry.path()),
            _ => {}
        }
    }

    // Disc tracks listed by a playlist belong to it: only the playlist is a game.
    let in_playlists: HashSet<String> = files
        .iter()
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("m3u"))
        })
        .filter_map(|path| fs::read_to_string(path).ok())
        .flat_map(|text| {
            text.lines()
                .map(|line| line.trim().rsplit('/').next().unwrap_or("").to_lowercase())
                .collect::<Vec<_>>()
        })
        .collect();

    for path in files {
        let Some(extension) = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
        else {
            continue;
        };
        if SKIPPED_FORMATS.contains(&extension.as_str()) {
            continue;
        }
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if in_playlists.contains(&file_name) {
            continue;
        }
        match system_of(&path, roots) {
            Some(system) => {
                let stem = path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default();
                scan.roms.push(Rom {
                    title: clean_title(&stem),
                    stem,
                    system: system.to_string(),
                    path,
                });
            }
            None if SHARED_FORMATS.contains(&extension.as_str()) => scan.unplaced += 1,
            None => {}
        }
    }
    for folder in folders {
        walk(&folder, depth + 1, roots, scan);
    }
}

/// Every game under `roots`, sorted by title.
pub fn scan(roots: &[PathBuf]) -> Scan {
    let mut scan = Scan::default();
    for root in roots {
        walk(root, 0, roots, &mut scan);
    }
    scan.roms.sort_by(|a, b| {
        a.title
            .to_lowercase()
            .cmp(&b.title.to_lowercase())
            .then(a.path.cmp(&b.path))
    });
    scan.roms.dedup_by(|a, b| a.path == b.path);
    scan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(root: &Path, relative: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"rom").unwrap();
    }

    #[test]
    fn titles_lose_their_tags_and_articles_move_to_the_front() {
        assert_eq!(
            clean_title("Super Mario World (USA) [!]"),
            "Super Mario World"
        );
        assert_eq!(
            clean_title("Legend of Zelda, The (Europe) (Rev 1)"),
            "The Legend of Zelda"
        );
        assert_eq!(
            clean_title("Final_Fantasy_VII (Disc 1)"),
            "Final Fantasy VII"
        );
        assert_eq!(clean_title("(Unknown)"), "(Unknown)");
    }

    #[test]
    fn games_are_found_by_extension_and_by_folder_for_shared_formats() {
        let root = tempfile::tempdir().unwrap();
        touch(root.path(), "gba/Metroid Fusion (USA).gba");
        touch(root.path(), "anything/Zelda.sfc");
        touch(root.path(), "psx/Crash Bandicoot (USA).chd");
        touch(root.path(), "psx/notes.txt");
        touch(root.path(), "misc/unknown disc.iso");
        touch(root.path(), ".hidden/Secret.nes");

        let scan = scan(&[root.path().to_path_buf()]);
        let found: Vec<(&str, &str)> = scan
            .roms
            .iter()
            .map(|rom| (rom.title.as_str(), rom.system.as_str()))
            .collect();
        assert_eq!(
            found,
            vec![
                ("Crash Bandicoot", "Sony - PlayStation"),
                ("Metroid Fusion", "Nintendo - Game Boy Advance"),
                ("Zelda", "Nintendo - Super Nintendo Entertainment System"),
            ]
        );
        assert_eq!(
            scan.unplaced, 1,
            "the disc image in an unnamed folder is reported"
        );
    }

    #[test]
    fn the_roms_folder_itself_may_be_named_after_the_system() {
        let parent = tempfile::tempdir().unwrap();
        let saturn = parent.path().join("Saturn");
        touch(&saturn, "Nights.cue");
        touch(&saturn, "Nights.bin");
        let scan = scan(std::slice::from_ref(&saturn));
        assert_eq!(scan.roms.len(), 2);
        assert!(scan.roms.iter().all(|rom| rom.system == "Sega - Saturn"));
    }

    #[test]
    fn disc_tracks_listed_in_a_playlist_are_not_separate_games() {
        let root = tempfile::tempdir().unwrap();
        touch(root.path(), "psx/Game (Disc 1).cue");
        touch(root.path(), "psx/Game (Disc 2).cue");
        fs::write(
            root.path().join("psx/Game.m3u"),
            "Game (Disc 1).cue\nGame (Disc 2).cue\n",
        )
        .unwrap();
        let scan = scan(&[root.path().to_path_buf()]);
        assert_eq!(scan.roms.len(), 1);
        assert_eq!(scan.roms[0].stem, "Game");
    }
}
