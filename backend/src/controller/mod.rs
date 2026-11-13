//! The controller driver: reads every physical controller and hides it behind a virtual Xbox 360
//! pad or DualShock 4 shaped by a profile, the way Steam Input does. While a game runs, that is
//! the game's profile; otherwise, system-wide, the default profile (so a drifting stick or a
//! remapped button is fixed everywhere). Controllers also navigate Lutra's menus.

pub mod battery;
pub mod codes;
pub mod convert;
pub(crate) mod device;
pub mod model;
pub mod nav;
mod virtual_ds4;
mod virtual_input;
mod virtual_pad;

use std::collections::{BTreeSet, HashMap};
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use evdev::{
    Device, EventSummary, FFEffect, FFEffectData, FFEffectKind, FFReplay, FFTrigger, InputEvent,
    UInputCode,
};

use crate::applog;
use codes::{Decoder, Family, Range};
pub use device::ControllerInfo;
use model::{OutputKind, PadSnapshot, PadState, Profile};
use nav::{GuideWatcher, NavAction, Navigator};

/// How often new or unplugged controllers are noticed.
const SCAN_INTERVAL: Duration = Duration::from_millis(1500);
/// Upper bound on how long a worker waits for input, so it notices session changes and
/// can repeat held menu directions.
const POLL_TIMEOUT_MS: i32 = 40;
/// Pace of the pointer while a stick is the mouse (about 125 steps a second).
const POINTER_TICK_MS: i32 = 8;

pub type NavCallback = Arc<dyn Fn(NavAction) + Send + Sync>;
pub type ChangeCallback = Arc<dyn Fn() + Send + Sync>;

/// Whether this user may create virtual devices (needs an udev rule or ACL on `/dev/uinput`).
pub fn uinput_available() -> bool {
    OpenOptions::new().write(true).open("/dev/uinput").is_ok()
}

/// Whether this user may create HID devices, which a DualShock 4 needs for games to show
/// PlayStation buttons (see `virtual_ds4`).
pub fn uhid_available() -> bool {
    virtual_ds4::available()
}

/// Numbers Lutra's HID controllers, so each has its own `phys`.
static NEXT_HID: AtomicUsize = AtomicUsize::new(0);

/// Whether Steam runs: it then drives the controllers itself (Steam Input, through
/// `/dev/hidraw`), and with Lutra's system-wide driver too every controller would show twice.
fn steam_running() -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.bytes().all(|byte| byte.is_ascii_digit()))
            && std::fs::read_to_string(entry.path().join("comm"))
                .is_ok_and(|comm| comm.trim_end() == "steam")
    })
}

/// Windows games read XInput, which only an Xbox pad speaks (see `OutputKind`).
fn windows_game(owner: &str) -> bool {
    owner.starts_with("wine:")
}

/// What every controller should do right now. `owner: None` means "no game running".
#[derive(Default)]
struct Session {
    owner: Option<String>,
    profile: Option<Profile>,
}

struct Shared {
    session: Mutex<Session>,
    /// Bumped on every session change so workers know to rebuild their virtual device.
    generation: AtomicU64,
    navigation: AtomicBool,
    /// Drive the controllers with the default profile when no game runs (user setting).
    system: AtomicBool,
    /// Steam runs, so the system-wide driver steps aside.
    steam: AtomicBool,
    /// The default profile, used system-wide.
    system_profile: Mutex<Profile>,
    /// Bumped when the default profile changes in a way workers apply without a new device.
    system_revision: AtomicU64,
    on_nav: NavCallback,
    /// The controller whose Guide button was held last: what the power menu switches off.
    power_target: Mutex<Option<ControllerInfo>>,
    /// The in-game overlay is open: the game gets a neutral controller and the overlay the input.
    overlay: AtomicBool,
}

impl Shared {
    /// Whether controllers are driven system-wide when no game runs.
    fn system_wide(&self) -> bool {
        self.system.load(Ordering::SeqCst)
            && !self.steam.load(Ordering::SeqCst)
            && uinput_available()
    }

    /// Makes every worker rebuild its virtual device, unless a game holds the controllers: its
    /// virtual pad must not disappear under it. The change is picked up when the game ends.
    fn rebuild_outside_games(&self) {
        let session = self.session.lock().expect("session");
        if session.owner.is_none() {
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
    }
}

struct Worker {
    info: ControllerInfo,
    state: Arc<Mutex<PadState>>,
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

pub struct ControllerService {
    shared: Arc<Shared>,
    workers: Arc<Mutex<HashMap<PathBuf, Worker>>>,
}

impl ControllerService {
    /// Starts watching for controllers. `on_nav` receives menu actions while no game runs;
    /// `on_change` is called when controllers come and go.
    /// `system` drives them with `system_profile` when no game runs.
    pub fn start(
        navigation: bool,
        system: bool,
        system_profile: Profile,
        on_nav: NavCallback,
        on_change: ChangeCallback,
    ) -> Self {
        let shared = Arc::new(Shared {
            session: Mutex::new(Session::default()),
            generation: AtomicU64::new(0),
            navigation: AtomicBool::new(navigation),
            system: AtomicBool::new(system),
            steam: AtomicBool::new(steam_running()),
            system_profile: Mutex::new(system_profile),
            system_revision: AtomicU64::new(0),
            on_nav,
            power_target: Mutex::new(None),
            overlay: AtomicBool::new(false),
        });
        let workers: Arc<Mutex<HashMap<PathBuf, Worker>>> = Arc::default();

        let (monitor_shared, monitor_workers) = (shared.clone(), workers.clone());
        std::thread::Builder::new()
            .name("controllers".into())
            .spawn(move || {
                loop {
                    let mut changed = refresh(&monitor_shared, &monitor_workers);
                    let steam = steam_running();
                    if monitor_shared.steam.swap(steam, Ordering::SeqCst) != steam {
                        applog::info(if steam {
                            "Steam is running: controllers are left to Steam Input outside games"
                        } else {
                            "Steam closed: Lutra drives the controllers again"
                        });
                        monitor_shared.rebuild_outside_games();
                        changed = true;
                    }
                    if changed {
                        on_change();
                    }
                    std::thread::sleep(SCAN_INTERVAL);
                }
            })
            .expect("controller monitor thread");

        Self { shared, workers }
    }

    /// The controller the power menu is about: the one held last if it is still connected,
    /// else the first Bluetooth one, else the first.
    pub fn power_target(&self) -> Option<ControllerInfo> {
        let connected = self.controllers();
        let asked = self
            .shared
            .power_target
            .lock()
            .expect("power target")
            .clone();
        asked
            .and_then(|asked| connected.iter().find(|info| info.id == asked.id).cloned())
            .or_else(|| {
                connected
                    .iter()
                    .find(|info| info.bus == "bluetooth")
                    .cloned()
            })
            .or_else(|| connected.first().cloned())
    }

    pub fn controllers(&self) -> Vec<ControllerInfo> {
        let workers = self.workers.lock().expect("workers");
        let mut list: Vec<ControllerInfo> =
            workers.values().map(|worker| worker.info.clone()).collect();
        list.sort_by(|a, b| a.path.cmp(&b.path));
        list
    }

    /// The physical state of a controller, before any profile is applied.
    pub fn state(&self, id: &str) -> Option<PadSnapshot> {
        let workers = self.workers.lock().expect("workers");
        let worker = workers.values().find(|worker| worker.info.id == id)?;
        let state = *worker.state.lock().expect("pad state");
        Some(state.snapshot())
    }

    /// The physical state of a controller, unprocessed.
    pub fn raw_state(&self, id: &str) -> Option<PadState> {
        let workers = self.workers.lock().expect("workers");
        let worker = workers.values().find(|worker| worker.info.id == id)?;
        let state = *worker.state.lock().expect("pad state");
        Some(state)
    }

    /// Hands every controller over to `profile` until `end_session(owner)`.
    pub fn start_session(&self, owner: &str, profile: Profile) {
        applog::info(format!(
            "Controllers: {owner} takes over as {:?}",
            profile.output
        ));
        let mut session = self.shared.session.lock().expect("session");
        session.owner = Some(owner.to_string());
        session.profile = Some(profile);
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Gives the controllers back, unless someone else took over in the meantime.
    pub fn end_session(&self, owner: &str) {
        let mut session = self.shared.session.lock().expect("session");
        if session.owner.as_deref() != Some(owner) {
            return;
        }
        applog::info(format!("Controllers: {owner} released them"));
        *session = Session::default();
        // No game: no overlay either.
        self.shared.overlay.store(false, Ordering::SeqCst);
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Who holds the controllers, and as what.
    pub fn session(&self) -> Option<(String, OutputKind)> {
        let session = self.shared.session.lock().expect("session");
        Some((session.owner.clone()?, session.profile.as_ref()?.output))
    }

    pub fn set_navigation(&self, enabled: bool) {
        self.shared.navigation.store(enabled, Ordering::SeqCst);
    }

    pub fn navigation(&self) -> bool {
        self.shared.navigation.load(Ordering::SeqCst)
    }

    pub fn set_system(&self, enabled: bool) {
        if self.shared.system.swap(enabled, Ordering::SeqCst) != enabled {
            applog::info(format!(
                "System-wide controller driver {}",
                if enabled { "on" } else { "off" }
            ));
            self.shared.rebuild_outside_games();
        }
    }

    pub fn system(&self) -> bool {
        self.shared.system.load(Ordering::SeqCst)
    }

    /// Whether the system-wide driver steps aside for Steam right now.
    pub fn steam(&self) -> bool {
        self.shared.steam.load(Ordering::SeqCst)
    }

    /// The default profile changed: the system-wide driver follows it. The virtual devices are
    /// only rebuilt when they have to change, so applications keep their controller while a
    /// slider moves.
    pub fn set_system_profile(&self, profile: Profile) {
        let mut current = self.shared.system_profile.lock().expect("system profile");
        let rebuild = current.output != profile.output
            || current.converts_to_keyboard() != profile.converts_to_keyboard();
        *current = profile;
        drop(current);
        if rebuild {
            self.shared.rebuild_outside_games();
        } else {
            self.shared.system_revision.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// The physical controllers a game must not read while Lutra drives them, in the format of
    /// `SDL_GAMECONTROLLER_IGNORE_DEVICES`, which SDL, Wine and Proton honour (Steam sets it the
    /// same way). Grabbing the input device does not cover `/dev/hidraw`, where SDL and Wine
    /// also find the controller: without this, the game sees it twice. The virtual controllers'
    /// own ids are never listed.
    pub fn hidden_from_games(&self) -> Option<String> {
        let (owner, profile) = {
            let session = self.shared.session.lock().expect("session");
            (session.owner.clone()?, session.profile.clone()?)
        };
        if profile.output == OutputKind::None {
            return None;
        }
        ignore_list(&self.controllers(), profile.output, windows_game(&owner))
    }

    /// Whether the in-game overlay is open.
    pub fn overlay_open(&self) -> bool {
        self.shared.overlay.load(Ordering::SeqCst)
    }

    /// Opens or closes the in-game overlay: while open, the game sees a neutral controller and
    /// the controller's input becomes menu actions.
    pub fn set_overlay(&self, open: bool) {
        self.shared.overlay.store(open, Ordering::SeqCst);
    }
}

/// `SDL_GAMECONTROLLER_IGNORE_DEVICES` for these physical controllers shown as `output`.
fn ignore_list(
    controllers: &[ControllerInfo],
    output: OutputKind,
    windows: bool,
) -> Option<String> {
    let shown: Vec<(u16, u16)> = controllers
        .iter()
        .filter_map(|controller| {
            let output = output.resolve(windows, controller.family == Family::PlayStation);
            virtual_pad::identity(output, (controller.vendor, controller.product))
        })
        .collect();
    let hidden: BTreeSet<(u16, u16)> = controllers
        .iter()
        .map(|controller| (controller.vendor, controller.product))
        .filter(|ids| !shown.contains(ids))
        .collect();
    (!hidden.is_empty()).then(|| {
        hidden
            .iter()
            .map(|(vendor, product)| format!("0x{vendor:04x}/0x{product:04x}"))
            .collect::<Vec<_>>()
            .join(",")
    })
}

/// Starts workers for new controllers and forgets unplugged ones. True if the list changed.
fn refresh(shared: &Arc<Shared>, workers: &Arc<Mutex<HashMap<PathBuf, Worker>>>) -> bool {
    let found = device::scan();
    let mut workers = workers.lock().expect("workers");
    let mut changed = false;

    let gone: Vec<PathBuf> = workers
        .iter()
        .filter(|(path, worker)| {
            worker.handle.is_finished() || !found.iter().any(|(found, _)| found == *path)
        })
        .map(|(path, _)| path.clone())
        .collect();
    for path in gone {
        if let Some(worker) = workers.remove(&path) {
            worker.stop.store(true, Ordering::SeqCst);
            applog::info(format!("Controller disconnected: {}", worker.info.name));
            changed = true;
        }
    }

    for (path, info) in found {
        if workers.contains_key(&path) {
            continue;
        }
        applog::info(format!(
            "Controller connected: {} ({:04x}:{:04x}, {})",
            info.name, info.vendor, info.product, info.bus
        ));
        let state = Arc::new(Mutex::new(PadState::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (worker_shared, worker_state, worker_stop, worker_info) =
            (shared.clone(), state.clone(), stop.clone(), info.clone());
        let thread_path = path.clone();
        let spawned = std::thread::Builder::new()
            .name(format!("controller {}", info.name))
            .spawn(move || {
                run_worker(
                    thread_path,
                    worker_info,
                    worker_shared,
                    worker_state,
                    worker_stop,
                )
            });
        match spawned {
            Ok(handle) => {
                workers.insert(
                    path,
                    Worker {
                        info,
                        state,
                        stop,
                        handle,
                    },
                );
                changed = true;
            }
            Err(error) => applog::error(format!("Could not watch {}: {error}", info.name)),
        }
    }
    changed
}

/// A controller driven by Lutra: the virtual device it drives and the rumble effects the game
/// uploaded, mirrored on the physical controller.
struct Active {
    /// Driven system-wide with the default profile, not handed to a game.
    system: bool,
    profile: Profile,
    /// The virtual gamepad; absent when the controller is only converted to keyboard and mouse.
    pad: Option<VirtualGamepad>,
    /// The virtual keyboard and mouse, when the profile converts anything to them.
    input: Option<virtual_input::VirtualInput>,
    converter: convert::Converter,
    /// What the profile currently sends to the keyboard and mouse.
    routed: model::Routed,
    previous: PadState,
    effects: HashMap<i16, FFEffect>,
    /// The rumble a HID controller's game asked for, played on the physical controller.
    hid_rumble: Option<FFEffect>,
    /// Touchpad and motion sensors of the same controller, held so they stay quiet in game.
    _companions: Vec<Device>,
}

/// The gamepad a game sees instead of the physical one.
enum VirtualGamepad {
    /// An input device (`uinput`): the Xbox 360 pad, or a DualShock 4 when HID devices cannot
    /// be created (games then show Xbox buttons for it).
    Input(virtual_pad::VirtualPad),
    /// A HID DualShock 4 (`uhid`), which games recognise as one.
    Hid(virtual_ds4::VirtualDs4),
}

impl VirtualGamepad {
    fn fd(&self) -> i32 {
        match self {
            VirtualGamepad::Input(pad) => pad.device.as_raw_fd(),
            VirtualGamepad::Hid(pad) => pad.fd(),
        }
    }
}

impl Active {
    /// Writes what the profile makes of `input` to the virtual devices.
    fn send(&mut self, input: &PadState) {
        let (output, routed) = model::convert(&self.profile, input);
        match self.pad.as_mut() {
            Some(VirtualGamepad::Input(pad)) => {
                let changes = codes::diff(self.profile.output, &self.previous, &output);
                if !changes.is_empty() {
                    let events: Vec<InputEvent> = changes
                        .iter()
                        .map(|(kind, code, value)| InputEvent::new(*kind, *code, *value))
                        .collect();
                    let _ = pad.device.emit(&events);
                }
            }
            Some(VirtualGamepad::Hid(pad)) if output != self.previous => pad.send(&output),
            _ => {}
        }
        self.previous = output;
        self.routed = routed;
    }
}

/// Hides the controller behind the virtual device `profile` asks for. `system`: no game, the
/// default profile everywhere.
fn activate(
    device: &mut Device,
    info: &ControllerInfo,
    mut profile: Profile,
    windows_game: bool,
    system: bool,
) -> Option<Active> {
    if profile.output == OutputKind::None {
        return None;
    }
    profile.output = profile
        .output
        .resolve(windows_game, info.family == Family::PlayStation);
    let physical = (info.vendor, info.product);
    let hid_ds4 = profile.output == OutputKind::Ds4 && virtual_ds4::available();
    if profile.output == OutputKind::Ds4 && !hid_ds4 {
        applog::warn(
            "The PlayStation 4 controller is an input device only (/dev/uhid is not writable): \
             games will show Xbox buttons for it",
        );
    }
    let created = if profile.output == OutputKind::Keyboard {
        None
    } else if hid_ds4 {
        let identity = virtual_pad::identity(OutputKind::Ds4, physical).unwrap_or(physical);
        Some(
            virtual_ds4::VirtualDs4::create(identity, NEXT_HID.fetch_add(1, Ordering::SeqCst))
                .map(VirtualGamepad::Hid),
        )
    } else {
        Some(virtual_pad::create(profile.output, physical).map(VirtualGamepad::Input))
    };
    let pad = match created {
        None => None,
        Some(created) => match created {
            Ok(pad) => Some(pad),
            Err(error) => {
                applog::error(format!(
                    "Could not create the virtual controller for {}: {error}",
                    info.name
                ));
                return None;
            }
        },
    };
    let input = if profile.converts_to_keyboard() {
        match virtual_input::create() {
            Ok(input) => Some(input),
            Err(error) => {
                applog::error(format!(
                    "Could not create the virtual keyboard and mouse for {}: {error}",
                    info.name
                ));
                None
            }
        }
    } else {
        None
    };
    if pad.is_none() && input.is_none() {
        return None;
    }
    // Exclusive access: without it the game would see both controllers and get every press twice.
    if let Err(error) = device.grab() {
        applog::warn(format!(
            "Could not take exclusive control of {}: {error}",
            info.name
        ));
    }
    // Outside games the touchpad keeps moving the pointer, as it does without Lutra.
    let companions: Vec<Device> = if system {
        Vec::new()
    } else {
        device::companions(info)
            .into_iter()
            .filter_map(|mut companion| companion.grab().ok().map(|_| companion))
            .collect()
    };

    applog::info(format!(
        "{} now appears {}as {:?}{}",
        info.name,
        if system { "system-wide " } else { "" },
        profile.output,
        if input.is_some() {
            " (with keyboard and mouse conversion)"
        } else {
            ""
        }
    ));
    Some(Active {
        system,
        profile,
        pad,
        input,
        converter: convert::Converter::new(Instant::now()),
        routed: model::Routed::default(),
        previous: PadState::default(),
        effects: HashMap::new(),
        hid_rumble: None,
        _companions: companions,
    })
}

/// Plays the rumble a HID controller's game asked for on the physical controller.
fn play_hid_rumble(active: &mut Active, device: &mut Device, rumble: virtual_ds4::Rumble) {
    if !active.profile.rumble {
        return;
    }
    let data = FFEffectData {
        direction: 0,
        trigger: FFTrigger::default(),
        // Length 0: until the game sets it again.
        replay: FFReplay {
            length: 0,
            delay: 0,
        },
        kind: FFEffectKind::Rumble {
            strong_magnitude: u16::from(rumble.strong) * 257,
            weak_magnitude: u16::from(rumble.weak) * 257,
        },
    };
    if active.hid_rumble.is_none() {
        active.hid_rumble = device.upload_ff_effect(data).ok();
    }
    if let Some(effect) = active.hid_rumble.as_mut() {
        let _ = effect.update(data);
        let _ = if rumble == virtual_ds4::Rumble::default() {
            effect.stop()
        } else {
            effect.play(1)
        };
    }
}

/// Forwards the game's rumble from the virtual controller to the physical one, and answers
/// what the kernel and the game ask of a HID controller.
fn forward_rumble(active: &mut Active, device: &mut Device) {
    let pad = match active.pad.as_mut() {
        Some(VirtualGamepad::Input(pad)) => pad,
        Some(VirtualGamepad::Hid(pad)) => {
            if let Some(rumble) = pad.process() {
                play_hid_rumble(active, device, rumble);
            }
            return;
        }
        None => return,
    };
    let events: Vec<InputEvent> = match pad.device.fetch_events() {
        Ok(events) => events.collect(),
        Err(_) => return,
    };
    for event in events {
        match event.destructure() {
            EventSummary::UInput(request, UInputCode::UI_FF_UPLOAD, _) => {
                let Ok(mut upload) = pad.device.process_ff_upload(request) else {
                    continue;
                };
                let id = upload.effect_id();
                let data = upload.effect();
                // The game must always get a success, even with rumble turned off.
                let uploaded = if !active.profile.rumble {
                    Ok(())
                } else if let Some(effect) = active.effects.get_mut(&id) {
                    effect.update(data)
                } else {
                    device.upload_ff_effect(data).map(|effect| {
                        active.effects.insert(id, effect);
                    })
                };
                upload.set_retval(if uploaded.is_ok() { 0 } else { -libc::EIO });
            }
            EventSummary::UInput(request, UInputCode::UI_FF_ERASE, _) => {
                if let Ok(erase) = pad.device.process_ff_erase(request) {
                    active.effects.remove(&(erase.effect_id() as i16));
                }
            }
            EventSummary::ForceFeedback(_, code, value) => {
                if let Some(effect) = active.effects.get_mut(&(code.0 as i16)) {
                    let _ = if value > 0 {
                        effect.play(value)
                    } else {
                        effect.stop()
                    };
                }
            }
            _ => {}
        }
    }
}

/// Does this look like a Bluetooth address (`aa:bb:cc:dd:ee:ff`)? The serial of a Bluetooth input
/// device is its address; anything else must never reach a command line.
fn is_bluetooth_address(text: &str) -> bool {
    let parts: Vec<&str> = text.split(':').collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|part| part.len() == 2 && part.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Why a controller could not be switched off.
pub const POWER_OFF_WIRED: &str = "wired";

/// How long `bluetoothctl` may take to end the link.
const POWER_OFF_TIMEOUT: Duration = Duration::from_secs(8);

/// Switches a Bluetooth controller off by ending its link (a DualShock or Xbox pad powers down
/// when the host disconnects it). The answer is what really happened: BlueZ's own message when it
/// refuses. A controller on a cable stays on: there is nothing to switch off, so unplugging it
/// is the only way (`POWER_OFF_WIRED`).
pub fn power_off(info: &ControllerInfo) -> Result<(), String> {
    use std::io::Read;
    use std::process::{Command, Stdio};

    if info.bus == "usb" {
        return Err(POWER_OFF_WIRED.into());
    }
    if info.bus != "bluetooth" || !is_bluetooth_address(&info.id) {
        return Err(format!("{} is not a Bluetooth controller", info.name));
    }
    let mut child = crate::wine::host_env(&mut Command::new("bluetoothctl"))
        .args(["disconnect", &info.id])
        // Not the backend's own stdin: that is the shell's request pipe.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("bluetoothctl is not available: {error}"))?;

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < POWER_OFF_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Bluetooth did not answer in time".into());
            }
            Err(error) => return Err(error.to_string()),
        }
    };
    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut output);
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut output);
    }
    if status.success() && !output.contains("Failed") {
        applog::info(format!("{} switched off (link ended)", info.name));
        return Ok(());
    }
    let reason = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Bluetooth refused")
        .to_string();
    applog::warn(format!("Could not switch {} off: {reason}", info.name));
    Err(reason)
}

fn run_worker(
    path: PathBuf,
    info: ControllerInfo,
    shared: Arc<Shared>,
    state: Arc<Mutex<PadState>>,
    stop: Arc<AtomicBool>,
) {
    let mut device = match Device::open(&path) {
        Ok(device) => device,
        Err(error) => {
            applog::warn(format!("Could not open {}: {error}", info.name));
            return;
        }
    };
    let _ = device.set_nonblocking(true);

    let ranges = device.get_abs_state().ok();
    let mut decoder = Decoder::new(info.family, |axis| {
        let range = ranges.as_ref()?.get(axis as usize)?;
        (range.maximum > range.minimum).then_some(Range {
            min: range.minimum,
            max: range.maximum,
        })
    });
    if info.bus != "usb" {
        decoder = decoder.bluetooth();
    }
    decoder.calibrate(|axis| Some(ranges.as_ref()?.get(axis as usize)?.value));

    let mut active: Option<Active> = None;
    let mut seen_generation = u64::MAX;
    let mut seen_revision = shared.system_revision.load(Ordering::SeqCst);
    let mut navigator = Navigator::default();
    // The Guide button during a game (see below).
    let mut game_guide = GuideWatcher::default();
    let mut overlay_was_open = false;

    while !stop.load(Ordering::SeqCst) {
        let generation = shared.generation.load(Ordering::SeqCst);
        if generation != seen_generation {
            seen_generation = generation;
            if active.take().is_some() {
                let _ = device.ungrab();
                applog::info(format!("{} is back to normal", info.name));
            }
            let (owner, profile) = {
                let session = shared.session.lock().expect("session");
                (session.owner.clone(), session.profile.clone())
            };
            active = match (owner, profile) {
                (Some(owner), Some(profile)) => {
                    activate(&mut device, &info, profile, windows_game(&owner), false)
                }
                (None, _) if shared.system_wide() => {
                    let profile = shared
                        .system_profile
                        .lock()
                        .expect("system profile")
                        .clone();
                    activate(&mut device, &info, profile, false, true)
                }
                _ => None,
            };
            seen_revision = shared.system_revision.load(Ordering::SeqCst);
            navigator.reset();
            game_guide = GuideWatcher::default();
            overlay_was_open = false;
        }

        let revision = shared.system_revision.load(Ordering::SeqCst);
        if revision != seen_revision {
            seen_revision = revision;
            if let Some(active) = active.as_mut().filter(|active| active.system) {
                // Same virtual device: only the settings change.
                let output = active.profile.output;
                active.profile = shared
                    .system_profile
                    .lock()
                    .expect("system profile")
                    .clone();
                active.profile.output = output;
            }
        }

        let mut fds = vec![libc::pollfd {
            fd: device.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        }];
        if let Some(pad) = active.as_ref().and_then(|active| active.pad.as_ref()) {
            fds.push(libc::pollfd {
                fd: pad.fd(),
                events: libc::POLLIN,
                revents: 0,
            });
        }
        // The pointer has to keep moving while a stick is held, though nothing arrives.
        // So does a HID controller's stream of reports.
        let timeout = match &active {
            Some(active)
                if active.routed.mouse != (0.0, 0.0)
                    || matches!(active.pad, Some(VirtualGamepad::Hid(_))) =>
            {
                POINTER_TICK_MS
            }
            _ => POLL_TIMEOUT_MS,
        };
        // SAFETY: `fds` is a valid array of pollfd for the duration of the call.
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) };
        if ready < 0 {
            continue;
        }
        if fds[0].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            break;
        }

        let mut frames = Vec::new();
        if fds[0].revents & libc::POLLIN != 0 {
            let events: Vec<InputEvent> = match device.fetch_events() {
                Ok(events) => events.collect(),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Vec::new(),
                // The controller was unplugged or switched off.
                Err(_) => break,
            };
            for event in events {
                if decoder.feed(event.event_type().0, event.code(), event.value()) {
                    frames.push(decoder.state());
                }
            }
        }
        let in_game = active.as_ref().is_some_and(|active| !active.system);
        // Outside games, the sticks' rest is measured again whenever the controller is put down.
        if !in_game && decoder.settle(Instant::now()) {
            frames.push(decoder.state());
        }

        for raw in frames {
            *state.lock().expect("pad state") = raw;
            let now = Instant::now();
            // The Guide button belongs to Lutra whenever it drives the controller: the virtual
            // controller never sends it.
            let mut seen = raw;
            seen.set(model::Button::Guide, false);

            if let Some(active) = active.as_mut().filter(|active| !active.system) {
                // In a game, a short press opens the overlay, a long hold (below) switches the
                // controller off.
                if game_guide.update(raw.pressed(model::Button::Guide), now) {
                    (shared.on_nav)(NavAction::Overlay);
                }
                let overlay = shared.overlay.load(Ordering::SeqCst);
                if overlay != overlay_was_open {
                    overlay_was_open = overlay;
                    navigator.reset();
                }
                if overlay {
                    // The overlay has the controller: it navigates, the game holds still.
                    for action in navigator.update(&seen, now) {
                        (shared.on_nav)(action);
                    }
                    seen = PadState::default();
                }
                active.send(&seen);
            } else {
                if let Some(active) = active.as_mut() {
                    active.send(&seen);
                }
                if shared.navigation.load(Ordering::SeqCst) {
                    for action in navigator.update(&raw, now) {
                        (shared.on_nav)(action);
                    }
                }
            }
        }

        let now = Instant::now();
        if let Some(active) = active.as_mut() {
            if fds.get(1).is_some_and(|fd| fd.revents & libc::POLLIN != 0) {
                forward_rumble(active, &mut device);
            }
            if let Some(VirtualGamepad::Hid(pad)) = active.pad.as_mut() {
                pad.tick(now);
            }
            if let Some(input) = active.input.as_mut() {
                let events =
                    active
                        .converter
                        .update(&active.routed, active.profile.mouse_speed, now);
                if !events.is_empty() {
                    input.emit(&events);
                }
            }
        }
        if in_game {
            // In a game there is no window to show a menu in: holding the Guide switches the pad off.
            if game_guide.held_long(now) {
                applog::info(format!(
                    "Switching {} off (Guide held in a game)",
                    info.name
                ));
                let info = info.clone();
                std::thread::spawn(move || {
                    let _ = power_off(&info);
                });
            }
            // A direction held in the overlay keeps repeating.
            if shared.overlay.load(Ordering::SeqCst)
                && let Some(action) = navigator.tick(now)
            {
                (shared.on_nav)(action);
            }
        } else if let Some(action) = navigator
            .tick(now)
            .filter(|_| shared.navigation.load(Ordering::SeqCst))
        {
            if action == NavAction::PowerMenu {
                applog::info(format!("Power menu (Guide held on {})", info.name));
                // The menu acts on the controller that asked for it.
                *shared.power_target.lock().expect("power target") = Some(info.clone());
                (shared.on_nav)(action);
            } else {
                (shared.on_nav)(action);
            }
        }
    }

    if active.is_some() {
        let _ = device.ungrab();
    }
}

/// Checks against the real system (needs `/dev/uinput`): `cargo test -- --ignored controller`.
#[cfg(test)]
mod hardware_tests {
    use super::*;

    #[test]
    #[ignore = "needs /dev/uinput"]
    fn virtual_controllers_appear_with_the_identity_of_the_real_ones() {
        for (kind, vendor, product) in [
            (OutputKind::Xbox360, 0x045e, 0x028e),
            (OutputKind::Ds4, 0x054c, 0x09cc),
        ] {
            let mut pad = virtual_pad::create(kind, (0, 0)).expect("virtual controller");
            let node = pad
                .device
                .enumerate_dev_nodes_blocking()
                .expect("device nodes")
                .filter_map(Result::ok)
                .find(|node| {
                    node.file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with("event"))
                })
                .expect("an event node");
            // udev sets the node's permissions a moment after it appears.
            let device = (0..40)
                .find_map(|_| {
                    Device::open(&node).ok().or_else(|| {
                        std::thread::sleep(Duration::from_millis(50));
                        None
                    })
                })
                .expect("open the virtual controller");

            assert_eq!(device.input_id().vendor(), vendor);
            assert_eq!(device.input_id().product(), product);
            assert!(
                !device::is_gamepad(&node, &device),
                "Lutra must never read its own virtual controllers"
            );

            let events: Vec<InputEvent> = codes::output_values(kind, &PadState::default())
                .into_iter()
                .map(|(kind, code, value)| InputEvent::new(kind, code, value))
                .collect();
            pad.device.emit(&events).expect("write a frame");
            drop(pad);
            assert!(
                !virtual_pad::is_own(&node),
                "a removed controller frees its node"
            );
        }
    }

    /// `cargo test -- --ignored hid_dualshock`
    #[test]
    #[ignore = "needs /dev/uhid"]
    fn a_hid_dualshock_is_taken_by_the_kernel_driver_with_a_hidraw_node() {
        let mut pad = virtual_ds4::VirtualDs4::create((0x054c, 0x05c4), 900).expect("uhid device");
        let phys = format!("{}/ds4-900", virtual_ds4::PHYS_PREFIX);
        // The driver asks for its feature reports while probing: answer them until it is done.
        let started = Instant::now();
        let mut found = None;
        while started.elapsed() < Duration::from_secs(5) && found.is_none() {
            pad.process();
            pad.tick(Instant::now());
            found = evdev::enumerate().find(|(path, device)| {
                device::is_lutra_hid(path)
                    && device
                        .supported_keys()
                        .is_some_and(|keys| keys.contains(evdev::KeyCode::BTN_SOUTH))
            });
            std::thread::sleep(Duration::from_millis(10));
        }
        let (path, device) = found.expect("hid-playstation created the gamepad");
        assert_eq!(device.input_id().vendor(), 0x054c);
        assert!(
            !device::is_gamepad(&path, &device),
            "Lutra must never read its own HID controllers"
        );
        let hidraw = std::fs::read_dir("/sys/class/hidraw")
            .expect("hidraw class")
            .filter_map(Result::ok)
            .any(|entry| {
                std::fs::read_to_string(entry.path().join("device/uevent"))
                    .is_ok_and(|uevent| uevent.contains(&format!("HID_PHYS={phys}")))
            });
        assert!(hidraw, "games find the controller through hidraw");
    }

    #[test]
    #[ignore = "needs /dev/uinput"]
    fn converted_input_arrives_as_keyboard_and_mouse_events() {
        let mut input = virtual_input::create().expect("virtual keyboard and mouse");
        let node = input
            .device
            .enumerate_dev_nodes_blocking()
            .expect("device nodes")
            .filter_map(Result::ok)
            .find(|node| {
                node.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("event"))
            })
            .expect("an event node");
        let mut reader = (0..40)
            .find_map(|_| {
                Device::open(&node).ok().or_else(|| {
                    std::thread::sleep(Duration::from_millis(50));
                    None
                })
            })
            .expect("open the virtual keyboard");
        reader.set_nonblocking(true).unwrap();

        let start = Instant::now();
        let mut converter = convert::Converter::new(start);
        let routed = model::Routed {
            keys: ["enter".to_owned()].into(),
            mouse: (1.0, 0.0),
        };
        let events = converter.update(&routed, 900.0, start + Duration::from_millis(10));
        input.emit(&events);
        std::thread::sleep(Duration::from_millis(100));

        let seen: Vec<(u16, u16, i32)> = reader
            .fetch_events()
            .expect("read events")
            .map(|event| (event.event_type().0, event.code(), event.value()))
            .collect();
        assert!(
            seen.contains(&(0x01, evdev::KeyCode::KEY_ENTER.0, 1)),
            "{seen:?}"
        );
        assert!(
            seen.contains(&(0x02, evdev::RelativeAxisCode::REL_X.0, 9)),
            "{seen:?}"
        );
    }

    /// Alternates A and the left stick half right for a while, so a Windows program can be pointed at
    /// the virtual controller: `LUTRA_HOLD_SECS=30 cargo test -- --ignored hold_virtual`.
    #[test]
    #[ignore = "needs /dev/uinput and a Windows program watching"]
    fn hold_virtual_xbox_pad_for_a_probe() {
        let secs: u64 = std::env::var("LUTRA_HOLD_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(20);
        let mut pad = virtual_pad::create(OutputKind::Xbox360, (0, 0)).expect("virtual controller");
        let mut previous = PadState::default();
        let started = Instant::now();
        // A and the stick alternate between pressed and released, so a program that opens the
        // controller late still sees changes.
        while started.elapsed() < Duration::from_secs(secs) {
            let mut state = PadState::default();
            if (started.elapsed().as_millis() / 400).is_multiple_of(2) {
                state.set(model::Button::South, true);
                state.lx = 0.5;
            }
            let events: Vec<InputEvent> = codes::diff(OutputKind::Xbox360, &previous, &state)
                .iter()
                .map(|(kind, code, value)| InputEvent::new(*kind, *code, *value))
                .collect();
            pad.device.emit(&events).expect("write a frame");
            previous = state;
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    #[ignore = "needs a connected controller"]
    fn connected_controllers_are_found() {
        let found = device::scan();
        for (path, info) in &found {
            println!("{} -> {info:?}", path.display());
        }
        assert!(!found.is_empty(), "plug a controller in to run this test");
    }
}

#[cfg(test)]
mod ignore_tests {
    use super::*;

    fn controller(vendor: u16, product: u16) -> ControllerInfo {
        ControllerInfo {
            id: format!("{vendor:04x}:{product:04x}"),
            name: "Controller".into(),
            path: "/dev/input/event0".into(),
            vendor,
            product,
            family: Family::from_vendor(vendor),
            model: codes::PadModel::from_ids(vendor, product),
            bus: "bluetooth",
        }
    }

    #[test]
    fn a_controller_on_a_cable_cannot_be_switched_off() {
        let mut info = controller(0x054c, 0x09cc);
        info.bus = "usb";
        assert_eq!(power_off(&info), Err(POWER_OFF_WIRED.to_string()));
        // Bluetooth, but without a Bluetooth address to disconnect.
        info.bus = "bluetooth";
        info.id = "not-an-address".into();
        assert!(power_off(&info).is_err());
    }

    #[test]
    fn games_are_told_to_ignore_the_physical_controller_but_not_the_virtual_one() {
        let ds4 = [controller(0x054c, 0x09cc)];
        // A Windows game gets an Xbox 360 pad.
        assert_eq!(
            ignore_list(&ds4, OutputKind::Auto, true).as_deref(),
            Some("0x054c/0x09cc")
        );
        // A DualShock 4 shown as a DualShock 4 takes other ids, so the physical one can be hidden.
        assert_eq!(
            ignore_list(&ds4, OutputKind::Auto, false).as_deref(),
            Some("0x054c/0x09cc")
        );
        // A physical Xbox 360 pad has the virtual one's ids: hiding it would hide both (xpad has no
        // hidraw node anyway, so grabbing it is enough).
        let xbox = [controller(0x045e, 0x028e)];
        assert_eq!(ignore_list(&xbox, OutputKind::Xbox360, true), None);
    }
}

#[cfg(test)]
mod power_tests {
    use super::is_bluetooth_address;

    #[test]
    fn only_bluetooth_addresses_are_passed_to_the_command() {
        assert!(is_bluetooth_address("b7:a4:0b:54:ed:e9"));
        assert!(is_bluetooth_address("B7:A4:0B:54:ED:E9"));
        assert!(!is_bluetooth_address("/dev/input/event30"));
        assert!(!is_bluetooth_address("b7:a4:0b:54:ed"));
        assert!(!is_bluetooth_address("b7:a4:0b:54:ed:zz"));
        assert!(!is_bluetooth_address("usb-0000:00:14.0-1/input0"));
    }
}
