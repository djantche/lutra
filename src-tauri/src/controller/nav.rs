//! Menu navigation with the controller: turns presses into the interface's input actions,
//! with key-repeat when a direction is held.

use std::time::{Duration, Instant};

use super::model::{Button, PadState};

const FIRST_REPEAT: Duration = Duration::from_millis(380);
const REPEAT: Duration = Duration::from_millis(120);
/// How long the Guide button must be held to switch the controller off.
const POWER_OFF_HOLD: Duration = Duration::from_millis(2000);
/// How far the left stick must be pushed to count as a direction.
const STICK_THRESHOLD: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavAction {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    TabPrev,
    TabNext,
    /// The Guide button, released after a short press.
    Menu,
    /// Options / Select: the settings of what is focused.
    Options,
    /// The Guide button, held long enough.
    PowerOff,
    /// The Guide button released briefly during a game: open (or close) the in-game overlay.
    Overlay,
}

impl NavAction {
    /// The name of the matching action in the interface's input layer.
    pub fn as_str(self) -> &'static str {
        match self {
            NavAction::Up => "up",
            NavAction::Down => "down",
            NavAction::Left => "left",
            NavAction::Right => "right",
            NavAction::Confirm => "confirm",
            NavAction::Back => "back",
            NavAction::TabPrev => "tabPrev",
            NavAction::TabNext => "tabNext",
            NavAction::Menu => "menu",
            NavAction::Options => "options",
            NavAction::PowerOff => "powerOff",
            NavAction::Overlay => "overlay",
        }
    }
}

const PRESSES: [(Button, NavAction); 6] = [
    (Button::South, NavAction::Confirm),
    (Button::East, NavAction::Back),
    (Button::L1, NavAction::TabPrev),
    (Button::R1, NavAction::TabNext),
    (Button::Start, NavAction::Options),
    (Button::Select, NavAction::Options),
];

fn direction(state: &PadState) -> Option<NavAction> {
    let dpad = [
        (Button::DpadUp, NavAction::Up),
        (Button::DpadDown, NavAction::Down),
        (Button::DpadLeft, NavAction::Left),
        (Button::DpadRight, NavAction::Right),
    ];
    if let Some((_, action)) = dpad.iter().find(|(button, _)| state.pressed(*button)) {
        return Some(*action);
    }

    let (x, y) = (state.lx, state.ly);
    if x.abs().max(y.abs()) < STICK_THRESHOLD {
        return None;
    }
    Some(if x.abs() > y.abs() {
        if x > 0.0 {
            NavAction::Right
        } else {
            NavAction::Left
        }
    } else if y > 0.0 {
        NavAction::Down
    } else {
        NavAction::Up
    })
}

/// The Guide button, which acts on release so that holding it can mean something else: a short
/// press is the menu (or the overlay in a game), a long hold switches the controller off.
#[derive(Default)]
pub struct GuideWatcher {
    down: bool,
    since: Option<Instant>,
    /// The hold already switched the controller off: releasing must not also act as a press.
    held_out: bool,
}

impl GuideWatcher {
    /// Feeds the button's state; true when a short press has just been released.
    pub fn update(&mut self, pressed: bool, now: Instant) -> bool {
        let released_short = self.down && !pressed && !self.held_out;
        if pressed && !self.down {
            self.since = Some(now);
            self.held_out = false;
        }
        if !pressed {
            self.since = None;
        }
        self.down = pressed;
        released_short
    }

    /// True once, when the button has been held long enough.
    pub fn held_long(&mut self, now: Instant) -> bool {
        match self.since {
            Some(since) if !self.held_out && now.duration_since(since) >= POWER_OFF_HOLD => {
                self.held_out = true;
                true
            }
            _ => false,
        }
    }
}

#[derive(Default)]
pub struct Navigator {
    previous: PadState,
    held: Option<(NavAction, Instant)>,
    guide: GuideWatcher,
}

impl Navigator {
    /// Actions triggered by a new frame: button presses, and a direction when it starts.
    pub fn update(&mut self, state: &PadState, now: Instant) -> Vec<NavAction> {
        let mut actions: Vec<NavAction> = PRESSES
            .iter()
            .filter(|(button, _)| state.pressed(*button) && !self.previous.pressed(*button))
            .map(|(_, action)| *action)
            .collect();

        if self.guide.update(state.pressed(Button::Guide), now) {
            actions.push(NavAction::Menu);
        }

        match (direction(state), self.held) {
            (Some(current), Some((held, _))) if current == held => {}
            (Some(current), _) => {
                actions.push(current);
                self.held = Some((current, now + FIRST_REPEAT));
            }
            (None, _) => self.held = None,
        }
        self.previous = *state;
        actions
    }

    /// The held direction again, once its repeat delay is over.
    pub fn tick(&mut self, now: Instant) -> Option<NavAction> {
        if self.guide.held_long(now) {
            return Some(NavAction::PowerOff);
        }
        let (action, due) = self.held?;
        if now < due {
            return None;
        }
        self.held = Some((action, now + REPEAT));
        Some(action)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod guide_tests {
    use super::*;

    #[test]
    fn a_short_press_is_reported_on_release_and_a_long_one_only_as_a_hold() {
        let mut guide = GuideWatcher::default();
        let start = Instant::now();
        assert!(!guide.update(true, start));
        assert!(!guide.held_long(start + Duration::from_millis(500)));
        assert!(guide.update(false, start + Duration::from_millis(600)));

        // Held long: reported once as a hold, and its release is not a press.
        assert!(!guide.update(true, start + Duration::from_secs(5)));
        assert!(guide.held_long(start + Duration::from_secs(8)));
        assert!(!guide.held_long(start + Duration::from_secs(9)));
        assert!(!guide.update(false, start + Duration::from_secs(10)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pressed(buttons: &[Button]) -> PadState {
        let mut state = PadState::default();
        for button in buttons {
            state.set(*button, true);
        }
        state
    }

    #[test]
    fn presses_fire_once_until_released() {
        let mut nav = Navigator::default();
        let now = Instant::now();
        assert_eq!(
            nav.update(&pressed(&[Button::South]), now),
            vec![NavAction::Confirm]
        );
        assert!(nav.update(&pressed(&[Button::South]), now).is_empty());
        nav.update(&PadState::default(), now);
        assert_eq!(
            nav.update(&pressed(&[Button::South]), now),
            vec![NavAction::Confirm]
        );
    }

    #[test]
    fn the_guide_button_acts_on_release_not_on_press() {
        let mut nav = Navigator::default();
        let start = Instant::now();
        assert!(nav.update(&pressed(&[Button::Guide]), start).is_empty());
        assert_eq!(
            nav.update(&PadState::default(), start + Duration::from_millis(300)),
            vec![NavAction::Menu]
        );
    }

    #[test]
    fn holding_the_guide_button_powers_off_and_does_not_open_the_menu() {
        let mut nav = Navigator::default();
        let start = Instant::now();
        nav.update(&pressed(&[Button::Guide]), start);
        assert_eq!(nav.tick(start + Duration::from_millis(1500)), None);
        assert_eq!(nav.tick(start + POWER_OFF_HOLD), Some(NavAction::PowerOff));
        // Only once, and releasing afterwards does nothing.
        assert_eq!(nav.tick(start + POWER_OFF_HOLD + REPEAT), None);
        assert!(
            nav.update(&PadState::default(), start + Duration::from_secs(3))
                .is_empty()
        );
    }

    #[test]
    fn options_and_select_both_open_the_settings_of_the_focused_item() {
        let mut nav = Navigator::default();
        let now = Instant::now();
        assert_eq!(
            nav.update(&pressed(&[Button::Start]), now),
            vec![NavAction::Options]
        );
        nav.update(&PadState::default(), now);
        assert_eq!(
            nav.update(&pressed(&[Button::Select]), now),
            vec![NavAction::Options]
        );
    }

    #[test]
    fn a_held_direction_repeats_after_a_delay() {
        let mut nav = Navigator::default();
        let start = Instant::now();
        assert_eq!(
            nav.update(&pressed(&[Button::DpadDown]), start),
            vec![NavAction::Down]
        );
        assert_eq!(nav.tick(start + Duration::from_millis(100)), None);
        assert_eq!(nav.tick(start + FIRST_REPEAT), Some(NavAction::Down));
        assert_eq!(
            nav.tick(start + FIRST_REPEAT + Duration::from_millis(50)),
            None
        );
        assert_eq!(
            nav.tick(start + FIRST_REPEAT + REPEAT),
            Some(NavAction::Down)
        );
    }

    #[test]
    fn the_left_stick_navigates_along_its_main_axis() {
        let mut nav = Navigator::default();
        let state = PadState {
            lx: 0.9,
            ly: -0.3,
            ..PadState::default()
        };
        assert_eq!(nav.update(&state, Instant::now()), vec![NavAction::Right]);
        let mut nav = Navigator::default();
        let light = PadState {
            lx: 0.3,
            ..PadState::default()
        };
        assert!(nav.update(&light, Instant::now()).is_empty());
    }

    #[test]
    fn releasing_stops_the_repeat() {
        let mut nav = Navigator::default();
        let start = Instant::now();
        nav.update(&pressed(&[Button::DpadUp]), start);
        nav.update(&PadState::default(), start);
        assert_eq!(nav.tick(start + Duration::from_secs(1)), None);
    }
}
