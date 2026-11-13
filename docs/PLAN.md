# Plan de travail (session autonome)

Ordre fixé par les priorités : d'abord que Wine et le natif marchent parfaitement, puis l'émulation, puis l'overlay.
Chaque étape se termine par des tests (`cargo test`, `cargo clippy`, `pnpm build`) ; si quelque chose casse, on corrige avant de continuer.
Cet état d'avancement est mis à jour au fil des étapes.

## 1. Lancement réel (fin du faux chargement)

Aujourd'hui `launchingId` retombe après un minuteur de 3,2 s : c'est du décor.

- Le backend émet `launch-progress { id, stage, step, steps, label }` à chaque étape **réelle** :
  vérification de la config → manette virtuelle → création du préfixe (si absent, détecté par `system.reg`) → démarrage du processus → jeu en cours.
- « Jeu en cours » n'est pas déclaré au `spawn` : on attend que le processus du jeu existe vraiment (`/proc`, l'exécutable apparaît dans une ligne de commande du groupe de processus), ou que le processus lanceur sorte (échec).
- Interface : un anneau de progression circulaire (`LaunchRing`) dont les segments correspondent aux étapes terminées ; la dernière étape (attente de la fenêtre du jeu) est indéterminée, puisqu'on ne sait pas combien de temps elle prend.
- Échec : l'état « lancement » s'arrête immédiatement avec la fin du log en notification.

## 2. Wine / Proton : configurations efficaces (comme Lutris)

- **Détection du matériel** (`hardware.rs`) : GPU présents (Intel / AMD / NVIDIA, laptop hybride), `/dev/ntsync`, version du noyau, GameMode, MangoHud, gamescope, umu.
- **Profil recommandé** calculé à partir de ça, appliqué à l'ajout d'un jeu et proposé en un clic ensuite : sync adapté (ntsync > fsync > esync), `__GL_THREADED_OPTIMIZATIONS`, `__GL_SHADER_DISK_CACHE`, `mesa_glthread`, `WINE_LARGE_ADDRESS_AWARE`, `STAGING_SHARED_MEMORY`, NVAPI si NVIDIA (Proton), offload PRIME vers le GPU choisi.
- **Choix du GPU** dans une liste des GPU réellement détectés (pas de saisie libre).
- Options exposées : limite de FPS (`DXVK_FRAME_RATE`), HUD DXVK, gamescope (résolution / FSR), Wayland natif Proton, HDR, couche graphique DXVK / VKD3D / WineD3D, DXVK-NVAPI.
- **Préfixes** : liste des préfixes existants (Lutra + Steam `compatdata`), avec type (Wine / Proton), architecture et jeux qui l'utilisent. Un préfixe incompatible avec le runner choisi est refusé avec la raison, jamais accepté pour planter plus tard.
- Wine nu : installation de DXVK / VKD3D-Proton dans le préfixe (téléchargement vérifié, copie des DLL, overrides) en une action.

## 3. Installer depuis un exécutable

- `install_from_installer` : crée (ou réutilise) un préfixe, lance l'installeur suivi comme un jeu (log, arrêt possible), puis, à sa fermeture, compare les `.exe` du préfixe avant / après.
- Les candidats sont classés (on écarte `unins*`, redistribuables, crash handlers…), l'utilisateur choisit l'exécutable du jeu, la fiche est créée.

## 4. Jeux natifs Linux

- Nouvelle source `native` : exécutable ELF, script shell, AppImage. Le bit d'exécution manquant est proposé en correction.
- Même configuration de lancement que Wine (arguments, dossier, variables, GameMode, MangoHud, GPU, gamescope), sans les options propres à Wine.
- Même suivi (temps de jeu, arrêt, logs, manette).

## 5. Logs du jeu

- Les logs sont ceux du **jeu lancé** : sortie standard et d'erreur du processus, plus le log Proton / DXVK quand le mode debug est actif (`PROTON_LOG`, `DXVK_LOG_PATH`).
- Les deux dernières exécutions sont conservées (`<id>.log`, `<id>.prev.log`). Suivi en direct, filtre, couleurs, accessible depuis la fiche du jeu et pendant qu'il tourne.

## 6. Ajout d'un jeu avec métadonnées

- Source : [LizardByte GameDB](https://github.com/LizardByte/GameDB) (IGDB), servie en JSON statique : recherche par préfixe (`buckets/<2 premiers caractères>.json`) puis fiche (`games/<id>.json`) avec nom, résumé, genres, développeur, date de sortie, plateformes, couverture.
- Dans l'ajout, après le choix de l'exécutable : proposition de correspondances (classées par similarité du nom), choix de l'utilisateur, tous les champs restent modifiables (vrai nom, date, développeur, genres, description). Rien n'est écrit sans validation.
- Les champs sont stockés dans la base (colonne `meta_json`) et priment sur toute récupération ultérieure.

## 7. Émulation

Voir [EMULATION.md](EMULATION.md) : décisions, architecture, phases.

## 8. Overlay en jeu

- Ouvert par le bouton Guide (appui long) pendant qu'un jeu tourne, dans le style de l'overlay Big Picture de Steam : colonne de menu avec logo du jeu (Reprendre, Manette, Détails, Logs, Quitter le jeu) et volet de contenu à droite, sur un fond assombri.
- Fenêtre Tauri dédiée, sans bordure, au premier plan. Pendant qu'elle est ouverte, la manette virtuelle du jeu est neutralisée et les actions de navigation vont à l'overlay.

## 9. Big Picture et bouton Guide (retours en cours de route)

- **Guide au relâchement** : l'action part au **relâchement** : en mode Bureau il passe en Big Picture, en Big Picture il ouvre le menu latéral. **Maintenu ~2 s, il éteint la manette** (déconnexion Bluetooth ; sans effet en USB) et rien d'autre ne se déclenche. Le retour au mode Bureau se fait depuis le menu latéral (et F11 au clavier).
- **Menu latéral Big Picture** : les réglages (dont les manettes) quittent la barre du haut et vivent dans un volet latéral qui s'ouvre avec le Guide.
- **Options / Select sur un jeu** : ouvre sa configuration depuis Big Picture, dans un design adapté (grandes lignes, valeurs qui se changent à la manette, pas de champ texte libre quand une liste existe).

## État

| Étape | État |
| --- | --- |
| 1. Lancement réel | fait : événements `launch-progress`, anneau `LaunchRing`, détection du processus via `/proc` (vérifiée avec un vrai Wine) |
| 2. Config Wine / Proton | fait : matériel, profil recommandé, GPU par liste, préfixes sûrs. **Pas fait** : installation de DXVK / VKD3D dans un préfixe Wine nu |
| 3. Installation depuis un exécutable | fait côté Rust (testé) et interface ; l'installeur réel n'a pas été exercé de bout en bout |
| 4. Natif Linux | fait : source `native`, vérification de l'exécutable, mêmes options et logs |
| 5. Logs du jeu | fait : sortie du jeu, exécution précédente, logs Proton / DXVK / VKD3D en debug |
| 6. Métadonnées | fait : GameDB (testé en réel), détails modifiables à l'ajout et dans la config |
| 7. Émulation | fait : cœurs, BIOS, ROM, vignettes, lancement via RetroArch piloté par Lutra (testé avec le vrai RetroArch). Voir EMULATION.md |
| 8. Overlay | fait côté code ; **non vérifié sur le compositeur réel** (fenêtre au-dessus d'un jeu plein écran) |
| 9. Big Picture / Guide | fait : Guide au relâchement, extinction par maintien, menu latéral, réglages d'un jeu |
