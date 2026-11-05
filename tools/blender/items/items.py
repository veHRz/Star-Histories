"""Objets d'inventaire posés dans le monde (lot F de M9, docs/m9-retours.md §3.6).

Un modèle par objet du catalogue (`assets/data/gameplay/items.ron`), « futur usé » : pratique,
usé, lisible, sans logo ni texte lisible. Un `INTERACT_* pickup` (paramètre `item`) fait
apparaître le modèle à sa place ; ramassé, il disparaît (lot L3).

Repères : 1 unité = 1 m, Z-haut, avant vers +Y (le côté tourné vers le joueur : loquets, voyants,
bas de l'écran), **origine au centre de la base** (l'objet repose sur une table ou au sol à
z = 0). Échelle réelle. Pas de collider.

Matériaux : `item_gear` (atlas `item_atlas.py`, un seul pour tous les objets) ; plus `item_suit`
(toile `char_cloth`, teintée) pour la combinaison ; le pistolet garde les quatre matériaux de
`characters/mag_pistol.py` (même modèle, importé, posé à plat).

Sorties : assets/models/items/<id>.gltf (+ .bin, images citées dans assets/textures/materials/),
art/items/<id>.blend.

    blender --background --factory-startup --python tools/blender/items/items.py -- [--only datapad,medkit]
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "characters"))
sys.path.insert(0, str(HERE))

import bpy  # noqa: E402
import common as c  # noqa: E402
import item_kit as k  # noqa: E402
from mathutils import Matrix, Vector, noise  # noqa: E402

ITEMS = ["datapad", "spare_parts", "mag_pistol", "flight_suit", "toolkit", "medkit", "ration", "data_chip", "cargo_manifest"]
MODELS = "assets/models/items"
STATS = c.ROOT / "target" / "art-previews" / "m9-lotF" / "stats.json"

# Lecture par un joueur placé devant l'objet (+Y) : droite = −X, haut de la face = −Y.
READ_U, READ_V = (-1, 0, 0), (0, -1, 0)


def _rb(b: k.Builder, name, size, center, cell, bevel=0.003, segments=2):
    return b.box(k.rounded_box(name, size, center, b.m["gear"], b.coll, bevel=bevel, segments=segments), cell)


def _rb_planar(b: k.Builder, name, size, center, cell, u_axis=(1, 0, 0), v_axis=(0, 1, 0), bounds=None, bevel=0.001,
               segments=1):
    obj = k.rounded_box(name, size, center, b.m["gear"], b.coll, bevel=bevel, segments=segments)
    if bounds is not None:
        # Bornes données dans le repère du modèle : ramenées dans le repère local de la pièce.
        au, av = Vector(u_axis), Vector(v_axis)
        off_u, off_v = Vector(center).dot(au), Vector(center).dot(av)
        bounds = (bounds[0] - off_u, bounds[1] - off_u, bounds[2] - off_v, bounds[3] - off_v)
    return b.planar(obj, cell, u_axis, v_axis, bounds)


def _led(b: k.Builder, name, center, cell, radius=0.0025, height=0.002, axis="Z"):
    """Petit voyant (cylindre) : la sous-case du voyant couvre la pastille."""
    obj = k.cyl(name, radius, height, b.m["gear"], b.coll, segments=10)
    if axis == "Y":
        obj.rotation_euler = (math.radians(-90), 0.0, 0.0)
    obj.location = center
    u, v = ((1, 0, 0), (0, 1, 0))
    return b.planar(obj, cell, u, v)


def _tube(b: k.Builder, name, points, radii, cell, segments=8, closed=False, ref=(1.0, 0.0, 0.0), refs=None, caps=True):
    obj = k.tube(name, points, radii, b.m["gear"], b.coll, segments=segments, ref=ref, refs=refs, closed=closed,
                 cap_start=caps, cap_end=caps)
    return b.tube(obj, cell, segments)


# --- Bloc-données ---------------------------------------------------------------------------------


def datapad(b: k.Builder) -> None:
    """Tablette robuste : coque olive-gris, pare-chocs de caoutchouc aux coins, écran émissif doux
    dans un joint noir, trois touches (celle du milieu rétroéclairée), voyant vert, stylet clipsé
    sur le flanc droit, cache de port."""
    W, D = 0.20, 0.145
    for sx in (-1, 1):
        for sy in (-1, 1):
            _rb(b, f"bumper_{sx}_{sy}", (0.034, 0.034, 0.024), (sx * (W / 2 - 0.015), sy * (D / 2 - 0.015), 0.012),
                "rubber", bevel=0.007)
    _rb(b, "body", (W - 0.006, D - 0.006, 0.018), (0.0, 0.0, 0.011), "shell", bevel=0.004)
    sy0 = -0.010  # écran décalé vers l'arrière : touches devant
    _rb_planar(b, "screen", (0.150, 0.094, 0.002), (0.0, sy0, 0.0205), "screen", READ_U, READ_V, bevel=0.0006)
    for s in (-1, 1):
        _rb(b, f"bezel_y_{s}", (0.162, 0.006, 0.0035), (0.0, sy0 + s * 0.050, 0.0215), "rubber", bevel=0.0012, segments=1)
        _rb(b, f"bezel_x_{s}", (0.006, 0.094, 0.0035), (s * 0.078, sy0, 0.0215), "rubber", bevel=0.0012, segments=1)
    for i, x in enumerate((0.03, 0.0, -0.03)):
        cell = "key_lit" if x == 0.0 else "rubber"
        _rb(b, f"key_{i}", (0.016, 0.009, 0.003), (x, 0.0525, 0.0205), cell, bevel=0.0012, segments=1)
    _led(b, "led", (-0.068, 0.0525, 0.0200), "led_green", radius=0.0022)
    # Stylet clipsé sur le flanc droit (+X), entre les pare-chocs.
    k.seg_cyl(b, "stylus", 0.0042, (W / 2 + 0.001, -0.046, 0.012), (W / 2 + 0.001, 0.040, 0.012), "metal", segments=10)
    k.seg_cyl(b, "stylus_tip", 0.0042, (W / 2 + 0.001, 0.040, 0.012), (W / 2 + 0.001, 0.050, 0.012), "rubber", segments=10,
              r2=0.0012)
    for y in (-0.025, 0.020):
        _rb(b, f"clip_{y}", (0.007, 0.008, 0.012), (W / 2 - 0.001, y, 0.012), "rubber", bevel=0.002, segments=1)
    _rb(b, "port_flap", (0.032, 0.005, 0.009), (0.040, -D / 2 + 0.001, 0.011), "rubber", bevel=0.0018, segments=1)


# --- Pièces détachées -----------------------------------------------------------------------------


def spare_parts(b: k.Builder) -> None:
    """Caisse ouverte olive usée, coins renforcés, poignées en fer rond ; dedans, sur un fond de
    calage sombre : grande et petite roues dentées, bobine de cuivre, coude de tuyau à brides,
    câbles (l'un pend par-dessus le bord), carte électronique, écrous, boulons, tige."""
    m, coll = b.m, b.coll
    W, D, H, t = 0.36, 0.25, 0.12, 0.010
    _rb(b, "bottom", (W, D, 0.012), (0.0, 0.0, 0.006), "paint_olive", bevel=0.003, segments=1)
    for s in (-1, 1):
        _rb(b, f"wall_y_{s}", (W, t, H - 0.012), (0.0, s * (D / 2 - t / 2), 0.012 + (H - 0.012) / 2), "paint_olive",
            bevel=0.003, segments=1)
        _rb(b, f"wall_x_{s}", (t, D - 2 * t, H - 0.012), (s * (W / 2 - t / 2), 0.0, 0.012 + (H - 0.012) / 2),
            "paint_olive", bevel=0.003, segments=1)
    for sx in (-1, 1):
        for sy in (-1, 1):
            _rb(b, f"corner_{sx}_{sy}", (0.024, 0.024, H + 0.004), (sx * (W / 2 - 0.009), sy * (D / 2 - 0.009), (H + 0.004) / 2),
                "metal", bevel=0.004, segments=1)
        # Poignée en fer rond sur chaque petit côté.
        x0, x1 = sx * (W / 2), sx * (W / 2 + 0.016)
        pts = [(x0, -0.055, 0.080), (x1, -0.050, 0.080), (x1, 0.050, 0.080), (x0, 0.055, 0.080)]
        _tube(b, f"handle_{sx}", k.spline(pts, 3), 0.0048, "metal", segments=6, ref=(0.0, 0.0, 1.0))
    # Fond de calage sombre et bosselé : le reste du bric-à-brac, dans l'ombre.
    def heap(x, y, z):
        if z > 0:
            z = z + 0.010 * noise.noise(Vector((x * 18.0, y * 18.0, 0.5))) + 0.006 * (1 - (2 * x / W) ** 2)
        return (x, y, z)
    b.box(k.soft_box("fill", (W - 2 * t - 0.002, D - 2 * t - 0.002, 0.050), (0.0, 0.0, 0.012 + 0.030), m["gear"], coll,
                     cuts=2, radius=0.006, deform=heap), "rubber")
    top = 0.072
    # Grande roue dentée appuyée contre la paroi arrière, petite roue à plat.
    g1 = k.gear("gear_big", 0.064, 0.055, 12, 0.011, 0.018, m["gear"], coll)
    g1.rotation_euler = (math.radians(62), 0.0, math.radians(8))
    g1.location = (-0.090, -0.058, top + 0.052)
    b.box(g1, "metal")
    g2 = k.gear("gear_small", 0.032, 0.026, 9, 0.008, 0.008, m["gear"], coll)
    g2.rotation_euler = (math.radians(10), math.radians(-14), 0.0)
    g2.location = (0.035, 0.050, top + 0.010)
    b.box(g2, "metal")
    # Bobine de cuivre couchée, flasques d'acier.
    k.seg_cyl(b, "coil", 0.021, (0.075, -0.045, top + 0.022), (0.135, -0.040, top + 0.024), "copper", segments=14)
    k.seg_cyl(b, "coil_flange_a", 0.027, (0.069, -0.0455, top + 0.022), (0.075, -0.045, top + 0.022), "metal", segments=14)
    k.seg_cyl(b, "coil_flange_b", 0.027, (0.135, -0.040, top + 0.024), (0.141, -0.0395, top + 0.024), "metal", segments=14)
    # Coude de tuyau qui dépasse du bord, brides aux deux bouts.
    pipe = [(-0.020, 0.040, top + 0.004), (-0.020, 0.030, top + 0.060), (-0.020, 0.0, top + 0.080), (-0.020, -0.060, top + 0.085)]
    pts = k.spline(pipe, 3)
    _tube(b, "pipe", pts, 0.011, "metal", segments=10)
    for i, (p0, p1) in enumerate(((pts[0], pts[1]), (pts[-1], pts[-2]))):
        d = (Vector(p1) - Vector(p0)).normalized()
        k.seg_cyl(b, f"pipe_flange_{i}", 0.016, tuple(Vector(p0) - d * 0.001), tuple(Vector(p0) + d * 0.007), "metal", segments=10)
    # Câbles : un rouge lové dans la caisse, un jaune qui pend par-dessus le bord avant.
    loop = []
    for i in range(13):
        a = 2 * math.pi * i / 12 * 1.25
        loop.append((0.100 + 0.040 * math.cos(a), 0.055 + 0.028 * math.sin(a), top + 0.006 + 0.010 * i / 12))
    _tube(b, "cable_red", k.spline(loop, 2), 0.0042, "cable_red", segments=6, ref=(0.0, 0.0, 1.0))
    drape = [(0.010, 0.010, top + 0.008), (0.030, 0.070, top + 0.018), (0.045, D / 2 + 0.004, H + 0.006),
             (0.052, D / 2 + 0.010, H - 0.030), (0.058, D / 2 + 0.008, H - 0.070)]
    _tube(b, "cable_yellow", k.spline(drape, 3), 0.0040, "cable_yellow", segments=6)
    # Carte électronique appuyée contre la paroi droite.
    board = k.rounded_box("board", (0.060, 0.040, 0.003), (0, 0, 0), m["gear"], coll, bevel=0.0008, segments=1)
    board.rotation_euler = (math.radians(-20), math.radians(55), math.radians(90))
    board.location = (0.150, 0.045, top + 0.030)
    b.planar(board, "chip", (1, 0, 0), (0, 1, 0))
    # Écrous et boulons semés sur le fond, tige d'acier en travers.
    rng = [(-0.120, 0.060, 0.4), (-0.140, -0.020, 1.1), (0.010, -0.075, 2.0), (0.055, 0.005, 0.7), (-0.050, 0.080, 1.6)]
    for i, (x, y, a) in enumerate(rng):
        nut = k.hex_prism(f"nut_{i}", 0.0085, 0.006, m["gear"], coll, hole=0.0042)
        nut.rotation_euler = (0.15 * i, 0.1, a)
        nut.location = (x, y, top + 0.004 + 0.002 * (i % 2))
        b.box(nut, "metal")
    for i, (x, y, a) in enumerate(((-0.115, 0.010, 0.3), (0.020, -0.030, 2.4))):
        d = Vector((math.cos(a), math.sin(a), 0.12))
        p0 = Vector((x, y, top + 0.006))
        k.seg_cyl(b, f"bolt_{i}", 0.0042, tuple(p0), tuple(p0 + d * 0.045), "metal", segments=8)
        head = k.hex_prism(f"bolt_head_{i}", 0.0078, 0.005, m["gear"], coll)
        k.orient(head, tuple(p0 - d * 0.0025), tuple(p0 + d * 0.0025))
        b.box(head, "metal")
    k.seg_cyl(b, "rod", 0.0055, (-0.150, -0.085, top + 0.004), (0.060, 0.090, top + 0.075), "metal", segments=8)


# --- Pistolet magnétique --------------------------------------------------------------------------


def mag_pistol_item() -> list:
    """Le pistolet du joueur (characters/mag_pistol.py, construit tel quel), couché sur son flanc
    gauche, canon vers +Y : origine au centre de la base."""
    import mag_pistol as mp

    merged = mp.build()  # repart d'une scène vide, renvoie les maillages fusionnés à l'identité
    lay = Matrix.Rotation(math.radians(-90), 4, "Y")  # flanc droit (+X) vers le haut
    for obj in merged:
        obj.data.transform(lay)
    k.recenter(merged)
    return merged


# --- Combinaison de vol ---------------------------------------------------------------------------


def _cushion_deform(W, D, H, puff, wrinkle, seed, bulge=0.05):
    def fn(x, y, z):
        nx, ny = 2 * x / W, 2 * y / D
        f = max(0.0, 1 - nx * nx) * max(0.0, 1 - ny * ny)
        if z > 0:
            z = z + puff * H / 2 * f * (z / (H / 2))
            z += wrinkle * noise.noise(Vector((x * 22.0 + seed, y * 22.0, seed))) * min(1.0, z / (H / 2)) * (0.4 + f)
        side = 1.0 + bulge * max(0.0, 1 - (2 * z / H) ** 2)
        return (x * side, y * side, z)
    return fn


def _cushion_top(W, D, H, puff, x, y, zc):
    nx, ny = 2 * x / W, 2 * y / D
    f = max(0.0, 1 - nx * nx) * max(0.0, 1 - ny * ny)
    return zc + H / 2 * (1 + puff * f)


def flight_suit(b: k.Builder) -> None:
    """Combinaison de vol pliée : deux épaisseurs de toile ardoise, anneau de col métallique (joint
    de casque) et col de toile, glissière, deux poches de poitrine à rabat et bouton-pression,
    pattes d'épaule, bouts de manches repliées (poignets de sangle) qui dépassent entre les
    épaisseurs, sangle de rangement à boucle."""
    m, coll = b.m, b.coll
    suit = m["suit"]
    W1, D1, H1 = 0.34, 0.25, 0.050
    low = k.soft_box("fold_low", (W1, D1, H1), (0.0, 0.0, H1 / 2), suit, coll, cuts=3, radius=0.022,
                     deform=_cushion_deform(W1, D1, H1, 0.10, 0.0030, 3.1))
    b.lib(low, 0.5)
    W2, D2, H2 = 0.30, 0.205, 0.044
    y2 = -0.012
    z2 = H1 + H2 / 2 - 0.006
    up = k.soft_box("fold_up", (W2, D2, H2), (0.0, y2, z2), suit, coll, cuts=3, radius=0.020,
                    deform=_cushion_deform(W2, D2, H2, 0.14, 0.0032, 7.7))
    b.lib(up, 0.5)

    def top(x, y):
        return _cushion_top(W2, D2, H2, 0.14, x, y - y2, z2)

    # Anneau de col (joint de casque) : section aplatie, posé sur l'épaisseur du dessus.
    ring_c = (0.0, -0.060)
    ring = []
    for i in range(18):
        a = 2 * math.pi * i / 18
        x, y = ring_c[0] + 0.054 * math.cos(a), ring_c[1] + 0.048 * math.sin(a)
        ring.append((x, y, top(x, y) + 0.003))
    _tube(b, "collar", ring, (0.0095, 0.0065), "metal", segments=8, closed=True, ref=(0.0, 0.0, 1.0))
    neck = k.soft_box("collar_cloth", (0.088, 0.078, 0.016), (ring_c[0], ring_c[1], top(*ring_c)), suit, coll,
                      cuts=1, radius=0.007)
    b.lib(neck, 0.5)
    # Glissière : bande d'acier qui suit le dessus, de l'anneau au bord avant, tirette.
    zip_pts = [(0.0, y, top(0.0, y) + 0.0012) for y in (-0.010, 0.015, 0.040, 0.062, 0.080)]
    _tube(b, "zip", zip_pts, (0.0042, 0.0013), "metal", segments=4, ref=(1.0, 0.0, 0.0))
    _rb(b, "zip_pull", (0.010, 0.018, 0.003), (0.0, 0.004, top(0.0, 0.004) + 0.0028), "metal", bevel=0.0012, segments=1)
    # Poches de poitrine à rabat, bouton-pression.
    for s in (-1, 1):
        px, py = s * 0.078, 0.030
        zt = top(px, py)
        pocket = k.soft_box(f"pocket_{s}", (0.074, 0.056, 0.010), (px, py, zt - 0.001), suit, coll, cuts=1, radius=0.004)
        b.lib(pocket, 0.5)
        flap = k.soft_box(f"pocket_flap_{s}", (0.078, 0.022, 0.006), (px, py - 0.020, top(px, py - 0.020) + 0.0045), suit, coll,
                          cuts=1, radius=0.0028)
        b.lib(flap, 0.5)
        k.seg_cyl(b, f"snap_{s}", 0.0045, (px, py - 0.014, top(px, py - 0.014) + 0.006),
                  (px, py - 0.014, top(px, py - 0.014) + 0.0085), "metal", segments=10)
        # Patte d'épaule (sangle) et son bouton.
        ex, ey = s * 0.100, -0.072
        tab = k.rounded_box(f"tab_{s}", (0.056, 0.016, 0.003), (ex, ey, top(ex, ey) + 0.0008), m["gear"], coll, bevel=0.0012,
                            segments=1)
        b.add(tab, "planar", "webbing", (0, 1, 0), (1, 0, 0), None)
        k.seg_cyl(b, f"tab_button_{s}", 0.0035, (ex - s * 0.020, ey, top(ex, ey) + 0.002),
                  (ex - s * 0.020, ey, top(ex, ey) + 0.0042), "metal", segments=8)
        # Bout de manche replié qui dépasse entre les épaisseurs : poignet de toile resserré par
        # une patte de sangle.
        sx = s * 0.085
        sleeve = k.tube(f"sleeve_{s}", [(sx, 0.070, H1 - 0.004), (sx, 0.100, H1 - 0.002), (sx, D1 / 2 + 0.010, H1 - 0.004),
                                        (sx, D1 / 2 + 0.016, H1 - 0.005)],
                        [(0.011, 0.030), (0.011, 0.030), (0.010, 0.028), (0.007, 0.022)], suit, coll, segments=10,
                        ref=(0.0, 0.0, 1.0))
        b.lib(sleeve, 0.5)
        _tube(b, f"cuff_{s}", [(sx, D1 / 2 + 0.002, H1 - 0.004), (sx, D1 / 2 + 0.009, H1 - 0.004)], [(0.0112, 0.0305)] * 2,
              "webbing", segments=10, ref=(0.0, 0.0, 1.0))
    # Sangle de rangement autour du paquet, boucle sur le dessus.
    sx = -0.118
    zt = top(sx, y2)
    loop = [(sx, 0.0, -0.0006), (sx, D1 / 2 + 0.006, 0.004), (sx, D1 / 2 + 0.011, H1 * 0.55), (sx, y2 + D2 / 2 + 0.004, H1 + 0.012),
            (sx, 0.040, zt + 0.004), (sx, -0.060, zt + 0.004), (sx, y2 - D2 / 2 - 0.004, H1 + 0.010),
            (sx, -D1 / 2 - 0.011, H1 * 0.55), (sx, -D1 / 2 - 0.006, 0.004)]
    _tube(b, "strap", loop, (0.013, 0.0018), "webbing", segments=4, closed=True, ref=(1.0, 0.0, 0.0))
    _rb(b, "buckle", (0.032, 0.036, 0.006), (sx, -0.010, zt + 0.006), "metal", bevel=0.0018, segments=1)


# --- Trousse à outils -----------------------------------------------------------------------------


def toolkit(b: k.Builder) -> None:
    """Mallette à outils ocre industrielle : corps et couvercle rivetés, joint de caoutchouc, bande
    de danger, deux grenouillères, coins d'acier, poignée caoutchoutée ; une clé plate tenue par
    deux sangles sur la face avant, voyant vert."""
    m, coll = b.m, b.coll
    W, D = 0.40, 0.20
    _rb(b, "body", (W, D, 0.115), (0.0, 0.0, 0.0575), "case_ochre", bevel=0.010, segments=2)
    _rb(b, "lid", (W - 0.004, D - 0.004, 0.040), (0.0, 0.0, 0.137), "case_ochre", bevel=0.010, segments=2)
    _rb(b, "gasket", (W - 0.012, D - 0.012, 0.006), (0.0, 0.0, 0.117), "rubber", bevel=0.002, segments=1)
    front = (D - 0.004) / 2
    _rb_planar(b, "hazard_band", (W - 0.090, 0.002, 0.016), (0.0, front + 0.0006, 0.137), "hazard", (-1, 0, 0), (0, 0, 1),
               bevel=0.0006)
    for sx in (-1, 1):
        x = sx * 0.155
        _rb(b, f"latch_{sx}", (0.034, 0.012, 0.044), (x, D / 2 + 0.004, 0.112), "metal", bevel=0.004, segments=1)
        _rb(b, f"latch_hook_{sx}", (0.026, 0.009, 0.012), (x, front + 0.003, 0.141), "metal", bevel=0.002, segments=1)
        for sy in (-1, 1):
            _rb(b, f"foot_{sx}_{sy}", (0.032, 0.032, 0.036), (sx * (W / 2 - 0.012), sy * (D / 2 - 0.012), 0.017), "metal",
                bevel=0.006, segments=1)
            _rb(b, f"cap_{sx}_{sy}", (0.030, 0.030, 0.030), (sx * (W / 2 - 0.013), sy * (front - 0.011), 0.142), "metal",
                bevel=0.006, segments=1)
        _rb(b, f"handle_post_{sx}", (0.020, 0.030, 0.026), (sx * 0.075, 0.0, 0.168), "metal", bevel=0.004, segments=1)
    grip = [(-0.078, 0.0, 0.170), (-0.066, 0.0, 0.190), (0.066, 0.0, 0.190), (0.078, 0.0, 0.170)]
    _tube(b, "handle", k.spline(grip, 3), 0.011, "rubber", segments=10, ref=(0.0, 1.0, 0.0))
    _led(b, "led", (0.120, front + 0.0012, 0.137), "led_green", radius=0.0028, height=0.003, axis="Y")
    # Clé plate sur la face avant, tenue par deux sangles.
    y = D / 2 + 0.004
    z = 0.052
    bar = k.rounded_box("wrench_bar", (0.150, 0.005, 0.016), (0.0, y, z), m["gear"], coll, bevel=0.002, segments=1)
    b.box(bar, "metal")
    ring = k.hex_prism("wrench_ring", 0.019, 0.006, m["gear"], coll, hole=0.0105)
    k.orient(ring, (0.085, y - 0.003, z), (0.085, y + 0.003, z), side=(1, 0, 0))
    b.box(ring, "metal")
    for s in (-1, 1):
        jaw = k.rounded_box(f"wrench_jaw_{s}", (0.028, 0.006, 0.010), (0, 0, 0), m["gear"], coll, bevel=0.002, segments=1)
        jaw.rotation_euler = (0.0, math.radians(s * 32), 0.0)
        jaw.location = (-0.093, y, z + s * 0.011)
        b.box(jaw, "metal")
    _rb(b, "wrench_head", (0.020, 0.006, 0.026), (-0.080, y, z), "metal", bevel=0.003, segments=1)
    for x in (-0.040, 0.040):
        _rb(b, f"strap_{x}", (0.014, 0.004, 0.032), (x, y + 0.004, z), "webbing", bevel=0.0012, segments=1)


# --- Trousse de soins -----------------------------------------------------------------------------


def medkit(b: k.Builder) -> None:
    """Trousse de soins rigide blanc cassé : coins de caoutchouc, plaque de premiers soins
    générique sur le couvercle (croix blanche sur vert), bandes vertes, loquet central, voyant
    vert, poignée de sangle sur le côté droit."""
    W, D = 0.30, 0.20
    _rb(b, "body", (W, D, 0.074), (0.0, 0.0, 0.037), "case_white", bevel=0.016, segments=3)
    _rb(b, "lid", (W - 0.004, D - 0.004, 0.032), (0.0, 0.0, 0.092), "case_white", bevel=0.014, segments=3)
    _rb(b, "gasket", (W - 0.014, D - 0.014, 0.006), (0.0, 0.0, 0.0755), "rubber", bevel=0.002, segments=1)
    _rb_planar(b, "emblem", (0.100, 0.100, 0.003), (0.0, 0.0, 0.1085), "med_cross", READ_U, READ_V, bevel=0.0012,
               segments=2)
    for s in (-1, 1):
        _rb_planar(b, f"band_y_{s}", (W - 0.060, 0.002, 0.014), (0.0, s * (D / 2 + 0.0002), 0.045), "med_green", (1, 0, 0),
                   (0, 0, 1), bevel=0.0006)
        _rb_planar(b, f"band_x_{s}", (0.002, D - 0.060, 0.014), (s * (W / 2 + 0.0002), 0.0, 0.045), "med_green", (0, 1, 0),
                   (0, 0, 1), bevel=0.0006)
    for sx in (-1, 1):
        for sy in (-1, 1):
            _rb(b, f"bumper_{sx}_{sy}", (0.032, 0.032, 0.070), (sx * (W / 2 - 0.010), sy * (D / 2 - 0.010), 0.036), "rubber",
                bevel=0.012, segments=2)
    _rb(b, "latch", (0.044, 0.012, 0.032), (0.0, D / 2 + 0.003, 0.078), "metal", bevel=0.004, segments=1)
    _led(b, "led", (0.036, D / 2 + 0.0012, 0.061), "led_green", radius=0.0028, height=0.003, axis="Y")
    x0, x1 = W / 2 - 0.002, W / 2 + 0.014
    strap = [(x0, -0.055, 0.058), (x1, -0.048, 0.064), (x1 + 0.002, 0.0, 0.066), (x1, 0.048, 0.064), (x0, 0.055, 0.058)]
    _tube(b, "handle", k.spline(strap, 3), (0.0025, 0.013), "webbing", segments=6, ref=(1.0, 0.0, 0.0))


# --- Ration ---------------------------------------------------------------------------------------


def ration(b: k.Builder) -> None:
    """Ration de survie en sachet souple : film olive imprimé (bandeau clair à pictogrammes
    abstraits, repère orange, ligne d'ouverture), soudures striées et dentelées aux deux bouts."""
    m, coll = b.m, b.coll
    L, Wd, T = 0.130, 0.085, 0.030
    zmid = 0.011
    half = L / 2

    def deform(x, y, z):
        taper = 0.10 + 0.90 * max(0.0, 1.0 - (abs(x) / half) ** 4) ** 0.5
        z = z * taper * (0.72 if z < 0 else 1.0)
        y = y * (1.0 - 0.04 * (1 - taper))
        return (x, y, z)

    body = k.soft_box("pack", (L, Wd, T), (0.0, 0.0, zmid), m["gear"], coll, cuts=3, radius=0.012, deform=deform)
    bounds = (-0.080, 0.080, -Wd / 2, Wd / 2)
    b.planar(body, "foil", (1, 0, 0), (0, 1, 0), (bounds[0], bounds[1], bounds[2], bounds[3]))
    for s in (-1, 1):
        f = k.flap(f"seal_{s}", s * (half - 0.003), s * 0.080, Wd / 2 - 0.001, zmid, 0.0012, 16, m["gear"], coll)
        b.planar(f, "foil", (1, 0, 0), (0, 1, 0), bounds)


# --- Puce de données ------------------------------------------------------------------------------


def data_chip(b: k.Builder) -> None:
    """Puce de données : galette polymère sombre, fenêtre de verre à pistes cyan lumineuses,
    languette de connexion à six contacts dorés vers l'avant (+Y), œillet de cordon à l'arrière."""
    Wc, Lc, T = 0.032, 0.055, 0.0045
    bounds = (-Lc / 2, Lc / 2, -Wc / 2, Wc / 2)
    u, v = (0, 1, 0), (-1, 0, 0)
    body_len = 0.046
    _rb_planar(b, "body", (Wc, body_len, T), (0.0, -Lc / 2 + body_len / 2, T / 2), "chip", u, v, bounds, bevel=0.0015,
               segments=2)
    _rb_planar(b, "tongue", (0.026, Lc - body_len + 0.002, 0.0022), (0.0, Lc / 2 - (Lc - body_len) / 2 - 0.001, 0.0016),
               "chip", u, v, bounds, bevel=0.0005)
    # Fenêtre de verre légèrement bombée (même projection : les pistes tombent dessous).
    _rb_planar(b, "window", (0.019, 0.029, 0.0008), (0.0, -0.0015, T + 0.0002), "chip", u, v, bounds, bevel=0.0003)


# --- Manifeste de fret ----------------------------------------------------------------------------


def cargo_manifest(b: k.Builder) -> None:
    """Manifeste de fret : ardoise mince anodisée (étiquette à pseudo-lignes, code-barres, sceau
    embouti, voyant ambre), cerclée d'un feuillard rouge et d'un plomb d'acier à voyant rouge."""
    W, D, T = 0.240, 0.170, 0.007
    _rb_planar(b, "slate", (W, D, T), (0.0, 0.0, T / 2), "manifest", READ_U, READ_V, bevel=0.002, segments=2)
    x = -0.023
    loop = [(x, 0.0, -0.0004), (x, D / 2 + 0.0006, 0.0006), (x, D / 2 + 0.0008, T / 2), (x, D / 2 + 0.0006, T - 0.0006),
            (x, 0.0, T + 0.0004), (x, -D / 2 - 0.0006, T - 0.0006), (x, -D / 2 - 0.0008, T / 2), (x, -D / 2 - 0.0006, 0.0006)]
    _tube(b, "seal_band", loop, (0.0060, 0.0006), "cable_red", segments=4, closed=True, ref=(1.0, 0.0, 0.0))
    _rb(b, "seal_clip", (0.018, 0.024, 0.006), (x, 0.040, T + 0.003), "metal", bevel=0.002, segments=1)
    _led(b, "seal_led", (x, 0.046, T + 0.0062), "led_red", radius=0.0024, height=0.0012)


BUILDERS = {
    "datapad": datapad,
    "spare_parts": spare_parts,
    "flight_suit": flight_suit,
    "toolkit": toolkit,
    "medkit": medkit,
    "ration": ration,
    "data_chip": data_chip,
    "cargo_manifest": cargo_manifest,
}


def build(item: str) -> list:
    if item == "mag_pistol":
        c.reset_scene()
        return mag_pistol_item()
    c.reset_scene()
    m = k.materials()
    if item == "flight_suit":
        m["suit"] = k.suit_material()
    coll = c.collection(f"ASSET_{item}")
    b = k.Builder(m, coll)
    BUILDERS[item](b)
    return k.finish(b.finalize(), item)


def main(argv: list[str]) -> None:
    only = None
    if "--only" in argv:
        only = argv[argv.index("--only") + 1].split(",")
    stats = json.loads(STATS.read_text(encoding="utf-8")) if STATS.exists() else {}
    for item in ITEMS:
        if only and item not in only:
            continue
        merged = build(item)
        lo, hi = k.bounds(merged)
        tris = k.triangles(merged)
        mats = sorted({mt.name for o in merged for mt in o.data.materials if mt})
        c.export_gltf(f"{MODELS}/{item}.gltf", merged)
        c.save_blend(f"art/items/{item}.blend")
        size = [round(float(v), 3) for v in hi - lo]
        stats[item] = {"tris": tris, "materials": mats, "size_m": size}
        print(f"objet {item} : {tris} triangles, matériaux {mats}, taille {size} m")
    STATS.parent.mkdir(parents=True, exist_ok=True)
    STATS.write_text(json.dumps(stats, indent=2, ensure_ascii=False), encoding="utf-8")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
