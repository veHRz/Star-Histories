# Scripts Blender

Chaque asset du jeu est produit par un script versionné ici (AGENTS.md §9) : paramétrique,
rejouable, sans clic manuel. Le MCP sert à exécuter et à regarder (`look`) ; le script reste la
source de vérité.

| Script | Produit |
|---|---|
| `common.py` | Bibliothèque : scène vide, matériaux partagés, pavés, cylindres, collisions `_col`, repères, fusion par matériau, export glTF, conversion de repère. Depuis M8 (lot 0) : `textured_material` (matériau de la bibliothèque, teinte, émissif), `uv_box` (projection boîte à densité constante, `offset` pour décaler chaque travée), `uv_trim` / `trim_band` (bande de la trim sheet), `export_gltf` (glTF séparé, images citées par chemin, tangentes vérifiées), `material_library`. Depuis M9 (lot A) : `set_detail` / `detail_of` (décor par niveau de qualité), `join_by_material` par (matériau, niveau), `half_extents` / `fit_interact` (volumes d'interaction) |
| `materials/library.py` | Bibliothèque « futur usé » (lot 0, M8) : 15 matériaux procéduraux (numpy, raccords parfaits, déterministes) cuits dans `assets/textures/materials/<mat>_{color,normal,orm,emissive}.png` ; tables `MATERIALS` (taille de tuile, rôle), `TINTS` (teintes d'identité P6), `TRIMS` (bandes de la trim sheet `trim_ship`). Modules : `texgen`, `surfaces`, `devices`, `trim`, `preview` |
| `materials/salle_temoin.py` | Salle témoin (coursive + atelier) : `assets/models/materials/salle_temoin/*.gltf`, lightmaps, descripteur `propositions/sectors/salle_temoin.ron` (`SH_START=sector:salle_temoin`, apparitions `entree`, `atelier`) ; `-- --previews` pour ses rendus |
| `materials/planche.py` | Planche de démonstration des matériaux (`target/art-previews/lot0/planche.png`, `-- --details` pour les gros plans) |
| `lightmap.py` | Éclairage précalculé : faces cachées retirées (`cull_hidden_faces`), 2e jeu d'UV (`lightmap`) par cellule, à la résolution qui tient 18 texels/m (`prepare`, écart de 2,5 px entre îlots), bake diffus Cycles (une lampe par `LIGHT_`), PNG sRGB normalisés + exposition écrite dans le descripteur. M9 : ce qui peut disparaître en jeu (`removable` : décor optionnel `detail` ≥ 1, et tout ce qui est sous un `INTERACT_* pickup`) est cuit sans ombre ni rebond sur les autres ; les `LIGHT_` de niveau ≥ 1 ne sont pas cuits |
| `kit/ship_interior.py` | Kit d'intérieur « futur usé » (P13, lot A de M8) : `room_shell` (styles `ship`, `industrial`, `station`, `hangar` (M9) ; pans coupés, nervures, `avoid=` pour les plafonniers, `floor_holes=` (M9) pour percer le sol, `dressing_detail=`), props historiques (mêmes colliders et repères), pièces d'habillage (`cable_tray`, `pipe`, `duct`, `light_band`, `hazard_stripe`, `ceiling_beam`, `junction_box`, `vent`, `wall_screen`, `stencil`, `beacon`, `hazard_frame`, `floor_grating`, `column`, `cabinet`, `pump`, `crate` : toutes avec `detail=` depuis M9), `apply_uvs` ; portes `door_sliding` et `door_station` (sans écran depuis M9). M9 : volumes d'interaction (sièges, consoles), `bed`, `locker(container=)`, `locker_bank`, `counter` (comptoir de PNJ), `wall_desk`, `shelf`, `cabin_porthole`, mallette provisoire d'objet ramassable réduite et `attach_pickup` (rattachée à son repère). Exporte `assets/models/kit/ship_interior/*.gltf`, `art/kit/ship_interior.blend` |
| `kit/hangar.py` | Pièces de hangar (M9, lot A), aussi accessibles par `ship_interior` (`kit.hangar_shell(...)`) : `hangar_shell` (coque haute, fermes en treillis, pannes, poutres de roulement, projecteurs `LIGHT_`, porte de baie), `floodlight`, `docking_cradle` (berceau : fosse, garde-corps, bras de maintien, sabots, marquages), `gangway` (passerelle plane), `gantry_crane`, `cargo_container`, `crate_stack`, `fuel_bollard` |
| `kit/vitrine.py` | Vitrine du kit (couloir, chambre de capitaine, salle `industrial`, hall `station` et son comptoir, hangar et berceau) rendue dans `target/art-previews/m9-lotA/` (`-- --only a,b --samples 32 --res 1280x720`) |
| `space/hexis_orbitals.py` | Installations orbitales d'Hexis (P15, P18 ; lot E de M8) : `assets/models/space/{l_enclume,portal_hub,shipyard,freighter}.gltf`, texture `orbital_lights_*`, `art/space/hexis_orbitals.blend` (`-- --textures --previews --only a,b`). Émissifs calibrés pour le jeu : l'émissif n'est pas atténué par l'exposition de la caméra (≈ 4 pour un feu, 1 à 2 pour un hublot) |
| `space/warships.py` | Corvettes pirate et de l'Union (P26 ; lot D de M8) : `assets/models/space/{pirate,ugl}_corvette.gltf`, textures `warship_rust_*` et `warship_lights_*`, `art/space/warships.blend` (`-- --previews`) |
| `characters/pirate.py`, `characters/mag_pistol.py` | Abordeur pirate et pistolet magnétique du joueur (P26 ; lot F de M8) : `assets/models/characters/pirate.gltf`, `weapons/mag_pistol.gltf`, sources dans `art/characters/` ; textures `char_gear_*` et `char_cloth_*` par `char_atlas.py` et `char_cloth.py` (à lancer avant, ou régénérées si absentes) ; `char_kit.py` (outils), `char_preview.py` (rendus) |
| `items/*.py` | Objets du catalogue (lot F de M9) : `item_atlas.py` (atlas `item_gear`, régénéré s'il manque), `items.py -- [--only a,b]` (modèles `assets/models/items/<id>.gltf`, sources `art/items/`), `icons.py -- [--only …] [--samples 64]` (icônes 256 px `assets/ui/icons/items/<id>.png` et planche), `previews.py` (aperçus) ; `item_kit.py` (outils communs) |
| `characters/clerks.py` (+ `clerk_kit.py`, `clerk_face.py`, `clerk_atlas.py`) | PNJ de comptoir (lot E de M9) : `assets/models/characters/clerk_{a,b,c}.gltf` (a : guichetière, contrats et banque ; b : marchand ; c : tenancier), textures `char_clerk_*`, `art/characters/clerks.blend` (`-- --atlas --previews`). Posés par le jeu sur un `SOCKET_<nom>_npc` (`kit.counter` : plateau à 1,05 m, bord côté employé à 0,195 m devant le socket) |
| `ships/player_ship.py` | Vaisseau du joueur (P12 ; lot B de M8) : une paire de modèles par cellule (`<cellule>.gltf`, `<cellule>_lod.gltf`) dans `assets/models/ships/player_ship/`, une lightmap par cellule dans `assets/textures/lightmaps/player_ship/`, le descripteur `assets/data/propositions/sectors/player_ship.ron`, `art/ships/player_ship.blend` (`-- --no-bake --no-export --lit --game --only a,b --samples N`). Dépliage de `lightmap.py` (`prepare_lightmap`), avec l'estrade du cockpit comme second sol |
| `places/hexis_spaceport.py`, `sector_tools.py` | Spatioport d'Hexis (P7 ; lot C de M8) et outils de secteur : cellules en style `station`, export glTF séparé (`export_sector`, colliders renommés au nom unique), descripteur, lightmaps, aperçus « comme en jeu » (`-- --no-bake --lit --only a,b --res --samples`). M9 : noms de salle (`meta["rooms"]` → `name_key: Some("room.<usage>")`, `room_key`, `ROOM_USAGES`), `half_extents` / `fit_interact`, `attach_pickup`, LOD sans décor optionnel |

## Lancer

En arrière-plan (sans interface), sous Windows puis sous Linux :

```powershell
& "$env:LOCALAPPDATA\Programs\Blender Foundation\Blender 5.2\blender.exe" --background --factory-startup --python tools/blender/ships/player_ship.py
```

```bash
blender --background --factory-startup --python tools/blender/ships/player_ship.py
```

Dans le Blender ouvert, par le MCP (`execute_blender_code`) :

```python
import sys; sys.path.insert(0, r"<racine du dépôt>/tools/blender")  # ex. D:\Projets\Star Histories, ~/Projets/Star-Histories
import common; common.run_in_open_blender("tools/blender/ships/player_ship.py")
```

Le bake Cycles (CPU) prend ≈ 10 min à 512 échantillons sur le poste Linux (i9-9900K) : lance-le
en arrière-plan plutôt que par le MCP, qui expire avant la fin. `SH_BAKE_SAMPLES=64` donne un
aperçu en ≈ 1 min (un peu de bruit). **Toute modification de géométrie impose un nouveau bake** :
les UV de lightmap changent avec elle.

Sur le poste Windows, Blender peut quitter avec le code 5 après des rendus EEVEE (plantage du
pilote Intel à la fermeture) alors que tous les fichiers sont écrits : vérifier les sorties et la
fin du journal plutôt que le code de retour. Rediriger la sortie de Blender vers un journal
(`target/art-previews/<lot>/…log`) : la console est bavarde.

Après un export : `cargo t` (le test `les_modeles_des_secteurs_existent` et la validation des
secteurs), puis une capture en jeu (`SH_START=sector:player_ship`, `SH_SPAWN=<id>`).

## Conventions relues par le jeu

| Nom (Blender) | Effet dans le jeu (`crates/sh_game/src/sector/gltf.rs`) |
|---|---|
| `*_col` | Collision (enveloppe convexe du maillage), jamais rendue |
| `*_lod1_*` | Version simplifiée (qualité Sectorielle), exportée dans `<cellule>_lod.gltf` |
| `LIGHT_<id>` | Lumière ponctuelle ; extras `lumens`, `range`, `color`, `shadow_candidate` (ombre dynamique, seulement sans lightmap). Le bake en fait une lampe Blender de `lumens / 683` W |
| UV `lightmap` | 2e jeu d'UV (TEXCOORD_1, `UV_1` dans Bevy), commun aux deux modèles d'une cellule : le jeu y pose la lightmap (`Lightmap`) |
| `INTERACT_<nom>` | Zone d'interaction ; extras `interact` et ses paramètres : `console` (`label`), `seat` (`role`), `pickup` (`item`), `ladder` ; M9 : `locker` (`container` : identifiant unique du contenu, ex. `corvette_chambre_casier_1`), `npc` (`role` : `contracts`, `bank`, `market`, `shipyard`, `bar`… ; `variant` : `a`, `b`, `c`) |
| `half_extents` (M9) | Extra `[x, y, z]` d'un `INTERACT_*` : capteur en pavé centré sur le nœud, demi-tailles en mètres dans le repère local du nœud (Y-haut) ; absent : sphère de 0,35 m (voir plus bas) |
| `pickup` (M9) | Le modèle de l'objet (`models/items/<item>.gltf`, posé 0,15 m sous le repère) remplace les enfants du `INTERACT_* pickup` (la mallette provisoire du kit, rattachée par `attach_pickup`) ; un objet pris ne revient plus (sauvegardé). Dans le bake, ce qui est sous le repère ne fait pas d'ombre |
| `SOCKET_<nom>_<usage>` | Point d'attache ; extra `usage` (`seat_eye` : œil d'un siège `INTERACT_<nom>`, nœud `SOCKET_<nom>_eye` ; M9 : `npc` : pied d'un PNJ de comptoir, nœud `SOCKET_<nom>_npc`, extras `role` et `variant`, regard vers +Y Blender : le jeu y pose `models/characters/clerk_<variant>.gltf`) |
| `detail` (M9) | Extra entier d'un nœud de décor : absent ou 0 = toujours affiché, 1 = Moyen et Haut, 2 = Haut seulement ; le jeu cache le nœud et ses enfants au-delà du `decor_detail` du préréglage, changement d'option appliqué en direct |
| `CELL_<id>` | Collection d'une cellule (une cellule = un fichier `.glb`) |
| `PORTAL_<a>_<b>`, `SPAWN_<id>` | Portails et apparitions : écrits dans le descripteur RON du secteur par le script |
| `name_key` de cellule (M9) | `meta["rooms"] = {cellule: usage}` du script de lieu → `name_key: Some("room.<usage>")` dans le descripteur (`sector_tools.room_key`) |

Repères : 1 unité = 1 m, Z-haut, l'avant d'un objet vers +Y (−Z dans le jeu). Matériaux PBR
partagés, cherchés par type de nœud (jamais par nom : Blender peut être traduit).

## Volumes d'interaction (M9)

- Un `INTERACT_*` qui porte l'extra `half_extents: [x, y, z]` devient dans le jeu un capteur en
  **pavé centré sur le nœud** (sinon une sphère de 0,35 m). Demi-tailles en mètres, dans le
  **repère local du nœud** (le pavé tourne avec l'objet), converti comme l'export glTF :
  `x` = x Blender, `y` = la hauteur (z Blender), `z` = la profondeur (y Blender).
- Le pavé couvre tout l'objet, avec une marge de 3 à 4 cm : le rayon de visée touche le capteur
  avant le collider de l'objet. Le kit place le repère au centre du volume (`_interact`) ; pour un
  objet fait à la main, `common.fit_interact(repère, objets)` recentre et écrit l'extra.
- Tout siège (`role` : `pilot`, `copilot`, `passenger`…, avec son `SOCKET_<nom>_eye`), toute
  console, tout casier (`container`) et tout comptoir de PNJ (`role`, `variant`, avec son
  `SOCKET_<nom>_npc`) en a un ; `export_kit` le vérifie.

## Décor par niveau de qualité (M9)

- Propriété personnalisée `detail` (entier, `common.set_detail`, exportée en extra glTF) sur les
  objets de décor : 0 ou absente = toujours, 1 = Moyen et Haut, 2 = Haut seulement. Les pièces
  d'habillage du kit prennent `detail=` (défaut 0 : comportement de M8) ; `room_shell` prend
  `dressing_detail=`, `shelf` et `wall_desk` `items_detail=`, `hangar_shell` `purlin_detail=`
  (1 par défaut) et `web_detail=`, `docking_cradle` `arms_detail=` et `markings_detail=`.
- Règles de production : un décor optionnel n'a **pas de collider** (le kit les supprime), n'est
  **jamais fusionné** avec un autre niveau (`join_by_material` groupe par matériau et par
  niveau : `<cellule>_<matériau>_d<n>`), ne va pas dans la version simplifiée (LOD) de la
  cellule, et dans le bake il **ne projette pas d'ombre** et ne renvoie pas de lumière (sa
  disparition en Bas ne laisse rien de fantôme) ; il reçoit sa lightmap comme le reste. Un
  `LIGHT_` de niveau ≥ 1 n'est pas cuit (lumière dynamique du Haut seulement). Un collider ou un
  repère de gameplay (`INTERACT_`, `SOCKET_`, `PORTAL_`, `SPAWN_`, `CELL_`) ne peut pas être
  optionnel : le jeu se joue pareil en Bas.
- Le niveau 0 (ce que voit le Bas) doit rester beau et léger ; le niveau 2 peut être riche.

## Textures partagées (M8)

- Les modèles texturés s'exportent en **glTF séparé** (`export_gltf` : `.gltf` + `.bin`) ; leurs
  images restent dans `assets/textures/materials/`, citées par chemin relatif, jamais copiées.
  Le jeu (`crates/sh_game/src/materials.rs`) précharge toute la bibliothèque, calcule les
  mipmaps et règle l'échantillonnage (répétition, anisotrope du préréglage).
- Contrat de nommage : `_color` et `_emissive` en sRGB, `_normal` (OpenGL) et `_orm` (R occlusion,
  G rugosité, B métal) en linéaire. Textures sur UV0, lightmap sur le 2e jeu (`lightmap`).
- Dans une cellule à lightmap en Bas et Moyen, seul le diffus est éclairé (lightmap × couleur ×
  (1 − métal)) : le métal des ORM est plafonné à 0,55 et 70 % de l'occlusion est recopiée dans la
  couleur. Les plafonniers (matériaux `sh_light*`) sont remplacés par leurs lampes au bake ; les
  bandeaux (`sh_band_*`) éclairent dans le bake.
