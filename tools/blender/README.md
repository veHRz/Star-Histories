# Scripts Blender

Chaque asset du jeu est produit par un script versionné ici (AGENTS.md §9) : paramétrique,
rejouable, sans clic manuel. Le MCP sert à exécuter et à regarder (`look`) ; le script reste la
source de vérité.

| Script | Produit |
|---|---|
| `common.py` | Bibliothèque : scène vide, matériaux partagés, pavés, cylindres, collisions `_col`, repères, fusion par matériau, export glTF, conversion de repère |
| `kit/ship_interior.py` | Kit d'intérieur de vaisseau (P13) : `assets/models/kit/ship_interior/*.glb`, `art/kit/ship_interior.blend` |
| `ships/player_ship.py` | Vaisseau du joueur (P12) : une paire de modèles par cellule dans `assets/models/ships/player_ship/`, le descripteur `assets/data/propositions/sectors/player_ship.ron`, `art/ships/player_ship.blend` |

## Lancer

En arrière-plan (sans interface) :

```powershell
& "$env:LOCALAPPDATA\Programs\Blender Foundation\Blender 5.2\blender.exe" --background --factory-startup --python tools/blender/ships/player_ship.py
```

Dans le Blender ouvert, par le MCP (`execute_blender_code`) :

```python
import sys; sys.path.insert(0, r"D:\Projets\Star Histories\tools\blender")
import common; common.run_in_open_blender("tools/blender/ships/player_ship.py")
```

Après un export : `cargo t` (le test `les_modeles_des_secteurs_existent` et la validation des
secteurs), puis une capture en jeu (`SH_START=sector:player_ship`, `SH_SPAWN=<id>`).

## Conventions relues par le jeu

| Nom (Blender) | Effet dans le jeu (`crates/sh_game/src/sector/gltf.rs`) |
|---|---|
| `*_col` | Collision (enveloppe convexe du maillage), jamais rendue |
| `*_lod1_*` | Version simplifiée (qualité Sectorielle), exportée dans `<cellule>_lod.glb` |
| `LIGHT_<id>` | Lumière ponctuelle ; extras `lumens`, `range`, `color`, `shadow_candidate` |
| `INTERACT_<nom>` | Zone d'interaction ; extras `interact` (`console`, `seat`, `pickup`, `ladder`) et ses paramètres (`label`, `role`, `item`) ; les enfants d'un `pickup` disparaissent avec lui |
| `SOCKET_<nom>_<usage>` | Point d'attache ; extra `usage` (`seat_eye` : œil d'un siège `INTERACT_<nom>`) |
| `CELL_<id>` | Collection d'une cellule (une cellule = un fichier `.glb`) |
| `PORTAL_<a>_<b>`, `SPAWN_<id>` | Portails et apparitions : écrits dans le descripteur RON du secteur par le script |

Repères : 1 unité = 1 m, Z-haut, l'avant d'un objet vers +Y (−Z dans le jeu). Matériaux PBR
partagés, cherchés par type de nœud (jamais par nom : Blender peut être traduit).
