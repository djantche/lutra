//! Reads the local Steam installation: libraries, installed apps and play time.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::vdf::{self, Value};

/// `StateFlags` bit set once an app is fully installed.
const STATE_FULLY_INSTALLED: u32 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct SteamGame {
    pub app_id: u32,
    pub name: String,
    pub last_played: Option<i64>,
    pub playtime_minutes: u32,
}

/// Steam stores tools (Proton, runtimes, redistributables) next to games, with the same manifests.
fn is_tool(name: &str, install_dir: &str) -> bool {
    const NAME_PREFIXES: [&str; 3] = ["Proton ", "Proton-", "Steam Linux Runtime"];
    const DIR_PREFIXES: [&str; 3] = ["Proton", "SteamLinuxRuntime", "Steamworks Shared"];
    name == "Proton"
        || name.starts_with("Steamworks Common Redistributables")
        || NAME_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
        || DIR_PREFIXES
            .iter()
            .any(|prefix| install_dir.starts_with(prefix))
}

fn candidate_roots(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ]
}

/// Existing Steam installs, deduplicated (`~/.steam/steam` is usually a symlink).
pub fn steam_roots(home: &Path) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    candidate_roots(home)
        .into_iter()
        .filter(|root| root.join("steamapps").is_dir())
        .filter(|root| seen.insert(fs::canonicalize(root).unwrap_or_else(|_| root.clone())))
        .collect()
}

/// Every library folder of an install, including secondary drives.
fn library_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = vec![root.to_path_buf()];
    let Ok(text) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) else {
        return paths;
    };
    let Ok(doc) = vdf::parse(&text) else {
        return paths;
    };
    if let Some(folders) = doc.get("libraryfolders") {
        for (_, folder) in folders.entries() {
            if let Some(path) = folder.get("path").and_then(Value::as_str) {
                let path = PathBuf::from(path);
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
        }
    }
    paths
}

fn parse_manifest(text: &str) -> Option<SteamGame> {
    let doc = vdf::parse(text).ok()?;
    let state = doc.get("AppState")?;
    let field = |key: &str| state.get(key).and_then(Value::as_str);

    let flags: u32 = field("StateFlags")?.parse().ok()?;
    if flags & STATE_FULLY_INSTALLED == 0 {
        return None;
    }

    let name = field("name")?.to_string();
    if is_tool(&name, field("installdir").unwrap_or_default()) {
        return None;
    }

    Some(SteamGame {
        app_id: field("appid")?.parse().ok()?,
        name,
        last_played: field("LastPlayed")
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|&timestamp| timestamp > 0),
        playtime_minutes: 0,
    })
}

/// Play time and last launch per app, from every Steam user's `localconfig.vdf`.
fn read_user_stats(root: &Path) -> HashMap<u32, (u32, Option<i64>)> {
    let mut stats: HashMap<u32, (u32, Option<i64>)> = HashMap::new();
    let Ok(users) = fs::read_dir(root.join("userdata")) else {
        return stats;
    };

    for user in users.flatten() {
        let path = user.path().join("config/localconfig.vdf");
        let Some(doc) = fs::read_to_string(path)
            .ok()
            .and_then(|text| vdf::parse(&text).ok())
        else {
            continue;
        };
        let Some(apps) = doc.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"])
        else {
            continue;
        };

        for (id, app) in apps.entries() {
            let Ok(app_id) = id.parse::<u32>() else {
                continue;
            };
            let playtime = app
                .get("Playtime")
                .and_then(Value::as_str)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let last_played = app
                .get("LastPlayed")
                .and_then(Value::as_str)
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|&timestamp| timestamp > 0);

            let entry = stats.entry(app_id).or_insert((0, None));
            entry.0 = entry.0.max(playtime);
            entry.1 = entry.1.max(last_played);
        }
    }
    stats
}

/// Installed Steam games found under `home`, without tools, one entry per app.
pub fn scan(home: &Path) -> Vec<SteamGame> {
    let mut games: HashMap<u32, SteamGame> = HashMap::new();

    for root in steam_roots(home) {
        let stats = read_user_stats(&root);

        for library in library_paths(&root) {
            let Ok(entries) = fs::read_dir(library.join("steamapps")) else {
                continue;
            };
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                if !(file_name.starts_with("appmanifest_") && file_name.ends_with(".acf")) {
                    continue;
                }
                let Some(mut game) = fs::read_to_string(entry.path())
                    .ok()
                    .and_then(|text| parse_manifest(&text))
                else {
                    continue;
                };

                if let Some(&(playtime, last_played)) = stats.get(&game.app_id) {
                    game.playtime_minutes = playtime;
                    game.last_played = game.last_played.max(last_played);
                }
                games.insert(game.app_id, game);
            }
        }
    }

    let mut games: Vec<SteamGame> = games.into_values().collect();
    games.sort_by_key(|game| game.name.to_lowercase());
    games
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(appid: u32, name: &str, install_dir: &str, flags: u32, last_played: i64) -> String {
        format!(
            "\"AppState\"\n{{\n\t\"appid\"\t\t\"{appid}\"\n\t\"name\"\t\t\"{name}\"\n\t\"installdir\"\t\t\"{install_dir}\"\n\t\"StateFlags\"\t\t\"{flags}\"\n\t\"LastPlayed\"\t\t\"{last_played}\"\n}}\n"
        )
    }

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn keeps_installed_games_and_skips_tools() {
        assert!(parse_manifest(&manifest(10, "Hollow Ember", "Hollow Ember", 4, 0)).is_some());
        assert!(
            parse_manifest(&manifest(
                1493710,
                "Proton Experimental",
                "Proton - Experimental",
                4,
                0
            ))
            .is_none()
        );
        assert!(
            parse_manifest(&manifest(
                4183110,
                "Steam Linux Runtime 4.0",
                "SteamLinuxRuntime_4",
                4,
                0
            ))
            .is_none()
        );
        assert!(
            parse_manifest(&manifest(
                228980,
                "Steamworks Common Redistributables",
                "Steamworks Shared",
                4,
                0
            ))
            .is_none()
        );
    }

    #[test]
    fn skips_games_that_are_not_fully_installed() {
        assert!(
            parse_manifest(&manifest(10, "Half Downloaded", "Half Downloaded", 1026, 0)).is_none()
        );
    }

    #[test]
    fn zero_last_played_means_never() {
        assert_eq!(
            parse_manifest(&manifest(10, "A", "A", 4, 0))
                .unwrap()
                .last_played,
            None
        );
        assert_eq!(
            parse_manifest(&manifest(10, "A", "A", 4, 1700000000))
                .unwrap()
                .last_played,
            Some(1700000000)
        );
    }

    #[test]
    fn scans_every_library_and_merges_user_stats() {
        let home = tempfile::tempdir().unwrap();
        let second_drive = tempfile::tempdir().unwrap();
        let steam = home.path().join(".local/share/Steam");

        write(
            &steam.join("steamapps/libraryfolders.vdf"),
            &format!(
                "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
                steam.display(),
                second_drive.path().display()
            ),
        );
        write(
            &steam.join("steamapps/appmanifest_10.acf"),
            &manifest(10, "Hollow Ember", "Hollow Ember", 4, 0),
        );
        write(
            &steam.join("steamapps/appmanifest_1493710.acf"),
            &manifest(
                1493710,
                "Proton Experimental",
                "Proton - Experimental",
                4,
                0,
            ),
        );
        write(
            &second_drive.path().join("steamapps/appmanifest_20.acf"),
            &manifest(20, "Tideborn", "Tideborn", 4, 1700000000),
        );
        write(
            &steam.join("userdata/1234/config/localconfig.vdf"),
            "\"UserLocalConfigStore\"\n{\n\t\"Software\"\n\t{\n\t\t\"Valve\"\n\t\t{\n\t\t\t\"Steam\"\n\t\t\t{\n\t\t\t\t\"apps\"\n\t\t\t\t{\n\t\t\t\t\t\"10\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"Playtime\"\t\t\"125\"\n\t\t\t\t\t\t\"LastPlayed\"\t\t\"1750000000\"\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n",
        );

        let games = scan(home.path());

        assert_eq!(games.len(), 2, "tools must be skipped: {games:?}");
        assert_eq!(games[0].name, "Hollow Ember");
        assert_eq!(games[0].playtime_minutes, 125);
        assert_eq!(games[0].last_played, Some(1750000000));
        assert_eq!(games[1].name, "Tideborn");
        assert_eq!(games[1].last_played, Some(1700000000));
    }

    #[test]
    fn missing_steam_yields_no_games() {
        let home = tempfile::tempdir().unwrap();
        assert!(scan(home.path()).is_empty());
    }
}
