//! Pictures of the systems, from RetroArch's own assets when they are installed (the
//! `retroarch-assets` package): its "systematic" theme draws each console, named after its
//! libretro database, which is exactly how Lutra names systems.

use std::path::{Path, PathBuf};

/// Themes tried in order: consoles drawn in colour, then their controllers.
const THEMES: [&str; 2] = ["systematic", "retrosystem"];

/// Where RetroArch's assets can be: the distribution's, then the user's own copy.
pub fn roots(home: &Path) -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/retroarch/assets/xmb"),
        PathBuf::from("/usr/share/libretro/assets/xmb"),
        home.join(".config/retroarch/assets/xmb"),
    ]
}

/// The picture of `system`, if one is installed.
pub fn system_icon(roots: &[PathBuf], system: &str) -> Option<PathBuf> {
    // A database name never holds a path separator; refuse one rather than leave the folder.
    if system.contains('/') {
        return None;
    }
    THEMES.iter().find_map(|theme| {
        roots
            .iter()
            .map(|root| root.join(theme).join("png").join(format!("{system}.png")))
            .find(|path| path.is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_console_drawing_wins_over_the_controller() {
        let dir = tempfile::tempdir().unwrap();
        for theme in ["systematic", "retrosystem"] {
            std::fs::create_dir_all(dir.path().join(theme).join("png")).unwrap();
        }
        let png = |theme: &str, name: &str| {
            dir.path()
                .join(theme)
                .join("png")
                .join(format!("{name}.png"))
        };
        std::fs::write(png("retrosystem", "Sega - Saturn"), b"").unwrap();
        std::fs::write(png("systematic", "Sony - PlayStation"), b"").unwrap();
        std::fs::write(png("retrosystem", "Sony - PlayStation"), b"").unwrap();
        let roots = [dir.path().to_path_buf()];

        assert_eq!(
            system_icon(&roots, "Sony - PlayStation"),
            Some(png("systematic", "Sony - PlayStation"))
        );
        assert_eq!(
            system_icon(&roots, "Sega - Saturn"),
            Some(png("retrosystem", "Sega - Saturn"))
        );
        assert_eq!(system_icon(&roots, "Unknown - Thing"), None);
        assert_eq!(system_icon(&roots, "../etc/passwd"), None);
    }
}
