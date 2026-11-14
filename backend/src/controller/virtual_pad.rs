//! Lutra's virtual controllers: `uinput` devices that look exactly like an Xbox 360 pad or a
//! DualShock 4, down to their USB ids, so games and SDL treat them as the real thing.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use evdev::uinput::VirtualDevice;
use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, FFEffectCode, InputId, KeyCode,
    UinputAbsSetup,
};

use super::codes::{abs, button_codes};
use super::model::OutputKind;

/// How many rumble effects a game may keep uploaded at once.
const MAX_EFFECTS: u32 = 16;

/// `/dev/input` nodes of Lutra's own virtual controllers, so they are never read back as input.
static OWN_NODES: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(Mutex::default);

pub fn is_own(path: &Path) -> bool {
    OWN_NODES.lock().is_ok_and(|nodes| nodes.contains(path))
}

/// A virtual controller. Its nodes stay registered as Lutra's own until it is dropped.
pub struct VirtualPad {
    pub device: VirtualDevice,
    nodes: Vec<PathBuf>,
}

impl Drop for VirtualPad {
    fn drop(&mut self) {
        if let Ok(mut own) = OWN_NODES.lock() {
            for node in &self.nodes {
                own.remove(node);
            }
        }
    }
}

/// The USB ids a virtual controller of `kind` takes, given the physical controllers connected
/// (`physicals`). They never equal a physical one's, so games can be told to ignore the physical
/// controllers (`SDL_GAMECONTROLLER_IGNORE_DEVICES`) and still see the virtual ones: it takes the
/// first of the ids that kind of controller has (a DualShock 4 has three) that no connected
/// controller uses. It must look at all of them: a virtual DualShock 4 made for an arcade stick
/// that took the ids of the real DualShock 4 beside it left that one visible to the game, which
/// then saw it twice. Xbox 360 pads have one set of ids: a real one is not told apart.
pub fn identity(kind: OutputKind, physicals: &[(u16, u16)]) -> Option<(u16, u16)> {
    let candidates: &[(u16, u16)] = match kind {
        OutputKind::Xbox360 => &[XBOX_360],
        OutputKind::Ds4 => &[DS4, DS4_FIRST, DS4_DONGLE],
        OutputKind::Auto | OutputKind::Keyboard | OutputKind::None => return None,
    };
    candidates
        .iter()
        .find(|ids| !physicals.contains(ids))
        .or(candidates.first())
        .copied()
}

const XBOX_360: (u16, u16) = (0x045e, 0x028e);
const DS4: (u16, u16) = (0x054c, 0x09cc);
const DS4_FIRST: (u16, u16) = (0x054c, 0x05c4);
/// Sony's USB adapter: the kernel's `hid-playstation` and SDL take it for a DualShock 4 as well.
const DS4_DONGLE: (u16, u16) = (0x054c, 0x0ba0);

pub fn create(kind: OutputKind, physicals: &[(u16, u16)]) -> io::Result<VirtualPad> {
    let Some((vendor, product)) = identity(kind, physicals) else {
        return Err(io::Error::other("this profile has no virtual gamepad"));
    };
    let (name, version, stick) = match kind {
        OutputKind::Xbox360 => (
            "Microsoft X-Box 360 pad",
            0x0114,
            AbsInfo::new(0, -32768, 32767, 16, 128, 0),
        ),
        OutputKind::Ds4 => (
            "Sony Interactive Entertainment Wireless Controller",
            0x8111,
            AbsInfo::new(128, 0, 255, 0, 0, 0),
        ),
        OutputKind::Auto | OutputKind::Keyboard | OutputKind::None => {
            return Err(io::Error::other("this profile has no virtual gamepad"));
        }
    };
    let id = InputId::new(BusType::BUS_USB, vendor, product, version);
    let trigger = AbsInfo::new(0, 0, 255, 0, 0, 0);
    let hat = AbsInfo::new(0, -1, 1, 0, 0, 0);

    let keys: AttributeSet<KeyCode> = button_codes(kind)
        .iter()
        .map(|(_, code)| KeyCode::new(*code))
        .collect();
    let rumble: AttributeSet<FFEffectCode> = [FFEffectCode::FF_RUMBLE].into_iter().collect();
    // No physical path: the kernel refuses UI_SET_PHYS as `evdev` sends it (EINVAL).
    let mut builder = VirtualDevice::builder()?
        .name(name)
        .input_id(id)
        .with_keys(&keys)?;
    for (axis, info) in [
        (abs::X, stick),
        (abs::Y, stick),
        (abs::RX, stick),
        (abs::RY, stick),
        (abs::Z, trigger),
        (abs::RZ, trigger),
        (abs::HAT0X, hat),
        (abs::HAT0Y, hat),
    ] {
        builder = builder.with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode(axis), info))?;
    }
    let mut device = builder
        .with_ff(&rumble)?
        .with_ff_effects_max(MAX_EFFECTS)
        .build()?;

    let nodes: Vec<PathBuf> = device
        .enumerate_dev_nodes_blocking()?
        .filter_map(Result::ok)
        .collect();
    if let Ok(mut own) = OWN_NODES.lock() {
        own.extend(nodes.iter().cloned());
    }
    Ok(VirtualPad { device, nodes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_virtual_dualshock_never_takes_the_ids_of_a_connected_controller() {
        assert_eq!(identity(OutputKind::Ds4, &[]), Some(DS4));
        // Beside a real second-generation DualShock 4, or one that is not Sony's at all.
        assert_eq!(identity(OutputKind::Ds4, &[DS4]), Some(DS4_FIRST));
        assert_eq!(identity(OutputKind::Ds4, &[(0x0079, 0x187c)]), Some(DS4));
        // Both generations at once, which is what two DualShocks of different models are.
        assert_eq!(
            identity(OutputKind::Ds4, &[DS4, DS4_FIRST]),
            Some(DS4_DONGLE)
        );
        // Every id taken: there is nothing better than the first.
        assert_eq!(
            identity(OutputKind::Ds4, &[DS4, DS4_FIRST, DS4_DONGLE]),
            Some(DS4)
        );
    }

    #[test]
    fn the_xbox_360_ids_and_the_kinds_without_a_pad() {
        assert_eq!(identity(OutputKind::Xbox360, &[]), Some(XBOX_360));
        assert_eq!(identity(OutputKind::Xbox360, &[XBOX_360]), Some(XBOX_360));
        for kind in [OutputKind::Auto, OutputKind::Keyboard, OutputKind::None] {
            assert_eq!(identity(kind, &[DS4]), None);
        }
    }
}
