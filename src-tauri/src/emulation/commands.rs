//! The emulation commands the interface calls, and the launch of an emulated game.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use super::bios::{self, FirmwareStatus};
use super::cores::{self, InstalledCore, Locations, Origin, RemoteCore};
use super::profile::EmulationProfile;
use super::retroarch::{self, EmulatorConfig, Layout};
use super::{icons, roms, systems};
use crate::db::{Db, EmulatorGame, EmulatorSettings, Game, GameMeta, stable_hash};
use crate::wine::LaunchSpec;
use crate::{ControllerService, Library, Running, applog, dir_name};

const ROM_DIRS: &str = "rom_dirs";
const BIOS_DIR: &str = "bios_dir";
/// A console's settings: `emulation_profile:<libretro database name>`.
const PROFILE_PREFIX: &str = "emulation_profile:";
/// The interface's language, which translated cores speak too.
pub const UI_LANGUAGE: &str = "ui_language";

fn layout(app: &AppHandle) -> Result<Layout, String> {
    Ok(Layout::new(&crate::app_dir(app)?))
}

fn locations(app: &AppHandle) -> Result<Locations, String> {
    Ok(Locations::system_default(layout(app)?.cores()))
}

fn rom_dirs(db: &Db) -> Vec<PathBuf> {
    db.setting(ROM_DIRS)
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok())
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

/// The settings of a console's games (defaults until the user changes them).
fn console_profile(db: &Db, system: &str) -> EmulationProfile {
    db.setting(&format!("{PROFILE_PREFIX}{system}"))
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn ui_language(db: &Db) -> String {
    db.setting(UI_LANGUAGE)
        .ok()
        .flatten()
        .unwrap_or_else(|| "en".into())
}

/// The folder BIOS files are looked for in: the user's choice, else Lutra's own.
fn bios_dir(db: &Db, layout: &Layout) -> PathBuf {
    db.setting(BIOS_DIR)
        .ok()
        .flatten()
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| layout.default_system())
}

/// The BIOS folder of the user's own RetroArch, offered as a suggestion (never read as config).
fn retroarch_system_dir(home: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(home.join(".config/retroarch/retroarch.cfg")).ok()?;
    let value = text
        .lines()
        .find_map(|line| line.strip_prefix("system_directory"))?
        .split_once('=')?
        .1
        .trim()
        .trim_matches('"')
        .to_string();
    let path = match value.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(value),
    };
    path.is_dir().then_some(path)
}

fn retroarch_version(binary: &Path) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.trim().strip_prefix("Version:"))
        .map(|version| version.trim().to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreView {
    id: String,
    display_name: String,
    version: Option<String>,
    origin: Origin,
    systems: Vec<String>,
    extensions: Vec<String>,
    firmware: Vec<FirmwareStatus>,
    /// Every required firmware is in place.
    ready: bool,
    hw_render: bool,
}

fn core_view(core: &InstalledCore, bios: &Path) -> CoreView {
    let firmware = bios::status(&core.info, bios);
    CoreView {
        id: core.info.id.clone(),
        display_name: core.info.display_name.clone(),
        version: core.info.version.clone(),
        origin: core.origin,
        systems: core.info.databases.clone(),
        extensions: core.info.extensions.clone(),
        ready: bios::ready(&firmware),
        firmware,
        hw_render: core.info.hw_render,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetroArchView {
    path: String,
    version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulationStatus {
    retroarch: Option<RetroArchView>,
    cores: Vec<CoreView>,
    systems: Vec<systems::System>,
    rom_dirs: Vec<String>,
    bios_dir: String,
    /// Where BIOS files go when no folder is chosen.
    default_bios_dir: String,
    /// The BIOS folder of the user's own RetroArch, if it exists and is not the one in use.
    suggested_bios_dir: Option<String>,
}

#[tauri::command]
pub async fn emulation_status(
    app: AppHandle,
    library: State<'_, Library>,
) -> Result<EmulationStatus, String> {
    let layout = layout(&app)?;
    let (dirs, bios) = {
        let db = library.lock()?;
        (rom_dirs(&db), bios_dir(&db, &layout))
    };
    let locations = locations(&app)?;
    let home = crate::home_dir()?;
    tauri::async_runtime::spawn_blocking(move || {
        let installed = cores::inventory(&locations);
        let retroarch = crate::wine::find_in_path("retroarch").map(|path| RetroArchView {
            version: retroarch_version(&path),
            path: path.to_string_lossy().into_owned(),
        });
        EmulationStatus {
            retroarch,
            cores: installed
                .iter()
                .map(|core| core_view(core, &bios))
                .collect(),
            systems: systems::playable(&installed),
            rom_dirs: dirs
                .iter()
                .map(|dir| dir.to_string_lossy().into_owned())
                .collect(),
            bios_dir: bios.to_string_lossy().into_owned(),
            default_bios_dir: layout.default_system().to_string_lossy().into_owned(),
            suggested_bios_dir: retroarch_system_dir(&home)
                .filter(|dir| *dir != bios)
                .map(|dir| dir.to_string_lossy().into_owned()),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Saves the folders to scan for ROMs and the BIOS folder (`None` goes back to Lutra's own).
#[tauri::command]
pub async fn set_emulation_dirs(
    library: State<'_, Library>,
    rom_dirs: Vec<String>,
    bios_dir: Option<String>,
) -> Result<(), String> {
    for dir in &rom_dirs {
        if !Path::new(dir).is_dir() {
            return Err(format!("not a folder: {dir}"));
        }
    }
    if let Some(dir) = bios_dir.as_deref().filter(|dir| !dir.trim().is_empty())
        && !Path::new(dir).is_dir()
    {
        return Err(format!("not a folder: {dir}"));
    }
    let db = library.lock()?;
    let mut unique = rom_dirs;
    unique.dedup();
    db.set_setting(
        ROM_DIRS,
        &serde_json::to_string(&unique).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    match bios_dir.filter(|dir| !dir.trim().is_empty()) {
        Some(dir) => db.set_setting(BIOS_DIR, &dir),
        None => db.delete_setting(BIOS_DIR),
    }
    .map_err(|e| e.to_string())?;
    applog::info("Emulation folders saved");
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    added: usize,
    removed: usize,
    total: usize,
    /// Disc images in folders that do not say which system they are for.
    unplaced: usize,
    /// Games whose system has no installed core.
    without_core: usize,
}

/// Looks through the ROM folders and makes the library match.
#[tauri::command]
pub async fn scan_roms(app: AppHandle, library: State<'_, Library>) -> Result<ScanSummary, String> {
    let dirs = rom_dirs(&*library.lock()?);
    let installed = cores::inventory(&locations(&app)?);
    let scan_dirs = dirs.clone();
    let scan = tauri::async_runtime::spawn_blocking(move || roms::scan(&scan_dirs))
        .await
        .map_err(|e| e.to_string())?;

    let mut without_core = 0;
    let games: Vec<EmulatorGame> = scan
        .roms
        .iter()
        .map(|rom| {
            let extension = rom
                .path
                .extension()
                .map(|ext| ext.to_string_lossy().into_owned())
                .unwrap_or_default();
            if systems::eligible_cores(&installed, &rom.system, &extension).is_empty() {
                without_core += 1;
            }
            let path = rom.path.to_string_lossy().into_owned();
            EmulatorGame {
                id: format!("emulator:{:08x}", stable_hash(&path)),
                title: rom.title.clone(),
                source_id: format!("{}|{}", rom.system, rom.stem),
                rom_path: path,
                config: EmulatorConfig {
                    system: rom.system.clone(),
                    ..EmulatorConfig::default()
                },
            }
        })
        .collect();

    let (added, removed) = library
        .lock()?
        .sync_emulator(&games, &dirs)
        .map_err(|e| e.to_string())?;
    applog::info(format!(
        "ROM scan: {} game(s), {added} new, {removed} removed, {} placed nowhere",
        games.len(),
        scan.unplaced
    ));
    crate::emit_library(&app);
    crate::start_import(&app, false);
    ensure_cores(
        &app,
        games
            .iter()
            .map(|game| game.config.system.clone())
            .collect(),
    );
    Ok(ScanSummary {
        added,
        removed,
        total: games.len(),
        unplaced: scan.unplaced,
        without_core,
    })
}

/// Downloads a core (or its newer build) from libretro's build server and checks it loads.
fn install_now(locations: &Locations, core_id: &str) -> Result<cores::Probe, String> {
    let agent = crate::artwork::agent();
    let remote = cores::fetch_index(&agent)?
        .into_iter()
        .find(|core| core.id == core_id)
        .ok_or_else(|| format!("{core_id} is not offered by the libretro build server"))?;
    let probe = cores::install(&agent, locations, &remote)?;
    // Cores the system does not describe get their `.info` from the build server.
    if !locations
        .system_info
        .join(format!("{core_id}_libretro.info"))
        .is_file()
    {
        let _ = cores::install_infos(&agent, locations);
    }
    applog::info(format!(
        "Installed the {} core {}",
        probe.name, probe.version
    ));
    Ok(probe)
}

/// Installs, in the background, the emulator Lutra picks for each of `systems` that has none,
/// and updates the ones Lutra installed. The user never chooses or downloads an emulator:
/// adding games is enough.
fn ensure_cores(app: &AppHandle, systems: BTreeSet<String>) {
    let Ok(locations) = locations(app) else {
        return;
    };
    std::thread::spawn(move || {
        let installed = cores::inventory(&locations);
        // The build server's list, read once and only if a core installed by Lutra is in use.
        let mut remote: Option<Vec<RemoteCore>> = None;
        for system in systems {
            let core = match systems::cores_for(&installed, &system).first() {
                // The emulator in use, when Lutra installed it and a newer build exists.
                Some(used) if used.origin == Origin::Lutra => {
                    let index = remote.get_or_insert_with(|| {
                        cores::fetch_index(&crate::artwork::agent()).unwrap_or_default()
                    });
                    if !cores::has_update(used, index) {
                        continue;
                    }
                    used.info.id.clone()
                }
                Some(_) => continue,
                None => match systems::recommended(&system) {
                    Some(core) => core.to_string(),
                    None => continue,
                },
            };
            if let Err(error) = install_now(&locations, &core) {
                applog::warn(format!(
                    "Could not install the emulator of {system}: {error}"
                ));
            }
        }
    });
}

/// At start: every console of the library gets its emulator, up to date.
pub fn ensure_library_cores(app: &AppHandle) {
    let systems = app
        .state::<Library>()
        .lock()
        .and_then(|db| db.list().map_err(|e| e.to_string()))
        .map(|games| games.into_iter().filter_map(|game| game.system).collect())
        .unwrap_or_default();
    ensure_cores(app, systems);
}

/// Installs the emulator Lutra picks for a system now (from the system's page).
#[tauri::command]
pub async fn install_core(app: AppHandle, core_id: String) -> Result<cores::Probe, String> {
    let locations = locations(&app)?;
    tauri::async_runtime::spawn_blocking(move || install_now(&locations, &core_id))
        .await
        .map_err(|e| e.to_string())?
}

/// A core the user can pick for a game.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreChoice {
    id: String,
    name: String,
    origin: Origin,
    /// Its required firmware is in place.
    ready: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorSettingsView {
    settings: EmulatorSettings,
    /// What the game uses when it has no settings of its own.
    console_profile: EmulationProfile,
    /// Installed cores that can play this game, the preferred one first.
    cores: Vec<CoreChoice>,
}

fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

#[tauri::command]
pub async fn emulator_settings(
    app: AppHandle,
    library: State<'_, Library>,
    id: String,
) -> Result<EmulatorSettingsView, String> {
    let layout = layout(&app)?;
    let (settings, bios, console_profile) = {
        let db = library.lock()?;
        let settings = db
            .emulator_settings(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("not an emulated game: {id}"))?;
        let console_profile = console_profile(&db, &settings.config.system);
        (settings, bios_dir(&db, &layout), console_profile)
    };
    let installed = cores::inventory(&locations(&app)?);
    let choices = systems::eligible_cores(
        &installed,
        &settings.config.system,
        &extension_of(&settings.rom_path),
    )
    .into_iter()
    .map(|core| CoreChoice {
        id: core.info.id.clone(),
        name: core.info.display_name.clone(),
        origin: core.origin,
        ready: bios::ready(&bios::status(&core.info, &bios)),
    })
    .collect();
    Ok(EmulatorSettingsView {
        settings,
        console_profile,
        cores: choices,
    })
}

#[tauri::command]
pub async fn save_emulator_settings(
    library: State<'_, Library>,
    settings: EmulatorSettings,
) -> Result<Game, String> {
    let title = settings.title.trim().to_string();
    if title.is_empty() {
        return Err("the game needs a name".into());
    }
    let db = library.lock()?;
    db.save_emulator_settings(&EmulatorSettings {
        title,
        ..settings.clone()
    })
    .map_err(|e| e.to_string())?;
    applog::info(format!("Emulator settings saved for {}", settings.title));
    db.get(&settings.id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "game not found".to_string())
}

/// What the emulation dashboard shows about one system.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemView {
    /// libretro database name.
    name: String,
    /// Installed cores for it, in the order Lutra tries them: the first is the one used.
    cores: Vec<CoreChoice>,
    /// The core Lutra installs when none is.
    recommended_core: Option<String>,
    /// BIOS files of the core in use.
    firmware: Vec<FirmwareStatus>,
    games: usize,
    /// File extensions the installed cores accept for it.
    extensions: Vec<String>,
    /// Picture of the console, from RetroArch's assets.
    icon: Option<String>,
    /// Its emulator draws with the GPU: it still runs in RetroArch, not in Lutra's player.
    needs_retroarch: bool,
}

/// Every system worth showing: those Lutra knows a good core for, those an installed core
/// handles, and those games in the library belong to.
#[tauri::command]
pub async fn emulation_systems(
    app: AppHandle,
    library: State<'_, Library>,
) -> Result<Vec<SystemView>, String> {
    let layout = layout(&app)?;
    let (bios, games) = {
        let db = library.lock()?;
        let games = db
            .list()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter_map(|game| game.system)
            .collect::<Vec<_>>();
        (bios_dir(&db, &layout), games)
    };
    let locations = locations(&app)?;
    let icon_roots = icons::roots(&crate::home_dir()?);
    tauri::async_runtime::spawn_blocking(move || {
        let installed = cores::inventory(&locations);
        let mut names: BTreeSet<String> = systems::known().map(str::to_string).collect();
        names.extend(
            installed
                .iter()
                .flat_map(|core| core.info.databases.iter().cloned()),
        );
        names.extend(games.iter().cloned());
        names
            .into_iter()
            .map(|name| {
                let handling = systems::cores_for(&installed, &name);
                let extensions: BTreeSet<String> = handling
                    .iter()
                    .flat_map(|core| core.info.extensions.iter().cloned())
                    .collect();
                SystemView {
                    firmware: handling
                        .first()
                        .map(|core| bios::status(&core.info, &bios))
                        .unwrap_or_default(),
                    cores: handling
                        .iter()
                        .map(|core| CoreChoice {
                            id: core.info.id.clone(),
                            name: core.info.display_name.clone(),
                            origin: core.origin,
                            ready: bios::ready(&bios::status(&core.info, &bios)),
                        })
                        .collect(),
                    recommended_core: systems::recommended(&name).map(str::to_string),
                    games: games.iter().filter(|system| **system == name).count(),
                    extensions: extensions.into_iter().collect(),
                    icon: icons::system_icon(&icon_roots, &name)
                        .map(|path| path.to_string_lossy().into_owned()),
                    needs_retroarch: handling.first().is_some_and(|core| core.info.hw_render),
                    name,
                }
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

/// A ROM the user adds by hand.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRom {
    path: String,
    system: String,
    title: String,
    meta: Option<GameMeta>,
}

#[tauri::command]
pub async fn add_rom(
    app: AppHandle,
    library: State<'_, Library>,
    rom: NewRom,
) -> Result<Game, String> {
    let path = Path::new(&rom.path);
    if !path.is_file() {
        return Err(format!("not a file: {}", rom.path));
    }
    if rom.system.trim().is_empty() {
        return Err("choose the game's system".into());
    }
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let title = match rom.title.trim() {
        "" => roms::clean_title(&stem),
        title => title.to_string(),
    };
    let game = EmulatorGame {
        // The same id a scan would give, so the game is not added twice.
        id: format!("emulator:{:08x}", stable_hash(&rom.path)),
        source_id: format!("{}|{stem}", rom.system),
        title: title.clone(),
        rom_path: rom.path.clone(),
        config: EmulatorConfig {
            system: rom.system.clone(),
            ..EmulatorConfig::default()
        },
    };
    let added = {
        let db = library.lock()?;
        if db.get(&game.id).map_err(|e| e.to_string())?.is_some() {
            return Err("this game is already in the library".into());
        }
        db.add_emulator(&game, rom.meta.as_ref())
            .map_err(|e| e.to_string())?;
        db.get(&game.id)
            .map_err(|e| e.to_string())?
            .ok_or("game not found")?
    };
    applog::info(format!("Added {title} ({})", rom.system));
    crate::emit_library(&app);
    crate::start_import(&app, false);
    ensure_cores(&app, BTreeSet::from([rom.system]));
    Ok(added)
}

/// The settings a console's games share.
#[tauri::command]
pub async fn emulation_profile(
    library: State<'_, Library>,
    system: String,
) -> Result<EmulationProfile, String> {
    Ok(console_profile(&*library.lock()?, &system))
}

#[tauri::command]
pub async fn save_emulation_profile(
    library: State<'_, Library>,
    system: String,
    profile: EmulationProfile,
) -> Result<(), String> {
    library
        .lock()?
        .set_setting(
            &format!("{PROFILE_PREFIX}{system}"),
            &serde_json::to_string(&profile).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    applog::info(format!("Emulation settings saved for {system}"));
    Ok(())
}

/// The options the emulator of a console offers, as it describes them (in the interface's
/// language when it is translated). Read by a separate Lutra process: loading a core inside
/// the library could take it down with the core.
#[tauri::command]
pub async fn core_options(
    app: AppHandle,
    library: State<'_, Library>,
    system: String,
) -> Result<crate::player::CoreOptions, String> {
    let language = ui_language(&*library.lock()?);
    let installed = cores::inventory(&locations(&app)?);
    let core = systems::cores_for(&installed, &system)
        .first()
        .map(|core| core.path.clone())
        .ok_or("no emulator is installed for this console yet")?;
    tauri::async_runtime::spawn_blocking(move || {
        let program = std::env::current_exe().map_err(|e| e.to_string())?;
        let output = Command::new(program)
            .arg("--core-options")
            .arg(&core)
            .arg(&language)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "the emulator did not describe its options: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        // Cores may print their own messages: the description is the last line.
        let text = String::from_utf8_lossy(&output.stdout);
        let json = text.lines().last().unwrap_or_default();
        serde_json::from_str(json).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Opens one of Lutra's emulation folders: `bios`, `saves`, `states` or `cores`.
#[tauri::command]
pub async fn open_emulation_folder(
    app: AppHandle,
    library: State<'_, Library>,
    what: String,
) -> Result<(), String> {
    let layout = layout(&app)?;
    let folder = match what.as_str() {
        "bios" => bios_dir(&*library.lock()?, &layout),
        "saves" => layout.saves(),
        "states" => layout.states(),
        "cores" => layout.cores(),
        other => return Err(format!("unknown folder: {other}")),
    };
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    tauri_plugin_opener::open_path(folder, None::<&str>).map_err(|e| e.to_string())
}

/// The command that runs a game in Lutra's own player (`lutra --play`), and the program it runs.
#[allow(clippy::too_many_arguments)]
fn player_launch(
    layout: &Layout,
    core: &InstalledCore,
    bios: &Path,
    rom: &Path,
    settings: &EmulatorSettings,
    profile: EmulationProfile,
    language: String,
    key: &str,
) -> Result<(LaunchSpec, PathBuf), String> {
    let config = crate::player::PlayConfig {
        core: core.path.clone(),
        rom: rom.to_path_buf(),
        title: settings.title.clone(),
        system_dir: bios.to_path_buf(),
        save_dir: layout.saves(),
        state_dir: layout.states(),
        fullscreen: settings.config.fullscreen,
        auto_resume: settings.config.auto_resume,
        profile,
        language,
    };
    let file = layout.root.join("play").join(format!("{key}.json"));
    fs::create_dir_all(layout.root.join("play")).map_err(|e| e.to_string())?;
    fs::write(
        &file,
        serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let program = std::env::current_exe().map_err(|e| e.to_string())?;
    let spec = LaunchSpec {
        program: program.clone(),
        args: vec!["--play".into(), file.into()],
        envs: Vec::new(),
        cwd: rom.parent().unwrap_or(Path::new("/")).to_path_buf(),
    };
    Ok((spec, program))
}

/// The command that runs a game in RetroArch with Lutra's configuration, and RetroArch's path.
#[allow(clippy::too_many_arguments)]
fn retroarch_launch(
    layout: &Layout,
    locations: &Locations,
    core: &InstalledCore,
    bios: &Path,
    rom: &Path,
    settings: &EmulatorSettings,
    profile: &EmulationProfile,
    key: &str,
) -> Result<(LaunchSpec, PathBuf), String> {
    let retroarch = crate::wine::find_in_path("retroarch").ok_or_else(|| {
        format!(
            "{} draws with the GPU: Lutra still needs RetroArch to run it",
            core.info.display_name
        )
    })?;
    // The core reads this game's options from where Lutra put them, not from RetroArch's own folder.
    let rom_stem = rom
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let options_file = layout.core_options(&core.info.core_name, &rom_stem);
    if !profile.core_options.is_empty() {
        if let Some(parent) = options_file.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(
            &options_file,
            retroarch::core_options_text(&profile.core_options),
        )
        .map_err(|e| e.to_string())?;
    }
    let info_dir = [locations.system_info.clone(), locations.lutra.join("info")]
        .into_iter()
        .find(|dir| dir.is_dir());
    let autoconfig = Path::new("/usr/share/libretro/autoconfig");
    fs::write(
        layout.config(),
        retroarch::main_config(
            layout,
            bios,
            &[core.path.parent().unwrap_or(Path::new("/usr/lib/libretro"))],
            info_dir.as_deref(),
            autoconfig.is_dir().then_some(autoconfig),
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        layout.game_config(key),
        retroarch::game_config(&settings.config),
    )
    .map_err(|e| e.to_string())?;

    let spec = retroarch::build_launch(
        &retroarch,
        &core.path,
        rom,
        &layout.config(),
        &layout.game_config(key),
    );
    Ok((spec, retroarch))
}

/// Starts an emulated game: picks the core, checks its firmware, writes Lutra's RetroArch
/// config for it, and runs RetroArch on it.
pub fn launch(app: &AppHandle, library: &Library, id: &str) -> Result<(), String> {
    if app
        .state::<Running>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .contains_key(id)
    {
        return Err("this game is already running".into());
    }
    let layout = layout(app)?;
    let (settings, bios, profile, language) = {
        let db = library.lock()?;
        let settings = db
            .emulator_settings(id)
            .map_err(|e| e.to_string())?
            .ok_or("this game is missing its settings")?;
        // The game's own settings, else its console's.
        let profile = settings
            .config
            .profile
            .clone()
            .unwrap_or_else(|| console_profile(&db, &settings.config.system));
        (settings, bios_dir(&db, &layout), profile, ui_language(&db))
    };
    let rom = PathBuf::from(&settings.rom_path);
    if !rom.is_file() {
        return Err(format!("the game's file is gone: {}", rom.display()));
    }
    let locations = locations(app)?;
    let system = settings.config.system.as_str();
    let extension = extension_of(&settings.rom_path);
    let mut installed = cores::inventory(&locations);
    // Lutra picks the emulator, and installs it the first time a game of this console starts.
    if systems::eligible_cores(&installed, system, &extension).is_empty() {
        let recommended = systems::recommended(system)
            .ok_or_else(|| format!("Lutra has no emulator for {system} yet"))?;
        crate::emit_progress(app, id, "core", 0, 3, crate::LaunchState::Progress, None);
        install_now(&locations, recommended)?;
        installed = cores::inventory(&locations);
    }
    let core = systems::eligible_cores(&installed, system, &extension)
        .first()
        .copied()
        .ok_or_else(|| format!("no emulator can open this file for {system}"))?;

    let firmware = bios::status(&core.info, &bios);
    if !bios::ready(&firmware) {
        let missing: Vec<&str> = firmware
            .iter()
            .filter(|file| !file.optional && file.state == bios::State::Missing)
            .map(|file| file.path.as_str())
            .collect();
        return Err(format!(
            "{} needs these BIOS files in {}: {}",
            core.info.display_name,
            bios.display(),
            missing.join(", ")
        ));
    }

    for dir in [
        layout.saves(),
        layout.states(),
        layout.screenshots(),
        layout.options(),
        layout.root.join("history"),
        layout.root.join("logs"),
        bios.clone(),
    ] {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let key = dir_name(id);
    let (mut spec, host) = if core.info.hw_render {
        // Cores that draw with the GPU still go through RetroArch, until Lutra's player can
        // lend them its OpenGL context.
        retroarch_launch(
            &layout, &locations, core, &bios, &rom, &settings, &profile, &key,
        )?
    } else {
        player_launch(
            &layout, core, &bios, &rom, &settings, profile, language, &key,
        )?
    };
    applog::info(format!(
        "Launching {} ({id}) with the {} core",
        settings.title, core.info.display_name
    ));
    // The same GPU as any other game: the dedicated one, the only one Vulkan shows.
    let hardware = crate::hardware_now();
    if let Some(gpu) = hardware.preferred_gpu() {
        for (key, value) in crate::hardware::gpu_env(&hardware, gpu) {
            spec.envs.push((key, value.into()));
        }
    }
    crate::take_controllers(app, id, &mut spec);
    let cwd = rom.parent().unwrap_or(Path::new("/")).to_path_buf();
    let child = crate::spawn_process(app, &key, &cwd, &spec).inspect_err(|error| {
        applog::error(format!("Could not launch {}: {error}", settings.title));
        app.state::<ControllerService>().end_session(id);
    })?;
    library
        .lock()?
        .mark_played(id, crate::now())
        .map_err(|e| e.to_string())?;
    let (pgid, exit) = crate::track_game(app, id, child);
    crate::watch_launch(
        app.clone(),
        crate::LaunchWatch {
            id: id.to_string(),
            pgid,
            exe: host.to_string_lossy().into_owned(),
            log: crate::app_dir(app)?.join("logs").join(format!("{key}.log")),
            prefix: cwd,
            proton: false,
            new_prefix: false,
            // The player (or RetroArch) is the program itself: surviving its first moments is
            // the signal.
            native: true,
            exit,
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_users_retroarch_bios_folder_is_found_without_reading_anything_else() {
        let home = tempfile::tempdir().unwrap();
        fs::create_dir_all(home.path().join(".config/retroarch")).unwrap();
        fs::create_dir_all(home.path().join("Games/bios")).unwrap();
        fs::write(
            home.path().join(".config/retroarch/retroarch.cfg"),
            "audio_driver = \"pulse\"\nsystem_directory = \"~/Games/bios\"\n",
        )
        .unwrap();
        assert_eq!(
            retroarch_system_dir(home.path()),
            Some(home.path().join("Games/bios"))
        );

        // A folder that does not exist is not suggested.
        fs::write(
            home.path().join(".config/retroarch/retroarch.cfg"),
            "system_directory = \"/nowhere/at/all\"\n",
        )
        .unwrap();
        assert_eq!(retroarch_system_dir(home.path()), None);
        assert_eq!(retroarch_system_dir(Path::new("/nonexistent")), None);
    }

    #[test]
    fn the_bios_folder_defaults_to_lutras_own_until_the_user_picks_one() {
        let db = Db::open_in_memory().unwrap();
        let layout = Layout::new(Path::new("/data/lutra"));
        assert_eq!(
            bios_dir(&db, &layout),
            PathBuf::from("/data/lutra/emulation/system")
        );
        db.set_setting(BIOS_DIR, "/home/me/bios").unwrap();
        assert_eq!(bios_dir(&db, &layout), PathBuf::from("/home/me/bios"));
        db.set_setting(BIOS_DIR, "  ").unwrap();
        assert_eq!(
            bios_dir(&db, &layout),
            PathBuf::from("/data/lutra/emulation/system")
        );
    }

    #[test]
    fn rom_folders_round_trip_through_the_settings() {
        let db = Db::open_in_memory().unwrap();
        assert!(rom_dirs(&db).is_empty());
        db.set_setting(ROM_DIRS, r#"["/roms","/more"]"#).unwrap();
        assert_eq!(
            rom_dirs(&db),
            vec![PathBuf::from("/roms"), PathBuf::from("/more")]
        );
    }
}
