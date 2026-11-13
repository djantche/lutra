//! The libretro API, from the frontend's side: loading a core and answering its callbacks.
//!
//! A core is a shared library that emulates a console and talks to its frontend through a
//! handful of C functions (`retro_run`, `retro_load_game`...) and callbacks (one frame of video,
//! audio samples, the state of the buttons, and `environment`, a catch-all for everything else).
//! See `libretro.h`; only what software-rendered cores need is implemented here.

use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::path::Path;

pub const DEVICE_JOYPAD: c_uint = 1;
pub const DEVICE_ANALOG: c_uint = 5;
const DEVICE_ID_JOYPAD_MASK: c_uint = 256;
const DEVICE_INDEX_ANALOG_BUTTON: c_uint = 2;
pub const MEMORY_SAVE_RAM: c_uint = 0;

// Joypad buttons, in libretro's (SNES) naming: B is the bottom face button, A the right one.
pub const JOYPAD_B: u16 = 1 << 0;
pub const JOYPAD_Y: u16 = 1 << 1;
pub const JOYPAD_SELECT: u16 = 1 << 2;
pub const JOYPAD_START: u16 = 1 << 3;
pub const JOYPAD_UP: u16 = 1 << 4;
pub const JOYPAD_DOWN: u16 = 1 << 5;
pub const JOYPAD_LEFT: u16 = 1 << 6;
pub const JOYPAD_RIGHT: u16 = 1 << 7;
pub const JOYPAD_A: u16 = 1 << 8;
pub const JOYPAD_X: u16 = 1 << 9;
pub const JOYPAD_L: u16 = 1 << 10;
pub const JOYPAD_R: u16 = 1 << 11;

const EXPERIMENTAL: c_uint = 0x10000;

#[repr(C)]
struct GameInfo {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct Geometry {
    pub base_width: c_uint,
    pub base_height: c_uint,
    pub max_width: c_uint,
    pub max_height: c_uint,
    pub aspect_ratio: f32,
}

impl Geometry {
    /// The shape the picture must have on screen.
    pub fn aspect(&self) -> f32 {
        if self.aspect_ratio > 0.0 {
            self.aspect_ratio
        } else if self.base_height > 0 {
            self.base_width as f32 / self.base_height as f32
        } else {
            4.0 / 3.0
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct Timing {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct AvInfo {
    pub geometry: Geometry,
    pub timing: Timing,
}

#[repr(C)]
struct SystemInfo {
    library_name: *const c_char,
    library_version: *const c_char,
    valid_extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}

#[repr(C)]
struct Variable {
    key: *const c_char,
    value: *const c_char,
}

#[repr(C)]
struct Message {
    msg: *const c_char,
    frames: c_uint,
}

#[repr(C)]
struct OptionValue {
    value: *const c_char,
    label: *const c_char,
}

/// `RETRO_NUM_CORE_OPTION_VALUES_MAX`.
const OPTION_VALUES: usize = 128;

#[repr(C)]
struct DefinitionV1 {
    key: *const c_char,
    desc: *const c_char,
    info: *const c_char,
    values: [OptionValue; OPTION_VALUES],
    default_value: *const c_char,
}

#[repr(C)]
struct IntlV1 {
    us: *const DefinitionV1,
    local: *const DefinitionV1,
}

#[repr(C)]
struct CategoryV2 {
    key: *const c_char,
    desc: *const c_char,
    info: *const c_char,
}

#[repr(C)]
struct DefinitionV2 {
    key: *const c_char,
    desc: *const c_char,
    desc_categorized: *const c_char,
    info: *const c_char,
    info_categorized: *const c_char,
    category_key: *const c_char,
    values: [OptionValue; OPTION_VALUES],
    default_value: *const c_char,
}

#[repr(C)]
struct OptionsV2 {
    categories: *const CategoryV2,
    definitions: *const DefinitionV2,
}

#[repr(C)]
struct IntlV2 {
    us: *const OptionsV2,
    local: *const OptionsV2,
}

#[repr(C)]
struct OptionDisplay {
    key: *const c_char,
    visible: bool,
}

/// One of the emulator's options, as the core describes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreOption {
    pub key: String,
    pub label: String,
    pub info: Option<String>,
    /// Key of its category, when the core groups its options.
    pub category: Option<String>,
    pub values: Vec<OptionChoice>,
    pub default: String,
    /// The core can hide options that make no sense with the others' values.
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionChoice {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionCategory {
    pub key: String,
    pub label: String,
    pub info: Option<String>,
}

/// Everything a core lets the user set.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreOptions {
    pub categories: Vec<OptionCategory>,
    pub options: Vec<CoreOption>,
}

/// A C string the core owns, if any.
///
/// # Safety
/// `text` is null or a valid C string.
unsafe fn text(text: *const c_char) -> Option<String> {
    // SAFETY: as stated by the caller.
    (!text.is_null()).then(|| {
        unsafe { CStr::from_ptr(text) }
            .to_string_lossy()
            .into_owned()
    })
}

/// The values of an option definition: the array ends at the first null value.
///
/// # Safety
/// `values` comes from a core's option definition.
unsafe fn choices(values: &[OptionValue; OPTION_VALUES]) -> Vec<OptionChoice> {
    values
        .iter()
        .take_while(|value| !value.value.is_null())
        .filter_map(|value| {
            // SAFETY: non-null strings of the core's definition.
            let raw = unsafe { text(value.value) }?;
            let label = unsafe { text(value.label) }.unwrap_or_else(|| raw.clone());
            Some(OptionChoice { value: raw, label })
        })
        .collect()
}

/// Reads a v1 definition array (ends with a null key).
///
/// # Safety
/// `cursor` is null or a core's array of definitions.
unsafe fn read_v1(mut cursor: *const DefinitionV1) -> Vec<CoreOption> {
    let mut options = Vec::new();
    // SAFETY: the array ends with an entry whose key is null.
    unsafe {
        while !cursor.is_null() && !(*cursor).key.is_null() {
            let definition = &*cursor;
            let values = choices(&definition.values);
            let key = text(definition.key).unwrap_or_default();
            options.push(CoreOption {
                label: text(definition.desc).unwrap_or_else(|| key.clone()),
                info: text(definition.info),
                category: None,
                default: text(definition.default_value)
                    .or_else(|| values.first().map(|choice| choice.value.clone()))
                    .unwrap_or_default(),
                values,
                visible: true,
                key,
            });
            cursor = cursor.add(1);
        }
    }
    options
}

/// Reads a v2 option set.
///
/// # Safety
/// `options` is null or a core's v2 option set.
unsafe fn read_v2(options: *const OptionsV2) -> CoreOptions {
    let mut read = CoreOptions::default();
    if options.is_null() {
        return read;
    }
    // SAFETY: both arrays end with an entry whose key is null.
    unsafe {
        let mut category = (*options).categories;
        while !category.is_null() && !(*category).key.is_null() {
            read.categories.push(OptionCategory {
                key: text((*category).key).unwrap_or_default(),
                label: text((*category).desc).unwrap_or_default(),
                info: text((*category).info),
            });
            category = category.add(1);
        }
        let mut cursor = (*options).definitions;
        while !cursor.is_null() && !(*cursor).key.is_null() {
            let definition = &*cursor;
            let values = choices(&definition.values);
            let key = text(definition.key).unwrap_or_default();
            let category = text(definition.category_key);
            // Inside its category, an option has a shorter name.
            let label = category
                .as_ref()
                .and_then(|_| text(definition.desc_categorized))
                .or_else(|| text(definition.desc))
                .unwrap_or_else(|| key.clone());
            let info = category
                .as_ref()
                .and_then(|_| text(definition.info_categorized))
                .or_else(|| text(definition.info));
            read.options.push(CoreOption {
                label,
                info,
                category,
                default: text(definition.default_value)
                    .or_else(|| values.first().map(|choice| choice.value.clone()))
                    .unwrap_or_default(),
                values,
                visible: true,
                key,
            });
            cursor = cursor.add(1);
        }
    }
    read
}

/// English definitions with the translated words laid over them.
fn translate(mut us: CoreOptions, local: CoreOptions) -> CoreOptions {
    for option in &mut us.options {
        if let Some(translated) = local.options.iter().find(|other| other.key == option.key) {
            option.label = translated.label.clone();
            option.info = translated.info.clone().or(option.info.take());
            for choice in &mut option.values {
                if let Some(label) = translated
                    .values
                    .iter()
                    .find(|other| other.value == choice.value)
                    .map(|other| other.label.clone())
                {
                    choice.label = label;
                }
            }
        }
    }
    for category in &mut us.categories {
        if let Some(translated) = local
            .categories
            .iter()
            .find(|other| other.key == category.key)
        {
            category.label = translated.label.clone();
            category.info = translated.info.clone().or(category.info.take());
        }
    }
    us
}

#[repr(C)]
struct RumbleInterface {
    set_rumble_state: unsafe extern "C" fn(port: c_uint, effect: c_uint, strength: u16) -> bool,
}

/// How the core lays out the pixels of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// 16 bits, the default: 5 bits per colour, the top bit unused.
    #[default]
    Rgb1555,
    /// 32 bits, `0xXXRRGGBB`.
    Xrgb8888,
    /// 16 bits: 5, 6 and 5 bits.
    Rgb565,
}

impl PixelFormat {
    pub fn bytes(self) -> usize {
        match self {
            PixelFormat::Xrgb8888 => 4,
            PixelFormat::Rgb1555 | PixelFormat::Rgb565 => 2,
        }
    }
}

/// The last picture the core drew, rows packed tightly.
#[derive(Default)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub format: PixelFormat,
    pub pixels: Vec<u8>,
    /// Drawn since the screen last showed it.
    pub fresh: bool,
}

/// What a player is doing this frame, for the core.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PadInput {
    /// `JOYPAD_*` bits.
    pub buttons: u16,
    /// Sticks, from -32768 to 32767 (Y grows downwards).
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
    /// Triggers, from 0 to 32767.
    pub l2: i16,
    pub r2: i16,
}

/// A core option: the value in use (the game's own, else the core's default).
struct OptionValueInUse {
    current: CString,
}

/// Everything the callbacks read and write. Cores call back on the thread that runs them,
/// so it lives in a thread-local.
#[derive(Default)]
pub struct Frontend {
    system_dir: CString,
    save_dir: CString,
    pub format: PixelFormat,
    pub frame: Frame,
    /// Stereo samples produced since they were last taken.
    pub audio: Vec<i16>,
    pub pads: [PadInput; 4],
    options: HashMap<String, OptionValueInUse>,
    /// The options as the core described them, for Lutra's settings screens.
    pub definitions: CoreOptions,
    /// `RETRO_LANGUAGE_*` of the interface: cores that are translated speak it.
    language: c_uint,
    /// The game's own option values, which win over the defaults.
    overrides: HashMap<String, String>,
    options_changed: bool,
    /// New sizes or timings announced by the core.
    pub av_info: Option<AvInfo>,
    /// Rumble the core asked for: `(port, strong, weak)`, 0 to 65535.
    pub rumble: [(u16, u16); 4],
    /// The core asked to quit.
    pub shutdown: bool,
}

thread_local! {
    pub static FRONTEND: RefCell<Frontend> = RefCell::new(Frontend::default());
}

fn with<R>(f: impl FnOnce(&mut Frontend) -> R) -> R {
    FRONTEND.with(|frontend| f(&mut frontend.borrow_mut()))
}

impl Frontend {
    /// Takes the options a core declared: their values become the game's own when valid, else
    /// the core's defaults.
    fn declare(&mut self, options: CoreOptions) {
        for option in &options.options {
            let current = self
                .overrides
                .get(&option.key)
                .filter(|wanted| option.values.iter().any(|choice| &choice.value == *wanted))
                .unwrap_or(&option.default);
            self.options.insert(
                option.key.clone(),
                OptionValueInUse {
                    current: CString::new(current.as_str()).unwrap_or_default(),
                },
            );
        }
        self.definitions = options;
        self.options_changed = true;
    }
}

/// Parses `"Description; first|second|third"`, the format of `SET_VARIABLES`.
fn option_values(spec: &str) -> Vec<String> {
    let values = spec.split_once(';').map_or(spec, |(_, values)| values);
    values
        .trim()
        .split('|')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

unsafe extern "C" fn environment(cmd: c_uint, data: *mut c_void) -> bool {
    // SAFETY (whole function): each command's `data` points to what `libretro.h` documents for
    // it; null is checked where the API allows it.
    unsafe {
        match cmd & !EXPERIMENTAL {
            // GET_OVERSCAN: no overscan cropping.
            2 => {
                *(data as *mut bool) = false;
                true
            }
            // GET_CAN_DUPE: the core may repeat the previous frame by sending none.
            3 => {
                *(data as *mut bool) = true;
                true
            }
            // SET_MESSAGE
            6 => {
                let message = &*(data as *const Message);
                if !message.msg.is_null() {
                    eprintln!("core: {}", CStr::from_ptr(message.msg).to_string_lossy());
                }
                true
            }
            // SHUTDOWN
            7 => with(|frontend| {
                frontend.shutdown = true;
                true
            }),
            // SET_PERFORMANCE_LEVEL, SET_INPUT_DESCRIPTORS, SET_SUPPORT_NO_GAME, SET_CONTROLLER_INFO,
            // SET_MEMORY_MAPS, SET_SUPPORT_ACHIEVEMENTS, SET_SERIALIZATION_QUIRKS,
            // SET_MINIMUM_AUDIO_LATENCY: noted, nothing to do.
            8 | 11 | 18 | 35 | 36 | 42 | 44 | 63 => true,
            // GET_SYSTEM_DIRECTORY (BIOS files), GET_CORE_ASSETS_DIRECTORY
            9 | 30 => with(|frontend| {
                *(data as *mut *const c_char) = frontend.system_dir.as_ptr();
                true
            }),
            // SET_PIXEL_FORMAT
            10 => {
                let format = match *(data as *const c_int) {
                    0 => PixelFormat::Rgb1555,
                    1 => PixelFormat::Xrgb8888,
                    2 => PixelFormat::Rgb565,
                    _ => return false,
                };
                with(|frontend| frontend.format = format);
                true
            }
            // GET_VARIABLE
            15 => {
                let variable = &mut *(data as *mut Variable);
                if variable.key.is_null() {
                    return false;
                }
                let key = CStr::from_ptr(variable.key).to_string_lossy().into_owned();
                with(|frontend| match frontend.options.get(&key) {
                    Some(option) => {
                        variable.value = option.current.as_ptr();
                        true
                    }
                    None => {
                        variable.value = std::ptr::null();
                        false
                    }
                })
            }
            // SET_VARIABLES: an array ending with a null key, `"Description; a|b|c"` each.
            16 => {
                let mut cursor = data as *const Variable;
                let mut declared = CoreOptions::default();
                while !cursor.is_null() && !(*cursor).key.is_null() {
                    let key = text((*cursor).key).unwrap_or_default();
                    let spec = text((*cursor).value).unwrap_or_default();
                    let values: Vec<OptionChoice> = option_values(&spec)
                        .into_iter()
                        .map(|value| OptionChoice {
                            label: value.clone(),
                            value,
                        })
                        .collect();
                    declared.options.push(CoreOption {
                        label: spec
                            .split_once(';')
                            .map_or(key.as_str(), |(label, _)| label)
                            .trim()
                            .to_string(),
                        info: None,
                        category: None,
                        default: values
                            .first()
                            .map(|choice| choice.value.clone())
                            .unwrap_or_default(),
                        values,
                        visible: true,
                        key,
                    });
                    cursor = cursor.add(1);
                }
                with(|frontend| frontend.declare(declared));
                true
            }
            // GET_VARIABLE_UPDATE
            17 => with(|frontend| {
                *(data as *mut bool) = std::mem::take(&mut frontend.options_changed);
                true
            }),
            // GET_RUMBLE_INTERFACE
            23 => {
                (*(data as *mut RumbleInterface)).set_rumble_state = set_rumble;
                true
            }
            // GET_INPUT_DEVICE_CAPABILITIES
            24 => {
                *(data as *mut u64) = (1 << DEVICE_JOYPAD) | (1 << DEVICE_ANALOG);
                true
            }
            // GET_SAVE_DIRECTORY
            31 => with(|frontend| {
                *(data as *mut *const c_char) = frontend.save_dir.as_ptr();
                true
            }),
            // SET_SYSTEM_AV_INFO
            32 => {
                let info = *(data as *const AvInfo);
                with(|frontend| frontend.av_info = Some(info));
                true
            }
            // SET_GEOMETRY: same timing, new picture size.
            37 => {
                let geometry = *(data as *const Geometry);
                with(|frontend| {
                    let mut info = frontend.av_info.unwrap_or_default();
                    info.geometry = geometry;
                    frontend.av_info = Some(info);
                });
                true
            }
            // GET_LANGUAGE: the interface's.
            39 => with(|frontend| {
                *(data as *mut c_uint) = frontend.language;
                true
            }),
            // GET_CORE_OPTIONS_VERSION: labels, explanations and categories are welcome.
            52 => {
                *(data as *mut c_uint) = 2;
                true
            }
            // SET_CORE_OPTIONS (v1)
            53 => {
                let options = read_v1(data as *const DefinitionV1);
                with(|frontend| {
                    frontend.declare(CoreOptions {
                        categories: Vec::new(),
                        options,
                    })
                });
                true
            }
            // SET_CORE_OPTIONS_INTL (v1)
            54 => {
                let intl = &*(data as *const IntlV1);
                let us = CoreOptions {
                    categories: Vec::new(),
                    options: read_v1(intl.us),
                };
                let local = CoreOptions {
                    categories: Vec::new(),
                    options: read_v1(intl.local),
                };
                with(|frontend| frontend.declare(translate(us, local)));
                true
            }
            // SET_CORE_OPTIONS_DISPLAY: show or hide one option.
            55 => {
                let display = &*(data as *const OptionDisplay);
                if let Some(key) = text(display.key) {
                    with(|frontend| {
                        if let Some(option) = frontend
                            .definitions
                            .options
                            .iter_mut()
                            .find(|option| option.key == key)
                        {
                            option.visible = display.visible;
                        }
                    });
                }
                true
            }
            // SET_CORE_OPTIONS_V2: true says categories are shown.
            67 => {
                let options = read_v2(data as *const OptionsV2);
                with(|frontend| frontend.declare(options));
                true
            }
            // SET_CORE_OPTIONS_V2_INTL
            68 => {
                let intl = &*(data as *const IntlV2);
                let options = translate(read_v2(intl.us), read_v2(intl.local));
                with(|frontend| frontend.declare(options));
                true
            }
            // GET_AUDIO_VIDEO_ENABLE: both.
            47 => {
                if !data.is_null() {
                    *(data as *mut c_int) = 3;
                }
                true
            }
            // GET_INPUT_BITMASKS: the whole joypad in one call.
            51 => true,
            // Anything else (hardware rendering, disk control, VFS...) is not offered.
            _ => false,
        }
    }
}

unsafe extern "C" fn set_rumble(port: c_uint, effect: c_uint, strength: u16) -> bool {
    with(|frontend| match frontend.rumble.get_mut(port as usize) {
        Some(rumble) => {
            if effect == 0 {
                rumble.0 = strength;
            } else {
                rumble.1 = strength;
            }
            true
        }
        None => false,
    })
}

unsafe extern "C" fn video_refresh(
    data: *const c_void,
    width: c_uint,
    height: c_uint,
    pitch: usize,
) {
    // A null frame repeats the previous one.
    if data.is_null() || width == 0 || height == 0 {
        return;
    }
    with(|frontend| {
        let format = frontend.format;
        let row = width as usize * format.bytes();
        let frame = &mut frontend.frame;
        frame.pixels.resize(row * height as usize, 0);
        for y in 0..height as usize {
            // SAFETY: the core guarantees `height` rows of `pitch` bytes at `data`.
            let source =
                unsafe { std::slice::from_raw_parts((data as *const u8).add(y * pitch), row) };
            frame.pixels[y * row..(y + 1) * row].copy_from_slice(source);
        }
        frame.width = width as usize;
        frame.height = height as usize;
        frame.format = format;
        frame.fresh = true;
    });
}

unsafe extern "C" fn audio_sample(left: i16, right: i16) {
    with(|frontend| frontend.audio.extend_from_slice(&[left, right]));
}

unsafe extern "C" fn audio_sample_batch(data: *const i16, frames: usize) -> usize {
    if !data.is_null() {
        // SAFETY: the core passes `frames` interleaved stereo frames.
        let samples = unsafe { std::slice::from_raw_parts(data, frames * 2) };
        with(|frontend| frontend.audio.extend_from_slice(samples));
    }
    frames
}

unsafe extern "C" fn input_poll() {}

unsafe extern "C" fn input_state(port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16 {
    with(|frontend| {
        let Some(pad) = frontend.pads.get(port as usize) else {
            return 0;
        };
        match (device & 0xff, index, id) {
            (DEVICE_JOYPAD, _, DEVICE_ID_JOYPAD_MASK) => pad.buttons as i16,
            (DEVICE_JOYPAD, _, id) if id < 16 => i16::from(pad.buttons & (1 << id) != 0),
            (DEVICE_ANALOG, 0, 0) => pad.lx,
            (DEVICE_ANALOG, 0, 1) => pad.ly,
            (DEVICE_ANALOG, 1, 0) => pad.rx,
            (DEVICE_ANALOG, 1, 1) => pad.ry,
            (DEVICE_ANALOG, DEVICE_INDEX_ANALOG_BUTTON, 12) => pad.l2,
            (DEVICE_ANALOG, DEVICE_INDEX_ANALOG_BUTTON, 13) => pad.r2,
            _ => 0,
        }
    })
}

type Callback<F> = unsafe extern "C" fn(F);

/// A loaded core and its entry points.
pub struct Core {
    handle: *mut c_void,
    run: unsafe extern "C" fn(),
    reset: unsafe extern "C" fn(),
    deinit: unsafe extern "C" fn(),
    unload_game: unsafe extern "C" fn(),
    load_game: unsafe extern "C" fn(*const GameInfo) -> bool,
    av_info: unsafe extern "C" fn(*mut AvInfo),
    serialize_size: unsafe extern "C" fn() -> usize,
    serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    unserialize: unsafe extern "C" fn(*const c_void, usize) -> bool,
    memory_data: unsafe extern "C" fn(c_uint) -> *mut c_void,
    memory_size: unsafe extern "C" fn(c_uint) -> usize,
    set_port_device: unsafe extern "C" fn(c_uint, c_uint),
    /// The ROM's bytes, when the core wants them in memory: they must outlive the game.
    _rom: Vec<u8>,
    loaded: bool,
}

/// Looks a function up in a core.
///
/// # Safety
/// `T` must be the function pointer type the symbol really has.
unsafe fn symbol<T: Copy>(handle: *mut c_void, name: &str) -> Result<T, String> {
    let c_name = CString::new(name).map_err(|e| e.to_string())?;
    // SAFETY: `handle` comes from `dlopen` and is still open.
    let found = unsafe { libc::dlsym(handle, c_name.as_ptr()) };
    if found.is_null() {
        return Err(format!("the core has no {name}"));
    }
    // SAFETY: the caller states that `T` is the symbol's type; both are pointer-sized.
    Ok(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&found) })
}

impl Core {
    /// Loads the core at `path` and starts it, with BIOS files looked for in `system_dir`, save
    /// files in `save_dir`, and the game's own option values.
    pub fn open(
        path: &Path,
        system_dir: &Path,
        save_dir: &Path,
        overrides: HashMap<String, String>,
        language: c_uint,
    ) -> Result<Self, String> {
        let c_path =
            CString::new(path.as_os_str().as_encoded_bytes()).map_err(|e| e.to_string())?;
        // SAFETY: `dlopen` of a core Lutra installed and checked (`cores::probe`).
        let handle = unsafe { libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        if handle.is_null() {
            return Err(format!("could not load the core {}", path.display()));
        }
        with(|frontend| {
            frontend.system_dir =
                CString::new(system_dir.as_os_str().as_encoded_bytes()).unwrap_or_default();
            frontend.save_dir =
                CString::new(save_dir.as_os_str().as_encoded_bytes()).unwrap_or_default();
            frontend.overrides = overrides;
            frontend.language = language;
        });

        // SAFETY: each type below is the one `libretro.h` gives the function.
        unsafe {
            symbol::<Callback<unsafe extern "C" fn(c_uint, *mut c_void) -> bool>>(
                handle,
                "retro_set_environment",
            )?(environment);
            symbol::<Callback<unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize)>>(
                handle,
                "retro_set_video_refresh",
            )?(video_refresh);
            symbol::<Callback<unsafe extern "C" fn(i16, i16)>>(handle, "retro_set_audio_sample")?(
                audio_sample,
            );
            symbol::<Callback<unsafe extern "C" fn(*const i16, usize) -> usize>>(
                handle,
                "retro_set_audio_sample_batch",
            )?(audio_sample_batch);
            symbol::<Callback<unsafe extern "C" fn()>>(handle, "retro_set_input_poll")?(input_poll);
            symbol::<Callback<unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16>>(
                handle,
                "retro_set_input_state",
            )?(input_state);
            symbol::<unsafe extern "C" fn()>(handle, "retro_init")?();

            Ok(Self {
                run: symbol(handle, "retro_run")?,
                reset: symbol(handle, "retro_reset")?,
                deinit: symbol(handle, "retro_deinit")?,
                unload_game: symbol(handle, "retro_unload_game")?,
                load_game: symbol(handle, "retro_load_game")?,
                av_info: symbol(handle, "retro_get_system_av_info")?,
                serialize_size: symbol(handle, "retro_serialize_size")?,
                serialize: symbol(handle, "retro_serialize")?,
                unserialize: symbol(handle, "retro_unserialize")?,
                memory_data: symbol(handle, "retro_get_memory_data")?,
                memory_size: symbol(handle, "retro_get_memory_size")?,
                set_port_device: symbol(handle, "retro_set_controller_port_device")?,
                handle,
                _rom: Vec::new(),
                loaded: false,
            })
        }
    }

    /// Loads the game, in memory unless the core reads the file itself.
    pub fn load(&mut self, rom: &Path) -> Result<AvInfo, String> {
        let mut info = SystemInfo {
            library_name: std::ptr::null(),
            library_version: std::ptr::null(),
            valid_extensions: std::ptr::null(),
            need_fullpath: false,
            block_extract: false,
        };
        // SAFETY: plain call into the core with a struct it fills.
        unsafe {
            symbol::<unsafe extern "C" fn(*mut SystemInfo)>(self.handle, "retro_get_system_info")?(
                &mut info,
            );
        }
        if !info.need_fullpath {
            self._rom = std::fs::read(rom).map_err(|e| format!("could not read the game: {e}"))?;
        }
        let path = CString::new(rom.as_os_str().as_encoded_bytes()).map_err(|e| e.to_string())?;
        let game = GameInfo {
            path: path.as_ptr(),
            data: if self._rom.is_empty() {
                std::ptr::null()
            } else {
                self._rom.as_ptr().cast()
            },
            size: self._rom.len(),
            meta: std::ptr::null(),
        };
        // SAFETY: `game` and the bytes it points to live until the game is unloaded.
        if !unsafe { (self.load_game)(&game) } {
            return Err("the core could not open this game".into());
        }
        self.loaded = true;
        for port in 0..4 {
            // SAFETY: plain call into the core.
            unsafe { (self.set_port_device)(port, DEVICE_JOYPAD) };
        }
        let mut av = AvInfo::default();
        // SAFETY: plain call into the core with a struct it fills.
        unsafe { (self.av_info)(&mut av) };
        Ok(av)
    }

    pub fn run_frame(&mut self) {
        // SAFETY: a game is loaded; callbacks go to this thread's `FRONTEND`.
        unsafe { (self.run)() }
    }

    pub fn reset(&mut self) {
        // SAFETY: plain call into the core.
        unsafe { (self.reset)() }
    }

    /// The game's battery-backed memory (its in-game saves), as the core holds it.
    pub fn save_ram(&mut self) -> Option<&mut [u8]> {
        // SAFETY: the core owns this memory for as long as the game is loaded.
        unsafe {
            let size = (self.memory_size)(MEMORY_SAVE_RAM);
            let data = (self.memory_data)(MEMORY_SAVE_RAM);
            (size > 0 && !data.is_null())
                .then(|| std::slice::from_raw_parts_mut(data as *mut u8, size))
        }
    }

    /// The whole state of the console, to resume later.
    pub fn save_state(&mut self) -> Option<Vec<u8>> {
        // SAFETY: the buffer is as large as the core asks.
        unsafe {
            let size = (self.serialize_size)();
            let mut state = vec![0u8; size];
            (size > 0 && (self.serialize)(state.as_mut_ptr().cast(), size)).then_some(state)
        }
    }

    pub fn load_state(&mut self, state: &[u8]) -> bool {
        // SAFETY: the core reads `state.len()` bytes.
        unsafe { (self.unserialize)(state.as_ptr().cast(), state.len()) }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        // SAFETY: the game is unloaded and the core stopped before its library is closed.
        unsafe {
            if self.loaded {
                (self.unload_game)();
            }
            (self.deinit)();
            libc::dlclose(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_values_come_after_the_description() {
        assert_eq!(
            option_values("Colour correction; disabled|enabled"),
            vec!["disabled", "enabled"]
        );
        assert_eq!(option_values("auto|on"), vec!["auto", "on"]);
    }

    #[test]
    fn the_joypad_answers_button_by_button_and_as_a_mask() {
        FRONTEND.with(|frontend| {
            frontend.borrow_mut().pads[0] = PadInput {
                buttons: JOYPAD_A | JOYPAD_START,
                lx: -32768,
                ..PadInput::default()
            };
        });
        // SAFETY: plain calls of our own callback.
        unsafe {
            assert_eq!(input_state(0, DEVICE_JOYPAD, 0, 8), 1);
            assert_eq!(input_state(0, DEVICE_JOYPAD, 0, 0), 0);
            assert_eq!(
                input_state(0, DEVICE_JOYPAD, 0, DEVICE_ID_JOYPAD_MASK) as u16,
                JOYPAD_A | JOYPAD_START
            );
            assert_eq!(input_state(0, DEVICE_ANALOG, 0, 0), -32768);
            assert_eq!(input_state(5, DEVICE_JOYPAD, 0, 8), 0);
        }
    }
}
