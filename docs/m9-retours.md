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

- **Pas de boucliers** (AGENTS.md §9.3 et §13, canon) : le schéma du vaisseau montre le **blindage
  par section** (proue, poupe, bâbord, tribord, P26) et l'état du cœur, jamais de bouclier.
  Question Q16.
- **Hexis** : le canon dit « monde-usine et place forte », ancien monde industriel ravagé,
  ≈ 1,5 Md d'habitants (§3.3), pas « planète-ville ». Hypothèse : un monde-usine très urbanisé,
  nappes de villes et de complexes industriels visibles de jour, lumières des villes côté nuit,
  zones ravagées. Question Q17.
- **Mondes isolés** (astres hors système, §3.4) : on peut s'y rendre par l'hyperespace (aucun
  Portail) et arriver en orbite ; pas d'amarrage (aucun n'a de spatioport). Les astres secrets 🔒
  restent hors de la carte. Myrth (« on n'y atterrit pas sans invitation ») et Vashyra (« refuse tout
  contact ») : orbite seulement. Question Q18.
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
| L4 | Bornes refaites (tous les écrans du terminal) ; conversation avec un PNJ qui ouvre les services ; la fiche (C) ouvre l'écran du personnage | dev (dépôt) | `crates/sh_game/src/ui/{terminal,theme}.rs`, `crates/sh_data/src/dialogue.rs`, `assets/data/propositions/dialogues/**`, `lang/*/terminal.ron` | L0, L3 | 2 |
| L5 | Écran du personnage : inventaire (icônes, équipement, réputations, emprunts), compétences (niveaux, barres d'expérience), rangement des casiers | dev (arbre) | `crates/sh_game/src/ui/character.rs`, `crates/sh_game/src/ui/character/**`, `lang/*/character.ron` | L0, L3, F | 2 |
| L6 | Hexis, monde-usine : surface urbaine et industrielle, lumières des villes côté nuit, atmosphère | dev (arbre) | `crates/sh_game/src/views/{planet,procedural}.rs`, `assets/shaders/planet*`, `assets/textures/planets/**`, `tools/planets/**` | — | 2 |
| L8 | Carte galactique refaite (fond, icônes par type, flottes, étiquettes sans chevauchement, légende) ; voyage vers les mondes isolés | dev (arbre) | `crates/sh_game/src/views/{galaxy,global}.rs`, `crates/sh_game/src/voyage.rs`, `crates/sh_sim/src/travel.rs`, `crates/sh_data/src/{travel,map,layout}.rs`, `assets/data/gameplay/travel.ron`, `tools/ui/map*`, `assets/ui/map/**`, `lang/*/map.ron` | — | 2 |
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
