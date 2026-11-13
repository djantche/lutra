# Lutra

Une bibliothèque de jeux unifiée et un lanceur pour Linux, dans l'esprit de [Lutris](https://github.com/lutris/lutris), avec une interface moderne, fluide et utilisable à la souris, au clavier et à la manette.

> *Lutra* est le mot latin pour « loutre », clin d'œil à Lutris (*lutris* = « de la loutre »).

## Fonctionnalités

- **Une seule bibliothèque** pour Steam (lecture des bibliothèques locales, tous disques confondus), les jeux Windows (Wine / Proton), les jeux natifs Linux et les jeux émulés.
- **Jeux Windows** : préfixes Wine gérés par Lutra, détection de Wine et de Proton (Steam, GE-Proton, `umu-run`), configuration par jeu (variables d'environnement, DLL, esync/fsync, DXVK/VKD3D, FSR, gamescope, GameMode, MangoHud, GPU choisi sur les portables hybrides…).
- **Émulation intégrée** : cœurs libretro installés et mis à jour automatiquement, BIOS, ROM, options par console et par jeu, sauvegardes. Les jeux tournent dans le lecteur de Lutra (RetroArch n'est utilisé que pour les cœurs qui exigent le rendu GPU). Voir [`docs/EMULATION.md`](docs/EMULATION.md).
- **Pilote de manettes** façon Steam Input : détection à chaud, zones mortes, courbes, remappage, conversion en clavier / souris, profils par jeu, manette virtuelle Xbox 360 ou DualShock 4 (HID), vibrations, overlay en jeu.
- **Illustrations** : jaquettes, bannières, héros et logos depuis Steam, [SteamGridDB](https://www.steamgriddb.com/) (clé API personnelle) et libretro, avec cache disque.
- **Deux interfaces** : un mode bureau moderne (thème clair / sombre) et un mode **Big Picture** plein écran pensé pour la manette.
- **Zone de notification**, démarrage réduit, journaux intégrés, interface en français et en anglais.

## Installation

Les paquets (AppImage, `.deb`, `.rpm`) sont disponibles dans les [releases](https://github.com/djantche/lutra/releases).

```sh
chmod +x Lutra_*.AppImage
./Lutra_*.AppImage
```

Options de lancement :

| Option | Effet |
| --- | --- |
| `--big-picture` | démarre directement en mode Big Picture |
| `--minimized` | démarre réduit dans la zone de notification |

### Manettes

Le pilote de manettes a besoin d'un accès en écriture à `/dev/uinput` (règle udev `uaccess`, souvent déjà fournie par les règles de Steam). Pour la DualShock 4 virtuelle en HID, il faut aussi `/dev/uhid`. L'écran **Manettes** de Lutra indique la commande à lancer si l'un d'eux manque.

## Développement

Prérequis : Node, [pnpm](https://pnpm.io), Rust (via `rustup`) et les dépendances système de Tauri (`webkit2gtk-4.1`, `gtk3`, `librsvg`, `libsoup3`, `base-devel` sur Arch).

```sh
pnpm install          # dépendances du frontend
pnpm tauri dev        # app en développement (Vite + Rust)
pnpm build            # vérifie les types et builde le frontend
pnpm tauri build      # paquets deb, rpm et AppImage

cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
cargo test --manifest-path src-tauri/Cargo.toml
```

### Stack

[Tauri 2](https://tauri.app) (Rust) · React + TypeScript + Vite · SCSS · SQLite · zustand.

Le backend Rust (`src-tauri/`) gère tout ce qui touche au système ; le frontend React (`src/`) se limite à l'affichage et à l'interaction. L'architecture détaillée, les conventions et les décisions sont dans [`AGENTS.md`](AGENTS.md).

## Contribuer

Les commits sont petits et ciblés, en anglais. Le code Rust doit passer `cargo fmt` et `cargo clippy`, le TypeScript est en mode strict, et tout texte d'interface passe par l'i18n.

## Licence

[GPL-3.0-or-later](LICENSE), comme Lutris.
