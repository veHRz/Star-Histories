"""Installations orbitales du système d'Hexis (propositions P15 et P18), vues de loin en vue Globale.

Refonte « futur usé » de M8 (lot E, docs/refonte-visuelle.md §2) :

- `l_enclume` : la station-chantier de cuirassés, **en construction** (canon §3.5) : épine
  dorsale en treillis de ≈ 4 km, complexe d'habitation à l'arrière, ailes de radiateurs, cale en
  cadres de treillis dont l'avant n'est pas encore bâti, ponts roulants et grue à flèche, coque
  de cuirassé inachevée (bordé à l'arrière, membrures nues à l'avant) ;
- `portal_hub` : le hub de Portails d'Hexis (canon §3.7) : station de contrôle et quatre anneaux
  segmentés. L'apparence d'un Portail n'est pas décrite par le canon : anneau de ≈ 700 m à
  lueur bleue (P18), gardé tel quel ;
- `shipyard` : chantiers orbitaux (canon §3.3) : trois cales de frégates (deux occupées) sur
  une poutre de service ;
- `freighter` : cargo générique du trafic ambiant (≈ 90 m), aussi épave à remorquer et coque du
  joueur dans les données de combat.

Identité Hexa-Corp proposée (P6) : industriel, jaune et noir de chantier, bandes de danger,
pochoirs génériques (aucun nom ni logo). Les vues se font de 150 m à des dizaines de
kilomètres : à ces distances les textures se moyennent (mipmaps), le détail vient donc de la
géométrie (treillis, modules, radiateurs, grues) et de la lumière (hublots, feux de balisage,
projecteurs, lueurs). Pas de lightmap : soleil dynamique, PBR complet. Le jeu n'ombre rien au
loin (cascades de 40 m) et son ambiante est presque nulle : les faces à l'ombre sont noires,
ce sont les émissifs qui les font lire.

Matériaux (≤ 8 par modèle) : bibliothèque du lot 0 (`hull_plates` teinté en tuiles de 16 à
64 m, `metal_dark`, `container_ribbed`, `trim_ship`) et un jeu propre au lot,
`orbital_lights` (palette de feux, rangées de hublots, lueur de Portail, tuyères), calculé ici
en numpy (`--textures`, ou automatiquement s'il manque).

Une unité = 1 m ; Z-haut ; l'avant des vaisseaux vers +Y ; origines et dimensions d'ensemble
inchangées depuis M2 (les données les posent). Sorties : assets/models/space/<nom>.gltf (+ .bin,
images citées dans assets/textures/materials/) et art/space/hexis_orbitals.blend.

    blender --background --factory-startup --python tools/blender/space/hexis_orbitals.py -- \\
        [--textures] [--only <nom>[,<nom>]] [--previews] [--no-export]

`--previews` : rendus EEVEE dans target/art-previews/lotE/ (soleil dur sans ombre, exposition du
jeu : `Exposure::INDOOR`, éclairement de 400 lx).
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
from mathutils import Matrix, Vector

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "materials"))

import common as c  # noqa: E402
import texgen as tg  # noqa: E402

OUT = "assets/models/space"
PREVIEWS = c.ROOT / "target" / "art-previews" / "lotE"
Z = Vector((0.0, 0.0, 1.0))
X = Vector((1.0, 0.0, 0.0))
Y = Vector((0.0, 1.0, 0.0))

# --- Texture `orbital_lights` -----------------------------------------------------------------
# 1024² ; ligne 0 = bas de l'image = v 0 (convention de texgen). Disposition :
# - lignes 0-63 et 992-1023 : métal sombre (garde : la répétition raccorde le haut et le bas) ;
# - bandes de 64 px à partir de la ligne 64 (BANDS) : hublots, lueur de Portail, tuyère, balises,
#   répétées le long de U (une tuile = 1024 px) ;
# - lignes 864-991 : palette de feux, 8 carrés de 128 px (PATCHES), lus en leur centre.
# Émission = couleur linéaire × `LIGHTS_STRENGTH`. Dans le jeu, l'émissif s'ajoute tel quel
# avant le tonemapping (l'exposition de la caméra ne l'atténue pas, contrairement à l'éclairage
# du soleil) : repère de la salle témoin, bandeaux ≈ 4, écrans ≈ 1-2. Ici : feux (valeur 1)
# ≈ 4, saturés ; hublots ≈ 0,25-0,45 → ≈ 1-1,8, chauds ; Portail et tuyères ≈ 2-4.

LIGHTS_TEX = "orbital_lights"
LIGHTS_STRENGTH = 4.0
TEX_RES = 1024
PATCHES = {
    # nom : couleur linéaire (base et émission)
    "warm": (1.0, 0.72, 0.40),
    "cold": (0.70, 0.85, 1.0),
    "red": (1.0, 0.02, 0.01),
    "green": (0.02, 1.0, 0.12),
    "portal": (0.04, 0.24, 0.90),
    "engine": (0.03, 0.15, 0.62),
    "amber": (1.0, 0.38, 0.03),
    "beacon": (1.0, 0.70, 0.10),
}
PATCH_ROW = 864
BANDS = ["win_warm", "win_cold", "win_sparse", "win_big", "portal_inner", "engine_glow", "light_chain"]
BAND_ROW0 = 64
BAND_PX = 64


def patch_uv(name: str) -> tuple[float, float]:
    k = list(PATCHES).index(name)
    return ((k + 0.5) * 128 / TEX_RES, (PATCH_ROW + 64) / TEX_RES)


def band_uv(name: str, pad_px: float = 3.0) -> tuple[float, float]:
    k = BANDS.index(name)
    v0 = BAND_ROW0 + k * BAND_PX
    return ((v0 + pad_px) / TEX_RES, (v0 + BAND_PX - pad_px) / TEX_RES)


def _to_srgb(lin: np.ndarray) -> np.ndarray:
    lin = np.clip(lin, 0.0, 1.0)
    return np.where(lin <= 0.0031308, lin * 12.92, 1.055 * np.power(lin, 1 / 2.4) - 0.055)


def make_light_textures() -> None:
    """Calcule `orbital_lights_{color,emissive,orm}.png` (déterministe, ≈ 1 s)."""
    res = TEX_RES
    rng = np.random.default_rng(4207)
    cv = tg.Canvas(res, 4.0, seed=4207)
    # Métal sombre de fond (cadres des hublots, garde) : légère variation lente.
    base = 0.035 + 0.008 * cv.noise(1, 1.5)
    color = np.stack([base, base * 1.01, base * 1.04], -1)  # linéaire
    emis = np.zeros((res, res, 3), np.float32)
    rough = np.full((res, res), 0.62, np.float32)
    metal = np.full((res, res), 0.5, np.float32)
    occl = np.ones((res, res), np.float32)
    cols = np.arange(res)

    def band_rows(name):
        k = BANDS.index(name)
        r0 = BAND_ROW0 + k * BAND_PX
        return r0, r0 + BAND_PX

    def windows(name, period, w, h, lit_p, level, tints):
        r0, r1 = band_rows(name)
        mid = (r0 + r1) / 2
        for j in range(res // period):
            c0 = j * period + (period - w) // 2
            rr0, rr1 = int(mid - h / 2), int(mid + h / 2)
            # Encadrement un peu plus clair (joint), puis la vitre.
            color[rr0 - 2:rr1 + 2, c0 - 2:c0 + w + 2] *= 1.8
            glass_lit = rng.random() < lit_p
            if glass_lit:
                tint = np.array(tints[rng.integers(len(tints))], np.float32)
                bright = level * rng.uniform(0.55, 1.0)
                # Plus clair en bas de la vitre (plafonniers vus de l'extérieur, reflet du sol).
                grad = np.linspace(1.0, 0.75, rr1 - rr0, dtype=np.float32)[:, None, None]
                emis[rr0:rr1, c0:c0 + w] = tint * bright * grad
                color[rr0:rr1, c0:c0 + w] = tint * 0.6
            else:
                color[rr0:rr1, c0:c0 + w] = (0.012, 0.014, 0.018)
            rough[rr0:rr1, c0:c0 + w] = 0.12
            metal[rr0:rr1, c0:c0 + w] = 0.0

    warm = [(1.0, 0.78, 0.50), (1.0, 0.70, 0.40), (0.95, 0.85, 0.70)]
    cold = [(0.75, 0.88, 1.0), (0.85, 0.92, 1.0)]
    windows("win_warm", 64, 30, 30, 0.72, 0.45, warm)
    windows("win_cold", 64, 36, 34, 0.85, 0.42, cold)
    windows("win_sparse", 64, 24, 26, 0.35, 0.45, [(1.0, 0.55, 0.18), (1.0, 0.75, 0.45)])
    # Baies panoramiques : larges, meneaux tous les 128 px.
    windows("win_big", 128, 116, 50, 0.92, 0.38, cold + [(0.95, 0.9, 0.8)])

    # Lueur intérieure d'un Portail : un canal bleu dont le cœur est vif, des émetteurs tous
    # les 64 px, un joint sombre tous les 256 px (segments de l'anneau).
    r0, r1 = band_rows("portal_inner")
    v = (np.arange(r0, r1) - (r0 + r1 - 1) / 2) / (r1 - r0)
    across = np.exp(-(v / 0.22) ** 2)[:, None]
    along = 0.70 + 0.30 * np.cos(cols / 64 * 2 * np.pi)[None, :] ** 2
    seam = np.where((cols % 1024) < 10, 0.2, 1.0)[None, :]
    glow = (0.10 + 0.90 * across) * along * seam
    portal = np.array(PATCHES["portal"], np.float32)
    emis[r0:r1] = glow[..., None] * portal
    color[r0:r1] = 0.25 * portal * glow[..., None] + 0.02
    rough[r0:r1], metal[r0:r1] = 0.3, 0.0

    # Intérieur de tuyère : V = 0 à la sortie (bleu sombre), V = 1 au col (blanc bleuté).
    r0, r1 = band_rows("engine_glow")
    t = np.linspace(0, 1, r1 - r0, dtype=np.float32)[:, None, None]
    eng = np.array(PATCHES["engine"], np.float32)
    emis[r0:r1] = (0.12 + 0.88 * t ** 2) * (eng * (1 - 0.4 * t) + 0.4 * t)
    color[r0:r1] = emis[r0:r1] * 0.5
    rough[r0:r1], metal[r0:r1] = 0.4, 0.0

    # Chaîne de balises : boîtiers sombres, lampe froide carrée tous les 128 px.
    r0, r1 = band_rows("light_chain")
    for j in range(res // 128):
        c0 = j * 128 + 48
        color[r0 + 14:r1 - 14, c0 - 6:c0 + 38] *= 2.5
        emis[r0 + 20:r1 - 20, c0:c0 + 32] = PATCHES["cold"]
        color[r0 + 20:r1 - 20, c0:c0 + 32] = PATCHES["cold"]

    # Palette de feux : 8 carrés pleins, bord sombre de 8 px (le filtrage reste dans le carré).
    for k, (name, col) in enumerate(PATCHES.items()):
        c0 = k * 128
        sl = (slice(PATCH_ROW + 8, PATCH_ROW + 120), slice(c0 + 8, c0 + 120))
        emis[sl] = col
        color[sl] = col
        rough[sl], metal[sl] = 0.25, 0.0

    out = c.TEXTURES_DIR
    tg.write_png(out / f"{LIGHTS_TEX}_color.png", _to_srgb(color))
    tg.write_png(out / f"{LIGHTS_TEX}_emissive.png", _to_srgb(emis))
    tg.write_png(out / f"{LIGHTS_TEX}_orm.png", np.stack([occl, rough, metal], -1))
    print(f"textures {LIGHTS_TEX} écrites dans {out}")


# --- Matériaux --------------------------------------------------------------------------------


def palette() -> dict:
    lib = c.material_library()
    t = lib.TINTS
    return {
        # Tôles de coque d'Hexa-Corp : gris chaud, jaune de chantier, noir.
        "hull": c.textured_material("orb_hull", "hull_plates", tint=(0.70, 0.69, 0.66)),
        "yellow": c.textured_material("orb_yellow", "hull_plates", tint=t["hexa_yellow"]),
        # Coques en construction : apprêt gris-vert, pas encore peint (sans émission : en jeu,
        # même faible, elle blanchissait les panneaux).
        "primer": c.textured_material("orb_primer", "hull_plates", tint=(0.50, 0.52, 0.50)),
        "dark": c.textured_material("orb_dark", "metal_dark"),
        "ribbed": c.textured_material("orb_ribbed", "container_ribbed", tint=(0.62, 0.63, 0.65)),
        "trim": c.textured_material("orb_trim", "trim_ship"),
        "lights": c.textured_material(
            "orb_lights", LIGHTS_TEX, emissive=(1.0, 1.0, 1.0), emissive_strength=LIGHTS_STRENGTH
        ),
        # Conteneurs du cargo : deux teintes de plus que le jaune.
        "cont_rust": c.textured_material("orb_cont_rust", "container_ribbed", tint=(0.66, 0.42, 0.30)),
        "cont_blue": c.textured_material("orb_cont_blue", "container_ribbed", tint=(0.42, 0.52, 0.62)),
    }


# --- Géométrie --------------------------------------------------------------------------------


def basis_quat(d, axis: str = "Y", up=Z):
    """Rotation qui porte l'axe local `axis` sur `d`, et l'axe suivant (Y → Z, Z → Y, X → Z)
    au plus près de `up`."""
    d = Vector(d).normalized()
    up = Vector(up).normalized()
    if abs(d.dot(up)) > 0.999:
        up = X if abs(d.x) < 0.9 else Y
    perp = (up - d * up.dot(d)).normalized()
    if axis == "Y":
        y, z = d, perp
        x = y.cross(z)
    elif axis == "Z":
        z, y = d, perp
        x = y.cross(z)
    else:
        x, z = d, perp
        y = z.cross(x)
    return Matrix((x, y, z)).transposed().to_quaternion()


class Build:
    """Pièces d'un modèle : chaque pièce est un objet (UV posés), fusionné par matériau à la fin."""

    def __init__(self, prefix: str, coll, m: dict, tiles: dict, seed: int, offset: float = 0.5):
        self.prefix, self.coll, self.m, self.tiles = prefix, coll, m, tiles
        self.parts: list = []
        self.n = 0
        self.rng = random.Random(seed)
        self.offset = offset  # décollement des bandes (hublots, bandes de danger) de leur paroi

    def _name(self, tag: str) -> str:
        self.n += 1
        return f"{self.prefix}_{tag}_{self.n}"

    def _uv(self, obj, key):
        c.uv_box(obj, self.tiles.get(key, 4.0), offset=(self.rng.random(), self.rng.random()))

    def _keep(self, obj, key, uv=True):
        if uv and key != "lights":
            self._uv(obj, key)
        self.parts.append(obj)
        return obj

    @staticmethod
    def _rotate(obj, rot):
        if rot is not None:
            obj.rotation_mode = "QUATERNION"
            obj.rotation_quaternion = rot

    # Primitives ------------------------------------------------------------------------------

    def box(self, key, size, center, rot=None, bevel=0.0, tag="box"):
        obj = c.box(self._name(tag), tuple(size), tuple(center), self.m[key], self.coll, bevel=bevel)
        self._rotate(obj, rot)
        return self._keep(obj, key)

    def beam(self, key, a, b, w, h=None, up=Z, tag="beam", ext=0.0):
        """Poutre de section w × h de `a` à `b` (`ext` : rallonge à chaque bout, pour les nœuds)."""
        a, b = Vector(a), Vector(b)
        d = b - a
        length = d.length + 2 * ext
        return self.box(key, (w, length, h or w), (a + b) / 2, rot=basis_quat(d, "Y", up), tag=tag)

    def cyl(self, key, center, d, r, length, seg=16, tag="cyl", r2=None, patch="engine"):
        """Cylindre (ou tronc de cône si `r2`) d'axe `d`, centré en `center` ; en matériau
        `lights`, il prend la couleur `patch` de la palette."""
        mesh = bpy.data.meshes.new(self._name(tag))
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r, radius2=r if r2 is None else r2, depth=length)
        bm.to_mesh(mesh)
        bm.free()
        mesh.materials.append(self.m[key])
        obj = bpy.data.objects.new(mesh.name, mesh)
        obj.location = tuple(center)
        self.coll.objects.link(obj)
        self._rotate(obj, basis_quat(d, "Z"))
        if key == "lights":
            self._patch_uv(obj, patch)
        return self._keep(obj, key)

    @staticmethod
    def _patch_uv(obj, kind):
        """UV ramassés au centre d'un carré de la palette (± 1 texel : pas de triangle dégénéré)."""
        u, v = patch_uv(kind)
        layer = obj.data.uv_layers.new(name=c.UV_TEXTURE)
        for li, loop in enumerate(layer.data):
            loop.uv = (u + 0.001 * (li % 3), v + 0.001 * ((li // 3) % 2))

    def mesh(self, key, verts, faces, uvs, tag="mesh", want=None):
        """Maillage libre : `uvs[i]` donne les UV des coins de `faces[i]`. `want(centre, normale)`
        renvoie la direction voulue de la face : elle est retournée si sa normale s'y oppose
        (les matériaux masquent les faces arrière, comme le jeu)."""
        verts = [Vector(v) for v in verts]
        faces, uvs = [list(f) for f in faces], [list(u) for u in uvs]
        if want is not None:
            for i, f in enumerate(faces):
                p = [verts[k] for k in f]
                n = (p[1] - p[0]).cross(p[2] - p[0])
                if len(p) > 3:
                    n = n + (p[2] - p[0]).cross(p[3] - p[0])
                ctr = sum(p, Vector()) / len(p)
                if n.dot(Vector(want(ctr, n))) < 0:
                    faces[i].reverse()
                    uvs[i].reverse()
        me = bpy.data.meshes.new(self._name(tag))
        me.from_pydata([tuple(v) for v in verts], [], [tuple(f) for f in faces])
        me.materials.append(self.m[key])
        layer = me.uv_layers.new(name=c.UV_TEXTURE)
        k = 0
        for poly, fuv in zip(me.polygons, uvs):
            for li, uv in zip(poly.loop_indices, fuv):
                layer.data[li].uv = uv
            k += 1
        me.update()
        obj = bpy.data.objects.new(me.name, me)
        self.coll.objects.link(obj)
        self.parts.append(obj)
        return obj

    # Éléments composés ---------------------------------------------------------------------------

    def truss(self, key, a, b, w, h, chord, strut, bay, up=Z, diag=True, tag="truss", chord_key=None):
        """Poutre-caisson en treillis : quatre membrures (`chord_key`, sinon `key`), cadres et
        diagonales en V alternés."""
        a, b = Vector(a), Vector(b)
        d = b - a
        length = d.length
        y = d / length
        up = Vector(up)
        if abs(up.normalized().dot(y)) > 0.999:
            up = X if abs(y.x) < 0.9 else Y
        z = (up - y * up.dot(y)).normalized()
        x = y.cross(z)
        n = max(1, round(length / bay))
        corners = [(-w / 2, -h / 2), (w / 2, -h / 2), (w / 2, h / 2), (-w / 2, h / 2)]

        def p(t, k):
            cx, cz = corners[k % 4]
            return a + y * (length * t) + x * cx + z * cz

        for k in range(4):
            self.beam(chord_key or key, p(0, k), p(1, k), chord, up=z, tag=tag, ext=chord / 2)
        for i in range(n + 1):
            for k in range(4):
                self.beam(key, p(i / n, k), p(i / n, k + 1), strut, up=y, tag=tag)
        if diag:
            for i in range(n):
                for k in range(4):
                    t0, t1 = i / n, (i + 1) / n
                    if (i + k) % 2:
                        self.beam(key, p(t0, k), p(t1, k + 1), strut * 0.8, up=y, tag=tag)
                    else:
                        self.beam(key, p(t0, k + 1), p(t1, k), strut * 0.8, up=y, tag=tag)

    def ptruss(self, key, a, b, side, width, chord, depth, strut, bay, normal, tag="ptruss", chord_key=None):
        """Treillis plan de `a` à `b` : membrures décalées de ±`width`/2 selon `side`, épaisseur
        `depth` selon `normal` (les cadres de cale)."""
        a, b = Vector(a), Vector(b)
        s = Vector(side).normalized() * (width / 2)
        d = b - a
        n = max(1, round(d.length / bay))
        nrm = Vector(normal)
        self.beam(chord_key or key, a + s, b + s, chord, depth, up=nrm, tag=tag, ext=chord / 2)
        self.beam(chord_key or key, a - s, b - s, chord, depth, up=nrm, tag=tag, ext=chord / 2)
        for i in range(n + 1):
            q = a + d * (i / n)
            self.beam(key, q - s, q + s, strut, depth * 0.7, up=nrm, tag=tag)
        for i in range(n):
            q0, q1 = a + d * (i / n), a + d * ((i + 1) / n)
            sg = 1 if i % 2 else -1
            self.beam(key, q0 + s * sg, q1 - s * sg, strut * 0.8, depth * 0.6, up=nrm, tag=tag)

    light_scale = 1.0  # taille des pastilles de feux (à 1,5-3 km, quelques pixels au plus)

    def light(self, pos, kind: str, size: float, tag="light"):
        """Feu (cube émissif) : UV ramassés au centre d'un carré de la palette."""
        size *= self.light_scale
        obj = c.box(self._name(tag), (size, size, size), tuple(pos), self.m["lights"], self.coll)
        self._patch_uv(obj, kind)
        self.parts.append(obj)
        return obj

    def strip(self, key, center, normal, along, length, height, band, mpt, tag="strip"):
        """Bande plane (hublots, danger, pochoirs) posée devant une paroi de normale `normal`.

        `band` : nom de BANDS (texture `orbital_lights`) ou de `library.TRIMS` (`trim_ship`) ;
        U le long de `along` (`mpt` mètres par répétition), V = toute la hauteur de la bande."""
        n = Vector(normal).normalized()
        al = Vector(along).normalized()
        up = n.cross(al).normalized()
        if up.dot(Z) < -1e-6 or (abs(up.dot(Z)) < 1e-6 and up.dot(Y) < 0):
            up = -up
            al = -al
        ctr = Vector(center) + n * self.offset
        hl, hh = al * (length / 2), up * (height / 2)
        verts = [ctr - hl - hh, ctr + hl - hh, ctr + hl + hh, ctr - hl + hh]
        v0, v1 = band_uv(band) if band in BANDS else c.trim_band(band)
        if band not in BANDS:
            pad = 2.0 / 1024
            v0, v1 = v0 + pad, v1 - pad
        u0 = self.rng.random()
        u1 = u0 + length / mpt
        uv = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]
        return self.mesh(key, verts, [(0, 1, 2, 3)], [uv], tag=tag, want=lambda p, nn: n)

    def box_windows(self, center, size, rot, faces, band, rows, mpt, row_h, margin=0.12):
        """Rangées de hublots sur les faces d'un pavé (`faces` : '+x', '-x', '+y', '-y')."""
        q = rot.to_matrix() if rot is not None else Matrix.Identity(3)
        cx, cy, cz = size
        ctr = Vector(center)
        for f in faces:
            sgn = 1 if f[0] == "+" else -1
            if f[1] == "x":
                n, al, half, length = q @ X * sgn, q @ Y, cx / 2, cy * (1 - 2 * margin)
            else:
                n, al, half, length = q @ Y * sgn, q @ X, cy / 2, cx * (1 - 2 * margin)
            up = q @ Z
            for r in range(rows):
                zr = (r + 1) / (rows + 1) - 0.5
                self.strip("lights", ctr + n * half + up * (zr * cz * (1 - margin)), n, al, length, row_h, band, mpt)

    def ring_band(self, key, center, axis, r, width, band, mpt, seg=32, inward=False, a0=0.0, a1=2 * math.pi, tag="ring"):
        """Bande cylindrique (sans fonds) de rayon `r`, de largeur `width` le long de `axis` :
        hublots autour d'un tambour, lueur intérieure d'un anneau."""
        ax = Vector(axis).normalized()
        e1 = (Z - ax * Z.dot(ax)) if abs(ax.dot(Z)) < 0.99 else X.copy()
        e1.normalize()
        e2 = ax.cross(e1)
        ctr = Vector(center)
        v0, v1 = band_uv(band) if band in BANDS else c.trim_band(band)
        verts, faces, uvs = [], [], []
        full = abs(a1 - a0 - 2 * math.pi) < 1e-6
        steps = seg if full else max(1, round(seg * (a1 - a0) / (2 * math.pi)))
        for i in range(steps + 1):
            a = a0 + (a1 - a0) * i / steps
            radial = e1 * math.cos(a) + e2 * math.sin(a)
            verts += [ctr + radial * r - ax * (width / 2), ctr + radial * r + ax * (width / 2)]
        u0 = self.rng.random()
        arc = r * (a1 - a0) / steps
        for i in range(steps):
            k = 2 * i
            faces.append((k, k + 2, k + 3, k + 1))
            ua, ub = u0 + i * arc / mpt, u0 + (i + 1) * arc / mpt
            uvs.append([(ua, v0), (ub, v0), (ub, v1), (ua, v1)])

        def want(p, n):
            out = p - ctr
            out = out - ax * out.dot(ax)
            return -out if inward else out

        return self.mesh(key, verts, faces, uvs, tag=tag, want=want)

    def loft(self, key, sections, tile, skip=None, caps=(False, False), tag="loft"):
        """Coque par sections : `sections` = [(y, [(x, z), …]), …], profils dans le sens
        trigonométrique (x vers z). U = y / tile (le long de la coque), V = périmètre / tile.
        `skip(i, j)` : panneau non posé (bordé en cours)."""
        verts, faces, uvs = [], [], []
        npts = len(sections[0][1])
        for y, prof in sections:
            for x, zz in prof:
                verts.append(Vector((x, y, zz)))
        perim = []
        for y, prof in sections:
            acc = [0.0]
            for j in range(npts):
                a, b = Vector(prof[j]), Vector(prof[(j + 1) % npts])
                acc.append(acc[-1] + (b - a).length)
            perim.append(acc)
        ctr_of = []
        for y, prof in sections:
            cx = sum(p[0] for p in prof) / npts
            cz = sum(p[1] for p in prof) / npts
            ctr_of.append(Vector((cx, y, cz)))
        for i in range(len(sections) - 1):
            for j in range(npts):
                if skip and skip(i, j):
                    continue
                j1 = (j + 1) % npts
                f = (i * npts + j, (i + 1) * npts + j, (i + 1) * npts + j1, i * npts + j1)
                ya, yb = sections[i][0], sections[i + 1][0]
                uvs.append([(ya / tile, perim[i][j] / tile), (yb / tile, perim[i + 1][j] / tile),
                            (yb / tile, perim[i + 1][j + 1] / tile), (ya / tile, perim[i][j + 1] / tile)])
                faces.append(f)
        for end, on in ((0, caps[0]), (len(sections) - 1, caps[1])):
            if not on:
                continue
            base = end * npts
            faces.append(tuple(base + j for j in range(npts)))
            uvs.append([(p[0] / tile, p[1] / tile) for p in sections[end][1]])

        def want(p, n):
            # Vers l'extérieur de l'axe de la coque ; les fonds, vers l'avant ou l'arrière.
            ys = [s[0] for s in sections]
            if abs(n.normalized().y) > 0.95:
                return Vector((0, 1 if p.y > (ys[0] + ys[-1]) / 2 else -1, 0))
            k = min(range(len(ys)), key=lambda i: abs(ys[i] - p.y))
            return p - ctr_of[k]

        return self.mesh(key, verts, faces, uvs, tag=tag, want=want)

    def wedge(self, key, center, axis, ri, ro, a0, a1, depth, tag="wedge"):
        """Voussoir d'anneau (secteur de couronne en prisme), posé dans le repère du segment
        pour une projection boîte propre : X radial, Y tangent, Z = axe de l'anneau."""
        ax = Vector(axis).normalized()
        e1 = (Z - ax * Z.dot(ax)) if abs(ax.dot(Z)) < 0.99 else X.copy()
        e1.normalize()
        e2 = ax.cross(e1)
        am = (a0 + a1) / 2
        radial = e1 * math.cos(am) + e2 * math.sin(am)
        tangent = ax.cross(radial)
        rot = Matrix((radial, tangent, ax)).transposed()
        half = (a1 - a0) / 2
        verts = []
        for zz in (-depth / 2, depth / 2):
            for rr in (ri, ro):
                for s in (-1, 1):
                    verts.append((rr * math.cos(half), s * rr * math.sin(half), zz))
        # Indices : (z, r, s) → 4 * z + 2 * r + s
        faces = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 2, 6, 4), (1, 5, 7, 3), (0, 4, 5, 1), (2, 3, 7, 6)]
        me = bpy.data.meshes.new(self._name(tag))
        me.from_pydata(verts, [], faces)
        bm = bmesh.new()
        bm.from_mesh(me)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        bm.to_mesh(me)
        bm.free()
        me.materials.append(self.m[key])
        obj = bpy.data.objects.new(me.name, me)
        obj.location = tuple(Vector(center))
        obj.rotation_mode = "QUATERNION"
        obj.rotation_quaternion = rot.to_quaternion()
        self.coll.objects.link(obj)
        return self._keep(obj, key)


def octo(w, h, ch, ridge=0.0, keel=0.0):
    """Profil octogonal (demi-largeur w, demi-hauteur h, pan coupé ch), sens trigonométrique,
    avec arête dorsale (`ridge`) et quille (`keel`) facultatives."""
    pts = [(w, -h + ch), (w, h - ch), (w - ch, h)]
    if ridge:
        pts += [(w * 0.35, h + ridge * 0.6), (0.0, h + ridge), (-w * 0.35, h + ridge * 0.6)]
    pts += [(-w + ch, h), (-w, h - ch), (-w, -h + ch), (-w + ch, -h)]
    if keel:
        pts += [(0.0, -h - keel)]
    pts += [(w - ch, -h)]
    return pts


def smooth(e0, e1, x):
    t = min(1.0, max(0.0, (x - e0) / (e1 - e0)))
    return t * t * (3 - 2 * t)


def hull_sections(y0, y1, w, h, ch, zc, n, ridge=0.0, keel=0.0, bow=0.28, stern=0.9):
    """Sections d'une coque de y0 (poupe) à y1 (proue) : pleine au milieu, effilée à l'avant
    (la quille remonte), un peu resserrée à la poupe."""
    out = []
    for i in range(n + 1):
        t = i / n
        y = y0 + (y1 - y0) * t
        wf = stern + (1 - stern) * smooth(0.0, 0.15, t)
        wf *= 1 - (1 - bow) * smooth(0.62, 1.0, t)
        hf = 1 - 0.45 * smooth(0.7, 1.0, t)
        lift = h * 0.45 * smooth(0.72, 1.0, t)
        prof = [(x * wf, zc + z * hf + lift * (1 if z < 0 else 0.2)) for x, z in octo(w, h, ch, ridge, keel)]
        out.append((y, prof))
    return out


def hull_frames(B, key, sections, idx, size, tag="frame"):
    """Membrures nues : un cadre de poutres sur le profil des sections `idx`."""
    for i in idx:
        y, prof = sections[i]
        pts = [Vector((x, y, zz)) for x, zz in prof]
        for j in range(len(pts)):
            B.beam(key, pts[j], pts[(j + 1) % len(pts)], size, size * 1.4, up=Y, tag=tag, ext=size / 2)


def hull_stringers(B, key, sections, i0, i1, size, every=1, tag="stringer"):
    """Lisses longitudinales aux sommets du profil, entre les sections i0 et i1."""
    npts = len(sections[0][1])
    for j in range(0, npts, every):
        for i in range(i0, i1):
            (ya, pa), (yb, pb) = sections[i], sections[i + 1]
            B.beam(key, (pa[j][0], ya, pa[j][1]), (pb[j][0], yb, pb[j][1]), size, up=Z, tag=tag, ext=size / 2)


# --- L'Enclume --------------------------------------------------------------------------------


def l_enclume(m, coll):
    """Station-chantier de cuirassés en construction (≈ 4,5 km de la poupe au bout de l'épine)."""
    B = Build("enclume", coll, m, {"hull": 48, "yellow": 32, "primer": 40, "dark": 16, "ribbed": 24}, seed=11, offset=1.0)
    B.light_scale = 0.5
    half_w, z0, z1 = 520.0, -330.0, 330.0
    spine_z = 440.0

    # Épine dorsale : treillis-caisson (membrures claires), coursive pressurisée au centre,
    # chaîne de balises sur le dessus, modules d'ateliers et de logements.
    B.truss("dark", (0, -2080, spine_z), (0, 2050, spine_z), 130, 96, 14, 8, 130, chord_key="hull")
    B.cyl("hull", (0, -15, spine_z), Y, 26, 4070, seg=16)
    B.strip("lights", (0, -15, spine_z + 26), Z, Y, 4060, 8, "light_chain", 160)
    B.light((0, 2070, spine_z), "cold", 44)
    # Conduites le long de l'épine (fluides, énergie).
    for sx, key, r in ((-1, "yellow", 9), (1, "dark", 7)):
        B.cyl(key, (sx * 50, -15, spine_z - 30), Y, r, 4060, seg=10)
    for k, y in enumerate((-1650, -1150, -640, -120, 420, 980, 1500)):
        size = (110, 170 + 40 * (k % 2), 56)
        ctr = (0, y, spine_z + 48 + size[2] / 2)
        B.box("hull" if k % 3 else "yellow", size, ctr, bevel=5)
        B.box_windows(ctr, size, None, ("+x", "-x"), "win_warm", 2, 32, 8)
        B.strip("trim", (0, y - size[1] / 2, ctr[2] + size[2] / 2 - 6), -Y, X, size[0] - 10, 10, "hazard", 80)
        B.box("dark", (46, 46, 18), (0, y + 40, ctr[2] + size[2] / 2 + 9))
        B.light((0, y - size[1] / 2, ctr[2] + size[2] / 2 + 8), "beacon", 12)

    # Complexe d'habitation et de commandement, à l'arrière.
    blk_c, blk = Vector((0, -2250, 420)), (360, 300, 260)
    B.box("hull", blk, blk_c, bevel=12)
    B.box_windows(blk_c, blk, None, ("+x", "-x", "-y"), "win_warm", 8, 40, 9)
    B.box("dark", (380, 40, 30), blk_c + Vector((0, 120, 115)))
    B.strip("trim", blk_c + Vector((0, -blk[1] / 2, blk[2] / 2 - 10)), -Y, X, 330, 14, "hazard", 112)
    for sx in (-1, 1):
        B.strip("trim", blk_c + Vector((sx * blk[0] / 2, 0, blk[2] / 2 - 10)), X * sx, Y, 270, 14, "hazard", 112)
    # Étage de contrôle (baies panoramiques) et mât de communication.
    top_c, top = blk_c + Vector((0, -30, 170)), (220, 160, 80)
    B.box("yellow", top, top_c, bevel=8)
    B.box_windows(top_c, top, None, ("+x", "-x", "-y", "+y"), "win_big", 1, 48, 24)
    B.truss("dark", top_c + Vector((60, 30, 40)), top_c + Vector((60, 30, 420)), 22, 22, 4, 3, 30)
    B.light(top_c + Vector((60, 30, 440)), "red", 40)
    B.light(top_c + Vector((60, 30, 250)), "cold", 12)
    for k, (dx, dy) in enumerate(((-110, 60), (-60, -80), (130, -90))):
        p = blk_c + Vector((dx, dy, blk[2] / 2 + 14))
        B.cyl("dark", p, Z, 6, 28, seg=8)
        B.cyl("hull", p + Vector((0, 0, 24)), Vector((0.3, 0.4 * (k - 1), 1)), 36 - 6 * k, 5, seg=20)
    # Tambours d'habitation, de part et d'autre (rangées de hublots en couronne).
    for sx in (-1, 1):
        dc = blk_c + Vector((sx * 400, 0, 0))
        B.cyl("hull", dc, Y, 110, 520, seg=32)
        for e in (-1, 1):
            B.cyl("dark", dc + Vector((0, e * 290, 0)), Y * e, 110, 60, seg=32, r2=46)
            B.cyl("dark", dc + Vector((0, e * 330, 0)), Y * e, 20, 30, seg=12)
        for k in range(5):
            B.ring_band("lights", dc + Vector((0, -200 + 100 * k, 0)), Y, 111, 13, "win_warm", 40, seg=32)
        for e in (-1, 1):
            B.ring_band("trim", dc + Vector((0, e * 252, 0)), Y, 111.5, 16, "hazard", 128, seg=32)
            B.cyl("dark", blk_c + Vector((sx * 230, e * 80, 0)), X, 26, 120, seg=12)
        B.light(dc + Vector((sx * 112, 0, 0)), "red" if sx < 0 else "green", 22)
        B.light(dc + Vector((0, -348, 0)), "cold", 16)
    # Réservoirs et centrale sous le complexe.
    for k, (dx, dy) in enumerate(((-90, -2160), (90, -2160), (-90, -2340), (90, -2340))):
        p = Vector((dx, dy, 220))
        B.cyl("hull" if k % 2 else "yellow", p, Z, 48, 90, seg=20)
        B.cyl("dark", p + Vector((0, 0, -62)), -Z, 48, 34, seg=20, r2=14)
        B.ring_band("trim", p, Z, 48.6, 10, "hazard", 80, seg=20)
    B.truss("dark", (0, -2250, 280), (0, -2250, 170), 200, 200, 8, 5, 55)
    # Berceaux d'amarrage à la poupe.
    for k, ang in enumerate((0.0, 0.5 * math.pi, math.pi, 1.5 * math.pi)):
        d = Vector((math.cos(ang), 0, math.sin(ang)))
        a = blk_c + Vector((0, -blk[1] / 2, 0)) + d * 60
        b = a + Vector((0, -140, 0)) + d * 50
        B.truss("yellow", a, b, 16, 16, 3, 2, 28, up=d.cross(Y))
        B.box("dark", (40, 30, 40), b + Vector((0, -15, 0)))
        B.light(b + Vector((0, -32, 0)) + d * 22, "amber", 10)

    # Ailes de radiateurs (panneaux clairs nervurés, collecteur jaune).
    for i, y in enumerate((-1500, -900, -300)):
        for sx in (-1, 1):
            B.truss("dark", (sx * 66, y, spine_z), (sx * 940, y, spine_z), 26, 26, 5, 3, 40, chord_key="hull")
            B.cyl("yellow", (sx * 500, y + 18, spine_z), X, 6, 880, seg=8)
            for k, xc in enumerate((230, 450, 670, 870)):
                w = 190 if k < 3 else 150
                for e in (-1, 1):
                    B.box("ribbed", (w, 150, 3), (sx * xc, y + e * 92, spine_z), tag="radiator")
                    B.box("dark", (w, 6, 6), (sx * xc, y + e * 167, spine_z), tag="radiator_edge")
            B.light((sx * 958, y, spine_z), "red" if sx < 0 else "green", 32)

    # Cale : cadres en treillis plan tous les 300 m (membrures claires). L'avant n'est pas bâti.
    corners = [(-half_w, z0), (half_w, z0), (half_w, z1), (-half_w, z1)]
    inward = [Z, -X, -Z, X]

    def frame_side(y, k, frac=1.0):
        (xa, za), (xb, zb) = corners[k], corners[(k + 1) % 4]
        a = Vector((xa, y, za))
        b = a + (Vector((xb, y, zb)) - a) * frac
        sd = inward[k]
        B.ptruss("dark", a + sd * 20, b + sd * 20, sd, 40, 12, 28, 7, 62, Y, chord_key="hull")
        if frac < 1.0:
            # Bout de cadre inachevé : projecteur et feu d'obstacle.
            B.light(b + sd * 20, "red", 12)

    plan = {}
    for y in range(-1800, 1801, 300):
        if y <= 600:
            plan[y] = [(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)]
    plan[900] = [(0, 1.0), (3, 1.0), (1, 0.55)]
    plan[1200] = [(0, 1.0), (3, 0.6)]
    plan[1500] = [(0, 1.0)]
    plan[1800] = [(0, 0.62)]
    for y, sides in plan.items():
        for k, frac in sides:
            frame_side(y, k, frac)
        built = len(sides) == 4
        # Projecteurs de chantier aux coins intérieurs, tournés vers la coque.
        for (cx, cz) in corners:
            if built or (cz < 0 and cx < 0):
                B.light((cx * 0.9, y, cz * 0.87), "amber" if (y // 300) % 2 else "warm", 16)
        if built:
            for sx in (-1, 1):
                B.beam("dark", (sx * 60, y, spine_z - 48), (sx * 60, y, z1 - 8), 12, tag="hanger")
        # Tins de cale : appuis jaunes sous la quille du cuirassé.
        if -1400 < y < 1200:
            for sx in (-1, 1):
                B.box("yellow", (36, 34, 44), (sx * 70, y, z0 + 62), tag="tin")
    # Longerons : bas sur toute la longueur, hauts et médians seulement sur la partie bâtie ;
    # poutres de fond entre les cadres.
    rails = [(-half_w, z0, 1800), (half_w, z0, 1800), (-half_w, z1, 700), (half_w, z1, 700), (-half_w, 0, 1250), (half_w, 0, 700)]
    for k, (x, zz, y_end) in enumerate(rails):
        B.box("hull", (32, y_end + 1830, 32), (x, (y_end - 1830) / 2, zz), tag="rail")
        sgn = 1 if x > 0 else -1
        B.strip("trim", (x + sgn * 16, (y_end - 1830) / 2, zz), X * sgn, Y, y_end + 1800, 11, "hazard", 88)
        if zz == z0:
            B.strip("lights", (x + sgn * 16, (y_end - 1830) / 2, zz + 13), X * sgn, Y, y_end + 1800, 6, "light_chain", 160)
    for x in (-250, 0, 250):
        B.box("dark", (16, 3610, 16), (x, 0, z0 + 22), tag="floor")
    for x in (-half_w, half_w):
        B.light((x, 1834, z0), "cold", 20)
        B.light((x, 732, z1), "red" if x < 0 else "green", 18)

    # Galeries d'ateliers accrochées aux flancs de la cale, entre les cadres.
    for sx in (-1, 1):
        for k, y in enumerate(range(-1650, 451, 300)):
            for lvl, zz in enumerate((-150, 150)):
                if lvl == 1 and k % 2:
                    continue
                size = (40, 250, 76)
                ctr = (sx * (half_w + 36), y, zz)
                B.box("yellow" if (k + lvl) % 3 == 0 else "hull", size, ctr, bevel=3)
                B.box_windows(ctr, size, None, ("+x" if sx > 0 else "-x",), "win_sparse" if lvl else "win_warm", 3, 28, 8)

    # Ponts roulants sous les cadres hauts (jaune de chantier), avec leurs charges.
    for k, y in enumerate((-1350, -560, 420)):
        B.truss("yellow", (-half_w + 30, y, 262), (half_w - 30, y, 262), 34, 36, 6, 4, 52, up=Z)
        for sx in (-1, 1):
            B.box("yellow", (40, 70, 34), (sx * (half_w - 40), y, 282))
            B.light((sx * (half_w - 40), y + 38, 282), "beacon", 9)
        tx = (-160, 120, -40)[k]
        B.box("yellow", (60, 52, 30), (tx, y, 232))
        B.strip("trim", (tx, y - 26, 232), -Y, X, 58, 14, "hazard", 120)
        for dx in (-16, 16):
            B.beam("dark", (tx + dx, y, 217), (tx + dx, y, 214), 3)
        B.box("primer", (150, 70, 46), (tx, y, 190), bevel=4, tag="block")
        B.strip("trim", (tx, y - 35, 190), -Y, X, 140, 8, "hazard", 64)
        B.light((tx, y + 30, 216), "warm", 10)

    # Grue à flèche sur l'avant non bâti, posant un bloc de coque sur les membrures.
    mast_base = Vector((-half_w, 1050, z0))
    mast_top = Vector((-half_w, 1050, 560))
    B.truss("yellow", mast_base, mast_top, 36, 36, 7, 4, 45, up=Y)
    B.truss("yellow", mast_top + Vector((0, 0, 20)), Vector((320, 1050, 580)), 28, 32, 6, 3, 46, up=Z)
    B.truss("yellow", mast_top + Vector((0, 0, 20)), Vector((-780, 1050, 580)), 28, 32, 6, 3, 46, up=Z)
    B.box("dark", (60, 50, 80), (-750, 1050, 540))
    B.box("dark", (60, 50, 80), (-686, 1050, 540))
    B.box("yellow", (44, 40, 32), (-half_w + 34, 1050, 548))
    B.box_windows((-half_w + 34, 1050, 548), (44, 40, 32), None, ("+y", "-y"), "win_cold", 1, 20, 12)
    B.truss("yellow", mast_top + Vector((0, 0, 20)), mast_top + Vector((0, 0, 140)), 18, 18, 3, 2, 40)
    B.light(mast_top + Vector((0, 0, 150)), "red", 16)
    B.light(Vector((320, 1050, 600)), "red", 14)
    for dy in (-12, 12):
        B.beam("dark", (160, 1050 + dy, 565), (160, 1050 + dy, 262), 3)
    B.box("primer", (180, 90, 60), (160, 1050, 232), bevel=4, tag="block")
    B.strip("trim", (160, 1005, 232), -Y, X, 170, 8, "hazard", 64)

    # Coque du cuirassé : bordé posé à l'arrière, panneaux manquants au milieu, membrures nues
    # à l'avant ; quille sur toute la longueur ; tuyères posées, pas encore raccordées.
    secs = hull_sections(-1450, 1250, 240, 180, 60, -50, 54, ridge=30, keel=20)
    ys = [s[0] for s in secs]
    i_mid = next(i for i, y in enumerate(ys) if y >= -350)
    i_end = next(i for i, y in enumerate(ys) if y >= 150)
    prng = random.Random(7)
    holes = {(i, j) for i in range(i_mid, i_end) for j in range(14) if prng.random() < 0.15 + 0.7 * (i - i_mid) / (i_end - i_mid)}
    # Blocs de coque d'apprêts différents (gris ou vert) : le patchwork d'un chantier.
    blocks = {(bi, bj) for bi in range(20) for bj in range(8) if prng.random() < 0.25}

    def patch(i, j):
        return (i // 3, j // 2) in blocks

    B.loft("primer", secs[: i_end + 1], 40, skip=lambda i, j: (i, j) in holes or patch(i, j), caps=(True, False))
    B.loft("hull", secs[: i_end + 1], 40, skip=lambda i, j: (i, j) in holes or not patch(i, j))
    hull_frames(B, "primer", secs, range(i_mid, len(secs)), 10)
    hull_stringers(B, "primer", secs, i_end, len(secs) - 1, 8, every=2)
    # Numéros de blocs au pochoir sur les flancs (génériques).
    for y in (-1300, -1000, -700, -400):
        for sx in (-1, 1):
            B.strip("trim", (sx * 240, y, -40), X * sx, Y, 70, 9, "stencils", 72)
    B.box("dark", (60, 2720, 30), (0, -100, -50 - 180 - 20 - 15), tag="keel")
    for (dx, dz) in ((-90, -60), (90, -60), (-90, 50), (90, 50)):
        B.cyl("dark", (dx, -1482, -50 + dz), Y, 60, 64, seg=24, r2=44)
    # Fondations de tourelles vides et casemates sur le pont arrière.
    for y in (-1250, -1000, -760):
        B.cyl("dark", (0, y, 168), Z, 56, 16, seg=24)
        B.ring_band("trim", (0, y, 168), Z, 56.5, 8, "hazard", 80, seg=24)
    for sx in (-1, 1):
        for y in (-1200, -950, -700):
            B.box("primer", (24, 140, 40), (sx * 252, y, -30), bevel=3)
    # Échafaudages et projecteurs autour des membrures nues.
    for k, y in enumerate(range(250, 1201, 190)):
        for sx in (-1, 1):
            zz = -140 + 90 * (k % 2)
            B.box("hull", (70, 130, 6), (sx * 300, y, zz), tag="scaffold")
            B.box("yellow", (72, 4, 8), (sx * 300, y - 65, zz + 4), tag="scaffold_edge")
            B.beam("dark", (sx * 330, y - 60, zz), (sx * 330, y - 60, z0 + 30), 5)
            B.light((sx * 280, y + 55, zz + 8), "warm", 11)
    # Piles de tôles et conteneurs sur les plateformes de fond.
    for k, y in enumerate((-1600, -1000, -200, 500, 1050, 1550)):
        sx = -1 if k % 2 else 1
        B.box("dark", (90, 180, 8), (sx * 390, y, z0 + 34))
        for s in range(3):
            B.box("primer", (70, 50, 10), (sx * 390, y - 50 + 50 * s, z0 + 44 + 12 * (s % 2)))
        B.box("yellow", (28, 60, 28), (sx * 390, y + 60, z0 + 52))

    # Feux d'extrémité.
    B.light((0, 1840, z0), "cold", 40)
    B.light((0, -2482, 420), "cold", 40)
    return B.parts


# --- Hub de Portails --------------------------------------------------------------------------


def portal_ring(B, center, axis, k):
    """Un Portail : couronne de 24 voussoirs (r 300 → 380), un émetteur tous les trois, canal
    bleu sur la face intérieure, balisage."""
    ax = Vector(axis).normalized()
    ctr = Vector(center)
    e1 = Z.copy()
    e2 = ax.cross(e1)
    n = 24
    gap = math.radians(0.9)
    for i in range(n):
        a0 = 2 * math.pi * i / n + gap / 2
        a1 = 2 * math.pi * (i + 1) / n - gap / 2
        emitter = i % 3 == 0
        am = (a0 + a1) / 2
        radial = e1 * math.cos(am) + e2 * math.sin(am)
        tangent = ax.cross(radial)
        rot = basis_quat(tangent, "Y", ax)
        if emitter:
            B.wedge("yellow" if i % 6 == 0 else "hull", ctr, ax, 300, 412, a0, a1, 104, tag="emitter")
            # Ailettes de refroidissement de l'émetteur, sur le dos de l'anneau.
            for s in (-1, 0, 1):
                B.box("ribbed", (44, 72, 4), ctr + radial * 434 + ax * (s * 30), rot=rot)
            B.light(ctr + radial * 458, "cold" if i % 6 else "red", 11)
        else:
            B.wedge("hull", ctr, ax, 300, 380, a0, a1, 84, tag="voussoir")
            B.wedge("dark", ctr, ax, 334, 362, a0 + math.radians(1.6), a1 - math.radians(1.6), 89, tag="bandeau")
            # Nervures sombres aux bouts du voussoir, caisson de service sur le dos.
            for aa in (a0 + math.radians(1.2), a1 - math.radians(1.2)):
                B.wedge("dark", ctr, ax, 376, 392, aa - math.radians(0.9), aa + math.radians(0.9), 92, tag="rib")
            B.box("dark" if i % 2 else "hull", (18, 34, 46), ctr + radial * 389, rot=basis_quat(tangent, "Y", radial))
            # Bandes de danger sur les deux faces, près du bord intérieur.
            for s in (-1, 1):
                p = ctr + radial * 312 + ax * (s * 42)
                B.strip("trim", p, ax * s, tangent, 2 * 312 * math.sin(math.pi / n) - 6, 12, "hazard", 100)
    # Canal lumineux intérieur et lèvres sombres qui le bordent.
    B.ring_band("lights", ctr, ax, 298.5, 40, "portal_inner", 78.5, seg=48, inward=True)
    for s in (-1, 1):
        B.ring_band("trim", ctr + ax * (s * 26), ax, 297, 8, "edge", 80, seg=48, inward=True)
    # Feux d'approche sur les deux faces de la lèvre (blanc froid), tous les 30°.
    for i in range(12):
        a = 2 * math.pi * (i + 0.5) / 12
        radial = e1 * math.cos(a) + e2 * math.sin(a)
        for s in (-1, 1):
            B.light(ctr + radial * 306 + ax * (s * 44), "cold", 7)


def portal_hub(m, coll):
    """Station de contrôle et quatre anneaux de Portail à 1,5 km (P18)."""
    B = Build("hub", coll, m, {"hull": 32, "yellow": 24, "primer": 32, "dark": 12, "ribbed": 16}, seed=23, offset=0.6)
    B.light_scale = 0.55
    # Colonne centrale.
    B.cyl("dark", (0, 0, 0), Z, 60, 900, seg=20)
    # Tambour principal (six ponts de hublots) et bandes de danger.
    B.cyl("hull", (0, 0, 0), Z, 170, 220, seg=40)
    for k in range(6):
        B.ring_band("lights", (0, 0, -85 + 34 * k), Z, 171, 9, "win_warm", 36, seg=40)
    for s in (-1, 1):
        B.ring_band("trim", (0, 0, s * 102), Z, 171.5, 14, "hazard", 112, seg=40)
    # Nervures verticales du tambour.
    for k in range(12):
        a = 2 * math.pi * (k + 0.5) / 12
        d = Vector((math.cos(a), math.sin(a), 0))
        B.box("dark", (8, 14, 200), d * 172, rot=basis_quat(d.cross(Z), "Y"))
    # Pont de contrôle : tronc de cône, baies panoramiques froides, coiffe, mât.
    B.cyl("hull", (0, 0, 140), Z, 170, 60, seg=40, r2=124)
    B.cyl("yellow", (0, 0, 200), Z, 124, 60, seg=40)
    B.ring_band("lights", (0, 0, 200), Z, 125, 26, "win_big", 48, seg=40)
    B.cyl("hull", (0, 0, 250), Z, 124, 40, seg=40, r2=50)
    B.truss("dark", (0, 0, 270), (0, 0, 560), 24, 24, 4, 3, 30, up=Y)
    B.light((0, 0, 580), "red", 30)
    B.light((0, 0, 420), "cold", 10)
    for k in range(3):
        a = 2 * math.pi * k / 3 + 0.3
        p = Vector((math.cos(a) * 90, math.sin(a) * 90, 280))
        B.cyl("dark", p, Z, 5, 30, seg=8)
        B.cyl("hull", p + Vector((0, 0, 20)), Vector((math.cos(a), math.sin(a), 1.3)), 34, 4, seg=20)
    # Dessous : tronc de cône, réservoirs autour de la colonne, ailettes de radiateurs.
    B.cyl("hull", (0, 0, -140), Z, 120, 60, seg=40, r2=170)
    for k in range(6):
        a = 2 * math.pi * k / 6 + math.pi / 6
        p = Vector((math.cos(a) * 100, math.sin(a) * 100, -260))
        B.cyl("yellow" if k % 2 else "hull", p, Z, 34, 180, seg=16)
        B.cyl("dark", p + Vector((0, 0, -112)), -Z, 34, 44, seg=16, r2=10)
        B.ring_band("trim", p + Vector((0, 0, 60)), Z, 34.5, 8, "hazard", 64, seg=16)
    for k in range(4):
        a = math.pi / 4 + k * math.pi / 2
        d = Vector((math.cos(a), math.sin(a), 0))
        B.beam("dark", d * 60 + Vector((0, 0, -360)), d * 330 + Vector((0, 0, -360)), 10)
        for j in range(4):
            B.box("ribbed", (3, 150, 60), d * (150 + 50 * j) + Vector((0, 0, -360)), rot=basis_quat(d.cross(Z), "Y"), tag="radiator")
    # Bras d'amarrage entre les pylônes, berceaux balisés.
    for k in range(4):
        a = math.pi / 4 + k * math.pi / 2
        d = Vector((math.cos(a), math.sin(a), 0))
        B.truss("dark", d * 170, d * 420, 20, 20, 4, 3, 36, chord_key="hull")
        B.box("yellow", (50, 40, 34), d * 440, rot=basis_quat(d, "Y"))
        B.strip("trim", d * 462, d, d.cross(Z), 40, 12, "hazard", 96)
        B.light(d * 462 + Vector((0, 0, 20)), "amber", 8)
        B.light(d * 462 + Vector((0, 0, -20)), "amber", 8)
    # Bras porteurs des anneaux : treillis, coursive pressurisée, poste de contrôle au pied.
    for i, angle in enumerate((0.0, 90.0, 180.0, 270.0)):
        a = math.radians(angle)
        d = Vector((math.cos(a), math.sin(a), 0.0))
        # Le bras porte l'anneau par sa base (l'anneau regarde la station, axe radial).
        root = d * 160 - Z * 40
        foot = d * 1500 - Z * 470
        B.truss("dark", root, foot, 46, 46, 7, 4, 48, chord_key="hull")
        B.cyl("hull", (root + foot) / 2 + Z * 30, foot - root, 11, (foot - root).length - 120, seg=12)
        B.box("yellow", (90, 120, 110), foot, rot=basis_quat(d, "Y"))
        B.box_windows(foot, (90, 120, 110), basis_quat(d, "Y"), ("+x", "-x"), "win_cold", 3, 24, 6)
        B.truss("dark", foot + Z * 50, d * 1500 - Z * 395, 60, 30, 8, 5, 30, up=d, chord_key="hull")
        B.light(foot - Z * 60, "amber", 12)
        for t in (0.25, 0.5, 0.75):
            B.light(root + (foot - root) * t + Z * 30, "amber", 7)
        portal_ring(B, d * 1500, d, i)
        B.light(d * 1500 + Vector((0, 0, 470)), "red", 28)
        side = d.cross(Z)
        B.light(d * 1500 + side * 470, "green", 14)
        B.light(d * 1500 - side * 470, "red", 14)
    return B.parts


# --- Chantiers orbitaux -----------------------------------------------------------------------


def frigate(B, x, built: float):
    """Frégate générique (≈ 400 m, avant vers +Y) ; `built` : part du bordé posée (1 = finie)."""
    secs = hull_sections(-200, 200, 70, 50, 18, 0, 18, ridge=10, keel=8, bow=0.3)
    secs = [(y, [(px + x, pz) for px, pz in prof]) for y, prof in secs]
    n = len(secs) - 1
    i_end = max(2, round(n * built))
    prng = random.Random(int(x) + 3)
    if built >= 0.99:
        removed = {(i, j) for i in range(n) for j in range(14) if prng.random() < 0.06}
        grey = {(i, j) for i in range(n) for j in range(14) if prng.random() < 0.3}
        B.loft("primer", secs, 24, skip=lambda i, j: (i, j) in removed or (i // 2, j // 2) in grey, caps=(True, True))
        B.loft("hull", secs, 24, skip=lambda i, j: (i, j) in removed or (i // 2, j // 2) not in grey)
        for sx in (-1, 1):
            B.strip("trim", (x + sx * 70, -40, 0), X * sx, Y, 40, 6, "stencils", 48)
    else:
        B.loft("primer", secs[: i_end + 1], 24, skip=lambda i, j: i > i_end - 3 and prng.random() < 0.5, caps=(True, False))
        hull_frames(B, "primer", secs, range(i_end - 2, n + 1), 4)
        hull_stringers(B, "primer", secs, i_end, n, 3, every=2)
    # Tuyères (trois) et bloc-passerelle bas, sans tour.
    for dx, dz in ((-32, -10), (32, -10), (0, 20)):
        B.cyl("dark", (x + dx, -213, dz), Y, 20, 26, seg=16, r2=14)
    if built > 0.5:
        B.box("primer", (50, 90, 20), (x, -60, 60), bevel=2)
        B.box("dark", (26, 40, 14), (x, -24, 72))
        for s in (-1, 1):
            B.box("primer", (18, 120, 24), (x + s * 78, -70, -4), bevel=2)
    else:
        B.box("dark", (40, 80, 6), (x, -60, 58))


def shipyard(m, coll):
    """Trois cales de frégates sur une poutre de service ; deux occupées."""
    B = Build("yard", coll, m, {"hull": 32, "yellow": 20, "primer": 24, "dark": 10, "ribbed": 16}, seed=37, offset=0.5)
    B.light_scale = 0.6
    w, l, h = 260.0, 520.0, 200.0
    for i, x in enumerate((-700.0, 0.0, 700.0)):
        corners = [(-w, -h), (w, -h), (w, h), (-w, h)]
        inward = [Z, -X, -Z, X]
        for y in (-l / 2, 0.0, l / 2):
            for k in range(4):
                (xa, za), (xb, zb) = corners[k], corners[(k + 1) % 4]
                sd = inward[k]
                B.ptruss("dark", Vector((x + xa, y, za)) + sd * 13, Vector((x + xb, y, zb)) + sd * 13, sd, 26, 10, 20, 5, 32, Y, chord_key="hull")
        # Bardage des flancs, sur le bas (quelques panneaux pas encore posés).
        for s in (-1, 1):
            for k, yc in enumerate((-195, -65, 65, 195)):
                if (i * 4 + k + (s > 0)) % 5 == 3:
                    continue
                B.box("hull" if (k + i) % 3 else "yellow", (6, 124, 150), (x + s * (w + 4), yc, -h + 95), tag="cladding")
                B.box("dark", (8, 124, 8), (x + s * (w + 5), yc, -h + 172), tag="cladding_edge")
        for xa in (-w, w):
            for za in (-h, h):
                B.box("hull", (16, l + 16, 16), (x + xa, 0, za), tag="rail")
            sgn = 1 if xa > 0 else -1
            B.strip("trim", (x + xa + sgn * 8, 0, -h), X * sgn, Y, l, 9, "hazard", 72)
            B.strip("lights", (x + xa + sgn * 8, 0, -h + 10), X * sgn, Y, l, 4, "light_chain", 96)
        # Tours de travail sur les flancs intérieurs, passerelles vers la coque.
        for s in (-1, 1):
            for y in (-130, 130):
                tx = x + s * (w - 40)
                B.truss("yellow" if (i + (y > 0)) % 2 else "dark", (tx, y, -h + 20), (tx, y, h - 20), 22, 22, 4, 2.5, 26, up=Y, chord_key="hull")
                for zz in (-90, 0, 90):
                    B.box("dark", (w - 120, 16, 4), (x + s * (w / 2 + 20), y, zz), tag="catwalk")
                    B.light((tx - s * 14, y + 10, zz + 6), "warm", 5)
        # Pont roulant jaune et sa charge.
        yc = (-120, 80, 160)[i]
        B.truss("yellow", (x - w + 14, yc, h - 34), (x + w - 14, yc, h - 34), 22, 24, 4, 2.5, 28)
        for s in (-1, 1):
            B.box("yellow", (26, 44, 22), (x + s * (w - 22), yc, h - 20))
        tx = x + (-60, 50, 0)[i]
        B.box("yellow", (40, 34, 20), (tx, yc, h - 56))
        B.strip("trim", (tx, yc - 17, h - 56), -Y, X, 38, 9, "hazard", 72)
        load_z = (100, 120, 0)[i]
        for dx in (-8, 8):
            B.beam("dark", (tx + dx, yc, h - 66), (tx + dx, yc, load_z + 18), 1.8)
        B.box("primer", (64, 44, 34), (tx, yc, load_z), bevel=2, tag="block")
        B.light((tx, yc, h - 68), "warm", 6)
        # Projecteurs aux coins des cadres, feux de position aux angles extérieurs.
        for y in (-l / 2, 0.0, l / 2):
            for (cx, cz) in corners:
                B.light((x + cx * 0.88, y, cz * 0.84), "amber" if y else "warm", 8)
        for (cx, cz) in ((-w, h), (w, h)):
            B.light((x + cx, -l / 2 - 9, cz), "red" if cx < 0 else "green", 10)
            B.light((x + cx, l / 2 + 9, cz), "cold", 9)
        if i < 2:
            frigate(B, x, (1.0, 0.55)[i])
            # Bras de maintien de la coque et tins de cale.
            for y in (-130, 0, 130):
                for s in (-1, 1):
                    B.beam("yellow", (x + s * (w - 12), y, -40), (x + s * 78, y, -20), 9, 11)
                B.box("yellow", (30, 24, h - 70), (x, y, -h + (h - 70) / 2 + 8), tag="tin")
        else:
            # Cale vide : blocs de coque préfabriqués en attente, plancher, projecteurs.
            for k, y in enumerate((-150, -50, 50, 150)):
                B.box("primer", (100, 80, 64), (x - 80 + 160 * (k % 2), y, -150), bevel=3, tag="block")
                B.strip("trim", (x - 80 + 160 * (k % 2), y - 40, -150), -Y, X, 90, 6, "hazard", 48)
            B.box("hull", (320, 440, 6), (x, 0, -186), tag="deck")
            B.strip("trim", (x, 0, -183), Z, Y, 430, 12, "chevrons", 96)
        # Ateliers sous la cale, de part et d'autre de la poutre de service.
        for s in (-1, 1):
            ctr = (x, s * 150, -236)
            size = (220, 110, 50)
            B.box("yellow" if (i + (s > 0)) % 2 else "hull", size, ctr, bevel=3)
            B.box_windows(ctr, size, None, ("+y" if s > 0 else "-y",), "win_warm", 2, 24, 6)

    # Poutre de service sous les cales, bureaux, logements, centrale, radiateurs.
    B.truss("dark", (-1020, 0, -260), (1020, 0, -260), 70, 60, 9, 5, 60, up=Z, chord_key="hull")
    for x in (-700.0, 0.0, 700.0):
        for s in (-1, 1):
            B.beam("dark", (x + s * 120, 0, -230), (x + s * 120, 0, -h - 12), 12)
    off_c, off = Vector((0, 0, -380)), (300, 160, 140)
    B.box("hull", off, off_c, bevel=8)
    B.box_windows(off_c, off, None, ("+y", "-y", "+x", "-x"), "win_warm", 5, 28, 7)
    B.box("yellow", (320, 30, 20), off_c + Vector((0, 0, 62)))
    B.strip("trim", off_c + Vector((0, -80, -62)), -Y, X, 290, 10, "hazard", 80)
    B.truss("dark", off_c + Vector((110, 40, 70)), off_c + Vector((110, 40, 300)), 14, 14, 2.5, 1.6, 22, up=Y)
    B.light(off_c + Vector((110, 40, 308)), "red", 10)
    for xc in (-330, 330):
        ctr = (xc, 0, -350)
        size = (150, 90, 80)
        B.box("hull", size, ctr, bevel=4)
        B.box_windows(ctr, size, None, ("+y", "-y"), "win_warm", 3, 24, 6)
        B.box("dark", (40, 40, 60), (xc, 0, -280))
    for k, xc in enumerate((-560, 560)):
        p = Vector((xc, 0, -380))
        B.cyl("hull" if k else "yellow", p, X, 55, 200, seg=24)
        for e in (-1, 1):
            B.cyl("dark", p + Vector((e * 120, 0, 0)), X * e, 55, 40, seg=24, r2=18)
        B.ring_band("trim", p, X, 55.5, 10, "hazard", 80, seg=24)
        B.beam("dark", (xc, 0, -330), (xc, 0, -290), 14)
    for sx in (-1, 1):
        B.truss("dark", (sx * 1020, 0, -260), (sx * 1240, 0, -260), 30, 30, 5, 3, 36, chord_key="hull")
        for k in range(3):
            B.box("ribbed", (60, 160, 3), (sx * (1060 + 60 * k), 0, -260), tag="radiator")
        B.light((sx * 1252, 0, -260), "red" if sx < 0 else "green", 12)
    return B.parts


# --- Cargo ------------------------------------------------------------------------------------


def freighter(m, coll):
    """Cargo générique d'≈ 90 m (y −48 → 46), avant vers +Y : passerelle à l'avant, berceau à
    conteneurs sur une quille, bloc des machines à nacelles et ailes de radiateurs à l'arrière."""
    tiles = {"hull": 12, "yellow": 10, "primer": 10, "dark": 4, "ribbed": 6, "cont_rust": 6, "cont_blue": 6}
    B = Build("freighter", coll, m, tiles, seed=53, offset=0.06)
    B.light_scale = 0.8
    # Quille et conduites.
    B.box("dark", (5, 54, 5), (0, -1, 0), bevel=0.3)
    for sx in (-1, 1):
        for sz in (-1, 1):
            B.cyl("yellow" if sz > 0 else "dark", (sx * 2.9, -1, sz * 2.0), Y, 0.45, 52, seg=8)
    # Berceau : cadres tous les 12,5 m autour des piles de conteneurs, griffes jaunes.
    for y in (-26.0, -13.5, -1.0, 11.5, 24.0):
        B.beam("dark", (-14, y, 9.6), (14, y, 9.6), 1.3, 1.1, ext=0.65)
        B.beam("dark", (-14, y, -9.6), (14, y, -9.6), 1.3, 1.1, ext=0.65)
        B.beam("dark", (-14, y, -9.6), (-14, y, 9.6), 1.3, 1.1, up=Y, ext=0.65)
        B.beam("dark", (14, y, -9.6), (14, y, 9.6), 1.3, 1.1, up=Y, ext=0.65)
        B.beam("dark", (-3, y, -9.6), (-3, y, 9.6), 0.8, 0.8, up=Y)
        B.beam("dark", (3, y, -9.6), (3, y, 9.6), 0.8, 0.8, up=Y)
        for sx in (-1, 1):
            for sz in (-1, 1):
                B.box("yellow", (2.4, 1.6, 1.4), (sx * 8.2, y, sz * 9.6), tag="clamp")
    prng = random.Random(5)
    keys = ["yellow", "cont_rust", "cont_blue", "cont_rust", "hull", "cont_blue"]
    for bay, yc in enumerate((-19.75, -7.25, 5.25, 17.75)):
        for sx in (-1, 1):
            for sz in (-1, 1):
                if (bay, sx, sz) in ((1, 1, 1), (3, -1, -1)):
                    continue  # emplacements vides : on voit le berceau
                key = keys[prng.randrange(len(keys))]
                ctr = Vector((sx * 8.2, yc, sz * 4.6))
                B.box(key, (9.4, 11.0, 8.0), ctr, bevel=0.12, tag="container")
                for e in (-1, 1):
                    B.box("dark", (9.7, 0.4, 8.3), ctr + Vector((0, e * 5.6, 0)), tag="container_end")
    # Bloc avant : coque à pans coupés, nez effilé, passerelle en surplomb, verrière froide.
    nose = [(25.0, octo(9, 7, 2.6)), (38.0, octo(9, 7, 2.6)),
            (43.0, [(x, z - 0.8) for x, z in octo(7.4, 5.6, 2.2)]),
            (46.0, [(x, z - 1.6) for x, z in octo(4.4, 3.2, 1.4)])]
    B.loft("hull", nose, 12, caps=(True, True))
    B.box("yellow", (18.3, 3.2, 14.3), (0, 27.2, 0), tag="band")
    deck = [(29.0, [(5.6, 0.0), (4.2, 2.8), (-4.2, 2.8), (-5.6, 0.0)]),
            (40.0, [(5.6, 0.0), (4.2, 2.8), (-4.2, 2.8), (-5.6, 0.0)]),
            (43.5, [(4.4, 0.0), (3.2, 1.5), (-3.2, 1.5), (-4.4, 0.0)])]
    deck = [(y, [(x, z + 6.9) for x, z in prof]) for y, prof in deck]
    B.loft("yellow", deck, 10, caps=(True, True))
    # Verrière : sur le pan avant incliné de la passerelle, puis sur ses flancs.
    fn = Vector((0, 1.3, 3.5)).normalized()
    B.strip("lights", (0, 41.75, 6.9 + 2.15), fn, X, 6.6, 2.2, "win_big", 7.0)
    for sx in (-1, 1):
        sn = Vector((sx * 2.8, 0, 1.4)).normalized()
        B.strip("lights", (sx * 4.9, 34.5, 6.9 + 1.4), sn, Y, 9.5, 1.0, "win_cold", 3.0)
        B.box_windows((sx * 9, 32, -1), (0.1, 10, 7), None, ("+x" if sx > 0 else "-x",), "win_warm", 2, 2.2, 0.8)
        B.strip("trim", (sx * 9, 32, -4.8), X * sx, Y, 9, 1.4, "stencils", 11.2)
        B.strip("trim", (sx * 8.2, 40.5, -1.2), Vector((sx * 0.93, 0.36, 0)), Vector((-0.36 * sx, 0.93, 0)), 4.0, 2.4, "hazard", 19)
        B.light((sx * 3.0, 45.9, -3.4), "warm", 0.8)
        B.box("dark", (1.8, 1.8, 1.8), (sx * 9.4, 27.5, 5.8), tag="rcs")
        B.box("dark", (1.8, 1.8, 1.8), (sx * 9.4, 27.5, -5.8), tag="rcs")
        B.light((sx * 9.4, 36, 0), "red" if sx < 0 else "green", 0.9)
    B.beam("dark", (2.2, 31.0, 9.7), (2.2, 31.0, 14.5), 0.35)
    B.light((2.2, 31.0, 14.8), "beacon", 0.7)
    B.cyl("dark", (-2.2, 31.5, 10.1), Z, 0.4, 1.2, seg=8)
    B.cyl("hull", (-2.2, 31.5, 10.9), Vector((0, 0.5, 1)), 1.6, 0.25, seg=16)
    # Bloc des machines (pans coupés), nacelles, ailes de radiateurs, réservoirs, tuyères.
    aft = [(-44.0, octo(10, 7.5, 3)), (-30.0, octo(11, 8, 3)), (-25.5, octo(8, 6, 2.4))]
    B.loft("hull", aft, 12, caps=(True, True))
    B.box("yellow", (22.4, 2.0, 16.4), (0, -38, 0), tag="band")
    B.strip("trim", (0, -33, 8.0), Z, X, 14, 2.4, "hazard", 19)
    for sx in (-1, 1):
        nc = Vector((sx * 13.8, -36, -2.5))
        B.cyl("hull", nc, Y, 3.6, 18, seg=16)
        B.cyl("dark", nc + Vector((0, 10.2, 0)), Y, 3.6, 2.4, seg=16, r2=1.6)
        B.ring_band("trim", nc + Vector((0, 6, 0)), Y, 3.65, 1.2, "hazard", 9, seg=16)
        B.beam("dark", (sx * 10, -36, -2.5), (sx * 10.5, -36, -2.5), 2.0, 5)
        B.cyl("dark", nc + Vector((0, -11.2, 0)), Y, 3.4, 4.4, seg=16, r2=2.4)
        B.cyl("lights", nc + Vector((0, -13.45, 0)), -Y, 3.0, 0.1, seg=16)
        # Ailes de radiateurs horizontales : silhouette en H vue de dessus.
        B.beam("dark", (sx * 10.5, -32, 5.0), (sx * 31, -32, 5.0), 1.0, 1.0)
        for k, xc in enumerate((17.5, 26.0)):
            B.box("ribbed", (8.0, 10.0, 0.35), (sx * xc, -32, 5.0), tag="radiator")
        B.light((sx * 31.4, -32, 5.0), "red" if sx < 0 else "green", 0.9)
        B.cyl("hull", (sx * 5.2, -35.5, 9.3), Y, 2.4, 11, seg=12)
        B.ring_band("trim", (sx * 5.2, -33, 9.3), Y, 2.45, 0.9, "hazard", 7, seg=12)
        B.box_windows((0, -36, 0), (22, 10, 12), None, ("+x" if sx > 0 else "-x",), "win_sparse", 1, 2.2, 0.8, margin=0.25)
        B.strip("trim", (sx * 10.8, -36, -4.2), X * sx, Y, 8, 1.4, "stencils", 11.2)
    for dx in (-4.2, 4.2):
        p = Vector((dx, -46.3, 1.0))
        B.cyl("dark", p, Y, 3.0, 4.6, seg=16, r2=2.1)
        B.cyl("lights", p + Vector((0, -2.35, 0)), -Y, 2.6, 0.1, seg=16)
    B.cyl("dark", (0, -27.0, 0), Y, 7.0, 2.6, seg=20)
    B.light((0, -44.6, 7.6), "cold", 0.8)
    B.beam("dark", (0, -40, 7.5), (0, -40, 12.5), 0.3)
    B.light((0, -40, 12.8), "red", 0.6)
    return B.parts


ASSETS = {
    "l_enclume": l_enclume,
    "portal_hub": portal_hub,
    "shipyard": shipyard,
    "freighter": freighter,
}


# --- Aperçus ----------------------------------------------------------------------------------


def _preview_setup(width=1600, height=900):
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x, scene.render.resolution_y = width, height
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGB"
    # Fond noir (l'ambiante éclaire sans se voir) : film transparent, enregistré sans alpha.
    scene.render.film_transparent = True
    scene.view_settings.view_transform = "AgX"
    # Exposition de la caméra du jeu (Exposure::INDOOR, EV100 7) : ×1/153,6 = −7,26 IL.
    # Le jeu expose le soleil (Exposure::INDOOR, ×1/153,6) mais pas l'émissif : exposition 0 et
    # soleil, ambiante divisés par 153,6 reproduisent les deux.
    scene.view_settings.exposure = 0.0
    try:
        scene.eevee.taa_render_samples = 16
    except AttributeError:
        pass
    w = scene.world or bpy.data.worlds.new("World")
    scene.world = w
    bg = next(n for n in w.node_tree.nodes if n.type == "BACKGROUND")
    # Ambiante de la vue Globale (GlobalAmbientLight : 2,5 cd/m², bleutée).
    bg.inputs["Color"].default_value = (0.6, 0.7, 0.9, 1.0)
    bg.inputs["Strength"].default_value = 2.5 / 153.6
    sun = bpy.data.lights.get("preview_sun") or bpy.data.lights.new("preview_sun", "SUN")
    sun.energy = 400.0 / 153.6  # 400 lx (SUNLIGHT_LUX) vus à l'exposition du jeu
    sun.use_shadow = False  # le jeu n'ombre rien au-delà de 40 m
    sun.angle = math.radians(0.5)
    obj = bpy.data.objects.get("preview_sun") or bpy.data.objects.new("preview_sun", sun)
    if obj.name not in scene.collection.objects:
        scene.collection.objects.link(obj)
    cam_data = bpy.data.cameras.get("preview_cam") or bpy.data.cameras.new("preview_cam")
    cam_data.sensor_fit = "VERTICAL"
    cam_data.angle = math.radians(75)  # champ vertical de la caméra du joueur
    cam_data.clip_start = 1.0
    cam_data.clip_end = 60000.0
    cam = bpy.data.objects.get("preview_cam") or bpy.data.objects.new("preview_cam", cam_data)
    if cam.name not in scene.collection.objects:
        scene.collection.objects.link(cam)
    scene.camera = cam
    return obj, cam


def _shot(sun, cam, path: Path, eye, target, sun_dir, fov=75.0):
    cam.data.angle = math.radians(fov)
    cam.location = Vector(eye)
    d = Vector(target) - Vector(eye)
    cam.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    sun.rotation_euler = Vector(sun_dir).to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.render.filepath = str(path)
    with contextlib.redirect_stdout(io.StringIO()):
        bpy.ops.render.render(write_still=True)
    print(f"aperçu : {path}")


def _show_only(objs):
    keep = {o.name for o in objs}
    for o in bpy.data.objects:
        if o.type == "MESH":
            o.hide_render = o.name not in keep


PREVIEW_SHOTS = {
    # nom : [(fichier, œil, cible, direction du soleil (vers où va la lumière), champ)]
    "l_enclume": [
        ("enclume_3km", (2300, 1500, 1100), (0, -150, 0), (-0.5, -0.6, -0.55), 75),
        ("enclume_3km_flanc", (-2700, -900, 500), (0, -200, 0), (0.7, 0.3, -0.5), 75),
        ("enclume_detail", (650, 1900, 260), (-80, 900, -40), (-0.5, -0.6, -0.55), 60),
        ("enclume_detail_arriere", (-900, -2900, 700), (-100, -2200, 380), (0.6, 0.7, -0.4), 60),
        ("enclume_25km", (17000, 14000, 8000), (0, 0, 0), (-0.5, -0.6, -0.55), 75),
    ],
    "portal_hub": [
        ("hub_1500", (1100, -1100, 600), (300, 300, 0), (-0.4, 0.7, -0.5), 75),
        ("hub_ensemble", (2600, -3400, 1900), (0, 0, 0), (-0.4, 0.7, -0.5), 75),
        ("hub_anneau", (1700, -900, 300), (1500, 0, 0), (-0.4, 0.7, -0.5), 60),
    ],
    "shipyard": [
        ("chantiers_1500", (900, -1250, 500), (0, 0, -60), (-0.6, 0.5, -0.6), 75),
        ("chantiers_dessous", (-1200, 900, -700), (-300, 0, -150), (0.5, -0.4, -0.6), 75),
    ],
    "freighter": [
        ("cargo_150", (95, 95, 45), (0, 0, 0), (-0.5, -0.4, -0.75), 60),
        ("cargo_zoom", (95, 95, 45), (0, 10, 0), (-0.5, -0.4, -0.75), 26),
        ("cargo_150_arriere", (-80, -110, 30), (0, -5, 0), (0.6, 0.5, -0.5), 60),
        ("cargo_600", (380, 400, 160), (0, 0, 0), (-0.5, -0.4, -0.75), 75),
    ],
}


def render_previews(name: str, merged) -> None:
    PREVIEWS.mkdir(parents=True, exist_ok=True)
    sun, cam = _preview_setup()
    _show_only(merged)
    for fname, eye, target, sun_dir, fov in PREVIEW_SHOTS[name]:
        _shot(sun, cam, PREVIEWS / f"{fname}.png", eye, target, sun_dir, fov)


# --- Programme --------------------------------------------------------------------------------


def _args(argv):
    opts = {"textures": "--textures" in argv, "previews": "--previews" in argv, "export": "--no-export" not in argv, "only": None}
    if "--only" in argv:
        opts["only"] = argv[argv.index("--only") + 1].split(",")
    return opts


def main(argv) -> None:
    opts = _args(argv)
    if opts["textures"] or not (c.TEXTURES_DIR / f"{LIGHTS_TEX}_emissive.png").exists():
        make_light_textures()
    c.reset_scene()
    m = palette()
    exports = c.collection("EXPORT")
    quiet = io.StringIO()
    names = opts["only"] or list(ASSETS)
    built = {}
    for name in names:
        coll = c.collection(f"ASSET_{name}")
        parts = ASSETS[name](m, coll)
        with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
            merged = c.join_by_material(parts, name, exports)
        # Les pièces sources ne servent plus : le .blend ne garde que les maillages exportés.
        for obj in parts:
            bpy.data.objects.remove(obj, do_unlink=True)
        tris = sum(len(p.vertices) - 2 for o in merged for p in o.data.polygons)
        mats = sorted(o.data.materials[0].name for o in merged)
        print(f"{name} : {len(parts)} pièces → {len(merged)} maillages, {tris} triangles, matériaux {mats}")
        if opts["export"]:
            with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
                c.export_gltf(f"{OUT}/{name}.gltf", merged)
        built[name] = merged
    for name, merged in built.items():
        if opts["previews"]:
            render_previews(name, merged)
    for merged in built.values():
        for obj in merged:
            obj.hide_set(True)
    for o in bpy.data.objects:
        if o.type == "MESH":
            o.hide_render = False
    if opts["export"] and opts["only"] is None:
        c.save_blend("art/space/hexis_orbitals.blend")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
