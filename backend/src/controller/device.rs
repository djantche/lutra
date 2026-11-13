//! Finding physical controllers among `/dev/input` devices.

use std::path::{Path, PathBuf};

use evdev::{AbsoluteAxisCode, BusType, Device, KeyCode};
use serde::Serialize;

use super::codes::{Family, PadModel, abs, key};

/// Steam's virtual gamepads (`28de:11ff`): reading them would echo Steam Input back into Lutra.
const STEAM_VIRTUAL: (u16, u16) = (0x28de, 0x11ff);

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerInfo {
    /// Stable across reconnections when the device has a serial (Bluetooth address, USB port).
    pub id: String,
    pub name: String,
    pub path: String,
    pub vendor: u16,
    pub product: u16,
    pub family: Family,
    /// What the controller looks like (for its picture).
    pub model: PadModel,
    /// `usb`, `bluetooth` or `other`.
    pub bus: &'static str,
}

/// One of Lutra's HID controllers (`virtual_ds4`), from this Lutra or another one. The input
/// devices the kernel makes for them carry no `phys`; their HID parent does.
pub fn is_lutra_hid(path: &Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };
    let uevent = Path::new("/sys/class/input")
        .join(name)
        .join("device/device/uevent");
    std::fs::read_to_string(uevent).is_ok_and(|text| {
        text.lines().any(|line| {
            line.strip_prefix("HID_PHYS=")
                .is_some_and(|phys| phys.starts_with(super::virtual_ds4::PHYS_PREFIX))
        })
    })
}

/// Whether `sysfs_path` (the resolved `/sys/class/input/<node>`) belongs to a `uinput` device.
/// The kernel puts those under `/devices/virtual/input/`, unlike USB or Bluetooth controllers
/// (a Bluetooth DualShock goes through `uhid`, under `/devices/virtual/misc/uhid/`).
fn is_uinput_path(sysfs_path: &Path) -> bool {
    sysfs_path
        .to_string_lossy()
        .contains("/devices/virtual/input/")
}

/// A virtual controller made with `uinput`, by this Lutra or another one (a second instance, or a
/// previous one that is still closing). The registry of `virtual_pad` only knows this process's:
/// without this, two instances take each other's virtual pads for real ones and keep making more.
/// Other programs' `uinput` pads (remappers, other virtual pads) are left alone too, since
/// driving them would only echo their output.
pub fn is_uinput_virtual(path: &Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };
    std::fs::canonicalize(Path::new("/sys/class/input").join(name))
        .is_ok_and(|resolved| is_uinput_path(&resolved))
}

/// A controller Lutra should drive: a gamepad that is not a virtual one (Lutra's or anyone's).
pub fn is_gamepad(path: &Path, device: &Device) -> bool {
    !super::virtual_pad::is_own(path)
        && !is_lutra_hid(path)
        && !is_uinput_virtual(path)
        && is_pad(device)
}

/// Any gamepad (Lutra's virtual ones included) but Steam's: a face button and a stick.
pub fn is_pad(device: &Device) -> bool {
    let id = device.input_id();
    if (id.vendor(), id.product()) == STEAM_VIRTUAL {
        return false;
    }
    let has_face_button = device
        .supported_keys()
        .is_some_and(|keys| keys.contains(KeyCode::new(key::SOUTH)));
    let has_stick = device
        .supported_absolute_axes()
        .is_some_and(|axes| axes.contains(AbsoluteAxisCode(abs::X)));
    has_face_button && has_stick
}

fn non_empty(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

pub fn describe(path: &Path, device: &Device) -> ControllerInfo {
    let id = device.input_id();
    ControllerInfo {
        id: non_empty(device.unique_name())
            .or_else(|| non_empty(device.physical_path()))
            .map(str::to_string)
            .unwrap_or_else(|| path.display().to_string()),
        name: device.name().unwrap_or("Controller").to_string(),
        path: path.display().to_string(),
        vendor: id.vendor(),
        product: id.product(),
        family: Family::from_vendor(id.vendor()),
        model: PadModel::from_ids(id.vendor(), id.product()),
        bus: match id.bus_type() {
            BusType::BUS_USB => "usb",
            BusType::BUS_BLUETOOTH => "bluetooth",
            _ => "other",
        },
    }
}

/// Every connected controller.
pub fn scan() -> Vec<(PathBuf, ControllerInfo)> {
    let mut found: Vec<(PathBuf, ControllerInfo)> = evdev::enumerate()
        .filter(|(path, device)| is_gamepad(path, device))
        .map(|(path, device)| {
            let info = describe(&path, &device);
            (path, info)
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// The other input devices of the same physical controller, such as a DualShock's touchpad
/// (which moves the mouse) or its motion sensors. Matched by serial, so only when there is one.
pub fn companions(controller: &ControllerInfo) -> Vec<Device> {
    let serial = controller.id.as_str();
    if serial == controller.path {
        return Vec::new();
    }
    evdev::enumerate()
        .filter(|(path, device)| {
            path.display().to_string() != controller.path
                && non_empty(device.unique_name()) == Some(serial)
        })
        .map(|(_, device)| device)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uinput_device_is_told_from_a_real_or_uhid_controller() {
        // Lutra's virtual Xbox 360 pad, or anyone's.
        assert!(is_uinput_path(Path::new(
            "/sys/devices/virtual/input/input52/event30"
        )));
        // A Bluetooth DualShock goes through uhid: a real controller.
        assert!(!is_uinput_path(Path::new(
            "/sys/devices/virtual/misc/uhid/0005:054C:09CC.0008/input/input35/event27"
        )));
        // A USB pad.
        assert!(!is_uinput_path(Path::new(
            "/sys/devices/pci0000:00/0000:00:14.0/usb1/1-2/1-2:1.0/input/input20/event12"
        )));
    }
}
