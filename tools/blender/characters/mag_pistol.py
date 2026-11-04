"""Pistolet magnétique du joueur (lot F de M8) : modèle autonome pour une future vue à la
première personne (aujourd'hui, le jeu n'affiche pas l'arme du joueur).

Canon : les armes de l'univers tirent des projectiles (magnétiques), sans bouclier (export §2.4).
Ce modèle est générique, sans marque ni inscription : carcasse anthracite usée, canon nu entre
deux rails, trois bobines de cuivre séparées par des entretoises, cellule d'énergie ocre glissée
sous le canon (fenêtre cyan), jauge de charge segmentée à l'arrière, tournée vers le tireur,
poignée caoutchouc à plaquettes quadrillées, câbles apparents (réparation), vis.

Repères : 1 unité = 1 m, Z-haut, canon vers +Y ; **origine à la poignée** (centre de la prise
de la main), l'axe du canon passe 7,6 cm au-dessus. Longueur ≈ 0,29 m.

Matériaux (4) : `char_metal` (`metal_dark`), `char_steel` (`metal_bare`), `char_rubber`
(`rubber`), `char_gear` (atlas du lot F : cuivre, peinture, quadrillage, émissifs).

Sorties : assets/models/characters/weapons/mag_pistol.gltf (+ .bin, images citées dans
assets/textures/materials/), art/characters/mag_pistol.blend.

    blender --background --factory-startup --threads 4 --python tools/blender/characters/mag_pistol.py -- [--previews]
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

BORE_Z = 0.076   # hauteur de l'axe du canon au-dessus de l'origine (la prise)
GRIP_TILT = 14.0  # inclinaison de la poignée vers l'arrière (degrés)
TILE = 0.25      # densité des métaux et du caoutchouc : vus de très près


def materials() -> dict:
    return {
        "metal": c.textured_material("char_metal", "metal_dark"),
        "steel": c.textured_material("char_steel", "metal_bare"),
        "rubber": c.textured_material("char_rubber", "rubber"),
        "gear": k.gear_material(),
    }


def _tilted(obj, pivot=(0.0, 0.0, 0.0)):
    """Incline une pièce de la poignée autour de l'axe X passant par `pivot`."""
    p = Vector(pivot)
    rot = Matrix.Translation(p) @ Matrix.Rotation(math.radians(-GRIP_TILT), 4, "X") @ Matrix.Translation(-p)
    obj.matrix_basis = rot @ obj.matrix_basis
    return obj


def frame(b: k.Builder) -> None:
    m, coll = b.m, b.coll
    z = BORE_Z
    # Carcasse : corps, culasse surélevée à l'arrière, nez sous le canon.
    b.lib(k.rounded_box("frame", (0.032, 0.150, 0.040), (0.0, 0.020, z - 0.018), m["metal"], coll, bevel=0.006, segments=3), TILE)
    b.lib(k.rounded_box("breech", (0.036, 0.085, 0.034), (0.0, -0.012, z + 0.012), m["metal"], coll, bevel=0.007, segments=3), TILE)
    b.lib(k.rounded_box("breech_top", (0.022, 0.070, 0.006), (0.0, -0.010, z + 0.031), m["steel"], coll, bevel=0.002), TILE)
    # Fentes d'aération sur la culasse (barrettes en relief).
    for i in range(4):
        for sx in (-1, 1):
            b.lib(k.rounded_box(f"vent_{i}_{sx}", (0.003, 0.006, 0.020), (sx * 0.0185, -0.035 + i * 0.012, z + 0.012),
                                m["steel"], coll, bevel=0.001), TILE)
    # Hausse (deux oreilles) et guidon.
    for sx in (-1, 1):
        b.lib(k.rounded_box(f"rear_sight_{sx}", (0.006, 0.012, 0.012), (sx * 0.008, -0.048, z + 0.037), m["metal"], coll,
                            bevel=0.002), TILE)
    # Jauge de charge à l'arrière, inclinée vers l'œil du tireur, dans un cadre.
    gauge_frame = k.rounded_box("gauge_frame", (0.030, 0.010, 0.020), (0, 0, 0), m["metal"], coll, bevel=0.003)
    gauge = k.rounded_box("gauge", (0.024, 0.006, 0.013), (0, 0, 0), m["gear"], coll, bevel=0.0015)
    tilt = Matrix.Rotation(math.radians(35), 4, "X")
    gauge_frame.matrix_basis = Matrix.Translation((0.0, -0.056, z + 0.012)) @ tilt
    gauge.matrix_basis = Matrix.Translation((0.0, -0.0595, z + 0.0135)) @ tilt
    b.lib(gauge_frame, TILE)
    b.cell(gauge, "charge")
    # Vis sur les flancs.
    for i, (y, zz) in enumerate(((-0.040, z - 0.005), (0.010, z - 0.028), (0.075, z - 0.028), (0.020, z + 0.020))):
        for sx in (-1, 1):
            k.seg_cyl(b, f"screw_{i}_{sx}", 0.0028, (sx * 0.016, y, zz), (sx * 0.0185, y, zz), "steel", "lib", TILE, segments=8)


def barrel(b: k.Builder) -> None:
    z = BORE_Z
    k.seg_cyl(b, "bore", 0.0075, (0.0, 0.030, z), (0.0, 0.232, z), "steel", "lib", TILE, segments=14)
    # Rails magnétiques de part et d'autre du canon, tenus par le bloc avant de la carcasse.
    for sx in (-1, 1):
        b.lib(k.rounded_box(f"rail_{sx}", (0.005, 0.145, 0.016), (sx * 0.0135, 0.163, z), b.m["metal"], b.coll, bevel=0.0018), TILE)
    b.lib(k.rounded_box("barrel_block", (0.036, 0.022, 0.034), (0.0, 0.098, z), b.m["metal"], b.coll, bevel=0.005, segments=2), TILE)
    for i, y in enumerate((0.124, 0.156, 0.188)):
        k.seg_cyl(b, f"coil_{i}", 0.0185, (0.0, y - 0.0115, z), (0.0, y + 0.0115, z), "gear", "cyl", "copper", segments=18,
                  side=(0, 0, 1))
        k.seg_cyl(b, f"spacer_{i}", 0.0215, (0.0, y + 0.0115, z), (0.0, y + 0.0155, z), "metal", "lib", TILE, segments=16,
                  bevel=0.0008)
    k.seg_cyl(b, "spacer_back", 0.0215, (0.0, 0.1085, z), (0.0, 0.1125, z), "metal", "lib", TILE, segments=16, bevel=0.0008)
    # Bouche : bague épaisse, frein à fentes, guidon.
    k.seg_cyl(b, "muzzle", 0.0175, (0.0, 0.2075, z), (0.0, 0.234, z), "metal", "lib", TILE, segments=16, bevel=0.0025)
    k.seg_cyl(b, "muzzle_tip", 0.011, (0.0, 0.234, z), (0.0, 0.240, z), "steel", "lib", TILE, segments=14, bevel=0.001)
    b.lib(k.rounded_box("front_sight", (0.004, 0.010, 0.010), (0.0, 0.226, z + 0.021), b.m["metal"], b.coll, bevel=0.0015), TILE)
    # Pont supérieur : relie la carcasse à la bouche par-dessus les bobines (rigidité, guidon).
    b.lib(k.rounded_box("top_strap", (0.010, 0.120, 0.005), (0.0, 0.158, z + 0.0195), b.m["metal"], b.coll, bevel=0.0018), TILE)


def cell_and_cables(b: k.Builder) -> None:
    z = BORE_Z
    cz = z - 0.043
    # Cellule d'énergie ocre, glissée sous le canon, culots d'acier, fenêtre cyan à gauche.
    k.seg_cyl(b, "cell", 0.0115, (0.0, 0.088, cz), (0.0, 0.178, cz), "gear", "cyl", "paint_ochre", segments=14, side=(0, 0, 1),
              bevel=0.0012)
    k.seg_cyl(b, "cell_cap", 0.0125, (0.0, 0.178, cz), (0.0, 0.188, cz), "steel", "lib", TILE, segments=14, bevel=0.0012)
    b.cell(k.rounded_box("cell_window", (0.003, 0.050, 0.008), (-0.0118, 0.135, cz), b.m["gear"], b.coll, bevel=0.001), "glow_cyan")
    b.cell(k.rounded_box("cell_window_r", (0.003, 0.050, 0.008), (0.0118, 0.135, cz), b.m["gear"], b.coll, bevel=0.001), "glow_cyan")
    # Épine sous les bobines : porte la cellule (de la carcasse à la bouche).
    b.lib(k.rounded_box("spine", (0.011, 0.118, 0.016), (0.0, 0.150, z - 0.026), b.m["metal"], b.coll, bevel=0.003), TILE)
    # Berceau de la cellule (deux colliers).
    for y in (0.105, 0.160):
        b.lib(k.tube(f"cell_clamp_{y}", [(0.0, y - 0.003, cz), (0.0, y + 0.003, cz)], 0.0128, b.m["metal"], b.coll, segments=14), TILE)
    # Câbles apparents : de la cellule à la première bobine, l'un réparé au ruban.
    for i, sx in enumerate((-1, 1)):
        pts = k.spline([(sx * 0.008, 0.183, cz + 0.006), (sx * 0.016, 0.192, cz + 0.020), (sx * 0.020, 0.170, z - 0.018),
                        (sx * 0.0175, 0.140, z - 0.014)], 4)
        b.lib(k.tube(f"cable_{i}", pts, 0.0019, b.m["rubber"], b.coll, segments=6), TILE)
    b.lib(k.tube("tape", [(-0.0185, 0.172, z - 0.019), (-0.0195, 0.164, z - 0.017)], 0.0028, b.m["rubber"], b.coll, segments=8), TILE)


def grip(b: k.Builder) -> None:
    m, coll = b.m, b.coll
    pivot = (0.0, 0.0, BORE_Z - 0.035)
    # Poignée caoutchouc, plaquettes quadrillées sur les flancs, rainures de doigts, talon.
    body = k.rounded_box("grip", (0.029, 0.040, 0.105), (0.0, -0.004, -0.004), m["rubber"], coll, bevel=0.009, segments=3)
    b.lib(_tilted(body, pivot), TILE)
    for sx in (-1, 1):
        plate = k.rounded_box(f"grip_plate_{sx}", (0.003, 0.030, 0.080), (sx * 0.0153, -0.006, -0.002), m["gear"], coll, bevel=0.0012)
        b.cell(_tilted(plate, pivot), "grip")
        screw = k.cyl(f"grip_screw_{sx}", 0.0025, 0.002, m["steel"], coll, segments=8)
        k.orient(screw, (sx * 0.0168, -0.006, 0.010), (sx * 0.0185, -0.006, 0.010))
        b.lib(_tilted(screw, pivot), TILE)
    for i in range(3):
        # Arêtes de doigts sur l'avant de la poignée (le caoutchouc creusé entre elles).
        bump = k.rounded_box(f"finger_{i}", (0.026, 0.006, 0.0065), (0.0, 0.0155, 0.009 - i * 0.024), m["rubber"], coll,
                             bevel=0.0025)
        b.lib(_tilted(bump, pivot), TILE)
    heel = k.rounded_box("heel", (0.032, 0.046, 0.010), (0.0, -0.004, -0.058), m["metal"], coll, bevel=0.003)
    b.lib(_tilted(heel, pivot), TILE)
    # Arrière de la poignée : appui de paume (bec) sous la culasse.
    tang = k.rounded_box("tang", (0.026, 0.020, 0.012), (0.0, -0.030, BORE_Z - 0.030), m["metal"], coll, bevel=0.004)
    b.lib(tang, TILE)
    # Pontet et détente.
    z = BORE_Z
    guard = k.spline([(0.0, 0.006, z - 0.040), (0.0, 0.050, z - 0.044), (0.0, 0.062, z - 0.052), (0.0, 0.055, z - 0.072),
                      (0.0, 0.034, z - 0.078), (0.0, 0.014, z - 0.070)], 3)
    b.lib(k.tube("guard", guard, (0.0028, 0.0045), m["metal"], coll, segments=8, refs=[(0, 0, 1)] * len(guard)), TILE)
    trig = k.spline([(0.0, 0.034, z - 0.040), (0.0, 0.037, z - 0.052), (0.0, 0.033, z - 0.064)], 3)
    b.lib(k.tube("trigger", trig, (0.0035, 0.0022), m["steel"], coll, segments=6, refs=[(1, 0, 0)] * len(trig)), TILE)


def build():
    c.reset_scene()
    m = materials()
    coll = c.collection("ASSET_mag_pistol")
    b = k.Builder(m, coll)
    frame(b)
    barrel(b)
    cell_and_cables(b)
    grip(b)
    parts = b.finalize()
    exports = c.collection("EXPORT")
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        merged = c.join_by_material(parts, "mag_pistol", exports)
    # Nœuds à l'identité : l'origine de chaque maillage est celle du modèle (pieds / poignée).
    for obj in merged:
        k.apply_transform(obj)
    coll.hide_render = True
    coll.hide_viewport = True
    return merged


def previews() -> None:
    import char_preview as pv

    center = (0.0, 0.08, 0.05)
    pv.setup("studio", center, scale=0.25, floor=False)
    pv.render("pistol_side", (0.55, 0.09, 0.06), (0.0, 0.09, 0.04), lens=60, res=(1400, 900))
    pv.render("pistol_side_left", (-0.55, 0.09, 0.06), (0.0, 0.09, 0.04), lens=60, res=(1400, 900))
    # Vue de l'épaule du tireur : derrière, au-dessus, un peu à droite (bras tendu).
    pv.render("pistol_shooter", (-0.13, -0.30, 0.17), (0.0, 0.12, 0.05), lens=35, res=(1600, 900))
    pv.render("pistol_3q", (0.30, 0.38, 0.18), (0.0, 0.09, 0.04), lens=60, res=(1400, 900))
    pv.setup("ambient", center, scale=0.25, floor=False)
    pv.render("pistol_ambient", (-0.13, -0.30, 0.17), (0.0, 0.12, 0.05), lens=35, res=(1600, 900))


def main(argv: list[str]) -> None:
    merged = build()
    tris = k.triangles(merged)
    c.export_gltf("assets/models/characters/weapons/mag_pistol.gltf", merged)
    print(f"mag_pistol : {len(merged)} maillages ({', '.join(o.name for o in merged)}), {tris} triangles")
    c.save_blend("art/characters/mag_pistol.blend")
    if "--previews" in argv:
        previews()


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
