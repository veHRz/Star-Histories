"""Aperçus du lot A de M10 : une salle `ship` de 6 × 5 m qui réunit les pièces refaites du kit
(porte blindée du sas et porte coulissante dans le mur sud, tableau électrique, frigo et
rangée de casiers nommés au nord, lit à l'est, casiers décoratifs, boîtier et pochoirs à
l'ouest), éclairée par des lampes Blender (une par repère LIGHT_, comme au bake). Pas de bake,
pas d'export : des rendus Cycles dans target/art-previews/A/.

    blender --background --factory-startup --python tools/blender/kit/apercus_m10.py -- [--only a,b] [--samples N] [--res 1280x720]

Avant / après : `-- --avant <dossier tools/blender d'une copie du kit d'avant>` construit la même
salle avec l'ancien kit (le frigo y est l'ancien « cold_locker », la porte du sas une porte
coulissante) et écrit `avant_<vue>.png` ; `-- --planches` assemble ensuite
`planche_<vue>.png` (avant à gauche, après à droite) pour les vues communes.
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ARGV = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
AVANT = ARGV[ARGV.index("--avant") + 1] if "--avant" in ARGV else None
if AVANT:
    sys.path.insert(0, str(Path(AVANT) / "kit"))
    sys.path.insert(0, str(Path(AVANT)))
else:
    sys.path.insert(0, str(HERE.parent))
    sys.path.insert(0, str(HERE))
sys.path.insert(1, str(HERE.parent / "materials"))

import bpy  # noqa: E402

import common as c  # noqa: E402
import lightmap as lm  # noqa: E402
import preview as pv  # noqa: E402
import ship_interior as kit  # noqa: E402

OUT = Path(__file__).resolve().parents[3] / "target" / "art-previews" / "A"
PREFIX = "avant" if AVANT else "apres"

X0, X1, Y0, Y1, H = -3.0, 3.0, 0.0, 5.0, 2.6
DOORS = [(-1.5, "airlock"), (1.5, "ship")]  # mur sud, x
LAMPS = [(-1.5, 2.5), (1.5, 2.5)]
PI = math.pi


def build(m) -> dict:
    coll = c.collection("SALLE")
    out = {"visual": [], "colliders": [], "marks": []}

    def add(res):
        vis, col, marks = res
        out["visual"] += vis
        out["colliders"] += col
        out["marks"] += marks

    openings = [("S", x, kit.DOOR_W, kit.DOOR_H) for x, _ in DOORS]
    shell = kit.room_shell("salle", X0, X1, Y0, Y1, 0.0, H, openings, m, coll, avoid=LAMPS)
    out["visual"] += shell["visual"]
    out["colliders"] += shell["colliders"]
    for i, (x, y) in enumerate(LAMPS):
        add(kit.ceiling_light(f"salle_l{i}", (x, y, H), PI / 2, m, coll, {"lumens": 3200.0, "range": 7.0, "color": [1.0, 0.95, 0.88], "shadow_candidate": 0}))
    # Nord : tableau électrique, frigo, rangée de casiers nommés.
    add(kit.cabinet("salle_cabinet", (-1.6, Y1, 0.0), PI, m, coll, size=(1.1, 0.32, 1.7)))
    if hasattr(kit, "fridge"):
        add(kit.fridge("salle_fridge", (0.4, Y1 - 0.35, 0.0), PI, m, coll, container="apercu_frigo"))
    else:
        add(kit.locker("salle_cold_locker", (0.4, Y1 - 0.28, 0.0), PI, m, coll))
    add(kit.locker_bank("salle_lockers", (2.0, Y1 - 0.285, 0.0), PI, m, coll, 3, containers="apercu_casier"))
    # Est : lit adossé au mur (−Y local vers +X).
    add(kit.bed("salle_bed", (X1 - 0.5, 2.6, 0.0), PI / 2, m, coll))
    # Ouest : deux casiers décoratifs, boîtier de jonction, pochoir, armoire étroite.
    add(kit.locker("salle_deco0", (X0 + 0.28, 2.0, 0.0), -PI / 2, m, coll))
    add(kit.locker("salle_deco1", (X0 + 0.28, 2.62, 0.0), -PI / 2, m, coll))
    add(kit.junction_box("salle_jbox", (X0, 3.8, 1.4), -PI / 2, m, coll))
    add(kit.stencil("salle_stencil", (X0, 1.0, 1.95), -PI / 2, m, coll, length=0.9))
    add(kit.stencil("salle_stencil2", (X0, 4.3, 2.2), -PI / 2, m, coll, length=0.5))
    add(kit.cabinet("salle_cabinet_w", (X0, 4.4, 0.0), -PI / 2, m, coll, size=(0.9, 0.32, 1.5)))
    # Portes fermées dans le mur sud, comme le jeu les pose (+Y du modèle vers la salle).
    dcoll = c.collection("PORTES")
    for i, (x, style) in enumerate(DOORS):
        if style == "airlock" and not hasattr(kit, "door_airlock"):
            style = "ship"
        vis, col, _ = kit.door_panel(f"door{i}", m, dcoll, style=style)
        kit._place(vis + col, (x, Y0 - kit.WALL / 2, 0.0), 0.0)
        out["visual"] += vis
        for o in col:
            o.hide_render = True
    return out


# Prises de vue : nom → (œil, cible, focale)
SHOTS = {
    "ensemble": ((-0.2, 0.5, 1.7), (0.2, 5.0, 1.0), 15),
    "tableau": ((-1.55, 3.3, 1.6), (-1.6, 5.0, 1.05), 26),
    "tableau_detail": ((-1.45, 4.15, 1.45), (-1.6, 5.0, 1.3), 30),
    "tableau_cote": ((-0.2, 3.9, 1.7), (-1.7, 5.0, 1.0), 24),
    "frigo": ((0.95, 2.3, 1.6), (0.4, 4.5, 1.0), 22),
    "frigo_detail": ((0.2, 3.25, 1.65), (0.4, 4.3, 1.45), 30),
    "casiers": ((2.0, 3.25, 1.6), (2.0, 5.0, 1.05), 24),
    "plaque": ((1.5, 3.8, 1.5), (1.4, 4.43, 1.42), 40),
    "casier_deco": ((-1.3, 2.3, 1.6), (-3.0, 2.3, 1.0), 24),
    "lit": ((0.9, 0.9, 1.7), (2.5, 2.8, 0.45), 22),
    "lit_haut": ((1.2, 1.5, 2.35), (2.55, 2.7, 0.45), 24),
    "portes": ((0.0, 3.6, 1.6), (0.0, 0.0, 1.1), 17),
    "sas": ((-1.0, 2.0, 1.6), (-1.5, 0.0, 1.15), 24),
    "sas_detail": ((-1.25, 1.0, 1.55), (-1.6, 0.0, 1.45), 30),
    "sas_biais": ((0.2, 0.9, 1.75), (-1.5, 0.0, 1.15), 22),
    "pochoirs": ((-1.5, 2.6, 1.75), (-3.0, 1.6, 1.8), 24),
}
BOARDS = ("tableau", "tableau_detail", "frigo", "casiers", "casier_deco", "lit", "lit_haut", "sas", "pochoirs")


def render_all(only, samples, rw, rh) -> None:
    c.reset_scene()
    m = kit.palette()
    k = build(m)
    bpy.context.view_layer.update()
    tris = kit._tris(k["visual"])
    print(f"salle d'aperçu ({PREFIX}) : {tris} triangles, {len(k['colliders'])} colliders")
    pv.setup_cycles(samples, rw, rh, exposure=2.2)
    pv.world((0, 0, 0), 0.0)
    for lamp in lm.spawn_lamps(k["marks"], c.collection("LAMPES")):
        lamp.data.shadow_soft_size = 0.06
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH" and obj.data.materials and obj.data.materials[0] and obj.data.materials[0].name.startswith("sh_light"):
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = False
        if obj.name.endswith("_col"):
            obj.hide_render = True
    for name, (eye, target, lens) in SHOTS.items():
        if only and name not in only:
            continue
        if AVANT and name not in BOARDS:
            continue
        pv.camera("cam", eye, target, lens=lens)
        pv.render(OUT / f"{PREFIX}_{name}.png")
        print(f"aperçu : {PREFIX}_{name}.png")
    if not AVANT and (not only or "sas_ambiante" in only):
        # Les portes comme en Bas / Moyen : sans lampe, ambiante uniforme (objet mobile sans lightmap).
        doors = {o.name for o in bpy.data.collections["PORTES"].objects}
        for obj in bpy.context.scene.objects:
            if obj.type == "LIGHT":
                obj.hide_render = True
            elif obj.type == "MESH" and obj.name not in doors:
                obj.visible_diffuse = obj.visible_glossy = obj.visible_shadow = False
        pv.world((1.0, 1.0, 1.0), 0.18)
        pv.camera("cam", (0.0, 3.6, 1.4), (0.0, 0.0, 1.1), lens=17)
        pv.render(OUT / f"{PREFIX}_portes_ambiante.png")
        print(f"aperçu : {PREFIX}_portes_ambiante.png")


def boards() -> None:
    """Planches avant / après : deux rendus côte à côte, séparés d'une bande sombre."""
    import numpy as np

    for name in BOARDS:
        a, b = OUT / f"avant_{name}.png", OUT / f"apres_{name}.png"
        if not (a.exists() and b.exists()):
            continue
        imgs = []
        for p in (a, b):
            img = bpy.data.images.load(str(p))
            w, h = img.size
            px = np.array(img.pixels[:], dtype=np.float32).reshape(h, w, 4)
            imgs.append(px)
            bpy.data.images.remove(img)
        h, w = imgs[0].shape[:2]
        gap = np.full((h, 12, 4), (0.05, 0.05, 0.05, 1.0), dtype=np.float32)
        board = np.concatenate([imgs[0], gap, imgs[1]], axis=1)
        out = bpy.data.images.new(f"planche_{name}", board.shape[1], board.shape[0], alpha=False)
        out.pixels[:] = board.ravel()
        out.filepath_raw = str(OUT / f"planche_{name}.png")
        out.file_format = "PNG"
        out.save()
        print(f"planche : planche_{name}.png")


def main(argv) -> None:
    if "--planches" in argv:
        boards()
        return
    samples = int(argv[argv.index("--samples") + 1]) if "--samples" in argv else 48
    res = argv[argv.index("--res") + 1] if "--res" in argv else "1280x720"
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    rw, rh = (int(v) for v in res.split("x"))
    render_all(only, samples, rw, rh)


if __name__ == "__main__":
    main(ARGV)
