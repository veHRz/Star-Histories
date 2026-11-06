# AGENTS.md — Star Histories

> Règles et contexte pour **tout agent IA** qui travaille sur ce dépôt (Claude Code le charge via `CLAUDE.md`).
> Lis ce fichier **en entier** au début de chaque session. Il prime sur tes habitudes et sur ta mémoire.
> Tu peux le faire évoluer quand une règle devient fausse ou incomplète : fais-le dans un commit dédié et signale-le à l'auteur.

---

## 0. En une minute

- **Le jeu** : *Star Histories*, RPG solo **space opera** en 3D, **caméra à la première personne**. Le joueur vit dans la galaxie d'**Astréa** en **425at**, en pleine « Fausse Paix armée ». Il monte en grade, change de gameplay (à pied, pilote, capitaine, marchand, espion…), rejoint des groupes, s'allie, **trahit**. **Le monde évolue tout seul**, et le joueur pèse lourd sur ce qui s'y passe.
- **L'auteur** (veHRz) écrit l'univers (le « canon »). **Toi, tu construis le jeu.** Tu ne réécris pas l'univers.
- **La pile** : Rust **1.99.1**, **Bevy 0.20.1**, physique **avian3d 0.8**, modèles 3D faits dans **Blender 5.2.3 LTS** piloté par **MCP for Blender**.
- **La machine de référence** est le poste de développement Windows (il y en a aussi un sous Linux, §3.1) : i5-12500, 32 Go de RAM, **GPU intégré Intel UHD 770**. Le jeu doit y tenir **60 i/s en 1080p en qualité « Basse »** (30 i/s minimum en toutes circonstances ; décision de l'auteur du 04/11/2026, Q15 : cette machine est faible, la cible était « Moyenne » jusqu'à M7). C'est une contrainte de conception, pas un détail de fin de projet.
- **Le plan** : `docs/ROADMAP.md`, jalons M0 → M8, à suivre dans l'ordre (M8, la refonte visuelle et le tutoriel : `docs/refonte-visuelle.md`). La conception du jeu est dans `docs/GDD.md`.

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
| `docs/refonte-visuelle.md` | Plan de M8 : direction artistique « futur usé », contraintes, lots et méthode orchestrateur + sous-agents | Toi |
| `docs/prompt-m8.md` | Le prompt qui lance M8 dans une nouvelle conversation | — |
| `.claude/agents/` | Définitions des sous-agents (`artiste-blender` pour la 3D, `dev-jeu` pour tout le reste) : règles de travail en parallèle | Toi, avec prudence |
| `.claude/skills/orchestrateur/SKILL.md` | Le mode orchestrateur (`/orchestrateur <jalon \| fichier \| tâches>`) : découpage en lots, sous-agents, intégration et tests par vague (§12.5) | Toi, avec prudence |
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
- **Shells** : fish pour l'auteur ; l'outil Bash de Claude Code y lance **zsh** : une variable non citée n'est pas découpée en mots (`env $vars cmd` passe une seule chaîne ; utiliser `eval` ou `${=vars}`), et un motif sans correspondance est une erreur. Le dépôt est cloné en SSH : si `git push` échoue (`Permission denied (publickey)`), la clé n'est pas chargée dans l'agent SSH, c'est à l'auteur de le faire (`ssh-add`).
- Python : `python3` système présent, mais pour les scripts du dépôt utilise comme sous Windows le Python de Blender ou `uv run --python 3.11`.

### 3.2 Commandes

| Commande | Effet |
|---|---|
| `cargo dev` | Lance le jeu en développement (liaison dynamique + outils Bevy). **La commande normale.** |
| `cargo lint` | Clippy sur tout le workspace, avertissements = erreurs |
| `cargo t` | Tous les tests du workspace |
| `cargo fmt` | Formatage |
| `cargo build --release` | Build optimisé (à utiliser pour **mesurer** les performances) |
| `powershell -ExecutionPolicy Bypass -File tools/package.ps1` (`tools/package.sh`) | Build distribuable (`--features dist` : pas de console sous Windows) + assets + notice, dans `target/dist/` |

Ces alias sont dans `.cargo/config.toml`. Ils activent tous `star_histories/dev` : **ne lance pas `cargo build` ou `cargo test` sans cette feature** pendant le développement, sinon Cargo recompile tout Bevy avec un autre jeu de features (plusieurs minutes perdues). Ordres de grandeur mesurés : compilation complète de Bevy ≈ 13 min, premier `cargo lint` ≈ 2,5 min, recompilation du jeu seul quelques secondes. Lance les compilations longues en arrière-plan.

### 3.3 L'autopilote (pour te vérifier toi-même)

Le jeu lit des variables d'environnement (`crates/sh_game/src/dev.rs`) :

| Variable | Effet |
|---|---|
| `SH_EXIT_AFTER_FRAMES=N` | Quitte proprement après N frames (test de fumée : pas de panique) |
| `SH_SCREENSHOT=target/shots/x.png` | Enregistre une capture puis quitte |
| `SH_SCREENSHOT_FRAME=N` | Frame de la capture (défaut 90) |
| `SH_START=<vue>[:<lieu>]` | Démarre directement dans une vue : `galaxy`, `global:<lieu>` (ex. `global:hexis`), `sector:<lieu>` (ex. `sector:corvette` : l'intérieur type du vaisseau du joueur ; `sector:spaceport` : le spatioport par défaut, vaisseau amarré dans le hangar ; `sector:salle_temoin` : salle témoin des matériaux de M8) ; `global:<lieu>` place le joueur aux commandes ; `new` = nouvelle partie, exactement comme le bouton du menu (debout dans la chambre du vaisseau, en vue Globale) ; `menu` = menu principal. Lieu inconnu = arrêt en erreur |
| `SH_PRESET=<préréglage>` | Préréglage graphique : `low`, `medium` (défaut) ou `high` |
| `SH_WINDOW=<L>x<H>` / `fullscreen` | Taille de la fenêtre (défaut 1600×900), ou plein écran sans bordure : les mesures de la machine de référence se font en `fullscreen` (1080p) |
| `SH_GPU_TIMINGS=1` | Temps GPU moyen de chaque passe de rendu, journalisé à la sortie (à comparer entre deux réglages : sur l'UHD 770, certaines passes comptent aussi des attentes) |
| `SH_USER_DIR=<dossier>` | Dossier des sauvegardes et réglages. Sans lui, l'autopilote utilise `target/autopilot-user` : jamais le dossier du joueur. Les réglages du joueur ne sont pas lus quand l'autopilote pilote |
| `SH_AUDIO=1`, `SH_TIPS=1` | L'autopilote est muet et sans conseils par défaut (il tourne pendant que l'auteur travaille ; captures propres) |
| `SH_KEYS=KeyE:KeyF,…` | Remappage imposé (vérification des touches et des aides) |
| `SH_PAUSE=1` / `load`, `SH_OPTIONS=1` / `keys`, `SH_TERMINAL=codex` / `fiche` / `journal` | Ouvre le menu de pause (ou sa liste de sauvegardes), les options (ou les touches) au menu principal, un écran du terminal : captures |
| `SH_SHOW_FPS=1` | Garde l'affichage FPS pendant une capture (masqué par défaut pour des images propres ; feature `dev`) |
| `SH_GPU_CLUSTERING=1` | Réactive le clustering GPU des lumières (cassé sur UHD 770, voir §4) |
| `SH_BAKED_LIGHTING=0` | Ignore les lightmaps : éclairage des intérieurs entièrement dynamique (comparaisons de performance) |
| `SH_SPAWN=<id>`, `SH_SPAWN_AT=x,y,z,cap,tangage` | Point d'apparition dans un secteur, ou position libre (degrés), pour les captures |
| `SH_LOOK=x,y,z` | Direction initiale de la caméra en vue Globale (captures du ciel, des astres) |
| `SH_ARRIVAL=<objet>,<distance>` | Vue Globale : arrivée près d'un autre objet (station, structure, astre), à cette distance en mètres |
| `SH_SCENARIO=walk_ship` | Avec `SH_START=sector:corvette` : sas, toutes les salles, cockpit, puis s'asseoir au poste de pilotage, avec mesure du temps de frame |
| `SH_SCENARIO=menu_return` | Avec `SH_START=new` : Échap vers le menu principal, puis « Nouvelle partie », qui doit repartir de zéro (une partie quittée est effacée, `state::forget_game`) |
| `SH_SCENARIO=save_load` | Avec `SH_USER_DIR` et `SH_START=new` (en vol) ou `sector:spaceport` (à quai) : sauvegarder, tout changer (argent, position du vaisseau), recharger, retrouver argent, date, vue et pose. `SH_START=load:<fichier>` démarre sur une sauvegarde ; une sauvegarde d'avant M9 se migre (champ `content` : `player_ship` → `corvette`, `hexis_spaceport` → `spaceport`, objets pris renommés) |
| `SH_SCENARIO=pause_escape` | Avec `SH_START=new` : Échap (vrais messages clavier), « Reprendre », Échap, options fermées par Échap puis par « Retour » : ni plantage, ni écran qui reste affiché |
| `SH_SCENARIO=map_key` | Avec `SH_START=new` (la partie commence debout : le scénario prend d'abord les commandes) : M tenu 150 ms comme par un joueur ouvre la carte, qui doit rester ouverte, puis la referme |
| `SH_DAYS=N` | La partie commence N jours après le jour 1 de 425at ; le monde vivant rattrape (captures des nouvelles, des flottes connues) |
| `SH_ROUTE=<système>`, `SH_HOVER=<système>` | Vue Galaxie : itinéraire posé depuis la position du vaisseau, survol simulé (captures du panneau et de l'infobulle) ; `SH_HOVER` accepte aussi une cible du détail d'un système (M10) |
| `SH_MAP_TARGET=<cible>`, `SH_MAP_ENGAGE=<s>`, `SH_MAP_CONTRACT=<système>` | Carte (M10) : cible du lieu courant proposée à l'autopilote ; autopilote engagé après ce délai, comme la touche ; un contrat de transport vers ce système accepté une fois (panneau des contrats en cours) |
| `SH_SCENARIO=voyage` | Avec `SH_START=global:hexis` : Hexis → Cygnara par Portail, retour par hyperespace, durées vérifiées (critère de M3) |
| `SH_SCENARIO=port_loop` | Avec `SH_START=sector:spaceport` : quai du hangar → sas → cockpit → départ → vol → amarrage → hangar (critère de M2). M11 : au hangar (départ et retour), du vaisseau amarré seul le sas est chargé ; le hall est prêt (`CellReady`) avant que la porte `couloir__hangar` ne quitte `Closed`, puis on la franchit |
| `SH_SCENARIO=airlock_dock` | Avec `SH_START=sector:spaceport SH_SPAWN_AT=11.0,0,0,-90,0` : immobile 10 s dans le sas amarré, tourné vers l'intérieur ; portes fermées, seul le sas est affiché (`CellStates::shown` ; rien d'autre ne se dessine, coque comprise), temps de frame du séjour journalisé ; la porte extérieure demandée (`Door::request_open`), le hangar est affiché dès la frame où elle n'est plus fermée (M11) |
| `SH_SCENARIO=contract_loop` | Avec `SH_START=global:hexis` : contrat de transport Hexis → Cygnara, livraison, paiement, réputation, échéance de la Banque, trahison d'un second contrat et ses conséquences, captures `target/shots/m5-*.png` (critère de M5) |
| `SH_RAID=1` / `alone` / `0` | Raid de la Ligue forcé à l'arrivée dans un système (`alone` : sans rescousse) ; `0` : aucun raid (scénarios hors combat) |
| `SH_SCENARIO=raid_win` | Avec `SH_START=global:cygnara SH_RAID=1` : raid gagné aux côtés de la Flotte de défense, temps de frame pendant le combat (10 coques), puis retour à Hexis, dégâts toujours là, réparation au chantier (critère de M6) |
| `SH_SCENARIO=raid_flee` / `raid_lose` | Avec `SH_RAID=1` / `SH_RAID=alone` : fuite (distancer, puis croisière) ; défaite (seul et passif) et écran de mort |
| `SH_BOARDING=N` | N pirates abordent le vaisseau du joueur par le couloir arrière |
| `SH_SCENARIO=firefight` / `firefight_lose` | Avec `SH_START=sector:corvette SH_SPAWN=soute SH_BOARDING=2` (ou `3`) : abordage repoussé à pied ; défaite passive |
| `SH_SCENARIO=fly_ship` | Avec `SH_START=global:hexis` : vol aux commandes (assisté, croisière, lacet, découplé), puis marche du cockpit au couloir en plein vol |
| `SH_SCREEN=inventory\|skills`, `SH_CONTAINER=<casier>` | Ouvre l'écran du personnage (onglet) ou le rangement d'un casier 3 s après l'arrivée (M9) |
| `SH_GIVE=<objet>:<n>,…`, `SH_XP=<compétence>:<points>,…` | Objets dans l'inventaire, expérience posée au début de la partie (captures de l'écran du personnage) |
| `SH_KEYHELP=0\|1` | Force l'encart des touches (replié / affiché) |
| `SH_TALK=<rôle>[:<variante>]` | Conversation avec un PNJ de comptoir (`contracts`, `bank`, `market`, `shipyard`, `bar`) 3 s après l'arrivée ; `SH_TERMINAL` accepte aussi `contrats`, `marche`, `banque`, `chantier`, `guichet`, `taverne` |
| `SH_OPTIONS=<page>` | `general` (ou `1`), `display`, `gameplay`, `sound`, `controls`, `keys:interface` (ou `keys`), `keys:foot`, `keys:piloting`, `keys:map` |
| `SH_KEYS=…` | Remappage imposé : `KeyE:KeyF` (toutes les actions de E), `foot_interact:KeyR`, `foot_jump:Space+KeyB` (deux touches), `foot_crouch:-` (case vide) |
| `SH_MAP_HEIGHT=<h>`, `SH_MAP_CENTER=x,y` | Cadrage de la carte galactique (1100 : tout ; 150 : plan rapproché ; de 45 à 16 : détail d'un système, astres, stations et points de livraison) |
| `SH_SCENARIO=pickup_persist` | Avec `SH_USER_DIR` et `SH_START=sector:spaceport` : ramasser la tablette de la chambre, sauvegarder, recharger : elle n'y est plus et reste dans l'inventaire |
| `SH_SCENARIO=terminal_tour` | Avec `SH_START=sector:spaceport SH_RAID=0` : chaque PNJ, le service qu'il ouvre, retour à la conversation, onglets de la tablette ; captures `target/shots/m9-borne-*.png` |
| `SH_SCENARIO=voyage_isolated` | Avec `SH_START=global:hexis SH_RAID=0` : Hexis → Calderis (monde isolé, hyperespace, arrivée en orbite) et retour, durées vérifiées ; capture `m9-isole-orbite.png` |
| `SH_SCENARIO=key_help` | Avec `SH_START=sector:spaceport SH_SPAWN=hall` : F1 tenue 150 ms replie puis redéplie l'encart des touches |
| `SH_SCENARIO=character` | Avec `SH_START=sector:corvette SH_SPAWN=soute` : I ouvre l'inventaire, O les compétences, Échap ferme sans ouvrir la pause ; au casier, Entrée prend un objet, E referme |
| `SH_TUTORIAL=1`, `SH_TUTORIAL_STEP=N` | Le tutoriel guidé (coupé par défaut quand l'autopilote pilote, comme les conseils) ; démarrer à l'étape N, de 1 (chambre) à 11 (sauvegarde) ; 12 = terminé (captures du panneau). M11 : la Banque est l'étape 7 |
| `SH_FPS_LIMIT=<n>`, `SH_VSYNC=1`, `SH_FPS_COUNTER=1` | Limite d'images (0 : illimitée, de 10 à 1000), synchronisation verticale et compteur d'i/s, pour l'autopilote qui ne lit pas les réglages du joueur (M11, `frame_pace`). Le joueur a 60 i/s par défaut ; l'autopilote, illimité |
| `SH_DYNRES=1` | Résolution dynamique pour l'autopilote (coupée par défaut pour des mesures comparables ; active par défaut pour le joueur). Le résumé de sortie dit l'échelle moyenne, la plus basse et le nombre de paliers ; la trace a les colonnes `render_scale` et `gpu_ms` (M11) |
| `SH_FRAME_TRACE=<fichier.csv>` | Une ligne par frame après le chargement : durée, mise à jour, rendu, attente du limiteur, vue, secteur, cellule, instances de cellule (Locales, Sectorielles, cachées, apparues, retirées), entités, lumières et maillages visibles, pipelines, envois au GPU ; écrite par blocs sur un fil à part (M11) |
| `SH_AUTOPILOT=<cible>` | Vue Globale : autopilote engagé vers cette cible du système à la première arrivée (astre, station, structure, port : `vorrh_shipyards`, `hexis_spaceport_approach`, `concordia_portal_hub`…), comme depuis la carte ; progression journalisée toutes les ≈ 10 s |
| `SH_CONTRACT=<système>` | Un contrat de transport d'Hexis vers ce système, accepté dès l'entrée en partie (captures du repère de destination) |
| `SH_SCENARIO=deliver_vorrh` | Avec `SH_START=global:hexis SH_RAID=0` : contrat vers Vorrh, Portail, autopilote jusqu'aux chantiers de Vorrh-II (≈ 4 min), livraison, paiement ; captures `target/shots/m10-vorrh-*.png` (critère de M10) |
| `SH_LOCKER_NAME=<conteneur>:<nom>,…`, `SH_NAMING=<conteneur>` | Casiers déjà nommés au début de la partie (le nom ne peut pas contenir d'espace dans `captures.*`) ; saisie du nom de ce casier ouverte 3 s après l'arrivée |
| `SH_SCENARIO=fridge` | Avec `SH_START=sector:corvette SH_GIVE=spare_parts:5 SH_CONTAINER=corvette_carre_frigo` : « Rations : 12 », pièces refusées (rien ne bouge), une ration prise puis remise ; capture `m10-frigo.png` |
| `SH_SCENARIO=locker_name` | Avec `SH_USER_DIR` et `SH_START=sector:spaceport` (on ne sauvegarde pas dans l'intérieur type seul) : nommer un casier de la chambre (« Mes affaires »), le nom sur sa plaque, sauvegarder, recharger : il est resté ; captures `m10-saisie-nom.png`, `m10-plaque.png`, `m10-plaque-rechargee.png` |
| `SH_SCENARIO=airlock_space` | Avec `SH_START=global:hexis SH_RAID=0` : en vol, au sas, la porte extérieure annonce le vide (`interact.door.void`) et la touche d'interaction ne la sollicite pas (elle reste fermée, sans son) ; capture `m10-sas-vide.png` |
| `SH_SCENARIO=tutorial` | Avec `SH_START=new SH_TUTORIAL=1 SH_RAID=0 SH_USER_DIR=…` : joue tout le tutoriel : chambre (casier, tablette, inventaire), tour du vaisseau de porte en porte, commandes, amarrage, hangar, hall, bureau et guichetier, contrat, Banque (guichet, compte ouvert, M11), écrans, départ, carte, Portail, livraison, sauvegarde ; captures `target/shots/m9-tuto-chambre.png`, `m9-tuto-tour.png`, `m11-tuto-banque.png`, `m11-tuto-guichet.png`, `m8-tuto-*.png` (critère de M8 et de M9). Une action propre d'un scénario se joue avec le `World` par `Step::Script` (`dev.rs`) |

À la sortie, le jeu journalise toujours le **temps de frame moyen et maximal** et le nombre de frames au-dessus de 33 ms (hors chargement), puis (M11) une ligne « temps de frame : p50, p95, p99, écart-type, chutes » (une chute : au moins 3 frames de suite à plus de 1,5 × la médiane) et les 10 pires frames avec leur contexte (« pire frame k/10 : … »). `tools/check.ps1` (Windows) et `tools/check.sh` (Linux) lancent un test de fumée de chaque point d'entrée (et, depuis M11, une fumée de la limite d'images : `SH_FPS_LIMIT=60`, Bas, 960 × 540, moyenne entre 15,5 et 18 ms) ; ces deux scripts font la même chose, modifie-les **ensemble**. Quand tout est vert, ils notent dans `target/check-ok.txt` (propre au poste) l'arbre vérifié (`tree=`, contenu du dossier de travail au départ du script) : s'il vaut `git rev-parse "HEAD^{tree}"`, l'état courant a déjà passé le contrôle sur ce poste. Les **mesures de performance** passent par `tools/perf.ps1` / `tools/perf.sh` (même règle) : build release, puis chaque scénario qui porte un critère de performance (`walk_ship` avec et sans lightmaps, `fly_ship`, `port_loop`, `raid_win`, `firefight`, `firefight_lose`), résumé dans `target/perf/` (exemple : `powershell -ExecutionPolicy Bypass -File tools/perf.ps1 -Preset low -Window fullscreen`, le préréglage de la cible ; `-Preset medium` pour comparer). Le jeu occupe l'écran pendant les mesures : préviens l'auteur.

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
- **Fenêtre sans focus bridée à 60 Hz (mesures de M6)** : le réglage par défaut (`WinitSettings::game()`) ralentit la boucle quand la fenêtre n'a pas le focus, ce qui est le cas d'une fenêtre lancée par un agent. Les frames rapides étaient alors comptées ≈ 16,7 ms (« calé sur 60 Hz ») et le vrai coût était masqué. L'autopilote passe en `WinitSettings::continuous()` dès qu'une de ses variables est posée (`dev.rs`).
- **Conditions d'un groupe chaîné (M7)** : `(a, b, c).chain().run_if(cond)` évalue `cond` **une fois**, avant `a`. Si `a` retire la ressource (commandes appliquées entre deux systèmes chaînés), `b` et `c` tournent sans elle et plantent. Mettre la condition **sur chaque système** (`(a.run_if(cond), b.run_if(cond)).chain()`) : elle est évaluée juste avant lui. Le menu de pause et les options ont planté ainsi au double Échap ; l'effacement d'une partie se fait pour la même raison dans `OnExit(AppState::InGame)` (`state::forget_game`). Un écran retiré avec sa ressource doit aussi effacer ses entités (sinon il reste affiché).
- **Touche tenue d'un contexte à l'autre (M8)** : un contexte qui s'active voit une touche encore enfoncée comme un nouvel appui (`Start`). M ouvrait la carte, le contexte Carte s'activait touche tenue et la refermait aussitôt (les deux vues s'enchaînent en quelques ms). Toute action « à un coup » porte `ActionSettings { require_reset: true }` (`input.rs`) : elle attend que la touche soit relâchée. Les scénarios simulent un appui sur un seul frame : `map_key` tient la touche 150 ms.
- **Liaisons de `bevy_enhanced_input` (M7)** : `Binding` est un composant **immuable** sur une entité de liaison (relation `BindingOf`) ; on remappe en réinsérant un `Binding` neuf (`options::apply_keys`).
- **Coût des lumières sur l'UHD 770 (M7)** : la passe opaque évalue toutes les `PointLight` de la grappe, par pixel ; une dizaine de lumières de cellule y coûtait 15 ms en 1080p. Dans une cellule à lightmap, elles ne servent qu'aux reflets : `baked_cell_lights` les supprime en Bas et Moyen. Le filtrage d'ombre `Gaussian` (9 lectures) coûte nettement plus que `Hardware2x2`. Une prépasse de profondeur **ralentit** (pas de surcharge à éviter). Les lumières ne filtrent pas par maillage (`RenderLayers` par vue seulement) : sans ombres, le soleil traverse les parois.
- **Émissif et exposition (M8)** : dans Bevy, l'émissif d'un matériau glTF (facteur × `KHR_materials_emissive_strength`) n'est **pas** atténué par l'exposition de la caméra (`Exposure::INDOOR`, EV100 7) : il s'ajoute presque tel quel avant le tonemapping. Repères qui rendent bien en jeu : ≈ 4 pour un bandeau ou un feu, 1 à 2 pour un écran ou un hublot. Des valeurs calibrées pour une exposition physique (500) sortent en blocs blancs.
- **Cadres d'interface (M8)** : un `ImageNode` se dessine par défaut dans la boîte de contenu (`VisualBox::ContentBox`), en retrait du `padding` : pour qu'un cadre en 9 tranches couvre tout le nœud, `visual_box: VisualBox::BorderBox` (`theme::UiFonts::frame`).
- **Captures des écrans de l'autopilote (M8)** : `SH_PAUSE` et `SH_TERMINAL` ouvrent leur écran 3 s après l'arrivée ; à plusieurs centaines d'i/s (poste Linux), `SH_SCREENSHOT_FRAME` doit dépasser ≈ 2 000. De même, une capture trop tôt après une arrivée en vue Globale peut montrer un modèle dont les matériaux ne sont pas encore prêts (seuls ses émissifs apparaissent).
- **Texte qui rétrécit (M9)** : dans une ligne libellé / valeur, un `Text` peut rétrécir jusqu'à sa largeur minimale et se briser mot par mot (« 45 000 cr » sur deux lignes). La valeur prend la largeur restante (`flex_grow`), et les nombres, montants et dates ne passent jamais à la ligne (`TextLayout::no_wrap()`). Vu dans l'écran du personnage et les bornes.
- **Seconde touche (M9)** : la deuxième touche d'une action passe par une liaison jumelle (`options::tag_bindings`, mêmes `Negate` / `SwizzleAxis`) ; une case vide devient `Binding::None`, que tout lecteur de liaisons doit ignorer (encart des touches). Les sensibilités sont des modificateurs `Scale` sur `Look` et `Steer`. Une étiquette de touche déjà résolue ne repasse pas par `Lang::key_label` (qui applique les remappages : double conversion = libellés faux, le « bug d'échange » de M7).
- **Son (M7)** : Bevy ne lit par défaut que l'OGG Vorbis ; la feature `wav` (lecteur `hound`) est activée pour les sons générés par `tools/sh_audio`. Le volume d'une boucle passe par son `AudioSink` (`AudioSinkPlayback::set_volume`).

- **Ressource absente au démarrage (M10)** : un paramètre `Res<T>` d'une ressource qui n'existe pas encore (`Data`, `Texts`, `UiFonts` avant le chargement) fait **paniquer** le système. Tout système qui tourne hors de la partie porte `run_if(in_state(AppState::InGame))` ou `run_if(resource_exists::<T>)`, ou lit `Option<Res<T>>` (la saisie du nom d'un casier a fait planter tous les démarrages).
- **Secteurs fusionnés à quai (M10)** : `sh_data::dock` part du quartier ; tout ce qui est propre au vaisseau (modèle de porte, etc.) doit être recopié sur ses portails (`Portal.door_model`), sinon le vaisseau prend le style du quartier (portes jaunes). De même, chercher la cellule d'un point passe par `Sector::cell_at` (la plus petite), jamais par la première trouvée.
- **Tout ce qui est instancié se dessine (M11)** : la phase opaque de Bevy n'est pas triée d'avant en arrière et il n'y a pas d'occultation automatique. À quai, le hangar, la coque et les coques simplifiées de tout le secteur se dessinaient derrière les cloisons fermées du sas (passe opaque à 12–13 ms au lieu de 6–7). Le streaming cache maintenant (`Visibility::Hidden`, sans décharger) toute cellule qu'on ne peut pas voir depuis la cellule courante par des ouvertures, des portes non fermées ou des **vitrages déclarés**. Le champ `windows` d'une cellule (`["hangar"]`, `["@outside"]`, `[]` ; écrit par `sector_tools.sector_ron` depuis la clé `"windows"` du `META` d'un script) doit lister toute vitre qui laisse voir une autre cellule : une vitre oubliée montre du vide. Sans champ, la règle prudente de M10 s'applique. Une `PointLight` sous une racine cachée n'est plus extraite.
- **Écrire un composant le marque modifié (M11)** : passer par `Mut` (`node.left = …`, `*visibility = …`) marque le composant même si la valeur ne change pas, et l'UI refait sa mise en page, la visibilité se repropage, le texte se reprépare, à chaque frame. Comparer avant d'écrire : `views::set_px_if_changed` pour les positions d'UI (seuil d'un demi-pixel), `set_if_neq` pour le reste.
- **Clés de langue citées dans le code (M11)** : `crates/sh_game/tests/textes.rs` relève toute chaîne littérale qui ressemble à une clé et exige qu'elle existe dans `fr.ron`. Retirer une clé de la langue impose de retirer aussi ses mentions dans le code, même dans un argument ignoré.
- **Limiteur d'images (M11)** : l'attente se fait à la fin de `Render` (système exclusif, fil du rendu), avant `send_time` ; `Time<Real>` suit alors les échéances. Une attente dans un système ordinaire bloquerait un fil de la réserve de calcul du monde principal. Sans limite ni vsync, les frames **alternent** (2,5 / 20,8 ms sur l'UHD 770) : la moyenne ment, l'œil voit une saccade ; avec la limite, 0,1 ms d'écart-type. La marge du limiteur (`pace_wait_ms`) est celle du processeur, pas du GPU (le GPU travaille pendant l'attente) : le temps GPU se mesure à part (`Queue::on_submitted_work_done`, `frame_pace`).
- **Résolution dynamique (M11)** : l'image cible est allouée une fois à l'échelle du préréglage, la caméra n'en rend qu'une partie (`Camera::viewport`) affichée étirée (`ImageNode::rect`). Les textures intermédiaires d'une caméra (HDR, profondeur, FXAA) suivent la taille de la **cible**, pas du viewport : un palier ne réalloue rien (seule la chaîne du bloom suit le viewport). Sur l'UHD 770, c'est elle qui tient 60 i/s en Moyenne et à quai (sas ouvert, départ), où un réglage fixe ne suffit pas.
- **Mesurer machine au repos (M11)** : des sous-agents qui compilent (`cargo lint`) ou un Blender qui rend ou cuit faussent complètement les pics (au même endroit, 96 puis 27 frames au-dessus de 33 ms). Les moyennes GPU (`SH_GPU_TIMINGS`) restent comparables ; les pics, les centiles et `tools/perf.*` se mesurent sans autre travail sur la machine.

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
  perf.ps1 / perf.sh       mesures de performance en release : scénarios des critères, résumé dans target/perf/
  captures.ps1 / .sh       captures à points de vue fixes (avant / après d'une refonte), target/shots/<préfixe>-*.png
  package.ps1 / package.sh build distribuable, assets et notice (package/LISEZMOI.txt) dans une archive
  sh_audio/                sons du jeu (WAV) par synthèse déterministe, sans échantillon extérieur
  ui/                      cadres d'interface en 9 tranches (assets/ui/), générés en Python
  setup-linux.sh           installation d'un poste Linux (paquets, LFS, sources de Bevy, addon MCP)
assets/
  data/canon/              données transcrites du canon (chaque entrée cite sa section source)
  data/propositions/       données inventées, status: Proposition
  data/gameplay/           réglages et contenu générique (équilibrage, préréglages graphiques, horloge…)
  data/lang/               textes visibles du joueur, une langue par fichier (fr.ron, la langue du jeu ; en.ron, mêmes clés)
  models/ textures/ audio/ fonts/ shaders/ ui/
art/                       sources .blend (LFS), même arborescence que assets/models
docs/                      conception, feuille de route, registres
```

État après M1 : tout ce qui précède existe (`tools/blender/`, `art/` et `assets/models/` depuis M1 ; lightmaps dans `assets/textures/lightmaps/`). Modules de `sh_game` : `state` (états), `loading`, `data`, `clock` (horloge du monde), `ui` (thème, menu, HUD, bandeau de pilotage), `input` (contextes), `views` (Galaxie : carte ; Globale : système à l'échelle, ciel, astres, cache procédural ; Sectorielle de test), `space` (origine flottante, référentiel du vaisseau), `sector` (secteurs, streaming, portes, glTF), `player` (à pied), `interaction`, `ship` (pilotage), `transition` (amarrage, départ, voyage), `voyage` (carte, itinéraires), `living` (pont avec `sh_sim` : nouvelles, marchés, connaissance du joueur), `career` (fiche du capitaine, échéances, livraison en orbite, épaves), `ui::terminal` (dialogues, contrats, marché, fiche, journal, chantier), `combat` (raids, `combat::space` : coques, tirs, IA de pilotes ; `combat::foot` : armes portatives, PNJ à couvert), `ui::combat_hud`, `graphics`, `dev` (autopilote, scénarios), `dev_tools` (feature `dev`), et depuis M7 : `save` (sauvegardes versionnées, Homme de fer), `options` (réglages du joueur, remappage), `audio` (musiques, ambiances, effets observés), `codex` (fiches publiques découvertes), `tips` (conseils de première fois), `ui::pause`, `ui::options`, et depuis M8 : `materials` (bibliothèque de textures partagées `assets/textures/materials/` : préchargement, mipmaps, échantillonnage), `tutorial` (tutoriel guidé : étapes sur faits observés, panneau, repère). Depuis M9 : `items` (catalogue d'objets, casiers, objets pris, équipement, messages `OpenContainer` / `TalkTo` / `OpenCharacterScreen`), `npc` (PNJ de comptoir posés sur leurs sockets), `ui::keys_panel` (encart des touches réutilisable, `KeyHelp`), `ui::character` (inventaire, compétences, rangement), `ui::terminal/` (bornes et conversations, en dossier : `page` sans `World`, `view`, `fuel`), matériau de planète peinte (`views::planet`, `assets/shaders/planet.wesl`). Outils depuis M9 : `tools/ui/hud.py` (schéma du vaisseau), `tools/map/map_art.py` (fond et icônes de la carte), `tools/planets/hexis.py` (textures d'Hexis), `tools/blender/items/` (objets et icônes), `tools/blender/characters/clerks.py` (PNJ), `tools/blender/kit/hangar.py`. Depuis M10 : `sector::door_signs` (noms de salle aux portes), `views::FootLabel` (étiquette « monde » à pied, réutilisable : portes, plaques des casiers), `ui::character::naming` (saisie du nom d'un casier), rangements par vaisseau (`items::Containers`, sous-catégories et `storage` du catalogue), autopilote (`ship::AutopilotRequest`, `ship::approach`), cibles d'un système (`sh_data::scene::nav_points`, hub générique), détail d'un système sur la carte (`sh_data::map::system_detail`), langue `nav`. Depuis M11 : `frame_pace` (limite d'images et vsync en options, limiteur dans le monde de rendu, compteur d'i/s du joueur ; trace par frame lue par `dev`), occultation par portails et vitrages dans `sector::streaming` (`CellPlan`, `Sector::seen_from`), porte de chargement (`sector::doors`, `UNLOCK_MAX_WAIT`), repères des objets du système (`ui::flight_hud`), zones libres du HUD (`views::LabelFreeZone`), vitesse de combat (`Handling::combat_max_speed`), résolution dynamique (`graphics::DynamicResolution`, `ScaleController`), préréglage Bas par défaut sur GPU intégré, croisière à montée progressive (`SystemScale::cruise_climb`, `ShipMotion::cruise_spool`), hystérésis du verrou de croisière (`combat::cruise_locked`). Outils depuis M8 : `tools/ui/` (cadres d'interface en 9 tranches, `assets/ui/`), `tools/captures.*` (points de vue fixes). `sh_sim` : `world` (monde vivant, `SimCommand`), `events`, `travel`, `career` (capitaine, contrats, réputations, grades, dette), `combat` (coques à sections, impacts, raids). `sh_data` : `dialogue` (dialogues en données).

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
- **Temps** : horloge de jeu (date en `at`, jour de l'année). Année de 365 jours, affichage « Jour N, 425at ». Rythme du temps dans les vues Locale, Sectorielle et Globale : **×30**, réglable dans les données (validé par l'auteur, P4) ; les voyages hyperspatiaux font passer des jours, simulés à vitesse accélérée.
- **Budget** : un pas journalier < 2 ms ; une année entière en headless < 1 s. Si c'est plus lourd, exécution sur `AsyncComputeTaskPool`.

---

## 8. Données de jeu

### 8.1 Principes

- Format **RON**, schémas dans `sh_data`, validation au chargement **et** dans un test (`cargo t` échoue si une donnée est incohérente).
- **Identifiants** : ASCII en `snake_case`, stables (`concordia`, `vorrh_ii`, `nsu_liberte`). **Noms propres du canon** : exacts, en français, avec accents (ils ne se traduisent pas, voir `docs/QUESTIONS.md` Q11).
- **Langue par domaine (M9)** : le jeu fusionne `assets/data/lang/<code>.ron` (commun) et tous les `assets/data/lang/<code>/*.ron` (un fichier par domaine : `options`, `hud`, `rooms`, `terminal`, `map`, `world`, `character`, `tutorial`, `sectors`) ; une clé en double est une erreur. Un lot écrit dans son fichier de domaine, `fr/` et `en/` ensemble.
- **Langue du jeu : le français** (décision de l'auteur, Q11, qui remplace Q8). Tout texte d'interface passe par une clé de `assets/data/lang/fr.ron` ; jamais de chaîne visible en dur dans le Rust. `en.ron` est tenu avec **les mêmes clés** (un test le vérifie) : toute nouvelle clé s'écrit dans les deux fichiers. Les nombres passent par `Lang::number` (espace fine des milliers, virgule décimale). Une touche citée dans un texte s'écrit en jeton `[KeyE]`, `[Space]` (nom `KeyCode`) : la langue affiche l'étiquette de la disposition réelle du clavier (Q13, `gameplay/keyboard.ron`).
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
- **Identités visuelles** (proposition à valider) : Empire = rigueur, classes uniformes, « une flotte pensée comme un marteau » ; UGL = flotte hétérogène, vaisseaux personnels des héros de la Rébellion ; pirates = assemblages, « facture pirate » ; Hexa-Corp = industriel, neutre, efficace. Ton général : space opera inspiré de Star Wars et Star Trek, **sans emprunt** (tout est original). **Direction artistique (décision de l'auteur, Q14, 04/11/2026) : futuriste, « futur usé » inspiré de Star Wars, détaillé (panneaux, tuyaux, usure, lumière contrastée), jamais copié** ; elle remplace la géométrie simple et lisible de M1–M7 (brief complet : `docs/refonte-visuelle.md` §2). **Pas de boucliers énergétiques**, jamais : seul le blindage protège.
- **Licences** : Poly Haven (CC0) autorisé ; Sketchfab et Poly Pizza seulement avec une licence compatible, vérifiée. Tout asset externe est consigné dans `assets/CREDITS.md` (source, auteur, licence). `generate_3d` (Hyper3D, Tripo, Meshy, Hunyuan) : services payants et licences propres, **uniquement avec l'accord de l'auteur**.

---

## 10. Budgets de performance (UHD 770, 1080p, préréglage « Bas »)

La cible est le préréglage **Bas** depuis le 04/11/2026 (décision de l'auteur, Q15 : la machine de référence est faible) ; de M1 à M7 c'était « Moyen », qui reste mesuré pour comparer (`tools/perf.ps1 -Preset medium`). Moyen et Haut sont pour les machines plus puissantes.

| Poste | Budget |
|---|---|
| Temps de frame | ≤ 16,6 ms visé, **jamais > 33 ms** (y compris pendant le streaming) |
| Triangles visibles | ≤ 1,5 M |
| Objets dessinés | ≤ 1 000 (fusionne, instancie, utilise `VisibilityRange`) |
| Ombres | 1 lumière directionnelle (2–3 cascades) + ≤ 2 lumières ponctuelles ombrées à la fois |
| Post-traitement | Tonemapping oui ; bloom selon le préréglage (coupé en Bas) ; SSAO, SSR, volumétrique désactivés en Bas et en Moyen |
| Anticrénelage | FXAA ou SMAA, pas de MSAA en Bas ni en Moyen |
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

- Identifiants Rust en **anglais** (conventions de l'écosystème) ; commentaires, docs, messages de commit et journaux en **français** ; **textes visibles du joueur en français** (et leur traduction anglaise), dans les fichiers de langue (§8.1). Les noms propres du canon restent en français partout, y compris dans les données et l'interface.
- Pas de `unwrap()` / `expect()` dans les systèmes de jeu (sauf invariant de démarrage documenté) : systèmes qui renvoient `Result`, erreurs typées (`thiserror`) dans les crates pures.
- Pas de `unsafe`. Pas d'état global mutable hors `Resource`. Constantes d'équilibrage dans les données, pas dans le code.
- Commente le **pourquoi**, pas le quoi. Doc `//!` en tête de chaque module de domaine.
- Tests : unitaires dans les crates pures ; test de cohérence des données ; test de déterminisme de la simulation ; tests Bevy headless (`App` minimale avec `MinimalPlugins`) pour les systèmes critiques.

### 12.4 Sessions longues et autonomie

- Avance jalon par jalon sans attendre de validation pour les choix techniques. Arrête-toi et demande quand : une installation exige l'admin, un choix d'univers ou de direction artistique structurant se présente, ou une dépense (service payant) serait nécessaire.
- Lance les commandes longues (première compilation, build release) en arrière-plan.
- Si tu bloques sur une API plus de deux essais : lis l'exemple Bevy correspondant plutôt que de deviner.

### 12.5 Travail en parallèle avec des sous-agents (mode orchestrateur)

- L'auteur lance le mode par **`/orchestrateur <jalon | fichier | liste de tâches>`** (`.claude/skills/orchestrateur/SKILL.md`, la méthode complète). L'**orchestrateur** ne produit rien lui-même, pas même une correction d'une ligne : il découpe le travail en lots et lance un sous-agent par lot (`.claude/agents/artiste-blender.md` pour la 3D, `.claude/agents/dev-jeu.md` pour le code, les données et les outils). Il garde pour lui l'intégration (dont l'export complet et le bake final d'une carte que plusieurs lots touchent), les tests, les commits, les docs et les échanges avec l'auteur. Exemple vécu : le plan de M8 (`docs/refonte-visuelle.md` §4 et §7).
- **Un fichier, un propriétaire** : deux sous-agents ne touchent jamais le même fichier. Les fichiers carrefours (`lib.rs`, `dev.rs`, fichiers de langue, `common.py`, `lightmap.py`, scripts de `tools/`) vont à un seul lot par vague ; `AGENTS.md` et `docs/` restent à l'orchestrateur.
- **Un seul `dev-jeu` à la fois dans le dossier du dépôt** : `cargo lint` compile tout le workspace, deux développeurs se cassent la compilation (vécu en M8). Le second travaille dans un arbre de travail séparé, que l'orchestrateur fusionne.
- **Arbres de travail séparés (vécu en M9)** : (1) ils partagent `target/` (`CARGO_TARGET_DIR`), mais Cargo calcule ses empreintes indépendamment du chemin et compare des dates : une compilation d'un autre arbre peut être jugée « à jour » et c'est son code qui tourne ou qui est vérifié. **Avant chaque compilation dans un arbre (ou dans le dépôt quand des arbres compilent), remettre à l'heure tous les `.rs`** : `Get-ChildItem crates, src, tools -Recurse -Include *.rs -File | ForEach-Object { $_.LastWriteTime = Get-Date }`. (2) Un arbre neuf n'a que des pointeurs LFS : `git -C <arbre> lfs checkout` (cache local, sans réseau) avant d'y lancer le jeu. (3) Un arbre peut être créé sur un ancien commit : vérifier sa base (`git -C <arbre> log -1`) au lancement. (4) Le dossier temporaire est partagé entre agents : noms de fichiers uniques. (6) **L'arbre créé par `isolation: "worktree"` de l'outil Agent part de `origin/main`, pas du `main` local (vécu en M10)** : quand un lot a besoin de commits non poussés, l'orchestrateur crée l'arbre lui-même (`git worktree add .claude/worktrees/<lot> -b <lot> main`, puis `git -C … lfs checkout`) et donne son chemin au sous-agent. (5) L'orchestrateur teste un lot dans son arbre (avec les commits de `main` ajoutés par `cherry-pick`), le commite sur sa branche, puis l'amène sur `main` par `cherry-pick`. (7) **Vécu en M11** : mettre tous les `dev-jeu` (et les artistes) dans des arbres laisse le dossier du dépôt sur l'état de départ, pour les captures et les mesures de l'avant. Pour un correctif après intégration, l'orchestrateur remet l'arbre du lot sur `main` (`git -C <arbre> reset --hard main`, arbre propre) avant de le renvoyer au sous-agent. Un sous-agent peut lancer un test ciblé rapide (`cargo t -p sh_game --lib <module>::`, `cargo test -p sh_data --lib`) quand l'orchestrateur le lui permet pour prouver un correctif.
- **Les sous-agents ne testent pas le jeu** : ni `cargo t`, ni le jeu, ni `tools/check.*`, `tools/perf.*`, `tools/captures.*`, ni `git`, ni sous-agent. Les artistes lancent Blender en arrière-plan (un processus chacun, jamais le Blender ouvert ni le MCP) et regardent leurs propres rendus ; le développeur peut lancer `cargo lint`. L'orchestrateur intègre et teste **une fois par vague**, puis commite lot par lot.
- Pas plus de deux bakes Cycles en même temps sur une machine ; les compilations Rust se partagent le dossier `target/` (verrou).

---

## 13. Interdits (récapitulatif)

- Boucliers énergétiques ; vaisseau capitale existant ; origine de l'Essaim expliquée ; désassimilation réussie ; retour de l'Essaim en v1 ; secret 🔒 affiché au joueur ; idée de `99-Idees/` (annexe B de l'export) présentée comme un fait.
- Contenu de `futur.md` implémenté sans demande.
- Texte de lore en dur dans le code ; nouvelle dépendance non vérifiée pour Bevy 0.20 ; asset sans licence claire ; service payant sans accord.
- Commit qui casse `cargo lint`, `cargo t` ou le démarrage du jeu.
- Modifier `_context/` (hors section « Suggestions » de `futur.md`), versionner des secrets techniques ou des fichiers lourds hors LFS.
