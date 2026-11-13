mod applog;
mod artwork;
mod audio;
mod controller;
mod db;
mod gamedb;
mod hardware;
pub mod host;
mod hyprland;
mod installer;
mod prefixes;
mod rpc;
mod runners;
mod session;
mod steam;
mod steamgriddb;
mod tray;
mod vdf;
mod wine;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use host::{AppHandle, State};
use serde::Serialize;
use serde_json::json;

use artwork::Error as ArtError;
use controller::model::{OutputKind, PadSnapshot, Profile, apply};
use controller::{ControllerInfo, ControllerService};
use db::{Db, Discovered, Game, Image, ImageTarget, ManualGame, WineSettings, stable_hash};
use session::{LaunchProgress, LaunchState};
use steamgriddb::{Candidate, Client, SearchHit};
use wine::{LaunchRequest, LaunchSpec, Runner, RunnerKind, Target, Tools, WineConfig};

const STEAMGRIDDB_KEY: &str = "steamgriddb_key";
const STEAM_ART_SOURCE: &str = "steam_art_source";
const STEAM_ART_FALLBACK: &str = "steam_art_fallback";
const DEFAULT_RUNNER: &str = "default_runner";
const CONTROLLER_NAVIGATION: &str = "controller_navigation";
const CONTROLLER_SYSTEM: &str = "controller_system";
/// Owner of the controller profile used by games without their own, and of test sessions.
const DEFAULT_PROFILE: &str = "default";
const TEST_SESSION: &str = "test";

/// Where Lutra keeps its data, known before Tauri starts (same as `app_data_dir`).
fn data_dir_early() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(base.join("dev.lutra.app"))
}

const OVERLAY_TITLE: &str = "Lutra Overlay";

/// Opens the in-game overlay over the running game, or closes it. The overlay is its own window
/// (made by the shell), so the game keeps running underneath; while it is open the controller
/// drives it, not the game.
fn toggle_overlay(app: &AppHandle) {
    let service = app.state::<ControllerService>();
    if service.overlay_open() {
        service.set_overlay(false);
        let _ = app.emit("host:overlay", json!({ "open": false }));
        return;
    }
    let game_running = app
        .state::<Running>()
        .0
        .lock()
        .is_ok_and(|running| running.keys().any(|id| !id.starts_with("install:")));
    if !game_running {
        return;
    }
    service.set_overlay(true);
    applog::info("Overlay opened");
    let _ = app.emit(
        "host:overlay",
        json!({
            "open": true,
            "title": OVERLAY_TITLE,
            // Hyprland allows a single fullscreen window per workspace: the overlay would take the
            // game's place. There it is placed over the screen instead (`overlay_shown`).
            "fullscreen": !hyprland::available(),
        }),
    );
}

/// The shell (process `pid`) made the overlay window: on Hyprland, float and pin it over the game.
// @rpc
async fn overlay_shown(pid: i64) -> Result<(), String> {
    hyprland::show_overlay(pid, OVERLAY_TITLE);
    Ok(())
}

/// The overlay window is gone (closed by the user, or it could not be made).
// @rpc
async fn overlay_closed(app: AppHandle) -> Result<(), String> {
    app.state::<ControllerService>().set_overlay(false);
    Ok(())
}

/// Closes the overlay; `show_lutra` also brings the Lutra window to the front.
// @rpc
async fn close_overlay(app: AppHandle, show_lutra: bool) -> Result<(), String> {
    app.state::<ControllerService>().set_overlay(false);
    let _ = app.emit("host:overlay", json!({ "open": false }));
    if show_lutra {
        let _ = app.emit("host:show", ());
    }
    Ok(())
}

struct Library(Mutex<Db>);

impl Library {
    fn lock(&self) -> Result<MutexGuard<'_, Db>, String> {
        self.0
            .lock()
            .map_err(|_| "library database is unavailable".to_string())
    }
}

/// Windows games currently running, with the process group to stop.
struct Running(Mutex<HashMap<String, u32>>);

#[derive(Clone, Serialize)]
struct RunningEvent {
    id: String,
    running: bool,
}

/// Guards against two artwork imports running at once.
struct ImportRunning(AtomicBool);

#[derive(Clone, Serialize)]
struct ImportProgress {
    done: usize,
    total: usize,
    running: bool,
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn app_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.data_dir())
}

/// Folder name for a game id (`steam:10` -> `steam_10`).
fn dir_name(id: &str) -> String {
    id.replace(':', "_")
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Tells the frontend the library changed behind its back (artwork arrived, a game ended...).
fn emit_library(app: &AppHandle) {
    let library = app.state::<Library>();
    if let Ok(games) = library
        .lock()
        .and_then(|db| db.list().map_err(|e| e.to_string()))
    {
        let _ = app.emit("library-changed", games);
    }
}

/// Interface mode requested on the command line (`lutra --big-picture`).
// @rpc
fn launch_mode() -> &'static str {
    if std::env::args().any(|arg| arg == "--big-picture") {
        "bigpicture"
    } else {
        "classic"
    }
}

/// Games already known, without touching the disk: used to paint the library instantly.
// @rpc
async fn list_games(library: State<'_, Library>) -> Result<Vec<Game>, String> {
    library.lock()?.list().map_err(|e| e.to_string())
}

/// Looks for installed games in every supported source, then returns the whole library.
// @rpc
async fn scan_sources(library: State<'_, Library>) -> Result<Vec<Game>, String> {
    let home = home_dir()?;

    let steam_games: Vec<Discovered> = steam::scan(&home)
        .into_iter()
        .map(|game| Discovered {
            source: "steam",
            source_id: game.app_id.to_string(),
            title: game.name,
            playtime_minutes: game.playtime_minutes,
            last_played: game.last_played,
        })
        .collect();

    applog::info(format!(
        "Scan: {} Steam game(s) installed",
        steam_games.len()
    ));
    let mut db = library.lock()?;
    db.sync_source("steam", &steam_games)
        .map_err(|e| e.to_string())?;
    db.list().map_err(|e| e.to_string())
}

// @rpc
async fn set_favorite(
    library: State<'_, Library>,
    id: String,
    favorite: bool,
) -> Result<(), String> {
    library
        .lock()?
        .set_favorite(&id, favorite)
        .map_err(|e| e.to_string())
}

/// Wine and Proton installs found on this machine.
// @rpc
async fn list_runners(app: AppHandle) -> Result<Vec<Runner>, String> {
    Ok(wine::detect_runners(
        &steam::steam_roots(&home_dir()?),
        Some(&app.data_dir()),
    ))
}

/// A runner Lutra downloaded, with how many games use it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagedRunner {
    #[serde(flatten)]
    runner: runners::Installed,
    games: usize,
}

/// Runners downloaded by Lutra (in its data folder).
// @rpc
async fn installed_runners(
    app: AppHandle,
    library: State<'_, Library>,
) -> Result<Vec<ManagedRunner>, String> {
    let data = app.data_dir();
    let found = host::async_runtime::spawn_blocking(move || runners::installed(&data))
        .await
        .map_err(|e| e.to_string())?;
    let db = library.lock()?;
    Ok(found
        .into_iter()
        .map(|runner| {
            let games = db
                .games_using_runner(&runner.path.to_string_lossy())
                .unwrap_or(0);
            ManagedRunner { runner, games }
        })
        .collect())
}

/// Recent releases of a runner source (GE-Proton, Wine, Wine Staging), read from GitHub.
// @rpc
async fn available_runners(
    app: AppHandle,
    source: runners::Source,
) -> Result<Vec<runners::Available>, String> {
    let data = app.data_dir();
    host::async_runtime::spawn_blocking(move || runners::available(&data, source))
        .await
        .map_err(|e| e.to_string())?
}

/// Starts downloading and installing a runner; progress arrives as `runner-install` events.
// @rpc
async fn install_runner(
    app: AppHandle,
    source: runners::Source,
    tag: String,
) -> Result<(), String> {
    runners::install(&app, app.data_dir(), source, tag)
}

/// Stops an install in progress, whatever stage it is in.
// @rpc
async fn cancel_runner_install(name: String) -> Result<(), String> {
    runners::cancel(&name);
    Ok(())
}

/// Names of the runner installs running right now (the page may open in the middle of one).
// @rpc
async fn running_runner_installs() -> Result<Vec<String>, String> {
    Ok(runners::running())
}

/// Deletes a runner Lutra downloaded.
// @rpc
async fn remove_runner(app: AppHandle, name: String) -> Result<(), String> {
    let data = app.data_dir();
    host::async_runtime::spawn_blocking(move || runners::remove(&data, &name))
        .await
        .map_err(|e| e.to_string())??;
    let _ = app.emit("runners-changed", ());
    Ok(())
}

/// Helper programs (Wine, umu, winetricks, gamemode, mangohud) and their versions.
// @rpc
async fn wine_tools() -> Result<Tools, String> {
    host::async_runtime::spawn_blocking(wine::detect_tools)
        .await
        .map_err(|e| e.to_string())
}

// @rpc
async fn default_runner(library: State<'_, Library>) -> Result<Option<String>, String> {
    library
        .lock()?
        .setting(DEFAULT_RUNNER)
        .map_err(|e| e.to_string())
}

/// Runner preselected when adding a game. `None` goes back to "the first one found".
// @rpc
async fn set_default_runner(
    library: State<'_, Library>,
    path: Option<String>,
) -> Result<(), String> {
    let db = library.lock()?;
    match path.filter(|path| !path.is_empty()) {
        Some(path) => db.set_setting(DEFAULT_RUNNER, &path),
        None => db.delete_setting(DEFAULT_RUNNER),
    }
    .map_err(|e| e.to_string())
}

/// What the user chose when adding a game by hand. Everything but the program is optional:
/// what is left out is filled in with the best values for this machine.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewGame {
    title: String,
    exe_path: String,
    /// `wine`, `proton` or `native`.
    runner_kind: String,
    #[serde(default)]
    runner_path: String,
    prefix_path: Option<String>,
    meta: Option<db::GameMeta>,
    config: Option<WineConfig>,
}

fn hardware_now() -> hardware::Hardware {
    hardware::detect()
}

/// Title from the program's name when the user left it empty.
fn title_or_stem(title: &str, exe: &Path) -> String {
    match title.trim() {
        "" => exe
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        title => title.to_string(),
    }
}

// @rpc
async fn add_game(
    app: AppHandle,
    library: State<'_, Library>,
    game: NewGame,
) -> Result<Game, String> {
    let exe = Path::new(&game.exe_path);
    if !exe.is_file() {
        return Err(format!("not a file: {}", game.exe_path));
    }
    let kind = RunnerKind::parse(&game.runner_kind).ok_or("unknown runner type")?;
    if kind != RunnerKind::Native && !Path::new(&game.runner_path).is_file() {
        return Err(format!("runner not found: {}", game.runner_path));
    }
    if kind == RunnerKind::Native {
        wine::ensure_runnable(exe)?;
    }

    let title = title_or_stem(&game.title, exe);
    let key = format!("{:08x}", stable_hash(&game.exe_path));
    let (source, id) = match kind {
        RunnerKind::Native => ("native", format!("native:{key}")),
        _ => ("wine", format!("wine:{key}")),
    };
    let prefix = match (
        kind,
        game.prefix_path.filter(|path| !path.trim().is_empty()),
    ) {
        (RunnerKind::Native, _) => String::new(),
        (_, Some(existing)) => {
            prefixes::check_usable(Path::new(&existing), kind)?;
            existing
        }
        (_, None) => app_dir(&app)?
            .join("prefixes")
            .join(&key)
            .to_string_lossy()
            .into_owned(),
    };
    let config = game.config.unwrap_or_else(|| {
        wine::recommended_config(
            &hardware_now(),
            kind,
            wine::find_in_path("gamemoderun").is_some(),
        )
    });

    let db = library.lock()?;
    db.add_manual(&ManualGame {
        id: id.clone(),
        source,
        title: title.clone(),
        exe_path: game.exe_path,
        runner_kind: kind.as_str().to_string(),
        runner_path: game.runner_path,
        prefix_path: prefix,
        config,
        meta: game.meta,
    })
    .map_err(|e| e.to_string())?;
    applog::info(format!("Added {title} ({id}, {})", kind.as_str()));
    db.get(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "game was not saved".to_string())
}

/// A Wine prefix that removing a game can delete too.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemovablePrefix {
    path: String,
    /// What it takes on disk.
    bytes: u64,
}

/// The game's prefix, when deleting it with the game is safe: it is one of Lutra's own (never
/// a folder the user pointed at, nor Steam's), and no other game uses it.
fn removable_prefix(app: &AppHandle, db: &Db, id: &str) -> Result<Option<PathBuf>, String> {
    let Some(settings) = db.wine_settings(id).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let prefix = PathBuf::from(&settings.prefix_path);
    let own = app_dir(app)?.join("prefixes");
    if settings.prefix_path.is_empty()
        || !prefix.starts_with(&own)
        || prefix == own
        || !prefix.is_dir()
    {
        return Ok(None);
    }
    let shared = db
        .prefix_users(&settings.prefix_path)
        .map_err(|e| e.to_string())?
        .iter()
        .any(|user| user != id);
    Ok((!shared).then_some(prefix))
}

/// Size of a folder, without following links (a prefix links to the whole file system).
fn folder_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => folder_size(&entry.path()),
            Ok(kind) if kind.is_file() => entry.metadata().map_or(0, |meta| meta.len()),
            _ => 0,
        })
        .sum()
}

/// What removing a game can delete besides its entry.
// @rpc
async fn removable_prefix_of(
    app: AppHandle,
    library: State<'_, Library>,
    id: String,
) -> Result<Option<RemovablePrefix>, String> {
    let Some(prefix) = removable_prefix(&app, &*library.lock()?, &id)? else {
        return Ok(None);
    };
    host::async_runtime::spawn_blocking(move || RemovablePrefix {
        bytes: folder_size(&prefix),
        path: prefix.to_string_lossy().into_owned(),
    })
    .await
    .map(Some)
    .map_err(|e| e.to_string())
}

/// Removes a game added by hand. With `everything`, its Wine prefix (game files installed in it
/// and save games included), its logs and its controller profile go too; otherwise the prefix
/// is kept, since it may hold save games.
// @rpc
async fn remove_game(
    app: AppHandle,
    library: State<'_, Library>,
    id: String,
    everything: bool,
) -> Result<(), String> {
    if !id.starts_with("wine:") && !id.starts_with("native:") {
        return Err("only games added by hand can be removed".into());
    }
    if app
        .state::<Running>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .contains_key(&id)
    {
        return Err("stop the game before removing it".into());
    }
    let prefix = if everything {
        let db = library.lock()?;
        let prefix = removable_prefix(&app, &db, &id)?;
        db.delete_controller_profile(&id)
            .map_err(|e| e.to_string())?;
        prefix
    } else {
        None
    };
    library.lock()?.remove(&id).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(app_dir(&app)?.join("images").join(dir_name(&id)));
    if everything {
        let logs = app_dir(&app)?.join("logs");
        let name = dir_name(&id);
        if let Ok(entries) = fs::read_dir(&logs) {
            for entry in entries.flatten() {
                let file = entry.file_name().to_string_lossy().into_owned();
                // `<name>.log`, `<name>.prev.log`, `<name>-winecfg.log`… and the `<name>.d` folder.
                if file.starts_with(&format!("{name}.")) || file.starts_with(&format!("{name}-")) {
                    let path = entry.path();
                    let _ = if path.is_dir() {
                        fs::remove_dir_all(path)
                    } else {
                        fs::remove_file(path)
                    };
                }
            }
        }
    }
    if let Some(prefix) = prefix {
        let path = prefix.clone();
        host::async_runtime::spawn_blocking(move || fs::remove_dir_all(path))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("the game is removed, but not its prefix: {e}"))?;
        applog::info(format!("Removed {id} with its prefix {}", prefix.display()));
    } else {
        applog::info(format!("Removed {id}"));
    }
    Ok(())
}

// @rpc
async fn wine_settings(library: State<'_, Library>, id: String) -> Result<WineSettings, String> {
    let mut settings = library
        .lock()?
        .wine_settings(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("not a game added by hand: {id}"))?;
    settings.prefix_exists = Path::new(&settings.prefix_path).is_dir();
    Ok(settings)
}

// @rpc
async fn save_wine_settings(
    library: State<'_, Library>,
    settings: WineSettings,
) -> Result<Game, String> {
    let kind = RunnerKind::parse(&settings.runner_kind).ok_or("unknown runner type")?;
    if !Path::new(&settings.exe_path).is_file() {
        return Err(format!("not a file: {}", settings.exe_path));
    }
    if kind != RunnerKind::Native {
        if !Path::new(&settings.runner_path).is_file() {
            return Err(format!("runner not found: {}", settings.runner_path));
        }
        prefixes::check_usable(Path::new(&settings.prefix_path), kind)?;
    }
    if let Some(size) = settings
        .config
        .virtual_desktop
        .as_deref()
        .filter(|size| !size.is_empty())
    {
        let valid = size
            .split_once('x')
            .is_some_and(|(w, h)| w.parse::<u32>().is_ok() && h.parse::<u32>().is_ok());
        if !valid {
            return Err("the virtual desktop size must look like 1920x1080".into());
        }
    }

    let mut settings = settings;
    settings.title = settings.title.trim().to_string();
    if settings.title.is_empty() {
        return Err("the game needs a name".into());
    }
    // Empty text fields mean "not set".
    settings.config.working_dir = settings
        .config
        .working_dir
        .filter(|dir| !dir.trim().is_empty());
    settings.config.virtual_desktop = settings
        .config
        .virtual_desktop
        .filter(|size| !size.trim().is_empty());
    settings.config.dxvk_hud = settings
        .config
        .dxvk_hud
        .filter(|hud| !hud.trim().is_empty());
    settings.config.fps_limit = settings.config.fps_limit.filter(|limit| *limit > 0);

    let db = library.lock()?;
    db.save_wine_settings(&settings)
        .map_err(|e| e.to_string())?;
    applog::info(format!("Settings saved for {}", settings.title));
    db.get(&settings.id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "game not found".to_string())
}

/// Looks a game up in GameDB, to fill in its name, release date, developer and genres.
// @rpc
async fn search_game_db(app: AppHandle, query: String) -> Result<Vec<gamedb::GameHit>, String> {
    let cache = app_dir(&app)?.join("gamedb");
    host::async_runtime::spawn_blocking(move || gamedb::search(&cache, &query, 6))
        .await
        .map_err(|e| e.to_string())?
}

/// Saves the details of a game as the user edited them.
// @rpc
async fn save_game_meta(
    library: State<'_, Library>,
    id: String,
    meta: db::GameMeta,
) -> Result<Game, String> {
    let db = library.lock()?;
    db.set_game_meta(&id, &meta).map_err(|e| e.to_string())?;
    db.get(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "game not found".to_string())
}

// @rpc
async fn game_meta(library: State<'_, Library>, id: String) -> Result<db::GameMeta, String> {
    library.lock()?.game_meta(&id).map_err(|e| e.to_string())
}

/// What this machine has: GPUs, NT sync, gamescope.
// @rpc
async fn hardware_info() -> Result<hardware::Hardware, String> {
    host::async_runtime::spawn_blocking(hardware_now)
        .await
        .map_err(|e| e.to_string())
}

/// The options that suit this machine best, for a kind of runner.
// @rpc
async fn recommended_config(runner_kind: String) -> Result<WineConfig, String> {
    let kind = RunnerKind::parse(&runner_kind).ok_or("unknown runner type")?;
    Ok(wine::recommended_config(
        &hardware_now(),
        kind,
        wine::find_in_path("gamemoderun").is_some(),
    ))
}

/// Resolves the runner, prefix and helper programs of a game into a command.
fn spec_for(
    settings: &WineSettings,
    config: &WineConfig,
    target: Target,
    log_dir: Option<&Path>,
) -> Result<LaunchSpec, String> {
    let runner = Runner {
        kind: RunnerKind::parse(&settings.runner_kind).ok_or("unknown runner type")?,
        label: String::new(),
        path: PathBuf::from(&settings.runner_path),
    };
    let steam_roots = steam::steam_roots(&home_dir()?);
    let umu = wine::find_in_path("umu-run");
    let gamemode = wine::find_in_path("gamemoderun");
    let mangohud = wine::find_in_path("mangohud");
    let hardware = hardware_now();

    wine::build_launch(&LaunchRequest {
        runner: &runner,
        prefix: Path::new(&settings.prefix_path),
        config,
        target,
        umu: umu.as_deref(),
        steam_root: steam_roots.first().map(PathBuf::as_path),
        gamemode: gamemode.as_deref(),
        mangohud: mangohud.as_deref(),
        hardware: Some(&hardware),
        log_dir,
    })
}

/// What was run, written at the top of a launch log: the first thing to read when a game does not start.
fn launch_header(spec: &LaunchSpec) -> String {
    let quote = |text: &std::ffi::OsStr| {
        let text = text.to_string_lossy();
        shlex::try_quote(&text)
            .map(|quoted| quoted.into_owned())
            .unwrap_or_else(|_| text.into_owned())
    };
    let command: Vec<String> = std::iter::once(spec.program.as_os_str())
        .chain(spec.args.iter().map(|arg| arg.as_os_str()))
        .map(quote)
        .collect();

    let mut header = format!(
        "=== Lutra · {} ===\nCommand: {}\nFolder: {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        command.join(" "),
        spec.cwd.display()
    );
    for (key, value) in &spec.envs {
        header.push_str(&format!("Env: {key}={}\n", value.to_string_lossy()));
    }
    header.push_str("===\n\n");
    header
}

/// Where a launch log goes, after keeping the previous run's log as `<name>.prev.log`.
fn rotate_log(logs: &Path, log_name: &str) -> PathBuf {
    let current = logs.join(format!("{log_name}.log"));
    if current.is_file() {
        let _ = fs::rename(&current, logs.join(format!("{log_name}.prev.log")));
    }
    current
}

/// Starts a process in its own process group (so the whole tree can be stopped), logging its output.
fn spawn_process(
    app: &AppHandle,
    log_name: &str,
    prefix: &Path,
    spec: &LaunchSpec,
) -> Result<Child, String> {
    let logs = app_dir(app)?.join("logs");
    fs::create_dir_all(&logs).map_err(|e| e.to_string())?;
    let mut log = File::create(rotate_log(&logs, log_name)).map_err(|e| e.to_string())?;
    let _ = log.write_all(launch_header(spec).as_bytes());

    let mut command = Command::new(&spec.program);
    wine::host_env(&mut command)
        .args(&spec.args)
        .envs(spec.envs.iter().map(|(key, value)| (key, value)))
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log)
        .process_group(0);
    if spec.cwd.is_dir() {
        command.current_dir(&spec.cwd);
    } else if prefix.is_dir() {
        command.current_dir(prefix);
    }
    command
        .spawn()
        .map_err(|e| format!("could not start {}: {e}", spec.program.display()))
}

fn emit_running(app: &AppHandle, id: &str, running: bool) {
    let _ = app.emit(
        "game-running",
        RunningEvent {
            id: id.to_string(),
            running,
        },
    );
    // The window goes to the notification area once a game is really up (see `hide_for_game`),
    // not when its process is created: the launch is followed from the window until then.
    if !running {
        // Installers do not count: the user follows them from Lutra's window.
        let games_running = app
            .state::<Running>()
            .0
            .lock()
            .is_ok_and(|running| running.keys().any(|key| !key.starts_with("install:")));
        tray::on_game(app, games_running);
    }
}

/// Called when a launch reaches "ready": the game is up, so the window can step aside.
fn hide_for_game(app: &AppHandle) {
    tray::on_game(app, true);
}

fn emit_progress(
    app: &AppHandle,
    id: &str,
    stage: &'static str,
    step: u32,
    steps: u32,
    state: LaunchState,
    message: Option<String>,
) {
    let _ = app.emit(
        "launch-progress",
        LaunchProgress {
            id: id.to_string(),
            stage,
            step,
            steps,
            state,
            message,
        },
    );
}

/// How a launched process ended, shared with the thread that watches the launch.
#[derive(Default)]
struct ExitFlag {
    done: AtomicBool,
    success: AtomicBool,
}

/// Watches the game in the background: tells the interface it is running, then counts play time.
fn track_game(app: &AppHandle, id: &str, mut child: Child) -> (u32, Arc<ExitFlag>) {
    let pid = child.id();
    app.state::<Running>()
        .0
        .lock()
        .expect("running registry")
        .insert(id.to_string(), pid);
    emit_running(app, id, true);

    let flag = Arc::new(ExitFlag::default());
    let exit = flag.clone();
    let handle = app.clone();
    let id = id.to_string();
    std::thread::spawn(move || {
        let started = Instant::now();
        let status = child.wait();
        applog::info(format!(
            "{id} exited ({}) after {}s",
            status
                .as_ref()
                .map_or_else(|e| e.to_string(), |status| status.to_string()),
            started.elapsed().as_secs()
        ));
        // Rounded to the nearest minute, so a quick test launch does not count as play time.
        let minutes = ((started.elapsed().as_secs() + 30) / 60) as u32;

        handle
            .state::<Running>()
            .0
            .lock()
            .expect("running registry")
            .remove(&id);
        handle.state::<ControllerService>().end_session(&id);
        if let Ok(db) = handle.state::<Library>().lock() {
            let _ = db.add_playtime(&id, minutes);
        }
        exit.success.store(
            status.is_ok_and(|status| status.success()),
            Ordering::SeqCst,
        );
        exit.done.store(true, Ordering::SeqCst);
        emit_running(&handle, &id, false);
        emit_library(&handle);
    });
    (pid, flag)
}

/// How long a launch is followed before the game is assumed to be up (launchers that start the
/// real game under another name never match the executable).
const READY_TIMEOUT: Duration = Duration::from_secs(60);

/// What the watcher of a launch needs to know.
struct LaunchWatch {
    id: String,
    pgid: u32,
    exe: String,
    log: PathBuf,
    prefix: PathBuf,
    proton: bool,
    /// The prefix did not exist: creating it is a stage of its own.
    new_prefix: bool,
    /// The program itself is the game (no Wine in between).
    native: bool,
    exit: Arc<ExitFlag>,
}

/// Follows a launch through its real stages and reports each one: the prefix being created, then
/// the game's own process showing up. Ends when the game is up, or when it stopped trying.
fn watch_launch(app: AppHandle, watch: LaunchWatch) {
    std::thread::spawn(move || {
        let steps = 3 + u32::from(watch.new_prefix);
        let started = Instant::now();
        let mut stage = "start";
        let mut step = 1;
        emit_progress(
            &app,
            &watch.id,
            stage,
            step,
            steps,
            LaunchState::Progress,
            None,
        );

        loop {
            std::thread::sleep(Duration::from_millis(250));
            if watch.exit.done.load(Ordering::SeqCst) {
                let ok = watch.exit.success.load(Ordering::SeqCst);
                let message = (!ok).then(|| {
                    let tail = session::log_tail(&watch.log, 6);
                    if tail.is_empty() {
                        "the game stopped right after starting".to_string()
                    } else {
                        tail
                    }
                });
                let state = if ok {
                    LaunchState::Ended
                } else {
                    LaunchState::Failed
                };
                emit_progress(&app, &watch.id, stage, step, steps, state, message);
                return;
            }

            if watch.new_prefix && step < 3 && !session::prefix_ready(&watch.prefix, watch.proton) {
                if stage != "prefix" {
                    stage = "prefix";
                    step = 2;
                    emit_progress(
                        &app,
                        &watch.id,
                        stage,
                        step,
                        steps,
                        LaunchState::Progress,
                        None,
                    );
                }
            } else if stage != "wait" {
                stage = "wait";
                step = steps - 1;
                emit_progress(
                    &app,
                    &watch.id,
                    stage,
                    step,
                    steps,
                    LaunchState::Progress,
                    None,
                );
            }

            if stage == "wait" {
                let procs = session::processes(Path::new("/proc"));
                let up = if watch.native {
                    // A native program is the game: surviving its first moments is the signal.
                    started.elapsed() > Duration::from_secs(2)
                        && session::group_alive(&procs, watch.pgid)
                } else {
                    session::game_running(&procs, watch.pgid, &watch.exe)
                        // A launcher that detached its game from its own family: once the launch
                        // has had a moment, a process of that name anywhere is the game.
                        || (started.elapsed() > Duration::from_secs(10)
                            && session::game_running_anywhere(&procs, &watch.exe))
                };
                if up || started.elapsed() > READY_TIMEOUT {
                    emit_progress(
                        &app,
                        &watch.id,
                        "wait",
                        steps,
                        steps,
                        LaunchState::Ready,
                        None,
                    );
                    if !watch.exit.done.load(Ordering::SeqCst) {
                        hide_for_game(&app);
                    }
                    return;
                }
            }
        }
    });
}

/// Helper processes (winecfg, winetricks, other programs) are not tracked: just reap them.
fn detach(mut child: Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

/// Follows a Steam game: ready when Steam's wrapper process appears, running until it is gone.
fn watch_steam(app: AppHandle, id: String, app_id: String) {
    std::thread::spawn(move || {
        let started = Instant::now();
        let procs = || session::processes(Path::new("/proc"));
        emit_progress(&app, &id, "wait", 1, 2, LaunchState::Progress, None);

        // Steam's wrapper process, which ends when the game does. Its group may be Steam's own,
        // so only this process is ever signalled.
        let pid = loop {
            std::thread::sleep(Duration::from_millis(500));
            let procs = procs();
            if let Some(proc) = procs
                .iter()
                .find(|proc| session::steam_app_running(std::slice::from_ref(proc), &app_id))
            {
                break proc.pid;
            }
            if started.elapsed() > Duration::from_secs(120) {
                emit_progress(&app, &id, "wait", 2, 2, LaunchState::Ended, None);
                return;
            }
        };

        app.state::<Running>()
            .0
            .lock()
            .expect("running registry")
            .insert(id.clone(), pid);
        emit_progress(&app, &id, "wait", 2, 2, LaunchState::Ready, None);
        emit_running(&app, &id, true);
        hide_for_game(&app);

        while Path::new(&format!("/proc/{pid}")).exists() {
            std::thread::sleep(Duration::from_secs(2));
        }
        app.state::<Running>()
            .0
            .lock()
            .expect("running registry")
            .remove(&id);
        emit_running(&app, &id, false);
    });
}

// @rpc
async fn launch_game(
    app: AppHandle,
    library: State<'_, Library>,
    id: String,
) -> Result<(), String> {
    let info = {
        let db = library.lock()?;
        db.launch_info(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("unknown game: {id}"))?
    };

    emit_progress(&app, &id, "check", 0, 3, LaunchState::Progress, None);
    let result = launch_inner(&app, &library, &id, &info);
    if let Err(message) = &result {
        emit_progress(
            &app,
            &id,
            "check",
            0,
            3,
            LaunchState::Failed,
            Some(message.clone()),
        );
    }
    result
}

fn launch_inner(
    app: &AppHandle,
    library: &Library,
    id: &str,
    info: &db::LaunchInfo,
) -> Result<(), String> {
    match info.source.as_str() {
        "steam" => {
            applog::info(format!("Launching {id} through Steam"));
            wine::open_on_host(format!("steam://rungameid/{}", info.source_id))?;
            library
                .lock()?
                .mark_played(id, now())
                .map_err(|e| e.to_string())?;
            watch_steam(app.clone(), id.to_string(), info.source_id.clone());
            Ok(())
        }
        "wine" | "native" => {
            if app
                .state::<Running>()
                .0
                .lock()
                .map_err(|e| e.to_string())?
                .contains_key(id)
            {
                return Err("this game is already running".into());
            }
            let settings = library
                .lock()?
                .wine_settings(id)
                .map_err(|e| e.to_string())?
                .ok_or("this game is missing its launch settings")?;
            let kind = RunnerKind::parse(&settings.runner_kind).ok_or("unknown runner type")?;
            if kind == RunnerKind::Native {
                wine::ensure_runnable(Path::new(&settings.exe_path))?;
            } else if !Path::new(&settings.runner_path).is_file() {
                return Err(format!(
                    "the runner is gone ({}): choose another one in the game's settings",
                    settings.runner_path
                ));
            }

            // Runner and graphics logs of this run only (debug mode); the previous ones are dropped.
            let extra_logs = app_dir(app)?
                .join("logs")
                .join(format!("{}.d", dir_name(id)));
            let _ = fs::remove_dir_all(&extra_logs);
            if settings.config.debug {
                fs::create_dir_all(&extra_logs).map_err(|e| e.to_string())?;
            }
            let mut spec = spec_for(
                &settings,
                &settings.config,
                Target::Program(Path::new(&settings.exe_path)),
                Some(&extra_logs),
            )?;
            applog::info(format!(
                "Launching {} ({id}) with {}",
                settings.title,
                if kind == RunnerKind::Native {
                    "no runner (native)"
                } else {
                    &settings.runner_path
                }
            ));
            let prefix = PathBuf::from(&settings.prefix_path);
            let new_prefix = kind != RunnerKind::Native
                && !session::prefix_ready(&prefix, kind == RunnerKind::Proton);

            take_controllers(app, id, &mut spec);
            let log_name = dir_name(id);
            let child = spawn_process(app, &log_name, &prefix, &spec).inspect_err(|error| {
                applog::error(format!("Could not launch {}: {error}", settings.title));
                app.state::<ControllerService>().end_session(id);
            })?;
            library
                .lock()?
                .mark_played(id, now())
                .map_err(|e| e.to_string())?;
            let (pgid, exit) = track_game(app, id, child);
            watch_launch(
                app.clone(),
                LaunchWatch {
                    id: id.to_string(),
                    pgid,
                    exe: settings.exe_path.clone(),
                    log: app_dir(app)?.join("logs").join(format!("{log_name}.log")),
                    prefix,
                    proton: kind == RunnerKind::Proton,
                    new_prefix,
                    native: kind == RunnerKind::Native,
                    exit,
                },
            );
            Ok(())
        }
        other => Err(format!("launching {other} games is not supported yet")),
    }
}

/// Stops a running Windows game: its whole process group, then the Wine server of its prefix.
// @rpc
async fn stop_game(app: AppHandle, library: State<'_, Library>, id: String) -> Result<(), String> {
    let pid = app
        .state::<Running>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(&id)
        .copied();
    let Some(pid) = pid else { return Ok(()) };
    applog::info(format!("Stopping {id} (process {pid})"));
    if id.starts_with("steam:") {
        // Steam's wrapper process: it takes the game down with it.
        let _ = wine::host_env(&mut Command::new("kill"))
            .args(["-TERM", &pid.to_string()])
            .status();
        return Ok(());
    }

    let _ = wine::host_env(&mut Command::new("kill"))
        .args(["-TERM", "--", &format!("-{pid}")])
        .status();

    let settings = library
        .lock()?
        .wine_settings(&id)
        .map_err(|e| e.to_string())?;
    if let Some(settings) = settings.filter(|settings| settings.runner_kind == "wine") {
        let _ = wine::host_env(&mut Command::new(
            Path::new(&settings.runner_path).with_file_name("wineserver"),
        ))
        .arg("-k")
        .env("WINEPREFIX", &settings.prefix_path)
        .status();
    }
    Ok(())
}

// @rpc
async fn running_games(app: AppHandle) -> Result<Vec<String>, String> {
    Ok(app
        .state::<Running>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .keys()
        .cloned()
        .collect())
}

/// Opens winecfg or winetricks on a game's prefix.
// @rpc
async fn launch_wine_tool(
    app: AppHandle,
    library: State<'_, Library>,
    id: String,
    tool: String,
) -> Result<(), String> {
    let settings = library
        .lock()?
        .wine_settings(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("not a Windows game: {id}"))?;

    let winetricks;
    let target = match tool.as_str() {
        "winecfg" => Target::Winecfg,
        "winetricks" => {
            winetricks = wine::find_in_path("winetricks").ok_or("winetricks is not installed")?;
            Target::Winetricks(&winetricks)
        }
        other => return Err(format!("unknown tool: {other}")),
    };

    let spec = spec_for(&settings, &settings.config, target, None)?;
    applog::info(format!("Opening {tool} for {}", settings.title));
    detach(spawn_process(
        &app,
        &format!("{}-{tool}", dir_name(&id)),
        Path::new(&settings.prefix_path),
        &spec,
    )?);
    Ok(())
}

/// Runs another program (an installer, a patch...) inside a game's prefix.
// @rpc
async fn run_in_prefix(
    app: AppHandle,
    library: State<'_, Library>,
    id: String,
    exe_path: String,
) -> Result<(), String> {
    let exe = Path::new(&exe_path);
    if !exe.is_file() {
        return Err(format!("not a file: {exe_path}"));
    }
    let settings = library
        .lock()?
        .wine_settings(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("not a Windows game: {id}"))?;

    // The game's own arguments and working folder do not apply to some other program.
    let config = WineConfig {
        args: String::new(),
        working_dir: None,
        ..settings.config.clone()
    };
    let spec = spec_for(&settings, &config, Target::Program(exe), None)?;
    applog::info(format!(
        "Running {exe_path} in the prefix of {}",
        settings.title
    ));
    detach(spawn_process(
        &app,
        &format!("{}-run", dir_name(&id)),
        Path::new(&settings.prefix_path),
        &spec,
    )?);
    Ok(())
}

/// Opens a folder of a Windows game in the file manager: `files`, `prefix` or `logs`.
// @rpc
async fn open_game_folder(
    app: AppHandle,
    library: State<'_, Library>,
    id: String,
    what: String,
) -> Result<(), String> {
    let settings = library
        .lock()?
        .wine_settings(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("not a Windows game: {id}"))?;

    let folder = match what.as_str() {
        "files" => Path::new(&settings.exe_path)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default(),
        "prefix" => wine::effective_prefix(
            RunnerKind::parse(&settings.runner_kind).ok_or("unknown runner type")?,
            Path::new(&settings.prefix_path),
        ),
        "logs" => app_dir(&app)?.join("logs"),
        other => return Err(format!("unknown folder: {other}")),
    };
    if !folder.is_dir() {
        return Err(format!(
            "{} does not exist yet; launch the game once first",
            folder.display()
        ));
    }
    wine::open_on_host(folder)
}

/// What the user asks for when installing a game from its setup program.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallRequest {
    installer_path: String,
    title: String,
    /// `wine` or `proton`.
    runner_kind: String,
    runner_path: String,
    prefix_path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallStarted {
    install_id: String,
    prefix_path: String,
}

/// Sent when the installer has closed: what appeared in the prefix, to pick the game from.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallFinished {
    install_id: String,
    prefix_path: String,
    success: bool,
    /// The last lines of the installer's log, when it failed.
    message: Option<String>,
    candidates: Vec<installer::Candidate>,
}

/// Runs a game's setup program in a prefix and, once it closes, reports the programs it installed.
// @rpc
async fn start_install(app: AppHandle, request: InstallRequest) -> Result<InstallStarted, String> {
    let installer_exe = PathBuf::from(&request.installer_path);
    if !installer_exe.is_file() {
        return Err(format!("not a file: {}", request.installer_path));
    }
    let kind = RunnerKind::parse(&request.runner_kind)
        .filter(|kind| *kind != RunnerKind::Native)
        .ok_or("an installer needs Wine or Proton")?;
    if !Path::new(&request.runner_path).is_file() {
        return Err(format!("runner not found: {}", request.runner_path));
    }

    let key = format!(
        "{:08x}",
        stable_hash(&format!("{}|{}", request.installer_path, request.title))
    );
    let install_id = format!("install:{key}");
    if app
        .state::<Running>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .contains_key(&install_id)
    {
        return Err("this installer is already running".into());
    }

    let prefix = match request.prefix_path.filter(|path| !path.trim().is_empty()) {
        Some(existing) => {
            prefixes::check_usable(Path::new(&existing), kind)?;
            PathBuf::from(existing)
        }
        None => app_dir(&app)?.join("prefixes").join(&key),
    };
    let drive_c = wine::effective_prefix(kind, &prefix).join("drive_c");
    let before = installer::scan_programs(&drive_c);

    let settings = WineSettings {
        id: install_id.clone(),
        title: request.title.clone(),
        exe_path: request.installer_path.clone(),
        runner_kind: kind.as_str().to_string(),
        runner_path: request.runner_path.clone(),
        prefix_path: prefix.to_string_lossy().into_owned(),
        config: wine::recommended_config(&hardware_now(), kind, false),
        prefix_exists: prefix.is_dir(),
    };
    let spec = spec_for(
        &settings,
        &settings.config,
        Target::Program(&installer_exe),
        None,
    )?;
    applog::info(format!(
        "Running the installer {} for {} in {}",
        request.installer_path,
        request.title,
        prefix.display()
    ));
    let log_name = dir_name(&install_id);
    let child = spawn_process(&app, &log_name, &prefix, &spec)?;
    let pid = child.id();
    app.state::<Running>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(install_id.clone(), pid);
    emit_running(&app, &install_id, true);

    let log = app_dir(&app)?.join("logs").join(format!("{log_name}.log"));
    let title = request.title;
    let handle = app.clone();
    let id = install_id.clone();
    let prefix_text = settings.prefix_path.clone();
    std::thread::spawn(move || {
        let mut child = child;
        let status = child.wait();
        // Installers often hand over to a helper that keeps going a moment: give it a little time.
        let waited = Instant::now();
        while waited.elapsed() < Duration::from_secs(4)
            && session::group_alive(&session::processes(Path::new("/proc")), pid)
        {
            std::thread::sleep(Duration::from_millis(300));
        }
        handle
            .state::<Running>()
            .0
            .lock()
            .expect("running registry")
            .remove(&id);
        let after = installer::scan_programs(&drive_c);
        let candidates = installer::candidates(&before, &after, &drive_c, &title);
        let success = status.is_ok_and(|status| status.success());
        applog::info(format!(
            "Installer of {title} closed ({}), {} new program(s)",
            if success { "ok" } else { "failed" },
            candidates.len()
        ));
        let message = (!success)
            .then(|| session::log_tail(&log, 6))
            .filter(|text| !text.is_empty());
        emit_running(&handle, &id, false);
        let _ = handle.emit(
            "install-finished",
            InstallFinished {
                install_id: id,
                prefix_path: prefix_text,
                success,
                message,
                candidates,
            },
        );
    });

    Ok(InstallStarted {
        install_id,
        prefix_path: settings.prefix_path,
    })
}

/// Closes an installer that is still running.
// @rpc
async fn cancel_install(app: AppHandle, install_id: String) -> Result<(), String> {
    let pid = app
        .state::<Running>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(&install_id)
        .copied();
    if let Some(pid) = pid {
        applog::info(format!("Cancelling {install_id}"));
        let _ = wine::host_env(&mut Command::new("kill"))
            .args(["-TERM", "--", &format!("-{pid}")])
            .status();
    }
    Ok(())
}

/// Wine prefixes that exist on this machine, to pick one for a game.
// @rpc
async fn list_prefixes(app: AppHandle) -> Result<Vec<prefixes::PrefixInfo>, String> {
    let lutra = app_dir(&app)?.join("prefixes");
    let roots = steam::steam_roots(&home_dir()?);
    Ok(prefixes::list(&lutra, &roots))
}

/// Tells whether a folder can be the prefix of a game run by `runner_kind`, and why not.
// @rpc
async fn check_prefix(path: String, runner_kind: String) -> Result<(), String> {
    let kind = RunnerKind::parse(&runner_kind).ok_or("unknown runner type")?;
    prefixes::check_usable(Path::new(&path), kind)
}

/// Opens the folder holding the Wine prefixes Lutra creates.
/// Opens a web link with the default browser, outside of the AppImage's environment.
// @rpc
async fn open_external(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("only web links can be opened".into());
    }
    wine::open_on_host(url)
}

// @rpc
async fn open_prefixes_folder(app: AppHandle) -> Result<(), String> {
    let folder = app_dir(&app)?.join("prefixes");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    wine::open_on_host(folder)
}

/// Forgets a game's artwork so it is downloaded again, and tells the interface.
fn reset_game_art(app: &AppHandle, library: &Library, id: &str) -> Result<(), String> {
    let game = {
        let db = library.lock()?;
        db.clear_game_images(id).map_err(|e| e.to_string())?;
        db.get(id).map_err(|e| e.to_string())?
    };
    let _ = fs::remove_dir_all(app_dir(app)?.join("images").join(dir_name(id)));
    if let Some(game) = game {
        let _ = app.emit("game-updated", game);
    }
    start_import(app, false);
    Ok(())
}

/// Looks games up on SteamGridDB so the user can pick the right one for a game's artwork.
// @rpc
async fn search_artwork(
    library: State<'_, Library>,
    query: String,
) -> Result<Vec<SearchHit>, String> {
    let key = library
        .lock()?
        .setting(STEAMGRIDDB_KEY)
        .map_err(|e| e.to_string())?
        .ok_or("no_key")?;

    host::async_runtime::spawn_blocking(move || Client::new(&key).search(&query))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| match e {
            ArtError::InvalidKey => "invalid_key".to_string(),
            other => other.to_string(),
        })
}

/// Uses a SteamGridDB game picked by the user for a game's artwork (`None`: back to automatic).
// @rpc
async fn pick_artwork(
    app: AppHandle,
    library: State<'_, Library>,
    id: String,
    sgdb_game: Option<u64>,
) -> Result<(), String> {
    library
        .lock()?
        .set_sgdb_game(&id, sgdb_game)
        .map_err(|e| e.to_string())?;
    reset_game_art(&app, &library, &id)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SteamArt {
    /// Steam's own artwork.
    Official,
    /// Images picked on SteamGridDB.
    Custom,
}

impl SteamArt {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "official" => Some(SteamArt::Official),
            "custom" => Some(SteamArt::Custom),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            SteamArt::Official => "official",
            SteamArt::Custom => "custom",
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtSettings {
    /// `official` or `custom`.
    steam_source: String,
    /// With custom images, use Steam's own when SteamGridDB has none.
    fallback: bool,
    has_key: bool,
}

fn read_art_settings(db: &Db) -> Result<(SteamArt, bool, Option<String>), String> {
    let source = db
        .setting(STEAM_ART_SOURCE)
        .map_err(|e| e.to_string())?
        .and_then(|value| SteamArt::parse(&value))
        .unwrap_or(SteamArt::Official);
    let fallback = db
        .setting(STEAM_ART_FALLBACK)
        .map_err(|e| e.to_string())?
        .as_deref()
        != Some("0");
    let key = db.setting(STEAMGRIDDB_KEY).map_err(|e| e.to_string())?;
    Ok((source, fallback, key))
}

fn art_settings_dto(db: &Db) -> Result<ArtSettings, String> {
    let (source, fallback, key) = read_art_settings(db)?;
    Ok(ArtSettings {
        steam_source: source.as_str().to_string(),
        fallback,
        has_key: key.is_some(),
    })
}

// @rpc
async fn art_settings(library: State<'_, Library>) -> Result<ArtSettings, String> {
    let db = library.lock()?;
    art_settings_dto(&db)
}

/// Saves how Steam games get their images. Changing it fetches their artwork again.
// @rpc
async fn set_art_settings(
    app: AppHandle,
    library: State<'_, Library>,
    steam_source: String,
    fallback: bool,
) -> Result<ArtSettings, String> {
    let source = SteamArt::parse(&steam_source).ok_or("unknown artwork source")?;

    let (settings, cleared) = {
        let db = library.lock()?;
        let (old_source, old_fallback, _) = read_art_settings(&db)?;
        db.set_setting(STEAM_ART_SOURCE, source.as_str())
            .map_err(|e| e.to_string())?;
        db.set_setting(STEAM_ART_FALLBACK, if fallback { "1" } else { "0" })
            .map_err(|e| e.to_string())?;

        let cleared = if old_source != source || old_fallback != fallback {
            db.clear_images("steam").map_err(|e| e.to_string())?
        } else {
            Vec::new()
        };
        (art_settings_dto(&db)?, cleared)
    };

    if !cleared.is_empty() {
        let images = app_dir(&app)?.join("images");
        for id in &cleared {
            let _ = fs::remove_dir_all(images.join(dir_name(id)));
        }
        emit_library(&app);
        start_import(&app, false);
    }
    Ok(settings)
}

/// Checks and stores the SteamGridDB key. An empty key removes it. Fails with `invalid_key` if rejected.
// @rpc
async fn save_steamgriddb_key(library: State<'_, Library>, key: String) -> Result<bool, String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        library
            .lock()?
            .delete_setting(STEAMGRIDDB_KEY)
            .map_err(|e| e.to_string())?;
        return Ok(false);
    }

    let to_check = key.clone();
    host::async_runtime::spawn_blocking(move || Client::new(&to_check).validate())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| match e {
            ArtError::InvalidKey => "invalid_key".to_string(),
            other => other.to_string(),
        })?;

    library
        .lock()?
        .set_setting(STEAMGRIDDB_KEY, &key)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// Returns the bytes of a downloaded artwork file. Only files inside the artwork folder are readable.
// @rpc
async fn read_artwork(app: AppHandle, path: String) -> Result<Vec<u8>, String> {
    let images = fs::canonicalize(app_dir(&app)?.join("images")).map_err(|e| e.to_string())?;
    let file = fs::canonicalize(&path).map_err(|e| e.to_string())?;
    if !file.starts_with(&images) {
        return Err("not an artwork file".into());
    }
    fs::read(file).map_err(|e| e.to_string())
}

/// Downloads artwork in the background. Progress and results come back as events.
// @rpc
async fn import_images(app: AppHandle, force: bool) -> Result<(), String> {
    start_import(&app, force);
    Ok(())
}

/// Starts an artwork import unless one is already running.
fn start_import(app: &AppHandle, force: bool) {
    if app.state::<ImportRunning>().0.swap(true, Ordering::SeqCst) {
        return;
    }

    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(error) = run_import(&app, force) {
            applog::error(format!("Importing artwork failed: {error}"));
        }
        app.state::<ImportRunning>()
            .0
            .store(false, Ordering::SeqCst);
        let _ = app.emit(
            "images-progress",
            ImportProgress {
                done: 0,
                total: 0,
                running: false,
            },
        );
    });
}

/// The system's sound: volume and device of the speakers and of the microphone.
// @rpc
async fn audio_status() -> Result<audio::Status, String> {
    audio::status()
}

// @rpc
async fn set_audio_volume(kind: String, percent: u8) -> Result<(), String> {
    audio::set_volume(audio::Kind::parse(&kind)?, percent)
}

// @rpc
async fn set_audio_muted(kind: String, muted: bool) -> Result<(), String> {
    audio::set_muted(audio::Kind::parse(&kind)?, muted)
}

// @rpc
async fn set_audio_device(kind: String, name: String) -> Result<(), String> {
    audio::set_device(audio::Kind::parse(&kind)?, &name)
}

/// How the artwork of games is looked up, from the user's settings.
struct ArtPlan {
    agent: ureq::Agent,
    client: Option<Client>,
    steam_source: SteamArt,
    fallback: bool,
}

impl ArtPlan {
    /// Whether this game can get artwork with what is configured.
    fn can_serve(&self, target: &ImageTarget) -> bool {
        if self.client.is_some() {
            return true;
        }
        target.source == "steam" && (self.steam_source == SteamArt::Official || self.fallback)
    }

    fn official(
        &self,
        app_id: &str,
        folder: &Path,
    ) -> Result<(Option<Image>, Option<Image>), ArtError> {
        let poster = artwork::download_first(
            &self.agent,
            &artwork::steam_poster_urls(app_id),
            folder,
            "poster",
        )?;
        let hero = artwork::download_first(
            &self.agent,
            &artwork::steam_hero_urls(app_id),
            folder,
            "hero",
        )?;
        Ok((poster.map(image), hero.map(image)))
    }

    /// The SteamGridDB game of `target`: the one the user picked, or the best guess.
    fn sgdb_game(client: &Client, target: &ImageTarget) -> Result<Option<u64>, ArtError> {
        let steam_app = (target.source == "steam").then_some(target.source_id.as_str());
        Ok(match target.sgdb_id {
            Some(picked) => Some(picked),
            None => client.find_game(&target.title, steam_app)?,
        })
    }

    /// The game's logo: Steam's own for a Steam game shown with Steam's artwork, SteamGridDB's
    /// otherwise (with Steam's as the fallback when the user allows it).
    fn fetch_logo(&self, images: &Path, target: &ImageTarget) -> Result<Option<Image>, ArtError> {
        let folder = images.join(dir_name(&target.id));
        let is_steam = target.source == "steam";
        let official = || {
            artwork::download_first(
                &self.agent,
                &artwork::steam_logo_urls(&target.source_id),
                &folder,
                "logo",
            )
            .map(|saved| saved.map(image))
        };
        if is_steam && self.steam_source == SteamArt::Official && target.sgdb_id.is_none() {
            return official();
        }
        let custom = match &self.client {
            Some(client) => match Self::sgdb_game(client, target)? {
                Some(game) => match client.logo_url(game)? {
                    Some(url) => {
                        artwork::download_first(&self.agent, &[url], &folder, "logo")?.map(image)
                    }
                    None => None,
                },
                None => None,
            },
            None => None,
        };
        if custom.is_none() && is_steam && (self.fallback || self.client.is_none()) {
            return official();
        }
        Ok(custom)
    }

    fn custom(
        &self,
        client: &Client,
        target: &ImageTarget,
        folder: &Path,
    ) -> Result<(Option<Image>, Option<Image>), ArtError> {
        let Some(game) = Self::sgdb_game(client, target)? else {
            return Ok((None, None));
        };

        let save = |url: Option<String>, stem: &str| -> Result<Option<Image>, ArtError> {
            match url {
                Some(url) => artwork::download_first(&self.agent, &[url], folder, stem)
                    .map(|saved| saved.map(image)),
                None => Ok(None),
            }
        };
        let poster = save(client.poster_url(game)?, "poster")?;
        let hero = save(client.hero_url(game)?, "hero")?;
        Ok((poster, hero))
    }

    fn fetch(
        &self,
        images: &Path,
        target: &ImageTarget,
    ) -> Result<(Option<Image>, Option<Image>), ArtError> {
        let folder = images.join(dir_name(&target.id));
        let is_steam = target.source == "steam";

        if is_steam && self.steam_source == SteamArt::Official && target.sgdb_id.is_none() {
            return self.official(&target.source_id, &folder);
        }

        let (mut poster, mut hero) = match &self.client {
            Some(client) => self.custom(client, target, &folder)?,
            None => (None, None),
        };
        if is_steam && self.fallback && (poster.is_none() || hero.is_none()) {
            let (official_poster, official_hero) = self.official(&target.source_id, &folder)?;
            poster = poster.or(official_poster);
            hero = hero.or(official_hero);
        }
        Ok((poster, hero))
    }
}

fn image((path, url): (PathBuf, String)) -> Image {
    Image {
        path: path.to_string_lossy().into_owned(),
        url,
    }
}

fn run_import(app: &AppHandle, force: bool) -> Result<(), String> {
    let library = app.state::<Library>();
    let (plan, targets) = {
        let db = library.lock()?;
        let (steam_source, fallback, key) = read_art_settings(&db)?;
        let plan = ArtPlan {
            agent: artwork::agent(),
            client: key.as_deref().map(Client::new),
            steam_source,
            fallback,
        };
        let mut targets = db.needing_images(force).map_err(|e| e.to_string())?;
        targets.retain(|target| plan.can_serve(target));
        (plan, targets)
    };
    if targets.is_empty() {
        return Ok(());
    }

    applog::info(format!("Artwork: looking up {} game(s)", targets.len()));
    let images = app_dir(app)?.join("images");
    let total = targets.len();
    let _ = app.emit(
        "images-progress",
        ImportProgress {
            done: 0,
            total,
            running: true,
        },
    );

    for (index, target) in targets.iter().enumerate() {
        if target.needs_logo {
            match plan.fetch_logo(&images, target) {
                Ok(logo) => {
                    applog::info(format!(
                        "Logo for {}: {}",
                        target.title,
                        logo.as_ref().map_or("none", |logo| logo.url.as_str())
                    ));
                    library
                        .lock()?
                        .set_logo(&target.id, logo.as_ref(), now())
                        .map_err(|e| e.to_string())?;
                }
                Err(ArtError::InvalidKey) => return Err("the SteamGridDB key was rejected".into()),
                Err(error) => applog::warn(format!("No logo for {}: {error}", target.title)),
            }
        }
        if !target.needs_images {
            // Only the logo was missing: show it.
            if let Some(game) = library.lock()?.get(&target.id).map_err(|e| e.to_string())? {
                let _ = app.emit("game-updated", game);
            }
        } else {
            match plan.fetch(&images, target) {
                Ok((poster, hero)) => {
                    let found = |image: &Option<Image>| {
                        image
                            .as_ref()
                            .map_or("none".to_string(), |image| image.url.clone())
                    };
                    applog::info(format!(
                        "Artwork for {}: cover {}, banner {}",
                        target.title,
                        found(&poster),
                        found(&hero)
                    ));
                    let updated = {
                        let db = library.lock()?;
                        db.set_images(&target.id, poster.as_ref(), hero.as_ref(), now())
                            .map_err(|e| e.to_string())?;
                        db.get(&target.id).map_err(|e| e.to_string())?
                    };
                    // Only this game changed: the interface can redraw just its card.
                    if let Some(game) = updated {
                        let _ = app.emit("game-updated", game);
                    }
                }
                Err(ArtError::InvalidKey) => return Err("the SteamGridDB key was rejected".into()),
                // A network hiccup: leave the game unchecked so it is retried next time.
                Err(error) => applog::warn(format!("No artwork for {}: {error}", target.title)),
            }
        }
        let _ = app.emit(
            "images-progress",
            ImportProgress {
                done: index + 1,
                total,
                running: true,
            },
        );
    }
    applog::info("Artwork: done");
    Ok(())
}

/// The SteamGridDB game used for a game's artwork: the one the user picked, or the best guess.
// @rpc
async fn artwork_match(
    library: State<'_, Library>,
    id: String,
) -> Result<Option<SearchHit>, String> {
    let (key, game, info) = {
        let db = library.lock()?;
        let key = db
            .setting(STEAMGRIDDB_KEY)
            .map_err(|e| e.to_string())?
            .ok_or("no_key")?;
        let game = db
            .get(&id)
            .map_err(|e| e.to_string())?
            .ok_or("unknown game")?;
        let info = db
            .launch_info(&id)
            .map_err(|e| e.to_string())?
            .ok_or("unknown game")?;
        (key, game, info)
    };

    host::async_runtime::spawn_blocking(move || {
        let client = Client::new(&key);
        let steam_app = (info.source == "steam").then_some(info.source_id.as_str());
        let found = match game.sgdb_game_id.and_then(|id| u64::try_from(id).ok()) {
            Some(picked) => Some(picked),
            None => client.find_game(&game.title, steam_app)?,
        };
        match found {
            Some(found) => client.game_info(found),
            None => Ok(None),
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(art_error)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtworkChoices {
    posters: Vec<Candidate>,
    heroes: Vec<Candidate>,
}

/// Every image the user can choose from: Steam's own (for Steam games) and SteamGridDB's.
// @rpc
async fn artwork_choices(
    library: State<'_, Library>,
    id: String,
    sgdb_game: Option<u64>,
) -> Result<ArtworkChoices, String> {
    let (key, info) = {
        let db = library.lock()?;
        (
            db.setting(STEAMGRIDDB_KEY).map_err(|e| e.to_string())?,
            db.launch_info(&id)
                .map_err(|e| e.to_string())?
                .ok_or("unknown game")?,
        )
    };

    host::async_runtime::spawn_blocking(move || {
        let official = |urls: Vec<String>| -> Vec<Candidate> {
            urls.into_iter()
                .take(1)
                .map(|url| Candidate {
                    thumb: url.clone(),
                    url,
                    width: None,
                    height: None,
                    author: None,
                    style: Some("official".into()),
                })
                .collect()
        };
        let mut choices = ArtworkChoices {
            posters: Vec::new(),
            heroes: Vec::new(),
        };
        if info.source == "steam" {
            choices.posters = official(artwork::steam_poster_urls(&info.source_id));
            choices.heroes = official(artwork::steam_hero_urls(&info.source_id));
        }
        if let (Some(key), Some(game)) = (key, sgdb_game) {
            let client = Client::new(&key);
            choices.posters.extend(client.poster_candidates(game)?);
            choices.heroes.extend(client.hero_candidates(game)?);
        }
        Ok(choices)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(art_error)
}

/// Downloads one image chosen by the user as a game's cover (`poster`) or banner (`hero`).
// @rpc
async fn set_artwork(
    app: AppHandle,
    library: State<'_, Library>,
    id: String,
    kind: String,
    url: String,
) -> Result<Game, String> {
    let stem = match kind.as_str() {
        "poster" | "hero" => kind.clone(),
        other => return Err(format!("unknown artwork kind: {other}")),
    };
    let folder = app_dir(&app)?.join("images").join(dir_name(&id));
    let source = url.clone();
    let saved = host::async_runtime::spawn_blocking(move || {
        artwork::download_first(&artwork::agent(), &[source], &folder, &stem)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(art_error)?
    .ok_or("this image is no longer available")?;

    let saved = image(saved);
    let game = {
        let db = library.lock()?;
        let (poster, hero) = if kind == "poster" {
            (Some(&saved), None)
        } else {
            (None, Some(&saved))
        };
        db.set_images(&id, poster, hero, now())
            .map_err(|e| e.to_string())?;
        db.get(&id)
            .map_err(|e| e.to_string())?
            .ok_or("unknown game")?
    };
    applog::info(format!("Artwork for {}: {kind} set to {url}", game.title));
    let _ = app.emit("game-updated", game.clone());
    Ok(game)
}

fn art_error(error: ArtError) -> String {
    match error {
        ArtError::InvalidKey => "invalid_key".to_string(),
        other => other.to_string(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LogContent {
    path: String,
    text: String,
    /// Older lines were left out to keep the view light.
    truncated: bool,
}

/// The file behind one of a game's logs: `game` (this run's output), `previous` (the run
/// before), `proton` or `graphics` (written in debug mode only).
fn game_log_path(logs: &Path, id: &str, kind: &str) -> Option<PathBuf> {
    let name = dir_name(id);
    match kind {
        "game" => Some(logs.join(format!("{name}.log"))),
        "previous" => Some(logs.join(format!("{name}.prev.log"))),
        "proton" | "graphics" => {
            let dir = logs.join(format!("{name}.d"));
            let mut files: Vec<PathBuf> = fs::read_dir(dir)
                .ok()?
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "log"))
                .collect();
            files.sort();
            let is_proton = |path: &PathBuf| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("steam-"))
            };
            // Graphics: DXVK writes one log per application; VKD3D's is `vkd3d.log`.
            files
                .into_iter()
                .find(|path| (kind == "proton") == is_proton(path))
        }
        _ => None,
    }
}

/// The end of a log: Lutra's own (`app`) or one of a game's logs (see `game_log_path`).
// @rpc
async fn read_log(
    app: AppHandle,
    target: String,
    kind: Option<String>,
) -> Result<LogContent, String> {
    let logs = app_dir(&app)?.join("logs");
    let path = if target == "app" {
        logs.join("lutra.log")
    } else {
        game_log_path(&logs, &target, kind.as_deref().unwrap_or("game"))
            .ok_or("this log does not exist")?
    };
    let (text, truncated) = applog::read_tail(&path, 256 * 1024).map_err(|e| e.to_string())?;
    Ok(LogContent {
        path: path.to_string_lossy().into_owned(),
        text,
        truncated,
    })
}

/// Which logs a game has right now, in the order to show them.
// @rpc
async fn game_logs(app: AppHandle, id: String) -> Result<Vec<String>, String> {
    let logs = app_dir(&app)?.join("logs");
    Ok(["game", "previous", "proton", "graphics"]
        .into_iter()
        .filter(|kind| game_log_path(&logs, &id, kind).is_some_and(|path| path.is_file()))
        .map(str::to_string)
        .collect())
}

/// Lets the interface write its own errors into Lutra's log.
// @rpc
async fn log_frontend(level: String, message: String) -> Result<(), String> {
    applog::write(applog::Level::parse(&level), &format!("[ui] {message}"));
    Ok(())
}

// @rpc
async fn open_logs_folder(app: AppHandle) -> Result<(), String> {
    let folder = app_dir(&app)?.join("logs");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    wine::open_on_host(folder)
}

fn load_profile(db: &Db, owner: &str) -> Result<Option<Profile>, String> {
    Ok(db
        .controller_profile(owner)
        .map_err(|e| e.to_string())?
        .and_then(|json| serde_json::from_str(&json).ok()))
}

/// The profile a game plays with: its own, or the default one.
fn profile_for(db: &Db, game: &str) -> Result<Profile, String> {
    Ok(match load_profile(db, game)? {
        Some(profile) => profile,
        None => load_profile(db, DEFAULT_PROFILE)?.unwrap_or_default(),
    })
}

/// Hands the controllers to a game, and adds to `spec` what hides the physical controllers from
/// it (see `ControllerService::hidden_from_games`). Call it just before spawning the game: the
/// virtual controller must exist before the game looks for controllers.
fn take_controllers(app: &AppHandle, game: &str, spec: &mut LaunchSpec) {
    let profile = match app
        .state::<Library>()
        .lock()
        .and_then(|db| profile_for(&db, game))
    {
        Ok(profile) => profile,
        Err(error) => {
            applog::warn(format!("No controller profile for {game}: {error}"));
            return;
        }
    };
    let service = app.state::<ControllerService>();
    if profile.output == OutputKind::None {
        // Left as they are, even by the system-wide driver.
        service.start_session(game, profile);
        return;
    }
    if profile.output == OutputKind::Ds4 && game.starts_with("wine:") {
        applog::warn(
            "This Windows game gets a PlayStation 4 controller: it only exists for DirectInput, \
             so games that read XInput will not see it (choose Automatic or Xbox 360)",
        );
    }
    if !controller::uinput_available() {
        applog::warn("Controllers stay as they are: /dev/uinput is not writable");
        return;
    }
    service.start_session(game, profile);
    if let Some(ids) = service.hidden_from_games() {
        applog::info(format!("Physical controllers hidden from {game}: {ids}"));
        add_to_list_env(spec, SDL_IGNORE_DEVICES, &ids);
    }
}

/// The controllers SDL (and Wine, and Proton) must not open.
const SDL_IGNORE_DEVICES: &str = "SDL_GAMECONTROLLER_IGNORE_DEVICES";

/// Adds `value` to a comma-separated variable, keeping what the game's settings already put in it.
fn add_to_list_env(spec: &mut LaunchSpec, key: &str, value: &str) {
    match spec.envs.iter_mut().find(|(name, _)| name == key) {
        Some((_, existing)) if !existing.is_empty() => {
            existing.push(",");
            existing.push(value);
        }
        Some((_, existing)) => *existing = value.into(),
        None => spec.envs.push((key.to_string(), value.into())),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ControllerSession {
    owner: String,
    output: OutputKind,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ControllerStatus {
    controllers: Vec<ControllerView>,
    /// Whether virtual controllers can be created at all.
    uinput: bool,
    /// Whether HID controllers can be created: a PlayStation 4 controller needs it for games to
    /// show PlayStation buttons.
    uhid: bool,
    session: Option<ControllerSession>,
    navigation: bool,
    /// Lutra drives the controllers system-wide when no game runs.
    system: bool,
    /// Steam runs, so the system-wide driver leaves the controllers to Steam Input.
    steam: bool,
}

/// A connected controller, with its battery when it has one.
#[derive(Serialize)]
struct ControllerView {
    #[serde(flatten)]
    info: ControllerInfo,
    battery: Option<controller::battery::Battery>,
}

// @rpc
async fn controller_status(
    service: State<'_, ControllerService>,
) -> Result<ControllerStatus, String> {
    Ok(ControllerStatus {
        controllers: service
            .controllers()
            .into_iter()
            .map(|info| ControllerView {
                battery: controller::battery::read(Path::new(&info.path)),
                info,
            })
            .collect(),
        uinput: controller::uinput_available(),
        uhid: controller::uhid_available(),
        session: service
            .session()
            .map(|(owner, output)| ControllerSession { owner, output }),
        navigation: service.navigation(),
        system: service.system(),
        steam: service.steam(),
    })
}

/// What a controller is doing right now, before any profile is applied: for the live tester.
// @rpc
async fn controller_state(
    service: State<'_, ControllerService>,
    id: String,
) -> Result<Option<PadSnapshot>, String> {
    Ok(service.state(&id))
}

#[derive(Serialize)]
struct PadPreview {
    /// What the controller does.
    raw: PadSnapshot,
    /// What the game would receive with `profile`.
    output: PadSnapshot,
}

/// A controller's state before and after a profile, for the test screen. Nothing reaches a game.
// @rpc
async fn controller_preview(
    service: State<'_, ControllerService>,
    id: String,
    profile: Profile,
) -> Result<Option<PadPreview>, String> {
    Ok(service.raw_state(&id).map(|raw| PadPreview {
        raw: raw.snapshot(),
        output: apply(&profile, &raw).snapshot(),
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileInfo {
    profile: Profile,
    /// For a game: whether it has its own profile rather than the default one.
    custom: bool,
}

/// The default profile (`game: None`) or the one a game plays with.
// @rpc
async fn controller_profile(
    library: State<'_, Library>,
    game: Option<String>,
) -> Result<ProfileInfo, String> {
    let db = library.lock()?;
    Ok(match game {
        None => ProfileInfo {
            profile: load_profile(&db, DEFAULT_PROFILE)?.unwrap_or_default(),
            custom: false,
        },
        Some(game) => match load_profile(&db, &game)? {
            Some(profile) => ProfileInfo {
                profile,
                custom: true,
            },
            None => ProfileInfo {
                profile: load_profile(&db, DEFAULT_PROFILE)?.unwrap_or_default(),
                custom: false,
            },
        },
    })
}

/// Saves the default profile (`game: None`) or a game's own profile.
// @rpc
async fn save_controller_profile(
    library: State<'_, Library>,
    service: State<'_, ControllerService>,
    game: Option<String>,
    profile: Profile,
) -> Result<(), String> {
    if let Some(unknown) = profile
        .keys
        .values()
        .find(|name| controller::convert::target(name).is_none())
    {
        return Err(format!("Unknown key: {unknown}"));
    }
    let owner = game.as_deref().unwrap_or(DEFAULT_PROFILE).to_string();
    let json = serde_json::to_string(&profile).map_err(|e| e.to_string())?;
    library
        .lock()?
        .set_controller_profile(&owner, &json)
        .map_err(|e| e.to_string())?;
    applog::info(format!("Controller profile saved for {owner}"));
    if game.is_none() {
        service.set_system_profile(profile.clone());
    }

    // A running test follows the profile being edited.
    if service
        .session()
        .is_some_and(|(session, _)| session == TEST_SESSION)
        && game.is_none()
    {
        service.start_session(TEST_SESSION, profile);
    }
    Ok(())
}

/// The keys and mouse buttons a controller button can be converted to.
// @rpc
async fn controller_keys() -> Vec<controller::convert::KeyInfo> {
    controller::convert::key_list()
}

/// Makes a game use the default profile again.
// @rpc
async fn reset_controller_profile(library: State<'_, Library>, game: String) -> Result<(), String> {
    library
        .lock()?
        .delete_controller_profile(&game)
        .map_err(|e| e.to_string())
}

/// Turns the virtual controller on without a game, to try a profile in any game or tester.
// @rpc
async fn start_controller_test(
    service: State<'_, ControllerService>,
    profile: Profile,
) -> Result<(), String> {
    if !controller::uinput_available() {
        return Err("uinput_unavailable".into());
    }
    if let Some((owner, _)) = service.session().filter(|(owner, _)| owner != TEST_SESSION) {
        return Err(format!("the controllers are in use by {owner}"));
    }
    service.start_session(TEST_SESSION, profile);
    Ok(())
}

// @rpc
async fn stop_controller_test(service: State<'_, ControllerService>) -> Result<(), String> {
    service.end_session(TEST_SESSION);
    Ok(())
}

// @rpc
async fn set_controller_navigation(
    library: State<'_, Library>,
    service: State<'_, ControllerService>,
    enabled: bool,
) -> Result<(), String> {
    library
        .lock()?
        .set_setting(CONTROLLER_NAVIGATION, if enabled { "1" } else { "0" })
        .map_err(|e| e.to_string())?;
    service.set_navigation(enabled);
    Ok(())
}

// @rpc
async fn set_controller_system(
    library: State<'_, Library>,
    service: State<'_, ControllerService>,
    enabled: bool,
) -> Result<(), String> {
    library
        .lock()?
        .set_setting(CONTROLLER_SYSTEM, if enabled { "1" } else { "0" })
        .map_err(|e| e.to_string())?;
    service.set_system(enabled);
    Ok(())
}

// @rpc
async fn tray_settings(app: AppHandle) -> Result<tray::TraySettings, String> {
    Ok(tray::settings(&app))
}

// @rpc
async fn set_tray_settings(app: AppHandle, settings: tray::TraySettings) -> Result<(), String> {
    tray::save_settings(&app, &settings)?;
    applog::info("Notification area settings saved");
    Ok(())
}

/// Starts the backend: sets everything up, then serves the shell's requests until it goes away.
pub fn run() {
    let Some(dir) = data_dir_early() else {
        eprintln!("HOME is not set");
        std::process::exit(1);
    };
    let app = AppHandle::new(dir.clone());
    if let Err(error) = setup(&app, &dir) {
        eprintln!("could not start: {error}");
        std::process::exit(1);
    }
    rpc::serve(&app);
}

fn setup(app: &AppHandle, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    applog::init(&dir.join("logs").join("lutra.log"))?;
    applog::info(format!("Lutra {} started", env!("CARGO_PKG_VERSION")));
    app.manage(Library(Mutex::new(Db::open(&dir.join("library.db"))?)));
    app.manage(ImportRunning(AtomicBool::new(false)));
    app.manage(Running(Mutex::new(HashMap::new())));
    tray::setup(app);

    let (navigation, system, system_profile) = {
        let db = app.state::<Library>();
        let db = db.lock()?;
        let enabled = |key: &str| {
            db.setting(key)
                .ok()
                .flatten()
                .is_none_or(|value| value != "0")
        };
        (
            enabled(CONTROLLER_NAVIGATION),
            enabled(CONTROLLER_SYSTEM),
            profile_for(&db, DEFAULT_PROFILE).unwrap_or_default(),
        )
    };
    let nav_handle = app.clone();
    let change_handle = app.clone();
    app.manage(ControllerService::start(
        navigation,
        system,
        system_profile,
        // The overlay is the backend's to open or close; every other menu action goes to the
        // shell, which hands it to the overlay when it is open, else to Lutra when it is in use.
        Arc::new(move |action| match action.as_str() {
            "overlay" => toggle_overlay(&nav_handle),
            _ => {
                let _ = nav_handle.emit("ui-action", action.as_str());
            }
        }),
        Arc::new(move || {
            let _ = change_handle.emit("controllers-changed", ());
        }),
    ));
    tray::announce(app);

    let watcher = app.clone();
    std::thread::spawn(move || watch_batteries(&watcher));
    Ok(())
}

/// How often the controllers' batteries are looked at.
const BATTERY_CHECK: Duration = Duration::from_secs(20);

/// Tells the interface when a controller's battery runs low (30 %, then 15 %), even while the
/// window is hidden in the notification area.
fn watch_batteries(app: &AppHandle) {
    let mut alerts = controller::battery::Alerts::default();
    std::thread::sleep(Duration::from_secs(3));
    loop {
        for info in app.state::<ControllerService>().controllers() {
            let Some(battery) = controller::battery::read(Path::new(&info.path)) else {
                continue;
            };
            if let Some(level) = alerts.check(&info.id, battery) {
                applog::info(format!(
                    "{} battery at {} % (alert at {level} %)",
                    info.name, battery.percent
                ));
                let _ = app.emit(
                    "controller-battery",
                    json!({
                        "id": info.id,
                        "name": info.name,
                        "percent": battery.percent,
                        "level": level,
                    }),
                );
            }
        }
        std::thread::sleep(BATTERY_CHECK);
    }
}

include!("rpc_dispatch.rs");
