"""Pièces de hangar du kit d'intérieur (M9, lot A), direction « futur usé ».

Le spatioport par défaut (docs/m9-retours.md §2, §3.8) accueille le vaisseau dans un hangar :
il repose dans un **berceau** creusé dans le sol, son pont au niveau du sol (z = 0), et l'on
sort du sas par une **passerelle plane** jusqu'au portail `hangar__@ship`. Ce module fournit
les pièces de ce hangar, avec la même API que `ship_interior` (triplet (visibles, colliders,
repères), origine au centre de la base, face utile vers +Y quand `facing` = 0, `detail=` pour
le décor par niveau de qualité) :

- `hangar_shell(...)` → {"visual", "structure", "colliders", "marks"} : coque haute (style
  `hangar` de `room_shell`), fermes de toit en treillis, pannes, poutres de roulement, grands
  projecteurs (`LIGHT_`), grande porte de baie facultative ;
- `docking_cradle(...)` : fosse bordée de garde-corps, bras de maintien, sabots, marquages ;
- `gangway(...)` : passerelle plane à garde-corps ;
- `gantry_crane(...)` : portique roulant sur rails, palan et crochet ;
- `cargo_container(...)`, `crate_stack(...)` : fret ;
- `fuel_bollard(...)` : borne de carburant ou d'énergie, son tuyau au sol.

Ces fonctions sont aussi accessibles depuis `ship_interior` (`kit.hangar_shell(...)`).
Inscriptions : pochoirs génériques de la trim sheet, aucun nom ni logo.
Repères Blender : X = tribord / est, Y = avant / nord, Z = haut.
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import bpy  # noqa: E402,F401
from mathutils import Vector  # noqa: E402

import common as c  # noqa: E402
import ship_interior as kit  # noqa: E402
from ship_interior import _FRONT, _box, _cyl, _finish, _ori, _place, _strut  # noqa: E402

# Projecteurs de hangar : sodium chaud (contraste avec le froid des vaisseaux).
SODIUM = [1.0, 0.74, 0.46]


# --- Coque ----------------------------------------------------------------------------------------


def hangar_shell(
    prefix: str,
    x0: float,
    x1: float,
    y0: float,
    y1: float,
    height: float,
    openings: list,
    m: dict,
    coll,
    *,
    z0: float = 0.0,
    bay: float = 6.0,
    truss_depth: float = 1.6,
    floor_holes=(),
    bay_door=None,
    lamps: bool = True,
    lamps_per_truss: int | None = None,
    lamp_lumens: float = 30000.0,
    lamp_range: float = 26.0,
    lamp_color=None,
    runway: bool = True,
    purlin_detail: int = 1,
    web_detail: int = 0,
) -> dict:
    """Coque de hangar : `room_shell` en style `hangar` (sol de béton percé de `floor_holes`,
    soubassement sombre, colonnes en H lourdes tous les `bay` m, bandeaux ambrés à 3 m,
    `openings` comme `room_shell` : portes DOOR_W × DOOR_H, passages, fenêtres), plus :

    - une ferme de toit en treillis (Warren) à chaque travée, au-dessus des colonnes, en travers
      du petit axe : membrures sombres, montants et diagonales de métal brut (`web_detail`),
      platines d'appui ; pannes longitudinales (`purlin_detail`, 1 par défaut : Moyen et Haut) ;
    - `runway` : poutres de roulement le long des grands murs, sur consoles (pont roulant) ;
    - `lamps` : projecteurs suspendus sous chaque ferme (`lamps_per_truss`, défaut selon la
      portée), lentille `sh_light_amber` remplacée au bake par sa lampe `LIGHT_` (`lamp_lumens`,
      `lamp_range`, `lamp_color`, défaut sodium) ;
    - `bay_door` = (côté, position, largeur, hauteur) : grande porte de baie fermée plaquée sur
      ce mur (décor : le mur et son collider restent ; montants à colliders).

    Renvoie {"visual", "structure", "colliders", "marks"} : `structure` (version simplifiée de la
    cellule) garde la coque, les membrures des fermes et la porte de baie."""
    out = kit.room_shell(prefix, x0, x1, y0, y1, z0, height, openings, m, coll, style="hangar", rib_every=bay, beams=False,
                         floor_holes=floor_holes)
    out["marks"] = []
    w, d = x1 - x0, y1 - y0
    long_y = d >= w - 1e-6
    zt = z0 + height
    color = lamp_color or SODIUM
    counter = [0]

    def nm(tag):
        counter[0] += 1
        return f"{prefix}_h{tag}{counter[0]}"

    def add(objs, structure=False, detail=0):
        if detail:
            c.set_detail(objs, detail)
        out["visual"].extend(objs)
        if structure:
            out["structure"].extend(objs)

    # Repère de la travée : u le long du grand axe, v en travers (de v0 à v1).
    if long_y:
        u0, u1, v0, v1 = y0, y1, x0, x1

        def P(u, v, z):
            return (v, u, z)
    else:
        u0, u1, v0, v1 = x0, x1, y0, y1

        def P(u, v, z):
            return (u, v, z)
    trusses = [u0 + k * bay for k in range(1, int((u1 - u0) / bay) + 1) if u0 + k * bay < u1 - 0.3]
    span = v1 - v0
    ztop = zt - 0.16  # dessus de la membrure haute, sous les tôles du toit
    zbot = zt - truss_depth
    panels = max(2, round(span / (truss_depth * 1.25)))
    if panels % 2:
        panels += 1
    step = span / panels

    for k, u in enumerate(trusses):
        add([_strut(nm("chord_top"), P(u, v0, ztop - 0.15), P(u, v1, ztop - 0.15), 0.26, 0.3, m["dark"], coll)], True)
        add([_strut(nm("chord_bot"), P(u, v0 + 0.3, zbot + 0.12), P(u, v1 - 0.3, zbot + 0.12), 0.22, 0.24, m["dark"], coll)], True)
        webs = []
        for i in range(panels + 1):
            v = v0 + i * step
            if 0 < i < panels:
                webs.append(_strut(nm("post"), P(u, v, zbot + 0.24), P(u, v, ztop - 0.3), 0.12, 0.12, m["bare"], coll))
            if i < panels:
                a, b = (v, v + step) if i % 2 == 0 else (v + step, v)
                va = max(v0 + 0.3, min(v1 - 0.3, a))
                webs.append(_strut(nm("diag"), P(u, va, zbot + 0.24), P(u, b, ztop - 0.3), 0.11, 0.11, m["bare"], coll))
        add(webs, detail=web_detail)
        # Platines d'appui sur les colonnes, goussets aux extrémités.
        for v in (v0 + 0.25, v1 - 0.25):
            add([_box(nm("bearing"), (0.6, 0.6, 0.12) if long_y else (0.6, 0.6, 0.12), P(u, v, zbot - 0.06), m["dark"], coll, bevel=0.01)])
        # Projecteurs suspendus sous la membrure basse.
        if lamps:
            n = lamps_per_truss or max(1, round(span / 9.0))
            for j in range(n):
                v = v0 + span * (j + 0.5) / n
                vis, _, marks = floodlight(nm("lamp"), P(u, v, zbot), 0.0 if long_y else math.pi / 2, m, coll,
                                           lumens=lamp_lumens, rng=lamp_range, color=color, drop=0.9)
                add(vis)
                out["marks"].extend(marks)

    # Pannes : entre les fermes, tous les ≈ 3 m en travers, sous les tôles du toit.
    if purlin_detail is not None:
        bounds = [u0] + trusses + [u1]
        nv = max(2, round(span / 3.0))
        purl = []
        for ua, ub in zip(bounds[:-1], bounds[1:]):
            for i in range(1, nv):
                v = v0 + span * i / nv
                purl.append(_strut(nm("purlin"), P(ua + 0.15, v, zt - 0.1), P(ub - 0.15, v, zt - 0.1), 0.12, 0.18, m["dark"], coll))
        add(purl, detail=purlin_detail)

    # Poutres de roulement sur consoles, le long des deux grands murs.
    if runway:
        zr = zbot - 1.0
        for v, sgn in ((v0, 1), (v1, -1)):
            vb = v + sgn * 0.75
            add([_strut(nm("runway"), P(u0 + 0.1, vb, zr), P(u1 - 0.1, vb, zr), 0.32, 0.5, m["dark"], coll)])
            add([_strut(nm("rail"), P(u0 + 0.1, vb, zr + 0.29), P(u1 - 0.1, vb, zr + 0.29), 0.08, 0.08, m["bare"], coll)])
            for u in trusses:
                add([_strut(nm("console"), P(u, v + sgn * 0.38, zr - 0.4), P(u, vb + sgn * 0.16, zr - 0.4), 0.3, 0.3, m["dark"], coll),
                     _strut(nm("console_brace"), P(u, v + sgn * 0.38, zr - 1.3), P(u, vb, zr - 0.56), 0.16, 0.16, m["dark"], coll)])

    if bay_door is not None:
        side, pos, dw, dh = bay_door
        fr = kit._Wall(side, x0, x1, y0, y1, z0, height)
        vis, col, marks = _bay_door(f"{prefix}_bay", fr, pos, dw, dh, m, coll)
        add(vis, True)
        out["colliders"].extend(col)
        out["marks"].extend(marks)
    return out


def floodlight(name: str, center, facing: float, m: dict, coll, *, lumens: float = 30000.0, rng: float = 26.0,
               color=None, drop: float = 0.9, detail: int = 0):
    """Grand projecteur de hangar suspendu (`center` : point d'accroche, le projecteur pend de
    `drop` m) : étrier, carter sombre à ailettes, lentille ambrée (`sh_light_amber`, remplacée au
    bake par sa lampe), grille ; repère `LIGHT_<name>` sous la lentille."""
    z = -drop
    vis = [
        _box(f"{name}_housing", (1.1, 0.55, 0.26), (0, 0, z - 0.13), m["dark"], coll, bevel=0.02),
        _box(f"{name}_lens", (0.94, 0.42, 0.03), (0, 0, z - 0.27), m["light_amber"], coll),
        _box(f"{name}_yoke", (1.24, 0.08, 0.06), (0, 0, z + 0.05), m["dark"], coll),
    ]
    for sx in (-1, 1):
        vis.append(_box(f"{name}_yoke_arm{sx}", (0.06, 0.08, 0.32), (sx * 0.6, 0, z - 0.1), m["dark"], coll))
        vis.append(_box(f"{name}_rod{sx}", (0.03, 0.03, drop), (sx * 0.3, 0, -drop / 2 + 0.03), m["bare"], coll))
    for i in range(5):
        vis.append(_box(f"{name}_fin{i}", (0.03, 0.5, 0.06), (-0.4 + i * 0.2, 0, z + 0.02), m["dark"], coll))
    for i in range(3):
        vis.append(_box(f"{name}_guard{i}", (0.02, 0.48, 0.02), (-0.3 + i * 0.3, 0, z - 0.295), m["dark"], coll))
    marks = [c.empty(f"LIGHT_{name}", (0, 0, z - 0.6), coll, {"lumens": lumens, "range": rng, "color": list(color or SODIUM), "shadow_candidate": 0})]
    _place(vis + marks, center, facing)
    return _finish(vis, [], marks, detail)


def _bay_door(name, fr, pos, width, height, m, coll):
    """Grande porte de baie fermée, plaquée sur le mur `fr` (repère `_Wall`) : deux vantaux à
    lames horizontales, joint central, plinthe à bandes de danger, gros montants noirs à bandes
    de danger, linteau, gyrophares. Décor : le mur garde son collider ; montants à colliders."""
    rt = tuple(fr.right())
    up = (0, 0, 1)
    vis, col, marks = [], [], []
    a, b = pos - width / 2, pos + width / 2

    def plate(tag, s, dd, z, sw, sh, th, mat, uv="box", bevel=0.0):
        obj = _ori(f"{name}_{tag}", (sw, sh, th), tuple(fr.p(s, dd + th / 2, z)), rt, up, mat, coll, uv, bevel=bevel)
        vis.append(obj)
        return obj

    # Fond épais (0,1 m) : il cache les nervures et le soubassement du mur derrière la porte.
    plate("back", pos, 0.0, height / 2, width, height, 0.1, m["dark"])
    slat = 0.6
    n = int((height - 0.6) / slat)
    for leaf, (sa, sb) in enumerate(((a, pos), (pos, b))):
        for i in range(n):
            z = 0.6 + slat * (i + 0.5)
            plate(f"slat{leaf}_{i}", (sa + sb) / 2, 0.1, z, (sb - sa) - 0.12, slat - 0.07, 0.05, m["door_paint"],
                  ("box", (0.13 * i + 0.5 * leaf, 0.31 * leaf)), bevel=0.008)
    plate("seam", pos, 0.1, height / 2, 0.14, height, 0.07, m["rubber"])
    plate("kick", pos, 0.1, 0.3, width, 0.6, 0.07, m["trim_sheet"], "hazard")
    plate("label", a + width * 0.25, 0.15, height - 1.2, min(2.0, width * 0.25), 0.12, 0.006, m["trim_sheet"], "stencils")
    # Montants et linteau.
    for s, sgn in ((a, -1), (b, 1)):
        sc = s + sgn * 0.35
        jamb = (0.7, 0.5, height + 0.6) if fr.side in "NS" else (0.5, 0.7, height + 0.6)
        ctr = fr.p(sc, 0.25, (height + 0.6) / 2)
        vis.append(_box(f"{name}_jamb{sgn}", jamb, tuple(ctr), m["paint_black"], coll, bevel=0.03))
        col.append(c.collider_box(f"{name}_jamb{sgn + 1}", jamb, tuple(ctr), coll))
        plate(f"jamb_hazard{sgn}", sc, 0.5, 1.6, 0.4, 3.0, 0.01, m["trim_sheet"], "hazard")
        bvis, _, bmarks = kit.beacon(f"{name}_beacon{sgn + 1}", tuple(fr.p(sc, 0.5, height + 0.2)), fr.facing(), m, coll, lumens=600.0, rng=6.0)
        vis += bvis
        marks += bmarks
    lin = (width + 1.4, 0.55, 0.9) if fr.side in "NS" else (0.55, width + 1.4, 0.9)
    vis.append(_box(f"{name}_lintel", lin, tuple(fr.p(pos, 0.275, height + 0.45)), m["paint_black"], coll, bevel=0.03))
    plate("lintel_band", pos, 0.55, height + 0.45, width + 1.2, 0.14, 0.008, m["trim_sheet"], "hazard")
    # Bande de danger au sol le long du seuil.
    p0, p1 = fr.p(a, 0.9, 0.0), fr.p(b, 0.9, 0.0)
    vis += kit.hazard_stripe(f"{name}_threshold", tuple(p0), tuple(p1), m, coll, width=0.3)[0]
    return vis, col, marks


# --- Berceau d'amarrage -----------------------------------------------------------------------------


def docking_cradle(
    name: str,
    x0: float,
    x1: float,
    y0: float,
    y1: float,
    m: dict,
    coll,
    *,
    z0: float = 0.0,
    depth: float = 3.0,
    gaps=(),
    arms: int = 3,
    arm_reach: float = 1.1,
    arm_height: float | None = None,
    saddles: int = 2,
    saddle_height: float = 0.7,
    rail_height: float = 1.1,
    rail_offset: float = 0.25,
    pit_lights: bool = True,
    markings: bool = True,
    markings_detail: int = 0,
    arms_detail: int = 0,
):
    """Berceau d'amarrage : fosse rectangulaire (x0..x1, y0..y1) creusée dans le sol du hangar
    (à percer aussi dans le sol : `hangar_shell(floor_holes=[(x0, x1, y0, y1)])`), profonde de
    `depth` m sous le sol (z0). Le vaisseau y repose, son pont au niveau du sol.

    - parois de béton à nervures d'acier, fond en tôle larmée à caniveau en caillebotis, bandeaux
      ambrés au pied des parois (cuits), lampes de fosse (`pit_lights`, repères `LIGHT_`) ;
    - margelle d'acier jaune et bande de danger tout autour ;
    - garde-corps jaunes (lisse à `rail_height`, sous-lisse, plinthe) à `rail_offset` m du bord,
      interrompus aux passerelles : `gaps` = [(côté, position, largeur)] (côtés N, S, E, W ;
      position le long du côté : x pour N et S, y pour E et W) ; colliders sur chaque tronçon ;
    - `arms` bras de maintien hydrauliques sur chaque grand côté (bloc mural, vérin, bras jaune,
      patin de caoutchouc à `arm_reach` m de la paroi, à `arm_height`, défaut z0 − 0,6), de
      niveau `arms_detail` ;
    - `saddles` sabots transversaux au fond (hauteur `saddle_height`, dessus caoutchouc) ;
    - `markings` : marquages au sol autour (angles jaunes, chevrons vers les passerelles), de
      niveau `markings_detail`.
    Colliders : parois, fond, garde-corps. Renvoie (visibles, colliders, repères)."""
    vis, col, marks = [], [], []
    zb = z0 - depth
    t = 0.3
    w, d = x1 - x0, y1 - y0
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    long_y = d >= w
    n = [0]

    def nm(tag):
        n[0] += 1
        return f"{name}_{tag}{n[0]}"

    # Parois (sous la dalle du sol : leur dessus à z0 − WALL) et leur collider.
    zw = z0 - kit.WALL
    walls = {
        "N": ((w + 2 * t, t, zw - zb), (cx, y1 + t / 2, (zw + zb) / 2)),
        "S": ((w + 2 * t, t, zw - zb), (cx, y0 - t / 2, (zw + zb) / 2)),
        "E": ((t, d, zw - zb), (x1 + t / 2, cy, (zw + zb) / 2)),
        "W": ((t, d, zw - zb), (x0 - t / 2, cy, (zw + zb) / 2)),
    }
    for side, (size, center) in walls.items():
        vis.append(_box(nm(f"wall_{side}"), size, center, m["concrete"], coll, ("box", (0.37 * "NSEW".index(side), 0.0))))
        # Collider sous la dalle du sol (jamais de rebord invisible autour de la fosse).
        hc = zw - zb + 0.2
        col.append(c.collider_box(nm(f"wall_{side}"), (size[0], size[1], hc), (center[0], center[1], zb - 0.2 + hc / 2), coll))
    # Bandeau de tôle entre le dessous de la dalle et la margelle (le chant de la dalle se voit).
    frames = {s: kit._Wall(s, x0, x1, y0, y1, zb, depth) for s in "NSEW"}
    for side, fr in frames.items():
        rt = tuple(fr.right())
        vis.append(_ori(nm("lip"), (fr.hi - fr.lo, 0.42, 0.02), tuple(fr.p((fr.lo + fr.hi) / 2, 0.01, depth - 0.21)), rt, (0, 0, 1), m["paint_yellow"], coll))
        vis.append(_ori(nm("lip_band"), (fr.hi - fr.lo, 0.14, 0.006), tuple(fr.p((fr.lo + fr.hi) / 2, 0.023, depth - 0.3)), rt, (0, 0, 1), m["trim_sheet"], coll, "hazard"))
        # Nervures d'acier tous les 2 m, bandeau ambré au pied entre les nervures.
        ribs = [fr.lo + k * 2.0 for k in range(1, int((fr.hi - fr.lo) / 2.0) + 1) if fr.lo + k * 2.0 < fr.hi - 0.3]
        for s in ribs:
            size = (0.2, 0.16, depth - 0.45) if side in "NS" else (0.16, 0.2, depth - 0.45)
            vis.append(_box(nm("rib"), size, tuple(fr.p(s, 0.08, (depth - 0.45) / 2)), m["dark"], coll))
            vis.append(_ori(nm("rib_face"), (depth - 0.45, 0.2, 0.012), tuple(fr.p(s, 0.166, (depth - 0.45) / 2)), (0, 0, 1),
                            tuple(fr.n.cross(Vector((0, 0, 1)))), m["trim_sheet"], coll, "rivets"))
        bounds = [fr.lo] + ribs + [fr.hi]
        for sa, sb in zip(bounds[:-1], bounds[1:]):
            if sb - sa > 0.8:
                kit._corner_band(name, fr, sa + 0.25, sb - 0.25, 0.45, m["band_amber"], m, coll, vis.append, nm, cove=False)
    # Fond : tôle larmée, caniveau à caillebotis lumineux le long du grand axe.
    vis.append(_box(nm("floor"), (w, d, 0.2), (cx, cy, zb - 0.1), m["floor_plate"], coll))
    col.append(c.collider_box(nm("floor"), (w, d, 0.2), (cx, cy, zb - 0.1), coll))
    gsize = (0.8, d - 1.0) if long_y else (w - 1.0, 0.8)
    vis += kit.floor_grating(nm("drain"), (cx, cy, zb), gsize, 0.0, m, coll, glow=None)[0]
    # Sabots transversaux.
    for i in range(saddles):
        f = (i + 1) / (saddles + 1)
        if long_y:
            sc, size = (cx, y0 + d * f, zb + saddle_height / 2), (w - 1.6, 0.8, saddle_height)
        else:
            sc, size = (x0 + w * f, cy, zb + saddle_height / 2), (0.8, d - 1.6, saddle_height)
        vis.append(_box(nm("saddle"), size, sc, m["dark"], coll, bevel=0.03))
        top = (size[0] - 0.1, size[1] - 0.1, 0.08)
        vis.append(_box(nm("saddle_pad"), top, (sc[0], sc[1], zb + saddle_height + 0.04), m["rubber"], coll))
        # Bande de danger sur la face du sabot tournée vers +Y (ou +X).
        zc = zb + saddle_height * 0.5
        if long_y:
            bc, right = (sc[0], sc[1] + size[1] / 2 + 0.003, zc), (-1, 0, 0)
        else:
            bc, right = (sc[0] + size[0] / 2 + 0.003, sc[1], zc), (0, 1, 0)
        vis.append(_ori(nm("saddle_band"), (max(size[0], size[1]) - 0.2, 0.2, 0.006), bc, right, (0, 0, 1), m["trim_sheet"], coll, "hazard"))
    # Bras de maintien sur les grands côtés.
    ah = arm_height if arm_height is not None else z0 - 0.6
    arm_objs = []
    for side in (("E", "W") if long_y else ("N", "S")):
        fr = frames[side]
        for i in range(arms):
            s = fr.lo + (fr.hi - fr.lo) * (i + 0.5) / arms
            zl = ah - zb  # hauteur du patin au-dessus du fond
            base = fr.p(s, 0.3, zl - 1.0)
            arm_objs.append(_box(nm("arm_base"), (0.7, 0.6, 0.9) if side in "NS" else (0.6, 0.7, 0.9), tuple(base), m["dark"], coll, bevel=0.03))
            pivot = fr.p(s, 0.45, zl - 0.75)
            tip = fr.p(s, arm_reach, zl - 0.1)
            arm_objs.append(_strut(nm("arm"), tuple(pivot), tuple(tip), 0.3, 0.26, m["paint_yellow"], coll, bevel=0.02))
            arm_objs.append(_strut(nm("arm_band"), tuple(pivot + (tip - pivot) * 0.3), tuple(pivot + (tip - pivot) * 0.75), 0.304, 0.12, m["trim_sheet"], coll, "hazard"))
            ram0 = fr.p(s, 0.05, zl - 2.0)
            ram1 = pivot + (tip - pivot) * 0.55
            arm_objs.append(_strut(nm("ram"), tuple(ram0), tuple(ram0 + (ram1 - ram0) * 0.55), 0.18, 0.18, m["dark"], coll))
            arm_objs.append(_strut(nm("rod"), tuple(ram0 + (ram1 - ram0) * 0.5), tuple(ram1), 0.08, 0.08, m["bare"], coll))
            pad = fr.p(s, arm_reach + 0.08, zl)
            arm_objs.append(_box(nm("pad"), (0.6, 0.18, 0.4) if side in "NS" else (0.18, 0.6, 0.4), tuple(pad), m["rubber"], coll, bevel=0.02))
    if arms_detail:
        c.set_detail(arm_objs, arms_detail)
    vis += arm_objs
    # Lampes de fosse (cuites) le long du grand axe.
    if pit_lights:
        nl = max(2, round((d if long_y else w) / 8.0))
        for i in range(nl):
            f = (i + 0.5) / nl
            p = (cx, y0 + d * f, zb + 1.2) if long_y else (x0 + w * f, cy, zb + 1.2)
            marks.append(c.empty(f"LIGHT_{name}_pit{i}", p, coll, {"lumens": 2500.0, "range": 9.0, "color": kit.AMBER, "shadow_candidate": 0}))
    # Margelle jaune sur le sol (dessus à z0 + 12 mm) et bande de danger.
    rims = {
        "N": ((w + 0.5, 0.25), (cx, y1 + 0.125)),
        "S": ((w + 0.5, 0.25), (cx, y0 - 0.125)),
        "E": ((0.25, d), (x1 + 0.125, cy)),
        "W": ((0.25, d), (x0 - 0.125, cy)),
    }
    for side, ((sx, sy), (px, py)) in rims.items():
        vis.append(_box(nm("coping"), (sx, sy, 0.012), (px, py, z0 + 0.006), m["paint_yellow"], coll))
    vis += _ring_stripes(nm("rim"), x0 - 0.25, x1 + 0.25, y0 - 0.25, y1 + 0.25, 0.3, z0 + 0.0, m, coll, "hazard")
    # Garde-corps.
    rv, rc = _guardrails(name, x0 - rail_offset - 0.25, x1 + rail_offset + 0.25, y0 - rail_offset - 0.25, y1 + rail_offset + 0.25,
                         z0, rail_height, gaps, m, coll)
    vis += rv
    col += rc
    # Marquages : équerres jaunes aux angles, chevrons vers chaque passerelle.
    if markings:
        mk = []
        off = 1.4
        for (px, py, dx, dy) in ((x0 - off, y0 - off, 1, 1), (x1 + off, y0 - off, -1, 1), (x0 - off, y1 + off, 1, -1), (x1 + off, y1 + off, -1, -1)):
            mk.append(_box(nm("corner"), (1.4, 0.16, 0.006), (px + dx * 0.7, py, z0 + 0.003), m["paint_yellow"], coll))
            mk.append(_box(nm("corner"), (0.16, 1.4, 0.006), (px, py + dy * 0.7, z0 + 0.003), m["paint_yellow"], coll))
        for side, pos, gw in gaps:
            # Allée de chevrons perpendiculaire au bord : ils pointent vers la passerelle (+U de
            # la bande = de p0 vers p1).
            fr = frames[side]
            p0 = fr.p(pos, -rail_offset - 3.8, depth)
            p1 = fr.p(pos, -rail_offset - 0.9, depth)
            mk += kit.hazard_stripe(nm("chevron"), tuple(p0), tuple(p1), m, coll, width=min(0.45, gw * 0.3), trim="chevrons")[0]
        if markings_detail:
            c.set_detail(mk, markings_detail)
        vis += mk
    return vis, col, marks


def _ring_stripes(name, x0, x1, y0, y1, width, z, m, coll, trim):
    """Bande de la trim sheet posée au sol tout autour d'un rectangle (à l'extérieur)."""
    out = []
    W = width
    segs = (((x0 - W, y0 - W / 2), (x1 + W, y0 - W / 2)), ((x1 + W, y1 + W / 2), (x0 - W, y1 + W / 2)),
            ((x1 + W / 2, y0), (x1 + W / 2, y1)), ((x0 - W / 2, y1), (x0 - W / 2, y0)))
    for i, (a, b) in enumerate(segs):
        out += kit.hazard_stripe(f"{name}_{i}", (a[0], a[1], z - 0.0005 * (i // 2)), (b[0], b[1], z - 0.0005 * (i // 2)), m, coll, W, trim=trim)[0]
    return out


def _guardrails(name, x0, x1, y0, y1, z0, height, gaps, m, coll):
    """Garde-corps sur le pourtour d'un rectangle, interrompus aux `gaps` (côté, position,
    largeur) : poteaux tous les ≤ 1,8 m, lisse, sous-lisse, plinthe ; un collider par tronçon."""
    vis, col = [], []
    sides = {"N": ("x", y1, x0, x1), "S": ("x", y0, x0, x1), "E": ("y", x1, y0, y1), "W": ("y", x0, y0, y1)}
    k = 0
    for side, (axis, fixed, lo, hi) in sides.items():
        holes = [(p - gw / 2, p + gw / 2) for sd, p, gw in gaps if sd == side]
        for sa, sb in kit._cuts(lo, hi, holes):
            k += 1
            L = sb - sa
            npost = max(1, math.ceil(L / 1.8))

            def P(s, z):
                return (s, fixed, z) if axis == "x" else (fixed, s, z)
            for i in range(npost + 1):
                s = sa + L * i / npost
                vis.append(_box(f"{name}_post{k}_{i}", (0.06, 0.06, height), P(s, z0 + height / 2), m["paint_yellow"], coll))
                vis.append(_box(f"{name}_foot{k}_{i}", (0.14, 0.14, 0.02), P(s, z0 + 0.01), m["dark"], coll))
            for tag, z, th in (("rail", height - 0.025, 0.05), ("mid", height * 0.5, 0.04)):
                vis.append(_strut(f"{name}_{tag}{k}", P(sa, z0 + z), P(sb, z0 + z), th, th, m["paint_yellow"], coll, bevel=0.01))
            vis.append(_strut(f"{name}_toe{k}", P(sa, z0 + 0.075), P(sb, z0 + 0.075), 0.015, 0.15, m["trim_sheet"], coll, "hazard"))
            size = (L, 0.08, height) if axis == "x" else (0.08, L, height)
            col.append(c.collider_box(f"{name}_rail{k}", size, P((sa + sb) / 2, z0 + height / 2), coll))
    return vis, col


# --- Passerelle -----------------------------------------------------------------------------------


def gangway(name: str, p0, p1, m: dict, coll, width: float = 1.6, *, rails=(True, True), rail_height: float = 1.1,
            posts_every: float = 1.5, support: float | None = None, detail: int = 0):
    """Passerelle plane de p0 à p1 (axe X ou Y ; z de p0 = dessus du caillebotis, au niveau du
    sol et du pont du vaisseau : on y marche à plat). Caillebotis dans un cadre de profilés
    sombres, traverses dessous, bandes de danger le long des bords et en travers des deux bouts,
    garde-corps jaunes (`rails` = (gauche, droite) vus de p0 vers p1 ; lisse, sous-lisse,
    plinthe) ; `support` : z du fond sous la passerelle, poteaux tous les ≈ 4 m.
    Colliders : tablier (dessus à z) et garde-corps."""
    p0, p1 = Vector(p0), Vector(p1)
    ax = 0 if abs(p1.x - p0.x) >= abs(p1.y - p0.y) else 1
    L = abs((p1 - p0)[ax])
    z = p0.z
    t = Vector((1, 0, 0)) if ax == 0 else Vector((0, 1, 0))
    if (p1 - p0)[ax] < 0:
        t = -t
    left = Vector((0, 0, 1)).cross(t)
    mid = Vector(((p0.x + p1.x) / 2, (p0.y + p1.y) / 2, z))
    vis, col = [], []

    def dims(along, across, h):
        return (along, across, h) if ax == 0 else (across, along, h)

    vis.append(_box(f"{name}_deck", dims(L, width - 0.1, 0.04), tuple(mid - Vector((0, 0, 0.02))), m["grating_plain"], coll))
    for s in (-1, 1):
        vis.append(_box(f"{name}_stringer{s}", dims(L, 0.08, 0.26), tuple(mid + left * s * (width / 2 - 0.04) - Vector((0, 0, 0.13))), m["dark"], coll))
        vis.append(_strut(f"{name}_edge{s}", tuple(mid - t * (L / 2) + left * s * (width / 2 - 0.09) + Vector((0, 0, 0.003))),
                          tuple(mid + t * (L / 2) + left * s * (width / 2 - 0.09) + Vector((0, 0, 0.003))), 0.1, 0.006, m["trim_sheet"], coll, "hazard"))
    nx = max(1, int(L // 1.0))
    for i in range(nx + 1):
        p = mid - t * (L / 2) + t * (L * i / nx)
        p = p + t * (0.04 if i == 0 else (-0.04 if i == nx else 0.0))
        vis.append(_box(f"{name}_cross{i}", dims(0.06, width - 0.12, 0.12), tuple(p - Vector((0, 0, 0.1))), m["dark"], coll))
    for e, sgn in ((p0, 1), (p1, -1)):
        e2 = Vector((e.x, e.y, z))
        e2[1 - ax] = mid[1 - ax]
        a = e2 + t * sgn * 0.12 - left * (width / 2 - 0.15)
        b = e2 + t * sgn * 0.12 + left * (width / 2 - 0.15)
        vis += kit.hazard_stripe(f"{name}_end{sgn + 1}", tuple(a + Vector((0, 0, 0.001))), tuple(b + Vector((0, 0, 0.001))), m, coll, width=0.2)[0]
    col.append(c.collider_box(f"{name}_deck", dims(L, width, 0.1), tuple(mid - Vector((0, 0, 0.05))), coll))
    for s, on in zip((1, -1), rails):
        if not on:
            continue
        off = left * s * (width / 2 - 0.03)
        npost = max(1, math.ceil(L / posts_every))
        for i in range(npost + 1):
            p = mid - t * (L / 2) + t * (L * i / npost) + off
            p = p + t * (0.03 if i == 0 else (-0.03 if i == npost else 0.0))
            vis.append(_box(f"{name}_post{s}_{i}", (0.05, 0.05, rail_height), tuple(p + Vector((0, 0, rail_height / 2))), m["paint_yellow"], coll))
        a, b = mid - t * (L / 2) + off, mid + t * (L / 2) + off
        for tag, h, th in (("rail", rail_height - 0.025, 0.05), ("mid", rail_height * 0.5, 0.035)):
            vis.append(_strut(f"{name}_{tag}{s}", tuple(a + Vector((0, 0, h))), tuple(b + Vector((0, 0, h))), th, th, m["paint_yellow"], coll, bevel=0.008))
        vis.append(_strut(f"{name}_toe{s}", tuple(a + Vector((0, 0, 0.07))), tuple(b + Vector((0, 0, 0.07))), 0.012, 0.12, m["dark"], coll))
        size = dims(L, 0.08, rail_height)
        col.append(c.collider_box(f"{name}_rail{s + 1}", size, tuple(a + (b - a) / 2 + Vector((0, 0, rail_height / 2))), coll))
    if support is not None and z - support > 0.5:
        ns = max(1, round(L / 4.0))
        for i in range(ns):
            p = mid - t * (L / 2) + t * (L * (i + 0.5) / ns)
            for s in (-1, 1):
                q = p + left * s * (width / 2 - 0.1)
                vis.append(_box(f"{name}_leg{i}{s}", (0.14, 0.14, z - 0.26 - support), tuple(Vector((q.x, q.y, (z - 0.26 + support) / 2))), m["dark"], coll))
    return _finish(vis, col, [], detail)


# --- Portique roulant -----------------------------------------------------------------------------


def gantry_crane(name: str, center, facing: float, m: dict, coll, span: float = 10.0, height: float = 8.0, *,
                 rail_length: float = 12.0, trolley: float = 0.2, hook_z: float = 3.2, beacon: bool = True, detail: int = 0):
    """Portique roulant : deux palées en A (poteaux inclinés, longrine à bogies) sur rails au
    sol (le long de Y, `rail_length` m), poutre-caisson jaune à bandes de danger (portée `span`
    le long de X, dessus à `height`), chariot et treuil (position `trolley` en fraction de la
    portée depuis le centre), câbles, moufle et crochet à `hook_z`, boîte de commande pendante,
    gyrophare. `center` : milieu entre les palées, au sol. Colliders : palées, moufle s'il
    descend sous 2,1 m. Pièce visible de loin : la mettre dans la version simplifiée de la
    cellule (`structure`) si besoin."""
    vis, col = [], []
    gz = height - 0.6  # axe de la poutre
    for sx in (-1, 1):
        x = sx * span / 2
        vis.append(_box(f"{name}_sill{sx}", (0.5, 3.0, 0.45), (x, 0, 0.45), m["dark"], coll, bevel=0.03))
        for sy in (-1, 1):
            vis.append(_box(f"{name}_bogie{sx}{sy}", (0.42, 0.7, 0.3), (x, sy * 1.15, 0.2), m["dark"], coll, bevel=0.02))
            for k in (-1, 1):
                vis.append(_cyl(f"{name}_wheel{sx}{sy}{k}", 0.13, 0.12, (x, sy * 1.15 + k * 0.2, 0.13), m["bare"], coll, axis="X", segments=12))
            vis.append(_strut(f"{name}_leg{sx}{sy}", (x, sy * 1.2, 0.65), (x, sy * 0.45, gz - 0.6), 0.34, 0.34, m["paint_yellow"], coll, bevel=0.02))
        vis.append(_strut(f"{name}_tie{sx}", (x, -0.85, gz * 0.45), (x, 0.85, gz * 0.45), 0.2, 0.2, m["dark"], coll))
        vis.append(_box(f"{name}_carriage{sx}", (1.0, 1.4, 0.7), (x, 0, gz - 0.3), m["dark"], coll, bevel=0.03))
        vis.append(_box(f"{name}_leg_plate{sx}", (0.52, 0.9, 0.02), (x, 0, 0.69), m["dark"], coll))
        # Bande de danger sur la face extérieure de la longerine.
        vis.append(_ori(f"{name}_sill_band{sx}", (2.6, 0.2, 0.006), (x + sx * 0.253, 0, 0.45), (0, sx, 0), (0, 0, 1), m["trim_sheet"], coll, "hazard"))
        # Rails au sol, platines d'ancrage.
        vis.append(_box(f"{name}_rail{sx}", (0.1, rail_length, 0.08), (x, 0, 0.04), m["bare"], coll))
        for i in range(int(rail_length // 1.5) + 1):
            y = -rail_length / 2 + i * 1.5
            vis.append(_box(f"{name}_anchor{sx}_{i}", (0.34, 0.12, 0.02), (x, y, 0.01), m["dark"], coll))
    # Poutre-caisson et ses bandes de danger.
    gl = span + 1.2
    vis.append(_box(f"{name}_girder", (gl, 0.9, 1.2), (0, 0, gz), m["paint_yellow"], coll, bevel=0.03))
    for sy in (-1, 1):
        vis.append(_ori(f"{name}_girder_band{sy}", (gl - 0.4, 0.2, 0.006), (0, sy * 0.453, gz - 0.45), (-sy, 0, 0), (0, 0, 1), m["trim_sheet"], coll, "hazard"))
        vis.append(_ori(f"{name}_girder_label{sy}", (1.6, 0.12, 0.004), (sy * span * 0.3, sy * 0.452, gz + 0.3), (-sy, 0, 0), (0, 0, 1), m["trim_sheet"], coll, "stencils"))
    vis.append(_box(f"{name}_track", (gl - 0.6, 0.08, 0.06), (0, 0.3, gz + 0.63), m["bare"], coll))
    vis.append(_box(f"{name}_track2", (gl - 0.6, 0.08, 0.06), (0, -0.3, gz + 0.63), m["bare"], coll))
    # Chariot, treuil, câbles, moufle, crochet.
    tx = trolley * span / 2
    vis.append(_box(f"{name}_trolley", (1.5, 1.3, 0.7), (tx, 0, gz + 1.0), m["dark"], coll, bevel=0.03))
    vis.append(_cyl(f"{name}_drum", 0.3, 1.0, (tx, 0, gz + 1.0), m["bare"], coll, axis="Y", segments=16))
    vis.append(_ori(f"{name}_trolley_vent", (1.0, 0.3, 0.006), (tx, 0.653, gz + 1.05), *_FRONT, m["trim_sheet"], coll, "vent"))
    top = gz - 0.6
    for k in (-1, 1):
        vis.append(_cyl(f"{name}_cable{k}", 0.018, top - hook_z - 0.3, (tx + k * 0.12, 0, (top + hook_z + 0.3) / 2), m["dark"], coll, segments=6))
    vis.append(_box(f"{name}_block", (0.45, 0.3, 0.55), (tx, 0, hook_z + 0.1), m["paint_yellow"], coll, bevel=0.03))
    vis.append(_cyl(f"{name}_sheave", 0.16, 0.32, (tx, 0, hook_z + 0.25), m["dark"], coll, axis="Y", segments=12))
    vis.append(_box(f"{name}_hook_shank", (0.08, 0.08, 0.25), (tx, 0, hook_z - 0.28), m["bare"], coll))
    vis.append(_box(f"{name}_hook_bow", (0.3, 0.08, 0.07), (tx + 0.11, 0, hook_z - 0.42), m["bare"], coll))
    vis.append(_box(f"{name}_hook_tip", (0.07, 0.08, 0.14), (tx + 0.24, 0, hook_z - 0.35), m["bare"], coll))
    # Boîte de commande pendante.
    vis.append(_cyl(f"{name}_pendant_cable", 0.008, top - 1.5, (tx - 0.6, 0.4, (top + 1.5) / 2), m["rubber"], coll, segments=6))
    vis.append(_box(f"{name}_pendant", (0.1, 0.08, 0.28), (tx - 0.6, 0.4, 1.36), m["paint_yellow"], coll, bevel=0.01))
    vis.append(_ori(f"{name}_pendant_buttons", (0.07, 0.2, 0.004), (tx - 0.6, 0.442, 1.36), *_FRONT, m["buttons"], coll, tile=0.25))
    if beacon:
        # Gyrophare posé sur la poutre (sans lumière : simple voyant).
        vis.append(_cyl(f"{name}_beacon_base", 0.09, 0.06, (span * 0.42, 0, gz + 0.63), m["dark"], coll, segments=12))
        vis.append(_cyl(f"{name}_beacon", 0.07, 0.14, (span * 0.42, 0, gz + 0.73), m["beacon"], coll, segments=12))
    for sx in (-1, 1):
        col.append(c.collider_box(f"{name}_sill{sx + 1}", (0.6, 3.0, 2.2), (sx * span / 2, 0, 1.1), coll))
    if hook_z - 0.5 < kit.HEAD_CLEAR:
        col.append(c.collider_box(f"{name}_block", (0.5, 0.35, 1.0), (tx, 0, hook_z - 0.1), coll))
    _place(vis + col, center, facing)
    return _finish(vis, col, [], detail)


# --- Fret -------------------------------------------------------------------------------------------


def cargo_container(name: str, center, facing: float, m: dict, coll, size=(6.0, 2.4, 2.6), variant: str = "slate", *,
                    detail: int = 0):
    """Conteneur de fret (long le long de X) : parois de tôle nervurée teintée (`variant` :
    "slate", "rust", "yellow", "olive"), cadre sombre, pièces de coin en métal brut, portes à
    deux vantaux à +X (barres de verrouillage, poignées, charnières), plaques à pochoirs.
    Collider : le volume."""
    L, W, H = size
    mat = m["crate"] if variant == "olive" else m[f"crate_{variant}"]
    vis = [_box(f"{name}", (L - 0.12, W - 0.06, H - 0.12), (0, 0, H / 2), mat, coll)]
    for dx in (-1, 1):
        for dy in (-1, 1):
            vis.append(_box(f"{name}_post{dx}{dy}", (0.16, 0.16, H), (dx * (L / 2 - 0.08), dy * (W / 2 - 0.08), H / 2), m["dark"], coll))
            for z in (0.09, H - 0.09):
                vis.append(_box(f"{name}_casting{dx}{dy}{z:.1f}", (0.19, 0.19, 0.18), (dx * (L / 2 - 0.085), dy * (W / 2 - 0.085), z), m["bare"], coll, bevel=0.01))
        for z in (0.08, H - 0.08):
            vis.append(_box(f"{name}_end_rail{dx}{z:.1f}", (0.12, W - 0.3, 0.16), (dx * (L / 2 - 0.06), 0, z), m["dark"], coll))
    for dy in (-1, 1):
        for z in (0.08, H - 0.08):
            vis.append(_box(f"{name}_side_rail{dy}{z:.1f}", (L - 0.3, 0.12, 0.16), (0, dy * (W / 2 - 0.06), z), m["dark"], coll))
        vis.append(_ori(f"{name}_label{dy}", (1.6, 0.2, 0.004), (-L * 0.22, dy * (W / 2 - 0.028), H - 0.42), (-dy, 0, 0), (0, 0, 1), m["trim_sheet"], coll, "stencils"))
        vis.append(_ori(f"{name}_mark{dy}", (0.8, 0.12, 0.004), (L * 0.3, dy * (W / 2 - 0.028), 0.4), (-dy, 0, 0), (0, 0, 1), m["trim_sheet"], coll, "warning_red"))
    # Portes (+X) : deux vantaux, quatre barres, poignées, charnières.
    xd = L / 2 - 0.05
    for k, sy in enumerate((-1, 1)):
        vis.append(_ori(f"{name}_door{k}", (W / 2 - 0.2, H - 0.36, 0.03), (xd, sy * (W / 4 - 0.02), H / 2), (0, 1, 0), (0, 0, 1), mat, coll, ("box", (0.37 * k, 0.0))))
        for j in (-1, 1):
            y = sy * (W / 4 - 0.02) + j * 0.22
            vis.append(_cyl(f"{name}_bar{k}{j}", 0.022, H - 0.4, (xd + 0.045, y, H / 2), m["bare"], coll, segments=8))
            vis.append(_box(f"{name}_cam{k}{j}", (0.05, 0.08, 0.08), (xd + 0.05, y, 0.5), m["dark"], coll))
        vis.append(_box(f"{name}_handle{k}", (0.04, 0.3, 0.05), (xd + 0.09, sy * (W / 4 - 0.02), 1.15), m["dark"], coll))
        for z in (0.5, H / 2, H - 0.5):
            vis.append(_box(f"{name}_hinge{k}{z:.1f}", (0.06, 0.08, 0.14), (xd + 0.02, sy * (W / 2 - 0.2), z), m["dark"], coll))
    vis.append(_ori(f"{name}_door_label", (0.5, 0.1, 0.004), (xd + 0.017, -W / 4 + 0.02, H - 0.55), (0, 1, 0), (0, 0, 1), m["trim_sheet"], coll, "stencils"))
    col = [c.collider_box(f"{name}", (L, W, H), (0, 0, H / 2), coll)]
    _place(vis + col, center, facing)
    return _finish(vis, col, [], detail)


def crate_stack(name: str, center, facing: float, m: dict, coll, *, variants=("olive", "slate", "rust"), pallets: bool = True,
                detail: int = 0):
    """Pile de caisses sur palettes de métal : deux caisses de 1,2 m côte à côte (le long de X),
    une caisse de 0,9 m posée de biais sur la première. Colliders : ceux des caisses."""
    vis, col = [], []
    pz = 0.14 if pallets else 0.0
    for i, x in enumerate((-0.65, 0.65)):
        if pallets:
            vis += _pallet(f"{name}_pallet{i}", (x, 0.0, 0.0), m, coll)
        v, cl, _ = kit.crate(f"{name}_c{i}", (x, 0.0, pz), 0.04 * (1 - 2 * i), m, coll, size=(1.2, 1.2, 1.2), variant=variants[i % len(variants)])
        vis += v
        col += cl
    v, cl, _ = kit.crate(f"{name}_c2", (-0.6, 0.05, pz + 1.2), 0.22, m, coll, size=(0.9, 0.9, 0.9), variant=variants[2 % len(variants)])
    vis += v
    col += cl
    if pallets:
        col.append(c.collider_box(f"{name}_pallets", (2.55, 1.25, pz), (0, 0, pz / 2), coll))
    _place(vis + col, center, facing)
    return _finish(vis, col, [], detail)


def _pallet(name, center, m, coll):
    """Palette de métal (patins de métal brut, lattes sombres), 1,25 × 1,25 × 0,14 m."""
    x, y, z = center
    vis = []
    for j, dy in enumerate((-0.5, 0.0, 0.5)):
        vis.append(_box(f"{name}_runner{j}", (1.25, 0.14, 0.1), (x, y + dy, z + 0.05), m["bare"], coll))
    for j in range(5):
        vis.append(_box(f"{name}_slat{j}", (0.2, 1.25, 0.04), (x - 0.52 + j * 0.26, y, z + 0.12), m["dark"], coll))
    return vis


# --- Bornes -----------------------------------------------------------------------------------------


def fuel_bollard(name: str, center, facing: float, m: dict, coll, *, kind: str = "fuel", hose_to=None, detail: int = 0):
    """Borne de service au sol, face vers +Y : `kind` "fuel" (carburant : fût jaune, tuyau et
    pistolet) ou "power" (énergie : fût sombre à tête jaune, câble et connecteur). Tête à écran et
    boutons, voyant, enrouleur sur le flanc +X, étui sur le flanc −X, deux potelets de
    protection devant. `hose_to` = (x, y) local : le tuyau court au sol jusque-là (en L), sinon
    il reste enroulé. Colliders : fût, potelets."""
    fuel = kind == "fuel"
    body = m["paint_yellow"] if fuel else m["dark"]
    head = m["dark"] if fuel else m["paint_yellow"]
    hose = 0.045 if fuel else 0.06
    vis = [
        _box(f"{name}_body", (0.55, 0.45, 1.0), (0, 0, 0.5), body, coll, bevel=0.02),
        _box(f"{name}_plinth", (0.65, 0.55, 0.08), (0, 0, 0.04), m["dark"], coll, bevel=0.01),
        _box(f"{name}_head", (0.6, 0.5, 0.48), (0, 0, 1.24), head, coll, bevel=0.025),
        _box(f"{name}_cap", (0.5, 0.4, 0.05), (0, 0, 1.505), m["dark"], coll),
        _ori(f"{name}_band", (0.55, 0.14, 0.006), (0, 0.228, 0.82), *_FRONT, m["trim_sheet"], coll, "hazard" if fuel else "warning_red"),
        _ori(f"{name}_screen", (0.3, 0.18, 0.006), (0.05, 0.253, 1.32), *_FRONT, m["screen"], coll, tile=0.6),
        _ori(f"{name}_buttons", (0.12, 0.2, 0.006), (-0.19, 0.253, 1.28), *_FRONT, m["buttons"], coll, tile=0.3),
        _ori(f"{name}_label", (0.4, 0.08, 0.004), (0, 0.252, 1.07), *_FRONT, m["trim_sheet"], coll, "stencils"),
        _ori(f"{name}_vent", (0.4, 0.12, 0.006), (0, 0.228, 0.3), *_FRONT, m["trim_sheet"], coll, "vent"),
        _cyl(f"{name}_lamp", 0.05, 0.08, (0.18, 0.1, 1.57), m["beacon"], coll, segments=12),
    ]
    # Enrouleur (+X) et étui (−X).
    vis.append(_cyl(f"{name}_reel", 0.24, 0.16, (0.37, 0, 0.62), m["bare"], coll, axis="X", segments=16))
    vis.append(_cyl(f"{name}_reel_hose", 0.2, 0.12, (0.38, 0, 0.62), m["rubber"], coll, axis="X", segments=16))
    vis.append(_cyl(f"{name}_reel_hub", 0.06, 0.2, (0.38, 0, 0.62), m["dark"], coll, axis="X", segments=10))
    vis.append(_box(f"{name}_holster", (0.1, 0.16, 0.3), (-0.33, 0.05, 0.95), m["dark"], coll, bevel=0.01))
    nozzle = (0.08, 0.1, 0.24) if fuel else (0.1, 0.12, 0.16)
    vis.append(_box(f"{name}_nozzle", nozzle, (-0.33, 0.1, 1.1), m["bare"] if fuel else m["paint_yellow"], coll, bevel=0.01))
    # Potelets de protection.
    for sx in (-1, 1):
        vis.append(_cyl(f"{name}_post{sx}", 0.07, 0.9, (sx * 0.45, 0.62, 0.45), m["paint_yellow"], coll, segments=12))
        vis.append(_cyl(f"{name}_post_band{sx}", 0.072, 0.2, (sx * 0.45, 0.62, 0.65), m["trim_sheet"], coll, segments=12, uv=("wrap", "hazard", 0.5)))
        vis.append(_cyl(f"{name}_post_foot{sx}", 0.11, 0.03, (sx * 0.45, 0.62, 0.015), m["dark"], coll, segments=12))
    col = [c.collider_box(f"{name}", (0.7, 0.55, 1.55), (0, 0, 0.775), coll)]
    for sx in (-1, 1):
        col.append(c.collider_box(f"{name}_post{sx + 1}", (0.16, 0.16, 0.9), (sx * 0.45, 0.62, 0.45), coll))
    # Tuyau au sol en L : de l'enrouleur vers +Y, puis le long de X.
    if hose_to is not None:
        hx, hy = hose_to
        x0 = 0.42
        path = [(x0, 0.0), (x0, hy), (hx, hy)]
        vis.append(_cyl(f"{name}_hose_drop", hose, 0.5, (x0, 0.0, 0.35), m["rubber"], coll, segments=8))
        for i, (a, b) in enumerate(zip(path[:-1], path[1:])):
            if math.dist(a, b) < 0.05:
                continue
            axis = "Y" if abs(b[1] - a[1]) > abs(b[0] - a[0]) else "X"
            mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, hose)
            vis.append(_cyl(f"{name}_hose{i}", hose, math.dist(a, b), mid, m["rubber"], coll, axis=axis, segments=8))
        vis.append(_box(f"{name}_hose_end", (0.14, 0.1, 0.1), (hx, hy, 0.05), m["bare"] if fuel else m["paint_yellow"], coll, bevel=0.01))
    _place(vis + col, center, facing)
    return _finish(vis, col, [], detail)
