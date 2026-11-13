//! libretro `.info` files: what a core is, which systems and files it handles, which firmware it
//! needs. They are the source of truth for everything Lutra knows about a core.

use std::collections::HashMap;

use serde::Serialize;

/// A firmware (BIOS) file a core asks for.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Firmware {
    pub description: String,
    /// Relative to the system directory (it may hold a sub-folder: `dc/dc_boot.bin`).
    pub path: String,
    /// The core works without it, with fewer features or games.
    pub optional: bool,
    /// Expected MD5, when the core's notes give one.
    pub md5: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreInfo {
    /// File stem without `_libretro`: `snes9x`.
    pub id: String,
    pub display_name: String,
    /// The core's own short name (`Snes9x`): RetroArch keeps per-core files in a folder named so.
    pub core_name: String,
    pub version: Option<String>,
    /// libretro database names (`Nintendo - Game Boy`), one per system the core handles.
    pub databases: Vec<String>,
    pub extensions: Vec<String>,
    pub firmware: Vec<Firmware>,
    /// The core wants a path on disk rather than the file's bytes (needed for disc images).
    pub needs_fullpath: bool,
    /// Needs OpenGL or Vulkan rendering from the frontend.
    pub hw_render: bool,
    /// Runs without any game (a core that is its own program).
    pub supports_no_game: bool,
}

/// `key = "value"` lines; comments start with `#`.
pub fn parse_pairs(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            ))
        })
        .collect()
}

fn list(value: Option<&String>) -> Vec<String> {
    value
        .map(|text| {
            text.split('|')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// MD5s listed in a core's notes: `(!) BS-X.bin (md5): fed4d8...|(!) other.bin (md5): ...`.
fn notes_md5(notes: &str) -> HashMap<String, String> {
    notes
        .split('|')
        .filter_map(|part| {
            let (name, hash) = part.split_once("(md5):")?;
            let name = name
                .trim()
                .trim_start_matches("(!)")
                .trim_start_matches("(*)")
                .trim();
            let hash = hash.trim().to_lowercase();
            (hash.len() == 32 && hash.chars().all(|c| c.is_ascii_hexdigit()))
                .then(|| (name.to_lowercase(), hash))
        })
        .collect()
}

fn flag(pairs: &HashMap<String, String>, key: &str) -> bool {
    pairs.get(key).is_some_and(|value| value == "true")
}

pub fn parse_core(id: &str, text: &str) -> CoreInfo {
    let pairs = parse_pairs(text);
    let hashes = pairs
        .get("notes")
        .map(|notes| notes_md5(notes))
        .unwrap_or_default();
    let count: usize = pairs
        .get("firmware_count")
        .and_then(|count| count.parse().ok())
        .unwrap_or(0);
    let firmware = (0..count)
        .filter_map(|index| {
            let path = pairs.get(&format!("firmware{index}_path"))?.clone();
            let file = path.rsplit('/').next().unwrap_or(&path).to_lowercase();
            Some(Firmware {
                description: pairs
                    .get(&format!("firmware{index}_desc"))
                    .cloned()
                    .unwrap_or_else(|| path.clone()),
                optional: pairs
                    .get(&format!("firmware{index}_opt"))
                    .is_none_or(|value| value == "true"),
                md5: hashes.get(&file).cloned(),
                path,
            })
        })
        .collect();

    CoreInfo {
        id: id.to_string(),
        display_name: pairs
            .get("display_name")
            .cloned()
            .unwrap_or_else(|| id.to_string()),
        core_name: pairs
            .get("corename")
            .cloned()
            .unwrap_or_else(|| id.to_string()),
        version: pairs.get("display_version").cloned(),
        databases: list(pairs.get("database")),
        extensions: list(pairs.get("supported_extensions"))
            .into_iter()
            .map(|ext| ext.to_lowercase())
            .collect(),
        firmware,
        needs_fullpath: flag(&pairs, "needs_fullpath"),
        hw_render: flag(&pairs, "hw_render"),
        supports_no_game: flag(&pairs, "supports_no_game"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNES9X: &str = r#"# Software Information
display_name = "Nintendo - SNES / SFC (Snes9x)"
corename = "Snes9x"
supported_extensions = "smc|sfc|swc|fig|bs|st"
display_version = "1.61"
firmware_count = 2
firmware0_desc = "BS-X.bin (BS-X)"
firmware0_path = "BS-X.bin"
firmware0_opt = "true"
firmware1_desc = "STBIOS.bin (Sufami Turbo)"
firmware1_path = "bios/STBIOS.bin"
firmware1_opt = "false"
notes = "(!) BS-X.bin (md5): FED4D8242CBED61343D53D48432ACED|(!) STBIOS.bin (md5): d3a44ba7d42a74d3ac58cb9c14c6a5ca|(!) free text"
hw_render = "false"
needs_fullpath = "false"
database = "Nintendo - Super Nintendo Entertainment System|Nintendo - Sufami Turbo"
"#;

    #[test]
    fn a_core_is_described_by_its_info_file() {
        let info = parse_core("snes9x", SNES9X);
        assert_eq!(info.display_name, "Nintendo - SNES / SFC (Snes9x)");
        assert_eq!(info.version.as_deref(), Some("1.61"));
        assert_eq!(info.core_name, "Snes9x");
        assert_eq!(
            info.extensions,
            vec!["smc", "sfc", "swc", "fig", "bs", "st"]
        );
        assert_eq!(
            info.databases,
            vec![
                "Nintendo - Super Nintendo Entertainment System",
                "Nintendo - Sufami Turbo"
            ]
        );
        assert!(!info.needs_fullpath && !info.hw_render);
    }

    #[test]
    fn firmware_comes_with_optionality_and_a_md5_when_the_notes_have_a_valid_one() {
        let info = parse_core("snes9x", SNES9X);
        assert_eq!(info.firmware.len(), 2);
        // The first hash in the notes is 31 characters: it is not trusted.
        assert_eq!(info.firmware[0].md5, None);
        assert!(info.firmware[0].optional);
        assert_eq!(
            info.firmware[1].md5.as_deref(),
            Some("d3a44ba7d42a74d3ac58cb9c14c6a5ca")
        );
        assert!(!info.firmware[1].optional);
        assert_eq!(info.firmware[1].path, "bios/STBIOS.bin");
    }

    #[test]
    fn missing_fields_fall_back_to_safe_defaults() {
        let info = parse_core("mystery", "");
        assert_eq!(info.display_name, "mystery");
        assert!(
            info.databases.is_empty() && info.extensions.is_empty() && info.firmware.is_empty()
        );
        assert!(!info.supports_no_game);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let pairs = parse_pairs("# c\n\nkey = \"v\"\nother=1\n");
        assert_eq!(pairs.get("key").map(String::as_str), Some("v"));
        assert_eq!(pairs.get("other").map(String::as_str), Some("1"));
        assert_eq!(pairs.len(), 2);
    }
}
