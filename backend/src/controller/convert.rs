//! Turning controller input into keyboard and mouse input, for games that have no gamepad
//! support (or whose gamepad support is poor): a button becomes a key, a stick becomes the
//! pointer or the direction keys. The profile decides (`model::convert`); this module only
//! knows how a key name becomes an `evdev` event, and how pointer movement is paced.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use evdev::{KeyCode, RelativeAxisCode};
use serde::Serialize;

use super::model::Routed;

const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;

/// A pointer step is never computed over more than this, so a stall does not throw the cursor.
const MAX_STEP: Duration = Duration::from_millis(50);

/// What a name in a profile does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Key(u16),
    /// One notch of the wheel when pressed (+1 up, -1 down).
    Wheel(i32),
}

/// A key a button can be bound to, as the interface lists it.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct KeyInfo {
    pub name: &'static str,
    pub group: &'static str,
}

macro_rules! keys {
    ($group:literal: $($name:literal => $code:ident),+ $(,)?) => {
        &[$(($name, $group, Target::Key(KeyCode::$code.0)),)+]
    };
}

const LETTERS: &[(&str, &str, Target)] = keys!("letters":
    "a" => KEY_A, "b" => KEY_B, "c" => KEY_C, "d" => KEY_D, "e" => KEY_E, "f" => KEY_F,
    "g" => KEY_G, "h" => KEY_H, "i" => KEY_I, "j" => KEY_J, "k" => KEY_K, "l" => KEY_L,
    "m" => KEY_M, "n" => KEY_N, "o" => KEY_O, "p" => KEY_P, "q" => KEY_Q, "r" => KEY_R,
    "s" => KEY_S, "t" => KEY_T, "u" => KEY_U, "v" => KEY_V, "w" => KEY_W, "x" => KEY_X,
    "y" => KEY_Y, "z" => KEY_Z,
);

const DIGITS: &[(&str, &str, Target)] = keys!("digits":
    "0" => KEY_0, "1" => KEY_1, "2" => KEY_2, "3" => KEY_3, "4" => KEY_4,
    "5" => KEY_5, "6" => KEY_6, "7" => KEY_7, "8" => KEY_8, "9" => KEY_9,
);

const CONTROL: &[(&str, &str, Target)] = keys!("control":
    "enter" => KEY_ENTER, "escape" => KEY_ESC, "space" => KEY_SPACE, "tab" => KEY_TAB,
    "backspace" => KEY_BACKSPACE, "shift" => KEY_LEFTSHIFT, "ctrl" => KEY_LEFTCTRL,
    "alt" => KEY_LEFTALT, "up" => KEY_UP, "down" => KEY_DOWN, "left" => KEY_LEFT,
    "right" => KEY_RIGHT, "home" => KEY_HOME, "end" => KEY_END, "pageup" => KEY_PAGEUP,
    "pagedown" => KEY_PAGEDOWN, "insert" => KEY_INSERT, "delete" => KEY_DELETE,
);

const FUNCTION: &[(&str, &str, Target)] = keys!("function":
    "f1" => KEY_F1, "f2" => KEY_F2, "f3" => KEY_F3, "f4" => KEY_F4, "f5" => KEY_F5,
    "f6" => KEY_F6, "f7" => KEY_F7, "f8" => KEY_F8, "f9" => KEY_F9, "f10" => KEY_F10,
    "f11" => KEY_F11, "f12" => KEY_F12,
);

const MOUSE: &[(&str, &str, Target)] = &[
    ("mouse_left", "mouse", Target::Key(KeyCode::BTN_LEFT.0)),
    ("mouse_right", "mouse", Target::Key(KeyCode::BTN_RIGHT.0)),
    ("mouse_middle", "mouse", Target::Key(KeyCode::BTN_MIDDLE.0)),
    ("wheel_up", "mouse", Target::Wheel(1)),
    ("wheel_down", "mouse", Target::Wheel(-1)),
];

const TABLES: [&[(&str, &str, Target)]; 5] = [LETTERS, DIGITS, CONTROL, FUNCTION, MOUSE];

/// Looks a profile's key name up.
pub fn target(name: &str) -> Option<Target> {
    TABLES
        .iter()
        .flat_map(|table| table.iter())
        .find(|(known, _, _)| *known == name)
        .map(|(_, _, target)| *target)
}

/// Every key a button can be bound to, for the interface.
pub fn key_list() -> Vec<KeyInfo> {
    TABLES
        .iter()
        .flat_map(|table| table.iter())
        .map(|(name, group, _)| KeyInfo { name, group })
        .collect()
}

/// Every evdev key code the virtual keyboard must declare.
pub fn key_codes() -> Vec<u16> {
    TABLES
        .iter()
        .flat_map(|table| table.iter())
        .filter_map(|(_, _, target)| match target {
            Target::Key(code) => Some(*code),
            Target::Wheel(_) => None,
        })
        .collect()
}

/// Tracks what is held on the virtual keyboard and mouse, and writes only the differences.
pub struct Converter {
    held: BTreeSet<String>,
    /// Pointer movement smaller than a pixel, carried to the next step.
    carry: (f32, f32),
    last: Instant,
}

impl Converter {
    pub fn new(now: Instant) -> Self {
        Self {
            held: BTreeSet::new(),
            carry: (0.0, 0.0),
            last: now,
        }
    }

    /// The `(type, code, value)` events that bring the virtual devices to `routed`.
    /// `speed` is the pointer's pixels per second at full tilt.
    pub fn update(&mut self, routed: &Routed, speed: f32, now: Instant) -> Vec<(u16, u16, i32)> {
        let mut events = Vec::new();

        for name in self.held.difference(&routed.keys) {
            if let Some(Target::Key(code)) = target(name) {
                events.push((EV_KEY, code, 0));
            }
        }
        for name in routed.keys.difference(&self.held) {
            match target(name) {
                Some(Target::Key(code)) => events.push((EV_KEY, code, 1)),
                Some(Target::Wheel(notches)) => {
                    events.push((EV_REL, RelativeAxisCode::REL_WHEEL.0, notches));
                }
                None => {}
            }
        }
        self.held.clone_from(&routed.keys);

        let seconds = now
            .saturating_duration_since(self.last)
            .min(MAX_STEP)
            .as_secs_f32();
        self.last = now;
        if routed.mouse == (0.0, 0.0) {
            self.carry = (0.0, 0.0);
        } else {
            let step = |push: f32, carry: &mut f32| {
                let total = push * speed * seconds + *carry;
                let whole = total.trunc();
                *carry = total - whole;
                whole as i32
            };
            let dx = step(routed.mouse.0, &mut self.carry.0);
            let dy = step(routed.mouse.1, &mut self.carry.1);
            if dx != 0 {
                events.push((EV_REL, RelativeAxisCode::REL_X.0, dx));
            }
            if dy != 0 {
                events.push((EV_REL, RelativeAxisCode::REL_Y.0, dy));
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn routed(keys: &[&str], mouse: (f32, f32)) -> Routed {
        Routed {
            keys: keys.iter().map(|key| (*key).to_owned()).collect(),
            mouse,
        }
    }

    #[test]
    fn every_listed_key_has_a_target_and_a_unique_name() {
        let list = key_list();
        assert!(list.len() > 60);
        let mut names: Vec<&str> = list.iter().map(|key| key.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), list.len());
        assert!(list.iter().all(|key| target(key.name).is_some()));
        assert_eq!(target("enter"), Some(Target::Key(KeyCode::KEY_ENTER.0)));
        assert_eq!(target("nope"), None);
    }

    #[test]
    fn keys_are_pressed_once_and_released_when_the_button_is_let_go() {
        let start = Instant::now();
        let mut converter = Converter::new(start);
        let enter = KeyCode::KEY_ENTER.0;

        assert_eq!(
            converter.update(&routed(&["enter"], (0.0, 0.0)), 900.0, start),
            vec![(EV_KEY, enter, 1)]
        );
        assert!(
            converter
                .update(&routed(&["enter"], (0.0, 0.0)), 900.0, start)
                .is_empty()
        );
        assert_eq!(
            converter.update(&routed(&[], (0.0, 0.0)), 900.0, start),
            vec![(EV_KEY, enter, 0)]
        );
    }

    #[test]
    fn the_wheel_turns_one_notch_per_press() {
        let start = Instant::now();
        let mut converter = Converter::new(start);
        let up = routed(&["wheel_up"], (0.0, 0.0));
        assert_eq!(
            converter.update(&up, 900.0, start),
            vec![(EV_REL, RelativeAxisCode::REL_WHEEL.0, 1)]
        );
        assert!(converter.update(&up, 900.0, start).is_empty());
    }

    #[test]
    fn the_pointer_moves_with_the_time_elapsed_and_keeps_the_fractions() {
        let start = Instant::now();
        let mut converter = Converter::new(start);
        let push = routed(&[], (1.0, 0.0));

        // 900 px/s over 10 ms is 9 px.
        let events = converter.update(&push, 900.0, start + Duration::from_millis(10));
        assert_eq!(events, vec![(EV_REL, RelativeAxisCode::REL_X.0, 9)]);

        // A half push at 100 px/s over 10 ms is half a pixel: nothing yet, then a whole one.
        let half = routed(&[], (0.5, 0.0));
        let mut sent = 0;
        let mut now = start + Duration::from_millis(10);
        for _ in 0..4 {
            now += Duration::from_millis(10);
            sent += converter
                .update(&half, 100.0, now)
                .iter()
                .map(|event| event.2)
                .sum::<i32>();
        }
        assert_eq!(sent, 2);
    }

    #[test]
    fn a_stall_does_not_throw_the_pointer_across_the_screen() {
        let start = Instant::now();
        let mut converter = Converter::new(start);
        let events = converter.update(
            &routed(&[], (1.0, 0.0)),
            1000.0,
            start + Duration::from_secs(5),
        );
        assert_eq!(events, vec![(EV_REL, RelativeAxisCode::REL_X.0, 50)]);
    }
}
