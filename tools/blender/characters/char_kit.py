"""Outils de modélisation des personnages et des armes portatives (lot F de M8).

Formes organiques sans sculpture ni squelette : tout se construit en `bmesh` à partir de
quelques primitives paramétriques, rejouables :

- `tube` : anneaux elliptiques le long d'un chemin (membres, torse, tuyaux, sangles plates) ;
- `shell` : coque en arc (plaques bombées autour du torse, d'un bras, d'une épaule, visière) ;
- `orient` : place un objet construit le long de Z sur un segment quelconque ;
- `uv_cell` / `uv_cell_cyl` : UV dans une case de l'atlas `char_gear` (char_atlas.py).

Les textures de la bibliothèque (`fabric`, `rubber`, `metal_dark`, `metal_bare`) se posent avec
`common.uv_box` à une densité de personnage (`TILE_*`).
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))

import char_atlas  # noqa: E402
import common as c  # noqa: E402

# Densité des textures sur un personnage (mètres par tuile) : toile `char_cloth` à sa taille
# réelle, caoutchouc et métaux de la bibliothèque resserrés (tuiles de décor de 1 à 2 m).
TILE_FABRIC = 0.5
TILE_RUBBER = 0.5
TILE_METAL = 0.5


# --- Matériaux -----------------------------------------------------------------------------------


def gear_material() -> bpy.types.Material:
    """Atlas `char_gear` : pièces peintes, rouille, cuivre, sangles, toile, émissifs."""
    maps = {role: char_atlas.OUT_DIR / f"{char_atlas.NAME}_{role}.png" for role in ("color", "normal", "orm", "emissive")}
    if not maps["color"].exists():
        char_atlas.generate()
    return c.textured_material("char_gear", maps, emissive=(1.0, 1.0, 1.0), emissive_strength=3.0)


def cloth_material(name: str, tint) -> bpy.types.Material:
    """Toile de combinaison `char_cloth` (char_cloth.py), teintée par le facteur glTF."""
    import char_cloth

    maps = {role: char_cloth.OUT_DIR / f"{char_cloth.NAME}_{role}.png" for role in ("color", "normal", "orm")}
    if not maps["color"].exists():
        char_cloth.generate()
    mat = c.textured_material(name, maps, tint=tint)
    mat["sh_tile_m"] = char_cloth.TILE_M
    return mat


# --- Géométrie -----------------------------------------------------------------------------------


def _frame(t: Vector, ref: Vector) -> tuple[Vector, Vector]:
    """Repère d'anneau (x, y) perpendiculaire à `t`, x aussi proche que possible de `ref`."""
    x = ref - t * ref.dot(t)
    if x.length < 1e-5:
        x = Vector((0, 0, 1)) - t * t.z
        if x.length < 1e-5:
            x = Vector((1, 0, 0)) - t * t.x
    x.normalize()
    y = t.cross(x)
    return x, y


def tube(name: str, points, radii, mat, coll, segments: int = 10, ref=(1.0, 0.0, 0.0),
         refs=None, offsets=None, cap_start: bool = True, cap_end: bool = True, smooth: bool = True,
         closed: bool = False) -> bpy.types.Object:
    """Tube d'anneaux elliptiques le long de `points`.

    - `radii` : un rayon, ou un (rx, ry) par point (rx le long de l'axe x de l'anneau) ;
    - `ref` / `refs` : direction vers laquelle pointe l'axe x des anneaux (un pour tous ou un
      par point) : pour un torse le long de Z, x = côté (X) et y = avant (Y) ;
    - `offsets` : décalage (ox, oy) de chaque anneau dans son repère (ventre, dos, poitrine) ;
    - `closed` : boucle fermée (ceinture, bandoulière), sans bouchons.
    """
    pts = [Vector(p) for p in points]
    n = len(pts)
    if not isinstance(radii, (list, tuple)) or (len(radii) == 2 and n != 2 and not isinstance(radii[0], (list, tuple))):
        radii = [radii] * n
    rr = [(r, r) if not isinstance(r, (list, tuple)) else r for r in radii]
    bm = bmesh.new()
    rings = []
    for i, p in enumerate(pts):
        if closed:
            t = pts[(i + 1) % n] - pts[i - 1]
        elif i == 0:
            t = pts[1] - pts[0]
        elif i == n - 1:
            t = pts[-1] - pts[-2]
        else:
            t = (pts[i + 1] - pts[i]).normalized() + (pts[i] - pts[i - 1]).normalized()
        t.normalize()
        r = Vector(refs[i]) if refs is not None else Vector(ref)
        x, y = _frame(t, r)
        ox, oy = offsets[i] if offsets is not None else (0.0, 0.0)
        ring = []
        for k in range(segments):
            a = 2 * math.pi * k / segments
            v = p + x * (math.cos(a) * rr[i][0] + ox) + y * (math.sin(a) * rr[i][1] + oy)
            ring.append(bm.verts.new(v))
        rings.append(ring)
    span = n if closed else n - 1
    for i in range(span):
        a, b = rings[i], rings[(i + 1) % n]
        for k in range(segments):
            k2 = (k + 1) % segments
            bm.faces.new((a[k], a[k2], b[k2], b[k]))
    if not closed:
        if cap_start:
            bm.faces.new(list(reversed(rings[0])))
        if cap_end:
            bm.faces.new(rings[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return _finish(name, bm, mat, coll, smooth)


def shell(name: str, radii, z0: float, z1: float, a0: float, a1: float, thickness: float, mat, coll,
          segments: int = 10, rings: int = 1, center=(0.0, 0.0), bulge: float = 0.0, taper: float = 0.0,
          smooth: bool = True) -> bpy.types.Object:
    """Coque en arc elliptique autour de Z (repère local), d'angle a0 → a1 (degrés, 0 = +X,
    90 = +Y : l'avant), de z0 à z1, d'épaisseur `thickness` vers l'extérieur.

    `radii` : (rx, ry) au bas de la coque ; `taper` : variation relative du rayon en haut
    (négatif : rentre, épaule, casque) ; `bulge` : bombé vertical (m) au milieu de la hauteur.
    Construite le long de Z : `orient` la pose ailleurs."""
    bm = bmesh.new()
    rx, ry = radii
    cx, cy = center
    nz = rings + 1
    grid_out, grid_in = [], []
    for j in range(nz + 0):
        f = j / rings
        z = z0 + (z1 - z0) * f
        s = 1.0 + taper * f + bulge * 4 * f * (1 - f) / max(rx, ry)
        row_o, row_i = [], []
        for k in range(segments + 1):
            a = math.radians(a0 + (a1 - a0) * k / segments)
            ca, sa = math.cos(a), math.sin(a)
            row_i.append(bm.verts.new((cx + ca * rx * s, cy + sa * ry * s, z)))
            row_o.append(bm.verts.new((cx + ca * (rx * s + thickness), cy + sa * (ry * s + thickness), z)))
        grid_out.append(row_o)
        grid_in.append(row_i)
    for j in range(rings):
        for k in range(segments):
            o, i = grid_out, grid_in
            bm.faces.new((o[j][k], o[j][k + 1], o[j + 1][k + 1], o[j + 1][k]))
            bm.faces.new((i[j][k], i[j + 1][k], i[j + 1][k + 1], i[j][k + 1]))
    # Tranches : bas, haut, et les deux bouts de l'arc.
    for k in range(segments):
        bm.faces.new((grid_in[0][k], grid_in[0][k + 1], grid_out[0][k + 1], grid_out[0][k]))
        bm.faces.new((grid_out[rings][k], grid_out[rings][k + 1], grid_in[rings][k + 1], grid_in[rings][k]))
    for j in range(rings):
        bm.faces.new((grid_out[j][0], grid_out[j + 1][0], grid_in[j + 1][0], grid_in[j][0]))
        bm.faces.new((grid_in[j][segments], grid_in[j + 1][segments], grid_out[j + 1][segments], grid_out[j][segments]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    obj = _finish(name, bm, mat, coll, smooth)
    # Axe de la coque, relu par `uv_cell_shell` (déroulé angle × hauteur).
    obj["sh_shell_center"] = (float(cx), float(cy))
    return obj


def dome(name: str, center, radii, height: float, mat, coll, segments: int = 16, rings: int = 6,
         z_bottom: float = 0.0, smooth: bool = True) -> bpy.types.Object:
    """Demi-ellipsoïde (casque, réservoir) : base elliptique `radii` (rx, ry), sommet à
    `height` ; `z_bottom` < 0 prolonge le bas en cylindre."""
    cx, cy, cz = center
    rx, ry = radii
    pts, rr = [], []
    if z_bottom < 0:
        pts.append((cx, cy, cz + z_bottom))
        rr.append((rx, ry))
    for j in range(rings):
        f = j / rings
        ang = f * math.pi / 2
        pts.append((cx, cy, cz + height * math.sin(ang)))
        rr.append((rx * math.cos(ang), ry * math.cos(ang)))
    pts.append((cx, cy, cz + height * math.sin(math.pi / 2 * (rings - 0.15) / rings)))
    rr.append((rx * 0.18, ry * 0.18))
    return tube(name, pts, rr, mat, coll, segments=segments, ref=(1, 0, 0), smooth=smooth)


def rounded_box(name: str, size, center, mat, coll, bevel: float = 0.01, segments: int = 2) -> bpy.types.Object:
    """Pavé aux arêtes arrondies (biseau à `segments` pas) : pièces mécaniques, poches."""
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * size[0], v.co.y * size[1], v.co.z * size[2]))
    if bevel > 0:
        bmesh.ops.bevel(bm, geom=list(bm.edges), offset=bevel, segments=segments, affect="EDGES", profile=0.5)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    obj.location = center
    coll.objects.link(obj)
    _auto_smooth(obj)
    return obj


def cyl(name: str, radius: float, depth: float, mat, coll, segments: int = 12, r2: float | None = None,
        bevel: float = 0.0) -> bpy.types.Object:
    """Cylindre (ou tronc de cône) le long de Z, centré à l'origine : `orient` le place."""
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=segments, radius1=radius,
                          radius2=radius if r2 is None else r2, depth=depth)
    if bevel > 0:
        rims = [e for e in bm.edges if len(e.link_faces) == 2 and
                abs(e.link_faces[0].normal.dot(e.link_faces[1].normal)) < 0.5]
        bmesh.ops.bevel(bm, geom=rims, offset=bevel, segments=1, affect="EDGES", profile=0.5)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    _auto_smooth(obj)
    return obj


def _finish(name, bm, mat, coll, smooth) -> bpy.types.Object:
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    if smooth:
        _auto_smooth(obj)
    return obj


def _auto_smooth(obj: bpy.types.Object, angle_deg: float = 50.0) -> None:
    """Lissage par angle : faces lissées, arêtes vives au-delà de `angle_deg` (glTF : sommets
    dupliqués sur les arêtes vives)."""
    mesh = obj.data
    mesh.polygons.foreach_set("use_smooth", [True] * len(mesh.polygons))
    bm = bmesh.new()
    bm.from_mesh(mesh)
    limit = math.radians(angle_deg)
    for e in bm.edges:
        if len(e.link_faces) == 2:
            e.smooth = e.link_faces[0].normal.angle(e.link_faces[1].normal, 0.0) < limit
        else:
            e.smooth = False
    bm.to_mesh(mesh)
    bm.free()


def orient(obj: bpy.types.Object, start, end=None, side=(1.0, 0.0, 0.0), roll_deg: float = 0.0,
           at=None) -> bpy.types.Object:
    """Pose un objet construit le long de +Z local : origine en `start` (ou `at`), +Z vers `end`,
    +X local vers `side` (projeté), puis rotation `roll_deg` autour de l'axe."""
    s = Vector(start)
    if end is None:
        z = Vector((0, 0, 1))
    else:
        z = (Vector(end) - s).normalized()
    x, y = _frame(z, Vector(side))
    rot = Matrix((x, y, z)).transposed()
    if roll_deg:
        rot = rot @ Matrix.Rotation(math.radians(roll_deg), 3, "Z")
    obj.matrix_basis = Matrix.Translation(Vector(at) if at is not None else s) @ rot.to_4x4()
    return obj


def mirror_x(obj: bpy.types.Object, name: str) -> bpy.types.Object:
    """Copie en miroir par rapport au plan X = 0 (appliqué dans le maillage)."""
    dup = obj.copy()
    dup.data = obj.data.copy()
    dup.name = name
    obj.users_collection[0].objects.link(dup)
    dup.matrix_basis = Matrix.Scale(-1, 4, (1, 0, 0)) @ obj.matrix_basis
    apply_transform(dup)
    bm = bmesh.new()
    bm.from_mesh(dup.data)
    bmesh.ops.reverse_faces(bm, faces=bm.faces)
    bm.to_mesh(dup.data)
    bm.free()
    return dup


def apply_transform(obj: bpy.types.Object) -> None:
    """Applique position, rotation et échelle au maillage. `matrix_basis` et non
    `matrix_world` : ce dernier n'est recalculé qu'à la mise à jour du graphe de dépendances,
    il ignore un `location` posé juste avant (objets sans parent ici)."""
    obj.data.transform(obj.matrix_basis.copy())
    obj.matrix_basis = Matrix.Identity(4)


# --- Assemblage ----------------------------------------------------------------------------------


class Builder:
    """Accumule les pièces avec leur mode d'UV, posé à la fin (après les transformations)."""

    def __init__(self, m: dict, coll):
        self.m = m
        self.coll = coll
        self.parts: list = []

    def add(self, obj, uv: str, arg=None):
        """uv : `lib` (texture de bibliothèque, arg = tuile), `cell` (case de l'atlas : coque
        déroulée, sinon projection boîte), `cyl` (case de l'atlas en cylindre, objet le long de
        Z local)."""
        self.parts.append((obj, uv, arg))
        return obj

    def lib(self, obj, tile):
        return self.add(obj, "lib", tile)

    def cell(self, obj, cell):
        return self.add(obj, "cell", cell)

    def cyl(self, obj, cell):
        return self.add(obj, "cyl", cell)

    def finalize(self):
        out = []
        for i, (obj, uv, arg) in enumerate(self.parts):
            if uv == "lib":
                uv_lib(obj, arg, offset=((i * 0.37) % 1.0, (i * 0.61) % 1.0))
            elif uv == "cell":
                uv_cell_shell(obj, arg)
            elif uv == "cyl":
                uv_cell_cyl(obj, arg)
            if "sh_shell_center" in obj:
                del obj["sh_shell_center"]  # ne pas l'exporter en extras glTF
            out.append(obj)
        return out


def seg_box(b: Builder, name, size, start, end, side, mat_key, uv, arg, roll=0.0, offset=(0.0, 0.0)):
    """Pavé arrondi posé le long d'un segment (largeur x, épaisseur y, longueur z), décalé de
    `offset` (x, y) dans son repère."""
    obj = rounded_box(name, size, (0, 0, 0), b.m[mat_key], b.coll, bevel=min(size) * 0.42, segments=3)
    obj.data.transform(Matrix.Translation((offset[0], offset[1], size[2] / 2)))
    orient(obj, start, end, side=side, roll_deg=roll)
    return b.add(obj, uv, arg)


def seg_cyl(b: Builder, name, radius, start, end, mat_key, uv, arg, segments=12, r2=None, side=(1, 0, 0), bevel=0.0):
    length = (Vector(end) - Vector(start)).length
    obj = cyl(name, radius, length, b.m[mat_key], b.coll, segments=segments, r2=r2, bevel=bevel)
    obj.data.transform(Matrix.Translation((0, 0, length / 2)))
    orient(obj, start, end, side=side)
    return b.add(obj, uv, arg)


def seg_shell(b: Builder, name, start, end, side, radii, z0, z1, a0, a1, thickness, mat_key, cell,
              segments=10, rings=2, taper=0.0, bulge=0.0, center=(0.0, 0.0)):
    obj = shell(name, radii, z0, z1, a0, a1, thickness, b.m[mat_key], b.coll, segments=segments,
                rings=rings, taper=taper, bulge=bulge, center=center)
    orient(obj, start, end, side=side)
    return b.cell(obj, cell)


# --- UV ------------------------------------------------------------------------------------------


def _loops(obj: bpy.types.Object):
    """Positions locales (à l'échelle) par boucle, normales par face, face de chaque boucle."""
    mesh = obj.data
    s = np.array(obj.scale, dtype=np.float64)
    co = np.empty(len(mesh.vertices) * 3)
    mesh.vertices.foreach_get("co", co)
    co = co.reshape(-1, 3) * s
    lv = np.empty(len(mesh.loops), dtype=np.int64)
    mesh.loops.foreach_get("vertex_index", lv)
    nrm = np.empty(len(mesh.polygons) * 3)
    mesh.polygons.foreach_get("normal", nrm)
    nrm = nrm.reshape(-1, 3)
    tot = np.empty(len(mesh.polygons), dtype=np.int64)
    mesh.polygons.foreach_get("loop_total", tot)
    return co[lv], nrm, np.repeat(np.arange(len(mesh.polygons)), tot)


def uv_cell(obj: bpy.types.Object, cell: str, pad_px: float = 4.0) -> None:
    """Projection boîte dans une case de l'atlas : la pièce entière tient dans la case.

    Pour chaque axe de projection, une dimension au moins aussi longue que 30 % de la plus
    grande remplit la case (les arêtes de la pièce tombent sur le bord usé de la case) ; une
    dimension plus courte (tranche d'une plaque) garde l'échelle commune et reste en bande le
    long du bord. Pas de miroir sur les faces opposées."""
    u0, v0, u1, v1 = char_atlas.cell_rect(cell, pad_px)
    p, nrm, lp = _loops(obj)
    n = nrm[lp]
    axis = np.argmax(np.abs(n), axis=1)
    sign = np.sign(n[np.arange(len(n)), axis])
    a = np.where(axis == 0, sign * p[:, 1], np.where(axis == 1, -sign * p[:, 0], p[:, 0]))
    b = np.where(axis == 2, sign * p[:, 1], p[:, 2])
    big = max(float(np.ptp(p[:, k])) for k in range(3)) or 1.0
    uu = np.zeros(len(p))
    vv = np.zeros(len(p))
    for ax in range(3):
        m = axis == ax
        if not m.any():
            continue
        for src, dst in ((a, uu), (b, vv)):
            vals = src[m]
            lo, span = vals.min(), np.ptp(vals)
            scale = span if span >= 0.3 * big else big
            dst[m] = (vals - lo) / max(scale, 1e-9)
    uv = np.stack([u0 + np.clip(uu, 0, 1) * (u1 - u0), v0 + np.clip(vv, 0, 1) * (v1 - v0)], 1)
    layer = c._uv_layer(obj.data)
    layer.data.foreach_set("uv", uv.ravel())


def uv_cell_cyl(obj: bpy.types.Object, cell: str, pad_px: float = 4.0, turns: float = 1.0) -> None:
    """UV cylindriques dans une case (objet construit le long de Z local) : U = angle (`turns`
    fois la case par tour), V = hauteur. Les bouchons prennent une projection plane."""
    u0, v0, u1, v1 = char_atlas.cell_rect(cell, pad_px)
    p, nrm, lp = _loops(obj)
    z = p[:, 2]
    zlo, zspan = z.min(), max(np.ptp(z), 1e-9)
    rmax = max(np.abs(p[:, :2]).max(), 1e-9)
    uu = np.zeros(len(p))
    vv = np.zeros(len(p))
    for poly in range(len(nrm)):
        idx = np.nonzero(lp == poly)[0]
        if idx.size == 0:
            continue
        if abs(nrm[poly][2]) > 0.7:
            uu[idx] = 0.5 + 0.5 * p[idx, 0] / rmax
            vv[idx] = 0.5 + 0.5 * p[idx, 1] / rmax
            continue
        ang = (np.arctan2(p[idx, 1], p[idx, 0]) / (2 * np.pi)) % 1.0
        if ang.max() - ang.min() > 0.5:
            ang = np.where(ang < 0.5, ang + 1.0, ang)
        uu[idx] = (ang * turns) % 1.0001 if turns != 1.0 else ang
        vv[idx] = (z[idx] - zlo) / zspan
    uu = np.clip(uu, 0.0, 1.0)
    uv = np.stack([u0 + uu * (u1 - u0), v0 + np.clip(vv, 0, 1) * (v1 - v0)], 1)
    layer = c._uv_layer(obj.data)
    layer.data.foreach_set("uv", uv.ravel())


def uv_cell_shell(obj: bpy.types.Object, cell: str, pad_px: float = 4.0) -> None:
    """Déroulé d'une coque (`shell`) dans une case : U = angle autour de son axe (de bout en
    bout de l'arc), V = hauteur. Les bords de la plaque tombent sur le bord usé de la case ;
    les tranches prennent la couleur du bord. Sans `sh_shell_center` : `uv_cell`."""
    if "sh_shell_center" not in obj:
        uv_cell(obj, cell, pad_px)
        return
    u0, v0, u1, v1 = char_atlas.cell_rect(cell, pad_px)
    cx, cy = obj["sh_shell_center"]
    p, _, _ = _loops(obj)
    x, y = p[:, 0] - cx, p[:, 1] - cy
    # Angles relatifs à la direction moyenne : continus pour tout arc < 360°.
    mx, my = x.mean(), y.mean()
    ref = math.atan2(my, mx)
    ang = np.arctan2(y, x) - ref
    ang = (ang + np.pi) % (2 * np.pi) - np.pi
    uu = (ang - ang.min()) / max(np.ptp(ang), 1e-9)
    vv = (p[:, 2] - p[:, 2].min()) / max(np.ptp(p[:, 2]), 1e-9)
    uv = np.stack([u0 + uu * (u1 - u0), v0 + vv * (v1 - v0)], 1)
    layer = c._uv_layer(obj.data)
    layer.data.foreach_set("uv", uv.ravel())


def uv_lib(obj: bpy.types.Object, tile: float, offset=(0.0, 0.0)) -> None:
    """Texture de la bibliothèque à densité de personnage (`common.uv_box`, repère du monde :
    appliquer la transformation d'abord pour que les pièces orientées raccordent)."""
    apply_transform(obj)
    c.uv_box(obj, tile, offset)


def triangles(objs) -> int:
    return sum(len(p.vertices) - 2 for o in objs for p in o.data.polygons)


def spline(points, per_segment: int = 4) -> list[tuple[float, float, float]]:
    """Catmull-Rom passant par `points` (tuyaux, câbles) : `per_segment` pas par segment."""
    pts = [Vector(p) for p in points]
    ext = [pts[0] * 2 - pts[1]] + pts + [pts[-1] * 2 - pts[-2]]
    out = []
    for i in range(1, len(ext) - 2):
        p0, p1, p2, p3 = ext[i - 1], ext[i], ext[i + 1], ext[i + 2]
        for k in range(per_segment):
            t = k / per_segment
            t2, t3 = t * t, t * t * t
            v = 0.5 * ((2 * p1) + (-p0 + p2) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t2 + (-p0 + 3 * p1 - 3 * p2 + p3) * t3)
            out.append(tuple(v))
    out.append(tuple(pts[-1]))
    return out
