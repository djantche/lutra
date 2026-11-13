# Lutra

A unified game library and launcher for Linux, in the spirit of [Lutris](https://github.com/lutris/lutris), with a modern, fluid interface you can drive with a mouse, a keyboard or a gamepad.

> *Lutra* is Latin for "otter", a nod to Lutris (*lutris* = "of the otter").

## Features

- **One library** for Steam (local libraries, all disks included), Windows games (Wine / Proton) and native Linux games.
- **Windows games**: Wine prefixes managed by Lutra, Wine and Proton detection (Steam, GE-Proton, `umu-run`), per-game configuration (environment variables, DLLs, esync/fsync, DXVK/VKD3D, FSR, gamescope, GameMode, MangoHud, GPU choice on hybrid laptops...), installers followed from start to finish.
- **Controller driver**, Steam Input style: hot-plugging, dead zones, curves, remapping, conversion to keyboard / mouse, per-game profiles, virtual Xbox 360 or DualShock 4 (HID) pad, rumble, in-game overlay.
- **Artwork**: covers, banners, heroes and logos from Steam and [SteamGridDB](https://www.steamgriddb.com/) (personal API key), cached on disk.
- **Two interfaces**: a modern desktop mode (light / dark theme) and a fullscreen **Big Picture** mode built for gamepads.
- **Notification area**, start minimized, built-in logs, interface in English and French.

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

Prerequisites: Node, [pnpm](https://pnpm.io), Rust (via `rustup`) and Tauri's system dependencies (`webkit2gtk-4.1`, `gtk3`, `librsvg`, `libsoup3`, `base-devel` on Arch).

```sh
pnpm install          # frontend dependencies
pnpm tauri dev        # run the app in development (Vite + Rust)
pnpm build            # type-check and build the frontend
pnpm tauri build      # deb, rpm and AppImage packages

cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
cargo test --manifest-path src-tauri/Cargo.toml
```

### Stack

[Tauri 2](https://tauri.app) (Rust) · React + TypeScript + Vite · SCSS · SQLite · zustand.

The Rust backend (`src-tauri/`) handles everything that touches the system; the React frontend (`src/`) only displays and reacts. Architecture, conventions and decisions are documented in [`AGENTS.md`](AGENTS.md).

## Contributing

Commits are small and focused, written in English. Rust code must pass `cargo fmt` and `cargo clippy`, TypeScript is strict, and every user-facing text goes through i18n (English and French).

## License

[GPL-3.0-or-later](LICENSE), like Lutris.
