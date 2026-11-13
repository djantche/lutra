//! How a console's games look and play: the picture, the controls, and the emulator's own
//! options. Set per console, and per game when the user keeps settings for one game.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::controller::model::Button;

/// How the console's pixels are drawn on a modern screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Scaling {
    /// Square pixels, crisp at any size, without shimmering.
    #[default]
    Sharp,
    /// Smoothed, as a television would.
    Smooth,
    /// Only whole multiples of the console's picture: perfectly even pixels, with borders.
    Integer,
}

/// The shape of the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Aspect {
    /// The console's own (4:3 for a television console, 3:2 for a Game Boy Advance).
    #[default]
    Console,
    /// Each pixel square, whatever the console's screen looked like.
    Square,
    /// The whole screen, stretched.
    Stretch,
}

/// A look of old screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Effect {
    #[default]
    None,
    /// The dark lines between a tube television's rows.
    Scanlines,
    /// The grid of a handheld's liquid-crystal screen.
    Lcd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct VideoSettings {
    pub scaling: Scaling,
    pub aspect: Aspect,
    pub effect: Effect,
}

/// A button of the console's controller, by libretro's names (a Super Nintendo pad: B at the
/// bottom, A on the right, X at the top, Y on the left).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RetroButton {
    B,
    Y,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
    A,
    X,
    L,
    R,
    L2,
    R2,
    L3,
    R3,
}

impl RetroButton {
    pub const ALL: [RetroButton; 16] = [
        RetroButton::B,
        RetroButton::Y,
        RetroButton::Select,
        RetroButton::Start,
        RetroButton::Up,
        RetroButton::Down,
        RetroButton::Left,
        RetroButton::Right,
        RetroButton::A,
        RetroButton::X,
        RetroButton::L,
        RetroButton::R,
        RetroButton::L2,
        RetroButton::R2,
        RetroButton::L3,
        RetroButton::R3,
    ];

    /// Its bit in libretro's joypad mask (`RETRO_DEVICE_ID_JOYPAD_*`).
    pub fn bit(self) -> u16 {
        1 << (self as u16)
    }

    /// The controller button that presses it unless the user changed it: the same position.
    pub fn default_button(self) -> Button {
        match self {
            RetroButton::B => Button::South,
            RetroButton::A => Button::East,
            RetroButton::Y => Button::West,
            RetroButton::X => Button::North,
            RetroButton::Select => Button::Select,
            RetroButton::Start => Button::Start,
            RetroButton::Up => Button::DpadUp,
            RetroButton::Down => Button::DpadDown,
            RetroButton::Left => Button::DpadLeft,
            RetroButton::Right => Button::DpadRight,
            RetroButton::L => Button::L1,
            RetroButton::R => Button::R1,
            RetroButton::L2 => Button::L2,
            RetroButton::R2 => Button::R2,
            RetroButton::L3 => Button::L3,
            RetroButton::R3 => Button::R3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ControlSettings {
    /// The controller button that presses each console button, when not the same position.
    pub remap: BTreeMap<RetroButton, Button>,
    /// The left stick also works as the directional pad (consoles that had no stick).
    pub stick_as_dpad: bool,
}

impl ControlSettings {
    /// The controller button that presses `button`.
    pub fn button_for(&self, button: RetroButton) -> Button {
        self.remap
            .get(&button)
            .copied()
            .unwrap_or_else(|| button.default_button())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct EmulationProfile {
    pub video: VideoSettings,
    pub controls: ControlSettings,
    /// The emulator's options the user changed, by key; the others keep the core's defaults.
    pub core_options: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_keep_their_position_until_remapped() {
        let mut controls = ControlSettings::default();
        assert_eq!(controls.button_for(RetroButton::A), Button::East);
        controls.remap.insert(RetroButton::A, Button::South);
        assert_eq!(controls.button_for(RetroButton::A), Button::South);
        assert_eq!(RetroButton::A.bit(), 1 << 8);
    }

    #[test]
    fn a_profile_tolerates_missing_and_unknown_fields() {
        let profile: EmulationProfile =
            serde_json::from_str(r#"{"video":{"effect":"scanlines"},"later":1}"#).unwrap();
        assert_eq!(profile.video.effect, Effect::Scanlines);
        assert_eq!(profile.video.scaling, Scaling::Sharp);
        assert!(profile.core_options.is_empty());
    }
}
