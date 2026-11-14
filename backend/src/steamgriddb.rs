//! Client for the SteamGridDB API (https://www.steamgriddb.com/api/v2): finds and downloads artwork.

use chrono::{DateTime, Datelike};
use serde::Serialize;
use serde_json::Value;

use crate::artwork::{self, Error};

const API: &str = "https://www.steamgriddb.com/api/v2";

/// Percent-encodes a path segment.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// A game found on SteamGridDB.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchHit {
    pub id: u64,
    pub name: String,
    pub year: Option<i32>,
    pub verified: bool,
}

/// One image offered for a game.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Candidate {
    pub url: String,
    /// Smaller version for previews.
    pub thumb: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub author: Option<String>,
    /// SteamGridDB style (`alternate`, `blurred`...) or `official` for Steam's own.
    pub style: Option<String>,
}

fn parse_candidates(body: &Value) -> Vec<Candidate> {
    let Some(items) = body.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let url = item.get("url")?.as_str()?.to_string();
            Some(Candidate {
                thumb: item
                    .get("thumb")
                    .and_then(Value::as_str)
                    .unwrap_or(&url)
                    .to_string(),
                width: item.get("width").and_then(Value::as_u64).map(|v| v as u32),
                height: item.get("height").and_then(Value::as_u64).map(|v| v as u32),
                author: item
                    .get("author")
                    .and_then(|author| author.get("name"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                style: item
                    .get("style")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                url,
            })
        })
        .collect()
}

fn parse_hit(item: &Value) -> Option<SearchHit> {
    Some(SearchHit {
        id: item.get("id")?.as_u64()?,
        name: item.get("name")?.as_str()?.to_string(),
        year: item
            .get("release_date")
            .and_then(Value::as_i64)
            .and_then(|timestamp| DateTime::from_timestamp(timestamp, 0))
            .map(|date| date.year()),
        verified: item
            .get("verified")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn parse_hits(body: &Value) -> Vec<SearchHit> {
    let Some(items) = body.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    items.iter().filter_map(parse_hit).collect()
}

fn first_id(body: &Value) -> Option<u64> {
    match body.get("data")? {
        Value::Array(items) => items.first()?.get("id")?.as_u64(),
        object => object.get("id")?.as_u64(),
    }
}

fn first_url(body: &Value) -> Option<String> {
    body.get("data")?
        .as_array()?
        .first()?
        .get("url")?
        .as_str()
        .map(str::to_string)
}

pub struct Client {
    key: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(key: &str) -> Self {
        let agent = artwork::agent();
        Self {
            key: key.trim().to_string(),
            agent,
        }
    }

    /// `Ok(None)` when the API answers 404 (nothing found).
    fn get(&self, path: &str) -> Result<Option<Value>, Error> {
        let response = self
            .agent
            .get(format!("{API}{path}"))
            .header("Authorization", format!("Bearer {}", self.key))
            .call();

        match response {
            Ok(mut response) => response
                .body_mut()
                .read_json::<Value>()
                .map(Some)
                .map_err(|e| Error::Other(e.to_string())),
            Err(ureq::Error::StatusCode(401 | 403)) => Err(Error::InvalidKey),
            Err(ureq::Error::StatusCode(404)) => Ok(None),
            Err(e) => Err(Error::Other(e.to_string())),
        }
    }

    /// Cheap request used to check a key before saving it.
    pub fn validate(&self) -> Result<(), Error> {
        self.get("/search/autocomplete/a").map(|_| ())
    }

    /// Games matching what the user typed, best match first.
    pub fn search(&self, term: &str) -> Result<Vec<SearchHit>, Error> {
        Ok(self
            .get(&format!("/search/autocomplete/{}", encode(term.trim())))?
            .as_ref()
            .map(parse_hits)
            .unwrap_or_default())
    }

    /// Name and year of a SteamGridDB game, to show which one is used.
    pub fn game_info(&self, game: u64) -> Result<Option<SearchHit>, Error> {
        Ok(self
            .get(&format!("/games/id/{game}"))?
            .as_ref()
            .and_then(|body| body.get("data"))
            .and_then(parse_hit))
    }

    /// Every portrait cover offered for a game.
    pub fn poster_candidates(&self, game: u64) -> Result<Vec<Candidate>, Error> {
        Ok(self
            .get(&format!(
                "/grids/game/{game}?dimensions=600x900,342x482,660x930&types=static"
            ))?
            .as_ref()
            .map(parse_candidates)
            .unwrap_or_default())
    }

    /// Every logo offered for a game.
    pub fn logo_candidates(&self, game: u64) -> Result<Vec<Candidate>, Error> {
        Ok(self
            .get(&format!("/logos/game/{game}?types=static"))?
            .as_ref()
            .map(parse_candidates)
            .unwrap_or_default())
    }

    /// Every wide banner offered for a game.
    pub fn hero_candidates(&self, game: u64) -> Result<Vec<Candidate>, Error> {
        Ok(self
            .get(&format!("/heroes/game/{game}?types=static"))?
            .as_ref()
            .map(parse_candidates)
            .unwrap_or_default())
    }

    pub fn find_game(&self, title: &str, steam_app_id: Option<&str>) -> Result<Option<u64>, Error> {
        if let Some(id) = steam_app_id
            .and_then(|app_id| self.get(&format!("/games/steam/{app_id}")).transpose())
            .transpose()?
            .as_ref()
            .and_then(first_id)
        {
            return Ok(Some(id));
        }
        Ok(self
            .get(&format!("/search/autocomplete/{}", encode(title)))?
            .as_ref()
            .and_then(first_id))
    }

    fn artwork(&self, kind: &str, game: u64, dimensions: &str) -> Result<Option<String>, Error> {
        let filtered = self.get(&format!(
            "/{kind}/game/{game}?dimensions={dimensions}&types=static"
        ))?;
        if let Some(url) = filtered.as_ref().and_then(first_url) {
            return Ok(Some(url));
        }
        Ok(self
            .get(&format!("/{kind}/game/{game}?types=static"))?
            .as_ref()
            .and_then(first_url))
    }

    /// Portrait box art, used for covers.
    pub fn poster_url(&self, game: u64) -> Result<Option<String>, Error> {
        self.artwork("grids", game, "600x900,342x482,660x930")
    }

    /// Transparent title art, shown instead of the game's name. It sits on a darkened banner,
    /// so a white logo is asked for first (an official one is often black), then any other.
    pub fn logo_url(&self, game: u64) -> Result<Option<String>, Error> {
        for styles in ["&styles=white", "&styles=official", ""] {
            let found = self.get(&format!("/logos/game/{game}?types=static{styles}"))?;
            if let Some(url) = found.as_ref().and_then(first_url) {
                return Ok(Some(url));
            }
        }
        Ok(None)
    }

    /// Wide banner, used for hero backgrounds.
    pub fn hero_url(&self, game: u64) -> Result<Option<String>, Error> {
        self.artwork("heroes", game, "1920x620,3840x1240,1600x650")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_titles_for_the_url_path() {
        assert_eq!(encode("Moss & Mortar"), "Moss%20%26%20Mortar");
        assert_eq!(encode("Brine-Bone_2"), "Brine-Bone_2");
        assert_eq!(encode("Pokémon"), "Pok%C3%A9mon");
    }

    #[test]
    fn reads_the_game_id_from_search_and_direct_lookups() {
        let search: Value =
            serde_json::from_str(r#"{"success":true,"data":[{"id":5247,"name":"A"},{"id":1}]}"#)
                .unwrap();
        let direct: Value =
            serde_json::from_str(r#"{"success":true,"data":{"id":42,"name":"B"}}"#).unwrap();
        let empty: Value = serde_json::from_str(r#"{"success":true,"data":[]}"#).unwrap();

        assert_eq!(first_id(&search), Some(5247));
        assert_eq!(first_id(&direct), Some(42));
        assert_eq!(first_id(&empty), None);
    }

    #[test]
    fn reads_search_results_with_year_and_verified_flag() {
        let body: Value = serde_json::from_str(
            r#"{"success":true,"data":[
                {"id":1,"name":"Brawlhalla","release_date":1534377600,"verified":true},
                {"id":2,"name":"Unknown Date","release_date":null},
                {"name":"Missing id"}
            ]}"#,
        )
        .unwrap();

        let hits = parse_hits(&body);
        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0],
            SearchHit {
                id: 1,
                name: "Brawlhalla".into(),
                year: Some(2018),
                verified: true
            }
        );
        assert_eq!(hits[1].year, None);
        assert!(!hits[1].verified);
        assert!(parse_hits(&serde_json::json!({"data": null})).is_empty());
    }

    #[test]
    fn reads_image_candidates_with_their_author() {
        let body: Value = serde_json::from_str(
            r#"{"success":true,"data":[
                {"id":1,"url":"https://cdn/a.png","thumb":"https://cdn/a_t.png","width":600,"height":900,"style":"alternate","author":{"name":"Ann"}},
                {"id":2,"url":"https://cdn/b.png"},
                {"id":3}
            ]}"#,
        )
        .unwrap();

        let candidates = parse_candidates(&body);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].thumb, "https://cdn/a_t.png");
        assert_eq!(candidates[0].author.as_deref(), Some("Ann"));
        assert_eq!(candidates[0].width, Some(600));
        assert_eq!(
            candidates[1].thumb, "https://cdn/b.png",
            "the full image stands in for a missing thumbnail"
        );
    }

    #[test]
    fn reads_the_first_artwork_url() {
        let body: Value = serde_json::from_str(r#"{"data":[{"url":"https://cdn.example/a.webp"},{"url":"https://cdn.example/b.png"}]}"#).unwrap();
        assert_eq!(
            first_url(&body).as_deref(),
            Some("https://cdn.example/a.webp")
        );
        assert_eq!(first_url(&serde_json::json!({"data": []})), None);
    }
}
