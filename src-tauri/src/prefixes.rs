//! Wine prefixes: what is in a folder, which runner can use it, and which ones exist on this
//! machine. A game's prefix is chosen from this list, and a folder that cannot work with the
//! chosen runner is refused with the reason instead of failing at launch.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::vdf;
use crate::wine::RunnerKind;

/// What made a prefix: Wine keeps `system.reg` at its root, Proton one level down in `pfx/`.
pub fn classify(path: &Path) -> Option<RunnerKind> {
    if path.join("pfx/system.reg").is_file() {
        Some(RunnerKind::Proton)
    } else if path.join("system.reg").is_file() {
        Some(RunnerKind::Wine)
    } else {
        None
    }
}

/// `win64` or `win32`, from the registry header Wine writes.
pub fn architecture(path: &Path) -> Option<&'static str> {
    let root = match classify(path)? {
        RunnerKind::Proton => path.join("pfx"),
        _ => path.to_path_buf(),
    };
    let text = fs::read_to_string(root.join("system.reg")).ok()?;
    text.lines().take(8).find_map(|line| match line.trim() {
        "#arch=win64" => Some("win64"),
        "#arch=win32" => Some("win32"),
        _ => None,
    })
}

fn is_empty_dir(path: &Path) -> bool {
    fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_none())
}

/// Can `kind` run games from this folder? A new or empty folder can: the runner creates the
/// prefix there. A prefix made by the other runner, or a folder full of something else, cannot.
pub fn check_usable(path: &Path, kind: RunnerKind) -> Result<(), String> {
    if kind == RunnerKind::Native {
        return Ok(());
    }
    if path.as_os_str().is_empty() {
        return Err("choose a prefix".into());
    }
    match classify(path) {
        Some(found) if found == kind => Ok(()),
        Some(found) => Err(format!(
            "this prefix was created by {}: it cannot be used with {}",
            found.as_str(),
            kind.as_str()
        )),
        None if !path.exists() || is_empty_dir(path) => Ok(()),
        None if path.is_dir() => Err(format!(
            "{} is not a Wine prefix (no registry found); pick an empty folder to create a new one",
            path.display()
        )),
        None => Err(format!("{} is not a folder", path.display())),
    }
}

/// Where a prefix lives, which tells the user what it is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Lutra,
    Steam,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefixInfo {
    pub path: String,
    pub name: String,
    pub origin: Origin,
    /// `None` for a folder where nothing ran yet.
    pub kind: Option<RunnerKind>,
    pub arch: Option<&'static str>,
}

fn steam_game_name(steamapps: &Path, app_id: &str) -> Option<String> {
    let text = fs::read_to_string(steamapps.join(format!("appmanifest_{app_id}.acf"))).ok()?;
    let manifest = vdf::parse(&text).ok()?;
    manifest
        .path(&["AppState", "name"])
        .and_then(vdf::Value::as_str)
        .map(str::to_string)
}

fn describe(path: PathBuf, name: String, origin: Origin) -> PrefixInfo {
    PrefixInfo {
        name,
        origin,
        kind: classify(&path),
        arch: architecture(&path),
        path: path.to_string_lossy().into_owned(),
    }
}

/// Prefixes Lutra created (`<data>/prefixes/*`) and Steam's own (`compatdata/*`), so a game can
/// share one. Only folders that hold a real prefix are listed.
pub fn list(lutra_prefixes: &Path, steam_roots: &[PathBuf]) -> Vec<PrefixInfo> {
    let mut found = Vec::new();

    if let Ok(entries) = fs::read_dir(lutra_prefixes) {
        for entry in entries.flatten().filter(|entry| entry.path().is_dir()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let info = describe(entry.path(), name, Origin::Lutra);
            if info.kind.is_some() {
                found.push(info);
            }
        }
    }

    for root in steam_roots {
        let steamapps = root.join("steamapps");
        let Ok(entries) = fs::read_dir(steamapps.join("compatdata")) else {
            continue;
        };
        for entry in entries.flatten().filter(|entry| entry.path().is_dir()) {
            let app_id = entry.file_name().to_string_lossy().into_owned();
            let name =
                steam_game_name(&steamapps, &app_id).unwrap_or_else(|| format!("Steam {app_id}"));
            let info = describe(entry.path(), name, Origin::Steam);
            if info.kind.is_some() {
                found.push(info);
            }
        }
    }

    found.sort_by(|a, b| {
        (a.origin as u8, a.name.to_lowercase()).cmp(&(b.origin as u8, b.name.to_lowercase()))
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wine_prefix(path: &Path, arch: &str) {
        fs::create_dir_all(path).unwrap();
        fs::write(
            path.join("system.reg"),
            format!(
                "WINE REGISTRY Version 2\n;; All keys relative to \\\\Machine\n\n#arch={arch}\n"
            ),
        )
        .unwrap();
    }

    fn proton_prefix(path: &Path) {
        fs::create_dir_all(path.join("pfx")).unwrap();
        fs::write(
            path.join("pfx/system.reg"),
            "WINE REGISTRY Version 2\n#arch=win64\n",
        )
        .unwrap();
    }

    #[test]
    fn wine_and_proton_prefixes_are_told_apart_with_their_architecture() {
        let dir = tempfile::tempdir().unwrap();
        wine_prefix(&dir.path().join("w"), "win32");
        proton_prefix(&dir.path().join("p"));
        fs::create_dir_all(dir.path().join("empty")).unwrap();

        assert_eq!(classify(&dir.path().join("w")), Some(RunnerKind::Wine));
        assert_eq!(architecture(&dir.path().join("w")), Some("win32"));
        assert_eq!(classify(&dir.path().join("p")), Some(RunnerKind::Proton));
        assert_eq!(architecture(&dir.path().join("p")), Some("win64"));
        assert_eq!(classify(&dir.path().join("empty")), None);
    }

    #[test]
    fn a_prefix_made_by_the_other_runner_is_refused_with_the_reason() {
        let dir = tempfile::tempdir().unwrap();
        wine_prefix(&dir.path().join("w"), "win64");
        proton_prefix(&dir.path().join("p"));

        assert!(check_usable(&dir.path().join("w"), RunnerKind::Wine).is_ok());
        assert!(check_usable(&dir.path().join("p"), RunnerKind::Proton).is_ok());
        let error = check_usable(&dir.path().join("w"), RunnerKind::Proton).unwrap_err();
        assert!(
            error.contains("wine") && error.contains("proton"),
            "{error}"
        );
        assert!(check_usable(&dir.path().join("p"), RunnerKind::Wine).is_err());
    }

    #[test]
    fn new_or_empty_folders_are_fine_but_random_ones_are_not() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("empty")).unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::write(dir.path().join("docs/notes.txt"), "x").unwrap();
        fs::write(dir.path().join("file"), "x").unwrap();

        assert!(check_usable(&dir.path().join("missing"), RunnerKind::Wine).is_ok());
        assert!(check_usable(&dir.path().join("empty"), RunnerKind::Proton).is_ok());
        assert!(check_usable(&dir.path().join("docs"), RunnerKind::Wine).is_err());
        assert!(check_usable(&dir.path().join("file"), RunnerKind::Wine).is_err());
        assert!(check_usable(Path::new(""), RunnerKind::Wine).is_err());
        assert!(check_usable(&dir.path().join("docs"), RunnerKind::Native).is_ok());
    }

    #[test]
    fn listing_finds_lutra_and_steam_prefixes_with_game_names() {
        let dir = tempfile::tempdir().unwrap();
        let lutra = dir.path().join("prefixes");
        wine_prefix(&lutra.join("abc"), "win64");
        fs::create_dir_all(lutra.join("not-a-prefix")).unwrap();

        let steam = dir.path().join("steam");
        proton_prefix(&steam.join("steamapps/compatdata/730"));
        proton_prefix(&steam.join("steamapps/compatdata/999"));
        fs::write(
            steam.join("steamapps/appmanifest_730.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\"730\"\n\t\"name\"\t\"Counter-Strike 2\"\n}\n",
        )
        .unwrap();

        let list = list(&lutra, &[steam]);
        let names: Vec<(&str, Origin)> = list.iter().map(|p| (p.name.as_str(), p.origin)).collect();
        assert_eq!(
            names,
            vec![
                ("abc", Origin::Lutra),
                ("Counter-Strike 2", Origin::Steam),
                ("Steam 999", Origin::Steam)
            ]
        );
        assert_eq!(list[0].kind, Some(RunnerKind::Wine));
    }
}
