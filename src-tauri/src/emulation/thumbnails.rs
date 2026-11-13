//! Artwork from libretro's thumbnail server, named after the game's No-Intro file name:
//! `thumbnails.libretro.com/<system>/Named_Boxarts/<name>.png`. No key is needed.

const BASE: &str = "https://thumbnails.libretro.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Boxart,
    Title,
    Snap,
    /// Transparent title art.
    Logo,
}

impl Kind {
    fn folder(self) -> &'static str {
        match self {
            Kind::Boxart => "Named_Boxarts",
            Kind::Title => "Named_Titles",
            Kind::Snap => "Named_Snaps",
            Kind::Logo => "Named_Logos",
        }
    }
}

/// The server replaces these characters of a game's name with `_` in file names.
pub fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if "&*/:`\"<>?\\|".contains(c) { '_' } else { c })
        .collect()
}

fn encode(segment: &str) -> String {
    segment
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b'('
            | b')'
            | b'!'
            | b'\''
            | b','
            | b'+'
            | b'&' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// The names to try for a file stem: as is, then without trailing `(Rev 1)`-style groups, which
/// the thumbnail set often leaves out.
fn names(stem: &str) -> Vec<String> {
    let mut found = vec![stem.to_string()];
    let mut current = stem.trim().to_string();
    while current.ends_with(')') {
        let Some(open) = current.rfind(" (") else {
            break;
        };
        current.truncate(open);
        if !current.is_empty() {
            found.push(current.clone());
        }
    }
    found.dedup();
    found
}

/// URLs to try, best first, for a game of `system` (libretro database name) whose file is `stem`.
pub fn candidates(system: &str, stem: &str, kind: Kind) -> Vec<String> {
    names(stem)
        .into_iter()
        .map(|name| {
            format!(
                "{BASE}/{}/{}/{}.png",
                encode(system),
                kind.folder(),
                encode(&sanitize(&name))
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_characters_become_underscores() {
        assert_eq!(sanitize("Q*bert: The Game?"), "Q_bert_ The Game_");
        assert_eq!(sanitize("AC/DC"), "AC_DC");
    }

    #[test]
    fn the_url_names_the_system_kind_and_game() {
        let urls = candidates(
            "Nintendo - Super Nintendo Entertainment System",
            "Super Mario World (USA)",
            Kind::Boxart,
        );
        assert_eq!(
            urls[0],
            "https://thumbnails.libretro.com/Nintendo%20-%20Super%20Nintendo%20Entertainment%20System/Named_Boxarts/Super%20Mario%20World%20(USA).png"
        );
    }

    #[test]
    fn trailing_tags_are_dropped_one_by_one_as_fallbacks() {
        let urls = candidates("Sony - PlayStation", "Game (Europe) (Rev 1)", Kind::Title);
        assert_eq!(urls.len(), 3);
        assert!(urls[0].ends_with("Game%20(Europe)%20(Rev%201).png"));
        assert!(urls[1].ends_with("Game%20(Europe).png"));
        assert!(urls[2].ends_with("/Named_Titles/Game.png"));
    }

    #[test]
    fn a_name_without_tags_has_a_single_candidate() {
        assert_eq!(candidates("X", "Tetris", Kind::Snap).len(), 1);
    }
}
