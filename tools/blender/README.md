# Scripts Blender

Chaque asset du jeu est produit par un script versionné ici (AGENTS.md §9) : paramétrique,
rejouable, sans clic manuel. Le MCP sert à exécuter et à regarder (`look`) ; le script reste la
source de vérité.

| Script | Produit |
|---|---|
| `common.py` | Bibliothèque : scène vide, matériaux partagés, pavés, cylindres, collisions `_col`, repères, fusion par matériau, export glTF, conversion de repère. Depuis M8 (lot 0) : `textured_material` (matériau de la bibliothèque, teinte, émissif), `uv_box` (projection boîte à densité constante, `offset` pour décaler chaque travée), `uv_trim` / `trim_band` (bande de la trim sheet), `export_gltf` (glTF séparé, images citées par chemin, tangentes vérifiées), `material_library` |
| `materials/library.py` | Bibliothèque « futur usé » (lot 0, M8) : 15 matériaux procéduraux (numpy, raccords parfaits, déterministes) cuits dans `assets/textures/materials/<mat>_{color,normal,orm,emissive}.png` ; tables `MATERIALS` (taille de tuile, rôle), `TINTS` (teintes d'identité P6), `TRIMS` (bandes de la trim sheet `trim_ship`). Modules : `texgen`, `surfaces`, `devices`, `trim`, `preview` |
| `materials/salle_temoin.py` | Salle témoin (coursive + atelier) : `assets/models/materials/salle_temoin/*.gltf`, lightmaps, descripteur `propositions/sectors/salle_temoin.ron` (`SH_START=sector:salle_temoin`, apparitions `entree`, `atelier`) ; `-- --previews` pour ses rendus |
| `materials/planche.py` | Planche de démonstration des matériaux (`target/art-previews/lot0/planche.png`, `-- --details` pour les gros plans) |
| `lightmap.py` | Éclairage précalculé : 2e jeu d'UV (`lightmap`) par cellule, bake diffus Cycles (une lampe par `LIGHT_`), PNG sRGB normalisés + exposition écrite dans le descripteur |
| `kit/ship_interior.py` | Kit d'intérieur de vaisseau (P13) : `assets/models/kit/ship_interior/*.glb`, `art/kit/ship_interior.blend` |
| `space/hexis_orbitals.py` | Installations orbitales d'Hexis (P15, P18) : `assets/models/space/{l_enclume,portal_hub,shipyard,freighter}.glb`, `art/space/hexis_orbitals.blend` |
| `ships/player_ship.py` | Vaisseau du joueur (P12) : une paire de modèles par cellule dans `assets/models/ships/player_ship/`, une lightmap par cellule dans `assets/textures/lightmaps/player_ship/`, le descripteur `assets/data/propositions/sectors/player_ship.ron`, `art/ships/player_ship.blend` |

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

Après un export : `cargo t` (le test `les_modeles_des_secteurs_existent` et la validation des
secteurs), puis une capture en jeu (`SH_START=sector:player_ship`, `SH_SPAWN=<id>`).

## Conventions relues par le jeu

| Nom (Blender) | Effet dans le jeu (`crates/sh_game/src/sector/gltf.rs`) |
|---|---|
| `*_col` | Collision (enveloppe convexe du maillage), jamais rendue |
| `*_lod1_*` | Version simplifiée (qualité Sectorielle), exportée dans `<cellule>_lod.glb` |
| `LIGHT_<id>` | Lumière ponctuelle ; extras `lumens`, `range`, `color`, `shadow_candidate` (ombre dynamique, seulement sans lightmap). Le bake en fait une lampe Blender de `lumens / 683` W |
| UV `lightmap` | 2e jeu d'UV (TEXCOORD_1, `UV_1` dans Bevy), commun aux deux modèles d'une cellule : le jeu y pose la lightmap (`Lightmap`) |
| `INTERACT_<nom>` | Zone d'interaction ; extras `interact` (`console`, `seat`, `pickup`, `ladder`) et ses paramètres (`label`, `role`, `item`) ; les enfants d'un `pickup` disparaissent avec lui |
| `SOCKET_<nom>_<usage>` | Point d'attache ; extra `usage` (`seat_eye` : œil d'un siège `INTERACT_<nom>`) |
| `CELL_<id>` | Collection d'une cellule (une cellule = un fichier `.glb`) |
| `PORTAL_<a>_<b>`, `SPAWN_<id>` | Portails et apparitions : écrits dans le descripteur RON du secteur par le script |

Repères : 1 unité = 1 m, Z-haut, l'avant d'un objet vers +Y (−Z dans le jeu). Matériaux PBR
partagés, cherchés par type de nœud (jamais par nom : Blender peut être traduit).

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
