# AGENTS.md

> **Lutra**: from Latin *lutra*, the otter, a nod to Lutris (*lutris* = "of the otter").
> License: GPL-3.0-or-later, like Lutris.
> This file guides agents (and humans) working on the project. Keep it up to date when a decision changes.

## Goal

Build a **unified game library and launcher for Linux**, in the spirit of [Lutris](https://github.com/lutris/lutris), with a **much more modern interface**, focused on **PC games**.

The project must:

- Gather in one library the games coming from several sources.
- Launch those games (native, Wine, Proton) from the interface.
- Offer a polished UX: smooth, animated, usable with mouse, keyboard and gamepad.
- Offer a **plugin system**, whose first major plugin is **controller management** (mapping, per-game profiles, Steam Input style).

### Out of scope (for now)

- Emulation: Lutra used to embed libretro cores and a player; it was removed to focus on PC games. It may come back later as a separate module.
- Lutris-style community install scripts (may come later).
- Replacing Wine, Proton, Legendary, gogdl, etc.: we rely on existing tools and do not reimplement their logic.
- Windows / macOS support: Linux only.

## Game sources

| Source | Approach |
| --- | --- |
| Steam | Reads the local Steam libraries, launches through `steam://` |
| GOG / Epic | Through existing tools (gogdl, legendary, comet...), planned |
| Local Windows games | Manual executables, Wine prefixes and Proton runners |
| Native Linux games | Manual executables, AppImages and scripts |

**Images**: covers, banners, heroes and logos come from Steam (no key) or **SteamGridDB** (API key set by the user, local disk cache).

## Stack

- **App shell**: [Electron](https://www.electronjs.org) (windows, tray icon, dialogs) around a **Rust backend** that runs as a child process (see Architecture)
- **Frontend**: React + TypeScript + Vite
- **Styles**: SCSS (SCSS modules, shared design tokens)
- **Package manager**: pnpm
- **Database**: SQLite
- **Distribution**: AppImage (plus deb and rpm), built with electron-builder
- **i18n**: translatable interface from day one (FR and EN at least)

## Architecture

- **Rust (`backend/`)**: everything that touches the system: source detection and reading, process launching and supervision, file access, local database, network calls (SteamGridDB), gamepad access. It is a standalone program (`lutra-backend`) started by the Electron shell, which talks to it in JSON lines over stdin / stdout (`rpc.rs`): requests `{id, cmd, args}` get `{id, ok}` or `{id, err}`, and events `{event, payload}` flow the other way. Nothing but those lines may ever be written to stdout (logs go to stderr and to the log file). The backend exits when its stdin closes, i.e. when the shell is gone.
- **Commands**: a backend command is a function of `backend/src/lib.rs` marked with a `// @rpc` line. `backend/src/rpc_dispatch.rs` is generated from them: run `python3 backend/tools/gen_rpc.py` after adding, removing or changing one. Arguments arrive by name in camelCase (`showLutra` for `show_lutra`), `AppHandle` and `State<..>` arguments are filled in by the dispatcher. `host.rs` is the small replacement for what Tauri used to provide (shared state, `emit`, `spawn_blocking`).
- **Electron shell (`electron/`)**: `main.cjs` creates the windows (main, in-game overlay), the tray icon and the native dialogs, starts the backend, and forwards its events to the windows; `preload.cjs` exposes the narrow `window.lutra` bridge. Events named `host:*` are for the shell itself (`host:tray`, `host:game`, `host:overlay`, `host:show`), the others are forwarded to the interface. The window shows only once the backend has said whether a notification area exists. Only one Lutra can run (`requestSingleInstanceLock`): a second one brings the first to the front.
- **React (`src/`)**: display and interaction only. No system logic in the frontend. `lib/host.ts` is its only link with the shell (`invoke`, `listen`, window controls, dialogs); the interface is served by the shell under its own `lutra://app/` scheme.
- **Why Electron**: with WebKitGTK (Tauri), animations on the dev machine (hybrid Intel / NVIDIA laptop, Hyprland) showed a stale frame, usually the starting position, just before the final one. Measured by filming the screen: 10 to 22 backward jumps per run in WebKitGTK against 0 to 5 in Chrome and Electron, and no WebKitGTK setting (`WEBKIT_DISABLE_COMPOSITING_MODE`, `WEBKIT_DISABLE_DMABUF_RENDERER`, `__NV_DISABLE_EXPLICIT_SYNC`, forcing the render GPU) fixed it.
- **Modules**: the core is split into internal Rust modules (game sources, controllers, images) behind clear interfaces. The **controllers module** is the first use case and drives the design of the plugin API. The open plugin API (third-party plugins) will come once the internal modules are stable.
- **Sources**: each game source produces `Discovered` entries (see `db.rs`) and calls `sync_source`, so the library stays source-agnostic.
- **Storage**: SQLite (`rusqlite`) for the library, controller profiles and settings.
- **AppImage environment**: the AppImage's `AppRun` exports `PYTHONHOME`, `PYTHONPATH`, `LD_LIBRARY_PATH`, `PERLLIB`, `GST_*`, `GTK_*`... pointing into its own mount. Anything Lutra launches must not inherit them: `umu-run` is a Python script and dies with `No module named 'encodings'`. Every child process goes through `wine::host_env`, which drops every entry under `$APPDIR`.

### Controllers module (`backend/src/controller/`)

Gamepad driver inspired by Steam Input:

1. **Detection** (`device.rs`): any evdev gamepad (`BTN_SOUTH` button + `ABS_X` axis), rescanned every 1.5 s for hot-plugging. Virtual pads are ignored: Lutra's own (`virtual_pad::is_own` registry), any `uinput` device (`device::is_uinput_virtual`: the kernel puts them under `/sys/devices/virtual/input/`, which also covers another Lutra instance, whose registry this process cannot see; without it two instances keep creating virtual pads from each other's) and Steam's (`28de:11ff`).
2. **Reading** (`codes.rs`, `Decoder`): raw events become a normalized `PadState`, by position (South = A/✕). Axis ranges come from the pad's `absinfo`. xpad swaps X/Y, which is fixed on read. **Drift recentering**: a stick within 30 % of the center at connection is considered at rest and its offset is subtracted (a DS4 over Bluetooth was measured at rest at 153 instead of 128, i.e. 20 %, beyond any dead zone). Outside a game, the rest position is measured again when the pad has not been touched for 5 s (`Decoder::settle`). Never in game: a game may ask the player to hold a stick still.
3. **Profile** (`model.rs`, `apply`): radial dead zones, aiming curve, inversion, stick swap, Nintendo layout, button-by-button remapping, rumble. Stored as JSON in the `controller_profiles` table (`default` + one per game).
4. **Output** (`virtual_pad.rs`, `virtual_input.rs`): a `uinput` pad with the exact identity of an Xbox 360 (`045e:028e`, xpad's X/Y order) or a DualShock 4 (`054c:09cc`), and/or a virtual keyboard + mouse. Only changes are written (`codes::diff`). The `auto` choice (default) gives an Xbox 360 to Windows games and the real pad's type to the others. **Never offer the DS4 as the default for a Windows game**: under Wine/Proton it only exists for DirectInput, never for XInput (verified with an XInput probe: no pad seen with the DS4, full pad with the Xbox 360). Steam always presents an Xbox 360, which is why some games work with it and not with a virtual DS4.
   **DS4 over HID** (`virtual_ds4.rs`): when `/dev/uhid` is accessible (udev rule given by the Controllers screen), the virtual DS4 is a real HID device. The kernel's `hid-playstation` driver takes it for a DS4 plugged over USB (`hidraw` node, gamepad, touchpad, sensors). Proton and SDL read Sony pads through `hidraw`: that is what makes games show PlayStation prompts. A `uinput` DS4 has no `hidraw`, and the game only sees a generic XInput pad (Xbox prompts). Lutra itself answers the feature reports the kernel asks for (0x02 calibration, 0x12 address, 0xa3 firmware), sends a report every 8 ms and forwards rumble from report 0x05 to the real pad. Its devices have a `phys` starting with `lutra-virtual`, never read back. Real test: `cargo test -- --ignored hid_dualshock`.
   **Conversion** (`model::convert`, `convert.rs`): besides button → button remapping, a button can become a key or a mouse button (`keys`), and each physical stick can become the mouse, WASD or the arrows (`left_stick` / `right_stick`, `mouse_speed`). Whatever is converted is removed from the virtual pad. The `keyboard` output removes the virtual pad. Key names come from `convert::KEYS` (`controller_keys` command, a single source for the interface).
5. **System driver** (`mod.rs`, `controller_system` setting, on by default): like Steam Input, Lutra takes every pad as soon as it connects, even without a game, and presents the system with a virtual pad tuned by the **default profile** (dead zones, recentered drift, remapping). The touchpad is not captured outside a game (it stays a mouse). The Guide button stays with Lutra. Editing the default profile only recreates the virtual pad when its type changes (otherwise applications would lose it at every cursor move). **When Steam runs** (`steam` process), Lutra steps aside outside a game: Steam reads the pad through `/dev/hidraw` and would otherwise also see Lutra's virtual pad, hence two pads.
   **Session**: when a game starts, each pad is grabbed exclusively (`EVIOCGRAB`, touchpad and sensors of the same device included). It is replaced by the virtual pad, and the game's rumble is forwarded to the real pad. Everything is given back when the game closes. A "Try" mode does the same without a game. A game whose profile is "Unchanged" also opens a session, without a virtual pad, so the system driver leaves it alone.
   **Hiding the real pad from games**: `EVIOCGRAB` does not cover `/dev/hidraw`, where SDL (HIDAPI) and Wine/Proton (winebus) also find Sony, Nintendo, etc. pads. With nothing done, the game sees the pad twice. Like Steam, Lutra gives the games it launches `SDL_GAMECONTROLLER_IGNORE_DEVICES` with the ids of the real pads (honored by SDL, Wine and Proton). The virtual pad therefore never shares the real one's ids: a DS4 presented in front of a DS4 v2 (`054c:09cc`) takes the v1's (`054c:05c4`) (`virtual_pad::identity`). Applications launched outside Lutra can still open the `hidraw`: same limit as Steam, and only root could avoid it.
6. **Menus** (`nav.rs`): without a running game (system driver included), the pad sends the interface actions (`ui-action`, with repeat while a direction is held), only if the Lutra window has focus. The **Guide button acts on release** (`GuideWatcher`): short press = `menu` (Desktop → Big Picture, or Big Picture's side menu), held 2 s = `powerOff` (Bluetooth disconnect of the pad, no effect over USB). Options and Select give `options` (settings of the selected item).
7. **In-game overlay**: during a game, the Guide is removed from the virtual pad (the game never sees it). A short press opens or closes the overlay (`NavAction::Overlay`): the backend sends `host:overlay` and the shell opens a transparent, frameless, always-on-top window (`?overlay=1`), then calls `overlay_shown` with its process id so the backend can find it in Hyprland. Under Hyprland (`hyprland.rs`), a fullscreen game hides every other window and a window opened by Lutra lands on Lutra's workspace: the overlay is therefore made **floating and pinned** (the only case drawn above a fullscreen window), placed on the game's screen. `hyprctl dispatch` takes Lua expressions in Hyprland 0.56 (`hl.dsp.window.pin({ action = 'enable', window = 'address:0x…' })`); the old `focuswindow title:…` syntax is silently rejected. Verified on a real fullscreen game with a screenshot. While it is open, the game gets a neutral pad and navigation actions go to the overlay (`src/overlay/`).

Pad visuals: `src/assets/controllers/` holds front-facing renders on a transparent background (`playstation.webp` for now). `ControllerPhoto` displays them and overlays the live state at coordinates measured on the image itself. Families without a render keep the SVG diagram of `GamepadVisual`. To add a pad you need a strictly front view with no pressed button, then measure its zones by drawing them over the image.

Steam games are not handled by the driver: Steam Input already takes care of them. Prerequisite: `/dev/uinput` writable (udev `uaccess` rule, which Steam's rules often provide). The Controllers screen gives the command if it is not. Tests on the real system: `cargo test -- --ignored controller`.

These points are intentions: they are refined as decisions are made, and this section is updated.

## Notification area

The icon and its menu (open, Big Picture, quit) belong to the shell (`electron/main.cjs`), in the interface language (`set_tray_labels`, answered by the shell itself). `backend/src/tray.rs` keeps the settings and tells the shell what to do (`host:tray`, `host:game`). Settings: closing the window keeps Lutra open, minimize during games (the window comes back at the end), start minimized (or `lutra --minimized`). Nothing is ever hidden without a `StatusNotifierWatcher` on the session bus, otherwise Lutra would be unreachable. The main window is created invisible and shown by the shell once the backend has said whether it may start hidden.

## Conventions

- **Language**: everything is in **English**: documentation, code, identifiers, comments and commit messages. UI texts go through i18n (FR and EN at least), never hard-coded.
- **TypeScript**: strict mode, no `any` without justification.
- **Rust**: clean `cargo fmt` and `cargo clippy`.
- **SCSS**: centralized variables / design tokens (colors, spacing, radii, shadows, animations), light and dark themes.
- **Accessibility**: full keyboard and gamepad navigation from the start, not as a retrofit.
- **Split into components**: any reusable piece of interface, or big enough to have a role of its own, becomes a component (a `components/<Name>/` folder with `.tsx` and `.module.scss`), and a complex screen is split into sections. Without overdoing it: no component for a single `div` used once, no layers that serve nothing. Before writing markup, check that an existing component does not already do the job.
- **No native browser controls**: never `<select>`, `<input type="checkbox">`, or a default-styled field. Use `Select`, `Switch`, `TextField`, `SegmentedControl`, `FilePicker`, `Dialog`, `FormField` (or create a new one if the need does not exist). A form component handles its own keyboard and focus.
- **Focus**: never the browser's rectangular outline. Focus follows the rounded shape of the element (`--focus-ring` in `_tokens.scss`, `box-shadow`); each component can redefine it. An `<input>` carries no ring: its rounded frame lights up (`:focus-within`).
- **State**: `zustand` for the shared library state (`state/library.ts`). Components subscribe to the slice they need, and an update only replaces the game concerned (`upsertGame`), so a single card redraws. Cards are memoized (`memo`). The backend pushes its changes through events.
- **Right click**: the webview's menu (Reload, Inspect...) never shows. Text fields keep the native menu (copy/paste); everywhere else, `useContextMenu` opens the app's menu (`ContextMenuHost`, one per shell). A game has its menu through `useGameMenu`, and a page background through `usePageMenu`. Any new relevant clickable area should offer a consistent menu.
- **Visible errors**: an action that fails does not settle for a `console.error`. Call `reportError`, which shows a notification to the user.
- **Performance**: the interface must stay smooth on modest machines.
  - Avoid `backdrop-filter`: a more opaque background does the same job for much less.
  - `will-change` only on large single elements animated continuously (Big Picture backgrounds, banners), never on repeated elements (cards, tiles), because every layer is expensive.
  - No expensive drawing hidden under an image: `GameArt` only renders its generated visual until the real image is loaded.
  - A game list always goes through `VirtualGrid` (grids and lists, desktop and Big Picture): only the visible rows are mounted. A big library can reach thousands of games, each card also loading its image.
  - Updates pushed by the backend (`game-updated`, one per artwork found) are batched every 300 ms (`LibraryEffects`, `upsertGames`): never a new library render per game.
- **Local images**: never a file path directly in an `<img>` (the page cannot read the disk). Go through `useArtworkUrl`, which reads the file through the `read_artwork` command (bytes travel as base64, `{"$bytes": ...}`).
- **No side lists in menus**: sections of a settings page or of a game's settings are a `TabBar` (left / right to change section, down to enter the content), never a vertical list next to the content, which is awkward to cross with a keyboard or a pad. The desktop mode's main sidebar and Big Picture's side menu are navigation between pages, not sections of a menu.
- **Menus are lists of rows**: in a settings page, navigation (keyboard, pad) lands on a whole row (`SettingsRow`, `data-nav-row`), never on the small control inside it, like the menu of a game. Confirm acts on the row's control (a switch toggles, a list opens, a field takes focus; Escape gives focus back to the row), and a row holding several controls keeps one stop per control. Up and down leave single-line text fields. Down on the last item scrolls to the end of the page. Rows in Big Picture are outlined in white (`--row-focus`).
- **Sounds**: `lib/sounds.ts` plays a few short sounds (`move`, `confirm`, `back`, `on`, `off`, `open`, `close`, `error`) loaded at start with the Web Audio API. They are played where the action happens: `input/navigation.ts` (selection moves, confirm), `input/bus.ts` (a screen took back / menu / tab), `Switch`, `Dialog` and `reportError`. The user can turn them off (`lutra.sounds` in `localStorage`). Files and credits are in `src/assets/sounds/` (Kenney, CC0); Electron starts with `autoplay-policy=no-user-gesture-required` so they play without a click.
- **Selection styles follow the last input**: after a click, a focus given by a script is not `:focus-visible` to the browser, so styles that must show a keyboard or pad selection also match `[data-input="nav"]` (set by `input/navigation.ts`, cleared on a pointer press).
- **Intuitive UX**: never show help about keys (arrows, Enter, etc.). Only exception: the controller test screen, where you leave by holding a button (progress ring), since all buttons are captured there. The interface must be understandable by itself: mouse, keyboard and gamepad work everywhere. In Big Picture, pointing at an element selects it (`hoverFocus`) and the cursor hides when not needed.
- **Two visual identities**: desktop mode follows the codes of a modern desktop app (integrated title bar, light/dark theme). Big Picture mode follows those of a console (fullscreen, always dark, game artwork as background, big selectable elements).
- **Commits**: small and focused, one subject at a time.
- Make minimal changes consistent with the existing code; do not add a dependency without a reason.

## Commands

Prerequisites: Node, pnpm and Rust (via `rustup`).

```sh
pnpm install          # install front dependencies
pnpm start            # build the interface and the backend, then run the app
pnpm build            # type-check and build the front only
pnpm dist             # produce the packages in release/ (AppImage, deb, rpm)
python3 backend/tools/gen_rpc.py   # after changing a backend command
cargo fmt --manifest-path backend/Cargo.toml
cargo clippy --manifest-path backend/Cargo.toml --all-targets
cargo test --manifest-path backend/Cargo.toml
```

Front lint and tests: to be added.

## Structure

- `src/`: React frontend. `styles/` for SCSS tokens and global styles, `i18n/` for translations (`locales/en.json`, `locales/fr.json`), component styles in `*.module.scss`.
  - `shells/classic/` and `shells/bigpicture/`: the two interface modes. They share the same data (`state/`), components (`components/`) and common views (`views/`); only the layout changes.
  - `input/`: input layer. The UI never reads the keyboard or the gamepad directly: it reacts to **actions** (`up`, `confirm`, `back`...) through `useInputAction`. The keyboard and the controllers module (Rust, which sends the same actions through events) are the sources. Every navigable element carries `data-focusable`, a modal overlay carries `data-nav-trap`.
  - `data/mockGames.ts`: fake games for the browser-only mode.
- Settings (`views/SettingsView.tsx`): one category at a time (General, Windows games, Artwork, Logs, About), chosen in a `TabBar` (a row of tabs with the content under it). Each section is made of named groups, never the tab name repeated. Another page can open a specific category (`section`).
- Big Picture: top bar (Home, Library) and **side menu** (`BigPictureMenu`, opened by the Guide) for controllers, settings and the way back to the desktop. Options / Select on a game opens `GameSettings` (reuses the desktop tabs, automatic saving).
- Direct launch in Big Picture: `lutra --big-picture` (backend command `launch_mode`, the shell passes the option on). F11 switches between the two modes.
- `backend/`: Rust backend (crate `lutra`, lib `lutra_lib`, binary `lutra-backend`). `electron/`: the shell. `build/`: icons used for the packages.
  - See also, first of all: `hardware.rs` (GPUs, including hybrid laptops, ntsync, gamescope; PRIME offload environment), `session.rs` (real tracking of a launch through `/proc`: the game's process carries its `.exe` name in `comm`), `prefixes.rs` (prefix types, runner ⇄ prefix compatibility, Lutra + Steam list), `installer.rs` (comparison of a prefix's `.exe` files before/after an installer), `gamedb.rs` (LizardByte GameDB: prefix search then IGDB entry).
  - Launch: `launch_game` emits `launch-progress` (real steps: check, start, prefix creation if missing, waiting for the game process); the interface shows a ring whose every arc is a step (`LaunchRing`), never a timer. Native games (`native`) go through the same mechanics. A game's logs are those of the launched game: process output, previous run (`.prev.log`), and Proton / DXVK / VKD3D logs in debug mode (`<id>.d/`).
  - GPU on a hybrid machine (`hardware::gpu_env`): besides PRIME offload, Vulkan only sees the chosen GPU (`MESA_VK_DEVICE_SELECT` + `MESA_VK_DEVICE_SELECT_FORCE_DEFAULT_DEVICE=1`, also verified in the steamrt4 container). Without it, DXVK/VKD3D present the iGPU as a second Direct3D card, and some games pick it (white screen of The Walking Dead: The Telltale Definitive Series). A game with no chosen GPU ("Automatic") or whose GPU has disappeared gets `preferred_gpu` (the dedicated card). Only Steam games escape it: Steam launches them itself.
  - Launch configs (`WineConfig`): GPU chosen from the list of detected GPUs, FPS limit, DXVK HUD, ntsync, NVAPI, Wayland, FSR, gamescope, driver tweaks; `recommended_config` sets them from the hardware. The proposed values always come from what is detected (list, never free input when it can be avoided).
  - `vdf.rs`: parser of Valve's text format. `steam.rs`: reads the local Steam libraries (all disks), ignores tools (Proton, runtimes) and reads playtime from `localconfig.vdf`. `db.rs`: SQLite (`~/.local/share/dev.lutra.app/library.db`), which keeps favorites and launches between scans.
  - `wine.rs`: detects Wine and Proton (Steam and `compatibilitytools.d`, hence GE-Proton) and builds the launch command. Proton goes through `umu-run` if installed, otherwise through the `proton` script. `steamgriddb.rs`: SteamGridDB API client (covers in `600x900`, banners in `1920x620`), images saved in `<data>/images/`.
  - Backend commands: `list_games`, `scan_sources`, `set_favorite`, `launch_game` (Steam through `steam://rungameid/<id>`, Wine games tracked until they close to count playtime, logs in `<data>/logs/`), `launch_mode`, `list_runners`, `add_game`, `remove_game` (with `everything`: prefix, logs and controller profile too), `removable_prefix`, `art_settings`, `set_art_settings`, `save_steamgriddb_key`, `read_artwork`, `import_images`.
  - Every game also has a **logo** (`logo_path`), shown instead of its name by `GameLogo`, always in a fixed-size box specific to each screen (`--logo-width`, `--logo-height`) so a very wide logo never overflows: Steam's for a Steam game with official images, otherwise SteamGridDB's (white first, the banner being darkened). The logo is looked up separately from covers (`logo_checked_at`), so a game that already has its images still gets its logo.
  - Steam games' images come by default from Steam (no key), or from SteamGridDB if the user picks "Custom" (with optional fallback to Steam). Games added by hand always use SteamGridDB.
  - The backend notifies the frontend through events (`library-changed`, `game-updated`, `images-progress`) when something changes without a user action.
  - The SteamGridDB key is stored in clear text in the database's `settings` table (the user's data folder). Go through the system keyring if we want better.
  - Windows games: a game's configuration is a JSON (`WineConfig` in `wine.rs`) stored in the `config_json` column. It covers arguments, working directory, environment variables, DLL overrides, esync/fsync, graphics layer (Proton), virtual desktop and 32-bit architecture (Wine), GameMode, MangoHud and debug messages. `build_launch` turns all this into a command and can be tested without launching Wine. Tools (winecfg, winetricks, "run a program in the prefix") reuse the same mechanics. A launched game runs in its own process group so it can be stopped (`stop_game`).
  - Artwork: each game can be matched to a SteamGridDB game chosen by the user (`pick_artwork`, `sgdb_game_id` column). This choice stays active until the user goes back to "automatic".
  - Logs: `applog.rs` writes Lutra's log (`<data>/logs/lutra.log`, moved to `lutra.old.log` past 2 MB). Every notable action (scan, artwork, launch, stop, game exit, saved settings) leaves a line, and interface errors arrive through `log_frontend`. Every Windows game has its launch log (`<data>/logs/<id>.log`) which starts with the exact command, folder and environment variables used. `LogViewer` shows both in the app (filter, colors, live follow while the game runs).
  - Artwork gallery: `artwork_match` gives the SteamGridDB game in use, `artwork_choices` all the proposed images (Steam's for a Steam game, plus SteamGridDB's), `set_artwork` downloads the chosen image. The origin URL of each image is kept (`poster_url`, `hero_url`) to show which one is in place.
  - Wine prefixes live in `<data>/prefixes/<id>` and are never deleted automatically (they may hold saves). Only exception, on request: removing a game offers to delete its prefix too, with its size, when it is a Lutra prefix no other game uses (never a user-chosen folder nor a Steam prefix). The size is measured without following links: a prefix points to the whole file system.
  - To add a source: produce `Discovered` entries (see `db.rs`) and call `sync_source`.
- In a plain browser (`pnpm dev`), `lib/backend.ts` detects the absence of the shell (`lib/host.ts`) and the app runs on the fake games of `data/mockGames.ts`. In the Lutra window, only real games appear.

## Open questions

- Format and security model of the third-party plugin API (to decide after the internal modules).
- Crates for controllers: `evdev` + `uinput` first, `gilrs` possibly for detection; to validate with a prototype.
- Packaging: AppImage first (udev rules cannot be installed by the AppImage itself, a dedicated install step will be needed).
