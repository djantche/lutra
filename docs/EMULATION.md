# Émulation dans Lutra

Objectif : lancer des jeux de console depuis la bibliothèque, avec **un contrôle complet côté Lutra** (cœurs, BIOS, sauvegardes, options, images), sans réimplémenter les émulateurs.

## Ce qu'est libretro (et ce qu'il n'est pas)

- Un **cœur libretro** est une bibliothèque partagée (`snes9x_libretro.so`) qui émule une console. Elle n'ouvre ni fenêtre ni son : elle expose une petite API C (`retro_load_game`, `retro_run`, des rappels vidéo / audio / entrées).
- Un **frontend** charge un cœur, lui donne le jeu, affiche l'image, joue le son, lit la manette. RetroArch est le frontend de référence ; les cœurs sont écrits pour lui.
- Chaque cœur est décrit par un fichier **`.info`** (nom, système, extensions, **firmware requis**, besoin de chemin complet, base de données). Ces fichiers sont la source de vérité : Lutra ne maintient pas de table à la main des systèmes.
- Les **BIOS** sont des fichiers que certains cœurs exigent (PlayStation, Saturn, GBA…). Ils ne sont jamais fournis par Lutra : l'utilisateur les place dans un dossier, Lutra vérifie leur présence et leur empreinte MD5.
- Les **vignettes** (jaquette, titre, capture) sont publiées par libretro (`thumbnails.libretro.com/<base de données>/Named_Boxarts/<nom>.png`), nommées d'après les noms No-Intro. Aucune clé n'est nécessaire.

## Décision : Lutra gère tout, et fait tourner les jeux lui-même

| Option | Contrôle | Coût | Verdict |
| --- | --- | --- | --- |
| A. RetroArch comme processus hôte, **piloté par Lutra** (config dédiée, cœurs gérés par Lutra) | Cœurs, BIOS, dossiers, options, sauvegardes, manette : tout est décidé par Lutra. Seuls l'affichage et le son restent chez RetroArch. | Faible, utilisable tout de suite | Première étape ; reste pour les cœurs à rendu GPU |
| B. Frontend libretro écrit dans Lutra (`dlopen` des cœurs, vidéo, audio, entrées) | Total, y compris l'overlay en jeu et le rendu | Très élevé : rendu matériel (GL / Vulkan) pour N64, PSP, Dolphin, shaders, latence audio, états, rembobinage | **En place pour les cœurs à rendu logiciel** (étape 1) ; rendu GPU à venir (étape 2) |
| C. Émulateurs autonomes (Dolphin, PCSX2, DuckStation, RPCS3…) | Moyen (leurs propres réglages) | Faible | Pris en charge pour les consoles où ils sont nettement meilleurs |

**Pourquoi A ne vend pas le contrôle** : Lutra n'utilise jamais `~/.config/retroarch`. Chaque lancement passe `--config` (un `retroarch.cfg` écrit par Lutra) et `--appendconfig` (les options du jeu), avec `-L <cœur choisi par Lutra>`. Les dossiers système, sauvegardes, états et captures sont ceux de Lutra. L'utilisateur peut garder son RetroArch intact à côté.

**Comment B reste possible** : tout ce que Lutra décide est derrière une interface (`Emulator`) : « voici un jeu, donne-moi la commande ». Remplacer RetroArch par un hôte maison ne change ni la base, ni les écrans, ni les cœurs, ni les BIOS.

## Architecture (`src-tauri/src/emulation/`)

- `info.rs` : lit les `.info` (cœurs système de `/usr/share/libretro/info`, ou ceux que Lutra télécharge).
- `cores.rs` : inventaire des cœurs, dans l'ordre de préférence **Lutra** (`<données>/emulation/cores/`) puis **système** (`/usr/lib/libretro`). Téléchargement et mise à jour depuis le buildbot libretro (`.index-extended` donne la date et le CRC de chaque cœur : une mise à jour se détecte pour de vrai). Chaque cœur est vérifié par `dlopen` + `retro_api_version()` avant d'être accepté.
- `systems.rs` : les systèmes se déduisent des `.info` (`database`, `supported_extensions`). Une courte table donne seulement le **cœur préféré** par système.
- `roms.rs` : parcours des dossiers de ROM, détection du système par extension et nom de dossier, nom propre (balises `(USA)`, `[!]` retirées).
- `bios.rs` : firmwares demandés par le cœur choisi, présents / manquants / empreinte différente.
- `retroarch.rs` : écrit le `retroarch.cfg` de Lutra et construit la commande (cœurs à rendu GPU seulement).

## Le lecteur de Lutra (`src-tauri/src/player/`)

`lutra --play <config.json>` : le même exécutable, dans un processus à part (un cœur qui plante n'emporte pas la bibliothèque, et arrêter un jeu revient à arrêter un processus). Lutra écrit la config (`PlayConfig` : cœur, ROM, dossiers BIOS / sauvegardes / états, plein écran, reprise, options du cœur) dans `<données>/emulation/play/<jeu>.json`.

- `libretro.rs` : `dlopen` du cœur, ses fonctions, et les rappels (`environment`, image, son, entrées, vibrations). Les options du cœur viennent de `SET_VARIABLES` ; les valeurs du jeu l'emportent sur celles par défaut. Pas de rendu matériel (`SET_HW_RENDER` refusé).
- `video.rs` : OpenGL 3.3 (glutin + glow), l'image en texture, à sa vraie forme, centrée, pixels nets sans scintillement. OpenGL plutôt qu'un simple tampon de pixels : l'étape 2 prêtera ce contexte aux cœurs 3D.
- `audio.rs` : cpal (ALSA, donc PipeWire), rééchantillonnage linéaire et contrôle dynamique du débit (±0,5 %) pour que le tampon reste à 64 ms.
- `input.rs` : les manettes lues dans `/dev/input` (la manette virtuelle que Lutra présente au jeu, celle que le pilote a capturée étant ignorée) et le clavier (flèches, X/Z, S/A, Q/W, Entrée, Maj droite). Échap quitte, F11 bascule le plein écran.
- Cadence : les images sont réglées sur l'horloge de la console (`timing.fps`), pas sur l'écran ; trois images de rattrapage au plus.
- Sauvegardes : la mémoire de sauvegarde (`.srm`) est rechargée au lancement, écrite toutes les 10 s et à la fermeture (SIGTERM compris) ; avec « reprendre », l'état complet (`.state`) aussi. Écriture atomique.
- Choix : un cœur `hw_render` passe par RetroArch, tous les autres par le lecteur.

## Réglages des consoles et des jeux (`profile.rs`)

Un `EmulationProfile` par console (réglage `emulation_profile:<console>`), et par jeu quand l'utilisateur garde des réglages propres à ce jeu (`EmulatorConfig.profile`, une copie de ceux de la console au départ). Au lancement : ceux du jeu, sinon ceux de sa console.

- **Graphismes** (lecteur) : rendu des pixels (nets, lissés, entiers), format (console, pixels carrés, étiré), effet (lignes CRT, grille LCD), dans le shader.
- **Commandes** : pour chaque bouton de la console (nommé comme sur la console, `lib/consoleButtons.ts`), le bouton de la manette qui le déclenche ; le stick gauche peut servir de croix.
- **Options de l'émulateur** : celles que le cœur déclare, lues par `lutra --core-options <cœur> <langue>` (processus à part) via l'API d'options v2 (libellés, explications, catégories, traductions ; v1 et `SET_VARIABLES` aussi). Les catégories vidéo vont dans Graphismes, celles des entrées dans Commandes, le reste dans Émulateur. Seules les valeurs changées sont gardées : les autres suivent le cœur.
- Écrans : « Réglages de la console » sur la page de la console (enregistrés à chaque changement) ; onglets Graphismes / Commandes / Émulateur dans les réglages d'un jeu, avec l'interrupteur « Réglages propres à ce jeu ».
- `thumbnails.rs` : URLs des vignettes libretro à partir du système et du nom.
- `icons.rs` : image de chaque console, tirée des assets de RetroArch quand ils sont installés (`retroarch-assets` ; thème « systematic », qui dessine les consoles, puis « retrosystem »). Les fichiers portent le nom de la base libretro, comme les systèmes de Lutra. `read_artwork` n'accepte que ces PNG en dehors des images de Lutra.

## Interface

- Page **Émulation** : tableau de bord façon EmulationStation (`emulation_systems`). Une tuile par console, regroupée par constructeur, avec son nombre de jeux et son état. Les consoles affichées : « Mes consoles » (avec des jeux), « Jouables » (un cœur installé), « Toutes ».
- Page d'une console : ses jeux, l'émulateur que Lutra utilise et ses BIOS.
- **Lutra choisit l'émulateur, jamais l'utilisateur** : le premier cœur installé de `PREFERRED` pour la console. Il installe celui qui manque (au démarrage pour les consoles de la bibliothèque, après un scan ou un ajout, ou au lancement) et met à jour ceux qu'il a installés (`ensure_cores`). Il n'y a plus de choix par console ni par jeu.
- **Ajouter un jeu → Console** (`add_rom`) : console, fichier filtré sur les extensions des cœurs installés, détails GameDB. Une ROM ajoutée ainsi peut être hors des dossiers de ROM : un scan ne retire que les jeux dont le fichier est dans un dossier scanné ou a disparu.
- Les réglages (dossiers de ROM, BIOS, RetroArch) restent dans Paramètres → Émulation.

## Stockage

- Jeu : `source = 'emulator'`, `exe_path` = ROM, `runner_kind = 'libretro'`, `runner_path` = identifiant du cœur, `config_json` = système, options propres au jeu.
- Réglages (`settings`) : dossiers de ROM, dossier BIOS.
- Fichiers : `<données>/emulation/{cores,system,saves,states,screenshots,retroarch.cfg}`.

## Manette

Pendant un jeu, le module controllers capture la vraie manette et présente une manette virtuelle (Xbox 360 ou DualShock 4) ; le lecteur la lit comme n'importe quelle manette (RetroArch aussi, par son autoconfiguration). Le profil de manette du jeu (zones mortes, remappage) s'applique comme pour un jeu Windows. Le bouton Guide reste à Lutra : il ouvre l'overlay, d'où l'on quitte le jeu (sauvegarde écrite).

## Limites connues

- Étape 2 à faire : rendu matériel (N64, PSP, Dreamcast, GameCube, Wii) dans le lecteur, en prêtant son contexte OpenGL au cœur (`SET_HW_RENDER`). D'ici là, RetroArch.
- Le lecteur n'a pas encore d'états manuels, de rembobinage ni de filtres (CRT) : ils viendront dans l'overlay de Lutra.
- Cœurs lourds (PS2, Wii U, Switch) : émulateurs autonomes.
- Les BIOS ne sont jamais téléchargés par Lutra.

## État

Voir [PLAN.md](PLAN.md), étape 7.
