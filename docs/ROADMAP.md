# Feuille de route

> Suis les jalons **dans l'ordre**. Coche chaque tâche (`[x]`) dans le commit qui la termine. Un jalon est fini quand **tous ses critères « Fini quand »** sont vérifiés (tests, autopilote, captures regardées). Push à la fin de chaque jalon.
> Tu peux découper une tâche, en ajouter, en réordonner à l'intérieur d'un jalon : note-le dans `docs/JOURNAL.md`. Changer l'ordre des jalons ou leur périmètre : demande à l'auteur.

État (04/11/2026) : **M0 terminé ; M1 à M6 faits**, critères fonctionnels vérifiés par l'autopilote et les tests, sous Linux puis sous Windows (`tools/check.ps1` vert). **Les critères de performance de M1, M2 et M6 sont mesurés sur la machine de référence et ne sont pas tenus** : ≈ 30 i/s en Moyen 1080p partout, limite du GPU (détails dans `docs/JOURNAL.md`, mesures rejouables par `tools/perf.ps1`). La passe de performance ouvre M7.

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
- [x] Nouvelles mesures sur UHD 770 en release, préréglage Moyen, après les lightmaps (faites le 04/11/2026 : objectif non atteint, voir `docs/JOURNAL.md`).

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

- [x] Mesures sur UHD 770 en release, préréglage Moyen : `fly_ship` et `port_loop` (vol près d'Hexis, transitions) (faites le 04/11/2026 : objectif non atteint, voir `docs/JOURNAL.md`).

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
- [x] Mesures sur UHD 770 en release, préréglage Moyen : `raid_win` (10 coques) et abordages (faites le 04/11/2026 : objectif non atteint, voir `docs/JOURNAL.md`).

**Fini quand** : on peut gagner, fuir ou perdre un raid ; les dégâts persistent jusqu'à réparation ; performance tenue pendant un combat à 10 vaisseaux.

---

## M7 — Tranche verticale « Le contrat d'Hexis » et finition

But : 30 à 60 minutes jouables, de la nouvelle partie à la sauvegarde.

- [ ] Passe de performance (en premier : les critères de M1, M2 et M6 ne sont pas tenus, ≈ 30 i/s en Moyen 1080p, limite du GPU) : temps GPU par passe de rendu, puis réglages et préréglages ; viser ≥ 60 i/s en Moyen sur UHD 770 (`tools/perf.ps1`).
- [ ] Sauvegarde / chargement complets (simulation, joueur, lieu, versionnage du format).
- [ ] Options (graphismes, son, contrôles remappables, sensibilité), préréglages validés sur UHD 770.
- [ ] Audio : musiques, ambiances, effets (sources CC0 ou générées, consignées dans `assets/CREDITS.md`).
- [ ] Codex (informations publiques), aides contextuelles, tutoriel léger.
- [ ] Stabilité ; build release ; script d'empaquetage (exécutable + assets dans une archive).

**Fini quand** : une partie complète de la tranche se joue sans bug bloquant, à ≥ 60 i/s en Moyen sur la machine de référence ; l'auteur l'a testée.

---

## Après la v1 (non planifié, à décider avec l'auteur)

Débuts multiples (`_context/futur.md`), nouveaux systèmes et royaumes à mesure que le canon grandit, commandement de flottes, personnages non humains, magie, histoires écrites de la Trame Actuelle.
