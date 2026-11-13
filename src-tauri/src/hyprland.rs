//! Showing the in-game overlay on Hyprland.
//!
//! Hyprland hides every window behind a fullscreen one, and a window Lutra opens lands on the
//! workspace Lutra was started from, not on the game's. A floating, pinned window is the one
//! kind that is drawn above a fullscreen window, so the overlay is made one and placed over the
//! screen the game is on. Hyprland's Lua configuration (Omarchy's) takes Lua expressions in
//! `hyprctl dispatch`; the old `focuswindow title:…` syntax is silently rejected.

use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

/// How long to wait for the compositor to know about the overlay window.
const MAP_TIMEOUT: Duration = Duration::from_secs(3);

pub fn available() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

fn hyprctl_json(what: &str) -> Option<Value> {
    let output = crate::wine::host_env(&mut Command::new("hyprctl"))
        .args(["-j", what])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| serde_json::from_slice(&output.stdout).ok())
        .flatten()
}

fn dispatch(lua: &str) -> bool {
    crate::wine::host_env(&mut Command::new("hyprctl"))
        .args(["dispatch", lua])
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "ok"
        })
}

/// Where the overlay goes: its window, and the screen (in global layout coordinates) to cover.
#[derive(Debug, PartialEq)]
struct Placement {
    address: String,
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}

fn int(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

/// Picks the overlay window and the monitor of the game: the fullscreen window that is not
/// Lutra's, else the monitor that has the focus.
fn plan(clients: &Value, monitors: &Value, own_pid: i64, title: &str) -> Option<Placement> {
    let clients = clients.as_array()?;
    let overlay = clients
        .iter()
        .find(|client| client.get("title").and_then(Value::as_str) == Some(title))?;
    let address = overlay.get("address")?.as_str()?.to_owned();

    let monitors = monitors.as_array()?;
    let game_monitor = clients
        .iter()
        .find(|client| int(client, "pid") != own_pid && int(client, "fullscreen") > 0)
        .map(|game| int(game, "monitor"));
    let monitor = game_monitor
        .and_then(|id| monitors.iter().find(|monitor| int(monitor, "id") == id))
        .or_else(|| {
            monitors
                .iter()
                .find(|monitor| monitor.get("focused").and_then(Value::as_bool) == Some(true))
        })
        .or_else(|| monitors.first())?;

    let scale = monitor
        .get("scale")
        .and_then(Value::as_f64)
        .filter(|scale| *scale > 0.0)
        .unwrap_or(1.0);
    Some(Placement {
        address,
        x: int(monitor, "x"),
        y: int(monitor, "y"),
        width: (int(monitor, "width") as f64 / scale).round() as i64,
        height: (int(monitor, "height") as f64 / scale).round() as i64,
    })
}

/// Makes the overlay window cover the game's screen, above the game. Waits for the window to
/// appear; best effort, since the overlay still works as a plain window elsewhere.
pub fn show_overlay(title: &'static str) {
    if !available() {
        return;
    }
    thread::spawn(move || {
        let started = Instant::now();
        let placement = loop {
            let found = hyprctl_json("clients")
                .zip(hyprctl_json("monitors"))
                .and_then(|(clients, monitors)| {
                    plan(&clients, &monitors, i64::from(std::process::id()), title)
                });
            if let Some(found) = found {
                break found;
            }
            if started.elapsed() > MAP_TIMEOUT {
                crate::applog::warn("Overlay: Hyprland never listed the window");
                return;
            }
            thread::sleep(Duration::from_millis(50));
        };

        let window = format!("window = 'address:{}'", placement.address);
        let steps = [
            format!("hl.dsp.window.float({{ action = 'enable', {window} }})"),
            format!("hl.dsp.window.pin({{ action = 'enable', {window} }})"),
            format!(
                "hl.dsp.window.move({{ x = {}, y = {}, {window} }})",
                placement.x, placement.y
            ),
            format!(
                "hl.dsp.window.resize({{ x = {}, y = {}, {window} }})",
                placement.width, placement.height
            ),
            format!("hl.dsp.window.bring_to_top({{ {window} }})"),
            format!("hl.dsp.focus({{ {window} }})"),
        ];
        for step in &steps {
            if !dispatch(step) {
                crate::applog::warn(format!("Overlay: Hyprland refused `{step}`"));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn monitors() -> Value {
        json!([
            {"id": 0, "x": 0, "y": 0, "width": 2880, "height": 1800, "scale": 1.5, "focused": true},
            {"id": 1, "x": 1920, "y": 0, "width": 1920, "height": 1080, "scale": 1.0, "focused": false}
        ])
    }

    #[test]
    fn the_overlay_covers_the_monitor_of_the_fullscreen_game() {
        let clients = json!([
            {"address": "0xa", "title": "Lutra Overlay", "pid": 10, "fullscreen": 0, "monitor": 0},
            {"address": "0xb", "title": "Zenonia", "pid": 99, "fullscreen": 2, "monitor": 1}
        ]);
        let placement = plan(&clients, &monitors(), 10, "Lutra Overlay").unwrap();
        assert_eq!(
            placement,
            Placement {
                address: "0xa".into(),
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080
            }
        );
    }

    #[test]
    fn a_scaled_monitor_is_covered_in_layout_pixels() {
        let clients = json!([
            {"address": "0xa", "title": "Lutra Overlay", "pid": 10, "fullscreen": 0, "monitor": 0}
        ]);
        let placement = plan(&clients, &monitors(), 10, "Lutra Overlay").unwrap();
        assert_eq!((placement.width, placement.height), (1920, 1200));
    }

    #[test]
    fn lutra_s_own_fullscreen_window_is_not_mistaken_for_the_game() {
        let clients = json!([
            {"address": "0xa", "title": "Lutra Overlay", "pid": 10, "fullscreen": 1, "monitor": 1},
            {"address": "0xc", "title": "Lutra", "pid": 10, "fullscreen": 2, "monitor": 1}
        ]);
        let placement = plan(&clients, &monitors(), 10, "Lutra Overlay").unwrap();
        // Falls back to the focused monitor.
        assert_eq!(placement.x, 0);
    }

    #[test]
    fn nothing_is_placed_until_the_window_exists() {
        let clients = json!([{"address": "0xb", "title": "Zenonia", "pid": 99, "fullscreen": 2, "monitor": 0}]);
        assert_eq!(plan(&clients, &monitors(), 10, "Lutra Overlay"), None);
    }
}
