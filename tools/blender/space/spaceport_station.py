"""Spatioport orbital par défaut (proposition P31, lot S de M10) : modèle extérieur, vu en vue Globale.

Une station orbitale à hangars de ≈ 600 m, industrielle et neutre (aucun insigne, aucun texte
lisible), modèle par défaut de tout lieu d'amarrage sans modèle propre. Le joueur s'amarre dans
la grande bouche de hangar, à l'avant (+Y Blender, −Z dans le jeu), et se retrouve dans le
hangar intérieur du secteur `spaceport`.

Silhouette, de l'avant vers l'arrière :

- **tête des hangars** : coque octogonale massive, percée à l'avant d'une bouche de 130 × 50 m
  éclairée de l'intérieur (sol à aires d'appontage, fermes de plafond et projecteurs, galerie
  de contrôle au fond, deux navettes posées), porte blindée relevée, cadre à bandes de danger,
  deux rampes de balises qui tracent le couloir d'approche ; pont supérieur et passerelle de
  commandement ; bras d'amarrage latéraux (un cargo à quai à tribord) ;
- **épine** : coursive octogonale, modules empilés (logements, caissons de fret) ;
- **anneau partiel** d'habitation (un quart encore en chantier : membrures nues) et ses rayons ;
- **arrière** : bloc d'énergie, réservoirs, ailes de radiateurs, tuyères de maintien d'orbite,
  mâts de communication.

Feux : rouges à bâbord (−X), verts à tribord (+X), blancs aux extrémités, ambre autour de la
bouche et sur le couloir d'approche ; émissif ≈ 4 (AGENTS.md §4 : l'émissif n'est pas atténué
par l'exposition de la caméra), hublots ≈ 1-2, intérieur du hangar ≈ 0,3 à 2.

Matériaux (8) : bibliothèque du lot 0 (`hull_plates` en deux teintes neutres et une teinte de
plaques de réparation, `metal_dark`, `container_ribbed`, `trim_ship` pour les bandes de danger),
`orbital_lights` (palette de feux et bandes de hublots du lot E, réutilisée telle quelle) et une
texture propre, `station_hangar` (parois et sol du hangar éclairés, cuits en numpy :
`--textures`, ou automatiquement si elle manque).

Une unité = 1 m ; Z-haut ; bouche principale vers +Y ; origine au centre de masse (calculé sur
les volumes fermés, arrondi au demi-mètre). Sorties :

- assets/models/space/spaceport_station/spaceport_station.gltf (+ .bin) : modèle complet ;
- assets/models/space/spaceport_station/spaceport_station_lod.gltf (+ .bin) : version lointaine ;
- art/space/spaceport_station.blend.

    blender --background --factory-startup --python tools/blender/space/spaceport_station.py -- \\
        [--textures] [--previews] [--no-export]

`--previews` : rendus EEVEE dans target/art-previews/S/ (mêmes réglages que le lot E : soleil
dur sans ombre, exposition du jeu).
"""

from __future__ import annotations

import contextlib
import io
import math
import random
import sys
from pathlib import Path

import bmesh
import bpy
import numpy as np
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "materials"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import common as c  # noqa: E402
import hexis_orbitals as ho  # noqa: E402  (Build, octo, aperçus : API du lot E, en lecture)
import texgen as tg  # noqa: E402

NAME = "spaceport_station"
OUT = f"assets/models/space/{NAME}"
BLEND = f"art/space/{NAME}.blend"
PREVIEWS = c.ROOT / "target" / "art-previews" / "S"
X, Y, Z = ho.X, ho.Y, ho.Z

# --- Dimensions (repère de construction, avant le recentrage) -----------------------------------
# Tête des hangars : coque octogonale de y = HEAD_Y0 à HEAD_Y1 (face avant), demi-largeur
# HEAD_W, demi-hauteur HEAD_H autour de HEAD_ZC.
HEAD_Y0, HEAD_Y1 = 40.0, 250.0
HEAD_W, HEAD_H, HEAD_ZC, HEAD_CH = 160.0, 75.0, 5.0, 32.0
# Bouche principale : x ∈ ±MOUTH_W/2, z ∈ [MOUTH_Z0, MOUTH_Z1], fond en y = MOUTH_YB.
MOUTH_W, MOUTH_Z0, MOUTH_Z1 = 130.0, -30.0, 20.0
MOUTH_YB = 90.0
MOUTH_YF = HEAD_Y1
FRAME = 8.0  # épaisseur du cadre de la bouche (saillie vers l'avant)
# Aires d'appontage dessinées au sol du hangar (mètres, repère du sol : x, profondeur depuis la bouche).
PADS = [(-30.0, 100.0), (32.0, 60.0)]
PAD_R = 14.0
# Projecteurs du plafond : x et profondeur depuis la bouche (fermes tous les 20 m).
FLOOD_X = (-40.0, 0.0, 40.0)
FLOOD_D = tuple(10.0 + 20.0 * k for k in range(8))

# --- Texture `station_hangar` ---------------------------------------------------------------------
# 1024² ; ligne 0 = bas de l'image = v 0 (convention de texgen).
# - lignes 0-511 (v 0 → 0,5) : paroi de hangar (U le long de la paroi, V du sol au plafond) ;
# - lignes 512-1023 (v 0,5 → 1) : sol (U = largeur de la bouche, V = profondeur depuis la bouche).
# L'intérieur n'est éclairé par rien dans le jeu (pas d'ombre au loin, ambiante presque nulle) :
# l'émissif porte la lumière des projecteurs = éclairement × albédo, et les luminaires eux-mêmes.
HANGAR_TEX = "station_hangar"
HANGAR_STRENGTH = 3.0
WALL_V = (2.0 / 1024, 510.0 / 1024)
FLOOR_V = (514.0 / 1024, 1022.0 / 1024)


def _to_srgb(lin: np.ndarray) -> np.ndarray:
    lin = np.clip(lin, 0.0, 1.0)
    return np.where(lin <= 0.0031308, lin * 12.92, 1.055 * np.power(lin, 1 / 2.4) - 0.055)


def _smooth(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def make_hangar_textures() -> None:
    """Calcule `station_hangar_{color,emissive,orm}.png` (déterministe, ≈ 2 s)."""
    res, half = 1024, 512
    rng = np.random.default_rng(6131)
    cv = tg.Canvas(res, 64.0, seed=6131)
    stain = cv.noise(3, 20.0, 1.0)
    fine = cv.noise(4, 3.0, 0.3)
    color = np.zeros((res, res, 3), np.float32)
    emis = np.zeros((res, res, 3), np.float32)
    rough = np.full((res, res), 0.7, np.float32)
    metal = np.full((res, res), 0.3, np.float32)
    occl = np.ones((res, res), np.float32)
    warm = np.array((1.0, 0.86, 0.68), np.float32)
    cold = np.array((0.70, 0.86, 1.0), np.float32)

    # Paroi : panneaux de 64 × 85 px (≈ 16 × 4 de la paroi), joints sombres, crasse en bas.
    u = (np.arange(res, dtype=np.float32) + 0.5) / res
    vw = (np.arange(half, dtype=np.float32) + 0.5) / half
    U, V = np.meshgrid(u, vw)
    sl = slice(0, half)
    seam = ((np.arange(res) % 64) < 3)[None, :] | ((np.arange(half) % 85) < 3)[:, None]
    panel_tone = rng.uniform(0.8, 1.15, (half // 85 + 1, res // 64 + 1)).astype(np.float32)
    tone = panel_tone[(np.arange(half) // 85)[:, None], (np.arange(res) // 64)[None, :]]
    alb = 0.15 * tone * (1.0 + 0.10 * fine[sl]) * (1.0 - 0.25 * _smooth(0.2, 0.0, V) * (0.6 + 0.4 * stain[sl]))
    alb = np.where(seam, 0.10, alb)
    # Bandeau rivé au tiers bas (passerelle) et plinthe sombre.
    alb = np.where((V > 0.30) & (V < 0.33), 0.16, alb)
    alb = np.where(V < 0.04, 0.12, alb)
    color[sl] = alb[..., None] * np.array((1.0, 0.98, 0.94), np.float32)
    # Éclairement : six cônes de projecteurs qui tombent du plafond, plus un fond et le rebond du sol.
    illum = 0.06 + 0.16 * _smooth(0.35, 0.0, V)
    for k in range(6):
        uk = (k + 0.5) / 6
        width = 0.016 + 0.045 * (1.0 - V)
        illum = illum + (0.35 + 1.6 * V) * np.exp(-((U - uk) / width) ** 2) * _smooth(1.0, 0.75, V)
    emis[sl] = (illum * alb)[..., None] * warm
    # Luminaires sous le plafond (tout en haut) et petits feux de passerelle.
    for k in range(6):
        c0 = int(((k + 0.5) / 6) * res)
        emis[half - 22:half - 10, c0 - 22:c0 + 22] = warm
        color[half - 22:half - 10, c0 - 22:c0 + 22] = warm * 0.8
    for k in range(32):
        c0 = int((k + 0.5) / 32 * res)
        r0 = int(0.335 * half)
        emis[r0:r0 + 5, c0 - 3:c0 + 3] = (1.0, 0.45, 0.06)
    rough[sl] = np.where(seam, 0.85, 0.65)

    # Sol : plaques de 4 m, taches d'huile, traces de patins, bandes peintes, aires d'appontage.
    sf = slice(half, res)
    fx = U * MOUTH_W - MOUTH_W / 2          # x en mètres
    fd = vw[:, None] * (MOUTH_YF - MOUTH_YB) + 0 * U  # profondeur depuis la bouche, en mètres
    plate = ((np.mod(fx + MOUTH_W / 2, 8.0) < 0.25) | (np.mod(fd, 8.0) < 0.35))
    falb = 0.24 * (1.0 + 0.08 * fine[sf]) * (1.0 - 0.35 * np.clip(stain[sf] - 0.6, 0, 1))
    falb = np.where(plate, 0.13, falb)
    falb3 = falb[..., None] * np.array((1.0, 0.98, 0.95), np.float32)
    yellow = np.array((0.55, 0.36, 0.05), np.float32)
    white = np.array((0.55, 0.55, 0.52), np.float32)
    # Axe d'entrée en tirets jaunes, lignes de bord de piste blanches.
    lane = (np.abs(fx) < 0.6) & (np.mod(fd, 10.0) < 6.0)
    edges = (np.abs(np.abs(fx) - 50.0) < 0.5)
    falb3 = np.where(lane[..., None], yellow, falb3)
    falb3 = np.where(edges[..., None], white, falb3)
    # Seuil de la bouche : bandes de danger à 45°.
    hz = fd < 3.0
    stripes = (np.mod(fx + fd, 4.0) < 2.0)
    falb3 = np.where((hz & stripes)[..., None], yellow, np.where(hz[..., None], 0.03, falb3))
    pads = []
    for px, pd in PADS:
        r = np.sqrt((fx - px) ** 2 + (fd - pd) ** 2)
        ring = (np.abs(r - PAD_R) < 0.6) | (np.abs(r - PAD_R * 0.45) < 0.35)
        cross = ((np.abs(fx - px) < 0.35) | (np.abs(fd - pd) < 0.35)) & (r < PAD_R * 0.3)
        falb3 = np.where((ring | cross)[..., None], yellow, falb3)
        # Brûlures des tuyères au centre de l'aire.
        falb3 = falb3 * (1.0 - 0.55 * np.exp(-(r / (PAD_R * 0.5)) ** 2))[..., None]
        pads.append((px, pd))
    color[sf] = falb3
    # Éclairement du sol : flaques sous les projecteurs, fond plus sombre vers la bouche.
    fill = 0.08 + 0.12 * _smooth(0.0, 40.0, fd)
    for qx in FLOOD_X:
        for qd in FLOOD_D:
            fill = fill + 1.1 * np.exp(-(((fx - qx) / 11.0) ** 2 + ((fd - qd) / 9.0) ** 2))
    emis[sf] = falb3 * fill[..., None] * warm * 0.9
    # Feux encastrés autour des aires (froids) et le long des lignes de bord (ambre).
    for px, pd in pads:
        for k in range(12):
            a = 2 * math.pi * k / 12
            qx, qd = px + (PAD_R + 1.5) * math.cos(a), pd + (PAD_R + 1.5) * math.sin(a)
            m = ((fx - qx) ** 2 + (fd - qd) ** 2) < 0.75 ** 2
            emis[sf][m] = cold
            color[sf][m] = cold * 0.8
    for sx in (-1, 1):
        for d in np.arange(4.0, MOUTH_YF - MOUTH_YB, 8.0):
            m = ((fx - sx * 50.0) ** 2 / 0.6 ** 2 + (fd - d) ** 2 / 1.0 ** 2) < 1.0
            emis[sf][m] = (1.0, 0.5, 0.08)
            color[sf][m] = (0.9, 0.45, 0.07)
    rough[sf] = np.where(plate, 0.8, 0.6 - 0.15 * np.clip(stain[sf], 0, 1))
    metal[sf] = 0.15

    out = c.TEXTURES_DIR
    tg.write_png(out / f"{HANGAR_TEX}_color.png", _to_srgb(color))
    tg.write_png(out / f"{HANGAR_TEX}_emissive.png", _to_srgb(emis))
    tg.write_png(out / f"{HANGAR_TEX}_orm.png", np.stack([occl, rough, metal], -1))
    print(f"textures {HANGAR_TEX} écrites dans {out}")


# --- Matériaux ----------------------------------------------------------------------------------


def palette() -> dict:
    return {
        # Coque grise neutre (ni jaune Hexa-Corp, ni insigne) et modules plus froids.
        "hull": c.textured_material("stn_hull", "hull_plates", tint=(0.68, 0.67, 0.64)),
        "hull2": c.textured_material("stn_hull2", "hull_plates", tint=(0.44, 0.47, 0.51)),
        # Plaques de réparation : tôle d'apprêt rouillée, rapportée sur le bordé.
        "patch": c.textured_material("stn_patch", "hull_plates", tint=(0.55, 0.40, 0.30)),
        "dark": c.textured_material("stn_dark", "metal_dark"),
        "ribbed": c.textured_material("stn_ribbed", "container_ribbed", tint=(0.62, 0.63, 0.65)),
        "trim": c.textured_material("stn_trim", "trim_ship"),
        "lights": c.textured_material(
            "stn_lights", ho.LIGHTS_TEX, emissive=(1.0, 1.0, 1.0), emissive_strength=ho.LIGHTS_STRENGTH
        ),
        "hangar": c.textured_material(
            "stn_hangar", HANGAR_TEX, emissive=(1.0, 1.0, 1.0), emissive_strength=HANGAR_STRENGTH
        ),
    }


# --- Géométrie ----------------------------------------------------------------------------------


def quad(B, key, pts, uvs, normal, tag="quad"):
    """Quadrilatère `pts` (4 coins) aux UV donnés, face tournée vers `normal`."""
    return B.mesh(key, pts, [(0, 1, 2, 3)], [uvs], tag=tag, want=lambda p, n: Vector(normal))


def ring_cap(B, key, y, outer, inner, normal_y, tile, tag="cap"):
    """Face plane en y percée d'un trou : couronne entre le profil `outer` et le rectangle (ou
    polygone convexe) `inner`, tous deux en (x, z), triangulée par fusion angulaire autour du
    centre du trou (les deux contours sont étoilés par rapport à lui)."""
    cx = sum(p[0] for p in inner) / len(inner)
    cz = sum(p[1] for p in inner) / len(inner)

    def ang(p):
        return math.atan2(p[1] - cz, p[0] - cx) % (2 * math.pi)

    o = sorted(outer, key=ang)
    i = sorted(inner, key=ang)
    verts = [Vector((x, y, z)) for x, z in o] + [Vector((x, y, z)) for x, z in i]
    no, ni = len(o), len(i)
    faces = []
    a, b = 0, 0
    while a < no or b < ni:
        oa, ob = a % no, (a + 1) % no
        ia, ib = no + b % ni, no + (b + 1) % ni
        next_o = ang(o[(a + 1) % no]) + (2 * math.pi if a + 1 >= no else 0)
        next_i = ang(i[(b + 1) % ni]) + (2 * math.pi if b + 1 >= ni else 0)
        if b >= ni or (a < no and next_o <= next_i):
            faces.append((oa, ob, ia))
            a += 1
        else:
            faces.append((oa, ib, ia))
            b += 1
    uvs = [[(verts[k].x / tile, verts[k].z / tile) for k in f] for f in faces]
    return B.mesh(key, verts, faces, uvs, tag=tag, want=lambda p, n: Vector((0, normal_y, 0)))


def closed(obj) -> bool:
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    ok = len(bm.faces) > 0 and all(e.is_manifold for e in bm.edges)
    bm.free()
    return ok


def center_of_mass(parts) -> Vector:
    """Centre de masse des volumes fermés (densité uniforme), par tétraèdres signés."""
    total, acc = 0.0, Vector()
    for obj in parts:
        if obj.type != "MESH" or not closed(obj):
            continue
        mw = obj.matrix_world
        me = obj.data
        me.calc_loop_triangles()
        co = [mw @ v.co for v in me.vertices]
        for tri in me.loop_triangles:
            a, b, d = (co[k] for k in tri.vertices)
            vol = a.dot(b.cross(d)) / 6.0
            total += vol
            acc += vol * (a + b + d) / 4.0
    return acc / total if abs(total) > 1e-6 else Vector()


class Station:
    """Construit la station (complète, ou version lointaine `lod`) dans un `ho.Build`."""

    def __init__(self, B, lod: bool):
        self.B, self.lod = B, lod
        self.rng = random.Random(97)

    def truss(self, key, a, b, w, h, chord, strut, bay, up=Z, chord_key=None):
        """Treillis complet, ou simple poutre-caisson dans la version lointaine."""
        if self.lod:
            self.B.beam(chord_key or key, a, b, w * 0.6, h * 0.6, up=up)
        else:
            self.B.truss(key, a, b, w, h, chord, strut, bay, up=up, chord_key=chord_key)

    def light(self, pos, kind, size, lod_keep=False):
        if not self.lod or lod_keep:
            self.B.light(pos, kind, size * (1.6 if self.lod else 1.0))

    # Tête des hangars --------------------------------------------------------------------------

    def head(self):
        B, lod = self.B, self.lod
        prof = lambda w, h, ch: [(x, HEAD_ZC + z) for x, z in ho.octo(w, h, ch)]  # noqa: E731
        full = prof(HEAD_W, HEAD_H, HEAD_CH)
        lip = prof(HEAD_W - 12, HEAD_H - 9, HEAD_CH - 4)
        sections = [
            (HEAD_Y0, prof(70, 52, 22)),
            (HEAD_Y0 + 35, full),
            (HEAD_Y1 - 15, full),
            (HEAD_Y1, lip),
        ]
        # Coque bicolore : l'avant (autour de la bouche) en teinte froide, le reste en gris clair.
        split = 198.0
        B.loft("hull", sections[:2] + [(split, full)], 20, caps=(True, False))
        B.loft("hull2", [(split, full)] + sections[2:], 20, caps=(False, False))
        # Témoin fermé de la tête pour le centre de masse (la coque est ouverte sur la bouche).
        self.mass_proxy = B.loft("hull", sections, 20, caps=(True, True), tag="proxy")
        B.parts.remove(self.mass_proxy)
        hw = MOUTH_W / 2
        hole = [(hw, MOUTH_Z0), (hw, MOUTH_Z1), (-hw, MOUTH_Z1), (-hw, MOUTH_Z0)]
        ring_cap(B, "hull", HEAD_Y1, lip, hole, 1, 20)
        # Membrures extérieures qui suivent le profil (raidisseurs de la coque, tous les 30 m).
        for y in ((HEAD_Y0 + 38, HEAD_Y1 - 17) if lod else (HEAD_Y0 + 38, 108, 138, 168, 198, HEAD_Y1 - 17)):
            self.rib(full, y)
        self.facade()

        self.mouth()

        # Flancs : rangées de hublots, ouïes, plaques de réparation, portes de navettes fermées.
        for sx in (-1, 1):
            n = X * sx
            for zr, band in ((-20.0, "win_warm"), (-6.0, "win_sparse"), (26.0, "win_warm")):
                if lod and band == "win_sparse":
                    continue
                B.strip("lights", (sx * HEAD_W, 150, zr), n, Y, 150, 3.2, band, 22)
            if not lod:
                B.strip("trim", (sx * HEAD_W, 150, 8.0), n, Y, 150, 2.4, "vent", 6)
                B.strip("trim", (sx * HEAD_W, 150, -34.0), n, Y, 150, 3.0, "hazard", 12)
                # Porte de soute à navettes (fermée) : cadre sombre, liseré de lumière.
                B.box("dark", (3, 34, 22), (sx * (HEAD_W + 1), 205, 10))
                for zz in (20.0, 0.5):
                    B.strip("trim", (sx * (HEAD_W + 2.5), 205, zz), n, Y, 34, 2.0, "hazard", 8)
                B.box("hull2", (3.4, 28, 12), (sx * (HEAD_W + 1.2), 205, 10))
                B.strip("lights", (sx * (HEAD_W + 2.5), 205, -1.5), n, Y, 30, 1.0, "win_cold", 4)
                B.light((sx * (HEAD_W + 3), 187, 21), "amber", 2.2)
                B.light((sx * (HEAD_W + 3), 223, 21), "amber", 2.2)
        # Pont supérieur (bureaux du port) et passerelle de commandement à baies.
        deck_c, deck = Vector((0, 150, HEAD_ZC + HEAD_H + 14)), (230, 150, 28)
        B.box("hull2", deck, deck_c, bevel=3)
        B.box_windows(deck_c, deck, None, ("+x", "-x", "+y"), "win_warm", 2, 20, 3.0)
        bridge_c, bridge = deck_c + Vector((0, 40, 26)), (90, 44, 24)
        B.box("hull", bridge, bridge_c, bevel=3)
        B.box_windows(bridge_c, bridge, None, ("+x", "-x", "+y", "-y"), "win_big", 1, 16, 7, margin=0.08)
        B.box("dark", (100, 50, 3), bridge_c + Vector((0, 0, 13.5)))
        self.light(bridge_c + Vector((0, 22, 16)), "cold", 2.5)
        # Mâts de communication sur le pont.
        for sx, h, dish in ((-1, 120, True), (1, 75, False)):
            base = Vector((sx * 85, 115, deck_c.z + 14))
            top = base + Vector((0, 0, h))
            self.truss("dark", base, top, 8, 8, 1.4, 0.8, 10, up=Y)
            self.light(top + Vector((0, 0, 3)), "red", 3.0, lod_keep=True)
            if not lod:
                self.light(base + Vector((0, 0, h * 0.55)), "red", 1.6)
                B.beam("dark", top - Vector((14, 0, 6)), top + Vector((14, 0, -6)), 1.0)
            if dish:
                p = base + Vector((0, 0, h * 0.45))
                B.cyl("dark", p + Vector((6, 0, 0)), X, 1.5, 12, seg=8)
                B.cyl("hull", p + Vector((16, 4, 4)), Vector((1, 0.6, 0.5)), 14, 2.0, seg=20 if not lod else 12, r2=5)
        # Antennes et boîtiers sous la tête.
        for k, (x, y) in enumerate(((-90, 120), (60, 180), (110, 90))):
            p = Vector((x, y, HEAD_ZC - HEAD_H))
            B.box("dark", (22, 18, 8), p + Vector((0, 0, -4)))
            if not lod:
                B.beam("dark", p, p + Vector((0, 0, -40 - 10 * k)), 1.2)
                self.light(p + Vector((0, 0, -42 - 10 * k)), "cold", 1.6)
        # Plaques de réparation et caissons rapportés (seulement en pleine qualité).
        if not lod:
            self.patches()

    def rib(self, prof, y, size=(3.0, 2.4)):
        """Cadre de poutres sombres posé sur le profil `prof` (x, z), en y."""
        B = self.B
        cz = sum(p[1] for p in prof) / len(prof)
        n = len(prof)
        for j in range(n):
            (xa, za), (xb, zb) = prof[j], prof[(j + 1) % n]
            mid = Vector(((xa + xb) / 2, 0, (za + zb) / 2 - cz))
            seg = Vector((xb - xa, 0, zb - za))
            out = Vector((-seg.z, 0, seg.x)).normalized()
            if out.dot(mid) < 0:
                out = -out
            off = out * (size[1] / 2)
            a = Vector((xa, y, za)) + off
            b = Vector((xb, y, zb)) + off
            B.beam("dark", a, b, size[0], size[1], up=out, ext=size[1] / 2)

    def facade(self):
        """Face avant autour de la bouche : auvent, pylônes, galeries vitrées, panneaux rapportés."""
        B, lod = self.B, self.lod
        yf = HEAD_Y1
        hw = MOUTH_W / 2 + FRAME
        # Auvent au-dessus de la bouche (abrite le linteau), bande de danger sur son nez.
        brow_c = Vector((0, yf + 7, MOUTH_Z1 + FRAME + 7))
        B.box("hull2", (2 * hw + 52, 14, 12), brow_c, bevel=1.5)
        B.box("dark", (2 * hw + 56, 4, 3), brow_c + Vector((0, 6, -7)))
        B.strip("trim", brow_c + Vector((0, 7, 2)), Y, X, 2 * hw + 48, 4.0, "hazard", 16)
        # Pylônes qui encadrent la bouche (guides de la porte blindée).
        for sx in (-1, 1):
            pc = Vector((sx * (hw + 9), yf + 6, -6))
            B.box("hull2", (14, 12, 84), pc, bevel=1.2)
            B.box("dark", (16, 4, 86), pc + Vector((0, -4, 0)))
            if not lod:
                B.strip("trim", pc + Vector((0, 6, -20)), Y, Z, 30, 6.0, "vent", 8)
                B.strip("lights", pc + Vector((0, 6, 22)), Y, Z, 24, 2.0, "light_chain", 8)
            self.light(pc + Vector((0, 7, 43)), "amber", 3.0, lod_keep=True)
            # Feux de position aux angles de la face avant.
            self.light(Vector((sx * (HEAD_W - 10), yf + 1, HEAD_ZC + 20)), "red" if sx < 0 else "green", 4.0, lod_keep=True)
        # Galeries vitrées au-dessus de l'auvent : le contrôle d'approche regarde le couloir.
        for z, band, length in ((54.0, "win_cold", 210), (62.0, "win_big", 120)):
            if lod and band == "win_big":
                continue
            B.strip("lights", (0, yf, z), Y, X, length, 3.4 if band == "win_cold" else 5.0, band, 18)
        B.box("dark", (220, 5, 2.4), (0, yf + 2.5, 49))
        if lod:
            return
        # Panneaux rapportés et ouïes de part et d'autre des pylônes.
        rng = random.Random(41)
        for sx in (-1, 1):
            x0 = hw + 20
            for k, (z, h) in enumerate(((-44, 18), (-22, 22), (4, 24), (30, 16))):
                w = rng.uniform(24, 46)
                x = sx * (x0 + w / 2 + rng.uniform(0, 30))
                if abs(x) + w / 2 > HEAD_W - 18:
                    w = 2 * (HEAD_W - 18 - abs(x))
                key = ("hull2", "patch", "hull", "patch")[(k + (sx > 0)) % 4]
                B.box(key, (w, 1.2, h), (x, yf + 0.6, z), tag="panel")
            B.strip("trim", (sx * 118, yf, -52), Y, X, 50, 3.0, "vent", 6)
            B.strip("lights", (sx * 118, yf, 40), Y, X, 46, 2.6, "win_sparse", 10)

    def mouth(self):
        """Bouche du hangar principal : parois, sol, plafond, fermes, projecteurs, galerie,
        navettes, porte blindée relevée, cadre et couloir d'approche."""
        B, lod = self.B, self.lod
        hw = MOUTH_W / 2
        x0, x1, z0, z1 = -hw, hw, MOUTH_Z0, MOUTH_Z1
        yf, yb = MOUTH_YF, MOUTH_YB
        depth = yf - yb
        w0, w1 = WALL_V
        f0, f1 = FLOOR_V
        # Sol (U : x, V : profondeur depuis la bouche).
        quad(B, "hangar", [(x0, yf, z0), (x1, yf, z0), (x1, yb, z0), (x0, yb, z0)],
             [(0, f0), (1, f0), (1, f1), (0, f1)], Z, tag="floor")
        # Parois latérales et fond (U le long de la paroi, V du sol au plafond).
        for sx in (-1, 1):
            xw = sx * hw
            quad(B, "hangar", [(xw, yf, z0), (xw, yb, z0), (xw, yb, z1), (xw, yf, z1)],
                 [(0, w0), (1, w0), (1, w1), (0, w1)], -X * sx, tag="wall")
        quad(B, "hangar", [(x0, yb, z0), (x1, yb, z0), (x1, yb, z1), (x0, yb, z1)],
             [(0, w0), (1, w0), (1, w1), (0, w1)], Y, tag="back")
        # Plafond sombre (la lumière vient d'en dessous des fermes).
        t = 16.0
        quad(B, "dark", [(x0, yf, z1), (x1, yf, z1), (x1, yb, z1), (x0, yb, z1)],
             [(x0 / t, yf / t), (x1 / t, yf / t), (x1 / t, yb / t), (x0 / t, yb / t)], -Z, tag="ceiling")
        # Fermes du plafond et projecteurs.
        for d in FLOOD_D:
            y = yf - d
            if lod and int(d) % 40 != 10:
                continue
            B.beam("dark", (x0, y, z1 - 2), (x1, y, z1 - 2), 3.0, 4.0)
            for x in FLOOD_X:
                self.light((x, y, z1 - 5), "warm", 2.6, lod_keep=True)
        if not lod:
            for x in (-45, 0, 45):
                B.beam("dark", (x, yb, z1 - 5), (x, yf - 4, z1 - 5), 2.0, 2.0)
        # Galerie de contrôle au fond, sas intérieur et ses feux.
        B.strip("lights", (0, yb, z1 - 12), Y, X, MOUTH_W - 24, 7, "win_big", 14)
        B.box("dark", (MOUTH_W - 16, 4, 2.4), (0, yb + 2, z1 - 16.5))
        B.box("dark", (40, 2.5, 24), (0, yb + 1.25, z0 + 12))
        B.strip("trim", (0, yb + 2.5, z0 + 2), Y, X, 38, 2.0, "hazard", 8)
        for sx in (-1, 1):
            self.light((sx * 22, yb + 2, z0 + 22), "amber", 1.6)
        # Passerelles le long des parois (au tiers de la hauteur).
        if not lod:
            for sx in (-1, 1):
                B.box("dark", (4, depth - 4, 1.0), (sx * (hw - 2), (yf + yb) / 2, z0 + 16))
            self.shuttles()
        # Porte blindée relevée : son bas dépasse sous le linteau, bande de danger.
        B.box("dark", (MOUTH_W, 5, 7), (0, yf - 6, z1 - 3.5))
        B.strip("trim", (0, yf - 3.5, z1 - 5.5), Y, X, MOUTH_W - 4, 2.6, "hazard", 10)
        # Cadre en saillie : poutres sombres, bandes de danger, feux ambre aux coins.
        fy = yf + FRAME / 2
        B.box("dark", (MOUTH_W + 2 * FRAME, FRAME, FRAME), (0, fy, z1 + FRAME / 2))
        B.box("dark", (MOUTH_W + 2 * FRAME, FRAME, FRAME), (0, fy, z0 - FRAME / 2))
        for sx in (-1, 1):
            B.box("dark", (FRAME, FRAME, z1 - z0), (sx * (hw + FRAME / 2), fy, (z0 + z1) / 2))
        B.strip("trim", (0, yf + FRAME, z1 + FRAME / 2), Y, X, MOUTH_W + 2 * FRAME - 2, 4.0, "hazard", 16)
        B.strip("trim", (0, yf + FRAME, z0 - FRAME / 2), Y, X, MOUTH_W + 2 * FRAME - 2, 4.0, "hazard", 16)
        for sx in (-1, 1):
            B.strip("trim", (sx * (hw + FRAME / 2), yf + FRAME, (z0 + z1) / 2), Y, Z, z1 - z0, 4.0, "hazard", 16)
        # Chaîne de feux sur le linteau et le seuil (la bouche se lit de loin).
        B.strip("lights", (0, yf + FRAME, z1 + FRAME - 1.2), Y, X, MOUTH_W, 1.6, "light_chain", 10)
        B.strip("lights", (0, yf + FRAME, z0 - FRAME + 1.2), Y, X, MOUTH_W, 1.6, "light_chain", 10)
        for sx in (-1, 1):
            for zc in (z0 - FRAME, z1 + FRAME):
                self.light((sx * (hw + FRAME), yf + FRAME + 1, zc), "amber", 3.2, lod_keep=True)
        # Couloir d'approche : deux rampes sous les coins de la bouche, feux ambre tous les 20 m,
        # blancs en bout.
        lane = 180.0
        for sx in (-1, 1):
            xa = sx * (hw + 6)
            za = z0 - FRAME - 6
            a, b = Vector((xa, yf + 2, za)), Vector((xa, yf + lane, za))
            if lod:
                B.beam("dark", a, b, 2.0, 2.0)
            else:
                B.truss("dark", a, b, 4, 4, 0.9, 0.5, 10)
            B.box("dark", (8, 10, 8), (xa, yf + 5, za))
            for k in range(1, int(lane // 20) + 1):
                kind = "cold" if k == int(lane // 20) else "amber"
                self.light((xa, yf + 20 * k, za + 3), kind, 2.6, lod_keep=(k % 3 == 0) or kind == "cold")

    def shuttles(self):
        """Deux navettes posées sur leurs aires : silhouettes contre le fond éclairé."""
        B = self.B
        for (px, pd), key, yaw in zip(PADS, ("hull2", "hull"), (1, -1)):
            y = MOUTH_YF - pd
            z = MOUTH_Z0 + 3.4
            secs = [(y - 12, [(x, z + zz) for x, zz in ho.octo(4.5, 2.6, 1.2)]),
                    (y + 6, [(x, z + zz) for x, zz in ho.octo(4.5, 2.6, 1.2)]),
                    (y + 12, [(x, z - 0.6 + zz) for x, zz in ho.octo(2.6, 1.6, 0.8)])]
            secs = [(yy, [(px + x, zz) for x, zz in prof]) for yy, prof in secs]
            B.loft(key, secs, 8, caps=(True, True))
            for sx in (-1, 1):
                B.box("dark", (5, 7, 1.2), (px + sx * 7, y - 6, z + 0.5))
                B.beam("dark", (px + sx * 3, y - 8, z - 2.6), (px + sx * 3, y + 6, z - 2.6), 0.5, 0.8)
            B.box("dark", (6, 3, 3.4), (px, y - 13.5, z + 0.4))
            B.strip("lights", (px, y + 9.5, z + 1.2), Vector((0, 0.6, 1)), X, 3.4, 0.9, "win_cold", 4)

    def patches(self):
        """Plaques de réparation, caissons et coulures de suie sur le bordé de la tête."""
        B, rng = self.B, self.rng
        for _ in range(26):
            sx = rng.choice((-1, 1))
            w, h = rng.uniform(8, 26), rng.uniform(5, 14)
            y = rng.uniform(HEAD_Y0 + 45, HEAD_Y1 - 30)
            z = rng.uniform(HEAD_ZC - HEAD_H + HEAD_CH + h / 2, HEAD_ZC + HEAD_H - HEAD_CH - h / 2)
            if any(abs(z - zr) < h / 2 + 2.5 for zr in (-20.0, -6.0, 26.0, 8.0, -34.0)):
                continue
            B.box(rng.choice(("patch", "patch", "hull2")), (0.8, w, h), (sx * (HEAD_W + 0.3), y, z), tag="patch")
        for _ in range(18):
            w, d = rng.uniform(10, 30), rng.uniform(8, 22)
            x = rng.uniform(-HEAD_W + HEAD_CH + w / 2, HEAD_W - HEAD_CH - w / 2)
            y = rng.uniform(HEAD_Y0 + 40, HEAD_Y1 - 20)
            if abs(x) < 125 and 70 < y < 230:
                continue  # sous le pont supérieur
            B.box(rng.choice(("patch", "hull2", "patch")), (w, d, 0.8), (x, y, HEAD_ZC + HEAD_H + 0.3), tag="patch")

    # Bras d'amarrage --------------------------------------------------------------------------

    def arms(self):
        B, lod = self.B, self.lod
        for sx in (-1, 1):
            root = Vector((sx * HEAD_W, 110, -25))
            elbow = Vector((sx * 235, 110, -25))
            tip = Vector((sx * 235, 235, -25))
            self.truss("hull2", root, elbow, 12, 12, 2.0, 1.1, 14, chord_key="dark")
            self.truss("hull2", elbow, tip, 12, 12, 2.0, 1.1, 14, chord_key="dark")
            B.cyl("hull", elbow, Z, 11, 16, seg=16 if not lod else 10)
            B.cyl("hull", (sx * 222, 172.5, -25), Y, 4.5, 125, seg=10 if not lod else 6)
            # Collerettes d'amarrage, feux.
            for y in (160.0, 220.0):
                B.cyl("dark", (sx * 247, y, -25), X * sx, 6, 12, seg=14 if not lod else 8)
                self.light((sx * 254, y, -16), "amber", 2.0)
            self.light(tip + Vector((0, 8, 0)), "cold", 3.0, lod_keep=True)
            self.light(elbow + Vector((sx * 12, 0, 0)), "red" if sx < 0 else "green", 4.0, lod_keep=True)
        if not lod:
            self.docked_ship(Vector((264, 195, -25)))

    def docked_ship(self, ctr):
        """Cargo à quai le long du bras tribord (≈ 72 m, avant vers +Y)."""
        B = self.B
        x, y, z = ctr
        prof = lambda w, h, ch, dz=0: [(x + px, z + dz + pz) for px, pz in ho.octo(w, h, ch)]  # noqa: E731
        B.loft("hull2", [(y - 36, prof(8, 7, 3)), (y + 14, prof(9, 8, 3)), (y + 26, prof(8, 7, 3)),
                         (y + 36, prof(4, 3.6, 1.6, -1.5))], 10, caps=(True, True))
        for k, yc in enumerate((-20, -6, 8)):
            B.box(("patch", "ribbed", "hull")[k], (20, 12, 18), (x, y + yc, z + 2), bevel=0.3)
        B.box("dark", (24, 4, 20), (x, y - 30, z))
        for sx in (-1, 1):
            B.cyl("dark", (x + sx * 6, y - 39, z), -Y, 3.0, 6, seg=12, r2=4.0)
            B.cyl("lights", (x + sx * 6, y - 42.1, z), -Y, 3.6, 0.2, seg=12)
        B.strip("lights", (x, y + 31, z + 4.5), Vector((0, 0.7, 1)), X, 7, 1.6, "win_cold", 5)
        B.box("dark", (8, 4, 4), (x - 10, y + 25, z))  # tunnel d'accès vers la collerette (y = 220)
        B.light((x - 9.5, y - 28, z + 9), "red", 1.2)
        B.light((x + 9.5, y - 28, z + 9), "green", 1.2)

    # Épine, modules, anneau ----------------------------------------------------------------------

    def spine(self):
        B, lod = self.B, self.lod
        y0, y1 = -235.0, HEAD_Y0 + 6
        prof = ho.octo(42, 34, 13)
        B.loft("hull", [(y0, prof), (y1, prof)], 16, caps=(True, True))
        for sx in (-1, 1):
            B.strip("lights", (sx * 42, (y0 + y1) / 2, 4), X * sx, Y, y1 - y0 - 20, 3.0, "win_warm", 18)
            if not lod:
                B.strip("lights", (sx * 42, (y0 + y1) / 2, -8), X * sx, Y, y1 - y0 - 20, 2.4, "win_sparse", 18)
        B.strip("lights", (0, (y0 + y1) / 2, 34), Z, Y, y1 - y0 - 10, 3.0, "light_chain", 30)
        # Conduites le long de l'épine.
        if not lod:
            for sx, key, r in ((-1, "dark", 3.2), (1, "hull2", 2.6)):
                B.cyl(key, (sx * 33, (y0 + y1) / 2, -30), Y, r, y1 - y0, seg=10)
        # Modules empilés : logements au-dessus, caissons de fret dessous.
        for k, (y, w, h) in enumerate(((-5.0, 80, 36), (-55.0, 64, 30), (-165.0, 70, 32))):
            ctr = Vector((0, y, 34 + h / 2))
            size = (w, 40, h)
            B.box("hull2" if k % 2 == 0 else "hull", size, ctr, bevel=2)
            B.box_windows(ctr, size, None, ("+x", "-x"), "win_warm", 2, 14, 2.6)
            B.box("dark", (w + 4, 4, 3), ctr + Vector((0, 20, h / 2 - 1.5)))
        if not lod:
            for k, y in enumerate((-20.0, -45.0, -150.0, -180.0)):
                for sx in (-1, 1):
                    key = ("ribbed", "patch", "ribbed", "hull2")[(k + (sx > 0)) % 4]
                    B.box(key, (22, 20, 18), (sx * 22, y, -34 - 9), bevel=0.4, tag="container")
        else:
            B.box("ribbed", (66, 50, 18), (0, -32, -43))
            B.box("ribbed", (66, 50, 18), (0, -165, -43))

    def ring(self):
        """Anneau d'habitation partiel (axe Y) : un quart encore en membrures nues."""
        B, lod = self.B, self.lod
        yc, ri, ro, depth = -105.0, 150.0, 184.0, 44.0
        step = 15.0 if not lod else 30.0
        gap = (200.0, 260.0)  # degrés : 0 = +Z, 90 = +X
        k = 0
        a = 0.0
        while a < 360.0 - 1e-6:
            a0, a1 = a, a + step
            a = a1
            if a0 >= gap[0] and a1 <= gap[1]:
                continue
            key = ("hull", "hull", "hull2", "hull", "patch", "hull")[k % 6] if not lod else "hull"
            k += 1
            B.wedge(key, (0, yc, 0), Y, ri, ro, math.radians(a0), math.radians(a1), depth)
        built = (math.radians(gap[1]), math.radians(gap[0] + 360.0))
        seg = 72 if not lod else 24
        for off, band in ((-9.0, "win_warm"), (9.0, "win_cold")):
            if lod and off > 0:
                continue
            B.ring_band("lights", (0, yc + off, 0), Y, ro + 0.2, 4.0, band, 14, seg=seg, a0=built[0], a1=built[1])
        if not lod:
            B.ring_band("lights", (0, yc, 0), Y, ri - 0.2, 4.0, "win_sparse", 14, seg=seg, inward=True, a0=built[0], a1=built[1])
            for e in (-1, 1):
                B.ring_band("trim", (0, yc + e * (depth / 2 - 3), 0), Y, ro + 0.25, 3.0, "hazard", 12, seg=seg, a0=built[0], a1=built[1])
            # Membrures nues du quart en chantier : cadres plans et lisses.
            for ad in range(int(gap[0]) + 15, int(gap[1]), 15):
                ar = math.radians(ad)
                radial = Z * math.cos(ar) + X * math.sin(ar)
                for e in (-1, 1):
                    B.beam("dark", (Vector((0, yc + e * depth / 2.4, 0)) + radial * ri),
                           (Vector((0, yc + e * depth / 2.4, 0)) + radial * ro), 2.2, 2.2, up=Y)
                B.beam("dark", Vector((0, yc - depth / 2.4, 0)) + radial * ro, Vector((0, yc + depth / 2.4, 0)) + radial * ro, 2.0)
            for rr in (ri + 2, ro - 2):
                for e in (-1, 1):
                    pts = []
                    for ad in range(int(gap[0]), int(gap[1]) + 1, 15):
                        ar = math.radians(ad)
                        pts.append(Vector((0, yc + e * depth / 2.4, 0)) + (Z * math.cos(ar) + X * math.sin(ar)) * rr)
                    for p, q in zip(pts, pts[1:]):
                        B.beam("dark", p, q, 1.6, 1.6, up=Y, ext=0.8)
        # Rayons : tubes pressurisés et câbles tendus.
        for ad in (0.0, 120.0, 300.0):
            ar = math.radians(ad)
            radial = Z * math.cos(ar) + X * math.sin(ar)
            a, b = Vector((0, yc, 0)) + radial * 38, Vector((0, yc, 0)) + radial * (ri + 1)
            B.cyl("hull2", (a + b) / 2, radial, 7, (b - a).length, seg=12 if not lod else 8)
            if not lod:
                for e in (-1, 1):
                    B.beam("dark", a + Y * (e * 12), b + Y * (e * 16), 1.2)
        # Moyeu de l'anneau autour de l'épine.
        B.cyl("dark", (0, yc, 0), Y, 56, 30, seg=24 if not lod else 12)
        B.ring_band("trim", (0, yc, 0), Y, 56.3, 6, "hazard", 14, seg=24 if not lod else 12)
        for ad in (90.0, 270.0):
            ar = math.radians(ad)
            p = Vector((0, yc, 0)) + (Z * math.cos(ar) + X * math.sin(ar)) * (ro + 2)
            self.light(p, "red" if ad > 180 else "green", 4.0, lod_keep=True)
        self.light(Vector((0, yc, ro + 2)), "cold", 3.0, lod_keep=True)

    # Arrière ----------------------------------------------------------------------------------

    def aft(self):
        B, lod = self.B, self.lod
        blk_c, blk = Vector((0, -280, 0)), (120, 90, 96)
        B.box("hull", blk, blk_c, bevel=6)
        B.box_windows(blk_c, blk, None, ("+x", "-x"), "win_sparse", 3, 16, 2.4)
        B.box("dark", (126, 10, 100), blk_c + Vector((0, 30, 0)))
        if not lod:
            B.strip("trim", blk_c + Vector((0, -45, 40)), -Y, X, 110, 4, "hazard", 14)
            B.strip("trim", blk_c + Vector((0, -45, -40)), -Y, X, 110, 4, "hazard", 14)
        # Réservoirs.
        for k, (dx, dz) in enumerate(((-38, 68), (38, 68), (-38, -68), (38, -68))):
            p = Vector((dx, -265, dz))
            B.cyl("hull2" if k % 3 else "patch", p, Y, 20, 80, seg=18 if not lod else 10)
            for e in (-1, 1):
                B.cyl("dark", p + Vector((0, e * 44, 0)), Y * e, 20, 8, seg=18 if not lod else 10, r2=9)
            if not lod:
                B.ring_band("trim", p + Vector((0, 18, 0)), Y, 20.3, 3, "hazard", 12, seg=18)
        # Tuyères de maintien d'orbite.
        for dx, dz in ((-30, 22), (30, 22), (-30, -22), (30, -22)):
            p = Vector((dx, -330, dz))
            B.cyl("dark", p, -Y, 11, 10, seg=16 if not lod else 8, r2=14)
            B.cyl("lights", p + Vector((0, -5.1, 0)), -Y, 12, 0.2, seg=16 if not lod else 8)
        # Ailes de radiateurs (horizontales) et feux de bout d'aile.
        for sx in (-1, 1):
            a, b = Vector((sx * 60, -280, 0)), Vector((sx * 290, -280, 0))
            self.truss("dark", a, b, 14, 14, 2.4, 1.3, 18, chord_key="hull2")
            if not lod:
                B.cyl("hull2", (sx * 175, -268, 0), X, 2.5, 230, seg=8)
            for k, xc in enumerate((100, 155, 210, 260)):
                w = 50 if k < 3 else 44
                if lod:
                    B.box("ribbed", (w, 120, 2), (sx * xc, -280, 0), tag="radiator")
                    continue
                for e in (-1, 1):
                    B.box("ribbed", (w, 54, 1.6), (sx * xc, -280 + e * 34, 0), tag="radiator")
                    B.box("dark", (w, 3, 3), (sx * xc, -280 + e * 62, 0), tag="radiator_edge")
            self.light((sx * 298, -280, 0), "red" if sx < 0 else "green", 4.5, lod_keep=True)
        # Mât arrière et antenne basse.
        base, top = blk_c + Vector((20, 0, 48)), blk_c + Vector((20, 0, 150))
        self.truss("dark", base, top, 7, 7, 1.3, 0.7, 10, up=Y)
        self.light(top + Vector((0, 0, 3)), "red", 3.0, lod_keep=True)
        B.cyl("hull", top + Vector((-14, 0, -30)), Vector((-1, -0.4, 0.6)), 12, 1.6, seg=20 if not lod else 12, r2=4)
        if not lod:
            B.beam("dark", top + Vector((-2, 0, -30)), top + Vector((-12, 0, -30)), 1.2)
            B.beam("dark", blk_c + Vector((0, -10, -48)), blk_c + Vector((0, -10, -120)), 1.6)
            self.light(blk_c + Vector((0, -10, -123)), "cold", 2.0)
        self.light(blk_c + Vector((0, -46, 0)), "cold", 3.0, lod_keep=True)

    def build(self):
        self.mass_proxy = None
        self.head()
        self.arms()
        self.spine()
        self.ring()
        self.aft()
        return self.B.parts


TILES = {"hull": 32, "hull2": 20, "patch": 12, "dark": 8, "ribbed": 10}


def build(m, coll, lod: bool, prefix: str):
    """Pièces de la station, et le témoin fermé de la tête (centre de masse, à supprimer)."""
    B = ho.Build(prefix, coll, m, TILES, seed=211 if not lod else 212, offset=0.35)
    st = Station(B, lod)
    return st.build(), st.mass_proxy


# --- Aperçus ------------------------------------------------------------------------------------


def _planet(radius: float, center) -> bpy.types.Object:
    """Planète d'arrière-plan des aperçus (texture d'Hexis), jamais exportée."""
    bpy.ops.mesh.primitive_uv_sphere_add(segments=96, ring_count=48, radius=radius, location=center)
    obj = bpy.context.active_object
    obj.name = "preview_planet"
    for p in obj.data.polygons:
        p.use_smooth = True
    mat = bpy.data.materials.new("preview_planet")
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
    tex = nodes.new("ShaderNodeTexImage")
    path = c.ROOT / "assets" / "textures" / "planets" / "hexis_color_2k.png"
    if path.exists():
        tex.image = bpy.data.images.load(str(path), check_existing=True)
        mat.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 0.9
    obj.data.materials.append(mat)
    obj.rotation_euler = (math.radians(20), 0, math.radians(40))
    return obj


def render_previews(full, lod, mouth) -> None:
    PREVIEWS.mkdir(parents=True, exist_ok=True)
    sun, cam = ho._preview_setup()
    mz = mouth.z
    my = mouth.y
    sun_front = (-0.35, -0.75, -0.55)
    shots = [
        # (fichier, objets, œil, cible, soleil, champ)
        ("station_3q_hangar", full, (430, my + 400, mz + 190), (0, 20, -10), sun_front, 60),
        ("station_3q_babord", full, (-520, my + 330, mz - 120), (0, 0, 0), (0.5, -0.7, -0.4), 60),
        ("station_anneau", full, (60, -1150, 40), (0, 0, 0), (0.3, 0.8, -0.5), 50),
        ("station_approche", full, (40, my + 520, mz + 30), (0, my - 40, mz), sun_front, 60),
        ("station_bouche", full, (60, my + 190, mz + 18), (0, my - 60, mz - 4), sun_front, 60),
        ("station_profil", full, (1250, 0, 120), (0, -10, 0), (-0.6, -0.2, -0.75), 50),
        ("station_arriere", full, (-700, -900, 380), (0, -60, 0), (0.5, 0.6, -0.6), 60),
        ("station_dessus", full, (300, 200, 1100), (0, -30, 0), (-0.3, -0.5, -0.8), 60),
        ("lod_3q_hangar", lod, (430, my + 400, mz + 190), (0, 20, -10), sun_front, 60),
        ("lod_arriere", lod, (-700, -900, 380), (0, -60, 0), (0.5, 0.6, -0.6), 60),
    ]
    for fname, objs, eye, target, sun_dir, fov in shots:
        ho._show_only(objs)
        ho._shot(sun, cam, PREVIEWS / f"{fname}.png", eye, target, sun_dir, fov)
    # De loin, sur fond noir avec la planète : modèle complet à 2,5 km et LOD à 6 km.
    planet = _planet(60000.0, (-20000, -60000, -52000))
    for fname, objs, eye in (("station_loin_2500", full, (1400, 1900, 700)),
                             ("lod_loin_6000", lod, (3300, 4600, 1700)),
                             ("station_loin_6000", full, (3300, 4600, 1700))):
        ho._show_only(list(objs) + [planet])
        cam.data.clip_end = 200000.0
        ho._shot(sun, cam, PREVIEWS / f"{fname}.png", eye, (0, -40, -60), sun_front, 50)
    bpy.data.objects.remove(planet, do_unlink=True)


# --- Programme ----------------------------------------------------------------------------------


def _args(argv):
    return {"textures": "--textures" in argv, "previews": "--previews" in argv, "export": "--no-export" not in argv}


def tri_count(objs) -> int:
    return sum(len(p.vertices) - 2 for o in objs for p in o.data.polygons)


def main(argv) -> None:
    opts = _args(argv)
    if opts["textures"] or not (c.TEXTURES_DIR / f"{HANGAR_TEX}_emissive.png").exists():
        make_hangar_textures()
    c.reset_scene()
    m = palette()
    exports = c.collection("EXPORT")
    quiet = io.StringIO()

    full_parts, proxy = build(m, c.collection("PARTS"), lod=False, prefix="stn")
    lod_parts, lod_proxy = build(m, c.collection("PARTS_LOD"), lod=True, prefix="stnlod")
    # Origine au centre de masse du modèle complet (même décalage pour la LOD) ; la tête compte
    # par son témoin fermé (la bouche du hangar, ≈ 12 % de son volume, n'est pas déduite).
    bpy.context.view_layer.update()
    com = center_of_mass(full_parts + [proxy])
    for o in (proxy, lod_proxy):
        bpy.data.objects.remove(o, do_unlink=True)
    shift = Vector((0.0, round(com.y * 2) / 2, round(com.z * 2) / 2))  # symétrique en x : x = 0
    print(f"centre de masse (construction) : {tuple(round(v, 2) for v in com)} ; décalage {tuple(shift)}")
    for obj in full_parts + lod_parts:
        obj.location = obj.location - shift
    bpy.context.view_layer.update()

    with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
        full = c.join_by_material(full_parts, NAME, exports)
        lod = c.join_by_material(lod_parts, f"{NAME}_lod", exports)
    for obj in full_parts + lod_parts:
        bpy.data.objects.remove(obj, do_unlink=True)
    # Origine des nœuds au centre du modèle (pas de translation dans le glTF).
    bpy.ops.object.select_all(action="DESELECT")
    for obj in full + lod:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = full[0]
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)

    def stats(label, objs):
        lo = Vector((min(v.co[i] for o in objs for v in o.data.vertices) for i in range(3)))
        hi = Vector((max(v.co[i] for o in objs for v in o.data.vertices) for i in range(3)))
        mats = sorted(o.data.materials[0].name for o in objs)
        print(f"{label} : {len(objs)} maillages, {tri_count(objs)} triangles, matériaux {mats}")
        print(f"  boîte (Blender) : {tuple(round(v, 1) for v in lo)} → {tuple(round(v, 1) for v in hi)}"
              f" ; dimensions {tuple(round(v, 1) for v in hi - lo)}")
        return lo, hi

    stats(NAME, full)
    stats(f"{NAME}_lod", lod)
    mouth = Vector((0.0, MOUTH_YF, (MOUTH_Z0 + MOUTH_Z1) / 2)) - shift
    print(f"centre de la bouche (Blender, plan de la face avant) : {tuple(round(v, 2) for v in mouth)}")
    print(f"centre de la bouche (Bevy, Y-haut) : {tuple(round(v, 2) for v in c.to_bevy(tuple(mouth)))}")

    if opts["export"]:
        with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
            c.export_gltf(f"{OUT}/{NAME}.gltf", full)
            c.export_gltf(f"{OUT}/{NAME}_lod.gltf", lod)
        print(f"exporté : {OUT}/{NAME}.gltf, {OUT}/{NAME}_lod.gltf")
    if opts["previews"]:
        render_previews(full, lod, mouth)
    for o in bpy.data.objects:
        if o.type == "MESH":
            o.hide_render = False
    for o in lod:
        o.hide_set(True)
    if opts["export"]:
        c.save_blend(BLEND)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
