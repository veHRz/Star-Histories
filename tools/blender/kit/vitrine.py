"""Vitrine du kit d'intérieur (M8 lot A, étendue en M9 lot A) : un couloir, une chambre de
capitaine (`ship` : couchette, casiers, bureau mural, étagère, hublot), une salle `industrial`, un
hall `station` avec son comptoir d'accueil, et un hangar (`hangar_shell`, berceau, passerelle,
portique, conteneurs, bornes), habillés avec les pièces du kit, éclairés par des lampes Blender
(une par repère LIGHT_, comme au bake) et par les bandeaux émissifs. Pas de bake, pas d'export :
seulement des aperçus Cycles dans target/art-previews/m9-lotA/.

    blender --background --factory-startup --threads 4 --python tools/blender/kit/vitrine.py -- [--only a,b] [--samples N] [--res 1280x720]

Repères d'aperçu seulement (jamais dans le kit) : une silhouette grise de PNJ (≈ 1,8 m) sur le
point d'attache du comptoir, une coque grise dans le berceau (masquée pour le gros plan).

Plan (repère Blender, X = est, Y = nord) :

    chambre (y 10,4 → 16,4)                           hangar (x 12 → 38, y −8 → 28)
             | porte                                   berceau au centre, porte de baie au nord
    hall ─ porte ─ couloir ─ porte ─ machines
    (ouest)        y 0 → 10            (est)
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
import hangar as hg  # noqa: E402
import lightmap as lm  # noqa: E402
import preview as pv  # noqa: E402
import ship_interior as kit  # noqa: E402

OUT = c.ROOT / "target" / "art-previews" / "m9-lotA"

CELLS = {
    "couloir": (-1.2, 1.2, 0.0, 10.0, 2.6, "ship"),
    "chambre": (-3.0, 3.0, 10.4, 16.4, 2.8, "ship"),
    "machines": (1.6, 9.6, -2.0, 8.0, 3.4, "industrial"),
    "hall": (-11.6, -1.6, 0.0, 10.0, 4.5, "station"),
    "hangar": (12.0, 38.0, -8.0, 28.0, 12.0, "hangar"),
}
# Portes : (x, y, axe, cellules, ouverte de 0 à 1, style)
DOORS = [
    (1.4, 3.0, "x", ("couloir", "machines"), 0.0, "ship"),
    (0.0, 10.2, "y", ("couloir", "chambre"), 0.0, "ship"),
    (-1.4, 7.0, "x", ("couloir", "hall"), 0.0, "station"),
    (12.0, 2.0, "x", ("hangar",), 0.0, "station"),
]
# Berceau du hangar (fosse) et passerelle vers la coque.
PIT = (20.0, 30.0, 0.0, 20.0)
PIT_GAP = ("W", 10.0, 1.8)


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
        out = {"visual": [], "colliders": [], "marks": [], "structure": [], "preview": []}

        def add(res):
            vis, col, marks = res
            out["visual"] += vis
            out["colliders"] += col
            out["marks"] += marks

        if cid == "couloir":
            lights = [(0.0, 1.0), (0.0, 5.0), (0.0, 8.0)]
            shell = kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, openings, m, coll, floor_strip=True, low_gaps=[("W", 3.0, 0.75)], avoid=lights,
                                   dressing_detail=1)
            for i, (x, y) in enumerate(lights):
                add(light(f"{cid}_l{i}", x, y, h, 2500.0, 6.0, kit.COOL, m, coll, facing=math.pi / 2))
            add(kit.locker(f"{cid}_locker", (-0.9, 3.0, 0.0), -math.pi / 2, m, coll, container="vitrine_couloir_casier"))
            add(kit.junction_box(f"{cid}_jbox", (-1.2, 5.0, 1.3), -math.pi / 2, m, coll))
            add(kit.wall_screen(f"{cid}_screen", (1.2, 6.0, 1.55), math.pi / 2, m, coll))
            add(kit.stencil(f"{cid}_stencil", (1.198, 1.0, 1.9), math.pi / 2, m, coll, length=1.4))
            add(kit.vent(f"{cid}_vent", (0.0, 0.0, 1.0), 0.0, m, coll, size=(0.9, 0.3)))
            add(kit.junction_box(f"{cid}_jbox_back", (-0.4, 0.0, 1.55), 0.0, m, coll, size=(0.9, 0.14, 0.7)))
            add(kit.pipe(f"{cid}_back_pipe", (0.75, 0.12, 0.0), (0.75, 0.12, 2.15), m, coll, radius=0.05))
        elif cid == "chambre":
            # Chambre du capitaine : couchette à l'ouest, casiers au nord, bureau mural et hublot
            # à l'est, étagère au nord-est.
            lights = [(-1.4, 13.4), (1.4, 13.4)]
            shell = kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, openings, m, coll, avoid=lights)
            for i, (x, y) in enumerate(lights):
                add(light(f"{cid}_l{i}", x, y, h, 3200.0, 7.0, [1.0, 0.93, 0.84], m, coll))
            # Nervure du mur ouest à y = 12,4 : x local = −(12,4 − 12,6) = 0,2.
            add(kit.bed(f"{cid}_bed", (-2.5, 12.6, 0.0), -math.pi / 2, m, coll, pad_gaps=[0.2]))
            add(kit.locker_bank(f"{cid}_lockers", (-0.9, 16.4 - 0.275, 0.0), math.pi, m, coll, 3, containers="vitrine_chambre_casier"))
            add(kit.shelf(f"{cid}_shelf", (1.5, 16.4, 0.0), math.pi, m, coll, width=1.0, items_detail=1))
            add(kit.wall_desk(f"{cid}_desk", (3.0, 13.6, 0.0), math.pi / 2, m, coll, width=1.3))
            add(kit.seat(f"{cid}_seat", (2.05, 13.6, 0.0), -math.pi / 2, m, coll, role="passenger"))
            add(kit.cabin_porthole(f"{cid}_porthole", (3.0, 11.7, 1.55), math.pi / 2, m, coll))
            add(kit.pickup(f"{cid}_pickup", (2.6, 13.25, 0.78), 0.4, m, coll, item="datapad"))
            add(kit.junction_box(f"{cid}_jbox", (-3.0, 15.3, 1.5), -math.pi / 2, m, coll, detail=1))
            add(kit.pipe(f"{cid}_pipe", (2.72, 10.5, 2.36), (2.72, 16.3, 2.36), m, coll, radius=0.045, detail=2))
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
            add(kit.crate(f"{cid}_crate1", (2.45, 6.95, 1.2), 0.2, m, coll, size=(0.7, 0.7, 0.7), variant="rust", detail=1))
            add(kit.crate(f"{cid}_crate2", (3.7, 7.2, 0.0), -0.12, m, coll, size=(0.9, 0.9, 0.9), variant="slate"))
            add(kit.pump(f"{cid}_pump", (8.5, -1.45, 0.0), 0.0, m, coll, height=h))
            add(kit.duct(f"{cid}_duct", (9.0, -1.9, 2.75), (9.0, 7.9, 2.75), m, coll, hang_to=h, detail=1))
            add(kit.pipe(f"{cid}_pipe0", (1.95, -1.95, 2.95), (1.95, 7.95, 2.95), m, coll, radius=0.09, hang_to=h))
            add(kit.pipe(f"{cid}_pipe1", (2.2, -1.95, 3.05), (2.2, 7.95, 3.05), m, coll, radius=0.05, hang_to=h, detail=2))
            add(kit.cable_tray(f"{cid}_tray", (4.6, -1.95, 3.25), (4.6, 7.95, 3.25), m, coll, hang_to=h, detail=1))
            add(kit.floor_grating(f"{cid}_grating", (6.0, 0.4, 0.0), (1.0, 2.4), math.pi / 2, m, coll))
            add(kit.beacon(f"{cid}_beacon", (9.39, 6.0, 2.3), math.pi / 2, m, coll))
            add(kit.cabinet(f"{cid}_cabinet", (9.6, 0.9, 0.0), math.pi / 2, m, coll))
            add(kit.light_band(f"{cid}_band", (5.2, -1.95, 2.35), (2.6, -1.95, 2.35), m, coll, normal=(0, 0.6, -0.8), tone="amber"))
        elif cid == "hall":
            lights = [(-9.0, 2.2), (-4.2, 2.2), (-9.0, 6.4), (-4.2, 6.4)]
            shell = kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, openings, m, coll, style="station", avoid=lights)
            for i, (x, y) in enumerate(lights):
                add(light(f"{cid}_l{i}", x, y, h, 9000.0, 11.0, kit.WARM, m, coll, drop=0.4))
            for i in range(4):
                add(kit.seat(f"{cid}_bench{i}", (-8.4 + i * 0.75, 3.4, 0.0), 0.0, m, coll))
            add(kit.console(f"{cid}_departures", (-11.25, 4.8, 0.0), -math.pi / 2, m, coll, label="departures"))
            add(kit.wall_screen(f"{cid}_board", (-4.0, 10.0, 2.6), math.pi, m, coll, size=(1.6, 0.8)))
            # Comptoir d'accueil contre le mur nord : client au sud, PNJ entre le comptoir et le mur.
            add(kit.counter(f"{cid}_counter", (-7.6, 8.4, 0.0), math.pi, m, coll, length=2.8, role="contracts", variant="a"))
            add(kit.crate(f"{cid}_crate0", (-10.9, 1.0, 0.0), 0.1, m, coll, variant="yellow"))
            add(kit.crate(f"{cid}_crate1", (-10.0, 0.9, 0.0), -0.08, m, coll, size=(0.8, 0.8, 0.7), variant="yellow", detail=1))
            add(kit.hazard_stripe(f"{cid}_lane0", (-3.0, 0.6, 0.0), (-3.0, 9.4, 0.0), m, coll, width=0.12))
            add(kit.hazard_stripe(f"{cid}_queue", (-9.2, 6.6, 0.0), (-6.0, 6.6, 0.0), m, coll, width=0.12))
            add(kit.duct(f"{cid}_duct", (-11.6, 9.4, 3.9), (-1.6, 9.4, 3.9), m, coll, size=(0.7, 0.45), hang_to=h, detail=1))
            add(kit.vent(f"{cid}_vent", (-8.0, 0.0, 2.6), 0.0, m, coll, size=(1.0, 0.4)))
            out["preview"] += npc_proxy(f"{cid}_counter", out["marks"])
        else:
            shell = hg.hangar_shell(cid, x0, x1, y0, y1, h, openings, m, coll, floor_holes=[PIT], bay_door=("N", 25.0, 14.0, 9.0))
            out["marks"] += shell["marks"]
            add(hg.docking_cradle(f"{cid}_cradle", *PIT, m, coll, gaps=[PIT_GAP], saddle_height=0.6))
            add(hg.gangway(f"{cid}_gangway", (19.45, 10.0, 0.0), (22.05, 10.0, 0.0), m, coll, width=1.6))
            add(hg.gantry_crane(f"{cid}_gantry", (34.0, 16.0, 0.0), 0.0, m, coll, span=6.0, height=8.0, rail_length=16.0, hook_z=3.4))
            add(hg.cargo_container(f"{cid}_cont0", (34.0, 12.0, 0.0), math.pi / 2, m, coll, variant="rust"))
            add(hg.cargo_container(f"{cid}_cont1", (34.0, 19.6, 0.0), math.pi / 2, m, coll, variant="slate"))
            add(hg.cargo_container(f"{cid}_cont2", (34.0, 19.6, 2.6), math.pi / 2 + 0.02, m, coll, variant="yellow", detail=1))
            add(hg.crate_stack(f"{cid}_crates0", (15.2, 23.5, 0.0), 0.3, m, coll))
            add(hg.crate_stack(f"{cid}_crates1", (14.0, -4.5, 0.0), -0.1, m, coll, variants=("yellow", "slate", "olive"), detail=1))
            # Bornes au bord est de la fosse, face au vaisseau (vers −X).
            add(hg.fuel_bollard(f"{cid}_fuel", (31.8, 2.5, 0.0), math.pi / 2, m, coll, hose_to=(0.5, 0.95)))
            add(hg.fuel_bollard(f"{cid}_power", (31.8, 5.5, 0.0), math.pi / 2, m, coll, kind="power"))
            add(kit.cabinet(f"{cid}_cabinet", (12.0, 6.5, 0.0), -math.pi / 2, m, coll))
            add(kit.wall_screen(f"{cid}_screen", (12.0, 8.6, 1.7), -math.pi / 2, m, coll, size=(1.2, 0.7)))
            add(kit.hazard_stripe(f"{cid}_lane0", (13.5, -2.5, 0.0), (36.5, -2.5, 0.0), m, coll, width=0.14))
            add(kit.hazard_stripe(f"{cid}_lane1", (17.5, -6.5, 0.0), (17.5, 26.5, 0.0), m, coll, width=0.14))
            out["preview"] += hull_proxy()
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
    cells["_doors"] = {"visual": doors, "colliders": [], "marks": [], "structure": [], "preview": []}
    return cells


def npc_proxy(counter_name, marks) -> list:
    """Silhouette grise (≈ 1,8 m) sur le point d'attache `SOCKET_<comptoir>_npc` : l'échelle du
    PNJ derrière le comptoir. Aperçu seulement."""
    sock = next(mk for mk in marks if mk.name == f"SOCKET_{counter_name}_npc")
    bpy.context.view_layer.update()
    p = sock.matrix_world.translation
    mat = c.material("preview_npc", (0.42, 0.43, 0.46), roughness=0.7)
    coll = c.collection("PREVIEW")
    out = [
        # Cotes du lot E : le corps avance jusqu'à 0,19 m devant le socket.
        c.cylinder("npc_legs", 0.13, 0.85, (p.x, p.y, 0.425), mat, coll, segments=12),
        c.cylinder("npc_body", 0.19, 0.65, (p.x, p.y, 1.17), mat, coll, segments=12),
        c.cylinder("npc_head", 0.1, 0.24, (p.x, p.y, 1.66), mat, coll, segments=12),
    ]
    # Mains posées à 1,05 m, entre 0,21 et 0,31 m devant le socket (vers +Y local).
    for sx in (-1, 1):
        q = sock.matrix_world @ Vector((sx * 0.17, 0.26, 1.065))
        hand = c.box(f"npc_hand{sx}", (0.08, 0.1, 0.03), tuple(q), mat, coll)
        hand.rotation_euler[2] = sock.matrix_world.to_euler()[2]
        out.append(hand)
    return out


def hull_proxy() -> list:
    """Coque grise dans le berceau (pont au niveau du sol) : la passerelle y aboutit. Aperçu
    seulement (la vraie coque est le lot C)."""
    mat = c.material("preview_hull", (0.30, 0.32, 0.35), metallic=0.3, roughness=0.5)
    coll = c.collection("PREVIEW")
    return [
        c.box("hull_lower", (5.2, 17.0, 2.2), (25.0, 10.0, -1.25), mat, coll, bevel=0.6),
        c.box("hull_upper", (6.0, 18.0, 3.4), (25.0, 10.0, 1.6), mat, coll, bevel=0.5),
    ]


# Prises de vue : nom → (œil, cible, focale[, objets d'aperçu masqués])
SHOTS = {
    "ensemble": ((15.0, -14.0, 78.0), (13.5, 9.0, 0.0), 24),
    "couloir": ((0.0, 0.45, 1.7), (0.0, 10.0, 1.35), 18),
    "chambre": ((0.6, 10.9, 1.7), (-1.6, 15.6, 0.9), 17),
    "chambre_retour": ((-1.0, 15.6, 1.7), (2.2, 11.4, 1.0), 17),
    "lit": ((-0.4, 11.4, 1.45), (-2.5, 12.6, 0.55), 24),
    "casiers": ((-0.6, 13.4, 1.6), (-0.9, 16.2, 1.05), 22),
    "bureau": ((0.2, 12.6, 1.65), (3.0, 13.0, 1.1), 22),
    "comptoir": ((-7.0, 5.0, 1.7), (-7.6, 8.6, 1.05), 22),
    "comptoir_cote": ((-4.4, 6.6, 1.9), (-7.6, 8.7, 0.95), 24),
    # Profil : plateau à 1,05 m, PNJ à 0,195 m derrière le bord, mains sur le plateau (lot E).
    "comptoir_profil": ((-4.4, 8.6, 1.05), (-7.6, 8.6, 1.0), 32),
    "porte": ((-0.75, 1.55, 1.55), (1.4, 3.0, 1.15), 24),
    "porte_face": ((-0.5, 3.0, 1.15), (1.4, 3.0, 1.1), 16),
    "porte_station": ((-4.0, 7.0, 1.2), (-1.6, 7.0, 1.1), 20),
    "hangar": ((13.2, -7.0, 9.0), (27.0, 14.0, 0.0), 16),
    "hangar_oeil": ((14.5, 2.5, 1.7), (24.0, 14.0, 1.6), 17),
    "hangar_baie": ((16.0, 20.5, 1.8), (26.0, 28.0, 4.0), 17),
    "berceau": ((19.0, -2.6, 4.2), (25.5, 6.5, -2.2), 18, ("hull_lower", "hull_upper")),
    "passerelle": ((16.8, 7.3, 1.7), (21.5, 10.0, 0.6), 22),
    "portique": ((28.8, 25.5, 2.0), (35.0, 14.0, 4.0), 17),
    "bornes": ((30.95, -1.2, 1.65), (31.6, 4.0, 0.8), 20),
    "machines": ((2.2, -1.35, 1.7), (7.4, 6.4, 1.3), 17),
    # Comme en Bas : décor de niveau ≥ 1 masqué (`detail`).
    "chambre_bas": ((-1.0, 15.6, 1.7), (2.2, 11.4, 1.0), 17, "bas"),
    "machines_bas": ((2.2, -1.35, 1.7), (7.4, 6.4, 1.3), 17, "bas"),
}


def stats(cells) -> None:
    for cid, k in cells.items():
        tris = kit._tris(k["visual"])
        by_level = {}
        for o in k["visual"]:
            if o.type == "MESH":
                by_level[c.detail_of(o)] = by_level.get(c.detail_of(o), 0) + kit._tris([o])
        mats = sorted({o.data.materials[0].name for o in k["visual"] if o.type == "MESH" and o.data.materials})
        lod = kit._tris(k["structure"])
        levels = ", ".join(f"niveau {lv} : {n}" for lv, n in sorted(by_level.items()))
        print(f"vitrine {cid} : {tris} triangles ({levels} ; {lod} en LOD), {len(mats)} matériaux, {len(k['colliders'])} colliders")


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
    c.save_blend("target/art-previews/m9-lotA/vitrine.blend")

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
    for name, shot in SHOTS.items():
        if only and name not in only:
            continue
        eye, target, lens = shot[:3]
        hidden = []
        spec = shot[3] if len(shot) > 3 else ()
        if spec == "bas":
            targets = [o for o in bpy.context.scene.objects if o.type == "MESH" and c.detail_of(o) > 0]
        else:
            targets = [bpy.data.objects.get(pname) for pname in spec]
        for obj in targets:
            if obj is not None and not obj.hide_render:
                obj.hide_render = True
                hidden.append(obj)
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
        for name in ("porte_face", "porte_station"):
            eye, target, lens = SHOTS[name][:3]
            pv.camera("cam", eye, target, lens=lens)
            pv.render(OUT / f"vitrine_{name}_ambiante.png")
            print(f"aperçu : vitrine_{name}_ambiante.png")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
