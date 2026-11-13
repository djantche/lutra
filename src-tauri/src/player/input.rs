//! What the players press: controllers read from `/dev/input`, and the keyboard.
//!
//! During a game Lutra's controller driver holds the physical controllers and presents a
//! virtual one shaped by the game's profile; that virtual controller is what is read here. A
//! controller another program holds exclusively is skipped, so each player gets the right port.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use evdev::{Device, FFEffect, FFEffectData, FFEffectKind, FFReplay, FFTrigger};
use winit::keyboard::KeyCode;

use super::libretro::*;
use crate::controller::codes::{Decoder, Family, Range};
use crate::controller::device;
use crate::controller::model::PadState;
use crate::emulation::profile::{ControlSettings, RetroButton};

/// How often new or unplugged controllers are noticed.
const RESCAN: Duration = Duration::from_secs(2);

struct Pad {
    path: PathBuf,
    device: Device,
    decoder: Decoder,
    state: PadState,
    rumble: Option<FFEffect>,
    rumbling: (u16, u16),
}

pub struct Input {
    controls: ControlSettings,
    pads: Vec<Pad>,
    /// Input devices that are not free controllers (keyboards, held pads), not opened again.
    ignored: Vec<PathBuf>,
    last_scan: Option<Instant>,
    /// Keyboard keys held down, which play as the first controller.
    keys: Vec<KeyCode>,
}

fn open(path: PathBuf) -> Option<Pad> {
    let mut device = Device::open(&path).ok()?;
    if !device::is_pad(&device) {
        return None;
    }
    // Held by someone else (Lutra's driver, behind its virtual controller): not ours to read.
    if device.grab().is_err() {
        return None;
    }
    let _ = device.ungrab();
    let _ = device.set_nonblocking(true);
    let ranges = device.get_abs_state().ok();
    let mut decoder = Decoder::new(Family::from_vendor(device.input_id().vendor()), |axis| {
        let range = ranges.as_ref()?.get(axis as usize)?;
        (range.maximum > range.minimum).then_some(Range {
            min: range.minimum,
            max: range.maximum,
        })
    });
    if device.input_id().bus_type() != evdev::BusType::BUS_USB {
        decoder = decoder.bluetooth();
    }
    decoder.calibrate(|axis| Some(ranges.as_ref()?.get(axis as usize)?.value));
    Some(Pad {
        path,
        device,
        decoder,
        state: PadState::default(),
        rumble: None,
        rumbling: (0, 0),
    })
}

fn axis(value: f32) -> i16 {
    (value.clamp(-1.0, 1.0) * 32767.0) as i16
}

/// A stick this far over also presses the directional pad, when the user asks for it.
const STICK_AS_DPAD: f32 = 0.5;

/// A controller's state as the console's buttons: by position, or as the user remapped them.
pub fn pad_input(state: &PadState, controls: &ControlSettings) -> PadInput {
    let mut buttons = RetroButton::ALL
        .iter()
        .filter(|button| state.pressed(controls.button_for(**button)))
        .fold(0, |bits, button| bits | button.bit());
    if controls.stick_as_dpad {
        for (pushed, bit) in [
            (state.ly < -STICK_AS_DPAD, JOYPAD_UP),
            (state.ly > STICK_AS_DPAD, JOYPAD_DOWN),
            (state.lx < -STICK_AS_DPAD, JOYPAD_LEFT),
            (state.lx > STICK_AS_DPAD, JOYPAD_RIGHT),
        ] {
            if pushed {
                buttons |= bit;
            }
        }
    }
    PadInput {
        buttons,
        lx: axis(state.lx),
        ly: axis(state.ly),
        rx: axis(state.rx),
        ry: axis(state.ry),
        l2: (state.lt.clamp(0.0, 1.0) * 32767.0) as i16,
        r2: (state.rt.clamp(0.0, 1.0) * 32767.0) as i16,
    }
}

/// The keyboard's buttons: arrows, X/Z for A/B, S/A for X/Y, Q/W for the shoulders,
/// Enter and right Shift for Start and Select.
fn key_bit(key: KeyCode) -> u16 {
    match key {
        KeyCode::ArrowUp => JOYPAD_UP,
        KeyCode::ArrowDown => JOYPAD_DOWN,
        KeyCode::ArrowLeft => JOYPAD_LEFT,
        KeyCode::ArrowRight => JOYPAD_RIGHT,
        KeyCode::KeyX => JOYPAD_A,
        KeyCode::KeyZ => JOYPAD_B,
        KeyCode::KeyS => JOYPAD_X,
        KeyCode::KeyA => JOYPAD_Y,
        KeyCode::KeyQ => JOYPAD_L,
        KeyCode::KeyW => JOYPAD_R,
        KeyCode::Enter => JOYPAD_START,
        KeyCode::ShiftRight => JOYPAD_SELECT,
        _ => 0,
    }
}

impl Input {
    pub fn new(controls: ControlSettings) -> Self {
        Self {
            controls,
            pads: Vec::new(),
            ignored: Vec::new(),
            last_scan: None,
            keys: Vec::new(),
        }
    }

    pub fn key(&mut self, key: KeyCode, down: bool) {
        self.keys.retain(|held| *held != key);
        if down {
            self.keys.push(key);
        }
    }

    fn rescan(&mut self) {
        // Unplugged controllers stop answering; one Lutra's driver took since (it hides it
        // behind a virtual controller) has nothing more to say.
        let mut taken = Vec::new();
        self.pads.retain_mut(|pad| {
            let free = pad.device.grab().is_ok();
            if free {
                let _ = pad.device.ungrab();
            } else {
                taken.push(pad.path.clone());
            }
            free && pad.path.exists()
        });
        self.ignored.extend(taken);
        let mut found: Vec<PathBuf> = std::fs::read_dir("/dev/input")
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.file_name()
                            .is_some_and(|name| name.to_string_lossy().starts_with("event"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        found.sort();
        // A node number freed by an unplugged device can come back as a controller.
        self.ignored.retain(|path| found.contains(path));
        for path in found {
            if self.pads.iter().any(|pad| pad.path == path) || self.ignored.contains(&path) {
                continue;
            }
            match open(path.clone()) {
                Some(pad) => self.pads.push(pad),
                None => self.ignored.push(path),
            }
        }
    }

    /// Reads every controller and returns what the four ports hold.
    pub fn poll(&mut self, now: Instant) -> [PadInput; 4] {
        if self
            .last_scan
            .is_none_or(|last| now.duration_since(last) >= RESCAN)
        {
            self.last_scan = Some(now);
            self.rescan();
        }
        let mut gone = Vec::new();
        for pad in &mut self.pads {
            match pad.device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        if pad
                            .decoder
                            .feed(event.event_type().0, event.code(), event.value())
                        {
                            pad.state = pad.decoder.state();
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => gone.push(pad.path.clone()),
            }
        }
        self.pads.retain(|pad| !gone.contains(&pad.path));

        let mut ports = [PadInput::default(); 4];
        for (port, pad) in self.pads.iter().take(4).enumerate() {
            ports[port] = pad_input(&pad.state, &self.controls);
        }
        ports[0].buttons |= self.keys.iter().fold(0, |bits, key| bits | key_bit(*key));
        ports
    }

    /// Plays the rumble the core asked for on each port's controller.
    pub fn rumble(&mut self, wanted: &[(u16, u16); 4]) {
        for (pad, &(strong, weak)) in self.pads.iter_mut().zip(wanted) {
            if pad.rumbling == (strong, weak) {
                continue;
            }
            pad.rumbling = (strong, weak);
            let data = FFEffectData {
                direction: 0,
                trigger: FFTrigger::default(),
                replay: FFReplay {
                    length: 0,
                    delay: 0,
                },
                kind: FFEffectKind::Rumble {
                    strong_magnitude: strong,
                    weak_magnitude: weak,
                },
            };
            if pad.rumble.is_none() {
                pad.rumble = pad.device.upload_ff_effect(data).ok();
            }
            if let Some(effect) = pad.rumble.as_mut() {
                let _ = effect.update(data);
                let _ = if strong == 0 && weak == 0 {
                    effect.stop()
                } else {
                    effect.play(1)
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::model::Button;

    #[test]
    fn buttons_map_by_position_and_sticks_to_the_full_range() {
        let mut state = PadState::default();
        state.lx = -1.0;
        state.rt = 1.0;
        state.set(Button::South, true);
        state.set(Button::DpadLeft, true);
        let input = pad_input(&state, &ControlSettings::default());
        assert_eq!(input.buttons, JOYPAD_B | JOYPAD_LEFT);
        assert_eq!((input.lx, input.r2), (-32767, 32767));

        // Remapped: the bottom button is now A; the stick also steers.
        let mut controls = ControlSettings {
            stick_as_dpad: true,
            ..ControlSettings::default()
        };
        controls.remap.insert(RetroButton::A, Button::South);
        controls.remap.insert(RetroButton::B, Button::East);
        let mut state = PadState::default();
        state.set(Button::South, true);
        state.lx = 0.9;
        assert_eq!(
            pad_input(&state, &controls).buttons,
            JOYPAD_A | JOYPAD_RIGHT
        );
    }
}
