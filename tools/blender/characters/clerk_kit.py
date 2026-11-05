"""Outils des PNJ de comptoir (lot E de M9) : tête, oreilles, mains, pièces de vêtements.

S'appuie sur `char_kit` (tubes, coques, boîtes arrondies, `Builder`) sans le modifier :
- `head_mesh` : maillage de la tête échantillonné sur `clerk_face` (grille uniforme en UV, donc
  dense au visage, lâche derrière le crâne), UV déjà posées dans la case du visage de l'atlas
  `char_clerk` ;
- `hand` : main en tubes (paume, quatre doigts à trois phalanges, pouce), à plat sur un comptoir
  ou au repos, posée par une matrice ;
- `torso_patch` : pan de vêtement conforme au torse (gilet, bavette de tablier), avec épaisseur ;
- `ClerkBuilder` : le `Builder` de `char_kit` plus deux modes d'UV propres à l'atlas
  `char_clerk` (`rect` : projection boîte dans une case, `keep` : UV déjà posées).
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

import char_kit as k  # noqa: E402
import clerk_face as cf  # noqa: E402
import common as c  # noqa: E402


# --- Tête ------------------------------------------------------------------------------------


def head_mesh(h: cf.Head, name: str, mat, coll, rect, nu: int = 48, nv: int = 44) -> bpy.types.Object:
    """Tête (peau, cheveux, barbe, cou) en un maillage fermé, repère local de `clerk_face`.

    `rect` : (u0, v0, u1, v1) de la case du visage dans l'atlas."""
    u0, v0, u1, v1 = rect
    us = np.arange(nu) / nu
    vs = np.arange(1, nv) / nv
    U, V = np.meshgrid(us, vs)
    a, e = cf.angles_from_uv(U, V)
    x, y, z, _ = cf.surface(h, a, e)
    tx, ty, tz, _ = cf.surface(h, np.array([0.0]), np.array([cf.E_HI - 1e-6]))
    bx, by, bz, _ = cf.surface(h, np.array([0.0]), np.array([cf.E_LO + 1e-6]))
    bm = bmesh.new()
    uv_layer = bm.loops.layers.uv.new(c.UV_TEXTURE)
    rows = []
    for j in range(nv - 1):
        rows.append([bm.verts.new((float(x[j, i]), float(y[j, i]), float(z[j, i]))) for i in range(nu)])
    top = bm.verts.new((0.0, 0.0, float(tz[0])))
    bot = bm.verts.new((0.0, 0.0, float(bz[0])))

    def at(uu, vv):
        return (u0 + uu * (u1 - u0), v0 + vv * (v1 - v0))

    def face(verts, uvs):
        f = bm.faces.new(verts)
        for loop, uv in zip(f.loops, uvs):
            loop[uv_layer].uv = at(*uv)
        f.smooth = True
        return f

    for j in range(nv - 2):
        va, vb = vs[j], vs[j + 1]
        for i in range(nu):
            i2 = (i + 1) % nu
            ua, ub = us[i], (us[i2] if i2 else 1.0)
            face((rows[j][i], rows[j][i2], rows[j + 1][i2], rows[j + 1][i]),
                 ((ua, va), (ub, va), (ub, vb), (ua, vb)))
    for i in range(nu):
        i2 = (i + 1) % nu
        ua, ub = us[i], (us[i2] if i2 else 1.0)
        face((rows[-1][i], rows[-1][i2], top), ((ua, vs[-1]), (ub, vs[-1]), ((ua + ub) / 2, 1.0)))
        face((rows[0][i2], rows[0][i], bot), ((ub, vs[0]), (ua, vs[0]), ((ua + ub) / 2, 0.0)))
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    return obj


def place_head(obj, eye_world, yaw_deg: float = 0.0, pitch_deg: float = 0.0, roll_deg: float = 0.0,
               pivot=(0.0, -0.02, -0.16), scale: float = 1.0) -> None:
    """Pose la tête : centre local aux coordonnées `eye_world`, petites rotations autour d'un
    pivot au bas du cou (local), échelle (tête un peu plus forte ou plus fine), appliquées au
    maillage."""
    pv = Vector(pivot)
    rot = (Matrix.Rotation(math.radians(yaw_deg), 4, "Z") @ Matrix.Rotation(math.radians(pitch_deg), 4, "X")
           @ Matrix.Rotation(math.radians(roll_deg), 4, "Y"))
    m = (Matrix.Translation(Vector(eye_world)) @ Matrix.Translation(pv) @ rot @ Matrix.Translation(-pv)
         @ Matrix.Scale(scale, 4))
    obj.data.transform(m)


def ear(h: cf.Head, name: str, mat, coll, rect, side: float) -> bpy.types.Object:
    """Oreille (repère local de la tête), côté +1 (droite, +X) ou −1. Plaque arrondie, inclinée
    vers l'arrière et décollée du crâne par son bord arrière. UV : projection depuis le centre de
    la tête dans la case du visage (la zone peinte sous l'oreille : même peau, rougeur, conque)."""
    zs = [-0.031, -0.021, -0.007, 0.008, 0.021, 0.029]
    ry = [0.0075, 0.0105, 0.0140, 0.0152, 0.0132, 0.0068]
    rx = [0.0038, 0.0048, 0.0054, 0.0054, 0.0048, 0.0030]
    oy = [0.0010, 0.0000, -0.0015, -0.0025, -0.0032, -0.0032]
    pts = [(0.0, 0.0, z) for z in zs]
    obj = k.tube(name, pts, list(zip(rx, ry)), mat, coll, segments=10, ref=(1, 0, 0),
                 offsets=[(0.0, o) for o in oy])
    xh = cf.EAR_X * h.width * side
    m = (Matrix.Translation((xh, cf.EAR_Y, cf.EAR_Z)) @ Matrix.Rotation(math.radians(14), 4, "X")
         @ Matrix.Rotation(math.radians(22 * side), 4, "Z") @ Matrix.Translation((0.0, -0.012, 0.0)))
    obj.data.transform(m)
    # UV : direction depuis le centre de la tête.
    mesh = obj.data
    co = np.array([v.co[:] for v in mesh.vertices])
    lv = np.empty(len(mesh.loops), dtype=np.int64)
    mesh.loops.foreach_get("vertex_index", lv)
    p = co[lv]
    a = cf.azimuth(p[:, 0], p[:, 1])
    e = np.arctan2(p[:, 2], np.hypot(p[:, 0], p[:, 1]))
    uu, vv = cf.uv_from_angles(a, e)
    u0, v0, u1, v1 = rect
    layer = c._uv_layer(mesh)
    layer.data.foreach_set("uv", np.stack([u0 + uu * (u1 - u0), v0 + vv * (v1 - v0)], 1).ravel())
    return obj


# --- UV dans l'atlas `char_clerk` -------------------------------------------------------------


def uv_rect_fit(obj: bpy.types.Object, rect) -> None:
    """Projection boîte dans une case de l'atlas (comme `char_kit.uv_cell`, case explicite) : la
    pièce entière tient dans la case, ses arêtes tombent sur le bord (couture, bord usé)."""
    u0, v0, u1, v1 = rect
    p, nrm, lp = k._loops(obj)
    n = nrm[lp]
    axis = np.argmax(np.abs(n), axis=1)
    sign = np.sign(n[np.arange(len(n)), axis])
    a = np.where(axis == 0, sign * p[:, 1], np.where(axis == 1, -sign * p[:, 0], p[:, 0]))
    b = np.where(axis == 2, sign * p[:, 1], p[:, 2])
    big = max(float(np.ptp(p[:, i])) for i in range(3)) or 1.0
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
    c._uv_layer(obj.data).data.foreach_set("uv", uv.ravel())


def uv_rect_tube(obj: bpy.types.Object, rect, segments: int, along_turns: float = 1.0) -> None:
    """UV d'un tube de `char_kit.tube` (anneaux de `segments` sommets, dans l'ordre) : U autour,
    V le long du chemin (longueur réelle, `along_turns` fois la case sur toute la longueur)."""
    u0, v0, u1, v1 = rect
    mesh = obj.data
    co = np.array([v.co[:] for v in mesh.vertices])
    n_rings = len(co) // segments
    centers = co[: n_rings * segments].reshape(n_rings, segments, 3).mean(axis=1)
    seg = np.r_[0.0, np.cumsum(np.linalg.norm(np.diff(centers, axis=0), axis=1))]
    tot = seg[-1] or 1.0
    uv = np.zeros((len(mesh.loops), 2))
    for poly in mesh.polygons:
        idx = list(poly.vertices)
        if len(idx) != 4:
            for li in poly.loop_indices:
                vi = mesh.loops[li].vertex_index
                uv[li] = (0.5, (seg[min(vi // segments, n_rings - 1)] / tot) * along_turns)
            continue
        ks = [vi % segments for vi in idx]
        wrap = max(ks) - min(ks) > 1
        for li, vi in zip(poly.loop_indices, idx):
            kk = vi % segments
            if wrap and kk == 0:
                kk = segments
            uv[li] = (kk / segments, (seg[vi // segments] / tot) * along_turns)
    uv[:, 1] = np.clip(uv[:, 1], 0.0, 1.0)
    uv = np.stack([u0 + uv[:, 0] * (u1 - u0), v0 + uv[:, 1] * (v1 - v0)], 1)
    c._uv_layer(mesh).data.foreach_set("uv", uv.ravel())


class ClerkBuilder(k.Builder):
    """`char_kit.Builder` plus les modes de l'atlas `char_clerk` :
    - `rect` (arg = (u0, v0, u1, v1)) : projection boîte ajustée à la case ;
    - `tube` (arg = (rect, segments)) : tube déroulé dans la case ;
    - `keep` : UV déjà posées (tête, oreilles).
    Toutes les transformations sont appliquées aux maillages (nœuds à l'identité)."""

    def finalize(self):
        out = []
        for i, (obj, uv, arg) in enumerate(self.parts):
            if uv == "lib":
                k.uv_lib(obj, arg, offset=((i * 0.37) % 1.0, (i * 0.61) % 1.0))
            elif uv == "cell":
                k.uv_cell_shell(obj, arg)
            elif uv == "cyl":
                k.uv_cell_cyl(obj, arg)
            elif uv == "rect":
                k.apply_transform(obj)
                uv_rect_fit(obj, arg)
            elif uv == "tube":
                uv_rect_tube(obj, *arg)
            if "sh_shell_center" in obj:
                del obj["sh_shell_center"]
            k.apply_transform(obj)
            out.append(obj)
        return out

    def rect(self, obj, rect):
        return self.add(obj, "rect", rect)

    def keep(self, obj):
        return self.add(obj, "keep")


def apply_modifiers(obj: bpy.types.Object) -> None:
    """Fige les modificateurs dans le maillage (épaisseur des pans de vêtement) avant les UV."""
    dg = bpy.context.evaluated_depsgraph_get()
    mesh = bpy.data.meshes.new_from_object(obj.evaluated_get(dg))
    old = obj.data
    obj.modifiers.clear()
    obj.data = mesh
    bpy.data.meshes.remove(old)


# --- Torse et vêtements ---------------------------------------------------------------------


class Torso:
    """Torse en anneaux (z, demi-largeur, demi-profondeur, décalage avant), interpolés."""

    def __init__(self, rings):
        self.rings = sorted(rings)

    def at(self, z: float):
        r = self.rings
        if z <= r[0][0]:
            return tuple(r[0][1:])
        for (z0, *a), (z1, *b) in zip(r[:-1], r[1:]):
            if z0 <= z <= z1:
                t = (z - z0) / (z1 - z0)
                return tuple(x + (y - x) * t for x, y in zip(a, b))
        return tuple(r[-1][1:])

    def point(self, z: float, deg: float, off: float = 0.0):
        """Point de la surface (angle 0 = +X, 90 = +Y devant), décollé de `off`."""
        rx, ry, oy = self.at(z)
        a = math.radians(deg)
        return (math.cos(a) * (rx + off), oy + math.sin(a) * (ry + off), z)

    def tube(self, name, mat, coll, z0: float, z1: float, segments: int = 20, off: float = 0.0,
             cap_start: bool = True, cap_end: bool = True):
        zs = [z0] + [r[0] for r in self.rings if z0 < r[0] < z1] + [z1]
        pts, radii, offs = [], [], []
        for z in zs:
            rx, ry, oy = self.at(z)
            pts.append((0.0, 0.0, z))
            radii.append((rx + off, ry + off))
            offs.append((0.0, oy))
        return k.tube(name, pts, radii, mat, coll, segments=segments, offsets=offs,
                      cap_start=cap_start, cap_end=cap_end)


def torso_patch(name, mat, coll, torso: Torso, z0: float, z1: float, a0: float, a1: float, off: float,
                thickness: float, nz: int = 12, na: int = 24, keep=None, span=None) -> bpy.types.Object:
    """Pan de vêtement conforme au torse (gilet, bavette), d'angle a0 → a1 (degrés, sens
    trigonométrique vu de dessus), de z0 à z1, décollé de `off`, épais de `thickness` (vers
    l'intérieur). `span(z)` → (a0, a1) : plage d'angles propre à chaque rangée (bords lisses :
    encolure en V) ; `keep(angle_deg, z)` : garder la face."""
    bm = bmesh.new()
    grid = []
    for j in range(nz + 1):
        z = z0 + (z1 - z0) * j / nz
        r0, r1 = span(z) if span is not None else (a0, a1)
        grid.append([bm.verts.new(torso.point(z, r0 + (r1 - r0) * i / na, off)) for i in range(na + 1)])
    for j in range(nz):
        for i in range(na):
            ac = a0 + (a1 - a0) * (i + 0.5) / na
            zc = z0 + (z1 - z0) * (j + 0.5) / nz
            if keep is not None and not keep(ac % 360.0, zc):
                continue
            bm.faces.new((grid[j][i], grid[j][i + 1], grid[j + 1][i + 1], grid[j + 1][i]))
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    mod = obj.modifiers.new("epaisseur", "SOLIDIFY")
    mod.thickness = thickness
    mod.offset = -1.0
    mod.use_even_offset = True
    apply_modifiers(obj)
    k._auto_smooth(obj, 40.0)
    return obj


# --- Mains -----------------------------------------------------------------------------------

# Doigts (index → auriculaire) : décalage latéral à la jointure (× côté du pouce), recul de la
# jointure, longueurs des phalanges, rayons (base, bout), écart angulaire (degrés).
FINGERS = [
    (0.0265, 0.093, (0.042, 0.025, 0.020), (0.0094, 0.0074), -4.0),
    (0.0090, 0.097, (0.046, 0.029, 0.021), (0.0097, 0.0077), 0.0),
    (-0.0085, 0.094, (0.044, 0.027, 0.020), (0.0091, 0.0071), 4.0),
    (-0.0255, 0.087, (0.034, 0.021, 0.018), (0.0081, 0.0063), 9.0),
]
POSES = {
    # Flexion des doigts (jointure, phalange, phalangette) en degrés ; pouce : articulations
    # (métacarpe, phalange, bout) en repère de main, pouce du côté +X.
    "flat": ((6.0, 13.0, 10.0), ((0.039, 0.043, -0.014), (0.046, 0.069, -0.020), (0.045, 0.092, -0.023))),
    "relaxed": ((20.0, 34.0, 22.0), ((0.037, 0.041, -0.022), (0.036, 0.064, -0.035), (0.027, 0.080, -0.043))),
    "grip": ((55.0, 75.0, 40.0), ((0.034, 0.040, -0.026), (0.022, 0.060, -0.040), (0.006, 0.070, -0.044))),
}


def _rot(v: Vector, axis: Vector, deg: float) -> Vector:
    return Matrix.Rotation(math.radians(deg), 3, axis) @ v


def hand(b: ClerkBuilder, name: str, mat_key: str, uv, thumb_side: float, pose: str = "flat",
         scale: float = 1.0, glove: bool = False):
    """Main en repère local (poignet à l'origine, doigts vers +Y, dos de la main vers +Z, pouce
    du côté `thumb_side` : −1 pour une main droite posée à plat). Renvoie les objets, à poser
    avec `place`. `uv` : (mode, argument) du `Builder`."""
    m, coll = b.m[mat_key], b.coll
    s = scale * (1.06 if glove else 1.0)
    objs = []
    palm_r = [(0.0, (0.029, 0.0165)), (0.03, (0.036, 0.016)), (0.065, (0.041, 0.0138)), (0.092, (0.042, 0.0122))]
    pts = [(0.0, y * s, 0.0) for y, _ in palm_r]
    radii = [(rx * s, rz * s) for _, (rx, rz) in palm_r]
    offs = [(thumb_side * 0.002 * s, 0.0), (thumb_side * 0.004 * s, -0.001 * s), (0.0, 0.0), (0.0, 0.001 * s)]
    objs.append(k.tube(f"{name}_palm", pts, radii, m, coll, segments=10, ref=(1, 0, 0), offsets=offs))
    curl, thumb = POSES[pose]
    g = 1.08 if glove else 1.0
    for fi, (dx, ky, lengths, (r0, r1), spread) in enumerate(FINGERS):
        base = Vector((thumb_side * dx * s, ky * s, -0.001 * s))
        d = _rot(Vector((0, 1, 0)), Vector((0, 0, 1)), -thumb_side * spread)
        path = [base - d * 0.012 * s, base]
        cur = base.copy()
        dd = d.copy()
        axis = dd.cross(Vector((0, 0, 1))).normalized()
        for L, bend in zip(lengths, curl):
            dd = _rot(dd, axis, -bend)
            mid = cur + dd * (L * s * 0.475)
            cur = cur + dd * (L * s * 0.95)
            path += [mid.copy(), cur.copy()]
        n = len(path)
        rr = [(r0 + (r1 - r0) * (i / (n - 1))) * s * g for i in range(n)]
        rr = [(r, r * 0.9) for r in rr]
        objs.append(k.tube(f"{name}_finger{fi}", [tuple(p) for p in path], rr, m, coll, segments=7, ref=tuple(axis)))
    # Pouce : de l'éminence (dans la paume) le long du flanc de l'index, articulations de la pose.
    joints = [(0.021, 0.015, -0.006)] + list(thumb)
    path = [Vector((thumb_side * x * s, y * s, z * s)) for x, y, z in joints]
    path = k.spline([tuple(p) for p in path], 2)
    n = len(path)
    rr = [(0.0135 + (0.0086 - 0.0135) * i / (n - 1)) * s * g for i in range(n)]
    objs.append(k.tube(f"{name}_thumb", path, [(r, r * 0.9) for r in rr], m, coll, segments=8, ref=(0, 0, 1)))
    for o in objs:
        b.add(o, *uv)
    return objs


def place(objs, origin, forward, up) -> None:
    """Pose des objets construits en repère local (+Y vers `forward`, +Z vers `up`) à `origin`."""
    y = Vector(forward).normalized()
    z = Vector(up)
    z = (z - y * z.dot(y)).normalized()
    x = y.cross(z)
    rot = Matrix((x, y, z)).transposed().to_4x4()
    mat = Matrix.Translation(Vector(origin)) @ rot
    for o in objs:
        o.matrix_basis = mat @ o.matrix_basis


def two_bone(shoulder, wrist, l1: float, l2: float, pole):
    """Coude d'un bras à deux segments (IK analytique), tourné vers `pole`. Renvoie (coude,
    poignet atteint)."""
    s, w, p = Vector(shoulder), Vector(wrist), Vector(pole)
    d = w - s
    dist = min(d.length, (l1 + l2) * 0.999)
    dn = d.normalized()
    a = (l1 * l1 + dist * dist - l2 * l2) / (2 * dist)
    hgt = math.sqrt(max(l1 * l1 - a * a, 0.0))
    pv = p - s
    pv = (pv - dn * pv.dot(dn)).normalized()
    elbow = s + dn * a + pv * hgt
    return tuple(elbow), tuple(s + dn * dist)


def lerp3(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def shoe(b: ClerkBuilder, tag: str, ankle, yaw_deg: float, upper_key: str, upper_uv, sole_key: str, sole_uv,
         boot: bool = False, scale: float = 1.0):
    """Chaussure de travail (tige basse ou montante) : empeigne arrondie, bout relevé, semelle
    épaisse. Repère : sol sous la cheville, pointe vers +Y tournée de `yaw_deg`."""
    m, coll = b.m, b.coll
    s = scale
    ax, ay, _ = ankle
    rot = Matrix.Rotation(math.radians(yaw_deg), 4, "Z")

    def put(obj, local):
        obj.matrix_basis = Matrix.Translation((ax, ay, 0.0)) @ rot @ Matrix.Translation(local)
        return obj

    foot = k.rounded_box(f"shoe_{tag}", (0.092 * s, 0.24 * s, 0.064 * s), (0, 0, 0), m[upper_key], coll,
                         bevel=0.027 * s, segments=2)
    b.add(put(foot, (0, 0.052 * s, 0.050 * s)), *upper_uv)
    toe = k.rounded_box(f"shoe_toe_{tag}", (0.086 * s, 0.07 * s, 0.040 * s), (0, 0, 0), m[upper_key], coll,
                        bevel=0.017 * s, segments=2)
    b.add(put(toe, (0, 0.135 * s, 0.034 * s)), *upper_uv)
    sole = k.rounded_box(f"sole_{tag}", (0.098 * s, 0.252 * s, 0.020 * s), (0, 0, 0), m[sole_key], coll,
                         bevel=0.006 * s)
    b.add(put(sole, (0, 0.054 * s, 0.010 * s)), *sole_uv)
    if boot:
        # Tige ouverte en haut : le bas du pantalon y rentre.
        shaft = k.tube(f"boot_shaft_{tag}", [(0, 0, 0.05 * s), (0, 0.0, 0.15 * s), (0, -0.004, 0.22 * s)],
                       [(0.056 * s, 0.062 * s), (0.054 * s, 0.058 * s), (0.060 * s, 0.063 * s)], m[upper_key], coll,
                       segments=12, cap_end=False)
        b.add(put(shaft, (0, 0, 0)), *upper_uv)
    else:
        collar = k.tube(f"shoe_collar_{tag}", [(0, -0.006, 0.065 * s), (0, -0.004, 0.10 * s)],
                        [(0.046 * s, 0.054 * s), (0.044 * s, 0.050 * s)], m[upper_key], coll, segments=10)
        b.add(put(collar, (0, 0, 0)), *upper_uv)
