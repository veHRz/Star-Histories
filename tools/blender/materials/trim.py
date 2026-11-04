"""Trim sheet `trim_ship` : bandes horizontales qui répètent le long de U (période 2 m).

La table des bandes (nom → (v0, v1) en UV de Blender) est `library.TRIMS` ; chaque bande a un
liseré en creux haut et bas pour que le mip-mapping ne mélange pas deux bandes voisines.
Pochoirs génériques uniquement (chiffres, flèches, chevrons, « ! ») : aucun mot, aucun nom.
"""

from __future__ import annotations

import numpy as np

from texgen import (
    DUST, GRIME, METAL_MAX, PRIMER, STEEL, Canvas, chip_field, cut, drips, even_spacing, fill, finish,
    lerp, rgb, sd_box, sd_circle, sd_poly, smoothstep, text_sd, text_width, tone,
)

ANTHRACITE = (0.19, 0.195, 0.205)


def trim_ship(res: int, tile_m: float, trims: dict):
    cv = Canvas(res, tile_m, seed=1515)
    T = cv.tile
    h = cv.zeros()
    col = cv.full((0.5, 0.5, 0.5))
    rough = cv.full(0.55)
    metal = cv.full(0.0)
    emis = cv.zeros(3)
    ao_extra = cv.full(1.0)
    n_mid = cv.noise(2, 0.12, 0.006)
    scr_all = cv.scratches(4, 380, (0.01, 0.12), 0.0009, curve=0.2)

    def band(name):
        v0, v1 = trims[name]
        y0, y1 = v0 * cv.tile_h, v1 * cv.tile_h
        m = ((cv.y >= y0) & (cv.y < y1)).astype(np.float32)
        return m, cv.y - y0, y1 - y0

    def rims(m, yl, bh, w=0.004, depth=0.0015):
        """Liseré en creux haut et bas d'une bande."""
        e = np.clip((1.0 - smoothstep(0.0, w, yl)) + (1.0 - smoothstep(0.0, w, bh - yl)), 0, 1) * m
        return -depth * e, e

    def worn(field, frac, m, salt):
        """Écaillures limitées à la bande (≈ frac de sa surface)."""
        f = chip_field(cv, salt, field, 0.008) * m - (1 - m) * 100
        return cut(f, frac * float(m.mean()), 0.08) * m

    def paint(m, c, r, mt):
        nonlocal col, rough, metal
        col = lerp(col, c, m)
        rough = lerp(rough, r, m)
        metal = lerp(metal, mt, m)

    # 1. Plat rivé anthracite (sert aussi de plinthe : éraflures et crasse plus fortes en bas).
    m, yl, bh = band("rivets")
    sp = T / 16
    dx = (cv.x % sp) - sp / 2
    d = np.hypot(dx, yl - bh * 0.55)
    riv = 0.0028 * np.sqrt(np.clip(1 - (d / 0.012) ** 2, 0, 1))
    eh, em = rims(m, yl, bh)
    h += m * (riv + 0.0002 * n_mid) + eh
    low = (1.0 - smoothstep(0.0, bh * 0.5, yl)) * m
    w = worn(em + 0.5 * low, 0.05, m, 10)
    head = smoothstep(0.0012, 0.0024, riv) * m  # sommet des têtes, poli par les frottements
    streak = drips(cv, (riv > 0.0015) * m, 0.04, 11) * m
    scuffs = cv.scratches(12, 50, (0.02, 0.12), 0.0012, angle=(0.0, 0.3)) * low
    c = rgb(ANTHRACITE) * tone(cv, 13, 0.6, 0.05)[..., None]
    c = lerp(c, rgb(STEEL) * 0.85, np.clip(w + scuffs * 0.6 + head * 0.55, 0, 1))
    c = lerp(c, rgb(GRIME), np.clip(streak * 0.45 + low * 0.2, 0, 1))
    paint(m, c, 0.5 - 0.2 * w - 0.15 * head + 0.1 * low, 0.12 + 0.4 * np.clip(w + head, 0, 1))

    # 2. Chanfrein de bord de panneau anthracite, arête écaillée.
    m, yl, bh = band("edge")
    t = yl / bh
    prof = np.clip(t / 0.35, 0, 1)
    eh, em = rims(m, yl, bh)
    h += m * 0.006 * prof + eh
    crest = np.clip(1 - np.abs(t - 0.36) / 0.07, 0, 1) * m
    w = worn(crest + 0.5 * em, 0.16, m, 20)
    c = rgb(ANTHRACITE) * tone(cv, 21, 0.6, 0.05)[..., None]
    c = lerp(c, rgb(STEEL), w)
    c = lerp(c, rgb(GRIME), (1 - smoothstep(0.0, 0.3, t)) * 0.45)
    paint(m, c, 0.52 - 0.2 * w, 0.12 + 0.4 * w)

    # 3. Bandes de danger jaune / noir à 45°, usées.
    m, yl, bh = band("hazard")
    period = T / 16
    stripe = ((cv.x + yl) % period) < period / 2
    eh, em = rims(m, yl, bh, 0.006, 0.002)
    h += eh + 0.0002 * n_mid * m
    w = worn(em + 0.3 * (1 - smoothstep(0, 0.05, yl)), 0.1, m, 30)
    primer_part = w * (cv.noise(31, 0.02, 0.003) > 0)
    yellow = rgb((0.86, 0.66, 0.1)) * tone(cv, 32, 0.4, 0.07)[..., None]
    black = rgb((0.07, 0.07, 0.07)) * tone(cv, 33, 0.4, 0.15)[..., None]
    c = np.where(stripe[..., None], black, yellow)
    c = lerp(c, rgb(PRIMER), primer_part)
    c = lerp(c, rgb(STEEL) * 0.85, w * (1 - primer_part))
    c = lerp(c, rgb(GRIME), np.clip(0.18 + 0.2 * cv.noise(34, 0.3, 0.02) + 0.3 * scr_all, 0, 1) * 0.5)
    c = lerp(c, rgb((0.18, 0.18, 0.18)), em)
    paint(m, c, 0.6 + 0.05 * n_mid - 0.2 * w, 0.45 * w * (1 - primer_part))

    # 4. Chevrons clairs sur fond sombre, pointant vers +U.
    m, yl, bh = band("chevrons")
    period = T / 8
    s = (cv.x % period) + 0.9 * np.abs(yl - bh / 2)
    chev = smoothstep(0.06, 0.063, s) * smoothstep(0.125, 0.122, s) * (np.abs(yl - bh / 2) < bh * 0.36)
    eh, em = rims(m, yl, bh)
    h += eh + 0.0002 * chev * m
    w = worn(em, 0.08, m, 40)
    c = rgb((0.15, 0.155, 0.16)) * tone(cv, 41, 0.6, 0.06)[..., None]
    c = lerp(c, rgb((0.85, 0.84, 0.78)), chev * (1 - w))
    c = lerp(c, rgb(GRIME), np.clip(0.25 + 0.25 * cv.noise(42, 0.3, 0.02), 0, 1) * 0.35)
    paint(m, c, 0.55 - 0.05 * chev, 0.1)

    # 5. Grille d'aération à fentes, crasse sous les fentes.
    m, yl, bh = band("vent")
    slot_len = 0.1
    per = even_spacing(T, 0.128)
    sx = (cv.x % per) - per / 2
    slots = cv.zeros()
    for r_ in range(4):
        cy = 0.025 + r_ * 0.0267
        slots = np.maximum(slots, fill(sd_box(sx, yl - cy, slot_len / 2, 0.0065, 0.006), cv.px))
    slots *= m
    eh, em = rims(m, yl, bh, 0.008, 0.003)
    h += eh - 0.015 * slots
    under = drips(cv, slots, 0.03, 50) * (1 - slots)
    w = worn(em + 0.5 * cv.blur(slots, 0.003) * 3, 0.05, m, 51)
    c = rgb((0.3, 0.305, 0.31)) * tone(cv, 52, 0.6, 0.06)[..., None]
    c = lerp(c, rgb(GRIME), np.clip(under * 0.7, 0, 1))
    c = lerp(c, rgb((0.02, 0.02, 0.022)), slots)
    c = lerp(c, rgb(STEEL), w)
    paint(m, c, 0.55 + 0.2 * under + 0.3 * slots, 0.25 * (1 - slots))
    ao_extra = ao_extra * (1.0 - 0.85 * slots)

    # 6. Faisceau de câbles gainés, colliers tous les 50 cm.
    m, yl, bh = band("cables")
    cables = [  # (centre en travers, rayon, couleur)
        (0.022, 0.017, (0.12, 0.12, 0.125)), (0.058, 0.019, (0.16, 0.16, 0.16)), (0.094, 0.016, (0.55, 0.16, 0.1)),
        (0.126, 0.015, (0.13, 0.13, 0.14)), (0.158, 0.017, (0.6, 0.5, 0.16)), (0.192, 0.016, (0.17, 0.27, 0.42)),
        (0.226, 0.016, (0.14, 0.14, 0.14)),
    ]
    ch_h = cv.zeros()
    ch_c = cv.zeros(3)
    ch_p = cv.zeros()
    for ci, (cyc, r, cc) in enumerate(cables):
        wob = 0.0015 * np.sin(cv.x / T * 2 * np.pi * (2 + ci) + ci)
        dy = yl - cyc - wob
        prof = np.sqrt(np.clip(1 - (dy / r) ** 2, 0, 1))
        hc = r * prof
        sel = (hc > ch_h) & (m > 0)
        ch_h = np.where(sel, hc, ch_h)
        ch_p = np.where(sel, prof, ch_p)
        # Ombrage cylindrique et reflet doux : le câble se lit rond même sous la lightmap.
        shade = (0.45 + 0.55 * prof ** 0.6) * (1.0 + 0.25 * np.exp(-((dy / r - 0.35) / 0.25) ** 2))
        ch_c = np.where(sel[..., None], rgb(cc) * shade[..., None], ch_c)
    tie_per = T / 4
    tie_d = np.abs((cv.x % tie_per) - tie_per / 2)
    tie = (tie_d < 0.008) & (yl > 0.003) & (yl < bh - 0.003)
    ch_h = np.where(tie, np.maximum(ch_h, 0.02 + 0.004 * (tie_d < 0.004)), ch_h)
    ch_c = np.where(tie[..., None], rgb((0.2, 0.2, 0.2)) * (0.8 + 0.3 * (tie_d < 0.004))[..., None], ch_c)
    dust = np.clip(0.35 + 0.4 * n_mid, 0, 1) * 0.2 * ch_p
    c = lerp(ch_c, rgb(DUST), dust)
    h += m * (ch_h * 0.3)
    paint(m, c, 0.62 - 0.15 * ch_p + 0.1 * dust, 0.0)

    # 7. Tuyau brut, brides boulonnées tous les 1 m, colliers jaunes.
    m, yl, bh = band("flange")
    fper = T / 2
    fx = np.abs((cv.x % fper) - fper / 2)
    flange = smoothstep(0.032, 0.028, fx)
    clamp = smoothstep(0.012, 0.009, np.abs(((cv.x + 0.25) % fper) - fper / 2))
    bolts = cv.zeros()
    for b in range(4):
        dd = np.hypot(fx, yl - (b + 0.5) * bh / 4)
        bolts = np.maximum(bolts, 0.003 * np.sqrt(np.clip(1 - (dd / 0.009) ** 2, 0, 1)))
    brushed = cv.noise(60, 0.4, 0.002, beta=0.6, stretch=(25, 1))
    h += m * (0.006 * flange + 0.003 * clamp + bolts * flange + 0.00002 * brushed)
    c = rgb((0.62, 0.62, 0.61)) * (1 + 0.05 * brushed[..., None]) * tone(cv, 61, 0.5, 0.05)[..., None]
    c = lerp(c, rgb((0.22, 0.22, 0.23)), flange * 0.8)
    c = lerp(c, rgb((0.6, 0.5, 0.14)), clamp * 0.75)
    c = lerp(c, rgb(GRIME), np.clip(cv.blur(flange, 0.012) * (1 - flange) * 1.6, 0, 1) * 0.55)
    c = lerp(c, rgb((0.75, 0.75, 0.73)), (bolts > 0.001) * flange * 0.5)
    paint(m, c, 0.38 + 0.15 * flange + 0.04 * n_mid, METAL_MAX * (1 - 0.6 * flange) + 0.2 * (bolts > 0.001) * flange)

    # 8. Joint caoutchouc à trois lèvres.
    m, yl, bh = band("gasket")
    t = yl / bh
    lips = 0.5 + 0.5 * np.cos(t * 2 * np.pi * 3)
    env = smoothstep(0.0, 0.1, t) * smoothstep(1.0, 0.9, t)
    h += m * 0.004 * lips * env
    c = rgb((0.07, 0.07, 0.075)) * (0.6 + 0.6 * lips * env)[..., None] * tone(cv, 70, 0.1, 0.1)[..., None]
    c = lerp(c, rgb(DUST), np.clip((1 - lips) * 0.3 * (0.5 + 0.5 * cv.noise(71, 0.4, 0.04)), 0, 1))
    paint(m, c, 0.82 - 0.15 * lips, 0.0)

    # 9. Bandeau lumineux segmenté (seule bande émissive).
    m, yl, bh = band("light_strip")
    t = yl / bh
    lens = smoothstep(0.18, 0.26, t) * smoothstep(0.82, 0.74, t)
    seg = T / 8
    to_joint = seg / 2 - np.abs((cv.x % seg) - seg / 2)
    gap = 1.0 - smoothstep(0.004, 0.007, to_joint)
    lit = lens * (1 - gap)
    ribs = 0.5 + 0.5 * np.cos(t * 2 * np.pi * 9)
    h += m * (0.003 * (1 - lens) + 0.0002 * ribs * lens)
    c = lerp(rgb((0.12, 0.12, 0.125)), rgb((0.9, 0.9, 0.88)), lit)
    paint(m, c, 0.3 + 0.2 * (1 - lens), 0.3 * (1 - lens))
    emis += (m * lit * (0.88 + 0.12 * ribs) * (0.85 + 0.15 * smoothstep(0.0, 0.05, to_joint)))[..., None]

    # 10. Plaque grise à pochoirs génériques.
    m, yl, bh = band("stencils")
    eh, em = rims(m, yl, bh, 0.005, 0.0015)
    h += eh + 0.0003 * n_mid * m
    sten = cv.zeros()
    items = ["07", "arrow", "A2", "warn", "12", "chev", "B4", "03"]
    slot = T / len(items)
    size = 0.075
    for i_, it in enumerate(items):
        gx = cv.wrap_dx(cv.x, (i_ + 0.5) * slot)
        gy = yl - bh / 2
        if it == "arrow":
            sd = np.minimum(sd_box(gx + 0.02, gy, 0.045, 0.012), sd_poly(gx, gy, [(0.02, -0.035), (0.075, 0.0), (0.02, 0.035)]))
        elif it == "warn":
            tri = sd_poly(gx, gy + 0.005, [(-0.045, -0.035), (0.045, -0.035), (0.0, 0.045)])
            ring = np.abs(tri + 0.006) - 0.006
            bang = np.minimum(sd_box(gx, gy + 0.008, 0.004, 0.016, 0.002), sd_circle(gx, gy + 0.005 + 0.027 - 0.06, 0.0045))
            sd = np.minimum(ring, bang)
        elif it == "chev":
            s2 = gx + 0.9 * np.abs(gy)
            sd = np.maximum(np.maximum(-s2, s2 - 0.03), np.abs(gy) - 0.035)
            s3 = gx - 0.04 + 0.9 * np.abs(gy)
            sd = np.minimum(sd, np.maximum(np.maximum(-s3, s3 - 0.03), np.abs(gy) - 0.035))
        else:
            tw = text_width(it, size)
            sd = text_sd(it, gx + tw / 2, gy + size / 2, size)
        sten = np.maximum(sten, fill(sd, cv.px))
    sten *= m
    fade = np.clip(0.75 + 0.3 * n_mid + 0.15 * cv.noise(80, 0.02, 0.002), 0, 1)
    w = worn(em, 0.05, m, 81)
    c = rgb((0.58, 0.58, 0.56)) * tone(cv, 82, 0.5, 0.05)[..., None]
    c = lerp(c, rgb((0.08, 0.08, 0.08)), sten * fade * (1 - w))
    c = lerp(c, rgb(GRIME), np.clip(0.2 + 0.25 * cv.noise(83, 0.3, 0.02), 0, 1) * 0.35)
    c = lerp(c, rgb(STEEL), w)
    c = lerp(c, rgb((0.2, 0.2, 0.2)), em)
    paint(m, c, 0.6 - 0.05 * sten, 0.45 * w)

    # 11. Bandes d'avertissement rouge / blanc.
    m, yl, bh = band("warning_red")
    period = T / 8
    stripe = ((cv.x - yl) % period) < period / 2
    eh, em = rims(m, yl, bh, 0.005, 0.0015)
    h += eh + 0.0002 * n_mid * m
    w = worn(em, 0.1, m, 90)
    c = np.where(stripe[..., None], rgb((0.62, 0.1, 0.07)), rgb((0.8, 0.79, 0.75))) * tone(cv, 91, 0.4, 0.06)[..., None]
    c = lerp(c, rgb(STEEL) * 0.85, w)
    c = lerp(c, rgb(GRIME), np.clip(0.2 + 0.25 * cv.noise(92, 0.3, 0.02) + 0.3 * scr_all, 0, 1) * 0.35)
    c = lerp(c, rgb((0.2, 0.2, 0.2)), em)
    paint(m, c, 0.6 - 0.2 * w, 0.45 * w)

    col = lerp(col, rgb((0.74, 0.73, 0.7)), scr_all * 0.2)
    return finish(cv, h, col, rough, metal, [(0.003, 0.001, 0.4), (0.012, 0.0025, 0.3)], ao_extra=ao_extra, emissive=emis, edge_light=0.1)
