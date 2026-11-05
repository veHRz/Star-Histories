"""PNJ de comptoir (lot E de M9) : trois employés anonymes, immobiles, debout derrière un comptoir.

- `clerk_a` : guichetière (bureau des contrats, banque) : chemise gris bleu, gilet de travail
  ardoise à poches et fermeture, casque-micro d'une oreille, tablette accrochée à la hanche ; les
  deux mains posées sur le plateau.
- `clerk_b` : marchand : combinaison de travail bleu délavé aux manches retroussées, lourd tablier
  de cuir, gants de cuir, sacoche de toile en bandoulière, bottes ; main droite sur le plateau,
  main gauche sur la sacoche.
- `clerk_c` : tenancier : chemise crème aux manches retroussées, pantalon brun, long tablier
  bordeaux noué à la taille, torchon rayé sur l'épaule ; appuyé des deux mains, écartées, sur le
  bar, tête un peu penchée.

Génériques (règle d'or n° 2) : aucun insigne, aucun logo, aucune inscription. Pose fixe sans
squelette. Mains posées : leur point le plus bas à `COUNTER` (1,05 m), à partir de 0,21 m devant
l'origine ; sous le plateau, le corps (ventre, tabliers) avance jusqu'à 0,19 m : le plateau doit
dépasser vers l'employé jusqu'à 0,15–0,20 m de l'origine, le caisson rester au-delà de 0,19 m.

Repères : 1 unité = 1 m, Z-haut, regard vers +Y (−Z dans le jeu), droite du personnage vers +X,
origine au centre de la base (entre les pieds). Pas de collision (le comptoir a la sienne).

Matériaux (4 par personnage, un maillage fusionné par matériau) : `char_clerk` (atlas du lot E,
`clerk_atlas.py` : visages, cheveux, peau, cuir, torchon, tablier, écran), toile `char_clerk_cloth`
teintée (chemise, gilet, combinaison, pantalon), `char_gear` (atlas du pirate : poches, sangles,
boîtiers, voyants), `char_rubber` (bottes du marchand).

Sorties : assets/models/characters/clerk_{a,b,c}.gltf (+ .bin, images citées dans
assets/textures/materials/), art/characters/clerks.blend (les trois, si tous sont construits).

    blender --background --factory-startup --threads 4 --python tools/blender/characters/clerks.py -- [--atlas] [--previews] [--views eye,lineup] [--only a,b] [--stats]

`--atlas` régénère l'atlas `char_clerk` et la toile `char_clerk_cloth` (sinon : seulement s'ils
manquent) ; `--previews` rend les aperçus dans target/art-previews/m9-lotE/ (`--views` : liste
parmi face, 3q, eye, bust, hands, ambient, lineup) ; `--stats` : triangles par pièce.
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

import bmesh  # noqa: E402
import bpy  # noqa: E402
import char_kit as k  # noqa: E402
import clerk_atlas as ca  # noqa: E402
import clerk_face as cf  # noqa: E402
import clerk_kit as ck  # noqa: E402
import common as c  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

COUNTER = 1.05          # plateau du comptoir (m)
HAND_Y = 0.25           # poignets des mains posées, devant l'origine

TILE = 0.4  # toile `char_clerk_cloth` : tuile de 0,4 m


# --- Matériaux -------------------------------------------------------------------------------


def clerk_material():
    maps = {role: ca.OUT_DIR / f"{ca.NAME}_{role}.png" for role in ("color", "normal", "orm", "emissive")}
    if not maps["color"].exists():
        ca.generate()
    return c.textured_material("char_clerk", maps, emissive=(1.0, 1.0, 1.0), emissive_strength=1.6)


def cloth_material(name: str, tint):
    """Toile de vêtement `char_clerk_cloth` (clerk_atlas.py), teintée par le facteur glTF."""
    maps = {role: ca.OUT_DIR / f"{ca.CLOTH}_{role}.png" for role in ("color", "normal", "orm")}
    if not maps["color"].exists():
        ca.generate_cloth()
    mat = c.textured_material(name, maps, tint=tint)
    mat["sh_tile_m"] = ca.CLOTH_TILE
    return mat


def materials(key: str, cloth: dict) -> dict:
    m = {"skin": clerk_material(), "gear": k.gear_material()}
    for slot, tint in cloth.items():
        m[slot] = cloth_material(f"clerk_{key}_{slot}", tint)
    return m


# --- Pièces communes ---------------------------------------------------------------------------


def blob(name, center, radii, mat, coll, seg: int = 12, rings: int = 8):
    """Ellipsoïde (épaule, chignon…)."""
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=seg, v_segments=rings, radius=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * radii[0], v.co.y * radii[1], v.co.z * radii[2]))
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    obj.location = center
    coll.objects.link(obj)
    k._auto_smooth(obj)
    return obj


class Figure:
    """Proportions d'un personnage, déduites de la taille (gabarit : 1,78 m)."""

    def __init__(self, key: str, height: float, rings, shoulder_x: float, hip_x: float, knee_x: float,
                 ankle_x: float, head: cf.Head, head_scale: float = 1.0):
        self.key = key
        self.h = height
        self.s = s = height / 1.78
        self.head = head
        _, _, top_z, _ = cf.surface(head, [0.0], [cf.E_HI - 1e-6])
        self.head_scale = head_scale
        self.eye_z = height - float(top_z[0]) * head_scale
        # Le gabarit place le haut du torse à 0,135 m sous les yeux ; on cale les anneaux dessus.
        zt = rings[-1][0]
        dz = (self.eye_z - 0.135) - zt * s
        self.torso = ck.Torso([(z * s + dz * (z / zt), rx, ry, oy) for z, rx, ry, oy in rings])
        self.top_z = self.eye_z - 0.135
        self.shoulder_z = self.eye_z - 0.222
        self.belt_z = 1.00 * s
        self.sh = (shoulder_x, -0.012, self.shoulder_z)
        self.hip = (hip_x, -0.005, 0.905 * s)
        self.knee = (knee_x, 0.012, 0.49 * s)
        self.ankle = (ankle_x, -0.012, 0.085 * s)
        self.l1, self.l2 = 0.293 * s, 0.258 * s


def ck_smooth(t: float) -> float:
    t = min(max(t, 0.0), 1.0)
    return t * t * (3.0 - 2.0 * t)


def head_parts(b: ck.ClerkBuilder, f: Figure, yaw=0.0, pitch=0.0, roll=0.0, extra=()):
    """Tête, oreilles et accessoires de tête (repère local de la tête), posés ensemble."""
    rect = ca.face_uv(f.key)
    parts = [ck.head_mesh(f.head, f"head_{f.key}", b.m["skin"], b.coll, rect),
             ck.ear(f.head, f"ear_r_{f.key}", b.m["skin"], b.coll, rect, 1.0),
             ck.ear(f.head, f"ear_l_{f.key}", b.m["skin"], b.coll, rect, -1.0)]
    for p in parts:
        b.keep(p)
    for p in list(parts) + list(extra):
        ck.place_head(p, (0.0, 0.012, f.eye_z), yaw, pitch, roll, scale=f.head_scale)


def legs(b: ck.ClerkBuilder, f: Figure, mat_key: str, hem: float = 0.053, flare: float = 1.0, end_z=None):
    """Jambes de pantalon ; `end_z` : bas de jambe rentré dans les bottes."""
    m, coll = b.m, b.coll
    for side in (1, -1):
        hip = (f.hip[0] * side, f.hip[1], f.hip[2])
        knee = (f.knee[0] * side, f.knee[1], f.knee[2])
        ankle = (f.ankle[0] * side, f.ankle[1], end_z if end_z is not None else f.ankle[2] + 0.02)
        pts = [hip, ck.lerp3(hip, knee, 0.4), knee, ck.lerp3(knee, ankle, 0.5), ankle]
        radii = [0.090 * flare, 0.080 * flare, 0.064 * flare, 0.058 * flare, hem]
        b.lib(k.tube(f"leg_{side}", pts, radii, m[mat_key], coll, segments=14, ref=(0, 1, 0)), TILE)


def arm(b: ck.ClerkBuilder, f: Figure, side: int, wrist, pole, sleeve_key: str, rolled: bool = False,
        cuff_key: str | None = None, glove: bool = False):
    """Bras (manche, avant-bras nu si retroussé). Renvoie (coude, poignet)."""
    m, coll = b.m, b.coll
    sh = (f.sh[0] * side, f.sh[1], f.sh[2])
    el, wr = ck.two_bone(sh, wrist, f.l1, f.l2, pole)
    sref = (side, 0, 0)
    # Épaule : petit deltoïde sous la ligne du torse ; le haut du bras remonte dans le torse.
    b.lib(blob(f"deltoid_{side}", (sh[0] + 0.010 * side, sh[1] - 0.002, sh[2] - 0.026), (0.050, 0.052, 0.046),
               m[sleeve_key], coll), TILE)
    top = (sh[0] - 0.012 * side, sh[1], sh[2] + 0.02)
    upper = [top, sh, ck.lerp3(sh, el, 0.5), el]
    b.lib(k.tube(f"upper_{side}", upper, [0.036, 0.050, 0.049, 0.044], m[sleeve_key], coll, segments=12, ref=sref),
          TILE)
    if rolled:
        roll_at = ck.lerp3(el, wr, 0.28)
        b.lib(k.tube(f"sleeve_{side}", [ck.lerp3(el, sh, 0.06), el, roll_at], [0.044, 0.044, 0.043],
                     m[sleeve_key], coll, segments=12, ref=sref), TILE)
        b.lib(k.tube(f"roll_{side}", [ck.lerp3(el, wr, 0.18), roll_at, ck.lerp3(el, wr, 0.33)],
                     [0.046, 0.050, 0.046], m[sleeve_key], coll, segments=12, ref=sref), TILE)
        fore = [ck.lerp3(el, wr, 0.2), ck.lerp3(el, wr, 0.55), wr]
        b.rect(k.tube(f"forearm_{side}", fore, [(0.038, 0.035), (0.033, 0.030), (0.027, 0.022)], m["skin"], coll,
                      segments=10, ref=sref), ca.rect_uv(f"skin_{f.key}"))
        if glove:
            b.rect(k.tube(f"glove_cuff_{side}", [ck.lerp3(el, wr, 0.80), ck.lerp3(el, wr, 0.97), wr],
                          [0.037, 0.035, 0.031], m["skin"], coll, segments=10, ref=sref), ca.rect_uv("leather"))
    else:
        fore = [el, ck.lerp3(el, wr, 0.5), ck.lerp3(el, wr, 0.92)]
        b.lib(k.tube(f"fore_{side}", fore, [0.044, 0.040, 0.036], m[sleeve_key], coll, segments=12, ref=sref), TILE)
        b.lib(k.tube(f"cuff_{side}", [ck.lerp3(el, wr, 0.84), ck.lerp3(el, wr, 0.99)], [0.037, 0.036],
                     m[cuff_key or sleeve_key], coll, segments=12, ref=sref), TILE)
    return el, wr


def counter_hand(b, f: Figure, side: int, x: float, inward_deg: float, glove: bool = False, y: float = HAND_Y):
    """Main posée à plat sur le plateau (son point le plus bas à `COUNTER`) : renvoie la
    position du poignet."""
    wrist = Vector((x * side, y, COUNTER + 0.03))
    a = math.radians(inward_deg) * side
    fwd = (-math.sin(a), math.cos(a), -0.03)
    uv = ("rect", ca.rect_uv("leather" if glove else f"skin_{f.key}"))
    objs = ck.hand(b, f"hand_{side}", "skin", uv, thumb_side=-side, pose="flat", scale=f.s, glove=glove)
    ck.place(objs, tuple(wrist), fwd, (0, 0, 1))
    low = min((o.matrix_basis @ v.co).z for o in objs for v in o.data.vertices)
    lift = Matrix.Translation((0.0, 0.0, COUNTER - low - 0.0015))
    for o in objs:
        o.matrix_basis = lift @ o.matrix_basis
    return tuple(lift @ wrist)


def collar(b, f: Figure, mat_key: str):
    """Col officier de la chemise : bande basse autour de la base du cou."""
    m, coll = b.m, b.coll
    z0, z1 = f.top_z - 0.018, f.top_z + 0.012
    rx, ry, oy = f.torso.at(f.top_z)
    band = k.tube("collar", [(0, oy, z0), (0, oy - 0.002, z1)], [(rx + 0.009, ry + 0.009), (rx + 0.004, ry + 0.005)],
                  m[mat_key], coll, segments=18, cap_start=False, cap_end=False)
    mod = band.modifiers.new("epaisseur", "SOLIDIFY")
    mod.thickness = 0.004
    ck.apply_modifiers(band)
    b.lib(band, TILE)


def buttons(b, f: Figure, z0: float, z1: float, n: int, off: float):
    for i in range(n):
        z = z0 + (z1 - z0) * i / max(1, n - 1)
        p = f.torso.point(z, 90.0, off)
        btn = k.cyl(f"button_{i}", 0.0055, 0.003, b.m["gear"], b.coll, segments=8)
        k.orient(btn, p, (p[0], p[1] + 1.0, p[2]))
        b.cell(btn, "grip")


def belt(b, f: Figure, z: float, off: float, mat_key: str, uv, width: float = 0.034):
    n = 28
    pts, refs = [], []
    for i in range(n):
        a = 360.0 * i / n
        pts.append(f.torso.point(z + 0.004 * math.sin(math.radians(a)), a, off))
        refs.append((0, 0, 1))
    obj = k.tube("belt", pts, (width / 2, 0.004), b.m[mat_key], b.coll, segments=4, refs=refs, closed=True, smooth=False)
    b.add(obj, *uv)
    return obj


def surface_frame(f: Figure, z: float, deg: float, off: float):
    """Position et normale (horizontale) de la surface du torse."""
    rx, ry, oy = f.torso.at(z)
    a = math.radians(deg)
    n = Vector((math.cos(a) / (rx + off), math.sin(a) / (ry + off), 0.0)).normalized()
    return Vector(f.torso.point(z, deg, off)), n


def stuck_box(b, name, f: Figure, z, deg, off, size, mat_key, uv, tilt=0.0):
    """Pavé arrondi plaqué sur le torse (poche, rabat, boîtier) : épaisseur selon la normale."""
    p, n = surface_frame(f, z, deg, off + size[1] / 2)
    obj = k.rounded_box(name, size, (0, 0, 0), b.m[mat_key], b.coll, bevel=min(size) * 0.35, segments=2)
    yaw = math.atan2(n.y, n.x) - math.pi / 2
    obj.matrix_basis = Matrix.Translation(p) @ Matrix.Rotation(yaw, 4, "Z") @ Matrix.Rotation(math.radians(tilt), 4, "X")
    return b.add(obj, *uv)


# --- Guichetière -------------------------------------------------------------------------------

RINGS_A = [
    (0.81, 0.104, 0.084, -0.012), (0.86, 0.158, 0.106, -0.015), (0.93, 0.167, 0.111, -0.012),
    (1.00, 0.146, 0.099, -0.004), (1.07, 0.130, 0.092, 0.000), (1.15, 0.136, 0.096, 0.004),
    (1.23, 0.150, 0.108, 0.014), (1.29, 0.157, 0.117, 0.021), (1.35, 0.164, 0.110, 0.014),
    (1.41, 0.168, 0.098, 0.004), (1.462, 0.140, 0.078, -0.004), (1.497, 0.078, 0.058, -0.006),
    (1.52, 0.054, 0.050, -0.006),
]


def clerk_a(coll) -> list:
    head = ca.HEADS["a"]
    f = Figure("a", 1.74, RINGS_A, 0.158, 0.094, 0.090, 0.100, head, head_scale=0.97)
    m = materials("a", {"shirt": (0.60, 0.63, 0.66), "suit": (0.33, 0.35, 0.37)})
    b = ck.ClerkBuilder(m, coll)
    t = f.torso
    # Chemise (haut), pantalon (bassin et jambes), chaussures de cuir.
    b.lib(t.tube("shirt", m["shirt"], coll, f.belt_z - 0.03, f.top_z, segments=22), TILE)
    b.lib(t.tube("hips", m["suit"], coll, 0.81 * f.s, f.belt_z + 0.01, segments=22), TILE)
    legs(b, f, "suit", hem=0.050, flare=0.96)
    for side in (1, -1):
        ck.shoe(b, f"a{side}", (f.ankle[0] * side, f.ankle[1], 0), 6 * side, "skin", ("rect", ca.rect_uv("leather")),
                "gear", ("cell", "grip"), scale=0.92)
    collar(b, f, "shirt")
    # Gilet de travail : fermé devant (fermeture), encolure en V aux bords lisses.
    zv = f.shoulder_z - 0.16

    def span(z):
        w = 0.5 + 44.0 * ck_smooth((z - zv) / (f.top_z - zv))
        return 90.0 + w, 450.0 - w

    vest = ck.torso_patch("vest", m["suit"], coll, t, f.belt_z - 0.075, f.top_z - 0.006, 0, 0, 0.011, 0.006,
                          nz=12, na=32, span=span)
    b.lib(vest, TILE)
    zipper = k.tube("zipper", [t.point(f.belt_z - 0.07, 90, 0.017), t.point((f.belt_z + zv) / 2, 90, 0.018),
                               t.point(zv, 90, 0.016)], (0.0045, 0.002), m["gear"], coll, segments=6, ref=(1, 0, 0))
    b.cell(zipper, "grip")
    pull = k.rounded_box("zip_pull", (0.008, 0.004, 0.022), t.point(zv - 0.03, 90, 0.022), m["gear"], coll, bevel=0.0015)
    b.cell(pull, "scrap")
    for side, deg in ((1, 62.0), (-1, 118.0)):
        z = f.shoulder_z - 0.125
        stuck_box(b, f"chest_pocket_{side}", f, z, deg, 0.017, (0.085, 0.016, 0.095), "suit", ("lib", TILE))
        stuck_box(b, f"chest_flap_{side}", f, z + 0.045, deg, 0.034, (0.090, 0.008, 0.030), "suit", ("lib", TILE))
        stuck_box(b, f"low_pocket_{side}", f, f.belt_z - 0.025, 60.0 if side > 0 else 120.0, 0.017,
                  (0.10, 0.018, 0.085), "suit", ("lib", TILE))
    # Stylo dans la poche de poitrine gauche.
    p, n = surface_frame(f, f.shoulder_z - 0.07, 112.0, 0.04)
    pen = k.cyl("pen", 0.0045, 0.06, m["gear"], coll, segments=8)
    k.orient(pen, tuple(p), tuple(p + Vector((0, 0, 1))))
    b.cell(pen, "grip")
    # Tablette accrochée à la hanche droite : boîtier, écran (émissif faible), pince.
    p, n = surface_frame(f, f.belt_z - 0.075, -12.0, 0.035)
    yaw = math.atan2(n.y, n.x) - math.pi / 2
    tab = k.rounded_box("tablet", (0.13, 0.016, 0.18), (0, 0, 0), m["gear"], coll, bevel=0.006)
    tab.matrix_basis = Matrix.Translation(p) @ Matrix.Rotation(yaw, 4, "Z") @ Matrix.Rotation(math.radians(-4), 4, "Y")
    b.cell(tab, "grip")
    scr = k.rounded_box("tablet_screen", (0.11, 0.004, 0.15), (0, 0, 0), m["skin"], coll, bevel=0.0015)
    scr.matrix_basis = tab.matrix_basis @ Matrix.Translation((0, 0.008, 0.004))
    b.rect(scr, ca.rect_uv("screen", 3))
    clip = k.rounded_box("tablet_clip", (0.04, 0.012, 0.05), (0, 0, 0), m["gear"], coll, bevel=0.003)
    clip.matrix_basis = tab.matrix_basis @ Matrix.Translation((0, -0.012, 0.09))
    b.cell(clip, "scrap")
    # Bras : mains posées sur le plateau.
    for side in (1, -1):
        wrist = counter_hand(b, f, side, 0.125, 20.0)
        arm(b, f, side, wrist, (0.45 * side, -0.35, f.shoulder_z - 0.5), "shirt")
    # Casque-micro : écouteur gauche, arceau, petit coussinet à droite, perche vers la bouche.
    extra = headset(b, head)
    head_parts(b, f, yaw=-4.0, pitch=2.0, roll=1.5, extra=extra)
    return finish(b, "a")


def now(b: ck.ClerkBuilder, obj, mode: str, arg):
    """UV posées tout de suite (repère local de la pièce), transformation appliquée : pour les
    pièces qui suivent ensuite la tête (`place_head` transforme le maillage)."""
    if mode == "cell":
        k.uv_cell_shell(obj, arg)
    elif mode == "cyl":
        k.uv_cell_cyl(obj, arg)
    if "sh_shell_center" in obj:
        del obj["sh_shell_center"]
    k.apply_transform(obj)
    return b.keep(obj)


def headset(b, head: cf.Head) -> list:
    """Casque-micro léger (repère local de la tête) : arceau suivant le dessus du crâne,
    écouteur à gauche, coussinet à droite, perche vers la bouche, voyant."""
    import numpy as np

    m, coll = b.m, b.coll
    parts = []
    xs = cf.EAR_X * head.width
    arc = []
    for t in np.linspace(-1.0, 1.0, 13):
        th = t * 1.36
        a = -math.pi / 2 if th > 0 else math.pi / 2
        x, y, z, r = cf.surface(head, np.array([a]), np.array([math.pi / 2 - abs(th)]))
        d = Vector((float(x[0]), float(y[0]), float(z[0])))
        d = d + d.normalized() * 0.0028
        d.y -= 0.006
        arc.append(tuple(d))
    parts.append(now(b, k.tube("headset_band", arc, (0.0075, 0.0024), m["gear"], coll, segments=6, ref=(0, 1, 0)),
                     "cell", "grip"))
    for side, r, dep in ((-1, 0.024, 0.016), (1, 0.016, 0.010)):
        cup = k.cyl(f"headset_cup_{side}", r, dep, m["gear"], coll, segments=14, bevel=0.003)
        cup.data.transform(Matrix.Translation((0, 0, dep / 2)))
        k.orient(cup, (side * (xs + 0.006), -0.012, -0.002), (side * (xs + 0.05), -0.012, -0.002))
        parts.append(now(b, cup, "cyl", "grip"))
    led = k.cyl("headset_led", 0.004, 0.004, m["gear"], coll, segments=8)
    led.data.transform(Matrix.Translation((0, 0, 0.002)))
    k.orient(led, (-(xs + 0.021), -0.004, 0.008), (-(xs + 0.05), -0.004, 0.008))
    parts.append(now(b, led, "cell", "leds"))
    boom = k.spline([(-(xs + 0.016), 0.000, -0.010), (-(xs + 0.002), 0.045, -0.040), (-0.050, 0.082, -0.060),
                     (-0.030, 0.095, -0.068)], 4)
    parts.append(now(b, k.tube("headset_boom", boom, 0.0022, m["gear"], coll, segments=6), "cell", "grip"))
    mic = k.cyl("headset_mic", 0.0045, 0.014, m["gear"], coll, segments=8)
    k.orient(mic, boom[-2], boom[-1])
    parts.append(now(b, mic, "cell", "scrap"))
    return parts


def shoulder_top(f: Figure, x: float) -> float:
    """Hauteur du dessus de l'épaule (pente du trapèze) à la distance |x| de l'axe."""
    z = f.shoulder_z
    best = z
    while z < f.top_z:
        if f.torso.at(z)[0] >= abs(x):
            best = z
        z += 0.002
    return best


# --- Marchand ----------------------------------------------------------------------------------

RINGS_B = [
    (0.81, 0.112, 0.092, -0.012), (0.86, 0.168, 0.118, -0.010), (0.93, 0.180, 0.128, 0.000),
    (1.00, 0.178, 0.135, 0.016), (1.08, 0.178, 0.138, 0.024), (1.17, 0.180, 0.134, 0.022),
    (1.26, 0.190, 0.130, 0.018), (1.34, 0.202, 0.126, 0.012), (1.41, 0.205, 0.115, 0.002),
    (1.465, 0.172, 0.092, -0.004), (1.50, 0.102, 0.074, -0.006), (1.522, 0.070, 0.064, -0.006),
]


def clerk_b(coll) -> list:
    head = ca.HEADS["b"]
    f = Figure("b", 1.83, RINGS_B, 0.188, 0.100, 0.108, 0.115, head, head_scale=1.045)
    m = materials("b", {"suit": (0.40, 0.50, 0.54)})
    m["rubber"] = c.textured_material("char_rubber", "rubber")
    b = ck.ClerkBuilder(m, coll)
    t = f.torso
    # Combinaison : torse, bassin et jambes d'une pièce ; col ouvert.
    b.lib(t.tube("suit_top", m["suit"], coll, 0.81 * f.s, f.top_z, segments=22), TILE)
    legs(b, f, "suit", hem=0.050, flare=1.08, end_z=0.19 * f.s)
    collar(b, f, "suit")
    buttons(b, f, f.belt_z + 0.10, f.shoulder_z - 0.06, 3, 0.002)
    for side in (1, -1):
        ck.shoe(b, f"b{side}", (f.ankle[0] * side, f.ankle[1], 0), 8 * side, "rubber", ("lib", k.TILE_RUBBER),
                "rubber", ("lib", k.TILE_RUBBER), boot=True, scale=1.04)
    # Tablier de cuir : bavette conforme, jupe devant les jambes, bretelle de cou, liens.
    leather = ("rect", ca.rect_uv("leather"))
    zb0, zb1 = f.belt_z - 0.02, f.shoulder_z - 0.05
    bib = ck.torso_patch("apron_bib", m["skin"], coll, t, zb0, zb1, 52.0, 128.0, 0.014, 0.006, nz=8, na=14)
    b.rect(bib, leather[1])
    skirt_t = ck.Torso([(0.50 * f.s, 0.205, 0.150, 0.010), (0.75 * f.s, 0.200, 0.150, 0.020),
                        (f.belt_z, 0.190, 0.150, 0.022)])
    skirt = ck.torso_patch("apron_skirt", m["skin"], coll, skirt_t, 0.52 * f.s, f.belt_z + 0.005, 40.0, 140.0, 0.0,
                           0.006, nz=8, na=16)
    b.rect(skirt, leather[1])
    for side in (1, -1):
        p0 = t.point(zb1 - 0.01, 90 - 34 * side, 0.016)
        neck_pts = [p0, t.point(f.top_z - 0.01, 90 - 52 * side, 0.010), t.point(f.top_z + 0.004, 90 - 110 * side, 0.004),
                    t.point(f.top_z + 0.006, 270, 0.004)]
        sp_pts = k.spline(neck_pts, 3)
        refs = [(px, py - t.at(pz)[2], 0.0) for px, py, pz in sp_pts]
        b.cell(k.tube(f"apron_strap_{side}", sp_pts, (0.003, 0.012), m["gear"], coll, segments=4, refs=refs,
                      smooth=False), "webbing")
    belt(b, f, f.belt_z + 0.005, 0.016, "gear", ("cell", "webbing"), width=0.022)
    # Sacoche de toile sur la hanche gauche, bandoulière en travers du torse.
    sp, sn = surface_frame(f, f.belt_z - 0.10, 160.0, 0.06)
    bag_m = Matrix.Translation(sp) @ Matrix.Rotation(math.atan2(sn.y, sn.x) - math.pi / 2, 4, "Z")
    bag = k.rounded_box("satchel", (0.22, 0.075, 0.17), (0, 0, 0), m["gear"], coll, bevel=0.018, segments=3)
    bag.matrix_basis = bag_m
    b.cell(bag, "canvas")
    flap = k.rounded_box("satchel_flap", (0.225, 0.012, 0.10), (0, 0, 0), m["skin"], coll, bevel=0.005)
    flap.matrix_basis = bag_m @ Matrix.Translation((0, 0.040, 0.045))
    b.rect(flap, ca.rect_uv("leather"))
    buckle = k.rounded_box("satchel_buckle", (0.03, 0.008, 0.026), (0, 0, 0), m["gear"], coll, bevel=0.003)
    buckle.matrix_basis = bag_m @ Matrix.Translation((0, 0.048, 0.005))
    b.cell(buckle, "scrap")
    n = 28
    pts, refs = [], []
    for i in range(n):
        a = 2 * math.pi * i / n
        z = f.belt_z + 0.18 + 0.30 * math.cos(a - math.pi)
        deg = math.degrees(a)
        on_apron = 50 < deg % 360 < 130 and zb0 < z < zb1
        off = 0.028 if on_apron else 0.014
        pts.append(t.point(min(z, f.top_z - 0.02), deg, off * (0.85 if z > f.shoulder_z else 1.0)))
        refs.append((math.cos(a), math.sin(a), 0))
    b.cell(k.tube("bandolier", pts, (0.006, 0.020), m["gear"], coll, segments=4, refs=refs, closed=True, smooth=False),
           "webbing")
    # Bras : main droite gantée sur le plateau ; main gauche sur la sacoche.
    wrist = counter_hand(b, f, 1, 0.20, 12.0, glove=True)
    arm(b, f, 1, wrist, (0.5, -0.35, f.shoulder_z - 0.5), "suit", rolled=True, glove=True)
    top = Vector(bag_m @ Vector((0.0, -0.01, 0.085)))
    wrist_l = (top.x, top.y - 0.03, top.z + 0.03)
    el, wr = arm(b, f, -1, wrist_l, (-0.6, -0.4, f.shoulder_z - 0.4), "suit", rolled=True, glove=True)
    objs = ck.hand(b, "hand_l", "skin", ("rect", ca.rect_uv("leather")), thumb_side=1.0, pose="relaxed", scale=f.s,
                   glove=True)
    ck.place(objs, wr, (0.08, 1.0, -0.55), (0.1, 0.45, 1.0))
    head_parts(b, f, yaw=3.0, pitch=-1.0, roll=-1.0)
    return finish(b, "b")


# --- Tenancier ---------------------------------------------------------------------------------

RINGS_C = [
    (0.81, 0.100, 0.086, -0.012), (0.86, 0.150, 0.106, -0.012), (0.93, 0.162, 0.114, -0.004),
    (1.00, 0.156, 0.122, 0.012), (1.08, 0.155, 0.126, 0.022), (1.17, 0.157, 0.120, 0.018),
    (1.26, 0.168, 0.117, 0.014), (1.34, 0.180, 0.113, 0.010), (1.41, 0.184, 0.103, 0.002),
    (1.465, 0.152, 0.083, -0.004), (1.50, 0.090, 0.066, -0.006), (1.522, 0.062, 0.057, -0.006),
]


def clerk_c(coll) -> list:
    head = ca.HEADS["c"]
    f = Figure("c", 1.79, RINGS_C, 0.176, 0.094, 0.104, 0.112, head)
    m = materials("c", {"shirt": (0.80, 0.76, 0.66), "trousers": (0.30, 0.26, 0.22)})
    b = ck.ClerkBuilder(m, coll)
    t = f.torso
    b.lib(t.tube("shirt", m["shirt"], coll, f.belt_z - 0.03, f.top_z, segments=22), TILE)
    b.lib(t.tube("hips", m["trousers"], coll, 0.81 * f.s, f.belt_z + 0.01, segments=22), TILE)
    legs(b, f, "trousers", hem=0.054)
    collar(b, f, "shirt")
    buttons(b, f, f.belt_z + 0.06, f.shoulder_z - 0.10, 4, 0.002)
    for side in (1, -1):
        ck.shoe(b, f"c{side}", (f.ankle[0] * side, f.ankle[1], 0), 9 * side, "skin", ("rect", ca.rect_uv("leather")),
                "gear", ("cell", "grip"))
    belt(b, f, f.belt_z, 0.006, "skin", ("rect", ca.rect_uv("leather")))
    # Long tablier noué à la taille : jupe enveloppante jusque sous le genou, cordon.
    apron_t = ck.Torso([(0.40 * f.s, 0.205, 0.150, 0.026), (0.70 * f.s, 0.196, 0.152, 0.028),
                        (0.92 * f.s, 0.180, 0.150, 0.026), (f.belt_z, 0.170, 0.134, 0.024)])
    apron = ck.torso_patch("apron", m["skin"], coll, apron_t, 0.42 * f.s, f.belt_z + 0.012, 18.0, 162.0, 0.012,
                           0.005, nz=10, na=22)
    b.rect(apron, ca.rect_uv("apron"))
    n = 28
    pts = [t.point(f.belt_z + 0.004, 360.0 * i / n, 0.020) for i in range(n)]
    b.rect(k.tube("apron_string", pts, (0.0065, 0.003), m["skin"], coll, segments=4, refs=[(0, 0, 1)] * n, closed=True,
                  smooth=False), ca.rect_uv("apron"))
    for i, (dx, ln) in enumerate(((0.02, 0.16), (-0.025, 0.13))):
        p0 = Vector(t.point(f.belt_z, 270.0, 0.02))
        tail = k.tube(f"apron_tail_{i}", [tuple(p0), tuple(p0 + Vector((dx, -0.012, -ln * 0.5))),
                                          tuple(p0 + Vector((dx * 1.6, -0.006, -ln)))], (0.010, 0.003), m["skin"], coll,
                      segments=4, ref=(1, 0, 0), smooth=False)
        b.rect(tail, ca.rect_uv("apron"))
    # Torchon sur l'épaule gauche : de la poitrine, par-dessus l'épaule, dans le dos.
    path = []
    for z, deg, off in ((f.shoulder_z - 0.20, 118.0, 0.012), (f.shoulder_z - 0.10, 122.0, 0.014),
                        (f.shoulder_z - 0.02, 128.0, 0.016)):
        path.append(t.point(z, deg, off))
    xs = -0.118
    zt = shoulder_top(f, xs) + 0.007
    path += [(xs, 0.040, zt - 0.010), (xs - 0.003, -0.006, zt), (xs, -0.050, zt - 0.012)]
    for z, deg, off in ((f.shoulder_z - 0.03, 232.0, 0.016), (f.shoulder_z - 0.13, 236.0, 0.013),
                        (f.shoulder_z - 0.24, 238.0, 0.012)):
        path.append(t.point(z, deg, off))
    towel = k.tube("towel", k.spline(path, 3), (0.060, 0.0055), m["skin"], coll, segments=8, ref=(1, 0, 0))
    b.add(towel, "tube", (ca.rect_uv("towel"), 8))
    # Bras : appuyé des deux mains sur le bar, écartées.
    for side in (1, -1):
        wrist = counter_hand(b, f, side, 0.29, 28.0, y=HAND_Y + 0.02)
        arm(b, f, side, wrist, (0.6 * side, -0.25, f.shoulder_z - 0.45), "shirt", rolled=True)
    head_parts(b, f, yaw=6.0, pitch=-2.0, roll=-3.5)
    return finish(b, "c")


# --- Assemblage --------------------------------------------------------------------------------


STATS = "--stats" in sys.argv


def finish(b: ck.ClerkBuilder, key: str) -> list:
    parts = b.finalize()
    if STATS:
        groups: dict[str, int] = {}
        for o in parts:
            name = o.name.split("_")[0] if not o.name.startswith(("hand", "head")) else o.name[:6]
            groups[name] = groups.get(name, 0) + k.triangles([o])
        top = sorted(groups.items(), key=lambda kv: -kv[1])[:12]
        print(f"clerk_{key} triangles par pièce : " + ", ".join(f"{n} {t}" for n, t in top))
    exports = c.collection(f"EXPORT_{key}")
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        merged = c.join_by_material(parts, f"clerk_{key}", exports)
    for obj in merged:
        k.apply_transform(obj)
    b.coll.hide_render = True
    b.coll.hide_viewport = True
    return merged


BUILDERS = {"a": clerk_a, "b": clerk_b, "c": clerk_c}


def build(only: str = "abc") -> dict:
    c.reset_scene()
    out = {}
    for key in "abc":
        if key in only:
            out[key] = BUILDERS[key](c.collection(f"ASSET_clerk_{key}"))
    return out


VIEWS = ("face", "3q", "eye", "bust", "hands", "ambient", "lineup")


def previews(models: dict, views=VIEWS) -> None:
    import char_preview as pv

    pv.OUT = ca.PREVIEW_DIR
    keys = list(models)
    # Chacun seul, centré à l'origine : face, trois-quarts, de près à hauteur d'yeux (1,7 m),
    # buste, mains sur le plateau, ambiante seule (comme en jeu dans une cellule à lightmap).
    for key in keys:
        for k2, objs in models.items():
            for o in objs:
                o.hide_render = k2 != key
        eye = 1.62 if key == "a" else (1.70 if key == "b" else 1.66)
        pv.setup("studio", (0.0, 0.1, 1.0))
        if "face" in views:
            pv.render(f"clerk_{key}_face", pv.orbit((0, 0.0, 1.0), 3.4, 0, 0.1), (0, 0, 0.95), lens=50)
        if "3q" in views:
            pv.render(f"clerk_{key}_3q", pv.orbit((0, 0.0, 1.0), 3.4, 35, 0.3), (0, 0, 0.95), lens=50)
        if "eye" in views:
            pv.render(f"clerk_{key}_eye", (0.15, 1.05, 1.70), (0.0, 0.0, eye - 0.08), lens=45, res=(1200, 900))
        if "bust" in views:
            pv.render(f"clerk_{key}_bust", pv.orbit((0, 0.05, eye - 0.05), 0.65, 22, 0.02), (0, 0.04, eye - 0.06),
                      lens=50, res=(1000, 1000))
        if "hands" in views:
            plate = c.box("preview_counter_top", (1.2, 0.62, 0.04), (0, 0.13 + 0.31, COUNTER - 0.02),
                          c.material("preview_counter_top", (0.16, 0.155, 0.15), metallic=0.3, roughness=0.35))
            plate["sh_preview"] = True
            pv.render(f"clerk_{key}_hands", (0.25, 0.95, 1.42), (0.0, 0.22, COUNTER), lens=40, res=(1200, 800))
            bpy.data.objects.remove(plate, do_unlink=True)
        if "ambient" in views:
            pv.setup("ambient", (0.0, 0.1, 1.0))
            pv.render(f"clerk_{key}_ambient", pv.orbit((0, 0.0, 1.0), 3.4, 20, 0.3), (0, 0, 0.95), lens=50)
    for o in [o for objs in models.values() for o in objs]:
        o.hide_render = False
    if "lineup" in views and len(keys) > 1:
        lineup(models, pv)


def lineup(models: dict, pv) -> None:
    """Les trois côte à côte derrière un pavé de 1,05 m (comptoir)."""
    spacing = 1.1
    keys = list(models)
    for i, key in enumerate(keys):
        dx = (i - (len(keys) - 1) / 2) * spacing
        for o in models[key]:
            o.location = (dx, 0.0, 0.0)
    mat = c.material("preview_counter", (0.10, 0.10, 0.11), roughness=0.5)
    top = c.material("preview_counter_top", (0.16, 0.155, 0.15), metallic=0.3, roughness=0.35)

    def counter():
        w = len(keys) * spacing + 0.3
        body = c.box("preview_counter", (w, 0.55, COUNTER - 0.04), (0, 0.15 + 0.275, (COUNTER - 0.04) / 2), mat)
        plate = c.box("preview_counter_top", (w + 0.1, 0.62, 0.04), (0, 0.13 + 0.31, COUNTER - 0.02), top)
        for o in (body, plate):
            o["sh_preview"] = True

    pv.setup("studio", (0.0, 0.3, 1.2))
    counter()
    pv.render("clerks_lineup", (0.4, 3.6, 1.75), (0.0, 0.0, 1.25), lens=40, res=(1600, 900))
    pv.render("clerks_lineup_eye", (-0.9, 1.9, 1.70), (0.0, 0.0, 1.35), lens=35, res=(1600, 900))
    pv.setup("dim", (0.0, 0.3, 1.2))
    counter()
    pv.render("clerks_lineup_dim", (0.3, 2.6, 1.70), (0.0, 0.0, 1.30), lens=40, res=(1600, 900))
    for key in keys:
        for o in models[key]:
            o.location = (0.0, 0.0, 0.0)


def main(argv: list[str]) -> None:
    if "--atlas" in argv:
        ca.generate()
    only = "abc"
    if "--only" in argv:
        only = argv[argv.index("--only") + 1].replace(",", "")
    models = build(only)
    for key, merged in models.items():
        tris = k.triangles(merged)
        c.export_gltf(f"assets/models/characters/clerk_{key}.gltf", merged)
        mats = sorted({o.data.materials[0].name for o in merged})
        co = [v.co for o in merged for v in o.data.vertices]
        lo = [min(p[i] for p in co) for i in range(3)]
        hi = [max(p[i] for p in co) for i in range(3)]
        # Contrainte du comptoir : avancée du corps sous le plateau, recul des mains posées.
        under = max(p[1] for p in co if p[2] < COUNTER - 0.005)
        on_top = [p[1] for p in co if COUNTER - 0.003 < p[2] < COUNTER + 0.004 and p[1] > 0.12]
        print(f"clerk_{key} : {len(merged)} maillages, {tris} triangles, matériaux {mats}, "
              f"boîte x [{lo[0]:.3f}, {hi[0]:.3f}] y [{lo[1]:.3f}, {hi[1]:.3f}] z [{lo[2]:.3f}, {hi[2]:.3f}], "
              f"corps sous le plateau jusqu'à y = {under:.3f}, appuis sur le plateau dès y = "
              f"{min(on_top) if on_top else float('nan'):.3f}")
    if only == "abc":
        c.save_blend("art/characters/clerks.blend")
    if "--previews" in argv:
        views = VIEWS
        if "--views" in argv:
            views = tuple(argv[argv.index("--views") + 1].split(","))
        previews(models, views)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
