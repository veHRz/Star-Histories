"""Kit modulaire d'intérieur de vaisseau (proposition P13, direction artistique validée :
« réaliste stylisé, géométrie simple et lisible »).

Ce module sert de bibliothèque aux scripts de vaisseaux (`tools/blender/ships/`) et, lancé
seul, exporte chaque pièce du kit dans `assets/models/kit/ship_interior/` avec sa source
`art/kit/ship_interior.blend`, pour la revue et la réutilisation.

Grille modulaire : 2 m (P5). Origines : centre de la base pour les props, coin de grille pour
les modules. Repères : X = tribord, Y = avant, Z = haut (Blender).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import common as c  # noqa: E402

GRID = 2.0
WALL = 0.2  # épaisseur des cloisons
DOOR_W = 1.4
DOOR_H = 2.2


def palette() -> dict:
    """Matériaux partagés du kit (peu nombreux : un appel de rendu par matériau et par cellule)."""
    return {
        "wall": c.material("sh_wall_panel", (0.50, 0.52, 0.54), metallic=0.25, roughness=0.55),
        "floor": c.material("sh_floor", (0.16, 0.17, 0.18), metallic=0.55, roughness=0.45),
        "ceiling": c.material("sh_ceiling", (0.30, 0.31, 0.33), metallic=0.3, roughness=0.6),
        "trim": c.material("sh_trim", (0.07, 0.075, 0.08), metallic=0.6, roughness=0.4),
        "accent": c.material("sh_accent", (0.80, 0.50, 0.13), metallic=0.1, roughness=0.5),
        "light": c.material(
            "sh_light", (1.0, 0.95, 0.88), emission=(1.0, 0.92, 0.80), emission_strength=6.0
        ),
        "screen": c.material(
            "sh_screen", (0.02, 0.05, 0.06), emission=(0.25, 0.85, 1.0), emission_strength=2.5
        ),
        "fabric": c.material("sh_fabric", (0.18, 0.22, 0.28), roughness=0.9),
        "crate": c.material("sh_crate", (0.30, 0.33, 0.21), metallic=0.2, roughness=0.6),
        "machine": c.material("sh_machine", (0.38, 0.39, 0.41), metallic=0.8, roughness=0.35),
        "hazard": c.material("sh_hazard", (0.85, 0.65, 0.08), roughness=0.5),
        "glass": c.material("sh_glass", (0.55, 0.70, 0.80), roughness=0.05, alpha=0.12),
    }


# --- Coque de salle -----------------------------------------------------------------------------


def room_shell(
    prefix: str,
    x0: float,
    x1: float,
    y0: float,
    y1: float,
    z0: float,
    height: float,
    openings: list[tuple[str, float, float, float]],
    m: dict,
    coll,
    ribs: bool = True,
    floor_strip: bool = False,
) -> dict:
    """Sol, plafond et quatre cloisons d'une salle de dimensions intérieures données.

    `openings` : (côté, position le long du mur, largeur, hauteur[, allège]) ; côtés N (+Y),
    S (−Y), E (+X), W (−X). Une allège > 0 fait une fenêtre : vitrage transparent, cadre et
    collision pleine (on ne passe pas). Renvoie {"visual": [...], "structure": [...],
    "colliders": [...]} (`structure` : ce qui reste dans la version simplifiée de la cellule).
    """
    out = {"visual": [], "structure": [], "colliders": []}
    w, d = x1 - x0, y1 - y0
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    zt = z0 + height

    def add(obj, structure=True):
        out["visual"].append(obj)
        if structure:
            out["structure"].append(obj)

    def solid(name, size, center, mat, structure=True):
        add(c.box(name, size, center, mat, coll), structure)
        out["colliders"].append(c.collider_box(name, size, center, coll))

    solid(f"{prefix}_floor", (w + 2 * WALL, d + 2 * WALL, WALL), (cx, cy, z0 - WALL / 2), m["floor"])
    solid(f"{prefix}_ceiling", (w + 2 * WALL, d + 2 * WALL, WALL), (cx, cy, zt + WALL / 2), m["ceiling"])

    walls = {
        # côté : (axe le long, origine, longueur, position fixe, normale vers l'intérieur)
        "N": ("x", x0 - WALL, w + 2 * WALL, y1 + WALL / 2, -1),
        "S": ("x", x0 - WALL, w + 2 * WALL, y0 - WALL / 2, 1),
        "E": ("y", y0, d, x1 + WALL / 2, -1),
        "W": ("y", y0, d, x0 - WALL / 2, 1),
    }
    for side, (axis, start, length, fixed, inward) in walls.items():
        holes = sorted(
            (o[1] - o[2] / 2, o[1] + o[2] / 2, o[3], o[4] if len(o) > 4 else 0.0)
            for o in openings
            if o[0] == side
        )
        cursor = start
        segments = []
        for a, b, oh, sill in holes:
            segments.append((cursor, a, z0, height))  # plein mur avant l'ouverture
            if sill > 0.0:
                segments.append((a, b, z0, sill))  # allège sous la fenêtre
            segments.append((a, b, z0 + sill + oh, height - sill - oh))  # linteau
            cursor = b
        segments.append((cursor, start + length, z0, height))
        for i, (a, b, zb, h) in enumerate(segments):
            if b - a < 0.01 or h < 0.01:
                continue
            mid = (a + b) / 2
            if axis == "x":
                size, center = (b - a, WALL, h), (mid, fixed, zb + h / 2)
            else:
                size, center = (WALL, b - a, h), (fixed, mid, zb + h / 2)
            solid(f"{prefix}_wall_{side}{i}", size, center, m["wall"])
        # Cadres de porte et de fenêtre ; vitrage et collision pleine pour les fenêtres.
        for a, b, oh, sill in holes:
            mid = (a + b) / 2
            zb = z0 + sill
            for edge in (a, b):
                if axis == "x":
                    add(c.box(f"{prefix}_jamb", (0.12, WALL + 0.08, oh), (edge, fixed, zb + oh / 2), m["trim"]), False)
                else:
                    add(c.box(f"{prefix}_jamb", (WALL + 0.08, 0.12, oh), (fixed, edge, zb + oh / 2), m["trim"]), False)
            header = m["trim"] if sill > 0.0 else m["accent"]
            frames = [zb + oh + 0.08] + ([zb - 0.08] if sill > 0.0 else [])
            for zf in frames:
                if axis == "x":
                    add(c.box(f"{prefix}_header", (b - a + 0.24, WALL + 0.1, 0.16), (mid, fixed, zf), header), False)
                else:
                    add(c.box(f"{prefix}_header", (WALL + 0.1, b - a + 0.24, 0.16), (fixed, mid, zf), header), False)
            if sill > 0.0:
                size = (b - a, 0.03, oh) if axis == "x" else (0.03, b - a, oh)
                center = (mid, fixed, zb + oh / 2) if axis == "x" else (fixed, mid, zb + oh / 2)
                add(c.box(f"{prefix}_glass", size, center, m["glass"], coll))
                wall_size = (b - a, WALL, oh) if axis == "x" else (WALL, b - a, oh)
                out["colliders"].append(c.collider_box(f"{prefix}_glass", wall_size, center, coll))
        if not ribs:
            continue
        # Membrures verticales tous les 2 m et plinthe : la lecture « intérieur de vaisseau ».
        inner = fixed + inward * (WALL / 2 + 0.03)
        pos = start + WALL + GRID
        while pos < start + length - WALL - 0.3:
            if all(not (a - 0.2 < pos < b + 0.2) for a, b, _, _ in holes):
                if axis == "x":
                    add(c.box(f"{prefix}_rib", (0.14, 0.06, height), (pos, inner, z0 + height / 2), m["trim"]), False)
                else:
                    add(c.box(f"{prefix}_rib", (0.06, 0.14, height), (inner, pos, z0 + height / 2), m["trim"]), False)
            pos += GRID
        span = (start + WALL, start + length - WALL)
        # La plinthe ne s'interrompt qu'aux portes (pas sous une fenêtre).
        doors = [(a, b) for a, b, _, sill in holes if sill <= 0.0]
        cuts = [span[0]] + [v for a, b in doors for v in (a, b)] + [span[1]]
        for a, b in zip(cuts[0::2], cuts[1::2]):
            if b - a < 0.05:
                continue
            if axis == "x":
                add(c.box(f"{prefix}_base", (b - a, 0.05, 0.14), ((a + b) / 2, inner, z0 + 0.07), m["trim"]), False)
            else:
                add(c.box(f"{prefix}_base", (0.05, b - a, 0.14), (inner, (a + b) / 2, z0 + 0.07), m["trim"]), False)
    # Poutres de plafond transversales.
    if ribs:
        if d >= w:
            y = y0 + GRID
            while y < y1 - 0.5:
                add(c.box(f"{prefix}_beam", (w, 0.16, 0.14), (cx, y, zt - 0.07), m["trim"]), False)
                y += GRID
        else:
            x = x0 + GRID
            while x < x1 - 0.5:
                add(c.box(f"{prefix}_beam", (0.16, d, 0.14), (x, cy, zt - 0.07), m["trim"]), False)
                x += GRID
    if floor_strip:
        long_y = d >= w
        size = (0.6, d - 0.4, 0.01) if long_y else (w - 0.4, 0.6, 0.01)
        add(c.box(f"{prefix}_grate", size, (cx, cy, z0 + 0.005), m["trim"]), False)
    return out


# --- Props ------------------------------------------------------------------------------------
# Chaque prop renvoie (objets visibles, colliders, repères). Orientation : `facing` en radians
# autour de Z, 0 = la face utile regarde vers +Y.


def _rotated(dx: float, dy: float, facing: float) -> tuple[float, float]:
    import math

    cs, sn = math.cos(facing), math.sin(facing)
    return dx * cs - dy * sn, dx * sn + dy * cs


def _place(objs, center, facing):
    for obj in objs:
        ox, oy = _rotated(obj.location.x, obj.location.y, facing)
        obj.location = (center[0] + ox, center[1] + oy, center[2] + obj.location.z)
        obj.rotation_euler[2] += facing


def console(name: str, center, facing: float, m: dict, coll, label: str = "console", low: bool = False):
    """Console ; `low` : écran bas, sous la ligne des yeux assis (postes devant un hublot)."""
    sz, sh = (0.38, 1.0) if low else (0.55, 1.2)
    vis = [
        c.box(f"{name}_base", (1.2, 0.6, 0.85), (0, 0, 0.425), m["machine"], coll, bevel=0.02),
        c.box(f"{name}_desk", (1.3, 0.45, 0.06), (0, 0.12, 0.88), m["trim"], coll),
        c.box(f"{name}_screen", (1.1, 0.05, sz), (0, -0.2, sh), m["screen"], coll),
        c.box(f"{name}_bezel", (1.2, 0.04, sz + 0.07), (0, -0.23, sh), m["trim"], coll),
    ]
    col = [c.collider_box(f"{name}", (1.3, 0.6, 1.0), (0, 0, 0.5), coll)]
    marks = [c.empty(f"INTERACT_{name}", (0, 0.1, 1.0), coll, {"interact": "console", "label": label})]
    _place(vis + col + marks, center, facing)
    return vis, col, marks


def seat(name: str, center, facing: float, m: dict, coll, role: str = "seat"):
    vis = [
        c.cylinder(f"{name}_post", 0.08, 0.4, (0, 0, 0.2), m["trim"], coll, segments=10),
        c.box(f"{name}_cushion", (0.6, 0.6, 0.12), (0, 0, 0.46), m["fabric"], coll, bevel=0.03),
        c.box(f"{name}_back", (0.6, 0.12, 0.75), (0, -0.28, 0.85), m["fabric"], coll, bevel=0.03),
        c.box(f"{name}_arm_l", (0.08, 0.45, 0.06), (-0.33, 0, 0.66), m["trim"], coll),
        c.box(f"{name}_arm_r", (0.08, 0.45, 0.06), (0.33, 0, 0.66), m["trim"], coll),
    ]
    col = [c.collider_box(f"{name}", (0.7, 0.7, 1.2), (0, -0.05, 0.6), coll)]
    marks = [
        c.empty(f"INTERACT_{name}", (0, 0, 0.9), coll, {"interact": "seat", "role": role}),
        c.empty(f"SOCKET_{name}_eye", (0, -0.05, 1.25), coll, {"usage": "seat_eye"}),
    ]
    _place(vis + col + marks, center, facing)
    return vis, col, marks


def bunk(name: str, center, facing: float, m: dict, coll):
    vis = [
        c.box(f"{name}_frame_lo", (2.0, 0.9, 0.35), (0, 0, 0.175), m["machine"], coll),
        c.box(f"{name}_mat_lo", (1.9, 0.8, 0.14), (0, 0, 0.42), m["fabric"], coll, bevel=0.03),
        c.box(f"{name}_frame_hi", (2.0, 0.9, 0.08), (0, 0, 1.3), m["machine"], coll),
        c.box(f"{name}_mat_hi", (1.9, 0.8, 0.14), (0, 0, 1.41), m["fabric"], coll, bevel=0.03),
        c.box(f"{name}_post_a", (0.08, 0.08, 1.9), (-0.96, -0.41, 0.95), m["trim"], coll),
        c.box(f"{name}_post_b", (0.08, 0.08, 1.9), (0.96, -0.41, 0.95), m["trim"], coll),
    ]
    col = [
        c.collider_box(f"{name}_lo", (2.0, 0.9, 0.5), (0, 0, 0.25), coll),
        c.collider_box(f"{name}_hi", (2.0, 0.9, 0.25), (0, 0, 1.35), coll),
    ]
    _place(vis + col, center, facing)
    return vis, col, []


def crate(name: str, center, facing: float, m: dict, coll, size=(0.9, 0.9, 0.9)):
    sx, sy, sz = size
    vis = [
        c.box(f"{name}", (sx, sy, sz), (0, 0, sz / 2), m["crate"], coll, bevel=0.03),
        c.box(f"{name}_band", (sx + 0.02, sy + 0.02, 0.08), (0, 0, sz * 0.7), m["hazard"], coll),
    ]
    col = [c.collider_box(f"{name}", (sx, sy, sz), (0, 0, sz / 2), coll)]
    _place(vis + col, center, facing)
    return vis, col, []


def locker(name: str, center, facing: float, m: dict, coll):
    vis = [
        c.box(f"{name}", (0.6, 0.55, 2.0), (0, 0, 1.0), m["wall"], coll, bevel=0.02),
        c.box(f"{name}_seam", (0.02, 0.57, 1.8), (0, 0, 1.0), m["trim"], coll),
    ]
    col = [c.collider_box(f"{name}", (0.6, 0.55, 2.0), (0, 0, 1.0), coll)]
    _place(vis + col, center, facing)
    return vis, col, []


def table(name: str, center, facing: float, m: dict, coll, size=(1.8, 0.9)):
    vis = [
        c.box(f"{name}_top", (size[0], size[1], 0.06), (0, 0, 0.76), m["trim"], coll),
        c.box(f"{name}_leg", (0.2, size[1] * 0.6, 0.73), (0, 0, 0.365), m["machine"], coll),
    ]
    col = [c.collider_box(f"{name}", (size[0], size[1], 0.8), (0, 0, 0.4), coll)]
    _place(vis + col, center, facing)
    return vis, col, []


def pickup(name: str, center, facing: float, m: dict, coll, item: str):
    """Petit objet ramassable (boîte à outils, bloc-données…)."""
    vis = [
        c.box(f"{name}", (0.45, 0.25, 0.2), (0, 0, 0.1), m["accent"], coll, bevel=0.02),
        c.box(f"{name}_handle", (0.25, 0.04, 0.06), (0, 0, 0.23), m["trim"], coll),
    ]
    marks = [c.empty(f"INTERACT_{name}", (0, 0, 0.15), coll, {"interact": "pickup", "item": item})]
    _place(vis + marks, center, facing)
    return vis, [], marks


def reactor(name: str, center, m: dict, coll):
    vis = [
        c.cylinder(f"{name}_core", 1.1, 3.0, (0, 0, 1.5), m["machine"], coll, segments=24),
        c.cylinder(f"{name}_ring_a", 1.25, 0.18, (0, 0, 0.6), m["hazard"], coll, segments=24),
        c.cylinder(f"{name}_ring_b", 1.25, 0.18, (0, 0, 2.4), m["hazard"], coll, segments=24),
        c.cylinder(f"{name}_glow", 0.6, 3.1, (0, 0, 1.55), m["screen"], coll, segments=12),
        c.cylinder(f"{name}_pipe_a", 0.18, 4.0, (0, 1.6, 2.6), m["trim"], coll, axis="Y", segments=10),
        c.cylinder(f"{name}_pipe_b", 0.18, 4.0, (0, -1.6, 2.6), m["trim"], coll, axis="Y", segments=10),
    ]
    col = [c.collider_box(f"{name}", (2.5, 2.5, 3.2), (0, 0, 1.6), coll)]
    _place(vis + col, center, 0.0)
    return vis, col, []


def ceiling_light(name: str, center, facing: float, m: dict, coll, props: dict):
    """Plafonnier émissif + repère LIGHT_ (le jeu crée la vraie lumière, avec ses réglages)."""
    vis = [
        c.box(f"{name}_housing", (1.1, 0.32, 0.06), (0, 0, -0.03), m["trim"], coll),
        c.box(f"{name}_lens", (1.0, 0.24, 0.02), (0, 0, -0.07), m["light"], coll),
    ]
    marks = [c.empty(f"LIGHT_{name}", (0, 0, -0.25), coll, props)]
    _place(vis + marks, center, facing)
    return vis, [], marks


def stairs(name: str, x0: float, x1: float, y0: float, rise: float, steps: int, m: dict, coll):
    """Marches montant vers +Y à partir de y0, sur la largeur [x0, x1]. Une boîte de collision
    par marche : le contrôleur du joueur monte les marches (pas d'une rampe invisible)."""
    vis, col = [], []
    tread = 0.3
    step_h = rise / steps
    for i in range(steps):
        h = step_h * (i + 1)
        center = ((x0 + x1) / 2, y0 + tread * (i + 0.5), h / 2)
        size = (x1 - x0, tread, h)
        vis.append(c.box(f"{name}_step{i}", size, center, m["floor"], coll))
        vis.append(c.box(f"{name}_nose{i}", (x1 - x0, 0.05, 0.03), (center[0], y0 + tread * i + 0.025, h + 0.015), m["hazard"], coll))
        col.append(c.collider_box(f"{name}_step{i}", size, center, coll))
    return vis, col, []


def ladder(name: str, center, facing: float, height: float, m: dict, coll):
    vis = [
        c.box(f"{name}_rail_l", (0.06, 0.06, height), (-0.22, 0, height / 2), m["trim"], coll),
        c.box(f"{name}_rail_r", (0.06, 0.06, height), (0.22, 0, height / 2), m["trim"], coll),
    ]
    rung = 0.3
    z = rung
    while z < height - 0.1:
        vis.append(c.box(f"{name}_rung", (0.44, 0.04, 0.04), (0, 0, z), m["machine"], coll))
        z += rung
    marks = [c.empty(f"INTERACT_{name}", (0, 0.2, 1.2), coll, {"interact": "ladder", "height": height})]
    _place(vis + marks, center, facing)
    return vis, [], marks


def door_panel(name: str, m: dict, coll):
    """Panneau de porte coulissante, origine au centre de sa base. Le jeu en place un à chaque
    portail de type porte et l'anime (il coulisse dans la cloison)."""
    vis = [
        c.box(f"{name}_panel", (DOOR_W, 0.08, DOOR_H), (0, 0, DOOR_H / 2), m["wall"], coll, bevel=0.02),
        c.box(f"{name}_stripe", (DOOR_W - 0.1, 0.1, 0.12), (0, 0, 1.15), m["accent"], coll),
        c.box(f"{name}_window", (0.5, 0.1, 0.35), (0, 0, 1.6), m["screen"], coll),
    ]
    col = [c.collider_box(f"{name}", (DOOR_W, 0.12, DOOR_H), (0, 0, DOOR_H / 2), coll)]
    return vis, col, []


# --- Export des pièces du kit ------------------------------------------------------------------


def export_kit() -> None:
    c.reset_scene()
    m = palette()
    pieces = {}

    def piece(name, builder):
        coll = c.collection(f"kit_{name}")
        vis, col, marks = builder(coll)
        pieces[name] = vis + col + marks

    piece("corridor_straight", lambda k: _shell(room_shell("cs", -1.2, 1.2, 0, 4, 0, 2.6, [], m, k, floor_strip=True)))
    piece("corridor_corner", lambda k: _shell(room_shell("cc", -1.2, 1.2, 0, 2.4, 0, 2.6, [("E", 1.2, DOOR_W + 0.6, 2.4)], m, k)))
    piece("corridor_t", lambda k: _shell(room_shell("ct", -1.2, 1.2, 0, 2.4, 0, 2.6, [("E", 1.2, 1.8, 2.4), ("W", 1.2, 1.8, 2.4)], m, k)))
    piece("room_6x6", lambda k: _shell(room_shell("rm", -3, 3, -3, 3, 0, 2.8, [("S", 0, DOOR_W, DOOR_H)], m, k)))
    piece("airlock", lambda k: _shell(room_shell("al", -2, 2, -2, 2, 0, 2.6, [("E", 0, DOOR_W, DOOR_H), ("W", 0, DOOR_W, DOOR_H)], m, k)))
    piece("door_sliding", lambda k: door_panel("door", m, k))
    piece("console", lambda k: console("console", (0, 0, 0), 0.0, m, k))
    piece("seat", lambda k: seat("seat", (0, 0, 0), 0.0, m, k))
    piece("bunk", lambda k: bunk("bunk", (0, 0, 0), 0.0, m, k))
    piece("crate", lambda k: crate("crate", (0, 0, 0), 0.0, m, k))
    piece("locker", lambda k: locker("locker", (0, 0, 0), 0.0, m, k))
    piece("ceiling_light", lambda k: ceiling_light("ceiling_light", (0, 0, 2.6), 0.0, m, k, {"lumens": 60000.0}))
    piece("stairs", lambda k: stairs("stairs", -0.8, 0.8, 0, 0.8, 4, m, k))
    piece("ladder", lambda k: ladder("ladder", (0, 0, 0), 0.0, 3.0, m, k))

    # Revue visuelle : pièces alignées dans la scène, une rangée tous les 8 m.
    for i, (name, objs) in enumerate(pieces.items()):
        c.export_glb(f"assets/models/kit/ship_interior/{name}.glb", objs)
        for obj in objs:
            obj.location.x += (i % 5) * 8.0
            obj.location.y += (i // 5) * 8.0
    c.save_blend("art/kit/ship_interior.blend")
    print(f"kit exporté : {len(pieces)} pièces")


def _shell(parts: dict):
    return parts["visual"], parts["colliders"], []


if __name__ == "__main__":
    export_kit()
