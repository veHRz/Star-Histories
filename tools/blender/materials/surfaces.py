"""Matériaux de surface de la bibliothèque « futur usé » (tôles, métaux, sols, coque).

Chaque générateur reçoit la résolution de travail et la taille réelle de la tuile (mètres) et
renvoie les cartes de `texgen.finish` (couleur, normale, ORM). Hauteurs en mètres.
"""

from __future__ import annotations

import math

import numpy as np

from texgen import (
    DUST, GRIME, METAL_MAX, PRIMER, RUST, STEEL, Canvas, chip_field, cut, drips, even_spacing, fill,
    finish, lerp, panel_field, rgb, rivets_along_x, screw, sd_box, sd_segment, seam_profile, smoothstep,
    stains, tone,
)


def _rivet(r, height):
    return lambda gx, gy: height * np.sqrt(np.clip(1.0 - (np.hypot(gx, gy) / r) ** 2, 0.0, 1.0))


def panel_painted(res: int = 1024, tile_m: float = 2.0):
    """Tôle peinte claire teintable : panneaux de 1 × 0,5 m (un coupé en deux), rivets le long
    des joints horizontaux, vis aux coins, trappe vissée, plaque signalétique, petite grille."""
    cv = Canvas(res, tile_m, seed=101)
    rects = [
        (0.0, 0.0, 1.0, 0.5), (1.0, 0.0, 2.0, 0.5),
        (0.0, 0.5, 1.0, 1.0), (1.0, 0.5, 1.5, 1.0), (1.5, 0.5, 2.0, 1.0),
        (0.0, 1.0, 1.0, 1.5), (1.0, 1.0, 2.0, 1.5),
        (0.0, 1.5, 1.0, 2.0), (1.0, 1.5, 2.0, 2.0),
    ]
    d, ident = panel_field(cv, rects, corner=0.012)
    rng = cv.rng(1)
    n = len(rects)
    shade = 1.0 + rng.normal(0, 0.03, n)
    shade[3] = 0.9    # panneau remplacé, plus sombre et un peu froid
    shade[6] = 1.05
    hue = np.ones((n, 3), dtype=np.float32)
    hue[3] = (0.96, 0.99, 1.04)
    hue[7] = (1.02, 1.0, 0.97)
    tilt = rng.normal(0, 0.0012, (n, 2))
    lift = rng.normal(0, 0.0003, n)
    centers = np.array([((r[0] + r[2]) / 2, (r[1] + r[3]) / 2) for r in rects], dtype=np.float32)

    h = seam_profile(d, 0.0025, 0.005, 0.0035)
    inside = smoothstep(0.003, 0.012, d)
    ox = cv.wrap_dx(cv.x, centers[ident, 0])
    oy = cv.wrap_dy(cv.y, centers[ident, 1])
    h += inside * (lift[ident] + tilt[ident, 0] * ox + tilt[ident, 1] * oy)
    h += inside * cv.noise(2, 0.5, 0.06, beta=1.2) * 0.0003

    # Rivets le long des joints horizontaux de 0 et 1 m, vis aux coins des autres rangées.
    riv = cv.zeros()
    for yl in (0.0, 1.0):
        for side in (-1, 1):
            riv = np.maximum(riv, rivets_along_x(cv, yl + side * 0.022, 0.077, 0.0065, 0.002, x_off=0.0385))
    riv *= smoothstep(0.014, 0.02, d)
    screws = cv.zeros()
    for (x0, y0, x1, y1) in rects:
        if (y0 % 1.0) == 0.5:
            for sx, sy in ((x0 + 0.035, y0 + 0.035), (x1 - 0.035, y0 + 0.035), (x0 + 0.035, y1 - 0.035), (x1 - 0.035, y1 - 0.035)):
                a = float(rng.uniform(0, math.pi))
                cv.stamp(screws, sx, sy, 0.012, 0.012, lambda gx, gy, a=a: screw(gx, gy, 0.0075, a, 0.0013))

    hatch_cx, hatch_cy = 0.5, 1.75
    hd = sd_box(cv.wrap_dx(cv.x, hatch_cx), cv.wrap_dy(cv.y, hatch_cy), 0.3, 0.14, 0.025)
    hatch_line = 1.0 - smoothstep(0.0, 0.0035, np.abs(hd))
    h += -0.002 * hatch_line + np.where(hd < 0, -0.0005, 0.0)
    for sx in (hatch_cx - 0.27, hatch_cx + 0.27):
        for sy in (hatch_cy - 0.11, hatch_cy + 0.11):
            cv.stamp(screws, sx, sy, 0.012, 0.012, lambda gx, gy: screw(gx, gy, 0.0075, 0.3, 0.0013))
    plate_cx, plate_cy = 0.2, 0.62
    pd = sd_box(cv.wrap_dx(cv.x, plate_cx), cv.wrap_dy(cv.y, plate_cy), 0.075, 0.038, 0.004)
    plate = fill(pd, cv.px)
    h += plate * 0.0008
    lines = cv.zeros()
    for k, ln in enumerate((0.1, 0.065, 0.11)):
        yy = plate_cy + 0.016 - k * 0.014
        ld = sd_box(cv.wrap_dx(cv.x, plate_cx - 0.055 + ln / 2), cv.wrap_dy(cv.y, yy), ln / 2, 0.003)
        lines = np.maximum(lines, fill(ld, cv.px))
    for sx in (plate_cx - 0.062, plate_cx + 0.062):
        for sy in (plate_cy - 0.026, plate_cy + 0.026):
            cv.stamp(riv, sx, sy, 0.006, 0.006, _rivet(0.0035, 0.001))
    vent_cx, vent_cy = 1.25, 0.75
    vent = cv.zeros()
    for k in range(5):
        vd = sd_box(cv.wrap_dx(cv.x, vent_cx), cv.wrap_dy(cv.y, vent_cy - 0.08 + k * 0.04), 0.16, 0.009, 0.008)
        vent = np.maximum(vent, fill(vd, cv.px * 1.5))
    h += -0.012 * vent + riv + screws

    # Usure : écaillures sur les arêtes (apprêt puis métal), autour des têtes et de la trappe.
    edge = 1.0 - smoothstep(0.003, 0.03, d)
    edge = np.maximum(edge, 0.8 * (1.0 - smoothstep(0.0, 0.025, np.abs(hd))))
    edge = np.maximum(edge, 0.7 * np.clip(cv.blur(((riv + screws) > 0.0004).astype(np.float32), 0.004) * 2.5, 0, 1))
    chips = chip_field(cv, 3, edge, 0.009, edge_weight=3.0)
    primer = cut(chips, 0.035)
    bare = cut(chips, 0.016)
    h += -0.0002 * primer - 0.0001 * bare

    scr = cv.scratches(5, 150, (0.02, 0.14), 0.0011, curve=0.15)
    scr_dark = cv.scratches(6, 40, (0.04, 0.2), 0.0012)
    # Crasse : coulures sous les rivets, la grille et les joints horizontaux ; mains autour de la
    # trappe ; variation lente.
    horiz = (1.0 - smoothstep(0.0, 0.005, np.abs(cv.wrap_dy(cv.y, 0.0)))) + (1.0 - smoothstep(0.0, 0.005, np.abs(cv.wrap_dy(cv.y, 1.0))))
    src = np.maximum(horiz * (cv.noise(7, 0.1, 0.01) > 0.4), ((riv + screws) > 0.0006) * 0.8)
    src = np.maximum(src, vent)
    streaks = drips(cv, src, 0.2, 8)
    hands = np.clip(1.0 - np.maximum(np.abs(hd), 0) / 0.12, 0, 1) ** 2 * stains(cv, 9, 0.15, 0.5, 0.6)
    grime = np.clip(0.4 * streaks + 0.25 * hands, 0, 1)

    base = np.array((0.63, 0.63, 0.62), dtype=np.float32)
    paint = base[None, None, :] * (shade[ident][..., None] * hue[ident]) * tone(cv, 10, 1.0, 0.025)[..., None]
    paint = lerp(paint, rgb((0.26, 0.27, 0.28)), plate)
    paint = lerp(paint, rgb((0.82, 0.82, 0.78)), lines * plate)
    col = lerp(paint, rgb(PRIMER), primer)
    col = lerp(col, rgb(STEEL) * tone(cv, 11, 0.03, 0.08)[..., None], bare)
    col = lerp(col, rgb((0.82, 0.8, 0.76)), scr * 0.4 * (1 - plate))
    col = lerp(col, rgb(GRIME), scr_dark * 0.35)
    col = lerp(col, rgb(GRIME), grime)
    col = lerp(col, rgb((0.04, 0.04, 0.04)), vent)

    rough = 0.55 + 0.04 * cv.noise(12, 0.2, 0.01) + 0.12 * primer + 0.2 * grime - 0.2 * bare - 0.12 * scr
    rough = lerp(rough, 0.42, plate)
    metal = METAL_MAX * bare + 0.25 * scr * (1 - primer) + 0.3 * plate
    return finish(cv, h, col, rough, metal, [(0.004, 0.0015, 0.45), (0.02, 0.0025, 0.3)], ao_extra=1.0 - 0.7 * vent, edge_light=0.12)


def metal_dark(res: int = 1024, tile_m: float = 2.0):
    """Anthracite usé (encadrements, nervures, poutres) : peinture poudrée sur acier, frottements
    horizontaux, rares éclats regroupés, rayures, traces d'huile."""
    cv = Canvas(res, tile_m, seed=202)
    h = 0.00004 * cv.noise(1, 0.004, 0.001) + 0.0003 * cv.noise(2, 0.4, 0.05)
    rub = cut(cv.noise(3, 0.4, 0.01, stretch=(6.0, 1.0)), 0.2, 0.8)
    # Éclats rares, groupés en deux ou trois zones de chocs (pas un piqueté uniforme).
    chips = cut(chip_field(cv, 4, np.zeros_like(h), 0.02) + 0.6 * cv.noise(41, 0.6, 0.15), 0.006, 0.06)
    h -= 0.00015 * chips
    scr = np.maximum(cv.scratches(5, 110, (0.02, 0.15), 0.0009, curve=0.08), cv.scratches(6, 50, (0.15, 0.45), 0.0008, angle=(0.0, 0.06)) * 0.8)
    h -= 0.00005 * scr
    oil = stains(cv, 7, 0.35, 0.06, 0.5)
    base = rgb((0.215, 0.22, 0.23)) * tone(cv, 8, 1.0, 0.05)[..., None]
    col = lerp(base, rgb((0.3, 0.3, 0.305)), rub * 0.4)
    col = lerp(col, rgb((0.11, 0.105, 0.1)), oil * 0.45)
    col = lerp(col, rgb(STEEL), np.clip(chips + scr * 0.4, 0, 1))
    rough = 0.52 + 0.04 * cv.noise(10, 0.2, 0.01) + 0.08 * rub - 0.2 * oil - 0.15 * scr - 0.12 * chips
    metal = 0.12 + 0.4 * np.clip(chips + scr, 0, 1) + 0.08 * rub
    return finish(cv, h, col, rough, metal, [(0.003, 0.0004, 0.4)], edge_light=0.06)


def metal_bare(res: int = 1024, tile_m: float = 2.0):
    """Métal brut brossé le long de U (machines, rails, tuyaux) : stries, rayures, rares taches
    de graisse, traces de doigts plus lisses."""
    cv = Canvas(res, tile_m, seed=303)
    brushed = cv.noise(1, 0.6, 0.0015, beta=0.6, stretch=(30.0, 1.0))
    brushed2 = cv.noise(2, 0.2, 0.002, beta=0.8, stretch=(12.0, 1.0))
    h = 0.000015 * brushed + 0.0002 * cv.noise(3, 0.5, 0.05)
    scr = np.maximum(cv.scratches(4, 380, (0.01, 0.12), 0.0007, curve=0.3), cv.scratches(5, 60, (0.2, 0.6), 0.0006, angle=(0.0, 0.05)) * 0.8)
    h -= 0.00004 * scr
    grease = stains(cv, 6, 0.5, 0.06, 0.9)
    # Stries discrètes : trop marquées, le brossé passe pour du bois sous une lumière chaude.
    base = rgb((0.64, 0.64, 0.635)) * (1.0 + 0.016 * brushed[..., None] + 0.01 * brushed2[..., None]) * tone(cv, 8, 1.2, 0.04)[..., None]
    col = lerp(base, rgb(GRIME) * 1.5, grease * 0.3)
    col = lerp(col, rgb((0.76, 0.76, 0.75)), scr * 0.4)
    rough = 0.36 + 0.05 * brushed2 + 0.25 * grease - 0.08 * scr
    metal = METAL_MAX - 0.25 * grease + 0.0 * scr
    return finish(cv, h, col, rough, metal, [(0.002, 0.0002, 0.3)], edge_light=0.03)


def floor_plate(res: int = 1024, tile_m: float = 2.0):
    """Tôle larmée antidérapante, plaques de 1 m boulonnées ; sommets des losanges polis par
    le passage, crasse dans les creux, rares taches d'huile."""
    cv = Canvas(res, tile_m, seed=404)
    pitch = cv.tile / 56.0  # 35,7 mm
    ix = np.floor(cv.x / pitch).astype(np.int32)
    iy = np.floor(cv.y / pitch).astype(np.int32)
    lx = (cv.x % pitch) - pitch / 2
    ly = (cv.y % pitch) - pitch / 2
    sign = np.where((ix + iy) % 2 == 0, 1.0, -1.0)
    u = (lx + sign * ly) * 0.7071
    v = (-sign * lx + ly) * 0.7071
    e = (u / 0.0145) ** 2 + (v / 0.0042) ** 2
    lozenge = smoothstep(1.0, 0.35, e)
    h = 0.0015 * lozenge
    rects = [(0, 0, 1, 1), (1, 0, 2, 1), (0, 1, 1, 2), (1, 1, 2, 2)]
    d, ident = panel_field(cv, rects, corner=0.005)
    margin = smoothstep(0.01, 0.018, d)
    h = h * margin + seam_profile(d, 0.002, 0.004, 0.003)
    lozenge = lozenge * margin

    bolt_val = cv.zeros()
    bolt_mask = cv.zeros()

    def bolt(gx, gy):
        dd = np.hypot(gx, gy)
        head = -0.0002 + 0.0005 * np.sqrt(np.clip(1 - (dd / 0.007) ** 2, 0, 1))
        cup = -0.001 * np.clip(1 - (dd - 0.007) / 0.006, 0, 1)
        slot = (np.abs(gx - gy) < 0.0012) & (dd < 0.006)
        return np.where(dd < 0.007, np.where(slot, -0.0006, head), cup) * (dd < 0.013)

    for (x0, y0, x1, y1) in rects:
        for t in np.arange(0.125, 1.0, 0.25):
            for (bx, by) in ((x0 + t, y0 + 0.032), (x0 + t, y1 - 0.032), (x0 + 0.032, y0 + t), (x1 - 0.032, y0 + t)):
                cv.stamp(bolt_val, bx, by, 0.014, 0.014, bolt, "add")
                cv.stamp(bolt_mask, bx, by, 0.014, 0.014, lambda gx, gy: smoothstep(0.014, 0.012, np.hypot(gx, gy)))
    h = lerp(h, bolt_val, bolt_mask)

    rng = cv.rng(2)
    plate_tone = 1.0 + rng.normal(0, 0.04, 4)
    traffic = np.clip(0.55 + 0.35 * cv.noise(3, 0.9, 0.15), 0, 1)
    polish = lozenge * np.clip(traffic + 0.15 * cv.noise(4, 0.03, 0.004), 0, 1)
    valley = (1.0 - lozenge) * margin
    oil = stains(cv, 5, 0.45, 0.018, 0.6)
    rust = stains(cv, 7, 0.08, 0.03, 0.4) * valley
    scr = cv.scratches(9, 160, (0.03, 0.25), 0.001, curve=0.1)
    base = rgb((0.37, 0.375, 0.37)) * (plate_tone[ident] * tone(cv, 10, 0.7, 0.04))[..., None]
    col = lerp(base, base * 0.55, valley * 0.6)
    col = lerp(col, rgb(STEEL) * 1.12, polish * 0.85)
    col = lerp(col, rgb(RUST), rust * 0.45)
    col = lerp(col, rgb((0.07, 0.065, 0.06)), oil * 0.5)
    col = lerp(col, rgb((0.52, 0.51, 0.49)), scr * 0.45)
    col = lerp(col, rgb((0.2, 0.2, 0.2)), bolt_mask * 0.4)
    rough = 0.6 - 0.25 * polish + 0.1 * valley - 0.3 * oil + 0.1 * rust
    metal = 0.12 + 0.43 * polish + 0.2 * scr - 0.1 * rust
    return finish(cv, h, col, rough, metal, [(0.004, 0.0008, 0.45), (0.02, 0.0015, 0.2)], edge_light=0.1)


def grating(res: int = 1024, tile_m: float = 2.0):
    """Caillebotis opaque (barres porteuses le long de V, entretoises le long de U), trous
    sombres avec un longeron devinable dessous ; émissif = lumière qui passe par les trous."""
    cv = Canvas(res, tile_m, seed=505)
    pitch_x = cv.tile / 48.0  # 41,7 mm
    pitch_y = cv.tile / 12.0  # 167 mm
    bx = np.abs((cv.x % pitch_x) - pitch_x / 2)
    by = np.abs((cv.y % pitch_y) - pitch_y / 2)
    bearing = smoothstep(0.0072, 0.0056, bx)  # ≈ 13 mm de dessus
    cross = smoothstep(0.0045, 0.0031, by)
    top = np.maximum(bearing, cross)
    serr = 0.5 + 0.5 * np.cos(cv.y / (cv.tile / 400.0) * 2 * math.pi)
    hole = 1.0 - top
    # Profondeur visible dans les trous : un longeron tous les 50 cm le long de U.
    beam = smoothstep(0.03, 0.02, np.abs((cv.x % 0.5) - 0.25))
    h = -0.035 * hole + 0.012 * hole * beam + bearing * 0.0004 * serr - cross * (1 - bearing) * 0.003
    joint = bearing * cross
    grime = np.clip(cv.blur(joint, 0.005) * 3.0, 0, 1) * tone(cv, 2, 0.2, 0.4)
    rust = stains(cv, 3, 0.1, 0.05, 0.4) * top
    polish = bearing * np.clip(0.55 + 0.35 * cv.noise(4, 0.8, 0.1), 0, 1) * (serr > 0.45)
    base = rgb((0.50, 0.505, 0.495)) * tone(cv, 5, 0.8, 0.06)[..., None]
    col = lerp(base, base * 0.75, cross * (1 - bearing))
    col = lerp(col, rgb(GRIME), grime * 0.55)
    col = lerp(col, rgb(RUST), rust * 0.45)
    col = lerp(col, rgb((0.68, 0.68, 0.66)), polish * 0.35)
    # Le fond n'est pas noir : on devine le dessous (tôle sombre, longeron plus clair).
    deep = lerp(rgb((0.055, 0.058, 0.062)) * tone(cv, 7, 0.6, 0.2)[..., None], rgb((0.13, 0.13, 0.13)), beam)
    col = lerp(col, deep, smoothstep(0.2, 0.9, hole))
    rough = 0.55 + 0.2 * grime - 0.2 * polish + 0.3 * hole
    metal = (0.45 - 0.25 * grime - 0.2 * rust) * top
    glow = cv.blur(hole * (1 - 0.7 * beam), 0.002) * tone(cv, 6, 1.5, 0.08)
    emis = np.repeat(np.clip(glow, 0, 1)[..., None], 3, axis=-1)
    ao_hole = 1.0 - 0.9 * smoothstep(0.1, 0.8, hole) * (1 - 0.5 * beam)
    return finish(cv, h, col, rough, metal, [(0.004, 0.004, 0.35)], ao_extra=ao_hole, emissive=emis, normal_strength=0.5, edge_light=0.1)


def rubber(res: int = 1024, tile_m: float = 1.0):
    """Caoutchouc noir mat : grain fin, poussière douce, frottements plus lisses, entailles."""
    cv = Canvas(res, tile_m, seed=606)
    grain = cv.noise(1, 0.004, 0.0008)
    h = 0.00002 * grain + 0.00025 * cv.noise(2, 0.2, 0.03)
    dust = np.clip(0.5 + 0.5 * cv.noise(3, 0.4, 0.05) + 0.15 * grain, 0, 1)
    scuff = stains(cv, 4, 0.12, 0.08, 0.5)
    cuts = cv.scratches(5, 60, (0.008, 0.05), 0.0007)
    h -= 0.0002 * cuts
    col = rgb((0.08, 0.08, 0.085)) * tone(cv, 6, 0.1, 0.06)[..., None]
    col = lerp(col, rgb((0.25, 0.24, 0.225)), dust * 0.35)
    col = lerp(col, rgb((0.055, 0.055, 0.055)), scuff * 0.4)
    col = lerp(col, rgb((0.2, 0.2, 0.2)), cuts * 0.6)
    rough = 0.86 + 0.03 * grain + 0.06 * dust - 0.3 * scuff
    return finish(cv, h, col, rough, np.zeros_like(rough), [(0.002, 0.0002, 0.3), (0.01, 0.0005, 0.2)], edge_light=0.0)


def fabric(res: int = 1024, tile_m: float = 1.0):
    """Similicuir matelassé teintable : bourrelets horizontaux de 12,5 cm, coutures piquées,
    grain, sommets usés et craquelés."""
    cv = Canvas(res, tile_m, seed=707)
    ch = cv.tile_h / 8.0
    t = (cv.y % ch) / ch
    puff = np.sin(np.pi * t) ** 0.5
    seam_d = np.minimum(t, 1 - t) * ch
    h = 0.008 * puff
    stitch = cv.zeros()
    sp = even_spacing(cv.tile, 0.007)
    sx = (cv.x % sp) - sp / 2
    for k in range(8):
        for off in (0.0055, -0.0055):
            dy = cv.wrap_dy(cv.y, k * ch + off)
            stitch = np.maximum(stitch, fill(sd_box(sx, dy, 0.0022, 0.0006, 0.0005), cv.px))
    h -= 0.0004 * stitch
    f1, f2, _ = cv.voronoi(1, 240)
    crease = 1.0 - smoothstep(0.0, 0.0011, f2 - f1)
    h -= 0.00007 * crease
    h += 0.00025 * cv.noise(2, 0.05, 0.004, stretch=(1.0, 2.5)) * puff
    wear = np.clip((puff ** 4) * (0.55 + 0.35 * cv.noise(3, 0.4, 0.004, beta=1.1)) - 0.3, 0, 1) * 1.8
    cracks = crease * smoothstep(0.35, 0.8, wear)
    base = rgb((0.48, 0.465, 0.45)) * tone(cv, 4, 0.5, 0.04)[..., None]
    col = lerp(base, rgb((0.6, 0.58, 0.55)), np.clip(wear, 0, 1) * 0.35)
    col = lerp(col, rgb((0.22, 0.2, 0.18)), cracks * 0.45)
    col = lerp(col, rgb((0.66, 0.64, 0.6)), stitch * 0.6)
    col = lerp(col, rgb(GRIME), (1.0 - smoothstep(0.0, 0.01, seam_d)) * 0.4)
    rough = 0.48 + 0.05 * cv.noise(5, 0.1, 0.006) + 0.25 * np.clip(wear, 0, 1) + 0.12 * crease
    return finish(cv, h, col, rough, np.zeros_like(rough), [(0.006, 0.002, 0.45), (0.025, 0.004, 0.35)], edge_light=0.05)


def container_ribbed(res: int = 1024, tile_m: float = 2.0):
    """Tôle nervurée teintable (caisses, conteneurs) : nervures trapézoïdales de 12,5 cm le long
    de V, crêtes écaillées, coulures de rouille, bosses."""
    cv = Canvas(res, tile_m, seed=808)
    pitch = cv.tile / 16.0
    t = (cv.x % pitch) / pitch
    prof = smoothstep(0.0, 1.0, np.clip(np.minimum(t / 0.1, (0.6 - t) / 0.1), 0.0, 1.0))
    h = 0.018 * prof
    crest = np.clip(1.0 - np.minimum(np.abs(t - 0.1), np.abs(t - 0.5)) / 0.035, 0, 1)
    dents = cv.zeros()
    rng = cv.rng(1)
    for _ in range(8):
        cx, cy = rng.random() * cv.tile, rng.random() * cv.tile
        r = rng.uniform(0.05, 0.12)
        dp = rng.uniform(0.002, 0.005)
        cv.stamp(dents, cx, cy, r, r, lambda gx, gy, r=r, dp=dp: dp * np.clip(1 - (np.hypot(gx, gy) / r) ** 2, 0, 1) ** 2, "max")
    h -= dents
    dent_rim = np.clip(cv.blur((dents > 0.0008).astype(np.float32), 0.01) * 1.5, 0, 1) * (dents < 0.0015)
    chips = chip_field(cv, 2, np.maximum(crest, 0.7 * dent_rim), 0.008)
    primer = cut(chips, 0.05)
    bare = cut(chips, 0.025)
    rust = drips(cv, bare * (cv.noise(3, 0.2, 0.02) > 0.3), 0.3, 4)
    trough = 1.0 - prof
    grime = np.clip(0.3 * trough * tone(cv, 5, 0.6, 0.5) + 0.35 * rust, 0, 1)
    scr = cv.scratches(6, 120, (0.03, 0.2), 0.001, angle=(0.0, 0.25))
    base = rgb((0.66, 0.66, 0.645)) * tone(cv, 7, 0.8, 0.04)[..., None]
    col = lerp(base, rgb(PRIMER), primer)
    col = lerp(col, rgb(STEEL) * 0.9, bare)
    col = lerp(col, rgb(RUST), rust * 0.5)
    col = lerp(col, rgb(GRIME), grime * 0.45)
    col = lerp(col, rgb((0.82, 0.8, 0.76)), scr * 0.4)
    rough = 0.56 + 0.04 * cv.noise(8, 0.12, 0.008) + 0.15 * grime + 0.1 * rust - 0.2 * bare
    metal = METAL_MAX * bare + 0.2 * scr
    return finish(cv, h, col, rough, metal, [(0.01, 0.004, 0.35), (0.04, 0.008, 0.25)], edge_light=0.08)


def ferrocrete(res: int = 1024, tile_m: float = 4.0):
    """Béton du spatioport : joints tous les 2 m, granulats, pores, fissures rares, taches
    d'huile et traces de roues."""
    cv = Canvas(res, tile_m, seed=909)
    pores = cut(-cv.noise(1, 0.008, cv.px), 0.025, 0.2)
    light_agg = cut(cv.noise(2, 0.01, cv.px * 1.2), 0.06, 0.2)
    dark_agg = cut(cv.noise(3, 0.012, cv.px * 1.2), 0.05, 0.2)
    h = 0.0003 * cv.noise(4, 0.1, 0.01) + 0.00008 * cv.noise(5, 0.012, cv.px) - 0.0008 * pores
    jd = np.minimum(np.minimum(cv.x % 2.0, 2.0 - cv.x % 2.0), np.minimum(cv.y % 2.0, 2.0 - cv.y % 2.0))
    joint = 1.0 - smoothstep(0.004, 0.007, jd)
    chipped = (1.0 - smoothstep(0.006, 0.02, jd)) * cut(cv.noise(6, 0.08, 0.01), 0.25, 0.3)
    h -= 0.006 * joint + 0.002 * chipped
    f1, f2, _ = cv.voronoi(7, 9)
    crack = (1.0 - smoothstep(0.0, 0.004, f2 - f1)) * cut(cv.noise(8, 0.8, 0.1), 0.18, 0.3)
    crack *= cut(cv.noise(9, 0.05, 0.005), 0.6, 0.3)
    h -= 0.001 * crack
    oil = stains(cv, 10, 0.5, 0.05, 0.5)
    tyre = stains(cv, 11, 0.6, 0.06, 0.6) * np.clip(0.5 + 0.5 * cv.noise(12, 0.5, 0.01, stretch=(15.0, 1.0)), 0, 1)
    base = rgb((0.52, 0.51, 0.485)) * (tone(cv, 13, 2.0, 0.06) * tone(cv, 14, 0.5, 0.04) * tone(cv, 15, 0.08, 0.025))[..., None]
    col = lerp(base, rgb((0.64, 0.62, 0.58)), light_agg * 0.3)
    col = lerp(col, rgb((0.36, 0.35, 0.33)), dark_agg * 0.3)
    col = lerp(col, rgb((0.14, 0.13, 0.12)), oil * 0.45)
    col = lerp(col, rgb((0.2, 0.2, 0.19)), tyre * 0.3)
    col = lerp(col, rgb((0.25, 0.24, 0.23)), np.clip(pores + crack + chipped * 0.5, 0, 1) * 0.6)
    col = lerp(col, rgb((0.1, 0.1, 0.1)), joint * 0.85)
    rough = 0.9 + 0.03 * cv.noise(16, 0.1, 0.01) - 0.3 * oil - 0.08 * tyre
    return finish(cv, h, col, rough, np.zeros_like(rough), [(0.006, 0.001, 0.4), (0.03, 0.003, 0.3)], edge_light=0.04)


def floor_tiles(res: int = 1024, tile_m: float = 2.0):
    """Dalles de 50 cm usées (hall, bureau), teintables : joints sales, deux dalles remplacées,
    paillettes, quelques coins écornés, une dalle fendue, rayures de passage."""
    cv = Canvas(res, tile_m, seed=1010)
    size = 0.5
    rects = [(i * size, j * size, (i + 1) * size, (j + 1) * size) for j in range(4) for i in range(4)]
    d, ident = panel_field(cv, rects, corner=0.006)
    h = seam_profile(d, 0.0025, 0.003, 0.0025)
    rng = cv.rng(1)
    shade = 1.0 + rng.normal(0, 0.03, 16)
    hue = np.ones((16, 3), dtype=np.float32)
    for k in (5, 14):
        shade[k] = 1.08
        hue[k] = (0.98, 1.0, 1.03)
    tilt = rng.normal(0, 0.0015, (16, 2))
    centers = np.array([((r[0] + r[2]) / 2, (r[1] + r[3]) / 2) for r in rects], dtype=np.float32)
    inside = smoothstep(0.002, 0.008, d)
    h += inside * (tilt[ident, 0] * cv.wrap_dx(cv.x, centers[ident, 0]) + tilt[ident, 1] * cv.wrap_dy(cv.y, centers[ident, 1]))
    _, _, cid = cv.voronoi(2, 360)
    fleck = (np.sin(cid * 12.9898) * 43758.5453) % 1.0
    traffic = np.clip(0.5 + 0.35 * cv.noise(3, 1.0, 0.02, beta=1.1), 0, 1)
    chips = cv.zeros()
    for (x0, y0, x1, y1) in rects:
        for (qx, qy) in ((x0, y0), (x1, y1)):
            if rng.random() < 0.35:
                r = rng.uniform(0.008, 0.02)
                ang = rng.uniform(0, 2 * math.pi)
                cv.stamp(chips, qx + 0.006 * math.cos(ang), qy + 0.006 * math.sin(ang), r * 1.5, r * 1.5,
                         lambda gx, gy, r=r: smoothstep(r, r * 0.6, np.hypot(gx, gy)))
    chips *= inside * np.clip(0.6 + 0.6 * cv.noise(4, 0.02, 0.003), 0, 1)
    h -= 0.0012 * chips
    crack = cv.zeros()
    rr = cv.rng(7)
    px0, py0 = 1.04, 0.62
    pts = [(px0, py0)]
    for _ in range(7):
        px0 += rr.uniform(0.03, 0.06)
        py0 += rr.uniform(-0.04, 0.05)
        pts.append((px0, py0))
    for a, b in zip(pts[:-1], pts[1:]):
        cx, cy = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
        cv.stamp(crack, cx, cy, 0.05, 0.05, lambda gx, gy, a=a, b=b, cx=cx, cy=cy: np.clip(1 - sd_segment(gx, gy, a[0] - cx, a[1] - cy, b[0] - cx, b[1] - cy) / 0.0012, 0, 1))
    crack *= (ident == 6) * 1.0
    h -= 0.0008 * crack
    scr = cv.scratches(8, 260, (0.02, 0.15), 0.0008, density=0.2 + 0.8 * traffic, curve=0.2)
    grout = 1.0 - smoothstep(0.0, 0.005, d)
    edge_dirt = (1.0 - smoothstep(0.003, 0.025, d)) * tone(cv, 9, 0.1, 0.5)
    base = rgb((0.60, 0.59, 0.56)) * (shade[ident][..., None] * hue[ident]) * tone(cv, 10, 0.8, 0.03)[..., None]
    base = base * (1.0 + 0.05 * (fleck[..., None] - 0.5))
    col = lerp(base, rgb((0.72, 0.71, 0.68)), (fleck > 0.94) * 0.4)
    col = lerp(col, rgb((0.42, 0.41, 0.39)), (fleck < 0.05) * 0.4)
    col = col * (1.0 - 0.06 * traffic[..., None])
    col = lerp(col, rgb(GRIME), np.clip(edge_dirt * 0.22 + grout * 0.6 + crack * 0.7, 0, 1))
    col = lerp(col, rgb((0.34, 0.33, 0.31)), chips * 0.8)
    col = lerp(col, rgb((0.74, 0.73, 0.7)), scr * 0.3)
    rough = 0.36 + 0.2 * traffic + 0.03 * cv.noise(11, 0.1, 0.006) + 0.35 * grout + 0.2 * chips + 0.1 * scr
    return finish(cv, h, col, rough, np.zeros_like(rough), [(0.004, 0.0012, 0.4), (0.02, 0.002, 0.25)], edge_light=0.06)


def hull_plates(res: int = 1024, tile_m: float = 4.0):
    """Blindage de coque extérieure teintable : plaques décalées en rangées de 1 m, rivets,
    trappes, patchwork de teintes lisible de loin, suie étirée le long de +U, impacts."""
    cv = Canvas(res, tile_m, seed=1111)
    rng = cv.rng(1)
    rects = []
    offsets = [0.0, 0.7, 0.3, 1.2]
    widths = [[1.5, 1.0, 1.5], [2.0, 1.0, 1.0], [1.0, 1.5, 1.5], [1.25, 1.5, 1.25]]
    for j in range(4):
        x = offsets[j]
        for w in widths[j]:
            rects.append((x, float(j), x + w, float(j + 1)))
            x += w
    d, ident = panel_field(cv, rects, corner=0.03)
    n = len(rects)
    h = seam_profile(d, 0.005, 0.012, 0.006)
    shade = 1.0 + rng.normal(0, 0.045, n)
    warm = rng.normal(0, 0.012, n)
    shade[4] = 0.84
    shade[9] = 1.1
    hue = np.stack([1 + warm, np.ones(n), 1 - warm], -1).astype(np.float32)
    tilt = rng.normal(0, 0.0012, (n, 2))
    centers = np.array([((r[0] + r[2]) / 2, (r[1] + r[3]) / 2) for r in rects], dtype=np.float32)
    inside = smoothstep(0.004, 0.02, d)
    h += inside * (tilt[ident, 0] * cv.wrap_dx(cv.x, centers[ident, 0]) + tilt[ident, 1] * cv.wrap_dy(cv.y, centers[ident, 1]))
    riv = cv.zeros()
    for j in range(4):
        for side in (-1, 1):
            riv = np.maximum(riv, rivets_along_x(cv, j + side * 0.045, 0.1, 0.012, 0.003, x_off=0.05))
    riv *= smoothstep(0.025, 0.035, d)
    h += riv
    hatch = cv.zeros()
    for (hx, hy, hw, hh) in ((0.75, 0.5, 0.25, 0.18), (2.9, 2.5, 0.18, 0.28), (1.6, 3.45, 0.3, 0.14)):
        hd = sd_box(cv.wrap_dx(cv.x, hx), cv.wrap_dy(cv.y, hy), hw, hh, 0.03)
        hatch = np.maximum(hatch, 1.0 - smoothstep(0.0, 0.006, np.abs(hd)))
    h -= 0.003 * hatch
    pits = cut(-cv.noise(2, 0.02, cv.px * 1.5) + 0.8 * cv.noise(21, 0.5, 0.1), 0.004, 0.3)  # impacts rares, groupés
    h -= 0.0008 * pits
    edge = 1.0 - smoothstep(0.006, 0.05, d)
    bare = cut(chip_field(cv, 3, np.maximum(edge, hatch * 0.7), 0.02), 0.025)
    soot_src = np.clip((riv > 0.001) * 1.0 + hatch * 0.8, 0, 1) * (cv.noise(4, 0.8, 0.2) > -0.2)
    soot = cv.smear(soot_src.astype(np.float32), 0.9, dx=1, dy=0)
    soot = np.clip(soot * np.clip(0.4 + 0.6 * cv.noise(5, 0.6, 0.01, stretch=(15.0, 1.0)), 0, 1), 0, 1)
    scorch = smoothstep(1.6, 3.0, cv.noise(6, 1.5, 0.08, stretch=(2.5, 1.0))) * 0.6
    base = rgb((0.62, 0.62, 0.61)) * (shade[ident][..., None] * hue[ident]) * (tone(cv, 7, 2.0, 0.05) * tone(cv, 8, 0.12, 0.02))[..., None]
    col = lerp(base, rgb(STEEL) * 0.85, bare)
    col = lerp(col, rgb((0.1, 0.095, 0.09)), np.clip(soot * 0.4 + scorch * 0.55, 0, 1))
    col = lerp(col, rgb((0.2, 0.2, 0.2)), pits * 0.6)
    rough = 0.55 + 0.04 * cv.noise(9, 0.12, 0.01) + 0.2 * scorch + 0.1 * soot - 0.2 * bare
    metal = 0.1 + 0.4 * bare - 0.1 * scorch
    return finish(cv, h, col, rough, metal, [(0.01, 0.003, 0.45), (0.05, 0.006, 0.3)], edge_light=0.1)
