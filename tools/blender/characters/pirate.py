"""Combattant pirate de la Ligue (M6, proposition P26), à pied : un abordeur.

Silhouette humaine simple et lisible (DA « réaliste stylisé »), ≈ 1,8 m : jambes, torse sous
une veste de plaques dépareillées (identité pirate proposée : assemblage, rouille, rouge sang),
tête casquée à visière, fusil tenu à deux mains. Pas de squelette en M6 : la pose est fixe,
l'accroupi et la chute sont faits par le jeu.

Une unité = 1 m ; l'avant vers +Y ; origine au centre de la base (les pieds).
Sorties : assets/models/characters/pirate.glb et art/characters/pirate.blend.
"""

from __future__ import annotations

import contextlib
import io
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import common as c  # noqa: E402


def palette() -> dict:
    return {
        "cloth": c.material("pirate_cloth", (0.16, 0.14, 0.12), roughness=0.9),
        "plate": c.material("pirate_plate", (0.42, 0.24, 0.14), metallic=0.5, roughness=0.6),
        "blood": c.material("pirate_band", (0.55, 0.06, 0.04), roughness=0.7),
        "skin": c.material("pirate_skin", (0.55, 0.40, 0.30), roughness=0.8),
        "dark": c.material("pirate_gun", (0.06, 0.06, 0.065), metallic=0.7, roughness=0.4),
        "visor": c.material(
            "pirate_visor", (0.9, 0.3, 0.1), emission=(1.0, 0.35, 0.1), emission_strength=4.0
        ),
    }


def pirate(m, coll):
    objs = []
    # Jambes et bottes.
    for side in (-1, 1):
        x = side * 0.12
        objs.append(c.box(f"leg{side}", (0.16, 0.18, 0.82), (x, 0.0, 0.47), m["cloth"], coll, bevel=0.02))
        objs.append(c.box(f"boot{side}", (0.17, 0.28, 0.12), (x, 0.04, 0.06), m["dark"], coll, bevel=0.02))
    # Torse et veste de plaques dépareillées.
    objs.append(c.box("torso", (0.44, 0.26, 0.62), (0.0, 0.0, 1.18), m["cloth"], coll, bevel=0.04))
    objs.append(c.box("plate_front", (0.40, 0.08, 0.36), (0.02, 0.14, 1.26), m["plate"], coll, bevel=0.02))
    objs.append(c.box("plate_side", (0.10, 0.24, 0.30), (-0.25, 0.02, 1.22), m["plate"], coll, bevel=0.02))
    objs.append(c.box("band", (0.46, 0.28, 0.06), (0.0, 0.0, 0.93), m["blood"], coll))
    objs.append(c.box("shoulder", (0.18, 0.22, 0.08), (0.27, 0.0, 1.47), m["blood"], coll, bevel=0.02))
    # Tête casquée, visière.
    objs.append(c.box("neck", (0.12, 0.12, 0.08), (0.0, 0.0, 1.52), m["skin"], coll))
    objs.append(c.box("helmet", (0.24, 0.26, 0.26), (0.0, 0.0, 1.67), m["plate"], coll, bevel=0.05))
    objs.append(c.box("visor", (0.20, 0.04, 0.07), (0.0, 0.13, 1.68), m["visor"], coll))
    # Bras tenant le fusil à hauteur de poitrine.
    for side, (x, y) in ((-1, (-0.22, 0.18)), (1, (0.2, 0.22))):
        objs.append(c.box(f"arm{side}", (0.11, 0.36, 0.11), (x, y, 1.28), m["cloth"], coll, bevel=0.02))
    objs.append(c.box("rifle", (0.07, 0.78, 0.10), (0.04, 0.42, 1.30), m["dark"], coll))
    objs.append(c.box("rifle_mag", (0.05, 0.08, 0.16), (0.04, 0.36, 1.18), m["dark"], coll))
    return objs


def main() -> None:
    c.reset_scene()
    m = palette()
    exports = c.collection("EXPORT")
    coll = c.collection("ASSET_pirate")
    parts = pirate(m, coll)
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        merged = c.join_by_material(parts, "pirate", exports)
        c.export_glb("assets/models/characters/pirate.glb", merged)
    tris = sum(len(p.vertices) - 2 for o in merged for p in o.data.polygons)
    print(f"pirate : {len(merged)} maillages, {tris} triangles")
    c.save_blend("art/characters/pirate.blend")


if __name__ == "__main__":
    main()
