"""Le spatioport d'Hexis, un quartier « futur usé » (proposition P7 ; M8, lot C).

Le canon dit seulement qu'Hexis a des spatioports (vide connu, §11.1 : « quartiers, villes et
spatioports […] d'Hexis ») : tout ici est proposé. Cinq cellules, kit d'intérieur de M8 en
style `station` (béton, dalles, pilastres noirs, jaune et noir de chantier d'Hexa-Corp, P6) :

    passerelle (le vaisseau s'y amarre, point d'attache « @ship » à son extrémité est) :
               coursive métallique, caillebotis, rambardes, cadres d'amarrage, lumière froide
    hall (salle des pas perdus) : tableaux et console des départs, bancs, kiosque, gaines
      ├── bureau (nord) : comptoir de guichet d'Hexa-Corp (pièce maîtresse), console des contrats
      ├── taverne (sud) : bar matelassé, étagères, tabourets, tables, lumière ambrée
      └── marche (ouest) : étals sous auvents de tôle, caisses, conteneurs, console du marché

Gameplay intact (non négociable) : mêmes cellules, portails, apparitions, mêmes repères
`INTERACT_` aux mêmes positions (consoles, sièges), et les trajets de l'autopilote dégagés
(`tutorial` : passerelle → hall par y = 0, x = −15, y = 4,5 → porte du bureau → console).
Inscriptions : pochoirs génériques de la trim sheet, aucun nom ni logo.

Sorties : assets/models/places/hexis_spaceport/<cellule>{,_lod}.gltf (glTF séparé, textures de
la bibliothèque citées par chemin), lightmaps dans assets/textures/lightmaps/hexis_spaceport/,
descripteur assets/data/propositions/sectors/hexis_spaceport.ron, art/places/hexis_spaceport.blend.

    blender --background --factory-startup --python tools/blender/places/hexis_spaceport.py -- \
        [--no-bake] [--lit] [--previews] [--only a,b] [--samples N] [--res 1280x720]

`--no-bake` : pas de lightmap (itération) ; `--lit` : aperçus Cycles avec les lampes ;
`--previews` : aperçus « comme dans le jeu » (lightmaps appliquées), dans target/art-previews/lotC/.
Repères Blender : X = est, Y = nord, Z = haut.
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "kit"))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "materials"))

import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import common as c  # noqa: E402
import sector_tools as st  # noqa: E402
import ship_interior as kit  # noqa: E402

SECTOR = "hexis_spaceport"
PREVIEWS = c.ROOT / "target" / "art-previews" / "lotC"

CELLS = {
    "passerelle": (-12.0, -0.1, -1.2, 1.2, 2.6),
    "hall": (-26.0, -12.4, -7.0, 7.0, 5.0),
    "bureau": (-24.0, -16.0, 7.4, 13.4, 3.0),
    "taverne": (-24.0, -16.0, -13.4, -7.4, 3.2),
    "marche": (-40.0, -26.4, -6.0, 6.0, 4.5),
}

PORTALS = [
    # Le vaisseau amarré : son sas s'ouvre ici (voir sh_data::dock).
    ("passerelle", "@ship", "Door", 0.0, 0.0, "x"),
    ("passerelle", "hall", "Opening", -12.2, 0.0, "x"),
    ("hall", "bureau", "Door", -20.0, 7.2, "y"),
    ("hall", "taverne", "Door", -20.0, -7.2, "y"),
    ("hall", "marche", "Door", -26.2, 0.0, "x"),
]

SPAWNS = [
    ("quai", "passerelle", -2.6, 0.0, 0.0, 90.0),
    ("hall", "hall", -19.0, 0.0, 0.0, -90.0),
]

COOL = [0.78, 0.88, 1.0]
WARM = [1.0, 0.8, 0.58]
SODIUM = [1.0, 0.7, 0.42]
AMBER = [1.0, 0.56, 0.24]
NEUTRAL = [1.0, 0.94, 0.86]

# Orientation d'un prop adossé à un mur (sa face +Y vers la pièce).
FACE_N, FACE_S, FACE_E, FACE_W = math.pi, 0.0, math.pi / 2, -math.pi / 2
FRONT = ((-1, 0, 0), (0, 0, 1))  # (right, up) d'une face tournée vers +Y


def palette() -> dict:
    """Palette du kit, plus le verre teinté des bouteilles de la taverne (sans texture)."""
    m = kit.palette()
    m["bottle"] = c.material("place_bottle_glass", (0.3, 0.13, 0.04), roughness=0.15)
    m["bottle_green"] = c.material("place_bottle_green", (0.09, 0.2, 0.1), roughness=0.15)
    return m


# --- Briques ----------------------------------------------------------------------------------


def _uv(obj, uv, tile=None):
    """UV de texture (1er jeu) : "box", ("box", (du, dv)), ("strip", mètres), nom de bande de
    trim ou (v0, v1), None (rien)."""
    if uv is None:
        return obj
    if uv == "box":
        c.uv_box(obj, tile)
    elif isinstance(uv, tuple) and uv and uv[0] == "box":
        c.uv_box(obj, tile, offset=uv[1])
    elif isinstance(uv, tuple) and uv and uv[0] == "strip":
        c.uv_trim(obj, (0.0, 1.0), meters_per_tile=uv[1], pad_px=0.0)
    else:
        c.uv_trim(obj, uv, meters_per_tile=tile or 2.0)
    return obj


def place(objs, center, facing):
    """Pose des objets construits autour de l'origine : rotation `facing` autour de Z, puis
    translation (comme le kit)."""
    cs, sn = math.cos(facing), math.sin(facing)
    for obj in objs:
        x, y = obj.location.x, obj.location.y
        obj.location = (center[0] + x * cs - y * sn, center[1] + x * sn + y * cs, center[2] + obj.location.z)
        obj.rotation_euler[2] += facing


class Cell:
    """Ce qu'une cellule exporte : visibles, structure (version simplifiée), colliders, repères."""

    def __init__(self, cid: str, m: dict):
        self.cid = cid
        self.m = m
        self.coll = c.collection(f"CELL_{cid}")
        self.visual, self.structure, self.colliders, self.marks, self.pickups = [], [], [], [], []
        self._group = None
        self._n = 0

    def nm(self, tag: str) -> str:
        self._n += 1
        return f"{self.cid}_{tag}{self._n}"

    # Objets construits en repère local, posés ensemble par `end(center, facing)`.
    def begin(self):
        self._group = []

    def end(self, center, facing=0.0):
        place(self._group, center, facing)
        self._group = None

    def _keep(self, obj, structure=False, collider=False):
        if self._group is not None:
            self._group.append(obj)
        if collider:
            self.colliders.append(obj)
        else:
            self.visual.append(obj)
            if structure:
                self.structure.append(obj)
        return obj

    def box(self, tag, size, center, mat, uv="box", bevel=0.0, tile=None, structure=False):
        obj = c.box(self.nm(tag), size, center, self.m[mat] if isinstance(mat, str) else mat, self.coll, bevel=bevel)
        return self._keep(_uv(obj, uv, tile), structure)

    def cyl(self, tag, radius, depth, center, mat, axis="Z", segments=12, uv="box", tile=None):
        obj = c.cylinder(self.nm(tag), radius, depth, center, self.m[mat] if isinstance(mat, str) else mat, self.coll, axis=axis, segments=segments)
        return self._keep(_uv(obj, uv, tile))

    def ori(self, tag, size, center, right, up, mat, uv="box", bevel=0.0, tile=None, structure=False):
        """Pavé d'axes (right, up, right × up) : la face +Z locale regarde vers right × up."""
        obj = c.box(self.nm(tag), size, (0, 0, 0), self.m[mat] if isinstance(mat, str) else mat, self.coll, bevel=bevel)
        r, u = Vector(right).normalized(), Vector(up).normalized()
        n = r.cross(u).normalized()
        u = n.cross(r).normalized()
        obj.matrix_basis = Matrix.Translation(Vector(center)) @ Matrix((r, u, n)).transposed().to_4x4()
        return self._keep(_uv(obj, uv, tile), structure)

    def plate(self, tag, size, center, mat, uv="box", tile=None, th=0.01):
        """Plaque tournée vers +Y (avant d'un prop construit en local)."""
        return self.ori(tag, (size[0], size[1], th), center, *FRONT, mat, uv, tile=tile)

    def col(self, tag, size, center):
        return self._keep(c.collider_box(self.nm(tag), size, center, self.coll), collider=True)

    def add(self, result, structure=False):
        """Résultat (visibles, colliders, repères) d'une pièce du kit."""
        vis, col, marks = result
        self.visual.extend(vis)
        if structure:
            self.structure.extend(vis)
        self.colliders.extend(col)
        self.marks.extend(marks)
        return result

    def shell(self, shell):
        self.visual.extend(shell["visual"])
        self.structure.extend(shell["structure"])
        self.colliders.extend(shell["colliders"])

    def light(self, tag, x, y, z, lumens, rng, color, lens="light", drop=0.0, facing=0.0):
        props = {"lumens": lumens, "range": rng, "color": color, "shadow_candidate": 0}
        return self.add(kit.ceiling_light(f"{self.cid}_{tag}", (x, y, z), facing, self.m, self.coll, props, lens=lens, drop=drop))

    def parts(self) -> dict:
        return {"visual": self.visual, "structure": self.structure, "colliders": self.colliders, "marks": self.marks, "pickups": self.pickups}


# --- Pièces propres au spatioport ---------------------------------------------------------------


def cheap_crate(k: Cell, tag, center, facing, size, variant):
    """Caisse économique (≈ 100 triangles, quatre fois moins que celle du kit, sans biseaux) :
    tôle nervurée, cerclages et cornières sombres, étiquette à pochoirs ; collider."""
    mat = "crate" if variant == "olive" else f"crate_{variant}"
    sx, sy, sz = size
    k.begin()
    k.box(f"{tag}", (sx - 0.03, sy - 0.03, sz - 0.08), (0, 0, sz / 2), mat)
    for zr in (0.035, sz - 0.035):
        k.box(f"{tag}_rim", (sx, sy, 0.07), (0, 0, zr), "dark")
    for dx in (-1, 1):
        for dy in (-1, 1):
            k.box(f"{tag}_post", (0.05, 0.05, sz - 0.14), (dx * (sx / 2 - 0.022), dy * (sy / 2 - 0.022), sz / 2), "dark")
    k.plate(f"{tag}_label", (min(sx, 0.9) * 0.55, 0.1), (0, sy / 2 - 0.008, sz * 0.55), "trim_sheet", "stencils", th=0.004)
    k.col(f"{tag}", (sx, sy, sz), (0, 0, sz / 2))
    k.end(center, facing)


def crate_stack(k: Cell, tag, base, facing, sizes, variants, hero=False):
    """Pile de caisses, `sizes` de bas en haut, décalées et tournées un peu ; `hero` : caisses
    détaillées du kit (au premier plan), sinon caisses économiques."""
    z = 0.0
    for i, (size, variant) in enumerate(zip(sizes, variants)):
        jitter = (0.04 * ((i * 37) % 5 - 2), 0.03 * ((i * 53) % 5 - 2))
        pos = (base[0] + jitter[0], base[1] + jitter[1], base[2] + z)
        rot = facing + 0.05 * ((i * 7) % 5 - 2)
        if hero:
            k.add(kit.crate(k.nm(tag), pos, rot, k.m, k.coll, size=size, variant=variant))
        else:
            cheap_crate(k, tag, pos, rot, size, variant)
        z += size[2]


def barrel(k: Cell, tag, x, y, mat="crate_slate"):
    """Fût de 200 l : virole peinte, deux cerclages sombres, bonde ; collider."""
    k.cyl(f"{tag}", 0.29, 0.88, (x, y, 0.44), mat, segments=8)
    for z in (0.26, 0.62):
        k.cyl(f"{tag}_hoop", 0.3, 0.04, (x, y, z), "dark", segments=8)
    k.cyl(f"{tag}_lid", 0.27, 0.02, (x, y, 0.89), "dark", segments=8)
    k.col(f"{tag}", (0.6, 0.6, 0.9), (x, y, 0.45))


def stool(k: Cell, tag, x, y):
    """Tabouret de bar : embase, fût, assise matelassée, repose-pieds ; collider."""
    k.cyl(f"{tag}_base", 0.21, 0.03, (x, y, 0.015), "dark", segments=12)
    k.cyl(f"{tag}_post", 0.035, 0.7, (x, y, 0.38), "bare", segments=8)
    k.cyl(f"{tag}_ring", 0.17, 0.025, (x, y, 0.3), "bare", segments=12)
    k.cyl(f"{tag}_seat", 0.2, 0.08, (x, y, 0.77), "fabric", segments=12)
    k.cyl(f"{tag}_pan", 0.17, 0.03, (x, y, 0.715), "dark", segments=12)
    k.col(f"{tag}", (0.42, 0.42, 0.82), (x, y, 0.41))


def handrail(k: Cell, tag, x0, x1, y, side):
    """Rambarde le long d'un mur de la passerelle : poteaux tous les 2 m, main courante jaune,
    lisse basse ; `side` = signe du mur (±1)."""
    for i, x in enumerate([x0 + 0.2 + 2.0 * j for j in range(int((x1 - x0 - 0.3) // 2.0) + 1)]):
        k.box(f"{tag}_post", (0.05, 0.05, 0.95), (x, y, 0.55), "dark")
        k.box(f"{tag}_bracket", (0.05, 0.2, 0.04), (x, y + side * 0.1, 0.92), "dark")
    length = x1 - x0
    k.cyl(f"{tag}_top", 0.028, length, ((x0 + x1) / 2, y, 1.0), "paint_yellow", axis="X", segments=10)
    k.cyl(f"{tag}_mid", 0.02, length, ((x0 + x1) / 2, y, 0.58), "bare", axis="X", segments=8)
    k.col(f"{tag}", (length, 0.3, 1.05), ((x0 + x1) / 2, side * (1.2 - 0.15), 0.525))


def collar(k: Cell, tag, x, half_w, h, tc, lc, depth=0.3, t=0.16):
    """Cadre d'amarrage qui épouse la section à pans coupés de la passerelle (jaune, chant
    sombre, bande de danger), du pied d'un mur au pied de l'autre."""
    pts = [(-(half_w - lc), 0.0), (-half_w, lc), (-half_w, h - tc), (-(half_w - tc), h), (half_w - tc, h), (half_w, h - tc), (half_w, lc), (half_w - lc, 0.0)]
    centroid = Vector((0.0, 0.0, h / 2))
    for i in range(len(pts) - 1):
        p0 = Vector((x, pts[i][0], pts[i][1]))
        p1 = Vector((x, pts[i + 1][0], pts[i + 1][1]))
        e = (p1 - p0).normalized()
        n = Vector((0.0, -e.z, e.y))
        mid = (p0 + p1) / 2
        if n.dot(centroid - mid) < 0:
            n = -n
        length = (p1 - p0).length + 2 * t * 0.42
        # Pavé : U = segment, V = profondeur (X), W = vers l'axe de la passerelle.
        v = Vector((1.0, 0.0, 0.0))
        if e.cross(v).dot(n) < 0:
            v = -v
        k.ori(f"{tag}_seg", (length, depth, t), tuple(mid + n * (t / 2)), tuple(e), tuple(v), "paint_yellow", bevel=0.012)
        k.ori(f"{tag}_lip", (length, 0.12, 0.012), tuple(mid + n * (t + 0.006)), tuple(e), tuple(v), "trim_sheet", "hazard")
    # Pieds : on ne traverse pas le cadre.
    for s in (-1, 1):
        k.col(f"{tag}_foot", (depth, 0.32, 2.2), (x, s * (half_w - 0.16), 1.1))


def kiosk(k: Cell, center, facing):
    """Kiosque d'information du hall : comptoir jaune et noir, panneau d'écrans, auvent éclairé."""
    k.begin()
    k.box("kiosk_body", (2.0, 0.7, 1.0), (0, 0.05, 0.5), "paint_black", bevel=0.015, structure=True)
    k.plate("kiosk_front", (1.86, 0.62), (0, 0.405, 0.56), "paint_yellow", th=0.012)
    k.plate("kiosk_band", (1.86, 0.12), (0, 0.415, 0.2), "trim_sheet", "hazard", th=0.008)
    k.plate("kiosk_label", (0.9, 0.11), (0.3, 0.415, 0.72), "trim_sheet", "stencils", th=0.006)
    k.box("kiosk_kick", (1.9, 0.06, 0.1), (0, 0.39, 0.05), "rubber")
    k.box("kiosk_top", (2.1, 0.8, 0.05), (0, 0.06, 1.03), "dark", bevel=0.01)
    k.box("kiosk_edge", (2.12, 0.03, 0.03), (0, 0.46, 1.03), "bare")
    k.plate("kiosk_desk_buttons", (0.5, 0.25), (-0.55, 0.05, 1.06), "buttons", th=0.006)
    k.box("kiosk_back", (2.2, 0.12, 2.5), (0, -0.4, 1.25), "paint_black", bevel=0.015, structure=True)
    for i, x in enumerate((-0.52, 0.52)):
        k.box("kiosk_bezel", (0.92, 0.04, 0.62), (x, -0.32, 1.72), "dark", bevel=0.01)
        k.plate("kiosk_screen", (0.84, 0.54), (x, -0.295, 1.72), "screen", tile=0.9)
    k.plate("kiosk_back_band", (2.0, 0.14), (0, -0.335, 1.2), "trim_sheet", "hazard", th=0.006)
    for s in (-1, 1):
        k.box("kiosk_side", (0.06, 0.9, 2.5), (s * 1.07, -0.05, 1.25), "dark")
        # Joues : bande jaune et pochoir à hauteur d'œil, bande de danger au pied (lisibles de loin).
        k.ori("kiosk_side_band", (0.86, 0.36, 0.01), (s * 1.105, -0.05, 1.62), (0, -s, 0), (0, 0, 1), "paint_yellow")
        k.ori("kiosk_side_label", (0.6, 0.1, 0.004), (s * 1.112, -0.05, 1.62), (0, -s, 0), (0, 0, 1), "trim_sheet", "stencils")
        k.ori("kiosk_side_hazard", (0.86, 0.2, 0.006), (s * 1.103, -0.05, 0.3), (0, -s, 0), (0, 0, 1), "trim_sheet", "hazard")
    k.box("kiosk_canopy", (2.4, 1.3, 0.1), (0, 0.15, 2.55), "paint_black", bevel=0.015, structure=True)
    k.plate("kiosk_canopy_face", (2.36, 0.08), (0, 0.805, 2.55), "paint_yellow", th=0.01)
    k.ori("kiosk_canopy_band", (2.1, 0.09, 0.02), (0, 0.55, 2.49), (1, 0, 0), (0, 1, 0), "band_warm", ("strip", 1.0))
    k.col("kiosk", (2.24, 1.0, 2.6), (0, 0.0, 1.3))
    k.end(center, facing)


def bench_table(k: Cell, tag, x, y):
    """Guéridon entre deux paires de sièges : fût, plateau, corbeille ; collider."""
    k.cyl(f"{tag}_foot", 0.24, 0.04, (x, y, 0.02), "dark", segments=12)
    k.cyl(f"{tag}_post", 0.05, 0.6, (x, y, 0.34), "bare", segments=8)
    k.box(f"{tag}_top", (0.62, 0.62, 0.04), (x, y, 0.66), "dark", bevel=0.01)
    k.box(f"{tag}_edge", (0.64, 0.64, 0.02), (x, y, 0.635), "paint_yellow")
    k.col(f"{tag}", (0.64, 0.64, 0.7), (x, y, 0.35))


def bin_(k: Cell, tag, x, y):
    """Corbeille cylindrique (tôle sombre, cerclage jaune) ; collider."""
    k.cyl(f"{tag}", 0.22, 0.8, (x, y, 0.4), "dark", segments=12)
    k.cyl(f"{tag}_ring", 0.225, 0.08, (x, y, 0.62), "paint_yellow", segments=12)
    k.cyl(f"{tag}_lid", 0.2, 0.04, (x, y, 0.82), "rubber", segments=12)
    k.col(f"{tag}", (0.46, 0.46, 0.85), (x, y, 0.42))


def door_sign(k: Cell, tag, x, y, z, facing, accent="paint_yellow", width=1.1):
    """Caisson lumineux au-dessus d'une porte : cadre noir, liseré de couleur, pochoirs, bande
    lumineuse (cuite)."""
    k.begin()
    k.box(f"{tag}_box", (width, 0.12, 0.36), (0, 0.06, 0), "paint_black", bevel=0.01)
    k.plate(f"{tag}_accent", (width - 0.06, 0.06), (0, 0.125, 0.13), accent, th=0.008)
    k.plate(f"{tag}_label", (width - 0.2, 0.11), (0, 0.126, 0.0), "trim_sheet", "stencils", th=0.006)
    k.plate(f"{tag}_glow", (width - 0.2, 0.05), (0, 0.126, -0.12), "trim_sheet", "light_strip", th=0.006)
    k.end((x, y, z), facing)


def counter_segment(k: Cell, tag, xa, xb, y_front, depth, facing):
    """Tronçon du comptoir de guichet d'Hexa-Corp (pièce maîtresse du bureau) : caisson noir,
    façade jaune à panneaux et couvre-joints, bande de danger, tablette de transaction à chant de
    métal brut, vitrage à montants noirs et hygiaphone, poste de l'agent derrière. Construit en
    repère local (façade vers +Y, à y = 0), posé en (milieu de [xa, xb], y_front) selon `facing`.
    Collider plein (on ne passe pas)."""
    L = xb - xa
    k.begin()
    k.box(f"{tag}_body", (L, 0.55, 1.0), (0, -0.275, 0.5), "paint_black", bevel=0.012, structure=True)
    k.box(f"{tag}_kick", (L - 0.04, 0.06, 0.12), (0, -0.02, 0.06), "rubber")
    # Façade : panneaux jaunes d'environ 0,9 m, séparés par des couvre-joints noirs.
    n = max(1, round(L / 0.9))
    w = L / n
    for i in range(n):
        x = -L / 2 + (i + 0.5) * w
        k.ori(f"{tag}_panel", (w - 0.08, 0.6, 0.014), (x, 0.007, 0.58), *FRONT, "paint_yellow", ("box", (0.37 * i, 0.21 * i)), bevel=0.004)
        k.ori(f"{tag}_panel_label", (min(0.5, w - 0.3), 0.1, 0.004), (x - 0.1, 0.016, 0.78), *FRONT, "trim_sheet", "stencils")
    for i in range(n + 1):
        x = min(max(-L / 2 + i * w, -L / 2 + 0.03), L / 2 - 0.03)
        k.ori(f"{tag}_batten", (0.06, 0.66, 0.02), (x, 0.01, 0.58), *FRONT, "paint_black")
    k.ori(f"{tag}_hazard", (L - 0.04, 0.12, 0.008), (0, 0.004, 0.2), *FRONT, "trim_sheet", "hazard")
    # Tablette de transaction (en saillie) et son chant.
    k.box(f"{tag}_ledge", (L + 0.02, 0.48, 0.05), (0, -0.2, 1.03), "dark", bevel=0.01, structure=True)
    k.box(f"{tag}_ledge_edge", (L + 0.04, 0.035, 0.035), (0, 0.045, 1.035), "bare")
    # Poste de l'agent : plan de travail bas derrière le vitrage, armoires au mur.
    k.box(f"{tag}_desk", (L, 0.36, 0.05), (0, -0.73, 0.8), "dark")
    k.box(f"{tag}_desk_body", (L, 0.3, 0.775), (0, -0.76, 0.3875), "dark")
    yg = -0.42
    posts = max(2, round(L / 1.0) + 1)
    for i in range(posts):
        x = -L / 2 + 0.04 + i * (L - 0.08) / (posts - 1)
        k.box(f"{tag}_mullion", (0.06, 0.07, 1.22), (x, yg, 1.66), "paint_black")
    k.box(f"{tag}_glass", (L - 0.08, 0.012, 1.14), (0, yg, 1.64), "glass")
    k.box(f"{tag}_rail", (L, 0.09, 0.06), (0, yg, 2.27), "paint_black")
    # Hygiaphone : petite grille au milieu du vitrage, à hauteur de bouche.
    k.ori(f"{tag}_grille", (0.24, 0.16, 0.006), (0, yg + 0.012, 1.48), *FRONT, "trim_sheet", "vent")
    k.col(f"{tag}", (L, depth, 1.1), (0, -depth / 2, 0.55))
    k.end(((xa + xb) / 2, y_front, 0.0), facing)


def bar_counter(k: Cell, tag, xa, xb, y_front, depth, facing):
    """Bar de la taverne (façade vers +Y local) : caisson sombre, façade matelassée, repose-pieds
    de métal brut, plateau à chant brillant, bandeau ambré sous la lèvre (cuit)."""
    L = xb - xa
    k.begin()
    k.box(f"{tag}_body", (L, depth, 1.0), (0, -depth / 2, 0.5), "dark", bevel=0.012, structure=True)
    k.ori(f"{tag}_front", (L - 0.1, 0.72, 0.05), (0, 0.025, 0.5), *FRONT, "fabric", bevel=0.02)
    k.ori(f"{tag}_kick", (L - 0.04, 0.1, 0.03), (0, 0.015, 0.06), *FRONT, "rubber")
    t0, t1 = -depth - 0.02, 0.17
    k.box(f"{tag}_top", (L + 0.08, t1 - t0, 0.06), (0, (t0 + t1) / 2, 1.05), "dark", bevel=0.015, structure=True)
    k.box(f"{tag}_top_edge", (L + 0.1, 0.04, 0.04), (0, t1, 1.05), "bare")
    k.ori(f"{tag}_glow", (L - 0.2, 0.06, 0.02), (0, 0.1, 1.005), (1, 0, 0), (0, 1, 0), "band_amber", ("strip", 1.0))
    k.cyl(f"{tag}_footrail", 0.025, L - 0.2, (0, 0.2, 0.2), "bare", axis="X", segments=8)
    for x in (-L / 2 + 0.3, 0.0, L / 2 - 0.3):
        k.box(f"{tag}_footrail_post", (0.04, 0.2, 0.04), (x, 0.1, 0.2), "bare")
    k.col(f"{tag}", (L, depth + 0.22, 1.1), (0, (0.22 - depth) / 2, 0.55))
    k.end(((xa + xb) / 2, y_front, 0.0), facing)


def keg(k: Cell, tag, x, y):
    k.cyl(f"{tag}", 0.28, 0.82, (x, y, 0.41), "bare", segments=10)
    for z in (0.12, 0.41, 0.7):
        k.cyl(f"{tag}_hoop", 0.29, 0.05, (x, y, z), "dark", segments=10)
    k.cyl(f"{tag}_valve", 0.06, 0.06, (x, y, 0.85), "paint_yellow", segments=8)
    k.col(f"{tag}", (0.6, 0.6, 0.9), (x, y, 0.45))


def back_bar(k: Cell, tag, xa, xb, y_wall):
    """Étagères du bar contre le mur sud : meuble bas, trois planches, bouteilles, bandeau chaud
    sous chaque planche."""
    L = xb - xa
    xc = (xa + xb) / 2
    k.box(f"{tag}_cab", (L, 0.42, 0.9), (xc, y_wall + 0.21, 0.45), "dark", bevel=0.01, structure=True)
    k.ori(f"{tag}_cab_doors", (L - 0.08, 0.7, 0.012), (xc, y_wall + 0.426, 0.47), (-1, 0, 0), (0, 0, 1), "fabric")
    k.box(f"{tag}_cab_top", (L + 0.04, 0.46, 0.04), (xc, y_wall + 0.23, 0.92), "bare")
    k.box(f"{tag}_back", (L, 0.03, 1.3), (xc, y_wall + 0.015, 1.65), "dark")
    for j, z in enumerate((1.25, 1.65, 2.05)):
        k.box(f"{tag}_shelf", (L, 0.26, 0.03), (xc, y_wall + 0.13, z), "dark")
        k.ori(f"{tag}_shelf_glow", (L - 0.1, 0.03, 0.01), (xc, y_wall + 0.24, z - 0.02), (1, 0, 0), (0, 1, 0), "band_warm", ("strip", 1.0))
        # Bouteilles par groupes (trous réguliers), deux verres, corps et goulot.
        count = int(L / 0.12)
        for i in range(count):
            if (i + 2 * j) % 5 == 4 or (i * 7 + j * 3) % 9 == 0:
                continue
            x = xa + 0.07 + i * (L - 0.14) / max(1, count - 1)
            body = 0.15 + 0.05 * ((i * 5 + j) % 3)
            rad = 0.034 + 0.008 * ((i + 2 * j) % 2)
            glass = "bottle" if (i // 3 + j) % 3 else "bottle_green"
            y = y_wall + 0.11 + 0.03 * ((i * 3 + j) % 2)
            k.cyl(f"{tag}_bottle", rad, body, (x, y, z + 0.015 + body / 2), glass, segments=5)
            k.cyl(f"{tag}_neck", 0.012, 0.09, (x, y, z + 0.015 + body + 0.045), glass, segments=4)
    k.col(f"{tag}", (L, 0.45, 2.1), (xc, y_wall + 0.225, 1.05))


def stall(k: Cell, tag, center, facing, body: str, roof: str, wares: list):
    """Étal du marché (face vers +Y local) : comptoir de tôle nervurée, plateau sombre, marchandises,
    poteaux, auvent de tôle incliné, bandeau chaud sous l'auvent, enseigne à pochoirs. Le stock
    (caisses) est posé derrière par l'appelant."""
    k.begin()
    k.box(f"{tag}_body", (2.4, 0.8, 0.92), (0, 0, 0.46), body, bevel=0.01, structure=True)
    k.box(f"{tag}_kick", (2.3, 0.05, 0.1), (0, 0.4, 0.05), "rubber")
    k.box(f"{tag}_top", (2.5, 0.9, 0.05), (0, 0.02, 0.945), "dark", bevel=0.01)
    k.box(f"{tag}_edge", (2.52, 0.03, 0.03), (0, 0.47, 0.945), "bare")
    k.plate(f"{tag}_label", (1.0, 0.11), (0.5, 0.405, 0.72), "trim_sheet", "stencils", th=0.005)
    for i, (dx, size, mat) in enumerate(wares):
        k.box(f"{tag}_ware", size, (dx, 0.05 + 0.04 * (i % 2), 0.97 + size[2] / 2), mat, bevel=0.006)
    for s in (-1, 1):
        k.box(f"{tag}_post", (0.08, 0.08, 2.55), (s * 1.16, -0.5, 1.275), "dark")
        k.box(f"{tag}_strut", (0.05, 1.1, 0.05), (s * 1.16, 0.02, 2.42), "dark")
    # Auvent : de 2,6 m à l'arrière à 2,3 m à l'avant (au-dessus de la tête : sans collider).
    k.ori(f"{tag}_roof", (2.6, 1.5, 0.04), (0, 0.05, 2.47), (1, 0, 0), (0, math.cos(0.2), -math.sin(0.2)), roof)
    k.ori(f"{tag}_roof_edge", (2.62, 0.1, 0.04), (0, 0.8, 2.32), (-1, 0, 0), (0, 0, 1), "paint_black")
    k.ori(f"{tag}_sign", (1.4, 0.1, 0.006), (0, 0.825, 2.32), (-1, 0, 0), (0, 0, 1), "trim_sheet", "stencils")
    k.ori(f"{tag}_glow", (2.2, 0.07, 0.02), (0, 0.6, 2.27), (1, 0, 0), (0, 1, 0), "band_warm", ("strip", 1.0))
    k.col(f"{tag}", (2.5, 0.9, 1.0), (0, 0.02, 0.5))
    for s in (-1, 1):
        k.col(f"{tag}_post", (0.12, 0.12, 2.55), (s * 1.16, -0.5, 1.275))
    k.end(center, facing)


# --- Cellules -----------------------------------------------------------------------------------


def build_passerelle(m: dict) -> Cell:
    cid = "passerelle"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [(-9.0, 0.0), (-5.0, 0.0), (-1.3, 0.0)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, PORTALS, {}), m, k.coll,
                           floor_strip=True, style="industrial", avoid=lamps))
    for i, (x, y) in enumerate(lamps):
        k.light(f"l{i}", x, y, h, 3500.0, 7.0, COOL)
    # Cadres d'amarrage (côté vaisseau, au milieu, côté hall).
    for i, x in enumerate((-0.55, -6.0, -11.6)):
        collar(k, f"collar{i}", x, 1.2, h, 0.45, 0.3)
    for s in (-1, 1):
        handrail(k, f"rail{s}", -11.4, -0.75, s * 0.97, s)
    # Bandes de danger au sol aux deux bouts, chevrons de cheminement vers le hall.
    k.add(kit.hazard_stripe(k.nm("stripe"), (-11.2, -0.8, 0.0), (-11.2, 0.8, 0.0), m, k.coll, width=0.16))
    k.add(kit.hazard_stripe(k.nm("stripe"), (-1.0, -0.8, 0.0), (-1.0, 0.8, 0.0), m, k.coll, width=0.16))
    # Équipement mural (derrière les rambardes, entre les pans coupés).
    k.add(kit.wall_screen(k.nm("dock_screen"), (-1.6, -1.2, 1.55), FACE_S, m, k.coll, size=(0.6, 0.4)))
    k.add(kit.junction_box(k.nm("jbox"), (-1.7, 1.2, 1.45), FACE_N, m, k.coll, size=(0.45, 0.14, 0.5)))
    k.add(kit.junction_box(k.nm("jbox"), (-8.0, -1.2, 1.5), FACE_S, m, k.coll, size=(0.5, 0.14, 0.55)))
    k.add(kit.vent(k.nm("vent"), (-4.0, 1.2, 1.55), FACE_N, m, k.coll, size=(0.7, 0.28)))
    k.add(kit.vent(k.nm("vent"), (-10.0, 1.2, 1.55), FACE_N, m, k.coll, size=(0.7, 0.28)))
    for i, (x, y, f) in enumerate(((-3.0, -1.2, FACE_S), (-7.0, 1.2, FACE_N), (-10.8, -1.2, FACE_S))):
        k.add(kit.stencil(k.nm("stencil"), (x, y, 1.9), f, m, k.coll, length=1.2))
    # Gyrophares rouges de part et d'autre du sas du vaisseau.
    for s, f in ((-1, FACE_S), (1, FACE_N)):
        k.add(kit.beacon(k.nm("beacon"), (-1.0, s * 1.2, 2.0), f, m, k.coll, lumens=700.0, rng=3.5))
    return k


def build_hall(m: dict) -> Cell:
    cid = "hall"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [(x, y) for x in (-22.0, -16.4) for y in (-5.0, -1.0, 3.0)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, PORTALS, {}), m, k.coll,
                           style="station", avoid=lamps))
    for i, (x, y) in enumerate(lamps):
        k.light(f"l{i}", x, y, h, 22000.0, 12.0, SODIUM, drop=0.9)
    # Départs : console tournée vers le hall (π de plus qu'en M7), tableau au mur au-dessus.
    k.add(kit.console(f"{cid}_departures", (-13.4, 4.5, 0.0), math.pi / 2, m, k.coll, label="departures"))
    k.add(kit.wall_screen(k.nm("board"), (-12.4, 3.15, 3.0), FACE_E, m, k.coll, size=(2.6, 1.0)))
    k.add(kit.wall_screen(k.nm("board"), (-16.0, 7.0, 3.2), FACE_N, m, k.coll, size=(3.0, 1.1)))
    k.add(kit.stencil(k.nm("stencil"), (-12.4, 3.15, 2.25), FACE_E, m, k.coll, length=2.0))
    k.add(kit.stencil(k.nm("stencil"), (-16.0, 7.0, 2.4), FACE_N, m, k.coll, length=2.4))
    # Bancs (mêmes sièges, mêmes repères qu'en M7), guéridons et corbeilles entre les paires.
    for row, y in enumerate((-2.2, 2.2)):
        for i, x in enumerate((-23.0, -21.6, -18.4, -17.0)):
            k.add(kit.seat(f"{cid}_bench{row}_{i}", (x, y, 0.0), 0.0 if row == 0 else math.pi, m, k.coll))
        bench_table(k, f"gueridon{row}", -20.0, y)
        bin_(k, f"bin{row}", -24.2, y)
    # Kiosque déplacé : l'apparition « hall » (−19, 0) tombait dans son collider.
    kiosk(k, (-23.8, -6.2, 0.0), 0.0)
    # Portes : caissons lumineux (bureau jaune, taverne ambre, marché).
    door_sign(k, "sign_bureau", -20.0, 7.0, 2.95, FACE_N, accent="paint_yellow", width=1.4)
    door_sign(k, "sign_taverne", -20.0, -7.0, 2.95, FACE_S, accent="paint_yellow", width=1.4)
    door_sign(k, "sign_marche", -26.0, 0.0, 2.95, FACE_W, accent="paint_yellow", width=1.4)
    # Sol : couloir de sortie de la passerelle (bandes jusqu'au hall).
    for s in (-1, 1):
        k.add(kit.hazard_stripe(k.nm("lane"), (-12.6, s * 1.45, 0.0), (-15.6, s * 1.45, 0.0), m, k.coll, width=0.12))
    # Gaine et chemin de câbles en hauteur.
    k.add(kit.duct(k.nm("duct"), (-25.8, 6.35, 4.05), (-12.6, 6.35, 4.05), m, k.coll, size=(0.7, 0.45), hang_to=h))
    k.add(kit.cable_tray(k.nm("tray"), (-19.2, -6.8, 4.55), (-19.2, 6.8, 4.55), m, k.coll, hang_to=h))
    k.add(kit.pipe(k.nm("pipe"), (-25.6, -6.8, 3.55), (-25.6, 6.8, 3.55), m, k.coll, radius=0.08, hang_to=None))
    # Murs : boîtiers, bouches, gyrophares au débouché de la passerelle.
    k.add(kit.junction_box(k.nm("jbox"), (-26.0, 3.6, 1.5), FACE_W, m, k.coll, size=(0.6, 0.16, 0.7)))
    k.add(kit.junction_box(k.nm("jbox"), (-14.6, -7.0, 1.5), FACE_S, m, k.coll, size=(0.5, 0.14, 0.6)))
    k.add(kit.vent(k.nm("vent"), (-24.0, 7.0, 2.6), FACE_N, m, k.coll, size=(1.0, 0.4)))
    k.add(kit.vent(k.nm("vent"), (-16.0, -7.0, 2.6), FACE_S, m, k.coll, size=(1.0, 0.4)))
    for s in (-1, 1):
        k.add(kit.beacon(k.nm("beacon"), (-12.4, s * 1.95, 2.45), FACE_E, m, k.coll, lumens=800.0, rng=4.0))
    # Fret en attente : caisses jaunes d'Hexa-Corp dans les angles (aucune sur le point de vue
    # des captures de l'avant, (−13 ; −6,5)).
    crate_stack(k, "crate", (-13.2, 6.2, 0.0), 0.1, [(1.2, 1.0, 1.0), (0.8, 0.7, 0.6)], ["yellow", "yellow"], hero=True)
    crate_stack(k, "crate", (-14.45, 6.35, 0.0), -0.15, [(0.9, 0.9, 0.8)], ["yellow"])
    crate_stack(k, "crate", (-25.3, 6.2, 0.0), 0.05, [(1.0, 1.0, 1.0), (0.9, 0.8, 0.7)], ["yellow", "yellow"])
    crate_stack(k, "crate", (-15.5, -6.35, 0.0), -0.08, [(1.1, 1.0, 0.9)], ["yellow"])
    return k


def build_bureau(m: dict) -> Cell:
    cid = "bureau"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [(-22.6, 9.6), (-17.4, 9.6), (-20.0, 11.6)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, PORTALS, {}), m, k.coll,
                           style="station", rib_every=8.0 / 3.0, avoid=lamps))
    for i, (x, y) in enumerate(lamps[:2]):
        k.light(f"l{i}", x, y, h, 9000.0, 8.0, NEUTRAL)
    k.light("l_guichet", lamps[2][0], lamps[2][1], h, 5000.0, 6.0, COOL)
    # Guichet : la console des contrats (même place qu'en M7) dans l'alcôve du comptoir.
    k.add(kit.console(f"{cid}_contracts", (-20.0, 12.9, 0.0), math.pi, m, k.coll, label="contracts"))
    y_front = 12.2
    depth = y1 - y_front
    counter_segment(k, "counter_w", -23.55, -20.75, y_front, depth, math.pi)
    counter_segment(k, "counter_e", -19.25, -16.45, y_front, depth, math.pi)
    # Joues de l'alcôve (bandes de danger) et tapis devant le guichet.
    for s, x in ((-1, -20.75), (1, -19.25)):
        k.ori("alcove_cheek", (0.6, 1.0, 0.02), (x - s * 0.01, y_front + 0.3, 0.5), (0, s, 0), (0, 0, 1), "paint_black")
        k.ori("alcove_hazard", (0.5, 0.14, 0.006), (x - s * 0.023, y_front + 0.3, 0.22), (0, s, 0), (0, 0, 1), "trim_sheet", "hazard")
    k.ori("alcove_mat", (1.3, 0.9, 0.012), (-20.0, y_front - 0.25, 0.006), (1, 0, 0), (0, 1, 0), "rubber")
    # Fond de l'alcôve, au-dessus de la console : panneau noir à liseré jaune, écran de service.
    k.box("alcove_back", (1.44, 0.04, 0.68), (-20.0, y1 - 0.02, 1.94), "paint_black")
    k.ori("alcove_back_stripe", (1.4, 0.06, 0.008), (-20.0, y1 - 0.044, 1.64), *FRONT, "paint_yellow")
    k.add(kit.wall_screen(k.nm("alcove_screen"), (-20.0, y1 - 0.04, 2.0), FACE_N, m, k.coll, size=(0.9, 0.34)))
    # Fronton au-dessus de tout le comptoir : caisson noir, liseré jaune, écran de file d'attente,
    # pochoirs, bandeau lumineux vers le bas.
    fx0, fx1 = -23.55, -16.45
    fl = fx1 - fx0
    fc = (fx0 + fx1) / 2
    fy = y_front + 0.1  # face avant du fronton
    k.box("fascia", (fl, 0.5, 0.42), (fc, fy + 0.25, 2.49), "paint_black", bevel=0.012, structure=True)
    k.ori("fascia_stripe", (fl - 0.04, 0.08, 0.01), (fc, fy - 0.006, 2.64), *FRONT, "paint_yellow")
    k.ori("fascia_hazard", (fl - 0.04, 0.1, 0.008), (fc, fy - 0.005, 2.34), *FRONT, "trim_sheet", "hazard")
    for i, x in enumerate((-22.4, -17.6)):
        k.ori("fascia_label", (1.6, 0.11, 0.006), (x, fy - 0.004, 2.5), *FRONT, "trim_sheet", "stencils")
    k.box("queue_bezel", (0.9, 0.05, 0.3), (-20.0, fy - 0.024, 2.49), "dark", bevel=0.008)
    k.ori("queue_screen", (0.8, 0.22, 0.01), (-20.0, fy - 0.054, 2.49), *FRONT, "screen", tile=0.8)
    k.ori("fascia_glow", (fl - 0.3, 0.08, 0.02), (fc, fy + 0.12, 2.27), (1, 0, 0), (0, 1, 0), "band_warm", ("strip", 1.0))
    # Derrière le vitrage : armoires, écrans et classeurs de l'agence (entre les pilastres).
    for i, x in enumerate((-23.05, -16.95)):
        k.add(kit.cabinet(k.nm("cabinet"), (x, y1, 0.0), FACE_N, m, k.coll, size=(1.0, 0.3, 1.9)))
    for i, x in enumerate((-22.05, -17.95)):
        k.add(kit.wall_screen(k.nm("back_screen"), (x, y1, 1.75), FACE_N, m, k.coll, size=(0.8, 0.5)))
    for i, (x, w_) in enumerate(((-23.0, 0.5), (-21.6, 0.35), (-18.3, 0.45), (-16.9, 0.3))):
        k.box("binder", (w_, 0.28, 0.2), (x, y_front + 0.72, 0.925), "crate_slate", bevel=0.005)
    for i, x in enumerate((-22.3, -17.7)):
        k.box("terminal", (0.42, 0.06, 0.3), (x, y_front + 0.8, 1.0), "dark", bevel=0.006)
        k.ori("terminal_screen", (0.36, 0.24, 0.006), (x, y_front + 0.767, 1.0), *FRONT, "screen", tile=0.6)
    # Salle d'attente : le bureau et son siège (repère inchangé), armoires déplacées au sud-est.
    k.add(kit.table(f"{cid}_desk", (-21.8, 10.6, 0.0), 0.0, m, k.coll, size=(1.8, 0.9)))
    k.add(kit.seat(f"{cid}_seat0", (-21.8, 9.8, 0.0), 0.0, m, k.coll))
    k.add(kit.locker(f"{cid}_locker0", (-16.3, 8.35, 0.0), -math.pi / 2, m, k.coll))
    k.add(kit.locker(f"{cid}_locker1", (-16.3, 9.0, 0.0), -math.pi / 2, m, k.coll))
    k.add(kit.wall_screen(k.nm("notice"), (-24.0, 11.45, 1.6), FACE_W, m, k.coll, size=(1.1, 0.7)))
    k.add(kit.stencil(k.nm("stencil"), (-24.0, 11.45, 2.1), FACE_W, m, k.coll, length=1.2))
    k.add(kit.junction_box(k.nm("jbox"), (-16.0, 11.0, 1.4), FACE_E, m, k.coll, size=(0.45, 0.14, 0.55)))
    k.add(kit.vent(k.nm("vent"), (-22.8, 7.4, 1.85), FACE_S, m, k.coll, size=(0.8, 0.3)))
    bin_(k, "bin", -23.6, 7.9)
    # File d'attente : deux lignes au sol de la porte à l'alcôve (rien qui gêne le passage).
    for s in (-1, 1):
        k.add(kit.hazard_stripe(k.nm("queue"), (-20.0 + s * 0.85, 8.0, 0.0), (-20.0 + s * 0.85, 11.8, 0.0), m, k.coll, width=0.08))
    return k


def build_taverne(m: dict) -> Cell:
    cid = "taverne"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [(-22.4, -9.6), (-19.0, -9.6), (-21.0, -12.0), (-17.0, -8.2)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, PORTALS, {}), m, k.coll,
                           style="station", avoid=lamps))
    for i, (x, y) in enumerate(lamps[:2]):
        k.light(f"l{i}", x, y, h, 4500.0, 6.0, AMBER, lens="light_amber", drop=0.85)
    k.light("l_bar", lamps[2][0], lamps[2][1], h, 3000.0, 5.0, WARM, lens="light_amber", drop=0.5)
    k.light("l_coin", lamps[3][0], lamps[3][1], h, 1800.0, 5.0, AMBER, lens="light_amber", drop=0.6)
    # Bar : console « bar » (même place qu'en M7) au bout du comptoir, côté est.
    k.add(kit.console(f"{cid}_tap", (-17.6, -12.9, 0.0), 0.0, m, k.coll, label="bar"))
    bar_counter(k, "bar", -22.9, -18.45, -11.8, 0.65, 0.0)
    for i, x in enumerate((-18.95, -18.75, -18.55)):
        k.cyl("tap", 0.02, 0.28, (x, -12.0, 1.22), "bare", segments=8)
        k.box("tap_handle", (0.03, 0.03, 0.12), (x, -12.0, 1.42), "paint_yellow" if i == 1 else "paint_black")
    back_bar(k, "backbar_w", -23.6, -20.35, y0)
    back_bar(k, "backbar_e", -19.65, -18.4, y0)
    k.add(kit.wall_screen(k.nm("menu"), (-24.0, -10.5, 1.75), FACE_W, m, k.coll, size=(1.0, 0.6)))
    for i, x in enumerate((-22.2, -21.2, -20.2, -19.2)):
        stool(k, f"stool{i}", x, -11.35)
    # Tables et sièges (mêmes repères qu'en M7), mugs.
    for i, (x, y) in enumerate(((-22.4, -9.6), (-19.0, -9.6))):
        k.add(kit.table(f"{cid}_table{i}", (x, y, 0.0), 0.0, m, k.coll, size=(1.2, 1.2)))
        k.add(kit.seat(f"{cid}_seat{i}a", (x - 1.0, y, 0.0), -math.pi / 2, m, k.coll))
        k.add(kit.seat(f"{cid}_seat{i}b", (x + 1.0, y, 0.0), math.pi / 2, m, k.coll))
        for j, (dx, dy) in enumerate(((-0.25, 0.2), (0.3, -0.15))):
            k.cyl("mug", 0.045, 0.12, (x + dx, y + dy + 0.05 * i, 0.85), "bare", segments=8)
    # Mange-debout dans l'angle nord-est.
    k.cyl("high_foot", 0.25, 0.04, (-17.0, -8.2, 0.02), "dark", segments=12)
    k.cyl("high_post", 0.05, 1.0, (-17.0, -8.2, 0.54), "bare", segments=8)
    k.cyl("high_top", 0.38, 0.05, (-17.0, -8.2, 1.06), "dark", segments=16)
    k.col("high_table", (0.78, 0.78, 1.1), (-17.0, -8.2, 0.55))
    # Réserve : fûts et caisse à l'ouest du bar.
    keg(k, "keg0", -23.55, -12.0)
    keg(k, "keg1", -23.5, -11.35)
    keg(k, "keg2", -23.45, -8.0)
    k.add(kit.junction_box(k.nm("jbox"), (-16.0, -11.5, 1.5), FACE_E, m, k.coll, size=(0.45, 0.14, 0.55)))
    k.add(kit.vent(k.nm("vent"), (-24.0, -12.6, 2.4), FACE_W, m, k.coll, size=(0.8, 0.3)))
    k.add(kit.pipe(k.nm("pipe"), (-23.85, -7.6, 2.95), (-16.15, -7.6, 2.95), m, k.coll, radius=0.06, hang_to=h))
    return k


def build_marche(m: dict) -> Cell:
    cid = "marche"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [(x, y) for x in (-38.0, -34.0, -30.0) for y in (-2.0, 2.0)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, PORTALS, {}), m, k.coll,
                           style="station", avoid=lamps))
    for i, (x, y) in enumerate(lamps):
        k.light(f"l{i}", x, y, h, 14000.0, 10.0, WARM, drop=0.9)
    # Console du marché (même place qu'en M7), tableau des cours au-dessus.
    k.add(kit.console(f"{cid}_terminal", (-28.4, 5.5, 0.0), math.pi, m, k.coll, label="market"))
    k.add(kit.wall_screen(k.nm("prices"), (-30.0, y1, 2.85), FACE_N, m, k.coll, size=(2.4, 0.9)))
    k.add(kit.stencil(k.nm("stencil"), (-30.0, y1, 2.2), FACE_N, m, k.coll, length=2.2))
    # Étals face à l'allée centrale, stock derrière.
    wares = [
        [(-0.7, (0.5, 0.4, 0.3), "crate_yellow"), (-0.1, (0.4, 0.35, 0.25), "crate_slate"), (0.6, (0.6, 0.4, 0.2), "bare")],
        [(-0.6, (0.6, 0.45, 0.35), "crate_rust"), (0.3, (0.5, 0.4, 0.15), "dark"), (0.8, (0.3, 0.3, 0.3), "crate_yellow")],
        [(-0.8, (0.4, 0.4, 0.4), "crate_slate"), (-0.2, (0.5, 0.3, 0.12), "bare"), (0.55, (0.55, 0.45, 0.3), "crate_rust")],
        [(-0.5, (0.7, 0.4, 0.25), "crate_yellow"), (0.4, (0.4, 0.35, 0.35), "crate_rust")],
    ]
    stalls = [((-36.5, -3.2), 0.0, "crate_slate", "crate_rust"), ((-31.5, -3.2), 0.0, "crate_rust", "crate_slate"),
              ((-36.5, 3.2), math.pi, "crate_rust", "crate_yellow"), ((-31.5, 3.2), math.pi, "crate_slate", "crate_rust")]
    for i, ((x, y), f, body, roof) in enumerate(stalls):
        stall(k, f"stall{i}", (x, y, 0.0), f, body, roof, wares[i])
        back = -1 if y < 0 else 1
        crate_stack(k, f"stock{i}", (x - 0.6, y + back * 1.55, 0.0), 0.2 * i, [(0.8, 0.7, 0.7), (0.6, 0.5, 0.5)], ["yellow" if i % 2 else "slate", "rust"])
        crate_stack(k, f"stock{i}b", (x + 0.7, y + back * 1.6, 0.0), -0.1 * i, [(0.7, 0.7, 0.6)], ["slate" if i % 2 else "yellow"])
    # Conteneurs le long du mur ouest, palettes près de l'entrée.
    for i, (y, variant) in enumerate(((-4.6, "rust"), (-2.0, "slate"), (2.2, "yellow"), (4.7, "slate"))):
        crate_stack(k, f"container{i}", (-39.15, y, 0.0), math.pi / 2, [(2.3, 1.3, 1.25)] + ([(2.0, 1.2, 1.1)] if i in (0, 2) else []), [variant, "slate" if variant != "slate" else "rust"])
    crate_stack(k, "pallet", (-27.4, -4.9, 0.0), 0.12, [(1.2, 1.0, 0.8), (0.8, 0.7, 0.6)], ["yellow", "slate"])
    crate_stack(k, "pallet", (-28.8, -5.2, 0.0), -0.2, [(0.9, 0.9, 0.9)], ["rust"])
    crate_stack(k, "pallet", (-26.95, 5.25, 0.0), 0.1, [(0.8, 0.8, 0.7), (0.6, 0.5, 0.45)], ["slate", "yellow"])
    # Fûts au fond de l'allée et contre les murs, chariot chargé dans l'allée (côté sud).
    for i, (x, y, mat) in enumerate(((-39.45, -0.45, "crate_slate"), (-39.45, 0.2, "paint_yellow"),
                                     (-34.0, -5.5, "crate_rust"), (-33.35, -5.55, "crate_slate"), (-34.1, 5.45, "paint_yellow"))):
        barrel(k, f"barrel{i}", x, y, mat)
    k.begin()
    k.box("cart_deck", (1.3, 0.75, 0.06), (0, 0, 0.2), "dark", bevel=0.01)
    k.box("cart_frame", (1.34, 0.79, 0.04), (0, 0, 0.15), "paint_yellow")
    k.box("cart_handle", (0.04, 0.7, 0.9), (0.66, 0, 0.65), "paint_black")
    for dx in (-0.5, 0.5):
        for dy in (-0.3, 0.3):
            k.cyl("cart_wheel", 0.07, 0.05, (dx, dy, 0.07), "rubber", axis="Y", segments=8)
    k.col("cart", (1.4, 0.8, 1.1), (0.0, 0, 0.55))
    k.end((-33.6, -1.45, 0.0), 0.08)
    crate_stack(k, "cart_load", (-33.75, -1.45, 0.23), 0.12, [(0.7, 0.6, 0.5), (0.5, 0.45, 0.35)], ["yellow", "slate"])
    # En hauteur : chemins de câbles et gaine en travers.
    k.add(kit.cable_tray(k.nm("tray"), (-39.8, 0.9, 3.7), (-26.6, 0.9, 3.7), m, k.coll, hang_to=h))
    k.add(kit.pipe(k.nm("pipe"), (-39.8, -0.9, 3.85), (-26.6, -0.9, 3.85), m, k.coll, radius=0.06, clamps=2.0, hang_to=h))
    k.add(kit.duct(k.nm("duct"), (-39.8, -4.6, 3.35), (-26.6, -4.6, 3.35), m, k.coll, size=(0.6, 0.4), hang_to=h))
    k.add(kit.junction_box(k.nm("jbox"), (-26.4, 3.2, 1.5), FACE_E, m, k.coll, size=(0.5, 0.14, 0.6)))
    k.add(kit.junction_box(k.nm("jbox"), (-34.0, -6.0, 1.6), FACE_S, m, k.coll, size=(0.6, 0.14, 0.7)))
    k.add(kit.vent(k.nm("vent"), (-34.0, 6.0, 2.6), FACE_N, m, k.coll, size=(1.0, 0.4)))
    door_sign(k, "sign_hall", -26.4, 0.0, 2.95, FACE_E, accent="paint_yellow", width=1.4)
    return k


BUILDERS = {
    "passerelle": build_passerelle,
    "hall": build_hall,
    "bureau": build_bureau,
    "taverne": build_taverne,
    "marche": build_marche,
}


# --- Aperçus ------------------------------------------------------------------------------------

GAME_LENS = 13.2  # champ vertical de 75° du jeu (`player.rs`), en 16:9 sur un capteur de 36 mm
EYE = 1.65


def _look(eye, yaw_deg, pitch_deg=0.0):
    import preview as pv

    return pv.look_from(eye, yaw_deg, pitch_deg)


def shots() -> dict:
    """Prises de vue : nom → (œil, cible, focale). Les deux premières reprennent les captures de
    l'avant (`SH_SPAWN_AT` : repère du jeu x, y, z → Blender x, −z, y)."""
    hall_eye = (-13.0, -6.5, EYE)
    bureau_eye = (-20.0, 7.9, EYE)
    return {
        "hall_avant": (hall_eye, _look(hall_eye, 45.0), GAME_LENS),
        "bureau_avant": (bureau_eye, _look(bureau_eye, 0.0, -5.0), GAME_LENS),
        "hall_depuis_passerelle": ((-12.0, 0.0, EYE), (-26.0, 0.6, 1.6), 16.0),
        "hall_vers_passerelle": ((-24.5, 1.0, EYE), (-12.4, -1.5, 1.6), 16.0),
        "hall_tuto": ((-15.0, 4.5, EYE), (-20.0, 7.2, 1.4), 16.0),
        "hall_kiosque": ((-19.6, -3.6, EYE), (-23.8, -6.2, 1.3), 18.0),
        "comptoir": ((-21.3, 10.3, 1.6), (-20.0, 12.5, 1.25), 20.0),
        "comptoir_face": ((-20.0, 9.0, EYE), (-20.0, 12.6, 1.35), 16.0),
        "bureau_retour": ((-17.2, 11.6, EYE), (-22.0, 7.6, 1.2), 16.0),
        "taverne": ((-20.0, -7.9, EYE), (-20.6, -13.0, 1.1), 15.0),
        "taverne_bar": ((-17.2, -9.0, EYE), (-22.0, -12.6, 1.2), 18.0),
        "marche": ((-26.9, 0.4, EYE), (-38.0, 0.0, 1.5), 15.0),
        "marche_console": ((-31.0, 1.0, EYE), (-28.4, 5.6, 1.2), 18.0),
        "passerelle": ((-1.6, 0.0, EYE), (-12.0, 0.0, 1.4), 16.0),
        "passerelle_quai": ((-10.5, 0.3, EYE), (0.0, 0.0, 1.3), 16.0),
    }


def _arg(argv, name, default):
    return argv[argv.index(name) + 1] if name in argv else default


def main(argv) -> None:
    bake = "--no-bake" not in argv
    samples = int(_arg(argv, "--samples", "48"))
    rw, rh = (int(v) for v in _arg(argv, "--res", "1280x720").split("x"))
    only = _arg(argv, "--only", None)
    only = only.split(",") if only else None
    c.reset_scene()
    m = palette()
    cells = {cid: build(m) for cid, build in BUILDERS.items()}
    bpy.context.view_layer.update()
    built = {cid: k.parts() for cid, k in cells.items()}
    meta = {
        "script": "tools/blender/places/hexis_spaceport.py",
        "comment": "Le spatioport d'Hexis, quartier « futur usé » (proposition P7 ; M8) : cellules, portails, apparitions.",
        "id": SECTOR,
        "kind": "District",
        "name_key": f"sector.{SECTOR}",
        "status": 'Proposition("P7")',
        "door_model": "models/kit/ship_interior/door_station.gltf",
        "models": f"models/places/{SECTOR}",
        "lightmaps": f"textures/lightmaps/{SECTOR}",
    }
    result = st.export_sector(meta, CELLS, PORTALS, SPAWNS, built, f"assets/data/propositions/sectors/{SECTOR}.ron",
                              f"art/places/{SECTOR}.blend", bake=bake)
    print(f"{SECTOR} : {len(CELLS)} cellules, {len(PORTALS)} portails exportés")
    if "--lit" not in argv and "--previews" not in argv:
        return
    # Les aperçus montrent ce qui est exporté (maillages fusionnés) et les portes du jeu.
    merged = {o for s in result["stats"].values() for o in s["merged"]}
    for obj in bpy.data.objects:
        if obj.type == "MESH":
            obj.hide_render = obj not in merged
    doors = st.door_panels(PORTALS, CELLS, m, c.collection("PREVIEW_DOORS"), style="station")
    bpy.context.view_layer.update()
    marks = [mk for k in cells.values() for mk in k.marks]
    if "--lit" in argv:
        for o in doors:
            o.hide_render = False
        st.preview_lit(shots(), marks, PREVIEWS, "lit_", samples=samples, res=(rw, rh), only=only)
    if "--previews" in argv and result["lightmaps"]:
        st.preview_game(shots(), result["stats"], result["lightmaps"], PREVIEWS, "jeu_", extra=doors,
                        samples=max(8, samples // 3), res=(rw, rh), only=only)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
