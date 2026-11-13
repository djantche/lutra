//! Lutra's own libretro player: `lutra --play <config.json>`.
//!
//! Lutra runs emulated games itself instead of handing them to RetroArch: this process loads
//! the core Lutra picked, opens a window, plays the sound, reads the controllers and keeps the
//! saves. It is a separate process (the same executable) so a crashing core never takes the
//! library down, and so stopping a game is stopping a process, as for any other game.
//!
//! First step: cores that draw in software, which is nearly every console up to the
//! PlayStation. Cores that need the GPU still go through RetroArch (see `docs/EMULATION.md`).

mod audio;
mod input;
mod libretro;
mod video;

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use glow::HasContext;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use serde::{Deserialize, Serialize};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::emulation::profile::EmulationProfile;
pub use libretro::CoreOptions;
use libretro::{AvInfo, Core, FRONTEND};

/// How often the game's own saves are written while it runs, besides when it ends.
const SAVE_EVERY: Duration = Duration::from_secs(10);
/// Frames run at most to catch up after a stall, so a hiccup does not fast-forward the game.
const MAX_CATCH_UP: u32 = 3;

/// What Lutra tells the player to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayConfig {
    pub core: PathBuf,
    pub rom: PathBuf,
    pub title: String,
    /// BIOS files.
    pub system_dir: PathBuf,
    /// In-game saves (`<rom name>.srm`).
    pub save_dir: PathBuf,
    /// The state kept to resume the game (`<rom name>.state`).
    pub state_dir: PathBuf,
    pub fullscreen: bool,
    /// Save the whole state when leaving and resume from it next time.
    pub auto_resume: bool,
    /// Picture, controls and core options: the game's own, or its console's.
    pub profile: EmulationProfile,
    /// The interface's language (`fr`, `en`...), which translated cores speak too.
    pub language: String,
}

/// `RETRO_LANGUAGE_*` for an interface language.
fn retro_language(code: &str) -> libc::c_uint {
    match code.split(['-', '_']).next().unwrap_or(code) {
        "fr" => 2,
        "es" => 3,
        "de" => 4,
        "it" => 5,
        _ => 0,
    }
}

/// Entry point of `lutra --core-options <core> <language>`: prints, as JSON, the options the
/// core declares. A separate process, so a misbehaving core cannot take Lutra down.
pub fn list_options(core: &Path, language: &str) -> i32 {
    let dir = std::env::temp_dir();
    match Core::open(core, &dir, &dir, HashMap::new(), retro_language(language)) {
        Ok(core) => {
            let options = FRONTEND.with(|frontend| frontend.borrow().definitions.clone());
            drop(core);
            println!("{}", serde_json::to_string(&options).unwrap_or_default());
            0
        }
        Err(error) => {
            eprintln!("lutra: {error}");
            1
        }
    }
}

/// SIGTERM, as sent when the game is stopped from Lutra: leave cleanly, saves written.
static QUIT: AtomicBool = AtomicBool::new(false);

extern "C" fn on_terminate(_: libc::c_int) {
    QUIT.store(true, Ordering::SeqCst);
}

/// Entry point of `lutra --play`. Returns the process exit code.
pub fn main(config: &Path) -> i32 {
    match run(config) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("lutra player: {error}");
            1
        }
    }
}

fn run(config: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(config).map_err(|e| format!("{}: {e}", config.display()))?;
    let config: PlayConfig = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    // SAFETY: the handler only stores into an atomic.
    unsafe {
        let handler = on_terminate as *const () as libc::sighandler_t;
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGINT, handler);
    }

    for dir in [&config.save_dir, &config.state_dir, &config.system_dir] {
        let _ = std::fs::create_dir_all(dir);
    }
    let overrides: HashMap<String, String> =
        config.profile.core_options.clone().into_iter().collect();
    let mut core = Core::open(
        &config.core,
        &config.system_dir,
        &config.save_dir,
        overrides,
        retro_language(&config.language),
    )?;
    let av = core.load(&config.rom)?;
    eprintln!(
        "lutra player: {} at {:.3} fps, {} Hz, {}x{}",
        config.title,
        av.timing.fps,
        av.timing.sample_rate,
        av.geometry.base_width,
        av.geometry.base_height
    );

    let stem = config
        .rom
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "game".into());
    let saves = Saves {
        ram: config.save_dir.join(format!("{stem}.srm")),
        state: config.state_dir.join(format!("{stem}.state")),
        auto_resume: config.auto_resume,
    };
    saves.load(&mut core);

    let audio = match audio::Audio::open(av.timing.sample_rate) {
        Ok(audio) => Some(audio),
        Err(error) => {
            eprintln!("lutra player: no sound: {error}");
            None
        }
    };

    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    let input = input::Input::new(config.profile.controls.clone());
    let mut player = Player {
        config,
        core,
        av,
        saves,
        audio,
        input,
        screen: None,
        next_frame: Instant::now(),
        last_save: Instant::now(),
        error: None,
    };
    event_loop.run_app(&mut player).map_err(|e| e.to_string())?;
    player.saves.write(&mut player.core);
    match player.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Where the game's saves go, and writing them.
struct Saves {
    ram: PathBuf,
    state: PathBuf,
    auto_resume: bool,
}

impl Saves {
    fn load(&self, core: &mut Core) {
        if let (Ok(data), Some(ram)) = (std::fs::read(&self.ram), core.save_ram()) {
            let size = data.len().min(ram.len());
            ram[..size].copy_from_slice(&data[..size]);
        }
        if self.auto_resume
            && let Ok(state) = std::fs::read(&self.state)
            && !core.load_state(&state)
        {
            eprintln!("lutra player: the saved state does not fit this core, starting over");
        }
    }

    fn write(&self, core: &mut Core) {
        if let Some(ram) = core.save_ram()
            && ram.iter().any(|byte| *byte != 0)
        {
            let _ = write_atomically(&self.ram, ram);
        }
        if self.auto_resume
            && let Some(state) = core.save_state()
        {
            let _ = write_atomically(&self.state, &state);
        }
    }
}

/// Writes through a temporary file, so a save is never left half written.
fn write_atomically(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, data)?;
    std::fs::rename(temporary, path)
}

/// The window and its OpenGL context.
struct Screen {
    window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    gl: glow::Context,
    renderer: video::Renderer,
}

struct Player {
    config: PlayConfig,
    core: Core,
    av: AvInfo,
    saves: Saves,
    audio: Option<audio::Audio>,
    input: input::Input,
    screen: Option<Screen>,
    next_frame: Instant,
    last_save: Instant,
    error: Option<String>,
}

fn open_screen(
    event_loop: &ActiveEventLoop,
    config: &PlayConfig,
    av: &AvInfo,
) -> Result<Screen, String> {
    let scale = 3;
    let mut attributes = Window::default_attributes()
        .with_title(config.title.clone())
        .with_inner_size(winit::dpi::LogicalSize::new(
            av.geometry.base_width.max(160) * scale,
            av.geometry.base_height.max(144) * scale,
        ));
    if config.fullscreen {
        attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    let template = ConfigTemplateBuilder::new();
    let (window, gl_config) = DisplayBuilder::new()
        .with_window_attributes(Some(attributes))
        .build(event_loop, template, |mut configs| {
            configs.next().expect("an OpenGL configuration")
        })
        .map_err(|e| e.to_string())?;
    let window = window.ok_or("no window")?;
    let handle = window.window_handle().map_err(|e| e.to_string())?.as_raw();
    let display = gl_config.display();
    let attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
        .build(Some(handle));
    // SAFETY: the window outlives the context and surface (both live in `Screen`).
    let context =
        unsafe { display.create_context(&gl_config, &attributes) }.map_err(|e| e.to_string())?;
    let surface_attributes = window
        .build_surface_attributes(Default::default())
        .map_err(|e| e.to_string())?;
    // SAFETY: as above.
    let surface = unsafe { display.create_window_surface(&gl_config, &surface_attributes) }
        .map_err(|e| e.to_string())?;
    let context = context.make_current(&surface).map_err(|e| e.to_string())?;
    // Frames are paced by the console's clock, not the screen's.
    let _ = surface.set_swap_interval(&context, SwapInterval::DontWait);
    // SAFETY: the loader comes from the current context's display.
    let gl =
        unsafe { glow::Context::from_loader_function_cstr(|name| display.get_proc_address(name)) };
    let renderer = video::Renderer::new(&gl)?;
    window.set_cursor_visible(false);
    Ok(Screen {
        window,
        surface,
        context,
        gl,
        renderer,
    })
}

impl Player {
    fn frame_duration(&self) -> Duration {
        let fps = if self.av.timing.fps > 1.0 {
            self.av.timing.fps
        } else {
            60.0
        };
        Duration::from_secs_f64(1.0 / fps)
    }

    /// Runs the frames that are due, then shows the last one.
    fn step(&mut self, now: Instant) {
        let mut ran = 0;
        while now >= self.next_frame && ran < MAX_CATCH_UP {
            let pads = self.input.poll(now);
            FRONTEND.with(|frontend| frontend.borrow_mut().pads = pads);
            self.core.run_frame();
            self.next_frame += self.frame_duration();
            ran += 1;

            let (samples, av_info, rumble, shutdown) = FRONTEND.with(|frontend| {
                let mut frontend = frontend.borrow_mut();
                (
                    std::mem::take(&mut frontend.audio),
                    frontend.av_info.take(),
                    frontend.rumble,
                    frontend.shutdown,
                )
            });
            if let Some(audio) = self.audio.as_mut() {
                audio.push(&samples);
            }
            self.input.rumble(&rumble);
            if let Some(info) = av_info {
                if info.timing.fps > 0.0 {
                    self.av.timing = info.timing;
                    if let Some(audio) = self.audio.as_mut() {
                        audio.set_input_rate(info.timing.sample_rate);
                    }
                }
                if info.geometry.base_width > 0 {
                    self.av.geometry = info.geometry;
                }
            }
            if shutdown {
                QUIT.store(true, Ordering::SeqCst);
            }
        }
        // Far behind (the window was hidden, the machine stalled): start again from now.
        if now > self.next_frame + self.frame_duration() * MAX_CATCH_UP {
            self.next_frame = now;
        }
        if ran > 0 {
            self.present();
        }
        if now.duration_since(self.last_save) >= SAVE_EVERY {
            self.last_save = now;
            if let Some(ram) = self.core.save_ram()
                && ram.iter().any(|byte| *byte != 0)
            {
                let _ = write_atomically(&self.saves.ram, ram);
            }
        }
    }

    fn present(&mut self) {
        let Some(screen) = self.screen.as_mut() else {
            return;
        };
        FRONTEND.with(|frontend| {
            let mut frontend = frontend.borrow_mut();
            if frontend.frame.fresh {
                frontend.frame.fresh = false;
                screen.renderer.upload(&screen.gl, &frontend.frame);
            }
        });
        let size = screen.window.inner_size();
        screen.renderer.draw(
            &screen.gl,
            (size.width.max(1), size.height.max(1)),
            self.av.geometry.aspect(),
            &self.config.profile.video,
        );
        let _ = screen.surface.swap_buffers(&screen.context);
    }
}

impl ApplicationHandler for Player {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.screen.is_some() {
            return;
        }
        match open_screen(event_loop, &self.config, &self.av) {
            Ok(screen) => {
                self.screen = Some(screen);
                self.next_frame = Instant::now();
            }
            Err(error) => {
                self.error = Some(format!("could not open the game's window: {error}"));
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(screen) = self.screen.as_ref() {
                    screen.surface.resize(
                        &screen.context,
                        NonZeroU32::new(size.width.max(1)).unwrap_or(NonZeroU32::MIN),
                        NonZeroU32::new(size.height.max(1)).unwrap_or(NonZeroU32::MIN),
                    );
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(key) = event.physical_key else {
                    return;
                };
                let down = event.state == ElementState::Pressed;
                match key {
                    KeyCode::Escape if down => event_loop.exit(),
                    KeyCode::F11 if down && !event.repeat => {
                        if let Some(screen) = self.screen.as_ref() {
                            let full = screen.window.fullscreen().is_some();
                            screen
                                .window
                                .set_fullscreen((!full).then_some(Fullscreen::Borderless(None)));
                        }
                    }
                    KeyCode::F2 if down && !event.repeat => self.core.reset(),
                    _ => self.input.key(key, down),
                }
            }
            WindowEvent::RedrawRequested => self.present(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if QUIT.load(Ordering::SeqCst) {
            event_loop.exit();
            return;
        }
        if self.screen.is_none() {
            return;
        }
        self.step(Instant::now());
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        // SAFETY: the context is still current; nothing draws after this.
        unsafe { self.gl.finish() };
    }
}

/// Runs real cores without a window: `cargo test -- --ignored player`.
#[cfg(test)]
mod core_tests {
    use super::*;

    /// 32 KiB Game Boy ROM: `nop; jp 0x150` at the entry point, an endless `jr -2` there.
    fn game_boy_rom(dir: &Path) -> PathBuf {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100..0x104].copy_from_slice(&[0x00, 0xC3, 0x50, 0x01]);
        rom[0x150..0x152].copy_from_slice(&[0x18, 0xFE]);
        rom[0x134..0x139].copy_from_slice(b"LUTRA");
        rom[0x14D] = rom[0x134..0x14D]
            .iter()
            .fold(0u8, |acc, byte| acc.wrapping_sub(*byte).wrapping_sub(1));
        let path = dir.join("Lutra Test.gb");
        std::fs::write(&path, rom).unwrap();
        path
    }

    /// Runs a second of `rom` on `core`: pictures and sound must come out, and the state must
    /// survive a save and a load.
    fn run_one_second(core: &Path, rom: &Path) {
        let dir = tempfile::tempdir().unwrap();
        let mut core = Core::open(core, dir.path(), dir.path(), HashMap::new(), 0).expect("core");
        let av = core.load(rom).expect("game");
        assert!(av.timing.fps > 30.0, "{av:?}");
        for _ in 0..av.timing.fps.round() as usize {
            core.run_frame();
        }
        let (width, height, audio) = FRONTEND.with(|frontend| {
            let frontend = frontend.borrow();
            (
                frontend.frame.width,
                frontend.frame.height,
                frontend.audio.len(),
            )
        });
        assert_eq!(
            (width, height),
            (
                av.geometry.base_width as usize,
                av.geometry.base_height as usize
            )
        );
        // About a second of stereo sound.
        let expected = av.timing.sample_rate * 2.0;
        assert!(
            (audio as f64) > expected * 0.9 && (audio as f64) < expected * 1.1,
            "{audio} samples, expected {expected}"
        );
        let state = core.save_state().expect("a state");
        assert!(core.load_state(&state));
    }

    #[test]
    #[ignore = "needs the gambatte core"]
    fn a_game_boy_game_runs_in_lutras_player() {
        let core = Path::new("/usr/lib/libretro/gambatte_libretro.so");
        let dir = tempfile::tempdir().unwrap();
        run_one_second(core, &game_boy_rom(dir.path()));
    }

    /// Any core and game: `LUTRA_TEST_CORE=… LUTRA_TEST_ROM=… cargo test -- --ignored any_game`.
    #[test]
    #[ignore = "needs LUTRA_TEST_CORE and LUTRA_TEST_ROM"]
    fn any_game_runs_in_lutras_player() {
        let (Some(core), Some(rom)) = (
            std::env::var_os("LUTRA_TEST_CORE"),
            std::env::var_os("LUTRA_TEST_ROM"),
        ) else {
            return;
        };
        run_one_second(Path::new(&core), Path::new(&rom));
    }
}
