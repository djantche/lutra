# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions follow [SemVer](https://semver.org/).

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
