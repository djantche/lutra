//! What a gamepad is, independently of any driver: buttons, a normalised state, and the
//! user's profile that turns the physical state into the one the game sees.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Buttons by position, following the Linux gamepad layout (South is the bottom face button:
/// A on Xbox, Cross on PlayStation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Button {
    South,
    East,
    North,
    West,
    L1,
    R1,
    L2,
    R2,
    Select,
    Start,
    Guide,
    L3,
    R3,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

impl Button {
    pub const ALL: [Button; 17] = [
        Button::South,
        Button::East,
        Button::North,
        Button::West,
        Button::L1,
        Button::R1,
        Button::L2,
        Button::R2,
        Button::Select,
        Button::Start,
        Button::Guide,
        Button::L3,
        Button::R3,
        Button::DpadUp,
        Button::DpadDown,
        Button::DpadLeft,
        Button::DpadRight,
    ];

    fn bit(self) -> u32 {
        1 << (self as u32)
    }

    /// The same position with A/B and X/Y swapped, as on Nintendo controllers.
    fn face_swapped(self) -> Self {
        match self {
            Button::South => Button::East,
            Button::East => Button::South,
            Button::North => Button::West,
            Button::West => Button::North,
            other => other,
        }
    }
}

/// A gamepad at one instant. Sticks go from -1 to 1 (Y grows downwards, like Linux), triggers
/// from 0 to 1.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PadState {
    pub(super) buttons: u32,
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    pub lt: f32,
    pub rt: f32,
}

impl PadState {
    pub fn pressed(&self, button: Button) -> bool {
        self.buttons & button.bit() != 0
    }

    pub fn set(&mut self, button: Button, down: bool) {
        if down {
            self.buttons |= button.bit();
        } else {
            self.buttons &= !button.bit();
        }
    }

    pub fn snapshot(&self) -> PadSnapshot {
        PadSnapshot {
            pressed: Button::ALL
                .into_iter()
                .filter(|button| self.pressed(*button))
                .collect(),
            lx: self.lx,
            ly: self.ly,
            rx: self.rx,
            ry: self.ry,
            lt: self.lt,
            rt: self.rt,
        }
    }
}

/// The state as the interface sees it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PadSnapshot {
    pub pressed: Vec<Button>,
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    pub lt: f32,
    pub rt: f32,
}

/// What the game sees instead of the physical controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputKind {
    /// Lutra picks what the game can use: an Xbox 360 pad for Windows games (they read XInput,
    /// which only an Xbox pad speaks), otherwise the family of the physical controller.
    #[default]
    Auto,
    /// A Microsoft Xbox 360 controller: the layout almost every PC game expects.
    Xbox360,
    /// A Sony DualShock 4: PlayStation button prompts in games that support them. Windows
    /// games only see it through DirectInput, never XInput.
    Ds4,
    /// No gamepad at all: the controller is converted to keyboard and mouse.
    Keyboard,
    /// The controller is left alone; the game sees it as it is.
    None,
}

impl OutputKind {
    /// The concrete pad a game gets: `Auto` is settled by the kind of game and the controller.
    pub fn resolve(self, windows_game: bool, physical_is_playstation: bool) -> Self {
        match self {
            OutputKind::Auto if windows_game => OutputKind::Xbox360,
            OutputKind::Auto if physical_is_playstation => OutputKind::Ds4,
            OutputKind::Auto => OutputKind::Xbox360,
            other => other,
        }
    }
}

/// What a stick turns into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StickMode {
    /// An analog stick, as the game sees a gamepad.
    #[default]
    Stick,
    /// The mouse pointer.
    Mouse,
    /// W, A, S and D.
    Wasd,
    /// The arrow keys.
    Arrows,
}

/// What the profile sends to the keyboard and mouse this instant: key names (see `convert`) and
/// the pointer's push, from -1 to 1 on each axis.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Routed {
    pub keys: BTreeSet<String>,
    pub mouse: (f32, f32),
}

/// How a controller behaves in a game. Stored as JSON, so new options can be added freely.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Profile {
    pub output: OutputKind,
    /// Physical button → button the game receives. Missing: unchanged. `null`: disabled.
    pub remap: BTreeMap<Button, Option<Button>>,
    /// Radius ignored around the centre of each stick, from 0 to 0.5.
    pub left_deadzone: f32,
    pub right_deadzone: f32,
    /// Trigger travel ignored at rest.
    pub trigger_deadzone: f32,
    /// Response curve of the right stick (aiming): 1 is linear, more is finer near the centre.
    pub right_curve: f32,
    pub invert_left_y: bool,
    pub invert_right_y: bool,
    pub swap_sticks: bool,
    /// Swap A/B and X/Y, for controllers with a Nintendo layout.
    pub swap_face: bool,
    /// Forward the game's vibrations to the controller.
    pub rumble: bool,
    /// Physical button → keyboard or mouse key (see `convert::KEYS`). It no longer reaches the
    /// gamepad, whatever `remap` says.
    pub keys: BTreeMap<Button, String>,
    pub left_stick: StickMode,
    pub right_stick: StickMode,
    /// Pointer speed in pixels per second at full tilt, when a stick is the mouse.
    pub mouse_speed: f32,
}

impl Profile {
    /// Does this profile need a virtual keyboard and mouse next to (or instead of) the pad?
    pub fn converts_to_keyboard(&self) -> bool {
        self.output == OutputKind::Keyboard
            || !self.keys.is_empty()
            || self.left_stick != StickMode::Stick
            || self.right_stick != StickMode::Stick
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            output: OutputKind::Auto,
            remap: BTreeMap::new(),
            left_deadzone: 0.08,
            right_deadzone: 0.08,
            trigger_deadzone: 0.04,
            right_curve: 1.0,
            invert_left_y: false,
            invert_right_y: false,
            swap_sticks: false,
            swap_face: false,
            rumble: true,
            keys: BTreeMap::new(),
            left_stick: StickMode::Stick,
            right_stick: StickMode::Stick,
            mouse_speed: 900.0,
        }
    }
}

/// Radial dead zone (keeps the direction, rescales what is left) then response curve.
fn shape_stick((x, y): (f32, f32), deadzone: f32, curve: f32) -> (f32, f32) {
    let magnitude = (x * x + y * y).sqrt();
    let deadzone = deadzone.clamp(0.0, 0.9);
    if magnitude <= deadzone || magnitude == 0.0 {
        return (0.0, 0.0);
    }
    let scaled = ((magnitude - deadzone) / (1.0 - deadzone))
        .min(1.0)
        .powf(curve.max(0.1));
    (x / magnitude * scaled, y / magnitude * scaled)
}

fn shape_trigger(value: f32, deadzone: f32) -> f32 {
    let deadzone = deadzone.clamp(0.0, 0.9);
    if value <= deadzone {
        0.0
    } else {
        ((value - deadzone) / (1.0 - deadzone)).min(1.0)
    }
}

/// A direction key held while a stick leans this far.
const STICK_KEY_THRESHOLD: f32 = 0.5;

/// Turns the physical state into what the game receives, following the profile.
pub fn apply(profile: &Profile, raw: &PadState) -> PadState {
    convert(profile, raw).0
}

/// Splits the physical state: what the virtual gamepad gets, and what goes to the keyboard and
/// mouse instead. Whatever is sent to the keyboard or mouse is taken away from the gamepad.
pub fn convert(profile: &Profile, raw: &PadState) -> (PadState, Routed) {
    let mut out = PadState::default();
    let mut routed = Routed::default();

    // A stick that is not a stick (mouse, direction keys) is taken out first, by physical side.
    let mut convert_stick = |value: (f32, f32), mode: StickMode, deadzone: f32, curve: f32| {
        if mode == StickMode::Stick {
            return value;
        }
        let (x, y) = shape_stick(value, deadzone, curve);
        match mode {
            StickMode::Mouse => {
                routed.mouse.0 += x;
                routed.mouse.1 += y;
            }
            StickMode::Wasd | StickMode::Arrows => {
                let names = if mode == StickMode::Wasd {
                    ["w", "s", "a", "d"]
                } else {
                    ["up", "down", "left", "right"]
                };
                for (name, held) in names.into_iter().zip([
                    y < -STICK_KEY_THRESHOLD,
                    y > STICK_KEY_THRESHOLD,
                    x < -STICK_KEY_THRESHOLD,
                    x > STICK_KEY_THRESHOLD,
                ]) {
                    if held {
                        routed.keys.insert(name.to_owned());
                    }
                }
            }
            StickMode::Stick => {}
        }
        (0.0, 0.0)
    };
    let mut left = convert_stick(
        (raw.lx, raw.ly),
        profile.left_stick,
        profile.left_deadzone,
        1.0,
    );
    let mut right = convert_stick(
        (raw.rx, raw.ry),
        profile.right_stick,
        profile.right_deadzone,
        profile.right_curve,
    );
    if profile.swap_sticks {
        std::mem::swap(&mut left, &mut right);
    }
    let (lx, ly) = shape_stick(left, profile.left_deadzone, 1.0);
    let (rx, ry) = shape_stick(right, profile.right_deadzone, profile.right_curve);
    out.lx = lx;
    out.ly = if profile.invert_left_y { -ly } else { ly };
    out.rx = rx;
    out.ry = if profile.invert_right_y { -ry } else { ry };

    // An analog trigger only reaches the game if its button was not moved elsewhere.
    let kept = |button: Button| {
        !profile.remap.contains_key(&button) && !profile.keys.contains_key(&button)
    };
    if kept(Button::L2) {
        out.lt = shape_trigger(raw.lt, profile.trigger_deadzone);
    }
    if kept(Button::R2) {
        out.rt = shape_trigger(raw.rt, profile.trigger_deadzone);
    }

    for button in Button::ALL
        .into_iter()
        .filter(|button| raw.pressed(*button))
    {
        let button = if profile.swap_face {
            button.face_swapped()
        } else {
            button
        };
        if let Some(key) = profile.keys.get(&button) {
            routed.keys.insert(key.clone());
            continue;
        }
        match profile.remap.get(&button) {
            Some(Some(target)) => out.set(*target, true),
            Some(None) => {}
            None => out.set(button, true),
        }
    }

    // A button sent to a trigger presses it fully.
    if out.pressed(Button::L2) {
        out.lt = out.lt.max(1.0);
    }
    if out.pressed(Button::R2) {
        out.rt = out.rt.max(1.0);
    }
    (out, routed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_gives_windows_games_an_xbox_pad_and_others_the_family_of_the_controller() {
        assert_eq!(OutputKind::Auto.resolve(true, true), OutputKind::Xbox360);
        assert_eq!(OutputKind::Auto.resolve(false, true), OutputKind::Ds4);
        assert_eq!(OutputKind::Auto.resolve(false, false), OutputKind::Xbox360);
        // A choice made by the user is never second-guessed.
        assert_eq!(OutputKind::Ds4.resolve(true, true), OutputKind::Ds4);
    }

    #[test]
    fn a_button_bound_to_a_key_leaves_the_gamepad() {
        let mut profile = Profile::default();
        profile.keys.insert(Button::South, "enter".into());
        let (out, routed) = convert(&profile, &pad(&[Button::South, Button::East]));
        assert!(!out.pressed(Button::South));
        assert!(out.pressed(Button::East));
        assert_eq!(routed.keys, BTreeSet::from(["enter".to_owned()]));
    }

    #[test]
    fn a_trigger_bound_to_a_key_no_longer_reaches_the_game_as_an_axis() {
        let mut profile = Profile::default();
        profile.keys.insert(Button::R2, "mouse_left".into());
        let mut raw = pad(&[Button::R2]);
        raw.rt = 1.0;
        let (out, routed) = convert(&profile, &raw);
        assert_eq!(out.rt, 0.0);
        assert!(routed.keys.contains("mouse_left"));
    }

    #[test]
    fn a_stick_as_wasd_holds_the_keys_of_its_direction_and_frees_the_axes() {
        let profile = Profile {
            left_stick: StickMode::Wasd,
            ..Profile::default()
        };
        let raw = PadState {
            lx: 0.9,
            ly: -0.9,
            ..PadState::default()
        };
        let (out, routed) = convert(&profile, &raw);
        assert_eq!((out.lx, out.ly), (0.0, 0.0));
        assert_eq!(
            routed.keys,
            BTreeSet::from(["w".to_owned(), "d".to_owned()])
        );
        // A light push stays under the threshold.
        let (_, light) = convert(
            &profile,
            &PadState {
                lx: 0.3,
                ..PadState::default()
            },
        );
        assert!(light.keys.is_empty());
    }

    #[test]
    fn a_stick_as_the_mouse_pushes_the_pointer_and_frees_the_axes() {
        let profile = Profile {
            right_stick: StickMode::Mouse,
            ..Profile::default()
        };
        let (out, routed) = convert(
            &profile,
            &PadState {
                rx: 1.0,
                ..PadState::default()
            },
        );
        assert_eq!((out.rx, out.ry), (0.0, 0.0));
        assert!(routed.mouse.0 > 0.9);
        assert!(profile.converts_to_keyboard());
        assert!(!Profile::default().converts_to_keyboard());
    }

    #[test]
    fn sticks_keep_their_mode_when_swapped() {
        let profile = Profile {
            swap_sticks: true,
            right_stick: StickMode::Mouse,
            ..Profile::default()
        };
        // The mode belongs to the physical stick: the right one is the mouse, whatever the swap.
        let (_, routed) = convert(
            &profile,
            &PadState {
                rx: 1.0,
                lx: 1.0,
                ..PadState::default()
            },
        );
        assert!(routed.mouse.0 > 0.9);
        // The physical left stick, still a stick, now drives the game's right one.
        let (out, _) = convert(
            &profile,
            &PadState {
                lx: 1.0,
                ..PadState::default()
            },
        );
        assert!(out.rx > 0.9 && out.lx == 0.0);
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn pad(buttons: &[Button]) -> PadState {
        let mut state = PadState::default();
        for button in buttons {
            state.set(*button, true);
        }
        state
    }

    #[test]
    fn small_stick_movements_inside_the_dead_zone_are_ignored() {
        let profile = Profile::default();
        let raw = PadState {
            lx: 0.05,
            ly: -0.05,
            ..PadState::default()
        };
        let out = apply(&profile, &raw);
        assert_eq!((out.lx, out.ly), (0.0, 0.0));
    }

    #[test]
    fn the_dead_zone_rescales_so_full_tilt_stays_full() {
        let profile = Profile::default();
        let out = apply(
            &profile,
            &PadState {
                lx: 1.0,
                ..PadState::default()
            },
        );
        assert!(close(out.lx, 1.0));
        let half = apply(
            &profile,
            &PadState {
                lx: 0.54,
                ..PadState::default()
            },
        );
        assert!(close(half.lx, 0.5), "{}", half.lx);
    }

    #[test]
    fn the_dead_zone_keeps_the_stick_direction() {
        let profile = Profile::default();
        let out = apply(
            &profile,
            &PadState {
                lx: 0.6,
                ly: 0.6,
                ..PadState::default()
            },
        );
        assert!(close(out.lx, out.ly));
    }

    #[test]
    fn a_steeper_curve_softens_small_aiming_movements() {
        let profile = Profile {
            right_curve: 2.0,
            right_deadzone: 0.0,
            ..Profile::default()
        };
        let out = apply(
            &profile,
            &PadState {
                rx: 0.5,
                ..PadState::default()
            },
        );
        assert!(close(out.rx, 0.25));
    }

    #[test]
    fn sticks_can_be_inverted_and_swapped() {
        let profile = Profile {
            invert_right_y: true,
            swap_sticks: true,
            left_deadzone: 0.0,
            right_deadzone: 0.0,
            ..Profile::default()
        };
        let out = apply(
            &profile,
            &PadState {
                ly: 0.5,
                ..PadState::default()
            },
        );
        assert!(close(out.ry, -0.5));
        assert_eq!(out.ly, 0.0);
    }

    #[test]
    fn buttons_pass_through_unless_remapped_or_disabled() {
        let mut profile = Profile::default();
        profile.remap.insert(Button::South, Some(Button::East));
        profile.remap.insert(Button::Select, None);

        let out = apply(
            &profile,
            &pad(&[Button::South, Button::Select, Button::Start]),
        );
        assert!(out.pressed(Button::East));
        assert!(!out.pressed(Button::South));
        assert!(!out.pressed(Button::Select));
        assert!(out.pressed(Button::Start));
    }

    #[test]
    fn the_nintendo_layout_swaps_face_buttons() {
        let profile = Profile {
            swap_face: true,
            ..Profile::default()
        };
        let out = apply(&profile, &pad(&[Button::South, Button::North]));
        assert!(out.pressed(Button::East));
        assert!(out.pressed(Button::West));
    }

    #[test]
    fn a_trigger_moved_to_a_button_no_longer_reaches_the_game_as_a_trigger() {
        let mut profile = Profile::default();
        profile.remap.insert(Button::L2, Some(Button::South));
        let mut raw = pad(&[Button::L2]);
        raw.lt = 1.0;

        let out = apply(&profile, &raw);
        assert_eq!(out.lt, 0.0);
        assert!(out.pressed(Button::South));
    }

    #[test]
    fn a_button_sent_to_a_trigger_presses_it_fully() {
        let mut profile = Profile::default();
        profile.remap.insert(Button::R1, Some(Button::R2));
        let out = apply(&profile, &pad(&[Button::R1]));
        assert_eq!(out.rt, 1.0);
    }

    #[test]
    fn trigger_dead_zone_ignores_a_resting_finger() {
        let profile = Profile::default();
        let out = apply(
            &profile,
            &PadState {
                lt: 0.02,
                rt: 1.0,
                ..PadState::default()
            },
        );
        assert_eq!(out.lt, 0.0);
        assert!(close(out.rt, 1.0));
    }

    #[test]
    fn profiles_survive_json_and_tolerate_missing_fields() {
        let mut profile = Profile {
            output: OutputKind::Ds4,
            ..Profile::default()
        };
        profile.remap.insert(Button::DpadUp, None);
        profile.remap.insert(Button::L1, Some(Button::L2));
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"output\":\"ds4\""));
        assert_eq!(serde_json::from_str::<Profile>(&json).unwrap(), profile);

        let sparse: Profile = serde_json::from_str(r#"{"output":"none"}"#).unwrap();
        assert_eq!(sparse.output, OutputKind::None);
        assert!(sparse.rumble);
    }

    #[test]
    fn snapshots_list_pressed_buttons_in_order() {
        let snapshot = pad(&[Button::Start, Button::South]).snapshot();
        assert_eq!(snapshot.pressed, vec![Button::South, Button::Start]);
    }
}
