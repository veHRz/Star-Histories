"""Kit modulaire d'intérieur (proposition P13), direction « futur usé » (M8, lot A).

Ce module sert de bibliothèque aux scripts de vaisseaux (`tools/blender/ships/`) et de lieux
(`tools/blender/places/`) et, lancé seul, exporte chaque pièce du kit dans
`assets/models/kit/ship_interior/<pièce>.gltf` (glTF séparé, textures de la bibliothèque citées
par chemin) avec sa source `art/kit/ship_interior.blend`, pour la revue et la réutilisation.

Direction (docs/refonte-visuelle.md §2, salle témoin validée) : coques à pans coupés, nervures
rivées tous les 2 m, UV décalées d'une travée à l'autre, bandeaux lumineux dans les angles
(`sh_band_*`, cuits dans les lightmaps), encadrements de portes épais, consoles denses. Trois
styles de coque : `ship` (tôle peinte claire, froide), `industrial` (sombre, ambre : soute,
machines), `station` (béton et carrelage d'Hexa-Corp, jaune et noir de chantier).

API (inchangée depuis M1, paramètres à mot-clé ajoutés en M8) :
- `palette()` : matériaux partagés (clés historiques + nouvelles) ;
- `room_shell(...)` → {"visual", "structure", "colliders"} ;
- props et habillage → (visibles, colliders, repères), origine au centre de la base, la face
  utile regarde vers +Y quand `facing` = 0 (radians autour de Z).
Toutes les pièces ont leurs UV de texture (1er jeu) : les clients n'ont plus qu'à déplier la
lightmap (`lightmap.unwrap`) et à fusionner par matériau. Un objet que le client crée lui-même
avec `common.box` doit recevoir ses UV (`apply_uvs`).

Gameplay intact : mêmes colliders, mêmes repères `INTERACT_` / `SOCKET_` / `LIGHT_` aux mêmes
positions relatives, mêmes ouvertures. Un détail qui avance de plus de 0,1 m dans la pièce sous
2,1 m a son collider (`_col`).

M9 (lot A, docs/m9-retours.md §3) :
- décor par niveau de qualité : les pièces d'habillage prennent `detail=` (0 par défaut : comme
  en M8 ; ≥ 1 : marquées `detail`, sans collider, `_finish`) ;
- volumes d'interaction : chaque `INTERACT_` de siège, console, casier ou comptoir est au centre
  de l'objet et porte `half_extents` (`_interact`) ; sièges toujours interactifs (`role` :
  "pilot", "copilot", "passenger" par défaut) ;
- portes sans écran ; nouvelles pièces : `bed`, `locker(container=)`, `locker_bank`, `counter`
  (comptoir de PNJ, `SOCKET_<nom>_npc`), `wall_desk`, `shelf`, `cabin_porthole` ; style de coque
  `hangar` et `floor_holes` ; pièces de hangar dans `kit/hangar.py` (aussi `kit.hangar_shell`…).

Grille modulaire : 2 m (P5). Repères : X = tribord, Y = avant, Z = haut (Blender).
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import common as c  # noqa: E402

GRID = 2.0
WALL = 0.2  # épaisseur des cloisons
DOOR_W = 1.4
DOOR_H = 2.2

STYLES = ("ship", "industrial", "station", "hangar")
# Hauteur sous laquelle un détail en saillie doit avoir un collider (le joueur y passe la tête).
HEAD_CLEAR = 2.1

# Décalages de texture (en tuiles) d'une travée à l'autre : la nervure cache le raccord et le
# motif (grilles, trappes) ne revient pas au même endroit.
_BAY_OFFSETS = [(0.0, 0.0), (0.5, 0.25), (0.25, 0.5), (0.75, 0.75), (0.375, 0.125), (0.125, 0.625), (0.625, 0.375), (0.875, 0.875)]

COOL = [0.74, 0.86, 1.0]
AMBER = [1.0, 0.55, 0.22]
WARM = [1.0, 0.82, 0.62]
RED = [1.0, 0.18, 0.12]


def palette() -> dict:
    """Matériaux partagés du kit (peu nombreux : un appel de rendu par matériau et par cellule).

    Clés historiques (M1) : wall, floor, ceiling, trim, accent, light, screen, fabric, crate,
    machine, hazard, glass. Nouvelles (M8) : panel, dark, bare, floor_plate, grating,
    grating_glow, trim_sheet (trim sheet `trim_ship`, à poser avec `common.uv_trim`), rubber, buttons,
    band_cool / band_amber / band_warm / band_reactor (bandeaux, cuits), light_amber, beacon,
    concrete, tiles, paint_yellow, paint_black, door_paint, crate_slate / crate_rust /
    crate_yellow. M9 : fabric_dark (couvertures), fabric_light (matelas, oreillers), glass_dark
    (verre fumé opaque), grating_plain (caillebotis sans lumière dessous : passerelles). Plusieurs
    clés peuvent désigner le même matériau.
    """
    lib = c.material_library()
    tints = lib.TINTS
    panel = c.textured_material("sh_panel", "panel_painted")
    dark = c.textured_material("sh_metal_dark", "metal_dark")
    bare = c.textured_material("sh_metal_bare", "metal_bare")
    floor = c.textured_material("sh_floor_plate", "floor_plate")
    yellow = c.textured_material("sh_paint_yellow", "panel_painted", tint=tints["hexa_yellow"])
    m = {
        "panel": panel,
        "dark": dark,
        "bare": bare,
        "floor_plate": floor,
        "grating": c.textured_material("sh_grating", "grating", emissive=(0.45, 0.68, 1.0), emissive_strength=0.3),
        "grating_glow": c.textured_material("sh_grating_glow", "grating", emissive=(0.45, 0.68, 1.0), emissive_strength=0.6),
        "trim_sheet": c.textured_material("sh_trim", "trim_ship", emissive=(0.85, 0.93, 1.0), emissive_strength=4.0),
        "rubber": c.textured_material("sh_rubber", "rubber"),
        "buttons": c.textured_material("sh_buttons", "buttons", emissive=(1.0, 1.0, 1.0), emissive_strength=3.0),
        "screen": c.textured_material("sh_screen", "screen", emissive=(1.0, 1.0, 1.0), emissive_strength=2.5),
        "fabric": c.textured_material("sh_fabric", "fabric", tint=(0.52, 0.42, 0.33)),
        "crate": c.textured_material("sh_crate_olive", "container_ribbed", tint=tints["olive"]),
        "crate_slate": c.textured_material("sh_crate_slate", "container_ribbed", tint=(0.55, 0.62, 0.70)),
        "crate_rust": c.textured_material("sh_crate_rust", "container_ribbed", tint=tints["pirate_rust"]),
        "crate_yellow": c.textured_material("sh_crate_yellow", "container_ribbed", tint=tints["hexa_yellow"]),
        "concrete": c.textured_material("sh_ferrocrete", "ferrocrete"),
        "tiles": c.textured_material("sh_floor_tiles", "floor_tiles", tint=tints["warm_gray"]),
        "paint_yellow": yellow,
        "paint_black": c.textured_material("sh_paint_black", "panel_painted", tint=tints["hexa_black"]),
        "door_paint": c.textured_material("sh_door_paint", "panel_painted", tint=(0.44, 0.56, 0.66)),
        "band_cool": c.textured_material("sh_band_cool", "light_band", emissive=tuple(COOL), emissive_strength=4.0),
        "band_amber": c.textured_material("sh_band_amber", "light_band", emissive=(1.0, 0.6, 0.26), emissive_strength=4.0),
        "band_warm": c.textured_material("sh_band_warm", "light_band", emissive=(1.0, 0.8, 0.55), emissive_strength=3.5),
        "band_reactor": c.textured_material("sh_band_reactor", "light_band", emissive=(0.35, 0.85, 1.0), emissive_strength=6.0),
        # Plafonniers : remplacés par leurs lampes au bake (lightmap.py masque `sh_light*`).
        "light": c.material("sh_light", (0.9, 0.95, 1.0), emission=(0.78, 0.9, 1.0), emission_strength=5.0),
        "light_amber": c.material("sh_light_amber", (1.0, 0.8, 0.6), emission=(1.0, 0.55, 0.2), emission_strength=4.0),
        "beacon": c.material("sh_beacon_red", (0.6, 0.05, 0.03), emission=(1.0, 0.12, 0.06), emission_strength=6.0),
        "glass": c.material("sh_glass", (0.55, 0.70, 0.80), roughness=0.05, alpha=0.12),
        # M9 : couverture des couchettes, verre fumé opaque (hublots de porte et de cabine : on
        # ne voit pas au travers, rien à charger derrière).
        "fabric_dark": c.textured_material("sh_fabric_dark", "fabric", tint=(0.30, 0.34, 0.41)),
        "grating_plain": c.textured_material("sh_grating_plain", "grating"),
        "fabric_light": c.textured_material("sh_fabric_light", "fabric", tint=(0.66, 0.66, 0.63)),
        "glass_dark": c.material("sh_glass_dark", (0.02, 0.026, 0.032), roughness=0.08),
    }
    # Clés historiques (M1), désormais texturées.
    m.update({
        "wall": panel,
        "floor": floor,
        "ceiling": panel,
        "trim": dark,
        "accent": yellow,
        "machine": bare,
        "hazard": yellow,
    })
    return m


# --- Briques géométriques ------------------------------------------------------------------------


def _uv(obj, uv="box", tile=None):
    """UV de texture d'un objet : "box" (projection boîte), ("box", (du, dv)) (décalée, en
    tuiles), ("strip", mètres) (bandeau : U le long, toute la hauteur en V), ("wrap", bande,
    mètres) (bande de trim autour d'un cylindre d'axe Z), un nom de bande de trim ou (v0, v1)
    (`common.uv_trim`), None : rien."""
    if uv is None:
        return obj
    if uv == "box":
        c.uv_box(obj, tile)
    elif isinstance(uv, tuple) and uv and uv[0] == "box":
        c.uv_box(obj, tile, offset=uv[1])
    elif isinstance(uv, tuple) and uv and uv[0] == "strip":
        c.uv_trim(obj, (0.0, 1.0), meters_per_tile=uv[1], pad_px=0.0)
    elif isinstance(uv, tuple) and uv and uv[0] == "wrap":
        _uv_wrap(obj, uv[1], uv[2])
    else:
        c.uv_trim(obj, uv, meters_per_tile=tile or 2.0)
    return obj


def apply_uvs(objects, offset=(0.0, 0.0)) -> None:
    """Projection boîte des objets (maillages) qui n'ont pas encore d'UV de texture : pour les
    pièces qu'un client crée lui-même avec `common.box` / `common.cylinder`."""
    for obj in objects:
        if obj.type == "MESH" and len(obj.data.uv_layers) == 0 and obj.data.materials:
            c.uv_box(obj, None, offset=offset)


def _uv_wrap(obj, trim, meters_per_tile: float = 2.0, pad_px: float = 2.0) -> None:
    """Bande de trim enroulée autour d'un cylindre d'axe Z local : U = longueur d'arc (nombre
    entier de répétitions sur le tour), V = la bande sur la hauteur. Bouchons : projection plane."""
    import numpy as np

    v0, v1 = c.trim_band(trim)
    v0, v1 = v0 + pad_px / 1024, v1 - pad_px / 1024
    mesh = obj.data
    co = np.array([v.co[:] for v in mesh.vertices])
    r = float(np.hypot(co[:, 0], co[:, 1]).max())
    zmin, zmax = co[:, 2].min(), co[:, 2].max()
    circ = 2 * math.pi * r
    reps = max(1, round(circ / meters_per_tile))
    mpt = circ / reps
    if len(mesh.uv_layers) == 0:
        mesh.uv_layers.new(name=c.UV_TEXTURE)
    layer = mesh.uv_layers[0]
    for poly in mesh.polygons:
        idx = list(poly.loop_indices)
        verts = [co[mesh.loops[i].vertex_index] for i in idx]
        if abs(poly.normal.z) < 0.5:
            ang = [math.atan2(p[1], p[0]) for p in verts]
            if max(ang) - min(ang) > math.pi:
                ang = [a + 2 * math.pi if a < 0 else a for a in ang]
            for i, p, a in zip(idx, verts, ang):
                layer.data[i].uv = (a * r / mpt, v0 + (p[2] - zmin) / max(zmax - zmin, 1e-9) * (v1 - v0))
        else:
            for i, p in zip(idx, verts):
                layer.data[i].uv = (p[0] / mpt, v0 + (p[1] + r) / (2 * r) * (v1 - v0))


def _box(name, size, center, mat, coll, uv="box", bevel=0.0, tile=None):
    return _uv(c.box(name, size, center, mat, coll, bevel=bevel), uv, tile)


def _cyl(name, radius, depth, center, mat, coll, axis="Z", segments=12, uv="box", tile=None):
    return _uv(c.cylinder(name, radius, depth, center, mat, coll, axis=axis, segments=segments), uv, tile)


def _ori(name, size, center, right, up, mat, coll, uv="box", bevel=0.0, tile=None):
    """Pavé dont les axes locaux sont (right, up, right × up) : la face +Z locale (la « face »)
    regarde vers right × up. Pour une face tournée vers +Y : right = (−1, 0, 0), up = (0, 0, 1)."""
    obj = c.box(name, size, (0, 0, 0), mat, coll, bevel=bevel)
    r, u = Vector(right).normalized(), Vector(up).normalized()
    n = r.cross(u).normalized()
    u = n.cross(r).normalized()
    obj.matrix_basis = Matrix.Translation(Vector(center)) @ Matrix((r, u, n)).transposed().to_4x4()
    return _uv(obj, uv, tile)


def _prism(name, section, axis, s0, s1, mat, coll, uv="box"):
    """Prisme : polygone `section` (points 3D pris à s = 0) extrudé le long de `axis` de s0 à s1."""
    axis = Vector(axis)
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    lo = [bm.verts.new(Vector(p) + axis * s0) for p in section]
    hi = [bm.verts.new(Vector(p) + axis * s1) for p in section]
    bm.faces.new(lo)
    bm.faces.new(list(reversed(hi)))
    n = len(section)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    return _uv(obj, uv)


def _rotated(dx: float, dy: float, facing: float) -> tuple[float, float]:
    cs, sn = math.cos(facing), math.sin(facing)
    return dx * cs - dy * sn, dx * sn + dy * cs


def _place(objs, center, facing):
    for obj in objs:
        ox, oy = _rotated(obj.location.x, obj.location.y, facing)
        obj.location = (center[0] + ox, center[1] + oy, center[2] + obj.location.z)
        obj.rotation_euler[2] += facing


def _tris(objs) -> int:
    return sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in objs if o.type == "MESH" and not o.name.endswith("_col"))


def _finish(vis, col, marks, detail: int = 0):
    """Applique le niveau de décor d'une pièce (M9, `common.set_detail`) : ses maillages (et ses
    `LIGHT_`) sont marqués, et un décor optionnel (niveau ≥ 1) n'a jamais de collider (ils sont
    supprimés de la scène). Niveau 0 : rien ne change (comportement de M8)."""
    if not detail:
        return vis, col, marks
    c.set_detail(vis + [mk for mk in marks if mk.name.startswith("LIGHT_")], detail)
    for obj in col:
        bpy.data.objects.remove(obj, do_unlink=True)
    return vis, [], marks


def _interact(name, center, size, props: dict, coll, margin: float = 0.04):
    """Repère `INTERACT_<name>` au centre du volume de l'objet (repère local de la pièce, avant
    `_place`), avec l'extra `half_extents` (M9) qui couvre `size` (tailles Blender) plus
    `margin` de chaque côté : le rayon de visée touche le capteur avant le collider."""
    props = dict(props)
    props["half_extents"] = c.half_extents(size, margin)
    return c.empty(f"INTERACT_{name}", center, coll, props)


def _strut(name, p0, p1, w, h, mat, coll, uv="box", tile=None, bevel=0.0):
    """Barre de section w × h de p0 à p1 (treillis, contreventements, bras) : la largeur w est
    horizontale quand c'est possible, la hauteur h dans le plan vertical de la barre."""
    p0, p1 = Vector(p0), Vector(p1)
    d = p1 - p0
    L = d.length
    t = d.normalized()
    side = t.cross(Vector((0, 0, 1)))
    if side.length < 1e-4:
        side = Vector((1, 0, 0))
    side.normalize()
    up = side.cross(t).normalized()
    obj = c.box(name, (L, w, h), (0, 0, 0), mat, coll, bevel=bevel)
    obj.matrix_basis = Matrix.Translation((p0 + p1) / 2) @ Matrix((t, side, up)).transposed().to_4x4()
    return _uv(obj, uv, tile)


def _ring(name, r_in, r_out, depth, center, mat, coll, axis="Y", segments=20):
    """Anneau plat (couronne extrudée) d'axe `axis` : cadres de hublot, colliers, enrouleurs."""
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    rings = []
    for z in (-depth / 2, depth / 2):
        for r in (r_out, r_in):
            rings.append([bm.verts.new((r * math.cos(2 * math.pi * i / segments), r * math.sin(2 * math.pi * i / segments), z)) for i in range(segments)])
    oz0, iz0, oz1, iz1 = rings
    for i in range(segments):
        j = (i + 1) % segments
        bm.faces.new((oz0[i], oz0[j], oz1[j], oz1[i]))  # flanc extérieur
        bm.faces.new((iz0[j], iz0[i], iz1[i], iz1[j]))  # flanc intérieur
        bm.faces.new((oz1[i], oz1[j], iz1[j], iz1[i]))  # face avant
        bm.faces.new((oz0[j], oz0[i], iz0[i], iz0[j]))  # face arrière
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    if axis == "Y":
        bmesh.ops.rotate(bm, verts=bm.verts, cent=(0, 0, 0), matrix=Matrix.Rotation(math.radians(90), 3, "X"))
    elif axis == "X":
        bmesh.ops.rotate(bm, verts=bm.verts, cent=(0, 0, 0), matrix=Matrix.Rotation(math.radians(90), 3, "Y"))
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    obj.location = center
    coll.objects.link(obj)
    return _uv(obj, "box")


# --- Repère d'une cloison -------------------------------------------------------------------------


class _Wall:
    """Repère d'une cloison : `s` le long du mur (coordonnée absolue X ou Y), `d` distance
    vers l'intérieur depuis la face intérieure, `z` hauteur depuis le sol de la pièce."""

    def __init__(self, side, x0, x1, y0, y1, z0, h):
        self.side = side
        if side == "N":
            self.o, self.t, self.n, self.lo, self.hi = Vector((0, y1, z0)), Vector((1, 0, 0)), Vector((0, -1, 0)), x0, x1
        elif side == "S":
            self.o, self.t, self.n, self.lo, self.hi = Vector((0, y0, z0)), Vector((1, 0, 0)), Vector((0, 1, 0)), x0, x1
        elif side == "E":
            self.o, self.t, self.n, self.lo, self.hi = Vector((x1, 0, z0)), Vector((0, 1, 0)), Vector((-1, 0, 0)), y0, y1
        else:
            self.o, self.t, self.n, self.lo, self.hi = Vector((x0, 0, z0)), Vector((0, 1, 0)), Vector((1, 0, 0)), y0, y1
        self.h = h
        self.span = (x1 - x0) if side in "EW" else (y1 - y0)  # profondeur de la pièce devant ce mur

    def p(self, s, d=0.0, z=0.0) -> Vector:
        return self.o + self.t * s + self.n * d + Vector((0, 0, z))

    def facing(self) -> float:
        """Angle (autour de Z) d'un prop dont la face +Y regarde vers l'intérieur depuis ce mur."""
        return math.atan2(-self.n.x, self.n.y)

    def right(self) -> Vector:
        """« Droite » d'un observateur qui regarde ce mur depuis la pièce (pour `_ori`)."""
        return (-self.n).cross(Vector((0, 0, 1)))


def _xs_slab(name, fr: _Wall, q1, q2, s0, s1, thick, mat, coll, uv="box", inset=0.0, extend=0.0, tile=None):
    """Plaque le long du mur dont la face visible va de q1 à q2 (points (d, z) de la section),
    de s0 à s1 ; `inset` > 0 : la plaque avance dans la pièce (nervure). Projection boîte locale :
    U le long du mur, V le long de la section."""
    p1, p2 = fr.p(0, *q1), fr.p(0, *q2)
    e = p2 - p1
    length = e.length
    e.normalize()
    nrm = fr.t.cross(e).normalized()
    mid = fr.p((s0 + s1) / 2, (q1[0] + q2[0]) / 2, (q1[1] + q2[1]) / 2)
    ref = fr.p((s0 + s1) / 2, max(1.0, fr.span / 2), fr.h / 2) - mid
    if nrm.dot(ref) < 0:
        nrm = -nrm
    xl = e.cross(nrm)
    obj = c.box(name, (abs(s1 - s0), length + 2 * extend, thick), (0, 0, 0), mat, coll)
    center = mid + nrm * (inset - thick / 2)
    obj.matrix_basis = Matrix.Translation(center) @ Matrix((xl, e, nrm)).transposed().to_4x4()
    return _uv(obj, uv, tile)


def _cuts(lo, hi, gaps):
    """Intervalles de [lo, hi] hors des `gaps` (a, b)."""
    out, cur = [], lo
    for a, b in sorted(gaps):
        if b <= cur or a >= hi:
            continue
        if a - cur > 0.05:
            out.append((cur, a))
        cur = max(cur, b)
    if hi - cur > 0.05:
        out.append((cur, hi))
    return out


# --- Coque de salle -----------------------------------------------------------------------------

_STYLE = {
    # wall, ceiling, floor, frame (encadrements, nervures), band (bandeau), kick (plinthe)
    "ship": {"wall": "panel", "ceiling": "panel", "floor": "floor_plate", "frame": "dark", "band": "band_cool", "rib": "dark"},
    "industrial": {"wall": "panel", "ceiling": "dark", "floor": "floor_plate", "frame": "dark", "band": "band_amber", "rib": "dark"},
    "station": {"wall": "concrete", "ceiling": "dark", "floor": "tiles", "frame": "paint_black", "band": "band_warm", "rib": "paint_black"},
    # M9 : grande salle haute (hangar) : sol de béton, soubassement sombre de 2,4 m à pied de
    # bandes de danger, colonnes en H tous les 6 m, bandeaux ambrés à 3 m.
    "hangar": {"wall": "panel", "ceiling": "dark", "floor": "concrete", "frame": "paint_black", "band": "band_amber", "rib": "dark"},
}


def _rect_minus(outer, holes):
    """Rectangles (x0, x1, y0, y1) qui couvrent `outer` privé des `holes` (même format), par
    découpage en grille puis fusion des cases voisines : peu de pavés (sol percé d'une fosse)."""
    ax0, ax1, ay0, ay1 = outer
    xs = sorted({ax0, ax1} | {min(max(h[i], ax0), ax1) for h in holes for i in (0, 1)})
    ys = sorted({ay0, ay1} | {min(max(h[i], ay0), ay1) for h in holes for i in (2, 3)})

    def kept(xa, xb, ya, yb):
        mx, my = (xa + xb) / 2, (ya + yb) / 2
        return not any(h[0] < mx < h[1] and h[2] < my < h[3] for h in holes)

    rows = []
    for ya, yb in zip(ys[:-1], ys[1:]):
        if yb - ya < 1e-6:
            continue
        runs, start = [], None
        for xa, xb in zip(xs[:-1], xs[1:]):
            if xb - xa < 1e-6:
                continue
            if kept(xa, xb, ya, yb):
                start = xa if start is None else start
                end = xb
            elif start is not None:
                runs.append((start, end))
                start = None
        if start is not None:
            runs.append((start, end))
        rows.append((ya, yb, runs))
    out = []
    open_ = {}
    for ya, yb, runs in rows:
        nxt = {}
        for run in runs:
            if run in open_:
                nxt[run] = (open_[run][0], yb)
            else:
                nxt[run] = (ya, yb)
        for run, span in open_.items():
            if run not in nxt:
                out.append((run[0], run[1], span[0], span[1]))
        open_ = nxt
    out += [(run[0], run[1], span[0], span[1]) for run, span in open_.items()]
    return out


def room_shell(
    prefix: str,
    x0: float,
    x1: float,
    y0: float,
    y1: float,
    z0: float,
    height: float,
    openings: list[tuple[str, float, float, float]],
    m: dict,
    coll,
    ribs: bool = True,
    floor_strip: bool = False,
    *,
    style: str = "ship",
    corridor: bool | None = None,
    top_chamfer: float | None = None,
    low_chamfer: float | None = None,
    low_gaps=(),
    bands: bool = True,
    dressing: bool = True,
    rib_every: float | None = None,
    beams=True,
    avoid=(),
    floor_holes=(),
    dressing_detail: int = 0,
) -> dict:
    """Sol, plafond et quatre cloisons d'une salle de dimensions intérieures données.

    `openings` : (côté, position le long du mur, largeur, hauteur[, allège]) ; côtés N (+Y),
    S (−Y), E (+X), W (−X). Une allège > 0 fait une fenêtre : vitrage transparent, cadre et
    collision pleine (on ne passe pas). Une ouverture de DOOR_W × DOOR_H est une porte (cadre
    épais, bandes de danger, seuil, boîtier de commande) ; une ouverture qui prend tout le mur
    (couloir qui continue) n'a pas de cadre.

    Paramètres ajoutés en M8 (à mot-clé) :
    - `style` : "ship" (tôle peinte claire, bandeaux froids), "industrial" (soubassement sombre,
      colonnes en H, poutres, bandeaux ambre), "station" (béton, carrelage, pilastres noirs,
      bandes jaune et noir, bandeaux chauds) ;
    - `corridor` : section de couloir (pans coupés le long des deux grands murs, chemin de
      câbles et tuyaux au plafond) ; None : automatique (plus petite dimension ≤ 3 m) ;
    - `top_chamfer` / `low_chamfer` : taille des pans coupés hauts et bas (m) ; None : selon le
      style (couloir : 0,45 et 0,3 ; salle `ship` : 0,35 en haut ; 0 : aucun). Le pan bas a un
      collider (bordure de 0,42 m, qu'on n'enjambe pas) ;
    - `low_gaps` : [(côté, position, largeur)] où le pan bas s'interrompt (casier contre le mur) ;
    - `bands` : bandeaux lumineux d'angle (`sh_band_*`, cuits) ; `dressing` : chemin de câbles et
      tuyaux des couloirs ;
    - `rib_every` : pas des nervures, colonnes et poutres (défaut 2 m, 4 m en `station`) ;
    - `beams` : poutres de plafond (True, False ou liste de positions le long du grand axe) ;
    - `avoid` : [(x, y)] (plafonniers) : pas de poutre à moins de 0,45 m, pas de traverse de
      nervure de couloir à moins de 0,55 m (le long du grand axe).

    Paramètres ajoutés en M9 (à mot-clé) :
    - `style` "hangar" : grande salle haute (sol de béton, soubassement sombre de 2,4 m, colonnes
      en H tous les 6 m, bandeaux ambrés à 3 m) ; `hangar.hangar_shell` y ajoute la charpente ;
    - `floor_holes` : [(x0, x1, y0, y1)] rectangles où le sol (visuel et collision) est percé
      (fosse d'un berceau d'amarrage, `hangar.docking_cradle`) ;
    - `dressing_detail` : niveau de décor de l'habillage des couloirs (chemin de câbles, tuyaux).

    Renvoie {"visual": [...], "structure": [...], "colliders": [...]} (`structure` : ce qui
    reste dans la version simplifiée de la cellule). Les colliders des murs, du sol, du plafond
    et des fenêtres sont exactement ceux de M1 ; s'y ajoutent ceux des détails en saillie.
    """
    if style not in _STYLE:
        raise ValueError(f"style inconnu : {style!r} ({', '.join(STYLES)})")
    S = {k: m[v] for k, v in _STYLE[style].items()}
    out = {"visual": [], "structure": [], "colliders": []}
    w, d = x1 - x0, y1 - y0
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    zt = z0 + height
    if corridor is None:
        corridor = min(w, d) <= 3.0
    long_y = d >= w - 1e-6
    long_sides = ("E", "W") if long_y else ("N", "S")
    every = rib_every or ({"station": 4.0, "hangar": 6.0}.get(style, GRID) if not corridor else GRID)
    if top_chamfer is None:
        top_chamfer = 0.45 if corridor else (0.35 if style == "ship" else 0.0)
    if low_chamfer is None:
        low_chamfer = 0.3 if corridor and style != "station" else 0.0
    tc = top_chamfer
    lc = low_chamfer
    chamfer_sides = long_sides if corridor else ("N", "S", "E", "W")
    n_obj = [0]

    def nm(tag):
        n_obj[0] += 1
        return f"{prefix}_{tag}{n_obj[0]}"

    def add(obj, structure=False):
        out["visual"].append(obj)
        if structure:
            out["structure"].append(obj)
        return obj

    def col(tag, size, center):
        # Nom unique : deux colliders du même nom deviendraient `…_col.001` dans Blender, que le
        # jeu ne reconnaît plus comme collision (il les dessinerait, sans matériau).
        out["colliders"].append(c.collider_box(nm(tag), size, center, coll))

    def bay_offset(side, i):
        k = ("NSEW".index(side) * 3 + i * 5) % len(_BAY_OFFSETS)
        return _BAY_OFFSETS[k]

    # --- Sol -----------------------------------------------------------------------------------
    if floor_holes:
        # Sol percé : un pavé (visuel et collision) par rectangle restant.
        for a0, a1, b0, b1 in _rect_minus((x0 - WALL, x1 + WALL, y0 - WALL, y1 + WALL), [tuple(h) for h in floor_holes]):
            size, center = (a1 - a0, b1 - b0, WALL), ((a0 + a1) / 2, (b0 + b1) / 2, z0 - WALL / 2)
            col("floor", size, center)
            add(_box(nm("floor"), size, center, S["floor"], coll), True)
    else:
        col("floor", (w + 2 * WALL, d + 2 * WALL, WALL), (cx, cy, z0 - WALL / 2))
    if floor_holes:
        pass
    elif floor_strip:
        L = d + 2 * WALL if long_y else w + 2 * WALL
        across = w if long_y else d
        side_w = (across + 2 * WALL - 0.8) / 2

        def lay(size_along, size_across, off, z_size, mat, uv="box", tag="floor"):
            size = (size_across, size_along, z_size) if long_y else (size_along, size_across, z_size)
            center = (cx + off, cy, z0 - z_size / 2) if long_y else (cx, cy + off, z0 - z_size / 2)
            return add(_box(nm(tag), size, center, mat, coll, uv), True)

        lay(L, 0.8, 0.0, 0.03, m["grating"], tag="grating")
        for sx in (-1, 1):
            lay(L, side_w, sx * (0.4 + side_w / 2), 0.1, S["floor"])
            # Cornières sombres le long du caillebotis.
            size = (0.04, L - 2 * WALL, 0.012) if long_y else (L - 2 * WALL, 0.04, 0.012)
            center = (cx + sx * 0.42, cy, z0 + 0.006) if long_y else (cx, cy + sx * 0.42, z0 + 0.006)
            add(_box(nm("angle"), size, center, m["dark"], coll))
    else:
        add(_box(nm("floor"), (w + 2 * WALL, d + 2 * WALL, WALL), (cx, cy, z0 - WALL / 2), S["floor"], coll), True)

    # --- Plafond (une plaque par travée, texture décalée) ----------------------------------------
    col("ceiling", (w + 2 * WALL, d + 2 * WALL, WALL), (cx, cy, zt + WALL / 2))
    if long_y:
        cuts = [y0 - WALL] + [y0 + k * every for k in range(1, int(d / every) + 1) if y0 + k * every < y1 - 0.3] + [y1 + WALL]
        for i, (a, b) in enumerate(zip(cuts[:-1], cuts[1:])):
            add(_box(nm("ceiling"), (w + 2 * WALL, b - a, WALL), (cx, (a + b) / 2, zt + WALL / 2), S["ceiling"], coll, ("box", _BAY_OFFSETS[i % 8])), True)
    else:
        cuts = [x0 - WALL] + [x0 + k * every for k in range(1, int(w / every) + 1) if x0 + k * every < x1 - 0.3] + [x1 + WALL]
        for i, (a, b) in enumerate(zip(cuts[:-1], cuts[1:])):
            add(_box(nm("ceiling"), (b - a, d + 2 * WALL, WALL), ((a + b) / 2, cy, zt + WALL / 2), S["ceiling"], coll, ("box", _BAY_OFFSETS[i % 8])), True)

    walls = {
        # côté : (axe le long, origine, longueur, position fixe)
        "N": ("x", x0 - WALL, w + 2 * WALL, y1 + WALL / 2),
        "S": ("x", x0 - WALL, w + 2 * WALL, y0 - WALL / 2),
        "E": ("y", y0, d, x1 + WALL / 2),
        "W": ("y", y0, d, x0 - WALL / 2),
    }
    frames = {s: _Wall(s, x0, x1, y0, y1, z0, height) for s in walls}
    holes_by_side = {}
    for side, (axis, start, length, fixed) in walls.items():
        holes_by_side[side] = sorted(
            (o[1] - o[2] / 2, o[1] + o[2] / 2, o[3], o[4] if len(o) > 4 else 0.0) for o in openings if o[0] == side
        )

    for side, (axis, start, length, fixed) in walls.items():
        fr = frames[side]
        holes = holes_by_side[side]
        rib_pos = [fr.lo + k * every for k in range(1, int((fr.hi - fr.lo) / every) + 1) if fr.lo + k * every < fr.hi - 0.3]
        # --- Cloisons : colliders de M1, visuels découpés par travée ------------------------------
        cursor = start
        segments = []
        for a, b, oh, sill in holes:
            segments.append((cursor, a, z0, height))
            if sill > 0.0:
                segments.append((a, b, z0, sill))
            segments.append((a, b, z0 + sill + oh, height - sill - oh))
            cursor = b
        segments.append((cursor, start + length, z0, height))
        for i, (a, b, zb, h) in enumerate(segments):
            if b - a < 0.01 or h < 0.01:
                continue
            mid = (a + b) / 2
            if axis == "x":
                size, center = (b - a, WALL, h), (mid, fixed, zb + h / 2)
            else:
                size, center = (WALL, b - a, h), (fixed, mid, zb + h / 2)
            col(f"wall_{side}{i}", size, center)
            bounds = [a] + [p for p in rib_pos if a + 0.05 < p < b - 0.05] + [b]
            for j, (pa, pb) in enumerate(zip(bounds[:-1], bounds[1:])):
                bay = sum(1 for p in rib_pos if p <= (pa + pb) / 2)
                pm = (pa + pb) / 2
                if axis == "x":
                    size, center = (pb - pa, WALL, h), (pm, fixed, zb + h / 2)
                else:
                    size, center = (WALL, pb - pa, h), (fixed, pm, zb + h / 2)
                add(_box(nm(f"wall_{side}"), size, center, S["wall"], coll, ("box", bay_offset(side, bay))), True)

        doors = [(a, b) for a, b, oh, sill in holes if sill <= 0.0]
        full = [(a, b) for a, b, oh, sill in holes if a <= fr.lo + 0.05 and b >= fr.hi - 0.05]

        # --- Cadres des ouvertures -------------------------------------------------------------
        for a, b, oh, sill in holes:
            if (a, b) in full:
                continue
            _opening_frame(prefix, fr, a, b, oh, sill, height, m, S, coll, add, col, nm, holes)

        # --- Pans coupés -----------------------------------------------------------------------
        if side in chamfer_sides and tc > 0.0:
            gaps = [(a - 0.3, b + 0.3) for a, b, oh, sill in holes if sill + oh + 0.3 > height - tc - 0.01 and (a, b) not in full]
            for k, (sa, sb) in enumerate(_cuts(fr.lo, fr.hi, gaps)):
                bounds = [sa] + [p for p in rib_pos if sa + 0.05 < p < sb - 0.05] + [sb]
                for pa, pb in zip(bounds[:-1], bounds[1:]):
                    bay = sum(1 for p in rib_pos if p <= (pa + pb) / 2)
                    add(_xs_slab(nm("chamfer"), fr, (0.0, height - tc), (tc, height), pa, pb, 0.1, S["wall"], coll, ("box", bay_offset(side, bay + 3))), True)
                for s_end, sgn in ((sa, 1), (sb, -1)):
                    tri = [fr.p(0, 0.0, height - tc), fr.p(0, 0.0, height), fr.p(0, tc, height)]
                    add(_prism(nm("chamfer_cap"), tri, fr.t, s_end if sgn > 0 else s_end - 0.02, s_end + 0.02 if sgn > 0 else s_end, m["dark"], coll))
                if bands:
                    _corner_band(prefix, fr, sa + 0.1, sb - 0.1, height - tc, S["band"], m, coll, add, nm)
        elif side in long_sides and bands and style != "ship" and not corridor:
            # Bandeaux de salle sans pan coupé : sous le plafond, entre les nervures (à 3 m dans
            # un hangar : ils éclairent le soubassement et le sol, pas la charpente).
            zb = 3.0 if style == "hangar" else height - (0.62 if style == "station" else 0.42)
            bounds = [fr.lo] + rib_pos + [fr.hi]
            for pa, pb in zip(bounds[:-1], bounds[1:]):
                seg = [(sa, sb) for sa, sb in _cuts(pa + 0.35, pb - 0.35, [(a - 0.3, b + 0.3) for a, b, oh, sill in holes if sill + oh + 0.3 > zb]) if sb - sa > 0.6]
                for sa, sb in seg:
                    _corner_band(prefix, fr, sa, sb, zb, S["band"], m, coll, add, nm, cove=False)

        if side in chamfer_sides and lc > 0.0:
            gaps = [(a - 0.26, b + 0.26) for a, b in doors if (a, b) not in full]
            gaps += [(p - wd / 2, p + wd / 2) for sd, p, wd in low_gaps if sd == side]
            for sa, sb in _cuts(fr.lo, fr.hi, gaps):
                add(_xs_slab(nm("kerb"), fr, (lc, 0.0), (0.0, lc), sa, sb, 0.1, m["dark"], coll, ("box", bay_offset(side, 5))), True)
                # Plat rivé au pied, là où les pieds frottent.
                q = (lc - 0.12 * 0.7071, 0.12 * 0.7071)
                add(_xs_slab(nm("kick"), fr, (lc, 0.0), q, sa, sb, 0.012, m["trim_sheet"], coll, "rivets", inset=0.012))
                for s_end, sgn in ((sa, 1), (sb, -1)):
                    tri = [fr.p(0, 0.0, 0.0), fr.p(0, lc, 0.0), fr.p(0, 0.0, lc)]
                    add(_prism(nm("kerb_cap"), tri, fr.t, s_end if sgn > 0 else s_end - 0.02, s_end + 0.02 if sgn > 0 else s_end, m["dark"], coll))
                # Bordure qu'on n'enjambe pas (0,42 m > marche du contrôleur).
                ctr = fr.p((sa + sb) / 2, lc / 4, 0.21)
                size = (sb - sa, lc / 2, 0.42) if side in "NS" else (lc / 2, sb - sa, 0.42)
                col(f"kerb_{side}", size, tuple(ctr))
        elif side in chamfer_sides or not corridor:
            # Plinthe / soubassement.
            gaps = [(a - 0.26, b + 0.26) for a, b in doors]
            for sa, sb in _cuts(fr.lo, fr.hi, gaps):
                _wainscot(prefix, fr, sa, sb, style, m, coll, add, nm)

        # --- Nervures --------------------------------------------------------------------------
        if not ribs:
            continue
        clear = [(a - 0.45, b + 0.45) for a, b, oh, sill in holes if (a, b) not in full]
        for p in rib_pos:
            if any(ca < p < cb for ca, cb in clear):
                continue
            if corridor and side not in long_sides:
                continue
            if style in ("industrial", "hangar") and not corridor and side in long_sides:
                _column(prefix, fr, p, height, m, coll, nm, col, add, heavy=style == "hangar")
                continue
            if style == "station" and not corridor:
                _pilaster(prefix, fr, p, height, m, coll, nm, col, add)
                continue
            prof = []
            if lc > 0 and side in chamfer_sides:
                prof.append(((lc, 0.0), (0.0, lc)))
                zlo = lc
            else:
                zlo = 0.0
            ztop = height - tc if (tc > 0 and side in chamfer_sides) else height
            prof.append(((0.0, zlo), (0.0, ztop)))
            if tc > 0 and side in chamfer_sides:
                prof.append(((0.0, height - tc), (tc, height)))
            for q1, q2 in prof:
                half = 0.086 if q1[0] != q2[0] else 0.09
                add(_xs_slab(nm("rib"), fr, q1, q2, p - half, p + half, 0.08, S["rib"], coll, "rivets", inset=0.08, extend=0.08 * 0.4142), True)
            near_lamp = any(abs((ay if long_y else ax) - p) < 0.55 for ax, ay in avoid)
            if corridor and side == long_sides[0] and not near_lamp:
                # Traverse du plafond : la nervure fait le tour de la section (sauf sous un
                # plafonnier de `avoid`).
                add(_xs_slab(nm("rib"), fr, (tc, height), (fr.span - tc, height), p - 0.09, p + 0.09, 0.08, S["rib"], coll, "rivets", inset=0.08, extend=0.08 * 0.4142), True)

    # --- Poutres de plafond (salles) -------------------------------------------------------------
    if ribs and not corridor and beams:
        if beams is True:
            lo, hi = (y0, y1) if long_y else (x0, x1)
            positions = [lo + k * every for k in range(1, int((hi - lo) / every) + 1) if lo + k * every < hi - 0.3]
        else:
            positions = list(beams)
        for p in positions:
            if any(abs((ay if long_y else ax) - p) < 0.45 for ax, ay in avoid):
                continue
            if long_y:
                p0, p1 = (x0, p, zt), (x1, p, zt)
            else:
                p0, p1 = (p, y0, zt), (p, y1, zt)
            depth = 0.26 if style != "ship" else 0.16
            vis, _, _ = ceiling_beam(nm("beam"), p0, p1, m, coll, depth=depth, width=0.22 if style == "ship" else 0.26, mat=S["rib"])
            for o in vis:
                add(o, True)

    # --- Habillage des couloirs : chemin de câbles et tuyaux au plafond --------------------------
    if corridor and dressing:
        if long_y:
            a, b = (cx + 0.42, y0 + 0.02, zt), (cx + 0.42, y1 - 0.02, zt)
            pa = [((cx - 0.42, y0 + 0.02, zt - 0.16), (cx - 0.42, y1 - 0.02, zt - 0.16)), ((cx - 0.58, y0 + 0.02, zt - 0.13), (cx - 0.58, y1 - 0.02, zt - 0.13))]
        else:
            a, b = (x0 + 0.02, cy + 0.42, zt), (x1 - 0.02, cy + 0.42, zt)
            pa = [((x0 + 0.02, cy - 0.42, zt - 0.16), (x1 - 0.02, cy - 0.42, zt - 0.16)), ((x0 + 0.02, cy - 0.58, zt - 0.13), (x1 - 0.02, cy - 0.58, zt - 0.13))]
        for o in cable_tray(nm("tray"), (a[0], a[1], zt - 0.12), (b[0], b[1], zt - 0.12), m, coll, hang_to=zt, detail=dressing_detail)[0]:
            add(o)
        for i, (q0, q1) in enumerate(pa):
            for o in pipe(nm("pipe"), q0, q1, m, coll, radius=0.05 if i == 0 else 0.035, hang_to=zt, detail=dressing_detail)[0]:
                add(o)
    return out


def _opening_frame(prefix, fr: _Wall, a, b, oh, sill, height, m, S, coll, add, col, nm, holes):
    """Cadre épais d'une ouverture (porte, passage, fenêtre) côté pièce : montants et linteau
    biseautés, goussets à 45° (ouverture à pans coupés), tableau sombre dans l'épaisseur du mur.
    Porte : bandes de danger sur les montants, lampe de linteau, seuil, boîtier de commande.
    L'ouverture libre ne change pas."""
    depth = 0.16
    zb = sill
    is_door = sill <= 0.0 and abs((b - a) - DOOR_W) < 0.01 and abs(oh - DOOR_H) < 0.01
    frame = S["frame"]
    rt = fr.right()
    hh = min(0.3, height - (zb + oh))
    for s, sgn in ((a, -1), (b, 1)):
        sc = s + sgn * 0.12
        ctr = fr.p(sc, depth / 2, zb + oh / 2)
        size = (0.24, depth, oh) if fr.side in "NS" else (depth, 0.24, oh)
        add(_box(nm("jamb"), size, tuple(ctr), frame, coll, bevel=0.025), True)
        if zb < HEAD_CLEAR:
            col("jamb", size, tuple(ctr))
        # Tableau sombre dans l'épaisseur du mur (moitié de cette cellule), 1 cm dans
        # l'ouverture : il cache les bords du panneau de porte fermé.
        lin = fr.p(s - sgn * 0.006, -WALL / 2, zb + oh / 2)
        size = (0.012, WALL, oh) if fr.side in "NS" else (WALL, 0.012, oh)
        add(_box(nm("reveal"), size, tuple(lin), m["dark"], coll))
        if sill <= 0.0:
            add(_ori(nm("jamb_hazard"), (min(oh * 0.5, 1.1), 0.12, 0.006), tuple(fr.p(sc, depth + 0.003, min(oh * 0.25, 0.55))),
                     (0, 0, 1), tuple(rt * -1.0), m["trim_sheet"], coll, "hazard"))
    top = fr.p((a + b) / 2, -WALL / 2, zb + oh - 0.006)
    size = (b - a, WALL, 0.012) if fr.side in "NS" else (WALL, b - a, 0.012)
    add(_box(nm("reveal"), size, tuple(top), m["dark"], coll))
    if hh > 0.05:
        ctr = fr.p((a + b) / 2, depth / 2, zb + oh + hh / 2)
        size = (b - a + 0.48, depth, hh) if fr.side in "NS" else (depth, b - a + 0.48, hh)
        add(_box(nm("header"), size, tuple(ctr), frame, coll, bevel=0.025), True)
        if hh >= 0.15:
            add(_ori(nm("header_lamp"), (min(0.6, b - a - 0.2), 0.06, 0.02), tuple(fr.p((a + b) / 2, depth + 0.01, zb + oh + hh * 0.6)),
                     tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "light_strip"))
    if oh >= 1.2:
        # Goussets à 45° dans les coins hauts : l'ouverture se lit à pans coupés.
        for s, sgn in ((a, 1), (b, -1)):
            ctr = fr.p(s + sgn * 0.08, depth / 2, zb + oh - 0.08)
            dr = fr.t * sgn
            add(_ori(nm("gusset"), (0.34, 0.12, depth - 0.012), tuple(ctr), tuple((dr + Vector((0, 0, 1))).normalized()),
                     tuple((-dr + Vector((0, 0, 1))).normalized()), frame, coll, bevel=0.02))
    if sill > 0.0:
        # Fenêtre : appui, vitrage et collision pleine (comme en M1), goussets bas.
        ctr = fr.p((a + b) / 2, depth / 2, zb - 0.07)
        size = (b - a + 0.48, depth + 0.04, 0.14) if fr.side in "NS" else (depth + 0.04, b - a + 0.48, 0.14)
        add(_box(nm("sill"), size, tuple(ctr), frame, coll, bevel=0.02), True)
        for s, sgn in ((a, 1), (b, -1)):
            ctr = fr.p(s + sgn * 0.08, depth / 2, zb + 0.08)
            dr = fr.t * sgn
            add(_ori(nm("gusset"), (0.34, 0.12, depth - 0.012), tuple(ctr), tuple((dr - Vector((0, 0, 1))).normalized()),
                     tuple((dr + Vector((0, 0, 1))).normalized()), frame, coll, bevel=0.02))
        mid = fr.p((a + b) / 2, -WALL / 2, zb + oh / 2)
        size = (b - a, 0.03, oh) if fr.side in "NS" else (0.03, b - a, oh)
        add(_box(nm("glass"), size, tuple(mid), m["glass"], coll), True)
        wall_size = (b - a, WALL, oh) if fr.side in "NS" else (WALL, b - a, oh)
        col("glass", wall_size, tuple(mid))
        bot = fr.p((a + b) / 2, -WALL / 2, zb + 0.006)
        size = (b - a, WALL, 0.012) if fr.side in "NS" else (WALL, b - a, 0.012)
        add(_box(nm("reveal"), size, tuple(bot), m["dark"], coll))
        return
    # Seuil : bande de danger au sol devant l'ouverture.
    add(_ori(nm("threshold"), (b - a, 0.22, 0.006), tuple(fr.p((a + b) / 2, 0.14, 0.003)), tuple(rt), tuple(-fr.n), m["trim_sheet"], coll, "hazard"))
    if not is_door:
        return
    # Boîtier de commande à droite de la porte (vu de la pièce), sinon à gauche, s'il y a la place.
    others = [(ha, hb) for ha, hb, _, _ in holes if (ha, hb) != (a, b)]
    rsign = 1 if rt.dot(fr.t) > 0 else -1
    for sgn in (rsign, -rsign):
        s = (b + 0.24 + 0.22) if sgn > 0 else (a - 0.24 - 0.22)
        if not (fr.lo + 0.2 < s < fr.hi - 0.2) or any(ha - 0.2 < s < hb + 0.2 for ha, hb in others):
            continue
        ctr = fr.p(s, 0.03, 1.25)
        size = (0.26, 0.06, 0.38) if fr.side in "NS" else (0.06, 0.26, 0.38)
        add(_box(nm("door_ctrl_box"), size, tuple(ctr), m["dark"], coll, bevel=0.01))
        add(_ori(nm("door_ctrl"), (0.2, 0.3, 0.01), tuple(fr.p(s, 0.063, 1.25)), tuple(rt), (0, 0, 1), m["buttons"], coll))
        break


def _corner_band(prefix, fr: _Wall, sa, sb, z, band_mat, m, coll, add, nm, cove=True):
    """Bandeau lumineux d'angle (carter sombre + diffuseur `sh_band_*`) le long du mur, à la
    hauteur z, tourné vers l'axe de la pièce et un peu vers le bas."""
    if sb - sa < 0.3:
        return
    nrm = (fr.n * 0.92 + Vector((0, 0, -0.38))).normalized()
    right = fr.t.copy()
    up = nrm.cross(right)
    if up.z < 0:
        right, up = -right, -up
    off = 0.07 if cove else 0.09
    corner = fr.p((sa + sb) / 2, 0.0, z) + nrm * off
    add(_ori(nm("band_housing"), (sb - sa, 0.16, 0.06), tuple(corner - nrm * 0.035), tuple(right), tuple(up), m["dark"], coll, bevel=0.01))
    add(_ori(nm("band"), (sb - sa - 0.2, 0.085, 0.02), tuple(corner + nrm * 0.002), tuple(right), tuple(up), band_mat, coll, ("strip", 1.0)))


def _wainscot(prefix, fr: _Wall, sa, sb, style, m, coll, add, nm):
    """Bas de mur : plinthe rivée (ship), soubassement sombre et lisse (industrial), socle noir,
    bande de danger et lisse (station)."""
    rt = fr.right()
    s = (sa + sb) / 2
    L = sb - sa
    if style == "ship":
        add(_ori(nm("kick"), (L, 0.12, 0.012), tuple(fr.p(s, 0.006, 0.065)), tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "rivets"))
    elif style == "industrial":
        size = (L, 0.04, 0.9) if fr.side in "NS" else (0.04, L, 0.9)
        add(_box(nm("wainscot"), size, tuple(fr.p(s, 0.02, 0.45)), m["dark"], coll))
        add(_ori(nm("rail"), (L, 0.11, 0.05), tuple(fr.p(s, 0.045, 0.95)), tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "edge"))
        add(_ori(nm("kick"), (L, 0.12, 0.012), tuple(fr.p(s, 0.046, 0.065)), tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "rivets"))
    elif style == "hangar":
        # Soubassement sombre de 2,4 m, lisse, pied à bandes de danger.
        size = (L, 0.04, 2.4) if fr.side in "NS" else (0.04, L, 2.4)
        add(_box(nm("wainscot"), size, tuple(fr.p(s, 0.02, 1.2)), m["dark"], coll), True)
        add(_ori(nm("rail"), (L, 0.11, 0.05), tuple(fr.p(s, 0.045, 2.45)), tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "edge"))
        add(_ori(nm("hazard_foot"), (L, 0.3, 0.012), tuple(fr.p(s, 0.046, 0.15)), tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "hazard"))
    else:
        size = (L, 0.05, 0.18) if fr.side in "NS" else (0.05, L, 0.18)
        add(_box(nm("socle"), size, tuple(fr.p(s, 0.025, 0.09)), m["paint_black"], coll))
        add(_ori(nm("hazard_band"), (L, 0.14, 0.012), tuple(fr.p(s, 0.006, 1.15)), tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "hazard"))
        add(_ori(nm("rail"), (L, 0.11, 0.05), tuple(fr.p(s, 0.025, 1.27)), tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "edge"))


def _column(prefix, fr: _Wall, s, height, m, coll, nm, col, add, heavy: bool = False):
    """Colonne en H contre le mur (style industriel) : âme, semelles, face rivée, gousset de
    pied ; collider. `heavy` (hangar) : profil de 0,5 m, pied jaune à bandes de danger sur 1,5 m."""
    z = Vector((0, 0, 1))
    k = 1.9 if heavy else 1.0
    wd, dp = 0.26 * k, 0.2 * k
    size_web = (0.025 * k, dp - 0.02, height) if fr.side in "NS" else (dp - 0.02, 0.025 * k, height)
    add(_box(nm("col_web"), size_web, tuple(fr.p(s, dp / 2, height / 2)), m["dark"], coll), True)
    size_back = (wd, 0.025 * k, height) if fr.side in "NS" else (0.025 * k, wd, height)
    add(_box(nm("col_back"), size_back, tuple(fr.p(s, 0.0125 * k, height / 2)), m["dark"], coll))
    add(_ori(nm("col_front"), (height, wd, 0.025 * k), tuple(fr.p(s, dp - 0.01 * k, height / 2)), (0, 0, 1), tuple(fr.n.cross(z)), m["trim_sheet"], coll, "rivets"), True)
    size_foot = (wd + 0.08 * k, dp + 0.04, 0.1 * k) if fr.side in "NS" else (dp + 0.04, wd + 0.08 * k, 0.1 * k)
    add(_box(nm("col_foot"), size_foot, tuple(fr.p(s, dp / 2 + 0.02, 0.05 * k)), m["dark"], coll, bevel=0.01))
    if heavy:
        # Gaine de protection jaune au pied (chocs des chariots), bandes de danger.
        guard = (wd + 0.04, 0.03, 1.5) if fr.side in "NS" else (0.03, wd + 0.04, 1.5)
        add(_box(nm("col_guard"), guard, tuple(fr.p(s, dp + 0.015, 0.75 + 0.1 * k)), m["paint_yellow"], coll))
        add(_ori(nm("col_guard_band"), (wd + 0.04, 0.3, 0.008), tuple(fr.p(s, dp + 0.034, 1.4)), tuple(fr.right()), (0, 0, 1), m["trim_sheet"], coll, "hazard"))
    size = (wd + 0.02, dp, height) if fr.side in "NS" else (dp, wd + 0.02, height)
    col("column", size, tuple(fr.p(s, dp / 2, height / 2)))


def _pilaster(prefix, fr: _Wall, s, height, m, coll, nm, col, add):
    """Pilastre noir du spatioport (style station) : bande jaune, pochoir, pied à bandes de
    danger ; collider."""
    size = (0.5, 0.18, height) if fr.side in "NS" else (0.18, 0.5, height)
    add(_box(nm("pilaster"), size, tuple(fr.p(s, 0.09, height / 2)), m["paint_black"], coll, bevel=0.02), True)
    rt = fr.right()
    add(_ori(nm("pilaster_band"), (0.5, 0.3, 0.01), tuple(fr.p(s, 0.185, 1.6)), tuple(rt), (0, 0, 1), m["paint_yellow"], coll))
    add(_ori(nm("pilaster_label"), (0.44, 0.12, 0.004), tuple(fr.p(s, 0.192, 1.6)), tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "stencils"))
    add(_ori(nm("pilaster_foot"), (0.5, 0.25, 0.01), tuple(fr.p(s, 0.185, 0.3)), tuple(rt), (0, 0, 1), m["trim_sheet"], coll, "hazard"))
    col("pilaster", size, tuple(fr.p(s, 0.09, height / 2)))


# --- Props ------------------------------------------------------------------------------------
# Chaque prop renvoie (objets visibles, colliders, repères). Orientation : `facing` en radians
# autour de Z, 0 = la face utile regarde vers +Y. Construits autour de l'origine, puis placés.

_FRONT = ((-1, 0, 0), (0, 0, 1))  # (right, up) d'une face tournée vers +Y


def console(name: str, center, facing: float, m: dict, coll, label: str = "console", low: bool = False):
    """Console inclinée : caisson, pupitre à boutons rétroéclairés, panneau d'écrans segmentés
    incliné vers l'arrière, visière éclairante, joues ; `low` : panneau bas, sous la ligne des
    yeux assis (postes devant un hublot). L'opérateur se tient côté +Y."""
    vis = []
    vis.append(_box(f"{name}_base", (1.2, 0.5, 0.8), (0, -0.04, 0.4), m["dark"], coll, bevel=0.02))
    vis.append(_box(f"{name}_kick", (1.1, 0.06, 0.1), (0, 0.18, 0.05), m["rubber"], coll))
    vis.append(_ori(f"{name}_vent", (0.8, 0.2, 0.006), (0, 0.213, 0.42), *_FRONT, m["trim_sheet"], coll, "vent"))
    vis.append(_ori(f"{name}_label", (0.5, 0.1, 0.004), (0.25, 0.212, 0.66), *_FRONT, m["trim_sheet"], coll, "stencils"))
    a = math.radians(20)
    up_desk = (0, -math.cos(a), math.sin(a))
    n_desk = Vector((0, math.sin(a), math.cos(a)))
    desk_c = Vector((0, 0.07, 0.86))
    vis.append(_ori(f"{name}_desk", (1.28, 0.46, 0.05), tuple(desk_c), (-1, 0, 0), up_desk, m["dark"], coll, bevel=0.01))
    vis.append(_ori(f"{name}_buttons", (1.08, 0.36, 0.012), tuple(desk_c + n_desk * 0.03), (-1, 0, 0), up_desk, m["buttons"], coll))
    vis.append(_ori(f"{name}_lip", (1.26, 0.05, 0.06), (0, 0.265, 0.81), *_FRONT, m["trim_sheet"], coll, "gasket"))
    # Leviers et molettes : du relief sur le pupitre.
    for i, x in enumerate((-0.5, -0.44, 0.44, 0.5)):
        p = desk_c + n_desk * 0.06 + Vector(up_desk) * -0.08
        vis.append(_cyl(f"{name}_lever{i}", 0.012, 0.07, (x, p.y, p.z), m["bare"], coll, segments=8))
    sz, sh = (0.42, 1.03) if low else (0.62, 1.24)
    b = math.radians(12)
    up_scr = (0, -math.sin(b), math.cos(b))
    n_scr = Vector((0, math.cos(b), math.sin(b)))
    up_v = Vector(up_scr)
    pc = Vector((0, -0.2, sh))
    vis.append(_ori(f"{name}_panel", (1.26, sz, 0.08), tuple(pc), (-1, 0, 0), up_scr, m["dark"], coll, bevel=0.015))
    for i, (x, w_) in enumerate(((-0.42, 0.34), (0.0, 0.4), (0.42, 0.34))):
        sc = pc + Vector((x, 0, 0)) + n_scr * 0.046 + up_v * (sz * 0.12)
        vis.append(_ori(f"{name}_screen{i}", (w_, sz * 0.6, 0.012), tuple(sc), (-1, 0, 0), up_scr, m["screen"], coll, tile=0.75))
    bc = pc + n_scr * 0.044 - up_v * (sz * 0.34)
    vis.append(_ori(f"{name}_panel_buttons", (1.1, sz * 0.2, 0.01), tuple(bc), (-1, 0, 0), up_scr, m["buttons"], coll))
    top = pc + up_v * (sz / 2)
    hood = top + Vector((0, 0.09, 0.02))
    vis.append(_ori(f"{name}_hood", (1.3, 0.22, 0.04), tuple(hood), (-1, 0, 0), (0, 1, 0), m["dark"], coll, bevel=0.01))
    vis.append(_ori(f"{name}_hood_lamp", (1.1, 0.05, 0.01), tuple(hood - Vector((0, -0.03, 0.022))), (-1, 0, 0), (0, 1, 0), m["trim_sheet"], coll, "light_strip"))
    for sx in (-1, 1):
        vis.append(_box(f"{name}_cheek{sx}", (0.04, 0.52, 0.95), (sx * 0.62, -0.03, 0.475), m["dark"], coll, bevel=0.01))
        hu = top.z - 0.9
        vis.append(_box(f"{name}_upright{sx}", (0.05, 0.1, hu), (sx * 0.61, -0.25, 0.9 + hu / 2), m["dark"], coll))
    vis.append(_cyl(f"{name}_conduit", 0.03, 0.95, (0.45, -0.27, 0.5), m["rubber"], coll, segments=8))
    col = [c.collider_box(f"{name}", (1.3, 0.6, 1.0), (0, 0, 0.5), coll)]
    # Volume d'interaction (M9) : toute la console, du sol à la visière.
    top = 1.3 if low else 1.6
    marks = [_interact(name, (0, 0, top / 2), (1.3, 0.6, top), {"interact": "console", "label": label}, coll)]
    _place(vis + col + marks, center, facing)
    return vis, col, marks


def seat(name: str, center, facing: float, m: dict, coll, role: str = "passenger"):
    """Siège de poste : pied à embase, assise et dossier matelassés, appui-tête, accoudoirs
    (pavé de commandes sur le droit). Assis face à +Y.

    Toujours interactif (`interact: "seat"`, `role` : "pilot", "copilot", "passenger"…), avec
    l'œil `SOCKET_<name>_eye` (`usage: "seat_eye"`) ; M9 : volume d'interaction qui couvre tout
    le siège (on s'assoit en visant n'importe où sur la chaise)."""
    vis = [
        _cyl(f"{name}_foot", 0.24, 0.04, (0, 0, 0.02), m["dark"], coll, segments=16),
        _cyl(f"{name}_post", 0.06, 0.34, (0, 0, 0.21), m["bare"], coll, segments=10),
        _box(f"{name}_pan", (0.56, 0.54, 0.06), (0, 0, 0.4), m["dark"], coll, bevel=0.01),
        _box(f"{name}_cushion", (0.56, 0.54, 0.11), (0, 0.01, 0.485), m["fabric"], coll, bevel=0.035),
    ]
    a = math.radians(8)
    up = Vector((0, -math.sin(a), math.cos(a)))
    nrm = Vector((0, math.cos(a), math.sin(a)))
    bc = Vector((0, -0.25, 0.88))
    vis.append(_ori(f"{name}_back", (0.54, 0.62, 0.1), tuple(bc), (-1, 0, 0), tuple(up), m["fabric"], coll, bevel=0.035))
    vis.append(_ori(f"{name}_shell", (0.58, 0.68, 0.05), tuple(bc - nrm * 0.07), (-1, 0, 0), tuple(up), m["dark"], coll, bevel=0.012))
    hc = bc + up * 0.43 + nrm * 0.0
    vis.append(_ori(f"{name}_head", (0.32, 0.18, 0.09), tuple(hc), (-1, 0, 0), tuple(up), m["fabric"], coll, bevel=0.03))
    for sx in (-1, 1):
        vis.append(_box(f"{name}_arm{sx}", (0.07, 0.42, 0.05), (sx * 0.32, -0.02, 0.66), m["dark"], coll, bevel=0.01))
        vis.append(_box(f"{name}_arm_pad{sx}", (0.065, 0.32, 0.025), (sx * 0.32, -0.05, 0.695), m["rubber"], coll))
        vis.append(_box(f"{name}_arm_post{sx}", (0.05, 0.05, 0.22), (sx * 0.32, 0.12, 0.54), m["dark"], coll))
        vis.append(_box(f"{name}_head_post{sx}", (0.025, 0.025, 0.12), (sx * 0.1, -0.31, 1.2), m["bare"], coll))
    vis.append(_ori(f"{name}_pad", (0.055, 0.12, 0.006), (0.32, 0.1, 0.688), (1, 0, 0), (0, 1, 0), m["buttons"], coll))
    col = [c.collider_box(f"{name}", (0.7, 0.7, 1.2), (0, -0.05, 0.6), coll)]
    marks = [
        _interact(name, (0, -0.05, 0.62), (0.7, 0.7, 1.24), {"interact": "seat", "role": role}, coll, margin=0.03),
        c.empty(f"SOCKET_{name}_eye", (0, -0.05, 1.25), coll, {"usage": "seat_eye"}),
    ]
    _place(vis + col + marks, center, facing)
    return vis, col, marks


def bunk(name: str, center, facing: float, m: dict, coll):
    """Couchettes superposées, ouvertes des deux côtés (on peut l'adosser par ±Y) : bâti
    sombre à tiroirs, matelas matelassés, oreillers, garde-corps haut, liseuse chaude (cuite)."""
    vis = [
        _box(f"{name}_base", (2.0, 0.9, 0.3), (0, 0, 0.15), m["dark"], coll, bevel=0.015),
        _box(f"{name}_mat_lo", (1.9, 0.8, 0.14), (0, 0, 0.42), m["fabric"], coll, bevel=0.035),
        _box(f"{name}_pillow_lo", (0.4, 0.6, 0.08), (-0.7, 0, 0.53), m["fabric"], coll, bevel=0.03),
        _box(f"{name}_frame_hi", (2.0, 0.9, 0.08), (0, 0, 1.3), m["dark"], coll, bevel=0.01),
        _box(f"{name}_mat_hi", (1.9, 0.8, 0.14), (0, 0, 1.41), m["fabric"], coll, bevel=0.035),
        _box(f"{name}_pillow_hi", (0.4, 0.6, 0.08), (-0.7, 0, 1.52), m["fabric"], coll, bevel=0.03),
        _box(f"{name}_lamp_box", (0.3, 0.12, 0.04), (-0.55, 0, 1.24), m["dark"], coll),
        _ori(f"{name}_lamp", (0.24, 0.06, 0.01), (-0.55, 0, 1.217), (1, 0, 0), (0, -1, 0), m["band_warm"], coll, ("strip", 0.5)),
    ]
    for sx in (-1, 1):
        for sy in (-1, 1):
            vis.append(_box(f"{name}_post{sx}{sy}", (0.06, 0.06, 1.9), (sx * 0.97, sy * 0.42, 0.95), m["dark"], coll))
        for i, x in enumerate((-0.5, 0.5)):
            vis.append(_ori(f"{name}_drawer{sx}{i}", (0.8, 0.2, 0.01), (x, sx * 0.452, 0.15), (-sx, 0, 0), (0, 0, 1), m["panel"], coll, ("box", (0.25 * i, 0.1))))
            vis.append(_ori(f"{name}_handle{sx}{i}", (0.2, 0.03, 0.02), (x, sx * 0.467, 0.21), (-sx, 0, 0), (0, 0, 1), m["bare"], coll))
        # Garde-corps du haut, côté ouvert, et plaque à pochoirs.
        vis.append(_box(f"{name}_rail{sx}", (1.1, 0.03, 0.03), (0.35, sx * 0.43, 1.62), m["bare"], coll))
        vis.append(_box(f"{name}_rail_post{sx}", (0.03, 0.03, 0.22), (-0.2, sx * 0.43, 1.5), m["bare"], coll))
        vis.append(_ori(f"{name}_label{sx}", (0.5, 0.08, 0.004), (0.6, sx * 0.452, 1.3), (-sx, 0, 0), (0, 0, 1), m["trim_sheet"], coll, "stencils"))
    col = [
        c.collider_box(f"{name}_lo", (2.0, 0.9, 0.5), (0, 0, 0.25), coll),
        c.collider_box(f"{name}_hi", (2.0, 0.9, 0.25), (0, 0, 1.35), coll),
    ]
    _place(vis + col, center, facing)
    return vis, col, []


def bed(name: str, center, facing: float, m: dict, coll, size=(2.1, 1.0), *, lamp: bool = True, wall_pad: bool = True,
        pad_gaps=()):
    """Couchette de capitaine (M9) : socle de rangement sombre à deux tiroirs (face +Y), cadre de
    métal brut, matelas et oreiller matelassés, couverture pliée, tête de lit (−X) à coussin et
    tablette, pied de lit à barre ; `wall_pad` : dosseret matelassé côté −Y (le lit s'adosse au
    mur par −Y), interrompu sur 0,26 m à chaque position x locale de `pad_gaps` (nervure du mur
    de `room_shell`, tous les 2 m) ; `lamp` : liseuse chaude et son repère `LIGHT_<name>_lamp`
    (220 lm, 2,5 m). `size` : (longueur le long de X, largeur le long de Y). Colliders : couchage
    et tête de lit."""
    L, Wd = size
    base_h = 0.36
    mz = base_h + 0.05  # dessous du matelas
    ys = 0.03 if wall_pad else 0.0  # le matelas s'écarte du dosseret
    vis = [
        _box(f"{name}_base", (L, Wd, base_h - 0.08), (0, 0, 0.08 + (base_h - 0.08) / 2), m["dark"], coll, bevel=0.012),
        _box(f"{name}_kick", (L - 0.08, Wd - 0.1, 0.08), (0, -0.03, 0.04), m["rubber"], coll),
        _box(f"{name}_mattress", (L - 0.12, Wd - 0.1 - ys, 0.17), (0.0, ys / 2, mz + 0.085), m["fabric_light"], coll, bevel=0.04),
        _box(f"{name}_pillow", (0.38, Wd - 0.34, 0.11), (-L / 2 + 0.3, ys / 2, mz + 0.22), m["fabric_light"], coll, bevel=0.045),
        _box(f"{name}_blanket", (0.72, Wd - 0.05 - ys, 0.06), (L / 2 - 0.46, ys / 2, mz + 0.2), m["fabric_dark"], coll, bevel=0.025),
        _box(f"{name}_blanket_fold", (0.26, Wd - 0.07 - ys, 0.05), (L / 2 - 0.24, ys / 2, mz + 0.255), m["fabric_dark"], coll, bevel=0.022),
    ]
    drawer_z = 0.08 + (base_h - 0.08) / 2
    for i, x in enumerate((-L / 4 + 0.04, L / 4 - 0.04)):
        dw = L / 2 - 0.2
        vis.append(_ori(f"{name}_drawer{i}", (dw, base_h - 0.15, 0.014), (x, Wd / 2 + 0.007, drawer_z), *_FRONT, m["panel"], coll, ("box", (0.3 * i, 0.1)), bevel=0.003))
        vis.append(_box(f"{name}_pull{i}", (0.22, 0.02, 0.03), (x, Wd / 2 + 0.024, drawer_z + 0.05), m["bare"], coll))
    vis.append(_ori(f"{name}_label", (0.3, 0.07, 0.004), (L / 4 - 0.04, Wd / 2 + 0.016, drawer_z - 0.045), *_FRONT, m["trim_sheet"], coll, "stencils"))
    # Cadre de métal brut : longerons et traverse de pied au ras du socle.
    for sy in (-1, 1):
        vis.append(_box(f"{name}_rail{sy}", (L, 0.05, 0.07), (0, sy * (Wd / 2 - 0.025), base_h + 0.035), m["bare"], coll, bevel=0.008))
    vis.append(_box(f"{name}_rail_foot", (0.05, Wd - 0.1, 0.07), (L / 2 - 0.025, 0, base_h + 0.035), m["bare"], coll, bevel=0.008))
    # Pied de lit : deux montants et une barre.
    for sy in (-1, 1):
        vis.append(_box(f"{name}_foot_post{sy}", (0.05, 0.05, 0.66 - base_h), (L / 2 - 0.025, sy * (Wd / 2 - 0.025), base_h + (0.66 - base_h) / 2), m["dark"], coll))
    vis.append(_box(f"{name}_foot_bar", (0.06, Wd, 0.05), (L / 2 - 0.025, 0, 0.66), m["dark"], coll, bevel=0.01))
    # Tête de lit (−X) : panneau, coussin, tablette, liseuse.
    hb = 1.05
    vis.append(_box(f"{name}_head", (0.08, Wd + 0.04, hb), (-L / 2 - 0.04, 0, hb / 2), m["dark"], coll, bevel=0.012))
    vis.append(_ori(f"{name}_head_pad", (Wd - 0.16, 0.42, 0.06), (-L / 2 + 0.03, ys / 2, mz + 0.45), (0, 1, 0), (0, 0, 1), m["fabric"], coll, bevel=0.025))
    vis.append(_box(f"{name}_shelf", (0.26, Wd + 0.04, 0.04), (-L / 2 + 0.05, 0, hb + 0.02), m["dark"], coll, bevel=0.008))
    vis.append(_box(f"{name}_shelf_lip", (0.02, Wd + 0.04, 0.03), (-L / 2 + 0.17, 0, hb + 0.055), m["bare"], coll))
    lx, ly = -L / 2 + 0.1, -Wd / 2 + 0.2
    vis.append(_box(f"{name}_lamp_box", (0.12, 0.2, 0.06), (lx, ly, hb - 0.1), m["dark"], coll, bevel=0.008))
    vis.append(_ori(f"{name}_lamp", (0.16, 0.06, 0.01), (lx + 0.005, ly, hb - 0.132), (1, 0, 0), (0, -1, 0), m["band_warm"], coll, ("strip", 0.5)))
    if wall_pad:
        lo, hi = -L / 2 + 0.21, L / 2 - 0.05
        for k, (sa, sb) in enumerate(_cuts(lo, hi, [(g - 0.13, g + 0.13) for g in pad_gaps])):
            if sb - sa < 0.25:
                continue
            xm = (sa + sb) / 2
            vis.append(_box(f"{name}_wall_frame{k}", (sb - sa, 0.02, 0.5), (xm, -Wd / 2 + 0.01, mz + 0.42), m["dark"], coll))
            vis.append(_ori(f"{name}_wall_pad{k}", (sb - sa - 0.08, 0.4, 0.05), (xm, -Wd / 2 + 0.045, mz + 0.42), *_FRONT, m["fabric"], coll,
                            ("box", (0.3 * k, 0.0)), bevel=0.02))
    col = [
        c.collider_box(f"{name}", (L, Wd, 0.62), (0, 0, 0.31), coll),
        c.collider_box(f"{name}_head", (0.1, Wd + 0.04, hb + 0.06), (-L / 2 - 0.04, 0, (hb + 0.06) / 2), coll),
    ]
    marks = []
    if lamp:
        marks.append(c.empty(f"LIGHT_{name}_lamp", (lx + 0.15, ly, hb - 0.3), coll, {"lumens": 220.0, "range": 2.5, "color": WARM, "shadow_candidate": 0}))
    _place(vis + col + marks, center, facing)
    return vis, col, marks


def counter(name: str, center, facing: float, m: dict, coll, length: float = 2.4, *, role: str = "contracts",
            variant: str = "a", screen: bool = True, style: str = "station", depth: float = 0.75, height: float = 1.05,
            npc_gap: float = 0.195):
    """Comptoir d'accueil (M9), client côté +Y, employé (PNJ) côté −Y. Un seul plateau à
    `height` sur toute la profondeur (chant de métal brut côté client), corps à façade en
    panneaux (noirs et jaunes en `station`, tôle claire en `ship`), bandeau lumineux sous le
    surplomb côté client, plinthe de caoutchouc ; pavé de paiement et petit écran incliné vers le
    client (`screen`) ; côté employé, tiroirs et portes de rangement, petit moniteur tourné vers
    le PNJ (posé loin du bord, hors de la zone des mains).

    Cotes du PNJ (lot E de M9, impératives) : le PNJ a son origine sur `SOCKET_<name>_npc` ;
    son corps avance jusqu'à 0,19 m devant le socket sous le plateau, ses mains se posent à
    1,05 m de haut entre 0,21 et 0,31 m devant le socket. D'où : plateau à 1,05 m (`height`),
    bord du plateau côté employé à `npc_gap` = 0,195 m devant le socket (0,19 à 0,20), caisson
    en retrait de 3 cm sous ce bord (à 0,225 m du socket). Tout bar où un PNJ se tient suit les
    mêmes cotes.

    Repères (convention docs/m9-retours.md §3.4) :
    - `SOCKET_<name>_npc` : pied du PNJ, regard vers +Y (le client) ; extras `usage: "npc"`,
      `role`, `variant` (le jeu y pose `models/characters/clerk_<variant>.gltf`) ;
    - `INTERACT_<name>` : `interact: "npc"`, mêmes `role` et `variant`, `half_extents` qui
      couvre le comptoir et le PNJ (≈ 1,8 m), du sol à 1,95 m.
    Le PNJ n'est pas dans le kit. Collider : tout le comptoir (le PNJ a le sien, s'il en faut un)."""
    L, D, Hc = length, depth, height
    station = style == "station"
    facade = m["paint_black"] if station else m["panel"]
    accent = m["paint_yellow"] if station else m["dark"]
    band = m["band_warm"] if station else m["band_cool"]
    yf = D / 2  # face client
    yb = -D / 2  # bord du plateau côté employé
    bb = yb + 0.03  # face du caisson côté employé (en retrait sous le plateau)
    bf = yf - 0.03  # face du caisson côté client (les panneaux de façade vont jusqu'à yf)
    hb = Hc - 0.05  # dessus du caisson, sous le plateau
    vis = [
        _box(f"{name}_body", (L, bf - bb, hb), (0, (bf + bb) / 2, hb / 2), m["dark"], coll),
        _box(f"{name}_kick", (L - 0.02, 0.02, 0.1), (0, bf + 0.01, 0.05), m["rubber"], coll),
        # Plateau unique : bord employé à yb, surplomb de 4 cm côté client, chant de métal brut.
        _box(f"{name}_top", (L + 0.06, D + 0.04, 0.05), (0, 0.02, Hc - 0.025), m["dark"], coll, bevel=0.01),
        _box(f"{name}_top_edge", (L + 0.08, 0.03, 0.035), (0, yf + 0.055, Hc - 0.03), m["bare"], coll, bevel=0.006),
        # Bandeau lumineux sous le surplomb (cuit : il éclaire la façade et le sol).
        _ori(f"{name}_band_housing", (L - 0.1, 0.08, 0.03), (0, yf + 0.002, Hc - 0.11), *_FRONT, m["dark"], coll, bevel=0.005),
        _ori(f"{name}_band", (L - 0.24, 0.035, 0.012), (0, yf + 0.022, Hc - 0.11), *_FRONT, band, coll, ("strip", 1.0)),
    ]
    # Façade : panneaux encadrés, montants d'accent, plaque à pochoirs.
    n = max(2, round(L / 0.8))
    pw = L / n
    for i in range(n):
        x = -L / 2 + pw * (i + 0.5)
        vis.append(_ori(f"{name}_panel{i}", (pw - 0.07, Hc - 0.32, 0.03), (x, yf - 0.015, 0.1 + (Hc - 0.32) / 2 + 0.03), *_FRONT, facade, coll, ("box", (0.37 * i, 0.11 * i)), bevel=0.006))
        if i > 0:
            vis.append(_ori(f"{name}_mullion{i}", (0.06, Hc - 0.26, 0.04), (x - pw / 2, yf - 0.01, 0.1 + (Hc - 0.26) / 2), *_FRONT, accent, coll))
    vis.append(_ori(f"{name}_label", (min(0.6, pw * 0.7), 0.1, 0.004), (-L / 2 + pw * 0.5, yf + 0.002, Hc - 0.32), *_FRONT, m["trim_sheet"], coll, "stencils"))
    for sx in (-1, 1):
        vis.append(_box(f"{name}_end{sx}", (0.05, bf - bb + 0.05, hb), (sx * (L / 2 - 0.025), (bf + bb) / 2 + 0.015, hb / 2), accent, coll, bevel=0.008))
    # Côté client : pavé posé à plat (le haut de la texture vers l'employé), écran incliné.
    vis.append(_ori(f"{name}_keypad", (0.16, 0.12, 0.012), (-L * 0.28, yf - 0.1, Hc + 0.006), (-1, 0, 0), (0, -1, 0), m["buttons"], coll, tile=0.3))
    if screen:
        sxp = L * 0.24
        a = math.radians(25)
        up = (0, -math.sin(a), math.cos(a))
        sy = yf - 0.22
        vis.append(_cyl(f"{name}_scr_post", 0.02, 0.14, (sxp, sy, Hc + 0.07), m["bare"], coll, segments=8))
        vis.append(_ori(f"{name}_scr_housing", (0.36, 0.24, 0.04), (sxp, sy, Hc + 0.2), (-1, 0, 0), up, m["dark"], coll, bevel=0.008))
        n_scr = Vector((0, math.cos(a), math.sin(a)))
        sc = Vector((sxp, sy, Hc + 0.2)) + n_scr * 0.021
        vis.append(_ori(f"{name}_scr", (0.3, 0.18, 0.004), tuple(sc), (-1, 0, 0), up, m["screen"], coll, tile=0.6))
    # Côté employé : tiroirs en haut, portes de rangement en bas, moniteur sur le plateau.
    ndr = max(2, round(L / 0.8))
    dw = (L - 0.12) / ndr
    for i in range(ndr):
        x = -L / 2 + 0.06 + dw * (i + 0.5)
        vis.append(_ori(f"{name}_drawer{i}", (dw - 0.04, 0.16, 0.012), (x, bb - 0.006, hb - 0.12), (1, 0, 0), (0, 0, 1), m["panel"], coll, ("box", (0.2 * i, 0.3)), bevel=0.003))
        vis.append(_box(f"{name}_dpull{i}", (0.14, 0.02, 0.022), (x, bb - 0.018, hb - 0.09), m["bare"], coll))
        vis.append(_ori(f"{name}_door{i}", (dw - 0.04, hb - 0.38, 0.012), (x, bb - 0.006, 0.12 + (hb - 0.38) / 2), (1, 0, 0), (0, 0, 1), m["panel"], coll, ("box", (0.31 * i, 0.6)), bevel=0.003))
        vis.append(_box(f"{name}_dgrip{i}", (0.02, 0.02, 0.14), (x + dw / 2 - 0.08, bb - 0.018, 0.12 + (hb - 0.38) * 0.75), m["bare"], coll))
    vis.append(_box(f"{name}_cab_kick", (L - 0.04, 0.012, 0.1), (0, bb - 0.006, 0.05), m["rubber"], coll))
    mx, my = -L * 0.3, yb + 0.43
    vis.append(_box(f"{name}_mon_base", (0.18, 0.12, 0.02), (mx, my + 0.02, Hc + 0.01), m["dark"], coll))
    vis.append(_box(f"{name}_mon_post", (0.04, 0.03, 0.12), (mx, my + 0.04, Hc + 0.07), m["dark"], coll))
    vis.append(_box(f"{name}_mon", (0.4, 0.04, 0.26), (mx, my, Hc + 0.22), m["dark"], coll, bevel=0.008))
    vis.append(_ori(f"{name}_mon_scr", (0.35, 0.21, 0.004), (mx, my - 0.022, Hc + 0.22), (1, 0, 0), (0, 0, 1), m["screen"], coll, tile=0.7))
    col = [c.collider_box(f"{name}", (L, D, Hc), (0, 0, Hc / 2), coll)]
    npc_y = yb - npc_gap
    back = npc_y - 0.32
    front = yf + 0.06
    props = {"role": role, "variant": variant}
    marks = [
        c.empty(f"SOCKET_{name}_npc", (0, npc_y, 0.0), coll, {"usage": "npc", **props}),
        _interact(name, (0, (front + back) / 2, 0.975), (L, front - back, 1.95), {"interact": "npc", **props}, coll, margin=0.03),
    ]
    _place(vis + col + marks, center, facing)
    return vis, col, marks


def wall_desk(name: str, center, facing: float, m: dict, coll, width: float = 1.2, *, shelf: bool = True, screen: bool = True,
              items_detail: int = 0):
    """Bureau mural (M9) : `center` au pied du mur, la pièce vers +Y. Platine sombre au mur,
    plateau à chant de métal brut sur deux consoles en équerre, caisson à tiroir, petit écran
    mural (`screen`), étagère haute éclairée par-dessous (`shelf`, liseré chaud cuit) et ses
    objets (boîtes, classeurs ; niveau de décor `items_detail`). Colliders : plateau jusqu'au
    sol (on ne se glisse pas dessous), étagère."""
    W = width
    vis = [
        _box(f"{name}_back", (W + 0.04, 0.02, 1.15), (0, 0.01, 0.5 + 1.15 / 2), m["dark"], coll, bevel=0.005),
        _box(f"{name}_top", (W, 0.58, 0.04), (0, 0.31, 0.76), m["dark"], coll, bevel=0.008),
        _box(f"{name}_edge", (W + 0.01, 0.025, 0.045), (0, 0.6, 0.755), m["bare"], coll, bevel=0.005),
        _box(f"{name}_drawer_box", (0.42, 0.5, 0.16), (W / 2 - 0.23, 0.28, 0.66), m["dark"], coll, bevel=0.006),
        _ori(f"{name}_drawer", (0.38, 0.12, 0.012), (W / 2 - 0.23, 0.536, 0.66), *_FRONT, m["panel"], coll, ("box", (0.4, 0.2)), bevel=0.003),
        _box(f"{name}_pull", (0.16, 0.02, 0.022), (W / 2 - 0.23, 0.55, 0.67), m["bare"], coll),
    ]
    for sx in (-1, 1):
        x = sx * (W / 2 - 0.08)
        vis.append(_box(f"{name}_cleat{sx}", (0.05, 0.03, 0.5), (x, 0.035, 0.5), m["bare"], coll))
        vis.append(_strut(f"{name}_brace{sx}", (x, 0.04, 0.32), (x, 0.5, 0.735), 0.04, 0.03, m["dark"], coll))
    if screen:
        vis.append(_box(f"{name}_scr_housing", (0.46, 0.04, 0.3), (-W * 0.15, 0.04, 1.12), m["dark"], coll, bevel=0.008))
        vis.append(_ori(f"{name}_scr", (0.4, 0.24, 0.004), (-W * 0.15, 0.062, 1.12), *_FRONT, m["screen"], coll, tile=0.6))
        vis.append(_ori(f"{name}_buttons", (0.14, 0.2, 0.006), (W * 0.2, 0.023, 1.1), *_FRONT, m["buttons"], coll, tile=0.3))
    col = [c.collider_box(f"{name}", (W, 0.62, 0.8), (0, 0.31, 0.4), coll)]
    items = []
    if shelf:
        zs = 1.48
        vis.append(_box(f"{name}_shelf", (W, 0.26, 0.03), (0, 0.13, zs), m["dark"], coll, bevel=0.006))
        vis.append(_box(f"{name}_shelf_lip", (W, 0.015, 0.05), (0, 0.255, zs + 0.01), m["bare"], coll))
        vis.append(_ori(f"{name}_shelf_light", (W - 0.2, 0.03, 0.008), (0, 0.2, zs - 0.019), (1, 0, 0), (0, -1, 0), m["band_warm"], coll, ("strip", 0.5)))
        for sx in (-1, 1):
            vis.append(_strut(f"{name}_shelf_brace{sx}", (sx * (W / 2 - 0.06), 0.03, zs - 0.2), (sx * (W / 2 - 0.06), 0.22, zs - 0.02), 0.03, 0.025, m["dark"], coll))
        x = -W / 2 + 0.12
        for i, (w_, h_, mat) in enumerate(((0.07, 0.24, "dark"), (0.07, 0.26, "fabric_dark"), (0.06, 0.22, "crate_slate"), (0.25, 0.12, "crate_rust"), (0.18, 0.09, "dark"))):
            items.append(_box(f"{name}_item{i}", (w_, 0.2, h_), (x + w_ / 2, 0.13, zs + 0.015 + h_ / 2), m[mat], coll, bevel=0.006))
            x += w_ + (0.02 if i < 2 else 0.08)
        col.append(c.collider_box(f"{name}_shelf", (W, 0.27, 0.1), (0, 0.135, zs + 0.03), coll))
    if items_detail:
        c.set_detail(items, items_detail)
    vis += items
    _place(vis + col, center, facing)
    return vis, col, []


def shelf(name: str, center, facing: float, m: dict, coll, width: float = 1.0, height: float = 1.8, depth: float = 0.34,
          levels: int = 4, *, items: bool = True, items_detail: int = 0, detail: int = 0):
    """Étagère sur pied adossée au mur (M9), `center` au pied du mur, la pièce vers +Y : joues
    perforées sombres, fond en tôle peinte, tablettes à rebord de métal brut, objets posés
    (caisses, boîtes, bidons ; niveau de décor `items_detail`, pour un Bas plus sobre). `detail` :
    niveau de toute l'étagère (alors sans collider)."""
    W, Hh, D = width, height, depth
    vis = [_ori(f"{name}_back", (W, Hh, 0.02), (0, 0.01, Hh / 2), *_FRONT, m["panel"], coll, ("box", (0.21, 0.4)))]
    for sx in (-1, 1):
        vis.append(_box(f"{name}_side{sx}", (0.04, D, Hh), (sx * (W / 2 - 0.02), D / 2, Hh / 2), m["dark"], coll, bevel=0.006))
        vis.append(_box(f"{name}_foot{sx}", (0.06, D + 0.04, 0.04), (sx * (W / 2 - 0.03), D / 2 + 0.02, 0.02), m["rubber"], coll))
    zs = [0.12 + i * (Hh - 0.2) / (levels - 1) for i in range(levels)]
    stuff = []
    palette_items = ("crate_slate", "dark", "crate_rust", "fabric_dark", "crate", "bare")
    for i, z in enumerate(zs):
        vis.append(_box(f"{name}_board{i}", (W - 0.08, D - 0.03, 0.025), (0, D / 2 + 0.005, z), m["dark"], coll))
        vis.append(_box(f"{name}_lip{i}", (W - 0.08, 0.012, 0.045), (0, D - 0.004, z + 0.01), m["bare"], coll))
        if not items or i == levels - 1 and Hh > 1.9:
            continue
        # Objets : suite déterministe de largeurs et de hauteurs (aucun hasard).
        x = -W / 2 + 0.07
        k = 0
        room = (zs[i + 1] - z - 0.06) if i + 1 < levels else 0.3
        while True:
            w_ = (0.14, 0.22, 0.1, 0.3, 0.18, 0.12)[(i * 2 + k) % 6]
            h_ = min(room, (0.18, 0.12, 0.26, 0.16, 0.22, 0.1)[(i + k * 3) % 6])
            if x + w_ > W / 2 - 0.07:
                break
            mat = m[palette_items[(i + k) % len(palette_items)]]
            if (i + k) % 5 == 4:
                stuff.append(_cyl(f"{name}_can{i}{k}", min(w_, 0.16) / 2, h_, (x + min(w_, 0.16) / 2, D / 2, z + 0.0125 + h_ / 2), mat, coll, segments=10))
            else:
                stuff.append(_box(f"{name}_box{i}{k}", (w_, D - 0.1, h_), (x + w_ / 2, D / 2 + 0.01, z + 0.0125 + h_ / 2), mat, coll, bevel=0.006))
            x += w_ + (0.03 if k % 2 else 0.07)
            k += 1
    if items_detail:
        c.set_detail(stuff, items_detail)
    vis += stuff
    col = [c.collider_box(f"{name}", (W, D, Hh), (0, D / 2, Hh / 2), coll)]
    _place(vis + col, center, facing)
    return _finish(vis, col, [], detail)


def cabin_porthole(name: str, center, facing: float, m: dict, coll, radius: float = 0.22, *, detail: int = 0):
    """Hublot de cabine (M9) : `center` sur la face du mur, à la hauteur du centre, face vers la
    pièce. Couronne sombre boulonnée, joint de caoutchouc, verre fumé opaque en retrait (on ne
    voit pas au travers : rien à charger derrière), plaque à pochoirs dessous. Sans collider
    (saillie < 0,1 m)."""
    r = radius
    vis = [
        _ring(f"{name}_frame", r, r + 0.075, 0.06, (0, 0.03, 0), m["dark"], coll, axis="Y", segments=16),
        _ring(f"{name}_gasket", r - 0.018, r + 0.004, 0.02, (0, 0.065, 0), m["rubber"], coll, axis="Y", segments=16),
        _cyl(f"{name}_glass", r, 0.02, (0, 0.018, 0), m["glass_dark"], coll, axis="Y", segments=16),
    ]
    for i in range(8):
        a = 2 * math.pi * (i + 0.5) / 8
        vis.append(_cyl(f"{name}_bolt{i}", 0.011, 0.016, ((r + 0.04) * math.cos(a), 0.066, (r + 0.04) * math.sin(a)), m["bare"], coll, axis="Y", segments=6))
    vis.append(_ori(f"{name}_label", (0.22, 0.06, 0.004), (0, 0.003, -r - 0.13), *_FRONT, m["trim_sheet"], coll, "stencils"))
    _place(vis, center, facing)
    return _finish(vis, [], [], detail)


def crate(name: str, center, facing: float, m: dict, coll, size=(0.9, 0.9, 0.9), variant: str = "olive", *, detail: int = 0):
    """Caisse de tôle nervurée à cornières sombres, cerclages et étiquette à pochoirs ;
    `variant` : "olive", "slate", "rust", "yellow". `detail` (M9) : niveau de décor (≥ 1 : sans
    collider), pour toutes les pièces d'habillage ci-dessous aussi."""
    sx, sy, sz = size
    mat = m["crate"] if variant == "olive" else m[f"crate_{variant}"]
    vis = [_box(f"{name}", (sx - 0.04, sy - 0.04, sz - 0.04), (0, 0, sz / 2), mat, coll, bevel=0.008)]
    for dx in (-1, 1):
        for dy in (-1, 1):
            vis.append(_box(f"{name}_post{dx}{dy}", (0.06, 0.06, sz), (dx * (sx / 2 - 0.03), dy * (sy / 2 - 0.03), sz / 2), m["dark"], coll, bevel=0.006))
    for zr in (0.03, sz - 0.03):
        vis.append(_box(f"{name}_rim{zr:.2f}", (sx - 0.008, sy - 0.008, 0.06), (0, 0, zr), m["dark"], coll, bevel=0.006))
    lw = min(sx, sy) * 0.55
    vis.append(_ori(f"{name}_label", (lw, 0.12, 0.004), (0, sy / 2 - 0.018, sz * 0.55), *_FRONT, m["trim_sheet"], coll, "stencils"))
    vis.append(_ori(f"{name}_hazard", (sy * 0.5, 0.1, 0.004), (sx / 2 - 0.018, 0, sz * 0.3), (0, 1, 0), (0, 0, 1), m["trim_sheet"], coll, "hazard"))
    col = [c.collider_box(f"{name}", (sx, sy, sz), (0, 0, sz / 2), coll)]
    _place(vis + col, center, facing)
    return _finish(vis, col, [], detail)


def locker(name: str, center, facing: float, m: dict, coll, both_sides: bool = True, *, container: str | None = None):
    """Casier : caisson sombre, porte peinte à grilles, poignée, pochoir. `both_sides` : une porte
    sur chaque face ±Y (les scripts de M1 l'adossent indifféremment par l'une ou l'autre).
    `container` (M9) : identifiant unique du contenu (ex. « corvette_chambre_casier_1 ») ; le
    casier devient interactif (`INTERACT_<name>`, `interact: "locker"`, `container`, volume
    qui couvre tout le casier). None : décor (comportement de M8)."""
    vis = [_box(f"{name}", (0.6, 0.51, 2.0), (0, 0, 1.0), m["dark"], coll, bevel=0.015)]
    for sy in ((1, -1) if both_sides else (1,)):
        right = (-sy, 0, 0)
        y = sy * 0.255
        vis.append(_ori(f"{name}_door{sy}", (0.52, 1.86, 0.022), (0, y + sy * 0.011, 1.0), right, (0, 0, 1), m["panel"], coll, ("box", (0.3 if sy > 0 else 0.7, 0.2))))
        for i, z in enumerate((1.7, 0.32)):
            vis.append(_ori(f"{name}_vent{sy}{i}", (0.42, 0.16, 0.006), (0, y + sy * 0.025, z), right, (0, 0, 1), m["trim_sheet"], coll, "vent"))
        vis.append(_ori(f"{name}_label{sy}", (0.36, 0.1, 0.004), (0, y + sy * 0.024, 1.45), right, (0, 0, 1), m["trim_sheet"], coll, "stencils"))
        vis.append(_box(f"{name}_handle{sy}", (0.03, 0.03, 0.2), (sy * -0.18, y + sy * 0.04, 1.05), m["bare"], coll))
        vis.append(_ori(f"{name}_lock{sy}", (0.06, 0.08, 0.006), (sy * -0.18, y + sy * 0.024, 1.22), right, (0, 0, 1), m["buttons"], coll))
    col = [c.collider_box(f"{name}", (0.6, 0.55, 2.0), (0, 0, 1.0), coll)]
    marks = []
    if container:
        marks.append(_interact(name, (0, 0, 1.0), (0.6, 0.55, 2.0), {"interact": "locker", "container": container}, coll, margin=0.03))
    _place(vis + col + marks, center, facing)
    return vis, col, marks


def locker_bank(name: str, center, facing: float, m: dict, coll, count: int = 3, *, containers=None,
                width: float = 0.6, depth: float = 0.55, height: float = 2.0, tint: str = "panel"):
    """Rangée de `count` casiers verticaux (M9), portes vers +Y, `center` au milieu de la base de
    la rangée (adossable à un mur par −Y). Bâti commun sombre (socle, couronne, montants),
    chaque porte peinte avec ses charnières, ses deux grilles, son numéro au pochoir, sa
    serrure à voyant et sa poignée ; un collider pour la rangée.

    Chaque casier est interactif : `INTERACT_<name>_<i>` (i = 1…count), `interact: "locker"`,
    `container` et `half_extents` (le casier, de la face avant au mur). `containers` : liste
    d'identifiants (un par casier), ou préfixe (« corvette_chambre_casier » →
    « corvette_chambre_casier_1 »…) ; None : « <name>_<i> ». `tint` : clé de palette des portes."""
    if isinstance(containers, str) or containers is None:
        prefix = containers or name
        containers = [f"{prefix}_{i + 1}" for i in range(count)]
    if len(containers) != count:
        raise ValueError(f"locker_bank {name} : {count} casiers mais {len(containers)} contenus")
    W = width * count
    vis = [
        _box(f"{name}_body", (W, depth - 0.03, height - 0.12), (0, -0.015, 0.1 + (height - 0.12) / 2), m["dark"], coll),
        _box(f"{name}_plinth", (W - 0.04, depth - 0.1, 0.1), (0, -0.04, 0.05), m["rubber"], coll),
        _box(f"{name}_crown", (W + 0.03, depth + 0.02, 0.06), (0, 0.0, height - 0.03), m["dark"], coll, bevel=0.01),
        _ori(f"{name}_crown_label", (W * 0.8, 0.045, 0.004), (0, depth / 2 + 0.012, height - 0.03), *_FRONT, m["trim_sheet"], coll, "rivets"),
    ]
    marks = []
    yf = depth / 2 - 0.015  # face avant du bâti
    door_mat = m[tint]
    for i in range(count):
        x = -W / 2 + width * (i + 0.5)
        # Montant entre deux portes (et aux deux bouts).
        vis.append(_box(f"{name}_stile{i}", (0.04, 0.03, height - 0.18), (x - width / 2 + 0.02, yf + 0.015, 0.1 + (height - 0.18) / 2 + 0.03), m["dark"], coll))
        dz = 0.1 + (height - 0.2) / 2 + 0.02
        dh = height - 0.24
        dw = width - 0.07
        vis.append(_ori(f"{name}_door{i}", (dw, dh, 0.022), (x, yf + 0.011, dz), *_FRONT, door_mat, coll, ("box", (0.31 * i, 0.17 * i)), bevel=0.004))
        df = yf + 0.022
        for j, z in enumerate((dz + dh * 0.36, dz - dh * 0.38)):
            vis.append(_ori(f"{name}_vent{i}{j}", (dw * 0.72, 0.15, 0.006), (x, df + 0.003, z), *_FRONT, m["trim_sheet"], coll, "vent"))
        # Numéro au pochoir : chaque porte montre un autre caractère de la bande (U calé sur
        # le début d'un groupe : « 07 », « 12 », « B4 », « 03 », « A2 »).
        lab = _ori(f"{name}_label{i}", (0.2, 0.1, 0.004), (x - dw * 0.18, df + 0.002, dz + dh * 0.18), *_FRONT, m["trim_sheet"], coll, None)
        c.uv_trim(lab, "stencils", meters_per_tile=2.0)
        u0 = min(d.uv[0] for d in lab.data.uv_layers[0].data)
        _shift_uv(lab, _STENCIL_STARTS[i % len(_STENCIL_STARTS)] - u0)
        hx = x + dw / 2 - 0.08  # côté poignée (charnières de l'autre côté)
        vis.append(lab)
        vis.append(_box(f"{name}_handle{i}", (0.03, 0.035, 0.22), (hx, df + 0.025, dz), m["bare"], coll, bevel=0.006))
        vis.append(_ori(f"{name}_lock{i}", (0.05, 0.07, 0.006), (hx, df + 0.003, dz + 0.2), *_FRONT, m["buttons"], coll, tile=0.3))
        for k, z in enumerate((dz + dh / 2 - 0.18, dz - dh / 2 + 0.18)):
            vis.append(_box(f"{name}_hinge{i}{k}", (0.025, 0.03, 0.12), (x - dw / 2 + 0.01, df + 0.008, z), m["bare"], coll))
        marks.append(_interact(f"{name}_{i + 1}", (x, 0.02, height / 2), (width - 0.02, depth + 0.04, height),
                               {"interact": "locker", "container": containers[i]}, coll, margin=0.0))
    vis.append(_box(f"{name}_stile_end", (0.04, 0.03, height - 0.18), (W / 2 - 0.02, yf + 0.015, 0.1 + (height - 0.18) / 2 + 0.03), m["dark"], coll))
    col = [c.collider_box(f"{name}", (W, depth, height), (0, 0, height / 2), coll)]
    _place(vis + col + marks, center, facing)
    return vis, col, marks


# Début (en U, tuile de 2 m) de groupes de caractères de la bande `stencils` de la trim sheet.
_STENCIL_STARTS = (0.022, 0.522, 0.772, 0.892, 0.272)


def _shift_uv(obj, du: float) -> None:
    """Décale le 1er jeu d'UV le long de U (`du` en unités de texture)."""
    layer = obj.data.uv_layers[0]
    for d in layer.data:
        d.uv = (d.uv[0] + du, d.uv[1])


def table(name: str, center, facing: float, m: dict, coll, size=(1.8, 0.9)):
    """Table (ou comptoir, s'il est long) : plateau sombre à chant de métal brut, pieds à
    platine, un pied tous les 1,6 m."""
    sx, sy = size
    vis = [
        _box(f"{name}_top", (sx, sy, 0.05), (0, 0, 0.765), m["dark"], coll, bevel=0.012),
        _box(f"{name}_edge", (sx + 0.02, sy + 0.02, 0.025), (0, 0, 0.73), m["bare"], coll),
    ]
    n = max(1, int(sx // 1.6))
    for i in range(n):
        x = (i - (n - 1) / 2) * (sx / n)
        vis.append(_box(f"{name}_leg{i}", (0.14, max(0.2, sy * 0.45), 0.7), (x, 0, 0.37), m["dark"], coll))
        vis.append(_box(f"{name}_foot{i}", (0.5, min(sy * 0.85, 0.9), 0.04), (x, 0, 0.02), m["dark"], coll, bevel=0.008))
        vis.append(_box(f"{name}_bracket{i}", (0.3, sy * 0.7, 0.03), (x, 0, 0.705), m["bare"], coll))
    col = [c.collider_box(f"{name}", (sx, sy, 0.8), (0, 0, 0.4), coll)]
    _place(vis + col, center, facing)
    return vis, col, []


def pickup(name: str, center, facing: float, m: dict, coll, item: str):
    """Objet ramassable provisoire : petite mallette jaune à joint sombre et voyant (repérable
    dans la pénombre), posée sur la surface (`center`). Le repère `INTERACT_<name>`
    (`interact: "pickup"`, `item`) est 0,15 m au-dessus de la surface : le jeu y remplace la
    mallette par le vrai modèle de l'objet (`models/items/<item>.gltf`, posé 0,15 m sous le
    repère) et la cache ; dans le bake, tout ce qui est sous ce repère ne fait pas d'ombre
    (`lightmap.removable`). M9 : réduite (≈ 50 triangles), c'est un repère, pas un modèle."""
    vis = [
        _box(f"{name}", (0.26, 0.17, 0.1), (0, 0, 0.05), m["accent"], coll, bevel=0.012),
        _box(f"{name}_seam", (0.265, 0.175, 0.01), (0, 0, 0.072), m["dark"], coll),
        _ori(f"{name}_light", (0.06, 0.03, 0.004), (0, 0.0875, 0.04), *_FRONT, m["buttons"], coll, tile=0.25),
    ]
    marks = [c.empty(f"INTERACT_{name}", (0, 0, 0.15), coll, {"interact": "pickup", "item": item})]
    _place(vis + marks, center, facing)
    return vis, [], marks


def attach_pickup(result):
    """Prépare un objet ramassable pour l'export (M9) : les pièces visibles de `pickup(...)`
    fusionnées en un seul objet (plusieurs matériaux) `<nom>_mesh`, **enfant** de son repère
    `INTERACT_<nom>` (transformation monde conservée). Le jeu remplace ces enfants par le modèle
    de l'objet, et le bake (`lightmap.removable`) ne leur fait pas projeter d'ombre. À mettre dans
    `parts["pickups"]` (jamais dans `visual` : il ne doit pas être fusionné avec la cellule).
    Renvoie (objet, repères)."""
    vis, _, marks = result
    root = next(mk for mk in marks if mk.name.startswith("INTERACT_"))
    bpy.context.view_layer.update()
    bpy.ops.object.select_all(action="DESELECT")
    for o in vis:
        o.select_set(True)
    bpy.context.view_layer.objects.active = vis[0]
    if len(vis) > 1:
        bpy.ops.object.join()
    joined = bpy.context.view_layer.objects.active
    joined.name = f"{root.name[len('INTERACT_'):]}_mesh"
    joined.data.name = joined.name
    bpy.context.view_layer.update()
    world = joined.matrix_world.copy()
    joined.parent = root
    joined.matrix_world = world
    return joined, marks


def reactor(name: str, center, m: dict, coll, *, ceiling: float | None = None):
    """Réacteur : socle octogonal cerclé de bandes de danger, cœur lumineux (`sh_band_reactor`,
    cuit) entre des douelles blindées, trois cerclages, chapiteau, deux gros tuyaux à brides
    vers ±Y ; `ceiling` (hauteur du plafond au-dessus de la base) : colonnes montantes."""
    vis = []
    plinth = _cyl(f"{name}_plinth", 1.24, 0.3, (0, 0, 0.15), m["dark"], coll, segments=8)
    plinth.rotation_euler[2] = math.pi / 8
    vis.append(plinth)
    vis.append(_cyl(f"{name}_plinth_band", 1.2, 0.1, (0, 0, 0.25), m["trim_sheet"], coll, segments=24, uv=("wrap", "hazard", 1.0)))
    vis.append(_cyl(f"{name}_core", 0.8, 2.4, (0, 0, 1.55), m["band_reactor"], coll, segments=16, uv=("wrap", (0.0, 1.0), 0.6)))
    n = 10
    for i in range(n):
        t = 2 * math.pi * i / n
        ct, st_ = math.cos(t), math.sin(t)
        vis.append(_ori(f"{name}_stave{i}", (0.24, 2.3, 0.1), (0.9 * ct, 0.9 * st_, 1.55), (-st_, ct, 0), (0, 0, 1), m["bare"], coll, ("box", (0.13 * i, 0.0))))
    for zr, mat in ((0.55, m["dark"]), (1.55, m["paint_yellow"]), (2.55, m["dark"])):
        vis.append(_cyl(f"{name}_ring{zr:.2f}", 1.04, 0.14, (0, 0, zr), mat, coll, segments=24))
    vis.append(_cyl(f"{name}_cap", 1.05, 0.2, (0, 0, 2.85), m["dark"], coll, segments=24))
    vis.append(_cyl(f"{name}_dome", 0.7, 0.18, (0, 0, 3.04), m["bare"], coll, segments=20))
    vis.append(_cyl(f"{name}_top", 0.3, 0.12, (0, 0, 3.19), m["dark"], coll, segments=12))
    # Panneau de commande sur un pan du socle (face +Y), pochoirs.
    vis.append(_ori(f"{name}_panel", (0.5, 0.16, 0.01), (0, 1.146 + 0.005, 0.12), *_FRONT, m["buttons"], coll))
    for sy in (-1, 1):
        y0, y1 = sy * 0.95, sy * 2.6
        vis.append(_cyl(f"{name}_pipe{sy}", 0.15, abs(y1 - y0), (0, (y0 + y1) / 2, 2.75), m["trim_sheet"], coll, axis="Y", segments=16, uv="flange"))
        vis.append(_cyl(f"{name}_flange{sy}", 0.21, 0.06, (0, sy * 1.0, 2.75), m["dark"], coll, axis="Y", segments=16))
        vis.append(_cyl(f"{name}_elbow{sy}", 0.19, 0.4, (0, y1, 2.75), m["dark"], coll, axis="Y", segments=16))
        top = ceiling if ceiling is not None else 3.4
        if top - 2.95 > 0.05:
            vis.append(_cyl(f"{name}_riser{sy}", 0.15, top - 2.75, (0, y1, 2.75 + (top - 2.75) / 2), m["trim_sheet"], coll, segments=16, uv="flange"))
    col = [c.collider_box(f"{name}", (2.5, 2.5, 3.2), (0, 0, 1.6), coll)]
    _place(vis + col, center, 0.0)
    return vis, col, []


def ceiling_light(name: str, center, facing: float, m: dict, coll, props: dict, *, lens: str = "light", drop: float = 0.0):
    """Plafonnier (carter, diffuseur `sh_light*`, grille) + repère LIGHT_ (le jeu crée la vraie
    lumière ; le bake le remplace par sa lampe). `lens` : "light" (froid) ou "light_amber" ;
    `drop` > 0 : suspendu par deux tiges, sous une poutre (le repère descend d'autant)."""
    z = -drop
    vis = [
        _box(f"{name}_housing", (0.9, 0.26, 0.07), (0, 0, z - 0.035), m["dark"], coll, bevel=0.01),
        _box(f"{name}_lens", (0.78, 0.16, 0.02), (0, 0, z - 0.075), m[lens], coll),
    ]
    for i, x in enumerate((-0.26, 0.0, 0.26)):
        vis.append(_box(f"{name}_guard{i}", (0.02, 0.24, 0.015), (x, 0, z - 0.092), m["dark"], coll))
    for sy in (-1, 1):
        vis.append(_box(f"{name}_lip{sy}", (0.86, 0.02, 0.045), (0, sy * 0.12, z - 0.085), m["dark"], coll))
    if drop > 0.0:
        for sx in (-1, 1):
            vis.append(_box(f"{name}_rod{sx}", (0.02, 0.02, drop), (sx * 0.35, 0, -drop / 2), m["dark"], coll))
    marks = [c.empty(f"LIGHT_{name}", (0, 0, -0.25 - drop), coll, props)]
    _place(vis + marks, center, facing)
    return vis, [], marks


def stairs(name: str, x0: float, x1: float, y0: float, rise: float, steps: int, m: dict, coll):
    """Marches montant vers +Y à partir de y0, sur la largeur [x0, x1]. Une boîte de collision
    par marche : le contrôleur du joueur monte les marches (pas d'une rampe invisible).
    Marches sombres, giron en tôle larmée, contremarches à bandes de danger."""
    vis, col = [], []
    tread = 0.3
    step_h = rise / steps
    cx = (x0 + x1) / 2
    for i in range(steps):
        h = step_h * (i + 1)
        center = (cx, y0 + tread * (i + 0.5), h / 2)
        size = (x1 - x0, tread, h)
        vis.append(_box(f"{name}_step{i}", (x1 - x0, tread, h - 0.012), (cx, center[1], (h - 0.012) / 2), m["dark"], coll))
        vis.append(_box(f"{name}_tread{i}", (x1 - x0 - 0.04, tread - 0.02, 0.012), (cx, center[1] + 0.01, h - 0.006), m["floor_plate"], coll))
        vis.append(_ori(f"{name}_riser{i}", (x1 - x0 - 0.04, step_h - 0.03, 0.006), (cx, y0 + tread * i - 0.003, h - step_h / 2), (1, 0, 0), (0, 0, 1), m["trim_sheet"], coll, "hazard"))
        col.append(c.collider_box(f"{name}_step{i}", size, center, coll))
    return vis, col, []


def ladder(name: str, center, facing: float, height: float, m: dict, coll):
    """Échelle contre une paroi (le mur est côté −Y) : montants sombres, barreaux de métal brut,
    pattes de fixation, têtes de montants jaunes."""
    vis = []
    for sx in (-1, 1):
        vis.append(_box(f"{name}_rail{sx}", (0.05, 0.05, height), (sx * 0.22, 0, height / 2), m["dark"], coll))
        vis.append(_box(f"{name}_cap{sx}", (0.06, 0.06, 0.15), (sx * 0.22, 0, height + 0.075), m["paint_yellow"], coll))
        for z in (0.3, height - 0.3):
            vis.append(_box(f"{name}_bracket{sx}{z:.1f}", (0.04, 0.14, 0.04), (sx * 0.22, -0.08, z), m["dark"], coll))
    z = 0.3
    i = 0
    while z < height - 0.1:
        vis.append(_cyl(f"{name}_rung{i}", 0.016, 0.44, (0, 0, z), m["bare"], coll, axis="X", segments=8))
        z += 0.3
        i += 1
    marks = [c.empty(f"INTERACT_{name}", (0, 0.2, 1.2), coll, {"interact": "ladder", "height": height})]
    _place(vis + marks, center, facing)
    return vis, [], marks


def door_panel(name: str, m: dict, coll, *, style: str = "ship"):
    """Panneau de porte coulissante, origine au centre de sa base, DOOR_W × DOOR_H, 0,12 m
    d'épaisseur au plus. Le jeu en place un à chaque portail de type porte et l'anime (il
    coulisse vers +X dans la cloison). Objet mobile sans lightmap, éclairé par l'ambiante en
    Bas et Moyen : relief, teintes moyennes et accents émissifs (bandeau, écran, voyants) pour
    qu'il se lise. Deux faces identiques (±Y). `style` : "ship" ou "station" (jaune et noir).

    M9 : plus d'écran ni de boutons sur le vantail (le boîtier de commande posé à côté de la
    porte par `room_shell` suffit). Caisson haut : hublot de verre fumé opaque (un bloc qui
    traverse le vantail, dans son joint), trappe de visite, plaque à pochoirs ; traverse
    médiane lumineuse ; caisson bas : raidisseurs rivés, grille ; montant d'attaque (−X) à bande
    de danger et voyant. Dimensions, collider et fonctionnement inchangés."""
    W, H = DOOR_W, DOOR_H
    paint = m["door_paint"] if style == "ship" else m["paint_yellow"]
    rail = m["dark"] if style == "ship" else m["paint_black"]
    # Hublot : centre (wx, wz), ouverture ww × wh, à la même place vue des deux faces.
    wx, wz, ww, wh = 0.3, 1.6, 0.14, 0.36
    vis = [
        _box(f"{name}_panel", (W, 0.06, H), (0, 0, H / 2), paint, coll, bevel=0.008),
        # Joint de caoutchouc sur le bord d'attaque (−X : le bord qui se ferme contre le montant).
        _box(f"{name}_gasket", (0.03, 0.064, H - 0.02), (-W / 2 + 0.014, 0, H / 2), m["rubber"], coll),
        # Verre fumé : dépasse de 4 mm de chaque face du vantail.
        _box(f"{name}_window", (ww, 0.068, wh), (wx, 0, wz), m["glass_dark"], coll),
    ]
    for sy in (1, -1):
        right = (-sy, 0, 0)
        nrm = Vector((0, sy, 0))

        def plate(tag, sx_, sz_, x, z, mat, uv="box", th=0.012, tile=None, bevel=0.0):
            vis.append(_ori(f"{name}_{tag}{sy}", (sx_, sz_, th), (x, sy * (0.03 + th / 2), z), right, (0, 0, 1), mat, coll, uv, tile=tile, bevel=bevel))

        # Cadre : montants, traverse haute rivée, plinthe à bandes de danger.
        plate("stile_l", 0.1, H, -W / 2 + 0.05, H / 2, rail, th=0.018)
        plate("stile_r", 0.1, H, W / 2 - 0.05, H / 2, rail, th=0.018)
        plate("top", W - 0.2, 0.12, 0, H - 0.06, m["trim_sheet"], "rivets", th=0.018)
        plate("kick", W - 0.2, 0.22, 0, 0.11, m["trim_sheet"], "hazard", th=0.018)
        # Traverse médiane et son bandeau lumineux (repère dans la pénombre).
        plate("channel", W - 0.2, 0.13, 0, 1.12, rail, th=0.014)
        plate("strip", W - 0.36, 0.05, 0, 1.12, m["trim_sheet"], "light_strip", th=0.022)
        # Montant d'attaque : bande de danger verticale (caisson bas) et voyant d'état (haut).
        plate("lead_hazard", 0.06, 0.8, -W / 2 + 0.05, 0.64, m["trim_sheet"], "hazard", th=0.024)
        plate("lead_lamp", 0.04, 0.24, -W / 2 + 0.05, 1.7, m["trim_sheet"], "light_strip", th=0.024)
        # Caisson haut : joint du hublot, trappe de visite encadrée et sa poignée, pochoirs.
        for tag, sx_, sz_, x, z in (
            ("win_t", ww + 0.07, 0.035, wx, wz + wh / 2 + 0.0175),
            ("win_b", ww + 0.07, 0.035, wx, wz - wh / 2 - 0.0175),
            ("win_l", 0.035, wh, wx - ww / 2 - 0.0175, wz),
            ("win_r", 0.035, wh, wx + ww / 2 + 0.0175, wz),
        ):
            plate(tag, sx_, sz_, x, z, m["rubber"], th=0.016)
        plate("hatch_frame", 0.4, 0.46, -0.2, 1.58, rail, th=0.01, bevel=0.004)
        plate("hatch", 0.32, 0.38, -0.2, 1.58, paint, ("box", (0.37, 0.21)), th=0.016, bevel=0.005)
        plate("hatch_grip", 0.14, 0.025, -0.2, 1.43, m["bare"], th=0.026)
        plate("label", 0.6, 0.1, 0.02, 1.94, m["trim_sheet"], "stencils", th=0.008)
        # Caisson bas : trois raidisseurs rivés, grille d'aération.
        for i, z in enumerate((0.62, 0.8, 0.98)):
            plate(f"rib{i}", 0.76, 0.06, 0.0, z, m["trim_sheet"], "rivets", th=0.016)
        plate("vent", 0.7, 0.16, 0.05, 0.38, m["trim_sheet"], "vent", th=0.006)
        plate("grip", 0.04, 0.42, -0.5 * sy, 0.78, m["bare"], th=0.028)
        plate("grip_plate", 0.1, 0.5, -0.5 * sy, 0.78, rail, th=0.008)
        # Goussets à 45° dans les quatre coins : le panneau se lit à pans coupés.
        for gx, gz, dx, dz in ((-1, 1, 1, 1), (1, 1, -1, 1), (-1, -1, 1, -1), (1, -1, -1, -1)):
            diag = Vector((dx, 0, -dz)).normalized()
            up = nrm.cross(diag)
            ctr = (gx * (W / 2 - 0.15), sy * (0.03 + 0.011), H - 0.17 if gz > 0 else 0.3)
            vis.append(_ori(f"{name}_gusset{sy}{gx}{gz}", (0.22, 0.08, 0.022), ctr, tuple(diag), tuple(up), rail, coll))
    col = [c.collider_box(f"{name}", (DOOR_W, 0.12, DOOR_H), (0, 0, DOOR_H / 2), coll)]
    return vis, col, []


# --- Habillage ---------------------------------------------------------------------------------
# Éléments de remplissage des pièces (lots B et C) : même triplet (visibles, colliders, repères).
# Les éléments linéaires (p0 → p1) suivent un axe (X, Y ou Z) ; les éléments muraux prennent
# `center` sur la face du mur (à la hauteur de leur centre) et `facing` vers la pièce.


def _span(p0, p1):
    p0, p1 = Vector(p0), Vector(p1)
    d = p1 - p0
    ax = max(range(3), key=lambda i: abs(d[i]))
    return ax, abs(d[ax]), (p0 + p1) / 2


def _auto_col(name, objs, coll, cols, force=None):
    """Collider englobant si l'objet descend sous HEAD_CLEAR (ou si `force`)."""
    if force is False:
        return
    bpy.context.view_layer.update()
    lo = Vector((1e9, 1e9, 1e9))
    hi = Vector((-1e9, -1e9, -1e9))
    for o in objs:
        for corner in o.bound_box:
            p = o.matrix_world @ Vector(corner)
            lo = Vector(map(min, lo, p))
            hi = Vector(map(max, hi, p))
    if force or lo.z < HEAD_CLEAR:
        size = hi - lo
        cols.append(c.collider_box(name, tuple(max(0.02, s) for s in size), tuple((lo + hi) / 2), coll))


def cable_tray(name: str, p0, p1, m: dict, coll, width: float = 0.34, hang_to: float | None = None, *, detail: int = 0):
    """Chemin de câbles horizontal (faisceaux colorés de la trim sheet), rebords sombres,
    étriers tous les 1 m, suspentes jusqu'à `hang_to` (hauteur du plafond)."""
    ax, L, mid = _span(p0, p1)
    along = (L, width, 0.06) if ax == 0 else (width, L, 0.06)
    vis = [_box(f"{name}", along, tuple(mid), m["trim_sheet"], coll, "cables")]
    for s in (-1, 1):
        off = Vector((0, s * (width / 2 + 0.01), 0.01)) if ax == 0 else Vector((s * (width / 2 + 0.01), 0, 0.01))
        size = (L, 0.02, 0.08) if ax == 0 else (0.02, L, 0.08)
        vis.append(_box(f"{name}_lip{s}", size, tuple(mid + off), m["dark"], coll))
    n = max(1, int(L // 1.0))
    for i in range(n):
        t = -L / 2 + (i + 0.5) * L / n
        p = mid + (Vector((t, 0, 0)) if ax == 0 else Vector((0, t, 0)))
        size = (0.04, width + 0.08, 0.1) if ax == 0 else (width + 0.08, 0.04, 0.1)
        vis.append(_box(f"{name}_strap{i}", size, tuple(p + Vector((0, 0, 0.02))), m["dark"], coll))
        if hang_to is not None and hang_to - p.z > 0.1:
            hz = hang_to - (p.z + 0.07)
            for s in (-1, 1):
                off = Vector((0, s * (width / 2 + 0.03), 0)) if ax == 0 else Vector((s * (width / 2 + 0.03), 0, 0))
                vis.append(_box(f"{name}_hanger{i}{s}", (0.02, 0.02, hz), tuple(p + off + Vector((0, 0, 0.07 + hz / 2))), m["dark"], coll))
    cols = []
    _auto_col(name, vis[:1], coll, cols, force=False if detail else None)
    return _finish(vis, cols, [], detail)


def pipe(name: str, p0, p1, m: dict, coll, radius: float = 0.05, clamps: float = 1.0, hang_to: float | None = None, mat: str | None = None,
         *, detail: int = 0):
    """Tuyau droit à brides (bande `flange` de la trim sheet : brides tous les 1 m, collier
    jaune), brides aux deux bouts, colliers tous les `clamps` m ; suspentes jusqu'à `hang_to`
    pour un tuyau horizontal. `mat` : clé de la palette pour un tuyau uni (projection boîte)."""
    ax, L, mid = _span(p0, p1)
    axis = "XYZ"[ax]
    seg = 16 if radius > 0.06 else 12
    body = _cyl(f"{name}", radius, L, tuple(mid), m[mat] if mat else m["trim_sheet"], coll, axis=axis, segments=seg, uv="box" if mat else "flange")
    vis = [body]
    unit = Vector([1.0 if i == ax else 0.0 for i in range(3)])
    for s in (-1, 1):
        vis.append(_cyl(f"{name}_end{s}", radius * 1.45, 0.04, tuple(mid + unit * s * (L / 2 - 0.02)), m["dark"], coll, axis=axis, segments=seg))
    if clamps:
        n = max(1, int(L // clamps))
        for i in range(n):
            t = -L / 2 + (i + 0.5) * L / n
            p = mid + unit * t
            size = [radius * 2.5] * 3
            size[ax] = 0.04
            vis.append(_box(f"{name}_clamp{i}", tuple(size), tuple(p), m["dark"], coll))
            if hang_to is not None and ax != 2 and hang_to - p.z > radius + 0.05:
                hz = hang_to - p.z - radius
                vis.append(_box(f"{name}_hanger{i}", (0.025, 0.025, hz), tuple(p + Vector((0, 0, radius + hz / 2))), m["dark"], coll))
    cols = []
    _auto_col(name, [body], coll, cols, force=False if detail else None)
    return _finish(vis, cols, [], detail)


def duct(name: str, p0, p1, m: dict, coll, size=(0.55, 0.36), vents: bool = True, hang_to: float | None = None, *, detail: int = 0):
    """Gaine de ventilation horizontale en tôle brute : joints tous les 1 m, grilles dessous
    tous les 2 m, suspentes jusqu'à `hang_to`."""
    ax, L, mid = _span(p0, p1)
    w, h = size
    dims = (L, w, h) if ax == 0 else (w, L, h)
    vis = [_box(f"{name}", dims, tuple(mid), m["bare"], coll)]
    n = max(1, int(L // 1.0))
    unit = Vector((1, 0, 0)) if ax == 0 else Vector((0, 1, 0))
    for i in range(n + 1):
        t = -L / 2 + i * L / n
        t = max(-L / 2 + 0.025, min(L / 2 - 0.025, t))
        jd = (0.05, w + 0.04, h + 0.04) if ax == 0 else (w + 0.04, 0.05, h + 0.04)
        vis.append(_box(f"{name}_joint{i}", jd, tuple(mid + unit * t), m["dark"], coll))
        if vents and i < n and i % 2 == 0:
            p = mid + unit * (t + L / n / 2) - Vector((0, 0, h / 2 + 0.005))
            vis.append(_ori(f"{name}_vent{i}", (min(0.5, w - 0.05), 0.25, 0.01), tuple(p), tuple(unit.cross(Vector((0, 0, 1))) * -1), tuple(unit), m["trim_sheet"], coll, "vent"))
        if hang_to is not None and hang_to - (mid.z + h / 2) > 0.05:
            hz = hang_to - (mid.z + h / 2)
            for s in (-1, 1):
                off = unit.cross(Vector((0, 0, 1))) * s * (w / 2 + 0.03)
                vis.append(_box(f"{name}_hanger{i}{s}", (0.025, 0.025, hz), tuple(mid + unit * t + off + Vector((0, 0, h / 2 + hz / 2))), m["dark"], coll))
    cols = []
    _auto_col(name, vis[:1], coll, cols, force=False if detail else None)
    return _finish(vis, cols, [], detail)


def junction_box(name: str, center, facing: float, m: dict, coll, size=(0.45, 0.16, 0.55), *, detail: int = 0):
    """Boîtier de jonction mural : caisson sombre, boutons et voyants, pochoir, grille basse,
    conduit qui monte. `center` : sur le mur, à la hauteur du centre du boîtier."""
    sx, sd, sz = size
    vis = [
        _box(f"{name}", (sx, sd, sz), (0, sd / 2, 0), m["dark"], coll, bevel=0.012),
        _ori(f"{name}_buttons", (sx * 0.78, sz * 0.42, 0.01), (0, sd + 0.004, sz * 0.12), *_FRONT, m["buttons"], coll),
        _ori(f"{name}_label", (sx * 0.7, 0.08, 0.004), (0, sd + 0.002, sz * 0.42), *_FRONT, m["trim_sheet"], coll, "stencils"),
        _ori(f"{name}_vent", (sx * 0.7, 0.1, 0.006), (0, sd + 0.003, -sz * 0.33), *_FRONT, m["trim_sheet"], coll, "vent"),
        _cyl(f"{name}_conduit", 0.025, 0.45, (sx * 0.25, sd * 0.5, sz / 2 + 0.225), m["trim_sheet"], coll, segments=10, uv="flange"),
    ]
    cols = []
    if sd > 0.1:
        cols.append(c.collider_box(f"{name}", (sx + 0.04, sd + 0.04, sz + 0.04), (0, sd / 2, 0), coll))
    _place(vis + cols, center, facing)
    if cols and center[2] - sz / 2 >= HEAD_CLEAR:
        for o in cols:
            bpy.data.objects.remove(o, do_unlink=True)
        cols = []
    return _finish(vis, cols, [], detail)


def vent(name: str, center, facing: float, m: dict, coll, size=(0.6, 0.25), surface: str = "wall", *, detail: int = 0):
    """Bouche d'aération à cadre sombre ; `surface` : "wall" (face vers `facing`), "ceiling"
    (face vers le bas) ou "floor" (vers le haut)."""
    w, h = size
    if surface == "wall":
        vis = [
            _ori(f"{name}_frame", (w + 0.06, h + 0.06, 0.02), (0, 0.01, 0), *_FRONT, m["dark"], coll, bevel=0.005),
            _ori(f"{name}", (w, h, 0.006), (0, 0.023, 0), *_FRONT, m["trim_sheet"], coll, "vent"),
        ]
    else:
        up = (0, 1, 0)
        right = (-1, 0, 0) if surface == "ceiling" else (1, 0, 0)
        dz = -1 if surface == "ceiling" else 1
        vis = [
            _ori(f"{name}_frame", (w + 0.06, h + 0.06, 0.02), (0, 0, dz * 0.01), right, up, m["dark"], coll, bevel=0.005),
            _ori(f"{name}", (w, h, 0.006), (0, 0, dz * 0.023), right, up, m["trim_sheet"], coll, "vent"),
        ]
    _place(vis, center, facing)
    return _finish(vis, [], [], detail)


def wall_screen(name: str, center, facing: float, m: dict, coll, size=(0.66, 0.46), *, detail: int = 0):
    """Écran mural : boîtier sombre, écran segmenté émissif, rangée de boutons dessous."""
    w, h = size
    vis = [
        _box(f"{name}_housing", (w + 0.08, 0.08, h + 0.16), (0, 0.04, -0.03), m["dark"], coll, bevel=0.015),
        _ori(f"{name}", (w, h, 0.01), (0, 0.085, 0.0), *_FRONT, m["screen"], coll, tile=max(0.6, w * 1.2)),
        _ori(f"{name}_buttons", (w * 0.8, 0.08, 0.008), (0, 0.084, -h / 2 - 0.05), *_FRONT, m["buttons"], coll),
    ]
    _place(vis, center, facing)
    return _finish(vis, [], [], detail)


def light_band(name: str, p0, p1, m: dict, coll, normal=(0, 0, -1), tone: str = "cool", *, detail: int = 0):
    """Bandeau lumineux (carter sombre + diffuseur `sh_band_<tone>`, cuit dans les lightmaps)
    de p0 à p1, sa face vers `normal`. `tone` : "cool", "amber", "warm", "reactor". Un bandeau
    de niveau ≥ 1 n'éclaire pas le bake (sa lumière disparaîtrait avec lui en Bas)."""
    p0, p1 = Vector(p0), Vector(p1)
    right = (p1 - p0).normalized()
    nrm = Vector(normal).normalized()
    up = nrm.cross(right)
    mid = (p0 + p1) / 2
    L = (p1 - p0).length
    vis = [
        _ori(f"{name}_housing", (L, 0.16, 0.06), tuple(mid + nrm * 0.03), tuple(right), tuple(up), m["dark"], coll, bevel=0.01),
        _ori(f"{name}", (max(0.1, L - 0.2), 0.085, 0.02), tuple(mid + nrm * 0.07), tuple(right), tuple(up), m[f"band_{tone}"], coll, ("strip", 1.0)),
    ]
    return _finish(vis, [], [], detail)


def hazard_stripe(name: str, p0, p1, m: dict, coll, width: float = 0.14, *, detail: int = 0, trim: str = "hazard"):
    """Bande de danger jaune et noir posée au sol (z de p0), de p0 à p1. `trim` (M9) : autre
    bande de la trim sheet (« chevrons », « warning_red »…)."""
    p0, p1 = Vector(p0), Vector(p1)
    right = (p1 - p0).normalized()
    up = Vector((0, 0, 1)).cross(right)
    mid = (p0 + p1) / 2 + Vector((0, 0, 0.003))
    vis = [_ori(f"{name}", ((p1 - p0).length, width, 0.006), tuple(mid), tuple(right), tuple(up), m["trim_sheet"], coll, trim)]
    return _finish(vis, [], [], detail)


def hazard_frame(name: str, center, size, m: dict, coll, width: float = 0.14, *, detail: int = 0):
    """Cadre de bandes de danger au sol autour d'un rectangle (trappe, zone de levage)."""
    cx, cy, cz = center
    sx, sy = size
    vis = []
    for i, (a, b) in enumerate((((-sx / 2 - width, -sy / 2 - width / 2), (sx / 2 + width, -sy / 2 - width / 2)),
                                ((sx / 2 + width, sy / 2 + width / 2), (-sx / 2 - width, sy / 2 + width / 2)))):
        vis += hazard_stripe(f"{name}_{i}", (cx + a[0], cy + a[1], cz), (cx + b[0], cy + b[1], cz), m, coll, width)[0]
    for i, (a, b) in enumerate((((sx / 2 + width / 2, -sy / 2), (sx / 2 + width / 2, sy / 2)),
                                ((-sx / 2 - width / 2, sy / 2), (-sx / 2 - width / 2, -sy / 2)))):
        v = hazard_stripe(f"{name}_{i + 2}", (cx + a[0], cy + a[1], cz - 0.0005), (cx + b[0], cy + b[1], cz - 0.0005), m, coll, width)[0]
        vis += v
    return _finish(vis, [], [], detail)


def ceiling_beam(name: str, p0, p1, m: dict, coll, depth: float = 0.26, width: float = 0.26, mat=None, *, detail: int = 0):
    """Poutre de plafond en I (âme, semelles, semelle basse rivée), p0 et p1 sur le plafond
    (dessus de la poutre). `mat` : matériau de l'âme (défaut : métal sombre)."""
    ax, L, mid = _span(p0, p1)
    mat = mat or m["dark"]
    unit = Vector((1, 0, 0)) if ax == 0 else Vector((0, 1, 0))
    side = Vector((0, 1, 0)) if ax == 0 else Vector((1, 0, 0))
    web = (L, 0.025, depth - 0.05) if ax == 0 else (0.025, L, depth - 0.05)
    top = (L, width, 0.025) if ax == 0 else (width, L, 0.025)
    vis = [
        _box(f"{name}_web", web, tuple(mid - Vector((0, 0, depth / 2))), mat, coll),
        _box(f"{name}_top", top, tuple(mid - Vector((0, 0, 0.0125))), mat, coll),
        _ori(f"{name}_bot", (L, width, 0.025), tuple(mid - Vector((0, 0, depth - 0.0125))), tuple(unit), tuple(-side if ax == 0 else side), m["trim_sheet"], coll, "rivets"),
    ]
    cols = []
    _auto_col(name, vis, coll, cols, force=False if detail else None)
    return _finish(vis, cols, [], detail)


def floor_grating(name: str, center, size, facing: float, m: dict, coll, glow: bool = True, *, detail: int = 0):
    """Caillebotis encastré au sol, cadre sombre ; `glow` : lumière par-dessous (True : vive,
    False : faible, None : aucune, M9)."""
    sx, sy = size
    mat = m["grating_glow"] if glow else (m["grating"] if glow is not None else m["grating_plain"])
    vis = [_box(f"{name}", (sx, sy, 0.012), (0, 0, 0.006), mat, coll)]
    for s in (-1, 1):
        vis.append(_box(f"{name}_rim_x{s}", (sx + 0.08, 0.04, 0.016), (0, s * (sy / 2 + 0.02), 0.008), m["dark"], coll))
        vis.append(_box(f"{name}_rim_y{s}", (0.04, sy, 0.016), (s * (sx / 2 + 0.02), 0, 0.008), m["dark"], coll))
    _place(vis, center, facing)
    return _finish(vis, [], [], detail)


def column(name: str, center, height: float, facing: float, m: dict, coll, *, detail: int = 0):
    """Colonne en H adossée au mur (`center` : pied, sur la face du mur ; `facing` vers la
    pièce), face rivée, gousset de pied ; collider."""
    vis = [
        _box(f"{name}_web", (0.025, 0.18, height), (0, 0.1, height / 2), m["dark"], coll),
        _box(f"{name}_back", (0.26, 0.025, height), (0, 0.0125, height / 2), m["dark"], coll),
        _ori(f"{name}_front", (height, 0.26, 0.025), (0, 0.19, height / 2), (0, 0, 1), (1, 0, 0), m["trim_sheet"], coll, "rivets"),
        _box(f"{name}_foot", (0.34, 0.24, 0.1), (0, 0.12, 0.05), m["dark"], coll, bevel=0.01),
    ]
    cols = [c.collider_box(f"{name}", (0.28, 0.2, height), (0, 0.1, height / 2), coll)]
    _place(vis + cols, center, facing)
    return _finish(vis, cols, [], detail)


def stencil(name: str, center, facing: float, m: dict, coll, length: float = 1.0, *, detail: int = 0):
    """Plaque à pochoirs génériques (chiffres, flèches) sur un mur."""
    vis = [_ori(f"{name}", (length, 0.12, 0.004), (0, 0.002, 0), *_FRONT, m["trim_sheet"], coll, "stencils")]
    _place(vis, center, facing)
    return _finish(vis, [], [], detail)


def beacon(name: str, center, facing: float, m: dict, coll, lumens: float = 900.0, rng: float = 4.0, *, detail: int = 0,
           light: bool = True):
    """Gyrophare rouge mural + repère LIGHT_ (lumière rouge du jeu) ; `light` = False : sans
    repère (simple voyant). De niveau ≥ 1, son `LIGHT_` l'est aussi : il n'est pas cuit."""
    vis = [
        _cyl(f"{name}_base", 0.07, 0.06, (0, 0.03, 0), m["dark"], coll, axis="Y", segments=16),
        _cyl(f"{name}", 0.055, 0.1, (0, 0.11, 0), m["beacon"], coll, axis="Y", segments=16),
    ]
    for i in range(3):
        a = i * math.pi / 3
        vis.append(_ori(f"{name}_cage{i}", (0.014, 0.13, 0.014), (0, 0.12, 0), (math.cos(a), 0, math.sin(a)), (0, 1, 0), m["dark"], coll))
    marks = [c.empty(f"LIGHT_{name}", (0, 0.3, 0), coll, {"lumens": lumens, "range": rng, "color": RED, "shadow_candidate": 0})] if light else []
    _place(vis + marks, center, facing)
    return _finish(vis, [], marks, detail)


def cabinet(name: str, center, facing: float, m: dict, coll, size=(1.1, 0.32, 1.7), *, detail: int = 0):
    """Armoire électrique posée au sol contre un mur : boutons, écran, grille, pochoir ;
    collider. `center` : pied, sur la face du mur."""
    sx, sd, sz = size
    vis = [
        _box(f"{name}", (sx, sd, sz), (0, sd / 2, sz / 2 + 0.1), m["dark"], coll, bevel=0.015),
        _box(f"{name}_plinth", (sx - 0.06, sd - 0.06, 0.1), (0, sd / 2, 0.05), m["rubber"], coll),
        _ori(f"{name}_buttons", (sx * 0.45, 0.4, 0.01), (-sx * 0.18, sd + 0.005, sz * 0.72), *_FRONT, m["buttons"], coll),
        _ori(f"{name}_screen", (sx * 0.27, 0.22, 0.01), (sx * 0.27, sd + 0.005, sz * 0.78), *_FRONT, m["screen"], coll, tile=0.75),
        _ori(f"{name}_vent", (sx * 0.82, 0.25, 0.006), (0, sd + 0.003, sz * 0.32), *_FRONT, m["trim_sheet"], coll, "vent"),
        _ori(f"{name}_label", (sx * 0.64, 0.12, 0.004), (0, sd + 0.002, sz * 0.97), *_FRONT, m["trim_sheet"], coll, "stencils"),
    ]
    cols = [c.collider_box(f"{name}", (sx, sd + 0.02, sz + 0.1), (0, sd / 2, (sz + 0.1) / 2), coll)]
    _place(vis + cols, center, facing)
    return _finish(vis, cols, [], detail)


def pump(name: str, center, facing: float, m: dict, coll, height: float | None = None, *, detail: int = 0):
    """Pompe sur châssis : réservoir cerclé, moteur et son pupitre, colonne montante (jusqu'à
    `height` si donné), durite ; collider."""
    vis = [
        _box(f"{name}_skid", (1.5, 0.9, 0.18), (0, 0, 0.09), m["dark"], coll, bevel=0.01),
        _cyl(f"{name}_tank", 0.36, 1.25, (-0.3, 0, 0.18 + 0.625), m["bare"], coll, segments=20),
        _box(f"{name}_motor", (0.5, 0.55, 0.5), (0.42, 0, 0.43), m["dark"], coll, bevel=0.02),
        _ori(f"{name}_panel", (0.4, 0.3, 0.01), (0.42, 0.28, 0.48), *_FRONT, m["buttons"], coll),
        _cyl(f"{name}_hose", 0.04, 0.8, (0.1, 0.2, 0.75), m["rubber"], coll, axis="X", segments=10),
    ]
    for zr in (0.3, 0.8, 1.35):
        vis.append(_cyl(f"{name}_ring{zr:.2f}", 0.38, 0.05, (-0.3, 0, zr), m["dark"], coll, segments=20))
    top = height if height is not None else 2.4
    vis.append(_cyl(f"{name}_riser", 0.07, top - 1.43, (-0.3, 0, 1.43 + (top - 1.43) / 2), m["trim_sheet"], coll, segments=12, uv="flange"))
    cols = [c.collider_box(f"{name}", (1.5, 0.95, 1.45), (0, 0, 0.72), coll)]
    _place(vis + cols, center, facing)
    return _finish(vis, cols, [], detail)


# --- Pièces de hangar (kit/hangar.py) --------------------------------------------------------------
# Accessibles aussi depuis ce module (`kit.hangar_shell(...)`), chargées à la demande (le module
# `hangar` importe celui-ci).

HANGAR_PIECES = ("hangar_shell", "floodlight", "docking_cradle", "gangway", "gantry_crane", "cargo_container", "crate_stack",
                 "fuel_bollard")


def __getattr__(name):
    if name in HANGAR_PIECES:
        import importlib

        here = str(Path(__file__).resolve().parent)
        if here not in sys.path:
            sys.path.insert(0, here)
        return getattr(importlib.import_module("hangar"), name)
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


# --- Export des pièces du kit ------------------------------------------------------------------

KIT_DIR = "assets/models/kit/ship_interior"
# Interactions qui doivent couvrir leur objet (M9, docs/m9-retours.md §3.3).
_VOLUME_INTERACTIONS = ("seat", "console", "locker", "npc")


def _check_interactions(objects, name: str) -> None:
    """Tout siège, console, casier ou comptoir de PNJ exporté a son `half_extents` ; un siège a
    son œil `SOCKET_<nom>_eye` ; un PNJ son `SOCKET_<nom>_npc`."""
    names = {o.name for o in objects}
    for obj in objects:
        if not obj.name.startswith("INTERACT_"):
            continue
        kind = obj.get("interact")
        if kind in _VOLUME_INTERACTIONS and "half_extents" not in obj.keys():
            raise RuntimeError(f"{name} : {obj.name} ({kind}) sans half_extents")
        base = obj.name[len("INTERACT_"):]
        if kind == "seat" and f"SOCKET_{base}_eye" not in names:
            raise RuntimeError(f"{name} : siège {obj.name} sans SOCKET_{base}_eye")
        if kind == "npc" and f"SOCKET_{base}_npc" not in names:
            raise RuntimeError(f"{name} : PNJ {obj.name} sans SOCKET_{base}_npc")


def export_kit() -> dict:
    """Exporte chaque pièce en glTF séparé (fusionnée par matériau et par niveau de décor,
    colliders et repères), et `art/kit/ship_interior.blend`. Renvoie {pièce: (triangles par
    niveau de décor, matériaux)}."""
    here = str(Path(__file__).resolve().parent)
    if here not in sys.path:
        sys.path.insert(0, here)
    import hangar as hg

    c.reset_scene()
    m = palette()
    pieces = {}

    def piece(name, builder):
        coll = c.collection(f"kit_{name}")
        vis, col, marks = builder(coll)
        pieces[name] = (vis, col, marks, coll)

    def shell(parts):
        return parts["visual"], parts["colliders"], parts.get("marks", [])

    piece("corridor_straight", lambda k: shell(room_shell("cs", -1.2, 1.2, 0, 4, 0, 2.6, [], m, k, floor_strip=True)))
    piece("corridor_corner", lambda k: shell(room_shell("cc", -1.2, 1.2, 0, 2.4, 0, 2.6, [("E", 1.2, DOOR_W + 0.6, 2.4)], m, k)))
    piece("corridor_t", lambda k: shell(room_shell("ct", -1.2, 1.2, 0, 2.4, 0, 2.6, [("E", 1.2, 1.8, 2.4), ("W", 1.2, 1.8, 2.4)], m, k)))
    piece("room_6x6", lambda k: shell(room_shell("rm", -3, 3, -3, 3, 0, 2.8, [("S", 0, DOOR_W, DOOR_H)], m, k)))
    piece("room_6x6_industrial", lambda k: shell(room_shell("ri", -3, 3, -3, 3, 0, 3.4, [("S", 0, DOOR_W, DOOR_H)], m, k, style="industrial")))
    piece("room_station", lambda k: shell(room_shell("rs", -4, 4, -4, 4, 0, 4.5, [("S", 0, DOOR_W, DOOR_H)], m, k, style="station")))
    piece("airlock", lambda k: shell(room_shell("al", -2, 2, -2, 2, 0, 2.6, [("E", 0, DOOR_W, DOOR_H), ("W", 0, DOOR_W, DOOR_H)], m, k)))
    piece("door_sliding", lambda k: door_panel("door", m, k))
    piece("door_station", lambda k: door_panel("door_st", m, k, style="station"))
    piece("console", lambda k: console("console", (0, 0, 0), 0.0, m, k))
    piece("console_low", lambda k: console("console_low", (0, 0, 0), 0.0, m, k, low=True))
    piece("seat", lambda k: seat("seat", (0, 0, 0), 0.0, m, k))
    piece("bunk", lambda k: bunk("bunk", (0, 0, 0), 0.0, m, k))
    piece("crate", lambda k: crate("crate", (0, 0, 0), 0.0, m, k))
    piece("locker", lambda k: locker("locker", (0, 0, 0), 0.0, m, k, container="kit_locker"))
    piece("table", lambda k: table("table", (0, 0, 0), 0.0, m, k))
    # Objet ramassable : mallette rattachée à son repère (convention des secteurs, `attach_pickup`).
    piece("pickup", lambda k: (lambda r: ([r[0]], [], r[1]))(attach_pickup(pickup("pickup", (0, 0, 0), 0.0, m, k, item="spare_parts"))))
    piece("reactor", lambda k: reactor("reactor", (0, 0, 0), m, k))
    piece("ceiling_light", lambda k: ceiling_light("ceiling_light", (0, 0, 2.6), 0.0, m, k, {"lumens": 60000.0}))
    piece("stairs", lambda k: stairs("stairs", -0.8, 0.8, 0, 0.8, 4, m, k))
    piece("ladder", lambda k: ladder("ladder", (0, 0, 0), 0.0, 3.0, m, k))
    piece("cable_tray", lambda k: cable_tray("cable_tray", (0, -2, 2.4), (0, 2, 2.4), m, k, hang_to=2.6))
    piece("pipe", lambda k: pipe("pipe", (0, -2, 2.4), (0, 2, 2.4), m, k, radius=0.06, hang_to=2.6))
    piece("duct", lambda k: duct("duct", (0, -2, 2.6), (0, 2, 2.6), m, k, hang_to=3.0))
    piece("junction_box", lambda k: junction_box("junction_box", (0, 0, 1.3), 0.0, m, k))
    piece("vent", lambda k: vent("vent", (0, 0, 1.0), 0.0, m, k))
    piece("wall_screen", lambda k: wall_screen("wall_screen", (0, 0, 1.5), 0.0, m, k))
    piece("light_band", lambda k: light_band("light_band", (-1, 0, 2.4), (1, 0, 2.4), m, k))
    piece("hazard_stripe", lambda k: hazard_stripe("hazard_stripe", (-1, 0, 0), (1, 0, 0), m, k))
    piece("ceiling_beam", lambda k: ceiling_beam("ceiling_beam", (-3, 0, 3.0), (3, 0, 3.0), m, k))
    piece("floor_grating", lambda k: floor_grating("floor_grating", (0, 0, 0), (1.0, 2.0), 0.0, m, k))
    piece("column", lambda k: column("column", (0, 0, 0), 3.2, 0.0, m, k))
    piece("beacon", lambda k: beacon("beacon", (0, 0, 2.2), 0.0, m, k))
    piece("cabinet", lambda k: cabinet("cabinet", (0, 0, 0), 0.0, m, k))
    piece("pump", lambda k: pump("pump", (0, 0, 0), 0.0, m, k))
    # M9 : chambre, comptoir, hangar.
    piece("bed", lambda k: bed("bed", (0, 0, 0), 0.0, m, k))
    piece("locker_bank", lambda k: locker_bank("locker_bank", (0, 0, 0), 0.0, m, k, 3, containers="kit_locker_bank"))
    piece("counter", lambda k: counter("counter", (0, 0, 0), 0.0, m, k))
    piece("wall_desk", lambda k: wall_desk("wall_desk", (0, 0, 0), 0.0, m, k))
    piece("shelf", lambda k: shelf("shelf", (0, 0, 0), 0.0, m, k, items_detail=1))
    piece("cabin_porthole", lambda k: cabin_porthole("cabin_porthole", (0, 0, 1.5), 0.0, m, k))
    piece("hangar_shell", lambda k: shell(hg.hangar_shell("hg", -9, 9, -12, 12, 10.0, [("S", -5.0, DOOR_W, DOOR_H)], m, k,
                                                         floor_holes=[(-4.0, 4.0, -6.0, 6.0)], bay_door=("N", 0.0, 10.0, 7.0))))
    piece("floodlight", lambda k: hg.floodlight("floodlight", (0, 0, 6.0), 0.0, m, k))
    piece("docking_cradle", lambda k: hg.docking_cradle("cradle", -4.0, 4.0, -6.0, 6.0, m, k, gaps=[("W", 0.0, 1.8)]))
    piece("gangway", lambda k: hg.gangway("gangway", (-3.0, 0.0, 0.0), (0.0, 0.0, 0.0), m, k))
    piece("gantry_crane", lambda k: hg.gantry_crane("gantry", (0, 0, 0), 0.0, m, k))
    piece("cargo_container", lambda k: hg.cargo_container("container", (0, 0, 0), 0.0, m, k))
    piece("crate_stack", lambda k: hg.crate_stack("crate_stack", (0, 0, 0), 0.0, m, k))
    piece("fuel_bollard", lambda k: hg.fuel_bollard("fuel_bollard", (0, 0, 0), 0.0, m, k, hose_to=(1.6, 1.4)))

    bpy.context.view_layer.update()
    exports = c.collection("EXPORT")
    stats = {}
    cursor_x, cursor_y, row_h = 0.0, 0.0, 0.0
    for name, (vis, col, marks, coll) in pieces.items():
        _check_interactions(marks, name)
        merged = c.join_by_material(vis, f"{name}", exports)
        c.export_gltf(f"{KIT_DIR}/{name}.gltf", merged + col + marks)
        by_level = {}
        for o in merged:
            by_level[c.detail_of(o)] = by_level.get(c.detail_of(o), 0) + _tris([o])
        stats[name] = (by_level, sorted({o.data.materials[0].name for o in merged}))
        # Revue visuelle dans le .blend : pièces en rangées, espacées selon leur emprise.
        lo = Vector((1e9, 1e9, 0.0))
        hi = Vector((-1e9, -1e9, 0.0))
        for o in merged:
            for b in o.bound_box:
                p = o.matrix_world @ Vector(b)
                lo.x, lo.y, hi.x, hi.y = min(lo.x, p.x), min(lo.y, p.y), max(hi.x, p.x), max(hi.y, p.y)
        sx, sy = hi.x - lo.x, hi.y - lo.y
        if cursor_x > 0 and cursor_x + sx > 60.0:
            cursor_x, cursor_y, row_h = 0.0, cursor_y + row_h + 3.0, 0.0
        dx, dy = cursor_x - lo.x, cursor_y - lo.y
        for obj in vis + col + marks + merged:
            if obj.parent is not None:
                continue  # suit son parent (mallette d'un objet ramassable)
            obj.location.x += dx
            obj.location.y += dy
        cursor_x += sx + 3.0
        row_h = max(row_h, sy)
        for obj in merged:
            obj.hide_set(True)
            obj.hide_render = True
    c.save_blend("art/kit/ship_interior.blend")
    for name, (by_level, mats) in stats.items():
        levels = ", ".join(f"niveau {k} : {v}" for k, v in sorted(by_level.items()))
        print(f"kit {name} : {sum(by_level.values())} triangles ({levels}), {len(mats)} matériaux ({', '.join(mats)})")
    print(f"kit exporté : {len(pieces)} pièces dans {KIT_DIR}/")
    return stats


if __name__ == "__main__":
    export_kit()
