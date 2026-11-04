"""Abordeur pirate de la Ligue (M6, proposition P26 ; refait en « futur usé » au lot F de M8).

Identité pirate proposée (P6, à valider) : un assemblage. Combinaison matelassée kaki, plaques
dépareillées soudées et rivetées par-dessus (rouge sang, rouille, ocre à bandes effacées, olive,
apprêt), sangles et poches de toile, respirateur à cartouche latérale relié par un tuyau annelé
à des bouteilles dorsales, casque peint à visière orangée émissive, fusil à projectiles
magnétiques bricolé (rails, bobines de cuivre nues, cellule d'énergie vissée sur le flanc,
câbles, voyants). Asymétrique partout : rien n'est d'origine, tout a été récupéré. Aucun
insigne, aucun logo, aucun texte hormis un pochoir générique (« A2 »).

Pose fixe, sans squelette (le jeu tasse le modèle pour l'accroupi et le bascule pour la chute,
`crates/sh_game/src/combat/foot.rs`) : debout, fusil épaulé à deux mains, canon à 1,33 m (la
bouche du jeu est à 1,35 m), pied gauche en avant.

Repères : 1 unité = 1 m, Z-haut, avant vers +Y, droite du personnage vers +X ; origine au
centre de la base (entre les pieds). ≈ 1,8 m.

Matériaux (5) : `char_suit` (toile `char_cloth` du lot F, teinte kaki), `char_rubber` (`rubber`),
`char_metal` (`metal_dark`), `char_steel` (`metal_bare`), `char_gear` (atlas du lot F,
`char_atlas.py` : pièces peintes, rouille, cuivre, sangles, toile, émissifs).

Sorties : assets/models/characters/pirate.gltf (+ .bin, images citées dans
assets/textures/materials/), art/characters/pirate.blend. L'ancien pirate.glb n'est pas touché.

    blender --background --factory-startup --threads 4 --python tools/blender/characters/pirate.py -- [--previews]
"""

from __future__ import annotations

import contextlib
import io
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))

import char_kit as k  # noqa: E402
import common as c  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

# Teinte de la combinaison (facteur glTF, linéaire) : kaki terne, assez clair pour se lire
# sous la seule ambiante du jeu.
SUIT_TINT = (0.58, 0.51, 0.39)

# Torse : anneaux (z, demi-largeur, demi-profondeur, décalage avant).
TORSO = [
    (0.795, 0.095, 0.080, -0.012),
    (0.85, 0.142, 0.106, -0.008),
    (0.93, 0.163, 0.118, -0.010),
    (1.03, 0.150, 0.110, 0.000),
    (1.15, 0.163, 0.117, 0.008),
    (1.27, 0.183, 0.127, 0.015),
    (1.37, 0.193, 0.122, 0.010),
    (1.44, 0.180, 0.106, 0.000),
    (1.49, 0.120, 0.085, 0.000),
    (1.52, 0.065, 0.062, 0.000),
]

# Articulations (droite = +X).
SHOULDER_R, ELBOW_R, WRIST_R, HAND_R = (0.19, -0.01, 1.42), (0.25, 0.04, 1.17), (0.12, 0.205, 1.235), (0.10, 0.262, 1.248)
SHOULDER_L, ELBOW_L, WRIST_L, HAND_L = (-0.19, -0.01, 1.42), (-0.20, 0.19, 1.215), (0.005, 0.375, 1.245), (0.075, 0.425, 1.262)
HIP_R, KNEE_R, ANKLE_R = (0.10, 0.0, 0.93), (0.14, -0.02, 0.50), (0.16, -0.10, 0.10)
HIP_L, KNEE_L, ANKLE_L = (-0.10, 0.0, 0.93), (-0.13, 0.10, 0.51), (-0.14, 0.07, 0.10)

# Fusil : axe du canon.
GUN_X, GUN_Z = 0.10, 1.33


def materials() -> dict:
    return {
        "suit": k.cloth_material("char_suit", SUIT_TINT),
        "rubber": c.textured_material("char_rubber", "rubber"),
        "metal": c.textured_material("char_metal", "metal_dark"),
        "steel": c.textured_material("char_steel", "metal_bare"),
        "gear": k.gear_material(),
    }


def _lerp(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def _torso_at(z: float):
    """(rx, ry, oy) du torse interpolés à la hauteur z."""
    for (z0, rx0, ry0, o0), (z1, rx1, ry1, o1) in zip(TORSO[:-1], TORSO[1:]):
        if z0 <= z <= z1:
            t = (z - z0) / (z1 - z0)
            return rx0 + (rx1 - rx0) * t, ry0 + (ry1 - ry0) * t, o0 + (o1 - o0) * t
    z0, rx, ry, o = TORSO[0] if z < TORSO[0][0] else TORSO[-1]
    return rx, ry, o


# --- Corps ---------------------------------------------------------------------------------------


def body(b: k.Builder) -> None:
    m, coll = b.m, b.coll
    pts = [(0.0, 0.0, z) for z, *_ in TORSO]
    radii = [(rx, ry) for _, rx, ry, _ in TORSO]
    offs = [(0.0, o) for *_, o in TORSO]
    b.lib(k.tube("torso", pts, radii, m["suit"], coll, segments=18, offsets=offs), k.TILE_FABRIC)
    # Cou sous une cagoule de caoutchouc (joint casque / combinaison).
    b.lib(k.tube("neck", [(0, 0.0, 1.49), (0, 0.01, 1.62)], [(0.056, 0.054), (0.054, 0.054)],
                 m["rubber"], coll, segments=10), k.TILE_RUBBER)
    # Col de joint caoutchouc (étanchéité casque / combinaison).
    b.lib(k.tube("collar", [(0, 0.0, 1.50), (0, 0.005, 1.545)], [(0.085, 0.078), (0.062, 0.060)],
                 m["rubber"], coll, segments=12), k.TILE_RUBBER)

    # Jambes : cuisse, mollet ; botte à tige, semelle crantée, coque de pointe.
    for tag, hip, knee, ankle, yaw in (("r", HIP_R, KNEE_R, ANKLE_R, -12.0), ("l", HIP_L, KNEE_L, ANKLE_L, 6.0)):
        thigh = [hip, _lerp(hip, knee, 0.35), _lerp(hip, knee, 0.75), knee]
        b.lib(k.tube(f"thigh_{tag}", thigh, [0.094, 0.090, 0.075, 0.060], m["suit"], coll, segments=12,
                     ref=(0, 1, 0)), k.TILE_FABRIC)
        calf = [knee, _lerp(knee, ankle, 0.3), _lerp(knee, ankle, 0.7), ankle]
        b.lib(k.tube(f"calf_{tag}", calf, [(0.058, 0.058), (0.064, 0.068), (0.052, 0.054), (0.045, 0.045)],
                     m["suit"], coll, segments=12, ref=(1, 0, 0), offsets=[(0, 0), (0, -0.008), (0, -0.004), (0, 0)]),
              k.TILE_FABRIC)
        boot(b, tag, ankle, yaw)


def boot(b: k.Builder, tag: str, ankle, yaw_deg: float) -> None:
    m, coll = b.m, b.coll
    ax, ay, _ = ankle
    rot = Matrix.Rotation(math.radians(yaw_deg), 4, "Z")

    def place(obj, local):
        obj.matrix_basis = Matrix.Translation((ax, ay, 0.0)) @ rot @ Matrix.Translation(local)
        return obj

    shaft = k.tube(f"boot_shaft_{tag}", [(0, 0, 0.05), (0, 0.0, 0.17), (0, -0.005, 0.25)],
                   [(0.060, 0.066), (0.056, 0.060), (0.058, 0.060)], m["rubber"], coll, segments=12)
    b.lib(place(shaft, (0, 0, 0)), k.TILE_RUBBER)
    foot = k.rounded_box(f"boot_foot_{tag}", (0.105, 0.24, 0.085), (0, 0, 0), m["rubber"], coll, bevel=0.03, segments=3)
    b.lib(place(foot, (0, 0.06, 0.068)), k.TILE_RUBBER)
    sole = k.rounded_box(f"boot_sole_{tag}", (0.112, 0.252, 0.03), (0, 0, 0), m["metal"], coll, bevel=0.009)
    b.lib(place(sole, (0, 0.062, 0.016)), k.TILE_METAL)
    # Coque de pointe en ferraille, sur le dessus du pied.
    toe = k.rounded_box(f"boot_toe_{tag}", (0.09, 0.07, 0.012), (0, 0, 0), m["gear"], coll, bevel=0.004)
    toe.data.transform(Matrix.Rotation(math.radians(-18), 4, "X"))
    b.cell(place(toe, (0, 0.135, 0.105)), "scrap")
    # Sangle de cheville à boucle.
    strap = k.tube(f"boot_strap_{tag}", [(0, 0, 0.19), (0, 0, 0.21)], 0.064, m["gear"], coll, segments=12)
    b.cell(place(strap, (0, 0, 0)), "webbing")
    buckle = k.rounded_box(f"boot_buckle_{tag}", (0.025, 0.008, 0.026), (0, 0, 0), m["steel"], coll, bevel=0.003)
    b.lib(place(buckle, (0.045 if tag == "r" else -0.045, 0.035, 0.20)), k.TILE_METAL)


def arms(b: k.Builder) -> None:
    m, coll = b.m, b.coll
    for tag, sh, el, wr, hand in (("r", SHOULDER_R, ELBOW_R, WRIST_R, HAND_R), ("l", SHOULDER_L, ELBOW_L, WRIST_L, HAND_L)):
        side = (1, 0, 0) if tag == "r" else (-1, 0, 0)
        upper = [sh, _lerp(sh, el, 0.4), el]
        b.lib(k.tube(f"upper_{tag}", upper, [0.060, 0.056, 0.047], m["suit"], coll, segments=10, ref=side), k.TILE_FABRIC)
        fore = [el, _lerp(el, wr, 0.35), wr]
        b.lib(k.tube(f"fore_{tag}", fore, [0.047, 0.046, 0.036], m["suit"], coll, segments=10, ref=side), k.TILE_FABRIC)
        # Gant : manchette, paume, doigts refermés sur la poignée (un bloc), pouce.
        cuff = k.tube(f"cuff_{tag}", [_lerp(el, wr, 0.72), _lerp(el, wr, 1.04)], [0.043, 0.041], m["rubber"], coll, segments=10, ref=side)
        b.lib(cuff, k.TILE_RUBBER)
        d = (Vector(hand) - Vector(wr)).normalized()
        k.seg_box(b, f"palm_{tag}", (0.078, 0.042, 0.075), wr, tuple(Vector(wr) + d), (0, 0, 1),
                       "rubber", "lib", k.TILE_RUBBER)
        # Repère local du gant : x vers le haut, y = dos de la main pour la droite (paume pour la
        # gauche, qui tient la poignée avant par la gauche).
        sgn = 1.0 if tag == "r" else -1.0
        k.seg_box(b, f"fingers_{tag}", (0.074, 0.048, 0.045), tuple(Vector(wr) + d * 0.06),
                tuple(Vector(wr) + d * 0.2), (0, 0, 1), "rubber", "lib", k.TILE_RUBBER, offset=(0, -0.012 * sgn))
        thumb_dir = Vector((0, 0, 1)).cross(d).normalized() * (-1 if tag == "r" else 1)
        k.seg_cyl(b, f"thumb_{tag}", 0.012, tuple(Vector(wr) + d * 0.03 + Vector((0, 0, 0.02))),
                tuple(Vector(wr) + d * 0.08 + Vector((0, 0, 0.035)) + thumb_dir * 0.01), "rubber", "lib", k.TILE_RUBBER, segments=8)
        # Plaque de dos de main (pièce récupérée).
        k.seg_box(b, f"knuckle_{tag}", (0.066, 0.01, 0.06), tuple(Vector(wr) + d * 0.015),
                tuple(Vector(wr) + d * 0.2), (0, 0, 1), "gear", "cell", "scrap", offset=(0, 0.026 * sgn))


# --- Plaques et équipement ---------------------------------------------------------------------


def plates(b: k.Builder) -> None:
    m, coll = b.m, b.coll
    # Poitrine : grande plaque rouge sang, décentrée vers la gauche.
    b.cell(k.shell("chest_plate", (0.188, 0.133), 1.17, 1.43, 38, 152, 0.014, m["gear"], coll, segments=14, rings=4,
                   center=(0.0, 0.012), taper=0.02, bulge=0.008), "paint_red")
    # Ventre : plaque ocre à bandes effacées, décalée à droite, par-dessus.
    b.cell(k.shell("belly_plate", (0.166, 0.125), 1.035, 1.175, 22, 112, 0.012, m["gear"], coll, segments=10, rings=2,
                   center=(0.0, 0.006)), "paint_ochre")
    # Flanc gauche : apprêt gris au pochoir.
    b.cell(k.shell("side_plate", (0.178, 0.126), 1.08, 1.30, 150, 212, 0.012, m["gear"], coll, segments=8, rings=2,
                   center=(0.0, 0.008)), "primer")
    # Dos : olive, rivets.
    b.cell(k.shell("back_plate", (0.192, 0.132), 1.12, 1.43, 222, 322, 0.014, m["gear"], coll, segments=12, rings=3,
                   center=(0.0, 0.004), taper=-0.04), "paint_olive")
    # Gorgerin rouillé (demi-col avant).
    b.cell(k.shell("gorget", (0.115, 0.09), 1.44, 1.50, 25, 155, 0.012, m["gear"], coll, segments=10, rings=1,
                   taper=-0.25), "paint_rust")

    # Épaule gauche : spalière en deux lames rouge sang et rouille, le long du bras (axe de
    # l'épaule vers le coude) ; l'angle 0 est en haut, ±90° devant et derrière.
    sl, el = Vector(SHOULDER_L), Vector(ELBOW_L)
    axis_l = (Vector((-0.30, -0.01, 1.355)) - Vector((-0.115, -0.01, 1.445))).normalized()
    root_l = Vector((-0.115, -0.01, 1.445))
    k.seg_shell(b, "pauldron_l", tuple(root_l), tuple(root_l + axis_l), (0, 0, 1), (0.088, 0.098), 0.0, 0.135, -105, 105,
              0.014, "gear", "paint_blood", segments=12, rings=2, taper=-0.08, bulge=0.006)
    k.seg_shell(b, "pauldron_l2", tuple(root_l + Vector((0, 0, -0.012))), tuple(root_l + Vector((0, 0, -0.012)) + axis_l), (0, 0, 1),
              (0.084, 0.092), 0.105, 0.20, -100, 100, 0.012, "gear", "paint_rust", segments=10, rings=1, taper=-0.1)
    del sl, el
    # Épaule droite : une seule lame de ferraille, échancrée devant (la crosse s'y appuie).
    root_r = Vector((0.115, -0.01, 1.445))
    axis_r = (Vector((0.29, -0.01, 1.36)) - root_r).normalized()
    k.seg_shell(b, "pauldron_r", tuple(root_r), tuple(root_r + axis_r), (0, 0, 1), (0.08, 0.088), 0.02, 0.15, -100, 55,
              0.010, "gear", "scrap", segments=10, rings=1, taper=-0.08)

    # Avant-bras : protège-bras dépareillés.
    for tag, el, wr, cell, side in (("r", ELBOW_R, WRIST_R, "paint_rust", (1, 0, 0)), ("l", ELBOW_L, WRIST_L, "paint_ochre", (-1, 0, 0))):
        k.seg_shell(b, f"bracer_{tag}", el, wr, side, (0.051, 0.051), 0.05, 0.155, -70, 70, 0.010, "gear", cell,
                  segments=8, rings=2, taper=-0.12)
        k.seg_shell(b, f"elbow_{tag}", el, tuple(Vector(el) * 2 - Vector(wr)), side, (0.055, 0.055), -0.03, 0.03, -60, 60,
                  0.010, "gear", "scrap", segments=6, rings=1)

    # Jambes : cuissard droit (apprêt), genouillères, jambière gauche.
    fwd_out_r = (0.4, 1.0, 0.0)
    # Repère local Z vers le haut (genou → hanche, cheville → genou) : pochoir à l'endroit.
    lt = (Vector(HIP_R) - Vector(KNEE_R)).length
    k.seg_shell(b, "thigh_plate_r", KNEE_R, HIP_R, fwd_out_r, (0.072, 0.074), lt - 0.34, lt - 0.11, -55, 65, 0.010, "gear",
              "primer", segments=8, rings=2, taper=0.25)
    for tag, knee, ankle, cell, th in (("l", KNEE_L, ANKLE_L, "paint_blood", 0.014), ("r", KNEE_R, ANKLE_R, "scrap", 0.012)):
        up_pt = tuple(Vector(knee) * 2 - Vector(ankle))
        k.seg_shell(b, f"knee_{tag}", knee, up_pt, (0, 1, 0), (0.064, 0.066), -0.06, 0.055, -70, 70, th, "gear", cell,
                  segments=8, rings=2, bulge=0.008)
    ls = (Vector(KNEE_L) - Vector(ANKLE_L)).length
    k.seg_shell(b, "shin_l", ANKLE_L, KNEE_L, (0, 1, 0), (0.054, 0.058), ls - 0.31, ls - 0.075, -60, 60, 0.010, "gear",
              "paint_olive", segments=8, rings=2, taper=0.22)
    # Rivets de fixation visibles (têtes d'acier) sur la poitrine et l'épaule.
    for i, (x, z) in enumerate(((-0.11, 1.40), (0.05, 1.41), (-0.13, 1.20), (0.06, 1.19))):
        _, ry, oy = _torso_at(z)
        ys = oy + ry * math.sqrt(max(0.0, 1 - (x / 0.2) ** 2)) + 0.022
        rv = k.cyl(f"rivet_{i}", 0.008, 0.008, m["steel"], coll, segments=8)
        k.orient(rv, (x, ys - 0.004, z), (x, ys + 0.004, z))
        b.lib(rv, k.TILE_METAL)


def belt_and_straps(b: k.Builder) -> None:
    m, coll = b.m, b.coll
    # Ceinturon : sangle plate fermée, légèrement inclinée.
    n = 24
    pts, refs = [], []
    for i in range(n):
        a = 2 * math.pi * i / n
        z = 0.975 + 0.012 * math.sin(a)
        rx, ry, oy = _torso_at(z)
        pts.append((math.cos(a) * (rx + 0.012), oy + math.sin(a) * (ry + 0.012), z))
        refs.append((0, 0, 1))
    b.cell(k.tube("belt", pts, (0.028, 0.008), m["gear"], coll, segments=4, refs=refs, closed=True, smooth=False), "webbing")
    b.lib(k.rounded_box("buckle", (0.06, 0.014, 0.045), (0.0, 0.142, 0.975), m["steel"], coll, bevel=0.005), k.TILE_METAL)
    # Poches de toile sur le ceinturon (angle autour du torse, largeur, hauteur).
    for i, (deg, w, h) in enumerate(((115, 0.075, 0.085), (145, 0.07, 0.09), (195, 0.085, 0.10), (300, 0.09, 0.08), (40, 0.06, 0.075))):
        a = math.radians(deg)
        rx, ry, oy = _torso_at(0.97)
        p = Vector((math.cos(a) * (rx + 0.035), oy + math.sin(a) * (ry + 0.035), 0.955))
        pouch = k.rounded_box(f"pouch_{i}", (w, 0.045, h), (0, 0, 0), m["gear"], coll, bevel=0.01)
        pouch.matrix_basis = Matrix.Translation(p) @ Matrix.Rotation(a - math.pi / 2, 4, "Z")
        b.cell(pouch, "canvas")

    # Bandoulière : boucle inclinée, de l'épaule gauche à la hanche droite, par-dessus les plaques.
    n = 28
    pts, refs = [], []
    for i in range(n):
        a = 2 * math.pi * i / n
        z = 1.205 + 0.235 * math.cos(a - math.pi)
        rx, ry, oy = _torso_at(min(z, 1.47))
        on_plate = 38 < math.degrees(a) % 360 < 152 and 1.17 < z < 1.43
        off = 0.03 if on_plate else 0.016
        shrink = 1.0 if z < 1.42 else 0.9
        pts.append((math.cos(a) * (rx * shrink + off), oy + math.sin(a) * (ry * shrink + off), z))
        refs.append((math.cos(a), math.sin(a), 0))
    strap = k.tube("bandolier", pts, (0.007, 0.027), m["gear"], coll, segments=4, refs=refs, closed=True, smooth=False)
    b.cell(strap, "webbing")
    # Étuis de munitions (barres de projectiles) sur la bandoulière, devant.
    for i, t in enumerate((0.20, 0.235, 0.27)):
        idx = t * n
        i0 = int(idx)
        p0, p1 = Vector(pts[i0]), Vector(pts[(i0 + 1) % n])
        p = p0.lerp(p1, idx - i0)
        tang = (p1 - p0).normalized()
        a = 2 * math.pi * t
        nrm = Vector((math.cos(a), math.sin(a), 0))
        case = k.rounded_box(f"ammo_{i}", (0.045, 0.028, 0.06), (0, 0, 0), m["gear"], coll, bevel=0.006)
        k.orient(case, tuple(p + nrm * 0.016), tuple(p + nrm * 0.016 + tang), side=tuple(nrm.cross(tang)))
        b.cell(case, "paint_olive" if i != 1 else "canvas")
    # Étui de pistolet sur la cuisse droite, crosse dépassante.
    hip, knee = Vector(HIP_R), Vector(KNEE_R)
    d = (knee - hip).normalized()
    p = hip + d * 0.16 + Vector((0.085, 0.0, 0.0))
    holster = k.rounded_box("holster", (0.03, 0.08, 0.16), (0, 0, 0), m["gear"], coll, bevel=0.01)
    k.orient(holster, tuple(p + d * 0.08), tuple(p - d * 0.08), side=(0, 1, 0))
    b.cell(holster, "canvas")
    b.lib(k.rounded_box("holster_grip", (0.022, 0.035, 0.07), tuple(p - d * 0.11 + Vector((0, -0.01, 0))),
                        m["metal"], coll, bevel=0.006), k.TILE_METAL)
    for z in (0.80, 0.66):
        t = (z - KNEE_R[2]) / (HIP_R[2] - KNEE_R[2])
        cx, cy, _ = _lerp(KNEE_R, HIP_R, t)
        rx = 0.058 + (0.088 - 0.058) * t + 0.006
        ring = [(cx + math.cos(a) * rx, cy + math.sin(a) * rx, z) for a in [2 * math.pi * i / 12 for i in range(12)]]
        b.cell(k.tube(f"thigh_strap_{z}", ring, (0.016, 0.005), m["gear"], coll, segments=4, refs=[(0, 0, 1)] * 12,
                      closed=True, smooth=False), "webbing")


def backpack(b: k.Builder) -> None:
    m, coll = b.m, b.coll
    b.lib(k.rounded_box("pack_frame", (0.25, 0.035, 0.30), (0.0, -0.152, 1.26), m["metal"], coll, bevel=0.01), k.TILE_METAL)
    for tag, x, y, z, r, h, cell in (("a", 0.065, -0.205, 1.25, 0.055, 0.30, "rusted"), ("b", -0.07, -0.198, 1.28, 0.045, 0.22, "paint_olive")):
        tank = k.cyl(f"tank_{tag}", r, h, m["gear"], coll, segments=14)
        tank.location = (x, y, z)
        b.cyl(tank, cell)
        for dz in (-1, 1):
            cap = k.cyl(f"tank_cap_{tag}{dz}", r * 0.85, 0.025, m["steel"], coll, segments=12, r2=r * 0.6)
            cap.location = (x, y, z + dz * (h / 2 + 0.01))
            if dz < 0:
                cap.rotation_euler = (math.pi, 0, 0)
            b.lib(cap, k.TILE_METAL)
        b.lib(k.tube(f"tank_band_{tag}", [(x, y, z + 0.06), (x, y, z + 0.08)], r + 0.004, m["rubber"], coll, segments=14), k.TILE_RUBBER)
    valve = k.cyl("valve", 0.012, 0.05, m["steel"], coll, segments=8)
    valve.location = (0.065, -0.205, 1.43)
    b.lib(valve, k.TILE_METAL)
    # Petit boîtier de voyants sur la sangle de poitrine (côté gauche).
    b.cell(k.rounded_box("chest_box", (0.05, 0.022, 0.035), (-0.12, 0.135, 1.33), m["gear"], coll, bevel=0.005), "leds")
    b.lib(k.rounded_box("chest_box_case", (0.058, 0.016, 0.043), (-0.12, 0.125, 1.33), m["metal"], coll, bevel=0.004), k.TILE_METAL)


HELMET_C = (0.0, 0.012, 1.665)   # centre de la base du dôme
HELMET_R = (0.118, 0.130)        # demi-largeur, demi-profondeur
HELMET_H = 0.13                  # hauteur du dôme au-dessus du centre


def _dome_scale(z: float) -> float:
    """Rayon relatif du dôme du casque à la hauteur z (1 sous le centre : partie cylindrique)."""
    t = (z - HELMET_C[2]) / HELMET_H
    if t <= 0:
        return 1.0
    return math.cos(math.asin(min(t, 0.999)))


def _on_helmet(b: k.Builder, name, z0, z1, a0, a1, thickness, gap, mat_key, uv, arg, segments=12):
    """Bande collée au casque entre z0 et z1 (suit la courbure du dôme), décollée de `gap`."""
    s0, s1 = _dome_scale(z0), _dome_scale(z1)
    radii = (HELMET_R[0] * s0 + gap, HELMET_R[1] * s0 + gap)
    taper = (HELMET_R[0] * s1 + gap) / radii[0] - 1.0
    obj = k.shell(name, radii, z0, z1, a0, a1, thickness, b.m[mat_key], b.coll, segments=segments,
                  rings=2 if z1 - z0 > 0.04 else 1, center=HELMET_C[:2], taper=taper)
    return b.add(obj, "cell" if uv == "cell" else "lib", arg)


def head(b: k.Builder) -> None:
    m, coll = b.m, b.coll
    b.cell(k.dome("helmet", HELMET_C, HELMET_R, HELMET_H, m["gear"], coll, segments=20, rings=6, z_bottom=-0.095),
           "paint_blood")
    # Rustine rouillée rivetée sur le côté droit, couvre-nuque évasé.
    _on_helmet(b, "helmet_patch", 1.70, 1.765, -55, 30, 0.007, 0.002, "gear", "cell", "paint_rust", segments=8)
    b.cell(k.shell("neck_guard", (0.128, 0.14), 1.565, 1.625, 205, 335, 0.012, m["gear"], coll, segments=10, rings=1,
                   center=HELMET_C[:2], taper=-0.08), "paint_blood")
    # Visière : bandeau courbe émissif entre une arcade et une joue sombres.
    _on_helmet(b, "visor", 1.652, 1.700, 46, 134, 0.008, 0.006, "gear", "cell", "visor")
    _on_helmet(b, "brow", 1.700, 1.716, 34, 146, 0.020, 0.0, "metal", "lib", k.TILE_METAL)
    _on_helmet(b, "cheek", 1.632, 1.652, 36, 144, 0.018, 0.0, "metal", "lib", k.TILE_METAL)
    # Respirateur : museau métallique, grille, cartouche filtrante à gauche, tuyau annelé vers le dos.
    b.lib(k.shell("muzzle", (0.08, 0.096), 1.565, 1.632, 50, 130, 0.026, m["metal"], coll, segments=10, rings=2,
                  center=(0.0, 0.038), taper=0.14), k.TILE_METAL)
    b.cell(k.rounded_box("grille", (0.05, 0.016, 0.032), (0.0, 0.158, 1.588), m["gear"], coll, bevel=0.005), "scrap")
    can_a, can_b = Vector((-0.058, 0.14, 1.585)), Vector((-0.118, 0.17, 1.54))
    k.seg_cyl(b, "filter", 0.026, tuple(can_a), tuple(can_b), "gear", "cyl", "paint_ochre", segments=12, side=(0, 0, 1), bevel=0.004)
    k.seg_cyl(b, "filter_cap", 0.028, tuple(can_b), tuple(can_b + (can_b - can_a).normalized() * 0.012), "steel", "lib", k.TILE_METAL,
            segments=12)
    hose = k.spline([tuple(can_b + Vector((-0.004, -0.012, -0.012))), (-0.16, 0.11, 1.495), (-0.18, 0.02, 1.48),
                     (-0.15, -0.10, 1.47), (-0.10, -0.165, 1.445), (-0.07, -0.20, 1.40)], 4)
    rr = [0.0155 if i % 2 else 0.0125 for i in range(len(hose))]
    b.lib(k.tube("hose", hose, rr, m["rubber"], coll, segments=8), k.TILE_RUBBER)
    # Écouteur à droite (voyant), antenne.
    k.seg_cyl(b, "ear", 0.034, (0.110, 0.0, 1.655), (0.138, 0.0, 1.655), "metal", "lib", k.TILE_METAL, segments=12, bevel=0.004)
    k.seg_cyl(b, "ear_light", 0.013, (0.138, 0.0, 1.655), (0.144, 0.0, 1.655), "gear", "cell", "leds", segments=8)
    k.seg_cyl(b, "antenna", 0.0035, (0.125, -0.03, 1.68), (0.16, -0.08, 1.88), "steel", "lib", k.TILE_METAL, segments=6)


# --- Fusil ---------------------------------------------------------------------------------------


def rifle(b: k.Builder) -> None:
    m, coll = b.m, b.coll
    x, z = GUN_X, GUN_Z
    Y = (0, 1, 0)
    b.lib(k.rounded_box("receiver", (0.056, 0.30, 0.085), (x, 0.30, z + 0.005), m["metal"], coll, bevel=0.008), k.TILE_METAL)
    b.lib(k.rounded_box("top_rail", (0.028, 0.24, 0.012), (x, 0.31, z + 0.053), m["steel"], coll, bevel=0.003), k.TILE_METAL)
    for i in range(6):
        b.lib(k.rounded_box(f"rail_tooth_{i}", (0.032, 0.012, 0.006), (x, 0.21 + i * 0.04, z + 0.061), m["steel"], coll, bevel=0.002),
              k.TILE_METAL)
    # Plaque soudée sur le flanc gauche, voyants.
    b.cell(k.rounded_box("receiver_patch", (0.006, 0.13, 0.06), (x - 0.030, 0.24, z + 0.002), m["gear"], coll, bevel=0.002), "scrap")
    b.cell(k.rounded_box("receiver_leds", (0.006, 0.05, 0.022), (x - 0.031, 0.36, z + 0.02), m["gear"], coll, bevel=0.002), "leds")
    # Crosse squelette en tubes et plaque de couche.
    k.seg_cyl(b, "stock_top", 0.009, (x, 0.16, z + 0.03), (x, 0.085, z + 0.035), "steel", "lib", k.TILE_METAL, segments=8)
    k.seg_cyl(b, "stock_low", 0.009, (x, 0.16, z - 0.03), (x, 0.085, z - 0.045), "steel", "lib", k.TILE_METAL, segments=8)
    b.lib(k.rounded_box("butt", (0.045, 0.028, 0.12), (x, 0.08, z - 0.005), m["rubber"], coll, bevel=0.008), k.TILE_RUBBER)
    # Canon, rails magnétiques, bobines de cuivre nues et entretoises.
    k.seg_cyl(b, "barrel", 0.0125, (x, 0.45, z), (x, 0.84, z), "steel", "lib", k.TILE_METAL, segments=12)
    for dz in (1, -1):
        b.lib(k.rounded_box(f"mag_rail_{dz}", (0.012, 0.37, 0.016), (x, 0.635, z + dz * 0.026), m["metal"], coll, bevel=0.003),
              k.TILE_METAL)
    for i, y in enumerate((0.51, 0.585, 0.66, 0.735)):
        k.seg_cyl(b, f"coil_{i}", 0.031, (x, y - 0.02, z), (x, y + 0.02, z), "gear", "cyl", "copper", segments=14, side=(0, 0, 1))
        k.seg_cyl(b, f"spacer_{i}", 0.036, (x, y + 0.02, z), (x, y + 0.028, z), "metal", "lib", k.TILE_METAL, segments=12)
    k.seg_cyl(b, "muzzle", 0.022, (x, 0.80, z), (x, 0.865, z), "metal", "lib", k.TILE_METAL, segments=12, bevel=0.004)
    # Cellule d'énergie vissée sur le flanc droit, câbles vers la première bobine.
    k.seg_cyl(b, "cell", 0.026, (x + 0.052, 0.235, z - 0.005), (x + 0.052, 0.38, z - 0.005), "gear", "cyl", "paint_ochre", segments=12,
            side=(0, 0, 1), bevel=0.003)
    for i, y in enumerate((0.225, 0.39)):
        k.seg_cyl(b, f"cell_cap_{i}", 0.022, (x + 0.052, y - 0.01, z - 0.005), (x + 0.052, y + 0.01, z - 0.005), "steel", "lib",
                k.TILE_METAL, segments=10)
    b.cell(k.rounded_box("cell_window", (0.006, 0.07, 0.016), (x + 0.078, 0.31, z - 0.005), m["gear"], coll, bevel=0.002), "leds")
    for i, dz in enumerate((0.012, -0.014)):
        cable = k.spline([(x + 0.052, 0.39, z - 0.005 + dz), (x + 0.06, 0.44, z + dz), (x + 0.045, 0.48, z + dz * 1.5),
                          (x + 0.03, 0.49, z + dz)], 3)
        b.lib(k.tube(f"cable_{i}", cable, 0.0055, m["rubber"], coll, segments=6), k.TILE_RUBBER)
    # Chargeur (pièce olive dépareillée), poignée pistolet, garde, poignée avant, viseur.
    mag = k.rounded_box("magazine", (0.03, 0.05, 0.10), (0, 0, 0), m["gear"], coll, bevel=0.006)
    mag.matrix_basis = Matrix.Translation((x, 0.37, z - 0.09)) @ Matrix.Rotation(math.radians(12), 4, "X")
    b.cell(mag, "paint_olive")
    k.seg_box(b, "pistol_grip", (0.032, 0.045, 0.105), (x, 0.262, z - 0.03), (x, 0.235, z - 0.14), Y, "gear", "cell", "grip")
    b.lib(k.rounded_box("guard", (0.012, 0.075, 0.008), (x, 0.30, z - 0.085), m["metal"], coll, bevel=0.003), k.TILE_METAL)
    k.seg_cyl(b, "trigger", 0.004, (x, 0.30, z - 0.04), (x, 0.295, z - 0.07), "steel", "lib", k.TILE_METAL, segments=6)
    k.seg_cyl(b, "fore_grip", 0.018, (x, 0.44, z - 0.035), (x, 0.43, z - 0.13), "gear", "cyl", "grip", segments=10)
    b.lib(k.rounded_box("sight", (0.03, 0.065, 0.034), (x, 0.28, z + 0.077), m["metal"], coll, bevel=0.006), k.TILE_METAL)
    k.seg_cyl(b, "sight_lens", 0.012, (x, 0.315, z + 0.08), (x, 0.32, z + 0.08), "gear", "cell", "leds", segments=10)
    b.lib(k.rounded_box("front_post", (0.006, 0.01, 0.03), (x, 0.82, z + 0.03), m["metal"], coll, bevel=0.002), k.TILE_METAL)


# --- Assemblage ----------------------------------------------------------------------------------


def build():
    c.reset_scene()
    m = materials()
    coll = c.collection("ASSET_pirate")
    b = k.Builder(m, coll)
    body(b)
    arms(b)
    plates(b)
    belt_and_straps(b)
    backpack(b)
    head(b)
    rifle(b)
    parts = b.finalize()
    exports = c.collection("EXPORT")
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        merged = c.join_by_material(parts, "pirate", exports)
    # Nœuds à l'identité : l'origine de chaque maillage est celle du modèle (pieds / poignée).
    for obj in merged:
        k.apply_transform(obj)
    coll.hide_render = True
    coll.hide_viewport = True
    return merged


def previews(merged) -> None:
    import char_preview as pv

    center = (0.0, 0.1, 0.95)
    pv.setup("studio", center)
    pv.render("pirate_face", pv.orbit((0, 0.0, 0.95), 3.4, 0, 0.15), (0, 0, 0.92), lens=50)
    pv.render("pirate_3q", pv.orbit((0, 0.0, 0.95), 3.4, 38, 0.35), (0, 0, 0.92), lens=50)
    pv.render("pirate_3q_left", pv.orbit((0, 0.0, 0.95), 3.4, -40, 0.25), (0, 0, 0.92), lens=50)
    pv.render("pirate_back", pv.orbit((0, 0.0, 0.95), 3.4, 160, 0.3), (0, 0, 0.92), lens=50)
    pv.render("pirate_bust", pv.orbit((0.02, 0.1, 1.42), 1.15, 28, 0.08), (0.02, 0.1, 1.42), lens=50, res=(1200, 1200))
    pv.render("pirate_rifle", pv.orbit((0.1, 0.5, 1.33), 0.9, 75, 0.15), (0.1, 0.48, 1.31), lens=50, res=(1400, 900))
    pv.setup("ambient", center)
    pv.render("pirate_ambient", pv.orbit((0, 0.0, 0.95), 3.4, 25, 0.3), (0, 0, 0.92), lens=50)
    pv.setup("dim", center)
    pv.render("pirate_dim", pv.orbit((0, 0.0, 1.2), 5.0, 15, 0.45), (0, 0, 1.0), lens=45, res=(1200, 900))


def main(argv: list[str]) -> None:
    merged = build()
    tris = k.triangles(merged)
    c.export_gltf("assets/models/characters/pirate.gltf", merged)
    print(f"pirate : {len(merged)} maillages ({', '.join(o.name for o in merged)}), {tris} triangles")
    c.save_blend("art/characters/pirate.blend")
    if "--previews" in argv:
        previews(merged)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
