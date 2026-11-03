"""Coques de combat du raid de M6 (proposition P26), vues de l'extérieur en vue Globale.

- `pirate_corvette` : corvette « de facture pirate » (canon §2.4), ≈ 70 m. Identité visuelle
  proposée (AGENTS.md §9.3) : un assemblage, des plaques dépareillées, une coque asymétrique,
  rouille et rouge sang ;
- `ugl_corvette` : corvette conventionnée de l'Union (classe normalisée, §2.4), ≈ 90 m, de la
  Flotte de défense ; gris bleu, bande bleu pâle (couleurs de l'Union non canon : proposées).

Géométrie simple et lisible (DA « réaliste stylisé »). Une unité = 1 m ; l'avant vers +Y,
origine au centre de masse. Sorties : assets/models/space/<nom>.glb et art/space/warships.blend.
"""

from __future__ import annotations

import contextlib
import io
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import common as c  # noqa: E402

OUT = "assets/models/space"


def palette() -> dict:
    return {
        "rust": c.material("pirate_rust", (0.30, 0.16, 0.10), metallic=0.6, roughness=0.75),
        "scrap": c.material("pirate_scrap", (0.20, 0.20, 0.19), metallic=0.7, roughness=0.6),
        "blood": c.material("pirate_blood", (0.42, 0.04, 0.03), metallic=0.3, roughness=0.6),
        "dark": c.material("warship_dark", (0.06, 0.06, 0.065), metallic=0.6, roughness=0.5),
        "ugl_hull": c.material("ugl_hull", (0.26, 0.30, 0.36), metallic=0.7, roughness=0.45),
        "ugl_plate": c.material("ugl_plate", (0.20, 0.23, 0.28), metallic=0.75, roughness=0.4),
        "ugl_stripe": c.material("ugl_stripe", (0.55, 0.72, 0.92), metallic=0.2, roughness=0.5),
        "red_engine": c.material(
            "pirate_engine", (1.0, 0.35, 0.15), emission=(1.0, 0.35, 0.12), emission_strength=18.0
        ),
        "engine": c.material(
            "ugl_engine", (0.3, 0.6, 1.0), emission=(0.45, 0.7, 1.0), emission_strength=18.0
        ),
        "light": c.material(
            "warship_light", (1.0, 0.95, 0.85), emission=(1.0, 0.9, 0.75), emission_strength=25.0
        ),
    }


def pirate_corvette(m, coll):
    """Corvette pirate, ≈ 70 m : une coque de cargo armée, des plaques rapportées."""
    objs = [
        c.box("pc_hull", (12, 52, 9), (0, 0, 0), m["rust"], coll, bevel=1.0),
        # Proue rapportée, plus étroite et décalée : on voit la soudure.
        c.box("pc_prow", (8, 16, 6), (1.2, 32, -0.5), m["scrap"], coll, bevel=0.8),
        c.box("pc_ram", (3, 6, 3), (1.2, 42, -1), m["blood"], coll, bevel=0.4),
        # Plaques de blindage dépareillées.
        c.box("pc_plate_a", (13, 10, 10), (0, 12, 0.3), m["scrap"], coll, bevel=0.3),
        c.box("pc_plate_b", (12.6, 8, 9.6), (-0.4, -8, 0.6), m["blood"], coll, bevel=0.3),
        c.box("pc_plate_c", (6, 14, 2), (-4, 4, 5.2), m["scrap"], coll),
        # Passerelle de fortune, hors de l'axe.
        c.box("pc_bridge", (6, 8, 4), (-2.5, 18, 6.5), m["rust"], coll, bevel=0.6),
        # Canon lourd en tourelle, et un second sur un bras de bâbord.
        c.cylinder("pc_turret", 2.2, 2, (0, 24, 5.5), m["dark"], coll, axis="Z"),
        c.box("pc_barrel", (0.9, 12, 0.9), (0, 31, 6.2), m["dark"], coll),
        c.box("pc_arm", (10, 2, 1.5), (-10, 2, 0), m["scrap"], coll),
        c.box("pc_pod", (3, 9, 3), (-15, 4, 0), m["rust"], coll, bevel=0.4),
        c.box("pc_engine_block", (16, 10, 11), (0, -30, 0), m["scrap"], coll, bevel=0.8),
    ]
    # Trois tuyères dépareillées.
    for i, (x, z, r) in enumerate(((-5, 0, 3.0), (5, 1, 3.4), (0, -3.5, 2.4))):
        objs.append(c.cylinder(f"pc_nozzle{i}", r, 4, (x, -37, z), m["dark"], coll, axis="Y"))
        objs.append(c.cylinder(f"pc_glow{i}", r * 0.8, 0.6, (x, -39.2, z), m["red_engine"], coll, axis="Y"))
    objs.append(c.box("pc_nav", (0.6, 0.6, 0.6), (-2.5, 22, 8.8), m["light"], coll))
    return objs


def ugl_corvette(m, coll):
    """Corvette de l'Union, ≈ 90 m : classe conventionnée, lignes régulières."""
    objs = [
        c.box("uc_hull", (14, 64, 10), (0, 0, 0), m["ugl_hull"], coll, bevel=1.4),
        c.box("uc_prow", (10, 16, 7), (0, 39, -0.5), m["ugl_plate"], coll, bevel=1.2),
        c.box("uc_stripe", (14.4, 5, 10.4), (0, 26, 0), m["ugl_stripe"], coll),
        c.box("uc_bridge", (8, 12, 5), (0, 16, 7.4), m["ugl_plate"], coll, bevel=0.8),
        c.box("uc_spine", (4, 40, 2.5), (0, -6, 6.2), m["ugl_plate"], coll, bevel=0.4),
        c.box("uc_engine_block", (20, 12, 12), (0, -36, 0), m["ugl_plate"], coll, bevel=1.0),
    ]
    # Deux tourelles de canons lourds, symétriques.
    for i, y in enumerate((30, 4)):
        objs.append(c.cylinder(f"uc_turret{i}", 2.4, 2, (0, y, 6.2 if i else 4.6), m["dark"], coll, axis="Z"))
        objs.append(c.box(f"uc_barrel{i}", (0.9, 12, 0.9), (0, y + 7, 7.0 if i else 5.4), m["dark"], coll))
    # Lance-missiles latéraux.
    for side in (-1, 1):
        objs.append(c.box(f"uc_launcher{side}", (3, 14, 4), (side * 8.5, 6, 0), m["ugl_plate"], coll, bevel=0.4))
    for i, x in enumerate((-6, 0, 6)):
        objs.append(c.cylinder(f"uc_nozzle{i}", 3.0, 4, (x, -44, 0), m["dark"], coll, axis="Y"))
        objs.append(c.cylinder(f"uc_glow{i}", 2.4, 0.6, (x, -46.2, 0), m["engine"], coll, axis="Y"))
    objs.append(c.box("uc_nav", (0.6, 0.6, 0.6), (0, 22, 10.2), m["light"], coll))
    return objs


def main() -> None:
    c.reset_scene()
    m = palette()
    exports = c.collection("EXPORT")
    assets = {
        "pirate_corvette": pirate_corvette,
        "ugl_corvette": ugl_corvette,
    }
    quiet = io.StringIO()
    for name, builder in assets.items():
        coll = c.collection(f"ASSET_{name}")
        parts = builder(m, coll)
        with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
            merged = c.join_by_material(parts, name, exports)
            c.export_glb(f"{OUT}/{name}.glb", merged)
            for obj in merged:
                obj.hide_set(True)
        tris = sum(len(p.vertices) - 2 for o in merged for p in o.data.polygons)
        print(f"{name} : {len(merged)} maillages, {tris} triangles")
    c.save_blend("art/space/warships.blend")


if __name__ == "__main__":
    main()
