//! Local SQLite library: games found by the sources plus what the user did with them.

use std::collections::HashSet;
use std::path::Path;

use chrono::{DateTime, SecondsFormat};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use crate::wine::WineConfig;

const GAME_COLUMNS: &str = "id, title, source, source_id, playtime_minutes, last_played, favorite, exe_path, runner_kind, poster_path, hero_path, poster_url, hero_url, images_checked_at, sgdb_game_id, meta_json, logo_path";

/// A game as the frontend sees it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub title: String,
    pub source: String,
    /// Placeholder artwork hue, derived from the id so it never changes.
    pub hue: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub igdb_id: Option<u64>,
    pub genres: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub playtime_minutes: u32,
    /// ISO 8601 date, absent if never launched.
    pub last_played: Option<String>,
    pub favorite: bool,
    /// Absolute path of the downloaded cover, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    /// Absolute path of the downloaded banner, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hero: Option<String>,
    /// Where the current cover and banner were downloaded from, to show which one is in use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hero_url: Option<String>,
    /// Absolute path of the downloaded logo (transparent title art), if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    /// Changes whenever artwork is downloaded again; lets the webview drop its cached copy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art_version: Option<i64>,
    /// SteamGridDB game picked by the user for this game's artwork, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sgdb_game_id: Option<i64>,
    /// Windows program of a game added by hand.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exe_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner_kind: Option<String>,
}

/// What is known about a game beyond its files: fetched from GameDB, then editable by the user.
/// The user's values are what is stored, so nothing fetched later overwrites them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GameMeta {
    pub developer: Option<String>,
    /// ISO date, `YYYY-MM-DD` (or just `YYYY`).
    pub release_date: Option<String>,
    pub genres: Vec<String>,
    pub description: Option<String>,
    /// IGDB id, kept so the entry can be refreshed from GameDB.
    pub igdb_id: Option<u64>,
}

/// What a source reports about a game it found.
#[derive(Debug, Clone)]
pub struct Discovered {
    pub source: &'static str,
    pub source_id: String,
    pub title: String,
    pub playtime_minutes: u32,
    pub last_played: Option<i64>,
}

impl Discovered {
    pub fn id(&self) -> String {
        format!("{}:{}", self.source, self.source_id)
    }
}

/// A Windows or Linux game added by hand.
#[derive(Debug, Clone)]
pub struct ManualGame {
    pub id: String,
    /// `wine` or `native`.
    pub source: &'static str,
    pub title: String,
    pub exe_path: String,
    pub runner_kind: String,
    pub runner_path: String,
    pub prefix_path: String,
    pub config: WineConfig,
    pub meta: Option<GameMeta>,
}

/// What is needed to start a game.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchInfo {
    pub source: String,
    pub source_id: String,
    pub exe_path: Option<String>,
    pub runner_kind: Option<String>,
    pub runner_path: Option<String>,
    pub prefix_path: Option<String>,
}

/// Everything the user can configure about a Windows game.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WineSettings {
    pub id: String,
    pub title: String,
    pub exe_path: String,
    pub runner_kind: String,
    pub runner_path: String,
    pub prefix_path: String,
    pub config: WineConfig,
    /// Filled when reading: whether the prefix was already created.
    #[serde(default)]
    pub prefix_exists: bool,
}

/// A downloaded image and where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub path: String,
    pub url: String,
}

/// A game that still needs artwork.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageTarget {
    pub id: String,
    pub source: String,
    pub source_id: String,
    pub title: String,
    /// SteamGridDB game picked by the user, used instead of searching by title.
    pub sgdb_id: Option<u64>,
    /// Its cover or banner is still to be looked up.
    pub needs_images: bool,
    /// Its logo is still to be looked up.
    pub needs_logo: bool,
}

pub struct Db {
    conn: Connection,
}

/// FNV-1a: tiny, stable across runs and platforms (unlike `DefaultHasher`).
pub fn stable_hash(text: &str) -> u32 {
    text.bytes().fold(2166136261u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16777619)
    })
}

fn hue_for(id: &str) -> u16 {
    (stable_hash(id) % 360) as u16
}

fn iso(timestamp: i64) -> Option<String> {
    DateTime::from_timestamp(timestamp, 0)
        .map(|date| date.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn game_from_row(row: &Row) -> rusqlite::Result<Game> {
    let id: String = row.get("id")?;
    let meta: GameMeta = row
        .get::<_, Option<String>>("meta_json")?
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();
    Ok(Game {
        hue: hue_for(&id),
        title: row.get("title")?,
        source: row.get("source")?,
        developer: meta.developer,
        year: meta
            .release_date
            .as_deref()
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse().ok()),
        release_date: meta.release_date,
        igdb_id: meta.igdb_id,
        genres: meta.genres,
        description: meta.description,
        playtime_minutes: row.get("playtime_minutes")?,
        last_played: row.get::<_, Option<i64>>("last_played")?.and_then(iso),
        favorite: row.get("favorite")?,
        poster: row.get("poster_path")?,
        hero: row.get("hero_path")?,
        poster_url: row.get("poster_url")?,
        hero_url: row.get("hero_url")?,
        logo: row.get("logo_path")?,
        art_version: row.get("images_checked_at")?,
        sgdb_game_id: row.get("sgdb_game_id")?,
        exe_path: row.get("exe_path")?,
        runner_kind: row.get("runner_kind")?,
        id,
    })
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS games (
                id               TEXT PRIMARY KEY,
                source           TEXT NOT NULL,
                source_id        TEXT NOT NULL,
                title            TEXT NOT NULL,
                playtime_minutes INTEGER NOT NULL DEFAULT 0,
                last_played      INTEGER,
                favorite         INTEGER NOT NULL DEFAULT 0
            );",
        )?;

        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 1 {
            for column in [
                "exe_path TEXT",
                "runner_kind TEXT",
                "runner_path TEXT",
                "prefix_path TEXT",
                "poster_path TEXT",
                "hero_path TEXT",
                "images_checked_at INTEGER",
            ] {
                conn.execute(&format!("ALTER TABLE games ADD COLUMN {column}"), [])?;
            }
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )?;
        }
        if version < 2 {
            for column in ["config_json TEXT", "sgdb_game_id INTEGER"] {
                conn.execute(&format!("ALTER TABLE games ADD COLUMN {column}"), [])?;
            }
        }
        if version < 3 {
            for column in ["poster_url TEXT", "hero_url TEXT"] {
                conn.execute(&format!("ALTER TABLE games ADD COLUMN {column}"), [])?;
            }
        }
        if version < 4 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS controller_profiles (owner TEXT PRIMARY KEY, json TEXT NOT NULL);",
            )?;
        }
        if version < 5 {
            conn.execute("ALTER TABLE games ADD COLUMN meta_json TEXT", [])?;
        }
        if version < 6 {
            for column in ["logo_path TEXT", "logo_url TEXT", "logo_checked_at INTEGER"] {
                conn.execute(&format!("ALTER TABLE games ADD COLUMN {column}"), [])?;
            }
        }
        if version < 7 {
            // Emulation was dropped: forget the emulated games and their settings (saves and
            // downloaded cores stay on disk).
            conn.execute_batch(
                "DELETE FROM controller_profiles WHERE owner IN (SELECT id FROM games WHERE source = 'emulator');
                 DELETE FROM games WHERE source = 'emulator';
                 DELETE FROM settings WHERE key IN ('rom_dirs', 'bios_dir', 'ui_language')
                    OR key LIKE 'emulation_profile:%';",
            )?;
        }
        conn.execute_batch("PRAGMA user_version = 7;")?;
        Ok(Self { conn })
    }

    pub fn list(&self) -> rusqlite::Result<Vec<Game>> {
        let mut statement = self.conn.prepare(&format!(
            "SELECT {GAME_COLUMNS} FROM games ORDER BY title COLLATE NOCASE"
        ))?;
        let games = statement.query_map([], game_from_row)?;
        games.collect()
    }

    pub fn get(&self, id: &str) -> rusqlite::Result<Option<Game>> {
        self.conn
            .query_row(
                &format!("SELECT {GAME_COLUMNS} FROM games WHERE id = ?1"),
                [id],
                game_from_row,
            )
            .optional()
    }

    pub fn launch_info(&self, id: &str) -> rusqlite::Result<Option<LaunchInfo>> {
        self.conn
            .query_row(
                "SELECT source, source_id, exe_path, runner_kind, runner_path, prefix_path FROM games WHERE id = ?1",
                [id],
                |row| {
                    Ok(LaunchInfo {
                        source: row.get(0)?,
                        source_id: row.get(1)?,
                        exe_path: row.get(2)?,
                        runner_kind: row.get(3)?,
                        runner_path: row.get(4)?,
                        prefix_path: row.get(5)?,
                    })
                },
            )
            .optional()
    }

    /// Makes the library match what `source` currently reports: adds, updates and removes games,
    /// while keeping what only the user knows (favorites, launches made from Lutra).
    pub fn sync_source(&mut self, source: &str, found: &[Discovered]) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        let mut keep = HashSet::new();

        for game in found {
            let id = game.id();
            tx.execute(
                "INSERT INTO games (id, source, source_id, title, playtime_minutes, last_played)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    playtime_minutes = excluded.playtime_minutes,
                    last_played = NULLIF(MAX(COALESCE(games.last_played, 0), COALESCE(excluded.last_played, 0)), 0)",
                params![id, game.source, game.source_id, game.title, game.playtime_minutes, game.last_played],
            )?;
            keep.insert(id);
        }

        let existing: Vec<String> = {
            let mut statement = tx.prepare("SELECT id FROM games WHERE source = ?1")?;
            let ids = statement.query_map([source], |row| row.get(0))?;
            ids.collect::<rusqlite::Result<_>>()?
        };
        for id in existing.iter().filter(|id| !keep.contains(*id)) {
            tx.execute("DELETE FROM games WHERE id = ?1", [id])?;
        }

        tx.commit()
    }

    /// Adds a game by hand, or updates it if the same program was already added.
    pub fn add_manual(&self, game: &ManualGame) -> rusqlite::Result<()> {
        let config = serde_json::to_string(&game.config).unwrap_or_default();
        let meta = game
            .meta
            .as_ref()
            .and_then(|meta| serde_json::to_string(meta).ok());
        self.conn.execute(
            "INSERT INTO games (id, source, source_id, title, exe_path, runner_kind, runner_path, prefix_path, config_json, meta_json)
             VALUES (?1, ?2, ?1, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                exe_path = excluded.exe_path,
                runner_kind = excluded.runner_kind,
                runner_path = excluded.runner_path,
                meta_json = COALESCE(excluded.meta_json, games.meta_json)",
            params![
                game.id,
                game.source,
                game.title,
                game.exe_path,
                game.runner_kind,
                game.runner_path,
                game.prefix_path,
                config,
                meta
            ],
        )?;
        Ok(())
    }

    pub fn game_meta(&self, id: &str) -> rusqlite::Result<GameMeta> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT meta_json FROM games WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .optional()?
            .flatten();
        Ok(json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default())
    }

    pub fn set_game_meta(&self, id: &str, meta: &GameMeta) -> rusqlite::Result<()> {
        let json = serde_json::to_string(meta).unwrap_or_default();
        self.conn.execute(
            "UPDATE games SET meta_json = ?2 WHERE id = ?1",
            params![id, json],
        )?;
        Ok(())
    }

    /// The games whose Wine prefix is `prefix`.
    pub fn prefix_users(&self, prefix: &str) -> rusqlite::Result<Vec<String>> {
        let mut statement = self
            .conn
            .prepare("SELECT id FROM games WHERE prefix_path = ?1")?;
        let ids = statement.query_map([prefix], |row| row.get(0))?;
        ids.collect()
    }

    pub fn wine_settings(&self, id: &str) -> rusqlite::Result<Option<WineSettings>> {
        self.conn
            .query_row(
                "SELECT id, title, exe_path, runner_kind, runner_path, prefix_path, config_json
                 FROM games WHERE id = ?1 AND source IN ('wine', 'native')",
                [id],
                |row| {
                    let config: Option<String> = row.get(6)?;
                    Ok(WineSettings {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        exe_path: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        runner_kind: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                        runner_path: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                        prefix_path: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                        config: config
                            .and_then(|json| serde_json::from_str(&json).ok())
                            .unwrap_or_default(),
                        prefix_exists: false,
                    })
                },
            )
            .optional()
    }

    pub fn save_wine_settings(&self, settings: &WineSettings) -> rusqlite::Result<()> {
        let config = serde_json::to_string(&settings.config).unwrap_or_default();
        self.conn.execute(
            "UPDATE games SET title = ?2, exe_path = ?3, runner_kind = ?4, runner_path = ?5,
                prefix_path = ?6, config_json = ?7
             WHERE id = ?1 AND source IN ('wine', 'native')",
            params![
                settings.id,
                settings.title,
                settings.exe_path,
                settings.runner_kind,
                settings.runner_path,
                settings.prefix_path,
                config
            ],
        )?;
        Ok(())
    }

    /// Pins (or, with `None`, unpins) the SteamGridDB game used for a game's artwork.
    pub fn set_sgdb_game(&self, id: &str, sgdb_game: Option<u64>) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE games SET sgdb_game_id = ?2 WHERE id = ?1",
            params![id, sgdb_game.map(|value| value as i64)],
        )?;
        Ok(())
    }

    /// Forgets one game's artwork so it gets fetched again.
    pub fn clear_game_images(&self, id: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE games SET poster_path = NULL, hero_path = NULL, poster_url = NULL, hero_url = NULL, images_checked_at = NULL, logo_path = NULL, logo_url = NULL, logo_checked_at = NULL WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn remove(&self, id: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM games WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn set_favorite(&self, id: &str, favorite: bool) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE games SET favorite = ?2 WHERE id = ?1",
            params![id, favorite],
        )?;
        Ok(())
    }

    pub fn mark_played(&self, id: &str, timestamp: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE games SET last_played = ?2 WHERE id = ?1",
            params![id, timestamp],
        )?;
        Ok(())
    }

    pub fn add_playtime(&self, id: &str, minutes: u32) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE games SET playtime_minutes = playtime_minutes + ?2 WHERE id = ?1",
            params![id, minutes],
        )?;
        Ok(())
    }

    /// Games whose artwork (cover, banner or logo) is incomplete and has not been looked up yet,
    /// or all of them if `force`.
    pub fn needing_images(&self, force: bool) -> rusqlite::Result<Vec<ImageTarget>> {
        const IMAGES: &str =
            "(poster_path IS NULL OR hero_path IS NULL) AND images_checked_at IS NULL";
        const LOGO: &str = "logo_path IS NULL AND logo_checked_at IS NULL";
        let filter = if force {
            String::new()
        } else {
            format!("WHERE ({IMAGES}) OR ({LOGO})")
        };
        let mut statement = self.conn.prepare(&format!(
            "SELECT id, source, source_id, title, sgdb_game_id, {IMAGES}, {LOGO} FROM games {filter} ORDER BY title COLLATE NOCASE"
        ))?;
        let targets = statement.query_map([], |row| {
            Ok(ImageTarget {
                id: row.get(0)?,
                source: row.get(1)?,
                source_id: row.get(2)?,
                title: row.get(3)?,
                sgdb_id: row
                    .get::<_, Option<i64>>(4)?
                    .and_then(|id| u64::try_from(id).ok()),
                needs_images: force || row.get(5)?,
                needs_logo: force || row.get(6)?,
            })
        })?;
        targets.collect()
    }

    /// Records a looked-up logo; `None` means there is none, and it is not asked for again.
    pub fn set_logo(
        &self,
        id: &str,
        logo: Option<&Image>,
        checked_at: i64,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE games SET logo_path = COALESCE(?2, logo_path), logo_url = COALESCE(?3, logo_url), logo_checked_at = ?4 WHERE id = ?1",
            params![id, logo.map(|logo| &logo.path), logo.map(|logo| &logo.url), checked_at],
        )?;
        Ok(())
    }

    /// Records downloaded artwork; a `None` keeps whatever was there.
    pub fn set_images(
        &self,
        id: &str,
        poster: Option<&Image>,
        hero: Option<&Image>,
        checked_at: i64,
    ) -> rusqlite::Result<()> {
        if let Some(poster) = poster {
            self.conn.execute(
                "UPDATE games SET poster_path = ?2, poster_url = ?3 WHERE id = ?1",
                params![id, poster.path, poster.url],
            )?;
        }
        if let Some(hero) = hero {
            self.conn.execute(
                "UPDATE games SET hero_path = ?2, hero_url = ?3 WHERE id = ?1",
                params![id, hero.path, hero.url],
            )?;
        }
        self.conn.execute(
            "UPDATE games SET images_checked_at = ?2 WHERE id = ?1",
            params![id, checked_at],
        )?;
        Ok(())
    }

    /// Forgets the artwork of every game of a source, so it gets fetched again.
    /// Returns the ids affected, to let the caller delete the files.
    pub fn clear_images(&self, source: &str) -> rusqlite::Result<Vec<String>> {
        let ids: Vec<String> = {
            let mut statement = self
                .conn
                .prepare("SELECT id FROM games WHERE source = ?1")?;
            let rows = statement.query_map([source], |row| row.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        self.conn.execute(
            "UPDATE games SET poster_path = NULL, hero_path = NULL, poster_url = NULL, hero_url = NULL, images_checked_at = NULL, logo_path = NULL, logo_url = NULL, logo_checked_at = NULL WHERE source = ?1",
            [source],
        )?;
        Ok(ids)
    }

    /// Controller profile of a game, or `default` for every game without its own.
    pub fn controller_profile(&self, owner: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT json FROM controller_profiles WHERE owner = ?1",
                [owner],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn set_controller_profile(&self, owner: &str, json: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO controller_profiles (owner, json) VALUES (?1, ?2)
             ON CONFLICT(owner) DO UPDATE SET json = excluded.json",
            params![owner, json],
        )?;
        Ok(())
    }

    pub fn delete_controller_profile(&self, owner: &str) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM controller_profiles WHERE owner = ?1", [owner])?;
        Ok(())
    }

    /// How many games are set to run with this runner (a `proton` script or a `wine` binary).
    pub fn games_using_runner(&self, path: &str) -> rusqlite::Result<usize> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE runner_path = ?1",
                [path],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as usize)
    }

    pub fn setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steam(app_id: &str, title: &str, playtime: u32, last_played: Option<i64>) -> Discovered {
        Discovered {
            source: "steam",
            source_id: app_id.to_string(),
            title: title.to_string(),
            playtime_minutes: playtime,
            last_played,
        }
    }

    fn img(path: &str) -> Image {
        Image {
            path: path.to_string(),
            url: format!("https://cdn.example{path}"),
        }
    }

    fn manual(id: &str, title: &str) -> ManualGame {
        ManualGame {
            id: id.to_string(),
            source: "wine",
            title: title.to_string(),
            exe_path: "/games/foo/foo.exe".into(),
            runner_kind: "wine".into(),
            runner_path: "/usr/bin/wine".into(),
            prefix_path: "/prefixes/foo".into(),
            config: WineConfig::default(),
            meta: None,
        }
    }

    #[test]
    fn syncing_adds_games_with_a_stable_hue() {
        let mut db = Db::open_in_memory().unwrap();
        db.sync_source(
            "steam",
            &[steam("10", "Hollow Ember", 90, Some(1700000000))],
        )
        .unwrap();

        let games = db.list().unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "steam:10");
        assert_eq!(games[0].source, "steam");
        assert_eq!(games[0].playtime_minutes, 90);
        assert_eq!(
            games[0].last_played.as_deref(),
            Some("2023-11-14T22:13:20Z")
        );
        assert!(games[0].hue < 360);
        assert_eq!(games[0].hue, hue_for("steam:10"));
    }

    #[test]
    fn resyncing_keeps_favorites_and_newer_launches() {
        let mut db = Db::open_in_memory().unwrap();
        db.sync_source(
            "steam",
            &[steam("10", "Hollow Ember", 90, Some(1700000000))],
        )
        .unwrap();
        db.set_favorite("steam:10", true).unwrap();
        db.mark_played("steam:10", 1800000000).unwrap();

        db.sync_source(
            "steam",
            &[steam("10", "Hollow Ember", 120, Some(1700000000))],
        )
        .unwrap();

        let game = &db.list().unwrap()[0];
        assert!(game.favorite);
        assert_eq!(game.playtime_minutes, 120);
        assert_eq!(game.last_played.as_deref(), Some("2027-01-15T08:00:00Z"));
    }

    #[test]
    fn uninstalled_games_are_removed_without_touching_other_sources() {
        let mut db = Db::open_in_memory().unwrap();
        db.sync_source(
            "steam",
            &[steam("10", "A", 0, None), steam("20", "B", 0, None)],
        )
        .unwrap();
        db.add_manual(&manual("wine:1", "C")).unwrap();

        db.sync_source("steam", &[steam("20", "B", 0, None)])
            .unwrap();

        let ids: Vec<String> = db.list().unwrap().into_iter().map(|game| game.id).collect();
        assert_eq!(ids, vec!["steam:20", "wine:1"]);
    }

    #[test]
    fn never_launched_games_have_no_date() {
        let mut db = Db::open_in_memory().unwrap();
        db.sync_source("steam", &[steam("10", "A", 0, None)])
            .unwrap();
        assert_eq!(db.list().unwrap()[0].last_played, None);
    }

    #[test]
    fn manual_games_keep_their_launch_details() {
        let db = Db::open_in_memory().unwrap();
        db.add_manual(&manual("wine:1", "Foo")).unwrap();

        let info = db.launch_info("wine:1").unwrap().unwrap();
        assert_eq!(info.source, "wine");
        assert_eq!(info.exe_path.as_deref(), Some("/games/foo/foo.exe"));
        assert_eq!(info.runner_path.as_deref(), Some("/usr/bin/wine"));
        assert_eq!(info.prefix_path.as_deref(), Some("/prefixes/foo"));

        let game = db.get("wine:1").unwrap().unwrap();
        assert_eq!(game.runner_kind.as_deref(), Some("wine"));
        assert_eq!(db.launch_info("wine:2").unwrap(), None);
    }

    #[test]
    fn adding_the_same_program_again_updates_it_and_keeps_the_favorite() {
        let db = Db::open_in_memory().unwrap();
        db.add_manual(&manual("wine:1", "Foo")).unwrap();
        db.set_favorite("wine:1", true).unwrap();

        db.add_manual(&manual("wine:1", "Foo Renamed")).unwrap();

        let games = db.list().unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].title, "Foo Renamed");
        assert!(games[0].favorite);
    }

    #[test]
    fn removing_a_game_deletes_it() {
        let db = Db::open_in_memory().unwrap();
        db.add_manual(&manual("wine:1", "Foo")).unwrap();
        db.remove("wine:1").unwrap();
        assert!(db.list().unwrap().is_empty());
    }

    #[test]
    fn playtime_accumulates() {
        let db = Db::open_in_memory().unwrap();
        db.add_manual(&manual("wine:1", "Foo")).unwrap();
        db.add_playtime("wine:1", 30).unwrap();
        db.add_playtime("wine:1", 15).unwrap();
        assert_eq!(db.get("wine:1").unwrap().unwrap().playtime_minutes, 45);
    }

    #[test]
    fn games_are_only_looked_up_for_artwork_once() {
        let mut db = Db::open_in_memory().unwrap();
        db.sync_source(
            "steam",
            &[steam("10", "A", 0, None), steam("20", "B", 0, None)],
        )
        .unwrap();
        assert_eq!(db.needing_images(false).unwrap().len(), 2);

        db.set_images(
            "steam:10",
            Some(&img("/img/a.png")),
            Some(&img("/img/a-hero.png")),
            1,
        )
        .unwrap();
        // B was checked but nothing was found: it must not be retried on every launch.
        db.set_images("steam:20", None, None, 1).unwrap();

        // The logo is looked up on its own.
        let targets = db.needing_images(false).unwrap();
        assert_eq!(targets.len(), 2);
        assert!(
            targets
                .iter()
                .all(|target| target.needs_logo && !target.needs_images)
        );
        db.set_logo("steam:10", Some(&img("/img/a-logo.png")), 1)
            .unwrap();
        db.set_logo("steam:20", None, 1).unwrap();

        assert!(db.needing_images(false).unwrap().is_empty());
        assert_eq!(db.needing_images(true).unwrap().len(), 2);

        let game = db.get("steam:10").unwrap().unwrap();
        assert_eq!(game.poster.as_deref(), Some("/img/a.png"));
        assert_eq!(game.hero.as_deref(), Some("/img/a-hero.png"));
        assert_eq!(game.logo.as_deref(), Some("/img/a-logo.png"));
        assert_eq!(
            game.poster_url.as_deref(),
            Some("https://cdn.example/img/a.png")
        );
    }

    #[test]
    fn new_artwork_never_erases_existing_artwork() {
        let mut db = Db::open_in_memory().unwrap();
        db.sync_source("steam", &[steam("10", "A", 0, None)])
            .unwrap();
        db.set_images("steam:10", Some(&img("/img/a.png")), None, 1)
            .unwrap();
        db.set_images("steam:10", None, Some(&img("/img/hero.png")), 2)
            .unwrap();

        let game = db.get("steam:10").unwrap().unwrap();
        assert_eq!(game.poster.as_deref(), Some("/img/a.png"));
        assert_eq!(game.hero.as_deref(), Some("/img/hero.png"));
    }

    #[test]
    fn wine_settings_round_trip_with_defaults_for_missing_config() {
        let db = Db::open_in_memory().unwrap();
        db.add_manual(&manual("wine:1", "Foo")).unwrap();

        let mut settings = db.wine_settings("wine:1").unwrap().unwrap();
        assert!(
            settings.config.esync,
            "a game never configured gets the defaults"
        );
        assert_eq!(settings.runner_kind, "wine");

        settings.title = "Foo 2".into();
        settings.config.args = "-windowed".into();
        settings.config.gamemode = true;
        db.save_wine_settings(&settings).unwrap();

        let saved = db.wine_settings("wine:1").unwrap().unwrap();
        assert_eq!(saved.title, "Foo 2");
        assert_eq!(saved.config.args, "-windowed");
        assert!(saved.config.gamemode);
        assert_eq!(db.wine_settings("steam:1").unwrap(), None);
    }

    #[test]
    fn a_picked_steamgriddb_game_is_kept_through_artwork_resets() {
        let mut db = Db::open_in_memory().unwrap();
        db.sync_source("steam", &[steam("10", "A", 0, None)])
            .unwrap();
        db.set_sgdb_game("steam:10", Some(5247)).unwrap();
        db.set_images("steam:10", Some(&img("/a.png")), None, 1)
            .unwrap();

        db.clear_game_images("steam:10").unwrap();

        let game = db.get("steam:10").unwrap().unwrap();
        assert_eq!(game.poster, None);
        assert_eq!(game.sgdb_game_id, Some(5247));
        assert_eq!(db.needing_images(false).unwrap()[0].sgdb_id, Some(5247));

        db.set_sgdb_game("steam:10", None).unwrap();
        assert_eq!(db.get("steam:10").unwrap().unwrap().sgdb_game_id, None);
    }

    #[test]
    fn clearing_images_only_affects_one_source() {
        let mut db = Db::open_in_memory().unwrap();
        db.sync_source("steam", &[steam("10", "A", 0, None)])
            .unwrap();
        db.add_manual(&manual("wine:1", "B")).unwrap();
        db.set_images(
            "steam:10",
            Some(&img("/a.png")),
            Some(&img("/a-hero.png")),
            1,
        )
        .unwrap();
        db.set_images("wine:1", Some(&img("/b.png")), None, 1)
            .unwrap();
        db.set_logo("steam:10", Some(&img("/a-logo.png")), 1)
            .unwrap();
        db.set_logo("wine:1", None, 1).unwrap();

        let cleared = db.clear_images("steam").unwrap();

        assert_eq!(cleared, vec!["steam:10"]);
        assert_eq!(db.get("steam:10").unwrap().unwrap().poster, None);
        assert_eq!(db.get("steam:10").unwrap().unwrap().logo, None);
        assert_eq!(db.needing_images(false).unwrap().len(), 1);
        assert_eq!(
            db.get("wine:1").unwrap().unwrap().poster.as_deref(),
            Some("/b.png")
        );
    }

    #[test]
    fn controller_profiles_are_stored_per_owner() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.controller_profile("default").unwrap(), None);
        db.set_controller_profile("default", "{}").unwrap();
        db.set_controller_profile("wine:1", "{\"output\":\"ds4\"}")
            .unwrap();
        db.set_controller_profile("wine:1", "{\"output\":\"none\"}")
            .unwrap();
        assert_eq!(
            db.controller_profile("wine:1").unwrap().as_deref(),
            Some("{\"output\":\"none\"}")
        );
        db.delete_controller_profile("wine:1").unwrap();
        assert_eq!(db.controller_profile("wine:1").unwrap(), None);
        assert!(db.controller_profile("default").unwrap().is_some());
    }

    #[test]
    fn settings_can_be_saved_and_deleted() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.setting("k").unwrap(), None);
        db.set_setting("k", "one").unwrap();
        db.set_setting("k", "two").unwrap();
        assert_eq!(db.setting("k").unwrap().as_deref(), Some("two"));
        db.delete_setting("k").unwrap();
        assert_eq!(db.setting("k").unwrap(), None);
    }

    #[test]
    fn upgrades_a_database_created_before_images_and_wine_existed() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE games (
                id TEXT PRIMARY KEY, source TEXT NOT NULL, source_id TEXT NOT NULL, title TEXT NOT NULL,
                playtime_minutes INTEGER NOT NULL DEFAULT 0, last_played INTEGER, favorite INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO games (id, source, source_id, title, favorite) VALUES ('steam:1', 'steam', '1', 'Old', 1);",
        )
        .unwrap();

        let db = Db::init(conn).unwrap();

        let game = db.get("steam:1").unwrap().unwrap();
        assert_eq!(game.title, "Old");
        assert!(game.favorite);
        assert_eq!(game.poster, None);
        db.set_setting("k", "v").unwrap();
    }

    #[test]
    fn native_games_have_settings_like_windows_ones() {
        let db = Db::open_in_memory().unwrap();
        db.add_manual(&ManualGame {
            id: "native:1".into(),
            source: "native",
            runner_kind: "native".into(),
            runner_path: String::new(),
            prefix_path: String::new(),
            exe_path: "/games/run.sh".into(),
            ..manual("native:1", "Linux Game")
        })
        .unwrap();
        let settings = db.wine_settings("native:1").unwrap().unwrap();
        assert_eq!(settings.runner_kind, "native");
        assert_eq!(db.get("native:1").unwrap().unwrap().source, "native");
    }

    #[test]
    fn the_config_given_at_creation_is_what_gets_stored() {
        let db = Db::open_in_memory().unwrap();
        let mut game = manual("wine:1", "Foo");
        game.config = WineConfig {
            gamemode: true,
            fps_limit: Some(90),
            ..WineConfig::default()
        };
        db.add_manual(&game).unwrap();
        let settings = db.wine_settings("wine:1").unwrap().unwrap();
        assert!(settings.config.gamemode);
        assert_eq!(settings.config.fps_limit, Some(90));
    }

    #[test]
    fn game_details_are_stored_and_survive_a_re_add() {
        let db = Db::open_in_memory().unwrap();
        let mut game = manual("wine:1", "Foo");
        game.meta = Some(GameMeta {
            developer: Some("EA Canada".into()),
            release_date: Some("2018-09-28".into()),
            genres: vec!["Sport".into()],
            description: Some("Football".into()),
            igdb_id: Some(96209),
        });
        db.add_manual(&game).unwrap();

        let stored = db.get("wine:1").unwrap().unwrap();
        assert_eq!(stored.developer.as_deref(), Some("EA Canada"));
        assert_eq!(stored.year, Some(2018));
        assert_eq!(stored.release_date.as_deref(), Some("2018-09-28"));
        assert_eq!(stored.genres, vec!["Sport"]);
        assert_eq!(stored.igdb_id, Some(96209));

        // Adding the same program again without details keeps what the user had.
        db.add_manual(&manual("wine:1", "Foo")).unwrap();
        assert_eq!(
            db.get("wine:1").unwrap().unwrap().developer.as_deref(),
            Some("EA Canada")
        );

        let mut meta = db.game_meta("wine:1").unwrap();
        meta.developer = Some("Someone else".into());
        db.set_game_meta("wine:1", &meta).unwrap();
        assert_eq!(
            db.get("wine:1").unwrap().unwrap().developer.as_deref(),
            Some("Someone else")
        );
    }
}
