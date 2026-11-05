"""Le spatioport par défaut : hangar, berceau d'amarrage et salles (proposition P7 ; M9, lot D).

Tout lieu d'amarrage qui ne déclare pas son propre spatioport utilise ce secteur (Hexis aussi,
docs/m9-retours.md §2). Le canon ne décrit aucun spatioport : tout ici est proposé. Il reprend
les salles du spatioport d'Hexis de M8 (`places/hexis_spaceport.py`, dont ce script est parti),
aux mêmes coordonnées, et remplace la passerelle par un hangar où le vaisseau du joueur repose
dans un berceau, son pont au niveau du sol :

               bureau                chantier (bureau du chantier, fenêtre sur le hangar)
                  │                     │
    marche ── hall ── couloir ── HANGAR (vaisseau dans son berceau, porte de baie au nord)
                  │  └── banque
               taverne

    hangar   : coque haute (`kit/hangar.py`), fermes en treillis, projecteurs, porte de baie,
               berceau (fosse de PIT_DEPTH m), passerelle plane jusqu'au portail `hangar__ship`
               (b = « @ship », la porte extérieure du sas du vaisseau), portique, conteneurs, bornes
    couloir  : coursive industrielle (l'ancienne passerelle), porte vers le hangar
    chantier : comptoir du chantier naval (PNJ `shipyard`), écrans, pièces détachées
    hall     : hall des arrivées (M10) : allée centrale de l'entrée (couloir) à la porte du
               marché, allée transversale du bureau à la taverne, banquettes dos à dos tournées
               vers deux grands panneaux d'information (écrans à lignes de glyphes, aucun texte
               lisible), bornes des départs, comptoir d'information, chariots à bagages, bacs à
               plantes, signalétique au sol (lignes, chevrons)
    bureau   : comptoir des contrats (PNJ `contracts`), console des contrats, tableau d'affichage
    banque   : salle de la Banque (M10, PNJ `bank`) : guichets derrière des vitres, porte de coffre
               massive en décor, écrans de cours, colonnes, sol de dalles sombres, plantes
    taverne  : bar (PNJ `bar`, plateau à 1,05 m), console du bar, tables (passage de 1,85 m de
               la porte au tavernier, M10)
    marche   : étals, comptoir du marchand (PNJ `market`), console du marché

PNJ de comptoir (docs/m9-retours.md §3.4) : `SOCKET_<nom>_npc` (pied du PNJ, regard vers +Y
local) et `INTERACT_<nom>` (`interact: "npc"`, `role`, `variant`, `half_extents` qui couvre le
PNJ et le comptoir). Comptoirs du kit (`kit.counter`) ; le bar suit les mêmes cotes.

Le vaisseau (§3.8) : les cotes de l'intérieur (lot de l'intérieur `corvette`) sont les
constantes `SHIP_*` en tête, dans le repère du jeu du vaisseau (x tribord, y haut, −z avant) ;
le berceau, la passerelle et la taille du hangar en découlent (le sas est tourné vers l'ouest,
vers le quai). Le jeu pose l'intérieur pour que sa porte `sas__@dock` coïncide avec
`hangar__ship`, dont la normale est tournée vers le vaisseau (corrigée dans le descripteur :
`sector_tools.portal_record` la tourne vers l'extérieur de la cellule). Pour le bake, un volume
gris qui tient la place de la coque (objet courbe, jamais exporté) fait de l'ombre dans la fosse.

Décor par niveau de qualité (§3.2) : niveau 0 sobre (le Bas), 1 (Moyen : pannes, tuyaux,
conteneurs en second rang…), 2 (Haut : coursive haute, gaines, chaînes, outillage, fouillis).
Aucun décor optionnel n'a de collider : les gros objets au sol sont de niveau 0.
Inscriptions : pochoirs génériques de la trim sheet, aucun nom ni logo.

Sorties : assets/models/places/spaceport/<cellule>{,_lod}.gltf, lightmaps dans
assets/textures/lightmaps/spaceport/, descripteur assets/data/propositions/sectors/spaceport.ron,
art/places/spaceport.blend.

    blender --background --factory-startup --python tools/blender/places/spaceport.py -- \
        [--no-bake] [--lit] [--previews] [--only a,b] [--samples N] [--res 1280x720]

`--no-bake` : pas de lightmap (itération) ; `--lit` : aperçus Cycles avec les lampes ;
`--previews` : aperçus « comme dans le jeu » (lightmaps appliquées). Aperçus dans
target/art-previews/D/. Repères Blender : X = est, Y = nord, Z = haut.
"""

from __future__ import annotations

import contextlib
import math
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "kit"))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "materials"))

import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import common as c  # noqa: E402
import hangar as hg  # noqa: E402
import lightmap as lm  # noqa: E402
import sector_tools as st  # noqa: E402
import ship_interior as kit  # noqa: E402

SECTOR = "spaceport"
PREVIEWS = c.ROOT / "target" / "art-previews" / "D"

# --- Le vaisseau amarré (cotes du lot de l'intérieur, repère du jeu du vaisseau) ---------------
# Cotes de l'intérieur type `corvette` (lot B de M9, définitives) : boîte englobante des cellules,
# cloisons comprises ; centre et normale de la porte extérieure du sas (`sas__@dock`, 1,4 × 2,2 m,
# dans la paroi x ∈ [−7,8 ; −7,6], z ∈ [2,3 ; 3,7]).
SHIP_BBOX = ((-7.8, -0.2, -20.6), (7.8, 3.6, 26.2))
SHIP_HATCH = (-7.7, 1.1, 3.0)
SHIP_HATCH_NORMAL = (-1.0, 0.0, 0.0)
# Coque extérieure (lot C) : tout tient dans cette boîte + HULL_MARGIN sur chaque face ; ses patins
# (point le plus bas, y = −1,7 du jeu) reposent sur les sabots du berceau ; au sas, un module
# d'embarquement dont la peau est à x = −9,3, traversé par un tunnel de 1,4 × 2,2 m jusqu'à la
# porte. Marges par direction du vaisseau (toutes égales aujourd'hui).
HULL_MARGIN = 1.5
HULL_SIDE = HULL_MARGIN  # bâbord et tribord
HULL_BOW = HULL_MARGIN  # nez (avant, −z du jeu)
HULL_STERN = HULL_MARGIN  # tuyères (arrière)
HULL_TOP = HULL_MARGIN
HULL_BOTTOM = HULL_MARGIN
PIT_DEPTH = 3.0  # fosse sous le pont
# Épaisseur du patin de caoutchouc posé sur chaque sabot (`hangar.docking_cradle`, `saddle_pad`) :
# le vaisseau repose sur le patin, pas sur le sabot (M10 : il s'y enfonçait de 8 cm). Vérifié
# après construction (`check_saddles`).
SADDLE_PAD = 0.08
PIT_CLEARANCE = 1.2  # jeu entre la coque et la paroi de la fosse (bras de maintien)
BAKE_SHIP_OCCLUDER = True  # coque grise dans le bake (ombre dans la fosse)

# --- Le hangar ---------------------------------------------------------------------------------
HANGAR_X0 = -4.0  # mur ouest (le couloir et le chantier y débouchent)
HANGAR_H = 12.0
APRON_W = 10.0  # quai : sol entre le mur ouest et la fosse
EAST_W = 5.0
NORTH_W = 5.0  # côté porte de baie (proue)
SOUTH_W = 7.0  # portique et conteneurs
BAY = 8.0  # pas des fermes de toit et des colonnes
TRUSS_DEPTH = 1.6
BAY_DOOR_H = 9.0
HATCH_Y = 0.0  # le sas face au couloir : du hall au vaisseau en ligne droite
# Résolution maximale de la lightmap du hangar. `lightmap.MAX_RES` (1024) donne ≈ 2 texels/m à
# cette salle de 40 × 66 × 12 m : ses milliers de petits îlots (fermes, projecteurs, garde-corps)
# ont chacun 2,5 texels de marge, qui mangent l'atlas. En attendant un réglage par cellule dans
# `sector_tools.export_sector` / `lightmap.prepare` (demandé à l'orchestrateur), ce script relève
# la borne pour la seule cellule `hangar` (`_prepare_with_hangar_res`).
HANGAR_LIGHTMAP_MAX = 2048
# Épaisseur minimale d'un îlot de lightmap, en texels. Sous un texel, un îlot (lisse de garde-corps
# de 5 cm à 9 texels/m) ne reçoit aucun texel cuit : il reste noir en Bas et en Moyen. Après le
# dépliage commun, les îlots trop fins sont épaissis le long de leur petit axe, puis l'atlas est
# réempaqueté (`_thicken_islands`).
MIN_ISLAND_TEXELS = 2.0
# Un îlot est « étroit » le long d'un axe principal sous cette largeur (texels) : une barre ou un
# tuyau dépliés d'un seul tenant (leurs faces côte à côte) ; on étire alors l'îlot jusqu'à ce que
# ses faces (largeur médiane pondérée par l'aire) fassent MIN_ISLAND_TEXELS, d'un facteur plafonné.
# Un grand îlot (sol bordé de chanfreins de 1 cm) n'est jamais étiré.
NARROW_ISLAND_TEXELS = 24.0
MAX_STRETCH = 16.0


def _thicken_islands(objects, res: int, density: float) -> int:
    """Étire le long de leur petit axe (axes principaux des UV) les îlots de lightmap étroits dont
    une face reste sous MIN_ISLAND_TEXELS ; renvoie le nombre d'îlots étirés."""
    import bmesh
    import numpy as np
    from bpy_extras import bmesh_utils

    tmin = MIN_ISLAND_TEXELS / res  # en unités d'UV
    narrow = NARROW_ISLAND_TEXELS / res
    count = 0
    for obj in objects:
        if obj.type != "MESH" or lm.UV_NAME not in obj.data.uv_layers:
            continue
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        uv = bm.loops.layers.uv[lm.UV_NAME]
        changed = False
        for island in bmesh_utils.bmesh_linked_uv_islands(bm, uv):
            loops, spans = [], []
            for f in island:
                spans.append((len(loops), len(loops) + len(f.loops)))
                loops.extend(f.loops)
            pts = np.array([lp[uv].uv[:] for lp in loops], dtype=np.float64)
            if len(pts) < 3:
                continue
            ctr = pts.mean(axis=0)
            d = pts - ctr
            _, v = np.linalg.eigh(d.T @ d)
            stretched = False
            for axis in v.T:
                proj = d @ axis
                ext = proj.max() - proj.min()
                if ext >= narrow:
                    continue
                # Largeur « typique » d'une face le long de l'axe : médiane pondérée par l'aire (les
                # chanfreins de 1 cm, minuscules, ne comptent pas ; les faces d'une barre, oui).
                widths = []
                for a, b in spans:
                    q = pts[a:b]
                    area = 0.5 * abs(float(np.dot(q[:, 0], np.roll(q[:, 1], 1)) - np.dot(q[:, 1], np.roll(q[:, 0], 1))))
                    widths.append((proj[a:b].max() - proj[a:b].min(), area))
                widths.sort()
                total, acc, typical = sum(a for _, a in widths), 0.0, widths[-1][0]
                for wdt, area in widths:
                    acc += area
                    if acc >= 0.5 * total:
                        typical = wdt
                        break
                # Plafond : l'îlot étiré ne dépasse pas NARROW_ISLAND_TEXELS.
                k = min(MAX_STRETCH, tmin / max(typical, 1e-9), narrow / max(ext, 1e-9))
                if k <= 1.0:
                    continue
                d = d + np.outer(proj * (k - 1.0), axis)
                stretched = True
            if stretched:
                for lp, p in zip(loops, ctr + d):
                    lp[uv].uv = (float(p[0]), float(p[1]))
                count += 1
                changed = True
        if changed:
            bm.to_mesh(obj.data)
        bm.free()
    return count


def _repack(objects, res: int) -> None:
    """Réempaquette le 2e jeu d'UV des objets, avec l'écart de `lightmap.unwrap`."""
    meshes = [o for o in objects if o.type == "MESH"]
    for obj in meshes:
        obj.data.uv_layers.active = obj.data.uv_layers[lm.UV_NAME]
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    with contextlib.suppress(RuntimeError):
        bpy.ops.uv.select_all(action="SELECT")
    margin = lm.GAP_TEXELS / res
    bpy.ops.uv.pack_islands(rotate=True, margin_method="FRACTION", margin=margin)
    bpy.ops.object.mode_set(mode="OBJECT")
    for obj in meshes:
        obj.data.uv_layers.active = obj.data.uv_layers[0]


def _prepare_spaceport(objects):
    """`lightmap.prepare`, avec HANGAR_LIGHTMAP_MAX pour les objets de la collection
    `CELL_hangar` (les autres cellules gardent la borne commune), puis les îlots trop fins
    épaissis et l'atlas réempaqueté."""
    hangar = any(coll.name == "CELL_hangar" for o in objects for coll in o.users_collection)
    old = lm.MAX_RES
    if hangar:
        lm.MAX_RES = HANGAR_LIGHTMAP_MAX
    try:
        res, got = _PREPARE(objects)
    finally:
        lm.MAX_RES = old
    n = _thicken_islands(objects, res, got)
    if n:
        _repack(objects, res)
        got = lm.density(objects, res)
    _THICKENED.append(n)
    return res, got


_PREPARE = lm.prepare
_THICKENED: list[int] = []
lm.prepare = _prepare_spaceport


def _snap_up(v: float, g: float = 2.0) -> float:
    return math.ceil(v / g - 1e-6) * g


def _snap_down(v: float, g: float = 2.0) -> float:
    return math.floor(v / g + 1e-6) * g


def _b2bl(v):
    """Repère du jeu (x, y haut, z arrière) → Blender (x, y avant, z haut)."""
    x, y, z = v
    return (x, -z, y)


def _turn(k: int, v):
    """Quarts de tour autour de Z (sens trigonométrique), sur (x, y)."""
    x, y = v[0], v[1]
    for _ in range(k % 4):
        x, y = -y, x
    return (x, y)


def ship_layout() -> dict:
    """Place le vaisseau (sas tourné vers l'ouest, sur HATCH_Y), en déduit la coque, la fosse et
    le hangar. Tout en repère Blender du secteur."""
    lo, hi = _b2bl(SHIP_BBOX[0]), _b2bl(SHIP_BBOX[1])
    bx0, bx1 = sorted((lo[0], hi[0]))
    by0, by1 = sorted((lo[1], hi[1]))
    bz0, bz1 = sorted((lo[2], hi[2]))
    hatch = _b2bl(SHIP_HATCH)
    n = _b2bl(SHIP_HATCH_NORMAL)
    turns = min(range(4), key=lambda k: (_turn(k, n)[0] + 1.0) ** 2 + _turn(k, n)[1] ** 2)
    # Coque dans le repère Blender du vaisseau (+Y = la proue), puis tournée, relative au sas.
    hull_l = (bx0 - HULL_SIDE, bx1 + HULL_SIDE, by0 - HULL_STERN, by1 + HULL_BOW)
    rel = [_turn(turns, (x - hatch[0], y - hatch[1])) for x in hull_l[:2] for y in hull_l[2:]]
    fx0, fx1 = min(p[0] for p in rel), max(p[0] for p in rel)
    fy0, fy1 = min(p[1] for p in rel), max(p[1] for p in rel)
    px0 = HANGAR_X0 + APRON_W
    hx = px0 + PIT_CLEARANCE - fx0
    hy = HATCH_Y
    hull = (hx + fx0, hx + fx1, hy + fy0, hy + fy1)
    pit = (hull[0] - PIT_CLEARANCE, hull[1] + PIT_CLEARANCE, hull[2] - PIT_CLEARANCE, hull[3] + PIT_CLEARANCE)
    x1 = _snap_up(pit[1] + EAST_W)
    y0 = min(_snap_down(pit[2] - SOUTH_W), -12.0)
    y1 = max(_snap_up(pit[3] + NORTH_W), 12.0)
    rh = _turn(turns, (hatch[0], hatch[1]))
    dock_n = _turn(turns, n)
    hull_z = (bz0 - HULL_BOTTOM, bz1 + HULL_TOP)
    return {
        "turns": turns,
        "hatch": (hx, hy),
        "hull": hull,
        "hull_z": hull_z,
        "pit": pit,
        "hangar": (HANGAR_X0, x1, y0, y1, HANGAR_H),
        "origin": (hx - rh[0], hy - rh[1], 0.0),
        # Normale du portail du hangar : vers le vaisseau (contre celle de son sas).
        "dock_normal": (-dock_n[0], -dock_n[1], 0.0),
        # Le vaisseau repose sur les patins des sabots : dessus du patin = dessous de la coque.
        "saddle": max(0.3, PIT_DEPTH + hull_z[0] - SADDLE_PAD),
    }


LAYOUT = ship_layout()

CELLS = {
    "hangar": LAYOUT["hangar"],
    "couloir": (-12.0, -4.4, -1.2, 1.2, 2.6),
    "chantier": (-11.0, -4.4, 1.6, 7.6, 3.0),
    "hall": (-26.0, -12.4, -7.0, 7.0, 5.0),
    "bureau": (-24.0, -16.0, 7.4, 13.4, 3.0),
    "taverne": (-24.0, -16.0, -13.4, -7.4, 3.2),
    "marche": (-40.0, -26.4, -6.0, 6.0, 4.5),
    # M10 : la Banque, au sud du couloir, derrière le mur est du hall (même façade sur le hangar
    # que le chantier), plus haute que les bureaux.
    "banque": (-12.0, -4.4, -11.6, -1.6, 4.0),
}

ROOMS = {
    "hangar": "hangar",
    "couloir": "corridor",
    "chantier": "shipyard_office",
    "hall": "arrivals_hall",
    "bureau": "contracts_office",
    "taverne": "tavern",
    "marche": "market",
    "banque": "bank",
}

# Le portail du vaisseau (point d'attache « @ship ») est au bout de la passerelle, dans le
# hangar : il ne perce aucun mur (exclu de `openings_for`).
DOCK_PORTAL = ("hangar", "@ship", "Door", LAYOUT["hatch"][0], LAYOUT["hatch"][1], "x")
WALL_PORTALS = [
    ("couloir", "hangar", "Door", -4.2, 0.0, "x"),
    ("couloir", "hall", "Opening", -12.2, 0.0, "x"),
    ("hangar", "chantier", "Door", -4.2, 5.6, "x"),
    ("hall", "bureau", "Door", -20.0, 7.2, "y"),
    ("hall", "taverne", "Door", -20.0, -7.2, "y"),
    ("hall", "marche", "Door", -26.2, 0.0, "x"),
    # M10 : la Banque, porte dans le mur est du hall, au sud de l'entrée (entre deux pilastres).
    ("hall", "banque", "Door", -12.2, -4.6, "x"),
]
PORTALS = [DOCK_PORTAL] + WALL_PORTALS

# Fenêtre du chantier sur le hangar (côté, position, largeur, hauteur, allège), des deux côtés.
WINDOW = (3.0, 2.2, 1.2, 0.95)
WINDOWS = {
    "chantier": [("E",) + WINDOW],
    "hangar": [("W",) + WINDOW],
}

SPAWNS = [
    # Au pied de la passerelle, côté hangar, regard vers le vaisseau (cap −90° = +X).
    ("quai", "hangar", LAYOUT["pit"][0] - 2.2, LAYOUT["hatch"][1], 0.0, -90.0),
    ("hall", "hall", -19.0, 0.0, 0.0, -90.0),
    # Entrée du hangar depuis le couloir.
    ("hangar", "hangar", HANGAR_X0 + 1.6, 0.0, 0.0, -90.0),
    # M10 : juste passé la porte de la Banque, regard vers le guichet (cap −90° = +X).
    ("banque", "banque", -11.2, -4.6, 0.0, -90.0),
]

COOL = [0.78, 0.88, 1.0]
COOL_W = [0.84, 0.92, 1.0]
WARM = [1.0, 0.8, 0.58]
SODIUM = [1.0, 0.7, 0.42]
AMBER = [1.0, 0.56, 0.24]
NEUTRAL = [1.0, 0.94, 0.86]

# Orientation d'un prop adossé à un mur (sa face +Y vers la pièce).
FACE_N, FACE_S, FACE_E, FACE_W = math.pi, 0.0, math.pi / 2, -math.pi / 2
FRONT = ((-1, 0, 0), (0, 0, 1))  # (right, up) d'une face tournée vers +Y
BACK = ((1, 0, 0), (0, 0, 1))  # (right, up) d'une face tournée vers −Y


def palette() -> dict:
    """Palette du kit, plus le verre teinté des bouteilles de la taverne (sans texture) ; M10 : sol
    et murs de la Banque (dalles et tôle de la bibliothèque, teintées), laiton (métal brut teinté),
    feuillages (sans texture)."""
    m = kit.palette()
    m["bottle"] = c.material("place_bottle_glass", (0.3, 0.13, 0.04), roughness=0.15)
    m["bottle_green"] = c.material("place_bottle_green", (0.09, 0.2, 0.1), roughness=0.15)
    m["bank_floor"] = c.textured_material("place_bank_floor", "floor_tiles", tint=(0.34, 0.32, 0.30))
    m["bank_wall"] = c.textured_material("place_bank_wall", "panel_painted", tint=(0.30, 0.42, 0.41))
    m["brass"] = c.textured_material("place_brass", "metal_bare", tint=(1.0, 0.74, 0.40))
    m["plant"] = c.material("place_plant", (0.07, 0.17, 0.05), roughness=0.7)
    m["plant_light"] = c.material("place_plant_light", (0.20, 0.30, 0.07), roughness=0.7)
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
    """Ce qu'une cellule exporte : visibles, structure (version simplifiée), colliders, repères.

    `with k.detail(n):` (M9) : tout ce qui est créé dedans est du décor de niveau n (propriété
    `detail`), sans collider (ceux des pièces du kit et de `col` sont supprimés), hors de la
    version simplifiée ; ses `LIGHT_` ne sont pas cuits."""

    def __init__(self, cid: str, m: dict):
        self.cid = cid
        self.m = m
        self.coll = c.collection(f"CELL_{cid}")
        self.visual, self.structure, self.colliders, self.marks, self.pickups = [], [], [], [], []
        self.level = 0
        self._group = None
        self._n = 0

    def nm(self, tag: str) -> str:
        self._n += 1
        return f"{self.cid}_{tag}{self._n}"

    @contextlib.contextmanager
    def detail(self, level: int):
        old, self.level = self.level, level
        try:
            yield
        finally:
            self.level = old

    # Objets construits en repère local, posés ensemble par `end(center, facing)`.
    def begin(self):
        self._group = []

    def end(self, center, facing=0.0):
        place(self._group, center, facing)
        self._group = None

    def _keep(self, obj, structure=False, collider=False):
        if collider and self.level > 0:
            bpy.data.objects.remove(obj, do_unlink=True)
            return None
        if self._group is not None:
            self._group.append(obj)
        if collider:
            self.colliders.append(obj)
        else:
            if self.level > 0:
                c.set_detail([obj], self.level)
            self.visual.append(obj)
            if structure and self.level == 0:
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

    def strut(self, tag, p0, p1, w, h, mat, uv="box", bevel=0.0):
        """Barre de section w × h de p0 à p1 (`ship_interior._strut`)."""
        obj = kit._strut(self.nm(tag), p0, p1, w, h, self.m[mat] if isinstance(mat, str) else mat, self.coll, uv=uv, bevel=bevel)
        return self._keep(obj)

    def plate(self, tag, size, center, mat, uv="box", tile=None, th=0.01):
        """Plaque tournée vers +Y (avant d'un prop construit en local)."""
        return self.ori(tag, (size[0], size[1], th), center, *FRONT, mat, uv, tile=tile)

    def col(self, tag, size, center):
        return self._keep(c.collider_box(self.nm(tag), size, center, self.coll), collider=True)

    def add(self, result, structure=False):
        """Résultat (visibles, colliders, repères) d'une pièce du kit."""
        vis, col, marks = result
        if self.level > 0:
            bad = [mk.name for mk in marks if not mk.name.startswith("LIGHT_")]
            if bad:
                raise ValueError(f"repères de gameplay dans du décor optionnel : {bad}")
            c.set_detail(list(vis) + list(marks), self.level)
            for o in col:
                bpy.data.objects.remove(o, do_unlink=True)
            col = []
            structure = False
        self.visual.extend(vis)
        if structure:
            self.structure.extend(vis)
        self.colliders.extend(col)
        self.marks.extend(marks)
        return vis, col, marks

    def mark(self, obj):
        """Repère fait à la main (siège d'une banquette) : posé avec le groupe en cours."""
        if self._group is not None:
            self._group.append(obj)
        self.marks.append(obj)
        return obj

    def screen(self, tag, size, center, right, up, tile, th=0.01):
        """Écran plat (matériau `screen`) qui montre une seule case de la texture d'écrans
        (`SCREEN_TILES`) : lignes de glyphes, barres, courbes ; jamais de texte lisible."""
        obj = self.ori(tag, (size[0], size[1], th), center, right, up, "screen", uv=None)
        if obj is not None:
            _uv_tile(obj, tile)
        return obj

    def shell(self, shell):
        self.visual.extend(shell["visual"])
        self.structure.extend(shell["structure"])
        self.colliders.extend(shell["colliders"])
        self.marks.extend(shell.get("marks", []))

    def light(self, tag, x, y, z, lumens, rng, color, lens="light", drop=0.0, facing=0.0):
        props = {"lumens": lumens, "range": rng, "color": color, "shadow_candidate": 0}
        return self.add(kit.ceiling_light(f"{self.cid}_{tag}", (x, y, z), facing, self.m, self.coll, props, lens=lens, drop=drop))

    def npc(self, name, socket, facing, role, variant, size, center_dy):
        """PNJ de comptoir fait à la main (bar) : `SOCKET_<name>_npc` au pied du PNJ (regard +Y
        local tourné de `facing`), `INTERACT_<name>` (`interact: "npc"`) dont le pavé (`size`,
        repère local) est centré à `center_dy` m devant le socket, du sol à 1,95 m."""
        props = {"role": role, "variant": variant}
        sx, sy = socket
        cs, sn = math.cos(facing), math.sin(facing)
        cx, cy = sx - center_dy * sn, sy + center_dy * cs
        self.marks.append(c.empty(f"SOCKET_{name}_npc", (sx, sy, 0.0), self.coll, {"usage": "npc", **props}, rotation_z=facing))
        mark = c.empty(f"INTERACT_{name}", (cx, cy, 0.975), self.coll, {"interact": "npc", **props}, rotation_z=facing)
        mark["half_extents"] = c.half_extents((size[0], size[1], 1.95), 0.03)
        self.marks.append(mark)

    def parts(self) -> dict:
        return {"visual": self.visual, "structure": self.structure, "colliders": self.colliders, "marks": self.marks, "pickups": self.pickups}


# --- Pièces propres au spatioport (de M8) ---------------------------------------------------------


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


def barrel(k: Cell, tag, x, y, mat="crate_slate", z=0.0):
    """Fût de 200 l : virole peinte, deux cerclages sombres, bonde ; collider."""
    k.cyl(f"{tag}", 0.29, 0.88, (x, y, z + 0.44), mat, segments=8)
    for zz in (0.26, 0.62):
        k.cyl(f"{tag}_hoop", 0.3, 0.04, (x, y, z + zz), "dark", segments=8)
    k.cyl(f"{tag}_lid", 0.27, 0.02, (x, y, z + 0.89), "dark", segments=8)
    k.col(f"{tag}", (0.6, 0.6, 0.9), (x, y, z + 0.45))


def stool(k: Cell, tag, x, y):
    """Tabouret de bar : embase, fût, assise matelassée, repose-pieds ; collider."""
    k.cyl(f"{tag}_base", 0.21, 0.03, (x, y, 0.015), "dark", segments=12)
    k.cyl(f"{tag}_post", 0.035, 0.7, (x, y, 0.38), "bare", segments=8)
    k.cyl(f"{tag}_ring", 0.17, 0.025, (x, y, 0.3), "bare", segments=12)
    k.cyl(f"{tag}_seat", 0.2, 0.08, (x, y, 0.77), "fabric", segments=12)
    k.cyl(f"{tag}_pan", 0.17, 0.03, (x, y, 0.715), "dark", segments=12)
    k.col(f"{tag}", (0.42, 0.42, 0.82), (x, y, 0.41))


def handrail(k: Cell, tag, x0, x1, y, side, half_w=1.2):
    """Rambarde le long d'un mur de coursive : poteaux tous les 2 m, main courante jaune, lisse
    basse ; `side` = signe du mur (±1)."""
    for x in [x0 + 0.2 + 2.0 * j for j in range(int((x1 - x0 - 0.3) // 2.0) + 1)]:
        k.box(f"{tag}_post", (0.05, 0.05, 0.95), (x, y, 0.55), "dark")
        k.box(f"{tag}_bracket", (0.05, 0.2, 0.04), (x, y + side * 0.1, 0.92), "dark")
    length = x1 - x0
    k.cyl(f"{tag}_top", 0.028, length, ((x0 + x1) / 2, y, 1.0), "paint_yellow", axis="X", segments=10)
    k.cyl(f"{tag}_mid", 0.02, length, ((x0 + x1) / 2, y, 0.58), "bare", axis="X", segments=8)
    k.col(f"{tag}", (length, 0.3, 1.05), ((x0 + x1) / 2, side * (half_w - 0.15), 0.525))


def collar(k: Cell, tag, x, half_w, h, tc, lc, depth=0.3, t=0.16):
    """Cadre qui épouse la section à pans coupés d'une coursive (jaune, chant sombre, bande de
    danger), du pied d'un mur au pied de l'autre."""
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
        v = Vector((1.0, 0.0, 0.0))
        if e.cross(v).dot(n) < 0:
            v = -v
        k.ori(f"{tag}_seg", (length, depth, t), tuple(mid + n * (t / 2)), tuple(e), tuple(v), "paint_yellow", bevel=0.012)
        k.ori(f"{tag}_lip", (length, 0.12, 0.012), tuple(mid + n * (t + 0.006)), tuple(e), tuple(v), "trim_sheet", "hazard")
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
    for x in (-0.52, 0.52):
        k.box("kiosk_bezel", (0.92, 0.04, 0.62), (x, -0.32, 1.72), "dark", bevel=0.01)
        k.plate("kiosk_screen", (0.84, 0.54), (x, -0.295, 1.72), "screen", tile=0.9)
    k.plate("kiosk_back_band", (2.0, 0.14), (0, -0.335, 1.2), "trim_sheet", "hazard", th=0.006)
    for s in (-1, 1):
        k.box("kiosk_side", (0.06, 0.9, 2.5), (s * 1.07, -0.05, 1.25), "dark")
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


def bar_counter(k: Cell, tag, xa, xb, y_front, depth, facing):
    """Bar de la taverne (façade vers +Y local) : caisson sombre, façade matelassée, repose-pieds
    de métal brut, plateau à chant brillant, bandeau ambré sous la lèvre (cuit). M9 : le dessus
    du plateau est à 1,05 m (mains du PNJ, lot E) et son bord côté employé à `depth` + 0,02 m
    derrière la façade. Renvoie y de ce bord (repère local, avant pose)."""
    L = xb - xa
    k.begin()
    k.box(f"{tag}_body", (L, depth, 1.0), (0, -depth / 2, 0.5), "dark", bevel=0.012, structure=True)
    k.ori(f"{tag}_front", (L - 0.1, 0.72, 0.05), (0, 0.025, 0.5), *FRONT, "fabric", bevel=0.02)
    k.ori(f"{tag}_kick", (L - 0.04, 0.1, 0.03), (0, 0.015, 0.06), *FRONT, "rubber")
    t0, t1 = -depth - 0.02, 0.17
    k.box(f"{tag}_top", (L + 0.08, t1 - t0, 0.05), (0, (t0 + t1) / 2, 1.025), "dark", bevel=0.012, structure=True)
    k.box(f"{tag}_top_edge", (L + 0.1, 0.04, 0.04), (0, t1, 1.03), "bare")
    k.ori(f"{tag}_glow", (L - 0.2, 0.06, 0.02), (0, 0.1, 0.99), (1, 0, 0), (0, 1, 0), "band_amber", ("strip", 1.0))
    k.cyl(f"{tag}_footrail", 0.025, L - 0.2, (0, 0.2, 0.2), "bare", axis="X", segments=8)
    for x in (-L / 2 + 0.3, 0.0, L / 2 - 0.3):
        k.box(f"{tag}_footrail_post", (0.04, 0.2, 0.04), (x, 0.1, 0.2), "bare")
    k.col(f"{tag}", (L, depth + 0.22, 1.1), (0, (0.22 - depth) / 2, 0.55))
    k.end(((xa + xb) / 2, y_front, 0.0), facing)
    return t0


def keg(k: Cell, tag, x, y):
    k.cyl(f"{tag}", 0.28, 0.82, (x, y, 0.41), "bare", segments=10)
    for z in (0.12, 0.41, 0.7):
        k.cyl(f"{tag}_hoop", 0.29, 0.05, (x, y, z), "dark", segments=10)
    k.cyl(f"{tag}_valve", 0.06, 0.06, (x, y, 0.85), "paint_yellow", segments=8)
    k.col(f"{tag}", (0.6, 0.6, 0.9), (x, y, 0.45))


def back_bar(k: Cell, tag, xa, xb, y_wall):
    """Étagères du bar contre le mur sud : meuble bas, trois planches, bouteilles, bandeau chaud
    sous chaque planche (les bouteilles de la planche du haut : décor de niveau 1)."""
    L = xb - xa
    xc = (xa + xb) / 2
    k.box(f"{tag}_cab", (L, 0.42, 0.9), (xc, y_wall + 0.21, 0.45), "dark", bevel=0.01, structure=True)
    k.ori(f"{tag}_cab_doors", (L - 0.08, 0.7, 0.012), (xc, y_wall + 0.426, 0.47), (-1, 0, 0), (0, 0, 1), "fabric")
    k.box(f"{tag}_cab_top", (L + 0.04, 0.46, 0.04), (xc, y_wall + 0.23, 0.92), "bare")
    k.box(f"{tag}_back", (L, 0.03, 1.3), (xc, y_wall + 0.015, 1.65), "dark")
    for j, z in enumerate((1.25, 1.65, 2.05)):
        k.box(f"{tag}_shelf", (L, 0.26, 0.03), (xc, y_wall + 0.13, z), "dark")
        k.ori(f"{tag}_shelf_glow", (L - 0.1, 0.03, 0.01), (xc, y_wall + 0.24, z - 0.02), (1, 0, 0), (0, 1, 0), "band_warm", ("strip", 1.0))
        count = int(L / 0.12)
        with k.detail(1 if j == 2 else 0):
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


def stall(k: Cell, tag, center, facing, body: str, roof: str, wares: list, counter=None):
    """Étal du marché (face vers +Y local) : comptoir de tôle nervurée, plateau sombre, marchandises,
    poteaux, auvent de tôle incliné, bandeau chaud sous l'auvent, enseigne à pochoirs. Le stock
    (caisses) est posé derrière par l'appelant. M9 : `counter` = (role, variant) : comptoir de
    PNJ du kit (plateau à 1,05 m, PNJ derrière) au lieu de l'étal, joues basses sur les côtés."""
    if counter is not None:
        role, variant = counter
        k.add(kit.counter(f"{k.cid}_{tag}", center, facing, k.m, k.coll, length=2.4, role=role, variant=variant, style="ship"))
    k.begin()
    if counter is None:
        k.box(f"{tag}_body", (2.4, 0.8, 0.92), (0, 0, 0.46), body, bevel=0.01, structure=True)
        k.box(f"{tag}_kick", (2.3, 0.05, 0.1), (0, 0.4, 0.05), "rubber")
        k.box(f"{tag}_top", (2.5, 0.9, 0.05), (0, 0.02, 0.945), "dark", bevel=0.01)
        k.box(f"{tag}_edge", (2.52, 0.03, 0.03), (0, 0.47, 0.945), "bare")
        k.plate(f"{tag}_label", (1.0, 0.11), (0.5, 0.405, 0.72), "trim_sheet", "stencils", th=0.005)
        for i, (dx, size, mat) in enumerate(wares):
            k.box(f"{tag}_ware", size, (dx, 0.05 + 0.04 * (i % 2), 0.97 + size[2] / 2), mat, bevel=0.006)
        k.col(f"{tag}", (2.5, 0.9, 1.0), (0, 0.02, 0.5))
    else:
        # Joues : tôle peinte du comptoir aux poteaux (on ne passe pas derrière le marchand).
        for s in (-1, 1):
            k.box(f"{tag}_cheek", (0.06, 0.9, 1.0), (s * 1.24, -0.75, 0.5), body, bevel=0.008)
            k.box(f"{tag}_cheek_cap", (0.08, 0.92, 0.04), (s * 1.24, -0.75, 1.02), "dark")
            k.col(f"{tag}_cheek", (0.1, 0.9, 1.1), (s * 1.24, -0.75, 0.55))
        # Marchandises en bout de plateau, hors de la place des mains du PNJ (niveau 1).
        with k.detail(1):
            for i, (dx, size, mat) in enumerate(wares[:2]):
                x = -0.95 if i == 0 else 0.95
                k.box(f"{tag}_ware", (min(size[0], 0.38), min(size[1], 0.3), size[2]), (x, 0.12, 1.05 + size[2] / 2), mat, bevel=0.006)
    for s in (-1, 1):
        k.box(f"{tag}_post", (0.08, 0.08, 2.55), (s * 1.16, -0.5, 1.275), "dark")
        k.box(f"{tag}_strut", (0.05, 1.1, 0.05), (s * 1.16, 0.02, 2.42), "dark")
    k.ori(f"{tag}_roof", (2.6, 1.5, 0.04), (0, 0.05, 2.47), (1, 0, 0), (0, math.cos(0.2), -math.sin(0.2)), roof)
    k.ori(f"{tag}_roof_edge", (2.62, 0.1, 0.04), (0, 0.8, 2.32), (-1, 0, 0), (0, 0, 1), "paint_black")
    k.ori(f"{tag}_sign", (1.4, 0.1, 0.006), (0, 0.825, 2.32), (-1, 0, 0), (0, 0, 1), "trim_sheet", "stencils")
    k.ori(f"{tag}_glow", (2.2, 0.07, 0.02), (0, 0.6, 2.27), (1, 0, 0), (0, 1, 0), "band_warm", ("strip", 1.0))
    for s in (-1, 1):
        k.col(f"{tag}_post", (0.12, 0.12, 2.55), (s * 1.16, -0.5, 1.275))
    k.end(center, facing)


# --- Pièces nouvelles (M9) ---------------------------------------------------------------------


def partition(k: Cell, tag, p0, p1, height=1.05, glass_top=2.1):
    """Cloison de guichet le long de X ou de Y, de p0 à p1 (x, y) : caisson noir à lisse jaune,
    plinthe, vitrage jusqu'à `glass_top` (0 : sans) ; collider plein (le joueur ne passe pas
    derrière les comptoirs)."""
    (ax, ay), (bx, by) = p0, p1
    along_x = abs(bx - ax) >= abs(by - ay)
    L = abs(bx - ax) if along_x else abs(by - ay)
    cx, cy = (ax + bx) / 2, (ay + by) / 2

    def dims(along, across, hh):
        return (along, across, hh) if along_x else (across, along, hh)

    def at(s, z):
        return (cx + s, cy, z) if along_x else (cx, cy + s, z)

    k.box(f"{tag}", dims(L, 0.1, height), (cx, cy, height / 2), "paint_black", bevel=0.008, structure=True)
    k.box(f"{tag}_cap", dims(L + 0.02, 0.14, 0.04), (cx, cy, height + 0.02), "paint_yellow", bevel=0.005)
    k.box(f"{tag}_kick", dims(L - 0.02, 0.12, 0.1), (cx, cy, 0.05), "rubber")
    top = height + 0.1
    if glass_top:
        gh = glass_top - height - 0.04
        k.box(f"{tag}_glass", dims(L - 0.1, 0.02, gh), (cx, cy, height + 0.04 + gh / 2), "glass")
        k.box(f"{tag}_rail", dims(L, 0.06, 0.05), (cx, cy, glass_top), "paint_black")
        for s in (-L / 2 + 0.03, L / 2 - 0.03):
            k.box(f"{tag}_mullion", dims(0.05, 0.06, gh), at(s, height + 0.04 + gh / 2), "paint_black")
        top = glass_top + 0.03
    k.col(f"{tag}", dims(L, 0.14, top), (cx, cy, top / 2))


def tow_tractor(k: Cell, tag, center, facing):
    """Tracteur de piste (pousse les vaisseaux légers) : châssis sombre, capot et caisse jaunes,
    siège, arceau, roues, pare-chocs à bandes de danger, gyrophare (voyant) ; collider."""
    k.begin()
    k.box(f"{tag}_chassis", (2.5, 1.3, 0.3), (0, 0, 0.42), "dark", bevel=0.03)
    k.box(f"{tag}_body", (1.2, 1.26, 0.55), (-0.5, 0, 0.84), "paint_yellow", bevel=0.05)
    k.box(f"{tag}_hood", (0.9, 1.22, 0.38), (0.72, 0, 0.76), "paint_yellow", bevel=0.05)
    k.box(f"{tag}_grille", (0.02, 0.9, 0.24), (1.18, 0, 0.74), "dark")
    k.ori(f"{tag}_grille_vent", (0.86, 0.2, 0.006), (1.195, 0, 0.74), (0, 1, 0), (0, 0, 1), "trim_sheet", "vent")
    k.box(f"{tag}_seat", (0.45, 0.5, 0.12), (-0.55, 0, 1.18), "fabric", bevel=0.03)
    k.box(f"{tag}_seat_back", (0.1, 0.5, 0.5), (-0.8, 0, 1.45), "fabric", bevel=0.03)
    k.box(f"{tag}_dash", (0.3, 0.9, 0.28), (0.15, 0, 1.15), "dark", bevel=0.02)
    k.ori(f"{tag}_dash_buttons", (0.5, 0.18, 0.006), (0.0, 0, 1.3), (0, -1, 0), (0.8, 0, 0.6), "buttons", tile=0.3)
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.box(f"{tag}_cage", (0.06, 0.06, 1.15), (-0.2 + sx * 0.75 - 0.15, sy * 0.58, 1.55), "dark")
            k.cyl(f"{tag}_wheel", 0.32, 0.26, (sx * 0.8, sy * 0.6, 0.32), "rubber", axis="Y", segments=14)
            k.cyl(f"{tag}_hub", 0.16, 0.28, (sx * 0.8, sy * 0.6, 0.32), "bare", axis="Y", segments=10)
    k.box(f"{tag}_roof", (1.7, 1.28, 0.06), (-0.35, 0, 2.14), "paint_black", bevel=0.02)
    k.cyl(f"{tag}_beacon", 0.07, 0.14, (-0.35, 0, 2.24), "beacon", segments=12)
    for sx in (-1, 1):
        k.box(f"{tag}_bumper", (0.14, 1.36, 0.22), (sx * 1.3, 0, 0.38), "dark", bevel=0.02)
        k.ori(f"{tag}_bumper_band", (1.3, 0.16, 0.006), (sx * 1.373, 0, 0.38), (0, sx, 0), (0, 0, 1), "trim_sheet", "hazard")
    k.box(f"{tag}_hitch", (0.3, 0.12, 0.1), (-1.5, 0, 0.36), "bare")
    for sy in (-1, 1):
        k.ori(f"{tag}_label", (0.5, 0.1, 0.004), (-0.45, sy * 0.632, 0.9), (sy * -1, 0, 0) if sy < 0 else (-1, 0, 0), (0, 0, 1), "trim_sheet", "stencils")
    k.col(f"{tag}", (2.8, 1.4, 2.2), (0, 0, 1.1))
    k.end(center, facing)


def tool_chest(k: Cell, tag, center, facing):
    """Servante d'atelier rouge rouille sur roulettes : tiroirs, poignées, outils sur le dessus ;
    collider (sans effet en décor optionnel)."""
    k.begin()
    k.box(f"{tag}", (0.8, 0.48, 0.92), (0, 0, 0.56), "crate_rust", bevel=0.015)
    for i in range(5):
        z = 0.22 + i * 0.17
        k.plate(f"{tag}_drawer", (0.72, 0.13), (0, 0.245, z), "dark", th=0.006)
        k.box(f"{tag}_pull", (0.4, 0.02, 0.02), (0, 0.26, z + 0.03), "bare")
    k.box(f"{tag}_top", (0.84, 0.5, 0.03), (0, 0, 1.035), "rubber")
    for dx in (-0.32, 0.32):
        for dy in (-0.18, 0.18):
            k.cyl(f"{tag}_caster", 0.045, 0.04, (dx, dy, 0.05), "rubber", axis="Y", segments=8)
    k.box(f"{tag}_tool", (0.3, 0.05, 0.03), (-0.15, 0.05, 1.065), "bare")
    k.box(f"{tag}_tool", (0.22, 0.07, 0.05), (0.2, -0.06, 1.075), "paint_yellow", bevel=0.01)
    k.col(f"{tag}", (0.84, 0.5, 1.08), (0, 0, 0.54))
    k.end(center, facing)


def chain(k: Cell, tag, x, y, z_top, z_bot):
    """Chaîne de levage pendue à une ferme : deux brins, moufle jaune, crochet (décor haut)."""
    for dx in (-0.04, 0.04):
        k.box(f"{tag}_link", (0.025, 0.025, z_top - z_bot), (x + dx, y, (z_top + z_bot) / 2), "dark")
    k.box(f"{tag}_block", (0.22, 0.14, 0.3), (x, y, z_bot - 0.15), "paint_yellow", bevel=0.02)
    k.box(f"{tag}_hook", (0.05, 0.05, 0.2), (x, y, z_bot - 0.4), "bare")
    k.box(f"{tag}_hook_tip", (0.14, 0.05, 0.05), (x + 0.05, y, z_bot - 0.5), "bare")


def catwalk(k: Cell, tag, x_face, y_a, y_b, z, width=1.1, posts=(), gaps=()):
    """Coursive haute le long d'un mur ouest (x_face = devant des colonnes) : caillebotis, longeron,
    consoles en équerre (aux positions `posts`, colonnes du mur, et tous les ≈ 4 m), garde-corps
    jaune à lisse, sous-lisse et plinthe. Décor : à poser dans un `detail` (pas de collider)."""
    L = y_b - y_a
    ym = (y_a + y_b) / 2
    xo = x_face + width
    k.box(f"{tag}_deck", (width, L, 0.05), (x_face + width / 2, ym, z - 0.025), "grating_plain")
    k.box(f"{tag}_stringer", (0.08, L, 0.22), (xo - 0.04, ym, z - 0.13), "dark")
    k.box(f"{tag}_stringer_in", (0.06, L, 0.16), (x_face + 0.03, ym, z - 0.1), "dark")
    supports = sorted(set([round(p, 2) for p in posts if y_a + 0.2 < p < y_b - 0.2] +
                          [round(y_a + 0.3 + j * 4.0, 2) for j in range(int((L - 0.6) / 4.0) + 1)]))
    for y in supports:
        k.box(f"{tag}_bracket", (width, 0.1, 0.12), (x_face + width / 2, y, z - 0.16), "dark")
        k.strut(f"{tag}_brace", (x_face - 0.3, y, z - 1.1), (xo - 0.12, y, z - 0.22), 0.08, 0.08, "dark")
    holes = [(a, b) for a, b in gaps]
    for sa, sb in kit._cuts(y_a, y_b, holes):
        n = max(1, math.ceil((sb - sa) / 1.6))
        for i in range(n + 1):
            y = sa + (sb - sa) * i / n
            k.box(f"{tag}_post", (0.05, 0.05, 1.05), (xo - 0.04, y, z + 0.525), "paint_yellow")
        for t_, hh, th in (("rail", 1.03, 0.05), ("mid", 0.55, 0.035)):
            k.box(f"{tag}_{t_}", (th, sb - sa, th), (xo - 0.04, (sa + sb) / 2, z + hh), "paint_yellow")
        k.ori(f"{tag}_toe", (sb - sa, 0.12, 0.012), (xo - 0.02, (sa + sb) / 2, z + 0.07), (0, 1, 0), (0, 0, 1), "trim_sheet", "hazard")


def cut_wainscot(shell: dict, cid: str, x_face: float, a: float, b: float, m: dict, coll) -> None:
    """Le soubassement de 2,4 m du hangar (`_wainscot`) passerait devant une fenêtre du mur ouest :
    il est découpé autour d'elle (de a à b le long de Y) et ne reste que sous l'allège."""
    for obj in list(shell["visual"]):
        if not obj.name.startswith(f"{cid}_wainscot") or obj.type != "MESH":
            continue
        loc, dim = obj.location.copy(), obj.dimensions.copy()
        if abs(loc.x - (x_face + 0.02)) > 0.05:
            continue
        lo, hi = loc.y - dim.y / 2, loc.y + dim.y / 2
        if hi <= a or lo >= b:
            continue
        shell["visual"].remove(obj)
        if obj in shell["structure"]:
            shell["structure"].remove(obj)
        bpy.data.objects.remove(obj, do_unlink=True)
        pieces = [((lo, a - 0.26), dim.z, loc.z), ((b + 0.26, hi), dim.z, loc.z), ((a - 0.26, b + 0.26), 0.8, 0.4)]
        for i, ((sa, sb), hz, zc) in enumerate(pieces):
            if sb - sa < 0.05:
                continue
            piece = kit._box(f"{cid}_wainscot_cut{i}", (dim.x, sb - sa, hz), (loc.x, (sa + sb) / 2, zc), m["dark"], coll)
            shell["visual"].append(piece)
            shell["structure"].append(piece)


# --- Pièces nouvelles (M10) : hall, banque ------------------------------------------------------

# Cases de la texture d'écrans de la bibliothèque (`materials/devices.screen`, grille 4 × 4,
# ligne 0 en haut) : lignes de glyphes, barres, courbes, voyants. Aucune ne contient de texte
# lisible ; les cases à chiffres (« digits_* ») ne servent pas aux panneaux.
SCREEN_TILES = {
    "bars_green": (0, 0), "digits_red": (0, 1), "text_green": (0, 2), "wave_orange": (0, 3),
    "text_orange": (1, 0), "wave_cyan": (1, 1), "status_orange": (1, 2), "radar_green": (1, 3),
    "radar_cyan": (2, 0), "status_green": (2, 1), "digits_cyan": (2, 2), "bars_orange": (2, 3),
    "digits_orange": (3, 0), "bars_cyan": (3, 1), "wave_green": (3, 2), "text_cyan": (3, 3),
}


def _uv_tile(obj, tile: str, inset: float = 0.06) -> None:
    """1er jeu d'UV d'une plaque (`Cell.ori`, X = largeur, Y = hauteur) : toute la plaque montre
    la case `tile` de la texture d'écrans, rognée de `inset` (sa marge sombre), sans miroir."""
    row, col = SCREEN_TILES[tile]
    u0, u1 = (col + inset) / 4.0, (col + 1 - inset) / 4.0
    v0, v1 = 1.0 - (row + 1 - inset) / 4.0, 1.0 - (row + inset) / 4.0
    mesh = obj.data
    xs = [v.co.x for v in mesh.vertices]
    ys = [v.co.y for v in mesh.vertices]
    ax0, ax1, ay0, ay1 = min(xs), max(xs), min(ys), max(ys)
    layer = c._uv_layer(mesh)
    for loop in mesh.loops:
        co = mesh.vertices[loop.vertex_index].co
        fu = (co.x - ax0) / max(ax1 - ax0, 1e-6)
        fv = (co.y - ay0) / max(ay1 - ay0, 1e-6)
        layer.data[loop.index].uv = (u0 + fu * (u1 - u0), v0 + fv * (v1 - v0))


def info_board(k: Cell, tag, center, facing, tiles, rows=3, cell=(1.0, 0.42), gap=0.05, glow="band_cool",
               standoff=0.0, hang_to=None):
    """Grand panneau d'information, face vers +Y local (`center` : centre du panneau, `tiles` :
    une case de `SCREEN_TILES` par colonne, de gauche à droite vu de face) : caisson noir à
    chants de métal brut, bandeau de tête jaune à pochoirs, grille d'écrans à lignes de glyphes
    (aucun texte lisible), bandeau lumineux dessous (cuit : il éclaire le sol au pied du
    panneau). `standoff` : consoles murales jusqu'au mur (derrière le caisson) ; `hang_to` :
    tiges jusqu'au plafond (z absolu). Pas de collider (au-dessus des têtes)."""
    cols = len(tiles)
    cw, ch = cell
    head = 0.2
    W = cols * cw + (cols + 1) * gap
    H = rows * ch + (rows + 1) * gap + head
    k.begin()
    k.box(f"{tag}_case", (W + 0.1, 0.16, H + 0.1), (0, -0.08, 0), "paint_black", bevel=0.015, structure=True)
    for s in (-1, 1):
        k.box(f"{tag}_rim", (W + 0.14, 0.05, 0.04), (0, -0.005, s * (H / 2 + 0.07)), "bare")
        k.box(f"{tag}_jamb", (0.04, 0.05, H + 0.1), (s * (W / 2 + 0.07), -0.005, 0), "bare")
    zh = H / 2 - head / 2
    k.plate(f"{tag}_head", (W - 0.04, head - 0.05), (0, 0.006, zh), "paint_yellow", th=0.008)
    for x in (-W * 0.3, W * 0.3):
        k.plate(f"{tag}_head_label", (min(1.1, W * 0.3), 0.11), (x, 0.012, zh), "trim_sheet", "stencils", th=0.004)
    z_top = H / 2 - head - gap
    for j in range(rows):
        z = z_top - ch / 2 - j * (ch + gap)
        for i, tile in enumerate(tiles):
            # Vu de face (depuis +Y), +X local est à gauche : colonne 0 à droite de X.
            x = W / 2 - gap - cw / 2 - i * (cw + gap)
            k.screen(f"{tag}_scr", (cw, ch), (x, 0.006, z), *FRONT, tile)
    k.ori(f"{tag}_glow", (W - 0.2, 0.06, 0.02), (0, -0.07, -H / 2 - 0.065), (1, 0, 0), (0, 1, 0), glow, ("strip", 1.0))
    if standoff > 0.0:
        for x in (-W * 0.36, W * 0.36):
            for z in (-H * 0.3, H * 0.3):
                k.box(f"{tag}_bracket", (0.08, standoff, 0.12), (x, -0.16 - standoff / 2, z), "dark")
    if hang_to is not None:
        zt = hang_to - center[2]
        z0 = H / 2 + 0.05
        for x in (-W * 0.36, W * 0.36):
            k.box(f"{tag}_rod", (0.03, 0.03, zt - z0), (x, -0.08, (zt + z0) / 2), "dark")
            k.box(f"{tag}_rod_plate", (0.16, 0.16, 0.02), (x, -0.08, zt - 0.01), "dark")
    k.end(center, facing)
    return W, H


def bench_block(k: Cell, tag, center, facing, seats=4, rows=2, pitch=0.66, fabric="fabric_dark"):
    """Banquette de salle d'attente : sièges en rangée sur une poutre à pieds en T, la rangée le
    long de X local. `rows` = 2 : deux rangées dos à dos de `seats` sièges, tournées vers +Y et
    −Y local, séparées par un dossier central noir à lisse jaune ; 1 : une rangée tournée vers
    +Y (dos à −Y, contre un mur). Assises et dossiers matelassés (`fabric`), coques sombres,
    accoudoirs partagés. Chaque siège est interactif comme `kit.seat` (`INTERACT_<nom>` :
    `seat`, `passenger`, volume ; œil `SOCKET_<nom>_eye`, regard vers l'avant du siège) ; un seul
    collider pour la banquette. `center` : au sol, au milieu de la banquette."""
    L = seats * pitch + 0.08
    sides = (1, -1) if rows == 2 else (1,)
    o = 0.0 if rows == 2 else -0.26  # décalage d'une rangée seule (dos vers −Y)
    beam_y = 0.0 if rows == 2 else o + 0.26
    a = math.radians(10)
    k.begin()
    for si, s in enumerate(sides):
        up = (0.0, -s * math.sin(a), math.cos(a))
        for i in range(seats):
            x = (i - (seats - 1) / 2) * pitch
            k.box(f"{tag}_pan", (0.56, 0.5, 0.03), (x, o + s * 0.37, 0.415), "dark")
            k.box(f"{tag}_cushion", (0.54, 0.48, 0.08), (x, o + s * 0.38, 0.47), fabric, bevel=0.025)
            k.ori(f"{tag}_back", (0.54, 0.5, 0.07), (x, o + s * 0.15, 0.83), (1, 0, 0), up, fabric, bevel=0.025)
            k.ori(f"{tag}_back_shell", (0.56, 0.54, 0.025), (x, o + s * 0.1, 0.825), (1, 0, 0), up, "dark")
            name = f"{k.cid}_{tag}_{'ab'[si]}{i}"
            rz = 0.0 if s > 0 else math.pi
            k.mark(c.empty(f"INTERACT_{name}", (x, o + s * 0.33, 0.62), k.coll,
                           {"interact": "seat", "role": "passenger", "half_extents": c.half_extents((0.62, 0.62, 1.24), 0.03)},
                           rotation_z=rz))
            k.mark(c.empty(f"SOCKET_{name}_eye", (x, o + s * 0.33, 1.25), k.coll, {"usage": "seat_eye"}, rotation_z=rz))
        for j in range(seats + 1):
            x = (j - seats / 2) * pitch
            k.box(f"{tag}_arm", (0.05, 0.36, 0.035), (x, o + s * 0.36, 0.66), "dark")
            k.box(f"{tag}_arm_pad", (0.056, 0.26, 0.02), (x, o + s * 0.38, 0.687), "rubber")
            k.box(f"{tag}_arm_post", (0.04, 0.04, 0.22), (x, o + s * 0.5, 0.54), "dark")
    for i in range(seats):
        x = (i - (seats - 1) / 2) * pitch
        k.box(f"{tag}_bracket", (0.06, 0.8 if rows == 2 else 0.5, 0.04), (x, beam_y if rows == 2 else o + 0.3, 0.38), "dark")
    k.box(f"{tag}_beam", (L, 0.12, 0.09), (0, beam_y, 0.32), "bare")
    for xl in (-L / 2 + 0.3, L / 2 - 0.3):
        k.box(f"{tag}_leg", (0.09, 0.09, 0.28), (xl, beam_y, 0.16), "dark")
        k.box(f"{tag}_foot", (0.12, 1.1 if rows == 2 else 0.6, 0.04), (xl, beam_y, 0.02), "dark", bevel=0.01)
    if rows == 2:
        k.box(f"{tag}_spine", (L, 0.06, 0.62), (0, 0, 0.79), "paint_black", bevel=0.01)
        k.box(f"{tag}_spine_cap", (L + 0.02, 0.1, 0.035), (0, 0, 1.11), "paint_yellow", bevel=0.006)
        k.col(f"{tag}", (L, 1.2, 1.1), (0, 0, 0.55))
    else:
        k.col(f"{tag}", (L, 0.7, 1.05), (0, o + 0.12, 0.525))
    k.end(center, facing)


def _blade(k: Cell, tag, base, height, width, lean, yaw, mat, thick=0.022):
    """Feuille en lame (sansevière) : section plate élargie à mi-hauteur, pointe ; penchée de
    `lean` vers +Y puis tournée de `yaw` (≈ 22 triangles, sans texture)."""
    import bmesh

    name = k.nm(tag)
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    w, t = width / 2, thick / 2
    low = [bm.verts.new(p) for p in ((-w, -t, 0.0), (w, -t, 0.0), (w, t, 0.0), (-w, t, 0.0))]
    mid = [bm.verts.new((x * 1.25, y * 0.8, height * 0.5)) for x, y, _ in ((-w, -t, 0), (w, -t, 0), (w, t, 0), (-w, t, 0))]
    tip = bm.verts.new((0.0, 0.0, height))
    for i in range(4):
        j = (i + 1) % 4
        bm.faces.new((low[i], low[j], mid[j], mid[i]))
        bm.faces.new((mid[i], mid[j], tip))
    bm.faces.new(list(reversed(low)))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(k.m[mat])
    obj = bpy.data.objects.new(name, mesh)
    k.coll.objects.link(obj)
    obj.matrix_basis = Matrix.Translation(Vector(base)) @ Matrix.Rotation(yaw, 4, "Z") @ Matrix.Rotation(-lean, 4, "X")
    c.uv_box(obj)
    return k._keep(obj)


def planter(k: Cell, tag, center, facing=0.0, size=(0.6, 0.6, 0.5), height=0.75, blades=11, seed=0):
    """Bac à plantes : caisson noir, lisse jaune, plinthe de caoutchouc, terre ; touffe de
    feuilles en lames (deux verts, sans texture), haute de `height` m. Collider (le bac seul)."""
    sx, sy, sz = size
    k.begin()
    k.box(f"{tag}_box", (sx, sy, sz), (0, 0, sz / 2), "paint_black", bevel=0.012, structure=True)
    for s in (-1, 1):
        k.box(f"{tag}_lip", (sx + 0.03, 0.05, 0.05), (0, s * (sy / 2 - 0.01), sz + 0.005), "paint_yellow")
        k.box(f"{tag}_lip", (0.05, sy - 0.07, 0.05), (s * (sx / 2 - 0.01), 0, sz + 0.005), "paint_yellow")
    k.box(f"{tag}_soil", (sx - 0.06, sy - 0.06, 0.02), (0, 0, sz + 0.01), "rubber")
    k.box(f"{tag}_kick", (sx + 0.012, sy + 0.012, 0.07), (0, 0, 0.035), "rubber")
    k.plate(f"{tag}_label", (min(sx, 0.5) * 0.6, 0.09), (0, sy / 2 + 0.002, sz * 0.62), "trim_sheet", "stencils", th=0.004)
    rx, ry = sx / 2 - 0.1, sy / 2 - 0.1
    for i in range(blades):
        f = (i * 0.618 + seed * 0.37) % 1.0
        phi = 2 * math.pi * f
        r = 0.25 + 0.75 * ((i * 7 + seed) % 5) / 4
        base = (math.cos(phi) * rx * r * 0.7, math.sin(phi) * ry * r * 0.7, sz + 0.02)
        hgt = height * (0.6 + 0.4 * ((i * 3 + seed) % 4) / 3)
        lean = 0.1 + 0.32 * r
        _blade(k, f"{tag}_leaf", base, hgt, 0.075, lean, phi - math.pi / 2, "plant_light" if (i + seed) % 4 == 0 else "plant")
    k.col(f"{tag}", (sx, sy, sz + 0.05), (0, 0, (sz + 0.05) / 2))
    k.end(center, facing)


def luggage_cart(k: Cell, tag, center, facing, load=()):
    """Chariot à bagages (poussé vers +Y local) : plateau sombre à tapis de caoutchouc sur quatre
    roulettes, cadre de tubes jaunes et poignée de poussée à l'arrière (−Y), rebord avant ;
    `load` : bagages [(taille, matériau)] empilés sur le plateau, décor de niveau 1. Collider."""
    W, L = 0.62, 1.0
    k.begin()
    k.box(f"{tag}_deck", (W, L, 0.05), (0, 0, 0.16), "dark", bevel=0.01)
    k.box(f"{tag}_mat", (W - 0.06, L - 0.12, 0.01), (0, 0.02, 0.19), "rubber")
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.box(f"{tag}_fork", (0.03, 0.06, 0.08), (sx * 0.25, sy * 0.4, 0.1), "dark")
            k.cyl(f"{tag}_wheel", 0.06, 0.04, (sx * 0.25, sy * 0.4, 0.06), "rubber", axis="X", segments=8)
        k.box(f"{tag}_upright", (0.04, 0.04, 0.95), (sx * 0.27, -L / 2 + 0.04, 0.66), "paint_yellow", bevel=0.008)
        k.box(f"{tag}_handle_arm", (0.03, 0.12, 0.03), (sx * 0.27, -L / 2 - 0.02, 1.12), "bare")
    for z in (0.55, 0.85):
        k.box(f"{tag}_rail", (W - 0.1, 0.03, 0.03), (0, -L / 2 + 0.04, z), "paint_yellow")
    k.cyl(f"{tag}_handle", 0.022, W + 0.04, (0, -L / 2 - 0.07, 1.12), "rubber", axis="X", segments=8)
    k.box(f"{tag}_lip", (W, 0.03, 0.09), (0, L / 2 - 0.015, 0.225), "paint_yellow")
    with k.detail(1):
        z = 0.195
        for i, (size, mat) in enumerate(load):
            k.box(f"{tag}_bag", size, (0.02 * (i % 2), -0.05 + 0.04 * (i % 3), z + size[2] / 2), mat, bevel=0.02)
            z += size[2]
    k.col(f"{tag}", (W + 0.06, L + 0.14, 1.15), (0, -0.03, 0.575))
    k.end(center, facing)


def hanging_sign(k: Cell, tag, center, facing, length=2.4, arrows=((1, 0.55), (-1, -0.55)), hang_to=None):
    """Panneau de direction suspendu, lisible des deux côtés (faces ±Y local) : caisson noir à
    liserés jaunes, bandes de chevrons de la trim sheet (chacune pointe vers +X ou −X local selon
    `arrows` = [(sens, position)]), tiges au plafond (`hang_to`, z absolu)."""
    k.begin()
    k.box(f"{tag}_box", (length, 0.1, 0.44), (0, 0, 0), "paint_black", bevel=0.012)
    for s in (-1, 1):
        k.box(f"{tag}_edge", (length + 0.02, 0.11, 0.03), (0, 0, s * 0.21), "paint_yellow")
    for d, xo in arrows:
        # Une seule plaque traverse le caisson : ses deux faces montrent les chevrons dans le même
        # sens (U de la bande le long de +X local de la plaque, tournée vers `d`).
        k.ori(f"{tag}_chevrons", (0.9, 0.2, 0.104), (xo, 0, -0.03), (d, 0, 0), (0, 0, 1), "trim_sheet", "chevrons")
    k.ori(f"{tag}_label", (0.5, 0.1, 0.104), (0, 0, 0.1), (1, 0, 0), (0, 0, 1), "trim_sheet", "stencils")
    if hang_to is not None:
        zt = hang_to - center[2]
        for x in (-length * 0.22, length * 0.22):
            k.box(f"{tag}_rod", (0.025, 0.025, zt - 0.22), (x, 0, (zt + 0.22) / 2), "dark")
    k.end(center, facing)


def stanchions(k: Cell, tag, points, h=0.95, rope="fabric"):
    """Barrière de potelets en laiton (socle, fût, boule) et barres matelassées entre eux,
    `points` = [(x, y)] alignés sur X ou sur Y ; collider sur chaque travée."""
    for x, y in points:
        k.cyl(f"{tag}_base", 0.15, 0.03, (x, y, 0.015), "brass", segments=16)
        k.cyl(f"{tag}_post", 0.025, h, (x, y, h / 2 + 0.02), "brass", segments=10)
        k.cyl(f"{tag}_cap", 0.045, 0.06, (x, y, h + 0.05), "brass", segments=10)
    for (xa, ya), (xb, yb) in zip(points[:-1], points[1:]):
        k.strut(f"{tag}_rope", (xa, ya, h - 0.12), (xb, yb, h - 0.12), 0.05, 0.06, rope)
        along_x = abs(xb - xa) >= abs(yb - ya)
        size = (abs(xb - xa), 0.12, h + 0.08) if along_x else (0.12, abs(yb - ya), h + 0.08)
        k.col(f"{tag}", size, ((xa + xb) / 2, (ya + yb) / 2, (h + 0.08) / 2))


def vault_door(k: Cell, tag, center, facing):
    """Porte de coffre (décor), face vers +Y local, `center` au pied du mur : cadre carré massif
    noir à bandes de danger et pochoirs, collerette de laiton, porte ronde de métal brut à deux
    gradins, couronne de boulons, moyeu et volant à trois rayons en laiton, gonds à droite (vu
    de face), pênes engagés dans leurs gâches à gauche, seuil d'acier. Fermée. Collider."""
    R, zc = 1.22, 1.45
    k.begin()
    k.box(f"{tag}_frame", (3.0, 0.32, 3.0), (0, 0.16, 1.5), "paint_black", bevel=0.03, structure=True)
    for z in (0.12, 2.86):
        k.ori(f"{tag}_frame_hazard", (2.9, 0.14, 0.008), (0, 0.324, z), *FRONT, "trim_sheet", "hazard")
    for x in (-1.05, 1.05):
        k.ori(f"{tag}_frame_label", (0.62, 0.1, 0.006), (x, 0.324, 2.62), *FRONT, "trim_sheet", "stencils")
    k._keep(kit._ring(k.nm(f"{tag}_collar"), R + 0.03, R + 0.24, 0.12, (0, 0.38, zc), k.m["brass"], k.coll, segments=32))
    k.cyl(f"{tag}_door", R, 0.36, (0, 0.5, zc), "bare", axis="Y", segments=32)
    k.cyl(f"{tag}_step", R - 0.2, 0.04, (0, 0.69, zc), "dark", axis="Y", segments=32)
    k.cyl(f"{tag}_face", R - 0.42, 0.04, (0, 0.72, zc), "bare", axis="Y", segments=28)
    for i in range(16):
        a = 2 * math.pi * i / 16
        k.cyl(f"{tag}_bolt", 0.045, 0.05, ((R - 0.1) * math.cos(a), 0.705, zc + (R - 0.1) * math.sin(a)), "dark", axis="Y", segments=8)
    k.cyl(f"{tag}_hub", 0.3, 0.14, (0, 0.79, zc), "dark", axis="Y", segments=20)
    k.cyl(f"{tag}_hub_cap", 0.13, 0.06, (0, 0.88, zc), "brass", axis="Y", segments=16)
    k._keep(kit._ring(k.nm(f"{tag}_wheel"), 0.56, 0.65, 0.06, (0, 0.88, zc), k.m["brass"], k.coll, segments=24))
    for i in range(3):
        a = math.pi / 2 + 2 * math.pi * i / 3
        k.strut(f"{tag}_spoke", (0.2 * math.cos(a), 0.88, zc + 0.2 * math.sin(a)), (0.6 * math.cos(a), 0.88, zc + 0.6 * math.sin(a)), 0.06, 0.05, "brass")
    # Vu de face, +X local est à gauche : gonds à −X (à droite), pênes à +X (à gauche).
    for dz in (-0.72, 0.72):
        k.cyl(f"{tag}_hinge", 0.15, 0.5, (-(R + 0.12), 0.55, zc + dz), "dark", axis="Z", segments=16)
        k.box(f"{tag}_hinge_arm", (0.5, 0.12, 0.3), (-(R - 0.08), 0.62, zc + dz), "bare", bevel=0.01)
    for dz in (-0.6, 0.0, 0.6):
        k.box(f"{tag}_keeper", (0.24, 0.22, 0.24), (R + 0.3, 0.43, zc + dz), "dark", bevel=0.01)
        k.box(f"{tag}_bolt_bar", (0.42, 0.1, 0.1), (R + 0.1, 0.52, zc + dz), "bare")
    k.box(f"{tag}_sill", (3.4, 1.1, 0.012), (0, 0.85, 0.006), "floor_plate")
    k.col(f"{tag}", (3.0, 1.0, 3.0), (0, 0.5, 1.5))
    k.end(center, facing)


def column_round(k: Cell, tag, x, y, h, r=0.24):
    """Colonne ronde de la Banque : fût noir, base et chapiteau à bagues de laiton ; collider."""
    k.cyl(f"{tag}_plinth", r + 0.08, 0.12, (x, y, 0.06), "paint_black", segments=20)
    k.cyl(f"{tag}_base_ring", r + 0.04, 0.05, (x, y, 0.145), "brass", segments=20)
    k.cyl(f"{tag}_shaft", r, h - 0.4, (x, y, 0.17 + (h - 0.4) / 2), "paint_black", segments=20)
    k.cyl(f"{tag}_band", r + 0.012, 0.08, (x, y, 1.6), "brass", segments=20)
    k.cyl(f"{tag}_capital_ring", r + 0.04, 0.05, (x, y, h - 0.205), "brass", segments=20)
    k.box(f"{tag}_capital", (2 * r + 0.2, 2 * r + 0.2, 0.18), (x, y, h - 0.09), "paint_black", bevel=0.015)
    k.col(f"{tag}", (2 * r + 0.16, 2 * r + 0.16, h), (x, y, h / 2))


# --- Cellules -----------------------------------------------------------------------------------


def build_hangar(m: dict) -> Cell:
    cid = "hangar"
    x0, x1, y0, y1, h = CELLS[cid]
    L = LAYOUT
    px0, px1, py0, py1 = L["pit"]
    hx, hy = L["hatch"]
    hull = L["hull"]
    k = Cell(cid, m)
    cxp = (px0 + px1) / 2
    bay_w = min(px1 - px0 + 1.0, x1 - x0 - 6.0)
    shell = hg.hangar_shell(cid, x0, x1, y0, y1, h, st.openings_for(cid, CELLS, WALL_PORTALS, WINDOWS), m, k.coll,
                            bay=BAY, truss_depth=TRUSS_DEPTH, floor_holes=[L["pit"]], bay_door=("N", cxp, bay_w, BAY_DOOR_H),
                            lamps=False, purlin_detail=1, web_detail=0)
    wy, ww = WINDOW[0], WINDOW[1]
    cut_wainscot(shell, cid, x0, wy - ww / 2, wy + ww / 2, m, k.coll)
    k.shell(shell)

    # Projecteurs sous les fermes (cuits) : sodium au-dessus du quai et du côté est, blanc froid
    # au-dessus des bords de la fosse (la coque, le jeu entre coque et paroi) une ferme sur deux.
    zbot = h - TRUSS_DEPTH
    trusses = [y0 + j * BAY for j in range(1, int((y1 - y0) / BAY) + 1) if y0 + j * BAY < y1 - 0.3]
    for j, u in enumerate(trusses):
        rows = [(x0 + 5.0, 60000.0, SODIUM)]
        if j % 2:
            rows += [(px0 + 1.0, 42000.0, COOL_W), (px1 - 1.0, 30000.0, COOL_W)]
        else:
            rows += [(x1 - 2.5, 34000.0, SODIUM)]
        for i, (x, lum, col) in enumerate(rows):
            k.add(hg.floodlight(f"{cid}_flood{j}_{i}", (x, u, zbot), 0.0, m, k.coll, lumens=lum, rng=30.0, color=col, drop=0.9))
    # Projecteurs muraux au-dessus du portique (bout sud, loin de la dernière ferme).
    for i, x in enumerate((x0 + 9.0, (x0 + x1) / 2 + 4.0, x1 - 6.0)):
        k.box("wall_arm", (0.12, 0.9, 0.12), (x, y0 + 0.45, 9.3), "dark")
        k.add(hg.floodlight(f"{cid}_wallflood{i}", (x, y0 + 0.8, 9.3), 0.0, m, k.coll, lumens=34000.0, rng=26.0, color=SODIUM, drop=0.25))

    # Berceau : fosse, garde-corps (ouverts à la passerelle), bras de maintien contre la coque,
    # sabots sous la coque, marquages. La lumière de la fosse vient de ses bandeaux (cuits).
    k.add(hg.docking_cradle(f"{cid}_cradle", px0, px1, py0, py1, m, k.coll, depth=PIT_DEPTH, gaps=[("W", hy, 1.7)],
                            arms=4, arm_reach=PIT_CLEARANCE - 0.15, saddles=4, saddle_height=L["saddle"], pit_lights=False))
    # Passerelle plane : du garde-corps au flanc de la coque (garde-corps), puis à travers la coque
    # jusqu'à la porte du sas (sans garde-corps : l'ouverture de la coque, lot C). Dessus à 4 mm
    # du sol (pas de scintillement sur la partie posée au sol).
    k.add(hg.gangway(f"{cid}_gangway", (px0 - 0.5, hy, 0.004), (hull[0], hy, 0.004), m, k.coll, width=1.6))
    k.add(hg.gangway(f"{cid}_gangway_in", (hull[0], hy, 0.004), (hx, hy, 0.004), m, k.coll, width=1.4, rails=(False, False)))
    # Murs invisibles : on ne saute ni dans la fosse (saut ≈ 1,08 m, garde-corps à 1,1 m) ni de la
    # passerelle.
    g = 0.5
    fx0, fx1, fy0, fy1 = px0 - g, px1 + g, py0 - g, py1 + g
    k.col("pit_fence", (fx1 - fx0, 0.1, 2.4), ((fx0 + fx1) / 2, fy0, 1.2))
    k.col("pit_fence", (fx1 - fx0, 0.1, 2.4), ((fx0 + fx1) / 2, fy1, 1.2))
    k.col("pit_fence", (0.1, fy1 - fy0, 2.4), (fx1, (fy0 + fy1) / 2, 1.2))
    for a, b in ((fy0, hy - 0.85), (hy + 0.85, fy1)):
        k.col("pit_fence", (0.1, b - a, 2.4), (fx0, (a + b) / 2, 1.2))
    for s in (-1, 1):
        k.col("gangway_guard", (hx - fx0, 0.1, 2.4), ((fx0 + hx) / 2, hy + s * 0.82, 1.2))

    # --- Quai (sol ouest) : niveau 0 -----------------------------------------------------------
    # Mur ouest, du sud au nord : colonnes tous les BAY m (sauf aux ouvertures), porte du couloir
    # (y = 0), fenêtre et porte du chantier (y ≈ 1,7 → 6,5) ; rien ne se pose devant une colonne.
    # Allée piétonne de la porte du couloir à la passerelle, ligne de cheminement le long du quai.
    for s in (-1, 1):
        k.add(kit.hazard_stripe(k.nm("lane"), (x0 + 0.7, hy + s * 1.25, 0.0), (px0 - 1.2, hy + s * 1.25, 0.0), m, k.coll, width=0.12))
    for a, b in kit._cuts(y0 + 1.0, y1 - 1.0, [(hy - 1.6, hy + 1.6)]):
        k.box("walkway", (0.12, b - a, 0.006), (x0 + 3.3, (a + b) / 2, 0.003), "paint_yellow")
    # Bornes de carburant et d'énergie au bord du quai, face au vaisseau.
    k.add(hg.fuel_bollard(f"{cid}_fuel", (px0 - 1.45, hy - 3.6, 0.0), FACE_W, m, k.coll))
    k.add(hg.fuel_bollard(f"{cid}_power", (px0 - 1.45, hy + 3.6, 0.0), FACE_W, m, k.coll, kind="power"))
    # Portes : caissons lumineux, écran des mouvements, armoires, gyrophares (voyants).
    door_sign(k, "sign_couloir", x0, 0.0, 2.72, FACE_W, width=1.4)
    door_sign(k, "sign_chantier", x0, 5.6, 2.72, FACE_W, width=1.4)
    k.add(kit.wall_screen(k.nm("movements"), (x0, -4.6, 1.85), FACE_W, m, k.coll, size=(1.4, 0.8)))
    k.add(kit.cabinet(k.nm("cabinet"), (x0, -7.0, 0.0), FACE_W, m, k.coll))
    k.add(kit.cabinet(k.nm("cabinet"), (x0, 8.4, 0.0), FACE_W, m, k.coll, size=(0.9, 0.32, 1.6)))
    for y in (-1.15, 1.15):
        k.add(kit.beacon(k.nm("beacon"), (x0, y, 2.55), FACE_W, m, k.coll, light=False))
    # Grands pochoirs (numéros de travée génériques) sur les murs, entre les colonnes.
    for y in (-14.0, 10.0):
        k.add(kit.stencil(k.nm("stencil"), (x0, y, 4.6), FACE_W, m, k.coll, length=3.2))
        k.add(kit.stencil(k.nm("stencil"), (x1, y, 4.6), FACE_E, m, k.coll, length=3.2))
    for x in (x0 + 8.0, x1 - 8.0):
        k.add(kit.stencil(k.nm("stencil"), (x, y0, 4.6), FACE_S, m, k.coll, length=3.2))
    # Fret du quai : conteneur et caisses dans les angles ouest, tracteur de piste au nord-ouest.
    k.add(hg.cargo_container(f"{cid}_cont_sw", (x0 + 1.6, y0 + 9.0, 0.0), math.pi / 2, m, k.coll, variant="rust"))
    k.add(hg.crate_stack(f"{cid}_crates_sw", (x0 + 1.6, y0 + 14.0, 0.0), math.pi / 2 + 0.06, m, k.coll, variants=("yellow", "slate", "olive")))
    k.add(hg.crate_stack(f"{cid}_crates_nw", (x0 + 1.6, y1 - 10.0, 0.0), math.pi / 2 - 0.05, m, k.coll, variants=("slate", "olive", "yellow")))
    tow_tractor(k, "tractor", (x0 + 6.0, y1 - 2.6, 0.0), 0.12)

    # --- Bout sud : portique roulant sur les conteneurs en attente -----------------------------
    yc = (y0 + py0 - g) / 2 - 0.2
    k.add(hg.gantry_crane(f"{cid}_gantry", (px1 - 7.0, yc, 0.0), math.pi / 2, m, k.coll, span=min(5.6, (py0 - g - y0) - 1.2),
                          height=8.0, rail_length=min(24.0, x1 - x0 - 16.0), hook_z=3.6))
    k.add(hg.cargo_container(f"{cid}_cont_s0", (px1 - 7.0, yc, 0.0), 0.0, m, k.coll, variant="slate"))
    k.add(hg.cargo_container(f"{cid}_cont_s1", (px0 + 5.0, yc, 0.0), 0.02, m, k.coll, variant="yellow"))
    # --- Côté est : conteneurs le long du mur --------------------------------------------------
    east = [(-18.0, "slate"), (-10.0, "rust"), (12.0, "olive"), (20.0, "slate")]
    for i, (y, variant) in enumerate(east):
        k.add(hg.cargo_container(f"{cid}_cont_e{i}", (x1 - 1.6, y, 0.0), math.pi / 2 + 0.01 * (i - 1.5), m, k.coll, variant=variant))
    k.add(hg.fuel_bollard(f"{cid}_power_e", (px1 + 1.45, hy - 4.0, 0.0), FACE_E, m, k.coll, kind="power"))

    # --- Niveau 1 (Moyen) ------------------------------------------------------------------------
    cols_w = [y0 + j * BAY for j in range(1, int((y1 - y0) / BAY) + 1) if y0 + j * BAY < y1 - 0.3]
    with k.detail(1):
        # Tuyauteries le long du mur ouest (devant les colonnes), consoles sur les colonnes.
        for i, (z, r) in enumerate(((4.0, 0.1), (4.35, 0.07))):
            k.add(kit.pipe(k.nm("pipe_w"), (x0 + 0.52, y0 + 0.3, z), (x0 + 0.52, y1 - 0.3, z), m, k.coll, radius=r, clamps=2.0))
        for y in cols_w:
            k.box("pipe_bracket", (0.2, 0.08, 0.5), (x0 + 0.48, y, 4.15), "dark")
        # Second rang de conteneurs, caisses sur les conteneurs.
        k.add(hg.cargo_container(f"{cid}_cont_sw2", (x0 + 1.6, y0 + 9.0, 2.6), math.pi / 2 + 0.015, m, k.coll, variant="slate"))
        k.add(hg.cargo_container(f"{cid}_cont_e1b", (x1 - 1.6, -10.0, 2.6), math.pi / 2 - 0.01, m, k.coll, variant="yellow"))
        k.add(hg.cargo_container(f"{cid}_cont_e3b", (x1 - 1.6, 20.0, 2.6), math.pi / 2 + 0.012, m, k.coll, variant="rust"))
        k.add(hg.cargo_container(f"{cid}_cont_s1b", (px0 + 5.0, yc, 2.6), -0.015, m, k.coll, variant="slate"))
        k.add(kit.crate(k.nm("top_crate"), (x1 - 1.4, -18.6, 2.6), 0.1, m, k.coll, size=(1.2, 1.2, 1.0), variant="yellow"))
        k.add(kit.crate(k.nm("top_crate"), (x1 - 1.8, 12.4, 2.6), -0.2, m, k.coll, size=(0.9, 0.9, 0.8), variant="slate"))
        # Écrans et boîtiers le long du quai, bouches d'aération hautes.
        k.add(kit.wall_screen(k.nm("screen"), (x0, -13.0, 1.8), FACE_W, m, k.coll, size=(1.0, 0.6)))
        k.add(kit.wall_screen(k.nm("screen"), (x0, 11.0, 1.8), FACE_W, m, k.coll, size=(1.0, 0.6)))
        for y in (-15.0, 17.5, 26.0):
            k.add(kit.junction_box(k.nm("jbox"), (x0, y, 1.6), FACE_W, m, k.coll, size=(0.55, 0.16, 0.7)))
        for x in (x0 + 4.0, (x0 + x1) / 2, x1 - 4.0):
            k.add(kit.vent(k.nm("vent"), (x, y0, 6.5), FACE_S, m, k.coll, size=(1.4, 0.6)))
        # Caillebotis d'évacuation aux angles de la fosse.
        for (x, y) in ((px0 - 2.6, py0 - 2.4), (px0 - 2.6, py1 + 2.4), (px1 + 2.4, py0 - 2.4), (px1 + 2.4, py1 + 2.4)):
            k.add(kit.floor_grating(k.nm("drain"), (x, y, 0.0), (1.2, 0.5), 0.0, m, k.coll, glow=None))

    # --- Niveau 2 (Haut) : très riche ------------------------------------------------------------
    with k.detail(2):
        # Coursive haute le long du mur ouest.
        catwalk(k, "catwalk", x0 + 0.42, y0 + 0.6, y1 - 0.6, 6.2, posts=cols_w)
        # Gaine de ventilation sous les fermes, au-dessus du quai (entre les projecteurs).
        k.add(kit.duct(k.nm("duct"), (x0 + 9.0, y0 + 0.3, 9.6), (x0 + 9.0, y1 - 0.3, 9.6), m, k.coll, size=(0.9, 0.6), hang_to=zbot))
        # Chemin de câbles au-dessus de la coursive.
        k.add(kit.cable_tray(k.nm("tray"), (x0 + 1.2, y0 + 0.3, 8.0), (x0 + 1.2, y1 - 0.3, 8.0), m, k.coll, width=0.5))
        for y in cols_w:
            k.box("tray_bracket", (0.9, 0.06, 0.08), (x0 + 0.85, y, 7.93), "dark")
        # Chaînes de levage pendues aux fermes, au-dessus du quai.
        for j, u in enumerate(trusses):
            chain(k, f"chain{j}", x0 + (2.6 if j % 2 else 6.6), u, zbot, 5.6 + (j % 3) * 0.8)
        # Fouillis au pied des murs : servantes, fûts, caisses, tuyau au sol des bornes.
        tool_chest(k, "chest", (x0 + 0.5, -8.6, 0.0), FACE_W)
        tool_chest(k, "chest", (x0 + 0.5, 10.0, 0.0), FACE_W)
        for i, (x, y, mat) in enumerate(((x0 + 0.5, y0 + 4.6, "crate_rust"), (x0 + 1.15, y0 + 4.2, "paint_yellow"),
                                         (x0 + 0.5, y0 + 3.9, "crate_slate"), (x1 - 0.5, -14.0, "paint_yellow"),
                                         (x1 - 0.55, -13.3, "crate_rust"), (x1 - 0.5, 16.0, "crate_slate"))):
            barrel(k, f"drum{i}", x, y, mat)
        crate_stack(k, "loose", (x0 + 0.8, y1 - 6.5, 0.0), 0.2, [(1.0, 0.9, 0.8), (0.7, 0.6, 0.5)], ["olive", "yellow"])
        crate_stack(k, "loose", (x1 - 0.8, -4.0, 0.0), -0.1, [(0.9, 0.9, 0.9), (0.6, 0.6, 0.5)], ["slate", "rust"])
        crate_stack(k, "loose", (x1 - 0.8, 4.0, 0.0), 0.15, [(1.1, 0.9, 0.7)], ["yellow"])
        crate_stack(k, "loose", (x0 + 10.0, y0 + 0.9, 0.0), 0.05, [(1.0, 1.0, 0.9), (0.8, 0.7, 0.6)], ["rust", "slate"])
        # Tuyaux des bornes : de l'enrouleur au bord de la fosse, posés au sol.
        for y in (hy - 3.6, hy + 3.6):
            k.cyl("hose", 0.045, 0.85, (px0 - 1.025, y - 0.38, 0.045), "rubber", axis="X", segments=8)
        # Échelles de visite dans le jeu de la fosse (sans interaction : décor), entre les nervures.
        for y in (py0 + 3.0, py1 - 3.0):
            for s in (-0.22, 0.22):
                k.box("pit_ladder", (0.05, 0.05, PIT_DEPTH - 0.2), (px0 + 0.1, y + s, -PIT_DEPTH / 2 - 0.1), "dark")
            for i in range(9):
                k.box("pit_rung", (0.03, 0.44, 0.03), (px0 + 0.1, y, -PIT_DEPTH + 0.3 + i * 0.3), "bare")
        # Écrans, boîtiers, pochoirs et gyrophares supplémentaires.
        k.add(kit.wall_screen(k.nm("screen"), (x0, -16.0, 1.8), FACE_W, m, k.coll, size=(0.8, 0.5)))
        k.add(kit.wall_screen(k.nm("screen"), (x0, 19.0, 1.8), FACE_W, m, k.coll, size=(0.8, 0.5)))
        k.add(kit.wall_screen(k.nm("screen"), (x0 + 12.0, y0, 2.0), FACE_S, m, k.coll, size=(1.2, 0.7)))
        for y in (-24.0, 20.0):
            k.add(kit.beacon(k.nm("beacon"), (x0, y, 5.2), FACE_W, m, k.coll, light=False))
        for x in (x0 + 16.0, x1 - 9.0):
            k.add(kit.junction_box(k.nm("jbox"), (x, y0, 1.6), FACE_S, m, k.coll, size=(0.6, 0.16, 0.8)))
        for y in (-28.0, -12.0, 24.0):
            k.add(kit.pipe(k.nm("riser"), (x0 + 0.3, y, 0.0), (x0 + 0.3, y, 6.0), m, k.coll, radius=0.06, clamps=1.5))
    return k


def build_couloir(m: dict) -> Cell:
    cid = "couloir"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [(-10.2, 0.0), (-6.4, 0.0)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, WALL_PORTALS, WINDOWS), m, k.coll,
                           floor_strip=True, style="industrial", avoid=lamps, dressing_detail=1))
    for i, (x, y) in enumerate(lamps):
        k.light(f"l{i}", x, y, h, 3500.0, 7.0, COOL)
    # Cadres aux deux bouts, rambardes (comme l'ancienne passerelle).
    for i, x in enumerate((-4.95, -11.55)):
        collar(k, f"collar{i}", x, 1.2, h, 0.45, 0.3)
    for s in (-1, 1):
        handrail(k, f"rail{s}", -11.2, -5.3, s * 0.97, s)
    k.add(kit.hazard_stripe(k.nm("stripe"), (-11.2, -0.8, 0.0), (-11.2, 0.8, 0.0), m, k.coll, width=0.16))
    k.add(kit.hazard_stripe(k.nm("stripe"), (-5.3, -0.8, 0.0), (-5.3, 0.8, 0.0), m, k.coll, width=0.16))
    k.add(kit.wall_screen(k.nm("screen"), (-5.8, -1.2, 1.55), FACE_S, m, k.coll, size=(0.6, 0.4)))
    k.add(kit.junction_box(k.nm("jbox"), (-5.9, 1.2, 1.45), FACE_N, m, k.coll, size=(0.45, 0.14, 0.5)))
    k.add(kit.vent(k.nm("vent"), (-8.3, 1.2, 1.55), FACE_N, m, k.coll, size=(0.7, 0.28)))
    k.add(kit.stencil(k.nm("stencil"), (-8.3, -1.2, 1.9), FACE_S, m, k.coll, length=1.2))
    # Gyrophares rouges de part et d'autre de la porte du hangar.
    for s, f in ((-1, FACE_S), (1, FACE_N)):
        k.add(kit.beacon(k.nm("beacon"), (-5.2, s * 1.2, 2.0), f, m, k.coll, lumens=700.0, rng=3.5))
    with k.detail(2):
        k.add(kit.junction_box(k.nm("jbox"), (-9.6, -1.2, 1.5), FACE_S, m, k.coll, size=(0.5, 0.14, 0.55)))
        k.add(kit.stencil(k.nm("stencil"), (-10.6, 1.2, 1.9), FACE_N, m, k.coll, length=1.0))
    return k


def build_chantier(m: dict) -> Cell:
    """Bureau du chantier naval : comptoir (PNJ `shipyard`) contre le mur ouest, face à la porte
    et à la fenêtre du hangar ; écrans de plans, pièces détachées, réacteur en démonstration."""
    cid = "chantier"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [(-10.0, 4.6), (-8.0, 2.6), (-6.0, 6.4), (-6.0, 2.6)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, WALL_PORTALS, WINDOWS), m, k.coll,
                           style="industrial", avoid=lamps))
    k.light("l_desk", *lamps[0], h, 5000.0, 6.0, COOL, facing=math.pi / 2)
    k.light("l0", *lamps[1], h, 4500.0, 7.0, AMBER, lens="light_amber")
    k.light("l1", *lamps[2], h, 4500.0, 7.0, AMBER, lens="light_amber")
    k.light("l2", *lamps[3], h, 3000.0, 6.0, COOL)
    # Comptoir : client côté est (+X), le PNJ entre le comptoir et le mur ouest. Colonnes du style
    # industriel sur les murs nord et sud à x = −9, −7, −5 : rien ne se pose devant.
    k.add(kit.counter(f"{cid}_guichet", (-10.0, 4.6, 0.0), FACE_W, m, k.coll, length=2.6, role="shipyard", variant="b", style="ship"))
    partition(k, "part_n", (-10.0, 5.9), (-10.0, y1), glass_top=0.0)
    partition(k, "part_s", (-10.0, y0), (-10.0, 3.3), glass_top=0.0)
    # Fronton au-dessus du comptoir : caisson, liseré, bandeau lumineux vers le bas.
    k.box("fascia", (0.5, 2.8, 0.36), (-10.55, 4.6, 2.62), "paint_black", bevel=0.012, structure=True)
    k.ori("fascia_stripe", (2.76, 0.07, 0.01), (-10.297, 4.6, 2.74), (0, 1, 0), (0, 0, 1), "paint_yellow")
    k.ori("fascia_label", (1.6, 0.11, 0.006), (-10.296, 4.6, 2.6), (0, 1, 0), (0, 0, 1), "trim_sheet", "stencils")
    k.ori("fascia_glow", (2.5, 0.08, 0.02), (-10.45, 4.6, 2.43), (0, 1, 0), (1, 0, 0), "band_cool", ("strip", 1.0))
    # Derrière le PNJ : plans de vaisseaux aux écrans muraux.
    for y in (3.8, 5.4):
        k.add(kit.wall_screen(k.nm("plans"), (x0, y, 1.8), FACE_W, m, k.coll, size=(0.9, 0.6)))
    # Côté client : étagère de pièces, armoire, tuyère de démonstration, banc.
    k.add(kit.shelf(k.nm("shelf"), (-8.0, y1, 0.0), FACE_N, m, k.coll, width=1.4, items_detail=1))
    k.add(kit.cabinet(k.nm("cabinet"), (-6.0, y1, 0.0), FACE_N, m, k.coll, size=(0.9, 0.32, 1.7)))
    k.add(kit.seat(f"{cid}_seat0", (-8.2, 2.1, 0.0), 0.0, m, k.coll))
    k.add(kit.seat(f"{cid}_seat1", (-7.4, 2.1, 0.0), 0.0, m, k.coll))
    # Tuyère de démonstration sur son berceau (gros objet : niveau 0, collider).
    k.begin()
    k.box("nozzle_stand", (1.3, 0.8, 0.4), (0, 0, 0.2), "dark", bevel=0.02)
    k.box("nozzle_stand_band", (1.32, 0.02, 0.12), (0, 0.41, 0.2), "paint_yellow")
    for sy in (-1, 1):
        k.box("nozzle_cradle", (0.12, 0.7, 0.36), (sy * 0.4, 0, 0.56), "dark", bevel=0.02)
    # Chambre de combustion, col, puis divergent en gradins (métal brut bleui, collerettes).
    k.cyl("nozzle_chamber", 0.26, 0.5, (-0.38, 0, 0.84), "bare", axis="X", segments=16)
    k.cyl("nozzle_throat", 0.18, 0.18, (-0.05, 0, 0.84), "dark", axis="X", segments=16)
    for i, (x, r) in enumerate(((0.12, 0.24), (0.3, 0.31), (0.48, 0.38), (0.64, 0.44))):
        k.cyl("nozzle_bell", r, 0.19, (x, 0, 0.84), "bare" if i % 2 else "dark", axis="X", segments=20)
    k.cyl("nozzle_ring", 0.29, 0.05, (-0.38, 0, 0.84), "paint_yellow", axis="X", segments=16)
    k.cyl("nozzle_cap", 0.2, 0.08, (-0.66, 0, 0.84), "dark", axis="X", segments=12)
    k.cyl("nozzle_feed", 0.05, 0.5, (-0.38, 0.0, 1.15), "trim_sheet", segments=10, uv="flange")
    k.col("nozzle", (1.5, 0.95, 1.3), (0.05, 0, 0.65))
    k.end((-7.0, 5.2, 0.0), 0.3)
    k.add(kit.hazard_frame(k.nm("nozzle_zone"), (-7.0, 5.2, 0.0), (1.9, 1.5), m, k.coll, width=0.1))
    k.add(kit.stencil(k.nm("stencil"), (-8.0, y0, 2.2), FACE_S, m, k.coll, length=1.2))
    k.add(kit.junction_box(k.nm("jbox"), (-6.0, y0, 1.4), FACE_S, m, k.coll, size=(0.45, 0.14, 0.55)))
    bin_(k, "bin", -9.25, 2.0)
    with k.detail(1):
        k.add(kit.pipe(k.nm("pipe"), (x0 + 0.2, y1 - 0.25, 2.6), (x1 - 0.2, y1 - 0.25, 2.6), m, k.coll, radius=0.06, hang_to=h))
        k.add(kit.crate(k.nm("parts"), (-5.0, 6.7, 0.0), 0.2, m, k.coll, size=(0.7, 0.6, 0.5), variant="slate"))
        k.add(kit.wall_screen(k.nm("screen"), (-8.0, y0, 1.6), FACE_S, m, k.coll, size=(0.7, 0.45)))
    with k.detail(2):
        tool_chest(k, "chest", (-6.0, 2.0, 0.0), FACE_S)
        k.add(kit.cable_tray(k.nm("tray"), (x0 + 0.3, 2.3, 2.75), (x1 - 0.3, 2.3, 2.75), m, k.coll, hang_to=h))
        k.add(kit.crate(k.nm("parts"), (-5.05, 6.65, 0.5), -0.3, m, k.coll, size=(0.45, 0.4, 0.3), variant="rust"))
        k.box("manual", (0.3, 0.22, 0.04), (-9.78, 5.7, 1.07), "fabric_dark", bevel=0.005)
        k.box("gauge", (0.16, 0.12, 0.1), (-9.8, 3.52, 1.1), "dark", bevel=0.01)
        k.cyl("gauge_dial", 0.045, 0.012, (-9.73, 3.52, 1.11), "buttons", axis="X", segments=12)
    return k


def build_hall(m: dict) -> Cell:
    """Hall des arrivées (M10, retour 13) : une allée centrale de l'entrée (couloir du hangar, à
    l'est) à la porte du marché (ouest), une allée transversale du bureau (nord) à la taverne
    (sud), dégagées toutes deux ; quatre banquettes dos à dos dont les rangées regardent l'un des
    deux grands panneaux d'information (au-dessus de la porte du marché et de l'entrée) ; bornes
    des départs sous le grand panneau ; comptoir d'information et chariots à bagages près de
    l'entrée ; porte de la Banque au sud de l'entrée ; bacs à plantes ; lignes, chevrons et
    tapis au sol ; panneau de direction suspendu au croisement. Lumière contrastée : blanc froid
    sur l'allée, suspensions sodium au-dessus des banquettes, lueur des panneaux (cuite)."""
    cid = "hall"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    bench_x = (-23.4, -16.6)  # centres des banquettes (x)
    bench_y = 4.07  # |y| du centre des banquettes
    aisle = [(-14.6, 0.0), (-17.6, 0.0), (-22.4, 0.0)]
    pendants = [(x, s * bench_y) for x in bench_x for s in (-1, 1)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, WALL_PORTALS, WINDOWS), m, k.coll,
                           style="station", avoid=aisle + pendants))
    for i, (x, y) in enumerate(aisle):
        k.light(f"aisle{i}", x, y, h, 20000.0, 12.0, COOL_W, drop=0.5)
    for i, (x, y) in enumerate(pendants):
        k.light(f"pendant{i}", x, y, h, 14000.0, 9.0, SODIUM, lens="light_amber", drop=1.5, facing=math.pi / 2)

    # --- Panneaux d'information : départs (ouest, au-dessus de la porte du marché), arrivées (est,
    # au-dessus de l'entrée). Lignes de glyphes, aucun texte lisible.
    info_board(k, "board_w", (x0 + 0.24, 0.0, 3.55), FACE_W,
               ["text_orange", "text_orange", "text_orange", "text_orange", "status_orange"], rows=3, glow="band_warm", standoff=0.08)
    info_board(k, "board_e", (x1 - 0.24, 0.0, 3.62), FACE_E,
               ["text_cyan", "text_cyan", "text_cyan", "status_green"], rows=2, cell=(0.9, 0.4), glow="band_cool", standoff=0.08)
    # Bornes des départs sous le grand panneau, de part et d'autre de la porte du marché.
    for i, y in enumerate((-1.95, 1.95)):
        k.add(kit.console(f"{cid}_departures{i}", (x0 + 0.55, y, 0.0), FACE_W, m, k.coll, label="departures"))

    # --- Banquettes dos à dos (rangées le long de Y : un côté vers l'ouest, l'autre vers l'est),
    # un bac à plantes au bout côté allée.
    for i, (x, s) in enumerate((bx, s) for bx in bench_x for s in (-1, 1)):
        bench_block(k, f"bench{i}", (x, s * bench_y, 0.0), math.pi / 2, seats=4)
        planter(k, f"bench_planter{i}", (x, s * 2.28, 0.0), size=(1.2, 0.55, 0.42), height=0.62, blades=13, seed=i)
    # Grands bacs dans les angles, de part et d'autre de l'entrée.
    planter(k, "planter_nw", (x0 + 0.7, y1 - 0.7, 0.0), size=(0.8, 0.8, 0.55), height=1.25, blades=16, seed=5)
    planter(k, "planter_sw", (x0 + 0.7, y0 + 0.7, 0.0), size=(0.8, 0.8, 0.55), height=1.25, blades=16, seed=6)
    planter(k, "planter_se", (x1 - 0.65, y0 + 0.65, 0.0), size=(0.7, 0.7, 0.5), height=1.1, blades=14, seed=7)
    planter(k, "planter_entry", (x1 - 0.36, -2.1, 0.0), size=(0.55, 0.55, 0.5), height=0.9, blades=11, seed=8)
    bin_(k, "bin0", -21.45, 2.2)
    bin_(k, "bin1", -18.55, -2.2)

    # --- Près de l'entrée : comptoir d'information (nord), chariots à bagages rangés (angle
    # nord-est), un chariot chargé laissé là (sud-est).
    kiosk(k, (x1 - 0.75, 3.4, 0.0), FACE_E)
    for i, x in enumerate((-15.25, -14.55, -13.85)):
        luggage_cart(k, f"cart{i}", (x, 6.15, 0.0), math.pi)
    k.add(kit.hazard_frame(k.nm("cart_bay"), (-14.55, 6.12, 0.0), (2.2, 1.12), m, k.coll, width=0.08))
    luggage_cart(k, "cart_loose", (-14.95, -6.0, 0.0), 0.35,
                 load=[((0.5, 0.74, 0.24), "fabric_dark"), ((0.42, 0.56, 0.2), "crate_slate"), ((0.34, 0.44, 0.3), "crate_rust")])
    k.add(kit.cabinet(k.nm("cabinet"), (-23.4, y0, 0.0), FACE_S, m, k.coll, size=(0.9, 0.32, 1.6)))

    # --- Signalétique : portes, panneau suspendu au croisement (chevrons vers le bureau et la
    # taverne), lignes jaunes des allées, chevrons au sol, tapis de l'entrée et de la Banque.
    door_sign(k, "sign_bureau", -20.0, y1, 2.95, FACE_N, accent="paint_yellow", width=1.4)
    door_sign(k, "sign_taverne", -20.0, y0, 2.95, FACE_S, accent="paint_yellow", width=1.4)
    door_sign(k, "sign_banque", x1, -4.6, 2.95, FACE_E, accent="crate_slate", width=1.4)
    # Au-dessus de la ligne du grand panneau vue depuis l'entrée (sinon il le coupe).
    hanging_sign(k, "sign_cross", (-20.0, 0.0, 4.2), math.pi / 2, length=2.6, arrows=((1, 0.62), (-1, -0.62)), hang_to=h)
    ax0, ax1, ae = x0 + 1.0, x1 - 1.2, 1.75  # allée centrale : bords en y = ±ae
    cx0, cx1 = -21.0, -19.0  # allée transversale
    for s in (-1, 1):
        for a, b in ((ax0, cx0), (cx1, ax1)):
            k.box("aisle_line", (b - a, 0.08, 0.006), ((a + b) / 2, s * ae, 0.003), "paint_yellow")
        for xl in (cx0, cx1):
            a, b = ae, 6.0
            k.box("cross_line", (0.08, b - a, 0.006), (xl, s * (a + b) / 2, 0.003), "paint_yellow")
    for (xa, xb, y) in ((-14.4, -16.0, 0.8), (-22.6, -24.2, 0.8), (-16.0, -14.4, -0.8), (-24.2, -22.6, -0.8)):
        k.add(kit.hazard_stripe(k.nm("chevrons"), (xa, y, 0.0), (xb, y, 0.0), m, k.coll, width=0.45, trim="chevrons"))
    for s in (-1, 1):
        k.add(kit.hazard_stripe(k.nm("chevrons"), (-20.0, s * 2.4, 0.0), (-20.0, s * 4.2, 0.0), m, k.coll, width=0.45, trim="chevrons"))
    k.add(kit.hazard_stripe(k.nm("chevrons"), (-13.5, -2.3, 0.0), (-13.5, -3.7, 0.0), m, k.coll, width=0.35, trim="chevrons"))
    k.box("mat_entry", (0.9, 2.3, 0.008), (x1 - 0.6, 0.0, 0.004), "rubber")
    k.box("mat_bank", (0.7, 1.5, 0.008), (x1 - 0.5, -4.6, 0.004), "fabric_dark")

    # --- Murs et plafond : gaine, chemin de câbles, boîtiers, bouches, gyrophares de l'entrée.
    k.add(kit.duct(k.nm("duct"), (x0 + 0.2, 6.35, 4.05), (x1 - 0.2, 6.35, 4.05), m, k.coll, size=(0.7, 0.45), hang_to=h))
    k.add(kit.cable_tray(k.nm("tray"), (-19.2, y0 + 0.2, 4.55), (-19.2, y1 - 0.2, 4.55), m, k.coll, hang_to=h))
    k.add(kit.junction_box(k.nm("jbox"), (x0, 3.6, 1.5), FACE_W, m, k.coll, size=(0.6, 0.16, 0.7)))
    k.add(kit.junction_box(k.nm("jbox"), (-14.6, y0, 1.5), FACE_S, m, k.coll, size=(0.5, 0.14, 0.6)))
    k.add(kit.vent(k.nm("vent"), (-24.0, y1, 2.6), FACE_N, m, k.coll, size=(1.0, 0.4)))
    k.add(kit.vent(k.nm("vent"), (-16.0, y0, 2.6), FACE_S, m, k.coll, size=(1.0, 0.4)))
    for s in (-1, 1):
        k.add(kit.beacon(k.nm("beacon"), (x1, s * 1.95, 2.45), FACE_E, m, k.coll, lumens=800.0, rng=4.0))
    with k.detail(1):
        k.add(kit.pipe(k.nm("pipe"), (x0 + 0.4, y0 + 0.2, 4.7), (x0 + 0.4, y1 - 0.2, 4.7), m, k.coll, radius=0.08))
        k.add(kit.wall_screen(k.nm("screen"), (x0, -4.2, 2.3), FACE_W, m, k.coll, size=(1.0, 0.6)))
        k.add(kit.wall_screen(k.nm("screen"), (-16.0, y1, 1.9), FACE_N, m, k.coll, size=(0.9, 0.5)))
        k.add(kit.stencil(k.nm("stencil"), (-16.0, y1, 2.5), FACE_N, m, k.coll, length=1.4))
    with k.detail(2):
        k.add(kit.duct(k.nm("duct"), (x0 + 0.2, -6.35, 4.05), (-16.8, -6.35, 4.05), m, k.coll, size=(0.6, 0.4), hang_to=h))
        k.add(kit.pipe(k.nm("pipe"), (x1 - 0.4, y0 + 0.2, 4.65), (x1 - 0.4, y1 - 0.2, 4.65), m, k.coll, radius=0.06, hang_to=h))
        barrel(k, "drum", x0 + 0.6, -4.9, "paint_yellow")
        k.add(kit.vent(k.nm("vent"), (-19.2, y1, 3.9), FACE_N, m, k.coll, size=(0.8, 0.3)))
        bin_(k, "bin2", -24.6, 6.4)
    return k


def build_bureau(m: dict) -> Cell:
    """Bureau des contrats : comptoir des contrats contre le mur nord (PNJ `contracts`), console
    des contrats à côté, cloison vitrée : on ne passe pas derrière le comptoir. M10 : le guichet
    de la Banque a sa salle (`build_banque`) ; à sa place, un tableau d'affichage des contrats,
    une armoire et un bac à plantes contre le mur est."""
    cid = "bureau"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [(-21.6, 9.8), (-18.6, 9.8), (-21.6, 12.75), (-16.5, 10.6)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, WALL_PORTALS, WINDOWS), m, k.coll,
                           style="station", rib_every=8.0 / 3.0, avoid=lamps))
    for i, (x, y) in enumerate(lamps[:2]):
        k.light(f"l{i}", x, y, h, 8000.0, 8.0, NEUTRAL)
    k.light("l_guichet", *lamps[2], h, 4000.0, 6.0, COOL)
    k.light("l_est", *lamps[3], h, 3500.0, 6.0, NEUTRAL, facing=math.pi / 2)
    # Comptoir des contrats (pièce maîtresse) et sa console.
    ccx, ccy, cl = -21.6, 12.25, 3.2
    k.add(kit.counter(f"{cid}_guichet", (ccx, ccy, 0.0), math.pi, m, k.coll, length=cl, role="contracts", variant="a"))
    k.add(kit.console(f"{cid}_contracts", (-19.0, 12.9, 0.0), math.pi, m, k.coll, label="contracts"))
    partition(k, "part_w", (x0, 12.3), (ccx - cl / 2, 12.3))
    # Fronton au-dessus du comptoir : caisson noir, liseré jaune, écran de file d'attente,
    # pochoirs, bandeau lumineux vers le bas.
    fx0, fx1 = ccx - cl / 2 - 0.1, ccx + cl / 2 + 0.1
    fl = fx1 - fx0
    fc = (fx0 + fx1) / 2
    fy = ccy + 0.05
    k.box("fascia", (fl, 0.5, 0.42), (fc, fy + 0.25, 2.55), "paint_black", bevel=0.012, structure=True)
    k.ori("fascia_stripe", (fl - 0.04, 0.08, 0.01), (fc, fy - 0.006, 2.7), *BACK, "paint_yellow")
    k.ori("fascia_hazard", (fl - 0.04, 0.1, 0.008), (fc, fy - 0.005, 2.4), *BACK, "trim_sheet", "hazard")
    for x in (fc - 1.05, fc + 1.05):
        k.ori("fascia_label", (0.9, 0.11, 0.006), (x, fy - 0.004, 2.56), *BACK, "trim_sheet", "stencils")
    k.box("queue_bezel", (0.8, 0.05, 0.28), (fc, fy - 0.024, 2.55), "dark", bevel=0.008)
    k.ori("queue_screen", (0.72, 0.2, 0.01), (fc, fy - 0.054, 2.55), *BACK, "screen", tile=0.8)
    k.ori("fascia_glow", (fl - 0.3, 0.08, 0.02), (fc, fy + 0.12, 2.33), (1, 0, 0), (0, 1, 0), "band_warm", ("strip", 1.0))
    # Derrière le guichetier : écrans muraux (entre les pilastres du mur nord).
    for x in (ccx - 0.8, ccx + 1.0):
        k.add(kit.wall_screen(k.nm("back_screen"), (x, y1, 1.75), FACE_N, m, k.coll, size=(0.8, 0.5)))
    k.add(kit.cabinet(k.nm("cabinet"), (-23.62, y1, 0.0), FACE_N, m, k.coll, size=(0.7, 0.32, 1.7)))
    # Mur est (l'ancien guichet de la Banque, M10) : tableau d'affichage des contrats (écrans à
    # lignes de glyphes), armoire, bac à plantes ; les pilastres du mur sont en y ≈ 10,1 et 12,7.
    k.box("notice_case", (0.1, 1.5, 1.0), (x1 - 0.05, 11.4, 1.75), "paint_black", bevel=0.012)
    for i, (y, tile) in enumerate(((11.75, "text_green"), (11.05, "status_green"))):
        k.screen("notice_scr", (0.62, 0.84), (x1 - 0.106, y, 1.75), (0, -1, 0), (0, 0, 1), tile)
    k.box("notice_head", (0.11, 1.5, 0.08), (x1 - 0.06, 11.4, 2.3), "paint_yellow")
    k.add(kit.cabinet(k.nm("cabinet"), (x1, 8.6, 0.0), FACE_E, m, k.coll, size=(0.9, 0.32, 1.6)))
    planter(k, "planter", (x1 - 0.45, y1 - 0.45, 0.0), size=(0.55, 0.55, 0.5), height=1.0, blades=12, seed=3)
    # Salle d'attente : sièges contre le mur ouest, tableau d'affichage, corbeille.
    for i, y in enumerate((8.9, 9.65, 10.4)):
        k.add(kit.seat(f"{cid}_seat{i}", (-23.55, y, 0.0), FACE_W, m, k.coll))
    k.add(kit.wall_screen(k.nm("notice"), (x0, 11.45, 1.75), FACE_W, m, k.coll, size=(1.0, 0.6)))
    k.add(kit.stencil(k.nm("stencil"), (x0, 11.45, 2.3), FACE_W, m, k.coll, length=1.0))
    k.add(kit.vent(k.nm("vent"), (-22.8, y0, 1.85), FACE_S, m, k.coll, size=(0.8, 0.3)))
    bin_(k, "bin", -23.6, 7.9)
    # File d'attente : deux lignes au sol de la porte au comptoir des contrats.
    for s in (-1, 1):
        k.add(kit.hazard_stripe(k.nm("queue"), (ccx + 0.6 + s * 0.6, 8.0, 0.0), (ccx + 0.6 + s * 0.6, 11.2, 0.0), m, k.coll, width=0.08))
    with k.detail(1):
        k.add(kit.junction_box(k.nm("jbox"), (-18.6, y0, 1.4), FACE_S, m, k.coll, size=(0.45, 0.14, 0.55)))
        for x, w_ in ((ccx - 1.1, 0.42), (ccx + 1.2, 0.3)):
            k.box("binder", (w_, 0.26, 0.16), (x, ccy + 0.12, 1.13), "crate_slate", bevel=0.005)
    with k.detail(2):
        k.add(kit.pipe(k.nm("pipe"), (x0 + 0.2, y0 + 0.25, 2.75), (x1 - 0.2, y0 + 0.25, 2.75), m, k.coll, radius=0.05, hang_to=h))
        bin_(k, "bin", -18.2, 7.9)
    return k


# Tables de la taverne (centres) : M10 (retour 14), la seconde table et ses sièges, qui barraient
# l'axe de la porte (x = −20), passent à l'est (à gauche en entrant) : de la porte au tavernier,
# un passage libre de x ≈ −21,0 (siège de la première table) à x ≈ −19,15 (siège de la seconde).
TAVERN_TABLES = ((-22.4, -9.6), (-17.75, -9.6))
# Tabourets du bar : aucun dans le passage, ni devant le tavernier (x = −20,6).
TAVERN_STOOLS = (-22.35, -21.5, -18.95)


def build_taverne(m: dict) -> Cell:
    cid = "taverne"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [TAVERN_TABLES[0], TAVERN_TABLES[1], (-21.0, -12.0), (-17.0, -8.2)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, WALL_PORTALS, WINDOWS), m, k.coll,
                           style="station", avoid=lamps))
    for i, (x, y) in enumerate(lamps[:2]):
        k.light(f"l{i}", x, y, h, 4500.0, 6.0, AMBER, lens="light_amber", drop=0.85)
    k.light("l_bar", lamps[2][0], lamps[2][1], h, 3000.0, 5.0, WARM, lens="light_amber", drop=0.5)
    k.light("l_coin", lamps[3][0], lamps[3][1], h, 1800.0, 5.0, AMBER, lens="light_amber", drop=0.6)
    # Bar : console « bar » au bout du comptoir (côté est), tenancier (PNJ) derrière.
    k.add(kit.console(f"{cid}_tap", (-17.6, -12.9, 0.0), 0.0, m, k.coll, label="bar"))
    y_front, depth = -11.8, 0.65
    t0 = bar_counter(k, "bar", -22.9, -18.45, y_front, depth, 0.0)
    edge = y_front + t0  # bord du plateau côté employé
    sx, sy = -20.6, edge - 0.195
    front, back = y_front + 0.25, sy - 0.32
    k.npc(f"{cid}_barman", (sx, sy), 0.0, "bar", "c", (2.4, front - back), (front + back) / 2 - sy)
    for i, x in enumerate((-18.95, -18.75, -18.55)):
        k.cyl("tap", 0.02, 0.28, (x, -12.0, 1.19), "bare", segments=8)
        k.box("tap_handle", (0.03, 0.03, 0.12), (x, -12.0, 1.39), "paint_yellow" if i == 1 else "paint_black")
    back_bar(k, "backbar_w", -23.6, -20.35, y0)
    back_bar(k, "backbar_e", -19.65, -18.4, y0)
    k.add(kit.wall_screen(k.nm("menu"), (-24.0, -10.5, 1.75), FACE_W, m, k.coll, size=(1.0, 0.6)))
    for i, x in enumerate(TAVERN_STOOLS):
        stool(k, f"stool{i}", x, -11.35)
    for i, (x, y) in enumerate(TAVERN_TABLES):
        k.add(kit.table(f"{cid}_table{i}", (x, y, 0.0), 0.0, m, k.coll, size=(1.2, 1.2)))
        k.add(kit.seat(f"{cid}_seat{i}a", (x - 1.0, y, 0.0), -math.pi / 2, m, k.coll))
        k.add(kit.seat(f"{cid}_seat{i}b", (x + 1.0, y, 0.0), math.pi / 2, m, k.coll))
        with k.detail(1):
            for j, (dx, dy) in enumerate(((-0.25, 0.2), (0.3, -0.15))):
                k.cyl("mug", 0.045, 0.12, (x + dx, y + dy + 0.05 * i, 0.85), "bare", segments=8)
    k.cyl("high_foot", 0.25, 0.04, (-17.0, -8.2, 0.02), "dark", segments=12)
    k.cyl("high_post", 0.05, 1.0, (-17.0, -8.2, 0.54), "bare", segments=8)
    k.cyl("high_top", 0.38, 0.05, (-17.0, -8.2, 1.06), "dark", segments=16)
    k.col("high_table", (0.78, 0.78, 1.1), (-17.0, -8.2, 0.55))
    keg(k, "keg0", -23.55, -12.0)
    keg(k, "keg1", -23.5, -11.35)
    keg(k, "keg2", -23.45, -8.0)
    k.add(kit.junction_box(k.nm("jbox"), (-16.0, -11.5, 1.5), FACE_E, m, k.coll, size=(0.45, 0.14, 0.55)))
    k.add(kit.vent(k.nm("vent"), (-24.0, -12.6, 2.4), FACE_W, m, k.coll, size=(0.8, 0.3)))
    with k.detail(1):
        k.add(kit.pipe(k.nm("pipe"), (-23.85, -7.6, 2.95), (-16.15, -7.6, 2.95), m, k.coll, radius=0.06, hang_to=h))
        k.cyl("mug", 0.045, 0.12, (-17.1, -8.1, 1.145), "bare", segments=8)
    with k.detail(2):
        for i, x in enumerate((-22.0, -21.1)):
            k.cyl("glass", 0.035, 0.1, (x, -11.55, 1.1), "bottle", segments=8)
        k.cyl("bottle", 0.04, 0.24, (-19.5, -11.6, 1.17), "bottle_green", segments=6)
        k.add(kit.wall_screen(k.nm("screen"), (-16.0, -9.6, 2.0), FACE_E, m, k.coll, size=(0.9, 0.55)))
        k.add(kit.light_band(k.nm("band"), (-23.8, -7.65, 2.6), (-16.2, -7.65, 2.6), m, k.coll, normal=(0, -0.6, -0.8), tone="amber"))
    return k


def build_marche(m: dict) -> Cell:
    cid = "marche"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    lamps = [(x, y) for x in (-38.0, -34.0, -30.0) for y in (-2.0, 2.0)]
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, WALL_PORTALS, WINDOWS), m, k.coll,
                           style="station", avoid=lamps))
    for i, (x, y) in enumerate(lamps):
        k.light(f"l{i}", x, y, h, 14000.0, 10.0, WARM, drop=0.9)
    k.add(kit.console(f"{cid}_terminal", (-28.4, 5.5, 0.0), math.pi, m, k.coll, label="market"))
    k.add(kit.wall_screen(k.nm("prices"), (-30.0, y1, 2.85), FACE_N, m, k.coll, size=(2.4, 0.9)))
    k.add(kit.stencil(k.nm("stencil"), (-30.0, y1, 2.2), FACE_N, m, k.coll, length=2.2))
    wares = [
        [(-0.7, (0.5, 0.4, 0.3), "crate_yellow"), (-0.1, (0.4, 0.35, 0.25), "crate_slate"), (0.6, (0.6, 0.4, 0.2), "bare")],
        [(-0.6, (0.6, 0.45, 0.35), "crate_rust"), (0.3, (0.5, 0.4, 0.15), "dark"), (0.8, (0.3, 0.3, 0.3), "crate_yellow")],
        [(-0.8, (0.4, 0.4, 0.4), "crate_slate"), (-0.2, (0.5, 0.3, 0.12), "bare"), (0.55, (0.55, 0.45, 0.3), "crate_rust")],
        [(-0.5, (0.7, 0.4, 0.25), "crate_yellow"), (0.4, (0.4, 0.35, 0.35), "crate_rust")],
    ]
    # Étals face à l'allée centrale ; le plus proche de la porte (sud) est le comptoir du marchand.
    stalls = [((-36.5, -3.2), 0.0, "crate_slate", "crate_rust", None),
              ((-31.5, -3.2), 0.0, "crate_rust", "crate_slate", ("market", "b")),
              ((-36.5, 3.2), math.pi, "crate_rust", "crate_yellow", None),
              ((-31.5, 3.2), math.pi, "crate_slate", "crate_rust", None)]
    for i, ((x, y), f, body, roof, counter) in enumerate(stalls):
        stall(k, f"stall{i}", (x, y, 0.0), f, body, roof, wares[i], counter=counter)
        back = -1 if y < 0 else 1
        crate_stack(k, f"stock{i}", (x - 0.6, y + back * 1.6, 0.0), 0.2 * i, [(0.8, 0.7, 0.7), (0.6, 0.5, 0.5)], ["yellow" if i % 2 else "slate", "rust"])
        crate_stack(k, f"stock{i}b", (x + 0.7, y + back * 1.65, 0.0), -0.1 * i, [(0.7, 0.7, 0.6)], ["slate" if i % 2 else "yellow"])
    for i, (y, variant) in enumerate(((-4.6, "rust"), (-2.0, "slate"), (2.2, "yellow"), (4.7, "slate"))):
        crate_stack(k, f"container{i}", (-39.15, y, 0.0), math.pi / 2, [(2.3, 1.3, 1.25)] + ([(2.0, 1.2, 1.1)] if i in (0, 2) else []), [variant, "slate" if variant != "slate" else "rust"])
    crate_stack(k, "pallet", (-27.4, -4.9, 0.0), 0.12, [(1.2, 1.0, 0.8), (0.8, 0.7, 0.6)], ["yellow", "slate"])
    crate_stack(k, "pallet", (-28.8, -5.2, 0.0), -0.2, [(0.9, 0.9, 0.9)], ["rust"])
    crate_stack(k, "pallet", (-26.95, 5.25, 0.0), 0.1, [(0.8, 0.8, 0.7), (0.6, 0.5, 0.45)], ["slate", "yellow"])
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
    k.end((-34.1, -1.15, 0.0), 0.08)
    crate_stack(k, "cart_load", (-34.25, -1.15, 0.23), 0.12, [(0.7, 0.6, 0.5), (0.5, 0.45, 0.35)], ["yellow", "slate"])
    k.add(kit.junction_box(k.nm("jbox"), (-26.4, 3.2, 1.5), FACE_E, m, k.coll, size=(0.5, 0.14, 0.6)))
    k.add(kit.junction_box(k.nm("jbox"), (-34.0, -6.0, 1.6), FACE_S, m, k.coll, size=(0.6, 0.14, 0.7)))
    k.add(kit.vent(k.nm("vent"), (-34.0, 6.0, 2.6), FACE_N, m, k.coll, size=(1.0, 0.4)))
    door_sign(k, "sign_hall", -26.4, 0.0, 2.95, FACE_E, accent="paint_yellow", width=1.4)
    with k.detail(1):
        k.add(kit.cable_tray(k.nm("tray"), (-39.8, 0.9, 3.7), (-26.6, 0.9, 3.7), m, k.coll, hang_to=h))
        k.add(kit.pipe(k.nm("pipe"), (-39.8, -0.9, 3.85), (-26.6, -0.9, 3.85), m, k.coll, radius=0.06, clamps=2.0, hang_to=h))
    with k.detail(2):
        k.add(kit.duct(k.nm("duct"), (-39.8, -4.6, 3.35), (-26.6, -4.6, 3.35), m, k.coll, size=(0.6, 0.4), hang_to=h))
        barrel(k, "drum", -39.4, 5.6, "crate_rust")
        k.add(kit.wall_screen(k.nm("screen"), (-38.0, y0, 2.4), FACE_S, m, k.coll, size=(1.0, 0.6)))
    return k


def deposit_wall(k: Cell, tag, center, facing, width=2.2, height=2.2, cols=5, rows=7):
    """Mur de coffres individuels (décor de la Banque), face vers +Y local, `center` au pied du
    mur : bâti sombre, portes de tôle peinte décalées, boutons de laiton, socle ; collider."""
    k.begin()
    k.box(f"{tag}_body", (width, 0.45, height), (0, 0.225, height / 2), "dark", bevel=0.01, structure=True)
    dw, dh = (width - 0.1) / cols, (height - 0.25) / rows
    for j in range(rows):
        for i in range(cols):
            x = -width / 2 + 0.05 + dw * (i + 0.5)
            z = 0.18 + dh * (j + 0.5)
            k.plate(f"{tag}_door", (dw - 0.035, dh - 0.035), (x, 0.455, z), "panel", ("box", (0.31 * i, 0.17 * j)), th=0.008)
            k.box(f"{tag}_knob", (0.05, 0.02, 0.025), (x + dw * 0.3, 0.467, z), "brass")
    k.box(f"{tag}_plinth", (width + 0.01, 0.46, 0.1), (0, 0.23, 0.05), "rubber")
    k.box(f"{tag}_crown", (width + 0.04, 0.48, 0.06), (0, 0.24, height + 0.03), "brass")
    k.col(f"{tag}", (width, 0.47, height + 0.06), (0, 0.235, (height + 0.06) / 2))
    k.end(center, facing)


def build_banque(m: dict) -> Cell:
    """Salle de la Banque (M10, retour 16 ; Banque d'Hexa-Corp, P25) : on entre par le hall
    (ouest) face au guichet. « Futur usé » mais cossu : dalles sombres, murs de tôle peinte gris
    bleu, laiton, colonnes rondes, bandeaux chauds.

    - ligne des guichets (x = −6,6) : guichet de la Banque (PNJ `bank`, comptoir du kit, vitre
      au-dessus du plateau), guichet fermé (rideau de tôle derrière la vitre), cloisons vitrées,
      fronton à écrans de cours (barres, courbes : aucun texte lisible) ; derrière, le personnel :
      mur de coffres individuels, bureau, armoire (on n'y passe pas) ;
    - au sud, la porte de coffre massive (décor) entre deux colonnes, derrière une barrière de
      potelets ; banquette et écran de cours le long du mur ouest ;
    - au nord, banquette d'attente tournée vers les guichets, écrans de cours, bacs à plantes."""
    cid = "banque"
    x0, x1, y0, y1, h = CELLS[cid]
    k = Cell(cid, m)
    # Coque de station, mais sol de dalles sombres et murs de tôle peinte (palette surchargée).
    mb = dict(m)
    mb["tiles"] = m["bank_floor"]
    mb["concrete"] = m["bank_wall"]
    lamps = [(-10.3, -3.0), (-9.4, -5.6), (-5.5, -4.6), (-8.2, -9.3), (-5.4, -9.9)]
    # Pilastres tous les 3,8 m : un au milieu des murs nord et sud (celui du sud couronne le
    # coffre), deux sur le mur est, un sur le mur ouest (l'autre tombe sur la porte).
    k.shell(kit.room_shell(cid, x0, x1, y0, y1, 0.0, h, st.openings_for(cid, CELLS, WALL_PORTALS, WINDOWS), mb, k.coll,
                           style="station", rib_every=3.8, avoid=lamps))
    k.light("l_client0", *lamps[0], h, 9000.0, 8.0, WARM, lens="light_amber", drop=0.9)
    k.light("l_client1", *lamps[1], h, 9000.0, 8.0, WARM, lens="light_amber", drop=0.9)
    k.light("l_staff", *lamps[2], h, 4000.0, 6.0, COOL, facing=math.pi / 2)
    k.light("l_vault", *lamps[3], h, 9000.0, 8.0, COOL_W, drop=0.4)
    k.light("l_corner", *lamps[4], h, 2500.0, 6.0, WARM, lens="light_amber")

    # --- Ligne des guichets : client à l'ouest, personnel à l'est. --------------------------------
    xc = -6.6
    gy, gl = -4.6, 2.0  # guichet de la Banque : face à la porte du hall
    k.add(kit.counter(f"{cid}_guichet", (xc, gy, 0.0), FACE_E, m, k.coll, length=gl, role="bank", variant="a"))
    # Vitre au-dessus du plateau (on parle par l'interstice de 20 cm) : visuelle seulement.
    k.box("guichet_glass", (0.012, gl - 0.08, 0.85), (xc - 0.05, gy, 1.675), "glass")
    k.box("guichet_sill", (0.05, gl, 0.03), (xc - 0.05, gy, 1.235), "brass")
    for s in (-1, 1):
        k.box("guichet_post", (0.07, 0.07, 1.05), (xc - 0.05, gy + s * gl / 2, 1.575), "paint_black")
    # Guichet fermé : caisson bas, vitre, rideau de tôle baissé derrière.
    fa, fb = gy + gl / 2 + 0.05, -2.2
    fm, fl = (fa + fb) / 2, fb - fa
    k.box("closed_body", (0.6, fl, 1.03), (xc, fm, 0.515), "paint_black", bevel=0.01, structure=True)
    k.box("closed_top", (0.72, fl + 0.04, 0.04), (xc - 0.04, fm, 1.05), "dark", bevel=0.008)
    k.box("closed_edge", (0.03, fl + 0.06, 0.03), (xc - 0.4, fm, 1.05), "brass")
    k.box("closed_kick", (0.02, fl - 0.02, 0.1), (xc - 0.305, fm, 0.05), "rubber")
    k.box("closed_glass", (0.012, fl - 0.06, 0.95), (xc - 0.05, fm, 1.6), "glass")
    k.box("closed_shutter", (0.03, fl - 0.06, 0.95), (xc + 0.06, fm, 1.6), "crate_slate")
    k.box("closed_post", (0.07, 0.07, 1.07), (xc - 0.05, fb, 1.585), "paint_black")
    k.col("closed", (0.72, fl, 2.12), (xc, fm, 1.06))
    partition(k, "part_n", (xc, fb), (xc, y1))
    partition(k, "part_s", (xc, -6.8), (xc, gy - gl / 2))
    partition(k, "part_staff", (xc, -6.8), (x1, -6.8))
    # Fronton : caisson noir à chants de laiton, écrans de cours tournés vers les clients,
    # bandeau chaud dessous (cuit, il éclaire les plateaux).
    fy0, fy1 = -6.8, y1
    fc, flen = (fy0 + fy1) / 2, fy1 - fy0
    k.box("fascia", (0.5, flen, 0.8), (xc + 0.1, fc, 2.5), "paint_black", bevel=0.015, structure=True)
    for z in (2.12, 2.88):
        k.box("fascia_rim", (0.04, flen, 0.04), (xc - 0.16, fc, z), "brass")
    for i, tile in enumerate(("bars_green", "wave_orange", "bars_orange", "wave_cyan", "bars_cyan")):
        y = fy1 - 0.55 - i * (flen - 1.1) / 4
        k.screen("fascia_scr", (0.8, 0.46), (xc - 0.156, y, 2.5), (0, -1, 0), (0, 0, 1), tile)
    k.ori("fascia_glow", (flen - 0.3, 0.06, 0.02), (xc - 0.02, fc, 2.085), (0, 1, 0), (1, 0, 0), "band_warm", ("strip", 1.0))
    # Côté personnel : mur de coffres individuels, bureau, armoire.
    deposit_wall(k, "deposit", (x1, -5.55, 0.0), FACE_E, width=2.2, height=2.2)
    k.add(kit.wall_desk(k.nm("desk"), (x1, -2.75, 0.0), FACE_E, m, k.coll, width=1.4, items_detail=1))
    k.add(kit.cabinet(k.nm("cabinet"), (-5.35, y1, 0.0), FACE_N, m, k.coll, size=(0.9, 0.32, 1.6)))

    # --- Le coffre : porte massive entre deux colonnes, barrière de potelets. --------------------
    vault_door(k, "vault", (-8.2, y0, 0.0), FACE_S)
    # Colonnes symétriques par rapport au coffre, hors de la vue de la porte vers le coffre.
    for i, x in enumerate((-11.2, -5.2)):
        column_round(k, f"column{i}", x, -7.2, h)
    stanchions(k, "barrier", [(-10.1, y0 + 0.3), (-10.1, -9.7), (-8.2, -9.7), (-6.3, -9.7), (-6.3, y0 + 0.3)])
    k.box("vault_line", (x1 - x0 - 0.1, 0.05, 0.006), ((x0 + x1) / 2, -7.2, 0.003), "brass")
    # Lambris noir à lisse de laiton (1,36 m) le long des murs de la salle des clients et du
    # coffre ; il couvre la bande de danger du style « station ».
    lam = [("W", y0, -8.1), ("W", -7.5, -5.65), ("W", -3.55, y1), ("N", x0, -8.5), ("N", -7.9, xc - 0.08),
           ("S", x0, -9.75), ("S", -6.65, x1), ("E", y0, -8.1), ("E", -7.5, -6.88)]
    for side, a, b in lam:
        ln, mid = b - a, (a + b) / 2
        if side in "NS":
            sgn = -1 if side == "N" else 1
            yw = y1 if side == "N" else y0
            k.box("wainscot", (ln, 0.04, 1.36), (mid, yw + sgn * 0.08, 0.68), "paint_black")
            k.box("wainscot_cap", (ln, 0.07, 0.04), (mid, yw + sgn * 0.075, 1.38), "brass")
        else:
            sgn = 1 if side == "W" else -1
            xw = x0 if side == "W" else x1
            k.box("wainscot", (0.04, ln, 1.36), (xw + sgn * 0.08, mid, 0.68), "paint_black")
            k.box("wainscot_cap", (0.07, ln, 0.04), (xw + sgn * 0.075, mid, 1.38), "brass")

    # --- Attente : banquettes, écrans de cours, plantes, tapis jusqu'au guichet. -----------------
    bench_block(k, "bench_n", (-10.25, y1 - 0.55, 0.0), math.pi, seats=3, rows=1)
    planter(k, "planter_nw", (-11.6, y1 - 0.4, 0.0), size=(0.5, 0.5, 0.5), height=0.95, blades=11, seed=1)
    planter(k, "planter_n", (-8.9, y1 - 0.4, 0.0), size=(0.5, 0.5, 0.5), height=0.95, blades=11, seed=2)
    k.box("rates_case", (2.7, 0.06, 0.78), (-10.25, y1 - 0.03, 2.35), "paint_black", bevel=0.01)
    for z in (1.95, 2.75):
        k.box("rates_rim", (2.74, 0.07, 0.03), (-10.25, y1 - 0.035, z), "brass")
    for i, tile in enumerate(("wave_green", "bars_orange", "wave_cyan")):
        k.screen("rates_scr", (0.78, 0.52), (-11.1 + i * 0.85, y1 - 0.066, 2.35), *BACK, tile)
    k.add(kit.light_band(k.nm("band"), (-11.5, y1 - 0.02, 3.0), (-9.0, y1 - 0.02, 3.0), m, k.coll, normal=(0, -0.6, -0.8), tone="warm"))
    bench_block(k, "bench_w", (x0 + 0.53, -9.8, 0.0), FACE_W, seats=2, rows=1)
    planter(k, "planter_sw", (x0 + 0.45, y0 + 0.45, 0.0), size=(0.6, 0.6, 0.5), height=1.15, blades=13, seed=4)
    planter(k, "planter_se", (x1 - 0.5, y0 + 0.5, 0.0), size=(0.7, 0.7, 0.55), height=1.3, blades=15, seed=9)
    k.box("rates_tall_case", (0.06, 1.0, 1.45), (x0 + 0.03, -6.65, 2.3), "paint_black", bevel=0.01)
    k.screen("rates_tall", (0.86, 1.3), (x0 + 0.066, -6.65, 2.3), (0, 1, 0), (0, 0, 1), "bars_green")
    k.add(kit.wall_screen(k.nm("screen"), (x1, -9.8, 1.8), FACE_E, m, k.coll, size=(1.0, 0.6)))
    # Tapis de dalles claires bordé de laiton, de la porte au guichet.
    rx0, rx1 = x0 + 0.05, xc - 0.42
    k.box("runner", (rx1 - rx0, 1.3, 0.006), ((rx0 + rx1) / 2, gy, 0.003), "tiles")
    for s in (-1, 1):
        k.box("runner_edge", (rx1 - rx0, 0.04, 0.008), ((rx0 + rx1) / 2, gy + s * 0.67, 0.004), "brass")
    door_sign(k, "sign_hall", x0, gy, 2.95, FACE_W, accent="paint_yellow", width=1.4)
    k.add(kit.vent(k.nm("vent"), (-10.6, y0, 3.2), FACE_S, m, k.coll, size=(0.8, 0.3)))
    with k.detail(1):
        k.add(kit.wall_screen(k.nm("screen"), (x0, -2.9, 2.1), FACE_W, m, k.coll, size=(0.7, 0.45)))
        k.box("closed_papers", (0.22, 0.3, 0.02), (xc - 0.15, fm, 1.08), "fabric_light", bevel=0.004)
        k.add(kit.light_band(k.nm("band"), (-9.6, y0 + 0.02, 3.3), (-6.8, y0 + 0.02, 3.3), m, k.coll, normal=(0, 0.6, -0.8), tone="warm"))
    with k.detail(2):
        k.add(kit.junction_box(k.nm("jbox"), (-5.0, -6.8, 1.4), FACE_S, m, k.coll, size=(0.4, 0.14, 0.5)))
    return k


BUILDERS = {
    "hangar": build_hangar,
    "couloir": build_couloir,
    "chantier": build_chantier,
    "hall": build_hall,
    "bureau": build_bureau,
    "taverne": build_taverne,
    "marche": build_marche,
    "banque": build_banque,
}


# --- Le vaisseau tenu par un volume gris (bake et aperçus, jamais exporté) ------------------------


def _prism_curve(name, pts, z0, z1, mat, coll):
    """Prisme vertical (courbe 2D fermée, remplie, extrudée) de z0 à z1 : un objet courbe, pas un
    maillage, que `lightmap.bake_cells` laisse donc dans la scène (il fait de l'ombre) et que
    l'export ne voit pas."""
    cu = bpy.data.curves.new(name, type="CURVE")
    cu.dimensions = "2D"
    cu.fill_mode = "BOTH"
    cu.extrude = (z1 - z0) / 2
    sp = cu.splines.new("POLY")
    sp.points.add(len(pts) - 1)
    for p, (x, y) in zip(sp.points, pts):
        p.co = (x, y, 0.0, 1.0)
    sp.use_cyclic_u = True
    cu.materials.append(mat)
    obj = bpy.data.objects.new(name, cu)
    obj.location = (0.0, 0.0, (z0 + z1) / 2)
    coll.objects.link(obj)
    return obj


def ship_proxy() -> list:
    """Coque grise à la place du vaisseau (boîte englobante + marges HULL_*, coins coupés), avec
    l'ouverture du sas au bout de la passerelle."""
    L = LAYOUT
    hx0, hx1, hy0, hy1 = L["hull"]
    zb, zt = L["hull_z"]
    hx, hy = L["hatch"]
    mat = c.material("preview_hull", (0.30, 0.32, 0.35), metallic=0.3, roughness=0.5)
    coll = c.collection("SHIP_PROXY")
    cb, cs = 2.4, 1.0  # coins coupés (proue au nord, poupe au sud)
    w = 0.9  # demi-largeur de l'ouverture du sas
    pts = [(hx0 + cs, hy0), (hx1 - cs, hy0), (hx1, hy0 + cs), (hx1, hy1 - cb), (hx1 - cb, hy1), (hx0 + cb, hy1), (hx0, hy1 - cb),
           (hx0, hy + w), (hx, hy + w), (hx, hy - w), (hx0, hy - w), (hx0, hy0 + cs)]
    notch = [(hx0, hy - w), (hx, hy - w), (hx, hy + w), (hx0, hy + w)]
    return [
        _prism_curve("ship_proxy_hull", pts, zb, zt, mat, coll),
        _prism_curve("ship_proxy_lintel", notch, kit.DOOR_H + 0.25, zt, mat, coll),
        _prism_curve("ship_proxy_sill", notch, zb, -0.3, mat, coll),
    ]


def npc_proxies(marks) -> list:
    """Silhouette grise (≈ 1,8 m) sur chaque `SOCKET_*_npc` : l'échelle du PNJ derrière son
    comptoir (vitrine du lot A). Aperçus seulement."""
    bpy.context.view_layer.update()
    mat = c.material("preview_npc", (0.42, 0.43, 0.46), roughness=0.7)
    coll = c.collection("PREVIEW")
    out = []
    for sock in marks:
        if not (sock.name.startswith("SOCKET_") and sock.name.endswith("_npc")):
            continue
        p = sock.matrix_world.translation
        tag = sock.name[len("SOCKET_"):]
        out += [
            c.cylinder(f"{tag}_legs", 0.13, 0.85, (p.x, p.y, 0.425), mat, coll, segments=12),
            c.cylinder(f"{tag}_body", 0.19, 0.65, (p.x, p.y, 1.17), mat, coll, segments=12),
            c.cylinder(f"{tag}_head", 0.1, 0.24, (p.x, p.y, 1.66), mat, coll, segments=12),
        ]
        for sx in (-1, 1):
            q = sock.matrix_world @ Vector((sx * 0.17, 0.26, 1.065))
            hand = c.box(f"{tag}_hand{sx}", (0.08, 0.1, 0.03), tuple(q), mat, coll)
            hand.rotation_euler[2] = sock.matrix_world.to_euler()[2]
            out.append(hand)
    return out


# --- Contrôles ---------------------------------------------------------------------------------


def fix_mirrored(objects) -> int:
    """Applique la transformation des maillages en miroir (déterminant < 0) et retourne leurs
    faces. `ship_interior._strut` (barres : fermes, lisses, bras, contreventements) pose une base
    (t, t × Z, …) de déterminant −1 ; Blender les affiche bien, mais `object.join` (dans
    `common.join_by_material`) ne retourne pas leurs faces : fusionnées, exportées et cuites,
    leurs normales regardent vers l'intérieur (lightmap noire, faces arrière en jeu). Correctif
    local en attendant celui du kit ; renvoie le nombre d'objets corrigés."""
    import bmesh

    bpy.context.view_layer.update()
    fixed = 0
    for obj in objects:
        if obj.type != "MESH" or obj.parent is not None:
            continue
        mw = obj.matrix_world.copy()
        if mw.determinant() >= 0.0:
            continue
        mesh = obj.data
        mesh.transform(mw)
        bm = bmesh.new()
        bm.from_mesh(mesh)
        bmesh.ops.reverse_faces(bm, faces=list(bm.faces))
        bm.to_mesh(mesh)
        bm.free()
        obj.matrix_world = Matrix.Identity(4)
        fixed += 1
    bpy.context.view_layer.update()
    return fixed



def check_saddles(cells: dict) -> float:
    """Le dessus des patins de caoutchouc des sabots est exactement sous la coque (M10) : renvoie
    sa hauteur, erreur si elle s'écarte de plus de 2 mm du dessous de la coque."""
    bpy.context.view_layer.update()
    pads = [o for o in cells["hangar"].visual if o.type == "MESH" and "saddle_pad" in o.name]
    if not pads:
        raise RuntimeError("berceau : aucun patin de sabot (`saddle_pad`) trouvé")
    top = max((o.matrix_world @ Vector(b)).z for o in pads for b in o.bound_box)
    want = LAYOUT["hull_z"][0]
    if abs(top - want) > 0.002:
        raise RuntimeError(f"berceau : dessus des patins à {top:.3f} m, coque à {want:.3f} m (SADDLE_PAD à revoir)")
    return top


def check_marks(cells: dict) -> None:
    """Tout siège, console, casier ou PNJ a son volume et ses sockets (`kit._check_interactions`) ;
    noms de repères uniques dans le secteur."""
    names = []
    for cid, k in cells.items():
        kit._check_interactions(k.marks, cid)
        names += [mk.name for mk in k.marks]
    dup = sorted({n for n in names if names.count(n) > 1})
    if dup or any(re.search(r"\.\d{3}$", n) for n in names):
        raise RuntimeError(f"repères en double : {dup or [n for n in names if re.search(r'.[0-9]{3}$', n)]}")


def patch_dock_normal(ron_rel: str) -> None:
    """`portal_record` tourne la normale d'un point d'attache « @… » vers l'extérieur de la
    cellule ; celle du hangar doit regarder le vaisseau (`sh_data::dock` retourne le sas contre
    elle)."""
    path = c.ROOT / ron_rel
    text = path.read_text(encoding="utf-8")
    normal = st.fmt(c.to_bevy(LAYOUT["dock_normal"]))
    pat = re.compile(r'(id: "hangar__ship",[^()]*center: \([^)]*\),\s*normal: )\([^)]*\)', re.S)
    new, n = pat.subn(lambda mt: mt.group(1) + normal, text, count=1)
    if n != 1:
        raise RuntimeError("portail hangar__ship introuvable dans le descripteur")
    path.write_text(new, encoding="utf-8", newline="\n")


def report(cells: dict, result: dict) -> None:
    """Lampes par cellule et par niveau, constantes du vaisseau."""
    for cid, k in cells.items():
        lamps = {}
        for mk in k.marks:
            if mk.name.startswith("LIGHT_"):
                lamps[c.detail_of(mk)] = lamps.get(c.detail_of(mk), 0) + 1
        s = result["stats"][cid]
        print(f"bilan {cid} : {s['tris']} triangles ({s['tris_by_detail']}), LOD {s['lod_tris']}, "
              f"{len(s['materials'])} matériaux, {len(s['merged'])} maillages, lampes {lamps}, lightmap {s['res']} px "
              f"({s['density']:.1f} texels/m)")
    print(f"îlots de lightmap épaissis (≥ {MIN_ISLAND_TEXELS} texels), par cellule : {dict(zip(cells, _THICKENED))}")
    L = LAYOUT
    print(f"vaisseau : quarts de tour {L['turns']}, origine {tuple(round(v, 3) for v in L['origin'])}, sas {L['hatch']}, "
          f"coque {tuple(round(v, 2) for v in L['hull'])} z {L['hull_z']}, fosse {tuple(round(v, 2) for v in L['pit'])}, "
          f"sabots {L['saddle']:.2f} m, hangar {CELLS['hangar']}")


# --- Aperçus ------------------------------------------------------------------------------------

GAME_LENS = 13.2  # champ vertical de 75° du jeu (`player.rs`), en 16:9 sur un capteur de 36 mm
EYE = 1.7


def _look(eye, yaw_deg, pitch_deg=0.0):
    import preview as pv

    return pv.look_from(eye, yaw_deg, pitch_deg)


def shots() -> dict:
    """Prises de vue : nom → {eye, target, lens, level (décor max affiché), hull (coque grise),
    plan (vue de dessus en coupe)}."""
    L = LAYOUT
    px0, px1, py0, py1 = L["pit"]
    hx, hy = L["hatch"]
    x0, x1, y0, y1, h = CELLS["hangar"]
    quai = (px0 - 2.2, hy, EYE)
    saddle_y = py0 + (py1 - py0) * 0.2  # premier sabot (`docking_cradle`, 4 sabots)

    def s(eye, target, lens=16.0, level=2, hull=True, plan=False, ortho=False, ambient=0.0, isolate=None):
        return {"eye": eye, "target": target, "lens": lens, "level": level, "hull": hull, "plan": plan, "ortho": ortho,
                "ambient": ambient, "isolate": isolate}

    over = ((x0 + 3.6, y0 + 2.2, 8.4), ((px0 + px1) / 2 + 1.0, py1 - 14.0, 0.0))
    cx_all, cy_all = (-40.0 + x1) / 2, (y0 + y1) / 2
    ortho = max(x1 + 44.0, (y1 - y0 + 4.0) * 16.0 / 9.0)
    return {
        # Caméra un peu au sud de la verticale : le nord en haut de l'image.
        "plan": s((cx_all, cy_all - 0.01, 120.0), (cx_all, cy_all, 0.0), ortho, plan=True),
        "hangar_quai": s(quai, (px0 + 6.0, hy, 2.6), GAME_LENS),
        "hangar_quai_biais": s((px0 - 3.0, hy - 2.0, EYE), (px0 - 1.0, hy + 14.0, 3.0), GAME_LENS),
        "hangar_quai_nord": s(quai, (px0 - 3.0, y1, 3.4), GAME_LENS),
        "hangar_quai_sud": s(quai, (px0 - 2.0, y0, 3.4), GAME_LENS),
        "hangar_entree": s((x0 + 1.6, -0.4, EYE), (px0 + 2.0, hy + 7.0, 3.2), GAME_LENS),
        "hangar_ensemble": s(*over, 12.0),
        "hangar_ensemble_bas": s(*over, 12.0, level=0),
        "hangar_ensemble_moyen": s(*over, 12.0, level=1),
        "hangar_nord_bas": s((x0 + 2.0, y1 - 2.0, 6.5), (px0 + 8.0, py0 + 6.0, 0.0), 13.0, level=0),
        "hangar_nord_haut": s((x0 + 2.0, y1 - 2.0, 6.5), (px0 + 8.0, py0 + 6.0, 0.0), 13.0, level=2),
        "hangar_baie": s((px0 - 1.0, py1 - 6.0, EYE), ((px0 + px1) / 2, y1, 4.5), GAME_LENS),
        "hangar_portique": s((px0 - 2.0, py0 + 4.0, EYE), (px1 - 7.0, y0 + 2.0, 3.5), GAME_LENS),
        "hangar_est": s((px1 + 2.5, py1 - 2.0, EYE), (px1 + 1.0, y0 + 10.0, 2.5), GAME_LENS),
        "passerelle": s((px0 - 3.2, hy - 3.6, 1.9), (px0 + 0.8, hy, 0.3), 20.0),
        # Depuis la porte du sas (dans l'ouverture de la coque), vers le quai et le couloir.
        "passerelle_depuis_sas": s((hx - 0.35, hy, EYE), (x0, hy - 0.6, 1.4), GAME_LENS),
        "berceau": s((px0 - 1.0, hy + 6.0, 3.2), (px0 + 6.0, hy + 14.0, -2.4), 16.0, hull=False),
        "couloir": s((-11.2, 0.0, EYE), (-4.2, 0.0, 1.3), GAME_LENS),
        "couloir_retour": s((-5.0, 0.3, EYE), (-12.4, 0.0, 1.3), GAME_LENS),
        "chantier": s((-4.9, 7.1, EYE), (-10.5, 3.4, 1.1), 14.0),
        "chantier_comptoir": s((-8.0, 4.6, EYE), (-10.5, 4.6, 1.2), 18.0),
        "chantier_fenetre": s((-9.2, 6.6, EYE), (-4.4, 2.6, 1.6), 14.0),
        # Hall (M10) : depuis l'entrée (comme la capture 13 de l'auteur), depuis l'angle sud-est,
        # depuis l'angle sud-ouest vers l'entrée, vers le couloir.
        "hall_entree": s((-12.9, 0.0, EYE), _look((-12.9, 0.0, EYE), 90.0, -4.0), GAME_LENS),
        "hall": s((-13.0, -6.5, EYE), _look((-13.0, -6.5, EYE), 45.0), GAME_LENS),
        "hall_ouest": s((-24.4, -5.2, EYE), (-13.0, 2.5, 1.8), GAME_LENS),
        "hall_vers_couloir": s((-24.5, 1.0, EYE), (-12.4, -1.5, 1.6), 16.0),
        "hall_banquettes": s((-20.0, -1.0, EYE), (-16.6, 4.6, 0.8), 18.0),
        "bureau": s((-20.0, 7.9, EYE), _look((-20.0, 7.9, EYE), 0.0, -5.0), GAME_LENS),
        "bureau_contrats": s((-21.0, 9.9, EYE), (-21.6, 12.8, 1.2), 18.0),
        "bureau_est": s((-19.6, 10.2, EYE), (-16.0, 11.0, 1.4), 18.0),
        # Taverne depuis la porte du hall : le passage jusqu'au tavernier.
        "taverne": s((-20.0, -7.9, EYE), (-20.6, -13.0, 1.1), 15.0),
        # Banque (M10) : depuis la porte, vers le coffre, le guichet de près, vue d'ensemble.
        "banque": s((-11.7, -4.6, EYE), _look((-11.7, -4.6, EYE), -90.0, -4.0), GAME_LENS),
        "banque_coffre": s((-10.9, -2.4, EYE), (-8.2, -11.6, 1.3), GAME_LENS),
        "banque_guichet": s((-8.6, -4.0, EYE), (-6.2, -4.7, 1.3), 18.0),
        "banque_ensemble": s((-11.7, -2.0, 3.6), (-6.4, -8.6, 0.6), 12.0),
        # Berceau en coupe : un sabot vu de profil sous la coque grise (dessus du patin = −1,70).
        # Vue orthographique de profil (vers +Y), 2,4 m de large : sabots, patins, fond de fosse et
        # coque grise seuls, à la lumière d'ambiance (`--quick` : noms des objets non fusionnés).
        "berceau_coupe": s((px0 + 1.4, saddle_y - 3.0, -1.7), (px0 + 1.4, saddle_y, -1.7), 2.4, ortho=True, ambient=1.0,
                           isolate=("cradle_saddle", "cradle_floor")),
        "berceau_patin": s((px0 + 0.3, saddle_y - 1.5, -1.4), (px0 + 1.4, saddle_y, -1.74), 30.0, ambient=1.0,
                           isolate=("cradle_saddle", "cradle_floor")),
        # Plan des salles (vue de dessus en coupe, plus serrée que « plan »).
        "plan_salles": s((-21.5, -0.01, 120.0), (-21.5, 0.0, 0.0), 48.0, plan=True),
        "taverne_bar": s((-20.6, -9.6, EYE), (-20.6, -12.8, 1.2), 18.0),
        "marche": s((-26.9, 0.4, EYE), (-38.0, 0.0, 1.5), 15.0),
        "marche_comptoir": s((-31.0, -0.4, EYE), (-31.5, -3.8, 1.15), 18.0),
    }


def _hide_lenses() -> None:
    """Les lentilles des plafonniers et projecteurs (`sh_light*`) n'éclairent pas : leurs lampes
    le font (comme au bake)."""
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH" and obj.data.materials and obj.data.materials[0] and obj.data.materials[0].name.startswith("sh_light"):
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = False


def _apply_shot(spec, merged, lamps, proxy):
    """Visibilités d'une prise : décor au-delà du niveau masqué (et ses lampes), coque grise."""
    for obj in merged:
        obj.hide_render = c.detail_of(obj) > spec["level"]
    for lamp, level in lamps:
        lamp.hide_render = level > spec["level"]
    for obj in proxy:
        obj.hide_render = not spec["hull"]


def render_shots(shots_: dict, prefix: str, merged, lamps, proxy, only, plan_objs=None):
    """`plan_objs` : objets d'avant la fusion (le plan masque tout ce qui commence au-dessus de
    2,6 m : un maillage fusionné mêle le sol et le plafond)."""
    import preview as pv

    bpy.context.view_layer.update()
    low = []
    for obj in plan_objs or []:
        if obj.type == "MESH" and min((obj.matrix_world @ Vector(b)).z for b in obj.bound_box) < 2.6:
            low.append(obj)
    for name, spec in shots_.items():
        if only and name not in only:
            continue
        if spec["plan"] and plan_objs is None:
            continue
        _apply_shot(spec, merged, lamps, proxy)
        if spec.get("isolate"):
            if not any(t in o.name for o in merged for t in spec["isolate"]):
                continue  # objets fusionnés : la prise ne se fait qu'en `--quick`
            for obj in merged:
                obj.hide_render = obj.hide_render or not any(t in obj.name for t in spec["isolate"])
        if spec["plan"]:
            # Plan : vue de dessus orthographique en coupe, coque grise comprise.
            for obj in merged:
                obj.hide_render = True
            for obj in low:
                obj.hide_render = False
            pv.world((0.7, 0.72, 0.75), 0.45)
            pv.camera("cam", spec["eye"], spec["target"], ortho=spec["lens"])
        else:
            amb = spec.get("ambient", 0.0)
            pv.world((0.7, 0.72, 0.75) if amb else (0, 0, 0), amb)
            if spec.get("ortho"):
                pv.camera("cam", spec["eye"], spec["target"], ortho=spec["lens"])
            else:
                pv.camera("cam", spec["eye"], spec["target"], lens=spec["lens"])
                bpy.data.cameras["cam"].type = "PERSP"
        pv.render(PREVIEWS / f"{prefix}{name}.png")
        print(f"aperçu : {prefix}{name}.png")
        for obj in low:
            obj.hide_render = True


def preview_lit(shots_, merged, marks, proxy, samples, res, only, plan_objs=None) -> None:
    import preview as pv

    exposure = float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2"))
    pv.setup_cycles(samples, res[0], res[1], exposure=exposure)
    lamps = []
    for lamp in lm.spawn_lamps(marks, c.collection("PREVIEW_LAMPS")):
        lamp.data.shadow_soft_size = 0.06
        src = bpy.data.objects.get(lamp.name[len("bake_"):])
        lamps.append((lamp, c.detail_of(src) if src else 0))
    _hide_lenses()
    render_shots(shots_, "lit_", merged, lamps, proxy, only, plan_objs)
    for lamp, _ in lamps:
        lamp.hide_render = True


def preview_game(shots_, stats, lightmaps, merged, extra, proxy, samples, res, only, ambient=0.35) -> None:
    """Comme le jeu en Bas et Moyen dans une cellule à lightmap : couleur × (1 − métal) × lightmap
    × exposition + émissif (`sector_tools.game_material`), sans lampe ni rebond ; portes, PNJ et
    coque grise (sans lightmap) : couleur × `ambient`."""
    import preview as pv

    exposure = float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2"))
    pv.setup_cycles(samples, res[0], res[1], exposure=exposure)
    bpy.context.scene.cycles.max_bounces = 0
    for obj in bpy.context.scene.objects:
        if obj.type == "LIGHT":
            obj.hide_render = True
    for cid, s in stats.items():
        info = lightmaps.get(cid)
        image, scale = None, ambient
        if info is not None:
            image = bpy.data.images.load(str(c.ROOT / "assets" / info["image"]), check_existing=True)
            image.colorspace_settings.name = "sRGB"
            scale = info["exposure"] / lm.WATTS_TO_LUMENS
        for obj in s["merged"]:
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = True
            mats = obj.data.materials
            for i, mat in enumerate(mats):
                mats[i] = st.game_material(mat, image, scale, cid, exposure)
    cache = {}
    for obj in list(extra) + list(proxy):
        # La coque d'aperçu, plus sombre : un repère d'encombrement, pas un rendu de la coque.
        amb = 0.12 if obj in proxy else ambient
        mats = obj.data.materials
        for i, mat in enumerate(mats):
            key = (mat.name, amb)
            if key not in cache:
                cache[key] = st.game_material(mat, None, amb, "extra", exposure)
            mats[i] = cache[key]
    render_shots(shots_, "jeu_", merged, [], proxy, only)


def _arg(argv, name, default):
    return argv[argv.index(name) + 1] if name in argv else default


def main(argv) -> None:
    bake = "--no-bake" not in argv
    samples = int(_arg(argv, "--samples", "32"))
    rw, rh = (int(v) for v in _arg(argv, "--res", "1280x720").split("x"))
    only = _arg(argv, "--only", None)
    only = only.split(",") if only else None
    quick = "--quick" in argv
    keep_cells = _arg(argv, "--cells", None)
    keep_cells = keep_cells.split(",") if keep_cells else None
    c.reset_scene()
    m = palette()
    proxy = ship_proxy() if BAKE_SHIP_OCCLUDER or "--lit" in argv or "--previews" in argv else []
    if quick and keep_cells:
        cells = {cid: build(m) for cid, build in BUILDERS.items() if cid in keep_cells}
    else:
        cells = {cid: build(m) for cid, build in BUILDERS.items()}
    bpy.context.view_layer.update()
    mirrored = fix_mirrored([o for k in cells.values() for o in k.visual])
    print(f"maillages en miroir corrigés (barres du kit) : {mirrored}")
    if "hangar" in cells:
        print(f"berceau : dessus des patins à {check_saddles(cells):.3f} m (dessous de la coque {LAYOUT['hull_z'][0]:.3f} m)")
    check_marks(cells)
    if quick:
        # Itération (`--quick [--cells a,b]`) : ni export ni bake, aperçus éclairés (Cycles, lampes)
        # des objets bruts ; rien n'est écrit dans assets/.
        visual = [o for k in cells.values() for o in k.visual]
        for k in cells.values():
            print(f"cellule {k.cid} : {kit._tris(k.visual)} triangles (avant fusion), {len(k.colliders)} colliders, "
                  f"{sum(1 for mk in k.marks if mk.name.startswith('INTERACT_'))} interactions")
        doors = st.door_panels([p for p in WALL_PORTALS if p[0] in cells or p[1] in cells], CELLS, m, c.collection("PREVIEW_DOORS"), style="station")
        marks = [mk for k in cells.values() for mk in k.marks]
        npc_proxies(marks)
        bpy.context.view_layer.update()
        preview_lit(shots(), visual, marks, proxy, samples, (rw, rh), only, plan_objs=visual + doors)
        return
    built = {cid: k.parts() for cid, k in cells.items()}
    L = LAYOUT
    meta = {
        "script": "tools/blender/places/spaceport.py",
        "comment": ("Le spatioport par défaut, hangar et salles (proposition P7 ; M9) : cellules, portails, apparitions. "
                    f"Vaisseau : boîte {SHIP_BBOX}, sas {SHIP_HATCH} normale {SHIP_HATCH_NORMAL} (intérieur type corvette)."),
        "id": SECTOR,
        "kind": "District",
        "name_key": f"sector.{SECTOR}",
        "status": 'Proposition("P7")',
        "door_model": "models/kit/ship_interior/door_station.gltf",
        "models": f"models/places/{SECTOR}",
        "lightmaps": f"textures/lightmaps/{SECTOR}",
        "rooms": ROOMS,
    }
    if not BAKE_SHIP_OCCLUDER:
        for obj in proxy:
            obj.hide_render = True
    ron = f"assets/data/propositions/sectors/{SECTOR}.ron"
    result = st.export_sector(meta, CELLS, PORTALS, SPAWNS, built, ron, f"art/places/{SECTOR}.blend", bake=bake)
    patch_dock_normal(ron)
    report(cells, result)
    print(f"{SECTOR} : {len(CELLS)} cellules, {len(PORTALS)} portails exportés ; portail du vaisseau : centre "
          f"{c.to_bevy((L['hatch'][0], L['hatch'][1], kit.DOOR_H / 2))}, normale {c.to_bevy(L['dock_normal'])}")
    if "--lit" not in argv and "--previews" not in argv:
        return
    merged = [o for s in result["stats"].values() for o in s["merged"]]
    keep = set(merged)
    for obj in bpy.data.objects:
        if obj.type == "MESH":
            obj.hide_render = obj not in keep
    doors = st.door_panels(WALL_PORTALS, CELLS, m, c.collection("PREVIEW_DOORS"), style="station")
    doors += st.door_panels([DOCK_PORTAL], CELLS, m, c.collection("PREVIEW_DOORS"), style="ship")
    marks = [mk for k in cells.values() for mk in k.marks]
    npcs = npc_proxies(marks)
    extra = doors + npcs
    for o in extra:
        o.hide_render = False
    bpy.context.view_layer.update()
    shots_ = shots()
    if "--lit" in argv:
        originals = [o for p in built.values() for o in p["visual"] + p["colliders"] if o.type == "MESH" and not o.name.endswith("_col")]
        preview_lit(shots_, merged, marks, proxy, samples, (rw, rh), only, plan_objs=originals)
    if "--previews" in argv and result["lightmaps"]:
        preview_game(shots_, result["stats"], result["lightmaps"], merged, extra, proxy, max(8, samples // 2), (rw, rh), only)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
