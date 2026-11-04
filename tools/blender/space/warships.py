"""Coques de combat du raid (proposition P26), vues de l'extérieur en vue Globale — M8, lot D.

- `pirate_corvette` : corvette « de facture pirate » (canon §2.4), ≈ 70 m. Identité proposée
  (P6) : un vieux cargo armé, assemblage asymétrique de pièces dépareillées, plaques rapiécées
  et soudées, rouille et rouge sang, armement rajouté ;
- `ugl_corvette` : corvette conventionnée de l'Union (classe normalisée, §2.4), ≈ 90 m, de la
  Flotte de défense ; lignes nettes et répétées, gris bleu à bande bleu pâle (couleurs de
  l'Union non canon : P6).

Direction « futur usé » (docs/refonte-visuelle.md §2) : la **géométrie** porte le détail lisible
de loin (plaques en relief, nervures, radiateurs, réservoirs, tourelles de canons magnétiques,
lance-missiles, antennes, tuyères, verrières) ; les textures de la bibliothèque
(`hull_plates`, `metal_dark`, `metal_bare`, `trim_ship`) l'usent de près. Aucun bouclier
(AGENTS.md §13), aucune inscription hors pochoirs génériques de la trim sheet.

Éclairage : vue Globale, soleil dynamique, pas de lightmap. Les feux, tuyères et hublots
sont émissifs : une petite feuille de pastilles unies cuite par ce script
(`assets/textures/materials/warship_lights_*.png`, 128 px) donne leurs couleurs à un seul
matériau (UV de chaque pièce posées au centre de sa pastille).

Textures propres au lot (cuites par ce script, déterministes, dans la bibliothèque partagée) :
`warship_lights_{color,emissive,orm}` (pastilles) et `warship_rust_{color,normal,orm}` (1K :
le blindage `hull_plates` de la bibliothèque repeint en brun rouille, plaques de rouille et
coulures ; sous une teinte rouge, le « rouge sang » de la corvette pirate). La corvette de
l'Union garde `hull_plates` teinté gris bleu (P6) et une bande bleu pâle.

Contrôle : chaque pièce fermée doit avoir un volume signé positif (le jeu masque les faces
arrière) ; sinon le script s'arrête en erreur.

Budgets (lot D) : ≤ 30 000 triangles et ≤ 6 matériaux par coque (dix coques à l'écran en
combat), fusion par matériau. Une unité = 1 m ; l'avant vers +Y, origine au centre de masse ;
les silhouettes restent dans les rayons de collision de `gameplay/combat.ron` (16 et 18 m en
largeur et en hauteur).

Sorties : assets/models/space/<nom>.gltf (+ .bin, images citées dans la bibliothèque) et
art/space/warships.blend. Lancer :

    blender --background --factory-startup --python tools/blender/space/warships.py -- [--previews]

`--previews` rend aussi les aperçus (Cycles CPU) dans target/art-previews/lotD/.
"""

from __future__ import annotations

import contextlib
import io
import math
import random
import sys
from collections import namedtuple
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import common as c  # noqa: E402

OUT = "assets/models/space"
PREVIEW_DIR = c.ROOT / "target" / "art-previews" / "lotD"

# --- Feuille de pastilles émissives ----------------------------------------------------------
# Grille 4 × 4 de pastilles unies (32 px) : (couleur de base sRGB, émission sRGB). Les UV d'une
# pièce lumineuse sont toutes au centre de sa pastille : dérivées nulles, donc toujours le
# niveau de mipmap 0, sans fuite de couleur d'une pastille à l'autre.

LIGHT_SHEET = "warship_lights"
SHEET_RES = 128
SWATCHES: dict[str, tuple[int, int, tuple, tuple]] = {
    # nom : (colonne, ligne, couleur, émission)
    "engine_red": (0, 0, (1.0, 0.60, 0.30), (1.0, 0.46, 0.08)),
    "engine_blue": (1, 0, (0.70, 0.85, 1.0), (0.45, 0.70, 1.0)),
    "core_hot": (2, 0, (1.0, 0.92, 0.80), (1.0, 0.86, 0.66)),
    "core_blue": (3, 0, (0.90, 0.95, 1.0), (0.82, 0.92, 1.0)),
    "nav_red": (0, 1, (1.0, 0.25, 0.20), (1.0, 0.06, 0.03)),
    "nav_green": (1, 1, (0.30, 1.0, 0.45), (0.10, 1.0, 0.30)),
    "nav_white": (2, 1, (1.0, 1.0, 0.95), (1.0, 0.96, 0.88)),
    "beacon_amber": (3, 1, (1.0, 0.70, 0.30), (1.0, 0.55, 0.10)),
    # Hublots : moins forts que les feux (l'intérieur d'un vaisseau, pas une lampe).
    "window_warm": (0, 2, (0.85, 0.62, 0.36), (0.55, 0.36, 0.14)),
    "window_cool": (1, 2, (0.62, 0.74, 0.86), (0.30, 0.42, 0.55)),
    "window_dark": (2, 2, (0.05, 0.06, 0.07), (0.0, 0.0, 0.0)),
    "strip_cool": (3, 2, (0.75, 0.85, 1.0), (0.35, 0.55, 0.85)),
    # Paroi intérieure des tuyères : plus sombre que le cœur (lisible sans éclairage indirect).
    "liner_red": (0, 3, (0.9, 0.50, 0.22), (0.85, 0.34, 0.04)),
    "liner_blue": (1, 3, (0.55, 0.70, 0.95), (0.20, 0.38, 0.75)),
}
LIGHT_STRENGTH = 8.0  # KHR_materials_emissive_strength (émissif non soumis à l'exposition dans Bevy)


def write_light_sheet() -> None:
    """Cuit `warship_lights_{color,emissive,orm}.png` (déterministe, quelques millisecondes)."""
    import numpy as np

    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "materials"))
    import texgen as tg

    cell = SHEET_RES // 4
    color = np.full((SHEET_RES, SHEET_RES, 3), 0.05, dtype=np.float32)
    emis = np.zeros((SHEET_RES, SHEET_RES, 3), dtype=np.float32)
    orm = np.zeros((SHEET_RES, SHEET_RES, 3), dtype=np.float32)
    orm[..., 0] = 1.0  # occlusion
    orm[..., 1] = 0.35  # rugosité : verre et diffuseurs
    orm[..., 2] = 0.0
    for col, row, base, glow in SWATCHES.values():
        sl = (slice(row * cell, (row + 1) * cell), slice(col * cell, (col + 1) * cell))
        color[sl] = base
        emis[sl] = glow
    # Les PNG sont écrits tels quels (valeurs sRGB pour la couleur et l'émission).
    for role, img in (("color", color), ("emissive", emis), ("orm", orm)):
        tg.write_png(c.TEXTURES_DIR / f"{LIGHT_SHEET}_{role}.png", img)


RUST = "warship_rust"


def write_rust_texture() -> None:
    """Cuit `warship_rust_{color,normal,orm}.png` (1024 px, tuile de 4 m comme `hull_plates`) :
    le relief du blindage de la bibliothèque (plaques, rivets, trappes) repeint en brun rouille,
    plaques de rouille groupées près des joints, piqûres, coulures vers le bas (V), rouille mate.
    Non teintée : le rouge sang de la corvette pirate est la même texture sous une teinte rouge.
    Déterministe (graines fixes) ; ≈ 10 s."""
    import numpy as np

    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "materials"))
    import surfaces
    import texgen as tg

    res, tile = 1024, 4.0
    base = surfaces.hull_plates(res, tile)
    cv = tg.Canvas(res, tile, seed=7310)
    col0, n0, orm0 = base["color"], base["normal"] * 2.0 - 1.0, base["orm"]
    lum = col0.mean(-1)
    shade = lum / lum.mean()
    seam = np.clip((1.0 - n0[..., 2]) * 8.0, 0.0, 1.0)  # joints, rivets, trappes : la normale s'incline
    near_seam = np.clip(cv.blur(seam, 0.04) * 3.0, 0.0, 1.0)
    blot = tg.cut(cv.noise(1, 1.4, 0.04) + 1.4 * near_seam + 0.4 * cv.noise(11, 0.2, 0.01), 0.26, 0.35)
    pits = tg.cut(cv.noise(2, 0.12, 0.008), 0.10, 0.25) * tg.smoothstep(-0.2, 0.6, cv.noise(3, 1.6, 0.3))
    rust = np.clip(blot + 0.7 * pits, 0.0, 1.0)
    streak = tg.drips(cv, np.clip(0.7 * seam + 0.6 * rust, 0.0, 1.0), 0.9, 4) * (1.0 - rust)
    paint = tg.rgb((0.47, 0.29, 0.20)) * shade[..., None] * tg.tone(cv, 8, 2.0, 0.06)[..., None]
    hue = np.clip(0.5 + 0.5 * cv.noise(5, 0.5, 0.01), 0.0, 1.0)[..., None]
    rust_col = tg.lerp(tg.rgb((0.30, 0.13, 0.06)), tg.rgb((0.60, 0.31, 0.12)), hue)
    col = tg.lerp(paint, rust_col, rust[..., None])
    col = tg.lerp(col, tg.rgb((0.17, 0.09, 0.05)), (0.6 * streak)[..., None])
    col = col * (0.85 + 0.15 * shade[..., None] / max(float(shade.max()), 1e-6))
    # Rouille en relief : grain fin dans les zones atteintes.
    h = 0.0006 * rust * cv.noise(6, 0.04, 0.003)
    n1 = cv.normal(h) * 2.0 - 1.0
    n = np.concatenate([n0[..., :2] + n1[..., :2], (n0[..., 2] * n1[..., 2])[..., None]], -1)
    n /= np.maximum(np.linalg.norm(n, axis=-1, keepdims=True), 1e-6)
    rough = np.clip(orm0[..., 1] * (1.0 - rust) + 0.9 * rust + 0.05 * streak, 0.05, 1.0)
    metal = np.clip(orm0[..., 2] * (1.0 - rust), 0.0, tg.METAL_MAX)
    orm = np.stack([orm0[..., 0], rough, metal], -1)
    for role, img in (("color", np.clip(col, 0, 1)), ("normal", n * 0.5 + 0.5), ("orm", orm)):
        tg.write_png(c.TEXTURES_DIR / f"{RUST}_{role}.png", img.astype(np.float32))


def swatch_uv(name: str) -> tuple[float, float]:
    col, row, _, _ = SWATCHES[name]
    return ((col + 0.5) / 4.0, (row + 0.5) / 4.0)


# --- Matériaux (6 par coque au plus) -----------------------------------------------------------


def palette() -> dict:
    shared = {
        "dark": c.textured_material("warship_metal_dark", "metal_dark"),
        # Métal brut assombri : sous un soleil sans atmosphère, il sortait blanc et neuf.
        "bare": c.textured_material("warship_metal_bare", "metal_bare", tint=(0.6, 0.58, 0.56)),
        "trim": c.textured_material("warship_trim", "trim_ship"),
        "lights": c.textured_material(
            "warship_lights", LIGHT_SHEET, emissive=(1.0, 1.0, 1.0), emissive_strength=LIGHT_STRENGTH
        ),
    }
    pirate = dict(shared)
    # Pirate : blindage rouillé cuit par ce script (`warship_rust`), brun rouille tel quel, et
    # sous une teinte rouge pour les tôles « rouge sang » (P6 : `pirate_rust`, `pirate_red`).
    pirate["hull"] = c.textured_material("warship_pirate_rust", RUST)
    pirate["accent"] = c.textured_material("warship_pirate_red", RUST, tint=(1.0, 0.36, 0.30))
    ugl = dict(shared)
    # Gris bleu P6 assombri et un peu plus bleu : la teinte de la bibliothèque, sur le blindage
    # clair et sous un soleil dur, sortait gris neutre.
    ugl["hull"] = c.textured_material("warship_ugl_hull", "hull_plates", tint=(0.48, 0.58, 0.72))
    # Bande bleu pâle (P6) : même blindage, plus clair et plus saturé que la coque.
    ugl["accent"] = c.textured_material("warship_ugl_stripe", "hull_plates", tint=(0.50, 0.80, 1.0))
    return {"pirate": pirate, "ugl": ugl}


# Densité des textures par matériau (mètres par tuile) : les coques font 70 à 90 m, la tuile
# de 4 m du catalogue y serait un damier ; les pièces rapportées gardent une tuile plus fine.
TILE = {"hull": 14.0, "accent": 10.0, "dark": 5.0, "bare": 3.0}


# --- Géométrie : bmesh dans le repère de la coque ---------------------------------------------

Sec = namedtuple("Sec", "y hw ht hb ch cx cz cb", defaults=(0.0, 0.0, -1.0))
"""Section d'un fuseau à pans coupés : position y, demi-largeur, hauteur au-dessus et
au-dessous du centre (cx, cz), chanfrein des arêtes hautes (`ch`) et basses (`cb`, négatif :
le même que `ch` ; ne pas mélanger les deux conventions dans un même fuseau)."""

IDENTITY = Matrix.Identity(4)


def T(x: float, y: float, z: float) -> Matrix:
    return Matrix.Translation((x, y, z))


def Rot(deg: float, axis) -> Matrix:
    return Matrix.Rotation(math.radians(deg), 4, axis)


def bm_box(sx: float, sy: float, sz: float, bevel: float = 0.0) -> bmesh.types.BMesh:
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * sx, v.co.y * sy, v.co.z * sz))
    if bevel > 0.0:
        bmesh.ops.bevel(bm, geom=list(bm.edges), offset=bevel, segments=1, affect="EDGES", profile=0.5)
    return bm


def ring_points(s: Sec) -> list[tuple[float, float]]:
    """Huit points de la section (x, z) ; chanfreins du haut et du bas bornés séparément (un
    pont plat garde ses arêtes hautes chanfreinées)."""
    hw, ht, hb, cx, cz = s.hw, s.ht, s.hb, s.cx, s.cz
    ct = max(0.02, min(s.ch, hw * 0.9, ht * 0.9))
    cb = max(0.02, min(s.cb if s.cb >= 0 else s.ch, hw * 0.9, hb * 0.9))
    return [
        (cx + hw - ct, cz + ht), (cx + hw, cz + ht - ct), (cx + hw, cz - hb + cb), (cx + hw - cb, cz - hb),
        (cx - hw + cb, cz - hb), (cx - hw, cz - hb + cb), (cx - hw, cz + ht - ct), (cx - hw + ct, cz + ht),
    ]


def bm_loft(secs: list[Sec]) -> bmesh.types.BMesh:
    """Fuseau à section octogonale (rectangle chanfreiné) le long de Y, fermé aux deux bouts."""
    bm = bmesh.new()
    rings = [[bm.verts.new((x, s.y, z)) for x, z in ring_points(s)] for s in secs]
    for a, b in zip(rings, rings[1:]):
        for i in range(8):
            j = (i + 1) % 8
            bm.faces.new((a[i], a[j], b[j], b[i]))
    bm.faces.new(rings[0])
    bm.faces.new(list(reversed(rings[-1])))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def section_at(secs: list[Sec], y: float) -> Sec:
    """Section interpolée d'un fuseau en y (bornée à ses extrémités)."""
    if y <= secs[0].y:
        return secs[0]._replace(y=y)
    for a, b in zip(secs, secs[1:]):
        if a.y <= y <= b.y:
            t = (y - a.y) / max(b.y - a.y, 1e-9)
            return Sec(y, *(av + (bv - av) * t for av, bv in zip(a[1:], b[1:])))
    return secs[-1]._replace(y=y)


def shell(secs: list[Sec], y0: float, y1: float, grow: float) -> list[Sec]:
    """Sections d'une bande qui épouse le fuseau entre y0 et y1, épaissie de `grow`."""
    ys = [y0] + [s.y for s in secs if y0 < s.y < y1] + [y1]
    out = []
    for y in ys:
        s = section_at(secs, y)
        cb = s.cb + grow * 0.41 if s.cb >= 0 else -1.0
        out.append(s._replace(hw=s.hw + grow, ht=s.ht + grow, hb=s.hb + grow, ch=s.ch + grow * 0.41, cb=cb))
    return out


def bm_lathe(profile: list[tuple[float, float]], seg: int = 12, closed: bool = False, smooth: float = 0.0,
             caps: bool = True):
    """Révolution autour de Y d'un profil (rayon, position y) ; rayon nul = pôle. `closed` : le
    profil est une boucle (tube épais, tuyère). `caps` : boucher les bouts d'un profil ouvert
    (False pour une bague ou une sangle posée sur un volume : pas de triangles cachés).
    `smooth` : ombrage lissé, arêtes vives au-delà de cet angle (degrés)."""
    bm = bmesh.new()
    angles = [2 * math.pi * i / seg for i in range(seg)]
    rings = []
    for r, y in profile:
        if r <= 1e-6:
            rings.append([bm.verts.new((0.0, y, 0.0))])
        else:
            rings.append([bm.verts.new((r * math.cos(t), y, r * math.sin(t))) for t in angles])

    def bridge(a, b):
        if len(a) == 1 and len(b) == 1:
            return
        for i in range(seg):
            j = (i + 1) % seg
            if len(a) == 1:
                bm.faces.new((a[0], b[i], b[j]))
            elif len(b) == 1:
                bm.faces.new((a[i], a[j], b[0]))
            else:
                bm.faces.new((a[i], a[j], b[j], b[i]))

    for a, b in zip(rings, rings[1:]):
        bridge(a, b)
    if closed:
        bridge(rings[-1], rings[0])
    elif caps:
        if len(rings[0]) > 1:
            bm.faces.new(rings[0])
        if len(rings[-1]) > 1:
            bm.faces.new(rings[-1])
    if closed or caps:
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    else:
        # Bague ouverte : normales vers l'extérieur (rayon), par construction.
        flip = []
        for f in bm.faces:
            f.normal_update()
            cen = f.calc_center_median()
            if f.normal.x * cen.x + f.normal.z * cen.z < 0:
                flip.append(f)
        bmesh.ops.reverse_faces(bm, faces=flip)
    if smooth > 0.0:
        for f in bm.faces:
            f.smooth = True
        limit = math.radians(smooth)
        for e in bm.edges:
            if len(e.link_faces) == 2 and e.calc_face_angle(0.0) > limit:
                e.smooth = False
    return bm


def bm_cyl(r: float, length: float, seg: int = 12, smooth: float = 50.0):
    """Cylindre le long de Y, centré, ombrage lissé sur le flanc."""
    h = length / 2
    return bm_lathe([(0, -h), (r, -h), (r, h), (0, h)], seg, smooth=smooth)


# --- Assemblage d'une coque ----------------------------------------------------------------------


class Ship:
    """Pièces d'une coque : chaque pièce est un maillage dans le repère de la coque, avec son
    matériau et ses UV (projection boîte décalée au hasard, bande de trim, ou pastille)."""

    def __init__(self, name: str, mats: dict, coll, seed: int):
        self.name = name
        self.mats = mats
        self.coll = coll
        self.rng = random.Random(seed)
        self.parts: list[bpy.types.Object] = []

    def add(self, bm, mat: str, M: Matrix = IDENTITY, trim=None, mpt: float = 2.0, swatch: str | None = None,
            tile: float | None = None):
        bmesh.ops.transform(bm, matrix=M, verts=bm.verts)
        if M.determinant() < 0:
            bmesh.ops.reverse_faces(bm, faces=bm.faces)
        name = f"{self.name}_{mat}_{len(self.parts):04d}"
        mesh = bpy.data.meshes.new(name)
        bm.to_mesh(mesh)
        bm.free()
        mesh.materials.append(self.mats[mat])
        obj = bpy.data.objects.new(name, mesh)
        self.coll.objects.link(obj)
        if mat == "lights":
            u, v = swatch_uv(swatch or "nav_white")
            layer = mesh.uv_layers.new(name=c.UV_TEXTURE)
            layer.data.foreach_set("uv", [u, v] * len(mesh.loops))
        elif mat == "trim":
            c.uv_trim(obj, trim or "edge", meters_per_tile=mpt)
        else:
            off = (self.rng.random(), self.rng.random())
            c.uv_box(obj, tile or TILE[mat], offset=off)
        self.parts.append(obj)
        return obj

    # Raccourcis -------------------------------------------------------------------------------

    def box(self, mat, size, at, M: Matrix = IDENTITY, bevel=0.0, rot: Matrix = IDENTITY, **kw):
        """Pavé centré en `at` dans le repère `M`, tourné sur lui-même par `rot`."""
        return self.add(bm_box(*size, bevel=bevel), mat, M @ T(*at) @ rot, **kw)

    def loft(self, mat, secs, M: Matrix = IDENTITY, **kw):
        return self.add(bm_loft(secs), mat, M, **kw)

    def lathe(self, mat, profile, M: Matrix = IDENTITY, seg=12, closed=False, smooth=0.0, caps=True, **kw):
        return self.add(bm_lathe(profile, seg, closed, smooth, caps), mat, M, **kw)

    def band(self, mat, r: float, y0: float, y1: float, M: Matrix = IDENTITY, seg=12, **kw):
        """Bague ouverte (sangle, frette, collier) de rayon r entre y0 et y1 le long de Y local."""
        return self.lathe(mat, [(r, y0), (r, y1)], M, seg=seg, caps=False, **kw)

    def cyl(self, mat, r, length, M: Matrix = IDENTITY, seg=12, **kw):
        return self.add(bm_cyl(r, length, seg), mat, M, **kw)

    def light(self, swatch, at, size=0.7, M: Matrix = IDENTITY):
        """Feu de position : petite sphère facettée émissive."""
        r = size / 2
        prof = [(0, -r), (r * 0.7, -r * 0.7), (r, 0), (r * 0.7, r * 0.7), (0, r)]
        return self.lathe("lights", prof, M @ T(*at), seg=8, swatch=swatch)

    def window(self, swatch, size, at, M: Matrix = IDENTITY, rot: Matrix = IDENTITY):
        return self.box("lights", size, at, M, rot=rot, swatch=swatch)

    # Sous-ensembles ---------------------------------------------------------------------------

    def turret(self, M: Matrix, house="hull", barrels=2, scale=1.0, length=8.0):
        """Tourelle de canons magnétiques (P26) : couronne, casemate chanfreinée, tubes à
        rails (frettes et bouche), capteur. Repère local : base en z = 0, tubes vers +Y."""
        s = scale
        self.lathe("dark", [(0, 0), (2.2 * s, 0), (2.2 * s, 0.35 * s), (1.9 * s, 0.7 * s), (0, 0.7 * s)], M @ Rot(90, "X"), seg=16)
        self.loft(house, [
            Sec(-2.1 * s, 1.7 * s, 0.75 * s, 0.75 * s, 0.45 * s, 0, 1.45 * s),
            Sec(1.0 * s, 1.9 * s, 0.85 * s, 0.75 * s, 0.5 * s, 0, 1.45 * s),
            Sec(2.1 * s, 1.4 * s, 0.35 * s, 0.6 * s, 0.3 * s, 0, 1.45 * s),
        ], M)
        self.box("dark", (2.6 * s, 0.5 * s, 1.0 * s), (0, 2.25 * s, 1.35 * s), M)  # masque
        self.box("dark", (0.5 * s, 0.9 * s, 0.5 * s), (1.65 * s, -0.4 * s, 2.1 * s), M)  # capteur
        self.window("window_dark", (0.36 * s, 0.1 * s, 0.3 * s), (1.65 * s, 0.07 * s, 2.1 * s), M)
        xs = [0.0] if barrels == 1 else [-0.6 * s, 0.6 * s]
        L = length * s
        r = 0.26 * s
        for x in xs:
            prof = [(0, 0), (r * 1.4, 0), (r * 1.4, 0.8 * s), (r, 0.8 * s), (r, L - 0.9 * s),
                    (r * 1.35, L - 0.9 * s), (r * 1.35, L), (r * 0.8, L), (0, L - 0.1 * s)]
            self.lathe("dark", prof, M @ T(x, 2.3 * s, 1.35 * s), seg=8, smooth=50.0)
            for k in range(3):  # frettes des rails
                self.band("bare", r * 1.25, 0, 0.25 * s, M @ T(x, 2.3 * s + (2.0 + k * 1.4) * s, 1.35 * s), seg=8)

    def launcher(self, M: Matrix, cols=4, rows=2, house="hull", depth=5.0, warning="warning_red"):
        """Lance-missiles en caisson : tubes en grille sur la face avant (+Y), trappes
        relevées, bande d'avertissement. Repère local : base en z = 0, centré en x."""
        pitch = 0.95
        w, h = cols * pitch + 0.5, rows * pitch + 0.5
        self.box(house, (w, depth, h), (0, 0, h / 2), M, bevel=0.12)
        self.box("dark", (w + 0.2, 0.35, h + 0.2), (0, depth / 2 - 0.1, h / 2), M)
        for i in range(cols):
            for j in range(rows):
                x = (i - (cols - 1) / 2) * pitch
                z = h / 2 + (j - (rows - 1) / 2) * pitch
                self.lathe("dark", [(0.3, 0.0), (0.4, 0.0), (0.4, 0.35), (0.3, 0.35)],
                           M @ T(x, depth / 2 + 0.05, z), seg=8, closed=True)
                self.box("bare", (0.62, 0.08, 0.62), (x, depth / 2 + 0.1, z), M)  # fond du tube (ogive)
        self.box("trim", (w + 0.06, depth * 0.35, 0.3), (0, -depth * 0.2, h + 0.02), M, trim=warning, mpt=2.0)

    def radiator(self, M: Matrix, length=8.0, height=2.6, fins=9, mat="bare"):
        """Panneau de radiateur : cadre et ailettes. Repère local : base en z = 0, le long de Y."""
        self.box("dark", (0.5, length, 0.4), (0, 0, 0.2), M)
        self.box("dark", (0.5, 0.3, height), (0, -length / 2, height / 2), M)
        self.box("dark", (0.5, 0.3, height), (0, length / 2, height / 2), M)
        self.box("dark", (0.5, length, 0.25), (0, 0, height), M)
        for k in range(fins):
            y = -length / 2 + (k + 1) * length / (fins + 1)
            self.box(mat, (0.9, 0.12, height - 0.4), (0, y, height / 2 + 0.05), M)

    def nozzle(self, M: Matrix, r_throat: float, r_exit: float, length: float, glow: str, core: str, wall=0.22,
               liner: str | None = None):
        """Tuyère en cloche (épaisse, fermée) vers −Y, disque incandescent au col. Repère local :
        y = 0 au col (contre la poupe)."""
        prof = [(r_throat + wall, 0.0), (r_exit + wall * 0.6, -length), (r_exit, -length),
                (r_throat * 1.02, -0.35), (r_throat * 0.98, 0.0)]
        self.lathe("dark", prof, M, seg=16, closed=True, smooth=30.0)
        self.lathe("bare", [(r_throat + wall, 0.25), (r_throat + wall + 0.14, 0.25), (r_throat + wall + 0.14, -0.35),
                            (r_throat + wall * 0.9, -0.35)], M, seg=16, closed=True)  # collier
        self.lathe("lights", [(0, -0.25), (r_throat * 1.0, -0.25), (r_throat, -0.15), (0, -0.15)], M, seg=16, swatch=glow)
        self.lathe("lights", [(0, -0.4), (r_throat * 0.55, -0.4), (r_throat * 0.55, -0.3), (0, -0.3)], M, seg=12, swatch=core)
        # Doublure incandescente contre la paroi intérieure : la cloche se lit allumée de trois
        # quarts arrière, même sans éclairage indirect (le jeu n'en a pas en vue Globale).
        liner = liner or ("liner_blue" if "blue" in glow else "liner_red")

        def wall_r(y: float) -> float:
            t = (-y - 0.35) / (length - 0.35)
            return r_throat * 1.02 + (r_exit - r_throat * 1.02) * t

        y0, y1 = -0.4, -length * 0.85
        prof = [(wall_r(y0) - 0.03, y0), (wall_r(y1) - 0.03, y1), (wall_r(y1) - 0.09, y1), (wall_r(y0) - 0.09, y0)]
        self.lathe("lights", prof, M, seg=16, closed=True, swatch=liner)

    def tank(self, M: Matrix, r: float, length: float, mat="bare", straps=3):
        """Réservoir couché le long de Y, fonds bombés, sangles."""
        h = length / 2
        prof = [(0, -h), (r * 0.55, -h + r * 0.12), (r * 0.9, -h + r * 0.45), (r, -h + r * 0.9),
                (r, h - r * 0.9), (r * 0.9, h - r * 0.45), (r * 0.55, h - r * 0.12), (0, h)]
        self.lathe(mat, prof, M, seg=14, smooth=40.0)
        for k in range(straps):
            y = -h + r + (k + 0.5) * (length - 2 * r) / straps
            self.band("dark", r + 0.08, y - 0.25, y + 0.25, M, seg=14)

    def mast(self, M: Matrix, height: float, light: str | None = "nav_white", arms=2):
        """Mât d'antennes : fût, vergues, feu au sommet. Repère local : pied en z = 0."""
        self.add(bm_cyl(0.16, height, 6), "bare", M @ T(0, 0, height / 2) @ Rot(90, "X"))
        self.box("dark", (0.6, 0.6, 0.5), (0, 0, 0.25), M)
        for k in range(arms):
            z = height * (0.55 + 0.3 * k)
            w = 2.4 - 0.8 * k
            self.box("dark", (w, 0.12, 0.12), (0, 0, z), M)
            for sx in (-1, 1):
                self.add(bm_cyl(0.05, 0.9, 4), "bare", M @ T(sx * w / 2, 0, z + 0.45) @ Rot(90, "X"))
        if light:
            self.light(light, (0, 0, height + 0.25), 0.6, M)

    def dish(self, M: Matrix, r: float):
        """Antenne parabolique : bol épais et cornet, vers +Y local, sur un pied."""
        prof = [(0.0, 0.0), (r * 0.5, r * 0.08), (r, r * 0.3), (r * 0.94, r * 0.36), (r * 0.46, r * 0.16), (0.0, 0.1 * r)]
        self.lathe("bare", prof, M, seg=12, closed=True, smooth=40.0)
        self.add(bm_cyl(0.06, r * 0.9, 4), "dark", M @ T(0, r * 0.45, 0))
        self.box("dark", (0.3, 0.3, 1.2), (0, -0.2, -0.6), M)


# --- Corvette pirate -----------------------------------------------------------------------------

def patches(s: Ship, secs: list[Sec], y0: float, y1: float, count: int, mats_w, skip=None, weld=0.5,
            faces=("side_l", "side_r", "top", "ch_tl", "ch_tr", "ch_bl", "ch_br")):
    """Plaques rapportées sur un fuseau (les surfaces planes de sa section) : tailles et
    teintes dépareillées, légèrement de travers, soudures sur un ou deux bords."""
    rng = s.rng
    mats = [m for m, _ in mats_w]
    weights = [w for _, w in mats_w]
    placed = 0
    tries = 0
    while placed < count and tries < count * 20:
        tries += 1
        face = rng.choice(faces)
        L = rng.uniform(2.2, 6.5)
        y = rng.uniform(y0 + L / 2, y1 - L / 2)
        sec = section_at(secs, y)
        a, b = section_at(secs, y - 0.5), section_at(secs, y + 0.5)
        d_hw, d_ht = b.hw - a.hw + (b.cx - a.cx), b.ht - a.ht + (b.cz - a.cz)
        if face.startswith("ch") and abs(b.hw - a.hw) > 0.01:
            continue
        flat_side = sec.ht + sec.hb - 2 * sec.ch
        flat_top = 2 * (sec.hw - sec.ch)
        chamfer_w = sec.ch * 1.414
        th = rng.uniform(0.12, 0.3)
        if face in ("side_l", "side_r"):
            sx = -1 if face == "side_l" else 1
            H = rng.uniform(1.2, min(3.4, flat_side - 0.2))
            zc = rng.uniform(sec.cz - sec.hb + sec.ch + H / 2, sec.cz + sec.ht - sec.ch - H / 2)
            pos = Vector((sec.cx + sx * (sec.hw + th / 2 - 0.05), y, zc))
            normal = Vector((sx, 0, 0))
            base = Rot(-math.degrees(math.atan(sx * d_hw)), "Z")
            size = (th, L, H)
        elif face == "top":
            W = rng.uniform(1.4, min(3.6, flat_top - 0.2))
            xc = rng.uniform(sec.cx - sec.hw + sec.ch + W / 2, sec.cx + sec.hw - sec.ch - W / 2)
            pos = Vector((xc, y, sec.cz + sec.ht + th / 2 - 0.05))
            normal = Vector((0, 0, 1))
            base = Rot(math.degrees(math.atan(d_ht)), "X")
            size = (W, L, th)
        else:
            sx = -1 if face.endswith("l") else 1
            sz = 1 if face[3] == "t" else -1
            W = chamfer_w * rng.uniform(0.7, 0.95)
            d = sec.ch / 2
            zedge = sec.cz + sz * (sec.ht if sz > 0 else sec.hb)
            pos = Vector((sec.cx + sx * (sec.hw - d), y, zedge - sz * d))
            normal = Vector((sx, 0, sz)).normalized()
            pos += normal * (th / 2 - 0.05)
            base = Rot(sx * sz * 45, "Y")  # épaisseur (Z local) le long de la normale
            size = (W, L, th)
        if skip and skip(pos):
            continue
        tilt = Rot(rng.uniform(-5, 5), normal)
        mat = rng.choices(mats, weights)[0]
        M = T(*pos) @ tilt @ base
        s.box(mat, size, (0, 0, 0), M)
        placed += 1
        if rng.random() < weld:
            # Soudure : un cordon le long d'un bord long (métal brut ou sombre).
            along_top = face not in ("side_l", "side_r")
            if along_top:
                off = (size[0] / 2) * rng.choice((-1, 1))
                s.box(rng.choice(("bare", "dark")), (0.16, L * rng.uniform(0.6, 1.0), th + 0.08), (off, 0, 0), M)
            else:
                off = (size[2] / 2) * rng.choice((-1, 1))
                s.box(rng.choice(("bare", "dark")), (th + 0.08, L * rng.uniform(0.6, 1.0), 0.16), (0, 0, off), M)


# Sections dépareillées : poupe d'un cargo (octogonale), travée à nu, milieu plus large et plus
# plat (une autre coque), proue rapportée désaxée.
P_AFT = [Sec(-26.5, 6.0, 4.2, 4.4, 2.4), Sec(-15.0, 6.4, 4.4, 4.6, 2.6)]
P_CORE = [Sec(-15.6, 3.6, 2.8, 3.0, 1.0), Sec(-8.0, 3.6, 2.8, 3.0, 1.0)]
P_MID = [
    Sec(-8.6, 6.6, 3.9, 4.8, 1.5, 0.3, 0.4),
    Sec(10.0, 6.6, 3.9, 4.8, 1.5, 0.3, 0.4),
    Sec(15.0, 5.6, 3.6, 4.3, 1.3, 0.3, 0.2),
    Sec(22.5, 4.4, 3.3, 3.6, 1.1, 0.4, -0.1),
]
P_PROW = [
    Sec(21.5, 4.9, 3.6, 3.9, 1.3, 0.7, -0.2),
    Sec(28.0, 3.8, 2.8, 3.3, 1.0, 0.7, -0.4),
    Sec(33.5, 1.9, 1.5, 2.4, 0.6, 0.7, -0.6),
]
P_ENGINE = [Sec(-33.0, 8.4, 5.6, 5.0, 2.2, -0.4), Sec(-25.5, 8.0, 5.4, 4.8, 2.0, -0.4)]


def top_z(secs: list[Sec], y: float) -> float:
    sec = section_at(secs, y)
    return sec.cz + sec.ht


def frame_ring(s: Ship, sec: Sec, depth: float, beam: float, mat: str = "dark") -> None:
    """Membrure à nu : huit poutres le long du contour d'une section (travée sans bordé)."""
    pts = ring_points(sec)
    for (x0, z0), (x1, z1) in zip(pts, pts[1:] + pts[:1]):
        dx, dz = x1 - x0, z1 - z0
        length = math.hypot(dx, dz) + beam * 0.8
        ang = math.degrees(math.atan2(-dz, dx))
        s.box(mat, (length, depth, beam), ((x0 + x1) / 2, sec.y, (z0 + z1) / 2), rot=Rot(ang, "Y"))


def pirate_corvette(mats, coll) -> list[bpy.types.Object]:
    """Corvette pirate, ≈ 70 m : la poupe et le milieu de deux cargos différents reliés par une
    travée restée à nu, une proue d'une troisième coque, un moteur d'appoint sur un bras de
    bâbord, une soute blindée rapportée à tribord, des plaques rapiécées partout."""
    s = Ship("pc", mats, coll, seed=1726)
    s.loft("hull", P_AFT)
    s.loft("hull", P_MID)
    s.loft("accent", P_PROW)
    s.loft("hull", P_ENGINE)
    # Raccords : colliers sombres et cordons de soudure en métal brut.
    s.loft("dark", shell(P_ENGINE, -33.4, -32.6, 0.25))
    s.loft("dark", shell(P_AFT, -26.0, -24.8, 0.2))
    s.loft("dark", shell(P_MID, -8.6, -7.6, 0.22))
    s.loft("dark", shell(P_MID, 20.8, 22.2, 0.22))
    for y in (21.0, 22.0):
        s.loft("bare", shell(P_MID, y - 0.1, y + 0.1, 0.34))
    s.box("dark", (15.0, 0.4, 9.0), (-0.4, -33.2, 0.3))  # plaque de poupe
    for y, w in ((-21.0, 1.0), (-17.5, 0.7)):
        s.loft("dark", shell(P_AFT, y - w / 2, y + w / 2, 0.18))
    for y, w in ((-2.0, 0.8), (4.6, 1.3), (12.5, 0.7)):
        s.loft("dark", shell(P_MID, y - w / 2, y + w / 2, 0.2))

    # Travée à nu entre la poupe et le milieu : âme sombre, membrures, lisses, câbles et tuyaux.
    s.loft("dark", P_CORE)
    outer = Sec(0, 6.3, 4.3, 4.6, 2.3, 0.1, 0.2)
    for y in (-14.4, -11.8, -9.2):
        frame_ring(s, outer._replace(y=y), 0.6, 0.45)
    for x, z in ring_points(outer)[::2] + [(-6.3, 0.2), (6.4, -1.2)]:
        s.box("bare", (0.3, 6.4, 0.3), (x, -11.8, z))
    for (x, z, r, band) in ((4.3, 1.4, 0.5, "cables"), (-4.4, 0.9, 0.42, "flange"), (2.4, 3.4, 0.36, "flange"),
                            (-2.2, -3.7, 0.45, "cables"), (-4.0, 2.6, 0.3, "cables")):
        s.add(bm_cyl(r, 7.6, 10), "trim", T(x, -11.8, z), trim=band, mpt=2.0)
    s.box("accent", (0.25, 3.0, 3.4), (-6.45, -12.6, -1.4), rot=Rot(4, "X"))  # tôle à moitié posée

    # Éperon sous la proue, joues de blindage soudées (dépareillées), bandes d'avertissement.
    s.loft("dark", [Sec(26, 1.6, 0.7, 0.7, 0.3, 0.7, -3.2), Sec(35.0, 0.35, 0.25, 0.3, 0.1, 0.7, -2.9)])
    s.box("dark", (0.5, 7.0, 3.2), (-3.4, 26.0, -0.6), bevel=0.1, rot=Rot(-11, "Z"))
    s.box("accent", (0.5, 5.0, 2.4), (4.6, 24.6, 0.4), bevel=0.1, rot=Rot(10, "Z") @ Rot(6, "Y"))
    s.box("trim", (0.12, 4.0, 0.6), (4.95, 24.8, -1.3), rot=Rot(10, "Z"), trim="warning_red", mpt=2.0)
    s.box("trim", (0.12, 3.5, 0.6), (-3.75, 26.3, -2.6), rot=Rot(-11, "Z"), trim="warning_red", mpt=2.0)

    # Plaques rapiécées (pas sur la soute de tribord ni sous la passerelle).
    def occupied(p: Vector) -> bool:
        if p.x > 6.0 and -6.5 < p.y < 8.5 and -3.0 < p.z < 2.6:
            return True
        return p.z > 3.8 and -5.8 < p.x < 1.0 and 3.0 < p.y < 13.5

    patches(s, P_AFT, -25.0, -15.5, 14, [("accent", 0.45), ("hull", 0.25), ("dark", 0.3)], skip=occupied)
    patches(s, P_MID, -7.5, 20.0, 24, [("accent", 0.4), ("hull", 0.35), ("dark", 0.25)], skip=occupied)
    patches(s, P_PROW, 22.5, 31.0, 6, [("hull", 0.5), ("dark", 0.5)], faces=("side_l", "side_r", "top"))
    patches(s, P_ENGINE, -32.5, -26.0, 5, [("accent", 0.6), ("dark", 0.4)], faces=("side_l", "side_r", "top", "ch_tl", "ch_tr"))

    # Tuyères dépareillées : une grosse cloche, une droite, une petite ; carter de pompe rajouté.
    back = -33.4
    s.nozzle(T(3.2, back, 0.7), 2.0, 3.0, 4.2, "engine_red", "core_hot")
    s.nozzle(T(-4.2, back, 1.6), 1.6, 1.9, 3.2, "engine_red", "core_hot", wall=0.3)
    s.nozzle(T(-2.4, back, -3.0), 1.1, 1.6, 2.8, "engine_red", "core_hot")
    s.box("bare", (2.4, 1.6, 1.0), (6.2, -33.8, -2.8), bevel=0.1)
    s.box("dark", (3.0, 4.0, 0.7), (-3.0, -29.0, 5.6), bevel=0.1)
    s.box("trim", (2.8, 3.6, 0.06), (-3.0, -29.0, 5.97), trim="vent", mpt=2.0)
    s.light("nav_white", (0.6, -33.6, 5.4), 0.6)

    # Bras de bâbord : treillis et moteur d'appoint récupéré, tourelle légère dessus.
    pod_x, pod_z = -11.6, -0.6
    s.box("dark", (5.0, 1.0, 0.9), (-8.6, -20.0, pod_z + 0.3), bevel=0.08)
    s.box("dark", (5.0, 1.0, 0.9), (-8.8, -4.5, pod_z + 0.3), bevel=0.08)
    s.box("dark", (0.5, 17.0, 0.5), (-8.4, -12.2, pod_z - 0.6), rot=Rot(-15, "Z"))
    pod = [(0, -23.0), (1.7, -23.0), (2.3, -21.5), (2.3, -7.0), (2.0, -4.4), (1.2, -2.6), (0, -2.0)]
    s.lathe("accent", pod, T(pod_x, 0, pod_z), seg=14, smooth=35.0)
    for y in (-19.0, -13.0, -6.2):
        s.band("dark", 2.42, y - 0.35, y + 0.35, T(pod_x, 0, pod_z), seg=14)
    s.nozzle(T(pod_x, -23.0, pod_z), 1.4, 1.9, 2.6, "engine_red", "core_hot")
    s.turret(T(pod_x, -12.0, pod_z + 2.25), house="hull", barrels=1, scale=0.7, length=7.0)
    s.light("nav_red", (pod_x - 2.2, -4.6, pod_z + 0.9), 0.8)
    s.light("nav_red", (pod_x, -1.9, pod_z), 0.5)
    s.add(bm_cyl(0.3, 2.2, 6), "bare", T(-14.2, -14.0, pod_z) @ Rot(90, "Z"))  # stabilisateur
    s.light("beacon_amber", (-15.4, -14.0, pod_z), 0.6)

    # Soute blindée rapportée à tribord : sangles, tôles rouges, pochoir, feu vert.
    s.box("hull", (5.0, 14.0, 5.2), (9.6, 1.0, -0.2), bevel=0.35)
    s.box("accent", (0.3, 5.0, 4.0), (12.2, -2.5, -0.1), rot=Rot(3, "X"))
    s.box("accent", (0.3, 3.4, 2.6), (12.2, 5.0, 0.6), rot=Rot(-4, "X"))
    for y in (-4.5, 1.2, 6.6):
        s.box("dark", (5.4, 0.7, 5.6), (9.6, y, -0.2))
    s.box("trim", (0.1, 3.0, 0.5), (12.16, 2.0, 1.7), trim="stencils", mpt=3.0)
    s.box("trim", (4.4, 0.2, 0.6), (9.6, 8.05, 1.7), trim="hazard", mpt=2.0)
    s.light("nav_green", (12.3, 7.6, 2.2), 0.8)
    # Tuyauterie le long du flanc de bâbord, brides.
    for z, r in ((1.9, 0.35), (1.1, 0.28)):
        s.add(bm_cyl(r, 17.0, 10), "trim", T(0.3 - 6.6 - r + 0.05, 1.0, z), trim="flange", mpt=2.0)

    # Réservoirs ventraux sanglés (dépareillés), visibles aussi dans la travée à nu.
    s.tank(T(3.6, -12.0, -5.2), 1.6, 13.0, "bare")
    s.tank(T(0.2, -13.5, -5.5), 1.25, 9.0, "accent", straps=2)
    s.tank(T(-3.4, -10.5, -5.0), 1.1, 7.0, "hull", straps=2)
    for y in (-17.0, -6.5):
        s.box("dark", (9.5, 0.5, 1.2), (0.2, y, -4.6))

    # Passerelle de fortune décalée à bâbord : verrière ambrée, hublots, mât, parabole.
    bridge = [Sec(4.0, 2.9, 1.8, 1.6, 0.6, -2.4, 5.9), Sec(10.5, 2.9, 1.8, 1.6, 0.6, -2.4, 5.9),
              Sec(13.0, 2.2, 0.8, 1.5, 0.4, -2.4, 5.9)]
    s.loft("hull", bridge)
    s.loft("dark", shell(bridge, 4.0, 4.6, 0.15))
    slope = math.degrees(math.atan2(1.0, 2.5))
    for k in range(5):
        x = -2.4 + (k - 2) * 0.78
        s.window("window_warm", (0.64, 0.62, 0.12), (x, 11.75, 7.22), rot=Rot(-slope, "X"))
    s.box("dark", (4.6, 0.35, 0.3), (-2.4, 12.6, 6.75))
    for sx in (-1, 1):
        for k in range(3):
            s.window("window_warm", (0.12, 0.7, 0.5), (-2.4 + sx * 2.92, 5.8 + k * 1.6, 6.3))
    s.box("trim", (0.1, 3.0, 0.5), (-5.36, 8.0, 5.0), trim="stencils", mpt=3.0)
    s.mast(T(-4.0, 5.0, 7.7), 4.5, "nav_white")
    s.box("dark", (1.2, 1.4, 0.8), (-1.0, 5.6, 8.1))
    s.dish(T(-1.0, 5.8, 9.3) @ Rot(-35, "Z") @ Rot(25, "X"), 1.5)

    # Tourelle lourde (deux tubes) sur le cou, tourelle ventrale, lance-missiles boulonné.
    zt = top_z(P_MID, 17.5)
    s.lathe("dark", [(0, 0), (2.6, 0), (2.6, 0.6), (0, 0.6)], T(0.6, 17.5, zt - 0.4) @ Rot(90, "X"), seg=12)
    s.turret(T(0.6, 17.5, zt + 0.2), house="accent", barrels=2, scale=1.0, length=9.0)
    s.turret(T(-0.6, 3.0, 0.4 - 4.8 + 0.05) @ Rot(180, "Y"), house="hull", barrels=2, scale=0.75, length=7.0)
    s.launcher(T(2.4, -20.5, top_z(P_AFT, -20.5) - 0.1) @ Rot(8, "X"), cols=3, rows=2, house="accent", depth=5.0)
    for y in (-22.5, -18.5):
        s.box("dark", (4.2, 0.4, 0.5), (2.4, y, top_z(P_AFT, y) - 0.05))
    s.radiator(T(-2.6, -21.0, top_z(P_AFT, -21.0) - 0.05), length=6.5, height=2.2, fins=8)
    s.radiator(T(-0.6, -1.5, 4.25) @ Rot(-9, "Y"), length=6.0, height=2.0, fins=6)  # tordu
    s.box("dark", (2.6, 3.0, 0.8), (-3.4, -6.0, 4.6), bevel=0.1)
    s.box("trim", (2.4, 2.6, 0.06), (-3.4, -6.0, 5.02), trim="vent", mpt=2.0)
    # Gros réservoir de récupération couché sur le dos, à tribord, sur des berceaux.
    s.tank(T(3.2, -1.0, 6.3), 2.0, 11.0, "bare", straps=3)
    for y in (-4.5, 2.5):
        s.box("dark", (3.6, 0.8, 1.6), (3.2, y, 4.9))
    s.box("trim", (0.08, 3.0, 0.5), (5.25, -1.0, 6.3), trim="hazard", mpt=2.0)

    # Hublots irréguliers (certains éteints) et feux.
    for secs, y0, y1, z in ((P_AFT, -25.0, -15.5, 1.2), (P_MID, -7.0, 20.0, 2.2)):
        for sx in (-1, 1):
            y = y0 + s.rng.uniform(0.0, 1.5)
            while y < y1:
                sec = section_at(secs, y)
                if not (sx > 0 and -6.5 < y < 8.5):
                    sw = "window_warm" if s.rng.random() < 0.65 else "window_dark"
                    s.window(sw, (0.12, 0.7, 0.55), (sec.cx + sx * (sec.hw + 0.02), y, z))
                y += s.rng.choice((2.0, 2.5, 3.0, 4.5))
    s.light("beacon_amber", (-1.0, -3.0, -4.75), 0.6)
    s.light("nav_white", (0.7, 33.0, 0.9), 0.5)
    return s.parts


# --- Corvette de l'Union -------------------------------------------------------------------------

# Section à quille : chanfreins hauts de 2,4 m, bas de 3,6 m (un V sous la coque).
U_HULL = [
    Sec(-36, 8.0, 5.4, 5.0, 2.2, cb=3.3),
    Sec(-30, 8.6, 5.8, 5.4, 2.4, cb=3.6),
    Sec(22, 8.6, 5.8, 5.4, 2.4, cb=3.6),
    Sec(34, 6.6, 4.8, 4.6, 1.9, cb=3.0),
    Sec(42, 3.6, 3.2, 3.6, 1.2, cb=2.0),
    Sec(44.6, 2.2, 2.3, 2.9, 0.8, cb=1.4),
]


def ugl_corvette(mats, coll) -> list[bpy.types.Object]:
    """Corvette de l'Union, ≈ 90 m : coque à pans coupés, nervures régulières, pont dorsal à
    trois tourelles, superstructure centrale, caissons lance-missiles symétriques."""
    s = Ship("uc", mats, coll, seed=4090)
    s.loft("hull", U_HULL)
    s.lathe("dark", [(0, 44.4), (1.6, 44.4), (1.4, 45.4), (0.6, 45.9), (0, 46.0)], T(0, 0, -0.3), seg=12, smooth=40.0)
    # Nervures tous les 6 m, plaques de blindage régulières entre elles (flancs, dessus, chanfreins).
    ribs = [-30 + 6 * k for k in range(9)]  # -30 … 18
    for y in ribs:
        s.loft("dark", shell(U_HULL, y - 0.35, y + 0.35, 0.22))
    for y0, y1 in zip(ribs, ribs[1:]):
        L = (y1 - y0) - 1.3
        yc = (y0 + y1) / 2
        sec = section_at(U_HULL, yc)
        for sx in (-1, 1):
            s.box("hull", (0.16, L, 1.5), (sx * (sec.hw + 0.06), yc, 1.6))
            s.box("hull", (0.16, L, 1.9), (sx * (sec.hw + 0.06), yc, -0.65))
            # chanfreins haut et bas : plaques inclinées à 45°
            for sz, ch, zc in ((1, sec.ch, sec.ht), (-1, sec.cb, -sec.hb)):
                d = ch / 2
                p = Vector((sx * (sec.hw - d), yc, zc - sz * d)) + Vector((sx, 0, sz)).normalized() * 0.06
                s.box("hull", (ch * 1.2, L, 0.16), tuple(p), rot=Rot(sx * sz * 45, "Y"))
        for sx in (-1, 1):  # dessus, de part et d'autre du pont
            s.box("hull", (1.8, L, 0.16), (sx * 4.6, yc, sec.ht + 0.06))
    # Bande bleu pâle (P6) à l'avant, et liserés.
    s.loft("accent", shell(U_HULL, 26.6, 31.4, 0.14))
    for y in (26.2, 31.8):
        s.loft("dark", shell(U_HULL, y - 0.2, y + 0.2, 0.2))
    # Pochoirs d'immatriculation génériques (trim sheet) sur les joues avant.
    taper = math.degrees(math.atan((6.6 - 3.6) / 8.0))
    sec = section_at(U_HULL, 36.5)
    for sx in (-1, 1):
        s.box("trim", (0.1, 4.0, 0.65), (sx * (sec.hw + 0.04), 36.5, 1.0), rot=Rot(sx * taper, "Z"), trim="stencils", mpt=4.0)
    # Préceintes : longerons sombres à l'arête entre flanc et quille.
    for sx in (-1, 1):
        s.box("dark", (0.45, 52.0, 0.45), (sx * 8.66, -4.0, -1.85))

    # Bloc moteur : quatre tuyères en carré, plaque de poupe, carénages.
    eng = [Sec(-42.6, 9.0, 6.0, 5.6, 2.6, cb=3.4), Sec(-35.0, 9.0, 6.0, 5.6, 2.6, cb=3.4)]
    s.loft("hull", eng)
    for y in (-41.8, -36.2):
        s.loft("dark", shell(eng, y - 0.3, y + 0.3, 0.2))
    s.loft("accent", shell(eng, -39.6, -38.4, 0.1))
    s.box("dark", (16.0, 0.4, 10.0), (0, -42.8, 0.2))
    for x in (-4.3, 4.3):
        for z in (-2.5, 2.7):
            s.nozzle(T(x, -43.0, z), 1.55, 2.3, 3.0, "engine_blue", "core_blue")
    for sx in (-1, 1):  # tuyères de manœuvre
        s.nozzle(T(sx * 6.6, -43.0, -3.4), 0.5, 0.75, 1.1, "engine_blue", "core_blue")
        s.radiator(T(sx * 9.0, -39.0, -1.5) @ Rot(sx * 90, "Y"), length=6.0, height=2.6, fins=8)

    # Pont dorsal et superstructure.
    deck = [Sec(-31, 3.6, 1.4, 0.3, 0.8, 0, 5.8), Sec(20, 3.6, 1.4, 0.3, 0.8, 0, 5.8), Sec(24, 2.6, 0.6, 0.3, 0.4, 0, 5.8)]
    s.loft("hull", deck)
    for y in (-24.0, -12.0, 0.0, 12.0):
        s.loft("dark", shell(deck, y - 0.3, y + 0.3, 0.12))
    tower = [Sec(-7.0, 3.2, 2.4, 0.2, 0.9, 0, 9.4), Sec(3.0, 3.2, 2.4, 0.2, 0.9, 0, 9.4), Sec(5.6, 2.6, 0.8, 0.2, 0.6, 0, 9.4)]
    s.loft("hull", [Sec(-8.0, 3.8, 2.2, 0.0, 0.9, 0, 7.2), Sec(5.0, 3.8, 2.2, 0.0, 0.9, 0, 7.2), Sec(6.6, 3.2, 1.0, 0.0, 0.6, 0, 7.2)])
    s.loft("hull", tower)
    s.loft("accent", shell(tower, -6.4, -4.4, 0.08))
    s.loft("dark", shell(tower, -7.0, -6.6, 0.12))
    # Verrière : vitres sur la face inclinée avant, et rangées latérales.
    slope = math.degrees(math.atan2(1.6, 2.6))
    for k in range(5):
        x = (k - 2) * 0.78
        s.window("window_cool", (0.66, 0.75, 0.12), (x, 4.3, 11.05), rot=Rot(-slope, "X"))
    s.box("dark", (5.4, 0.3, 0.3), (0, 5.55, 10.15))
    for sx in (-1, 1):
        for k in range(5):
            s.window("window_cool", (0.12, 0.8, 0.45), (sx * 3.22, -5.2 + k * 1.7, 10.4))
        for k in range(7):
            s.window("window_cool", (0.12, 0.7, 0.4), (sx * 3.82, -6.6 + k * 1.75, 8.1))
    s.mast(T(0, -3.5, 11.8), 5.0, "nav_white", arms=2)
    s.dish(T(0, -0.4, 12.9) @ Rot(35, "X"), 1.3)
    s.box("dark", (2.0, 2.0, 0.8), (0, -0.4, 12.0))
    for sx in (-1, 1):
        s.box("dark", (1.0, 2.4, 0.9), (sx * 2.4, -4.0, 12.2), bevel=0.1)

    # Trois tourelles lourdes : avant (sur la coque), surélevée (sur barbette), arrière.
    s.turret(T(0, 30.5, 5.15), house="hull", barrels=2, scale=1.0, length=9.0)
    s.lathe("dark", [(0, 0), (2.7, 0), (2.7, 1.7), (0, 1.7)], T(0, 17.0, 6.2) @ Rot(90, "X"), seg=16)
    s.turret(T(0, 17.0, 7.9), house="hull", barrels=2, scale=1.0, length=9.0)
    s.turret(T(0, -24.5, 7.2) @ Rot(180, "Z"), house="hull", barrels=2, scale=0.9, length=8.0)
    # Défense rapprochée : tourelles légères ventrales, symétriques.
    for sx in (-1, 1):
        for y in (10.0, -20.0):
            s.turret(T(sx * 3.4, y, -5.35) @ Rot(180, "Y"), house="hull", barrels=1, scale=0.55, length=6.0)
    # Caissons lance-missiles latéraux (4 × 2), sur des consoles.
    for sx in (-1, 1):
        s.box("hull", (2.4, 12.0, 3.2), (sx * 9.6, 7.0, 0.2), bevel=0.2)
        s.box("dark", (2.6, 0.5, 3.4), (sx * 9.6, 1.2, 0.2))
        s.launcher(T(sx * 10.0, 8.0, 1.8), cols=4, rows=2, house="hull", depth=6.0, warning="hazard")
        s.light("nav_red" if sx < 0 else "nav_green", (sx * 12.4, 12.6, 2.5), 0.8)
        s.box("trim", (0.1, 3.0, 0.4), (sx * 10.85, 4.0, 0.9), trim="hazard", mpt=2.0)
    # Radiateurs dorsaux par paires, sas latéraux, réservoirs encastrés.
    for sx in (-1, 1):
        s.radiator(T(sx * 5.7, -15.5, 5.8), length=9.0, height=2.4, fins=10)
        s.box("dark", (0.4, 3.0, 3.0), (sx * 8.75, -21.0, 0.4), bevel=0.1)
        s.box("trim", (0.08, 3.2, 0.25), (sx * 8.98, -21.0, 2.0), trim="gasket", mpt=2.0)
        s.box("trim", (0.08, 2.0, 0.3), (sx * 8.98, -21.0, -1.6), trim="chevrons", mpt=2.0)
        s.tank(T(sx * 2.6, -5.0, -5.8), 1.2, 12.0, "bare", straps=3)
    # Hublots en rangées régulières (deux par travée), feux de formation, feux de poupe.
    for sx in (-1, 1):
        for y0, y1 in zip(ribs, ribs[1:]):
            for f in (0.33, 0.67):
                y = y0 + (y1 - y0) * f
                if 0.5 < y < 13.5:
                    continue
                s.window("window_cool", (0.14, 0.75, 0.4), (sx * 8.63, y, 2.72))
        for y in (-28.0, -10.0, 8.0, 26.0):
            sec = section_at(U_HULL, y)
            s.window("strip_cool", (0.25, 1.6, 0.25), (sx * (sec.hw - sec.ch / 2 + 0.1), y, sec.ht - sec.ch / 2 + 0.1),
                     rot=Rot(sx * 45, "Y"))
    s.light("nav_white", (0, -43.2, 6.4), 0.7)
    s.light("nav_white", (0, 44.0, 2.4), 0.5)
    s.light("beacon_amber", (0, -2.0, -5.7), 0.6)
    return s.parts


# --- Aperçus -------------------------------------------------------------------------------------


def setup_preview_scene() -> None:
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 24
    scene.cycles.use_denoising = True
    scene.cycles.use_adaptive_sampling = True
    scene.cycles.max_bounces = 4
    scene.render.threads_mode = "FIXED"
    scene.render.threads = 4
    scene.render.resolution_x = 1280
    scene.render.resolution_y = 720
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    for name in ("AgX", "Filmic", "Standard"):
        try:
            scene.view_settings.view_transform = name
            break
        except TypeError:
            continue
    # Ciel noir étoilé simple : points de Voronoï sur la direction de vue, plus un fond très faible.
    world = bpy.data.worlds.new("sh_space")
    scene.world = world
    world.use_nodes = True
    nt = world.node_tree
    bg = next(n for n in nt.nodes if n.type == "BACKGROUND")
    coord = nt.nodes.new("ShaderNodeTexCoord")
    vor = nt.nodes.new("ShaderNodeTexVoronoi")
    vor.inputs["Scale"].default_value = 220.0
    less = nt.nodes.new("ShaderNodeMath")
    less.operation = "LESS_THAN"
    less.inputs[1].default_value = 0.035
    mul = nt.nodes.new("ShaderNodeMath")
    mul.operation = "MULTIPLY_ADD"
    mul.inputs[1].default_value = 1.2
    mul.inputs[2].default_value = 0.012
    nt.links.new(coord.outputs["Generated"], vor.inputs["Vector"])
    nt.links.new(vor.outputs["Distance"], less.inputs[0])
    nt.links.new(less.outputs[0], mul.inputs[0])
    nt.links.new(mul.outputs[0], bg.inputs["Strength"])
    bg.inputs["Color"].default_value = (0.8, 0.85, 1.0, 1.0)
    # Soleil dur, de l'avant, d'en haut, de tribord.
    sun_data = bpy.data.lights.new("sh_sun", "SUN")
    sun_data.energy = 4.5
    sun_data.angle = math.radians(0.4)
    sun = bpy.data.objects.new("sh_sun", sun_data)
    scene.collection.objects.link(sun)


def look_at(obj: bpy.types.Object, back: Vector, up: Vector) -> None:
    """Oriente une caméra ou une lampe : son −Z vers −`back`, son Y du côté de `up`."""
    z = back.normalized()
    x = up.cross(z).normalized()
    y = z.cross(x)
    obj.rotation_euler = Matrix((x, y, z)).transposed().to_euler()


def render_views(name: str, objects: list[bpy.types.Object], length: float) -> list[Path]:
    scene = bpy.context.scene
    for obj in scene.objects:
        if obj.type == "MESH":
            obj.hide_render = obj not in objects
    data = bpy.data.cameras.new(f"cam_{name}")
    data.clip_start = 0.5
    data.clip_end = 8000.0
    cam = bpy.data.objects.new(f"cam_{name}", data)
    scene.collection.objects.link(cam)
    scene.camera = cam
    L = length
    sun = bpy.data.objects["sh_sun"]
    zup, xup = Vector((0, 0, 1)), Vector((-1, 0, 0))
    # vue : (direction de la caméra depuis la cible, cible, distance, focale, haut de l'image)
    views = {
        "trois-quarts-avant": ((0.9, 1.0, 0.55), (0, 0, 0), 1.25 * L, 35, zup),
        "trois-quarts-arriere": ((-0.95, -1.0, 0.45), (0, -0.05 * L, 0), 1.25 * L, 35, zup),
        "profil": ((1.0, 0.0, 0.08), (0, 0, 0), 2.6 * L, 70, zup),
        "dessus": ((0.0, 0.0, 1.0), (0, 0, 0), 2.6 * L, 70, xup),
        "pres": ((0.75, 0.55, 0.45), (0, 0.18 * L, 4.0), 60.0, 50, zup),
        "loin": ((0.8, 0.9, 0.4), (0, 0, 0), 1500.0, 40, zup),
    }
    out = []
    for view, (direction, target, dist, lens, up) in views.items():
        d = Vector(direction).normalized()
        target = Vector(target)
        cam.location = target + d * dist
        look_at(cam, d, up)
        data.lens = lens
        # Soleil dur, à 40° de l'axe de la caméra, d'en haut : le côté vu est éclairé.
        light = Matrix.Rotation(math.radians(40), 3, "Z") @ d + Vector((0.25, 0.3, 0.6))
        look_at(sun, light, zup if abs(light.normalized().z) < 0.95 else xup)
        path = PREVIEW_DIR / f"{name}-{view}.png"
        scene.render.filepath = str(path)
        bpy.ops.render.render(write_still=True)
        out.append(path)
        print(f"aperçu : {path}")
        if view == "loin":
            out.append(zoom_crop(path, 4))
    return out


def zoom_crop(path: Path, factor: int) -> Path:
    """Centre de l'image agrandi `factor` fois sans filtrage (pixels visibles) : ce que la
    coque donne réellement à cette distance."""
    import numpy as np

    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "materials"))
    import texgen as tg

    img = bpy.data.images.load(str(path))
    w, h = img.size
    px = np.array(img.pixels[:], dtype=np.float32).reshape(h, w, 4)[..., :3]
    cw, ch = w // factor, h // factor
    crop = px[(h - ch) // 2:(h + ch) // 2, (w - cw) // 2:(w + cw) // 2]
    big = np.repeat(np.repeat(crop, factor, axis=0), factor, axis=1)
    out = path.with_name(path.stem + "-zoom.png")
    tg.write_png(out, big)
    bpy.data.images.remove(img)
    print(f"aperçu : {out}")
    return out


# --- Programme -----------------------------------------------------------------------------------


def inverted_parts(parts) -> list[str]:
    """Pièces fermées dont le volume signé est négatif (normales vers l'intérieur) : le jeu
    masque les faces arrière (`doubleSided: false`), une telle pièce y serait invisible ou
    creuse. Les bagues ouvertes sont orientées à la construction et ne sont pas testées."""
    bad = []
    for obj in parts:
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        if all(len(e.link_faces) == 2 for e in bm.edges):
            bmesh.ops.triangulate(bm, faces=bm.faces)
            vol = sum(f.verts[0].co.dot(f.verts[1].co.cross(f.verts[2].co)) for f in bm.faces) / 6.0
            if vol < 0.0:
                bad.append(obj.name)
        bm.free()
    return bad


def stats(objects) -> tuple[int, int]:
    tris = sum(len(p.vertices) - 2 for o in objects for p in o.data.polygons)
    mats = {o.data.materials[0].name for o in objects}
    return tris, len(mats)


def main(argv: list[str]) -> None:
    previews = "--previews" in argv
    c.reset_scene()
    write_light_sheet()
    write_rust_texture()
    mats = palette()
    exports = c.collection("EXPORT")
    assets = {
        "pirate_corvette": (pirate_corvette, mats["pirate"], 70.0),
        "ugl_corvette": (ugl_corvette, mats["ugl"], 90.0),
    }
    merged_by_name = {}
    quiet = io.StringIO()
    for name, (builder, m, _) in assets.items():
        coll = c.collection(f"ASSET_{name}")
        parts = builder(m, coll)
        bad = inverted_parts(parts)
        if bad:
            raise RuntimeError(f"{name} : normales inversées sur {len(bad)} pièces : {bad[:8]}")
        with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
            merged = c.join_by_material(parts, name, exports)
            c.export_gltf(f"{OUT}/{name}.gltf", merged)
        for obj in parts:
            obj.hide_render = True
            obj.hide_set(True)
        tris, n_mats = stats(merged)
        dims = [max(v[i] for o in merged for v in o.bound_box) for i in range(3)]
        mins = [min(v[i] for o in merged for v in o.bound_box) for i in range(3)]
        print(f"{name} : {len(parts)} pièces → {len(merged)} maillages, {n_mats} matériaux, {tris} triangles ; "
              f"x {mins[0]:.1f}…{dims[0]:.1f}, y {mins[1]:.1f}…{dims[1]:.1f}, z {mins[2]:.1f}…{dims[2]:.1f}")
        merged_by_name[name] = merged
    c.save_blend("art/space/warships.blend")
    if previews:
        PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
        setup_preview_scene()
        for name, (_, _, length) in assets.items():
            render_views(name, merged_by_name[name], length)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
