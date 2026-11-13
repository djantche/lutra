//! Knowing what a launched game is really doing, from `/proc`: whether its own process exists
//! yet (the launch is then over), and whether a Steam game is up. Nothing here guesses: a
//! stage is reported as done only when the system shows it is.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::Serialize;

/// One line of the launch progress shown to the user.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProgress {
    pub id: String,
    /// Machine name of the stage, translated by the interface.
    pub stage: &'static str,
    /// Stages finished so far, out of `steps`.
    pub step: u32,
    pub steps: u32,
    pub state: LaunchState,
    /// Set when `state` is `Failed`: what went wrong.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LaunchState {
    Progress,
    /// The game is up.
    Ready,
    /// The game stopped before it came up, with an error.
    Failed,
    /// The launch ended without a game (it exited normally right away).
    Ended,
}

/// A process as `/proc` describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Proc {
    pub pid: u32,
    pub ppid: u32,
    pub pgid: u32,
    /// Kernel name, 15 characters at most.
    pub comm: String,
    pub args: Vec<String>,
}

fn read_proc(dir: &Path) -> Option<Proc> {
    let pid = dir.file_name()?.to_str()?.parse().ok()?;
    let stat = fs::read_to_string(dir.join("stat")).ok()?;
    // `pid (comm) state ppid pgrp ...`: comm may contain spaces and parentheses.
    let after = &stat[stat.rfind(')')? + 1..];
    let mut fields = after.split_whitespace().skip(1);
    let ppid = fields.next()?.parse().ok()?;
    let pgid = fields.next()?.parse().ok()?;
    let comm = fs::read_to_string(dir.join("comm"))
        .unwrap_or_default()
        .trim_end()
        .to_string();
    let args = fs::read(dir.join("cmdline"))
        .map(|bytes| {
            bytes
                .split(|byte| *byte == 0)
                .filter(|arg| !arg.is_empty())
                .map(|arg| String::from_utf8_lossy(arg).into_owned())
                .collect()
        })
        .unwrap_or_default();
    Some(Proc {
        pid,
        ppid,
        pgid,
        comm,
        args,
    })
}

/// Every process currently visible under `proc_root` (`/proc`).
pub fn processes(proc_root: &Path) -> Vec<Proc> {
    fs::read_dir(proc_root)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| read_proc(&entry.path()))
                .collect()
        })
        .unwrap_or_default()
}

/// The kernel keeps 15 characters of a process name: that is how a Wine game shows up, under
/// its own executable's name, once Wine has started it.
pub fn comm_of(exe: &str) -> String {
    let name = exe.rsplit(['/', '\\']).next().unwrap_or(exe);
    name.chars().take(15).collect::<String>().to_lowercase()
}

/// The processes of a launch: the launcher's process group, and everything started from one of
/// its processes. A container (Proton through `umu-run`) or a `setsid` takes a child out of the
/// group, but not out of the family.
pub fn launch_family(procs: &[Proc], pgid: u32) -> HashSet<u32> {
    let mut family: HashSet<u32> = procs
        .iter()
        .filter(|proc| proc.pgid == pgid)
        .map(|proc| proc.pid)
        .collect();
    loop {
        let known = family.len();
        for proc in procs {
            if family.contains(&proc.ppid) {
                family.insert(proc.pid);
            }
        }
        if family.len() == known {
            return family;
        }
    }
}

/// Is the game's own process running as part of the launch?
///
/// The launcher (`umu-run`, `proton`, `wine`) also has the executable in its arguments, so the
/// arguments prove nothing: the process name does.
pub fn game_running(procs: &[Proc], pgid: u32, exe: &str) -> bool {
    let wanted = comm_of(exe);
    let family = launch_family(procs, pgid);
    procs
        .iter()
        .any(|proc| family.contains(&proc.pid) && proc.comm.to_lowercase() == wanted)
}

/// Is a process of that name running anywhere? Only a last resort, once a launch has had time
/// to start its game: a launcher that detaches the game from its own family would otherwise never
/// be seen.
pub fn game_running_anywhere(procs: &[Proc], exe: &str) -> bool {
    let wanted = comm_of(exe);
    procs.iter().any(|proc| proc.comm.to_lowercase() == wanted)
}

/// Is the process group still alive at all?
pub fn group_alive(procs: &[Proc], pgid: u32) -> bool {
    procs.iter().any(|proc| proc.pgid == pgid)
}

/// Is Steam running this app? Steam wraps every game in `reaper SteamLaunch AppId=<id> -- ...`.
pub fn steam_app_running(procs: &[Proc], app_id: &str) -> bool {
    let tag = format!("AppId={app_id}");
    procs
        .iter()
        .any(|proc| proc.args.iter().any(|arg| arg == "SteamLaunch") && proc.args.contains(&tag))
}

/// Does a Wine prefix hold its registry yet? Wine writes `system.reg` at the end of the first-run
/// setup, so its presence is the real end of "creating the prefix".
pub fn prefix_ready(prefix: &Path, proton: bool) -> bool {
    let root = if proton {
        prefix.join("pfx")
    } else {
        prefix.to_path_buf()
    };
    root.join("system.reg").is_file()
}

/// The last lines of a launch log, for an error message.
pub fn log_tail(path: &Path, lines: usize) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    // Skip the header Lutra writes (everything up to the "===" line that closes it).
    let body = text.split("\n===\n").nth(1).unwrap_or(&text);
    let tail: Vec<&str> = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    tail[tail.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(root: &Path, pid: u32, pgid: u32, comm: &str, args: &[&str]) {
        fake_under(root, pid, 1, pgid, comm, args);
    }

    fn fake_under(root: &Path, pid: u32, ppid: u32, pgid: u32, comm: &str, args: &[&str]) {
        let dir = root.join(pid.to_string());
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("stat"),
            format!("{pid} ({comm}) S {ppid} {pgid} {pgid} 0 -1 4194560"),
        )
        .unwrap();
        fs::write(dir.join("comm"), format!("{comm}\n")).unwrap();
        fs::write(dir.join("cmdline"), args.join("\0")).unwrap();
    }

    #[test]
    fn a_wine_game_is_running_when_a_process_carries_its_exe_name() {
        let root = tempfile::tempdir().unwrap();
        // The launcher has the exe in its arguments, but it is not the game.
        fake(
            root.path(),
            100,
            100,
            "umu-run",
            &["umu-run", "/games/FIFA 19/FIFA19.exe"],
        );
        assert!(!game_running(
            &processes(root.path()),
            100,
            "/games/FIFA 19/FIFA19.exe"
        ));

        fake(
            root.path(),
            105,
            100,
            "FIFA19.exe",
            &["Z:\\\\games\\\\FIFA19.exe"],
        );
        // Another group's process of the same name is not ours.
        fake(root.path(), 200, 200, "FIFA19.exe", &[]);
        let procs = processes(root.path());
        assert!(game_running(&procs, 100, "/games/FIFA 19/FIFA19.exe"));
        assert!(!game_running(&procs, 300, "/games/FIFA 19/FIFA19.exe"));
    }

    #[test]
    fn a_game_that_left_the_launchers_group_is_still_found_through_its_parents() {
        let root = tempfile::tempdir().unwrap();
        // umu-run (group 100) starts a container, which puts the game in groups of its own.
        fake(
            root.path(),
            100,
            100,
            "umu-run",
            &["umu-run", "/g/Game.exe"],
        );
        fake_under(root.path(), 110, 100, 110, "bwrap", &[]);
        fake_under(root.path(), 120, 110, 120, "wineserver", &[]);
        fake_under(root.path(), 130, 120, 130, "Game.exe", &[]);
        // The same name started by something unrelated is not ours.
        fake_under(root.path(), 900, 1, 900, "Game.exe", &[]);
        let procs = processes(root.path());
        assert!(game_running(&procs, 100, "/g/Game.exe"));
        assert!(!game_running(&procs, 777, "/g/Game.exe"));
        // Only the last resort sees a process that is related to nothing.
        assert!(game_running_anywhere(&procs, "/g/Game.exe"));
        assert!(!game_running_anywhere(&procs, "/g/Other.exe"));
    }

    #[test]
    fn long_names_are_cut_to_the_kernel_limit_and_case_is_ignored() {
        assert_eq!(
            comm_of("C:\\Games\\VeryLongGameName_x64.exe"),
            "verylonggamenam"
        );
        let root = tempfile::tempdir().unwrap();
        fake(root.path(), 7, 7, "VeryLongGameNam", &[]);
        assert!(game_running(
            &processes(root.path()),
            7,
            "/g/verylonggamename_x64.EXE"
        ));
    }

    #[test]
    fn a_process_name_with_spaces_and_parentheses_still_parses() {
        let root = tempfile::tempdir().unwrap();
        fake(root.path(), 9, 9, "my (odd) name", &[]);
        let procs = processes(root.path());
        assert_eq!(procs[0].pgid, 9);
        assert_eq!(procs[0].comm, "my (odd) name");
    }

    #[test]
    fn a_group_is_alive_while_any_member_remains() {
        let root = tempfile::tempdir().unwrap();
        fake(root.path(), 11, 10, "wineserver", &[]);
        assert!(group_alive(&processes(root.path()), 10));
        assert!(!group_alive(&processes(root.path()), 11));
    }

    #[test]
    fn steam_games_are_found_by_their_reaper_wrapper() {
        let root = tempfile::tempdir().unwrap();
        fake(
            root.path(),
            50,
            50,
            "reaper",
            &["reaper", "SteamLaunch", "AppId=730", "--", "/x/game"],
        );
        fake(
            root.path(),
            51,
            51,
            "reaper",
            &["reaper", "SteamLaunch", "AppId=7300"],
        );
        let procs = processes(root.path());
        assert!(steam_app_running(&procs, "730"));
        assert!(steam_app_running(&procs, "7300"));
        assert!(!steam_app_running(&procs, "73"));
    }

    #[test]
    fn the_prefix_is_ready_once_its_registry_is_written() {
        let prefix = tempfile::tempdir().unwrap();
        assert!(!prefix_ready(prefix.path(), false));
        fs::write(prefix.path().join("system.reg"), "").unwrap();
        assert!(prefix_ready(prefix.path(), false));
        assert!(!prefix_ready(prefix.path(), true));
        fs::create_dir_all(prefix.path().join("pfx")).unwrap();
        fs::write(prefix.path().join("pfx/system.reg"), "").unwrap();
        assert!(prefix_ready(prefix.path(), true));
    }

    #[test]
    fn the_log_tail_skips_the_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g.log");
        fs::write(
            &path,
            "=== Lutra ===\nCommand: x\n===\n\nline1\nline2\n\nline3\n",
        )
        .unwrap();
        assert_eq!(log_tail(&path, 2), "line2\nline3");
        assert_eq!(log_tail(&dir.path().join("missing"), 2), "");
    }
}
