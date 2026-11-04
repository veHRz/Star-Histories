"""Atlas de textures des personnages et de leurs armes (lot F de M8) : `char_gear`.

Pourquoi un atlas plutôt que la bibliothèque seule : les matériaux de `materials/library.py`
sont à l'échelle des décors (panneaux de 1 m, grilles, rivets de coque) ; posés sur une plaque
d'épaule de 20 cm, ils montrent des morceaux de motifs sans sens. Un abordeur pirate est un
assemblage de pièces dépareillées : une seule texture de 1K découpée en 16 cases de 256 px
(peintures différentes, rouille, ferraille soudée, cuivre de bobine, sangle, toile, grip,
émissifs) donne toutes ces pièces en **un seul matériau** (budget : ≤ 5 matériaux par modèle).

Chaque case est une plaque entière : ses bords sont les arêtes de la pièce (écaillures, rivets,
bord roulé). `char_kit.uv_cell` projette une pièce dans sa case, faces avant pleine case, flancs
en bande au bord (là où la peinture est usée).

Cartes écrites (contrat du jeu) dans assets/textures/materials/ :
`char_gear_color.png` (sRGB), `char_gear_normal.png` (OpenGL, linéaire), `char_gear_orm.png`
(R occlusion, G rugosité, B métal, linéaire), `char_gear_emissive.png` (sRGB).

Lancer (aperçu de l'atlas dans target/art-previews/lotF/) :

    blender --background --factory-startup --threads 4 --python tools/blender/characters/char_atlas.py
"""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "materials"))

import texgen as tg  # noqa: E402
from texgen import Canvas, cut, chip_field, smoothstep, stains, rgb  # noqa: E402

ROOT = HERE.parents[2]
OUT_DIR = ROOT / "assets" / "textures" / "materials"
PREVIEW_DIR = ROOT / "target" / "art-previews" / "lotF"
NAME = "char_gear"
RES = 1024
GRID = 4
CELL_PX = RES // GRID
CELL_M = 0.25  # taille « physique » d'une case : 1 px ≈ 1 mm

# Cases (colonne, ligne), ligne 0 en BAS de l'image (v = 0), comme les UV de Blender.
CELLS: dict[str, tuple[int, int]] = {
    "paint_red": (0, 0),      # rouge sang écaillé, rivets
    "paint_blood": (1, 0),    # rouge plus sombre, très usé, réparation soudée
    "paint_rust": (2, 0),     # peinture orangée rouille
    "paint_ochre": (3, 0),    # ocre terne, restes de bandes de danger
    "paint_olive": (0, 1),    # olive, pièce récupérée
    "primer": (1, 1),         # apprêt gris, pochoir générique
    "rusted": (2, 1),         # tôle rongée, restes de peinture
    "scrap": (3, 1),          # ferraille brute, cordons de soudure, teintes de chauffe
    "copper": (0, 2),         # enroulement de bobine (spires le long de U)
    "webbing": (1, 2),        # sangle tissée
    "canvas": (2, 2),         # toile de poche, rabat cousu, bouton-pression
    "grip": (3, 2),           # polymère quadrillé (poignées)
    "visor": (0, 3),          # visière : verre sombre, lueur orangée à lignes de balayage
    "leds": (1, 3),           # voyants vert / ambre / rouge
    "charge": (2, 3),         # jauge de charge segmentée, cyan
    "glow_cyan": (3, 3),      # lueur cyan (fenêtre de cellule, rails sous tension)
}


def cell_rect(name: str, pad_px: float = 4.0) -> tuple[float, float, float, float]:
    """(u0, v0, u1, v1) d'une case, rognée de `pad_px` texels (le filtrage ne déborde pas)."""
    col, row = CELLS[name]
    pad = pad_px / RES
    return (col / GRID + pad, row / GRID + pad, (col + 1) / GRID - pad, (row + 1) / GRID - pad)


def _finish(cv: Canvas, h, albedo, rough, metal, ao_layers, **kw):
    """`texgen.finish` avec rugosité et métal acceptés en scalaires."""
    shape = h.shape
    rough = np.broadcast_to(np.asarray(rough, np.float32), shape)
    metal = np.broadcast_to(np.asarray(metal, np.float32), shape)
    return tg.finish(cv, h, albedo, rough, metal, ao_layers, **kw)


# --- Briques -----------------------------------------------------------------------------------


def _rim(cv: Canvas) -> np.ndarray:
    """Distance (m) au bord de la case : les arêtes de la pièce."""
    return np.minimum(np.minimum(cv.x, CELL_M - cv.x), np.minimum(cv.y, CELL_M - cv.y))


def _dome(r: float, height: float):
    def fn(gx, gy):
        d = np.hypot(gx, gy)
        return height * np.sqrt(np.clip(1.0 - (d / r) ** 2, 0.0, 1.0))
    return fn


def _rivets_around(cv: Canvas, inset: float, spacing: float, r: float = 0.0045, height: float = 0.0016) -> np.ndarray:
    """Rivets le long du bord, à `inset` du bord."""
    out = cv.zeros()
    n = max(2, int(round((CELL_M - 2 * inset) / spacing)))
    ts = np.linspace(inset, CELL_M - inset, n + 1)
    pts = set()
    for t in ts:
        for p in ((t, inset), (t, CELL_M - inset), (inset, t), (CELL_M - inset, t)):
            pts.add((round(p[0], 5), round(p[1], 5)))
    for x, y in sorted(pts):
        cv.stamp(out, x, y, r * 1.5, r * 1.5, _dome(r, height))
    return out


def _weld(cv: Canvas, salt: int, path, width: float = 0.006) -> tuple[np.ndarray, np.ndarray]:
    """Cordon de soudure le long d'une polyligne : (hauteur, masque de chauffe autour)."""
    d = np.full((cv.res, cv.res), 1e3, dtype=np.float32)
    for (ax, ay), (bx, by) in zip(path[:-1], path[1:]):
        d = np.minimum(d, tg.sd_segment(cv.x, cv.y, ax, ay, bx, by))
    # Écailles du cordon : ondulation le long du trait, bord irrégulier.
    ripple = 0.5 + 0.5 * np.sin((cv.x + cv.y) / 0.0022 * np.pi)
    wob = 1.0 + 0.25 * cv.noise(salt, 0.03, 0.003)
    bead = np.clip(1.0 - (d / (width * wob)) ** 2, 0.0, 1.0)
    h = 0.0018 * np.sqrt(bead) * (0.8 + 0.2 * ripple)
    heat = np.clip(1.0 - d / (width * 4.0), 0.0, 1.0) ** 1.5
    return h, heat


def _paint(cv: Canvas, salt: int, color, wear: float = 1.0, rivets: bool = True, patch: bool = False,
           weld_path=None, stripes: bool = False, stencil: str | None = None, rust: float = 0.5):
    """Plaque peinte usée : bord roulé, rivets, écaillures (apprêt puis métal) concentrées sur
    les arêtes, rayures, crasse, coulures de rouille sous les rivets."""
    d = _rim(cv)
    h = -0.0018 * (1.0 - smoothstep(0.0, 0.012, d))
    h += 0.0004 * cv.noise(salt, 0.12, 0.008)
    riv = _rivets_around(cv, 0.016, 0.045) if rivets else cv.zeros()
    edge = 1.0 - smoothstep(0.003, 0.035, d)
    extra_edge = cv.zeros()
    heat = cv.zeros()
    if patch:
        # Rustine soudée dans un coin : petite plaque en relief, cordon tout autour.
        px0, py0, px1, py1 = 0.135, 0.03, 0.215, 0.10
        pd = tg.sd_box(cv.x - (px0 + px1) / 2, cv.y - (py0 + py1) / 2, (px1 - px0) / 2, (py1 - py0) / 2, 0.004)
        h += 0.0014 * tg.fill(pd, cv.px)
        wh, wheat = _weld(cv, salt + 40, [(px0, py0), (px1, py0), (px1, py1), (px0, py1), (px0, py0)], 0.0035)
        h += wh
        heat = np.maximum(heat, wheat)
        extra_edge = np.maximum(extra_edge, 0.8 * (1.0 - smoothstep(0.0, 0.01, np.abs(pd))))
    if weld_path is not None:
        wh, wheat = _weld(cv, salt + 41, weld_path)
        h += wh
        heat = np.maximum(heat, wheat)
        extra_edge = np.maximum(extra_edge, wheat)
    h += riv
    riv_mask = np.clip(cv.blur((riv > 0.0003).astype(np.float32), 0.003) * 2.5, 0.0, 1.0)
    edge = np.maximum(np.maximum(edge, 0.7 * riv_mask), extra_edge)
    chips = chip_field(cv, salt + 3, edge * wear, 0.006, edge_weight=3.0)
    primer = cut(chips, 0.12 * wear)
    bare = cut(chips, 0.055 * wear)
    h += -0.00025 * primer - 0.0001 * bare
    scr = cv.scratches(salt + 5, int(45 * wear), (0.01, 0.06), 0.0006, curve=0.04)
    grime = stains(cv, salt + 7, 0.12, 0.10, edge=0.6)
    dirt_edge = (1.0 - smoothstep(0.0, 0.02, d)) * 0.5
    rust_src = np.clip(riv_mask * 0.8 + bare * 0.7 + heat * 0.3, 0.0, 1.0) * cut(cv.noise(salt + 9, 0.05, 0.005), 0.6, 0.3)
    rust_run = tg.drips(cv, rust_src, 0.05, salt + 11) * rust

    paint = rgb(color) * tg.tone(cv, salt + 13, 0.1, 0.07)[..., None]
    if stripes:
        # Restes de bandes de danger à 45° (ocre et noir), largement effacées.
        band = (np.floor((cv.x + cv.y) / 0.035) % 2 == 0).astype(np.float32)
        fade = cut(cv.noise(salt + 15, 0.08, 0.004), 0.55, 0.25)
        paint = tg.lerp(paint, rgb((0.10, 0.09, 0.08)), band * fade * 0.85)
    if stencil:
        sd = tg.text_sd(stencil, cv.x - 0.05, cv.y - 0.13, 0.07, stroke=0.24)
        ink = tg.fill(sd, cv.px) * (0.3 + 0.7 * cut(cv.noise(salt + 17, 0.03, 0.002), 0.75, 0.3))
        paint = tg.lerp(paint, rgb((0.86, 0.84, 0.76)), ink)
    albedo = tg.lerp(paint, rgb(tg.PRIMER), primer)
    albedo = tg.lerp(albedo, rgb(tg.STEEL) * tg.tone(cv, salt + 19, 0.02, 0.1)[..., None], bare)
    albedo = tg.lerp(albedo, rgb((0.30, 0.20, 0.12)), heat * 0.55)
    albedo = tg.lerp(albedo, rgb(tg.RUST), np.clip(rust_run * 0.8, 0.0, 1.0))
    albedo = tg.lerp(albedo, rgb(tg.GRIME), np.clip(grime * 0.3 + dirt_edge * 0.4, 0.0, 0.8))
    albedo = tg.lerp(albedo, rgb((0.75, 0.73, 0.70)), scr * 0.55 * (1.0 - bare))
    rough = 0.58 + 0.08 * cv.noise(salt + 21, 0.05, 0.004)
    rough = tg.lerp(rough, 0.75, primer)
    rough = tg.lerp(rough, 0.35, bare)
    rough = tg.lerp(rough, 0.85, np.clip(grime * 0.6 + rust_run, 0.0, 1.0))
    metal = 0.55 * bare + 0.3 * scr * (1.0 - primer)
    return _finish(cv, h, albedo, rough, metal, [(0.004, 0.0012, 0.5), (0.015, 0.003, 0.3)])


def _rusted(cv: Canvas, salt: int):
    d = _rim(cv)
    f1, _, _ = cv.voronoi(salt, 9, 1.0)
    pits = cut(-f1 + 0.004 * cv.noise(salt + 1, 0.04, 0.004), 0.18, 0.004)
    h = 0.0006 * cv.noise(salt + 2, 0.06, 0.003) - 0.0006 * pits - 0.0015 * (1.0 - smoothstep(0.0, 0.01, d))
    flakes = cut(cv.noise(salt + 3, 0.15, 0.012) - 2.0 * (1.0 - smoothstep(0.0, 0.05, d)), 0.16, 0.15)
    h += 0.0003 * flakes
    t = cv.noise(salt + 4, 0.12, 0.008)
    albedo = tg.lerp(rgb((0.42, 0.21, 0.10)), rgb((0.58, 0.33, 0.16)), np.clip(0.5 + 0.35 * t, 0, 1))
    albedo = tg.lerp(albedo, rgb((0.22, 0.12, 0.07)), pits * 0.8)
    albedo = tg.lerp(albedo, rgb((0.50, 0.12, 0.08)), flakes * 0.85)
    rough = 0.88 - 0.25 * flakes
    metal = 0.05 + 0.1 * (1 - pits)
    return _finish(cv, h, albedo, rough, metal, [(0.003, 0.0008, 0.6), (0.012, 0.002, 0.3)])


def _scrap(cv: Canvas, salt: int):
    """Ferraille brute : brossé, meulage en arcs, deux cordons de soudure, teintes de chauffe."""
    d = _rim(cv)
    brushed = cv.noise(salt, 0.2, 0.0015, stretch=(0.08, 1.0))
    h = 0.00008 * brushed - 0.0015 * (1.0 - smoothstep(0.0, 0.01, d))
    wh1, heat1 = _weld(cv, salt + 1, [(0.0, 0.09), (0.08, 0.10), (0.17, 0.085), (0.25, 0.095)])
    wh2, heat2 = _weld(cv, salt + 2, [(0.16, 0.0), (0.155, 0.09)])
    h += wh1 + wh2
    heat = np.maximum(heat1, heat2)
    swirl = cv.scratches(salt + 3, 80, (0.01, 0.04), 0.0005, curve=0.3)
    albedo = rgb((0.56, 0.55, 0.53)) * (1.0 + 0.06 * brushed)[..., None]
    # Teinte de chauffe : paille puis bleu près du cordon.
    straw = rgb((0.62, 0.50, 0.30))
    blue = rgb((0.30, 0.34, 0.48))
    albedo = tg.lerp(albedo, straw, smoothstep(0.1, 0.5, heat) * 0.6)
    albedo = tg.lerp(albedo, blue, smoothstep(0.55, 0.9, heat) * 0.6)
    albedo = tg.lerp(albedo, rgb((0.36, 0.34, 0.32)), np.clip((wh1 + wh2) / 0.0018, 0, 1) * 0.7)
    grime = stains(cv, salt + 4, 0.12, 0.1, edge=0.6)
    albedo = tg.lerp(albedo, rgb(tg.GRIME), grime * 0.5)
    albedo = tg.lerp(albedo, rgb((0.75, 0.74, 0.72)), swirl * 0.5)
    rough = 0.38 + 0.25 * grime + 0.2 * heat
    metal = 0.55 * (1.0 - grime * 0.5)
    return _finish(cv, h, albedo, rough, metal, [(0.003, 0.0008, 0.5), (0.012, 0.002, 0.3)])


def _copper(cv: Canvas, salt: int):
    """Spires de fil de cuivre le long de U (le fil fait le tour de la bobine), oxydation."""
    pitch = CELL_M / 32
    t = (cv.y % pitch) / pitch * 2.0 - 1.0
    wire = np.sqrt(np.clip(1.0 - t * t, 0.0, 1.0))
    h = 0.0012 * wire
    ox = cut(cv.noise(salt, 0.1, 0.004), 0.08, 0.4)
    burn = cut(cv.noise(salt + 1, 0.15, 0.02), 0.06, 0.5)
    albedo = rgb((0.78, 0.45, 0.25)) * tg.tone(cv, salt + 2, 0.03, 0.08)[..., None]
    albedo = tg.lerp(albedo, rgb((0.35, 0.45, 0.36)), ox * 0.55)
    albedo = tg.lerp(albedo, rgb((0.24, 0.13, 0.08)), burn * 0.6)
    rough = 0.32 + 0.3 * ox + 0.2 * burn
    metal = 0.55 * (1.0 - ox * 0.6)
    return _finish(cv, h, albedo, rough, metal, [(0.002, 0.0008, 0.7)], edge_light=0.15)


def _weave(cv: Canvas, period: float, depth: float) -> np.ndarray:
    """Armure sergée (diagonale) le long de U."""
    a = np.sin((cv.x + cv.y) / period * np.pi)
    b = np.sin((cv.x - cv.y * 0.5) / (period * 0.7) * np.pi)
    return depth * (0.6 * a + 0.4 * b)


def _webbing(cv: Canvas, salt: int):
    h = _weave(cv, 0.0018, 0.00012)
    # Coutures en zigzag (bords de sangle) répétées : la sangle occupe une bande de la case.
    stitch_lines = np.zeros_like(cv.x)
    for y in np.arange(0.006, CELL_M, 0.03):
        zig = y + 0.0015 * np.sign(np.sin(cv.x / 0.003 * np.pi))
        stitch_lines = np.maximum(stitch_lines, np.clip(1.0 - np.abs(cv.y - zig) / 0.0008, 0, 1))
    h += 0.0002 * stitch_lines
    grime = stains(cv, salt, 0.12, 0.12, edge=0.6)
    albedo = rgb((0.34, 0.33, 0.24)) * tg.tone(cv, salt + 1, 0.04, 0.1)[..., None]
    albedo = tg.lerp(albedo, rgb((0.48, 0.45, 0.34)), stitch_lines * 0.6)
    albedo = tg.lerp(albedo, rgb(tg.GRIME), grime * 0.35)
    return _finish(cv, h, albedo, 0.85 + 0.05 * grime, 0.0, [(0.002, 0.0003, 0.5)], edge_light=0.0)


def _canvas(cv: Canvas, salt: int):
    """Toile de poche : rabat cousu en haut, bouton-pression, bords piqués, frottements."""
    d = _rim(cv)
    h = _weave(cv, 0.0012, 0.00008)
    flap_y = 0.155
    flap = smoothstep(flap_y - 0.002, flap_y + 0.002, cv.y)
    h += 0.0015 * flap - 0.0012 * (1.0 - smoothstep(0.0, 0.01, d))
    stitch = np.zeros_like(cv.x)
    for inset in (0.008,):
        sd = np.abs(tg.sd_box(cv.x - CELL_M / 2, cv.y - CELL_M / 2, CELL_M / 2 - inset, CELL_M / 2 - inset))
        dash = (np.sin((cv.x + cv.y) / 0.004 * np.pi) > 0).astype(np.float32)
        stitch = np.maximum(stitch, np.clip(1 - sd / 0.0009, 0, 1) * dash)
    fl = np.abs(cv.y - (flap_y + 0.006))
    stitch = np.maximum(stitch, np.clip(1 - fl / 0.0009, 0, 1) * (np.sin(cv.x / 0.004 * np.pi) > 0))
    h += 0.0002 * stitch
    snap = cv.zeros()
    cv.stamp(snap, CELL_M / 2, 0.19, 0.016, 0.016, _dome(0.011, 0.002))
    h += snap
    wear = cut(cv.noise(salt, 0.1, 0.004) + 2.5 * (1.0 - smoothstep(0.0, 0.03, d)), 0.15, 0.5)
    albedo = rgb((0.50, 0.46, 0.34)) * tg.tone(cv, salt + 1, 0.05, 0.1)[..., None]
    albedo = tg.lerp(albedo, rgb((0.62, 0.59, 0.48)), wear * 0.5)
    albedo = tg.lerp(albedo, rgb((0.30, 0.28, 0.22)), stitch * 0.5)
    albedo = tg.lerp(albedo, rgb((0.45, 0.44, 0.42)), (snap > 0.0002) * 1.0)
    albedo = tg.lerp(albedo, rgb(tg.GRIME), stains(cv, salt + 2, 0.12, 0.1, edge=0.6) * 0.3)
    metal = 0.5 * (snap > 0.0002)
    return _finish(cv, h, albedo, 0.85 - 0.4 * (snap > 0.0002), metal, [(0.003, 0.0006, 0.5), (0.01, 0.0015, 0.3)], edge_light=0.0)


def _grip(cv: Canvas, salt: int):
    k = 0.004
    a = np.abs(((cv.x + cv.y) % k) / k - 0.5)
    b = np.abs(((cv.x - cv.y) % k) / k - 0.5)
    h = 0.0004 * np.minimum(a, b) * 2.0
    worn = stains(cv, salt, 0.12, 0.15, edge=0.6)
    h *= 1.0 - 0.6 * worn
    albedo = rgb((0.17, 0.165, 0.16)) * tg.tone(cv, salt + 1, 0.04, 0.08)[..., None]
    albedo = tg.lerp(albedo, rgb((0.26, 0.25, 0.23)), worn * 0.6)
    return _finish(cv, h, albedo, 0.7 - 0.25 * worn, 0.0, [(0.002, 0.0003, 0.5)])


def _visor(cv: Canvas, salt: int):
    """Verre sombre teinté ; émission orangée, plus forte au centre, lignes de balayage."""
    d = _rim(cv)
    h = -0.001 * (1.0 - smoothstep(0.0, 0.008, d))
    scan = 0.82 + 0.18 * np.sin(cv.y / 0.0025 * np.pi) ** 2
    vgrad = 1.0 - 0.6 * np.abs(cv.y / CELL_M - 0.5) * 2.0
    hgrad = 0.75 + 0.25 * np.cos((cv.x / CELL_M - 0.5) * np.pi)
    scr = cv.scratches(salt, 40, (0.01, 0.06), 0.0006)
    lum = np.clip(vgrad * hgrad * scan * (1.0 - 0.5 * scr), 0.0, 1.0) * smoothstep(0.002, 0.01, d)
    emissive = rgb((1.0, 0.45, 0.14)) * lum[..., None]
    emissive = tg.lerp(emissive, rgb((1.0, 0.85, 0.55)), smoothstep(0.85, 1.0, lum) * 0.6)
    albedo = rgb((0.30, 0.12, 0.06)) + 0.1 * scr[..., None]
    return _finish(cv, h, albedo, 0.15 + 0.4 * scr, 0.0, [(0.002, 0.0005, 0.3)], emissive=emissive, edge_light=0.0)


def _housing(cv: Canvas, salt: int):
    """Fond commun des cases de voyants : boîtier anthracite usé."""
    d = _rim(cv)
    h = -0.0012 * (1.0 - smoothstep(0.0, 0.008, d)) + 0.0002 * cv.noise(salt, 0.05, 0.004)
    albedo = rgb((0.15, 0.15, 0.155)) * tg.tone(cv, salt + 1, 0.05, 0.08)[..., None]
    scr = cv.scratches(salt + 2, 50, (0.01, 0.05), 0.0006)
    albedo = tg.lerp(albedo, rgb((0.45, 0.45, 0.44)), scr * 0.5)
    return h, albedo


def _leds(cv: Canvas, salt: int):
    h, albedo = _housing(cv, salt)
    emis = cv.zeros(3)
    rng = cv.rng(salt + 3)
    palette = [(0.25, 1.0, 0.35), (1.0, 0.62, 0.12), (1.0, 0.16, 0.10), (0.25, 1.0, 0.35)]
    n = 4
    step = CELL_M / n
    for i in range(n):
        for j in range(n):
            cx, cy = (i + 0.5) * step, (j + 0.5) * step
            col = palette[(i + 2 * j + int(rng.integers(0, 3))) % len(palette)]
            on = 1.0 if rng.random() > 0.2 else 0.12
            r = 0.016
            dd = np.hypot(cv.x - cx, cv.y - cy)
            core = np.clip(1.0 - dd / r, 0.0, 1.0) ** 0.6
            bezel = np.clip(1.0 - np.abs(dd - r * 1.25) / 0.003, 0.0, 1.0)
            h = h + 0.0012 * core - 0.0004 * bezel
            emis += rgb(col) * (core * on)[..., None]
            albedo = tg.lerp(albedo, rgb(col) * 0.5, (core > 0.05) * 1.0)
            albedo = tg.lerp(albedo, rgb((0.05, 0.05, 0.05)), bezel * 0.8)
    return _finish(cv, h, albedo, 0.45, 0.1, [(0.003, 0.0006, 0.5)], emissive=emis, edge_light=0.0)


def _charge(cv: Canvas, salt: int):
    """Jauge de charge : cinq segments le long de U (le dernier presque éteint), graduations."""
    h, albedo = _housing(cv, salt)
    emis = cv.zeros(3)
    n = 5
    margin = 0.02
    seg_w = (CELL_M - 2 * margin) / n
    for k in range(n):
        cx = margin + (k + 0.5) * seg_w
        sd = tg.sd_box(cv.x - cx, cv.y - CELL_M / 2, seg_w * 0.4, CELL_M * 0.3, 0.004)
        m = tg.fill(sd, cv.px)
        level = 1.0 if k < n - 1 else 0.18
        grad = 0.75 + 0.25 * np.cos((cv.y / CELL_M - 0.5) * np.pi)
        emis += rgb((0.30, 0.95, 1.0)) * (m * level * grad)[..., None]
        albedo = tg.lerp(albedo, rgb((0.12, 0.35, 0.38)), m)
        h = h - 0.0006 * np.clip(1.0 - np.abs(sd) / 0.002, 0, 1)
    return _finish(cv, h, albedo, 0.3, 0.1, [(0.003, 0.0006, 0.5)], emissive=emis, edge_light=0.0)


def _glow(cv: Canvas, salt: int, color):
    d = _rim(cv)
    lum = 0.7 + 0.3 * smoothstep(0.0, 0.06, d)
    lum = lum * (0.94 + 0.06 * cv.noise(salt, 0.03, 0.004))
    emis = rgb(color) * lum[..., None]
    albedo = rgb(color) * 0.6
    h = -0.0008 * (1.0 - smoothstep(0.0, 0.006, d))
    return _finish(cv, h, albedo, 0.2, 0.0, [(0.002, 0.0005, 0.3)], emissive=emis, edge_light=0.0)


def _generators():
    return {
        "paint_red": lambda cv: _paint(cv, 100, (0.62, 0.13, 0.09), wear=1.0, patch=True),
        "paint_blood": lambda cv: _paint(cv, 200, (0.47, 0.08, 0.06), wear=1.5, rivets=False,
                                         weld_path=[(0.02, 0.17), (0.12, 0.15), (0.23, 0.18)], rust=0.8),
        "paint_rust": lambda cv: _paint(cv, 300, (0.70, 0.37, 0.21), wear=1.2, rust=0.9),
        "paint_ochre": lambda cv: _paint(cv, 400, (0.66, 0.52, 0.25), wear=1.0, rivets=False, stripes=True),
        "paint_olive": lambda cv: _paint(cv, 500, (0.44, 0.46, 0.31), wear=0.9, patch=True),
        "primer": lambda cv: _paint(cv, 600, (0.55, 0.55, 0.52), wear=0.8, stencil="A2"),
        "rusted": lambda cv: _rusted(cv, 700),
        "scrap": lambda cv: _scrap(cv, 800),
        "copper": lambda cv: _copper(cv, 900),
        "webbing": lambda cv: _webbing(cv, 1000),
        "canvas": lambda cv: _canvas(cv, 1100),
        "grip": lambda cv: _grip(cv, 1200),
        "visor": lambda cv: _visor(cv, 1300),
        "leds": lambda cv: _leds(cv, 1400),
        "charge": lambda cv: _charge(cv, 1500),
        "glow_cyan": lambda cv: _glow(cv, 1600, (0.35, 0.95, 1.0)),
    }


def generate() -> dict[str, Path]:
    maps = {
        "color": np.zeros((RES, RES, 3), np.float32),
        "normal": np.zeros((RES, RES, 3), np.float32),
        "orm": np.zeros((RES, RES, 3), np.float32),
        "emissive": np.zeros((RES, RES, 3), np.float32),
    }
    for name, gen in _generators().items():
        col, row = CELLS[name]
        # Une case = un canevas à part : ni le bruit ni les flous ne débordent sur la voisine.
        cv = Canvas(CELL_PX, CELL_M, seed=4242 + col + GRID * row)
        out = gen(cv)
        ys, xs = slice(row * CELL_PX, (row + 1) * CELL_PX), slice(col * CELL_PX, (col + 1) * CELL_PX)
        for role in ("color", "normal", "orm"):
            maps[role][ys, xs] = out[role]
        if "emissive" in out:
            maps["emissive"][ys, xs] = out["emissive"]
    written = {}
    for role, img in maps.items():
        path = OUT_DIR / f"{NAME}_{role}.png"
        tg.write_png(path, img)
        written[role] = path
    PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
    tg.write_png(PREVIEW_DIR / "atlas_color.png", maps["color"])
    tg.write_png(PREVIEW_DIR / "atlas_emissive.png", maps["emissive"])
    tg.write_png(PREVIEW_DIR / "atlas_normal.png", maps["normal"])
    print(f"atlas {NAME} : {len(written)} cartes {RES} px dans {OUT_DIR}")
    return written


if __name__ == "__main__":
    generate()
