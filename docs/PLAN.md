# Work plan

Priorities: Windows (Wine / Proton) and native games first, then the in-game overlay.
Every step ends with tests (`cargo test`, `cargo clippy`, `pnpm build`); if something breaks, fix it before moving on.
This progress report is updated as steps are completed.

## 1. Real launch (no fake loading)

- The backend emits `launch-progress { id, stage, step, steps, label }` at every **real** step: config check → prefix creation (if missing, detected through `system.reg`) → process start → game running.
- "Game running" is not declared at `spawn`: we wait for the game process to really exist (`/proc`, the executable appears in a command line of the process group), or for the launcher process to exit (failure).
- Interface: a circular progress ring (`LaunchRing`) whose segments match the finished steps; the last step (waiting for the game window) is indeterminate, since we cannot know how long it takes.
- Failure: the "launching" state stops immediately, with the end of the log in a notification.

## 2. Wine / Proton: efficient configurations (like Lutris)

- **Hardware detection** (`hardware.rs`): GPUs present (Intel / AMD / NVIDIA, hybrid laptop), `/dev/ntsync`, kernel version, GameMode, MangoHud, gamescope, umu.
- **Recommended profile** computed from it, applied when a game is added and offered in one click afterwards: suitable sync (ntsync > fsync > esync), `__GL_THREADED_OPTIMIZATIONS`, `__GL_SHADER_DISK_CACHE`, `mesa_glthread`, `WINE_LARGE_ADDRESS_AWARE`, `STAGING_SHARED_MEMORY`, NVAPI on NVIDIA (Proton), PRIME offload to the chosen GPU.
- **GPU choice** from a list of the GPUs really detected (no free input).
- Exposed options: FPS limit (`DXVK_FRAME_RATE`), DXVK HUD, gamescope (resolution / FSR), native Wayland for Proton, HDR, DXVK / VKD3D / WineD3D graphics layer, DXVK-NVAPI.
- **Prefixes**: list of existing prefixes (Lutra + Steam `compatdata`), with type (Wine / Proton), architecture and the games using them. A prefix incompatible with the chosen runner is refused with the reason, never accepted only to crash later.
- Bare Wine: installing DXVK / VKD3D-Proton in the prefix (verified download, DLL copy, overrides) in one action.

## 3. Installing from an executable

- `install_from_installer`: creates (or reuses) a prefix, runs the installer tracked like a game (log, can be stopped), then, when it closes, compares the prefix's `.exe` files before / after.
- Candidates are ranked (`unins*`, redistributables, crash handlers... are dropped), the user picks the game's executable, the entry is created.

## 4. Native Linux games

- New `native` source: ELF executable, shell script, AppImage. A missing execute bit is offered as a fix.
- Same launch configuration as Wine (arguments, folder, variables, GameMode, MangoHud, GPU, gamescope), without Wine-specific options.
- Same tracking (playtime, stop, logs, controller).

## 5. Game logs

- Logs are those of the **launched game**: the process's standard output and error, plus the Proton / DXVK log when debug mode is on (`PROTON_LOG`, `DXVK_LOG_PATH`).
- The last two runs are kept (`<id>.log`, `<id>.prev.log`). Live follow, filter, colors, reachable from the game's page and while it runs.

## 6. Adding a game with metadata

- Source: [LizardByte GameDB](https://github.com/LizardByte/GameDB) (IGDB), served as static JSON: prefix search (`buckets/<first 2 characters>.json`) then entry (`games/<id>.json`) with name, summary, genres, developer, release date, platforms, cover.
- When adding, after choosing the executable: proposed matches (ranked by name similarity), the user's choice, every field stays editable (real name, date, developer, genres, description). Nothing is written without validation.
- Fields are stored in the database (`meta_json` column) and take precedence over any later fetch.

## 7. In-game overlay

- Opened by the Guide button while a game runs, in the style of Steam's Big Picture overlay: menu column with the game's logo (Resume, Controller, Details, Logs, Quit game) and a content pane on the right, over a darkened background.
- Dedicated Tauri window, borderless, on top. While it is open, the game's virtual pad is neutralized and navigation actions go to the overlay.

## 8. Big Picture and the Guide button

- **Guide on release**: the action fires on **release**: in Desktop mode it switches to Big Picture, in Big Picture it opens the side menu. **Held ~2 s, it turns the pad off** (Bluetooth disconnect; no effect over USB) and nothing else fires. Going back to Desktop mode is done from the side menu (and F11 on the keyboard).
- **Big Picture side menu**: settings (controllers included) leave the top bar and live in a side pane opened with the Guide.
- **Options / Select on a game**: opens its configuration from Big Picture, in an adapted design (big rows, values changed with the pad, no free text field when a list exists).

## 9. Removed: emulation

Lutra used to ship an emulation module (libretro cores, BIOS, ROMs, its own player, RetroArch fallback). It was removed to focus on PC games. Games that were added as ROMs are dropped from the database on first start (database version 7).

## Status

| Step | Status |
| --- | --- |
| 1. Real launch | done: `launch-progress` events, `LaunchRing`, process detection through `/proc` (verified with a real Wine) |
| 2. Wine / Proton config | done: hardware, recommended profile, GPU from a list, safe prefixes. **Not done**: installing DXVK / VKD3D in a bare Wine prefix |
| 3. Install from an executable | done on the Rust side (tested) and in the interface; a real installer has not been exercised end to end |
| 4. Native Linux | done: `native` source, executable check, same options and logs |
| 5. Game logs | done: game output, previous run, Proton / DXVK / VKD3D logs in debug |
| 6. Metadata | done: GameDB (tested for real), editable details when adding and in the config |
| 7. Overlay | done on the code side and verified on a real fullscreen game under Hyprland |
| 8. Big Picture / Guide | done: Guide on release, hold to power off, side menu, per-game settings |
