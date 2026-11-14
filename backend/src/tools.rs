//! Installing the helper programs Lutra relies on (Wine, umu-launcher, Winetricks, GameMode,
//! MangoHud) through the system's own package manager.
//!
//! Lutra does not ship these: it asks the distribution for them, with `pkexec` (polkit) so the
//! person types their password in the system's own dialog and Lutra never sees it. The packages
//! are a fixed list per tool and per manager, so nothing from the interface reaches a command
//! line but the tool's name. Progress arrives as `tool-install` events: a line of the manager's
//! output at a time, then `done`, `failed` or `cancelled`.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::applog;
use crate::host::AppHandle;
use crate::wine::{self, find_in_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Wine,
    Umu,
    Winetricks,
    Gamemode,
    Mangohud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Manager {
    Pacman,
    Apt,
    Dnf,
    Zypper,
}

impl Manager {
    fn detect() -> Option<Self> {
        [
            (Manager::Pacman, "pacman"),
            (Manager::Apt, "apt-get"),
            (Manager::Dnf, "dnf"),
            (Manager::Zypper, "zypper"),
        ]
        .into_iter()
        .find(|(_, program)| find_in_path(program).is_some())
        .map(|(manager, _)| manager)
    }

    fn program(self) -> &'static str {
        match self {
            Manager::Pacman => "pacman",
            Manager::Apt => "apt-get",
            Manager::Dnf => "dnf",
            Manager::Zypper => "zypper",
        }
    }

    /// The arguments that install `packages` without asking anything.
    fn install_args(self, packages: &[&str]) -> Vec<String> {
        let fixed: &[&str] = match self {
            Manager::Pacman => &["-S", "--needed", "--noconfirm"],
            Manager::Apt => &["install", "-y"],
            Manager::Dnf => &["install", "-y"],
            Manager::Zypper => &["--non-interactive", "install"],
        };
        fixed
            .iter()
            .chain(packages)
            .map(|part| part.to_string())
            .collect()
    }
}

/// What the manager calls a tool, when it has it. umu-launcher is not in Debian's or openSUSE's
/// repositories: there is nothing to install it from.
fn packages(tool: Tool, manager: Manager) -> Option<&'static [&'static str]> {
    match (tool, manager) {
        (Tool::Wine, _) => Some(&["wine"]),
        (Tool::Winetricks, _) => Some(&["winetricks"]),
        (Tool::Gamemode, _) => Some(&["gamemode"]),
        (Tool::Mangohud, _) => Some(&["mangohud"]),
        (Tool::Umu, Manager::Pacman | Manager::Dnf) => Some(&["umu-launcher"]),
        (Tool::Umu, Manager::Apt | Manager::Zypper) => None,
    }
}

/// How a tool could be installed here, for the interface to offer it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub tool: Tool,
    /// The command to run as administrator, as a person would type it; `None` when this system's
    /// package manager has no such package (or Lutra does not know the manager).
    pub command: Option<String>,
    /// Whether Lutra can run it itself: `pkexec` is there to ask for the password.
    pub can_run: bool,
}

const TOOLS: [Tool; 5] = [
    Tool::Wine,
    Tool::Umu,
    Tool::Winetricks,
    Tool::Gamemode,
    Tool::Mangohud,
];

fn plan_for(tool: Tool, manager: Option<Manager>, has_pkexec: bool) -> Plan {
    let command = manager.and_then(|manager| {
        let args = manager.install_args(packages(tool, manager)?);
        Some(format!("sudo {} {}", manager.program(), args.join(" ")))
    });
    Plan {
        tool,
        can_run: command.is_some() && has_pkexec,
        command,
    }
}

pub fn plans() -> Vec<Plan> {
    let manager = Manager::detect();
    let has_pkexec = find_in_path("pkexec").is_some();
    TOOLS
        .into_iter()
        .map(|tool| plan_for(tool, manager, has_pkexec))
        .collect()
}

fn running() -> &'static Mutex<Option<Tool>> {
    static RUNNING: OnceLock<Mutex<Option<Tool>>> = OnceLock::new();
    RUNNING.get_or_init(|| Mutex::new(None))
}

/// The tool being installed right now, if any (the page may open in the middle of one).
pub fn installing() -> Option<Tool> {
    running().lock().ok().and_then(|tool| *tool)
}

/// Starts installing a tool. Only one install runs at a time: package managers hold a lock.
pub fn install(app: &AppHandle, tool: Tool) -> Result<(), String> {
    let manager = Manager::detect().ok_or("no_package_manager")?;
    let packages = packages(tool, manager).ok_or("no_package")?;
    let pkexec = find_in_path("pkexec").ok_or("no_pkexec")?;
    {
        let mut slot = running().lock().map_err(|error| error.to_string())?;
        if slot.is_some() {
            return Err("busy".into());
        }
        *slot = Some(tool);
    }

    let args = manager.install_args(packages);
    let app = app.clone();
    std::thread::spawn(move || {
        applog::info(format!(
            "Installing {tool:?}: pkexec {} {}",
            manager.program(),
            args.join(" ")
        ));
        let outcome = run(&app, tool, &pkexec, manager.program(), &args);
        if let Ok(mut slot) = running().lock() {
            *slot = None;
        }
        let (state, message) = match outcome {
            Ok(()) => ("done", String::new()),
            // pkexec's own answers: 126 is a dismissed dialog, 127 an authentication that failed
            // or no agent to ask with.
            Err(Failure::Code(126)) => ("cancelled", String::new()),
            Err(Failure::Code(127)) => ("failed", "no_authentication".to_string()),
            Err(Failure::Code(code)) => ("failed", format!("exit {code}")),
            Err(Failure::Other(error)) => ("failed", error),
        };
        applog::info(format!("Installing {tool:?}: {state} {message}"));
        let _ = app.emit(
            "tool-install",
            json!({ "tool": tool, "state": state, "message": message }),
        );
    });
    Ok(())
}

enum Failure {
    Code(i32),
    Other(String),
}

fn run(
    app: &AppHandle,
    tool: Tool,
    pkexec: &PathBuf,
    program: &str,
    args: &[String],
) -> Result<(), Failure> {
    // The manager's errors and its output go to one pipe, in the order they happened.
    let mut command = Command::new("sh");
    wine::host_env(&mut command)
        .args(["-c", "exec \"$@\" 2>&1", "sh"])
        .arg(pkexec)
        .arg(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| Failure::Other(error.to_string()))?;
    let mut output = child
        .stdout
        .take()
        .ok_or_else(|| Failure::Other("no output".into()))?;

    let mut pending = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = match output.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        for byte in &buffer[..read] {
            // Progress meters redraw their line with a carriage return.
            if *byte == b'\n' || *byte == b'\r' {
                emit_line(app, tool, &mut pending);
            } else {
                pending.push(*byte);
            }
        }
    }
    emit_line(app, tool, &mut pending);

    let status = child
        .wait()
        .map_err(|error| Failure::Other(error.to_string()))?;
    match status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(Failure::Code(code)),
        None => Err(Failure::Other("stopped by a signal".into())),
    }
}

fn emit_line(app: &AppHandle, tool: Tool, pending: &mut Vec<u8>) {
    let line = String::from_utf8_lossy(pending).trim().to_string();
    pending.clear();
    if !line.is_empty() {
        let _ = app.emit(
            "tool-install",
            json!({ "tool": tool, "state": "running", "line": line }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_manager_installs_without_asking_anything() {
        assert_eq!(
            Manager::Pacman.install_args(&["gamemode"]),
            ["-S", "--needed", "--noconfirm", "gamemode"]
        );
        assert_eq!(
            Manager::Apt.install_args(&["mangohud"]),
            ["install", "-y", "mangohud"]
        );
        assert_eq!(
            Manager::Dnf.install_args(&["wine"]),
            ["install", "-y", "wine"]
        );
        assert_eq!(
            Manager::Zypper.install_args(&["winetricks"]),
            ["--non-interactive", "install", "winetricks"]
        );
    }

    #[test]
    fn a_plan_is_the_command_a_person_would_type_and_whether_lutra_can_run_it() {
        let plan = plan_for(Tool::Gamemode, Some(Manager::Pacman), true);
        assert_eq!(
            plan.command.as_deref(),
            Some("sudo pacman -S --needed --noconfirm gamemode")
        );
        assert!(plan.can_run);
        // Without pkexec the command is still shown, but Lutra cannot run it.
        assert!(!plan_for(Tool::Gamemode, Some(Manager::Pacman), false).can_run);
        // A manager Lutra does not know: nothing to offer.
        assert_eq!(plan_for(Tool::Wine, None, true).command, None);
    }

    #[test]
    fn a_tool_a_distribution_does_not_package_has_no_plan() {
        assert_eq!(plan_for(Tool::Umu, Some(Manager::Apt), true).command, None);
        assert!(
            plan_for(Tool::Umu, Some(Manager::Dnf), true)
                .command
                .is_some()
        );
    }
}
