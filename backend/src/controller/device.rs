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
    lutra_hid_phys(path).is_some()
}

/// The `phys` of one of Lutra's own HID controllers (`lutra-virtual/ds4-3`), read from the HID
/// device behind this input node; `None` for any other device.
pub fn lutra_hid_phys(path: &Path) -> Option<String> {
    let name = path.file_name()?;
    let uevent = Path::new("/sys/class/input")
        .join(name)
        .join("device/device/uevent");
    std::fs::read_to_string(uevent)
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("HID_PHYS=")
                .filter(|phys| phys.starts_with(super::virtual_ds4::PHYS_PREFIX))
                .map(str::to_string)
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

/// What a USB device says about itself in its descriptors.
#[derive(Debug, Default, PartialEq)]
struct UsbStrings {
    product: Option<String>,
    manufacturer: Option<String>,
}

/// The USB strings of the device behind an input node, found from its resolved `sysfs` path: the
/// USB device is the closest ancestor with an `idVendor` file. Whatever driver took the controller
/// (`xpad`, `hid-generic`, a vendor's), the strings are the maker's own, while the input device's
/// name is the driver's and often a generic one.
fn usb_strings(resolved: &Path) -> UsbStrings {
    let read = |dir: &Path, file: &str| {
        std::fs::read_to_string(dir.join(file))
            .ok()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
    };
    resolved
        .ancestors()
        .find(|dir| dir.join("idVendor").is_file())
        .map(|usb| UsbStrings {
            product: read(usb, "product"),
            manufacturer: read(usb, "manufacturer"),
        })
        .unwrap_or_default()
}

/// The names a driver gives to controllers it knows nothing about, which say nothing of them.
const DRIVER_FALLBACK_NAMES: &[&str] = &["Generic X-Box pad"];

/// The name to show: the input device's, unless the driver made it up because it did not know the
/// controller, in which case the maker's own product name is better.
fn display_name(input_name: &str, usb_product: Option<&str>) -> String {
    match usb_product {
        Some(product) if DRIVER_FALLBACK_NAMES.contains(&input_name) => product.to_string(),
        _ => input_name.to_string(),
    }
}

pub fn describe(path: &Path, device: &Device) -> ControllerInfo {
    let id = device.input_id();
    let input_name = device.name().unwrap_or("Controller");
    let usb = path
        .file_name()
        .and_then(|node| std::fs::canonicalize(Path::new("/sys/class/input").join(node)).ok())
        .map(|resolved| usb_strings(&resolved))
        .unwrap_or_default();
    ControllerInfo {
        id: non_empty(device.unique_name())
            .or_else(|| non_empty(device.physical_path()))
            .map(str::to_string)
            .unwrap_or_else(|| path.display().to_string()),
        name: display_name(input_name, usb.product.as_deref()),
        path: path.display().to_string(),
        vendor: id.vendor(),
        product: id.product(),
        family: Family::from_vendor(id.vendor()),
        model: PadModel::detect(
            id.vendor(),
            id.product(),
            &[
                input_name,
                usb.product.as_deref().unwrap_or_default(),
                usb.manufacturer.as_deref().unwrap_or_default(),
            ],
        ),
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
    fn the_usb_strings_come_from_the_usb_device_above_the_input_node() {
        let root = tempfile::tempdir().unwrap();
        let usb = root.path().join("usb1/1-2");
        let node = usb.join("1-2:1.0/input/input91/event5");
        std::fs::create_dir_all(&node).unwrap();
        std::fs::write(usb.join("idVendor"), "0079\n").unwrap();
        std::fs::write(usb.join("product"), "FightStick\n").unwrap();
        std::fs::write(usb.join("manufacturer"), " \n").unwrap();

        assert_eq!(
            usb_strings(&node),
            UsbStrings {
                product: Some("FightStick".into()),
                // Blank, like the one this stick gives.
                manufacturer: None,
            }
        );
        // Bluetooth, or a laptop's own device: no USB device above, nothing to read.
        let other = root.path().join("bluetooth/hci0/input7/event9");
        std::fs::create_dir_all(&other).unwrap();
        assert_eq!(usb_strings(&other), UsbStrings::default());
    }

    #[test]
    fn a_made_up_driver_name_gives_way_to_the_makers_product_name() {
        assert_eq!(
            display_name("Generic X-Box pad", Some("FightStick")),
            "FightStick"
        );
        // The driver knows this one: its name stays.
        assert_eq!(
            display_name("Microsoft X-Box 360 pad", Some("Controller")),
            "Microsoft X-Box 360 pad"
        );
        assert_eq!(display_name("Generic X-Box pad", None), "Generic X-Box pad");
        assert_eq!(display_name("FightStick", Some("FightStick")), "FightStick");
    }

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
