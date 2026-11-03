# Journal de bord

> Une entrée courte par session (2 à 5 lignes) : ce qui a été fait, ce qui reste en suspens, les décisions prises. Le plus récent en haut.

## 03/11/2026 — Poste Linux (CachyOS) : fin de M2, pilotage et amarrage

- **À bord en vue Globale** : le secteur du vaisseau est co-chargé, sa racine est le référentiel de l'origine flottante (position **et orientation** : l'extérieur, le ciel et la lumière tournent autour de l'intérieur immobile). Départ de partie en données (`propositions/starts/hexis_captain.ron`, P1).
- **Pilotage** (`sh_game::ship`) : modèle de vol pur et testé (assisté, découplé, croisière selon la loi P2), siège du pilote → contexte Pilotage, F pour se lever, maniement dans `gameplay/flight.ron`, bandeau de pilotage, hublot avant du cockpit (fenêtres à allège dans le kit).
- **Amarrage** : portails externes (`@dock`, `@ship`) et secteur composé (`sh_data::dock`, testé) ; spatioport d'Hexis brut (P7 : passerelle, hall, bureau des contrats, taverne, marché) ; balise d'approche en orbite basse ; séquence d'approche automatique avec préchargement, fondu bref, reprise aux commandes ; départ symétrique. Ciel et textures préparés en tâche de fond à quai : `port_loop` sans aucune frame > 33 ms (pire 14 ms ici).
- Captures : `target/shots/m2-{cockpit-vol,croisiere,marche-en-vol,quai,hall,approche-spatioport}.png`.
- **À mesurer sous Windows** (UHD 770, release, Moyen) : `fly_ship` et `port_loop`, en plus des mesures de M1. Limites connues : texture d'Hexis floue en orbite très basse (512 px), pas encore de coque extérieure du vaisseau, `player_ship.py` n'utilise pas encore `sector_tools.py`.

## 03/11/2026 — Poste Linux (CachyOS) : M2, vue Globale d'Hexis

- **Origine flottante** (`sh_core::SpacePos`, `sh_game::space`) : positions absolues en f64, `Transform` recalculés depuis les f64 à chaque recentrage ; test : même précision près du centre et à 10⁹ km.
- **Échelle P2 chiffrée** (`gameplay/space.ron`) : tailles ÷ 10, distances ÷ 100, croisière à vitesse proportionnelle à la distance de la surface la plus proche (1 à 2 min entre planètes voisines, testé). Disposition générique des systèmes (P16) et mise en scène d'Hexis (P15).
- **Système d'Hexis** : ciel généré depuis M7 (P17 : bulbe à l'ouest, Croissant au nord), Hexis texturée, L'Enclume, hub de Portails, chantiers, trafic (P18, `tools/blender/space/hexis_orbitals.py`). Captures : `target/shots/m2-hexis-{arrivee,hub,chantiers,cargo}.png`, `m2-ciel-*.png`. Piège : la skybox de Bevy lit la cubemap avec z inversé.
- Q11 complétée (termes techniques traduits dans l'interface anglaise, hypothèse).

## 03/11/2026 — Poste Linux (CachyOS) : fin de M1, éclairage précalculé

- **Lightmaps** : `tools/blender/lightmap.py` déplie un 2e jeu d'UV par cellule (commun aux versions Locale et Sectorielle), cuit l'éclairage diffus dans Cycles (une lampe par `LIGHT_`, rebonds, ombres douces, lumière des écrans) et écrit des PNG sRGB normalisés + leur exposition dans le descripteur du secteur. ≈ 3 min pour le vaisseau à 512 échantillons (CPU i9-9900K). En jeu : `Lightmap` sur les maillages des cellules, lumières sans ombres dynamiques ni diffus sur les surfaces cuites (spéculaire et portes seulement), `SH_BAKED_LIGHTING=0` pour comparer. Captures : `target/shots/m1-lm-{soute,sas,cockpit,couloir}.png`.
- **Collisions** : les `_col` (tous des pavés) deviennent des `Collider::cuboid` au lieu d'enveloppes convexes calculées à l'instanciation (piste des pics de temps de frame).
- **Bug de marches (encore le taux de rafraîchissement)** : les lightmaps ont fait passer le vaisseau de ≈ 250 à ≈ 360 i/s sur ce poste et `walk_ship` rebloquait au pied de l'escalier : avec quelques millimètres d'avancée par frame, la capsule se posait sur l'arête du nez de marche et glissait en arrière. La montée sonde maintenant au moins 13 cm (débord nécessaire pour une normale praticable, × 1,5) ; vérifié en Bas, Moyen, Haut et sans lightmaps.
- **À mesurer sous Windows** (UHD 770, release, Moyen, `walk_ship`) : temps de frame moyen et pire, frames > 33 ms, avec et sans lightmaps ; vérifier aussi que `check.ps1` passe (marches à ≈ 35 i/s). C'est la seule chose qui reste dans M1.

## 03/11/2026 — Poste Linux (CachyOS), en alternance avec Windows

- Second poste : CachyOS, i9-9900K, Radeon RX 9070 XT. Il alterne avec le poste Windows (machine de référence) ; ses mesures de performance ne valent pas pour l'UHD 770 (AGENTS.md §3.1 bis et §10).
- Scripts multiplateformes : `tools/check.sh` (pendant de `check.ps1`), `tools/setup-linux.sh` (paquets, LFS local, sources de Bevy, addon MCP), recherche de Blender multiplateforme dans `tools/fonts/convert_inter.py`, commandes Linux dans les README. Procédure et prompts de `docs/reprise-autre-pc.md` réécrits pour l'alternance des deux postes.
- **Bug trouvé grâce au second poste** : à ≈ 230 i/s, le scénario `walk_ship` bloquait au pied de l'escalier du cockpit. Sur un nez de marche, la normale du contact de la capsule dépend de son débord, donc de la durée de la frame (39° à 35 i/s, 71° à 230 i/s, limite 46°). `player::walkable_contact` lit alors la normale de la face elle-même. `tools/check.sh` vert ; à revérifier sous Windows avec `check.ps1`.

## 03/11/2026 — M1 À pied (vaisseau du joueur)

- Pipeline Blender versionné (`tools/blender/`) : bibliothèque commune, kit d'intérieur paramétrique (P13), vaisseau du joueur en 8 cellules (P12) avec descripteur de secteur généré. Conventions `_col`, `LIGHT_`, `INTERACT_`, `SOCKET_` relues par le jeu.
- Jeu : streaming Locale/Sectorielle (anticipation, hystérésis, budget), portes de chargement, contrôleur cinématique avian3d (`MoveAndSlide`, marches), interactions par rayon, HUD minimal. Scénario `walk_ship` : sas → cockpit, 25 étapes, 12 transitions de cellule, toutes portes franchies.
- Non vérifié automatiquement : saut, accroupi, pentes (pas de rampe dans le vaisseau), s'asseoir. **Non fait** : essai de lightmaps précalculées (reporté, éclairage dynamique + budget d'ombres en attendant).
- **Mesures release (UHD 770, fenêtre 1600×900, scénario `walk_ship`)** : Moyen 28,6 ms (35 i/s), pire 86 ms, 310 frames > 33 ms ; Moyen avec 1 seule lumière ombrée (nouvelle valeur par défaut) 25,0 ms, pire 70 ms ; Bas 13,2 ms (76 i/s), pire 51 ms, 23 frames > 33 ms. **Critères de performance de M1 non atteints** : les ombres de lumières ponctuelles (cubemaps) et le bloom pèsent lourd, et l'instanciation d'une cellule (colliders convexes, scène) crée des pics. Pistes pour la suite : lightmaps précalculées, carte d'ombre ponctuelle réduite, colliders boîtes au lieu d'enveloppes convexes, instanciation étalée sur plusieurs frames.
- Arrêt demandé par l'auteur après M1 (limite d'usage) : M2 non commencé. `.gitignore` (caches Python) et `docs/reprise-autre-pc.md` ajoutés par l'auteur, commités ici.

## 03/11/2026 — M0 Fondations

- Environnement vérifié : jeu OK (fumée 120 frames), MCP Blender OK (Blender 5.2.3, addon 1.8). Réponses de l'auteur intégrées : **interface en anglais** (textes dans `assets/data/lang/en.ron`), temps ×30, année de 365 jours, DA « réaliste stylisé ».
- Crates `sh_core` (dates, grille, monnaies, identifiants typés, RNG ChaCha8 à sous-flux), `sh_data` (schémas, chargement, validation complète dont fuite de 🔒), `sh_sim` (socle), outil `sh_import` (carte JSON → `galaxy.ron`, rejouable, testé). Factions, classes, formations, monnaies transcrites à la main.
- `sh_game` : états + sous-états, chargement asynchrone validé, menu, entrées par contextes (BSN), vues de test Galaxie / Globale / Sectorielle, préréglages + échelle de rendu, autopilote `SH_START` + temps de frame, outils `dev` (F1 inspecteur, F2 caméra libre, F3 FPS), `tools/check.ps1`.
- **Pièges Bevy 0.20 trouvés** (détails AGENTS.md §4) : clustering GPU des lumières inopérant sur UHD 770 (forcé en CPU) ; caméra HDR + caméra d'interface à composer par mélange ; exposition intérieure.
- Déviation : les tâches de M0 sont regroupées en trois commits (interdépendantes) au lieu d'un par tâche. Police Inter (OFL) tirée de Blender et convertie en TTF : la police par défaut de Bevy n'a pas d'accents.

## 03/11/2026 — Mise en place

- Installé : Visual Studio Build Tools 2026 (MSVC 14.51 + SDK 10.0.26100), Rust 1.99.1, uv 0.13.7, Blender 5.2.3 LTS (portable, utilisateur), addon MCP for Blender activé, serveur MCP déclaré dans `.mcp.json` (version 2.2.6 épinglée, télémétrie désactivée).
- Clone de référence `.reference/bevy-0.20.1` (exemples, sources, notes de migration), ignoré par Git.
- Squelette : workspace Cargo, binaire `star_histories`, crate `sh_game` (scène de test + autopilote `SH_EXIT_AFTER_FRAMES` / `SH_SCREENSHOT`), liaison dynamique en dev, linker `rust-lld`.
- Docs : `AGENTS.md`, `docs/GDD.md`, `docs/ROADMAP.md`, registres. Prochaine étape : **M0**.
