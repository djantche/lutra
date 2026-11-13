//! Lutra in the notification area: an icon whose menu opens Lutra, switches to Big Picture or
//! quits, so the window can be closed (or hidden while a game runs) without quitting.
//!
//! Hiding the window is only ever done when a notification area is there to bring it back
//! (a `StatusNotifierWatcher` on the session bus): otherwise Lutra would be out of reach.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Manager, Wry};

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

/// The tray's menu labels, in the interface's language.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayLabels {
    pub open: String,
    pub big_picture: String,
    pub quit: String,
}

/// The icon, and whether a game made Lutra hide its window.
pub struct Tray {
    icon: TrayIcon,
    hidden_for_game: AtomicBool,
    available: bool,
}

/// Whether a notification area is there to show Lutra's icon.
fn host_available() -> bool {
    std::process::Command::new("busctl")
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

pub fn settings(app: &AppHandle) -> TraySettings {
    TraySettings {
        close_to_tray: flag(app, CLOSE_TO_TRAY, true),
        start_in_tray: flag(app, START_IN_TRAY, false),
        hide_during_games: flag(app, HIDE_DURING_GAMES, true),
        available: app.state::<Tray>().available,
    }
}

pub fn save_settings(app: &AppHandle, settings: &TraySettings) -> Result<(), String> {
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
    Ok(())
}

fn menu(app: &AppHandle, labels: &TrayLabels) -> tauri::Result<Menu<Wry>> {
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", &labels.open, true, None::<&str>)?,
            &MenuItem::with_id(app, "big-picture", &labels.big_picture, true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", &labels.quit, true, None::<&str>)?,
        ],
    )
}

/// Brings the window back and to the front.
pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Creates the icon. Called once at start, before the window is shown.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    // Until the interface sends its own words.
    let labels = TrayLabels {
        open: "Lutra".into(),
        big_picture: "Big Picture".into(),
        quit: "Quit".into(),
    };
    let mut builder = TrayIconBuilder::with_id("lutra")
        .tooltip("Lutra")
        .menu(&menu(app, &labels)?)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show(app),
            "big-picture" => {
                show(app);
                let _ = app.emit("tray-big-picture", ());
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let icon = builder.build(app)?;
    let available = host_available();
    if !available {
        applog::info("No notification area on this desktop: Lutra's window is never hidden");
    }
    app.manage(Tray {
        icon,
        hidden_for_game: AtomicBool::new(false),
        available,
    });

    let start_hidden = available
        && (flag(app, START_IN_TRAY, false) || std::env::args().any(|arg| arg == "--minimized"));
    if !start_hidden {
        show(app);
    }
    Ok(())
}

/// The menu in the interface's language.
pub fn set_labels(app: &AppHandle, labels: &TrayLabels) -> Result<(), String> {
    let menu = menu(app, labels).map_err(|e| e.to_string())?;
    app.state::<Tray>()
        .icon
        .set_menu(Some(menu))
        .map_err(|e| e.to_string())
}

/// Whether closing the window should hide it rather than quit.
pub fn closes_to_tray(app: &AppHandle) -> bool {
    app.state::<Tray>().available && flag(app, CLOSE_TO_TRAY, true)
}

/// A game started or ended: the window steps aside while games run, and comes back after.
pub fn on_game(app: &AppHandle, any_running: bool) {
    let tray = app.state::<Tray>();
    if any_running {
        if tray.available
            && flag(app, HIDE_DURING_GAMES, true)
            && let Some(window) = app.get_webview_window("main")
            && window.is_visible().unwrap_or(false)
        {
            let _ = window.hide();
            tray.hidden_for_game.store(true, Ordering::SeqCst);
        }
    } else if tray.hidden_for_game.swap(false, Ordering::SeqCst) {
        show(app);
    }
}
