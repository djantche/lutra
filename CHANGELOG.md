# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions follow [SemVer](https://semver.org/).

## [Unreleased]

## [0.6.1] - 2026-11-14

### Added

- **Updates** (Settings > About): Lutra looks at its GitHub releases when it starts and about once a day, tells you once per new version (a message and a system notification), and an AppImage can install the update itself: it downloads the release's AppImage, checks the SHA-256 GitHub published for it, keeps the old file as `<name>.previous` and offers to restart. Other installs (a package, a build from source) are sent to the release page. Checking, notifying, announcing only major versions (while below 1.0, each 0.x counts as one) and skipping a version are all options.
- **Theme** (Settings > General): light, dark, or the system's.
- **Install missing tools** (Settings > Windows games): Wine, umu-launcher, Winetricks, GameMode and MangoHud can be installed from the list through the system's package manager (the password is asked by polkit, never by Lutra), or the command is offered to copy.
- A button to clear the logs, in the log viewer and in Settings > Logs.
- A link to generate the SteamGridDB API key, next to the field.
- `pnpm dev:electron`: Vite's hot reload inside Electron, with a debug backend.

### Changed

- The interface mode setting is now the **default** mode Lutra opens in. Switching during a session no longer changes it.
- Face buttons on the test and remap screens are round badges in the maker's colours instead of sprite images (the sprite pack is gone).
- Settings sliders: confirm gives the slider the focus, left and right adjust it, back returns to its row, up and down move to the neighbouring items.

### Fixed

- X and Y swapped on arcade sticks (and any pad run by `xpad` that is not Microsoft's).
- "Waiting for the game…" unreadable on a game's banner in the light theme.
- What the keyboard or a controller selects is no longer left half under the title bar when the page scrolls to it.

## [0.6.0] - 2026-11-14

### Added

- **Components** in a Windows game's settings (the Wine tab, in both the desktop and Big Picture modes): the Windows libraries games most often ask for can be installed, switched on and off, and taken out, from one list: the shader compiler (`d3dcompiler_47`), the DirectX 9 and 11 libraries, the Visual C++ 2015-2022 runtime, Microsoft's core fonts and XACT audio. A download fills a real progress bar with its percentage; unpacking and registering, which cannot be measured, get an animated bar and their name. A switch turns a component's DLLs on and off without deleting anything. **Remove** puts Wine's own DLLs back, for the components that are plain DLL copies (the shader compiler and the DirectX libraries); the ones that ran an installer cannot be undone cleanly and only get the switch. Nothing changes while the game runs, since its prefix is in use.
- Lutra installs Microsoft's own `d3dcompiler_47` itself, instead of the 2013 build Winetricks offers: it reads that one file (about 2 MB) out of Electron's 158 MB Windows release with range requests, and only accepts the expected size and SHA-256. A prefix that has the old copy is offered an **Update**. With the old one, every vertex shader of some games (Tekken Tag Tournament 2's PC port, for one) failed to compile and fell back to the processor, which left the game at a few frames per second.
- **Arcade sticks** are recognised, whatever their maker: by what they call themselves (fightstick, fighting stick, arcade stick, Hitbox, leverless, Panthera, Qanba...), read from the USB product name, so a stick that appears under two different USB identities is found under both. They get their own line drawing: a lever seen from above that leans and lights up, two rows of four buttons in the Xbox order, and the system buttons. On the test and remap screens, what is pressed lights up like on the DualShock 4.
- **Every other controller gets a standard gamepad drawing** instead of the old schematic, with its buttons named the way its maker names them (A B X Y on an Xbox pad). The sticks move, the D-pad fills the direction pressed, the bumpers and the triggers light up with the buttons.
- **Your own artwork.** In a game's Artwork tab, a cover, a banner or a logo can be an image of your own, from a file or a link. Lutra keeps its own copy, so the original can move or disappear, and tells an image by its content, not by its name or its ending. Logos can now be picked from SteamGridDB too, in a **Logos** tab of the gallery.
- A new application icon.

### Changed

- A controller is shown under the name its maker gave it (its USB product name) when the system only knows it as the driver's made-up "Generic X-Box pad".
- The virtual controller Lutra presents takes USB ids that no connected controller uses. The one made for an arcade stick used to take the ids of a real DualShock 4 beside it, so that one was not hidden from the game, which saw it twice.
- The log only warns that a Windows game "gets a PlayStation 4 controller that XInput cannot see" when that is true (without `/dev/uhid`). The controller Lutra creates normally is read through `hidraw`, which Wine and Proton turn into an XInput pad too.

### Fixed

- A controller that worked at one launch of a game and did nothing at the next, in the menus as well as in games, when two controllers were connected. Lutra gave the same Bluetooth address to all its virtual DualShocks, and the kernel refuses a second one with an address it already has (`Duplicate device found for MAC address`): the one created last had no gamepad, and which one that was changed from a launch to the next. Each virtual DualShock now has its own address.

## [0.5.2] - 2026-11-13

### Changed

- Holding the controller's Guide button for 3 seconds opens a power menu, like a console's: quit Lutra, back to the desktop mode (from Big Picture), minimize, or switch the controller off. It appears even when Lutra is out of sight, and acts on the controller whose button was held. In a game, holding the Guide still switches the controller off directly.

### Fixed

- Switching the controller off now tells the truth. Lutra waits for Bluetooth's answer (it used to fire the command and assume it worked) and shows BlueZ's message if it refuses. A controller plugged in with a cable has no power switch: the menu says to unplug it instead of pretending.

## [0.5.1] - 2026-11-13

### Added

- Two more controllers get their own line drawing: the DualSense (PlayStation 5) and the Steam Deck, picked from the controller's USB ids. On the test and remap screens they light up like the DualShock 4 does: pressed buttons fill, sticks move, triggers fill with their pull. The Steam Deck's buttons are named like an Xbox pad's.

### Changed

- The controller on the Controllers page is a plain picture of the selected controller: it no longer lights up when you press a button (the test screen is for that). The whole card is an item of the page like the settings rows: the keyboard or the pad selects it and confirming opens the test.

### Fixed

- Only one Lutra can run, whatever way it is started. Besides the window lock (a second launch brings the first one to the front, and `lutra --big-picture` switches it to Big Picture), the backend now locks the data folder, so two Lutras can never fight over the controllers and the database, even with different settings folders. A second one says "Lutra is already running" and closes.
- The interface adapts to the window. Lutra asked for a window of at least 960 px, which a tiled window manager can make narrower: the page then laid out wider than the window and its right side was cut off. The minimum is gone, the sidebar becomes a rail of icons in a narrow window, and the home page, the library toolbar, the Controllers page, the Big Picture top bar and the search filters wrap instead of overflowing.

## [0.5.0] - 2026-11-13

### Added

- Lutra downloads Wine and Proton for you. A new **Runners** tab in the settings lists what Lutra has downloaded (with its size and how many games use it, and a way to remove it) and what can be installed: GE-Proton (GloriousEggroll's Proton), Wine and Wine Staging (Kron4ek's builds), straight from their GitHub releases. An install shows its real progress (bytes downloaded with speed and time left, then files extracted), checks the archive's SHA-256 before unpacking, keeps a log, and can be cancelled at any moment without leaving anything behind. It keeps running while you are on another page, and a message tells you how it ended.
- The README now shows screenshots of a real library.
- On a machine with no Wine or Proton, adding a Windows game offers to install the latest GE-Proton right there, and the runner list updates when it is done.

## [0.4.0] - 2026-11-13

### Changed

- Dropdown lists open in a layer above everything, so no window or scrolling area can cut them. They open above the field when there is more room there, follow it while the page scrolls, and a click outside only closes the list instead of also pressing what is underneath.
- The PlayStation controller is now a line drawing of a DualShock 4 (supplied as an SVG) instead of a photo. Pressing a button fills that button on the drawing (the face buttons in their PlayStation colors), the sticks move with the analogue values, and L2 / R2 fill with how far they are pulled.

### Added

- Audio settings, like Steam's: the volume and device of the speakers and of the microphone (read and set through PipeWire / PulseAudio, mute included), and the interface sounds switch, which moved here from the general settings. When the system's sound cannot be read, the section says so instead of showing made-up devices.
- Accessibility settings: the size of the interface (80 % to 150 %), a high contrast mode (light and dark, Big Picture included), and a reduce-motion switch that follows the system until you choose. Everything is kept and applied at start.
- The charge of each controller is also shown next to the clock at the top of Big Picture, with the controller it belongs to (its icon and name, numbered when several are connected).
- Search in Big Picture, like Steam's: press up at the very top of the screen and a search bar opens, with the game list below it and filters (favorites, each source) that show how many games match. Confirming the bar (X on a controller) opens an on-screen keyboard that always spans the screen's width and scales with it; the keyboard follows the interface language (AZERTY for French, QWERTY otherwise).
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
