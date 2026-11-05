# M9 — Retours de l'auteur après M8

> Plan de la mission lancée par l'auteur le 05/11/2026 (`/orchestrateur`, liste « Bugs / problèmes /
> améliorations »). Méthode : orchestrateur + sous-agents (AGENTS.md §12.5,
> `.claude/skills/orchestrateur/SKILL.md`). Ce fichier est aussi l'état de reprise si la session
> s'interrompt : tableau des lots (§4) et avancement (§7).

## 1. La demande, regroupée

| Domaine | Demande de l'auteur | Lot(s) |
|---|---|---|
| Qualité | Plus de décor en qualité haute, moins en basse : le Bas ne bride plus les autres, le Moyen se distingue du Bas | A (convention, bake), L3 (filtre en jeu), B, D (décor par niveau) |
| Objets | Les objets ramassables ont un modèle 3D et une icône d'inventaire ; un objet pris l'est pour de bon (tablette du vaisseau) | F (modèles, icônes), L3 (objet posé, persistance) |
| Aide aux touches | Encart à gauche, repliable par une touche, composant réutilisable ; le bandeau du haut reste (lieu, salle) | L2 |
| Inventaire | Vraie interface : objets à icônes, équipement porté, récapitulatif des réputations et des emprunts | L5 |
| Compétences | Niveaux et barre d'expérience vers le niveau suivant | L5 |
| Coque | Petit schéma toujours visible de l'état du vaisseau (comme Star Trek Online, en mieux) | L2 |
| Bornes | Écrans des bornes du spatioport refaits, plus beaux | L4 |
| Options | Onglets Général, Affichage, Gameplay, Son, Contrôles ; sensibilité propre au pilotage | L1 |
| Touches | Pages par catégorie, seconde touche par action, échanges intempestifs corrigés, avertissement de conflit | L1 |
| Carte galactique | Refonte complète, belle, icônes par type d'objet et pour les flottes ; voyage vers les mondes isolés | L8 |
| Pilotage | Sensibilité × 3 | L1 |
| Tutoriel | Commence dans la chambre, présente d'abord le rôle de chaque salle | L9 |
| Hexis | Planète laide ; une ville doit se voir depuis l'espace | L6 |
| Salles | Toutes les salles (vaisseau, spatioport) nommées par leur usage | L2 (affichage), B, D (noms dans les descripteurs) |
| Sièges | S'asseoir en visant la chaise, pas un point précis | A (volume), L3 (jeu) |
| Portes | Retirer l'écran au milieu des portes (le pavé à côté suffit) | A |
| Vaisseau | Intérieur type des corvettes (pas « du joueur ») ; cockpit : sièges plus près, vitre plus grande, sans poutres ; un vrai sas de sortie ; une chambre (lit, casiers fonctionnels), départ du tutoriel ; console du réacteur collée au mur | B, C (coque extérieure), L3 (casiers), L7 |
| Spatioport | Arrivée dans un hangar, on sort du vaisseau dans le hangar, les salles actuelles autour ; ce spatioport devient le spatioport par défaut, un lieu peut avoir le sien | D, L7 |
| PNJ | PNJ immobiles derrière des comptoirs, à qui parler pour les actions des bornes | E (modèles), L3 (placement), L4 (conversation) |

## 2. Décisions et hypothèses (réversibles, marquées)

Q16, Q17 et Q18 : hypothèses **validées par l'auteur** le 05/11/2026.

- **Pas de boucliers** (AGENTS.md §9.3 et §13, canon) : le schéma du vaisseau montre le **blindage
  par section** (proue, poupe, bâbord, tribord, P26) et l'état du cœur, jamais de bouclier (Q16).
- **Hexis** : le canon dit « monde-usine et place forte », ancien monde industriel ravagé,
  ≈ 1,5 Md d'habitants (§3.3), pas « planète-ville ». Retenu : un monde-usine très urbanisé,
  nappes de villes et de complexes industriels visibles de jour, lumières des villes côté nuit,
  zones ravagées (Q17).
- **Mondes isolés** (astres hors système, §3.4) : on peut s'y rendre par l'hyperespace (aucun
  Portail) et arriver en orbite ; pas d'amarrage (aucun n'a de spatioport). Les astres secrets 🔒
  restent hors de la carte. Myrth (« on n'y atterrit pas sans invitation ») et Vashyra (« refuse tout
  contact ») : orbite seulement (Q18).
- **Intérieur type** : le vaisseau du joueur, « petit marchand de taille corvette » (P12), utilise
  l'intérieur type `corvette` ; d'autres vaisseaux de la classe pourront le réutiliser. Mise à jour
  de P12.
- **Spatioport par défaut** : le secteur `spaceport` (hangar + salles) sert à tout lieu
  d'amarrage qui ne déclare pas le sien ; Hexis l'utilise. Mise à jour de P7.
- **Sortie au hangar** : le vaisseau repose dans un **berceau** creusé dans le sol du hangar, son
  pont au niveau du sol (y = 0) : on sort du sas par une passerelle plane, sans rampe (le
  contrôleur n'est pas éprouvé sur les pentes, journal de M1).
- **PNJ de comptoir** : anonymes, génériques (règle d'or n° 2 : permis), modèles sans insigne.
- **Objets** : contenu générique de gameplay (`assets/data/gameplay/items.ron`), pas de fait
  d'univers.

## 3. Conventions communes (contrats entre lots, fixés avant les vagues)

1. **Langue éclatée par domaine** (L0) : le jeu fusionne `assets/data/lang/<code>.ron` et tous les
   `assets/data/lang/<code>/*.ron` ; une clé en double est une erreur ; `fr` et `en` ont les mêmes
   clés (test). Chaque lot possède son fichier de domaine (`fr/<domaine>.ron` + `en/<domaine>.ron`).
2. **Décor par niveau de qualité** : propriété personnalisée Blender `detail` (entier) sur un objet
   de décor, exportée en extra glTF `"detail"` : absente ou 0 = toujours ; 1 = Moyen et Haut ;
   2 = Haut seulement. Le préréglage porte `decor_detail` (Bas 0, Moyen 1, Haut 2) ; le jeu cache
   un nœud (et ses enfants) dont `detail` dépasse ce niveau, **changement d'option appliqué en
   direct**. Règles de production : un décor optionnel n'a **pas de collision**, n'est **jamais
   fusionné** avec la géométrie de base (fusion par matériau **et** par niveau), et ne projette
   **pas d'ombre dans le bake** (sa disparition ne laisse pas d'ombre fantôme) ; il a ses UV de
   lightmap comme le reste. Le niveau 0 (ce que voit le Bas) doit rester beau et léger ; le niveau
   2 peut être riche (le Haut n'a pas de budget sur l'UHD 770).
3. **Volume d'interaction** : extra `half_extents: [x, y, z]` (mètres, repère du jeu Y-haut) sur un
   `INTERACT_*` → capteur en pavé centré sur le nœud ; absent → sphère de 0,35 m (comportement
   actuel). Tout siège, console, casier, comptoir de PNJ a un volume qui couvre l'objet.
4. **Nouvelles interactions** : `interact: "locker"` (paramètre `container` : identifiant unique,
   ex. `corvette_chambre_casier_1`) ; `interact: "npc"` (paramètres `role` : `contracts`, `bank`,
   `market`, `shipyard`, `bar`… et `variant` : `a`, `b`, `c`). Un PNJ est placé par un
   `SOCKET_<nom>_npc` (extra `usage: "npc"`, mêmes `role` et `variant`), regard vers +Y Blender ;
   son `INTERACT_<nom>` couvre le PNJ et le comptoir.
5. **Noms de salles** : chaque cellule d'un descripteur de secteur porte `name_key:
   Some("room.<usage>")` (ex. `room.cockpit`, `room.cabin`, `room.airlock`, `room.cargo_hold`,
   `room.engine_room`, `room.mess`, `room.crew_quarters`, `room.corridor`, `room.hangar`,
   `room.arrivals_hall`, `room.contracts_office`, `room.tavern`, `room.market`, `room.bank`,
   `room.shipyard_office`). Clés dans `lang/<code>/rooms.ron` (L2) ; une clé manquante est une
   erreur de validation. Un nouveau nom demandé par B ou D s'ajoute par L2 (ou le lot de langue de
   la vague).
6. **Objets** (L0) : catalogue `assets/data/gameplay/items.ron` (id, `name_key`, `description_key`,
   catégorie, emplacement d'équipement éventuel, pile max). Modèle `models/items/<id>.gltf`
   (origine au centre de la base, posé sur une surface), icône `ui/icons/items/<id>.png`
   (256 × 256, fond transparent). Objets de départ : `datapad`, `spare_parts`, `mag_pistol`
   (équipement : arme), `flight_suit` (équipement : tenue), `toolkit` (équipement : outil),
   `medkit`, `ration`, `data_chip`, `cargo_manifest`. Un `INTERACT_* pickup` (paramètre `item`)
   fait apparaître le modèle de l'objet à sa place ; ramassé, il est noté dans la sauvegarde et ne
   revient plus.
7. **Messages partagés** (L0, `crates/sh_game/src/items.rs`) : `OpenContainer(String)` (casier →
   écran de rangement, L5), `TalkTo { role, variant }` (PNJ → conversation, L4),
   `OpenCharacterScreen(CharacterTab)` (onglets Inventaire / Compétences, L5) ; ressources
   `ItemCatalog`, `Containers` (contenu par identifiant), `PickedUp`, `Equipment`.
8. **Vaisseau dans le hangar** : le pont du vaisseau est à y = 0 dans son repère ; le portail
   `sas__@dock` est la porte **extérieure** du sas (le sas a deux portes) ; la coque (C) enveloppe
   toutes les cellules et s'ouvre sur cette porte ; le hangar (D) a un berceau (fosse) sous la coque
   et une passerelle plane jusqu'à son portail `hangar__@ship`. B publie dans son rapport : boîte
   englobante des cellules, centre et normale de la porte extérieure ; C et D s'y calent.

## 4. Lots et vagues

Un fichier, un propriétaire par vague. `dev-jeu` : un seul dans le dossier du dépôt, les autres en
arbre de travail séparé (`CARGO_TARGET_DIR` partagé : `D:\Projets\Star Histories\target`).

| Lot | Contenu | Type | Fichiers possédés | Dépend de | Vague |
|---|---|---|---|---|---|
| L0 | Branchement : langue éclatée (et `fr.ron`/`en.ron` répartis par domaine), schémas partagés (`name_key` de cellule, catalogue d'objets, `decor_detail` du préréglage), actions nouvelles (aide aux touches, inventaire, compétences), types partagés (§3.7), modules vides branchés, `SH_SCREEN` | dev (dépôt) | `crates/sh_data/src/{lang,lib,validate,sector,settings,items}.rs`, `crates/sh_game/src/{lib,data,loading,input,items,npc,dev}.rs`, `crates/sh_game/src/ui/{mod,keys_panel,character}.rs`, `assets/data/lang/**`, `assets/data/gameplay/{items,graphics}.ron` | — | 0 |
| A | Kit : portes sans écran, volumes d'interaction (sièges, consoles), lit, casiers, comptoir de PNJ, pièces de hangar ; convention `detail` (fusion, bake sans ombre), noms de salle et `half_extents` dans les outils de secteur | artiste | `tools/blender/kit/**`, `tools/blender/{common,lightmap,sector_tools}.py`, `tools/blender/README.md`, `assets/models/kit/**`, `art/kit/**` | — | 1 |
| E | PNJ de comptoir : 3 variantes immobiles, sans insigne | artiste | `tools/blender/characters/clerk*.py`, `art/characters/clerk*`, `assets/models/characters/clerk_*`, textures neuves `char_clerk_*` | — | 1 |
| F | Objets : un modèle par objet du catalogue, icônes rendues depuis ces modèles | artiste | `tools/blender/items/**`, `art/items/**`, `assets/models/items/**`, `assets/ui/icons/items/**`, textures neuves `item_*` | L0 (liste) | 1 |
| L1 | Options à onglets, sensibilité de pilotage (et × 3), touches : pages par catégorie, seconde touche, échanges corrigés, avertissement de conflit | dev (arbre) | `crates/sh_game/src/{options,input,dev_tools}.rs`, `crates/sh_game/src/ui/options.rs`, `assets/data/gameplay/{flight,keyboard}.ron`, `lang/*/options.ron` | L0 | 1 |
| L2 | Interface en jeu : encart des touches à gauche (composant réutilisable), bandeau lieu + salle, schéma du vaisseau toujours visible | dev (arbre) | `crates/sh_game/src/ui/{hud,keys_panel,combat_hud,flight_hud,theme}.rs`, `crates/sh_game/src/{tips}.rs`, `crates/sh_game/src/views/mod.rs`, `tools/ui/**`, `assets/ui/*.png`, `assets/ui/hud/**`, `lang/*/{hud,rooms}.ron` | L0 | 1 |
| L3 | Objets et interactions : objet posé (modèle), persistance des objets pris, casiers (contenu, sauvegarde), volumes d'interaction, PNJ placés et `TalkTo`, décor filtré par qualité (en direct) | dev (dépôt) | `crates/sh_game/src/{interaction,save,graphics,items,npc,dev}.rs`, `crates/sh_game/src/sector/{gltf,mod}.rs`, `assets/data/gameplay/{graphics,items}.ron`, `tools/check.{ps1,sh}`, `lang/*/world.ron` | L0 | 1 |
| B | Intérieur type `corvette` : chambre, sas à deux portes, cockpit (sièges près d'une grande vitre sans poutres), console du réacteur au mur, noms de salle, décor par niveaux | artiste | `tools/blender/ships/corvette.py`, `art/ships/corvette.blend`, `assets/models/ships/corvette/**`, `assets/textures/lightmaps/corvette/**`, `assets/data/propositions/sectors/corvette.ron` | A | 2 |
| D | Spatioport par défaut `spaceport` : hangar et berceau, salles actuelles autour, comptoirs de PNJ, noms de salle, décor par niveaux | artiste | `tools/blender/places/spaceport.py`, `art/places/spaceport.blend`, `assets/models/places/spaceport/**`, `assets/textures/lightmaps/spaceport/**`, `assets/data/propositions/sectors/spaceport.ron` | A (+ cotes de B pour le berceau) | 2 |
| L4 | Bornes refaites (tous les écrans du terminal) ; conversation avec un PNJ qui ouvre les services ; la fiche (C) ouvre l'écran du personnage | dev (dépôt) | `crates/sh_game/src/ui/terminal.rs` (ou `ui/terminal/**`), `crates/sh_data/src/dialogue.rs`, `assets/data/propositions/dialogues/**`, `crates/sh_game/src/dev.rs`, `lang/*/{terminal,world}.ron` ; `ui/theme.rs` en lecture (L2) | L0, L3 | 2 (lancé pendant la vague 1) |
| L5 | Écran du personnage : inventaire (icônes, équipement, réputations, emprunts), compétences (niveaux, barres d'expérience), rangement des casiers | dev (arbre) | `crates/sh_game/src/ui/character.rs`, `crates/sh_game/src/ui/character/**`, `lang/*/character.ron` | L0, L3, F | 2 |
| L6 | Hexis, monde-usine : surface urbaine et industrielle, lumières des villes côté nuit, atmosphère | dev (arbre) | `crates/sh_game/src/views/{planet,procedural}.rs`, `assets/shaders/planet*`, `assets/textures/planets/**`, `tools/planets/**`, `assets/data/gameplay/planets.ron`, `crates/sh_data/src/planets.rs` (+ une ligne dans `sh_data/src/lib.rs`) | — | 2 (lancé pendant la vague 1) |
| L8 | Carte galactique refaite (fond, icônes par type, flottes, étiquettes sans chevauchement, légende) ; voyage vers les mondes isolés | dev (arbre) | `crates/sh_game/src/views/{galaxy,global}.rs`, `crates/sh_game/src/voyage.rs`, `crates/sh_sim/src/travel.rs`, `crates/sh_data/src/{travel,map,layout}.rs`, `assets/data/gameplay/travel.ron`, `tools/map/**`, `assets/ui/map/**`, `lang/*/map.ron` | — | 2 (lancé pendant la vague 1) |
| C | Coque extérieure de la corvette (enveloppe l'intérieur, s'ouvre sur le sas), LOD | artiste | `tools/blender/ships/corvette_hull.py`, `art/ships/corvette_hull.blend`, `assets/models/ships/corvette_hull/**`, textures neuves `hull_*` | B | 3 |
| L7 | Secteurs types et hangar : `corvette` et `spaceport` remplacent `player_ship` et `hexis_spaceport` (données, sauvegardes, scénarios, scripts), spatioport par défaut ou propre au lieu, arrivée au hangar, coque posée dans le berceau | dev (dépôt) | `crates/sh_data/src/{sector,scene,start,validate}.rs`, `crates/sh_game/src/{transition,save,dev}.rs`, `crates/sh_game/src/sector/**`, `crates/sh_game/src/views/sector.rs`, `assets/data/propositions/{starts,systems}/**`, `tools/{check,perf,captures}.*`, `lang/*/sectors.ron` | B, C, D | 3 |
| L9 | Tutoriel : départ dans la chambre, tour des salles et de leur rôle, puis la suite actuelle | dev (dépôt) | `crates/sh_game/src/tutorial.rs`, `lang/*/tutorial.ron`, `crates/sh_game/src/dev.rs` (scénario), `assets/data/propositions/starts/**` | L7 | 3 |

À l'orchestrateur : export et bake finals de `corvette` et `spaceport` (cartes à plusieurs lots :
kit A + assemblage B/D), suppression des anciens `player_ship` / `hexis_spaceport` une fois L7
intégré, captures, mesures, commits, docs.

## 5. Vérification

- `tools/check.ps1` à chaque vague ; scénarios existants intacts, puis adaptés par L7 et L9
  (`walk_ship`, `port_loop`, `tutorial`, `save_load`, abordages).
- Captures avant : `target/shots/m9-avant-*.png` (15 vues fixes + carte, options, touches, fiche,
  aux commandes ; Moyen, 1600 × 900, poste Windows, 05/11/2026). Après : mêmes vues, plus les
  nouvelles (encart des touches, inventaire, compétences, schéma de la coque, hangar, chambre, PNJ,
  carte, Hexis en orbite, décor en Bas / Moyen / Haut).
- Performances : `tools/perf.ps1 -Preset low -Window fullscreen` (≥ 60 i/s en Bas 1080p, jamais
  plus de 33 ms), en particulier le hangar (grande salle, coque, intérieur visible par le sas).

## 6. Risques

- Le hangar est la plus grande salle du jeu : lightmap dense, coque détaillée et intérieur du
  vaisseau visibles ensemble. Niveau 0 sobre, coque en LOD, portails fermés comme occultation.
- Les renommages de L7 touchent sauvegardes, scénarios et scripts : migration de sauvegarde
  versionnée, et une seule passe (L7).
- Le décor de niveau 2 multiplie les objets : fusion par matériau et niveau obligatoire (draw
  calls).

## 7. Avancement

- 05/11/2026 : plan écrit, captures avant prises (poste Windows), `check.ps1` vert sur l'état de
  départ (seuls des docs ont changé depuis le dernier passage).
- 05/11/2026 : L0 livré, vérifié (`cargo t`, démarrage) et commité (444d88f). Vague 1 lancée :
  A, E, F (artistes, dossier du dépôt), L3 (dépôt), L1 et L2 (arbres séparés). F livré (9 modèles,
  9 icônes, atlas `item_gear`). L8 lancé en avance (arbre séparé) sur la place libérée.
- 05/11/2026 : l'auteur autorise jusqu'à dix sous-agents (skill orchestrateur, 751942c). L5 et L6
  lancés en avance (arbres séparés) : huit sous-agents en parallèle. B et D attendent A.
- 05/11/2026 : L3 livré et vérifié à part (`cargo t`, scénario `pickup_persist` vert, tablette posée
  en capture) ; non commité (commit après le contrôle de fin de vague). Défaut vu : l'objet
  provisoire du kit laisse une ombre cuite sous l'objet → renvoyé à A (rien d'amovible ne projette
  d'ombre au bake). L4 lancé dans le dossier du dépôt, `ui/theme.rs` en lecture seule.
  Point ouvert : objets et PNJ sans lightmap dans les cellules cuites (Bas, Moyen : pas de lumière
  dynamique) risquent d'être sombres ; à juger sur les captures des PNJ.
- 05/11/2026 : **piège** : les arbres de travail qui partagent `target/` se contaminent (empreintes
  Cargo indépendantes du chemin, comparaison de dates) : un binaire d'un autre arbre a tourné.
  Parade : remettre à l'heure tous les `.rs` du dossier avant chaque compilation (consigne envoyée à
  tous les `dev-jeu`). Autre piège : les arbres neufs n'ont que des pointeurs LFS (`git lfs
  checkout` pour y lancer le jeu), et certains sont créés sur un vieux commit (vérifier la base).
- 05/11/2026 : L1 et L2 testés dans leur arbre (L1 + L2 ensemble), commités et amenés sur main
  (db733c1, 047f040). L5 testé (inventaire, compétences, casier) : une retouche demandée (colonne des
  emprunts). E livré (3 PNJ). Reste de la vague 1 : A (kit).
- 05/11/2026 : L5 corrigé, revu en capture, amené sur main (f40023e, 3ec2e7a). L8 livré (carte
  refaite, mondes isolés planifiés) ; suite en cours dans son arbre : départ réel vers un monde isolé
  (`transition.rs`), flottes observées en orbite (`living.rs`), Karza par Portail (canon §3.4 :
  « grand hub de Portails », fait canon, réglage ouvert). Manque découvert : aucun ravitaillement en
  carburant lourd → ajouté au chantier par L4 (proposition de prix à inscrire). Scénario
  `voyage_isolated` de L8 à faire ajouter dans `dev.rs` par son prochain propriétaire.
  Propositions à inscrire en fin de vague : prix du carburant, éclairage des mondes isolés (étoile
  lointaine sans nom, bulbe pour les errantes), fond de la carte (prolonge P17), intérieur type
  (P12), spatioport par défaut (P7), PNJ de comptoir et objets (contenu générique).
- 05/11/2026, **arrêt sur limite d'usage — état de reprise** :
  - **Commités sur main (non poussés)** : L0, L1, L2, L5, L8, L6, plus les docs.
  - **Livrés, vérifiés à part, NON commités** (dans le dossier du dépôt) : L3 (objets et interactions :
    `cargo t`, `pickup_persist`), L4 (bornes, PNJ, carburant au chantier : `terminal_tour`,
    `contract_loop`, `raid_win`, `voyage`, `tutorial` verts ; correctif des valeurs brisées capturé
    dans `target/shots/m9-l4b-*.png`, **pas encore regardé**), E (PNJ), F (objets et icônes),
    A (kit). Les commiter un par un après `tools/check.ps1` complet (aucun dev ne travaille dans le
    dossier du dépôt).
  - **Arrêtés en cours** : B (intérieur `corvette`) et D (spatioport `spaceport`). Leurs fichiers
    sont neufs et ne sont lus par aucun code : relancer ces deux lots (mêmes briefs) ou reprendre
    leurs scripts `tools/blender/ships/corvette.py` et `tools/blender/places/spaceport.py` s'ils
    existent.
  - **Reste à faire** : B, D, puis C (coque), L7 (secteurs types et hangar), L9 (tutoriel dans la
    chambre), un lot de branchement (scénario `voyage_isolated`, validation du carburant, `tips.port`,
    plein en orbite dans `career.rs`, clés `sector.corvette` / `sector.spaceport`, captures Hexis et
    carte dans `tools/captures.*`), bakes finals, `check.ps1`, captures après, `perf.ps1 -Preset low
    -Window fullscreen`, AGENTS.md (pièges des arbres de travail, nouvelles variables SH_*), journal,
    propositions, push. Arbres de travail des agents dans `.claude/worktrees/` : à nettoyer.
- 05/11/2026, reprise : A, E, F, L3, L4, L10 (branchements) commités ; `check.ps1` vert ; tout
  poussé (321837c, accord de l'auteur). **B livré** : intérieur `corvette` revu en aperçus et en jeu
  (`SH_START=sector:corvette`), bake final à 512 échantillons lancé par l'orchestrateur. Défaut
  vu : les cellules n'apparaissent qu'après ≈ 1,5 s (portes seules avant) → confié à L7 (écran de
  chargement tenu jusqu'aux cellules Locales prêtes). Cotes de B transmises à D (`HULL_MARGIN`
  1,5 m gardé, patins de la coque à y = −1,7). **Lancés** : C (coque, artiste), L7 (dépôt :
  renommages, spatioport par défaut, hangar, coque posée, migration des sauvegardes, départ dans
  la chambre, scénarios et scripts), L9 (arbre séparé : tutoriel dans la chambre et tour des
  salles, fond de l'encart des touches plus opaque). D en cours (aperçus, cotes définitives).
  **Reste** : bake final de `spaceport` après D, intégration L7 + L9 (L9 se cale sur l'API de L7),
  `git rm` des anciens `player_ship` / `hexis_spaceport` (scripts, modèles, lightmaps, `.blend`),
  `check.ps1`, captures après, perf, AGENTS.md §3.3, propositions (P7, P12, P25, carburant…),
  journal, validation de l'auteur, push, nettoyage de `.claude/worktrees/`.
- 05/11/2026 : **C, L7 et L9 arrêtés** à la demande de l'auteur, avant tout changement (rien
  n'était écrit), pour être relancés sur le poste Linux : leurs briefs complets sont au §8.
- 05/11/2026, **passage au poste Linux — état de reprise** :
  - **Commités sans contrôle complet** (décision de l'auteur, faute de temps), assets seuls que
    le code ne lit pas encore.
  - **B** : bake final interrompu. Modèles réexportés ; 5 lightmaps sur 9 à 512 échantillons,
    les 4 autres (quartiers, chambre, carré, cockpit) de l'aperçu à 64. **À recuire en
    entier.**
  - **D** : arrêté avant son rapport. Dernier export complet avec les cotes réelles de la
    corvette (`SHIP_BBOX`, `SHIP_HATCH`, `HULL_MARGIN` 1,5 m) et des lightmaps d'aperçu.
    **Aperçus non relus** : à regarder dans `target/art-previews/` (non versionné, à refaire
    sous Linux) et en jeu (`SH_START=sector:spaceport`), puis bake final.
  - **Ordre de reprise** :
    1. `git pull` et `tools/check.sh` ;
    2. revue de D en jeu ;
    3. C (§8.0) ;
    4. bakes finals de `corvette` puis de `spaceport` ;
    5. L7 (§8.1), puis L9 (§8.2) ;
    6. `git rm` des anciens `player_ship` / `hexis_spaceport` (scripts, modèles, lightmaps,
       `.blend`) une fois L7 intégré ;
    7. `check.sh`, captures après (`tools/captures.sh m9-apres`) ;
    8. AGENTS.md §3.3, journal, validation de l'auteur, push.
  - Les mesures de performance (`tools/perf.ps1 -Preset low -Window fullscreen`) restent à faire
    sous Windows. Propositions
  mises à jour (P7, P12, P18, P25 ; P28 carburant, P29 mondes isolés, P30 carte).

## 8. Briefs à relancer sur le poste Linux

C, L7 et L9 ont été arrêtés avant d'écrire quoi que ce soit. Ordre : **C** (artiste, coque) peut
partir tout de suite ; **L7** (`dev-jeu`, `/orchestrateur docs/m9-retours.md`), **dans le dossier
du dépôt**, une fois B, C et D intégrés (il peut commencer avant C : la coque absente ne doit pas
le gêner) ; **L9 ensuite** (il dépend de l'API choisie par L7 pour « le secteur intérieur du
vaisseau du joueur »), ou en parallèle dans un arbre séparé si l'on accepte de recaler sa petite
fonction après L7. Sous Linux, les vérifications passent par `tools/check.sh`, et la remise à
l'heure des sources avant une compilation partagée par
`find crates src tools -name '*.rs' -exec touch {} +`. Les mesures de performance restent à faire
sous Windows.

### 8.0 Lot C : coque extérieure de la corvette (artiste-blender)

- **But** : un modèle `corvette_hull` qui enveloppe exactement l'intérieur `corvette` (lot B),
  s'ouvre sur la porte extérieure du sas et laisse la verrière dégagée. Il est posé par le jeu
  dans le berceau du hangar, au même repère que l'intérieur, et servira pour toute corvette de la
  classe vue de dehors.
- **Fichiers à lui seul** :
  - `tools/blender/ships/corvette_hull.py`, `art/ships/corvette_hull.blend` ;
  - `assets/models/ships/corvette_hull/**` ;
  - textures neuves éventuelles `assets/textures/materials/hull_corvette_*`, en réutilisant
    d'abord la bibliothèque (`hull_plates_*`, `metal_*`, `panel_painted_*`) ;
  - aperçus dans `target/art-previews/m9-lotC/`.

  En lecture seule : `corvette.py` et `assets/models/ships/corvette/**` (à importer pour se
  caler), `common.py`, `lightmap.py`, `sector_tools.py`, le kit, `tools/blender/space/warships.py`
  (style des coques de M8, à imiter).
- **À lire** :
  - AGENTS.md §9 et §4 (« Émissif et exposition » : ≈ 4 pour un feu) ;
  - `tools/blender/README.md`, `docs/refonte-visuelle.md` §2 ;
  - ce plan, §2 (« Sortie au hangar »), §3.2 et §3.8 ;
  - `corvette.py` (CELLS, PORTALS, WINDOWS) et `warships.py`.
- **Direction artistique** : « futur usé » (Q14). Un **petit marchand indépendant de taille
  corvette** (P12) : utilitaire, robuste, rapiécé, peinture passée, plaques de réparation, traces
  de chauffe à l'arrière, pochoirs génériques, **aucun insigne de faction**. Silhouette originale.
  **Pas de boucliers** ni d'émetteurs ; des tourelles légères discrètes au plus.
- **Repère et cotes**, en repère du jeu ; Blender (x, y, z) = jeu (x, −z, y) :
  - **Origine** : celle du secteur `corvette`, pont à y = 0, avant vers −Z (+Y Blender).
  - **Boîte de l'intérieur**, cloisons comprises : (−7,8 ; −0,2 ; −20,6) → (7,8 ; 3,6 ; 26,2).
  - **Cellules, en x ; z ; hauteur du plafond** :
    - cockpit −4,2→4,2 ; −20,6→−14,2 ; 3,0
    - chambre −7,8→−1,4 ; −14,2→−7,8 ; 2,8
    - quartiers −7,8→−1,4 ; −7,8→−1,4 ; 2,8
    - carre 1,4→7,8 ; −9,8→−1,4 ; 2,8
    - couloirs −1,4→1,4 ; −14,2→14,2 ; 2,6
    - sas −7,8→−1,4 ; 1,0→5,0 ; 2,6
    - soute 1,4→7,8 ; 1,4→13,6 ; 3,4
    - machines −5,2→5,2 ; 14,2→26,2 ; 3,4
  - **Vides à recouvrir** : bâbord z −1,4→1,0 et 5,0→14,2 ; tribord z −14,2→−9,8 et −1,4→1,4.
  - **Enveloppe permise**, celle de `HULL_MARGIN` = 1,5 m du hangar de D : x ∈ [−9,3 ; 9,3],
    y ∈ [−1,7 ; 5,1], z ∈ [−22,1 ; 27,7]. Le point le plus bas (patins, à plat) est exactement à
    y = −1,7, sur les sabots du berceau.
  - **Jeu** : aucune face à moins de 0,2 m des parois des cellules. Enveloppe fermée : depuis le
    hangar, on ne voit l'intérieur que par le sas et la verrière (l'export de l'intérieur supprime
    des faces cachées).
  - **Sas** : porte `sas__@dock` centrée en (−7,7 ; 1,1 ; 3,0), normale (−1 ; 0 ; 0), 1,4 × 2,2 m,
    dans la paroi x ∈ [−7,8 ; −7,6], z ∈ [2,3 ; 3,7]. Un **module d'embarquement** porte la peau
    à x = −9,3, percé d'un **tunnel** de 1,4 × 2,2 m de x = −9,3 à −7,8 (centré z = 3,0).
    - Il est habillé (encadrement, bandes de danger, feux ambre), avec `corvette_hull_tunnel_col`
      (parois et plafond).
    - **Le sol du tunnel reste vide** : la passerelle du hangar le fournit.
  - **Verrière à laisser dégagée** : frontale z ∈ [−20,6 ; −20,4], x ∈ [−3,5 ; 3,5],
    y ∈ [0,85 ; 2,70] ; latérales x ∈ ±[4,0 ; 4,2], z ∈ [−20,0 ; −18,6].
    - Le verre est déjà dans le cockpit. La coque est affleurante autour (cadre, casquette au
      plus).
    - Depuis `SOCKET_cockpit_pilot_eye`, la vue reste dégagée ; un nez sous l'allège peut avancer
      jusqu'à z = −22,1.
    - Hublots de la chambre opaques : pas d'ouverture.
- **Contenu** :
  - nez et verrière, flancs à plaques, module d'embarquement à bâbord ;
  - porte de cargaison tribord en relief ;
  - bloc moteur arrière à 2 ou 3 tuyères (lueur ≈ 1 à 2), radiateurs, antennes, parabole ;
  - feux de navigation rouge, vert et blanc (≈ 4) ;
  - patins repliés posés à y = −1,7, trappes, tuyaux, rapiéçages.
- **Niveaux de décor** (§3.2) : niveau 0 complet, beau et sobre ; `detail` 1 pour les antennes
  fines, tuyaux et pochoirs ; `detail` 2 pour les greebles riches, câbles et rapiéçages en relief.
  Fusion par matériau **et** par niveau. Pas de lightmap (éclairage dynamique).
- **Budgets** :
  - ≤ 25 000 triangles au niveau 0, ≤ 60 000 avec les niveaux 1 et 2 ;
  - ≤ 8 matériaux, ≤ 20 objets dessinés ;
  - au plus 2 ou 3 textures neuves de 2K ;
  - LOD `corvette_hull_lod` ≤ 3 000 triangles.
- **Export** : glTF séparé (`.gltf` + `.bin`) dans `assets/models/ships/corvette_hull/` :
  - `corvette_hull.gltf`, avec ses extras `detail`, `corvette_hull_tunnel_col` et
    `corvette_hull_col` (3 à 6 boîtes de l'enveloppe, tunnel libre), non rendus ;
  - `corvette_hull_lod.gltf`.

  Le script est paramétrique et rejouable (`blender --background --factory-startup --python
  tools/blender/ships/corvette_hull.py`).
- **Aperçus** :
  - 3/4 avant, 3/4 arrière, profil bâbord et dessus, aux niveaux 0 et 2 ;
  - vue du quai vers le tunnel ;
  - vue de l'œil du pilote ;
  - coupe avec l'intérieur (sans chevauchement) ;
  - LOD.
- **Interdits** : ni `cargo`, ni le jeu, ni les scripts de vérification, ni git, ni sous-agent, ni
  le Blender ouvert ou le MCP, ni téléchargement ; pas de bake.
- **Rapport** :
  - chiffres ;
  - cotes réelles de l'enveloppe et du tunnel ;
  - noms des nœuds `_col` ;
  - ce que le jeu doit faire pour poser la coque ;
  - limites.

### 8.1 Lot L7 : secteurs types `corvette` et `spaceport`, spatioport par défaut, arrivée au hangar

- **But** :
  - le vaisseau du joueur utilise l'intérieur type `corvette` et Hexis le spatioport par défaut
    `spaceport` (hangar et salles), déjà livrés en données et en modèles ;
  - les anciens secteurs `player_ship` et `hexis_spaceport` ne sont plus référencés nulle part ;
  - un lieu d'amarrage sans spatioport propre utilise `spaceport`, et un lieu peut déclarer le
    sien ;
  - à quai, on arrive dans le hangar, le vaisseau posé dans son berceau avec sa coque, et on sort
    par le sas puis la passerelle.
- **Fichiers à lui seul** :
  - `crates/sh_data/src/{sector,scene,start,validate,dialogue}.rs` et
    `crates/sh_data/tests/donnees.rs` ;
  - `crates/sh_game/src/{transition,save,dev,loading}.rs`, `crates/sh_game/src/sector/**`,
    `crates/sh_game/src/views/{sector,mod}.rs` ;
  - `crates/sh_game/src/items.rs`, seulement les exemples et tests d'identifiants ;
  - `assets/data/propositions/{starts,systems,dialogues}/**` ;
  - `assets/data/gameplay/{travel,flight,combat}.ron`, si un renommage l'exige ;
  - `assets/data/lang/{fr,en}/sectors.ron` ;
  - `tools/{check,perf,captures}.{ps1,sh}` (les paires ensemble) ;
  - les mentions de `player_ship` / `hexis_spaceport` dans `tools/blender/README.md` ;
  - il peut supprimer `assets/data/propositions/sectors/{player_ship,hexis_spaceport}.ron` quand
    plus rien ne les lit.
- **En lecture seule** :
  - `assets/data/propositions/sectors/{corvette,spaceport}.ron` : réécrits par les exports et les
    bakes, les changements voulus sont à demander dans le rapport ;
  - `assets/models/**` : l'orchestrateur retire les anciens modèles ;
  - `crates/sh_game/src/tutorial.rs` et la fonction du scénario `tutorial` de `dev.rs` (lot L9),
    sauf les identifiants renommés.
- **À lire** :
  - AGENTS.md en entier (§3.3, §4, §6.2, §6.3, §8, §12.3) ;
  - ce plan, §2 et §3.5, §3.6, §3.8 ;
  - `sh_data/src/sector.rs` (`dock()`, portails `@dock` / `@ship`) ;
  - `sh_game/src/sector/{mod,gltf,streaming}.rs`, `transition.rs`, `save.rs` ;
  - `sh_data/src/{start,validate}.rs`, les deux descripteurs.
- **À faire** :
  1. **Renommages** `player_ship` → `corvette`, `hexis_spaceport` → `spaceport`, partout :
     départ, système, dialogues `sector:`, code, tests, scénarios et `//!` de `dev.rs`, scripts.
     `start.ship` sert à la fois d'identifiant de secteur et de clé de statistiques
     (`flight.ron`, `travel.ron`, `combat.ron`, « Votre cargo »). Recommandé : la clé de
     statistiques reste `player_ship`, et un champ comme `interior: "corvette"` donne le secteur
     intérieur. La validation vérifie les deux.
  2. **Spatioport par défaut** : une entrée `docks` sans secteur propre utilise `spaceport`
     (défaut dans les données). Le bandeau (`views/mod.rs`, `PlaceBanner`) montre le nom du
     lieu (« Spatioport d'Hexis »), jamais « Spatioport » générique, et la salle (`room.*`). Les
     PNJ d'Hexis parlent avec les dialogues `hexis_*`, rattachés au lieu (dock ou système), avec
     repli par rôle.
  3. **Vaisseau du joueur nommé** : dans ses cellules, le bandeau dit « Votre vaisseau » (clé
     `sector.player_ship`) ; « Corvette » reste le nom du secteur type.
  4. **Arrivée au hangar** :
     - `dock()` raccorde `sas__@dock` au portail `@ship` du hangar ; vérifier le berceau (pont à
       y = 0, sas face à la passerelle) ;
     - à quai seulement, poser `models/ships/corvette_hull/corvette_hull.gltf` avec la
       transformation du secteur du vaisseau ;
     - nœuds `*_col` non rendus et changés en colliders fixes (`corvette_hull_tunnel_col`,
       `corvette_hull_col`) ; nœuds `detail` soumis au préréglage
       (`graphics::apply_decor_detail`) ;
     - pas d'ombre ponctuelle si utile, occultation des portails intacte ;
     - fichier absent : ni panique, ni avertissement en boucle.
  5. **Arrivée sans pièces vides** : aujourd'hui les cellules n'apparaissent qu'après ≈ 1,5 s
     (portes et lampes seules avant). Garder l'écran de chargement (ou la transition) jusqu'à ce
     que la cellule du joueur et ses voisines Locales soient instanciées, avec un délai maximal :
     au démarrage, à l'arrivée dans un secteur et au chargement.
  6. **Départ dans la chambre** : `hexis_captain` commence à bord, au point d'apparition
     `chambre`, pas assis, en vue Globale en orbite d'Hexis. Les scénarios `SH_START=new`
     (`save_load`, `menu_return`, `pause_escape`, `map_key`…) vont s'asseoir ou changent de
     départ.
  7. **Sauvegardes** : une sauvegarde d'avant ce lot se charge.
     - Migration versionnée et testée des secteurs et des objets pris :
       `player_ship/quartiers/quartiers_pickup_datapad` → `corvette/chambre/chambre_pickup_datapad`.
     - Une position dans une cellule disparue retombe sur un point sûr (sas à bord, quai à quai).
  8. **Scénarios et scripts**, coordonnées Blender ; le jeu a (x, y, z) = Blender (x, z, −y) :
     - `walk_ship` et `fly_ship` : cockpit sans estrade, marcher jusqu'à w(0 ; 18,4) puis
       `SitAtHelm` ; `FLY_SHIP_WALK` part de (0 ; 18,6) au lieu de (0 ; 20,3), qui est dans la
       planche de bord ;
     - portes du couloir avant : quartiers (−1,4 ; 4,6), carré (1,4 ; 5,6), chambre
       (−1,4 ; 11,0), portail `couloir_avant__chambre` ;
     - sas de 6 m : w(−3,8 ; −3) devient (−5,4 ; −3), quai à x < −7,8 ;
     - abordage par `couloir_arriere` ;
     - `port_loop` : quai → sas → cockpit → vol → amarrage → hangar ;
     - `pickup_persist` : tablette dans la chambre ;
     - `terminal_tour` : nouveaux PNJ, ajouté à `tools/check.*` ;
     - `firefight` et `character` : `SH_START=sector:corvette SH_SPAWN=soute` ;
     - `tools/captures.*` : vues refaites dans les nouveaux secteurs, plus le hangar vu du quai,
       la passerelle vue du sas, la chambre au départ, un comptoir de PNJ, et la chambre en Bas,
       Moyen et Haut ;
     - lignes d'AGENTS.md §3.3 à changer, décrites dans le rapport.
  9. **Petits restes** :
     - test `detecte_un_ravitaillement_invalide` (bloc `refuel` de `combat.ron` : prix ≤ 0 ou
       marchandise inconnue) ;
     - le casier `corvette_chambre_casier_3`, sans entrée dans `items.ron`, est vide et sans
       avertissement.
- **Contraintes** :
  - pas de texte en dur (clés `fr` et `en`) ;
  - pas d'`unwrap` dans les systèmes ;
  - `//!` à jour ;
  - logique pure (choix du spatioport, migration) testée sans `World` ;
  - canon : P7, P12.
- **Interdits** : ni `cargo t`, ni le jeu, ni lancer `tools/check.*` / `perf.*` / `captures.*`,
  ni git, ni sous-agent ; `cargo lint` seulement.
- **Vérification de l'orchestrateur** :
  - `tools/check.sh` complet ;
  - captures du hangar (`SH_START=sector:spaceport`) ;
  - `port_loop`, `pickup_persist`, `save_load` avec une ancienne sauvegarde, `walk_ship`,
    `fly_ship`.

### 8.2 Lot L9 : tutoriel qui commence dans la chambre et fait le tour du vaisseau

- **But** : le tutoriel démarre dans la **chambre** et commence par faire découvrir le vaisseau,
  salle par salle, avec le rôle de chacune (demande de l'auteur). Ensuite vient la suite actuelle :
  commandes, amarrage, quai, contrat, écrans, départ, livraison, sauvegarde.
- **Fichiers à lui seul** :
  - `crates/sh_game/src/tutorial.rs` ;
  - `assets/data/lang/{fr,en}/tutorial.ron` (mêmes clés) ;
  - dans `dev.rs`, **seulement** la fonction du scénario `tutorial` et sa ligne `//!` ;
  - `crates/sh_game/src/ui/keys_panel.rs`, pour la seule retouche du point 6.

  Le reste est en lecture seule : une demande ailleurs (sauvegarde, départ) passe par le rapport.
- **Contexte** : après L7, le vaisseau du joueur est le secteur `corvette`, de cellules
  `cockpit`, `chambre`, `quartiers`, `carre`, `couloir_avant`, `couloir_arriere`, `sas`,
  `soute`, `machines`, chacune avec un `name_key` `room.*`. Le spatioport est `spaceport`, de
  cellules `hangar`, `couloir`, `chantier`, `hall`, `bureau`, `taverne`, `marche`, avec des PNJ
  de comptoir (interaction `npc`, rôle `contracts`…). Une nouvelle partie commence dans la
  chambre, pas assis, en vue Globale. Aucun identifiant de secteur en dur : une seule petite
  fonction « le joueur est-il dans son vaisseau, et dans quelle cellule », appuyée sur l'API de
  L7.
- **À lire** :
  - AGENTS.md en entier (§3.3 `SH_TUTORIAL`, §4, §8, §12.3) ;
  - `docs/refonte-visuelle.md` §6 et ce plan §2 ;
  - `tutorial.rs` en entier et son scénario ;
  - `corvette.ron`, pour les portails servant au repère.
- **À faire** :
  1. **Nouvelles étapes au début**, avant « Prendre les commandes », avancées par des faits
     observés, jamais par une minuterie :
     - **« Votre chambre »** : ouvrir un casier ([KeyE], écran de rangement), ramasser la
       tablette sur le bureau, ouvrir l'inventaire ([KeyI]) ;
     - **« Le tour du vaisseau »** : une vérification par salle visitée (cockpit, sas, soute,
       machines, carré, quartiers ; couloirs facultatifs), chacune avec son nom et son rôle. Le
       tour se fait dans n'importe quel ordre, et le repère montre la porte de la prochaine salle
       non visitée. Il se termine au cockpit, où « Prendre les commandes » commence par
       s'asseoir.
  2. **Rôle des salles** : ce que fait chaque salle dans le jeu tel qu'il est, à vérifier dans le
     code, sans mécanique promise ni fait d'univers inventé. Exemples :
     - cockpit : piloter, la carte, partir et s'amarrer ;
     - sas : la seule sortie, à quai ;
     - soute : la cargaison des contrats ;
     - machines : le réacteur et le carburant lourd ;
     - chambre : repos et casiers ;
     - carré et quartiers : l'équipage.
  3. **Suite à remettre à jour** :
     - « Prendre les commandes » commence par s'asseoir ;
     - « Descendre à quai » : sortie par le sas puis la passerelle ; le fait « sortir par le
       sas » devient « être dans le hangar », puis vient le hall, puis le bureau ;
     - « Prendre un contrat » : parler au **PNJ** du bureau (`npc`, rôle `contracts`) ; le fait
       « au bureau » se détecte par la cellule `bureau` ou par la conversation ;
     - « Garder le fil » : [KeyC], [KeyI] et [KeyO] ouvrent l'écran du personnage, et
       `Facts.sheet` compte dès qu'il est ouvert ; textes à corriger s'ils parlent encore de la
       « fiche du capitaine » (vérifier `ui/character.rs`, `input.rs`).
  4. **Avancement sauvegardé compatible** : une sauvegarde de M8 ou M9 prise en plein tutoriel
     se charge. Si l'étape est sauvegardée par son numéro, passer à un identifiant stable ou à un
     décalage, avec `serde` par défaut. Une sauvegarde d'avant M8 reste « terminé ». Si `save.rs`
     doit changer, le décrire.
  5. **Scénario `tutorial`** (`SH_START=new SH_TUTORIAL=1 SH_RAID=0 SH_USER_DIR=…`) :
     - il joue le nouveau début (casier, tablette, inventaire, tour en marchant de porte en
       porte, s'asseoir), puis la suite au hangar et au PNJ ;
     - captures `m8-tuto-*`, plus `m9-tuto-chambre.png` et `m9-tuto-tour.png` ;
     - coordonnées Blender : apparition `chambre` (−5,3 ; 11,6), cap −120° ; portes quartiers
       (−1,4 ; 4,6), carré (1,4 ; 5,6), chambre (−1,4 ; 11,0) ; sas de −1,4 à −7,7 sur y = −3 ;
       cockpit (0 ; 18,4) puis `SitAtHelm` ;
     - il échoue clairement (erreur et code de sortie) si une étape ne se coche pas.
  6. **Encart des touches** : fond trop transparent (une ligne illisible devant un bandeau
     lumineux de la chambre), à rendre plus opaque (≈ 0,85 à 0,9) sans changer la mise en page.
     Vérifier que le panneau du tutoriel (en haut à droite) ne chevauche ni `HudCorner` ni
     l'encart, en 1600 × 900 comme en 1920 × 1080.
- **Contraintes** :
  - logique pure testée sans `World` ;
  - textes dans `lang/*/tutorial.ron`, touches en jetons ;
  - pas d'`unwrap` dans les systèmes ;
  - `//!` à jour.
- **Interdits** : comme L7 ; `cargo lint` seulement.
- **Vérification de l'orchestrateur** : scénario `tutorial` après L7, et captures du panneau à
  chaque étape (`SH_TUTORIAL_STEP=<n>`).
