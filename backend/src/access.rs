//! The one-time permission controllers need: `/dev/uinput` and `/dev/uhid` writable by the person
//! at the computer (that is how Lutra makes virtual controllers), and their kernel modules loaded
//! at every boot. Nobody types a command: Lutra asks the system for the administrator's password
//! through polkit (`pkexec`, the same dialog as installing a tool), then does everything itself.

use std::io::Read;
use std::process::{Command, Stdio};

use crate::{applog, controller, wine};

/// What runs as the administrator. A fixed text: nothing the interface sends is part of it.
const SCRIPT: &str = r#"set -e
cat > /etc/udev/rules.d/60-lutra-uinput.rules <<'RULES'
KERNEL=="uinput", SUBSYSTEM=="misc", TAG+="uaccess", OPTIONS+="static_node=uinput"
RULES
cat > /etc/udev/rules.d/60-lutra-uhid.rules <<'RULES'
KERNEL=="uhid", SUBSYSTEM=="misc", TAG+="uaccess", OPTIONS+="static_node=uhid"
KERNEL=="hidraw*", KERNELS=="*054C:05C4*", MODE="0660", TAG+="uaccess"
KERNEL=="hidraw*", KERNELS=="*054C:09CC*", MODE="0660", TAG+="uaccess"
RULES
mkdir -p /etc/modules-load.d
printf 'uinput\nuhid\n' > /etc/modules-load.d/lutra.conf
modprobe uinput || true
modprobe uhid || true
udevadm control --reload
udevadm trigger --subsystem-match=misc --subsystem-match=hidraw --action=add
udevadm settle || true
"#;

/// Whether the system lets Lutra create virtual controllers and HID ones.
pub fn granted() -> bool {
    controller::uinput_available() && controller::uhid_available()
}

/// Asks for the administrator's password and sets controllers up. Errors are short codes the
/// interface turns into words: `no_pkexec`, `cancelled`, `no_authentication`, `not_applied`.
pub fn grant() -> Result<(), String> {
    let pkexec = wine::find_in_path("pkexec").ok_or("no_pkexec")?;
    applog::info("Setting up controller access (udev rules and modules) through pkexec");

    let mut command = Command::new(pkexec);
    let mut child = wine::host_env(&mut command)
        .args(["sh", "-c", SCRIPT])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut errors = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut errors);
    }
    let status = child.wait().map_err(|error| error.to_string())?;

    match status.code() {
        Some(0) => {}
        // pkexec's own answers: 126 is a dismissed dialog, 127 an authentication that failed or
        // no agent to ask with.
        Some(126) => return Err("cancelled".into()),
        Some(127) => return Err("no_authentication".into()),
        code => {
            applog::warn(format!(
                "Controller access setup failed ({code:?}): {errors}"
            ));
            return Err(format!("exit {}", code.unwrap_or(-1)));
        }
    }

    // The ACL is given by udev a moment after the script ends.
    for _ in 0..20 {
        if granted() {
            applog::info("Controller access is set up");
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    applog::warn("Controller access setup ran, but the devices are still not writable");
    Err("not_applied".into())
}
