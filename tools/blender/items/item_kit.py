"""Outils de modélisation des objets d'inventaire (lot F de M9).

S'appuie sur `characters/char_kit.py` (lu, jamais modifié) pour les primitives (`rounded_box`,
`cyl`, `tube`, `orient`, `spline`, `apply_transform`) et ajoute :

- les UV dans l'atlas `item_gear` (`item_atlas.py`) : `uv_box` (projection boîte qui remplit la
  case), `uv_planar` (projection plane orientée : écran, étiquette, puce), `uv_cyl`
  (cylindre construit le long de Z), `uv_tube` (tube de `char_kit.tube` : U autour, V le long) ;
- `Builder` : accumule les pièces et leur mode d'UV, posé à la fin ;
- formes : `soft_box` (pavé déformé puis subdivisé : tissu plié, emballage souple),
  `gear` (roue dentée percée), `flap` (languette à bord dentelé), `hex_prism` (écrou) ;
- `finish` : fusion par matériau, transformations appliquées, origine au centre de la base.

Repères des objets : 1 unité = 1 m, Z-haut, avant vers +Y, origine au centre de la base (l'objet
est posé sur une table ou au sol).
"""

from __future__ import annotations

import contextlib
import io
import math
import sys
from pathlib import Path

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "characters"))
sys.path.insert(0, str(HERE))

import char_kit as ck  # noqa: E402
import common as c  # noqa: E402
import item_atlas  # noqa: E402

rounded_box = ck.rounded_box
cyl = ck.cyl
tube = ck.tube
orient = ck.orient
spline = ck.spline
apply_transform = ck.apply_transform
triangles = ck.triangles
auto_smooth = ck._auto_smooth

# Teinte de la combinaison de vol (facteur glTF, linéaire) : gris ardoise bleuté, assez clair
# pour se lire sous la seule ambiante du jeu.
SUIT_TINT = (0.30, 0.40, 0.58)


# --- Matériaux -----------------------------------------------------------------------------------


def atlas_material() -> bpy.types.Material:
    """Atlas `item_gear` (un seul matériau pour tous les objets), régénéré s'il manque."""
    roles = ("color", "normal", "orm", "emissive")
    maps = {role: item_atlas.OUT_DIR / f"{item_atlas.NAME}_{role}.png" for role in roles}
    if not all(p.exists() for p in maps.values()):
        item_atlas.generate()
    return c.textured_material("item_gear", maps, emissive=(1.0, 1.0, 1.0), emissive_strength=item_atlas.EMIT_STRENGTH)


def suit_material() -> bpy.types.Material:
    """Toile `char_cloth` (lot F de M8, lue telle quelle) teintée gris ardoise."""
    import char_cloth

    maps = {role: char_cloth.OUT_DIR / f"{char_cloth.NAME}_{role}.png" for role in ("color", "normal", "orm")}
    if not maps["color"].exists():
        char_cloth.generate()
    mat = c.textured_material("item_suit", maps, tint=SUIT_TINT)
    mat["sh_tile_m"] = char_cloth.TILE_M
    return mat


def materials() -> dict:
    return {"gear": atlas_material()}


# --- UV ------------------------------------------------------------------------------------------


def _loops(obj: bpy.types.Object):
    """Positions locales (à l'échelle) par boucle, normales par face, face de chaque boucle,
    sommet de chaque boucle."""
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
    return co[lv], nrm, np.repeat(np.arange(len(mesh.polygons)), tot), lv


def _set_uv(obj, cell: str, uu, vv, pad_px: float = 4.0) -> None:
    u0, v0, u1, v1 = item_atlas.cell_rect(cell, pad_px)
    uv = np.stack([u0 + np.clip(uu, 0, 1) * (u1 - u0), v0 + np.clip(vv, 0, 1) * (v1 - v0)], 1)
    layer = c._uv_layer(obj.data)
    layer.data.foreach_set("uv", uv.ravel())


def uv_box(obj: bpy.types.Object, cell: str) -> None:
    """Projection boîte dans une case : chaque grande face remplit la case (ses arêtes tombent sur
    le bord usé) ; sur une face étroite, la petite dimension reste en bande le long du bord, à la
    même échelle que la grande (taille physique de la case, `item_atlas.cell_size`) : la
    peinture n'est pas étirée. Pas de miroir sur les faces opposées."""
    p, nrm, lp, _ = _loops(obj)
    n = nrm[lp]
    axis = np.argmax(np.abs(n), axis=1)
    sign = np.sign(n[np.arange(len(n)), axis])
    a = np.where(axis == 0, sign * p[:, 1], np.where(axis == 1, -sign * p[:, 0], p[:, 0]))
    b = np.where(axis == 2, sign * p[:, 1], p[:, 2])
    big = max(float(np.ptp(p[:, k])) for k in range(3)) or 1.0
    cw, ch = item_atlas.cell_size(cell)
    uu = np.zeros(len(p))
    vv = np.zeros(len(p))
    for ax in range(3):
        m = axis == ax
        if not m.any():
            continue
        av, bv = a[m], b[m]
        sa, sb = float(np.ptp(av)), float(np.ptp(bv))
        fill_a, fill_b = sa >= 0.3 * big, sb >= 0.3 * big
        if fill_a and fill_b:
            scale_a, scale_b = sa, sb
        elif fill_a:
            scale_a, scale_b = sa, max(sb, ch * sa / cw)
        elif fill_b:
            scale_a, scale_b = max(sa, cw * sb / ch), sb
        else:
            scale_a = scale_b = big
        uu[m] = (av - av.min()) / max(scale_a, 1e-9)
        vv[m] = (bv - bv.min()) / max(scale_b, 1e-9)
    _set_uv(obj, cell, uu, vv)


def uv_planar(obj: bpy.types.Object, cell: str, u_axis=(1, 0, 0), v_axis=(0, 1, 0), bounds=None) -> None:
    """Projection plane : U le long de `u_axis`, V le long de `v_axis` (repère local), étirée sur
    `bounds` (u_min, u_max, v_min, v_max ; défaut : l'étendue de la pièce). Les tranches prennent
    la couleur du bord de la case."""
    p, _, _, _ = _loops(obj)
    u = p @ np.asarray(u_axis, dtype=np.float64)
    v = p @ np.asarray(v_axis, dtype=np.float64)
    if bounds is None:
        bounds = (u.min(), u.max(), v.min(), v.max())
    umin, umax, vmin, vmax = bounds
    _set_uv(obj, cell, (u - umin) / max(umax - umin, 1e-9), (v - vmin) / max(vmax - vmin, 1e-9))


def _wrap_faces(uu, lp, n_faces) -> np.ndarray:
    """Corrige la couture d'un déroulé angulaire : sur une face qui chevauche 0/1, les U < 0,5
    passent à U + 1 (puis rabattus à 1 par le cadrage dans la case)."""
    out = uu.copy()
    for poly in range(n_faces):
        idx = np.nonzero(lp == poly)[0]
        if idx.size and out[idx].max() - out[idx].min() > 0.5:
            out[idx] = np.where(out[idx] < 0.5, out[idx] + 1.0, out[idx])
    return np.clip(out, 0.0, 1.0)


def uv_cyl(obj: bpy.types.Object, cell: str) -> None:
    """Cylindre construit le long de Z local : U = angle, V = hauteur ; bouchons en projection
    plane."""
    p, nrm, lp, _ = _loops(obj)
    z = p[:, 2]
    zlo, zspan = z.min(), max(np.ptp(z), 1e-9)
    rmax = max(np.abs(p[:, :2]).max(), 1e-9)
    ang = (np.arctan2(p[:, 1], p[:, 0]) / (2 * np.pi)) % 1.0
    uu = _wrap_faces(ang, lp, len(nrm))
    vv = (z - zlo) / zspan
    cap = np.abs(nrm[lp][:, 2]) > 0.7
    uu = np.where(cap, 0.5 + 0.5 * p[:, 0] / rmax, uu)
    vv = np.where(cap, 0.5 + 0.5 * p[:, 1] / rmax, vv)
    _set_uv(obj, cell, uu, vv)


def uv_tube(obj: bpy.types.Object, cell: str, segments: int) -> None:
    """Tube de `char_kit.tube` (sommets anneau par anneau) : U = rang dans l'anneau, V = abscisse
    le long du tube."""
    p, nrm, lp, lv = _loops(obj)
    nverts = len(obj.data.vertices)
    rings = nverts // segments
    co = np.empty(nverts * 3)
    obj.data.vertices.foreach_get("co", co)
    co = co.reshape(-1, 3)
    centers = co.reshape(rings, segments, 3).mean(axis=1)
    seg = np.linalg.norm(np.diff(centers, axis=0), axis=1)
    s = np.concatenate([[0.0], np.cumsum(seg)])
    s = s / max(s[-1], 1e-9)
    ring = lv // segments
    k = lv % segments
    uu = _wrap_faces(k / segments, lp, len(nrm))
    _set_uv(obj, cell, uu, s[ring])


# --- Assemblage ----------------------------------------------------------------------------------


class Builder:
    """Accumule les pièces avec leur mode d'UV : `box`, `planar`, `cyl`, `tube` (case de l'atlas)
    ou `lib` (texture répétable, argument = taille de tuile en m). Compatible avec
    `char_kit.seg_cyl` (`b.add(obj, mode, arg)`)."""

    def __init__(self, m: dict, coll):
        self.m = m
        self.coll = coll
        self.parts: list = []

    def add(self, obj, mode: str, *args, **kw):
        self.parts.append((obj, mode, args, kw))
        return obj

    def box(self, obj, cell):
        return self.add(obj, "box", cell)

    def planar(self, obj, cell, u_axis=(1, 0, 0), v_axis=(0, 1, 0), bounds=None):
        return self.add(obj, "planar", cell, u_axis, v_axis, bounds)

    def cyl(self, obj, cell):
        return self.add(obj, "cyl", cell)

    def tube(self, obj, cell, segments):
        return self.add(obj, "tube", cell, segments)

    def lib(self, obj, tile):
        return self.add(obj, "lib", tile)

    def finalize(self) -> list:
        out = []
        for i, (obj, mode, args, kw) in enumerate(self.parts):
            if mode == "box":
                uv_box(obj, *args)
            elif mode == "planar":
                uv_planar(obj, *args)
            elif mode == "cyl":
                uv_cyl(obj, *args)
            elif mode == "tube":
                uv_tube(obj, *args)
            elif mode == "lib":
                apply_transform(obj)
                c.uv_box(obj, args[0], offset=((i * 0.37) % 1.0, (i * 0.61) % 1.0))
            else:
                raise ValueError(mode)
            out.append(obj)
        return out


def seg_cyl(b: Builder, name, radius, start, end, cell, segments=12, r2=None, bevel=0.0, side=(1, 0, 0)):
    """Cylindre de `start` à `end`, UV cylindriques dans `cell`."""
    length = (Vector(end) - Vector(start)).length
    obj = cyl(name, radius, length, b.m["gear"], b.coll, segments=segments, r2=r2, bevel=bevel)
    obj.data.transform(Matrix.Translation((0, 0, length / 2)))
    orient(obj, start, end, side=side)
    return b.cyl(obj, cell)


def finish(parts: list, prefix: str, center_base: bool = True) -> list:
    """Fusion par matériau dans la collection EXPORT, transformations appliquées, puis origine
    au centre de la base (centre de l'emprise au sol, z minimal = 0)."""
    exports = c.collection("EXPORT")
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        merged = c.join_by_material(parts, prefix, exports)
    for obj in merged:
        apply_transform(obj)
    if center_base:
        recenter(merged)
    for coll in bpy.data.collections:
        if coll.name.startswith("ASSET_"):
            coll.hide_render = True
            coll.hide_viewport = True
    return merged


def recenter(objs) -> None:
    """Origine au centre de la base : centre de l'emprise au sol en (0, 0), z minimal à 0
    (maillages à l'identité)."""
    lo, hi = bounds(objs)
    shift = Vector((-(lo[0] + hi[0]) / 2, -(lo[1] + hi[1]) / 2, -lo[2]))
    for obj in objs:
        obj.data.transform(Matrix.Translation(shift))


def _verts(obj) -> np.ndarray:
    co = np.empty(len(obj.data.vertices) * 3)
    obj.data.vertices.foreach_get("co", co)
    return co.reshape(-1, 3)


def bounds(objs) -> tuple[np.ndarray, np.ndarray]:
    pts = np.concatenate([_verts(o) for o in objs])
    return pts.min(axis=0), pts.max(axis=0)


# --- Formes --------------------------------------------------------------------------------------


def apply_modifiers(obj: bpy.types.Object) -> None:
    """Applique les modificateurs dans le maillage (la fusion par matériau les perdrait)."""
    dg = bpy.context.evaluated_depsgraph_get()
    ev = obj.evaluated_get(dg)
    mesh = bpy.data.meshes.new_from_object(ev)
    old = obj.data
    obj.modifiers.clear()
    obj.data = mesh
    mesh.name = old.name
    if old.users == 0:
        bpy.data.meshes.remove(old)


def soft_box(name: str, size, center, mat, coll, cuts: int = 2, radius: float = 0.0, deform=None,
             subsurf: int = 1, smooth_angle: float = 70.0) -> bpy.types.Object:
    """Pavé `size` subdivisé (`cuts` coupes par arête), arêtes arrondies au rayon `radius` (m),
    déformé par `deform(x, y, z) -> (x, y, z)` (repère local, mètres), puis lissé par
    subdivision : tissu plié, emballage souple, coussin."""
    half = np.array(size, dtype=np.float64) / 2
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=2.0)
    if cuts > 0:
        bmesh.ops.subdivide_edges(bm, edges=list(bm.edges), cuts=cuts, use_grid_fill=True)
    r = min(radius, float(half.min()) * 0.98)
    for v in bm.verts:
        p = np.array(v.co, dtype=np.float64) * half
        if r > 0:
            # Projection sur le pavé arrondi : ce qui dépasse le noyau rétréci de `r` est ramené
            # à la distance `r` de ce noyau (arêtes et coins en quarts de cylindre et de sphère).
            q = np.clip(p, -(half - r), half - r)
            d = p - q
            ln = float(np.linalg.norm(d))
            if ln > 1e-9:
                p = q + d / ln * r
        if deform is not None:
            p = deform(*p)
        v.co = Vector(tuple(p))
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    if subsurf > 0:
        mod = obj.modifiers.new("subsurf", "SUBSURF")
        mod.levels = subsurf
        mod.render_levels = subsurf
        apply_modifiers(obj)
    obj.location = center
    auto_smooth(obj, smooth_angle)
    return obj


def gear(name: str, r_tip: float, r_root: float, teeth: int, thickness: float, hole: float, mat, coll,
         center=(0.0, 0.0, 0.0)) -> bpy.types.Object:
    """Roue dentée percée, à plat (axe Z), centrée : couronne de `teeth` dents trapézoïdales."""
    bm = bmesh.new()
    n = teeth * 4
    outer, inner = [], []
    for i in range(n):
        k, q = divmod(i, 4)
        base = 2 * math.pi * k / teeth
        step = 2 * math.pi / teeth
        # Pied, flanc montant, sommet, flanc descendant.
        a = base + step * (0.0, 0.18, 0.38, 0.56)[q]
        r = (r_root, r_tip, r_tip, r_root)[q]
        outer.append((r * math.cos(a), r * math.sin(a)))
        inner.append((hole * math.cos(a), hole * math.sin(a)))
    h = thickness / 2
    to = [bm.verts.new((x, y, h)) for x, y in outer]
    bo = [bm.verts.new((x, y, -h)) for x, y in outer]
    ti = [bm.verts.new((x, y, h)) for x, y in inner]
    bi = [bm.verts.new((x, y, -h)) for x, y in inner]
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((ti[i], to[i], to[j], ti[j]))
        bm.faces.new((bi[j], bo[j], bo[i], bi[i]))
        bm.faces.new((to[i], bo[i], bo[j], to[j]))
        bm.faces.new((ti[j], bi[j], bi[i], ti[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    obj.location = center
    auto_smooth(obj, 40.0)
    return obj


def flap(name: str, x0: float, x1: float, half_w: float, z: float, thickness: float, teeth: int, mat, coll,
         tooth: float = 0.0025) -> bpy.types.Object:
    """Languette soudée d'un emballage, de x0 (attache) à x1 (bord dentelé), largeur 2 × half_w,
    à la hauteur z ; sommets dans le repère du modèle (UV partagées avec le paquet)."""
    sgn = 1.0 if x1 > x0 else -1.0
    pts = [(x0, -half_w)]
    for i in range(teeth + 1):
        y = -half_w + 2 * half_w * i / teeth
        pts.append((x1 - (0.0 if i % 2 == 0 else sgn * tooth), y))
    pts.append((x0, half_w))
    if sgn < 0:
        pts.reverse()
    bm = bmesh.new()
    top = [bm.verts.new((x, y, z + thickness / 2)) for x, y in pts]
    bot = [bm.verts.new((x, y, z - thickness / 2)) for x, y in pts]
    bm.faces.new(top)
    bm.faces.new(list(reversed(bot)))
    m = len(pts)
    for i in range(m):
        j = (i + 1) % m
        bm.faces.new((top[i], bot[i], bot[j], top[j]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bmesh.ops.triangulate(bm, faces=[f for f in bm.faces if len(f.verts) > 4])
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    return obj


def hex_prism(name: str, radius: float, height: float, mat, coll, hole: float = 0.0) -> bpy.types.Object:
    """Écrou (prisme hexagonal le long de Z, centré), percé si `hole` > 0 (anneau)."""
    if hole <= 0:
        return cyl(name, radius, height, mat, coll, segments=6)
    bm = bmesh.new()
    h = height / 2
    o = [(radius * math.cos(math.pi / 3 * i), radius * math.sin(math.pi / 3 * i)) for i in range(6)]
    to = [bm.verts.new((x, y, h)) for x, y in o]
    bo = [bm.verts.new((x, y, -h)) for x, y in o]
    ti = [bm.verts.new((hole * x / radius, hole * y / radius, h)) for x, y in o]
    bi = [bm.verts.new((hole * x / radius, hole * y / radius, -h)) for x, y in o]
    for i in range(6):
        j = (i + 1) % 6
        bm.faces.new((ti[i], to[i], to[j], ti[j]))
        bm.faces.new((bi[j], bo[j], bo[i], bi[i]))
        bm.faces.new((to[i], bo[i], bo[j], to[j]))
        bm.faces.new((ti[j], bi[j], bi[i], ti[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    auto_smooth(obj, 40.0)
    return obj
