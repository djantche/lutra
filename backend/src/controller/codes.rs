//! Linux input event codes: reading a physical controller into a `PadState`, and writing a
//! `PadState` exactly the way a real Xbox 360 pad or DualShock 4 would.

use std::time::{Duration, Instant};

use serde::Serialize;

use super::model::{Button, OutputKind, PadState};

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_ABS: u16 = 0x03;
pub const SYN_REPORT: u16 = 0;

pub mod key {
    pub const SOUTH: u16 = 0x130;
    pub const EAST: u16 = 0x131;
    /// `BTN_NORTH`, also called `BTN_X`.
    pub const X: u16 = 0x133;
    /// `BTN_WEST`, also called `BTN_Y`.
    pub const Y: u16 = 0x134;
    pub const TL: u16 = 0x136;
    pub const TR: u16 = 0x137;
    pub const TL2: u16 = 0x138;
    pub const TR2: u16 = 0x139;
    pub const SELECT: u16 = 0x13a;
    pub const START: u16 = 0x13b;
    pub const MODE: u16 = 0x13c;
    pub const THUMBL: u16 = 0x13d;
    pub const THUMBR: u16 = 0x13e;
    pub const DPAD_UP: u16 = 0x220;
    pub const DPAD_DOWN: u16 = 0x221;
    pub const DPAD_LEFT: u16 = 0x222;
    pub const DPAD_RIGHT: u16 = 0x223;
}

pub mod abs {
    pub const X: u16 = 0x00;
    pub const Y: u16 = 0x01;
    pub const Z: u16 = 0x02;
    pub const RX: u16 = 0x03;
    pub const RY: u16 = 0x04;
    pub const RZ: u16 = 0x05;
    pub const GAS: u16 = 0x09;
    pub const BRAKE: u16 = 0x0a;
    pub const HAT0X: u16 = 0x10;
    pub const HAT0Y: u16 = 0x11;
}

/// The maker of a controller, which decides labels and a few driver quirks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Xbox,
    PlayStation,
    Nintendo,
    Generic,
}

/// Which physical controller it is, to show the right picture of it. `Family` says how its
/// buttons are named; this says what it looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PadModel {
    DualShock4,
    DualSense,
    SteamDeck,
    Other,
}

impl PadModel {
    pub fn from_ids(vendor: u16, product: u16) -> Self {
        match (vendor, product) {
            // DualSense and DualSense Edge.
            (0x054c, 0x0ce6 | 0x0df2) => PadModel::DualSense,
            (0x054c, _) => PadModel::DualShock4,
            (0x28de, 0x1205) => PadModel::SteamDeck,
            _ => PadModel::Other,
        }
    }
}

impl Family {
    pub fn from_vendor(vendor: u16) -> Self {
        match vendor {
            0x045e => Family::Xbox,
            0x054c => Family::PlayStation,
            0x057e => Family::Nintendo,
            // Valve's pads (Steam Deck, Steam Controller) use the Xbox layout and prompts.
            0x28de => Family::Xbox,
            _ => Family::Generic,
        }
    }
}

/// The range an axis reports, from the device's own `absinfo`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: i32,
    pub max: i32,
}

impl Range {
    const STICK: Range = Range {
        min: -32768,
        max: 32767,
    };
    const TRIGGER: Range = Range { min: 0, max: 255 };

    /// The stick's position in -1..1. The rest position is the integer centre of the range
    /// (128 on an 8-bit axis, not 127.5), and each side is scaled on its own, so a pad at rest
    /// reads exactly 0 instead of a small constant bias.
    fn stick(self, value: i32) -> f32 {
        if self.max <= self.min {
            return 0.0;
        }
        let centre = (self.min + self.max + 1) / 2;
        let span = if value >= centre {
            self.max - centre
        } else {
            centre - self.min
        };
        if span <= 0 {
            return 0.0;
        }
        ((value - centre) as f32 / span as f32).clamp(-1.0, 1.0)
    }

    fn trigger(self, value: i32) -> f32 {
        if self.max <= self.min {
            return 0.0;
        }
        ((value - self.min) as f32 / (self.max - self.min) as f32).clamp(0.0, 1.0)
    }
}

/// A stick found within this distance of the centre while nothing is touched is taken to be at
/// rest: whatever offset it has is its own, and is subtracted from then on (what Steam Input
/// calls centre calibration). Worn sticks drift far: a DualShock 4 over Bluetooth was measured
/// resting at a fifth of its travel.
const REST_CALIBRATION_WINDOW: f32 = 0.3;

/// A stick moving less than this is still (a resting stick jitters by a count or two).
const REST_JITTER: f32 = 0.02;

/// How long the whole controller must be left alone before the sticks' rest is measured again.
const REST_SETTLE: Duration = Duration::from_secs(5);

/// Past this travel an analog trigger also counts as its button being pressed.
const TRIGGER_AS_BUTTON: f32 = 0.5;

/// Accumulates a physical controller's events into a state, one frame (`SYN_REPORT`) at a time.
pub struct Decoder {
    ranges: [Range; 8],
    state: PadState,
    l2_key: bool,
    r2_key: bool,
    /// Position of the left X/Y and right X/Y sticks before calibration.
    raw: [f32; 4],
    /// Their rest offset, measured by `calibrate` and `settle`.
    rest: [f32; 4],
    /// Where the sticks were when the controller was last touched, and since when.
    still: ([f32; 4], Option<Instant>),
    /// xpad (USB) swaps X and Y; Bluetooth Xbox drivers do not.
    xpad_layout: bool,
}

const AXES: [u16; 8] = [
    abs::X,
    abs::Y,
    abs::Z,
    abs::RX,
    abs::RY,
    abs::RZ,
    abs::GAS,
    abs::BRAKE,
];

impl Decoder {
    /// `range_of` gives each axis' range as the device reports it, if it has that axis.
    pub fn new(family: Family, range_of: impl Fn(u16) -> Option<Range>) -> Self {
        let ranges = AXES.map(|axis| {
            range_of(axis).unwrap_or(match axis {
                abs::X | abs::Y | abs::RX | abs::RY => Range::STICK,
                _ => Range::TRIGGER,
            })
        });
        Self {
            ranges,
            state: PadState::default(),
            l2_key: false,
            r2_key: false,
            raw: [0.0; 4],
            rest: [0.0; 4],
            still: ([0.0; 4], None),
            xpad_layout: family == Family::Xbox,
        }
    }

    /// Xbox pads over Bluetooth (xpadneo, hid-microsoft) report X and Y by position, unlike
    /// xpad, which is USB only.
    pub fn bluetooth(mut self) -> Self {
        self.xpad_layout = false;
        self
    }

    /// Takes the sticks' current position as their rest position, when they are close enough
    /// to the centre to be at rest. `current` gives the axis' value as the device reports it.
    pub fn calibrate(&mut self, current: impl Fn(u16) -> Option<i32>) {
        for (slot, axis) in [abs::X, abs::Y, abs::RX, abs::RY].into_iter().enumerate() {
            let Some(value) = current(axis) else { continue };
            let position = self.range(axis).stick(value);
            self.raw[slot] = position;
            if position.abs() <= REST_CALIBRATION_WINDOW {
                self.rest[slot] = position;
            }
        }
        self.still.0 = self.raw;
    }

    /// Measures the sticks' rest again once the controller has been left alone for a while, so a
    /// drift that appears or changes after connection goes away too. True when the state changed.
    /// Only for when no game runs: a game may hold a stick still on purpose.
    pub fn settle(&mut self, now: Instant) -> bool {
        let state = self.state();
        let touched = state.buttons != 0
            || state.lt > REST_JITTER
            || state.rt > REST_JITTER
            || (0..4).any(|slot| (self.raw[slot] - self.still.0[slot]).abs() > REST_JITTER);
        let since = *self.still.1.get_or_insert(now);
        if touched {
            self.still = (self.raw, Some(now));
            return false;
        }
        if now.duration_since(since) < REST_SETTLE {
            return false;
        }
        let mut changed = false;
        for slot in 0..4 {
            let position = self.raw[slot];
            if position.abs() <= REST_CALIBRATION_WINDOW && position != self.rest[slot] {
                self.rest[slot] = position;
                changed = true;
            }
        }
        if changed {
            self.state.lx = self.stick(0);
            self.state.ly = self.stick(1);
            self.state.rx = self.stick(2);
            self.state.ry = self.stick(3);
        }
        changed
    }

    /// A stick's position with its rest offset taken away.
    fn stick(&self, slot: usize) -> f32 {
        let rest = self.rest[slot];
        let position = self.raw[slot];
        // Each side is rescaled so full tilt still reaches 1 despite the shifted centre.
        let span = if position >= rest {
            1.0 - rest
        } else {
            1.0 + rest
        };
        ((position - rest) / span).clamp(-1.0, 1.0)
    }

    fn range(&self, axis: u16) -> Range {
        let index = AXES
            .iter()
            .position(|candidate| *candidate == axis)
            .unwrap_or(0);
        self.ranges[index]
    }

    fn button_for(&self, code: u16) -> Option<Button> {
        // xpad reports Xbox X (left) as BTN_X and Y (top) as BTN_Y, the reverse of the
        // positional layout every other driver follows.
        let xbox = self.xpad_layout;
        Some(match code {
            key::SOUTH => Button::South,
            key::EAST => Button::East,
            key::X if xbox => Button::West,
            key::X => Button::North,
            key::Y if xbox => Button::North,
            key::Y => Button::West,
            key::TL => Button::L1,
            key::TR => Button::R1,
            key::SELECT => Button::Select,
            key::START => Button::Start,
            key::MODE => Button::Guide,
            key::THUMBL => Button::L3,
            key::THUMBR => Button::R3,
            key::DPAD_UP => Button::DpadUp,
            key::DPAD_DOWN => Button::DpadDown,
            key::DPAD_LEFT => Button::DpadLeft,
            key::DPAD_RIGHT => Button::DpadRight,
            _ => return None,
        })
    }

    /// Feeds one event. Returns true when a frame is complete and `state()` is up to date.
    pub fn feed(&mut self, kind: u16, code: u16, value: i32) -> bool {
        match kind {
            EV_SYN => return code == SYN_REPORT,
            EV_KEY => match code {
                key::TL2 => self.l2_key = value != 0,
                key::TR2 => self.r2_key = value != 0,
                _ => {
                    if let Some(button) = self.button_for(code) {
                        self.state.set(button, value != 0);
                    }
                }
            },
            EV_ABS => {
                let range = self.range(code);
                let mut stick = |slot: usize| {
                    self.raw[slot] = range.stick(value);
                    self.stick(slot)
                };
                match code {
                    abs::X => self.state.lx = stick(0),
                    abs::Y => self.state.ly = stick(1),
                    abs::RX => self.state.rx = stick(2),
                    abs::RY => self.state.ry = stick(3),
                    abs::Z | abs::BRAKE => self.state.lt = range.trigger(value),
                    abs::RZ | abs::GAS => self.state.rt = range.trigger(value),
                    abs::HAT0X => {
                        self.state.set(Button::DpadLeft, value < 0);
                        self.state.set(Button::DpadRight, value > 0);
                    }
                    abs::HAT0Y => {
                        self.state.set(Button::DpadUp, value < 0);
                        self.state.set(Button::DpadDown, value > 0);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        false
    }

    pub fn state(&self) -> PadState {
        let mut state = self.state;
        // Some controllers send a trigger button, some only the axis: either one counts.
        state.set(Button::L2, self.l2_key || state.lt > TRIGGER_AS_BUTTON);
        state.set(Button::R2, self.r2_key || state.rt > TRIGGER_AS_BUTTON);
        state
    }
}

/// Button codes of each emulated controller. The Xbox 360 pad keeps xpad's X/Y order (that is
/// what games and SDL expect from it) and has no trigger buttons, only axes.
pub fn button_codes(kind: OutputKind) -> &'static [(Button, u16)] {
    const XBOX: &[(Button, u16)] = &[
        (Button::South, key::SOUTH),
        (Button::East, key::EAST),
        (Button::West, key::X),
        (Button::North, key::Y),
        (Button::L1, key::TL),
        (Button::R1, key::TR),
        (Button::Select, key::SELECT),
        (Button::Start, key::START),
        (Button::Guide, key::MODE),
        (Button::L3, key::THUMBL),
        (Button::R3, key::THUMBR),
    ];
    const DS4: &[(Button, u16)] = &[
        (Button::South, key::SOUTH),
        (Button::East, key::EAST),
        (Button::North, key::X),
        (Button::West, key::Y),
        (Button::L1, key::TL),
        (Button::R1, key::TR),
        (Button::L2, key::TL2),
        (Button::R2, key::TR2),
        (Button::Select, key::SELECT),
        (Button::Start, key::START),
        (Button::Guide, key::MODE),
        (Button::L3, key::THUMBL),
        (Button::R3, key::THUMBR),
    ];
    match kind {
        OutputKind::Xbox360 => XBOX,
        OutputKind::Ds4 => DS4,
        OutputKind::Auto | OutputKind::Keyboard | OutputKind::None => &[],
    }
}

/// Every value the emulated controller reports for a state, as `(type, code, value)`.
pub fn output_values(kind: OutputKind, state: &PadState) -> Vec<(u16, u16, i32)> {
    if matches!(
        kind,
        OutputKind::None | OutputKind::Keyboard | OutputKind::Auto
    ) {
        return Vec::new();
    }
    let mut values: Vec<(u16, u16, i32)> = button_codes(kind)
        .iter()
        .map(|(button, code)| (EV_KEY, *code, i32::from(state.pressed(*button))))
        .collect();

    let stick = |value: f32| match kind {
        OutputKind::Ds4 => ((value.clamp(-1.0, 1.0) + 1.0) / 2.0 * 255.0).round() as i32,
        _ => (value.clamp(-1.0, 1.0) * 32767.0).round() as i32,
    };
    let trigger = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as i32;
    let hat = |negative: Button, positive: Button| {
        i32::from(state.pressed(positive)) - i32::from(state.pressed(negative))
    };

    values.extend([
        (EV_ABS, abs::X, stick(state.lx)),
        (EV_ABS, abs::Y, stick(state.ly)),
        (EV_ABS, abs::RX, stick(state.rx)),
        (EV_ABS, abs::RY, stick(state.ry)),
        (EV_ABS, abs::Z, trigger(state.lt)),
        (EV_ABS, abs::RZ, trigger(state.rt)),
        (EV_ABS, abs::HAT0X, hat(Button::DpadLeft, Button::DpadRight)),
        (EV_ABS, abs::HAT0Y, hat(Button::DpadUp, Button::DpadDown)),
    ]);
    values
}

/// Only what changed between two frames, ready to be written to the virtual device.
pub fn diff(kind: OutputKind, previous: &PadState, next: &PadState) -> Vec<(u16, u16, i32)> {
    let before = output_values(kind, previous);
    output_values(kind, next)
        .into_iter()
        .zip(before)
        .filter(|(after, before)| after != before)
        .map(|(after, _)| after)
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn controllers_are_told_apart_by_their_ids() {
        use super::{Family, PadModel};
        assert_eq!(PadModel::from_ids(0x054c, 0x09cc), PadModel::DualShock4);
        assert_eq!(PadModel::from_ids(0x054c, 0x05c4), PadModel::DualShock4);
        assert_eq!(PadModel::from_ids(0x054c, 0x0ce6), PadModel::DualSense);
        assert_eq!(PadModel::from_ids(0x054c, 0x0df2), PadModel::DualSense);
        assert_eq!(PadModel::from_ids(0x28de, 0x1205), PadModel::SteamDeck);
        assert_eq!(PadModel::from_ids(0x045e, 0x028e), PadModel::Other);
        assert_eq!(Family::from_vendor(0x28de), Family::Xbox);
    }

    use super::*;

    fn ds4_ranges(axis: u16) -> Option<Range> {
        match axis {
            abs::HAT0X | abs::HAT0Y => None,
            _ => Some(Range { min: 0, max: 255 }),
        }
    }

    #[test]
    fn a_dualshock_frame_is_decoded_by_position() {
        let mut decoder = Decoder::new(Family::PlayStation, ds4_ranges);
        decoder.feed(EV_KEY, key::X, 1); // Triangle, top
        decoder.feed(EV_ABS, abs::X, 255);
        decoder.feed(EV_ABS, abs::Y, 0);
        decoder.feed(EV_ABS, abs::HAT0Y, -1);
        assert!(decoder.feed(EV_SYN, SYN_REPORT, 0));

        let state = decoder.state();
        assert!(state.pressed(Button::North));
        assert!(state.pressed(Button::DpadUp));
        assert_eq!((state.lx, state.ly), (1.0, -1.0));
    }

    #[test]
    fn xpad_x_and_y_are_put_back_in_their_positions() {
        let mut decoder = Decoder::new(Family::Xbox, |_| None);
        decoder.feed(EV_KEY, key::X, 1); // Xbox X, the left button
        assert!(decoder.state().pressed(Button::West));
        decoder.feed(EV_KEY, key::Y, 1); // Xbox Y, the top button
        assert!(decoder.state().pressed(Button::North));
    }

    #[test]
    fn a_stick_at_rest_reads_zero_even_on_an_8_bit_axis() {
        let mut decoder = Decoder::new(Family::PlayStation, ds4_ranges);
        decoder.feed(EV_ABS, abs::X, 128);
        decoder.feed(EV_ABS, abs::RY, 128);
        let state = decoder.state();
        assert_eq!((state.lx, state.ry), (0.0, 0.0));
    }

    #[test]
    fn the_rest_offset_found_at_connection_is_subtracted() {
        let mut decoder = Decoder::new(Family::PlayStation, ds4_ranges);
        decoder.calibrate(|axis| Some(if axis == abs::X { 140 } else { 128 }));
        decoder.feed(EV_ABS, abs::X, 140);
        assert_eq!(decoder.state().lx, 0.0);
        decoder.feed(EV_ABS, abs::X, 255);
        assert_eq!(decoder.state().lx, 1.0);
    }

    #[test]
    fn a_stick_held_aside_at_connection_is_not_calibrated() {
        let mut decoder = Decoder::new(Family::PlayStation, ds4_ranges);
        decoder.calibrate(|_| Some(255));
        decoder.feed(EV_ABS, abs::X, 255);
        assert_eq!(decoder.state().lx, 1.0);
    }

    #[test]
    fn a_drift_of_a_fifth_of_the_travel_is_calibrated_at_connection() {
        // Measured on a DualShock 4 over Bluetooth: left Y resting at 153 instead of 128.
        let mut decoder = Decoder::new(Family::PlayStation, ds4_ranges);
        decoder.calibrate(|axis| Some(if axis == abs::Y { 153 } else { 128 }));
        decoder.feed(EV_ABS, abs::Y, 152);
        assert!(decoder.state().ly.abs() < 0.01);
        decoder.feed(EV_ABS, abs::Y, 255);
        assert_eq!(decoder.state().ly, 1.0);
    }

    #[test]
    fn a_drift_appearing_later_is_learned_once_the_controller_is_left_alone() {
        let start = Instant::now();
        let mut decoder = Decoder::new(Family::PlayStation, ds4_ranges);
        decoder.calibrate(|_| Some(128));
        decoder.feed(EV_ABS, abs::Y, 153);
        assert!(!decoder.settle(start));
        assert!(decoder.state().ly > 0.15);
        decoder.feed(EV_ABS, abs::Y, 152);
        assert!(!decoder.settle(start + REST_SETTLE / 2));
        assert!(decoder.settle(start + REST_SETTLE));
        assert!(decoder.state().ly.abs() < 0.01);
    }

    #[test]
    fn a_stick_is_not_recentred_while_the_controller_is_in_use() {
        let start = Instant::now();
        let mut decoder = Decoder::new(Family::PlayStation, ds4_ranges);
        decoder.calibrate(|_| Some(128));
        decoder.feed(EV_ABS, abs::Y, 153);
        decoder.feed(EV_KEY, key::SOUTH, 1);
        decoder.settle(start);
        assert!(!decoder.settle(start + REST_SETTLE * 2));
        assert!(decoder.state().ly > 0.15);
    }

    #[test]
    fn bluetooth_xbox_pads_keep_positional_x_and_y() {
        let mut decoder = Decoder::new(Family::Xbox, |_| None).bluetooth();
        decoder.feed(EV_KEY, key::X, 1);
        assert!(decoder.state().pressed(Button::North));
    }

    #[test]
    fn analog_triggers_also_count_as_buttons_past_half_travel() {
        let mut decoder = Decoder::new(Family::Xbox, |axis| {
            (axis == abs::Z).then_some(Range { min: 0, max: 1023 })
        });
        decoder.feed(EV_ABS, abs::Z, 1023);
        let state = decoder.state();
        assert_eq!(state.lt, 1.0);
        assert!(state.pressed(Button::L2));
        assert!(!state.pressed(Button::R2));
    }

    #[test]
    fn the_xbox_output_uses_xpad_codes_and_full_range_sticks() {
        let mut state = PadState {
            lx: -1.0,
            rt: 1.0,
            ..PadState::default()
        };
        state.set(Button::West, true);
        state.set(Button::DpadRight, true);
        let values = output_values(OutputKind::Xbox360, &state);

        assert!(
            values.contains(&(EV_KEY, key::X, 1)),
            "Xbox X (left) is BTN_X"
        );
        assert!(values.contains(&(EV_KEY, key::Y, 0)));
        assert!(values.contains(&(EV_ABS, abs::X, -32767)));
        assert!(values.contains(&(EV_ABS, abs::RZ, 255)));
        assert!(values.contains(&(EV_ABS, abs::HAT0X, 1)));
        assert!(
            !values.iter().any(|(_, code, _)| *code == key::TL2),
            "no trigger buttons on a 360 pad"
        );
    }

    #[test]
    fn the_dualshock_output_centres_sticks_at_128_and_has_trigger_buttons() {
        let mut state = PadState::default();
        state.set(Button::North, true);
        state.set(Button::L2, true);
        let values = output_values(OutputKind::Ds4, &state);

        assert!(values.contains(&(EV_ABS, abs::X, 128)));
        assert!(
            values.contains(&(EV_KEY, key::X, 1)),
            "Triangle (top) is BTN_NORTH"
        );
        assert!(values.contains(&(EV_KEY, key::TL2, 1)));
    }

    #[test]
    fn only_changes_are_written() {
        let before = PadState::default();
        let mut after = before;
        after.set(Button::South, true);
        assert_eq!(
            diff(OutputKind::Xbox360, &before, &after),
            vec![(EV_KEY, key::SOUTH, 1)]
        );
        assert!(diff(OutputKind::Xbox360, &after, &after).is_empty());
    }
}
