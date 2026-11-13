//! Lutra in the notification area. The icon and the window live in the Electron shell; the
//! backend keeps what the user chose and tells the shell what to do with the window.
//!
//! Hiding the window is only ever done when a notification area is there to bring it back
//! (a `StatusNotifierWatcher` on the session bus): otherwise Lutra would be out of reach.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::host::AppHandle;
use crate::{Library, applog};

const CLOSE_TO_TRAY: &str = "tray_close";
const START_IN_TRAY: &str = "tray_start";
const HIDE_DURING_GAMES: &str = "tray_games";

/// The user's choices about the notification area.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraySettings {
    /// Closing the window keeps Lutra running in the notification area.
    pub close_to_tray: bool,
    /// Lutra starts in the notification area, its window hidden.
    pub start_in_tray: bool,
    /// The window hides while a game runs and comes back when it ends.
    pub hide_during_games: bool,
    /// A notification area exists on this desktop (without one, nothing is ever hidden).
    pub available: bool,
}

/// Whether a notification area was found at start.
pub struct Tray {
    available: bool,
}

/// Whether a notification area is there to show Lutra's icon.
fn host_available() -> bool {
    crate::wine::host_env(&mut std::process::Command::new("busctl"))
        .args(["--user", "--no-legend", "list"])
        .output()
        .is_ok_and(|output| {
            String::from_utf8_lossy(&output.stdout).contains("org.kde.StatusNotifierWatcher")
        })
}

fn flag(app: &AppHandle, key: &str, default: bool) -> bool {
    app.state::<Library>()
        .lock()
        .ok()
        .and_then(|db| db.setting(key).ok().flatten())
        .map_or(default, |value| value == "1")
}

/// Looks for a notification area. Called once at start.
pub fn setup(app: &AppHandle) {
    let available = host_available();
    if !available {
        applog::info("No notification area on this desktop: Lutra's window is never hidden");
    }
    app.manage(Tray { available });
}

pub fn settings(app: &AppHandle) -> TraySettings {
    TraySettings {
        close_to_tray: flag(app, CLOSE_TO_TRAY, true),
        start_in_tray: flag(app, START_IN_TRAY, false),
        hide_during_games: flag(app, HIDE_DURING_GAMES, true),
        available: app.state::<Tray>().available,
    }
}

/// Tells the shell how closing and starting should behave. Sent at start and after every change.
pub fn announce(app: &AppHandle) {
    let settings = settings(app);
    let _ = app.emit(
        "host:tray",
        json!({
            "available": settings.available,
            "closeToTray": settings.available && settings.close_to_tray,
            "startInTray": settings.available && settings.start_in_tray,
        }),
    );
}

pub fn save_settings(app: &AppHandle, settings: &TraySettings) -> Result<(), String> {
    {
        let db = app.state::<Library>();
        let db = db.lock()?;
        for (key, value) in [
            (CLOSE_TO_TRAY, settings.close_to_tray),
            (START_IN_TRAY, settings.start_in_tray),
            (HIDE_DURING_GAMES, settings.hide_during_games),
        ] {
            db.set_setting(key, if value { "1" } else { "0" })
                .map_err(|e| e.to_string())?;
        }
    }
    announce(app);
    Ok(())
}

/// A game started or ended: the window steps aside while games run, and comes back after.
/// The shell remembers whether it was the one that hid the window.
pub fn on_game(app: &AppHandle, any_running: bool) {
    let hide = any_running && app.state::<Tray>().available && flag(app, HIDE_DURING_GAMES, true);
    let _ = app.emit(
        "host:game",
        json!({ "anyRunning": any_running, "hide": hide }),
    );
}
