"""Le vaisseau du joueur (proposition P12) : petit marchand de taille corvette, ≈ 60 m.

Assemblé à partir du kit `tools/blender/kit/ship_interior.py`. Un seul pont, une épine dorsale
de couloirs, des salles de part et d'autre :

    cockpit (avant, plate-forme surélevée)
    couloir_avant ── quartiers (bâbord) / carre (tribord)
    couloir_arriere ── sas (bâbord) / soute (tribord)
    machines (arrière)

Sorties :
- art/ships/player_ship.blend ;
- assets/models/ships/player_ship/<cellule>.glb (qualité Locale : tout, avec collisions) ;
- assets/models/ships/player_ship/<cellule>_lod.glb (qualité Sectorielle : coque seule) ;
- assets/data/propositions/sectors/player_ship.ron (cellules, portails, apparitions), généré.

Repères Blender : X = tribord, Y = avant, Z = haut. Le jeu reçoit X, Z, −Y (export glTF).
"""

from __future__ import annotations

import contextlib
import io
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "kit"))

import common as c  # noqa: E402
import ship_interior as kit  # noqa: E402

SECTOR = "player_ship"
MODELS = f"assets/models/ships/{SECTOR}"
DOOR_MODEL = "models/kit/ship_interior/door_sliding.glb"
CORRIDOR_H = 2.6

# Cellules : id → (x0, x1, y0, y1, hauteur sous plafond). Intérieurs ; les cloisons (0,2 m)
# sont à l'extérieur de ces bornes, deux salles voisines laissent 0,4 m entre elles.
CELLS = {
    "machines": (-5.0, 5.0, -26.0, -14.4, 3.4),
    "couloir_arriere": (-1.2, 1.2, -14.0, 0.0, CORRIDOR_H),
    "sas": (-5.6, -1.6, -5.0, -1.0, CORRIDOR_H),
    "soute": (1.6, 10.0, -12.0, -2.0, 3.4),
    "couloir_avant": (-1.2, 1.2, 0.0, 14.0, CORRIDOR_H),
    "quartiers": (-8.4, -1.6, 2.0, 10.0, 2.8),
    "carre": (1.6, 8.4, 2.0, 10.0, 2.8),
    "cockpit": (-4.0, 4.0, 14.4, 22.0, 3.4),
}

# Portails : (cellule a, cellule b, genre, x, y, axe). « axis » = mur traversé ("x" : mur
# perpendiculaire à X, donc ouverture vers ±X ; "y" : vers ±Y).
PORTALS = [
    ("couloir_arriere", "machines", "Door", 0.0, -14.2, "y"),
    ("couloir_arriere", "soute", "Door", 1.4, -7.0, "x"),
    ("couloir_arriere", "sas", "Door", -1.4, -3.0, "x"),
    ("couloir_arriere", "couloir_avant", "Opening", 0.0, 0.0, "y"),
    ("couloir_avant", "quartiers", "Door", -1.4, 6.0, "x"),
    ("couloir_avant", "carre", "Door", 1.4, 6.0, "x"),
    ("couloir_avant", "cockpit", "Door", 0.0, 14.2, "y"),
]

SPAWNS = [
    # (id, cellule, x, y, z, cap en degrés autour de Z : 0 = vers l'avant +Y)
    ("sas", "sas", -3.8, -3.0, 0.0, -90.0),
    ("cockpit", "cockpit", 0.0, 15.2, 0.0, 0.0),
    ("soute", "soute", 5.8, -10.5, 0.0, 0.0),
]


def openings_for(cell: str) -> list[tuple[str, float, float, float]]:
    """Ouvertures de la coque d'une cellule, déduites des portails."""
    x0, x1, y0, y1, h = CELLS[cell]
    out = []
    for a, b, kind, x, y, axis in PORTALS:
        if cell not in (a, b):
            continue
        if kind == "Opening":
            width, height = (x1 - x0) if axis == "y" else (y1 - y0), h
        else:
            width, height = kit.DOOR_W, kit.DOOR_H
        if axis == "y":
            side = "N" if abs(y - y1) < abs(y - y0) else "S"
            out.append((side, x, width, height))
        else:
            side = "E" if abs(x - x1) < abs(x - x0) else "W"
            out.append((side, y, width, height))
    return out


def light(name, x, y, z_ceiling, m, coll, lumens, rng, shadow=False, facing=0.0):
    props = {"lumens": lumens, "range": rng, "color": [1.0, 0.93, 0.84], "shadow_candidate": int(shadow)}
    return kit.ceiling_light(name, (x, y, z_ceiling), facing, m, coll, props)


def build_cell(cell: str, m: dict) -> dict:
    """Construit une cellule dans sa collection CELL_<id>."""
    x0, x1, y0, y1, h = CELLS[cell]
    coll = c.collection(f"CELL_{cell}")
    corridor = cell.startswith("couloir")
    shell = kit.room_shell(cell, x0, x1, y0, y1, 0.0, h, openings_for(cell), m, coll, floor_strip=corridor)
    out = {
        "visual": list(shell["visual"]),
        "structure": list(shell["structure"]),
        "colliders": list(shell["colliders"]),
        "marks": [],
        "pickups": [],
    }

    def add(result, pickup=False):
        vis, col, marks = result
        if pickup:
            # L'objet ramassable reste un objet à part, enfant de son repère INTERACT_ :
            # le jeu le retire d'un bloc quand on le ramasse.
            root = marks[0]
            c.bpy.context.view_layer.update()
            for v in vis:
                v.parent = root
                v.matrix_parent_inverse = root.matrix_world.inverted()
            out["pickups"].extend(vis)
        else:
            out["visual"].extend(vis)
        out["colliders"].extend(col)
        out["marks"].extend(marks)

    cx = (x0 + x1) / 2
    if cell == "couloir_arriere":
        for i, y in enumerate((-12.0, -8.0, -4.0)):
            add(light(f"{cell}_l{i}", 0.0, y, h, m, coll, 18000.0, 7.0, shadow=(i == 1), facing=math.pi / 2))
        add(kit.locker(f"{cell}_locker0", (0.85, -11.0, 0.0), -math.pi / 2, m, coll))
        add(kit.locker(f"{cell}_locker1", (0.85, -10.3, 0.0), -math.pi / 2, m, coll))
    elif cell == "couloir_avant":
        for i, y in enumerate((2.0, 8.0, 12.0)):
            add(light(f"{cell}_l{i}", 0.0, y, h, m, coll, 18000.0, 7.0, shadow=(i == 1), facing=math.pi / 2))
    elif cell == "machines":
        add(kit.reactor(f"{cell}_reactor", (0.0, -21.0, 0.0), m, coll))
        add(kit.console(f"{cell}_console", (3.4, -16.0, 0.0), math.pi, m, coll, label="engineering"))
        add(kit.crate(f"{cell}_crate0", (-3.6, -16.2, 0.0), 0.2, m, coll))
        add(kit.crate(f"{cell}_crate1", (-3.8, -17.4, 0.0), -0.1, m, coll, size=(1.2, 1.2, 1.0)))
        for i, (x, y) in enumerate(((-2.5, -17.0), (2.5, -17.0), (-2.5, -24.0), (2.5, -24.0))):
            add(light(f"{cell}_l{i}", x, y, h, m, coll, 30000.0, 10.0, shadow=(i == 0)))
    elif cell == "soute":
        stacks = [(3.2, -4.0), (3.2, -5.2), (4.4, -4.0), (8.4, -4.2), (8.4, -5.5), (8.4, -10.4), (7.1, -10.4)]
        for i, (x, y) in enumerate(stacks):
            add(kit.crate(f"{cell}_crate{i}", (x, y, 0.0), 0.05 * i, m, coll, size=(1.2, 1.2, 1.2)))
        add(kit.crate(f"{cell}_crate_top", (3.2, -4.0, 1.2), 0.3, m, coll))
        add(kit.pickup(f"{cell}_pickup_parts", (6.0, -7.0, 0.0), 0.4, m, coll, item="spare_parts"), pickup=True)
        for i, (x, y) in enumerate(((4.0, -4.5), (8.0, -4.5), (4.0, -9.5), (8.0, -9.5))):
            add(light(f"{cell}_l{i}", x, y, h, m, coll, 30000.0, 10.0, shadow=(i == 3)))
    elif cell == "sas":
        add(kit.locker(f"{cell}_suit0", (-5.15, -4.4, 0.0), math.pi / 2, m, coll))
        add(kit.locker(f"{cell}_suit1", (-5.15, -1.6, 0.0), math.pi / 2, m, coll))
        # Écoutille extérieure (fermée en M1 : l'amarrage arrive en M2).
        out["visual"].append(c.box(f"{cell}_hatch", (0.12, 1.6, 2.2), (x0 - 0.01, -3.0, 1.1), m["hazard"], coll))
        out["visual"].append(c.box(f"{cell}_hatch_frame", (0.14, 1.8, 0.12), (x0 - 0.01, -3.0, 2.26), m["trim"], coll))
        add(light(f"{cell}_l0", -3.6, -3.0, h, m, coll, 15000.0, 6.0, shadow=True))
    elif cell == "quartiers":
        add(kit.bunk(f"{cell}_bunk0", (-7.85, 4.0, 0.0), math.pi / 2, m, coll))
        add(kit.bunk(f"{cell}_bunk1", (-7.85, 8.0, 0.0), math.pi / 2, m, coll))
        add(kit.locker(f"{cell}_locker0", (-4.0, 9.65, 0.0), math.pi, m, coll))
        add(kit.locker(f"{cell}_locker1", (-4.7, 9.65, 0.0), math.pi, m, coll))
        add(kit.table(f"{cell}_desk", (-4.6, 2.6, 0.0), 0.0, m, coll, size=(1.6, 0.8)))
        add(kit.pickup(f"{cell}_pickup_datapad", (-4.9, 2.6, 0.79), 0.3, m, coll, item="datapad"), pickup=True)
        for i, (x, y) in enumerate(((-5.0, 4.0), (-5.0, 8.0))):
            add(light(f"{cell}_l{i}", x, y, h, m, coll, 20000.0, 8.0, shadow=(i == 0)))
    elif cell == "carre":
        add(kit.table(f"{cell}_table", (5.0, 6.0, 0.0), math.pi / 2, m, coll, size=(2.4, 1.0)))
        add(kit.seat(f"{cell}_seat0", (4.1, 5.2, 0.0), -math.pi / 2, m, coll))
        add(kit.seat(f"{cell}_seat1", (5.9, 6.8, 0.0), math.pi / 2, m, coll))
        add(kit.console(f"{cell}_galley", (7.85, 3.0, 0.0), -math.pi / 2, m, coll, label="galley"))
        for i, (x, y) in enumerate(((5.0, 4.0), (5.0, 8.0))):
            add(light(f"{cell}_l{i}", x, y, h, m, coll, 20000.0, 8.0, shadow=(i == 0)))
    elif cell == "cockpit":
        rise = 0.8
        platform_y0 = 16.8
        out["visual"].append(c.box(f"{cell}_platform", (x1 - x0, y1 - platform_y0, rise), (cx, (platform_y0 + y1) / 2, rise / 2), m["floor"], coll))
        out["structure"].append(out["visual"][-1])
        out["colliders"].append(c.collider_box(f"{cell}_platform", (x1 - x0, y1 - platform_y0, rise), (cx, (platform_y0 + y1) / 2, rise / 2), coll))
        add(kit.stairs(f"{cell}_stairs", -1.0, 1.0, platform_y0 - 1.2, rise, 4, m, coll))
        # Garde-corps de part et d'autre de l'escalier.
        for side in (-1, 1):
            xr = side * 2.5
            out["visual"].append(c.box(f"{cell}_rail{side}", (3.0, 0.06, 0.06), (xr, platform_y0 + 0.03, rise + 1.0), m["trim"], coll))
            out["colliders"].append(c.collider_box(f"{cell}_rail{side}", (3.0, 0.1, 1.0), (xr, platform_y0 + 0.05, rise + 0.5), coll))
        add(kit.seat(f"{cell}_pilot", (-1.0, 19.6, rise), 0.0, m, coll, role="pilot"))
        add(kit.seat(f"{cell}_copilot", (1.0, 19.6, rise), 0.0, m, coll, role="copilot"))
        add(kit.console(f"{cell}_helm", (-1.0, 21.2, rise), math.pi, m, coll, label="helm"))
        add(kit.console(f"{cell}_nav", (1.0, 21.2, rise), math.pi, m, coll, label="navigation"))
        add(kit.console(f"{cell}_comms", (3.4, 18.5, rise), math.pi / 2, m, coll, label="comms"))
        # Grand écran de vue avant (l'extérieur réel arrive en M2).
        out["visual"].append(c.box(f"{cell}_viewscreen", (6.0, 0.05, 1.4), (0.0, y1 - 0.02, rise + 1.9), m["screen"], coll))
        for i, (x, y) in enumerate(((0.0, 15.6), (0.0, 19.0))):
            add(light(f"{cell}_l{i}", x, y, h, m, coll, 22000.0, 9.0, shadow=(i == 1)))
    return out


def portal_record(a, b, kind, x, y, axis):
    x0, x1, y0, y1, h = CELLS[a]
    if kind == "Opening":
        width = (x1 - x0) if axis == "y" else (y1 - y0)
        height = h
    else:
        width, height = kit.DOOR_W, kit.DOOR_H
    # Normale : de a vers b.
    bx0, bx1, by0, by1, _ = CELLS[b]
    bc = ((bx0 + bx1) / 2, (by0 + by1) / 2)
    if axis == "y":
        normal = (0.0, 1.0 if bc[1] > y else -1.0, 0.0)
    else:
        normal = (1.0 if bc[0] > x else -1.0, 0.0, 0.0)
    return {
        "id": f"{a}__{b}",
        "a": a,
        "b": b,
        "kind": kind,
        "center": c.to_bevy((x, y, height / 2)),
        "normal": c.to_bevy(normal),
        "width": round(width, 3),
        "height": round(height, 3),
    }


def fmt(v) -> str:
    return "(" + ", ".join(f"{x:.3f}" for x in v) + ")"


def sector_ron() -> str:
    lines = [
        "// Généré par tools/blender/ships/player_ship.py : NE PAS ÉDITER À LA MAIN.",
        "// Le vaisseau du joueur (proposition P12) : cellules, portails, apparitions.",
        "// Coordonnées dans le repère du jeu (Y-haut, −Z = avant du vaisseau), en mètres.",
        "(",
        f'    id: "{SECTOR}",',
        "    kind: Ship,",
        '    name_key: "sector.player_ship",',
        '    status: Proposition("P12"),',
        f'    door_model: "{DOOR_MODEL}",',
        "    cells: [",
    ]
    for cell, (x0, x1, y0, y1, h) in CELLS.items():
        lo = c.to_bevy((x0 - kit.WALL, y1 + kit.WALL, -kit.WALL))
        hi = c.to_bevy((x1 + kit.WALL, y0 - kit.WALL, h + kit.WALL))
        model = f"models/ships/{SECTOR}/{cell}.glb"
        lines += [
            "        (",
            f'            id: "{cell}",',
            f'            model: "{model}",',
            f'            lod: "{model[:-4]}_lod.glb",',
            f"            min: {fmt(lo)},",
            f"            max: {fmt(hi)},",
            "        ),",
        ]
    lines.append("    ],")
    lines.append("    portals: [")
    for p in PORTALS:
        r = portal_record(*p)
        lines += [
            "        (",
            f'            id: "{r["id"]}",',
            f'            a: "{r["a"]}",',
            f'            b: "{r["b"]}",',
            f"            kind: {r['kind']},",
            f"            center: {fmt(r['center'])},",
            f"            normal: {fmt(r['normal'])},",
            f"            width: {r['width']:.3f},",
            f"            height: {r['height']:.3f},",
            "        ),",
        ]
    lines.append("    ],")
    lines.append("    spawns: [")
    for sid, cell, x, y, z, heading in SPAWNS:
        lines.append(
            f'        (id: "{sid}", cell: "{cell}", pos: {fmt(c.to_bevy((x, y, z)))}, heading_deg: {heading:.1f}),'
        )
    lines += ["    ],", ")", ""]
    return "\n".join(lines)


def main() -> None:
    c.reset_scene()
    m = kit.palette()
    exports = c.collection("EXPORT")
    built = {cell: build_cell(cell, m) for cell in CELLS}

    quiet = io.StringIO()
    with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
        for cell, parts in built.items():
            # Qualité Locale : un maillage par matériau + collisions + repères + objets à part.
            merged = c.join_by_material(parts["visual"], cell, exports)
            full = merged + parts["colliders"] + parts["marks"] + parts["pickups"]
            c.export_glb(f"{MODELS}/{cell}.glb", full)
            # Qualité Sectorielle : la coque seule, fusionnée, sans props ni collisions.
            lod = c.join_by_material(parts["structure"], f"{cell}_lod1", exports)
            c.export_glb(f"{MODELS}/{cell}_lod.glb", lod)
            for obj in merged + lod:
                obj.hide_set(True)
    c.write_text(f"assets/data/propositions/sectors/{SECTOR}.ron", sector_ron())
    c.save_blend(f"art/ships/{SECTOR}.blend")
    print(f"{SECTOR} : {len(CELLS)} cellules, {len(PORTALS)} portails exportés")


if __name__ == "__main__":
    main()
