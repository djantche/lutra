//! The battery of a controller, as the kernel reports it, and the alerts when it runs low.
//!
//! A wireless controller shows up in `/sys/class/power_supply` as a battery whose `device` is
//! the HID device that owns the controller's input nodes. Matching on that parent, rather than
//! on a name, works the same over USB and Bluetooth, for PlayStation and Xbox pads alike.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

/// Alert once when the battery first drops to this much...
pub const LOW: u8 = 30;
/// ...and again when it drops to this much.
pub const CRITICAL: u8 = 15;
/// A battery above this is considered recovered: the next drop alerts again.
const RECOVERED: u8 = LOW + 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

/// The battery of the controller whose input node is `node` (`/dev/input/eventN`), if it has one.
pub fn read(node: &Path) -> Option<Battery> {
    read_in(Path::new("/sys"), node)
}

fn read_in(sys: &Path, node: &Path) -> Option<Battery> {
    let input = fs::canonicalize(
        sys.join("class/input")
            .join(node.file_name()?)
            .join("device"),
    )
    .ok()?;
    for entry in fs::read_dir(sys.join("class/power_supply")).ok()?.flatten() {
        let supply = entry.path();
        let text = |name: &str| {
            fs::read_to_string(supply.join(name))
                .ok()
                .map(|s| s.trim().to_string())
        };
        if text("type").as_deref() != Some("Battery") {
            continue;
        }
        let Ok(owner) = fs::canonicalize(supply.join("device")) else {
            continue;
        };
        if !input.starts_with(&owner) {
            continue;
        }
        let percent = text("capacity")?.parse::<u8>().ok()?.min(100);
        let charging = text("status").as_deref() == Some("Charging");
        return Some(Battery { percent, charging });
    }
    None
}

/// Remembers, per controller, which alert was already given, so each is given once.
#[derive(Default)]
pub struct Alerts {
    given: HashMap<String, u8>,
}

impl Alerts {
    /// The alert level (`LOW` or `CRITICAL`) to give for this reading, if one is due now.
    ///
    /// Charging, or climbing back above `RECOVERED`, rearms the alerts. A drop straight to the
    /// critical level gives only the critical alert.
    pub fn check(&mut self, id: &str, battery: Battery) -> Option<u8> {
        let given = self.given.entry(id.to_string()).or_insert(0);
        if battery.charging || battery.percent > RECOVERED {
            *given = 0;
            return None;
        }
        let due = if battery.percent <= CRITICAL {
            CRITICAL
        } else if battery.percent <= LOW {
            LOW
        } else {
            return None;
        };
        if *given == 0 || due < *given {
            *given = due;
            Some(due)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn battery(percent: u8, charging: bool) -> Battery {
        Battery { percent, charging }
    }

    #[test]
    fn alerts_come_once_at_30_then_once_at_15() {
        let mut alerts = Alerts::default();
        assert_eq!(alerts.check("pad", battery(80, false)), None);
        assert_eq!(alerts.check("pad", battery(31, false)), None);
        assert_eq!(alerts.check("pad", battery(30, false)), Some(LOW));
        assert_eq!(alerts.check("pad", battery(28, false)), None);
        assert_eq!(alerts.check("pad", battery(16, false)), None);
        assert_eq!(alerts.check("pad", battery(15, false)), Some(CRITICAL));
        assert_eq!(alerts.check("pad", battery(9, false)), None);
    }

    #[test]
    fn a_drop_straight_to_the_critical_level_gives_only_that_alert() {
        let mut alerts = Alerts::default();
        assert_eq!(alerts.check("pad", battery(80, false)), None);
        assert_eq!(alerts.check("pad", battery(12, false)), Some(CRITICAL));
    }

    #[test]
    fn charging_or_recovering_rearms_the_alerts() {
        let mut alerts = Alerts::default();
        assert_eq!(alerts.check("pad", battery(14, false)), Some(CRITICAL));
        // Plugged in: no alert while charging, and the next drop alerts again.
        assert_eq!(alerts.check("pad", battery(14, true)), None);
        assert_eq!(alerts.check("pad", battery(14, false)), Some(CRITICAL));
        // A little above the threshold is not recovered; well above is.
        assert_eq!(alerts.check("pad", battery(33, false)), None);
        assert_eq!(alerts.check("pad", battery(30, false)), None);
        assert_eq!(alerts.check("pad", battery(60, false)), None);
        assert_eq!(alerts.check("pad", battery(30, false)), Some(LOW));
    }

    #[test]
    fn controllers_are_followed_separately() {
        let mut alerts = Alerts::default();
        assert_eq!(alerts.check("a", battery(20, false)), Some(LOW));
        assert_eq!(alerts.check("b", battery(20, false)), Some(LOW));
        assert_eq!(alerts.check("a", battery(20, false)), None);
    }

    /// A fake `/sys` with a controller and two batteries: its own, and an unrelated one.
    #[test]
    fn the_battery_is_found_through_the_hid_device_that_owns_the_input() {
        let sys = tempfile::tempdir().unwrap();
        let root = sys.path();
        let hid = root.join("devices/virtual/misc/uhid/0005:054C:09CC.0001");
        let input = hid.join("input/input9");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(root.join("class/input/event7")).unwrap();
        symlink(&input, root.join("class/input/event7/device")).unwrap();

        let supply = |name: &str, owner: &Path, capacity: &str, status: &str| {
            let dir = root.join("class/power_supply").join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("type"), "Battery\n").unwrap();
            fs::write(dir.join("capacity"), format!("{capacity}\n")).unwrap();
            fs::write(dir.join("status"), format!("{status}\n")).unwrap();
            symlink(owner, dir.join("device")).unwrap();
        };
        let other = root.join("devices/virtual/misc/uhid/0003:054C:05C4.0002");
        fs::create_dir_all(&other).unwrap();
        supply("someone-else", &other, "5", "Discharging");
        supply("ps-controller-battery", &hid, "42", "Charging");

        let found = read_in(root, Path::new("/dev/input/event7"));
        assert_eq!(found, Some(battery(42, true)));
        // A controller with no battery of its own gets none.
        fs::create_dir_all(root.join("class/input/event8")).unwrap();
        let lonely = root.join("devices/usb/1-1/input/input10");
        fs::create_dir_all(&lonely).unwrap();
        symlink(&lonely, root.join("class/input/event8/device")).unwrap();
        assert_eq!(read_in(root, Path::new("/dev/input/event8")), None);
    }
}
