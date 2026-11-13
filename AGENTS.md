# AGENTS.md

> **Lutra** : du latin *lutra*, la loutre, clin d'œil à Lutris (*lutris* = « de la loutre »).
> Licence : GPL-3.0-or-later, comme Lutris.
> Ce fichier guide les agents (et les humains) qui travaillent sur le projet. Garde-le à jour quand une décision change.

## Objectif

Créer une **bibliothèque de jeux unifiée et un lanceur pour Linux**, dans l'esprit de [Lutris](https://github.com/lutris/lutris), avec une **interface beaucoup plus moderne**.

Le projet doit :

- Agréger dans une seule bibliothèque les jeux venant de plusieurs sources.
- Lancer ces jeux (natif, Wine, Proton, émulateurs) depuis l'interface.
- Offrir une UX soignée : fluide, animée, utilisable à la souris, au clavier et à la manette.
- Proposer un **système de plugins**, dont le premier plugin majeur est la **gestion des controllers** (mapping, profils par jeu, façon Steam Input).

### Hors périmètre (pour l'instant)

- Scripts d'installation communautaires à la Lutris (peut venir plus tard).
- Remplacer Wine, Proton, Legendary, gogdl, etc. : on s'appuie sur les outils existants, on ne réimplémente pas leur logique.
- Support Windows / macOS : Linux uniquement.

## Sources de jeux visées (à terme)

| Source | Approche prévue |
| --- | --- |
| Steam | Lecture de la bibliothèque locale Steam, lancement via `steam://` |
| GOG / Epic | Via des outils existants (gogdl, legendary, comet...) |
| Jeux locaux Windows | Ajout manuel d'exécutables, gestion des préfixes Wine et des runners Proton |
| Émulateurs / ROMs | RetroArch, Dolphin, PCSX2, etc. |
| Jeux natifs Linux | Ajout manuel ou détection d'exécutables / `.desktop` |

**Images** : couvertures, bannières, héros et logos sont importés depuis **SteamGridDB** (clé API configurée par l'utilisateur, cache disque local).

## Stack

- **Shell applicatif** : [Tauri 2](https://tauri.app) (backend Rust)
- **Frontend** : React + TypeScript + Vite
- **Styles** : SCSS (modules SCSS ou fichiers par composant, tokens de design partagés)
- **Gestionnaire de paquets** : pnpm
- **Base de données** : SQLite
- **Distribution** : AppImage
- **i18n** : interface traduisible dès le départ (FR et EN au minimum)

## Architecture (intentions)

- **Rust (`src-tauri/`)** : tout ce qui touche au système : détection et lecture des sources, lancement et supervision des processus, accès fichiers, base de données locale, appels réseau (SteamGridDB), accès manettes. Exposé au frontend via des commandes et événements Tauri typés.
- **React (`src/`)** : affichage et interaction uniquement. Aucune logique système côté frontend.
- **Modules** : le cœur est découpé en modules Rust internes (sources de jeux, controllers, images) derrière des interfaces claires. Le **module controllers** est le premier cas d'usage et guide la conception de l'API de plugins. L'API de plugins ouverte (plugins tiers) viendra après, une fois les modules internes stabilisés.
- **Sources** : chaque source de jeux implémente une interface commune (`list`, `launch`, métadonnées) pour que la bibliothèque reste agnostique.
- **Stockage** : SQLite (via `rusqlite` ou `sqlx`) pour la bibliothèque, les profils de controllers et les réglages.

### Module controllers (`src-tauri/src/controller/`)

Pilote de manettes inspiré de Steam Input, en place :

1. **Détection** (`device.rs`) : toute manette evdev (bouton `BTN_SOUTH` + axe `ABS_X`), rescannée toutes les 1,5 s pour le branchement à chaud. Les manettes virtuelles de Lutra (registre `virtual_pad::is_own`) et celles de Steam (`28de:11ff`) sont ignorées.
2. **Lecture** (`codes.rs`, `Decoder`) : les événements bruts deviennent un `PadState` normalisé, par position (South = A/✕). Les plages des axes viennent de l'`absinfo` de la manette. xpad inverse X/Y, c'est corrigé à la lecture. **Recentrage du drift** : un stick à moins de 30 % du centre à la connexion est considéré au repos et son décalage est soustrait (une DS4 en Bluetooth mesurée au repos à 153 au lieu de 128, soit 20 %, au-delà de toute zone morte). Hors jeu, le repos est remesuré quand la manette n'est pas touchée pendant 5 s (`Decoder::settle`). Jamais en jeu : un jeu peut demander de tenir un stick immobile.
3. **Profil** (`model.rs`, `apply`) : zones mortes radiales, courbe de visée, inversion, échange des sticks, disposition Nintendo, remappage bouton par bouton, vibrations. Stocké en JSON dans la table `controller_profiles` (`default` + un par jeu).
4. **Sortie** (`virtual_pad.rs`, `virtual_input.rs`) : une manette `uinput` avec l'identité exacte d'une Xbox 360 (`045e:028e`, ordre X/Y de xpad) ou d'une DualShock 4 (`054c:09cc`), et/ou un clavier + souris virtuel. Seuls les changements sont écrits (`codes::diff`). Le choix `auto` (défaut) donne une Xbox 360 aux jeux Windows et le type de la vraie manette aux autres. **Ne jamais proposer la DS4 comme choix par défaut à un jeu Windows** : sous Wine/Proton elle n'existe que pour DirectInput, jamais pour XInput (vérifié avec une sonde XInput : aucune manette vue avec la DS4, manette complète avec la Xbox 360). Steam présente toujours une Xbox 360, d'où des jeux qui marchent avec lui et pas avec une DS4 virtuelle.
   **DS4 en HID** (`virtual_ds4.rs`) : quand `/dev/uhid` est accessible (règle udev donnée par l'écran Manettes), la DS4 virtuelle est un vrai périphérique HID. Le pilote `hid-playstation` du noyau la prend comme une DS4 branchée en USB (nœud `hidraw`, manette, pavé tactile, capteurs). Proton et SDL lisent les manettes Sony par `hidraw` : c'est ce qui fait afficher les touches PlayStation aux jeux. Une DS4 `uinput` n'a pas de `hidraw`, et le jeu ne la voit que comme une manette XInput générique (touches Xbox). Lutra répond lui-même aux rapports de fonctionnalité demandés par le noyau (0x02 calibration, 0x12 adresse, 0xa3 firmware), envoie un rapport toutes les 8 ms et renvoie les vibrations du rapport 0x05 vers la vraie manette. Ses appareils ont un `phys` qui commence par `lutra-virtual`, jamais relus. Test réel : `cargo test -- --ignored hid_dualshock`.
   **Conversion** (`model::convert`, `convert.rs`) : en plus du remappage bouton → bouton, un bouton peut devenir une touche ou un bouton de souris (`keys`), et chaque stick physique peut devenir la souris, ZQSD ou les flèches (`left_stick` / `right_stick`, `mouse_speed`). Ce qui est converti est retiré de la manette virtuelle. La sortie `keyboard` supprime la manette virtuelle. Les noms de touches viennent de `convert::KEYS` (commande `controller_keys`, une seule source pour l'interface).
5. **Pilote système** (`mod.rs`, réglage `controller_system`, actif par défaut) : comme Steam Input, Lutra prend chaque manette dès sa connexion, même sans jeu, et présente au système une manette virtuelle réglée par le **profil par défaut** (zones mortes, drift recentré, remappage). Le pavé tactile n'est pas capturé hors jeu (il reste une souris). Le Guide reste à Lutra. Modifier le profil par défaut ne recrée la manette virtuelle que si son type change (sinon les applications la perdraient à chaque curseur déplacé). **Quand Steam tourne** (processus `steam`), Lutra s'efface hors jeu : Steam lit la manette par `/dev/hidraw` et verrait sinon aussi la manette virtuelle de Lutra, donc deux manettes.
   **Session** : quand un jeu démarre, chaque manette est capturée en exclusif (`EVIOCGRAB`, pavé tactile et capteurs du même appareil compris). Elle est remplacée par la manette virtuelle, et les vibrations du jeu sont renvoyées vers la vraie manette. Tout est rendu à la fermeture du jeu. Un mode « Essayer » fait la même chose sans jeu. Un jeu dont le profil est « Inchangée » ouvre aussi une session, sans manette virtuelle, pour que le pilote système le laisse tranquille.
   **Cacher la vraie manette aux jeux** : `EVIOCGRAB` ne couvre pas `/dev/hidraw`, où SDL (HIDAPI) et Wine/Proton (winebus) trouvent aussi les manettes Sony, Nintendo, etc. Sans rien, le jeu voit la manette en double. Comme Steam, Lutra donne aux jeux qu'il lance `SDL_GAMECONTROLLER_IGNORE_DEVICES` avec les identifiants des vraies manettes (respecté par SDL, Wine et Proton). La manette virtuelle ne partage donc jamais les identifiants de la vraie : une DS4 présentée devant une DS4 v2 (`054c:09cc`) prend ceux de la v1 (`054c:05c4`) (`virtual_pad::identity`). Les applications lancées hors de Lutra peuvent toujours ouvrir le `hidraw` : c'est la même limite que Steam, et seul root pourrait l'éviter.
6. **Menus** (`nav.rs`) : sans jeu en cours (pilote système compris), la manette envoie les actions de l'interface (`ui-action`, avec répétition quand une direction est maintenue), seulement si la fenêtre Lutra a le focus. Le bouton **Guide agit au relâchement** (`GuideWatcher`) : appui court = `menu` (Bureau → Big Picture, ou menu latéral de Big Picture), maintenu 2 s = `powerOff` (déconnexion Bluetooth de la manette, sans effet en USB). Options et Select donnent `options` (réglages de l'élément sélectionné).
7. **Overlay en jeu** : pendant une partie, le Guide est retiré de la manette virtuelle (le jeu ne le voit jamais). Un appui court ouvre ou ferme l'overlay (`NavAction::Overlay`, fenêtre Tauri `overlay`, transparente). Sous Hyprland (`hyprland.rs`), un jeu en plein écran cache toute autre fenêtre et une fenêtre ouverte par Lutra atterrit sur l'espace de travail de Lutra : l'overlay est donc rendu **flottant et épinglé** (le seul cas dessiné au-dessus d'un plein écran), placé sur l'écran du jeu. `hyprctl dispatch` prend des expressions Lua en Hyprland 0.56 (`hl.dsp.window.pin({ action = 'enable', window = 'address:0x…' })`) ; l'ancienne syntaxe `focuswindow title:…` est rejetée sans bruit. Vérifié sur un vrai jeu en plein écran avec une capture d'écran. Tant qu'il est ouvert, le jeu reçoit une manette neutre et les actions de navigation vont à l'overlay (`src/overlay/`).

Visuels des manettes : `src/assets/controllers/` contient des rendus vus de face, sur fond transparent (`playstation.webp` pour l'instant). `ControllerPhoto` les affiche et superpose l'état en direct à des coordonnées mesurées sur l'image elle-même. Les familles sans rendu gardent le schéma SVG de `GamepadVisual`. Pour ajouter une manette, il faut une vue strictement de face, sans bouton enfoncé, puis mesurer ses zones en les dessinant par-dessus l'image.

Les jeux Steam ne sont pas pris en charge par le pilote : Steam Input s'en occupe déjà. Prérequis : `/dev/uinput` accessible en écriture (règle udev `uaccess`, que les règles de Steam fournissent souvent). L'écran Manettes donne la commande si ce n'est pas le cas. Tests sur le vrai système : `cargo test -- --ignored controller`.

Ces points sont des intentions : on les affine au fil des décisions, et on met cette section à jour.

## Émulation

Décisions et architecture dans [`docs/EMULATION.md`](docs/EMULATION.md). En bref : Lutra gère les cœurs libretro (inventaire, téléchargement vérifié par `dlopen`, mises à jour), les BIOS (jamais fournis), les ROM, les options par jeu et les vignettes libretro. **Les jeux tournent dans le lecteur de Lutra** (`src-tauri/src/player/`, `lutra --play <config.json>`, le même exécutable dans un processus à part) : il charge le cœur, ouvre la fenêtre (winit + OpenGL), joue le son (cpal), lit les manettes et tient les sauvegardes. Seuls les cœurs qui dessinent avec le GPU (`hw_render`) passent encore par RetroArch, lancé avec une config écrite par Lutra et jamais celle de `~/.config/retroarch`. Tests réels : `cargo test -- --ignored player` (et `retroarch`). Réglages par console et par jeu (graphismes, commandes, options du cœur) : voir `docs/EMULATION.md`.
- **Zone de notification** (`tray.rs`) : icône avec menu (ouvrir, Big Picture, quitter), dans la langue de l'interface (`set_tray_labels`). Réglages : fermer la fenêtre garde Lutra ouvert, se réduire pendant les parties (la fenêtre revient à la fin), démarrer réduit (ou `lutra --minimized`). Rien n'est jamais caché sans `StatusNotifierWatcher` sur le bus de session, sinon Lutra serait inaccessible. La fenêtre principale est créée invisible (`tauri.conf.json`) et montrée au démarrage par `tray::setup`.

## Conventions

- **Langues** : ce fichier et la documentation interne en français. Code, identifiants, commentaires et messages de commit en **anglais**. Les textes de l'UI passent par l'i18n, jamais en dur.
- **TypeScript** : mode strict, pas de `any` sans justification.
- **Rust** : `cargo fmt` et `cargo clippy` propres.
- **SCSS** : variables / tokens de design centralisés (couleurs, espacements, rayons, ombres, animations), thème clair et sombre.
- **Accessibilité** : navigation clavier et manette complète dès le début, pas en rattrapage.
- **Découper en composants** : toute portion d'interface réutilisable ou assez grosse pour avoir un rôle propre devient un composant (un dossier `components/<Nom>/` avec `.tsx` et `.module.scss`), et un écran complexe est découpé en sections. Sans abuser : pas de composant pour un seul `div` utilisé une fois, ni de couches qui ne servent à rien. Avant d'écrire du balisage, vérifier qu'un composant existant ne fait pas déjà le travail.
- **Pas de contrôles natifs du navigateur** : jamais de `<select>`, `<input type="checkbox">`, ni de champ stylé par défaut. Utiliser `Select`, `Switch`, `TextField`, `SegmentedControl`, `FilePicker`, `Dialog`, `FormField` (ou en créer un nouveau si le besoin n'existe pas). Un composant de formulaire gère son propre clavier et son focus.
- **Focus** : jamais le contour rectangulaire du navigateur. Le focus suit la forme arrondie de l'élément (`--focus-ring` dans `_tokens.scss`, `box-shadow`) ; chaque composant peut le redéfinir. Un `<input>` ne porte pas d'anneau : c'est son cadre arrondi qui s'allume (`:focus-within`).
- **État** : `zustand` pour l'état partagé de la bibliothèque (`state/library.ts`). Les composants s'abonnent à la tranche dont ils ont besoin, et une mise à jour ne remplace que le jeu concerné (`upsertGame`), pour qu'une carte seule se redessine. Les cartes sont mémorisées (`memo`). Le backend pousse ses changements par événements.
- **Clic droit** : le menu du webview (Recharger, Inspecter…) n'apparaît jamais. Les champs texte gardent le menu natif (copier/coller) ; partout ailleurs, `useContextMenu` ouvre le menu de l'app (`ContextMenuHost`, un par shell). Un jeu a son menu via `useGameMenu`, et le fond d'une page via `usePageMenu`. Toute nouvelle zone cliquable pertinente doit proposer un menu cohérent.
- **Erreurs visibles** : une action qui échoue ne se contente pas d'un `console.error`. On appelle `reportError`, qui affiche une notification à l'utilisateur.
- **Performances (WebKitGTK)** : le rendu passe par WebKitGTK, bien moins tolérant que Chromium.
  - Jamais de `backdrop-filter` : il fait scintiller et ralentir, surtout pendant le défilement. Un fond plus opaque fait le même travail.
  - `will-change` seulement sur de grands éléments uniques et animés en continu (fonds Big Picture, bannières), jamais sur des éléments répétés (cartes, tuiles), car chaque calque coûte cher.
  - Pas de dessin coûteux caché sous une image : `GameArt` ne rend son visuel généré que tant que la vraie image n'est pas chargée.
  - Une liste de jeux passe toujours par `VirtualGrid` (grilles et listes, bureau comme Big Picture) : seules les rangées visibles sont montées. Un dossier de ROM peut faire des milliers de jeux (2 895 ROM GBA testées), et WebKitGTK s'arrête de dessiner après quelques rangées si tout est monté, chaque carte chargeant en plus son image.
  - Les mises à jour poussées par le backend (`game-updated`, une par illustration trouvée) sont regroupées toutes les 300 ms (`LibraryEffects`, `upsertGames`) : jamais un nouveau rendu de la bibliothèque par jeu.
  - Le « rendu de compatibilité » (réglage, fichier `<données>/compat-rendering`, lu au démarrage) désactive le moteur DMA-BUF de WebKitGTK. C'est plus lent, mais ça corrige les scintillements sur certains GPU.
- **Images locales** : jamais de chemin de fichier directement dans un `<img>` (le protocole `asset:` n'est pas fiable ici). On passe par `useArtworkUrl`, qui lit le fichier via la commande `read_artwork`.
- **UX intuitive** : jamais d'aide affichée sur les touches (flèches, Entrée, etc.). Seule exception : l'écran de test des manettes, où l'on sort en maintenant un bouton (anneau de progression), puisque tous les boutons y sont capturés. L'interface doit se comprendre seule : souris, clavier et manette marchent partout. En Big Picture, pointer un élément le sélectionne (`hoverFocus`) et le curseur se masque quand il ne sert pas.
- **Deux identités visuelles** : le mode bureau suit les codes d'une app desktop moderne (barre de titre intégrée, thème clair/sombre). Le mode Big Picture suit ceux d'une console (plein écran, toujours sombre, artwork du jeu en fond, gros éléments sélectionnables).
- **Commits** : petits et ciblés, un sujet à la fois.
- Faire des changements minimaux et cohérents avec le code existant ; ne pas ajouter de dépendance sans raison.

## Commandes

Prérequis : Node, pnpm, Rust (via `rustup`) et les dépendances système de Tauri (`webkit2gtk-4.1`, `gtk3`, `librsvg`, `libsoup3`, `base-devel` sur Arch).

```sh
pnpm install          # installer les dépendances front
pnpm tauri dev        # lancer l'app en développement (Vite + Rust)
pnpm build            # vérifier les types et builder le front seul
pnpm tauri build      # produire les paquets (deb, rpm, AppImage)
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
cargo test --manifest-path src-tauri/Cargo.toml
```

Lint front et tests : à ajouter.

## Structure

- `src/` : frontend React. `styles/` pour les tokens et styles globaux SCSS, `i18n/` pour les traductions (`locales/en.json`, `locales/fr.json`), styles de composants en `*.module.scss`.
  - `shells/classic/` et `shells/bigpicture/` : les deux modes d'interface. Ils partagent les mêmes données (`state/`), composants (`components/`) et vues communes (`views/`) ; seule la mise en page change.
  - `input/` : couche d'entrées. L'UI ne lit jamais le clavier ni la manette directement : elle réagit à des **actions** (`up`, `confirm`, `back`...) via `useInputAction`. Le clavier est la seule source pour l'instant ; le module controllers (Rust) enverra les mêmes actions par événements. Tout élément navigable porte `data-focusable`, un overlay modal porte `data-nav-trap`.
  - `data/mockGames.ts` : jeux fictifs, à remplacer par les vraies sources.
- Paramètres (`views/SettingsView.tsx`) : une catégorie à la fois (Général, Jeux Windows, Émulation, Illustrations, Journaux, À propos), choisie dans une `SideNav`. Chaque section est faite de groupes nommés, jamais du nom de l'onglet répété. Une autre page peut ouvrir une catégorie précise (`section`).
- Émulation (`views/emulation/`, page du mode bureau et onglet du Big Picture) : un tableau de bord façon EmulationStation. Chaque console est une tuile (image tirée des assets de RetroArch, thème « systematic », sinon son nom court sur sa couleur), regroupée par constructeur, avec son nombre de jeux et son état (prête, BIOS manquant, émulateur à installer). Une console ouvre sa page : ses jeux, l'émulateur que Lutra utilise pour elle et ses BIOS. **L'utilisateur ne choisit jamais d'émulateur** : Lutra prend le cœur préféré de la console (`systems::PREFERRED`), l'installe tout seul (au démarrage, après un scan ou un ajout de ROM, ou au premier lancement, étape `core` de l'anneau) et met à jour ceux qu'il a installés. Dans la barre latérale (et les filtres du Big Picture), les jeux émulés sont rangés par console (filtre `system:<console>`), jamais en une seule source « Émulateurs ». Noms courts, constructeurs et couleurs : `lib/platforms.ts`. Ajouter un jeu propose aussi « Console » : console, fichier filtré sur ses formats, puis les mêmes détails que les autres jeux.
- Big Picture : barre du haut (Accueil, Bibliothèque, Émulation) et **menu latéral** (`BigPictureMenu`, ouvert par le Guide) pour les manettes, les réglages et le retour au bureau. Options / Select sur un jeu ouvre `GameSettings` (réutilise les onglets du mode bureau, enregistrement automatique).
- Lancement direct en Big Picture : `lutra --big-picture` (commande Tauri `launch_mode`). F11 bascule entre les deux modes.
- `src-tauri/` : backend Rust (crate `lutra`, lib `lutra_lib`) et config Tauri.
  - Voir aussi, en tête de liste : `hardware.rs` (GPU, dont portables hybrides, ntsync, gamescope ; environnement d'offload PRIME), `session.rs` (suivi réel d'un lancement via `/proc` : le processus du jeu porte le nom de son `.exe` dans `comm`), `prefixes.rs` (types de préfixes, compatibilité runner ⇄ préfixe, liste Lutra + Steam), `installer.rs` (comparaison des `.exe` d'un préfixe avant/après un installeur), `gamedb.rs` (LizardByte GameDB : recherche par préfixe puis fiche IGDB), `emulation/` (voir plus haut).
  - Lancement : `launch_game` émet `launch-progress` (étapes réelles : vérification, démarrage, création du préfixe si absent, attente du processus du jeu) ; l'interface affiche un anneau dont chaque arc est une étape (`LaunchRing`), jamais un minuteur. Les jeux natifs (`native`) et émulés (`emulator`) passent par la même mécanique. Les logs d'un jeu sont ceux du jeu lancé : sortie du processus, exécution précédente (`.prev.log`), et logs Proton / DXVK / VKD3D en mode debug (`<id>.d/`).
  - GPU sur une machine hybride (`hardware::gpu_env`) : en plus de l'offload PRIME, Vulkan ne voit que le GPU choisi (`MESA_VK_DEVICE_SELECT` + `MESA_VK_DEVICE_SELECT_FORCE_DEFAULT_DEVICE=1`, vérifié aussi dans le conteneur steamrt4). Sans ça, DXVK/VKD3D présentent l'iGPU comme une seconde carte Direct3D, et certains jeux la choisissent (écran blanc de The Walking Dead: The Telltale Definitive Series). Un jeu sans GPU choisi (« Automatique ») ou dont le GPU a disparu reçoit `preferred_gpu` (la carte dédiée). Les jeux émulés aussi. Seuls les jeux Steam y échappent : Steam les lance lui-même.
  - Configs de lancement (`WineConfig`) : GPU choisi dans la liste des GPU détectés, limite de FPS, HUD DXVK, ntsync, NVAPI, Wayland, FSR, gamescope, tweaks de pilote ; `recommended_config` les règle d'après le matériel. Les valeurs proposées viennent toujours de ce qui est détecté (liste, jamais saisie libre quand on peut l'éviter).
  - `vdf.rs` : parseur du format texte de Valve. `steam.rs` : lit les bibliothèques Steam locales (tous les disques), ignore les outils (Proton, runtimes) et lit le temps de jeu dans `localconfig.vdf`. `db.rs` : SQLite (`~/.local/share/dev.lutra.app/library.db`), qui garde favoris et lancements entre deux scans.
  - `wine.rs` : détecte Wine et Proton (Steam et `compatibilitytools.d`, donc GE-Proton) et construit la commande de lancement. Proton passe par `umu-run` s'il est installé, sinon par le script `proton`. `steamgriddb.rs` : client de l'API SteamGridDB (jaquettes en `600x900`, bannières en `1920x620`), images enregistrées dans `<données>/images/`.
  - Commandes Tauri : `list_games`, `scan_sources`, `set_favorite`, `launch_game` (Steam via `steam://rungameid/<id>`, jeux Wine suivis jusqu'à leur fermeture pour compter le temps de jeu, logs dans `<données>/logs/`), `launch_mode`, `list_runners`, `add_wine_game`, `remove_game` (avec `everything` : préfixe, journaux et profil de manette aussi), `removable_prefix_of`, `art_settings`, `set_art_settings`, `save_steamgriddb_key`, `read_artwork`, `import_images`.
  - Chaque jeu a aussi un **logo** (`logo_path`), affiché à la place de son nom par `GameLogo`, toujours dans une boîte de taille fixe propre à chaque écran (`--logo-width`, `--logo-height`) pour qu'un logo très large ne déborde jamais : celui de Steam pour un jeu Steam en images officielles, sinon celui de SteamGridDB (blanc d'abord, la bannière étant assombrie). Pour un jeu émulé, il vient de libretro (`Named_Logos`), jamais d'une recherche SteamGridDB par ROM. Un jeu émulé garde sa jaquette sur les cartes ; sa capture de gameplay (libretro `Named_Snaps`) sert de bannière. Le logo est cherché à part des jaquettes (`logo_checked_at`), donc un jeu qui a déjà ses images reçoit quand même son logo.
  - Les images des jeux Steam viennent par défaut de Steam (aucune clé), ou de SteamGridDB si l'utilisateur choisit « Personnalisées » (avec repli optionnel sur Steam). Les jeux ajoutés à la main utilisent toujours SteamGridDB.
  - Le backend prévient le frontend par événements (`library-changed`, `game-updated`, `images-progress`) quand quelque chose change sans action de l'utilisateur.
  - La clé SteamGridDB est stockée en clair dans la table `settings` de la base (dossier de données de l'utilisateur). À passer par le trousseau système si on veut mieux.
  - Jeux Windows : la configuration d'un jeu est un JSON (`WineConfig` dans `wine.rs`) stocké dans la colonne `config_json`. Elle couvre les arguments, le dossier de travail, les variables d'environnement, les remplacements de DLL, esync/fsync, la couche graphique (Proton), le bureau virtuel et l'architecture 32 bits (Wine), GameMode, MangoHud et les messages de débogage. `build_launch` transforme tout ça en commande et se teste sans lancer Wine. Les outils (winecfg, winetricks, « lancer un programme dans le préfixe ») réutilisent la même mécanique. Un jeu lancé tourne dans son propre groupe de processus pour pouvoir être arrêté (`stop_game`).
  - Illustrations : chaque jeu peut être associé à un jeu SteamGridDB choisi par l'utilisateur (`pick_artwork`, colonne `sgdb_game_id`). Ce choix reste actif tant que l'utilisateur ne revient pas à « automatique ».
  - Journaux : `applog.rs` écrit le journal de Lutra (`<données>/logs/lutra.log`, déplacé en `lutra.old.log` au-delà de 2 Mo). Toute action notable (scan, illustrations, lancement, arrêt, sortie d'un jeu, réglages enregistrés) y laisse une ligne, et les erreurs de l'interface y arrivent via `log_frontend`. Chaque jeu Windows a son journal de lancement (`<données>/logs/<id>.log`) qui commence par la commande exacte, le dossier et les variables d'environnement utilisés. `LogViewer` affiche les deux dans l'app (filtre, couleurs, suivi en direct pendant que le jeu tourne).
  - Galerie d'illustrations : `artwork_match` donne le jeu SteamGridDB utilisé, `artwork_choices` toutes les images proposées (celles de Steam pour un jeu Steam, plus celles de SteamGridDB), `set_artwork` télécharge l'image choisie. L'URL d'origine de chaque image est gardée (`poster_url`, `hero_url`) pour montrer laquelle est en place.
  - Les préfixes Wine sont dans `<données>/prefixes/<id>` et ne sont jamais supprimés automatiquement (ils peuvent contenir des sauvegardes). Seule exception, sur demande : retirer un jeu propose de supprimer aussi son préfixe, avec sa taille, quand c'est un préfixe de Lutra qu'aucun autre jeu n'utilise (jamais un dossier choisi par l'utilisateur ni un préfixe de Steam). La taille est mesurée sans suivre les liens : un préfixe pointe vers tout le système de fichiers.
  - Pour ajouter une source : produire des `Discovered` (voir `db.rs`) et appeler `sync_source`.
- Dans un navigateur simple (`pnpm dev`), `lib/backend.ts` détecte l'absence de Tauri et l'app tourne sur les jeux fictifs de `data/mockGames.ts`. Dans la fenêtre Tauri, seuls les vrais jeux apparaissent.

## Questions ouvertes

- Format et modèle de sécurité de l'API de plugins tiers (à décider après les modules internes).
- Crates pour les controllers : `evdev` + `uinput` en priorité, `gilrs` éventuellement pour la détection ; à valider par un prototype.
- Packaging : AppImage en priorité (les règles udev ne peuvent pas être installées par l'AppImage elle-même, il faudra une étape d'installation dédiée).
