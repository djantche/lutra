# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Changed

- Dropdown lists open in a layer above everything, so no window or scrolling area can cut them. They open above the field when there is more room there, follow it while the page scrolls, and a click outside only closes the list instead of also pressing what is underneath.
- The PlayStation controller is now a line drawing of a DualShock 4 (supplied as an SVG) instead of a photo. Pressing a button fills that button on the drawing (the face buttons in their PlayStation colors), the sticks move with the analogue values, and L2 / R2 fill with how far they are pulled.

### Added

- The charge of each controller is also shown next to the clock at the top of Big Picture.
- "Minimize" and "Quit Lutra" in the Big Picture menu: Minimize puts Lutra out of sight in the notification area (or minimizes it when there is none).
- While Lutra is in the notification area (or minimized) and no game runs, the Guide button (PlayStation button and equivalents) brings it back, in Big Picture. During a game it still opens the overlay.

### Fixed

- The window did not go to the notification area after a Windows game started: Lutra looked for the game's process only in the launcher's process group, which a Proton container leaves. It now follows everything started by the launch, and after ten seconds accepts a process of the game's name anywhere.

## [0.3.0] - 2026-11-13

### Added

- Interface sounds, short and soft like a console's menus: the cursor moving, confirming, going back, switches, windows opening, errors. They come from Kenney's Interface Sounds (CC0) and can be turned off in the general settings.
- Real logos for the game sources (Steam, GOG, Epic, Wine, Linux) in the sidebar and on the game cards, from Simple Icons (CC0), instead of colored dots.
- A page of its own to remap buttons, laid out like the controller test: the controller in the middle and what each button does on the sides. Click a button, then press the one it should act as on the controller; "Remap all" goes through every button in turn, and Escape or Cancel stops at any moment. A button can also be sent to a key or a mouse button, or disabled.

- The battery of a wireless controller: its charge (and a bolt while charging) next to the controller on the Controllers page, when the system reports it. Lutra warns when it drops to 30 % and again to 15 %, with a message in the app and a system notification that also shows while the window is in the notification area.

### Changed

- A page of options is a centered column on the desktop (equal space left and right) and uses the whole width in Big Picture, instead of sitting on the left.
- Pages always keep a margin under their last row, in Big Picture too, where the last row used to touch the bottom of the screen.
- The Controllers page is a single column, like the settings: the controller and its state on top, then groups of options. The side column is gone.
- The selected tab of a menu is a filled pill (white in Big Picture) instead of an outline plus an underline.
- The selection of a menu row and of a tab shows when it comes from the keyboard or the pad even right after a mouse click.

## [0.2.1] - 2026-11-13

### Changed

- Menus select a whole row, like the menu of a game: up and down move from row to row, Enter (or the controller's confirm button) acts on the row's setting, and Escape leaves a text field back to its row. A row of several choices keeps one stop per choice. The selected row has a white outline in Big Picture.

### Fixed

- The keyboard got stuck in a text field (up and down did nothing there), so the rest of a settings page could not be reached. Up and down now leave a single-line field.
- Down on the last item of a page now scrolls to the end of the page, so what lies past it can be read.
- The choices of a segmented control (such as the controller type a game sees) no longer overflow their card: they wrap onto a second line.

## [0.2.0] - 2026-11-13

### Changed

- Lutra now runs on Electron instead of Tauri. On WebKitGTK, which Tauri uses on Linux, animations could show a stale frame (often the starting position) right before the final one, which looked like shaking; Chromium does not. The Rust backend is unchanged in what it does and now runs as a child process of Electron.
- Only one Lutra can run at a time: launching it again brings the open window to the front.
- Sections of the settings and of a game's settings are now a row of tabs instead of a side list, easier to cross with a keyboard or a gamepad (also in Big Picture).

### Removed

- The "compatibility rendering" setting, which only applied to WebKitGTK.

### Fixed

- Two Lutra instances running at the same time took each other's virtual Xbox 360 pads for real controllers and kept creating more (over a hundred, listed as "Xbox controller" in the Controllers screen). Virtual `uinput` controllers are now recognised system-wide and never driven.

## [0.1.2] - 2026-11-13

### Fixed

- With "minimize during games" on, the window went to the notification area the moment a Windows game was launched, hiding the launch progress. It now waits until the game is really running (and stays visible if the launch fails).

## [0.1.1] - 2026-11-13

### Changed

- New application icon: an otter, on the accent color.
- Lutra now focuses on PC games. Documentation, changelog and README are written in English.

### Removed

- Emulation: libretro cores, BIOS and ROM management, the built-in player, RetroArch support, the Emulation page and settings, and every console-related filter, tab and dialog. Emulated games and their settings are dropped from the library database on first start; saves and downloaded cores stay on disk.

### Fixed

- Windows games failed to start from the AppImage (`Fatal Python error: Failed to import encodings module`). The AppImage's `AppRun` exported `PYTHONHOME`, `PYTHONPATH`, `LD_LIBRARY_PATH` and other variables pointing into its own mount, which `umu-run` (a Python script) inherited. Every process launched by Lutra now gets the environment of the system.

## [0.1.0] - 2026-11-13

First public release.

### Added

- Unified library: Steam (local libraries), Windows games (Wine / Proton), native Linux games and emulated games, with favorites, playtime and filters by source.
- Windows game launching: Lutra-managed Wine prefixes, Wine, Proton and GE-Proton detection, `umu-run`, per-game configuration, GPU choice on hybrid machines, stopping a game, launch logs.
- Built-in emulation: libretro cores installed and updated automatically, BIOS, ROMs, per-console and per-game options, saves, Lutra's own player (RetroArch for cores that need GPU rendering).
- Controller driver: hot-plugging, drift recentering, dead zones, curves, remapping, conversion to keyboard / mouse, default and per-game profiles, virtual Xbox 360 or DualShock 4 (HID) pad, rumble, menu navigation, in-game overlay.
- Artwork from Steam, SteamGridDB and libretro (covers, banners, logos), choice gallery, disk cache.
- Desktop mode (light / dark theme) and fullscreen Big Picture mode, keyboard and gamepad navigation.
- Notification area, start minimized, `--big-picture` option.
- Interface in English and French.
- AppImage, `.deb` and `.rpm` packages.
