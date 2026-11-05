# Feuille de route

> Suis les jalons **dans l'ordre**. Coche chaque tâche (`[x]`) dans le commit qui la termine. Un jalon est fini quand **tous ses critères « Fini quand »** sont vérifiés (tests, autopilote, captures regardées). Push à la fin de chaque jalon.
> Tu peux découper une tâche, en ajouter, en réordonner à l'intérieur d'un jalon : note-le dans `docs/JOURNAL.md`. Changer l'ordre des jalons ou leur périmètre : demande à l'auteur.

État (04/11/2026, soir) : **M0 à M7 terminés** sur le poste Windows (critères vérifiés par l'autopilote, `tools/check.ps1` vert, Moyen à ≈ 60–83 i/s en 1080p sur l'UHD 770). L'auteur a testé la tranche : elle fonctionne, mais le visuel ne suffit pas. **M8 en cours, refonte visuelle et tutoriel** (plan détaillé : `docs/refonte-visuelle.md`, avancement au §9) : lot 0 livré et validé par l'auteur (avec le lot U) ; vagues A, D, E, F, T, U puis B, C en cours. Cible de performance abaissée à ≥ 60 i/s en **Bas** (Q15).

---

## M0 — Fondations

But : une architecture propre, les données canon chargées, des outils pour te vérifier toi-même.

- [x] Créer les crates `sh_core`, `sh_data`, `sh_sim` et l'outil `tools/sh_import` (§5.1 d'AGENTS.md). Dépendances communes dans `[workspace.dependencies]`.
- [x] `sh_core` : date stellaire (`bt`/`at`, jour de l'année, affichage « 425at »), case de grille (depuis x, y ; affichage « K7 »), monnaies (crédit, Wuro), identifiants typés, RNG déterministe. Tests unitaires.
- [x] `tools/sh_import` : `_context/star-histories-carte.json` → `assets/data/canon/galaxy.ron` (zones, systèmes, astres, stations, hubs, flottes, drapeaux `secret`). Un README d'usage. Rejouable à l'identique.
- [x] `sh_data` : schémas, chargement, validation (identifiants uniques, références résolues, case cohérente avec les coordonnées, `source` présente sur les entrées canon). Test `les_donnees_sont_valides`.
- [x] Transcrire en données les fondamentaux canon utiles tout de suite : factions (§4), classes de vaisseaux (§2.4), formations (§2.5), monnaies (§2.8), hubs (§3.7).
- [x] `sh_game` : `AppState` (Boot → Loading → MainMenu → InGame) et sous-état `ViewLevel` (Galaxy, Global, Sector) avec `DespawnOnExit` ; écran de chargement minimal ; menu principal (Nouvelle partie, Continuer [grisé], Options [vide], Quitter).
- [x] Entrées : `bevy_enhanced_input` avec les contextes À pied, Pilotage, Carte, Menu.
- [x] Autopilote : `SH_START=<vue>[:<lieu>]` pour démarrer directement dans une vue ; journalisation du temps de frame moyen et maximal à la sortie.
- [x] Outils de debug derrière la feature `dev` : inspecteur (`bevy-inspector-egui`), affichage FPS/temps de frame, caméra libre (`bevy_camera_controller`).
- [x] Préréglages graphiques Bas / Moyen / Haut + échelle de rendu (structure et réglages de base).
- [x] `tools/check.ps1` : fmt --check, lint, tests, test de fumée de chaque `SH_START`.
- [x] Mettre à jour la section 5.1 d'AGENTS.md si la structure réelle diffère.

**Fini quand** : `tools/check.ps1` passe ; le jeu démarre sur le menu ; `SH_START=galaxy` et `SH_START=sector:test` ouvrent des vues de test ; le journal affiche le nombre de systèmes, astres et stations chargés depuis les données.

---

## M1 — À pied : vues Sectorielle et Locale (le vaisseau du joueur)

But : marcher dans son vaisseau, à la première personne, sans aucun chargement visible.

- [x] Pipeline Blender (§9 d'AGENTS.md) : `tools/blender/README.md`, script commun d'export, conventions de noms vérifiées de bout en bout (empties `CELL_`, `PORTAL_`, `SPAWN_`, `INTERACT_`, colliders `_col` lus dans Bevy).
- [x] Kit modulaire d'intérieur de vaisseau généré par script : couloirs (droit, angle, T), sas, porte coulissante, salle, escalier ou échelle, console, siège, couchette, caisses, éclairages. Captures Blender (`look`) et en jeu.
- [x] Le vaisseau du joueur (≈ 60–80 m, proposition) assemblé à partir du kit : cockpit, quartiers, soute, salle des machines, sas.
- [x] Format de secteur (cellules, portails, apparitions) lu depuis le glTF et/ou le RON.
- [x] Contrôleur FPS (avian3d) : marcher, courir, s'accroupir, sauter, escaliers et pentes, visée souris, capture du curseur.
- [x] Interactions (rayon de visée + touche) : portes, consoles, siège, objets à ramasser.
- [x] Streaming de cellules Locale ↔ Sectorielle : chargement asynchrone anticipé, hystérésis, portes comme portes de chargement, budget par frame.
- [x] Éclairage d'intérieur performant : lightmaps précalculées dans Blender (Cycles), lumières sans ombres dynamiques dans les cellules cuites ; collisions en cuboïdes (plus d'enveloppes convexes à l'instanciation).
- [x] Mesures sur UHD 770 en release, préréglage Moyen (faites : objectif non atteint, voir `docs/JOURNAL.md`).
- [x] Nouvelles mesures sur UHD 770 en release, préréglage Moyen, après les lightmaps (04/11/2026 : non atteint, ≈ 31 i/s ; **atteint après la passe de M7** : 83 i/s, aucune frame > 33 ms, voir `docs/JOURNAL.md`).

**Fini quand** : on parcourt tout le vaisseau du sas au cockpit, portes comprises ; aucune frame > 33 ms pendant les transitions de cellules (mesuré par l'autopilote) ; ≥ 60 i/s en Moyen ; captures regardées et propres.

---

## M2 — Vue Globale : un système stellaire et le pilotage

But : décoller d'Hexis, voler dans le système, revenir s'amarrer.

- [x] Origine flottante + positions `f64` (§6.4 d'AGENTS.md) ; tests de précision à grande distance.
- [x] Échelle de jeu des systèmes (proposition chiffrée dans `docs/propositions.md`).
- [x] Système d'Hexis en vue Globale : étoile, planète Hexis, chantiers orbitaux, L'Enclume en construction, hub de Portails, trafic ambiant. Ciel étoilé cohérent avec la position M7 dans la galaxie.
- [x] S'asseoir au poste de pilotage → contexte Pilotage ; modèle de vol 6 axes (assisté / découplé), croisière intra-système ; HUD de pilotage.
- [x] Intérieur du vaisseau co-chargé en vue Globale, vaisseau = référentiel : se lever et marcher en vol.
- [x] Amarrage / désamarrage : transition Globale ↔ Sectorielle, chargement masqué par la séquence.
- [x] Spatioport d'Hexis (un quartier, Sectorielle) en version brute : arrivée du sas, un bureau, un marché, une taverne (contenu proposé, consigné).

- [x] Mesures sur UHD 770 en release, préréglage Moyen : `fly_ship` et `port_loop` (04/11/2026 : non atteint ; **atteint après la passe de M7** : 63 à 66 i/s, quelques pics aux amarrages, voir `docs/JOURNAL.md`).

**Fini quand** : boucle complète quai → cockpit → vol → amarrage → quai, sans écran de chargement visible entre l'intérieur du vaisseau et l'espace ; performance tenue en vol près de la planète.

---

## M3 — Vue Galaxie et voyages

But : traverser la galaxie par Portail ou par hyperespace, en voyant le temps passer.

- [x] Carte galactique depuis les données : contour, zones (UGL, Empire, No Man's Land, Zone Inexplorée, marches), systèmes, astres, stations, hubs, grille A–T × 1–13 ; panoramique et zoom ; infobulles d'informations **publiques** uniquement.
- [x] Planification : Portails (hubs et paires, autorisation et péage), hyperespace (durée selon la distance, nébuleuses bloquantes, routes cartographiées), estimation de carburant.
- [x] Exécution du voyage : la date avance, écran de chargement à l'arrivée en vue Globale.
- [x] Système Cygnara en vue Globale (brut).

**Fini quand** : Hexis → Cygnara par le hub et par hyperespace avec des durées différentes cohérentes ; la date affichée avance ; les secrets n'apparaissent nulle part sur la carte (test automatisé sur les données affichées).

---

## M4 — Le monde vivant (simulation macro)

But : la galaxie bouge toute seule, de façon cohérente avec le canon.

- [x] État initial de la simulation depuis les données : factions, contrôle des systèmes, flottes et leurs positions en 425at (§6.1), économie de base, projets (§9.3).
- [x] Pas journalier : missions des flottes (patrouille, réparation, escorte), production et consommation → prix, stabilité (Primaterre, Ostrag), raids de la Ligue (épargnant l'Empire 🔒), guerre Tosear–Ligue, avancement des projets, calendrier politique.
- [x] Rails souples et interdits durs (`assets/data/canon/rails.ron`, §7.2 d'AGENTS.md) avec tests.
- [x] Pont Bevy : `Resource`, accélération du temps pendant les voyages, `SimEvent` → fil de nouvelles et marchés.
- [x] Connaissance du joueur : brouillard d'information sur la carte (flottes connues, date de l'information).
- [x] Tests : déterminisme (même graine ⇒ même histoire), deux ans de simulation headless < 2 s, aucun rail violé.

**Fini quand** : deux ans simulés produisent un journal d'histoire lisible et plausible ; les nouvelles en jeu reflètent la simulation ; tests verts.

---

## M5 — Le cœur RPG

But : un personnage, des réputations, des organisations, des contrats.

- [x] Fiche de personnage, compétences par l'usage, inventaire, portefeuille crédit/Wuro avec taux d'Hexis.
- [x] Réputations par organisation et sous-organisation ; appartenance et échelles de grades (données ; grades non canon proposés) ; promotion, dégradation ; détection et conséquences des trahisons ; mémoire des PNJ.
- [x] Système de dialogues en données + interface de dialogue.
- [x] Contrats Hexa-Corp (CT, CR, CRS en premier) et missions systémiques issues de la simulation ; journal de missions.
- [x] Interface de commerce sur les terminaux de marché.
- [x] Dettes à la Banque d'Hexis (échéances, saisie).

**Fini quand** : prendre un contrat de transport à Hexis, le livrer à Cygnara, être payé, voir sa réputation changer, rembourser une échéance ; la trahison d'un contrat a des conséquences visibles.

---

## M6 — Combat

But : des combats courts, brutaux, sans boucliers.

- [x] Armes à pied (projectiles, recul, munitions), dégâts et armure, IA de combat des PNJ (couverture, alerte), mort et rechargement.
- [x] Combat spatial : sections de coque et blindage, canons magnétiques, missiles et torpilles, contre-mesures, effets à l'intérieur du vaisseau, IA de pilotes.
- [x] Rencontre : raid de pirates de la Ligue dans le système Cygnara, issu de la simulation.
- [x] Dégâts persistants et réparation (chantier d'Hexis).
- [x] Mesures sur UHD 770 en release, préréglage Moyen : `raid_win` (10 coques) et abordages (04/11/2026 : non atteint ; **atteint après la passe de M7** : 61 i/s en combat, ≈ 80 i/s à pied, voir `docs/JOURNAL.md`).

**Fini quand** : on peut gagner, fuir ou perdre un raid ; les dégâts persistent jusqu'à réparation ; performance tenue pendant un combat à 10 vaisseaux.

---

## M7 — Tranche verticale « Le contrat d'Hexis » et finition

But : 30 à 60 minutes jouables, de la nouvelle partie à la sauvegarde.

- [x] Passe de performance : temps GPU par passe (`SH_GPU_TIMINGS`), puis préréglages mesurés en A/B (lumières des cellules cuites, filtrage et taille des ombres, vitrages, échelle de rendu 0,8 en Moyen) : ≥ 60 i/s en Moyen 1080p sur UHD 770 (`tools/perf.ps1`).
- [x] Sauvegarde / chargement complets (simulation, joueur, lieu, versionnage du format).
- [x] Options (graphismes, son, contrôles remappables, sensibilité), préréglages validés sur UHD 770.
- [x] Audio : musiques, ambiances, effets (sources CC0 ou générées, consignées dans `assets/CREDITS.md`).
- [x] Codex (informations publiques), aides contextuelles, tutoriel léger.
- [x] Stabilité ; build release ; script d'empaquetage (exécutable + assets dans une archive).

**Fini quand** : une partie complète de la tranche se joue sans bug bloquant, à ≥ 60 i/s en Moyen sur la machine de référence ; l'auteur l'a testée.

Testée par l'auteur le 04/11/2026 : jouable de bout en bout (un bug du menu de pause corrigé le jour même), mais « moche » (murs plats, aucun détail) et sans vrai tutoriel : d'où M8.

---

## M8 — Refonte visuelle « futur usé » et tutoriel guidé

But : un jeu beau et compréhensible dès la première minute, sans rien perdre en performances. Décision de l'auteur (Q14) : direction futuriste inspirée de Star Wars, sans emprunt. Plan, lots, règles et méthode (orchestrateur + sous-agents) : `docs/refonte-visuelle.md`.

- [x] Lot 0 : bibliothèque de matériaux « futur usé » (textures cuites dans Blender, trim sheets) ; textures partagées et mipmaps côté Bevy ; **point de contrôle avec l'auteur** sur captures. *Livré le 04/11/2026 (poste Windows) ; captures complètes de la salle témoin en Moyen et en Bas sur le poste Linux, **direction validée par l'auteur le 04/11/2026**, lot U demandé.*
- [x] Lot A : kit d'intérieur refait (même API). *04/11/2026, poste Linux : styles `ship` / `industrial` / `station`, habillage, nouvelle porte.*
- [x] Lots B et C : vaisseau du joueur et spatioport d'Hexis réassemblés et habillés, lightmaps recuites. *04/11/2026, poste Linux ; colliders en double du kit corrigés au passage.*
- [x] Lots D et E : corvettes, cargo, L'Enclume, hub de Portails, chantiers. *04/11/2026, poste Linux.*
- [x] Lot F : abordeur pirate (et armes portatives). *04/11/2026, poste Linux : pistolet magnétique vu à la première personne pendant les combats à pied.*
- [x] Lot T : tutoriel guidé des premières minutes, désactivable dans les options, joué en entier par un scénario d'autopilote. *04/11/2026, poste Linux : `SH_SCENARIO=tutorial` dans `check.sh` / `check.ps1`.*
- [x] Lot U (demandé par l'auteur le 04/11/2026) : interface dans la même direction. *04/11/2026, poste Linux (reste à habiller : options, carte, bandeau du lieu, écrans de transition et de mort).*
- [ ] Intégration et tests par l'orchestrateur à chaque vague *(fait sur le poste Linux pour les deux vagues : `check.sh` vert, captures avant / après ; restent les mesures sous Windows et la validation de l'auteur)* : `tools/check.ps1`, captures avant / après, `tools/perf.ps1` (≥ 60 i/s en Bas sur UHD 770 : cible abaissée de Moyen à Bas par l'auteur, Q15).

**Fini quand** : l'auteur valide le nouveau visuel en jeu ; le tutoriel mène une nouvelle partie jusqu'à la première livraison et la sauvegarde ; `tools/check.ps1` vert ; ≥ 60 i/s en Bas sur la machine de référence.

---

## M9 — Retours de l'auteur après M8

But : traiter la liste de l'auteur du 05/11/2026 (interface, options et touches, carte, objets et inventaire, vaisseau type et hangar, PNJ de comptoir, décor selon la qualité, Hexis, tutoriel). Plan, conventions entre lots, tableau des lots et avancement : `docs/m9-retours.md`.

- [x] L0 : branchement (langue par domaine, schémas et types partagés, actions nouvelles).
- [x] Vague 1 : kit (A), PNJ (E), objets et icônes (F), options et touches (L1), interface en jeu (L2), objets et interactions (L3).
- [x] Vague 2 : intérieur type `corvette` (B), spatioport par défaut avec hangar (D), bornes et conversation (L4), écran du personnage (L5), Hexis (L6), carte galactique (L8). *B et D relus en jeu et recuits à 512 échantillons sous Linux.*
- [x] Vague 3 : coque de la corvette (C), secteurs types et arrivée au hangar (L7), tutoriel dans la chambre (L9) ; bakes finals ; anciens secteurs `player_ship` / `hexis_spaceport` retirés. `tools/check.sh` vert (poste Linux).
- [ ] Intégration et tests par l'orchestrateur à chaque vague : `tools/check.ps1`, captures avant / après, `tools/perf.ps1 -Preset low -Window fullscreen`. *Reste : `check.ps1` et mesures de performance sous Windows (UHD 770), validation des captures par l'auteur.*
- [x] Points ouverts de M9 corrigés pendant M10 : berceau recalé de 8 cm (lot D de M10), combat à pied qui résout la bonne salle à quai (lot L1 de M10). Laissés tels quels à la demande de l'auteur : éclairage de l'intérieur en vol, repère hors champ sur le schéma de la coque.

**Fini quand** : chaque point de la liste est traité ou tranché avec l'auteur ; `tools/check.ps1` vert ; ≥ 60 i/s en Bas 1080p sur la machine de référence ; l'auteur valide les captures avant / après.

---

## M10 — Deuxième passe de retours de l'auteur

But : traiter la liste de l'auteur du 05/11/2026 après M9 (sas et casiers, frigo et sous-catégories d'objets, décor de la corvette, noms de salle aux portes, portes jaunes à quai, station orbitale du spatioport, nez de la corvette, hall, taverne et banque du spatioport, invites et encart des touches, fiche du capitaine, codex, carte zoomée et autopilote, livraison dans tous les systèmes). Plan, causes, conventions entre lots et avancement : `docs/m10.md`.

- [x] Vague 1 : kit (A), nez de la corvette (C), station orbitale (S), spatioport (D), portes (L1), rangements et interactions (L2), écrans et codex (L3), livraison et autopilote (L4).
- [x] Vague 2 : intérieur de la corvette (B), carte (L5), branchement et scénarios (L6) ; bakes finals de `corvette` et `spaceport`. *`tools/check.sh` vert (poste Linux).*
- [ ] Intégration et tests par l'orchestrateur à chaque vague : `tools/check.sh` (Linux) puis `tools/check.ps1`, captures avant / après, `tools/perf.ps1 -Preset low -Window fullscreen` sous Windows.

**Fini quand** : chaque point de la liste est traité ou tranché avec l'auteur ; une livraison vers Vorrh se fait de bout en bout (scénario) ; `tools/check.ps1` vert ; ≥ 60 i/s en Bas 1080p sur la machine de référence ; l'auteur valide les captures avant / après.

---

## Après la v1 (non planifié, à décider avec l'auteur)

Débuts multiples (`_context/futur.md`), nouveaux systèmes et royaumes à mesure que le canon grandit, commandement de flottes, personnages non humains, magie, histoires écrites de la Trame Actuelle.
