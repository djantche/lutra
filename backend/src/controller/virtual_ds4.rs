//! A DualShock 4 made of a real HID device (`/dev/uhid`), not just an input device.
//!
//! Games that show PlayStation buttons find the DualShock through its HID interface: Proton and
//! SDL read Sony controllers from `/dev/hidraw`, which a `uinput` pad does not have, so such a
//! pad only reaches the game as a generic XInput controller with Xbox buttons. Through `uhid`
//! the kernel's own `hid-playstation` driver takes the device, exactly as for a DualShock 4 on
//! a cable: a `hidraw` node, a gamepad, a touchpad and motion sensors.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::time::{Duration, Instant};

use super::model::{Button, PadState};

const UHID_PATH: &str = "/dev/uhid";

// Event types of `linux/uhid.h`.
const UHID_DESTROY: u32 = 1;
const UHID_OUTPUT: u32 = 6;
const UHID_GET_REPORT: u32 = 9;
const UHID_GET_REPORT_REPLY: u32 = 10;
const UHID_CREATE2: u32 = 11;
const UHID_INPUT2: u32 = 12;
const UHID_SET_REPORT: u32 = 13;
const UHID_SET_REPORT_REPLY: u32 = 14;
/// `sizeof(struct uhid_event)`: the type and the largest request (`uhid_create2_req`).
const EVENT_SIZE: usize = 4 + 128 + 64 + 64 + 2 + 2 + 4 * 4 + 4096;
const BUS_USB: u16 = 0x03;
const EIO: u16 = 5;

/// The `phys` of Lutra's HID controllers, so Lutra never reads them back as input.
pub const PHYS_PREFIX: &str = "lutra-virtual";

/// A real DualShock sends a report every 4 ms; some readers think a silent one is gone.
const REPORT_INTERVAL: Duration = Duration::from_millis(8);

/// Feature reports the device answers, with their size without the report id. The kernel
/// driver asks for 0x02 (calibration), 0x12 (pairing, holding the address) and 0xa3 (firmware).
const FEATURES: &[(u8, u8)] = &[
    (0x02, 36),
    (0x04, 36),
    (0x08, 3),
    (0x10, 4),
    (0x11, 2),
    (0x12, 15),
    (0x13, 22),
    (0x14, 16),
    (0x15, 44),
    (0x80, 6),
    (0x81, 6),
    (0x82, 5),
    (0xa3, 48),
];

/// The address a virtual controller reports in its pairing information. The kernel's
/// `hid-playstation` driver keeps every DualShock 4 by its address and refuses a second one with
/// the same address (`Duplicate device found for MAC address`, error -17): the controller then
/// has a `hidraw` node but no gamepad, and the game gets nothing from it. With two physical
/// controllers there are two virtual ones, so each needs its own, and which of them lost used to
/// depend on which was created first: a controller that worked one launch was dead the next.
///
/// Locally administered (the 0x02 bit), so it never is a real one. It holds the process id and the
/// number of the virtual controller: unique among those alive, and never the same two launches
/// in a row for devices that the kernel has not finished removing.
fn address(index: usize) -> [u8; 6] {
    let process = std::process::id();
    [
        0x02,
        (process >> 8) as u8,
        process as u8,
        (index >> 16) as u8,
        (index >> 8) as u8,
        index as u8,
    ]
}

/// The HID report descriptor of a DualShock 4 on USB: report 0x01 (input, 64 bytes), 0x05
/// (output: rumble and light bar) and the feature reports above.
fn descriptor() -> Vec<u8> {
    let mut rd = vec![
        0x05, 0x01, // Usage Page (Generic Desktop)
        0x09, 0x05, // Usage (Game Pad)
        0xa1, 0x01, // Collection (Application)
        0x85, 0x01, //   Report ID (1)
        0x09, 0x30, 0x09, 0x31, 0x09, 0x32, 0x09, 0x35, // Usage (X, Y, Z, Rz)
        0x15, 0x00, 0x26, 0xff, 0x00, // Logical 0..255
        0x75, 0x08, 0x95, 0x04, 0x81, 0x02, // 4 x 8 bits, Input
        0x09, 0x39, // Usage (Hat switch)
        0x15, 0x00, 0x25, 0x07, 0x35, 0x00, 0x46, 0x3b, 0x01, 0x65,
        0x14, // 0..7, 0..315 degrees
        0x75, 0x04, 0x95, 0x01, 0x81, 0x42, // 4 bits, Input (Null state)
        0x65, 0x00, // Unit (None)
        0x05, 0x09, 0x19, 0x01, 0x29, 0x0e, // Usage Page (Button), 1..14
        0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x0e, 0x81, 0x02, // 14 x 1 bit, Input
        0x06, 0x00, 0xff, 0x09, 0x20, // Usage Page (Vendor), Usage (0x20): counter
        0x75, 0x06, 0x95, 0x01, 0x15, 0x00, 0x25, 0x7f, 0x81, 0x02, // 6 bits, Input
        0x05, 0x01, 0x09, 0x33, 0x09, 0x34, // Usage (Rx, Ry): triggers
        0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x02, 0x81, 0x02, // 2 x 8 bits, Input
        0x06, 0x00, 0xff, 0x09, 0x21, 0x95, 0x36, 0x81, 0x02, // Vendor, 54 bytes, Input
        0x85, 0x05, 0x09, 0x22, 0x95, 0x1f, 0x91, 0x02, // Report ID 5, 31 bytes, Output
    ];
    for (id, size) in FEATURES {
        rd.extend_from_slice(&[0x85, *id, 0x09, 0x23, 0x95, *size, 0xb1, 0x02]);
    }
    rd.push(0xc0); // End Collection
    rd
}

/// The bytes of a feature report, report id first.
fn feature(id: u8, address: &[u8; 6]) -> Option<Vec<u8>> {
    let size = FEATURES.iter().find(|(known, _)| *known == id)?.1;
    let mut report = vec![0u8; size as usize + 1];
    report[0] = id;
    match id {
        // Calibration (USB layout): gyro biases, then plus and minus ranges, speed, accelerometer.
        0x02 => {
            let values: [i16; 17] = [
                0, 0, 0, // biases
                8800, 8800, 8800, -8800, -8800, -8800, // gyro plus, then minus
                540, 540, // gyro speed
                8192, -8192, 8192, -8192, 8192, -8192, // accelerometer x, y, z
            ];
            for (index, value) in values.iter().enumerate() {
                report[1 + index * 2..3 + index * 2].copy_from_slice(&value.to_le_bytes());
            }
        }
        // Pairing information: the controller's address, least significant byte first.
        0x12 | 0x81 => {
            for (index, byte) in address.iter().rev().enumerate() {
                report[1 + index] = *byte;
            }
        }
        _ => {}
    }
    Some(report)
}

/// Report 0x01 for a state, as a DualShock 4 on USB sends it.
fn input_report(state: &PadState, counter: u8, timestamp: u16) -> [u8; 64] {
    let axis = |value: f32| ((value.clamp(-1.0, 1.0) + 1.0) / 2.0 * 255.0).round() as u8;
    let trigger = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let pressed = |button| state.pressed(button);

    let hat = match (
        pressed(Button::DpadUp),
        pressed(Button::DpadRight),
        pressed(Button::DpadDown),
        pressed(Button::DpadLeft),
    ) {
        (true, false, _, true) => 7,
        (true, true, _, _) => 1,
        (true, false, _, false) => 0,
        (false, true, true, _) => 3,
        (false, true, false, _) => 2,
        (false, false, true, true) => 5,
        (false, false, true, false) => 4,
        (false, false, false, true) => 6,
        _ => 8,
    };
    let bit = |button, mask: u8| if pressed(button) { mask } else { 0 };

    let mut report = [0u8; 64];
    report[0] = 0x01;
    report[1] = axis(state.lx);
    report[2] = axis(state.ly);
    report[3] = axis(state.rx);
    report[4] = axis(state.ry);
    report[5] = hat
        | bit(Button::West, 0x10)
        | bit(Button::South, 0x20)
        | bit(Button::East, 0x40)
        | bit(Button::North, 0x80);
    report[6] = bit(Button::L1, 0x01)
        | bit(Button::R1, 0x02)
        | bit(Button::L2, 0x04)
        | bit(Button::R2, 0x08)
        | bit(Button::Select, 0x10)
        | bit(Button::Start, 0x20)
        | bit(Button::L3, 0x40)
        | bit(Button::R3, 0x80);
    report[7] = bit(Button::Guide, 0x01) | (counter << 2);
    report[8] = trigger(state.lt);
    report[9] = trigger(state.rt);
    report[10..12].copy_from_slice(&timestamp.to_le_bytes());
    // Lying flat and still: gravity on the accelerometer's Y axis.
    report[21..23].copy_from_slice(&8192i16.to_le_bytes());
    // On a cable, battery full.
    report[30] = 0x1b;
    // No finger on the touchpad (bit 7 set: not touching).
    report[35] = 0x80;
    report[39] = 0x80;
    report
}

/// Whether this user may create HID devices (needs an udev rule on `/dev/uhid`).
pub fn available() -> bool {
    OpenOptions::new().write(true).open(UHID_PATH).is_ok()
}

pub struct VirtualDs4 {
    file: File,
    address: [u8; 6],
    state: PadState,
    counter: u8,
    started: Instant,
    last_report: Instant,
}

/// The rumble the game asked for, from 0 to 255 per motor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rumble {
    pub strong: u8,
    pub weak: u8,
}

impl VirtualDs4 {
    /// Creates the controller with the ids `(vendor, product)`; `index` tells apart the virtual
    /// controllers of several physical ones.
    pub fn create((vendor, product): (u16, u16), index: usize) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(UHID_PATH)?;

        let rd = descriptor();
        let mut event = vec![0u8; EVENT_SIZE];
        event[0..4].copy_from_slice(&UHID_CREATE2.to_ne_bytes());
        let name = b"Sony Interactive Entertainment Wireless Controller";
        event[4..4 + name.len()].copy_from_slice(name);
        let phys = format!("{PHYS_PREFIX}/ds4-{index}");
        event[132..132 + phys.len()].copy_from_slice(phys.as_bytes());
        event[260..262].copy_from_slice(&(rd.len() as u16).to_ne_bytes());
        event[262..264].copy_from_slice(&BUS_USB.to_ne_bytes());
        event[264..268].copy_from_slice(&u32::from(vendor).to_ne_bytes());
        event[268..272].copy_from_slice(&u32::from(product).to_ne_bytes());
        event[272..276].copy_from_slice(&0x0100u32.to_ne_bytes());
        event[280..280 + rd.len()].copy_from_slice(&rd);
        file.write_all(&event)?;

        let now = Instant::now();
        let mut pad = Self {
            file,
            address: address(index),
            state: PadState::default(),
            counter: 0,
            started: now,
            last_report: now,
        };
        pad.report()?;
        Ok(pad)
    }

    pub fn fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    /// Sends `state` to the game.
    pub fn send(&mut self, state: &PadState) {
        self.state = *state;
        let _ = self.report();
    }

    /// Keeps reports flowing while nothing changes, as a real controller does.
    pub fn tick(&mut self, now: Instant) {
        if now.duration_since(self.last_report) >= REPORT_INTERVAL {
            let _ = self.report();
        }
    }

    fn report(&mut self) -> io::Result<()> {
        self.counter = (self.counter + 1) & 0x3f;
        // The timestamp counts in units of 5.33 microseconds.
        let timestamp = (self.started.elapsed().as_micros() * 3 / 16) as u16;
        let report = input_report(&self.state, self.counter, timestamp);
        let mut event = [0u8; 6 + 64];
        event[0..4].copy_from_slice(&UHID_INPUT2.to_ne_bytes());
        event[4..6].copy_from_slice(&(report.len() as u16).to_ne_bytes());
        event[6..].copy_from_slice(&report);
        self.last_report = Instant::now();
        self.file.write_all(&event)
    }

    /// Answers what the kernel and the game ask of the device. Returns the rumble the game set,
    /// if it changed.
    pub fn process(&mut self) -> Option<Rumble> {
        let mut rumble = None;
        let mut event = vec![0u8; EVENT_SIZE];
        while let Ok(read) = self.file.read(&mut event) {
            if read < 4 {
                break;
            }
            let kind = u32::from_ne_bytes(event[0..4].try_into().unwrap_or_default());
            match kind {
                UHID_GET_REPORT => {
                    let id = &event[4..8];
                    let reply = feature(event[8], &self.address);
                    let mut answer = vec![0u8; 12 + 4096];
                    answer[0..4].copy_from_slice(&UHID_GET_REPORT_REPLY.to_ne_bytes());
                    answer[4..8].copy_from_slice(id);
                    match reply {
                        Some(data) => {
                            answer[10..12].copy_from_slice(&(data.len() as u16).to_ne_bytes());
                            answer[12..12 + data.len()].copy_from_slice(&data);
                        }
                        None => answer[8..10].copy_from_slice(&EIO.to_ne_bytes()),
                    }
                    let _ = self.file.write_all(&answer);
                }
                UHID_SET_REPORT => {
                    let mut answer = [0u8; 10];
                    answer[0..4].copy_from_slice(&UHID_SET_REPORT_REPLY.to_ne_bytes());
                    answer[4..8].copy_from_slice(&event[4..8]);
                    let _ = self.file.write_all(&answer);
                }
                UHID_OUTPUT => {
                    let size = u16::from_ne_bytes([event[4100], event[4101]]) as usize;
                    let data = &event[4..4 + size.min(4096)];
                    // Report 0x05: flags, then the light (weak) and heavy (strong) motors.
                    if data.len() >= 6 && data[0] == 0x05 && data[1] & 0x01 != 0 {
                        rumble = Some(Rumble {
                            weak: data[4],
                            strong: data[5],
                        });
                    }
                }
                _ => {}
            }
        }
        rumble
    }
}

impl Drop for VirtualDs4 {
    fn drop(&mut self) {
        let mut event = [0u8; 4];
        event.copy_from_slice(&UHID_DESTROY.to_ne_bytes());
        let _ = self.file.write_all(&event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_becomes_a_dualshock_report() {
        let mut state = PadState {
            lx: -1.0,
            ry: 1.0,
            rt: 1.0,
            ..PadState::default()
        };
        state.set(Button::South, true);
        state.set(Button::Start, true);
        state.set(Button::DpadUp, true);
        state.set(Button::DpadRight, true);
        let report = input_report(&state, 5, 0);
        assert_eq!(report[0], 0x01);
        assert_eq!((report[1], report[2], report[4]), (0, 128, 255));
        // Hat north-east (1) and cross.
        assert_eq!(report[5], 0x01 | 0x20);
        assert_eq!(report[6], 0x20);
        assert_eq!(report[7] >> 2, 5);
        assert_eq!(report[9], 255);
        assert_eq!(input_report(&PadState::default(), 0, 0)[5] & 0x0f, 8);
    }

    #[test]
    fn the_kernel_driver_gets_the_feature_reports_it_needs() {
        // hid-playstation reads these on USB and checks their exact size.
        let address = address(7);
        for (id, size) in [(0x02u8, 37usize), (0x12, 16), (0xa3, 49)] {
            let report = feature(id, &address).unwrap();
            assert_eq!(report.len(), size);
            assert_eq!(report[0], id);
        }
        // The address comes least significant byte first, like on a real controller.
        let reversed: Vec<u8> = address.iter().rev().copied().collect();
        assert_eq!(&feature(0x12, &address).unwrap()[1..7], &reversed[..]);
        assert!(feature(0x42, &address).is_none());
    }

    #[test]
    fn every_virtual_controller_has_its_own_address() {
        let addresses: Vec<[u8; 6]> = (0..70_000).map(address).collect();
        let distinct: std::collections::HashSet<_> = addresses.iter().collect();
        assert_eq!(distinct.len(), addresses.len());
        // Locally administered and unicast: never a real controller's.
        assert!(addresses.iter().all(|a| a[0] & 0x03 == 0x02));
        // Two controllers made one after the other share nothing the kernel compares.
        assert_ne!(address(0), address(1));
    }

    #[test]
    fn the_descriptor_declares_every_feature_report() {
        let rd = descriptor();
        for (id, size) in FEATURES {
            assert!(
                rd.windows(6)
                    .any(|item| item == [0x85, *id, 0x09, 0x23, 0x95, *size]),
                "feature {id:#x}"
            );
        }
        assert_eq!(rd.last(), Some(&0xc0));
    }
}
