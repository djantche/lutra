//! Lutra's virtual keyboard and mouse: one `uinput` device that carries the keys and pointer
//! movement a profile converts the controller into.

use std::io;

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, BusType, InputEvent, InputId, KeyCode, RelativeAxisCode};

use super::convert;

pub struct VirtualInput {
    pub device: VirtualDevice,
}

pub fn create() -> io::Result<VirtualInput> {
    let keys: AttributeSet<KeyCode> = convert::key_codes().into_iter().map(KeyCode::new).collect();
    let axes: AttributeSet<RelativeAxisCode> = [
        RelativeAxisCode::REL_X,
        RelativeAxisCode::REL_Y,
        RelativeAxisCode::REL_WHEEL,
    ]
    .into_iter()
    .collect();
    let device = VirtualDevice::builder()?
        .name("Lutra Keyboard and Mouse")
        .input_id(InputId::new(BusType::BUS_VIRTUAL, 0x1209, 0x1a7a, 1))
        .with_keys(&keys)?
        .with_relative_axes(&axes)?
        .build()?;
    Ok(VirtualInput { device })
}

impl VirtualInput {
    pub fn emit(&mut self, events: &[(u16, u16, i32)]) {
        let events: Vec<InputEvent> = events
            .iter()
            .map(|(kind, code, value)| InputEvent::new(*kind, *code, *value))
            .collect();
        let _ = self.device.emit(&events);
    }
}
