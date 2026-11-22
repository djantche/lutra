//! Games the user already set up in other launchers: Lutris, Heroic (sideloaded games and Epic
//! ones) and Bottles. Only what they keep on disk is read, never changed, and a game found here
//! is offered for import: its program, its prefix and a hint of the runner it used.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

/// A game another launcher knows, ready to be imported.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Importable {
    /// Stable between two scans: `lutris:2`, `heroic:<app>`, `bottles:<bottle>:<program>`.
    pub key: String,
    /// `lutris`, `heroic` or `bottles`.
    pub from: &'static str,
    pub title: String,
    pub exe_path: String,
    /// The Wine prefix the other launcher used, when it had one of its own.
    pub prefix_path: Option<String>,
    /// The name of the runner it used (`GE-Proton10-3`, `soda-9.0-1`), to find the same here.
    pub runner_hint: Option<String>,
    /// A Linux program, not a Windows one.
    pub native: bool,
    pub args: String,
    pub working_dir: Option<String>,
    /// The program is not where the other launcher says (a disk that is not mounted, a game
    /// that was deleted).
    pub missing: bool,
    /// Already in Lutra's library.
    pub already: bool,
}

impl Importable {
    fn new(from: &'static str, key: String, title: String, exe_path: String) -> Self {
        Self {
            key,
            from,
            title,
            missing: !Path::new(&exe_path).is_file(),
            exe_path,
            prefix_path: None,
            runner_hint: None,
            native: false,
            args: String::new(),
            working_dir: None,
            already: false,
        }
    }
}

/// Every game found in the launchers installed for this user, sorted by launcher then title.
pub fn discover(home: &Path) -> Vec<Importable> {
    let mut found = Vec::new();
    for root in lutris_roots(home) {
        found.extend(lutris(&root.data, &root.config));
    }
    for root in heroic_roots(home) {
        found.extend(heroic(&root));
    }
    for root in bottles_roots(home) {
        found.extend(bottles(&root));
    }
    found.sort_by(|a, b| (a.from, a.title.to_lowercase()).cmp(&(b.from, b.title.to_lowercase())));
    // The same program seen twice (a Flatpak and a native install share a folder) counts once.
    found.dedup_by(|a, b| a.from == b.from && a.exe_path == b.exe_path);
    found
}

// --- A little YAML -------------------------------------------------------------------------
//
// Lutris and Bottles keep plain, machine-written YAML: nested mappings of scalars, a few lists
// and long values folded over several lines. That is all this reads; anything else becomes null.

fn unquote(raw: &str) -> String {
    let raw = raw.trim();
    for quote in ['"', '\''] {
        if let Some(inner) = raw
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner.replace(&format!("{quote}{quote}"), &quote.to_string());
        }
    }
    raw.to_string()
}

fn scalar(raw: &str) -> Value {
    let text = unquote(raw);
    match raw.trim() {
        "{}" => Value::Object(Default::default()),
        "[]" | "~" | "null" => Value::Null,
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::String(text),
    }
}

/// Splits `key: value` (the value may be empty). Colons inside a value are fine: only the first
/// `: ` (or a trailing `:`) separates.
fn split_key(content: &str) -> Option<(String, &str)> {
    if content.starts_with('-') && content[1..].starts_with(' ') {
        return None;
    }
    let (key, value) = match content.find(": ") {
        Some(at) => (&content[..at], &content[at + 2..]),
        None => (content.strip_suffix(':')?, ""),
    };
    // A key never holds a space-colon inside a quote of its own in what these launchers write.
    Some((unquote(key), value))
}

pub fn parse_yaml(text: &str) -> Value {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| {
            let content = line.trim_start();
            (line.len() - content.len(), content.trim_end())
        })
        .collect();
    let mut at = 0;
    block(&lines, &mut at, 0)
}

fn block(lines: &[(usize, &str)], at: &mut usize, indent: usize) -> Value {
    let mut map = serde_json::Map::new();
    while *at < lines.len() {
        let (level, content) = lines[*at];
        if level < indent {
            break;
        }
        if level > indent {
            // Stray deeper line that no key claimed: skip it.
            *at += 1;
            continue;
        }
        let Some((key, value)) = split_key(content) else {
            *at += 1;
            continue;
        };
        *at += 1;
        let value = value.trim();
        if value.is_empty() {
            // Either a nested mapping, a list (skipped, also when its dashes are not indented),
            // or nothing.
            let mut nested = Value::Null;
            if *at < lines.len() {
                let (next_level, next) = lines[*at];
                if next_level > indent && !next.starts_with("- ") {
                    nested = block(lines, at, next_level);
                }
            }
            while *at < lines.len() {
                let (next_level, next) = lines[*at];
                let dash = next.starts_with("- ") || next == "-";
                if next_level > indent || (next_level == indent && dash) {
                    *at += 1;
                } else {
                    break;
                }
            }
            map.insert(key, nested);
        } else {
            // A value folded over the following, deeper lines.
            let mut text = value.to_string();
            let folded = !text.starts_with('|') && !text.starts_with('>');
            while *at < lines.len() && lines[*at].0 > indent {
                if folded {
                    text.push(' ');
                    text.push_str(lines[*at].1);
                }
                *at += 1;
            }
            map.insert(key, scalar(&text));
        }
    }
    Value::Object(map)
}

fn text<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |value, key| value.get(key))?
        .as_str()
        .filter(|text| !text.is_empty())
}

// --- Lutris --------------------------------------------------------------------------------

struct LutrisRoot {
    data: PathBuf,
    config: PathBuf,
}

fn lutris_roots(home: &Path) -> Vec<LutrisRoot> {
    let native = LutrisRoot {
        data: home.join(".local/share/lutris"),
        config: home.join(".config/lutris"),
    };
    let flatpak = home.join(".var/app/net.lutris.Lutris");
    let flatpak = LutrisRoot {
        data: flatpak.join("data/lutris"),
        config: flatpak.join("config/lutris"),
    };
    [native, flatpak]
        .into_iter()
        .filter(|root| root.data.join("pga.db").is_file())
        .collect()
}

fn lutris(data: &Path, config: &Path) -> Vec<Importable> {
    let Ok(db) = rusqlite::Connection::open_with_flags(
        data.join("pga.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) else {
        return Vec::new();
    };
    let Ok(mut query) = db.prepare(
        "SELECT id, name, runner, executable, directory, configpath FROM games
         WHERE installed = 1 AND runner IN ('wine', 'linux')",
    ) else {
        return Vec::new();
    };
    let rows = query.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    });
    let Ok(rows) = rows else {
        return Vec::new();
    };

    let mut games = Vec::new();
    for (id, name, runner, executable, directory, configpath) in rows.flatten() {
        // The game's own file wins over the library's summary of it. Newer versions keep it in
        // the data folder, older ones under the configuration.
        let settings = configpath
            .filter(|path| !path.is_empty())
            .and_then(|path| {
                [data, config].into_iter().find_map(|root| {
                    fs::read_to_string(root.join("games").join(format!("{path}.yml"))).ok()
                })
            })
            .map(|yaml| parse_yaml(&yaml))
            .unwrap_or(Value::Null);

        let exe = text(&settings, &["game", "exe"])
            .map(str::to_string)
            .or(executable.filter(|path| !path.is_empty()));
        let Some(mut exe) = exe else { continue };
        let folder = directory.filter(|path| !path.is_empty());
        if Path::new(&exe).is_relative() {
            match &folder {
                Some(folder) => exe = Path::new(folder).join(&exe).to_string_lossy().into_owned(),
                None => continue,
            }
        }

        let mut game = Importable::new("lutris", format!("lutris:{id}"), name, exe);
        game.native = runner == "linux";
        game.args = text(&settings, &["game", "args"]).unwrap_or("").to_string();
        game.working_dir = text(&settings, &["game", "working_dir"]).map(str::to_string);
        if !game.native {
            game.prefix_path = text(&settings, &["game", "prefix"]).map(str::to_string);
            game.runner_hint = text(&settings, &["wine", "version"]).map(str::to_string);
        }
        games.push(game);
    }
    games
}

// --- Heroic --------------------------------------------------------------------------------

fn heroic_roots(home: &Path) -> Vec<PathBuf> {
    [
        home.join(".config/heroic"),
        home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ]
    .into_iter()
    .filter(|root| root.is_dir())
    .collect()
}

fn read_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

/// What Heroic keeps for one game: its Wine prefix and runner, and its launch options.
fn heroic_settings(root: &Path, app_name: &str, game: &mut Importable) {
    let all = read_json(&root.join("GamesConfig").join(format!("{app_name}.json")));
    let settings = all.get(app_name).unwrap_or(&all);
    if game.native {
        return;
    }
    game.prefix_path = settings
        .get("winePrefix")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .map(str::to_string);
    game.runner_hint = settings
        .get("wineVersion")
        .and_then(|version| version.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);
    game.args = settings
        .get("launcherArgs")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
}

fn heroic(root: &Path) -> Vec<Importable> {
    let mut games = Vec::new();

    // Games the user added to Heroic by hand.
    let sideload = read_json(&root.join("sideload_apps/library.json"));
    for entry in sideload
        .get("games")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let (Some(app), Some(title), Some(exe)) = (
            text(entry, &["app_name"]),
            text(entry, &["title"]),
            text(entry, &["install", "executable"]),
        ) else {
            continue;
        };
        let mut game = Importable::new(
            "heroic",
            format!("heroic:{app}"),
            title.to_string(),
            exe.to_string(),
        );
        game.native = text(entry, &["install", "platform"])
            .is_some_and(|platform| platform.eq_ignore_ascii_case("linux"));
        heroic_settings(root, app, &mut game);
        games.push(game);
    }

    // Epic games, installed by Legendary (its titles are in the store cache).
    let installed = read_json(&root.join("legendaryConfig/legendary/installed.json"));
    let library = read_json(&root.join("store_cache/legendary_library.json"));
    for (app, entry) in installed.as_object().into_iter().flatten() {
        let (Some(folder), Some(executable)) =
            (text(entry, &["install_path"]), text(entry, &["executable"]))
        else {
            continue;
        };
        let title = library
            .get("library")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|game| text(game, &["app_name"]) == Some(app.as_str()))
            .and_then(|game| text(game, &["title"]))
            .or(text(entry, &["title"]))
            .unwrap_or(app);
        let exe = Path::new(folder).join(executable.replace('\\', "/"));
        let mut game = Importable::new(
            "heroic",
            format!("heroic:{app}"),
            title.to_string(),
            exe.to_string_lossy().into_owned(),
        );
        game.native = text(entry, &["platform"]).is_some_and(|p| p.eq_ignore_ascii_case("linux"));
        heroic_settings(root, app, &mut game);
        games.push(game);
    }
    games
}

// --- Bottles -------------------------------------------------------------------------------

fn bottles_roots(home: &Path) -> Vec<PathBuf> {
    [
        home.join(".local/share/bottles/bottles"),
        home.join(".var/app/com.usebottles.bottles/data/bottles/bottles"),
    ]
    .into_iter()
    .filter(|root| root.is_dir())
    .collect()
}

/// A bottle is a Wine prefix with a list of the programs the user added to it.
fn bottles(root: &Path) -> Vec<Importable> {
    let mut games = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return games;
    };
    for entry in entries.flatten() {
        let folder = entry.path();
        let Ok(yaml) = fs::read_to_string(folder.join("bottle.yml")) else {
            continue;
        };
        let bottle = parse_yaml(&yaml);
        let name = entry.file_name().to_string_lossy().into_owned();
        let runner = text(&bottle, &["Runner"]).map(str::to_string);
        let Some(programs) = bottle.get("External_Programs").and_then(Value::as_object) else {
            continue;
        };
        for (id, program) in programs {
            let (Some(title), Some(path)) = (text(program, &["name"]), text(program, &["path"]))
            else {
                continue;
            };
            let mut game = Importable::new(
                "bottles",
                format!("bottles:{name}:{id}"),
                title.to_string(),
                path.to_string(),
            );
            game.prefix_path = Some(folder.to_string_lossy().into_owned());
            game.runner_hint = runner.clone();
            game.args = text(program, &["arguments"]).unwrap_or("").to_string();
            game.working_dir = text(program, &["folder"]).map(str::to_string);
            games.push(game);
        }
    }
    games
}

/// Does a runner's name (`GE-Proton10-3`, a folder name, a label) match the one another
/// launcher used? Compared without case or punctuation, and a hint that is only a part of the
/// name (`GE-Proton10-3` in `GE-Proton10-3 (Steam)`) matches too.
pub fn runner_matches(hint: &str, candidate: &str) -> bool {
    fn squash(text: &str) -> String {
        text.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    }
    let (hint, candidate) = (squash(hint), squash(candidate));
    !hint.is_empty()
        && !candidate.is_empty()
        && (candidate.contains(&hint) || hint.contains(&candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_reads_nested_values_folded_lines_and_skips_lists() {
        let yaml = "game:\n  exe: /a/b c/d.exe\n  prefix: /p\nscript:\n  files:\n  - setup: N/A:Select\n  game:\n    exe: x\nwine:\n  version: lutris-GE-Proton8-26-x86_64\n  overrides: {}\nname: 'It''s'\nlong:\n  exe: /games/SP\n    Life 2027/start.exe\n  after: 1\n";
        let value = parse_yaml(yaml);
        assert_eq!(text(&value, &["game", "exe"]), Some("/a/b c/d.exe"));
        assert_eq!(text(&value, &["game", "prefix"]), Some("/p"));
        assert_eq!(text(&value, &["script", "game", "exe"]), Some("x"));
        assert_eq!(
            text(&value, &["wine", "version"]),
            Some("lutris-GE-Proton8-26-x86_64")
        );
        assert_eq!(text(&value, &["name"]), Some("It's"));
        assert_eq!(
            text(&value, &["long", "exe"]),
            Some("/games/SP Life 2027/start.exe")
        );
        assert_eq!(text(&value, &["long", "after"]), Some("1"));
    }

    #[test]
    fn lutris_games_come_from_its_database_and_files() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("lutris");
        fs::create_dir_all(data.join("games")).unwrap();
        let db = rusqlite::Connection::open(data.join("pga.db")).unwrap();
        db.execute_batch(
            "CREATE TABLE games (id INTEGER PRIMARY KEY, name TEXT, runner TEXT, executable TEXT, directory TEXT, installed INTEGER, configpath TEXT);
             INSERT INTO games VALUES (1, 'Fifa', 'wine', NULL, NULL, 1, 'fifa-1');
             INSERT INTO games VALUES (2, 'Native', 'linux', '/opt/n/run.sh', NULL, 1, NULL);
             INSERT INTO games VALUES (3, 'Emulated', 'dolphin', NULL, NULL, 1, NULL);
             INSERT INTO games VALUES (4, 'Gone', 'wine', NULL, NULL, 0, 'gone');",
        )
        .unwrap();
        drop(db);
        fs::write(
            data.join("games/fifa-1.yml"),
            "game:\n  exe: /g/FIFA.exe\n  prefix: /g/pfx\n  args: -windowed\nwine:\n  version: ge-proton9\n",
        )
        .unwrap();

        let found = lutris(&data, &dir.path().join("config"));
        assert_eq!(found.len(), 2);
        let fifa = found.iter().find(|g| g.title == "Fifa").unwrap();
        assert_eq!(fifa.exe_path, "/g/FIFA.exe");
        assert_eq!(fifa.prefix_path.as_deref(), Some("/g/pfx"));
        assert_eq!(fifa.runner_hint.as_deref(), Some("ge-proton9"));
        assert_eq!(fifa.args, "-windowed");
        assert!(fifa.missing && !fifa.native);
        let native = found.iter().find(|g| g.title == "Native").unwrap();
        assert!(native.native && native.prefix_path.is_none());
    }

    #[test]
    fn heroic_reads_sideloaded_and_epic_games() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("sideload_apps")).unwrap();
        fs::create_dir_all(root.join("GamesConfig")).unwrap();
        fs::create_dir_all(root.join("legendaryConfig/legendary")).unwrap();
        fs::create_dir_all(root.join("store_cache")).unwrap();
        fs::write(
            root.join("sideload_apps/library.json"),
            r#"{"games":[{"app_name":"abc","title":"My Game","install":{"executable":"/x/game.exe","platform":"Windows"}}]}"#,
        )
        .unwrap();
        fs::write(
            root.join("GamesConfig/abc.json"),
            r#"{"abc":{"winePrefix":"/pfx/abc","wineVersion":{"name":"Wine-GE-8","type":"wine"},"launcherArgs":"-x"}}"#,
        )
        .unwrap();
        fs::write(
            root.join("legendaryConfig/legendary/installed.json"),
            r#"{"Fortnite":{"install_path":"/epic/Fn","executable":"bin\\fn.exe","platform":"Windows"}}"#,
        )
        .unwrap();
        fs::write(
            root.join("store_cache/legendary_library.json"),
            r#"{"library":[{"app_name":"Fortnite","title":"Fortnite!"}]}"#,
        )
        .unwrap();

        let found = heroic(root);
        assert_eq!(found.len(), 2);
        let mine = found.iter().find(|g| g.title == "My Game").unwrap();
        assert_eq!(mine.prefix_path.as_deref(), Some("/pfx/abc"));
        assert_eq!(mine.runner_hint.as_deref(), Some("Wine-GE-8"));
        assert_eq!(mine.args, "-x");
        let epic = found.iter().find(|g| g.title == "Fortnite!").unwrap();
        assert_eq!(epic.exe_path, "/epic/Fn/bin/fn.exe");
    }

    #[test]
    fn bottles_lists_the_programs_added_to_each_bottle() {
        let dir = tempfile::tempdir().unwrap();
        let bottle = dir.path().join("Gaming");
        fs::create_dir_all(&bottle).unwrap();
        fs::write(
            bottle.join("bottle.yml"),
            "Name: Gaming\nRunner: soda-9.0-1\nExternal_Programs:\n  p1:\n    name: Hades\n    path: /games/hades.exe\n    arguments: ''\n  p2:\n    name: Other\n    path: /games/o.exe\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("Empty")).unwrap();
        fs::write(
            dir.path().join("Empty/bottle.yml"),
            "Name: Empty\nExternal_Programs: {}\n",
        )
        .unwrap();

        let found = bottles(dir.path());
        assert_eq!(found.len(), 2);
        let hades = found.iter().find(|g| g.title == "Hades").unwrap();
        assert_eq!(hades.runner_hint.as_deref(), Some("soda-9.0-1"));
        assert!(hades.prefix_path.as_deref().unwrap().ends_with("Gaming"));
    }

    #[test]
    fn runner_names_match_loosely() {
        assert!(runner_matches(
            "lutris-GE-Proton8-26-x86_64",
            "GE-Proton8-26"
        ));
        assert!(runner_matches("GE-Proton10-3", "GE-Proton10-3 (Lutra)"));
        assert!(!runner_matches("GE-Proton9", "GE-Proton10"));
        assert!(!runner_matches("", "anything"));
    }

    /// Reads this machine's own launchers: `cargo test -- --ignored --nocapture real_launchers`.
    #[test]
    #[ignore]
    fn real_launchers() {
        let home = PathBuf::from(std::env::var("HOME").unwrap());
        for game in discover(&home) {
            println!("{game:#?}");
        }
    }
}
