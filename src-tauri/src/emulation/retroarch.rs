//! Running a core through RetroArch with Lutra in charge: its own config file, its own folders,
//! the core Lutra chose. RetroArch's usual config in `~/.config/retroarch` is never read or written.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::profile::EmulationProfile;
use crate::wine::LaunchSpec;

/// Lutra's emulation folders under `<data>/emulation`.
#[derive(Debug, Clone)]
pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            root: data_dir.join("emulation"),
        }
    }

    pub fn cores(&self) -> PathBuf {
        self.root.join("cores")
    }
    /// Where BIOS files go when the user has not picked a folder of their own.
    pub fn default_system(&self) -> PathBuf {
        self.root.join("system")
    }
    pub fn saves(&self) -> PathBuf {
        self.root.join("saves")
    }
    pub fn states(&self) -> PathBuf {
        self.root.join("states")
    }
    pub fn screenshots(&self) -> PathBuf {
        self.root.join("screenshots")
    }
    pub fn options(&self) -> PathBuf {
        self.root.join("options")
    }
    /// RetroArch's per-core and per-game files (core options, overrides, remaps).
    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }
    pub fn config(&self) -> PathBuf {
        self.root.join("retroarch.cfg")
    }
    /// The per-game overrides appended to the main config.
    pub fn game_config(&self, game: &str) -> PathBuf {
        self.options().join(format!("{game}.cfg"))
    }
    /// Where RetroArch reads a game's core options: `<config>/<core name>/<game>.opt`, with
    /// `game_specific_options` on.
    pub fn core_options(&self, core_name: &str, rom_stem: &str) -> PathBuf {
        self.config_dir()
            .join(core_name)
            .join(format!("{rom_stem}.opt"))
    }
}

/// What the user can set for one game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EmulatorConfig {
    /// libretro database name of the game's system.
    pub system: String,
    pub fullscreen: bool,
    /// Saves the state when leaving and loads it on the next start.
    pub auto_resume: bool,
    /// Picture, controls and emulator options kept for this game only; its console's otherwise.
    pub profile: Option<EmulationProfile>,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            system: String::new(),
            fullscreen: true,
            auto_resume: false,
            profile: None,
        }
    }
}

fn quoted(value: &Path) -> String {
    format!("\"{}\"", value.display())
}

/// The main `retroarch.cfg`. Everything that decides where files go is Lutra's.
pub fn main_config(
    layout: &Layout,
    system_dir: &Path,
    libretro_dirs: &[&Path],
    info_dir: Option<&Path>,
    autoconfig_dir: Option<&Path>,
) -> String {
    let mut lines = vec![
        format!("system_directory = {}", quoted(system_dir)),
        format!("savefile_directory = {}", quoted(&layout.saves())),
        format!("savestate_directory = {}", quoted(&layout.states())),
        format!("screenshot_directory = {}", quoted(&layout.screenshots())),
        // Saves and states are grouped by system, so two games of one name never clash.
        "sort_savefiles_by_content_enable = \"true\"".into(),
        "sort_savestates_by_content_enable = \"true\"".into(),
        "sort_screenshots_by_content_enable = \"true\"".into(),
        // Everything RetroArch keeps for itself stays under Lutra's folder: its per-core files,
        // remaps, playlists, history and cache. Nothing goes to ~/.config/retroarch.
        format!("rgui_config_directory = {}", quoted(&layout.config_dir())),
        format!(
            "input_remapping_directory = {}",
            quoted(&layout.root.join("remaps"))
        ),
        format!(
            "playlist_directory = {}",
            quoted(&layout.root.join("playlists"))
        ),
        format!("cache_directory = {}", quoted(&layout.root.join("cache"))),
        format!("log_dir = {}", quoted(&layout.root.join("logs"))),
        format!(
            "content_history_path = {}",
            quoted(&layout.root.join("history/content_history.lpl"))
        ),
        format!(
            "content_favorites_path = {}",
            quoted(&layout.root.join("history/favorites.lpl"))
        ),
        format!(
            "content_image_history_path = {}",
            quoted(&layout.root.join("history/image_history.lpl"))
        ),
        format!(
            "content_music_history_path = {}",
            quoted(&layout.root.join("history/music_history.lpl"))
        ),
        format!(
            "content_video_history_path = {}",
            quoted(&layout.root.join("history/video_history.lpl"))
        ),
        "history_list_enable = \"false\"".into(),
        "game_specific_options = \"true\"".into(),
        // Lutra decides what runs: nothing is written back, no core is fetched by RetroArch.
        "config_save_on_exit = \"false\"".into(),
        "savestate_thumbnail_enable = \"true\"".into(),
        "pause_nonactive = \"true\"".into(),
        "quit_press_twice = \"false\"".into(),
        "video_fullscreen = \"true\"".into(),
        "video_windowed_fullscreen = \"true\"".into(),
        "input_autodetect_enable = \"true\"".into(),
        "menu_show_online_updater = \"false\"".into(),
        "menu_show_core_updater = \"false\"".into(),
    ];
    if let Some(dir) = libretro_dirs.first() {
        lines.push(format!("libretro_directory = {}", quoted(dir)));
    }
    if let Some(dir) = info_dir {
        lines.push(format!("libretro_info_path = {}", quoted(dir)));
    }
    if let Some(dir) = autoconfig_dir {
        lines.push(format!("joypad_autoconfig_dir = {}", quoted(dir)));
    }
    lines.join("\n") + "\n"
}

/// The overrides of one game, appended to the main config.
pub fn game_config(config: &EmulatorConfig) -> String {
    [
        format!("video_fullscreen = \"{}\"", config.fullscreen),
        format!("savestate_auto_save = \"{}\"", config.auto_resume),
        format!("savestate_auto_load = \"{}\"", config.auto_resume),
    ]
    .join("\n")
        + "\n"
}

/// Core options in the file format cores read: `key = "value"`.
pub fn core_options_text(options: &BTreeMap<String, String>) -> String {
    options
        .iter()
        .filter(|(key, _)| !key.trim().is_empty())
        .map(|(key, value)| format!("{} = \"{}\"\n", key.trim(), value.replace('"', "")))
        .collect()
}

/// The command that runs `rom` with `core`.
pub fn build_launch(
    retroarch: &Path,
    core: &Path,
    rom: &Path,
    config: &Path,
    append: &Path,
) -> LaunchSpec {
    let os = |text: &str| OsString::from(text);
    LaunchSpec {
        program: retroarch.to_path_buf(),
        args: vec![
            os("--verbose"),
            os("--config"),
            config.as_os_str().to_owned(),
            os("--appendconfig"),
            append.as_os_str().to_owned(),
            os("-L"),
            core.as_os_str().to_owned(),
            rom.as_os_str().to_owned(),
        ],
        envs: Vec::new(),
        cwd: rom.parent().map(Path::to_path_buf).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::new(Path::new("/data/lutra"))
    }

    #[test]
    fn the_main_config_points_every_folder_at_lutra() {
        let text = main_config(
            &layout(),
            Path::new("/home/me/Games/bios"),
            &[Path::new("/usr/lib/libretro")],
            Some(Path::new("/usr/share/libretro/info")),
            Some(Path::new("/usr/share/libretro/autoconfig")),
        );
        assert!(text.contains("system_directory = \"/home/me/Games/bios\""));
        assert!(text.contains("savefile_directory = \"/data/lutra/emulation/saves\""));
        assert!(text.contains("savestate_directory = \"/data/lutra/emulation/states\""));
        assert!(text.contains("config_save_on_exit = \"false\""));
        assert!(text.contains("joypad_autoconfig_dir = \"/usr/share/libretro/autoconfig\""));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn the_game_config_sets_fullscreen_and_resume() {
        let config = EmulatorConfig {
            fullscreen: false,
            auto_resume: true,
            ..EmulatorConfig::default()
        };
        let text = game_config(&config);
        assert!(text.contains("video_fullscreen = \"false\""));
        assert!(text.contains("savestate_auto_load = \"true\""));
    }

    #[test]
    fn core_options_live_where_retroarch_looks_for_a_games_options() {
        assert_eq!(
            layout().core_options("Snes9x", "Zelda (USA)"),
            PathBuf::from("/data/lutra/emulation/config/Snes9x/Zelda (USA).opt")
        );
    }

    #[test]
    fn nothing_retroarch_keeps_for_itself_goes_outside_lutras_folder() {
        let text = main_config(&layout(), Path::new("/bios"), &[], None, None);
        for key in [
            "rgui_config_directory",
            "input_remapping_directory",
            "playlist_directory",
            "cache_directory",
            "log_dir",
        ] {
            let line = text
                .lines()
                .find(|line| line.starts_with(key))
                .unwrap_or_else(|| panic!("{key} is not set"));
            assert!(line.contains("/data/lutra/emulation/"), "{line}");
        }
        assert!(text.contains("game_specific_options = \"true\""));
    }

    #[test]
    fn core_options_are_written_one_per_line_without_stray_quotes() {
        let text = core_options_text(&BTreeMap::from([
            ("snes9x_hires_blend".into(), "merge".into()),
            ("  ".into(), "ignored".into()),
            ("a".into(), "b\"c".into()),
        ]));
        assert_eq!(text, "a = \"bc\"\nsnes9x_hires_blend = \"merge\"\n");
    }

    #[test]
    fn the_launch_command_names_the_core_the_rom_and_both_configs() {
        let spec = build_launch(
            Path::new("/usr/bin/retroarch"),
            Path::new("/cores/snes9x_libretro.so"),
            Path::new("/roms/snes/Zelda.sfc"),
            Path::new("/data/retroarch.cfg"),
            Path::new("/data/options/g.cfg"),
        );
        let args: Vec<&str> = spec.args.iter().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            vec![
                "--verbose",
                "--config",
                "/data/retroarch.cfg",
                "--appendconfig",
                "/data/options/g.cfg",
                "-L",
                "/cores/snes9x_libretro.so",
                "/roms/snes/Zelda.sfc"
            ]
        );
        assert_eq!(spec.cwd, PathBuf::from("/roms/snes"));
    }

    #[test]
    fn a_config_survives_json_and_tolerates_missing_fields() {
        let config: EmulatorConfig =
            serde_json::from_str(r#"{"system":"Sega - Dreamcast"}"#).unwrap();
        assert!(config.fullscreen);
        // A core picked by older versions is ignored: Lutra chooses the emulator.
        let old: EmulatorConfig =
            serde_json::from_str(r#"{"system":"Sega - Dreamcast","core":"flycast"}"#).unwrap();
        assert_eq!(old.system, "Sega - Dreamcast");
    }

    /// Runs the real RetroArch headless on a tiny Game Boy ROM, with the files Lutra writes.
    /// Needs RetroArch and the gambatte core: `cargo test -- --ignored retroarch`.
    #[test]
    #[ignore]
    fn the_real_retroarch_runs_a_rom_with_lutras_config() {
        let core = Path::new("/usr/lib/libretro/gambatte_libretro.so");
        let Some(retroarch) = crate::wine::find_in_path("retroarch").filter(|_| core.exists())
        else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let layout = Layout::new(dir.path());
        for folder in [
            layout.saves(),
            layout.states(),
            layout.screenshots(),
            layout.options(),
        ] {
            std::fs::create_dir_all(folder).unwrap();
        }
        // 32 KiB ROM: `nop; jp 0x150` at the entry point, an endless `jr -2` there.
        let mut rom = vec![0u8; 0x8000];
        rom[0x100..0x104].copy_from_slice(&[0x00, 0xC3, 0x50, 0x01]);
        rom[0x150..0x152].copy_from_slice(&[0x18, 0xFE]);
        rom[0x134..0x139].copy_from_slice(b"LUTRA");
        let checksum = rom[0x134..0x14D]
            .iter()
            .fold(0u8, |acc, byte| acc.wrapping_sub(*byte).wrapping_sub(1));
        rom[0x14D] = checksum;
        let rom_path = dir.path().join("Lutra Test.gb");
        std::fs::write(&rom_path, rom).unwrap();

        let system = dir.path().join("system");
        std::fs::create_dir_all(&system).unwrap();
        let mut main = main_config(
            &layout,
            &system,
            &[Path::new("/usr/lib/libretro")],
            None,
            None,
        );
        // No window and no sound for the test.
        main.push_str("video_driver = \"null\"\naudio_driver = \"null\"\ninput_driver = \"null\"\ninput_joypad_driver = \"null\"\n");
        std::fs::write(layout.config(), main).unwrap();
        let options = layout.core_options("Gambatte", "Lutra Test");
        std::fs::create_dir_all(options.parent().unwrap()).unwrap();
        std::fs::write(
            &options,
            core_options_text(&BTreeMap::from([(
                "gambatte_gb_colorization".into(),
                "disabled".into(),
            )])),
        )
        .unwrap();
        std::fs::write(
            layout.game_config("test"),
            game_config(&EmulatorConfig::default()),
        )
        .unwrap();

        let spec = build_launch(
            &retroarch,
            core,
            &rom_path,
            &layout.config(),
            &layout.game_config("test"),
        );
        let output = std::process::Command::new(&spec.program)
            .args(&spec.args)
            .arg("--max-frames=120")
            .current_dir(&spec.cwd)
            .output()
            .unwrap();
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!("{log}");
        assert!(output.status.success(), "RetroArch failed:\n{log}");
        assert!(log.contains("Gambatte"), "the core was not loaded:\n{log}");
        assert!(
            log.contains("Lutra Test.gb"),
            "the game was not loaded:\n{log}"
        );
        // Lutra's own options file is the one the core reads, and nothing lands outside Lutra's folder.
        assert!(
            log.contains("emulation/config/Gambatte"),
            "the options file was not used:\n{log}"
        );
        assert!(
            !log.contains(".config/retroarch"),
            "RetroArch touched the user's config:\n{log}"
        );
    }
}
