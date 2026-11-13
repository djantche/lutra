# Changelog

Format inspiré de [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versions selon [SemVer](https://semver.org/lang/fr/).

## [0.1.0] - 2026-11-13

Première version publique.

### Ajouté

- Bibliothèque unifiée : Steam (bibliothèques locales), jeux Windows (Wine / Proton), jeux natifs Linux et jeux émulés, avec favoris, temps de jeu et filtres par source ou par console.
- Lancement des jeux Windows : préfixes Wine gérés par Lutra, détection de Wine, Proton et GE-Proton, `umu-run`, configuration par jeu, choix du GPU sur les machines hybrides, arrêt d'un jeu, journaux de lancement.
- Émulation intégrée : cœurs libretro installés et mis à jour automatiquement, BIOS, ROM, options par console et par jeu, sauvegardes, lecteur propre à Lutra (RetroArch pour les cœurs à rendu GPU).
- Pilote de manettes : détection à chaud, recentrage du drift, zones mortes, courbes, remappage, conversion en clavier / souris, profils par défaut et par jeu, manette virtuelle Xbox 360 ou DualShock 4 (HID), vibrations, navigation dans les menus, overlay en jeu.
- Illustrations depuis Steam, SteamGridDB et libretro (jaquettes, bannières, logos), galerie de choix, cache disque.
- Mode bureau (thème clair / sombre) et mode Big Picture plein écran, navigation clavier et manette.
- Zone de notification, démarrage réduit, option `--big-picture`.
- Interface en français et en anglais.
- Paquets AppImage, `.deb` et `.rpm`.
