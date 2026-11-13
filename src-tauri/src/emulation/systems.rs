//! Systems and which core plays what. The cores' `.info` files say which systems and file
//! extensions each handles; the few tables here only cover what they cannot: which system a
//! file extension means, and which core to prefer.

use serde::Serialize;

use super::cores::InstalledCore;

/// A console or computer, named like libretro's database (and its thumbnail folder).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct System {
    pub name: String,
    /// Ids of the installed cores that handle this system, the preferred one first.
    pub cores: Vec<String>,
}

/// Extensions that mean one system only.
const EXTENSION_SYSTEMS: &[(&str, &str)] = &[
    ("nes", "Nintendo - Nintendo Entertainment System"),
    ("fds", "Nintendo - Family Computer Disk System"),
    ("smc", "Nintendo - Super Nintendo Entertainment System"),
    ("sfc", "Nintendo - Super Nintendo Entertainment System"),
    ("swc", "Nintendo - Super Nintendo Entertainment System"),
    ("fig", "Nintendo - Super Nintendo Entertainment System"),
    ("gb", "Nintendo - Game Boy"),
    ("gbc", "Nintendo - Game Boy Color"),
    ("gba", "Nintendo - Game Boy Advance"),
    ("nds", "Nintendo - Nintendo DS"),
    ("z64", "Nintendo - Nintendo 64"),
    ("n64", "Nintendo - Nintendo 64"),
    ("v64", "Nintendo - Nintendo 64"),
    ("vb", "Nintendo - Virtual Boy"),
    ("md", "Sega - Mega Drive - Genesis"),
    ("gen", "Sega - Mega Drive - Genesis"),
    ("smd", "Sega - Mega Drive - Genesis"),
    ("32x", "Sega - 32X"),
    ("sms", "Sega - Master System - Mark III"),
    ("gg", "Sega - Game Gear"),
    ("sg", "Sega - SG-1000"),
    ("pce", "NEC - PC Engine - TurboGrafx 16"),
    ("sgx", "NEC - PC Engine SuperGrafx"),
    ("a26", "Atari - 2600"),
    ("a78", "Atari - 7800"),
    ("lnx", "Atari - Lynx"),
    ("ngp", "SNK - Neo Geo Pocket"),
    ("ngc", "SNK - Neo Geo Pocket Color"),
    ("ws", "Bandai - WonderSwan"),
    ("wsc", "Bandai - WonderSwan Color"),
    ("cso", "Sony - PlayStation Portable"),
    ("pbp", "Sony - PlayStation Portable"),
    ("gdi", "Sega - Dreamcast"),
    ("cdi", "Sega - Dreamcast"),
    ("rvz", "Nintendo - GameCube"),
    ("gcz", "Nintendo - GameCube"),
    ("gcm", "Nintendo - GameCube"),
    ("wbfs", "Nintendo - Wii"),
];

/// Names of folders that tell which system the ROMs inside belong to. Needed for formats that
/// several systems share (`iso`, `cue`, `chd`, `bin`, `zip`).
const FOLDER_SYSTEMS: &[(&[&str], &str)] = &[
    (
        &["psx", "ps1", "playstation", "psone"],
        "Sony - PlayStation",
    ),
    (
        &["ps2", "playstation2", "playstation 2"],
        "Sony - PlayStation 2",
    ),
    (&["psp"], "Sony - PlayStation Portable"),
    (&["saturn"], "Sega - Saturn"),
    (
        &["segacd", "megacd", "mega-cd", "mega cd", "sega cd"],
        "Sega - Mega-CD - Sega CD",
    ),
    (&["dreamcast", "dc"], "Sega - Dreamcast"),
    (
        &["megadrive", "mega drive", "genesis", "md"],
        "Sega - Mega Drive - Genesis",
    ),
    (
        &["pcecd", "turbocd", "tg-cd", "pce-cd"],
        "NEC - PC Engine CD - TurboGrafx-CD",
    ),
    (
        &["pcengine", "pc engine", "tg16", "turbografx"],
        "NEC - PC Engine - TurboGrafx 16",
    ),
    (&["gamecube", "gc"], "Nintendo - GameCube"),
    (&["wii"], "Nintendo - Wii"),
    (&["3do"], "The 3DO Company - 3DO"),
    (&["arcade", "fbneo", "fba", "mame"], "FBNeo - Arcade Games"),
    (&["neogeo", "neo geo"], "SNK - Neo Geo"),
    (
        &["nes", "famicom"],
        "Nintendo - Nintendo Entertainment System",
    ),
    (
        &["snes", "sfc", "supernes", "super nintendo"],
        "Nintendo - Super Nintendo Entertainment System",
    ),
    (&["n64"], "Nintendo - Nintendo 64"),
    (&["gb", "gameboy", "game boy"], "Nintendo - Game Boy"),
    (&["gbc"], "Nintendo - Game Boy Color"),
    (&["gba"], "Nintendo - Game Boy Advance"),
    (&["nds", "ds"], "Nintendo - Nintendo DS"),
];

/// The system an unambiguous extension means.
pub fn system_for_extension(extension: &str) -> Option<&'static str> {
    let extension = extension.to_lowercase();
    EXTENSION_SYSTEMS
        .iter()
        .find(|(ext, _)| *ext == extension)
        .map(|(_, system)| *system)
}

/// The system a folder name stands for.
pub fn system_for_folder(folder: &str) -> Option<&'static str> {
    let folder = folder.trim().to_lowercase();
    FOLDER_SYSTEMS
        .iter()
        .find(|(names, _)| names.contains(&folder.as_str()))
        .map(|(_, system)| *system)
}

/// Cores in the order to try them for a system.
const PREFERRED: &[(&str, &[&str])] = &[
    (
        "Nintendo - Nintendo Entertainment System",
        &["nestopia", "fceumm", "mesen"],
    ),
    (
        "Nintendo - Super Nintendo Entertainment System",
        &["snes9x", "bsnes"],
    ),
    ("Nintendo - Game Boy", &["gambatte", "mgba"]),
    ("Nintendo - Game Boy Color", &["gambatte", "mgba"]),
    ("Nintendo - Game Boy Advance", &["mgba"]),
    (
        "Nintendo - Nintendo 64",
        &["mupen64plus_next", "parallel_n64"],
    ),
    ("Nintendo - Nintendo DS", &["melonds", "desmume"]),
    (
        "Sega - Mega Drive - Genesis",
        &["genesis_plus_gx", "blastem", "picodrive"],
    ),
    (
        "Sega - Master System - Mark III",
        &["genesis_plus_gx", "picodrive"],
    ),
    ("Sega - Game Gear", &["genesis_plus_gx"]),
    (
        "Sega - Mega-CD - Sega CD",
        &["genesis_plus_gx", "picodrive"],
    ),
    ("Sega - Saturn", &["mednafen_saturn", "kronos", "yabause"]),
    ("Sega - Dreamcast", &["flycast"]),
    (
        "Sony - PlayStation",
        &["swanstation", "pcsx_rearmed", "mednafen_psx"],
    ),
    ("Sony - PlayStation Portable", &["ppsspp"]),
    (
        "NEC - PC Engine - TurboGrafx 16",
        &["mednafen_pce_fast", "mednafen_pce"],
    ),
    (
        "NEC - PC Engine CD - TurboGrafx-CD",
        &["mednafen_pce_fast", "mednafen_pce"],
    ),
    ("Atari - 2600", &["stella"]),
    ("Atari - Lynx", &["handy", "mednafen_lynx"]),
    ("SNK - Neo Geo Pocket Color", &["mednafen_ngp"]),
    ("Bandai - WonderSwan Color", &["mednafen_wswan"]),
    ("FBNeo - Arcade Games", &["fbneo", "mame"]),
    ("Nintendo - GameCube", &["dolphin"]),
    ("Nintendo - Wii", &["dolphin"]),
];

/// Systems Lutra knows a good core for, whether or not one is installed.
pub fn known() -> impl Iterator<Item = &'static str> {
    PREFERRED.iter().map(|(system, _)| *system)
}

/// The core to install for a system that has none: the preferred one.
pub fn recommended(system: &str) -> Option<&'static str> {
    PREFERRED
        .iter()
        .find(|(name, _)| *name == system)
        .and_then(|(_, ids)| ids.first().copied())
}

/// Puts `cores` in the order to try them for `system`: the preferred ones, then the others by
/// name. Lutra decides; the user never picks an emulator.
fn sort_for(cores: &mut [&InstalledCore], system: &str) {
    let order = PREFERRED
        .iter()
        .find(|(name, _)| *name == system)
        .map_or(&[][..], |(_, ids)| *ids);
    cores.sort_by_key(|core| {
        (
            order
                .iter()
                .position(|id| *id == core.info.id)
                .unwrap_or(order.len()),
            core.info.display_name.to_lowercase(),
        )
    });
}

/// Installed cores that handle `system`, in the order to try them.
pub fn cores_for<'a>(cores: &'a [InstalledCore], system: &str) -> Vec<&'a InstalledCore> {
    let mut handling: Vec<&InstalledCore> = cores
        .iter()
        .filter(|core| {
            core.info
                .databases
                .iter()
                .any(|database| database == system)
        })
        .collect();
    sort_for(&mut handling, system);
    handling
}

/// Installed cores able to play a file of `extension` on `system`, in the order to try them.
///
/// A core qualifies when its `.info` lists the system and the extension: a Game Boy core that
/// does not list `gba` is not offered a Game Boy Advance game.
pub fn eligible_cores<'a>(
    cores: &'a [InstalledCore],
    system: &str,
    extension: &str,
) -> Vec<&'a InstalledCore> {
    let extension = extension.to_lowercase();
    cores_for(cores, system)
        .into_iter()
        .filter(|core| core.info.extensions.contains(&extension))
        .collect()
}

/// The systems Lutra can play right now: those at least one installed core lists.
pub fn playable(cores: &[InstalledCore]) -> Vec<System> {
    let mut names: Vec<&str> = cores
        .iter()
        .flat_map(|core| core.info.databases.iter().map(String::as_str))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .map(|name| System {
            name: name.to_string(),
            cores: cores_for(cores, name)
                .iter()
                .map(|core| core.info.id.clone())
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emulation::cores::Origin;
    use crate::emulation::info::parse_core;
    use std::path::PathBuf;

    fn core(id: &str, databases: &str, extensions: &str) -> InstalledCore {
        InstalledCore {
            info: parse_core(
                id,
                &format!(
                    "database = \"{databases}\"\nsupported_extensions = \"{extensions}\"\ndisplay_name = \"{id}\"\n"
                ),
            ),
            path: PathBuf::from(format!("/cores/{id}_libretro.so")),
            origin: Origin::System,
            crc: None,
        }
    }

    #[test]
    fn extensions_and_folders_map_to_systems() {
        assert_eq!(
            system_for_extension("GBA"),
            Some("Nintendo - Game Boy Advance")
        );
        assert_eq!(
            system_for_extension("sfc"),
            Some("Nintendo - Super Nintendo Entertainment System")
        );
        assert_eq!(system_for_extension("iso"), None);
        assert_eq!(system_for_extension("zip"), None);
        assert_eq!(system_for_folder("PSX"), Some("Sony - PlayStation"));
        assert_eq!(
            system_for_folder(" Mega-CD "),
            Some("Sega - Mega-CD - Sega CD")
        );
        assert_eq!(system_for_folder("Games"), None);
    }

    #[test]
    fn a_core_must_list_both_the_system_and_the_extension() {
        let cores = vec![
            core(
                "gambatte",
                "Nintendo - Game Boy|Nintendo - Game Boy Color",
                "gb|gbc|dmg",
            ),
            core(
                "mgba",
                "Nintendo - Game Boy|Nintendo - Game Boy Color|Nintendo - Game Boy Advance",
                "gb|gbc|gba",
            ),
        ];
        let ids = |system, ext| {
            eligible_cores(&cores, system, ext)
                .iter()
                .map(|core| core.info.id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids("Nintendo - Game Boy Advance", "gba"), vec!["mgba"]);
        // Both handle a Game Boy game; Gambatte is the preferred one.
        assert_eq!(ids("Nintendo - Game Boy", "gb"), vec!["gambatte", "mgba"]);
        // A core that does not list the extension is never offered the file.
        assert_eq!(
            ids("Nintendo - Game Boy Color", "gbc"),
            vec!["gambatte", "mgba"]
        );
        assert!(ids("Nintendo - Game Boy Color", "gba").len() == 1);
        assert!(ids("Nintendo - Game Boy Color", "nes").is_empty());
    }

    #[test]
    fn playable_systems_come_from_the_installed_cores_with_the_preferred_first() {
        let cores = vec![
            core(
                "bsnes",
                "Nintendo - Super Nintendo Entertainment System",
                "sfc",
            ),
            core(
                "snes9x",
                "Nintendo - Super Nintendo Entertainment System",
                "sfc",
            ),
            core("flycast", "Sega - Dreamcast", "gdi"),
        ];
        let systems = playable(&cores);
        assert_eq!(systems.len(), 2);
        let snes = systems
            .iter()
            .find(|s| s.name.contains("Super Nintendo"))
            .unwrap();
        assert_eq!(snes.cores, vec!["snes9x", "bsnes"]);
    }
}
