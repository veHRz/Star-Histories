# AGENTS.md — Star Histories

> Règles et contexte pour **tout agent IA** qui travaille sur ce dépôt (Claude Code le charge via `CLAUDE.md`).
> Lis ce fichier **en entier** au début de chaque session. Il prime sur tes habitudes et sur ta mémoire.
> Tu peux le faire évoluer quand une règle devient fausse ou incomplète : fais-le dans un commit dédié et signale-le à l'auteur.

---

## 0. En une minute

- **Le jeu** : *Star Histories*, RPG solo **space opera** en 3D, **caméra à la première personne**. Le joueur vit dans la galaxie d'**Astréa** en **425at**, en pleine « Fausse Paix armée ». Il monte en grade, change de gameplay (à pied, pilote, capitaine, marchand, espion…), rejoint des groupes, s'allie, **trahit**. **Le monde évolue tout seul**, et le joueur pèse lourd sur ce qui s'y passe.
- **L'auteur** (veHRz) écrit l'univers (le « canon »). **Toi, tu construis le jeu.** Tu ne réécris pas l'univers.
- **La pile** : Rust **1.99.1**, **Bevy 0.20.1**, physique **avian3d 0.8**, modèles 3D faits dans **Blender 5.2.3 LTS** piloté par **MCP for Blender**.
- **La machine de référence** est le poste de développement Windows (il y en a aussi un sous Linux, §3.1) : i5-12500, 32 Go de RAM, **GPU intégré Intel UHD 770**. Le jeu doit y tenir **60 i/s en 1080p en qualité « Moyenne »** (30 i/s minimum en toutes circonstances). C'est une contrainte de conception, pas un détail de fin de projet.
- **Le plan** : `docs/ROADMAP.md`, jalons M0 → M7, à suivre dans l'ordre. La conception du jeu est dans `docs/GDD.md`.

---

## 1. Règles d'or (non négociables)

1. **Le canon fait foi.** `_context/STAR-HISTORIES-export-IA.md` (+ `star-histories-carte.json`) est la bible. Tu ne contredis jamais un fait canon, un nom propre (accents, articles, tirets compris : « le Garant », « Vorrh-III ») ni un choix d'auteur (§11.2 de l'export).
2. **Aucune invention silencieuse.** Tout fait d'univers absent du canon (personnage nommé important, lieu, date, institution, technologie, événement historique) est une **proposition** : inscris-la dans `docs/propositions.md` et marque-la `status: Proposition` dans les données. Le contenu générique de gameplay (PNJ anonymes, noms tirés au hasard, missions procédurales, objets) est permis s'il ne crée pas de fait d'univers et ne contredit rien.
3. **Les secrets 🔒 ne sont jamais montrés au joueur** (dialogue, codex, UI, journal, infobulle, nouvelles) sans décision écrite de l'auteur. Ils **pilotent** la simulation (la Ligue épargne les convois impériaux, la Fédération arme en silence…) : le joueur peut observer les effets, jamais lire l'explication. Une donnée secrète porte `secret: true` et l'UI la filtre.
4. **Les mystères non tranchés restent ouverts** (origine de l'Essaim : définitivement inconnue ; « moyens flous » de Sarkis ; cache des Golden Sword…). Le jeu ne les résout pas.
5. **`_context/futur.md` n'est pas dans le plan.** N'implémente rien de ce fichier sans demande de l'auteur. Mais l'architecture ne doit jamais l'empêcher (débuts multiples, factions jouables, rôles variés).
6. **Ne te fie pas à ta mémoire de Bevy.** Bevy 0.20 a changé beaucoup d'API depuis ce que tu as appris (voir §4). Avant d'utiliser une API, vérifie-la dans `.reference/bevy-0.20.1/` (exemples et sources) ou dans le code des crates sous `%USERPROFILE%\.cargo\registry\src\` (Linux : `~/.cargo/registry/src/`). Pareil pour toute crate tierce.
7. **Toujours vert.** Avant chaque commit : `cargo fmt`, `cargo lint`, `cargo t` passent, et le jeu démarre sans panique (autopilote, §3.3).
8. **Vérifie ce que tu vois.** Tout changement visible est contrôlé par une capture d'écran de l'autopilote que tu ouvres et regardes. « Ça compile » ne veut pas dire « ça marche ».
9. **Données hors du code.** Lore, textes visibles du joueur, statistiques, listes canon : dans `assets/data/` (RON), jamais en dur dans le Rust.
10. **Ce qui revient à l'auteur lui revient.** Lore, ton, direction artistique, périmètre : note la question dans `docs/QUESTIONS.md`, avance sur une hypothèse **réversible et marquée** comme telle, et continue.
11. **`_context/` est en lecture seule**, sauf `futur.md` où tu peux ajouter des idées dans la section « Suggestions » (sans réécrire le texte de l'auteur). Quand l'auteur met à jour l'export, régénère les données (§8.2).

---

## 2. Où est quoi

| Chemin | Rôle | Qui écrit |
|---|---|---|
| `_context/STAR-HISTORIES-export-IA.md` | Le canon résumé (≈ 1 400 lignes). Sections clés : §0 consignes, §2 règles du monde, §3 géographie, §4 factions, §6 flottes, §9 état du monde en 425at, §10 secrets, §11 vides et choix d'auteur | L'auteur (régénéré depuis son wiki) |
| `_context/star-histories-carte.json` | Coordonnées, zones, populations, hubs de Portails, positions des flottes | L'auteur |
| `_context/description.md` | La demande initiale de l'auteur | L'auteur |
| `_context/futur.md` | Idées futures, **hors plan** | L'auteur (+ section « Suggestions ») |
| `AGENTS.md` | Ce fichier : règles et architecture | Toi, avec prudence |
| `docs/GDD.md` | Conception du jeu (piliers, boucles, systèmes) | Toi + l'auteur |
| `docs/ROADMAP.md` | Jalons et tâches, cases à cocher | Toi (coche au fil de l'eau) |
| `docs/propositions.md` | Registre de tout ce qui a été inventé, à valider | Toi |
| `docs/QUESTIONS.md` | Questions ouvertes pour l'auteur | Toi ; l'auteur y répond |
| `docs/JOURNAL.md` | Journal de bord court, une entrée par session | Toi |
| `docs/prompt-initial.md` | Le prompt qui a lancé le projet | — |
| `.reference/bevy-0.20.1/` | Clone en lecture seule de Bevy v0.20.1 (exemples, sources, guides de migration). Ignoré par Git | Personne |
| `assets/` | Tout ce que le jeu charge à l'exécution | Toi |
| `art/` | Fichiers sources `.blend` (Git LFS) | Toi via Blender |
| `tools/` | Outils hors jeu : import du canon, scripts Blender, scripts de vérification | Toi |

---

## 3. Environnement

### 3.1 Ce qui est installé (vérifié le 03/11/2026)

**Deux postes alternent** sur `main` (procédure dans `docs/reprise-autre-pc.md`) : le poste **Windows** (machine de référence, détaillé ci-dessous) et un poste **Linux** (§3.1 bis). Un seul travaille à la fois : `git pull` en arrivant, tout commité et poussé en partant. Note le poste dans chaque entrée de `docs/JOURNAL.md`. Regarde sur quel système tu tournes avant de choisir une commande.

| Outil | Version | Emplacement / remarque |
|---|---|---|
| Windows 11 Éducation | 26200 | Compte de domaine **non administrateur** : toute installation qui exige l'admin passe par l'auteur |
| Rust (rustup, utilisateur) | 1.99.1, cible `x86_64-pc-windows-msvc` | `%USERPROFILE%\.cargo\bin` ; épinglé par `rust-toolchain.toml` |
| Visual Studio Build Tools 2026 | MSVC 14.51, SDK Windows 10.0.26100 | Requis par la cible MSVC |
| Bevy | 0.20.1 | Via Cargo ; sources de référence dans `.reference/bevy-0.20.1` |
| Blender | 5.2.3 LTS (portable) | `%LOCALAPPDATA%\Programs\Blender Foundation\Blender 5.2\blender.exe` (raccourci dans le menu Démarrer) |
| Addon MCP for Blender | addon 1.8 (protocole 15) | `%APPDATA%\Blender Foundation\Blender\5.2\scripts\addons\blender_mcp.py`, activé |
| Serveur MCP for Blender | 2.2.6 (épinglé) | Lancé par Claude Code via `.mcp.json` (`uvx`), télémétrie désactivée |
| uv / uvx | 0.13.7 | Via winget |
| Git + Git LFS | 2.55 / 3.7.1 | Dépôt `https://github.com/veHRz/Star-Histories` (**public**) |
| Node.js | présent | Non utilisé par le projet |

- **Pas de Python système** (seulement l'alias du Microsoft Store, inutilisable). Pour un script Python : le Python de Blender (`blender --background --python script.py`) ou `uv run --python 3.11 script.py`.
- **Shells** : PowerShell 5.1 (pas de `&&` : utilise `;` ou `if ($?) { }`) et Git Bash. Si `cargo` est introuvable dans un shell, ajoute `$env:USERPROFILE\.cargo\bin` au `PATH`.

### 3.1 bis Le poste Linux

| Outil | Version | Emplacement / remarque |
|---|---|---|
| CachyOS (Arch) | noyau 7.3, Wayland | i9-9900K, 32 Go, **Radeon RX 9070 XT** : bien plus puissant que l'UHD 770, ses mesures de performance **ne valent pas** pour la machine de référence (§10) |
| Rust (rustup, paquet `rustup`) | 1.99.1, cible `x86_64-unknown-linux-gnu` | Épinglé par `rust-toolchain.toml` ; `rust-lld` y est déjà le linker par défaut |
| Blender | 5.2.3 LTS (paquet `blender`) | `/usr/bin/blender` ; données dans `/usr/share/blender/5.2` |
| Addon MCP for Blender | addon 1.8 | `~/.config/blender/5.2/scripts/addons/blender_mcp.py`, activé |
| uv / uvx, Git LFS | paquets `uv`, `git-lfs` | LFS configuré **localement** au dépôt (`git lfs install --local`) |

- **Installation et remise à niveau** : `tools/setup-linux.sh` (rejouable ; demande sudo seulement s'il manque des paquets).
- **Shells** : fish pour l'auteur ; l'outil Bash de Claude Code est un shell POSIX. Le dépôt est cloné en SSH : si `git push` échoue (`Permission denied (publickey)`), la clé n'est pas chargée dans l'agent SSH, c'est à l'auteur de le faire (`ssh-add`).
- Python : `python3` système présent, mais pour les scripts du dépôt utilise comme sous Windows le Python de Blender ou `uv run --python 3.11`.

### 3.2 Commandes

| Commande | Effet |
|---|---|
| `cargo dev` | Lance le jeu en développement (liaison dynamique + outils Bevy). **La commande normale.** |
| `cargo lint` | Clippy sur tout le workspace, avertissements = erreurs |
| `cargo t` | Tous les tests du workspace |
| `cargo fmt` | Formatage |
| `cargo build --release` | Build optimisé (à utiliser pour **mesurer** les performances) |

Ces alias sont dans `.cargo/config.toml`. Ils activent tous `star_histories/dev` : **ne lance pas `cargo build` ou `cargo test` sans cette feature** pendant le développement, sinon Cargo recompile tout Bevy avec un autre jeu de features (plusieurs minutes perdues). Ordres de grandeur mesurés : compilation complète de Bevy ≈ 13 min, premier `cargo lint` ≈ 2,5 min, recompilation du jeu seul quelques secondes. Lance les compilations longues en arrière-plan.

### 3.3 L'autopilote (pour te vérifier toi-même)

Le jeu lit des variables d'environnement (`crates/sh_game/src/dev.rs`) :

| Variable | Effet |
|---|---|
| `SH_EXIT_AFTER_FRAMES=N` | Quitte proprement après N frames (test de fumée : pas de panique) |
| `SH_SCREENSHOT=target/shots/x.png` | Enregistre une capture puis quitte |
| `SH_SCREENSHOT_FRAME=N` | Frame de la capture (défaut 90) |
| `SH_START=<vue>[:<lieu>]` | Démarre directement dans une vue : `galaxy`, `global:<lieu>` (ex. `global:hexis`), `sector:<lieu>` (ex. `sector:test`) ; `menu` = menu principal. Lieu inconnu = arrêt en erreur |
| `SH_PRESET=<préréglage>` | Préréglage graphique : `low`, `medium` (défaut) ou `high` |
| `SH_SHOW_FPS=1` | Garde l'affichage FPS pendant une capture (masqué par défaut pour des images propres ; feature `dev`) |
| `SH_GPU_CLUSTERING=1` | Réactive le clustering GPU des lumières (cassé sur UHD 770, voir §4) |
| `SH_BAKED_LIGHTING=0` | Ignore les lightmaps : éclairage des intérieurs entièrement dynamique (comparaisons de performance) |
| `SH_SPAWN=<id>`, `SH_SPAWN_AT=x,y,z,cap,tangage` | Point d'apparition dans un secteur, ou position libre (degrés), pour les captures |
| `SH_LOOK=x,y,z` | Direction initiale de la caméra en vue Globale (captures du ciel, des astres) |
| `SH_SCENARIO=walk_ship` | Parcours scripté du vaisseau du joueur (sas → cockpit), avec mesure du temps de frame |

À la sortie, le jeu journalise toujours le **temps de frame moyen et maximal** et le nombre de frames au-dessus de 33 ms (hors chargement). `tools/check.ps1` (Windows) et `tools/check.sh` (Linux) lancent un test de fumée de chaque point d'entrée ; ces deux scripts font la même chose, modifie-les **ensemble**.

Exemple (PowerShell) :

```powershell
$env:SH_SCREENSHOT = "target/shots/m1-couloir.png"; cargo dev; Remove-Item Env:SH_SCREENSHOT
```

Exemple (Linux) :

```bash
SH_SCREENSHOT=target/shots/m1-couloir.png cargo dev
```

Puis ouvre l'image avec ton outil de lecture de fichiers et **regarde-la**. Range les captures dans `target/shots/` (non versionné). Étends l'autopilote dès qu'un test manuel revient souvent (scénarios scriptés, entrées simulées, mesures de temps de frame).

---

## 4. Bevy 0.20 : ce qui a changé et les pièges

Ta mémoire contient probablement des API de Bevy 0.14 à 0.17. Bevy 0.20 diffère. Points vérifiés dans les sources locales :

- **Événements bufferisés = `Message`** : `#[derive(Message)]`, `MessageWriter<T>` / `MessageReader<T>`, `app.add_message::<T>()`. `EventWriter`/`EventReader` n'existent plus.
- **Observateurs** : `#[derive(Event)]` / `#[derive(EntityEvent)]`, paramètre `On<T>` dans l'observateur, `commands.trigger(...)`, `.observe(...)`.
- **États** : `States`, `SubStates`, `ComputedStates`, `OnEnter` / `OnExit`, `NextState::set`. Nettoyage automatique : composant **`DespawnOnExit(état)`** (l'ancien `StateScoped` n'existe plus).
- **Scènes BSN** : nouvelle syntaxe déclarative (`bsn!`, `bsn_list!`, `SceneList`, `asset_value(...)`), utilisée par les exemples (`examples/3d/3d_scene.rs`). `commands.spawn((A, B, C))` reste valide. Choisis une approche par domaine et tiens-t'y.
- **Lumières** : `PointLight { shadow_maps_enabled: true, .. }` (plus `shadows_enabled`). Unités photométriques : `DirectionalLight::illuminance` en lux, `PointLight::intensity` en lumens (constantes dans `light_consts::lux` / `light_consts::lumens`). Une `PointLight` par défaut éclaire très peu une scène : règle toujours l'intensité.
- **Requêtes** : `Single<D, F>` comme paramètre ; `query.single()` renvoie un `Result`. Les systèmes peuvent renvoyer `Result` et utiliser `?` : préfère-le à `unwrap()`.
- **Hiérarchie** : macro `children![...]`, composants requis (`#[require(...)]`).
- **Fenêtre** : `CursorOptions` / `CursorGrabMode` pour capturer la souris ; `AccumulatedMouseMotion` pour la visée.
- **Rendu utile** : `VisibilityRange` (LOD par distance), `OcclusionCulling`, `DistanceFog`, `Skybox`, `Atmosphere`, `Bloom`, `Lightmap` (éclairage précalculé, idéal pour le GPU intégré), captures `Screenshot`.
- **Outils** : `bevy_camera_controller` (`FreeCamera`, `PanOrbitCamera`) pour le debug ; widgets UI « feathers » (dont champ de texte) dans `bevy_ui_widgets` / `bevy_feathers`.
- **Features** : profils `2d`, `3d`, `ui`, `audio` ; collection `dev`. Liste dans `.reference/bevy-0.20.1/docs/cargo_features.md`.
- **Guides de migration et notes de version** : `.reference/bevy-0.20.1/_release-content/`.

Pièges rencontrés dans ce dépôt (M0) :

- **Clustering GPU des lumières cassé sur la machine de référence** (UHD 770, Vulkan) : les `PointLight` n'éclairent rien. `graphics.rs` force le clustering CPU (`GlobalClusterSettings::gpu_clustering = None`). À revérifier à chaque mise à jour de Bevy (`SH_GPU_CLUSTERING=1`).
- **Caméra du monde HDR + caméra d'interface séparée** : elles n'ont pas la même texture intermédiaire ; l'interface écrase la 3D. La caméra d'interface rend sur fond transparent et se **mélange** (`CameraOutputMode::Write { blend_state: Some(PREMULTIPLIED_ALPHA_BLENDING), .. }`), géré par `graphics::sync_ui_clear`.
- **Exposition** : la valeur par défaut vise un extérieur ensoleillé ; en intérieur, `Exposure::INDOOR` sur la caméra.
- **Police par défaut sans accents** : `ui::setup_fonts` remplace la police d'identifiant par défaut par Inter (`assets/fonts`). Bevy ne charge que TTF et OTF.
- **`bevy_enhanced_input` 0.27** : `actions!` et `bindings!` sont dépréciés (refusés par `cargo lint`) : contextes et liaisons en **BSN** (`commands.spawn_scene(bsn! { … })`), voir `input.rs`.
- **Conflits de requêtes** : un filtre `Added<T>` lit `T` ; il entre en conflit avec un `&mut T` dans un autre paramètre. Utiliser `Mut::is_added()`.
- **Boutons** : `bevy::ui_widgets::{Button, ActivateOnPress, Activate}` (observateur `On<Activate>`), survol par `On<PointerOver>` / `On<PointerOut>`, désactivation par `InteractionDisabled`.
- `RenderTarget` est un composant séparé de `Camera` ; `ClearColorConfig` et `RenderTarget` n'implémentent pas `PartialEq` (comparer avec `matches!`).
- **glTF (M1)** : une scène s'instancie avec `WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(…)))` (l'ancien `SceneRoot`) ; l'instance prête déclenche `On<WorldInstanceReady>` sur la racine ; les extras d'un nœud arrivent en JSON dans `GltfExtras { value }` ; le maillage d'un nœud est souvent sur ses enfants (primitives).
- **Lightmaps (M1)** : composant `bevy::pbr::Lightmap { image, .. }` sur chaque entité maillage qui a un 2e jeu d'UV (`Mesh::ATTRIBUTE_UV_1`, TEXCOORD_1 du glTF : il faut donc aussi un TEXCOORD_0) ; la valeur lue est multipliée par `StandardMaterial::lightmap_exposure` puis par la couleur diffuse. `PointLight::affects_lightmapped_mesh_diffuse: false` évite de compter deux fois la lumière cuite (le spéculaire reste dynamique) ; `GlobalAmbientLight::affects_lightmapped_meshes` idem pour l'ambiante. Le chargeur EXR de Bevy produit du `Rgba32Float` (non filtrable sur tous les GPU) : les lightmaps sont des **PNG sRGB normalisés** (`tools/blender/lightmap.py`).
- **avian3d 0.8** : `MoveAndSlide` (paramètre système) fait le « collide and slide » d'un contrôleur cinématique ; dans `ShapeHitData`, `normal1` est la normale de la surface touchée. Un collider sans corps rigide s'attache au `RigidBody` ancêtre le plus proche.

Quand une API te surprend, cherche un exemple : `rg -l "NomDeLApi" .reference/bevy-0.20.1/examples`.

---

## 5. Architecture

### 5.1 Structure cible du dépôt

```
Cargo.toml                 workspace + binaire `star_histories` (src/main.rs ne fait que lancer sh_game)
crates/
  sh_core/                 types purs partagés, SANS Bevy : dates (bt/at), cases de grille A–T × 1–13,
                           monnaies (crédit, Wuro), identifiants typés, RNG déterministe
  sh_data/                 schémas des données (serde) + chargement RON + validation (références, unicité,
                           cohérence case ↔ coordonnées, drapeaux secret/proposition). SANS Bevy
  sh_sim/                  simulation macro du monde vivant : headless, déterministe, testée. SANS Bevy
  sh_game/                 tous les plugins Bevy (états, vues, joueur, vaisseaux, PNJ, UI, audio, sauvegarde,
                           pont avec sh_sim, outils de dev)
tools/
  sh_import/               CLI : _context/*.json → assets/data/canon/*.ron (reproductible)
  blender/                 scripts bpy versionnés qui génèrent chaque asset (voir §9)
  fonts/                   conversion de la police Inter (fournie avec Blender) en TTF
  check.ps1 / check.sh     vérification complète (Windows / Linux) : fmt --check, lint, tests, import à jour, fumée de chaque SH_START
  setup-linux.sh           installation d'un poste Linux (paquets, LFS, sources de Bevy, addon MCP)
assets/
  data/canon/              données transcrites du canon (chaque entrée cite sa section source)
  data/propositions/       données inventées, status: Proposition
  data/gameplay/           réglages et contenu générique (équilibrage, préréglages graphiques, horloge…)
  data/lang/               textes visibles du joueur, une langue par fichier (en.ron)
  models/ textures/ audio/ fonts/ shaders/ ui/
art/                       sources .blend (LFS), même arborescence que assets/models
docs/                      conception, feuille de route, registres
```

État après M1 : tout ce qui précède existe (`tools/blender/`, `art/` et `assets/models/` depuis M1 ; lightmaps dans `assets/textures/lightmaps/`). Modules de `sh_game` : `state` (états), `loading` (démarrage, chargement asynchrone et validation), `data` (ressources de données et textes), `ui` (thème, menu), `input` (contextes), `views` (vues de test Galaxie, Globale, Sectorielle), `graphics` (préréglages, échelle de rendu), `dev` (autopilote), `dev_tools` (feature `dev`).

### 5.2 Règles de dépendance

- `sh_core` ← `sh_data` ← `sh_sim` ← `sh_game` ← binaire. Jamais dans l'autre sens.
- `sh_core`, `sh_data`, `sh_sim` **ne dépendent pas de Bevy** : compilation rapide, tests instantanés, simulation exécutable sans fenêtre. `sh_game` les enveloppe (newtypes, `Resource`).
- Un domaine de jeu = un `Plugin` dans son module de `sh_game` (`views::galaxy`, `player`, `ship`, `ui::hud`…). `StarHistoriesPlugin` les agrège.
- Systèmes regroupés en `SystemSet` nommés, ordonnés explicitement. Pas d'ordre implicite entre domaines.
- Logique pure (calculs de dégâts, économie, réputation, trajets) dans des fonctions testables sans `World`.

### 5.3 Dépendances autorisées (compatibilité Bevy 0.20 vérifiée le 03/11/2026)

| Crate | Version | Usage |
|---|---|---|
| `avian3d` | 0.8 | Physique (contrôleur FPS, collisions, vaisseaux) |
| `bevy_enhanced_input` | 0.27 | Entrées par **contextes** (à pied, pilotage, carte, menus) et remappage |
| `bevy_asset_loader` | 0.28 | États de chargement, collections d'assets |
| `bevy-inspector-egui` / `bevy_egui` | 0.38 / 0.43 | Inspecteur de debug, **uniquement derrière la feature `dev`** |
| `serde`, `serde_json`, `ron`, `thiserror` | récentes | Données, erreurs (`serde_json` : seulement `tools/sh_import`) |
| `rand` + `rand_chacha` | récentes | Hasard **déterministe** de la simulation |
| `bevy_hanabi` | 0.20 | Particules GPU (avec parcimonie : GPU intégré) |
| `bevy_kira_audio` | 0.27 | Seulement si l'audio de Bevy ne suffit plus (mixage, flux) |
| `bevy_tweening` | 0.17 | Animations d'UI et de transitions, si utile |

À éviter ou à justifier : `big_space` (seulement 0.14.0-rc pour Bevy 0.20 : préfère l'origine flottante maison du §6.4), `bevy_yarnspinner` (bloqué sur Bevy 0.19 : dialogues maison en RON). Toute nouvelle dépendance : vérifie sur crates.io qu'elle cible `bevy 0.20`, ajoute-la à ce tableau avec sa raison. Versions communes dans `[workspace.dependencies]`.

---

## 6. Le système de vues

L'auteur a imaginé quatre vues pour répartir la charge. Le principe est conservé ; voici la conception retenue.

### 6.1 Les quatre vues

| Vue | Ce qu'elle montre | Mécanique | Entrée / sortie |
|---|---|---|---|
| **Galaxie** | La carte d'Astréa : zones, systèmes, astres, stations, hubs, flottes **connues du joueur** | Carte stratégique, planification de trajets, temps accéléré pendant le voyage | Écran de chargement autorisé vers/depuis Globale |
| **Globale** | Un système stellaire, une planète vue d'orbite, ou un gros vaisseau dans son ensemble | Vol spatial depuis le cockpit, combat spatial, approche, amarrage, choix d'un site d'atterrissage | Chargement autorisé vers/depuis Sectorielle, **masqué** par une séquence (amarrage, descente) |
| **Sectorielle** | Un secteur : quartier d'une planète, section d'un gros vaisseau, **ou tout un petit vaisseau** | Le « niveau » jouable, découpé en **cellules** | Aucune rupture visible avec Locale |
| **Locale** | La pièce ou le couloir où se trouve le joueur, et ses voisins | Pleine qualité : props, lumières, physique active, PNJ complets | Streaming continu dans la vue Sectorielle |

### 6.2 Implémentation

- **Galaxie, Globale, Sectorielle sont des états Bevy** : `AppState { Boot, Loading, MainMenu, InGame }` et un sous-état `ViewLevel { Galaxy, Global, Sector }` actif en `InGame`. Chaque entité d'une vue porte `DespawnOnExit(ViewLevel::…)`. Quitter une vue la décharge entièrement.
- **Locale n'est pas un état : c'est un niveau de détail streamé à l'intérieur de Sectorielle.** Un secteur est un **graphe de cellules** (pièces, couloirs, îlots de rues) reliées par des **portails** (portes, sas, ouvertures, coins de rue).
  - Cellule du joueur + voisines (profondeur configurable, 1 par défaut) : qualité **Locale**.
  - Cellules plus lointaines mais potentiellement visibles : qualité **Sectorielle** (maillage fusionné simplifié, pas de props, pas de physique, PNJ absents ou figurants).
  - Au-delà : rien n'est affiché (les portails fermés servent d'occultation).
  - Chargement **asynchrone et anticipé** des voisines, avec **hystérésis** pour éviter les allers-retours, et un budget par frame : aucune frame au-dessus de 33 ms pendant le streaming.
  - Les portes sont des **portes de chargement** : leur animation d'ouverture couvre la fin d'un chargement si nécessaire.
- **Le vaisseau du joueur est toujours chargé** quand il est à bord, y compris en vue Globale : son intérieur est un petit secteur rattaché au vaisseau. Quand le joueur est à bord, **le vaisseau est le référentiel** : l'intérieur reste immobile autour de l'origine (physique stable), c'est l'extérieur qui bouge. Le joueur peut quitter le siège de pilotage et marcher dans son vaisseau en plein vol.
- **Descripteur de lieu** (`assets/data/…/places/*.ron`) : id, type (système, planète, station, vaisseau, secteur), parent, scène glTF, cellules, portails, points d'apparition, liens de transition. Les cellules et portails sont aussi lisibles depuis les **empties Blender** exportés dans le glTF (§9.3).

### 6.3 Transitions

| Transition | Comment |
|---|---|
| Galaxie → Globale | Arrivée d'un saut ou d'un Portail : écran de chargement (citation, astuce, info publique du lieu) |
| Globale → Sectorielle | Amarrage ou atterrissage : la séquence se joue pendant le chargement en arrière-plan ; écran de chargement court seulement si nécessaire |
| Sectorielle ↔ Locale | Streaming invisible |
| Globale ↔ intérieur du vaisseau du joueur | Aucune transition : déjà chargé |

### 6.4 Échelles et précision

- **Positions de simulation en `f64`** (`DVec3`) ; le rendu travaille en `f32` relatif à une **origine flottante** (le vaisseau du joueur, ou le joueur à pied). Recentrage automatique au-delà d'un seuil.
- **Les systèmes stellaires sont compressés** pour être jouables (distances et tailles de jeu ≠ réalité, vitesse de croisière intra-système). Échelle exacte à fixer en M2 : c'est une **proposition** de gameplay (`docs/propositions.md`), pas un fait d'univers.
- **Carte galactique** : repère canon 1000 × 660 unités, ellipse de centre (500, 330), grille A→T × 1→13 (cases de 50). L'échelle « 1 case ≈ 2 semaines de saut, ≈ 10 unités ≈ 3 jours » est une **déduction non canon** de l'export (§3.1) : utilisable pour l'équilibrage, à ne jamais afficher comme un fait.
- Corps lointains : imposteurs ou maillages très simplifiés ; ciel étoilé généré à partir de la position réelle dans la galaxie (direction du noyau, bras, nébuleuses).

---

## 7. Le monde vivant

### 7.1 Deux couches

- **Macro** (`sh_sim`) : toute la galaxie, abstraite, **pas d'une journée de jeu**. Factions (trésor, production, objectifs, relations publiques et cachées), systèmes et astres (contrôle, population, stabilité, prospérité, production), flottes (composition par classe, état, position, mission, **carburant lourd**, moral), personnages notables (vivant, mort, poste), marchés (prix par lieu, crédit/Wuro, taux d'Hexis), contrats Hexa-Corp, projets (L'Enclume, Le Berceau, Rempart…), calendrier politique (Arène ≈ 426at, fin du mandat de Nyra Maren en 427at).
- **Micro** (`sh_game`) : les entités concrètes de la vue chargée, en temps réel. En entrant dans un lieu, l'état macro est **matérialisé** (vaisseaux, PNJ, marchandises) ; en sortant, ce qui a changé est **réécrit** dans la macro.

### 7.2 Règles de la simulation

- **Déterministe** : graine unique, RNG `ChaCha`, pas fixe, aucune dépendance à l'ordre d'itération d'une `HashMap` (`BTreeMap` ou vecteurs triés). Test : même graine + mêmes commandes ⇒ même histoire.
- **Entrées / sorties explicites** : le joueur agit via des `SimCommand` (convoi détruit, cargaison livrée, information vendue, trahison…) ; la simulation produit des `SimEvent` (batailles, raids, révoltes, prix, nominations) qui alimentent les nouvelles, les missions et les réputations.
- **Décisions des factions** : IA à base d'objectifs et d'utilité, alimentée par les objectifs canon (§4 et §9 de l'export). L'Empire veut écraser l'UGL ; l'UGL reconstruit et affaiblit l'Empire ; la Ligue frappe l'UGL et Tosear mais épargne l'Empire (🔒 financée par lui) ; etc.
- **« Rails souples » du canon** (`assets/data/canon/rails.ron`) : sans intervention du joueur, la simulation suit à peu près les trajectoires connues (la Flotte d'Attaque fédérale reste cachée et n'agit pas avant ≈ 430at, Le Berceau avance…). Le joueur peut les dévier ; la simulation seule, non.
- **Interdits durs en v1** : l'Essaim ne se manifeste pas (aucun événement, aucune attaque) ; aucun secret n'est révélé publiquement ; aucun vaisseau capitale n'apparaît ; aucune désassimilation ne réussit.
- **Connaissance du joueur** (`PlayerKnowledge`) : faits, positions de flottes, prix et entrées de codex **connus du joueur**. La carte galactique a un brouillard d'information. Les secrets n'entrent jamais dans cette base en v1.
- **Temps** : horloge de jeu (date en `at`, jour de l'année). Année de 365 jours, affichage « Day N, 425at ». Rythme du temps dans les vues Locale, Sectorielle et Globale : **×30**, réglable dans les données (validé par l'auteur, P4) ; les voyages hyperspatiaux font passer des jours, simulés à vitesse accélérée.
- **Budget** : un pas journalier < 2 ms ; une année entière en headless < 1 s. Si c'est plus lourd, exécution sur `AsyncComputeTaskPool`.

---

## 8. Données de jeu

### 8.1 Principes

- Format **RON**, schémas dans `sh_data`, validation au chargement **et** dans un test (`cargo t` échoue si une donnée est incohérente).
- **Identifiants** : ASCII en `snake_case`, stables (`concordia`, `vorrh_ii`, `nsu_liberte`). **Noms propres du canon** : exacts, en français, avec accents (ils ne se traduisent pas, voir `docs/QUESTIONS.md` Q11).
- **Langue du jeu : l'anglais** (décision de l'auteur, Q8). Tout texte d'interface passe par une clé de `assets/data/lang/en.ron` ; jamais de chaîne visible en dur dans le Rust.
- Chaque entrée canon porte `source: "§3.3"` (section de l'export) ; chaque entrée inventée porte `status: Proposition` et figure dans `docs/propositions.md` ; chaque fait secret porte `secret: true`.
- Les textes visibles du joueur vivent dans les données (prêts pour une traduction future), jamais dans le code.

### 8.2 Import du canon

- `tools/sh_import` lit `_context/star-histories-carte.json` et produit `assets/data/canon/galaxy.ron` (systèmes, astres, stations, zones, hubs, positions des flottes). Les autres données canon (factions, classes de vaisseaux, flottes, personnages, monnaies) sont transcrites à la main depuis l'export dans `assets/data/canon/`, section par section, avec leur `source`.
- Quand l'auteur régénère l'export : relance l'import, lance `cargo t`, lis le diff des données, signale les changements de canon qui touchent le jeu.
- Les incohérences du wiki (§12 de l'export) ne sont pas à trancher par toi : applique la règle de l'export (« la case canon fait foi ») et note le reste dans `docs/QUESTIONS.md`.

---

## 9. Assets 3D : Blender + MCP

### 9.1 Fonctionnement du MCP

- Le serveur MCP (`.mcp.json`, entrée `blender`) est lancé par Claude Code ; il parle à l'addon dans Blender sur `localhost:9876`. **Blender doit être ouvert avec son interface** (l'addon ne démarre pas son serveur en `--background`) ; il démarre seul à l'ouverture de Blender.
- Lancer Blender : sous Windows `Start-Process "$env:LOCALAPPDATA\Programs\Blender Foundation\Blender 5.2\blender.exe"`, sous Linux `setsid blender >/dev/null 2>&1 &` ; puis vérifie avec l'outil MCP `get_addon_status`.
- Outils : `execute_blender_code` (Python bpy dans le Blender ouvert), `look` (voir le résultat : viewport, caméra, multi-angles, rendu), `get_scene_info`, `search_assets` / `import_asset` (Poly Haven, Sketchfab, Poly Pizza), `generate_3d` (services payants).
- **Sauvegarde le `.blend` avant tout script risqué.** Si le MCP n'est pas disponible dans ta session, tu peux exécuter tes scripts en batch : `blender --background --python tools/blender/<…>.py -- <args>` (pas de `look` dans ce cas : fais un rendu dans un PNG et ouvre-le).

### 9.2 Méthode

1. **Chaque asset est produit par un script versionné** dans `tools/blender/<catégorie>/<asset>.py` : paramétrique, rejouable, sans clic manuel. Le MCP sert à exécuter, regarder (`look`) et itérer ; le script reste la source de vérité.
2. Le script enregistre la source dans `art/<catégorie>/<asset>.blend` et exporte `assets/models/<catégorie>/<asset>.glb` (glTF binaire).
3. Contrôle visuel dans Blender (`look`, plusieurs angles), puis **dans le jeu** (capture d'autopilote).
4. Les kits modulaires (couloirs, salles, portes, consoles) passent avant les pièces uniques : ils permettent de construire vite des secteurs entiers.

### 9.3 Conventions

- **Unités** : 1 unité Blender = **1 mètre**. Grille modulaire des intérieurs : **2 m** (proposition, à fixer en M1).
- **Axes** : Blender est Z-haut, l'export glTF convertit en Y-haut. **L'avant d'un objet pointe vers +Y dans Blender**, ce qui donne −Z dans Bevy (`Transform::forward()`).
- **Origine** : centre de la base pour props et personnages ; centre de masse pour les vaisseaux ; coin de grille pour les modules.
- **Noms dans Blender** (relus par le jeu via les extras glTF, `GltfExtras`) : `*_col` collision simplifiée (non rendue), `*_lod1` / `*_lod2` niveaux de détail, empties `SPAWN_<id>`, `CELL_<id>`, `PORTAL_<celluleA>_<celluleB>`, `SOCKET_<usage>`, `INTERACT_<type>`. Les propriétés personnalisées Blender deviennent des extras glTF : sers-t'en pour les métadonnées (type de porte, faction, interaction).
- **Matériaux** : peu de matériaux PBR partagés (trim sheets, palettes, couleurs de sommets), émissif pour les lumières de décor. Textures 1K par défaut, 2K pour les pièces maîtresses, KTX2 si possible. L'éclairage des intérieurs peut être **précalculé dans Blender** (bake Cycles → `Lightmap` dans Bevy) : c'est le meilleur levier de performance sur GPU intégré.
- **Identités visuelles** (proposition à valider) : Empire = rigueur, classes uniformes, « une flotte pensée comme un marteau » ; UGL = flotte hétérogène, vaisseaux personnels des héros de la Rébellion ; pirates = assemblages, « facture pirate » ; Hexa-Corp = industriel, neutre, efficace. Ton général : space opera inspiré de Star Wars et Star Trek, **sans emprunt** (tout est original). **Direction artistique validée : réaliste stylisé, géométrie simple et lisible.** **Pas de boucliers énergétiques**, jamais : seul le blindage protège.
- **Licences** : Poly Haven (CC0) autorisé ; Sketchfab et Poly Pizza seulement avec une licence compatible, vérifiée. Tout asset externe est consigné dans `assets/CREDITS.md` (source, auteur, licence). `generate_3d` (Hyper3D, Tripo, Meshy, Hunyuan) : services payants et licences propres, **uniquement avec l'accord de l'auteur**.

---

## 10. Budgets de performance (UHD 770, 1080p, préréglage « Moyen »)

| Poste | Budget |
|---|---|
| Temps de frame | ≤ 16,6 ms visé, **jamais > 33 ms** (y compris pendant le streaming) |
| Triangles visibles | ≤ 1,5 M |
| Objets dessinés | ≤ 1 000 (fusionne, instancie, utilise `VisibilityRange`) |
| Ombres | 1 lumière directionnelle (2–3 cascades) + ≤ 2 lumières ponctuelles ombrées à la fois |
| Post-traitement | Bloom et tonemapping oui ; SSAO, SSR, volumétrique désactivés en « Moyen » |
| Anticrénelage | FXAA ou SMAA, pas de MSAA en « Moyen » |
| Mémoire vidéo (partagée) | ≤ 1,5 Go par secteur chargé |
| Physique | Colliders simples (boîtes, capsules, convexes) ; physique active seulement dans les cellules Locales |

- Préréglages **Bas / Moyen / Haut** et échelle de rendu dès M0–M1 ; options de rendu coûteuses (raytracing Solari, géométrie virtuelle) : non.
- Mesure en **release** (`cargo build --release`) avec `FrameTimeDiagnosticsPlugin` ; l'autopilote doit pouvoir journaliser le temps de frame moyen et le pire d'un scénario. Une régression de performance est un bug.
- **Seules les mesures faites sur la machine de référence (poste Windows, UHD 770) comptent.** Sur le poste Linux, implémente et vérifie visuellement les optimisations, mais laisse les critères de performance ouverts et note dans `docs/JOURNAL.md` ce qui reste à mesurer sous Windows. Les chiffres du poste Linux peuvent servir à comparer deux versions entre elles, jamais à valider un budget.

---

## 11. Conception du jeu (résumé)

Détails dans `docs/GDD.md`. Les piliers :

1. **Un monde qui vit sans toi** (simulation macro, nouvelles, flottes qui bougent, prix qui changent).
2. **Tu pèses** (chaque action devient une commande de simulation ; réputations ; chaînes de conséquences).
3. **Plusieurs vies dans une vie** (à pied, pilote, capitaine, marchand, contrebandier, mercenaire, espion ; grades dans les organisations).
4. **Loyauté et trahison** (rejoindre, s'allier, trahir ; les organisations se souviennent ; « la dette est sacrée » chez Tosear, la guilde jaskari se ferme pour toujours à qui escroque un Jaskari).
5. **Brutal et court** (combats létaux à pied comme dans l'espace, pas de boucliers, chaque impact compte).
6. **Le mystère respecté** (les secrets se devinent à leurs effets, jamais par une explication).

**Début de partie en v1 (proposition)** : un seul départ, *capitaine indépendant d'un petit vaisseau marchand à Hexis*, le carrefour neutre qui touche les deux blocs. Les débuts multiples de `futur.md` viendront plus tard ; l'architecture les prévoit (un départ = un fichier de données).

---

## 12. Méthode de travail

### 12.1 La boucle, pour chaque tâche

1. Lis la tâche dans `docs/ROADMAP.md`, les sections utiles de ce fichier, du GDD et de l'export (cherche par mot-clé : `rg -n "Hexis" _context/STAR-HISTORIES-export-IA.md`).
2. Vérifie les API dans `.reference/` avant d'écrire du code Bevy que tu n'as pas déjà vu fonctionner dans ce dépôt.
3. Implémente par petits incréments, avec tests pour la logique pure et la simulation.
4. `cargo fmt` ; `cargo lint` ; `cargo t`.
5. Autopilote : test de fumée + capture, que tu regardes.
6. Mets à jour la roadmap (case cochée), `docs/JOURNAL.md` (2–5 lignes), et si besoin `docs/propositions.md`, `docs/QUESTIONS.md`.
7. Commit.

### 12.2 Git

- Branche `main`. Commits **en français**, format `type(portée): résumé` (`feat`, `fix`, `refactor`, `perf`, `assets`, `data`, `docs`, `test`, `chore`), une tâche par commit.
- **Push à la fin de chaque jalon**, quand l'auteur le demande, **et avant que l'auteur change de poste** (§3.1) : en fin de session, ne laisse rien de non poussé sans le lui dire. Le dépôt est **public** : ne versionne jamais de clé d'API, d'identifiant ni de fichier personnel.
- Binaires (`.blend`, `.glb`, textures, audio, polices) via **Git LFS** (`.gitattributes`) ; surveille le quota LFS du compte GitHub. Jamais de `target/`, de `.reference/`, de `*.blend1`.

### 12.3 Code

- Identifiants Rust en **anglais** (conventions de l'écosystème) ; commentaires, docs, messages de commit et journaux en **français** ; **textes visibles du joueur en anglais**, dans les fichiers de langue (§8.1). Les noms propres du canon restent en français partout, y compris dans les données et l'interface.
- Pas de `unwrap()` / `expect()` dans les systèmes de jeu (sauf invariant de démarrage documenté) : systèmes qui renvoient `Result`, erreurs typées (`thiserror`) dans les crates pures.
- Pas de `unsafe`. Pas d'état global mutable hors `Resource`. Constantes d'équilibrage dans les données, pas dans le code.
- Commente le **pourquoi**, pas le quoi. Doc `//!` en tête de chaque module de domaine.
- Tests : unitaires dans les crates pures ; test de cohérence des données ; test de déterminisme de la simulation ; tests Bevy headless (`App` minimale avec `MinimalPlugins`) pour les systèmes critiques.

### 12.4 Sessions longues et autonomie

- Avance jalon par jalon sans attendre de validation pour les choix techniques. Arrête-toi et demande quand : une installation exige l'admin, un choix d'univers ou de direction artistique structurant se présente, ou une dépense (service payant) serait nécessaire.
- Lance les commandes longues (première compilation, build release) en arrière-plan.
- Si tu bloques sur une API plus de deux essais : lis l'exemple Bevy correspondant plutôt que de deviner.

---

## 13. Interdits (récapitulatif)

- Boucliers énergétiques ; vaisseau capitale existant ; origine de l'Essaim expliquée ; désassimilation réussie ; retour de l'Essaim en v1 ; secret 🔒 affiché au joueur ; idée de `99-Idees/` (annexe B de l'export) présentée comme un fait.
- Contenu de `futur.md` implémenté sans demande.
- Texte de lore en dur dans le code ; nouvelle dépendance non vérifiée pour Bevy 0.20 ; asset sans licence claire ; service payant sans accord.
- Commit qui casse `cargo lint`, `cargo t` ou le démarrage du jeu.
- Modifier `_context/` (hors section « Suggestions » de `futur.md`), versionner des secrets techniques ou des fichiers lourds hors LFS.
