"""Matériaux « appareils » de la bibliothèque : écrans segmentés, panneau de boutons
rétroéclairés, diffuseur de bandeau lumineux. Ils ont une carte émissive (sRGB).

Contenu générique uniquement : chiffres, barres, courbes, tirets en guise de texte ; aucun mot,
aucun logo.
"""

from __future__ import annotations

import math

import numpy as np

from texgen import Canvas, cut, even_spacing, fill, finish, lerp, rgb, screw, sd_box, sd_circle, sd_segment, seam_profile, smoothstep

_AMBER = (1.0, 0.6, 0.16)
_CYAN = (0.35, 0.86, 1.0)
_GREEN = (0.45, 1.0, 0.5)
_RED = (1.0, 0.24, 0.18)


def _seven_seg(digit: int, gx, gy, w, hgt, t):
    """Distance d'un chiffre à sept segments (origine en bas à gauche)."""
    segs = {
        "a": ((t, hgt - t / 2), (w - t, hgt - t / 2)),
        "b": ((w - t / 2, hgt - t), (w - t / 2, hgt / 2 + t / 2)),
        "c": ((w - t / 2, hgt / 2 - t / 2), (w - t / 2, t)),
        "d": ((t, t / 2), (w - t, t / 2)),
        "e": ((t / 2, hgt / 2 - t / 2), (t / 2, t)),
        "f": ((t / 2, hgt - t), (t / 2, hgt / 2 + t / 2)),
        "g": ((t, hgt / 2), (w - t, hgt / 2)),
    }
    on = ["abcdef", "bc", "abged", "abgcd", "fgbc", "afgcd", "afgedc", "abc", "abcdefg", "abcdfg"][digit]
    lit = np.full(np.shape(gx), 1e3, dtype=np.float32)
    off = np.full(np.shape(gx), 1e3, dtype=np.float32)
    for k, (a, b) in segs.items():
        dd = sd_segment(gx, gy, a[0], a[1], b[0], b[1]) - t * 0.38
        if k in on:
            lit = np.minimum(lit, dd)
        else:
            off = np.minimum(off, dd)
    return lit, off


def screen(res: int = 1024, tile_m: float = 1.0):
    cv = Canvas(res, tile_m, seed=1212)
    rng = cv.rng(1)
    cell = cv.tile / 4.0
    emis = cv.zeros(3)
    content = cv.zeros()
    h = cv.zeros()
    ix = np.floor(cv.x / cell).astype(np.int32)
    iy = np.floor(cv.y / cell).astype(np.int32)
    lx = cv.x - ix * cell
    ly = cv.y - iy * cell
    bezel_w = 0.014
    inner = -sd_box(lx - cell / 2, ly - cell / 2, cell / 2 - bezel_w, cell / 2 - bezel_w, 0.006)
    glass = smoothstep(-0.001, 0.001, inner)
    h += 0.002 * (1.0 - smoothstep(-0.003, 0.0, inner)) - 0.0004 * glass
    kinds = ["digits", "bars", "scope", "text", "grid", "status", "digits", "bars", "text", "scope", "status", "grid", "bars", "digits", "text", "scope"]
    colors = [_AMBER, _CYAN, _GREEN, _CYAN, _CYAN, _GREEN, _CYAN, _AMBER, _AMBER, _CYAN, _AMBER, _GREEN, _GREEN, _RED, _GREEN, _AMBER]
    for j in range(4):
        for i in range(4):
            k = j * 4 + i
            kind = kinds[k]
            colr = np.asarray(colors[k], dtype=np.float32)
            x0, y0 = i * cell + bezel_w + 0.01, j * cell + bezel_w + 0.01
            w = cell - 2 * (bezel_w + 0.01)
            mask = cv.zeros()
            dim = cv.zeros()
            sel = (ix == i) & (iy == j)
            gx = cv.x - x0
            gy = cv.y - y0
            if kind == "digits":
                dw, dh, t = 0.034, 0.06, 0.008
                for row in range(2):
                    for col in range(4):
                        dg = int(rng.integers(0, 10))
                        lit, off = _seven_seg(dg, gx - 0.012 - col * (dw + 0.012), gy - 0.025 - row * (dh + 0.03), dw, dh, t)
                        mask = np.maximum(mask, fill(lit, cv.px))
                        dim = np.maximum(dim, fill(off, cv.px) * 0.12)
                ln = rng.uniform(0.06, 0.15)
                bd = sd_box(gx - 0.012 - ln / 2, gy - 0.01, ln / 2, 0.002)
                mask = np.maximum(mask, fill(bd, cv.px) * 0.7)
            elif kind == "bars":
                nb = 9
                for b in range(nb):
                    val = rng.uniform(0.15, 0.95)
                    bx = 0.012 + b * (w - 0.02) / nb
                    bw = (w - 0.02) / nb * 0.62
                    seg_h = 0.009
                    nseg = int(val * 16)
                    for s in range(16):
                        sd = sd_box(gx - bx - bw / 2, gy - 0.02 - s * (seg_h + 0.003) - seg_h / 2, bw / 2, seg_h / 2)
                        if s < nseg:
                            mask = np.maximum(mask, fill(sd, cv.px) * (1.0 if s < 12 else 0.85))
                        else:
                            dim = np.maximum(dim, fill(sd, cv.px) * 0.1)
            elif kind == "scope":
                for gl in range(1, 6):
                    gd = np.minimum(np.abs(gx - gl * w / 6), np.abs(gy - gl * w / 6))
                    dim = np.maximum(dim, fill(gd - 0.0005, cv.px) * 0.25)
                phase = rng.uniform(0, 6.28)
                yy = w / 2 + 0.05 * np.sin(gx / w * 2 * np.pi * 2.0 + phase) * np.exp(-((gx / w - 0.5) ** 2) * 3) + 0.012 * np.sin(gx / w * 2 * np.pi * 11 + phase)
                mask = np.maximum(mask, fill(np.abs(gy - yy) - 0.0016, cv.px))
            elif kind == "text":
                for row in range(13):
                    xpos = 0.008
                    while xpos < w - 0.02:
                        ln = rng.uniform(0.006, 0.035)
                        if xpos + ln > w - 0.008:
                            break
                        td = sd_box(gx - xpos - ln / 2, gy - w + 0.02 + row * 0.0145, ln / 2, 0.0035)
                        mask = np.maximum(mask, fill(td, cv.px) * (1.0 if row != 0 else 0.6))
                        xpos += ln + rng.uniform(0.004, 0.012)
                        if rng.random() < 0.15:
                            break
            elif kind == "grid":
                for gl in range(0, 9):
                    gd = np.minimum(np.abs(gx - gl * w / 8), np.abs(gy - gl * w / 8))
                    dim = np.maximum(dim, fill(gd - 0.0004, cv.px) * 0.3)
                ring = np.abs(np.hypot(gx - w / 2, gy - w / 2) - w * 0.32)
                dim = np.maximum(dim, fill(ring - 0.0006, cv.px) * 0.5)
                for _ in range(7):
                    bx, by = rng.uniform(0.02, w - 0.02, 2)
                    bd = sd_box(gx - bx, gy - by, 0.004, 0.004, 0.001)
                    mask = np.maximum(mask, fill(bd, cv.px))
                ang = np.arctan2(gy - w / 2, gx - w / 2)
                sweep = np.clip(1 - ((ang - 0.8) % (2 * np.pi)) / 1.2, 0, 1) * (np.hypot(gx - w / 2, gy - w / 2) < w * 0.45)
                dim = np.maximum(dim, sweep * 0.35)
            elif kind == "status":
                for r_ in range(6):
                    for c_ in range(4):
                        state = rng.random()
                        sd = sd_box(gx - 0.012 - c_ * 0.048 - 0.018, gy - 0.016 - r_ * 0.034 - 0.011, 0.018, 0.011, 0.002)
                        mval = fill(sd, cv.px)
                        if state < 0.55:
                            mask = np.maximum(mask, mval)
                        elif state < 0.7:
                            emis += (mval * sel)[..., None] * np.asarray(_RED, dtype=np.float32) * 0.9
                        else:
                            dim = np.maximum(dim, mval * 0.12)
            area = sel * glass
            emis += ((mask + dim) * area)[..., None] * colr
            content = np.maximum(content, (mask + dim * 0.5) * area)
            # Halo de rétroéclairage de la dalle.
            emis += (area * 0.035)[..., None] * colr
    scan = 0.9 + 0.1 * np.cos(cv.y / (2 * cv.py) * np.pi)
    emis *= scan[..., None]
    smudge = cut(cv.noise(2, 0.15, 0.01) + 0.5 * cv.noise(3, 0.05, 0.004), 0.25, 0.4)
    screws = cv.zeros()
    for j in range(4):
        for i in range(4):
            for (sx, sy) in ((0.007, 0.007), (cell - 0.007, 0.007), (0.007, cell - 0.007), (cell - 0.007, cell - 0.007)):
                cv.stamp(screws, i * cell + sx, j * cell + sy, 0.006, 0.006, lambda gx, gy: screw(gx, gy, 0.0035, 0.8, 0.0006))
    h += screws
    bezel = 1.0 - glass
    n_mid = cv.noise(4, 0.1, 0.005)
    col = lerp(rgb((0.018, 0.024, 0.026)), rgb((0.12, 0.12, 0.125)) * (1 + 0.1 * n_mid[..., None]), bezel)
    col = col + emis * 0.25 * glass[..., None]
    col = lerp(col, rgb((0.2, 0.2, 0.2)), smudge * glass * 0.06)
    rough = np.where(glass > 0.5, 0.3 + 0.05 * smudge, 0.5 + 0.05 * n_mid)  # vitre un peu mate : pas de taches de reflets
    metal = bezel * 0.3
    return finish(cv, h, col, rough, metal, [(0.004, 0.0008, 0.4)], emissive=emis, normal_strength=1.0)


def buttons(res: int = 1024, tile_m: float = 0.5):
    cv = Canvas(res, tile_m, seed=1313)
    rng = cv.rng(1)
    mod = cv.tile / 4.0  # modules de 12,5 cm
    h = cv.zeros()
    emis = cv.zeros(3)
    col = cv.full((0.13, 0.135, 0.14))
    rough = cv.full(0.5)
    metal = cv.full(0.1)
    wear = cv.zeros()
    # Rainures entre modules et vis.
    ix = np.floor(cv.x / mod)
    iy = np.floor(cv.y / mod)
    lx = cv.x - ix * mod - mod / 2
    ly = cv.y - iy * mod - mod / 2
    md = -sd_box(lx, ly, mod / 2, mod / 2, 0.004)
    h += seam_profile(md, 0.001, 0.0015, 0.0012)
    plan = ["keypad", "rockers", "leds", "bigkeys",
            "dials", "toggles", "keypad", "label",
            "bigkeys", "leds", "rockers", "dials",
            "label", "keypad", "toggles", "bigkeys"]
    lit_colors = [_AMBER, _GREEN, _RED, (0.75, 0.88, 1.0)]

    def put(cx, cy, rx, ry, fn, target, op="max"):
        cv.stamp(target, cx, cy, rx, ry, fn, op)

    def key(cx, cy, hw, hh, cap, lit=None, glow=1.0, r=0.0015):
        # Touche carrée : relief, couleur de capuchon, émissif si allumée.
        put(cx, cy, hw + 0.003, hh + 0.003, lambda gx, gy: 0.0018 * smoothstep(0.0006, -0.0008, sd_box(gx, gy, hw, hh, r)) - 0.0006 * fill(np.abs(sd_box(gx, gy, hw + 0.0012, hh + 0.0012, r)) - 0.0006, cv.px), h, "add")
        m = cv.zeros()
        put(cx, cy, hw + 0.002, hh + 0.002, lambda gx, gy: fill(sd_box(gx, gy, hw, hh, r), cv.px), m)
        c = np.asarray(cap, dtype=np.float32)
        col[:] = lerp(col, rgb(c), m)
        rough[:] = lerp(rough, 0.42, m)
        if lit is not None:
            inner = cv.zeros()
            put(cx, cy, hw, hh, lambda gx, gy: fill(sd_box(gx, gy, hw * 0.7, hh * 0.7, r), cv.px * 2) * glow, inner)
            emis[:] = emis + inner[..., None] * np.asarray(lit, dtype=np.float32)
            col[:] = lerp(col, rgb(lit) * 0.7, inner * 0.8)
        wear[:] = np.maximum(wear, m * rng.uniform(0.2, 1.0))

    def led(cx, cy, r, colr, on):
        put(cx, cy, r * 2, r * 2, lambda gx, gy: 0.0008 * np.sqrt(np.clip(1 - (np.hypot(gx, gy) / r) ** 2, 0, 1)), h, "add")
        m = cv.zeros()
        put(cx, cy, r * 2, r * 2, lambda gx, gy: fill(sd_circle(gx, gy, r), cv.px), m)
        ring = cv.zeros()
        put(cx, cy, r * 2, r * 2, lambda gx, gy: fill(np.abs(sd_circle(gx, gy, r * 1.35)) - r * 0.18, cv.px), ring)
        col[:] = lerp(col, rgb((0.3, 0.3, 0.3)), ring)
        c = np.asarray(colr, dtype=np.float32)
        col[:] = lerp(col, rgb(c) * (0.85 if on else 0.18), m)
        rough[:] = lerp(rough, 0.2, m)
        if on:
            halo = cv.zeros()
            put(cx, cy, r * 3, r * 3, lambda gx, gy: np.exp(-(np.hypot(gx, gy) / (r * 0.9)) ** 2), halo)
            emis[:] = emis + (np.maximum(m, halo * 0.5))[..., None] * c

    for k, kind in enumerate(plan):
        i, j = k % 4, k // 4
        cx0, cy0 = i * mod + mod / 2, j * mod + mod / 2
        if kind == "keypad":
            for a in range(3):
                for b in range(3):
                    lit = lit_colors[int(rng.integers(0, 4))] if rng.random() < 0.3 else None
                    key(cx0 + (a - 1) * 0.03, cy0 + (b - 1) * 0.03 - 0.008, 0.011, 0.011, (0.2, 0.2, 0.21), lit, 0.8)
            led(cx0 - 0.03, cy0 + 0.047, 0.0028, _GREEN, True)
            led(cx0 - 0.018, cy0 + 0.047, 0.0028, _RED, rng.random() < 0.5)
        elif kind == "rockers":
            for a in range(4):
                up = rng.random() < 0.5
                x = cx0 + (a - 1.5) * 0.026
                key(x, cy0 - 0.005, 0.008, 0.02, (0.08, 0.08, 0.085), None)
                put(x, cy0 - 0.005 + (0.01 if up else -0.01), 0.008, 0.01, lambda gx, gy: 0.0012 * fill(sd_box(gx, gy, 0.006, 0.008, 0.002), cv.px), h, "add")
                led(x, cy0 + 0.035, 0.0026, _AMBER if up else _GREEN, up)
                lab = cv.zeros()
                put(x, cy0 - 0.042, 0.01, 0.003, lambda gx, gy: fill(sd_box(gx, gy, 0.008, 0.0016), cv.px), lab)
                col[:] = lerp(col, rgb((0.7, 0.7, 0.66)), lab * 0.8)
        elif kind == "leds":
            for a in range(2):
                for b in range(6):
                    state = rng.random()
                    c = _GREEN if state < 0.5 else (_RED if state < 0.7 else _AMBER)
                    led(cx0 - 0.02 + a * 0.04, cy0 - 0.045 + b * 0.018, 0.0032, c, rng.random() < 0.7)
                    lab = cv.zeros()
                    put(cx0 - 0.004 + a * 0.04, cy0 - 0.045 + b * 0.018, 0.012, 0.003, lambda gx, gy: fill(sd_box(gx, gy, 0.009, 0.0015), cv.px), lab)
                    col[:] = lerp(col, rgb((0.65, 0.65, 0.6)), lab * 0.7)
        elif kind == "bigkeys":
            for a in range(2):
                for b in range(2):
                    c = lit_colors[(a + 2 * b + k) % 4]
                    key(cx0 + (a - 0.5) * 0.05, cy0 + (b - 0.5) * 0.05, 0.02, 0.02, (0.25, 0.25, 0.25), c if rng.random() < 0.75 else None, 0.9, 0.003)
        elif kind == "dials":
            for a in range(2):
                x = cx0 + (a - 0.5) * 0.055
                put(x, cy0, 0.03, 0.03, lambda gx, gy: 0.004 * smoothstep(0.0008, -0.0008, sd_circle(gx, gy, 0.017)) + 0.0006 * np.cos(np.arctan2(gy, gx) * 24) * (np.hypot(gx, gy) > 0.014) * (np.hypot(gx, gy) < 0.017), h, "add")
                m = cv.zeros()
                put(x, cy0, 0.02, 0.02, lambda gx, gy: fill(sd_circle(gx, gy, 0.017), cv.px), m)
                col[:] = lerp(col, rgb((0.07, 0.07, 0.07)), m)
                ang = rng.uniform(0, 6.28)
                put(x, cy0, 0.02, 0.02, lambda gx, gy, ang=ang: fill(sd_segment(gx, gy, 0, 0, 0.013 * math.cos(ang), 0.013 * math.sin(ang)) - 0.0012, cv.px), m)
                tick = cv.zeros()
                for t_ in range(9):
                    ta = math.radians(-135 + t_ * 33.75) + math.pi / 2
                    put(x + 0.023 * math.cos(ta), cy0 + 0.023 * math.sin(ta), 0.004, 0.004, lambda gx, gy: fill(sd_circle(gx, gy, 0.0012), cv.px), tick)
                col[:] = lerp(col, rgb((0.75, 0.75, 0.7)), tick * 0.8)
                wear[:] = np.maximum(wear, m * 0.8)
            led(cx0, cy0 + 0.045, 0.003, _AMBER, True)
        elif kind == "toggles":
            for a in range(3):
                x = cx0 + (a - 1) * 0.035
                put(x, cy0, 0.02, 0.03, lambda gx, gy: 0.002 * fill(sd_box(gx, gy, 0.012, 0.022, 0.003), cv.px) - 0.0016 * fill(sd_box(gx, gy, 0.0085, 0.0185, 0.002), cv.px), h, "add")
                guard = cv.zeros()
                put(x, cy0, 0.02, 0.03, lambda gx, gy: fill(np.abs(sd_box(gx, gy, 0.0105, 0.0205, 0.003)) - 0.0015, cv.px), guard)
                col[:] = lerp(col, rgb((0.55, 0.12, 0.08)) if a == 1 else rgb((0.3, 0.3, 0.3)), guard)
                put(x, cy0 + 0.004, 0.005, 0.012, lambda gx, gy: 0.003 * fill(sd_box(gx, gy, 0.0022, 0.009, 0.0015), cv.px), h, "add")
                lev = cv.zeros()
                put(x, cy0 + 0.004, 0.005, 0.012, lambda gx, gy: fill(sd_box(gx, gy, 0.0022, 0.009, 0.0015), cv.px), lev)
                col[:] = lerp(col, rgb((0.65, 0.65, 0.63)), lev)
                metal[:] = lerp(metal, 0.5, lev)
                led(x, cy0 - 0.035, 0.0028, _RED if a == 1 else _GREEN, rng.random() < 0.6)
        elif kind == "label":
            lab = cv.zeros()
            put(cx0, cy0, 0.055, 0.05, lambda gx, gy: fill(sd_box(gx, gy, 0.05, 0.045, 0.003), cv.px), lab)
            col[:] = lerp(col, rgb((0.55, 0.54, 0.5)), lab)
            h += 0.0005 * lab
            dash = cv.zeros()
            for r_ in range(6):
                xp = cx0 - 0.042
                while xp < cx0 + 0.035:
                    ln = rng.uniform(0.006, 0.022)
                    put(xp + ln / 2, cy0 + 0.032 - r_ * 0.013, ln / 2 + 0.002, 0.003, lambda gx, gy, ln=ln: fill(sd_box(gx, gy, ln / 2, 0.0022), cv.px), dash)
                    xp += ln + 0.005
            col[:] = lerp(col, rgb((0.1, 0.1, 0.1)), dash * lab)
            for (sx, sy) in ((-0.045, -0.04), (0.045, -0.04), (-0.045, 0.04), (0.045, 0.04)):
                cv.stamp(h, cx0 + sx, cy0 + sy, 0.004, 0.004, lambda gx, gy: screw(gx, gy, 0.0025, 0.5, 0.0005), "add")
    # Usure : peinture frottée autour des touches les plus utilisées, crasse dans les creux.
    n_mid = cv.noise(2, 0.05, 0.003)
    finger = np.clip(cv.blur(wear, 0.006) * 1.5, 0, 1) * np.clip(0.6 + 0.6 * n_mid, 0, 1)
    col = lerp(col, col * 1.35 + 0.03, finger * 0.35)
    scr = cv.scratches(3, 120, (0.005, 0.03), 0.0004)
    col = lerp(col, rgb((0.4, 0.4, 0.4)), scr * 0.4)
    rough = rough + 0.05 * n_mid - 0.1 * finger
    return finish(cv, h, col, rough, metal, [(0.002, 0.0006, 0.4), (0.006, 0.001, 0.3)], emissive=emis, edge_light=0.06)


def light_band(res: int = 512, tile_m: float = 1.0):
    cv = Canvas(res, tile_m, seed=1414)
    v = cv.y / cv.tile_h  # 0 à 1 en travers de la bande
    lens = smoothstep(0.1, 0.16, v) * smoothstep(0.9, 0.84, v)
    ribs = 0.5 + 0.5 * np.cos(v * 2 * np.pi * 14)
    joint_sp = 0.5
    jx = np.abs(((cv.x % joint_sp) - joint_sp / 2))
    joint = smoothstep(joint_sp / 2 - 0.012, joint_sp / 2 - 0.008, jx)
    h = 0.002 * (1 - lens) + 0.0003 * ribs * lens - 0.001 * joint * lens
    screws = cv.zeros()
    for xs in (0.0, 0.5):
        for vs in (0.06, 0.94):
            cv.stamp(screws, xs, vs * cv.tile_h, 0.01, 0.01, lambda gx, gy: screw(gx, gy, 0.006, 0.3, 0.0008))
    h += screws
    n_big = cv.noise(1, 0.4, 0.04)
    spots = cut(cv.noise(2, 0.02, 0.004), 0.04, 0.2) * lens
    falloff = smoothstep(0.0, 0.08, joint_sp / 2 - jx)
    glow = lens * (1 - joint) * (0.86 + 0.1 * ribs) * (0.85 + 0.15 * falloff) * (1 - 0.35 * spots) * (1 + 0.04 * n_big)
    emis = np.repeat(np.clip(glow, 0, 1)[..., None], 3, axis=-1)
    housing = rgb((0.14, 0.14, 0.145))
    col = lerp(housing, rgb((0.88, 0.88, 0.86)), lens * (1 - joint))
    col = lerp(col, rgb((0.35, 0.33, 0.3)), spots * 0.5)
    rough = np.where(lens > 0.5, 0.3, 0.5)
    metal = (1 - lens) * 0.4
    return finish(cv, h, col, rough, metal, [(0.004, 0.0008, 0.3)], emissive=emis)


