"""Salle témoin de la direction « futur usé » (lot 0 de M8, proposition P13).

Un petit secteur de deux cellules, construit avec la bibliothèque de matériaux (géométrie propre
à ce script, pas le kit), qui montre la direction à hauteur d'œil :

    coursive (2,4 × 8 m, 2,6 m) : section à pans coupés, nervures tous les 2 m, bandeaux
        lumineux froids dans les angles, caillebotis éclairé par-dessous, gaines au plafond ;
    porte (ouverture kit.DOOR_W × kit.DOOR_H, cadre épais à pans coupés ; le jeu y pose
        models/kit/ship_interior/door_sliding.gltf) ;
    atelier (6 × 8 m, 3,2 m) : lumière sodium, poutres et colonnes apparentes, tuyaux, gaine,
        console inclinée (boutons, écrans), caisses, casier, pompe, trappe et bandes de danger,
        gyrophare rouge.

Sorties : assets/models/materials/salle_temoin/<cellule>{,_lod}.gltf (+ .bin, textures par
chemin relatif vers assets/textures/materials/), lightmaps dans
assets/textures/lightmaps/salle_temoin/, descripteur
assets/data/propositions/sectors/salle_temoin.ron, art/materials/salle_temoin.blend ;
aperçus (`--previews`) dans target/art-previews/lot0/.

    SH_BAKE_SAMPLES=128 blender --background --factory-startup --python tools/blender/materials/salle_temoin.py -- [--no-bake] [--previews] [--samples N]

Repères Blender : X = droite (tribord), Y = avant (vers l'atelier), Z = haut.
"""

from __future__ import annotations

import contextlib
import io
import math
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "kit"))
sys.path.insert(0, str(HERE))

import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import common as c  # noqa: E402
import lightmap as lm  # noqa: E402
import sector_tools as st  # noqa: E402
import ship_interior as kit  # noqa: E402

SECTOR = "salle_temoin"
MODELS = f"models/materials/{SECTOR}"
LIGHTMAPS = f"textures/lightmaps/{SECTOR}"

CELLS = {
    "coursive": (-1.2, 1.2, 0.0, 8.0, 2.6),
    "atelier": (-3.0, 3.0, 8.4, 16.4, 3.2),
}
PORTALS = [("coursive", "atelier", "Door", 0.0, 8.2, "y")]
SPAWNS = [
    ("entree", "coursive", 0.0, 1.0, 0.0, 0.0),
    ("atelier", "atelier", -0.6, 9.6, 0.0, 0.0),
]
DOOR_Y = 8.2

COOL = [0.74, 0.86, 1.0]
SODIUM = [1.0, 0.55, 0.22]
RED = [1.0, 0.18, 0.12]

# Section octogonale intérieure de la coursive (x, z), du sol au plafond, côté gauche puis droit.
OCT_LEFT = [(-0.8, 0.0), (-1.2, 0.4), (-1.2, 2.0), (-0.6, 2.6)]
OCT_RIGHT = [(0.8, 0.0), (1.2, 0.4), (1.2, 2.0), (0.6, 2.6)]
OCT_CENTER = (0.0, 1.3)


def palette() -> dict:
    """Matériaux de la salle (≤ 12 par cellule). Les plafonniers gardent le nom `sh_light`
    (repéré par lightmap.py) ; `sh_light_amber` est masqué du bake par ce script."""
    m = {
        "panel": c.textured_material("sh_panel", "panel_painted"),
        "dark": c.textured_material("sh_metal_dark", "metal_dark"),
        "bare": c.textured_material("sh_metal_bare", "metal_bare"),
        "floor": c.textured_material("sh_floor_plate", "floor_plate"),
        "grating": c.textured_material("sh_grating", "grating", emissive=(0.45, 0.68, 1.0), emissive_strength=0.3),
        "trim": c.textured_material("sh_trim", "trim_ship", emissive=(0.85, 0.93, 1.0), emissive_strength=4.0),
        "crate": c.textured_material("sh_crate_olive", "container_ribbed", tint=(0.62, 0.66, 0.46)),
        "rubber": c.textured_material("sh_rubber", "rubber"),
        "buttons": c.textured_material("sh_buttons", "buttons", emissive=(1.0, 1.0, 1.0), emissive_strength=3.0),
        "screen": c.textured_material("sh_screen", "screen", emissive=(1.0, 1.0, 1.0), emissive_strength=2.5),
        "band_cool": c.textured_material("sh_band_cool", "light_band", emissive=(0.74, 0.86, 1.0), emissive_strength=4.0),
        "band_amber": c.textured_material("sh_band_amber", "light_band", emissive=(1.0, 0.6, 0.26), emissive_strength=4.0),
        "light": c.material("sh_light", (0.9, 0.95, 1.0), emission=(0.78, 0.9, 1.0), emission_strength=5.0),
        "light_amber": c.material("sh_light_amber", (1.0, 0.8, 0.6), emission=(1.0, 0.55, 0.2), emission_strength=4.0),
        "beacon": c.material("sh_beacon_red", (0.6, 0.05, 0.03), emission=(1.0, 0.12, 0.06), emission_strength=6.0),
    }
    return m


# --- Briques géométriques ------------------------------------------------------------------------


class Cell:
    """Collecte des objets d'une cellule : visibles (et leur projection d'UV), structure (version
    simplifiée), collisions, repères."""

    def __init__(self, cell_id: str):
        self.id = cell_id
        self.coll = c.collection(f"CELL_{cell_id}")
        self.visual: list = []
        self.structure: list = []
        self.colliders: list = []
        self.marks: list = []

    def add(self, obj, uv="box", structure=False, tile=None):
        """`uv` : "box" (projection boîte), ("box", (du, dv)) (décalée, en tuiles), un nom de
        bande de trim / un couple (v0, v1) (uv_trim), ou ("strip", mètres) pour un bandeau (U le
        long, toute la hauteur en V)."""
        if uv == "box":
            c.uv_box(obj, tile)
        elif isinstance(uv, tuple) and uv and uv[0] == "box":
            c.uv_box(obj, tile, offset=uv[1])
        elif isinstance(uv, tuple) and uv and uv[0] == "strip":
            c.uv_trim(obj, (0.0, 1.0), meters_per_tile=uv[1], pad_px=0.0)
        else:
            c.uv_trim(obj, uv, meters_per_tile=tile or 2.0)
        self.visual.append(obj)
        if structure:
            self.structure.append(obj)
        return obj

    def box(self, name, size, center, mat, uv="box", structure=False, bevel=0.0, tile=None):
        return self.add(c.box(f"{self.id}_{name}", size, center, mat, self.coll, bevel=bevel), uv, structure, tile)

    def cyl(self, name, radius, depth, center, mat, axis="Z", segments=16, uv="box", tile=None):
        return self.add(c.cylinder(f"{self.id}_{name}", radius, depth, center, mat, self.coll, axis=axis, segments=segments), uv, False, tile)

    def oriented(self, name, size, center, right, up, mat, uv="box", structure=False, bevel=0.0, tile=None):
        """Pavé dont les axes locaux sont (right, up, right × up)."""
        obj = c.box(f"{self.id}_{name}", size, (0, 0, 0), mat, self.coll, bevel=bevel)
        r, u = Vector(right).normalized(), Vector(up).normalized()
        n = r.cross(u).normalized()
        u = n.cross(r).normalized()
        obj.matrix_basis = Matrix.Translation(Vector(center)) @ Matrix((r, u, n)).transposed().to_4x4()
        return self.add(obj, uv, structure, tile)

    def col(self, name, size, center):
        self.colliders.append(c.collider_box(f"{self.id}_{name}", size, center, self.coll))

    def light(self, name, location, lumens, rng, color, shadow=False):
        props = {"lumens": lumens, "range": rng, "color": color, "shadow_candidate": int(shadow)}
        self.marks.append(c.empty(f"LIGHT_{self.id}_{name}", location, self.coll, props))

    def parts(self) -> dict:
        return {"visual": self.visual, "structure": self.structure, "colliders": self.colliders, "marks": self.marks, "pickups": []}


def slab(cell: Cell, name, p1, p2, y0, y1, thick, mat, uv="box", structure=False, inset=0.0, extend=0.0, tile=None):
    """Plaque le long de Y dont la face intérieure va de p1 à p2 (points (x, z), p1 en bas) :
    axes locaux (le long de Y, de p1 vers p2, normale vers l'axe de la coursive). La projection
    boîte y donne U le long de la coursive et V qui monte le long du pan."""
    (x1, z1), (x2, z2) = p1, p2
    s = Vector((x2 - x1, 0.0, z2 - z1))
    length = s.length + 2 * extend
    s.normalize()
    n = Vector((-s.z, 0.0, s.x))
    mid = Vector(((x1 + x2) / 2, (y0 + y1) / 2, (z1 + z2) / 2))
    if n.dot(Vector((OCT_CENTER[0] - mid.x, 0.0, OCT_CENTER[1] - mid.z))) < 0:
        n = -n
    xl = s.cross(n)
    obj = c.box(f"{cell.id}_{name}", (y1 - y0, length, thick), (0, 0, 0), mat, cell.coll)
    center = mid - n * (thick / 2 - inset)
    obj.matrix_basis = Matrix.Translation(center) @ Matrix((xl, s, n)).transposed().to_4x4()
    return cell.add(obj, uv, structure, tile)


def octagon_sides():
    """Les huit pans (p1 en bas, p2 en haut) : sol, chanfreins bas, murs, chanfreins hauts,
    plafond."""
    sides = []
    for pts in (OCT_LEFT, OCT_RIGHT):
        for a, b in zip(pts[:-1], pts[1:]):
            sides.append((a, b))
    return sides


# --- Coursive ------------------------------------------------------------------------------------


def build_coursive(m: dict) -> Cell:
    k = Cell("coursive")
    y0, y1 = CELLS["coursive"][2], CELLS["coursive"][3]
    wall_t = 0.12

    # Sol : caillebotis central (émissif : lumière dessous), tôles larmées de part et d'autre,
    # cornières sombres.
    k.box("grating", (0.8, y1 - y0, 0.03), (0, (y0 + y1) / 2, -0.015), m["grating"], structure=True)
    for sx in (-1, 1):
        k.box(f"floorplate_{sx}", (0.4, y1 - y0, 0.1), (sx * 0.6, (y0 + y1) / 2, -0.05), m["floor"], structure=True)
        k.box(f"angle_{sx}", (0.04, y1 - y0, 0.012), (sx * 0.42, (y0 + y1) / 2, 0.006), m["dark"])
    k.col("floor", (2.8, y1 - y0 + 0.4, 0.2), (0, (y0 + y1) / 2, -0.1))

    # Coque à pans coupés : chanfreins bas sombres (plinthes), murs et chanfreins hauts peints.
    # Une plaque par travée de 2 m, texture décalée d'une travée à l'autre : la nervure cache le
    # raccord et le motif (grilles, trappe) ne revient plus au même endroit.
    bay_offsets = [(0.0, 0.0), (0.5, 0.25), (0.25, 0.5), (0.75, 0.75)]
    bays = [(y0 + 2.0 * b, y0 + 2.0 * (b + 1), bay_offsets[b % 4]) for b in range(int((y1 - y0) / 2.0))]
    for i, (p1, p2) in enumerate(octagon_sides()):
        lower = p1[1] < 0.1
        mat = m["dark"] if lower else m["panel"]
        for b, (ya, yb, off) in enumerate(bays):
            slab(k, f"hull_{i}_{b}", p1, p2, ya, yb, wall_t, mat, uv=("box", off), structure=True)
        if lower:
            # Plat rivé de 12 cm au pied du chanfrein, là où les pieds frottent.
            d = Vector((p2[0] - p1[0], p2[1] - p1[1]))
            q = (p1[0] + d.x * 0.12 / d.length, p1[1] + d.y * 0.12 / d.length)
            slab(k, f"kick_{i}", p1, q, y0, y1, 0.012, m["trim"], uv="rivets", inset=0.012)
    for b, (ya, yb, off) in enumerate(bays):
        slab(k, f"ceiling_{b}", (-0.6, 2.6), (0.6, 2.6), ya, yb, wall_t, m["panel"], uv=("box", off), structure=True)
    for sx in (-1, 1):
        k.col(f"wall_{sx}", (0.2, y1 - y0 + 0.4, 2.8), (sx * 1.3, (y0 + y1) / 2, 1.3))
        # Chanfrein bas : une marche trop haute pour le contrôleur (0,42 m > STEP_HEIGHT).
        k.col(f"kerb_{sx}", (0.45, y1 - y0, 0.42), (sx * 1.025, (y0 + y1) / 2, 0.21))
    k.col("ceiling", (2.8, y1 - y0 + 0.4, 0.2), (0, (y0 + y1) / 2, 2.7))

    # Nervures tous les 2 m : cadre octogonal rivé.
    for yr in (2.0, 4.0, 6.0):
        for i, (p1, p2) in enumerate(octagon_sides() + [((-0.6, 2.6), (0.6, 2.6))]):
            # Rallonge = épaisseur × tan(22,5°) : les nervures se rejoignent sans encoche. Les
            # nervures des chanfreins sont un peu plus étroites : leurs flancs ne sont pas
            # coplanaires avec ceux des nervures voisines (faces confondues : taches noires).
            chamfer = abs(p1[0] - p2[0]) > 0.01 and abs(p1[1] - p2[1]) > 0.01
            half = 0.086 if chamfer else 0.09
            slab(k, f"rib_{yr:g}_{i}", p1, p2, yr - half, yr + half, 0.09, m["trim"], uv="rivets", inset=0.09, extend=0.09 * 0.4142, structure=True)
        # Gousset du pied de nervure, sur le sol latéral.
        for sx in (-1, 1):
            k.box(f"foot_{yr:g}_{sx}", (0.1, 0.22, 0.05), (sx * 0.74, yr, 0.025), m["dark"], bevel=0.008)

    # Bandeaux lumineux dans les angles hauts (entre mur et chanfrein), carter sombre.
    for sx in (-1, 1):
        nrm = Vector((-sx * 0.92, 0.0, -0.38)).normalized()  # vers l'axe, un peu vers le bas
        right = Vector((0.0, 1.0, 0.0)) if sx < 0 else Vector((0.0, -1.0, 0.0))
        up = nrm.cross(right)  # (right, up, right × up = nrm)
        corner =Vector((sx * 1.2, (y0 + y1) / 2, 2.0)) + nrm * 0.07
        k.oriented(f"band_housing_{sx}", (y1 - y0 - 0.1, 0.16, 0.06), corner - nrm * 0.035, right, up, m["dark"], bevel=0.01)
        k.oriented(f"band_{sx}", (y1 - y0 - 0.3, 0.085, 0.02), corner + nrm * 0.002, right, up, m["band_cool"], uv=("strip", 1.0))

    # Plafond : chemin de câbles central, deux tuyaux à brides, colliers tous les 1 m.
    k.box("tray", (0.34, y1 - y0, 0.06), (0, (y0 + y1) / 2, 2.47), m["trim"], uv="cables")
    for yb in [0.5 + i for i in range(8)]:
        k.box(f"tray_strap_{yb:g}", (0.42, 0.04, 0.14), (0, yb, 2.53), m["dark"])
    for sx, r in ((-1, 0.05), (1, 0.038)):
        x = sx * 0.5
        k.cyl(f"pipe_{sx}", r, y1 - y0, (x, (y0 + y1) / 2, 2.44), m["trim"], axis="Y", segments=16, uv="flange")
        for yb in [1.0 + 2 * i for i in range(4)]:
            k.box(f"pipe_clamp_{sx}_{yb:g}", (0.03, 0.05, 0.16), (x, yb, 2.5), m["dark"])

    # Petits plafonniers froids sous le chemin de câbles (lumières du jeu : reflets en Haut).
    for i, yl in enumerate((1.5, 4.0, 6.5)):
        k.cyl(f"lamp_ring_{i}", 0.09, 0.03, (0, yl, 2.425), m["dark"], segments=20)
        k.cyl(f"lamp_lens_{i}", 0.07, 0.012, (0, yl, 2.405), m["light"], segments=20)
        k.light(f"l{i}", (0, yl, 2.2), 2500.0, 6.0, COOL, shadow=(i == 1))

    # Mur du fond (y = 0) : panneau de service, tuyau qui plonge, pochoirs.
    k.box("back_wall", (2.8, 0.2, 2.8), (0, y0 - 0.1, 1.3), m["panel"], structure=True)
    k.col("back_wall", (2.8, 0.2, 2.8), (0, y0 - 0.1, 1.3))
    k.box("service_box", (1.0, 0.14, 1.3), (-0.15, y0 + 0.07, 1.25), m["dark"], bevel=0.015)
    # Faces tournées vers +Y : « droite » vue de la coursive = −X.
    k.oriented("service_buttons", (0.5, 0.36, 0.012), (-0.35, y0 + 0.145, 1.45), (-1, 0, 0), (0, 0, 1), m["buttons"])
    k.oriented("service_screen", (0.32, 0.22, 0.012), (0.12, y0 + 0.145, 1.5), (-1, 0, 0), (0, 0, 1), m["screen"])
    k.oriented("service_vent", (0.8, 0.25, 0.01), (-0.15, y0 + 0.145, 0.85), (-1, 0, 0), (0, 0, 1), m["trim"], uv="vent")
    k.cyl("back_pipe", 0.05, 2.0, (0.75, y0 + 0.12, 1.3), m["trim"], segments=16, uv="flange")

    # Mur gauche : boîtier de jonction et sa gaine ; mur droit : écran mural, pochoirs.
    k.box("junction", (0.16, 0.45, 0.55), (-1.12, 3.1, 1.25), m["dark"], bevel=0.012)
    # Mur gauche (faces vers +X) : droite = +Y ; mur droit (faces vers −X) : droite = −Y.
    k.oriented("junction_buttons", (0.36, 0.28, 0.01), (-1.035, 3.1, 1.3), (0, 1, 0), (0, 0, 1), m["buttons"])
    k.cyl("junction_conduit", 0.025, 0.45, (-1.12, 3.1, 1.75), m["trim"], segments=12, uv="flange")
    k.col("junction", (0.2, 0.5, 0.6), (-1.1, 3.1, 1.25))
    k.box("screen_housing", (0.08, 0.66, 0.46), (1.16, 4.6, 1.55), m["dark"], bevel=0.015)
    k.oriented("screen", (0.58, 0.38, 0.01), (1.115, 4.6, 1.55), (0, -1, 0), (0, 0, 1), m["screen"], tile=0.8)
    k.oriented("stencils", (1.5, 0.12, 0.004), (1.198, 1.9, 1.86), (0, -1, 0), (0, 0, 1), m["trim"], uv="stencils")
    k.oriented("stencils_l", (1.0, 0.12, 0.004), (-1.198, 5.6, 1.86), (0, 1, 0), (0, 0, 1), m["trim"], uv="stencils")
    # Commande de porte, à droite de l'ouverture.
    k.box("door_ctrl_box", (0.06, 0.26, 0.38), (1.17, 7.55, 1.25), m["dark"], bevel=0.01)
    k.oriented("door_ctrl", (0.2, 0.3, 0.01), (1.137, 7.55, 1.25), (0, -1, 0), (0, 0, 1), m["buttons"])

    # Mur de la porte (y = 8) et cadre épais à pans coupés, bande de danger au seuil.
    dw, dh = kit.DOOR_W, kit.DOOR_H
    wy = y1 + 0.1
    k.box("door_wall_l", (1.4 - dw / 2, 0.2, 2.8), (-(1.4 + dw / 2) / 2, wy, 1.3), m["panel"], structure=True)
    k.box("door_wall_r", (1.4 - dw / 2, 0.2, 2.8), ((1.4 + dw / 2) / 2, wy, 1.3), m["panel"], structure=True)
    k.box("door_wall_top", (dw, 0.2, 2.7 - dh), (0, wy, (dh + 2.7) / 2), m["panel"], structure=True)
    k.col("door_wall_l", (1.4 - dw / 2, 0.2, 2.8), (-(1.4 + dw / 2) / 2, wy, 1.3))
    k.col("door_wall_r", (1.4 - dw / 2, 0.2, 2.8), ((1.4 + dw / 2) / 2, wy, 1.3))
    k.col("door_wall_top", (dw, 0.2, 2.8 - dh), (0, wy, (dh + 2.8) / 2))
    door_frame(k, m, y1, -1)
    k.oriented("threshold", (dw, 0.25, 0.006), (0, y1 - 0.2, 0.003), (1, 0, 0), (0, 1, 0), m["trim"], uv="hazard")
    return k


def door_frame(k: Cell, m: dict, face_y: float, side: int):
    """Cadre épais autour de l'ouverture, côté `side` (−1 : la pièce est vers −Y) : montants et
    linteau biseautés, goussets à 45° dans les coins hauts (ouverture à pans coupés), tableau
    sombre dans l'épaisseur du mur. L'ouverture libre reste DOOR_W × DOOR_H."""
    dw, dh = kit.DOOR_W, kit.DOOR_H
    depth = 0.16
    fy = face_y + side * depth / 2
    facing = (1, 0, 0) if side < 0 else (-1, 0, 0)  # « droite » vue de la pièce : face vers la pièce
    for sx in (-1, 1):
        k.box(f"jamb_{sx}", (0.24, depth, dh), (sx * (dw / 2 + 0.12), fy, dh / 2), m["dark"], bevel=0.03, structure=True)
        k.col(f"jamb_{sx}", (0.24, depth, dh), (sx * (dw / 2 + 0.12), fy, dh / 2))
        # Bande de danger verticale sur la face du montant (normale vers la pièce).
        k.oriented(f"jamb_hazard_{sx}", (dh * 0.5, 0.12, 0.006), (sx * (dw / 2 + 0.12), face_y + side * (depth + 0.003), 0.55),
                   (0, 0, 1), (side * 1.0, 0, 0), m["trim"], uv="hazard")
        # Tableau sombre dans l'épaisseur du mur (moitié de cette cellule), 1 cm devant la
        # tranche du mur : il cache les bords du panneau de porte fermé.
        k.box(f"reveal_{sx}", (0.012, 0.2, dh), (sx * (dw / 2 - 0.004), DOOR_Y + side * 0.1, dh / 2), m["dark"])
    k.box("header", (dw + 0.48, depth, 0.3), (0, fy, dh + 0.15), m["dark"], bevel=0.03, structure=True)
    k.box("reveal_top", (dw, 0.2, 0.012), (0, DOOR_Y + side * 0.1, dh - 0.004), m["dark"])
    for sx in (-1, 1):
        # Goussets à 45° : l'ouverture se lit à pans coupés, comme la coursive.
        k.oriented(f"gusset_{sx}", (0.34, 0.12, depth - 0.012), (sx * (dw / 2 - 0.08), fy, dh - 0.08),
                   (sx * 0.7071, 0, 0.7071), (-sx * 0.7071, 0, 0.7071), m["dark"], bevel=0.02)
    k.oriented("header_lamp", (0.6, 0.06, 0.02), (0, face_y + side * (depth + 0.012), dh + 0.2), facing, (0, 0, 1), m["trim"], uv="light_strip")


# --- Atelier -------------------------------------------------------------------------------------


def build_atelier(m: dict) -> Cell:
    k = Cell("atelier")
    x0, x1, y0, y1, h = CELLS["atelier"]
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    W = 0.2

    # Sol, plafond, murs (soubassement sombre, chanfrein, panneaux peints au-dessus).
    k.box("floor", (x1 - x0 + 2 * W, y1 - y0 + 2 * W, W), (cx, cy, -W / 2), m["floor"], structure=True)
    k.col("floor", (x1 - x0 + 2 * W, y1 - y0 + 2 * W, W), (cx, cy, -W / 2))
    k.box("ceiling", (x1 - x0 + 2 * W, y1 - y0 + 2 * W, W), (cx, cy, h + W / 2), m["dark"], structure=True)
    k.col("ceiling", (x1 - x0 + 2 * W, y1 - y0 + 2 * W, W), (cx, cy, h + W / 2))
    for sx in (-1, 1):
        xw = sx * (x1 + W / 2)
        k.box(f"wall_{sx}", (W, y1 - y0 + 2 * W, h), (xw, cy, h / 2), m["panel"], structure=True)
        k.col(f"wall_{sx}", (W, y1 - y0 + 2 * W, h + 0.4), (xw, cy, h / 2))
        k.box(f"wainscot_{sx}", (0.04, y1 - y0, 0.9), (sx * (x1 - 0.02), cy, 0.45), m["dark"])
        k.oriented(f"chair_rail_{sx}", (y1 - y0, 0.11, 0.05), (sx * (x1 - 0.045), cy, 0.95), (0, -sx, 0), (0, 0, 1), m["trim"], uv="edge")
        k.oriented(f"kick_{sx}", (y1 - y0, 0.12, 0.012), (sx * (x1 - 0.046), cy, 0.065), (0, -sx, 0), (0, 0, 1), m["trim"], uv="rivets")
    k.box("back_wall", (x1 - x0 + 2 * W, W, h), (cx, y1 + W / 2, h / 2), m["panel"], structure=True)
    k.col("back_wall", (x1 - x0 + 2 * W, W, h + 0.4), (cx, y1 + W / 2, h / 2))
    k.box("back_wainscot", (x1 - x0, 0.04, 0.9), (cx, y1 - 0.02, 0.45), m["dark"])
    k.oriented("back_rail", (x1 - x0, 0.11, 0.05), (cx, y1 - 0.045, 0.95), (1, 0, 0), (0, 0, 1), m["trim"], uv="edge")

    # Mur de la porte (y = 8,4) et son cadre.
    dw, dh = kit.DOOR_W, kit.DOOR_H
    wy = y0 - W / 2
    half = (x1 - x0) / 2 + W
    k.box("door_wall_l", (half - dw / 2, W, h), (-(half + dw / 2) / 2, wy, h / 2), m["panel"], structure=True)
    k.box("door_wall_r", (half - dw / 2, W, h), ((half + dw / 2) / 2, wy, h / 2), m["panel"], structure=True)
    k.box("door_wall_top", (dw, W, h - dh), (0, wy, (h + dh) / 2), m["panel"], structure=True)
    k.col("door_wall_l", (half - dw / 2, W, h + 0.4), (-(half + dw / 2) / 2, wy, h / 2))
    k.col("door_wall_r", (half - dw / 2, W, h + 0.4), ((half + dw / 2) / 2, wy, h / 2))
    k.col("door_wall_top", (dw, W, h + 0.2 - dh), (0, wy, (h + 0.2 + dh) / 2))
    for sx in (-1, 1):
        k.box(f"door_wainscot_{sx}", (half - dw / 2 - 0.45, 0.04, 0.9), (sx * (dw / 2 + 0.24 + (half - dw / 2 - 0.45) / 2), y0 + 0.02, 0.45), m["dark"])
    door_frame(k, m, y0, 1)

    # Colonnes en H contre les murs et poutres transversales tous les 2 m.
    beams_y = (y0 + 2.0, y0 + 4.0, y0 + 6.0)
    for yb in beams_y:
        for sx in (-1, 1):
            xw = sx * x1
            k.box(f"col_web_{yb:g}_{sx}", (0.18, 0.025, h), (xw - sx * 0.1, yb, h / 2), m["dark"], structure=True)
            k.box(f"col_back_{yb:g}_{sx}", (0.025, 0.26, h), (xw - sx * 0.0125, yb, h / 2), m["dark"])
            k.oriented(f"col_front_{yb:g}_{sx}", (h, 0.26, 0.025), (xw - sx * 0.19, yb, h / 2), (0, 0, 1), (0, sx * 1.0, 0), m["trim"], uv="rivets", structure=True)
            k.col(f"col_{yb:g}_{sx}", (0.2, 0.28, h), (xw - sx * 0.1, yb, h / 2))
        k.box(f"beam_web_{yb:g}", (x1 - x0, 0.025, 0.26), (cx, yb, h - 0.15), m["dark"], structure=True)
        k.box(f"beam_top_{yb:g}", (x1 - x0, 0.26, 0.025), (cx, yb, h - 0.0125), m["dark"])
        k.oriented(f"beam_bot_{yb:g}", (x1 - x0, 0.26, 0.025), (cx, yb, h - 0.29), (1, 0, 0), (0, -1, 0), m["trim"], uv="rivets", structure=True)

    # Tuyaux à brides le long du mur gauche, gaine de ventilation à droite, chemin de câbles.
    for i, (xp, r, zp) in enumerate(((-2.55, 0.12, 2.62), (-2.2, 0.075, 2.7))):
        k.cyl(f"pipe_{i}", r, y1 - y0, (xp, cy, zp), m["trim"], axis="Y", segments=20, uv="flange")
        for yb in beams_y:
            k.box(f"pipe_hanger_{i}_{yb:g}", (0.03, 0.04, h - 0.29 - zp), (xp, yb + 0.16, (h - 0.29 + zp) / 2), m["dark"])
    k.box("duct", (0.55, y1 - y0, 0.36), (2.25, cy, 2.62), m["bare"])
    for yj in [y0 + 0.5 + i for i in range(8)]:
        k.box(f"duct_joint_{yj:g}", (0.59, 0.05, 0.4), (2.25, yj, 2.62), m["dark"])
    k.oriented("duct_vent", (0.5, 0.25, 0.01), (2.25, y0 + 3.0, 2.435), (0, 1, 0), (1, 0, 0), m["trim"], uv="vent")
    k.box("tray", (0.4, y1 - y0, 0.06), (0.1, cy, 2.86), m["trim"], uv="cables")

    # Lampes sodium suspendues (lentilles masquées du bake : les lampes du jeu les remplacent).
    for i, (xl, yl) in enumerate(((-1.2, y0 + 3.0), (1.0, y0 + 3.0), (-1.2, y0 + 5.0), (1.0, y0 + 5.0))):
        # Cloche profonde, petite lentille en retrait, cage à quatre montants et croisillon.
        k.box(f"lamp_rod_{i}", (0.025, 0.025, 0.3), (xl, yl, h - 0.15), m["dark"])
        k.cyl(f"lamp_cap_{i}", 0.07, 0.06, (xl, yl, h - 0.33), m["dark"], segments=16)
        k.cyl(f"lamp_hood_{i}", 0.2, 0.1, (xl, yl, h - 0.41), m["dark"], segments=24)
        k.cyl(f"lamp_lens_{i}", 0.12, 0.03, (xl, yl, h - 0.465), m["light_amber"], segments=20)
        for a in range(4):
            ang = a * math.pi / 2 + math.pi / 4
            k.box(f"lamp_cage_{i}_{a}", (0.014, 0.014, 0.13), (xl + 0.17 * math.cos(ang), yl + 0.17 * math.sin(ang), h - 0.525), m["dark"])
        for a in range(2):
            ang = a * math.pi / 2 + math.pi / 4
            k.oriented(f"lamp_cross_{i}_{a}", (0.5, 0.014, 0.014), (xl, yl, h - 0.59), (math.cos(ang), math.sin(ang), 0), (-math.sin(ang), math.cos(ang), 0), m["dark"])
        k.light(f"sodium_{i}", (xl, yl, h - 0.65), 12000.0, 9.0, SODIUM, shadow=(i == 0))

    # Bandeau ambre sous la gaine, au-dessus de la console.
    nrm = Vector((-0.6, 0.0, -0.8))
    k.oriented("band_amber", (1.55, 0.085, 0.02), (2.9, y0 + 3.0, 2.34), (0, -1, 0), nrm.cross(Vector((0, -1, 0))), m["band_amber"], uv=("strip", 1.0))
    k.oriented("band_amber_housing", (1.65, 0.16, 0.05), (2.93, y0 + 3.0, 2.37), (0, -1, 0), nrm.cross(Vector((0, -1, 0))), m["dark"], bevel=0.01)

    build_console(k, m, (2.45, y0 + 3.0))

    # Caisses empilées (coin du fond à gauche), casier près de la porte, pompe au fond à droite.
    for i, (pos, size, rot) in enumerate((((-2.35, y1 - 0.65), 1.0, 0.0), ((-1.25, y1 - 0.55), 0.8, 0.14), ((-2.3, y1 - 0.7), 0.62, -0.2))):
        z0 = 1.0 if i == 2 else 0.0
        crate(k, m, f"crate{i}", (pos[0], pos[1], z0), size, rot)
    locker(k, m, (-2.68, y0 + 0.95))
    pump(k, m, (1.7, y1 - 0.75))

    # Fond : armoire électrique entre les caisses et la pompe, conduits verticaux, pochoirs.
    k.box("cabinet", (1.1, 0.32, 1.7), (-0.1, y1 - 0.16, 1.15), m["dark"], bevel=0.015)
    k.oriented("cabinet_buttons", (0.5, 0.4, 0.01), (-0.3, y1 - 0.325, 1.45), (1, 0, 0), (0, 0, 1), m["buttons"])
    k.oriented("cabinet_screen", (0.3, 0.22, 0.01), (0.2, y1 - 0.325, 1.55), (1, 0, 0), (0, 0, 1), m["screen"], tile=0.75)
    k.oriented("cabinet_vent", (0.9, 0.25, 0.006), (-0.1, y1 - 0.323, 0.62), (1, 0, 0), (0, 0, 1), m["trim"], uv="vent")
    k.oriented("cabinet_label", (0.7, 0.12, 0.004), (-0.1, y1 - 0.323, 1.9), (1, 0, 0), (0, 0, 1), m["trim"], uv="stencils")
    k.col("cabinet", (1.1, 0.34, 1.9), (-0.1, y1 - 0.17, 1.15))
    for i, xc_ in enumerate((0.58, 0.68)):
        k.cyl(f"conduit_{i}", 0.03, h, (xc_, y1 - 0.06, h / 2), m["trim"], segments=12, uv="flange")
    # Mur gauche, entre deux colonnes : armoire murale, conduit et boîtier de commande.
    k.box("wall_cabinet", (0.3, 1.0, 0.8), (x0 + 0.19, y0 + 3.0, 1.55), m["dark"], bevel=0.015)
    k.oriented("wall_cabinet_vent", (0.85, 0.25, 0.006), (x0 + 0.343, y0 + 3.0, 1.3), (0, 1, 0), (0, 0, 1), m["trim"], uv="vent")
    k.oriented("wall_cabinet_label", (0.6, 0.12, 0.004), (x0 + 0.343, y0 + 3.0, 1.82), (0, 1, 0), (0, 0, 1), m["trim"], uv="stencils")
    k.box("wall_box", (0.12, 0.3, 0.4), (x0 + 0.1, y0 + 4.6, 1.3), m["dark"], bevel=0.01)
    k.oriented("wall_box_buttons", (0.24, 0.32, 0.008), (x0 + 0.164, y0 + 4.6, 1.3), (0, 1, 0), (0, 0, 1), m["buttons"])
    k.cyl("wall_conduit", 0.025, h - 1.5, (x0 + 0.06, y0 + 4.6, 1.5 + (h - 1.5) / 2), m["trim"], segments=12, uv="flange")
    k.col("wall_cabinet", (0.34, 1.0, 0.8), (x0 + 0.19, y0 + 3.0, 1.55))
    # Palette et petites caisses à côté de la pile.
    k.box("pallet", (1.0, 0.8, 0.12), (-1.2, y1 - 1.75, 0.06), m["dark"], bevel=0.01)
    crate(k, m, "crate3", (-1.4, y1 - 1.75, 0.12), 0.45, 0.3)
    crate(k, m, "crate4", (-0.95, y1 - 1.65, 0.12), 0.4, -0.15)
    k.col("pallet", (1.0, 0.8, 0.12), (-1.2, y1 - 1.75, 0.06))

    # Trappe au sol cerclée de bandes de danger, poignées ; gyrophare rouge sur la colonne.
    hx, hy, hs = 0.6, y0 + 5.2, 1.2
    k.box("hatch", (hs, hs, 0.012), (hx, hy, 0.006), m["floor"], bevel=0.004)
    for i, (dx, dy, sx_, sy_, right) in enumerate(((0, hs / 2 + 0.07, hs + 0.28, 0.14, (1, 0, 0)), (0, -hs / 2 - 0.07, hs + 0.28, 0.14, (-1, 0, 0)),
                                                   (hs / 2 + 0.07, 0, hs, 0.14, (0, -1, 0)), (-hs / 2 - 0.07, 0, hs, 0.14, (0, 1, 0)))):
        k.oriented(f"hatch_hazard_{i}", (sx_, sy_, 0.008 if i < 2 else 0.0065), (hx + dx, hy + dy, 0.004 if i < 2 else 0.00325), right, (-right[1], right[0], 0), m["trim"], uv="hazard")
    for sy in (-1, 1):
        k.cyl(f"hatch_handle_{sy}", 0.012, 0.22, (hx, hy + sy * 0.4, 0.03), m["bare"], axis="X", segments=10)
    k.cyl("beacon_base", 0.07, 0.06, (x1 - 0.22, beams_y[2], 2.25), m["dark"], axis="X", segments=16)
    k.cyl("beacon", 0.055, 0.1, (x1 - 0.3, beams_y[2], 2.25), m["beacon"], axis="X", segments=16)
    k.light("beacon", (x1 - 0.5, beams_y[2], 2.25), 900.0, 4.0, RED)
    return k


def build_console(k: Cell, m: dict, at):
    """Console inclinée contre le mur droit, face à −X : caisson, pupitre à boutons, panneau
    d'écrans incliné, joues biseautées."""
    xc, yc = at
    width = 1.6
    k.box("console_base", (0.62, width, 0.82), (xc + 0.12, yc, 0.41), m["dark"], bevel=0.02)
    k.box("console_kick", (0.06, width - 0.1, 0.1), (xc - 0.2, yc, 0.05), m["rubber"])
    a = math.radians(24)
    right, up_desk = (0, -1, 0), (math.cos(a), 0, math.sin(a))
    k.oriented("console_desk", (width - 0.08, 0.5, 0.05), (xc + 0.05, yc, 0.93), right, up_desk, m["dark"], bevel=0.01)
    k.oriented("console_buttons", (width - 0.2, 0.42, 0.012), (xc + 0.04, yc, 0.958), right, up_desk, m["buttons"])
    k.oriented("console_lip", (width - 0.05, 0.05, 0.06), (xc - 0.2, yc, 0.84), right, (0, 0, 1), m["trim"], uv="gasket")
    b = math.radians(14)
    up_scr = (math.sin(b), 0, math.cos(b))
    k.oriented("console_panel", (width - 0.05, 0.62, 0.08), (xc + 0.33, yc, 1.42), right, up_scr, m["dark"], bevel=0.015)
    for i, (dy, w_, h_) in enumerate(((-0.5, 0.42, 0.34), (0.0, 0.46, 0.4), (0.5, 0.42, 0.34))):
        k.oriented(f"console_screen_{i}", (w_, h_, 0.012), (xc + 0.285 + 0.0, yc - dy, 1.44), right, up_scr, m["screen"], tile=0.75)
    for sy in (-1, 1):
        k.oriented(f"console_cheek_{sy}", (0.7, 1.6, 0.05), (xc + 0.16, yc + sy * (width / 2 + 0.02), 0.8), (1, 0, 0), (0, 0, 1), m["dark"], bevel=0.02)
    k.cyl("console_rail", 0.018, width - 0.1, (xc - 0.3, yc, 0.9), m["bare"], axis="Y", segments=12)
    k.col("console", (0.8, width + 0.1, 1.0), (xc + 0.1, yc, 0.5))
    k.col("console_panel", (0.3, width, 0.8), (xc + 0.35, yc, 1.4))


def crate(k: Cell, m: dict, name: str, base, size: float, rot: float):
    """Caisse nervurée olive à cornières sombres et étiquette à pochoirs."""
    x, y, z = base
    s = size
    cs, sn = math.cos(rot), math.sin(rot)
    right, fwd = (cs, sn, 0), (-sn, cs, 0)

    def at(dx, dy, dz):
        return (x + dx * cs - dy * sn, y + dx * sn + dy * cs, z + dz)

    k.oriented(f"{name}_body", (s - 0.04, s - 0.04, s - 0.04), at(0, 0, s / 2), right, fwd, m["crate"], bevel=0.008)
    for dx in (-1, 1):
        for dy in (-1, 1):
            k.oriented(f"{name}_post_{dx}_{dy}", (0.06, 0.06, s), at(dx * (s / 2 - 0.03), dy * (s / 2 - 0.03), s / 2), right, fwd, m["dark"], bevel=0.006)
    for dz in (0.03, s - 0.03):
        k.oriented(f"{name}_rim_{dz:g}", (s - 0.008, s - 0.008, 0.06), at(0, 0, dz), right, fwd, m["dark"], bevel=0.006)
    k.oriented(f"{name}_label", (s * 0.55, 0.12, 0.004), at(0, -(s / 2 - 0.018), s * 0.55), (cs, sn, 0), (0, 0, 1), m["trim"], uv="stencils")
    corners = [at(dx * s / 2, dy * s / 2, 0) for dx in (-1, 1) for dy in (-1, 1)]
    lo = [min(p[0] for p in corners), min(p[1] for p in corners)]
    hi = [max(p[0] for p in corners), max(p[1] for p in corners)]
    k.col(name, (hi[0] - lo[0], hi[1] - lo[1], s), ((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, z + s / 2))


def locker(k: Cell, m: dict, at):
    x, y = at
    k.box("locker_body", (0.55, 0.62, 2.0), (x, y, 1.0), m["dark"], bevel=0.015)
    for i, z in enumerate((1.72, 0.3)):
        k.oriented(f"locker_vent_{i}", (0.5, 0.18, 0.006), (x + 0.278, y, z), (0, 1, 0), (0, 0, 1), m["trim"], uv="vent")
    k.box("locker_seam", (0.01, 0.02, 1.9), (x + 0.277, y, 1.0), m["rubber"])
    k.box("locker_handle", (0.04, 0.03, 0.18), (x + 0.29, y + 0.08, 1.1), m["bare"])
    k.col("locker", (0.6, 0.65, 2.0), (x, y, 1.0))


def pump(k: Cell, m: dict, at):
    x, y = at
    k.box("pump_skid", (1.5, 0.9, 0.18), (x, y, 0.09), m["dark"], bevel=0.01)
    k.cyl("pump_tank", 0.36, 1.25, (x - 0.3, y, 0.18 + 0.625), m["bare"], segments=24)
    for zr in (0.3, 0.8, 1.35):
        k.cyl(f"pump_ring_{zr:g}", 0.38, 0.05, (x - 0.3, y, zr), m["dark"], segments=24)
    k.box("pump_motor", (0.5, 0.55, 0.5), (x + 0.42, y, 0.43), m["dark"], bevel=0.02)
    k.oriented("pump_panel", (0.4, 0.3, 0.01), (x + 0.42, y - 0.28, 0.48), (1, 0, 0), (0, 0, 1), m["buttons"])
    k.cyl("pump_riser", 0.07, 3.2 - 1.43, (x - 0.3, y, 1.43 + (3.2 - 1.43) / 2), m["trim"], segments=16, uv="flange")
    k.cyl("pump_hose", 0.04, 0.8, (x + 0.1, y - 0.2, 0.75), m["rubber"], axis="X", segments=12)
    k.col("pump", (1.5, 0.95, 1.45), (x, y, 0.72))


# --- Export, lightmaps, descripteur, aperçus ------------------------------------------------------


def export(cells: dict, bake: bool) -> dict:
    exports = c.collection("EXPORT")
    resolutions = {cid: lm.resolution_for(k.visual) for cid, k in cells.items()}
    bake_input = {}
    stats = {}
    quiet = io.StringIO()
    with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
        for cid, k in cells.items():
            lm.unwrap(k.visual, resolutions[cid])
            merged = c.join_by_material(k.visual, cid, exports)
            c.export_gltf(f"assets/{MODELS}/{cid}.gltf", merged + k.colliders + k.marks)
            lod = c.join_by_material(k.structure, f"{cid}_lod1", exports)
            c.export_gltf(f"assets/{MODELS}/{cid}_lod.gltf", lod)
            for obj in merged + lod:
                obj.hide_set(True)
            stats[cid] = {
                "tris": sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in merged),
                "lod_tris": sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in lod),
                "materials": sorted({o.data.materials[0].name for o in merged}),
                "res": resolutions[cid],
                "merged": merged,
            }
            bake_input[cid] = {"bake": merged, "res": resolutions[cid], "marks": k.marks}
    for cid, s in stats.items():
        print(f"cellule {cid} : {s['tris']} triangles, {len(s['materials'])} matériaux, LOD {s['lod_tris']} triangles, lightmap {s['res']} px")
        print(f"  matériaux : {', '.join(s['materials'])}")
    lightmaps = {}
    if bake:
        # Les lentilles ambrées sont remplacées par leurs lampes (comme `sh_light`, que
        # lightmap.py masque lui-même) : elles n'éclairent pas une seconde fois.
        amber = [o for s in stats.values() for o in s["merged"] if o.data.materials[0].name == "sh_light_amber"]
        for obj in amber:
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = False
        lightmaps = lm.bake_cells(bake_input, LIGHTMAPS, c.collection("BAKE_LAMPS"))
        for obj in amber:
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = True
    return {"stats": stats, "lightmaps": lightmaps}


def write_descriptor(lightmaps: dict) -> None:
    meta = {
        "script": "tools/blender/materials/salle_temoin.py",
        "comment": "Salle témoin de la direction « futur usé » (M8, lot 0 ; proposition P13) : coursive et atelier.",
        "id": SECTOR,
        "kind": "Ship",
        "name_key": f"sector.{SECTOR}",
        "status": 'Proposition("P13")',
        "door_model": "models/kit/ship_interior/door_sliding.gltf",
        "models": MODELS,
    }
    text = st.sector_ron(meta, CELLS, PORTALS, SPAWNS, lightmaps)
    # Modèles en glTF séparé (le modèle de porte du kit reste en .glb).
    lines = []
    for line in text.splitlines():
        stripped = line.lstrip()
        if stripped.startswith("model:") or stripped.startswith("lod:"):
            line = line.replace('.glb"', '.gltf"')
        lines.append(line)
    c.write_text(f"assets/data/propositions/sectors/{SECTOR}.ron", "\n".join(lines) + "\n")


def previews(samples: int, cells: dict) -> None:
    """Rendus Cycles à hauteur d'œil (1,7 m) : lampes du jeu + surfaces émissives, lentilles des
    plafonniers visibles mais sans double éclairage."""
    import preview as pv

    pv.setup_cycles(samples, 1600, 900, exposure=float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2")))
    pv.world((0, 0, 0), 0.0)
    marks = [mk for k in cells.values() for mk in k.marks]
    lamps = lm.spawn_lamps(marks, c.collection("PREVIEW_LAMPS"))
    for lamp in lamps:
        lamp.data.shadow_soft_size = 0.06
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH" and obj.data.materials and obj.data.materials[0] and obj.data.materials[0].name in ("sh_light", "sh_light_amber"):
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = False
    only = os.environ.get("SH_PREVIEW_ONLY")
    for name, (loc, target, lens) in SHOTS.items():
        if only and name not in only.split(","):
            continue
        pv.camera("cam", loc, target, lens=lens)
        pv.render(pv.PREVIEWS / f"salle_{name}.png")
        print(f"aperçu : salle_{name}.png")


SHOTS = {
    "coursive_enfilade": ((0.0, 0.35, 1.7), (0.0, 8.0, 1.35), 18),
    "coursive_detail_mur_sol": ((0.35, 2.6, 1.25), (-1.15, 3.6, 0.45), 32),
    "atelier_vue_large": ((-1.75, 8.75, 1.7), (1.6, 14.8, 1.0), 17),
    "atelier_console": ((1.05, 10.2, 1.62), (2.55, 11.4, 1.08), 28),
    "atelier_vers_porte": ((0.35, 15.75, 1.7), (-0.3, 8.6, 1.2), 18),
    "atelier_trappe": ((-1.6, 11.2, 1.7), (0.8, 13.6, 0.0), 24),
}


def _game_material(mat, image, scale: float, tag: str):
    """Copie d'un matériau qui rend ce que le jeu affiche dans une cellule à lightmap, en Moyen
    (aucune lumière dynamique) : émission = couleur × (1 − métal) × lightmap × échelle +
    émissif. Les normales n'y jouent pas (la lightmap ne les lit pas)."""
    g = mat.copy()
    g.name = f"game_{tag}_{mat.name}"
    nodes, links = g.node_tree.nodes, g.node_tree.links
    bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
    out = next(n for n in nodes if n.type == "OUTPUT_MATERIAL")

    def source(name):
        s = bsdf.inputs[name]
        return s.links[0].from_socket if s.is_linked else None, s.default_value

    def feed(socket, src, value):
        if src is not None:
            links.new(src, socket)
        else:
            socket.default_value = value if not hasattr(value, "__len__") else tuple(value)[: len(socket.default_value)]

    base_src, base_val = source("Base Color")
    met_src, met_val = source("Metallic")
    em_src, em_val = source("Emission Color")
    strength = bsdf.inputs["Emission Strength"].default_value
    uv = nodes.new("ShaderNodeUVMap")
    uv.uv_map = lm.UV_NAME
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = image
    links.new(uv.outputs["UV"], tex.inputs["Vector"])
    mul = nodes.new("ShaderNodeVectorMath")
    mul.operation = "MULTIPLY"
    feed(mul.inputs[0], base_src, base_val)
    links.new(tex.outputs["Color"], mul.inputs[1])
    inv = nodes.new("ShaderNodeMath")
    inv.operation = "SUBTRACT"
    inv.inputs[0].default_value = 1.0
    feed(inv.inputs[1], met_src, met_val)
    k = nodes.new("ShaderNodeMath")
    k.operation = "MULTIPLY"
    links.new(inv.outputs[0], k.inputs[0])
    k.inputs[1].default_value = scale
    diffuse = nodes.new("ShaderNodeVectorMath")
    diffuse.operation = "SCALE"
    links.new(mul.outputs[0], diffuse.inputs[0])
    links.new(k.outputs[0], next(s for s in diffuse.inputs if s.name == "Scale"))
    glow = nodes.new("ShaderNodeVectorMath")
    glow.operation = "SCALE"
    feed(glow.inputs[0], em_src, em_val)
    # Bevy affiche l'émissif sans l'exposition de la caméra (emissive_exposure_weight = 0) :
    # on la retire ici pour que l'aperçu montre la même luminance que le jeu.
    next(s for s in glow.inputs if s.name == "Scale").default_value = strength / (2.0 ** float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2")))
    add = nodes.new("ShaderNodeVectorMath")
    add.operation = "ADD"
    links.new(diffuse.outputs[0], add.inputs[0])
    links.new(glow.outputs[0], add.inputs[1])
    emit = nodes.new("ShaderNodeEmission")
    emit.inputs["Strength"].default_value = 1.0
    links.new(add.outputs[0], emit.inputs["Color"])
    links.new(emit.outputs["Emission"], out.inputs["Surface"])
    return g


def game_previews(samples: int, stats: dict, lightmaps: dict) -> None:
    """Aperçus « comme dans le jeu » (Moyen, cellules à lightmap) : émission seule, sans lampe."""
    import preview as pv

    pv.setup_cycles(samples, 1600, 900, exposure=float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2")))
    bpy.context.scene.cycles.max_bounces = 0
    pv.world((0, 0, 0), 0.0)
    for obj in list(bpy.context.scene.objects):
        if obj.type == "LIGHT":
            obj.hide_render = True
    for cid, s in stats.items():
        info = lightmaps[cid]
        image = bpy.data.images.load(str(c.ROOT / "assets" / info["image"]), check_existing=True)
        image.colorspace_settings.name = "sRGB"
        scale = info["exposure"] / lm.WATTS_TO_LUMENS
        for obj in s["merged"]:
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = True
            mats = obj.data.materials
            for i, mat in enumerate(mats):
                mats[i] = _game_material(mat, image, scale, cid)
    only = os.environ.get("SH_PREVIEW_ONLY")
    for name, (loc, target, lens) in SHOTS.items():
        if only and name not in only.split(","):
            continue
        pv.camera("cam", loc, target, lens=lens)
        pv.render(pv.PREVIEWS / f"salle_jeu_{name}.png")
        print(f"aperçu jeu : salle_jeu_{name}.png")


def main(argv):
    bake = "--no-bake" not in argv
    samples = int(argv[argv.index("--samples") + 1]) if "--samples" in argv else 64
    c.reset_scene()
    m = palette()
    cells = {"coursive": build_coursive(m), "atelier": build_atelier(m)}
    # Matrices à jour avant dépliage et fusion (les pavés orientés posent matrix_basis).
    bpy.context.view_layer.update()
    result = export(cells, bake)
    # Sans bake, le descripteur ne cite pas de lightmap (éclairage dynamique dans le jeu).
    write_descriptor(result["lightmaps"])
    c.save_blend(f"art/materials/{SECTOR}.blend")
    if "--previews" in argv:
        # Les aperçus montrent exactement ce qui est exporté : les maillages fusionnés.
        shown = {o for s in result["stats"].values() for o in s["merged"]}
        for obj in bpy.data.objects:
            if obj.type == "MESH":
                obj.hide_render = obj not in shown
        if "--game-only" not in argv:
            previews(samples, cells)
        if result["lightmaps"]:
            game_previews(max(8, samples // 2), result["stats"], result["lightmaps"])


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
