//! Running Windows programs: finding Wine and Proton installs, per-game settings, and building
//! the launch command (in the spirit of Lutris' Wine runner options).

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::hardware::{self, GpuVendor, Hardware};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunnerKind {
    Wine,
    Proton,
    /// A Linux program run as is, with no compatibility layer.
    Native,
}

impl RunnerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RunnerKind::Wine => "wine",
            RunnerKind::Proton => "proton",
            RunnerKind::Native => "native",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "wine" => Some(RunnerKind::Wine),
            "proton" => Some(RunnerKind::Proton),
            "native" => Some(RunnerKind::Native),
            _ => None,
        }
    }
}

/// Gamescope, Valve's game compositor: a fixed-size game window with upscaling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Gamescope {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    /// AMD FidelityFX Super Resolution upscaling.
    pub fsr: bool,
}

impl Default for Gamescope {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fullscreen: true,
            fsr: false,
        }
    }
}

/// A Wine binary or a Proton `proton` script that can run Windows programs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Runner {
    pub kind: RunnerKind,
    pub label: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

/// Per-game options. Stored as JSON, so new options can be added without a migration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WineConfig {
    /// Command-line arguments passed to the game, as typed (quotes allowed).
    pub args: String,
    /// Defaults to the program's folder.
    pub working_dir: Option<String>,
    pub env: Vec<EnvVar>,
    /// `WINEDLLOVERRIDES`, e.g. `d3d11=n,b;dinput8=n,b`.
    pub dll_overrides: String,
    pub esync: bool,
    pub fsync: bool,
    /// Proton only: use WineD3D (OpenGL) instead of DXVK/VKD3D.
    pub wined3d: bool,
    /// Wine only: run inside a virtual desktop of this size, e.g. `1920x1080`.
    pub virtual_desktop: Option<String>,
    /// Wine only: create the prefix as 32-bit. Only matters before the prefix exists.
    pub arch32: bool,
    pub gamemode: bool,
    pub mangohud: bool,
    /// Keep Wine/Proton debug output (very verbose) instead of silencing it.
    pub debug: bool,
    /// PCI address of the GPU the game must render on; `None` picks the best one (the dedicated
    /// GPU of a hybrid machine).
    pub gpu: Option<String>,
    /// Frame rate cap (DXVK / VKD3D-Proton / MangoHud).
    pub fps_limit: Option<u32>,
    /// DXVK's on-screen display: `fps`, `full`...
    pub dxvk_hud: Option<String>,
    /// Proton only: the kernel's NT synchronisation driver (needs `/dev/ntsync`).
    pub ntsync: bool,
    /// Proton only: DXVK-NVAPI, for DLSS and NVIDIA-specific features.
    pub nvapi: bool,
    /// Proton only: Wayland driver instead of XWayland.
    pub wayland: bool,
    /// Proton only: FidelityFX upscaling of lower resolutions in fullscreen.
    pub fsr: bool,
    /// Driver caches and threading that every game benefits from.
    pub perf_tweaks: bool,
    pub gamescope: Option<Gamescope>,
}

impl Default for WineConfig {
    fn default() -> Self {
        Self {
            args: String::new(),
            working_dir: None,
            env: Vec::new(),
            dll_overrides: String::new(),
            esync: true,
            fsync: true,
            wined3d: false,
            virtual_desktop: None,
            arch32: false,
            gamemode: false,
            mangohud: false,
            debug: false,
            gpu: None,
            fps_limit: None,
            dxvk_hud: None,
            ntsync: false,
            nvapi: false,
            wayland: false,
            fsr: false,
            perf_tweaks: true,
            gamescope: None,
        }
    }
}

/// The options that suit this machine best (what Lutris would tune by hand): the discrete GPU on
/// a hybrid laptop, NT synchronisation when the kernel has it, NVAPI on NVIDIA, GameMode if installed.
pub fn recommended_config(
    hardware: &Hardware,
    kind: RunnerKind,
    gamemode_installed: bool,
) -> WineConfig {
    let mut config = WineConfig {
        gamemode: gamemode_installed,
        ..WineConfig::default()
    };
    if let Some(gpu) = hardware.preferred_gpu() {
        if hardware.is_hybrid() {
            config.gpu = Some(gpu.id.clone());
        }
        config.nvapi = kind == RunnerKind::Proton && gpu.vendor == GpuVendor::Nvidia;
    }
    config.ntsync = kind == RunnerKind::Proton && hardware.ntsync;
    config
}

/// Helper programs found on this machine.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    pub wine: Option<PathBuf>,
    pub wine_version: Option<String>,
    pub umu: Option<PathBuf>,
    pub winetricks: Option<PathBuf>,
    pub gamemode: Option<PathBuf>,
    pub mangohud: Option<PathBuf>,
}

pub fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// What to change in a child's environment so it sees the system and not Lutra's AppImage.
///
/// The AppImage's `AppRun` points `PYTHONHOME`, `LD_LIBRARY_PATH`, `PERLLIB`, `GST_*`, `GTK_*`
/// and more at its own mount. A game inherits them, and `umu-run` (a Python script) then dies
/// looking for `encodings` in a folder that has no Python. Every entry under `appdir` is removed;
/// a variable left empty is unset. Returns `(name, new value)`, `None` meaning unset.
pub fn host_env_changes(
    vars: impl IntoIterator<Item = (String, String)>,
    appdir: &str,
) -> Vec<(String, Option<String>)> {
    let appdir = appdir.trim_end_matches('/');
    if appdir.is_empty() {
        return Vec::new();
    }
    let inside = |entry: &str| {
        entry
            .strip_prefix(appdir)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    };
    vars.into_iter()
        .filter(|(name, value)| name != "APPDIR" && value.split(':').any(inside))
        .map(|(name, value)| {
            let kept: Vec<&str> = value.split(':').filter(|entry| !inside(entry)).collect();
            let kept = kept.join(":");
            (name, (!kept.is_empty()).then_some(kept))
        })
        .collect()
}

/// Opens a folder or a URL with the desktop's default application (`xdg-open`), with the
/// environment of the system rather than the AppImage's, which would break the application that
/// opens it (Steam for `steam://`, the file manager for a folder).
pub fn open_on_host(target: impl AsRef<std::ffi::OsStr>) -> Result<(), String> {
    let mut command = Command::new("xdg-open");
    host_env(&mut command)
        .arg(target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not run xdg-open: {e}"))
}

/// Gives `command` the environment of the system instead of the one set up by Lutra's AppImage.
/// Does nothing outside an AppImage.
pub fn host_env(command: &mut Command) -> &mut Command {
    let Some(appdir) = std::env::var_os("APPDIR").and_then(|dir| dir.into_string().ok()) else {
        return command;
    };
    let vars = std::env::vars().filter(|(name, _)| name != "APPDIR");
    for (name, value) in host_env_changes(vars, &appdir) {
        match value {
            Some(value) => command.env(name, value),
            None => command.env_remove(name),
        };
    }
    command
}

pub fn detect_tools() -> Tools {
    let wine = find_in_path("wine");
    let wine_version = wine.as_ref().and_then(|wine| {
        let output = host_env(Command::new(wine).arg("--version"))
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!text.is_empty()).then_some(text)
    });
    Tools {
        wine,
        wine_version,
        umu: find_in_path("umu-run"),
        winetricks: find_in_path("winetricks"),
        gamemode: find_in_path("gamemoderun"),
        mangohud: find_in_path("mangohud"),
    }
}

fn proton_in(dir: &Path, seen: &mut HashSet<PathBuf>, out: &mut Vec<Runner>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let script = entry.path().join("proton");
        if !script.is_file() {
            continue;
        }
        if !seen.insert(fs::canonicalize(&script).unwrap_or_else(|_| script.clone())) {
            continue;
        }
        out.push(Runner {
            kind: RunnerKind::Proton,
            label: entry.file_name().to_string_lossy().into_owned(),
            path: script,
        });
    }
}

/// Proton builds from Steam and from `compatibilitytools.d` (GE-Proton and friends), the builds
/// Lutra downloaded itself (`managed` is its data folder, see `runners.rs`), then system Wine.
pub fn detect_runners(steam_roots: &[PathBuf], managed: Option<&Path>) -> Vec<Runner> {
    let mut seen = HashSet::new();
    let mut protons = Vec::new();

    for root in steam_roots {
        proton_in(&root.join("steamapps/common"), &mut seen, &mut protons);
        proton_in(&root.join("compatibilitytools.d"), &mut seen, &mut protons);
    }
    let mut wines = Vec::new();
    for build in managed.map(crate::runners::installed).unwrap_or_default() {
        let runner = Runner {
            kind: build.kind,
            label: build.name,
            path: build.path,
        };
        match build.kind {
            RunnerKind::Wine => wines.push(runner),
            _ => protons.push(runner),
        }
    }
    protons.sort_by_key(|runner| std::cmp::Reverse(runner.label.to_lowercase()));

    let mut runners = protons;
    runners.extend(wines);
    if let Some(wine) = find_in_path("wine") {
        runners.push(Runner {
            kind: RunnerKind::Wine,
            label: "Wine".into(),
            path: wine,
        });
    }
    runners
}

/// What to run inside the prefix.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// A Windows program, with the game's arguments and working folder.
    Program(&'a Path),
    /// Wine's configuration window.
    Winecfg,
    /// Winetricks, at the given path.
    Winetricks(&'a Path),
}

/// Everything needed to start something, independent of how it is spawned.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub envs: Vec<(String, OsString)>,
    pub cwd: PathBuf,
}

pub struct LaunchRequest<'a> {
    pub runner: &'a Runner,
    pub prefix: &'a Path,
    pub config: &'a WineConfig,
    pub target: Target<'a>,
    pub umu: Option<&'a Path>,
    pub steam_root: Option<&'a Path>,
    pub gamemode: Option<&'a Path>,
    pub mangohud: Option<&'a Path>,
    /// Needed to resolve `config.gpu` and `config.gamescope`.
    pub hardware: Option<&'a Hardware>,
    /// Where the runner and graphics layers write their own logs when `config.debug` is on.
    pub log_dir: Option<&'a Path>,
}

/// Proton keeps its Wine prefix in a `pfx` subfolder.
pub fn effective_prefix(kind: RunnerKind, prefix: &Path) -> PathBuf {
    match kind {
        RunnerKind::Wine | RunnerKind::Native => prefix.to_path_buf(),
        RunnerKind::Proton => prefix.join("pfx"),
    }
}

fn os(text: &str) -> OsString {
    OsString::from(text)
}

/// Makes sure a native program can be started, so the user cannot pick something that would only
/// fail at launch: it must be a Linux binary or a script, and gets its execute bit if it lacks it.
pub fn ensure_runnable(exe: &Path) -> Result<(), String> {
    use std::io::Read;
    use std::os::unix::fs::PermissionsExt;

    let mut magic = [0u8; 4];
    fs::File::open(exe)
        .and_then(|mut file| file.read(&mut magic))
        .map_err(|error| format!("cannot read {}: {error}", exe.display()))?;
    if magic.starts_with(b"MZ") {
        return Err("this is a Windows program: add it as a Windows game instead".into());
    }
    let known = magic == *b"\x7fELF" || magic.starts_with(b"#!");
    if !known {
        return Err("this is not a Linux program (expected a binary, AppImage or script)".into());
    }
    let metadata = fs::metadata(exe).map_err(|error| error.to_string())?;
    let mode = metadata.permissions().mode();
    if mode & 0o111 == 0 {
        let mut permissions = metadata.permissions();
        permissions.set_mode(mode | 0o755);
        fs::set_permissions(exe, permissions)
            .map_err(|error| format!("cannot make {} executable: {error}", exe.display()))?;
    }
    Ok(())
}

/// Merges `WINEDLLOVERRIDES` lists (`a=n,b;c=d`): later lists win per library, order is kept.
pub fn merge_dll_overrides(lists: &[&str]) -> String {
    let mut merged: Vec<(String, String)> = Vec::new();
    for entry in lists.iter().flat_map(|list| list.split(';')) {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (libs, mode) = entry.split_once('=').unwrap_or((entry, ""));
        for lib in libs.split(',').map(str::trim).filter(|lib| !lib.is_empty()) {
            match merged.iter_mut().find(|(name, _)| name == lib) {
                Some((_, existing)) => *existing = mode.to_string(),
                None => merged.push((lib.to_string(), mode.to_string())),
            }
        }
    }
    merged
        .into_iter()
        .map(|(lib, mode)| format!("{lib}={mode}"))
        .collect::<Vec<_>>()
        .join(";")
}

/// Builds the command that runs `request.target` with the runner and the game's options.
///
/// Proton goes through `umu-run` when available (it provides the Steam Linux Runtime container
/// that Proton expects outside Steam); otherwise the `proton` script is called directly.
pub fn build_launch(request: &LaunchRequest) -> Result<LaunchSpec, String> {
    let LaunchRequest {
        runner,
        prefix,
        config,
        target,
        ..
    } = request;
    let kind = runner.kind;
    let is_wine_like = kind != RunnerKind::Native;

    // What the runner is asked to execute.
    let mut payload: Vec<OsString> = Vec::new();
    let mut cwd = config.working_dir.clone().map(PathBuf::from);
    match target {
        Target::Program(exe) => {
            if let Some(size) = config
                .virtual_desktop
                .as_deref()
                .filter(|_| kind == RunnerKind::Wine)
            {
                payload.push(os("explorer"));
                payload.push(os(&format!("/desktop=Lutra,{size}")));
            }
            payload.push(exe.as_os_str().to_owned());
            let args =
                shlex::split(&config.args).ok_or("the launch arguments have an unclosed quote")?;
            payload.extend(args.iter().map(|arg| os(arg)));
            cwd = cwd.or_else(|| exe.parent().map(Path::to_path_buf));
        }
        Target::Winecfg => payload.push(os("winecfg")),
        Target::Winetricks(_) => {}
    }
    let cwd = cwd.unwrap_or_else(|| prefix.to_path_buf());

    let mut envs: Vec<(String, OsString)> = Vec::new();
    if is_wine_like {
        // Errors stay visible even without the debug option: a missing DLL or a failed graphics
        // call is the first thing to read in a game's log. Everything else is silenced.
        envs.push((
            "WINEDEBUG".into(),
            os(if config.debug {
                ""
            } else {
                "fixme-all,err+all"
            }),
        ));
        // No desktop entries or file associations created behind the user's back (Wine only:
        // Proton's runtime already handles it).
        let base = if kind == RunnerKind::Wine {
            "winemenubuilder.exe=d"
        } else {
            ""
        };
        let overrides = merge_dll_overrides(&[base, config.dll_overrides.trim()]);
        if !overrides.is_empty() {
            envs.push(("WINEDLLOVERRIDES".into(), os(&overrides)));
        }
    }

    // GPU selection, driver tweaks and display options apply to every kind of game. Without a
    // choice (or when the chosen GPU is gone), the game gets the best one.
    if let Some(hardware) = request.hardware
        && let Some(gpu) = config
            .gpu
            .as_deref()
            .and_then(|id| hardware.gpu(id))
            .or_else(|| hardware.preferred_gpu())
    {
        for (key, value) in hardware::gpu_env(hardware, gpu) {
            envs.push((key, os(&value)));
        }
    }
    if config.perf_tweaks {
        for (key, value) in [
            ("__GL_THREADED_OPTIMIZATIONS", "1"),
            ("__GL_SHADER_DISK_CACHE", "1"),
            ("__GL_SHADER_DISK_CACHE_SKIP_CLEANUP", "1"),
            ("mesa_glthread", "true"),
        ] {
            envs.push((key.into(), os(value)));
        }
        if kind == RunnerKind::Wine {
            envs.push(("WINE_LARGE_ADDRESS_AWARE".into(), os("1")));
            envs.push(("STAGING_SHARED_MEMORY".into(), os("1")));
        }
    }
    if let Some(limit) = config.fps_limit.filter(|limit| *limit > 0) {
        envs.push(("DXVK_FRAME_RATE".into(), os(&limit.to_string())));
        envs.push(("VKD3D_FRAME_RATE".into(), os(&limit.to_string())));
        if config.mangohud {
            envs.push(("MANGOHUD_CONFIG".into(), os(&format!("fps_limit={limit}"))));
        }
    }
    if config.debug
        && is_wine_like
        && let Some(dir) = request.log_dir
    {
        envs.push(("DXVK_LOG_LEVEL".into(), os("info")));
        envs.push(("DXVK_LOG_PATH".into(), dir.as_os_str().to_owned()));
        envs.push(("VKD3D_DEBUG".into(), os("warn")));
        envs.push((
            "VKD3D_LOG_FILE".into(),
            dir.join("vkd3d.log").into_os_string(),
        ));
        if kind == RunnerKind::Proton {
            envs.push(("PROTON_LOG_DIR".into(), dir.as_os_str().to_owned()));
        }
    }
    if let Some(hud) = config.dxvk_hud.as_deref().filter(|hud| !hud.is_empty()) {
        envs.push(("DXVK_HUD".into(), os(hud)));
    }

    let (mut program, mut args): (PathBuf, Vec<OsString>) = match (kind, target) {
        (RunnerKind::Native, Target::Program(exe)) => {
            // `payload` starts with the program itself.
            let mut rest = payload;
            rest.remove(0);
            (exe.to_path_buf(), rest)
        }
        (RunnerKind::Native, _) => return Err("this tool is only for Windows games".into()),
        (RunnerKind::Wine, Target::Winetricks(winetricks)) => {
            envs.push(("WINEPREFIX".into(), prefix.as_os_str().to_owned()));
            envs.push(("WINE".into(), runner.path.as_os_str().to_owned()));
            (winetricks.to_path_buf(), Vec::new())
        }
        (RunnerKind::Wine, _) => {
            envs.push(("WINEPREFIX".into(), prefix.as_os_str().to_owned()));
            envs.push(("WINEESYNC".into(), os(if config.esync { "1" } else { "0" })));
            envs.push(("WINEFSYNC".into(), os(if config.fsync { "1" } else { "0" })));
            if config.arch32 && !prefix.exists() {
                envs.push(("WINEARCH".into(), os("win32")));
            }
            (runner.path.clone(), payload)
        }
        (RunnerKind::Proton, Target::Winetricks(winetricks)) => {
            let proton_dir = runner.path.parent().ok_or("invalid Proton path")?;
            envs.push((
                "WINEPREFIX".into(),
                effective_prefix(kind, prefix).into_os_string(),
            ));
            envs.push((
                "WINE".into(),
                proton_dir.join("files/bin/wine").into_os_string(),
            ));
            (winetricks.to_path_buf(), Vec::new())
        }
        (RunnerKind::Proton, _) => {
            if !config.esync {
                envs.push(("PROTON_NO_ESYNC".into(), os("1")));
            }
            if !config.fsync {
                envs.push(("PROTON_NO_FSYNC".into(), os("1")));
            }
            if config.ntsync {
                envs.push(("PROTON_USE_NTSYNC".into(), os("1")));
            }
            if config.wined3d {
                envs.push(("PROTON_USE_WINED3D".into(), os("1")));
            }
            if config.nvapi {
                envs.push(("PROTON_ENABLE_NVAPI".into(), os("1")));
                envs.push(("DXVK_ENABLE_NVAPI".into(), os("1")));
            }
            if config.wayland {
                envs.push(("PROTON_ENABLE_WAYLAND".into(), os("1")));
            }
            if config.fsr {
                envs.push(("WINE_FULLSCREEN_FSR".into(), os("1")));
            }
            if config.debug {
                envs.push(("PROTON_LOG".into(), os("1")));
            }

            if let Some(umu) = request.umu {
                let proton_dir = runner.path.parent().ok_or("invalid Proton path")?;
                envs.push(("WINEPREFIX".into(), prefix.as_os_str().to_owned()));
                envs.push(("PROTONPATH".into(), proton_dir.as_os_str().to_owned()));
                envs.push(("GAMEID".into(), os("umu-0")));
                (umu.to_path_buf(), payload)
            } else {
                let steam_root = request
                    .steam_root
                    .ok_or("Steam is required to run Proton without umu-launcher")?;
                envs.push((
                    "STEAM_COMPAT_DATA_PATH".into(),
                    prefix.as_os_str().to_owned(),
                ));
                envs.push((
                    "STEAM_COMPAT_CLIENT_INSTALL_PATH".into(),
                    steam_root.as_os_str().to_owned(),
                ));
                let mut with_run = vec![os("run")];
                with_run.extend(payload);
                (runner.path.clone(), with_run)
            }
        }
    };

    // Wrappers go in front of everything: `gamescope -- mangohud gamemoderun <runner> <args>`.
    // Tools opened for the prefix (winecfg, winetricks) are never wrapped.
    let is_program = matches!(target, Target::Program(_));
    let wrappers = [
        config.mangohud.then_some(request.mangohud).flatten(),
        config.gamemode.then_some(request.gamemode).flatten(),
    ];
    for wrapper in wrappers.into_iter().flatten().rev() {
        args.insert(0, program.into_os_string());
        program = wrapper.to_path_buf();
    }
    if let (true, Some(gamescope), Some(path)) = (
        is_program,
        config.gamescope.as_ref(),
        request
            .hardware
            .and_then(|hardware| hardware.gamescope.as_ref()),
    ) {
        let mut front: Vec<OsString> = vec![
            os("-W"),
            os(&gamescope.width.to_string()),
            os("-H"),
            os(&gamescope.height.to_string()),
        ];
        if gamescope.fullscreen {
            front.push(os("-f"));
        }
        if gamescope.fsr {
            front.push(os("-F"));
            front.push(os("fsr"));
        }
        front.push(os("--"));
        front.push(program.into_os_string());
        front.append(&mut args);
        args = front;
        program = path.clone();
    }

    // The user's own variables win over everything above.
    for var in config.env.iter().filter(|var| !var.key.trim().is_empty()) {
        envs.retain(|(key, _)| key != var.key.trim());
        envs.push((var.key.trim().to_string(), os(&var.value)));
    }

    Ok(LaunchSpec {
        program,
        args,
        envs,
        cwd,
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn host_env_drops_what_points_into_the_appimage() {
        let appdir = "/tmp/.mount_Lutra.abc";
        let vars = [
            ("PYTHONHOME", "/tmp/.mount_Lutra.abc/usr/"),
            ("PYTHONPATH", "/tmp/.mount_Lutra.abc/usr/share/pyshared/:"),
            ("PATH", "/tmp/.mount_Lutra.abc/usr/bin/:/usr/bin:/bin"),
            ("LD_LIBRARY_PATH", "/tmp/.mount_Lutra.abc/usr/lib/"),
            ("GTK_DATA_PREFIX", "/tmp/.mount_Lutra.abc"),
            ("HOME", "/home/me"),
            ("LIKE_IT", "/tmp/.mount_Lutra.abcd/usr"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()));
        let changes: std::collections::HashMap<_, _> =
            host_env_changes(vars, appdir).into_iter().collect();
        assert_eq!(changes["PYTHONHOME"], None);
        assert_eq!(changes["LD_LIBRARY_PATH"], None);
        assert_eq!(changes["GTK_DATA_PREFIX"], None);
        // Only the AppRun's trailing empty entry would be left: the variable goes.
        assert_eq!(changes["PYTHONPATH"], None);
        assert_eq!(changes["PATH"].as_deref(), Some("/usr/bin:/bin"));
        assert!(!changes.contains_key("HOME"));
        assert!(
            !changes.contains_key("LIKE_IT"),
            "a sibling folder is not the AppImage"
        );
        assert!(host_env_changes([("A".to_string(), "/x".to_string())], "").is_empty());
    }
    use super::*;

    fn runner(kind: RunnerKind, path: &str) -> Runner {
        Runner {
            kind,
            label: "r".into(),
            path: path.into(),
        }
    }

    fn request<'a>(
        runner: &'a Runner,
        config: &'a WineConfig,
        target: Target<'a>,
        umu: Option<&'a Path>,
        steam: Option<&'a Path>,
    ) -> LaunchRequest<'a> {
        LaunchRequest {
            runner,
            prefix: Path::new("/prefixes/foo"),
            config,
            target,
            umu,
            steam_root: steam,
            gamemode: Some(Path::new("/usr/bin/gamemoderun")),
            mangohud: Some(Path::new("/usr/bin/mangohud")),
            hardware: None,
            log_dir: None,
        }
    }

    fn env<'a>(spec: &'a LaunchSpec, key: &str) -> Option<&'a OsString> {
        spec.envs.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn exe() -> &'static Path {
        Path::new("/games/foo/foo.exe")
    }

    #[test]
    fn wine_runs_the_exe_in_its_own_folder_and_prefix() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let config = WineConfig::default();
        let spec =
            build_launch(&request(&wine, &config, Target::Program(exe()), None, None)).unwrap();

        assert_eq!(spec.program, PathBuf::from("/usr/bin/wine"));
        assert_eq!(spec.args, vec![OsString::from("/games/foo/foo.exe")]);
        assert_eq!(spec.cwd, PathBuf::from("/games/foo"));
        assert_eq!(
            env(&spec, "WINEPREFIX"),
            Some(&OsString::from("/prefixes/foo"))
        );
        assert_eq!(
            env(&spec, "WINEDEBUG"),
            Some(&OsString::from("fixme-all,err+all"))
        );
        assert_eq!(env(&spec, "WINEESYNC"), Some(&OsString::from("1")));
    }

    #[test]
    fn arguments_keep_their_quotes_and_follow_the_program() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let config = WineConfig {
            args: "-windowed -user \"John Doe\"".into(),
            ..WineConfig::default()
        };
        let spec =
            build_launch(&request(&wine, &config, Target::Program(exe()), None, None)).unwrap();

        let args: Vec<&str> = spec.args.iter().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            vec!["/games/foo/foo.exe", "-windowed", "-user", "John Doe"]
        );
    }

    #[test]
    fn an_unclosed_quote_is_reported_instead_of_guessed() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let config = WineConfig {
            args: "-name \"oops".into(),
            ..WineConfig::default()
        };
        assert!(
            build_launch(&request(&wine, &config, Target::Program(exe()), None, None)).is_err()
        );
    }

    #[test]
    fn virtual_desktop_wraps_the_program_for_wine_only() {
        let config = WineConfig {
            virtual_desktop: Some("1280x720".into()),
            ..WineConfig::default()
        };
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let spec =
            build_launch(&request(&wine, &config, Target::Program(exe()), None, None)).unwrap();
        let args: Vec<&str> = spec.args.iter().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            vec!["explorer", "/desktop=Lutra,1280x720", "/games/foo/foo.exe"]
        );

        let proton = runner(RunnerKind::Proton, "/tools/GE/proton");
        let spec = build_launch(&request(
            &proton,
            &config,
            Target::Program(exe()),
            Some(Path::new("/usr/bin/umu-run")),
            None,
        ))
        .unwrap();
        assert_eq!(spec.args, vec![OsString::from("/games/foo/foo.exe")]);
    }

    #[test]
    fn custom_environment_and_dll_overrides_are_applied_and_win_over_defaults() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let config = WineConfig {
            dll_overrides: "d3d11=n,b".into(),
            env: vec![
                EnvVar {
                    key: "DXVK_HUD".into(),
                    value: "fps".into(),
                },
                EnvVar {
                    key: "WINEDEBUG".into(),
                    value: "+timestamp".into(),
                },
                EnvVar {
                    key: "  ".into(),
                    value: "ignored".into(),
                },
            ],
            ..WineConfig::default()
        };
        let spec =
            build_launch(&request(&wine, &config, Target::Program(exe()), None, None)).unwrap();

        assert_eq!(
            env(&spec, "WINEDLLOVERRIDES"),
            Some(&OsString::from("winemenubuilder.exe=d;d3d11=n,b"))
        );
        assert_eq!(env(&spec, "DXVK_HUD"), Some(&OsString::from("fps")));
        assert_eq!(env(&spec, "WINEDEBUG"), Some(&OsString::from("+timestamp")));
        assert_eq!(
            spec.envs.iter().filter(|(k, _)| k == "WINEDEBUG").count(),
            1
        );
    }

    #[test]
    fn sync_options_map_to_wine_and_proton_variables() {
        let config = WineConfig {
            esync: false,
            fsync: false,
            wined3d: true,
            ..WineConfig::default()
        };
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let spec =
            build_launch(&request(&wine, &config, Target::Program(exe()), None, None)).unwrap();
        assert_eq!(env(&spec, "WINEESYNC"), Some(&OsString::from("0")));
        assert_eq!(env(&spec, "WINEFSYNC"), Some(&OsString::from("0")));

        let proton = runner(RunnerKind::Proton, "/tools/GE/proton");
        let spec = build_launch(&request(
            &proton,
            &config,
            Target::Program(exe()),
            Some(Path::new("/usr/bin/umu-run")),
            None,
        ))
        .unwrap();
        assert_eq!(env(&spec, "PROTON_NO_ESYNC"), Some(&OsString::from("1")));
        assert_eq!(env(&spec, "PROTON_NO_FSYNC"), Some(&OsString::from("1")));
        assert_eq!(env(&spec, "PROTON_USE_WINED3D"), Some(&OsString::from("1")));
    }

    #[test]
    fn proton_prefers_umu_when_available() {
        let proton = runner(RunnerKind::Proton, "/tools/GE-Proton9/proton");
        let config = WineConfig::default();
        let spec = build_launch(&request(
            &proton,
            &config,
            Target::Program(exe()),
            Some(Path::new("/usr/bin/umu-run")),
            None,
        ))
        .unwrap();

        assert_eq!(spec.program, PathBuf::from("/usr/bin/umu-run"));
        assert_eq!(
            env(&spec, "PROTONPATH"),
            Some(&OsString::from("/tools/GE-Proton9"))
        );
        assert_eq!(
            env(&spec, "WINEPREFIX"),
            Some(&OsString::from("/prefixes/foo"))
        );
        assert_eq!(env(&spec, "GAMEID"), Some(&OsString::from("umu-0")));
    }

    #[test]
    fn proton_without_umu_calls_the_script_with_steam_compat_env() {
        let proton = runner(RunnerKind::Proton, "/tools/GE-Proton9/proton");
        let config = WineConfig::default();
        let steam = Path::new("/home/me/.local/share/Steam");
        let spec = build_launch(&request(
            &proton,
            &config,
            Target::Program(exe()),
            None,
            Some(steam),
        ))
        .unwrap();

        assert_eq!(spec.program, PathBuf::from("/tools/GE-Proton9/proton"));
        assert_eq!(
            spec.args,
            vec![OsString::from("run"), OsString::from("/games/foo/foo.exe")]
        );
        assert_eq!(
            env(&spec, "STEAM_COMPAT_DATA_PATH"),
            Some(&OsString::from("/prefixes/foo"))
        );
        assert_eq!(
            env(&spec, "STEAM_COMPAT_CLIENT_INSTALL_PATH"),
            Some(&OsString::from("/home/me/.local/share/Steam"))
        );
    }

    #[test]
    fn proton_without_umu_or_steam_is_an_error() {
        let proton = runner(RunnerKind::Proton, "/tools/GE-Proton9/proton");
        let config = WineConfig::default();
        assert!(
            build_launch(&request(
                &proton,
                &config,
                Target::Program(exe()),
                None,
                None
            ))
            .is_err()
        );
    }

    #[test]
    fn wrappers_go_in_front_with_mangohud_outermost() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let config = WineConfig {
            gamemode: true,
            mangohud: true,
            ..WineConfig::default()
        };
        let spec =
            build_launch(&request(&wine, &config, Target::Program(exe()), None, None)).unwrap();

        assert_eq!(spec.program, PathBuf::from("/usr/bin/mangohud"));
        let args: Vec<&str> = spec.args.iter().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            vec![
                "/usr/bin/gamemoderun",
                "/usr/bin/wine",
                "/games/foo/foo.exe"
            ]
        );
    }

    #[test]
    fn a_wrapper_that_is_not_installed_is_skipped() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let config = WineConfig {
            gamemode: true,
            ..WineConfig::default()
        };
        let mut req = request(&wine, &config, Target::Program(exe()), None, None);
        req.gamemode = None;
        let spec = build_launch(&req).unwrap();
        assert_eq!(spec.program, PathBuf::from("/usr/bin/wine"));
    }

    #[test]
    fn thirty_two_bit_only_applies_to_a_prefix_that_does_not_exist_yet() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let config = WineConfig {
            arch32: true,
            ..WineConfig::default()
        };
        let spec =
            build_launch(&request(&wine, &config, Target::Program(exe()), None, None)).unwrap();
        assert_eq!(env(&spec, "WINEARCH"), Some(&OsString::from("win32")));

        let existing = tempfile::tempdir().unwrap();
        let mut req = request(&wine, &config, Target::Program(exe()), None, None);
        req.prefix = existing.path();
        let spec = build_launch(&req).unwrap();
        assert_eq!(env(&spec, "WINEARCH"), None);
    }

    #[test]
    fn tools_run_in_the_games_prefix() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let config = WineConfig::default();

        let spec = build_launch(&request(&wine, &config, Target::Winecfg, None, None)).unwrap();
        assert_eq!(spec.program, PathBuf::from("/usr/bin/wine"));
        assert_eq!(spec.args, vec![OsString::from("winecfg")]);

        let tricks = Path::new("/usr/bin/winetricks");
        let spec = build_launch(&request(
            &wine,
            &config,
            Target::Winetricks(tricks),
            None,
            None,
        ))
        .unwrap();
        assert_eq!(spec.program, PathBuf::from("/usr/bin/winetricks"));
        assert_eq!(
            env(&spec, "WINEPREFIX"),
            Some(&OsString::from("/prefixes/foo"))
        );
        assert_eq!(env(&spec, "WINE"), Some(&OsString::from("/usr/bin/wine")));

        let proton = runner(RunnerKind::Proton, "/tools/GE-Proton9/proton");
        let spec = build_launch(&request(
            &proton,
            &config,
            Target::Winetricks(tricks),
            None,
            None,
        ))
        .unwrap();
        assert_eq!(
            env(&spec, "WINEPREFIX"),
            Some(&OsString::from("/prefixes/foo/pfx"))
        );
        assert_eq!(
            env(&spec, "WINE"),
            Some(&OsString::from("/tools/GE-Proton9/files/bin/wine"))
        );
    }

    #[test]
    fn config_survives_a_json_round_trip_and_tolerates_missing_fields() {
        let config = WineConfig {
            args: "-x".into(),
            env: vec![EnvVar {
                key: "A".into(),
                value: "1".into(),
            }],
            ..WineConfig::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(serde_json::from_str::<WineConfig>(&json).unwrap(), config);

        let sparse: WineConfig = serde_json::from_str(r#"{"gamemode": true}"#).unwrap();
        assert!(sparse.gamemode);
        assert!(sparse.esync, "missing fields fall back to the defaults");
    }

    #[test]
    fn lists_the_builds_lutra_downloaded_itself() {
        let data = tempfile::tempdir().unwrap();
        let runners_dir = data.path().join("runners");
        fs::create_dir_all(runners_dir.join("GE-Proton11-1")).unwrap();
        fs::write(runners_dir.join("GE-Proton11-1/proton"), "").unwrap();
        fs::create_dir_all(runners_dir.join("wine-11.0-amd64-wow64/bin")).unwrap();
        fs::write(runners_dir.join("wine-11.0-amd64-wow64/bin/wine"), "").unwrap();
        // An install in progress is never listed.
        fs::create_dir_all(runners_dir.join(".wine-12.work/bin")).unwrap();

        let runners = detect_runners(&[], Some(data.path()));
        let found: Vec<(RunnerKind, &str)> = runners
            .iter()
            .map(|runner| (runner.kind, runner.label.as_str()))
            .filter(|(_, label)| *label != "Wine")
            .collect();
        assert_eq!(
            found,
            [
                (RunnerKind::Proton, "GE-Proton11-1"),
                (RunnerKind::Wine, "wine-11.0-amd64-wow64"),
            ]
        );
    }

    #[test]
    fn detects_proton_builds_and_ignores_folders_without_a_proton_script() {
        let steam = tempfile::tempdir().unwrap();
        for dir in [
            "steamapps/common/Proton - Experimental",
            "compatibilitytools.d/GE-Proton9-1",
        ] {
            let path = steam.path().join(dir);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("proton"), "#!/usr/bin/env python3\n").unwrap();
        }
        fs::create_dir_all(steam.path().join("steamapps/common/Some Game")).unwrap();

        let runners = detect_runners(&[steam.path().to_path_buf()], None);
        let labels: Vec<&str> = runners
            .iter()
            .filter(|runner| runner.kind == RunnerKind::Proton)
            .map(|runner| runner.label.as_str())
            .collect();

        assert_eq!(labels, vec!["Proton - Experimental", "GE-Proton9-1"]);
    }

    fn nvidia_laptop() -> Hardware {
        use crate::hardware::Gpu;
        let gpu = |vendor, vendor_id, id: &str, primary| Gpu {
            id: id.into(),
            vendor,
            vendor_id,
            device_id: 0x1c20,
            name: "gpu".into(),
            primary,
            driver: None,
        };
        Hardware {
            gpus: vec![
                gpu(GpuVendor::Intel, 0x8086, "0000:00:02.0", true),
                gpu(GpuVendor::Nvidia, 0x10de, "0000:01:00.0", false),
            ],
            ntsync: true,
            gamescope: Some(PathBuf::from("/usr/bin/gamescope")),
        }
    }

    #[test]
    fn dll_overrides_merge_with_the_later_list_winning_per_library() {
        assert_eq!(
            merge_dll_overrides(&["winemenubuilder.exe=d", "d3d11=n,b;winemenubuilder.exe=n"]),
            "winemenubuilder.exe=n;d3d11=n,b"
        );
        assert_eq!(merge_dll_overrides(&["", "  "]), "");
        assert_eq!(merge_dll_overrides(&["a,b=n"]), "a=n;b=n");
    }

    #[test]
    fn the_recommended_config_picks_the_discrete_gpu_and_ntsync_for_proton() {
        let hardware = nvidia_laptop();
        let proton = recommended_config(&hardware, RunnerKind::Proton, true);
        assert_eq!(proton.gpu.as_deref(), Some("0000:01:00.0"));
        assert!(proton.nvapi && proton.ntsync && proton.gamemode);

        let wine = recommended_config(&hardware, RunnerKind::Wine, false);
        assert_eq!(wine.gpu.as_deref(), Some("0000:01:00.0"));
        assert!(!wine.nvapi && !wine.ntsync && !wine.gamemode);

        let none = recommended_config(&Hardware::default(), RunnerKind::Proton, false);
        assert_eq!(none.gpu, None);
    }

    #[test]
    fn the_selected_gpu_and_driver_tweaks_reach_the_environment() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let hardware = nvidia_laptop();
        let config = WineConfig {
            gpu: Some("0000:01:00.0".into()),
            fps_limit: Some(60),
            dxvk_hud: Some("fps".into()),
            ..WineConfig::default()
        };
        let mut req = request(&wine, &config, Target::Program(exe()), None, None);
        req.hardware = Some(&hardware);
        let spec = build_launch(&req).unwrap();

        assert_eq!(
            env(&spec, "__NV_PRIME_RENDER_OFFLOAD"),
            Some(&OsString::from("1"))
        );
        assert_eq!(env(&spec, "DXVK_FRAME_RATE"), Some(&OsString::from("60")));
        assert_eq!(env(&spec, "DXVK_HUD"), Some(&OsString::from("fps")));
        assert_eq!(
            env(&spec, "__GL_THREADED_OPTIMIZATIONS"),
            Some(&OsString::from("1"))
        );
        assert_eq!(
            env(&spec, "STAGING_SHARED_MEMORY"),
            Some(&OsString::from("1"))
        );

        let off = WineConfig {
            perf_tweaks: false,
            ..WineConfig::default()
        };
        let spec = build_launch(&request(&wine, &off, Target::Program(exe()), None, None)).unwrap();
        assert_eq!(env(&spec, "__GL_THREADED_OPTIMIZATIONS"), None);
    }

    #[test]
    fn a_game_without_a_gpu_or_with_one_that_is_gone_gets_the_best_one() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let hardware = nvidia_laptop();
        for gpu in [None, Some("0000:09:00.0".to_string())] {
            let config = WineConfig {
                gpu,
                ..WineConfig::default()
            };
            let mut req = request(&wine, &config, Target::Program(exe()), None, None);
            req.hardware = Some(&hardware);
            let spec = build_launch(&req).unwrap();
            assert_eq!(
                env(&spec, "__NV_PRIME_RENDER_OFFLOAD"),
                Some(&OsString::from("1"))
            );
            assert_eq!(
                env(&spec, "MESA_VK_DEVICE_SELECT_FORCE_DEFAULT_DEVICE"),
                Some(&OsString::from("1"))
            );
        }
    }

    #[test]
    fn proton_only_options_do_not_leak_into_wine() {
        let config = WineConfig {
            ntsync: true,
            nvapi: true,
            wayland: true,
            fsr: true,
            ..WineConfig::default()
        };
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let spec =
            build_launch(&request(&wine, &config, Target::Program(exe()), None, None)).unwrap();
        assert_eq!(env(&spec, "PROTON_USE_NTSYNC"), None);
        assert_eq!(env(&spec, "PROTON_ENABLE_NVAPI"), None);

        let proton = runner(RunnerKind::Proton, "/tools/GE/proton");
        let spec = build_launch(&request(
            &proton,
            &config,
            Target::Program(exe()),
            Some(Path::new("/usr/bin/umu-run")),
            None,
        ))
        .unwrap();
        assert_eq!(env(&spec, "PROTON_USE_NTSYNC"), Some(&OsString::from("1")));
        assert_eq!(
            env(&spec, "PROTON_ENABLE_NVAPI"),
            Some(&OsString::from("1"))
        );
        assert_eq!(
            env(&spec, "PROTON_ENABLE_WAYLAND"),
            Some(&OsString::from("1"))
        );
        assert_eq!(
            env(&spec, "WINE_FULLSCREEN_FSR"),
            Some(&OsString::from("1"))
        );
    }

    #[test]
    fn gamescope_wraps_everything_and_needs_to_be_installed() {
        let wine = runner(RunnerKind::Wine, "/usr/bin/wine");
        let hardware = nvidia_laptop();
        let config = WineConfig {
            gamemode: true,
            gamescope: Some(Gamescope {
                width: 1280,
                height: 720,
                fullscreen: true,
                fsr: true,
            }),
            ..WineConfig::default()
        };
        let mut req = request(&wine, &config, Target::Program(exe()), None, None);
        req.hardware = Some(&hardware);
        let spec = build_launch(&req).unwrap();
        assert_eq!(spec.program, PathBuf::from("/usr/bin/gamescope"));
        let args: Vec<&str> = spec.args.iter().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            vec![
                "-W",
                "1280",
                "-H",
                "720",
                "-f",
                "-F",
                "fsr",
                "--",
                "/usr/bin/gamemoderun",
                "/usr/bin/wine",
                "/games/foo/foo.exe"
            ]
        );

        // Not installed: the game still starts, unwrapped.
        let mut req = request(&wine, &config, Target::Program(exe()), None, None);
        req.hardware = None;
        assert_eq!(
            build_launch(&req).unwrap().program,
            PathBuf::from("/usr/bin/gamemoderun")
        );
    }

    #[test]
    fn native_games_run_directly_with_their_own_arguments_and_no_wine_variables() {
        let native = runner(RunnerKind::Native, "");
        let config = WineConfig {
            args: "--fullscreen \"my save\"".into(),
            gamemode: true,
            ..WineConfig::default()
        };
        let exe = Path::new("/games/linux/run.sh");
        let spec =
            build_launch(&request(&native, &config, Target::Program(exe), None, None)).unwrap();

        assert_eq!(spec.program, PathBuf::from("/usr/bin/gamemoderun"));
        let args: Vec<&str> = spec.args.iter().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(args, vec!["/games/linux/run.sh", "--fullscreen", "my save"]);
        assert_eq!(spec.cwd, PathBuf::from("/games/linux"));
        assert_eq!(env(&spec, "WINEPREFIX"), None);
        assert_eq!(env(&spec, "WINEDEBUG"), None);
        assert_eq!(
            env(&spec, "__GL_THREADED_OPTIMIZATIONS"),
            Some(&OsString::from("1"))
        );

        assert!(build_launch(&request(&native, &config, Target::Winecfg, None, None)).is_err());
    }

    #[test]
    fn native_programs_are_checked_and_made_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, bytes: &[u8]| {
            let path = dir.path().join(name);
            fs::write(&path, bytes).unwrap();
            path
        };

        let script = write("run.sh", b"#!/bin/sh\necho hi\n");
        assert_eq!(
            fs::metadata(&script).unwrap().permissions().mode() & 0o111,
            0
        );
        ensure_runnable(&script).unwrap();
        assert_ne!(
            fs::metadata(&script).unwrap().permissions().mode() & 0o111,
            0
        );

        assert!(ensure_runnable(&write("game", b"\x7fELF\x02\x01")).is_ok());
        let windows = ensure_runnable(&write("game.exe", b"MZ\x90\x00")).unwrap_err();
        assert!(windows.contains("Windows"));
        assert!(ensure_runnable(&write("notes.txt", b"hello")).is_err());
        assert!(ensure_runnable(&dir.path().join("missing")).is_err());
    }

    #[test]
    fn debug_mode_sends_the_runner_and_graphics_logs_to_the_games_log_folder() {
        let proton = runner(RunnerKind::Proton, "/tools/GE/proton");
        let config = WineConfig {
            debug: true,
            ..WineConfig::default()
        };
        let dir = Path::new("/logs/wine_1.d");
        let mut req = request(
            &proton,
            &config,
            Target::Program(exe()),
            Some(Path::new("/usr/bin/umu-run")),
            None,
        );
        req.log_dir = Some(dir);
        let spec = build_launch(&req).unwrap();
        assert_eq!(env(&spec, "PROTON_LOG"), Some(&OsString::from("1")));
        assert_eq!(
            env(&spec, "PROTON_LOG_DIR"),
            Some(&OsString::from("/logs/wine_1.d"))
        );
        assert_eq!(
            env(&spec, "DXVK_LOG_PATH"),
            Some(&OsString::from("/logs/wine_1.d"))
        );
        assert_eq!(env(&spec, "WINEDEBUG"), Some(&OsString::from("")));

        // Without the option nothing extra is written.
        let quiet = WineConfig::default();
        let mut req = request(
            &proton,
            &quiet,
            Target::Program(exe()),
            Some(Path::new("/usr/bin/umu-run")),
            None,
        );
        req.log_dir = Some(dir);
        let spec = build_launch(&req).unwrap();
        assert_eq!(env(&spec, "DXVK_LOG_PATH"), None);
        assert_eq!(env(&spec, "PROTON_LOG"), None);
    }

    /// Runs a real Windows program through the command `build_launch` produces, with this machine's
    /// recommended options. Needs Wine; creates a prefix (about 30 s): `cargo test -- --ignored wine::`.
    #[test]
    #[ignore]
    fn real_wine_runs_a_program_with_the_recommended_options() {
        let Some(wine_path) = find_in_path("wine") else {
            return;
        };
        let cmd = Path::new("/usr/lib/wine/x86_64-windows/cmd.exe");
        if !cmd.is_file() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().join("prefix");
        let hardware = crate::hardware::detect();
        let runner = Runner {
            kind: RunnerKind::Wine,
            label: "Wine".into(),
            path: wine_path,
        };
        let config = WineConfig {
            args: "/c echo lutra-ok & exit 3".into(),
            ..recommended_config(&hardware, RunnerKind::Wine, false)
        };
        let spec = build_launch(&LaunchRequest {
            runner: &runner,
            prefix: &prefix,
            config: &config,
            target: Target::Program(cmd),
            umu: None,
            steam_root: None,
            gamemode: None,
            mangohud: None,
            hardware: Some(&hardware),
            log_dir: None,
        })
        .unwrap();
        println!(
            "env: {:?}",
            spec.envs
                .iter()
                .map(|(k, _)| k.as_str())
                .collect::<Vec<_>>()
        );

        let output = Command::new(&spec.program)
            .args(&spec.args)
            .envs(spec.envs.iter().map(|(k, v)| (k, v)))
            .current_dir(if spec.cwd.is_dir() {
                &spec.cwd
            } else {
                dir.path()
            })
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&output.stdout);
        println!("{text}\n{}", String::from_utf8_lossy(&output.stderr));
        assert!(text.contains("lutra-ok"), "the program did not run: {text}");
        // The exit code of the Windows program comes back through Wine.
        assert_eq!(output.status.code(), Some(3));
        assert!(
            prefix.join("system.reg").is_file(),
            "the prefix was not created"
        );
    }
}
