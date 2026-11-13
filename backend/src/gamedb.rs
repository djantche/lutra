//! Game details from [LizardByte GameDB](https://github.com/LizardByte/GameDB): IGDB's data as
//! static JSON. Titles are indexed in small "bucket" files by their first characters; each game
//! has its own file. Buckets are cached on disk, so a search costs one download at most.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use serde_json::Value;

use crate::db::GameMeta;

const BASE: &str = "https://app.lizardbyte.dev/GameDB";
/// IGDB platform ids.
const PLATFORM_PC: i64 = 6;
const PLATFORM_LINUX: i64 = 3;
const BUCKET_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);
/// Candidates whose details are fetched, to tell apart editions and remakes of the same name.
const DETAILED_HITS: usize = 6;

/// A game found in GameDB, with what is needed to choose between look-alikes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameHit {
    pub id: u64,
    pub name: String,
    pub year: Option<u16>,
    pub developer: Option<String>,
    /// Cover image (remote URL) for the preview.
    pub cover: Option<String>,
    /// Released on PC (Windows or Linux): the likeliest match for a game added here.
    pub pc: bool,
    pub meta: GameMeta,
}

fn alnum(c: char) -> bool {
    c.is_alphanumeric()
}

/// Lowercase letters and digits only: `FIFA 19: Legacy` and `fifa19legacy` compare equal.
fn compact(text: &str) -> String {
    text.chars()
        .filter(|c| alnum(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The bucket file (without `.json`) that lists a title, following GameDB's rule: the first two
/// alphanumeric characters; one character when the second is a space; `@` otherwise.
pub fn bucket_for(name: &str) -> String {
    let mut chars = name.trim().chars();
    let (Some(first), second) = (chars.next(), chars.next()) else {
        return "@".into();
    };
    if !alnum(first) {
        return "@".into();
    }
    let first: String = first.to_lowercase().collect();
    match second {
        Some(' ') | None => first,
        Some(c) if alnum(c) => format!("{first}{}", c.to_lowercase().collect::<String>()),
        Some(_) => "@".into(),
    }
}

/// How well a title answers a query, 0 for no match: exact, then prefix, then containing every word.
fn score(query: &str, name: &str) -> u32 {
    let (query_compact, name_compact) = (compact(query), compact(name));
    if query_compact.is_empty() || name_compact.is_empty() {
        return 0;
    }
    if name_compact == query_compact {
        return 100;
    }
    if name_compact.starts_with(&query_compact) {
        // The closer in length, the likelier the right edition.
        return 80 - (name_compact.len() - query_compact.len()).min(30) as u32;
    }
    let name_lower = name.to_lowercase();
    let words: Vec<String> = query
        .split(|c: char| !alnum(c))
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect();
    if !words.is_empty() && words.iter().all(|word| name_lower.contains(word.as_str())) {
        return 40;
    }
    0
}

/// The best matches among a bucket's `{ id: { name } }` entries, most relevant first.
pub fn rank(bucket: &Value, query: &str, limit: usize) -> Vec<(u64, String)> {
    let Some(entries) = bucket.as_object() else {
        return Vec::new();
    };
    let mut scored: Vec<(u32, u64, String)> = entries
        .iter()
        .filter_map(|(id, entry)| {
            let name = entry.get("name")?.as_str()?;
            let id = id.parse().ok()?;
            let value = score(query, name);
            (value > 0).then(|| (value, id, name.to_string()))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.2.len().cmp(&b.2.len()))
            .then(a.1.cmp(&b.1))
    });
    scored.dedup_by(|a, b| a.1 == b.1);
    scored
        .into_iter()
        .take(limit)
        .map(|(_, id, name)| (id, name))
        .collect()
}

/// Cover URL at a usable size (`//images.igdb.com/.../t_thumb/x.jpg` is a thumbnail).
fn cover_url(raw: &str) -> String {
    let url = if raw.starts_with("//") {
        format!("https:{raw}")
    } else {
        raw.to_string()
    };
    url.replace("/t_thumb/", "/t_cover_big/")
}

/// A hit from a game's own JSON file.
pub fn parse_game(id: u64, game: &Value) -> Option<GameHit> {
    let name = game.get("name")?.as_str()?.to_string();
    let developer = game
        .get("involved_companies")
        .and_then(Value::as_array)
        .and_then(|companies| {
            companies.iter().find_map(|entry| {
                (entry.get("developer").and_then(Value::as_bool) == Some(true))
                    .then(|| entry.pointer("/company/name")?.as_str().map(str::to_string))
                    .flatten()
            })
        });
    // The first release anywhere is the game's release date.
    let first = game
        .get("release_dates")
        .and_then(Value::as_array)
        .and_then(|dates| {
            dates
                .iter()
                .filter_map(|entry| entry.get("date")?.as_i64())
                .min()
        });
    let release_date = first
        .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
        .map(|date| date.format("%Y-%m-%d").to_string());
    let genres = game
        .get("genres")
        .and_then(Value::as_array)
        .map(|genres| {
            genres
                .iter()
                .filter_map(|genre| genre.get("name")?.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let pc = game
        .get("platforms")
        .and_then(Value::as_array)
        .is_some_and(|platforms| {
            platforms
                .iter()
                .filter_map(Value::as_i64)
                .any(|platform| platform == PLATFORM_PC || platform == PLATFORM_LINUX)
        });
    let description = game
        .get("summary")
        .and_then(Value::as_str)
        .map(str::to_string);
    let year = release_date
        .as_deref()
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse().ok());

    Some(GameHit {
        id,
        name,
        year,
        developer: developer.clone(),
        cover: game
            .pointer("/cover/url")
            .and_then(Value::as_str)
            .map(cover_url),
        pc,
        meta: GameMeta {
            developer,
            release_date,
            genres,
            description,
            igdb_id: Some(id),
        },
    })
}

fn get_json(agent: &ureq::Agent, url: &str) -> Result<Value, String> {
    agent
        .get(url)
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .read_json::<Value>()
        .map_err(|error| error.to_string())
}

/// A bucket, from the disk cache when fresh enough. A missing bucket means no such titles.
fn bucket(agent: &ureq::Agent, cache: &Path, name: &str) -> Result<Value, String> {
    let file = cache.join(format!("{}.json", name.replace('@', "_at")));
    let fresh = fs::metadata(&file)
        .and_then(|meta| meta.modified())
        .is_ok_and(|modified| {
            SystemTime::now()
                .duration_since(modified)
                .is_ok_and(|age| age < BUCKET_MAX_AGE)
        });
    if fresh
        && let Ok(text) = fs::read_to_string(&file)
        && let Ok(value) = serde_json::from_str(&text)
    {
        return Ok(value);
    }
    match agent.get(&format!("{BASE}/buckets/{name}.json")).call() {
        Ok(mut response) => {
            let text = response
                .body_mut()
                .read_to_string()
                .map_err(|e| e.to_string())?;
            let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let _ = fs::create_dir_all(cache);
            let _ = fs::write(&file, text);
            Ok(value)
        }
        Err(ureq::Error::StatusCode(404)) => Ok(Value::Object(Default::default())),
        // Offline: a stale copy beats nothing.
        Err(error) => fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .ok_or_else(|| error.to_string()),
    }
}

/// Searches GameDB for `query` and fetches the details of the best matches. PC releases come
/// first, since the game being added runs on a PC.
pub fn search(cache: &Path, query: &str, limit: usize) -> Result<Vec<GameHit>, String> {
    let query = query.trim();
    if compact(query).len() < 2 {
        return Ok(Vec::new());
    }
    let agent = crate::artwork::agent();

    // The title's own bucket, plus "th" for titles that start with "The" when the query does not.
    let mut names = vec![bucket_for(query)];
    if !query.to_lowercase().starts_with("the ") && names[0] != "th" {
        names.push("th".into());
    }
    let mut ranked: Vec<(u64, String)> = Vec::new();
    let mut last_error = None;
    for name in &names {
        match bucket(&agent, cache, name) {
            Ok(value) => {
                // Titles in the "th" bucket are matched against the query with "the" in front.
                let text = if name == "th" && names[0] != "th" {
                    format!("the {query}")
                } else {
                    query.to_string()
                };
                ranked.extend(rank(&value, &text, DETAILED_HITS));
            }
            Err(error) => last_error = Some(error),
        }
    }
    if ranked.is_empty() {
        return last_error.map_or(Ok(Vec::new()), Err);
    }
    ranked.dedup_by(|a, b| a.0 == b.0);
    ranked.sort_by_key(|(_, name)| std::cmp::Reverse(score(query, name)));
    ranked.truncate(DETAILED_HITS);

    let mut hits: Vec<GameHit> = std::thread::scope(|scope| {
        let handles: Vec<_> = ranked
            .iter()
            .map(|(id, _)| {
                let agent = &agent;
                scope.spawn(move || {
                    get_json(agent, &format!("{BASE}/games/{id}.json"))
                        .ok()
                        .and_then(|game| parse_game(*id, &game))
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok().flatten())
            .collect()
    });
    // Keep the ranking, but put PC games ahead of console-only ones of the same score.
    hits.sort_by_key(|hit| {
        (
            std::cmp::Reverse(score(query, &hit.name)),
            !hit.pc,
            std::cmp::Reverse(hit.year),
        )
    });
    hits.truncate(limit);
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn titles_go_to_the_bucket_their_first_characters_name() {
        assert_eq!(bucket_for("Halo"), "ha");
        assert_eq!(bucket_for("FIFA 19"), "fi");
        assert_eq!(bucket_for("A Plague Tale"), "a");
        assert_eq!(bucket_for("  The Witcher 3"), "th");
        assert_eq!(bucket_for("!Trap"), "@");
        assert_eq!(bucket_for("F.E.A.R."), "@");
        assert_eq!(bucket_for(""), "@");
    }

    #[test]
    fn compact_comparison_ignores_spaces_and_punctuation() {
        assert_eq!(score("FIFA19", "FIFA 19"), 100);
        assert_eq!(score("fifa 19", "FIFA 19"), 100);
        assert!(score("fifa", "FIFA 19") > score("fifa", "FIFA 19: Legacy Edition"));
        assert!(score("witcher 3", "The Witcher 3: Wild Hunt") > 0);
        assert_eq!(score("zelda", "FIFA 19"), 0);
        assert_eq!(score("", "FIFA 19"), 0);
    }

    #[test]
    fn ranking_puts_exact_names_first_and_closest_editions_next() {
        let bucket = json!({
            "1": {"name": "FIFA 19: Legacy Edition"},
            "2": {"name": "FIFA 19"},
            "3": {"name": "FIFA 18"},
            "4": {"name": "FIFA 19"},
            "5": {"name": "Fifa Soccer 19 Ultimate Team Mobile Special"},
        });
        let ranked = rank(&bucket, "fifa 19", 10);
        let ids: Vec<u64> = ranked.iter().map(|(id, _)| *id).collect();
        assert_eq!(&ids[..2], &[2, 4]);
        assert_eq!(ids[2], 1);
        assert!(!ids.contains(&3));
        assert_eq!(rank(&bucket, "fifa 19", 2).len(), 2);
        assert!(rank(&json!([]), "x", 3).is_empty());
    }

    #[test]
    fn a_game_file_becomes_editable_details() {
        let game = json!({
            "name": "FIFA 19",
            "summary": "Football.",
            "genres": [{"id": 14, "name": "Sport"}],
            "platforms": [6, 48],
            "cover": {"id": 1, "url": "//images.igdb.com/igdb/image/upload/t_thumb/co1.jpg"},
            "involved_companies": [
                {"company": {"name": "EA Sports"}, "developer": false},
                {"company": {"name": "EA Canada"}, "developer": true}
            ],
            "release_dates": [
                {"date": 1538006400, "platform": 6},
                {"date": 1537920000, "platform": 48},
                {"platform": 9}
            ]
        });
        let hit = parse_game(96209, &game).unwrap();
        assert_eq!(hit.developer.as_deref(), Some("EA Canada"));
        assert_eq!(hit.meta.release_date.as_deref(), Some("2018-09-26"));
        assert_eq!(hit.year, Some(2018));
        assert_eq!(hit.meta.genres, vec!["Sport"]);
        assert_eq!(hit.meta.igdb_id, Some(96209));
        assert!(hit.pc);
        assert_eq!(
            hit.cover.as_deref(),
            Some("https://images.igdb.com/igdb/image/upload/t_cover_big/co1.jpg")
        );
    }

    #[test]
    fn a_sparse_game_file_still_parses_and_console_only_games_are_not_pc() {
        let hit = parse_game(1, &json!({"name": "Old Game", "platforms": [48]})).unwrap();
        assert!(!hit.pc);
        assert_eq!(hit.year, None);
        assert!(hit.meta.genres.is_empty());
        assert!(parse_game(2, &json!({"summary": "no name"})).is_none());
    }

    #[test]
    fn buckets_are_cached_on_disk_and_reused_without_the_network() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("fi.json"), r#"{"2": {"name": "FIFA 19"}}"#).unwrap();
        // A fresh cache answers even when the network is unreachable.
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(1)))
            .build()
            .into();
        let value = bucket(&agent, dir.path(), "fi").unwrap();
        assert_eq!(value["2"]["name"], "FIFA 19");
    }

    /// Needs the network: `cargo test -- --ignored gamedb`.
    #[test]
    #[ignore]
    fn live_search_finds_a_game_from_an_executable_name() {
        let dir = tempfile::tempdir().unwrap();
        let hits = search(dir.path(), "FIFA19", 6).unwrap();
        println!("{hits:#?}");
        assert!(
            hits.iter()
                .any(|hit| hit.name == "FIFA 19" && hit.year == Some(2018))
        );
        assert!(dir.path().join("fi.json").is_file());
        let witcher = search(dir.path(), "witcher 3", 6).unwrap();
        assert!(
            witcher
                .iter()
                .any(|hit| hit.name.starts_with("The Witcher 3"))
        );
    }
}
