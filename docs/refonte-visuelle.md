# M8 — Refonte visuelle « futur usé » et tutoriel guidé

> Plan de travail du jalon M8 (décision de l'auteur du 04/11/2026, `docs/QUESTIONS.md` Q14). Il
> est écrit pour un **agent orchestrateur** qui confie la production des assets à des
> **sous-agents** (`.claude/agents/artiste-blender.md`, `.claude/agents/dev-jeu.md`) et garde
> pour lui la coordination, l'intégration, les tests et les commits. Lis d'abord `AGENTS.md`.

## 1. Pourquoi

L'auteur a joué la tranche de M7 : elle fonctionne (sauvegarde, combat, contrats, 60 i/s), mais
**elle est moche** : murs plats, aucun détail, un monde de pavés gris. Et il manque un vrai
**tutoriel** qui explique le jeu et guide les premières minutes. M8 corrige les deux, sans rien
perdre de ce qui marche (gameplay, données, performances).

## 2. Direction artistique (brief pour tous les lots)

**Futuriste, inspiré de Star Wars, sans emprunt** (AGENTS.md §9.3) : l'esprit du « futur usé »
de la trilogie d'origine, jamais ses designs.

- **Futur usé** : métal peint et écaillé, joints de panneaux, rivets, traces d'usure sur les
  arêtes, crasse dans les coins, éraflures près des portes et des consoles. Rien de neuf et
  brillant, sauf ce qui l'est par fonction (écrans, voyants).
- **Fonctionnel et dense** : tuyaux, gaines, câbles en chemins de câbles, bouches d'aération,
  boîtiers de jonction, nervures structurelles tous les 2 m (la grille du kit), plafonds à
  poutres apparentes, sols en caillebotis avec un peu de lumière dessous.
- **Formes** : couloirs à pans coupés (section octogonale ou trapézoïdale) plutôt que boîtes,
  encadrements de portes épais et arrondis, consoles inclinées couvertes de petits boutons
  rétroéclairés et d'écrans segmentés, sas massifs.
- **Lumière** : contrastes chaud / froid. Ambre ou sodium dans la soute et la salle des
  machines, blanc froid ou bleu au cockpit, voyants rouges et verts. Bandeaux émissifs dans les
  angles plutôt que plafonniers seuls. Elle se **cuit** dans les lightmaps (§5).
- **Identités** (P6, toujours en proposition) : Hexa-Corp industriel (jaune et noir de chantier,
  pochoirs), UGL gris bleu à bande bleu pâle, pirates rouille et rouge sang (« facture pirate » :
  pièces dépareillées, soudures visibles), Empire anthracite et rigoureux.
- **Interdits** : aucun design reconnaissable de Star Wars ou de Star Trek (silhouettes de
  vaisseaux célèbres, insignes, armures, sabres, polices de titres), aucun logo réel, pas de
  boucliers énergétiques (AGENTS.md §13). Tout est original. Aucun fait d'univers inventé dans
  un décor (inscriptions, noms) : génériques ou consignés dans `docs/propositions.md`.
- **Lisibilité** : les portes, consoles, sièges et objets à ramasser restent repérables au
  premier coup d'œil (accent de couleur, lumière). Le jeu se joue à la première personne.

## 3. Contraintes techniques (non négociables)

- **Performances** : la machine de référence (UHD 770) doit garder **≥ 60 i/s en Moyen, 1080p**
  (`tools/perf.ps1`, poste Windows seulement ; mesures de fin de M7 dans `docs/JOURNAL.md` :
  12 à 16,5 ms selon la scène). Budgets d'AGENTS.md §10 : ≤ 1,5 M triangles visibles, ≤ 1 000
  objets dessinés, ≤ 1,5 Go de mémoire vidéo par secteur. Fusion par matériau
  (`common.join_by_material`), niveaux de détail `_lod1`, peu de matériaux partagés.
- **Textures** : 1K par défaut, 2K pour les pièces maîtresses (cockpit, sas, comptoir du bureau).
  Jeux PBR : couleur (sRGB), normale, ORM (occlusion, rugosité, métal empaquetés). **Tout
  matériau procédural de Blender doit être cuit en images avant l'export** : le glTF n'exporte
  pas les nœuds procéduraux. Le 2e jeu d'UV (`lightmap`) reste réservé aux lightmaps ; les
  textures utilisent le 1er (UV0), avec des **trim sheets** et des textures répétables.
- **Doublons en mémoire vidéo** : un `.glb` embarque ses images ; dix cellules qui partagent un
  matériau en chargeraient dix copies. À trancher au lot 0 : glTF séparé (`.gltf` + `.bin`, images
  communes dans `assets/textures/materials/`, une seule poignée par chemin dans Bevy) ou autre
  solution vérifiée. Si le `.bin` arrive, l'ajouter à `.gitattributes` (LFS).
- **Mipmaps** : sans elles, les textures scintillent au loin et coûtent en bande passante. Bevy
  0.20 propose `MipGenerationJobs` (génération sur le GPU, `bevy_core_pipeline::mip_generation`)
  et un `compressed_image_saver` (traitement d'assets) ; KTX2 demanderait un outil externe
  (installation : demander à l'auteur). À vérifier dans `.reference/` et à trancher au lot 0.
- **Conventions du jeu inchangées** (`tools/blender/README.md`) : `_col` (pavés simples),
  `LIGHT_`, `INTERACT_`, `SOCKET_`, `CELL_`, `PORTAL_`, `SPAWN_`, 1 unité = 1 m, Z-haut, avant
  vers +Y, grille de 2 m, dimensions des portes (`kit.DOOR_W`, `kit.DOOR_H`) et des cellules :
  le gameplay (collisions, portes, scénarios de l'autopilote) ne doit pas bouger.
- **Un asset = un script versionné** (`tools/blender/…`), rejouable en arrière-plan ; `.blend`
  dans `art/`, exports dans `assets/` (Git LFS). Licences : Poly Haven (CC0) autorisé en
  complément, consigné dans `assets/CREDITS.md` ; services payants (`generate_3d`) interdits
  sans accord de l'auteur.
- **Lightmaps** : toute géométrie modifiée impose un nouveau bake (`tools/blender/lightmap.py`).
  Bake Cycles sur CPU : long (≈ 10 min par secteur à 512 échantillons sur le poste Linux, plus
  sur l'i5 du poste Windows) ; `SH_BAKE_SAMPLES=64` pour un aperçu. Pas plus de deux bakes en
  même temps sur une machine.

## 4. Les lots

Chaque lot a un **propriétaire unique** de ses fichiers : deux sous-agents ne touchent jamais le
même fichier. L'orchestrateur seul modifie le code Rust partagé, les fichiers de langue (sauf
lot T), `AGENTS.md`, `docs/`, et fait les commits.

| Lot | Contenu | Fichiers possédés | Dépend de |
|---|---|---|---|
| **0. Matériaux** | Bibliothèque de matériaux « futur usé » : textures cuites (panneaux, métal peint écaillé, caillebotis, tuyaux, caoutchouc, écrans, pochoirs), trim sheets, fonction `common` qui crée un matériau texturé ; planche de démonstration rendue. Côté Bevy (orchestrateur) : chargement des textures partagées, mipmaps, échantillonnage | `tools/blender/materials/**`, `assets/textures/materials/**`, ajout de fonctions dans `tools/blender/common.py` (seul lot à y toucher) | — |
| **A. Kit d'intérieur** | Kit refait avec la même API : coque de salle à pans coupés et nervures, plafonds, sols, portes, sas, consoles, sièges, couchettes, casiers, caisses, réacteur, éclairages (bandeaux) | `tools/blender/kit/**`, `art/kit/**`, `assets/models/kit/**` | 0 |
| **B. Vaisseau du joueur** | Réassemblage avec le nouveau kit, habillage propre à chaque pièce (cockpit dense, soute, machines), re-export, lightmaps recuites | `tools/blender/ships/**`, `art/ships/**`, `assets/models/ships/**`, `assets/textures/lightmaps/player_ship/**`, `assets/data/propositions/sectors/player_ship.ron` | A |
| **C. Spatioport d'Hexis** | Idem pour le quartier (hall, bureau Hexa-Corp, marché, taverne, passerelle) | `tools/blender/places/**`, `tools/blender/sector_tools.py`, `art/places/**`, `assets/models/places/**`, `assets/textures/lightmaps/hexis_spaceport/**`, son descripteur de secteur | A |
| **D. Vaisseaux de combat** | Corvettes pirate et de l'Union | `tools/blender/space/warships.py`, `art/space/warships.blend`, `assets/models/space/{pirate,ugl}_corvette.*` | 0 |
| **E. Orbitales d'Hexis** | L'Enclume, hub de Portails, chantiers, cargo du trafic (et coque du joueur) | `tools/blender/space/hexis_orbitals.py`, `art/space/hexis_orbitals.blend`, `assets/models/space/{l_enclume,portal_hub,shipyard,freighter}.*` | 0 |
| **F. Personnages** | Abordeur pirate (et armes portatives si l'orchestrateur l'ajoute) | `tools/blender/characters/**`, `art/characters/**`, `assets/models/characters/**` | 0 |
| **T. Tutoriel** | Code : tutoriel guidé (§6) | `crates/sh_game/src/tutorial.rs` (+ textes `tutorial.*` des deux fichiers de langue), branchement minimal indiqué par l'orchestrateur | — |
| **U. Interface** (si l'auteur le veut) | Thème des menus, du HUD, du terminal dans la même direction | `crates/sh_game/src/ui/theme.rs` et ce que l'orchestrateur attribue | 0 |

Vagues conseillées : **lot 0 seul** (puis point de contrôle avec l'auteur, §7), ensuite **A, D, E,
F, T** en parallèle, puis **B et C** quand A est livré. `lightmap.py` et `common.py` ne changent
que par le lot 0 ou par l'orchestrateur.

## 5. Éclairage

Les intérieurs restent cuits (le meilleur levier sur GPU intégré, AGENTS.md §4) : les lots B et
C posent leurs lampes (`LIGHT_` : `lumens`, `range`, `color`) et leurs surfaces émissives, puis
recuisent. Les lumières dynamiques ne servent qu'aux objets mobiles en Haut
(`baked_cell_lights`). Les bandeaux émissifs comptent dans le bake (lumière des écrans).

## 6. Le tutoriel (lot T)

Remplace les conseils de première fois pour le début de partie (les conseils restent pour les
situations plus tardives : raid, abordage). **Désactivable dans les options** (« Tutoriel »),
et sautable à tout moment ; son avancement est sauvegardé (`SaveFile`, `#[serde(default)]`).

- Un **panneau d'objectif** (titre court, consigne, touches affichées selon la disposition et le
  remappage : jetons `[KeyE]`) et, quand c'est utile, un **repère** à l'écran (balise, porte,
  console, comme les étiquettes de la vue Globale).
- Des **étapes** qui avancent sur un fait observé, jamais sur une minuterie :
  1. Aux commandes en orbite d'Hexis : regarder, pousser, la croisière.
  2. Rejoindre la balise du spatioport et s'amarrer (`[KeyG]`).
  3. Se lever, sortir par le sas, trouver le bureau d'Hexa-Corp.
  4. Parler au guichetier, prendre un contrat de transport.
  5. Ouvrir le journal (`[KeyJ]`), la fiche (`[KeyC]`), le codex (`[KeyK]`).
  6. Revenir au cockpit, partir (`[KeyG]`), ouvrir la carte (`[KeyM]`), choisir Cygnara.
  7. Voyager par Portail, livrer en orbite près du hub, être payé.
  8. Sauvegarder (`[F5]` ou le menu de pause) : fin du tutoriel.
- Textes dans `assets/data/lang/fr.ron` et `en.ron` (mêmes clés, test existant), sans fait
  d'univers inventé. Un scénario d'autopilote qui joue tout le tutoriel est la preuve de fin.

## 7. Méthode de l'orchestrateur

1. **Préparer** : lire `AGENTS.md`, ce plan, `tools/blender/README.md` ; vérifier Blender
   (`blender --version`) et l'état du dépôt (`git pull`, `tools/check.ps1` vert). Configurer ce
   qu'il faut pour que les sous-agents travaillent sans bloquer (permissions des commandes
   Blender en arrière-plan, dossiers de sortie) ; relire et ajuster les définitions
   `.claude/agents/artiste-blender.md` et `.claude/agents/dev-jeu.md`. Faire les **captures de
   l'avant** à points de vue fixes (`target/shots/m8-avant-*.png`), à refaire à la fin.
2. **Briefer chaque sous-agent** : son lot (§4), ses fichiers, le brief (§2), les contraintes
   (§3), ce qu'il doit rendre. Un sous-agent ne lance **ni** `cargo`, **ni** le jeu, **ni** les
   scripts de vérification, **ni** `git` : il produit et regarde ses rendus Blender.
3. **Point de contrôle après le lot 0** : intégrer la planche de matériaux et une salle témoin
   dans le jeu, faire des captures (`SH_START=sector:player_ship`, `SH_SPAWN_AT`) et **les
   montrer à l'auteur** avant de lancer la production : la direction artistique lui revient
   (AGENTS.md §1, règle 10). Lui demander aussi s'il veut le lot U.
4. **Paralléliser** les vagues (§4). Pendant qu'elles tournent, préparer l'intégration côté Bevy.
5. **Intégrer et tester, seul, à la fin de chaque vague** : relancer les exports si besoin,
   `cargo fmt`, `cargo lint`, `cargo t`, `tools/check.ps1`, captures regardées (les mêmes
   points de vue qu'avant pour comparer), `tools/perf.ps1 -Preset medium -Window fullscreen`.
   Corriger (ou renvoyer au sous-agent avec un retour précis), puis **un commit par lot**.
6. **Fin de M8** : feuille de route cochée, journal, registres, AGENTS.md à jour (dans un commit
   dédié), push, bilan à l'auteur avec les captures avant / après.

## 8. Fini quand

- Les intérieurs (vaisseau, spatioport), les vaisseaux et les installations orbitales ont la
  nouvelle direction ; l'auteur les a vus en jeu et les valide.
- Le tutoriel mène une nouvelle partie jusqu'à la première livraison et la sauvegarde ; il se
  coupe dans les options ; un scénario de l'autopilote le joue en entier.
- `tools/check.ps1` vert ; `tools/perf.ps1` en Moyen 1080p sur l'UHD 770 : ≥ 60 i/s, aucune
  régression des pics.
