//! Firmware (BIOS) files: which ones a core asks for, and which are in the user's folder.
//! Lutra never provides them; it only tells what is missing.

use std::fs;
use std::path::Path;

use md5::{Digest, Md5};
use serde::Serialize;

use super::info::{CoreInfo, Firmware};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// The file is there (and has the expected MD5 when one is known).
    Present,
    /// A file with that name exists but its MD5 is not the expected one: another revision,
    /// which may or may not work.
    Different,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareStatus {
    pub description: String,
    pub path: String,
    pub optional: bool,
    pub state: State,
}

fn md5_hex(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(
        Md5::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

fn check(firmware: &Firmware, system_dir: &Path) -> State {
    let path = system_dir.join(&firmware.path);
    if !path.is_file() {
        return State::Missing;
    }
    match (&firmware.md5, md5_hex(&path)) {
        (Some(expected), Some(actual)) if *expected != actual => State::Different,
        _ => State::Present,
    }
}

/// The state of each firmware a core asks for.
pub fn status(core: &CoreInfo, system_dir: &Path) -> Vec<FirmwareStatus> {
    core.firmware
        .iter()
        .map(|firmware| FirmwareStatus {
            description: firmware.description.clone(),
            path: firmware.path.clone(),
            optional: firmware.optional,
            state: check(firmware, system_dir),
        })
        .collect()
}

/// Can the core start? Only a missing firmware the core cannot do without says no.
pub fn ready(statuses: &[FirmwareStatus]) -> bool {
    statuses
        .iter()
        .all(|firmware| firmware.optional || firmware.state != State::Missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emulation::info::parse_core;

    fn core(text: &str) -> CoreInfo {
        parse_core("c", text)
    }

    const TEXT: &str = r#"firmware_count = 3
firmware0_desc = "bios.bin (BIOS)"
firmware0_path = "bios.bin"
firmware0_opt = "false"
firmware1_desc = "sub/boot.bin (Boot)"
firmware1_path = "sub/boot.bin"
firmware1_opt = "false"
firmware2_desc = "extra.bin"
firmware2_path = "extra.bin"
firmware2_opt = "true"
notes = "(!) bios.bin (md5): 5d41402abc4b2a76b9719d911017c592|(!) boot.bin (md5): 5d41402abc4b2a76b9719d911017c592"
"#;

    #[test]
    fn firmware_is_present_different_or_missing() {
        let dir = tempfile::tempdir().unwrap();
        // md5("hello") is the expected hash of both.
        fs::write(dir.path().join("bios.bin"), "hello").unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/boot.bin"), "another revision").unwrap();

        let statuses = status(&core(TEXT), dir.path());
        assert_eq!(statuses[0].state, State::Present);
        assert_eq!(statuses[1].state, State::Different);
        assert_eq!(statuses[2].state, State::Missing);
    }

    #[test]
    fn a_core_is_ready_unless_a_required_firmware_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let info = core(TEXT);
        assert!(!ready(&status(&info, dir.path())));

        fs::write(dir.path().join("bios.bin"), "hello").unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/boot.bin"), "x").unwrap();
        // The optional one is still missing: that does not stop the core.
        assert!(ready(&status(&info, dir.path())));
    }

    #[test]
    fn a_core_without_firmware_is_always_ready() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ready(&status(&core(""), dir.path())));
    }
}
