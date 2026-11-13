# Lutra

A unified game library and launcher for Linux, in the spirit of [Lutris](https://github.com/lutris/lutris), with a modern, fluid interface you can drive with a mouse, a keyboard or a gamepad.

> *Lutra* is Latin for "otter", a nod to Lutris (*lutris* = "of the otter").

## Features

- **One library** for Steam (local libraries, all disks included), Windows games (Wine / Proton) and native Linux games.
- **Windows games**: Wine prefixes managed by Lutra, Wine and Proton detection (Steam, GE-Proton, `umu-run`), per-game configuration (environment variables, DLLs, esync/fsync, DXVK/VKD3D, FSR, gamescope, GameMode, MangoHud, GPU choice on hybrid laptops...), installers followed from start to finish.
- **Controller driver**, Steam Input style: hot-plugging, dead zones, curves, remapping, conversion to keyboard / mouse, per-game profiles, virtual Xbox 360 or DualShock 4 (HID) pad, rumble, in-game overlay.
- **Artwork**: covers, banners, heroes and logos from Steam and [SteamGridDB](https://www.steamgriddb.com/) (personal API key), cached on disk.
- **Two interfaces**: a modern desktop mode (light / dark theme) and a fullscreen **Big Picture** mode built for gamepads.
- **Notification area**, start minimized, built-in logs, interface in English and French, and short console-like interface sounds.

## Install

Packages (AppImage, `.deb`, `.rpm`) are available on the [releases page](https://github.com/djantche/lutra/releases).

```sh
chmod +x Lutra_*.AppImage
./Lutra_*.AppImage
```

Launch options:

| Option | Effect |
| --- | --- |
| `--big-picture` | start directly in Big Picture mode |
| `--minimized` | start minimized in the notification area |

### Controllers

The controller driver needs write access to `/dev/uinput` (udev `uaccess` rule, often already provided by Steam's rules). The virtual HID DualShock 4 also needs `/dev/uhid`. Lutra's **Controllers** screen shows the command to run when one of them is missing.

## Development

Prerequisites: Node, [pnpm](https://pnpm.io) and Rust (via `rustup`).

```sh
pnpm install          # dependencies (Electron included)
pnpm start            # build the interface and the backend, then run the app
pnpm build            # type-check and build the interface only
pnpm dist             # AppImage, deb and rpm packages in release/

cargo fmt --manifest-path backend/Cargo.toml
cargo clippy --manifest-path backend/Cargo.toml --all-targets
cargo test --manifest-path backend/Cargo.toml
```

### Stack

[Electron](https://www.electronjs.org) · Rust · React + TypeScript + Vite · SCSS · SQLite · zustand.

The Rust backend (`backend/`) handles everything that touches the system and runs as a child process of the Electron shell (`electron/`), which shows the React interface (`src/`). Architecture, conventions and decisions are documented in [`AGENTS.md`](AGENTS.md).

## Contributing

Commits are small and focused, written in English. Rust code must pass `cargo fmt` and `cargo clippy`, TypeScript is strict, and every user-facing text goes through i18n (English and French).

## Credits

- Interface sounds: [Interface Sounds](https://opengameart.org/content/interface-sounds) by [Kenney](https://www.kenney.nl), CC0 (`src/assets/sounds/CREDITS.md`).
- Source logos: [Simple Icons](https://simpleicons.org), CC0.
- Controller artwork: see `src/assets/controllers/CREDITS.md`.

## License

[GPL-3.0-or-later](LICENSE), like Lutris.
