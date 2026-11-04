"""Vitrine du kit d'intérieur (M8, lot A) : un couloir, une salle `ship`, une salle `industrial`
et un coin `station`, habillés avec les props et les éléments du kit, éclairés par des lampes
Blender (une par repère LIGHT_, comme au bake) et par les bandeaux émissifs. Pas de bake, pas
d'export : seulement des aperçus Cycles dans target/art-previews/lotA/.

    blender --background --factory-startup --threads 4 --python tools/blender/kit/vitrine.py -- [--only a,b] [--samples N] [--res 1280x720]

Plan (repère Blender, X = est, Y = nord) :

    salle « ship » (quartiers)            y 10,4 → 16,4
             | porte
    station ─ porte ─ couloir ─ porte ─ salle « industrial » (machines)
    (ouest)          y 0 → 10              (est)
"""

from __future__ import annotations

import math
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "materials"))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

import common as c  # noqa: E402
import lightmap as lm  # noqa: E402
import preview as pv  # noqa: E402
import ship_interior as kit  # noqa: E402

OUT = c.ROOT / "target" / "art-previews" / "lotA"

CELLS = {
    "couloir": (-1.2, 1.2, 0.0, 10.0, 2.6, "ship"),
    "quartiers": (-3.0, 3.0, 10.4, 16.4, 2.8, "ship"),
    "machines": (1.6, 9.6, -2.0, 8.0, 3.4, "industrial"),
    "hall": (-11.6, -1.6, 0.0, 10.0, 4.5, "station"),
}
# Portes : (x, y, axe, cellules, ouverte de 0 à 1, style)
DOORS = [
    (1.4, 3.0, "x", ("couloir", "machines"), 0.0, "ship"),
    (0.0, 10.2, "y", ("couloir", "quartiers"), 0.0, "ship"),
    (-1.4, 7.0, "x", ("couloir", "hall"), 0.0, "station"),
]


def light(name, x, y, h, lumens, rng, color, m, coll, facing=0.0, lens="light", drop=0.0):
    props = {"lumens": lumens, "range": rng, "color": color, "shadow_candidate": 0}
    return kit.ceiling_light(name, (x, y, h), facing, m, coll, props, lens=lens, drop=drop)


def build(m) -> dict:
    cells = {}
    for cid, (x0, x1, y0, y1, h, style) in CELLS.items():
        coll = c.collection(f"CELL_{cid}")
        openings = []
        for x, y, axis, pair, _, _ in DOORS:
            if cid not in pair:
                continue
            if axis == "y":
                openings.append(("N" if abs(y - y1) < abs(y - y0) else "S", x, kit.DOOR_W, kit.DOOR_H))
            else:
                openings.append(("E" if abs(x - x1) < abs(x - x0) else "W", y, kit.DOOR_W, kit.DOOR_H))
        out = {"visual": [], "colliders": [], "marks": [], "structure": []}

        def add(res):
            vis, col, marks = res
            out["visual"] += vis
            out["colliders"] += col
            out["marks"] += marks

        if cid == "couloir":
            lights = [(0.0, 1.0), (0.0, 5.0), (0.0, 8.0)]
            shell = kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, openings, m, coll, floor_strip=True, low_gaps=[("W", 3.0, 0.75)], avoid=lights)
            for i, (x, y) in enumerate(lights):
                add(light(f"{cid}_l{i}", x, y, h, 2500.0, 6.0, kit.COOL, m, coll, facing=math.pi / 2))
            add(kit.locker(f"{cid}_locker", (-0.9, 3.0, 0.0), -math.pi / 2, m, coll))
            add(kit.junction_box(f"{cid}_jbox", (-1.2, 5.0, 1.3), -math.pi / 2, m, coll))
            add(kit.wall_screen(f"{cid}_screen", (1.2, 6.0, 1.55), math.pi / 2, m, coll))
            add(kit.stencil(f"{cid}_stencil", (1.198, 1.0, 1.9), math.pi / 2, m, coll, length=1.4))
            add(kit.vent(f"{cid}_vent", (0.0, 0.0, 1.0), 0.0, m, coll, size=(0.9, 0.3)))
            add(kit.junction_box(f"{cid}_jbox_back", (-0.4, 0.0, 1.55), 0.0, m, coll, size=(0.9, 0.14, 0.7)))
            add(kit.pipe(f"{cid}_back_pipe", (0.75, 0.12, 0.0), (0.75, 0.12, 2.15), m, coll, radius=0.05))
        elif cid == "quartiers":
            lights = [(-1.4, 13.4), (1.4, 13.4)]
            shell = kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, openings, m, coll, avoid=lights)
            for i, (x, y) in enumerate(lights):
                add(light(f"{cid}_l{i}", x, y, h, 4000.0, 7.0, [1.0, 0.93, 0.84], m, coll))
            add(kit.bunk(f"{cid}_bunk", (-2.5, 13.6, 0.0), math.pi / 2, m, coll))
            add(kit.locker(f"{cid}_locker0", (-1.1, 16.08, 0.0), math.pi, m, coll))
            add(kit.locker(f"{cid}_locker1", (-0.45, 16.08, 0.0), math.pi, m, coll))
            add(kit.table(f"{cid}_table", (0.9, 12.9, 0.0), 0.0, m, coll, size=(1.4, 0.8)))
            add(kit.seat(f"{cid}_seat0", (0.5, 12.0, 0.0), 0.0, m, coll))
            add(kit.seat(f"{cid}_seat1", (1.3, 13.8, 0.0), math.pi, m, coll))
            add(kit.pickup(f"{cid}_pickup", (1.0, 12.9, 0.79), 0.3, m, coll, item="datapad"))
            add(kit.console(f"{cid}_console", (2.6, 15.2, 0.0), math.pi / 2, m, coll, label="console"))
            add(kit.wall_screen(f"{cid}_screen", (1.3, 16.4, 1.5), math.pi, m, coll))
            add(kit.junction_box(f"{cid}_jbox", (-3.0, 11.3, 1.3), -math.pi / 2, m, coll))
            add(kit.pipe(f"{cid}_pipe", (2.72, 10.5, 2.36), (2.72, 16.3, 2.36), m, coll, radius=0.045))
            add(kit.vent(f"{cid}_vent", (-1.6, 10.4, 0.45), 0.0, m, coll))
        elif cid == "machines":
            lights = [(3.4, 1.0), (7.9, 1.0), (3.4, 5.0), (7.9, 6.9)]
            shell = kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, openings, m, coll, style="industrial", avoid=lights)
            for i, (x, y) in enumerate(lights):
                add(light(f"{cid}_l{i}", x, y, h, 9000.0, 9.0, kit.AMBER, m, coll, lens="light_amber"))
            add(kit.reactor(f"{cid}_reactor", (6.0, 4.0, 0.0), m, coll, ceiling=h))
            add(kit.hazard_frame(f"{cid}_reactor_zone", (6.0, 4.0, 0.0), (2.7, 2.7), m, coll))
            add(kit.console(f"{cid}_console", (3.6, -1.65, 0.0), 0.0, m, coll, label="engineering"))
            add(kit.crate(f"{cid}_crate0", (2.4, 6.9, 0.0), 0.0, m, coll, size=(1.2, 1.2, 1.2)))
            add(kit.crate(f"{cid}_crate1", (2.45, 6.95, 1.2), 0.2, m, coll, size=(0.7, 0.7, 0.7), variant="rust"))
            add(kit.crate(f"{cid}_crate2", (3.7, 7.2, 0.0), -0.12, m, coll, size=(0.9, 0.9, 0.9), variant="slate"))
            add(kit.pump(f"{cid}_pump", (8.5, -1.45, 0.0), 0.0, m, coll, height=h))
            add(kit.duct(f"{cid}_duct", (9.0, -1.9, 2.75), (9.0, 7.9, 2.75), m, coll, hang_to=h))
            add(kit.pipe(f"{cid}_pipe0", (1.95, -1.95, 2.95), (1.95, 7.95, 2.95), m, coll, radius=0.09, hang_to=h))
            add(kit.pipe(f"{cid}_pipe1", (2.2, -1.95, 3.05), (2.2, 7.95, 3.05), m, coll, radius=0.05, hang_to=h))
            add(kit.cable_tray(f"{cid}_tray", (4.6, -1.95, 3.25), (4.6, 7.95, 3.25), m, coll, hang_to=h))
            add(kit.floor_grating(f"{cid}_grating", (6.0, 0.4, 0.0), (1.0, 2.4), math.pi / 2, m, coll))
            add(kit.beacon(f"{cid}_beacon", (9.39, 6.0, 2.3), math.pi / 2, m, coll))
            add(kit.cabinet(f"{cid}_cabinet", (9.6, 0.9, 0.0), math.pi / 2, m, coll))
            add(kit.light_band(f"{cid}_band", (5.2, -1.95, 2.35), (2.6, -1.95, 2.35), m, coll, normal=(0, 0.6, -0.8), tone="amber"))
        else:
            lights = [(-9.0, 2.2), (-4.2, 2.2), (-9.0, 6.4), (-4.2, 6.4)]
            shell = kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, openings, m, coll, style="station", avoid=lights)
            for i, (x, y) in enumerate(lights):
                add(light(f"{cid}_l{i}", x, y, h, 9000.0, 11.0, kit.WARM, m, coll, drop=0.4))
            for i in range(4):
                add(kit.seat(f"{cid}_bench{i}", (-8.4 + i * 0.75, 4.6, 0.0), 0.0, m, coll))
            add(kit.console(f"{cid}_departures", (-11.25, 6.8, 0.0), -math.pi / 2, m, coll, label="departures"))
            add(kit.wall_screen(f"{cid}_board", (-6.6, 10.0, 2.4), math.pi, m, coll, size=(1.6, 0.8)))
            add(kit.crate(f"{cid}_crate0", (-10.8, 9.2, 0.0), 0.1, m, coll, variant="yellow"))
            add(kit.crate(f"{cid}_crate1", (-9.85, 9.25, 0.0), -0.08, m, coll, size=(0.8, 0.8, 0.7), variant="yellow"))
            add(kit.table(f"{cid}_kiosk", (-5.0, 7.6, 0.0), 0.0, m, coll, size=(1.8, 0.8)))
            add(kit.hazard_stripe(f"{cid}_lane0", (-3.0, 0.6, 0.0), (-3.0, 9.4, 0.0), m, coll, width=0.12))
            add(kit.hazard_stripe(f"{cid}_lane1", (-10.6, 1.0, 0.0), (-4.0, 1.0, 0.0), m, coll, width=0.12))
            add(kit.duct(f"{cid}_duct", (-11.6, 8.9, 3.9), (-1.6, 8.9, 3.9), m, coll, size=(0.7, 0.45), hang_to=h))
            add(kit.vent(f"{cid}_vent", (-8.0, 0.0, 2.6), 0.0, m, coll, size=(1.0, 0.4)))
        out["visual"] = shell["visual"] + out["visual"]
        out["colliders"] = shell["colliders"] + out["colliders"]
        out["structure"] = shell["structure"]
        cells[cid] = out

    # Panneaux de porte, comme le jeu les pose (`doors.rs`) : à la base du portail, +Y du modèle
    # vers la normale du portail, coulissant vers +X local.
    coll = c.collection("DOORS")
    doors = []
    for i, (x, y, axis, pair, openness, style) in enumerate(DOORS):
        vis, col, _ = kit.door_panel(f"door{i}", m, coll, style=style)
        facing = -math.pi / 2 if axis == "x" else 0.0
        if axis == "x" and x < 0:
            facing = math.pi / 2
        for o in vis + col:
            o.location.x += openness * (kit.DOOR_W + 0.05)
        kit._place(vis + col, (x, y, 0.0), facing)
        doors += vis
    cells["_doors"] = {"visual": doors, "colliders": [], "marks": [], "structure": []}
    return cells


# Prises de vue : nom → (œil, cible, focale)
SHOTS = {
    "couloir": ((0.0, 0.45, 1.7), (0.0, 10.0, 1.35), 18),
    "couloir_retour": ((0.3, 9.5, 1.7), (-0.2, 0.0, 1.3), 18),
    "couloir_detail": ((0.5, 3.6, 1.45), (-1.15, 4.7, 0.6), 26),
    "quartiers": ((2.3, 10.9, 1.7), (-2.0, 15.6, 1.0), 17),
    "quartiers_porte": ((-1.6, 15.7, 1.7), (0.4, 10.4, 1.2), 18),
    "machines": ((2.2, -1.35, 1.7), (7.4, 6.4, 1.3), 17),
    "machines_retour": ((9.0, 7.5, 1.7), (3.0, -1.2, 1.2), 17),
    "station": ((-2.4, 0.6, 1.7), (-10.0, 9.0, 2.0), 17),
    "station_porte": ((-5.2, 4.6, 1.7), (-1.6, 7.0, 1.3), 22),
    "porte": ((-0.75, 1.55, 1.55), (1.4, 3.0, 1.15), 24),
    "porte_face": ((-0.5, 3.0, 1.15), (1.4, 3.0, 1.1), 16),
    "console": ((3.6, 0.2, 1.65), (3.6, -1.7, 1.05), 28),
    "siege": ((0.5, 13.5, 1.35), (0.5, 12.0, 0.75), 28),
    "reacteur": ((2.6, 1.1, 1.7), (6.0, 4.0, 1.6), 20),
    "ensemble": ((14.0, -10.0, 16.0), (-1.5, 5.5, 0.0), 22),
}


def stats(cells) -> None:
    for cid, k in cells.items():
        tris = kit._tris(k["visual"])
        mats = sorted({o.data.materials[0].name for o in k["visual"] if o.type == "MESH" and o.data.materials})
        lod = kit._tris(k["structure"])
        print(f"vitrine {cid} : {tris} triangles ({lod} en LOD), {len(mats)} matériaux, {len(k['colliders'])} colliders")


def main(argv) -> None:
    samples = int(argv[argv.index("--samples") + 1]) if "--samples" in argv else 48
    res = argv[argv.index("--res") + 1] if "--res" in argv else "1280x720"
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    rw, rh = (int(v) for v in res.split("x"))
    c.reset_scene()
    m = kit.palette()
    cells = build(m)
    bpy.context.view_layer.update()
    stats(cells)
    c.save_blend("target/art-previews/lotA/vitrine.blend")

    exposure = float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2"))
    pv.setup_cycles(samples, rw, rh, exposure=exposure)
    bpy.context.scene.render.threads_mode = "FIXED"
    bpy.context.scene.render.threads = 4
    pv.world((0, 0, 0), 0.0)
    marks = [mk for k in cells.values() for mk in k["marks"]]
    for lamp in lm.spawn_lamps(marks, c.collection("PREVIEW_LAMPS")):
        lamp.data.shadow_soft_size = 0.06
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH" and obj.data.materials and obj.data.materials[0] and obj.data.materials[0].name.startswith("sh_light"):
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = False
        if obj.name.endswith("_col"):
            obj.hide_render = True
    for name, (eye, target, lens) in SHOTS.items():
        if only and name not in only:
            continue
        hidden = []
        if name == "ensemble":
            # Vue d'ensemble en coupe : plafonds, poutres et tout ce qui est en hauteur masqués.
            for obj in bpy.context.scene.objects:
                if obj.type != "MESH" or obj.hide_render:
                    continue
                zs = [(obj.matrix_world @ Vector(b)).z for b in obj.bound_box]
                if min(zs) > 2.0:
                    obj.hide_render = True
                    hidden.append(obj)
            pv.world((0.6, 0.65, 0.7), 0.35)
        pv.camera("cam", eye, target, lens=lens)
        pv.render(OUT / f"vitrine_{name}.png")
        print(f"aperçu : vitrine_{name}.png")
        for obj in hidden:
            obj.hide_render = False
        if name == "ensemble":
            pv.world((0, 0, 0), 0.0)
    if (not only) or "porte_ambiante" in only:
        # La porte comme en Bas / Moyen : sans lampe, éclairée par une ambiante uniforme.
        # Aucune lampe, et aucun objet n'éclaire ni n'ombre les autres (bandeaux compris) : la
        # seule lumière est le fond uniforme, comme l'ambiante du jeu sur un objet mobile.
        doors = {o.name for o in cells["_doors"]["visual"]}
        for obj in bpy.context.scene.objects:
            if obj.type == "LIGHT":
                obj.hide_render = True
            elif obj.type == "MESH" and obj.name not in doors:
                obj.visible_diffuse = obj.visible_glossy = obj.visible_shadow = False
        pv.world((1.0, 1.0, 1.0), 0.18)
        eye, target, lens = SHOTS["porte_face"]
        pv.camera("cam", eye, target, lens=lens)
        pv.render(OUT / "vitrine_porte_ambiante.png")
        print("aperçu : vitrine_porte_ambiante.png")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
