"""Atlas de textures des objets d'inventaire (lot F de M9) : `item_gear`.

Pourquoi un atlas : neuf petits objets (5 à 45 cm), chacun avec deux ou trois matières (coque
peinte, caoutchouc, métal, écran, voyants). Les matériaux de la bibliothèque sont à l'échelle
des décors (panneaux de 1 m) et un jeu de textures 512 par objet chargerait neuf fois quatre
images. Une seule texture de 1K découpée en 16 cases de 256 px (équivalent de quatre jeux 512)
donne toutes ces matières en **un seul matériau** partagé par tous les objets.

Chaque case a sa propre taille physique (celle de la face qu'elle habille : la case de l'écran
du bloc-données couvre 15 × 9,5 cm, celle de la puce 5,5 × 3,2 cm) : reliefs, normales et
usure sont à la bonne échelle une fois la case étirée sur la pièce. Les bords d'une case sont
les arêtes de la pièce (écaillures, bord poli, crasse).

Émission : un seul facteur pour tout le matériau (`EMIT_STRENGTH`, sortie en
KHR_materials_emissive_strength) ; chaque case écrit sa luminance visée **en linéaire**, convertie
en sRGB / `EMIT_STRENGTH` à l'écriture. Repères du jeu (AGENTS.md §4, l'émissif n'est pas atténué
par l'exposition) : écran ≈ 1 à 1,6, voyant ≈ 4.

Cartes écrites (contrat du jeu) dans assets/textures/materials/ :
`item_gear_color.png` (sRGB), `item_gear_normal.png` (OpenGL, linéaire), `item_gear_orm.png`
(R occlusion, G rugosité, B métal, linéaire), `item_gear_emissive.png` (sRGB).

Aucun texte lisible, aucun logo : pseudo-lignes en tirets, pictogrammes abstraits, symbole de
premiers soins générique (croix blanche sur fond vert).

    blender --background --factory-startup --python tools/blender/items/item_atlas.py
"""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "materials"))

import texgen as tg  # noqa: E402
from texgen import Canvas, cut, rgb, smoothstep, stains  # noqa: E402

ROOT = HERE.parents[2]
OUT_DIR = ROOT / "assets" / "textures" / "materials"
PREVIEW_DIR = ROOT / "target" / "art-previews" / "m9-lotF"
NAME = "item_gear"
RES = 1024
GRID = 4
CELL_PX = RES // GRID
EMIT_STRENGTH = 4.0  # facteur d'émission du matériau (luminance linéaire d'un pixel blanc)

# Cases (colonne, ligne), ligne 0 en BAS de l'image (v = 0), comme les UV de Blender ; taille
# physique (largeur U, hauteur V) en mètres.
CELLS: dict[str, tuple[int, int]] = {
    "shell": (0, 0),        # coque du bloc-données : polymère olive-gris moulé, vis, usure
    "rubber": (1, 0),       # caoutchouc noir (pare-chocs, poignées, gaines)
    "case_ochre": (2, 0),   # mallette à outils : peinture ocre industrielle, rivets
    "case_white": (3, 0),   # trousse de soins : peinture blanc cassé, salissures
    "med_cross": (0, 1),    # plaque de premiers soins : croix blanche sur vert (générique)
    "foil": (1, 1),         # emballage de ration : film métallisé imprimé olive, froissé
    "chip": (2, 1),         # dessus de la puce : cadre, fenêtre à pistes lumineuses, contacts
    "manifest": (3, 1),     # face du manifeste : ardoise anodisée, étiquette, code-barres, sceau
    "screen": (0, 2),       # écran du bloc-données (émissif doux)
    "metal": (1, 2),        # acier brossé usé (engrenages, loquets, outils, anneau de col)
    "copper": (2, 2),       # bobinage de cuivre
    "cables": (3, 2),       # quatre gaines de câble (rouge, jaune, bleu, noir) en colonnes
    "webbing": (0, 3),      # sangle tissée (U en travers, V le long)
    "leds": (1, 3),         # quatre quadrants : voyant vert, ambre, rouge, touche rétroéclairée
    "paint_olive": (2, 3),  # caisse de pièces : olive usé, rivets, rouille
    "hazard": (3, 3),       # bandes de danger ocre / noir à 45°
}
SIZES: dict[str, tuple[float, float]] = {
    "shell": (0.20, 0.145),
    "rubber": (0.10, 0.10),
    "case_ochre": (0.40, 0.20),
    "case_white": (0.30, 0.20),
    "med_cross": (0.09, 0.09),
    "foil": (0.15, 0.085),
    "chip": (0.055, 0.032),
    "manifest": (0.24, 0.17),
    "screen": (0.15, 0.095),
    "metal": (0.15, 0.15),
    "copper": (0.08, 0.08),
    "cables": (0.10, 0.25),
    "webbing": (0.05, 0.25),
    "leds": (0.08, 0.08),
    "paint_olive": (0.36, 0.12),
    "hazard": (0.10, 0.10),
}
# Sous-cases (fractions de case : u0, v0, u1, v1).
SUBCELLS: dict[str, tuple[str, tuple[float, float, float, float]]] = {
    "led_green": ("leds", (0.0, 0.0, 0.5, 0.5)),
    "led_amber": ("leds", (0.5, 0.0, 1.0, 0.5)),
    "led_red": ("leds", (0.0, 0.5, 0.5, 1.0)),
    "key_lit": ("leds", (0.5, 0.5, 1.0, 1.0)),
    "cable_red": ("cables", (0.0, 0.0, 0.25, 1.0)),
    "cable_yellow": ("cables", (0.25, 0.0, 0.5, 1.0)),
    "cable_blue": ("cables", (0.5, 0.0, 0.75, 1.0)),
    "cable_black": ("cables", (0.75, 0.0, 1.0, 1.0)),
    "med_green": ("med_cross", (0.07, 0.07, 0.30, 0.30)),  # vert uni (coin de la plaque, hors croix)
}


def cell_size(name: str) -> tuple[float, float]:
    """Taille physique (m) d'une case ou d'une sous-case : (largeur U, hauteur V)."""
    if name in SUBCELLS:
        parent, (a0, b0, a1, b1) = SUBCELLS[name]
        w, h = SIZES[parent]
        return (w * (a1 - a0), h * (b1 - b0))
    return SIZES[name]


def cell_rect(name: str, pad_px: float = 4.0) -> tuple[float, float, float, float]:
    """(u0, v0, u1, v1) d'une case ou d'une sous-case, rognée de `pad_px` texels (le filtrage
    et les mipmaps proches ne débordent pas sur la voisine)."""
    if name in SUBCELLS:
        parent, (a0, b0, a1, b1) = SUBCELLS[name]
        col, row = CELLS[parent]
        cu, cv_ = col / GRID, row / GRID
        s = 1.0 / GRID
        pad = pad_px / RES
        return (cu + a0 * s + pad, cv_ + b0 * s + pad, cu + a1 * s - pad, cv_ + b1 * s - pad)
    col, row = CELLS[name]
    pad = pad_px / RES
    return (col / GRID + pad, row / GRID + pad, (col + 1) / GRID - pad, (row + 1) / GRID - pad)


def linear_to_srgb(c):
    c = np.clip(np.asarray(c, dtype=np.float32), 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * np.power(c, 1.0 / 2.4) - 0.055).astype(np.float32)


# --- Briques -----------------------------------------------------------------------------------


class Layers:
    """Couches d'une case : hauteur (m), couleur sRGB, rugosité, métal, émission linéaire."""

    def __init__(self, cv: Canvas, color, rough: float = 0.6, metal: float = 0.0):
        self.h = cv.zeros()
        self.albedo = cv.full(color)
        self.rough = cv.full(rough)
        self.metal = cv.full(metal)
        self.emis = cv.zeros(3)

    def paint(self, mask, color, rough=None, metal=None, h=None):
        self.albedo = tg.lerp(self.albedo, rgb(color), mask)
        if rough is not None:
            self.rough = tg.lerp(self.rough, rough, mask)
        if metal is not None:
            self.metal = tg.lerp(self.metal, metal, mask)
        if h is not None:
            self.h = self.h + h * mask


def _rim(cv: Canvas) -> np.ndarray:
    """Distance (m) au bord de la case : les arêtes de la pièce."""
    return np.minimum(np.minimum(cv.x, cv.tile - cv.x), np.minimum(cv.y, cv.tile_h - cv.y))


def _dome(r: float, height: float):
    def fn(gx, gy):
        d = np.hypot(gx, gy)
        return height * np.sqrt(np.clip(1.0 - (d / r) ** 2, 0.0, 1.0))
    return fn


def _rivets_border(cv: Canvas, inset: float, spacing: float, r: float = 0.003, height: float = 0.0011) -> np.ndarray:
    out = cv.zeros()
    w, hh = cv.tile, cv.tile_h
    pts = set()
    nx = max(1, int(round((w - 2 * inset) / spacing)))
    ny = max(1, int(round((hh - 2 * inset) / spacing)))
    for t in np.linspace(inset, w - inset, nx + 1):
        pts.add((round(float(t), 5), round(inset, 5)))
        pts.add((round(float(t), 5), round(hh - inset, 5)))
    for t in np.linspace(inset, hh - inset, ny + 1):
        pts.add((round(inset, 5), round(float(t), 5)))
        pts.add((round(w - inset, 5), round(float(t), 5)))
    for x, y in sorted(pts):
        cv.stamp(out, x, y, r * 1.5, r * 1.5, _dome(r, height))
    return out


def _screws(cv: Canvas, points, r: float = 0.0028) -> tuple[np.ndarray, np.ndarray]:
    """Vis fendues bombées : (hauteur, masque)."""
    h = cv.zeros()
    for i, (x, y) in enumerate(points):
        cv.stamp(h, x, y, r * 1.6, r * 1.6, lambda gx, gy, a=0.5 + i * 0.9: tg.screw(gx, gy, r, a, 0.0007))
    mask = np.clip(cv.blur((h > 0.00005).astype(np.float32), cv.px) * 1.5, 0.0, 1.0)
    return h, mask


def _rect_sd(cv: Canvas, x0, y0, x1, y1, r=0.0):
    return tg.sd_box(cv.x - (x0 + x1) / 2, cv.y - (y0 + y1) / 2, (x1 - x0) / 2, (y1 - y0) / 2, r)


def _wear(cv: Canvas, L: Layers, salt: int, wear: float, under, bare, bare_metal: float, edge_w: float = 0.018,
          scratches: int = 50, grime: float = 0.3, extra_edge=None):
    """Usure d'une peinture : écaillures sur les arêtes (sous-couche puis matière nue), rayures,
    crasse dans les coins et en bas."""
    d = _rim(cv)
    edge = 1.0 - smoothstep(0.0015, edge_w, d)
    if extra_edge is not None:
        edge = np.maximum(edge, extra_edge)
    chips = tg.chip_field(cv, salt, edge * wear, 0.004, edge_weight=3.0)
    under_m = cut(chips, 0.10 * wear)
    bare_m = cut(chips, 0.045 * wear)
    L.h = L.h - 0.00018 * under_m - 0.00008 * bare_m
    L.paint(under_m, under, rough=0.75)
    L.paint(bare_m, bare, rough=0.38, metal=bare_metal)
    scr = cv.scratches(salt + 5, scratches, (0.006, 0.045), 0.0004, curve=0.05)
    L.paint(scr * 0.3, tg.lerp(np.asarray(under), np.asarray(bare), 0.5).tolist())
    g = stains(cv, salt + 7, min(cv.tile, cv.tile_h) * 0.5, 0.12, edge=1.2, breakup=0.15) * 0.5
    corner = (1.0 - smoothstep(0.0, 0.012, d)) * 0.5
    L.paint(np.clip(g * grime + corner * grime, 0.0, 0.8), tg.GRIME, rough=0.85)
    return under_m, bare_m


def _done(cv: Canvas, L: Layers, ao_layers, edge_light: float = 0.06, normal_strength: float = 1.0):
    out = tg.finish(cv, L.h, L.albedo, L.rough, L.metal, ao_layers, edge_light=edge_light,
                    normal_strength=normal_strength)
    out["emissive"] = L.emis
    return out


# --- Cases -------------------------------------------------------------------------------------


def _shell(cv: Canvas, salt: int):
    """Coque de bloc-données : polymère moulé olive-gris, ligne de joint, quatre vis, usure."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.40, 0.41, 0.36), rough=0.62)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.08, 0.05)[..., None]
    L.h += 0.00004 * cv.noise(salt + 1, 0.004, 0.0012)  # grain moulé
    d = _rim(cv)
    L.h += -0.0006 * (1.0 - smoothstep(0.0, 0.006, d))
    groove = np.clip(1.0 - np.abs(-_rect_sd(cv, 0, 0, w, hh) - 0.0045) / 0.0007, 0.0, 1.0)
    L.h -= 0.00025 * groove
    sh, smask = _screws(cv, [(0.05, 0.0075), (w - 0.05, 0.0075), (0.05, hh - 0.0075), (w - 0.05, hh - 0.0075)])
    L.h += sh
    L.paint(smask, (0.50, 0.50, 0.48), rough=0.35, metal=0.5)
    _wear(cv, L, salt + 10, 0.9, (0.27, 0.28, 0.25), (0.52, 0.52, 0.49), 0.0, scratches=60, grime=0.35)
    return _done(cv, L, [(0.003, 0.0006, 0.5), (0.012, 0.0015, 0.3)])


def _rubber(cv: Canvas, salt: int):
    L = Layers(cv, (0.085, 0.085, 0.09), rough=0.86)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.05, 0.08)[..., None]
    L.h += 0.00012 * cv.noise(salt + 1, 0.01, 0.0015)
    d = _rim(cv)
    L.h += -0.0005 * (1.0 - smoothstep(0.0, 0.006, d))
    # Quadrillage d'adhérence effacé par endroits.
    k = 0.003
    a = np.abs(((cv.x + cv.y) % k) / k - 0.5)
    b = np.abs(((cv.x - cv.y) % k) / k - 0.5)
    knurl = np.minimum(a, b) * 2.0
    worn = stains(cv, salt + 2, 0.05, 0.3, edge=0.6)
    L.h += 0.00018 * knurl * (1.0 - worn) * smoothstep(0.006, 0.012, d)
    scuff = cv.scratches(salt + 3, 40, (0.005, 0.03), 0.0006)
    L.paint(np.clip(scuff * 0.6 + worn * 0.25, 0, 1), (0.17, 0.17, 0.17), rough=0.65)
    dust = stains(cv, salt + 4, 0.05, 0.15, edge=0.7)
    L.paint(dust * 0.25, tg.DUST)
    return _done(cv, L, [(0.002, 0.0003, 0.5)], edge_light=0.0)


def _case(cv: Canvas, salt: int, color, rivets: bool, wear: float, under, grime: float, rust: float):
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, color, rough=0.55)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.15, 0.035)[..., None]
    d = _rim(cv)
    L.h += -0.0007 * (1.0 - smoothstep(0.0, 0.008, d)) + 0.00003 * cv.noise(salt + 1, 0.03, 0.004)
    riv_mask = cv.zeros()
    if rivets:
        riv = _rivets_border(cv, 0.0095, 0.036, 0.0032, 0.0011)
        L.h += riv
        riv_mask = np.clip(cv.blur((riv > 0.0002).astype(np.float32), 0.002) * 2.5, 0.0, 1.0)
    _, bare = _wear(cv, L, salt + 10, wear, under, tg.STEEL, 0.5, scratches=22, grime=grime,
                    extra_edge=0.7 * riv_mask)
    # Coulures de rouille sous les rivets et les éclats, crasse plus forte vers le bas (V).
    src = np.clip(riv_mask * 0.7 + bare * 0.6, 0.0, 1.0) * cut(cv.noise(salt + 20, 0.03, 0.003), 0.6, 0.3)
    run = tg.drips(cv, src, 0.035, salt + 21) * rust
    L.paint(np.clip(run * 0.8, 0, 1), tg.RUST, rough=0.85)
    low = (1.0 - smoothstep(0.0, hh * 0.35, cv.y)) * np.clip(0.5 + 0.3 * cv.noise(salt + 22, 0.05, 0.004), 0.0, 1.0)
    L.paint(low * grime * 0.45, tg.GRIME, rough=0.85)
    return _done(cv, L, [(0.004, 0.0008, 0.5), (0.015, 0.002, 0.3)])


def _med_cross(cv: Canvas, salt: int):
    """Plaque de premiers soins générique : croix blanche sur vert, liseré sombre, usée."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.07, 0.43, 0.32), rough=0.5)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.05, 0.05)[..., None]
    d = _rim(cv)
    L.h += -0.0005 * (1.0 - smoothstep(0.0, 0.004, d))
    border = 1.0 - smoothstep(0.0035, 0.0045, d)
    L.paint(border, (0.10, 0.11, 0.11), rough=0.6)
    cx, cy = w / 2, hh / 2
    arm, half = 0.0125, 0.031
    sd = np.minimum(tg.sd_box(cv.x - cx, cv.y - cy, arm, half, 0.0015), tg.sd_box(cv.x - cx, cv.y - cy, half, arm, 0.0015))
    cross = tg.fill(sd, cv.px)
    L.paint(cross, (0.90, 0.90, 0.86), rough=0.55, h=0.00025)
    _wear(cv, L, salt + 10, 0.7, (0.30, 0.32, 0.30), tg.STEEL, 0.5, scratches=35, grime=0.25,
          extra_edge=0.35 * (1.0 - smoothstep(0.0, 0.002, np.abs(sd))))
    return _done(cv, L, [(0.002, 0.0004, 0.5), (0.01, 0.0012, 0.3)])


def _foil(cv: Canvas, salt: int):
    """Film métallisé imprimé olive : plis froissés, soudures striées aux deux bouts, bandeau
    clair à pictogrammes abstraits, repère orange, ligne d'ouverture en tirets."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.31, 0.33, 0.23), rough=0.42, metal=0.12)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.04, 0.05)[..., None]
    crease = cv.zeros()
    for i, (big, st) in enumerate(((0.03, (1.0, 1.0)), (0.02, (2.5, 1.0)), (0.015, (1.0, 2.5)))):
        n = cv.noise(salt + 1 + i, big, 0.002, stretch=st)
        crease = np.maximum(crease, 1.0 - smoothstep(0.0, 0.10, np.abs(n)))
    L.h += 0.00022 * crease + 0.00006 * cv.noise(salt + 5, 0.006, 0.0012)
    seal = 0.013
    ends = np.maximum(1.0 - smoothstep(seal - 0.001, seal, cv.x), smoothstep(w - seal, w - seal + 0.001, cv.x))
    ribs = 0.5 + 0.5 * np.sin(cv.y / 0.0016 * 2 * np.pi)
    L.h = L.h * (1.0 - ends) + 0.00018 * ribs * ends
    L.paint(ends * 0.8, (0.62, 0.62, 0.58), rough=0.3, metal=0.5)
    # Bandeau clair imprimé (en travers du paquet), pictogrammes sombres sans texte.
    band = tg.fill(_rect_sd(cv, 0.052, -0.01, 0.098, hh + 0.01), cv.px)
    L.paint(band, (0.70, 0.64, 0.47), rough=0.55, metal=0.0)
    ink = cv.zeros()
    for j, yy in enumerate((0.022, 0.042, 0.062)):
        ink = np.maximum(ink, tg.fill(tg.sd_box(cv.x - 0.062, cv.y - yy, 0.0048, 0.0048, 0.0012), cv.px))
        ink = np.maximum(ink, tg.fill(_rect_sd(cv, 0.071, yy - 0.0016, 0.071 + (0.016, 0.022, 0.012)[j], yy + 0.0016, 0.0008), cv.px))
    ink = np.maximum(ink, tg.fill(_rect_sd(cv, 0.056, 0.072, 0.094, 0.0755, 0.0008), cv.px))
    L.paint(ink * band, (0.16, 0.16, 0.12))
    stripe = tg.fill(_rect_sd(cv, 0.030, -0.01, 0.0355, hh + 0.01), cv.px)
    L.paint(stripe, (0.80, 0.40, 0.10), rough=0.45, metal=0.0)
    dash = (np.sin(cv.y / 0.0025 * np.pi) > 0.0).astype(np.float32)
    tear = np.clip(1.0 - np.abs(cv.x - 0.020) / 0.0005, 0.0, 1.0) * dash
    L.paint(tear, (0.12, 0.12, 0.09))
    # Encre arrachée sur les plis : le métal du film apparaît.
    scuff = np.clip(crease * cut(cv.noise(salt + 8, 0.02, 0.002), 0.25, 0.3) + cv.scratches(salt + 9, 40, (0.004, 0.02), 0.0003) * 0.8, 0, 1)
    L.paint(scuff * (1.0 - ends) * 0.7, (0.66, 0.66, 0.62), rough=0.28, metal=0.5)
    L.paint(stains(cv, salt + 10, 0.05, 0.1, edge=0.6) * 0.25, tg.GRIME)
    return _done(cv, L, [(0.002, 0.0002, 0.4)], edge_light=0.1)


def _chip(cv: Canvas, salt: int):
    """Dessus de la puce (U le long de la puce, contacts vers +U) : cadre polymère, fenêtre de
    verre à pistes cyan lumineuses, six contacts dorés, œillet."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.13, 0.14, 0.15), rough=0.42)
    d = _rim(cv)
    L.h += -0.0003 * (1.0 - smoothstep(0.0, 0.002, d)) + 0.00002 * cv.noise(salt, 0.003, 0.0006)
    win_sd = _rect_sd(cv, 0.011, 0.006, 0.041, 0.026, 0.002)
    win = tg.fill(win_sd, cv.px)
    L.paint(win, (0.035, 0.06, 0.07), rough=0.5, h=-0.00015)
    L.paint(np.clip(1.0 - np.abs(win_sd) / 0.0006, 0, 1) * 0.8, (0.30, 0.31, 0.32), rough=0.3, metal=0.5)
    # Pistes : segments de Manhattan tirés au hasard (graine fixe), nœuds plus brillants.
    rng = cv.rng(salt + 1)
    trace = cv.zeros()
    nodes = cv.zeros()
    for _ in range(14):
        x, y = rng.uniform(0.013, 0.039), rng.uniform(0.008, 0.024)
        for _s in range(3):
            if rng.random() < 0.5:
                x2, y2 = float(np.clip(x + rng.uniform(-0.012, 0.012), 0.013, 0.039)), y
            else:
                x2, y2 = x, float(np.clip(y + rng.uniform(-0.008, 0.008), 0.008, 0.024))
            trace = np.maximum(trace, np.clip(1.0 - tg.sd_segment(cv.x, cv.y, x, y, x2, y2) / 0.00028, 0, 1))
            x, y = x2, y2
        nodes = np.maximum(nodes, np.clip(1.0 - np.hypot(cv.x - x, cv.y - y) / 0.0008, 0, 1))
    glow = cv.blur(trace, 0.0005)
    L.emis += rgb((0.35, 0.95, 1.0)) * (win * (0.10 + 2.4 * trace + 1.0 * glow + 3.5 * nodes))[..., None]
    L.paint(np.clip(trace + nodes, 0, 1) * win, (0.20, 0.55, 0.60))
    # Contacts dorés vers +U.
    gold = cv.zeros()
    for k in range(6):
        y0 = 0.0045 + k * 0.0040
        gold = np.maximum(gold, tg.fill(_rect_sd(cv, 0.0445, y0, 0.0535, y0 + 0.0028, 0.0004), cv.px))
    L.paint(gold, (0.80, 0.62, 0.27), rough=0.3, metal=0.55, h=0.00008)
    # Œillet (cordon) vers −U.
    rr = np.hypot(cv.x - 0.0055, cv.y - hh / 2)
    L.paint(tg.fill(rr - 0.0032, cv.px), (0.42, 0.42, 0.41), rough=0.35, metal=0.55, h=0.0001)
    L.paint(tg.fill(rr - 0.0022, cv.px), (0.02, 0.02, 0.02), rough=0.9, h=-0.0004)
    scr = cv.scratches(salt + 3, 25, (0.003, 0.015), 0.00015)
    L.paint(scr * 0.4, (0.35, 0.36, 0.37))
    return _done(cv, L, [(0.001, 0.0002, 0.5)], edge_light=0.05)


def _manifest(cv: Canvas, salt: int):
    """Face du manifeste : ardoise anodisée bleu-gris, lèvre de cadre, vis, étiquette pâle à
    pseudo-lignes (tirets illisibles), code-barres, sceau embouti géométrique, voyant ambre."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.22, 0.28, 0.36), rough=0.55, metal=0.15)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.1, 0.05)[..., None]
    d = _rim(cv)
    lip = 1.0 - smoothstep(0.0065, 0.0075, d)
    L.paint(lip, (0.18, 0.20, 0.23), rough=0.5, h=0.0004)
    L.h += -0.0004 * (1.0 - smoothstep(0.0, 0.003, d))
    sh, smask = _screws(cv, [(0.0045 + 0.0005, 0.005), (w - 0.005, 0.005), (0.005, hh - 0.005), (w - 0.005, hh - 0.005)], 0.0024)
    L.h += sh
    L.paint(smask, (0.50, 0.50, 0.48), rough=0.35, metal=0.5)
    # Étiquette : rangées de tirets de longueurs tirées au hasard (aucune lettre).
    lab_sd = _rect_sd(cv, 0.016, 0.068, 0.136, 0.152, 0.002)
    lab = tg.fill(lab_sd, cv.px)
    L.paint(lab, (0.80, 0.77, 0.66), rough=0.8, metal=0.0, h=0.0001)
    rng = cv.rng(salt + 1)
    ink = tg.fill(_rect_sd(cv, 0.022, 0.138, 0.090, 0.146, 0.0008), cv.px)
    for r in range(6):
        y = 0.128 - r * 0.0105
        x = 0.022
        while x < 0.128:
            ln = float(rng.uniform(0.004, 0.018))
            x1 = min(x + ln, 0.130)
            ink = np.maximum(ink, tg.fill(_rect_sd(cv, x, y - 0.0018, x1, y + 0.0018, 0.0006), cv.px))
            x = x1 + float(rng.uniform(0.0025, 0.005))
    L.paint(ink * lab * 0.9, (0.14, 0.14, 0.13))
    # Code-barres.
    bar_bg = tg.fill(_rect_sd(cv, 0.150, 0.098, 0.226, 0.152, 0.0015), cv.px)
    L.paint(bar_bg, (0.80, 0.77, 0.66), rough=0.8, metal=0.0, h=0.0001)
    x = 0.155
    bars = cv.zeros()
    while x < 0.221:
        bw = float(rng.choice([0.0008, 0.0012, 0.002, 0.0028]))
        bars = np.maximum(bars, tg.fill(_rect_sd(cv, x, 0.103, min(x + bw, 0.221), 0.147), cv.px))
        x += bw + float(rng.choice([0.0008, 0.0012, 0.0018]))
    L.paint(bars * bar_bg, (0.10, 0.10, 0.10))
    # Sceau embouti : anneau et rayons (géométrique, sans emblème).
    sx, sy = 0.190, 0.045
    rr = np.hypot(cv.x - sx, cv.y - sy)
    ang = np.arctan2(cv.y - sy, cv.x - sx)
    ring = np.clip(1.0 - np.abs(rr - 0.020) / 0.0015, 0, 1) + np.clip(1.0 - np.abs(rr - 0.0135) / 0.001, 0, 1)
    spokes = (np.cos(ang * 12) > 0.6) * ((rr > 0.005) & (rr < 0.012))
    star = np.clip(ring + spokes * 1.0, 0, 1)
    L.h += 0.0005 * cv.blur(star.astype(np.float32), 0.0004)
    L.paint(tg.fill(rr - 0.0215, cv.px) * 0.35, (0.36, 0.40, 0.46), metal=0.45)
    # Voyant ambre segmenté (état du scellé).
    status = cv.zeros()
    for k in range(4):
        x0 = 0.020 + k * 0.0105
        status = np.maximum(status, tg.fill(_rect_sd(cv, x0, 0.020, x0 + 0.0085, 0.029, 0.0008), cv.px))
    slot = tg.fill(_rect_sd(cv, 0.017, 0.017, 0.0645, 0.032, 0.0012), cv.px)
    L.paint(slot, (0.05, 0.05, 0.05), rough=0.2, metal=0.0, h=-0.0002)
    L.paint(status, (0.45, 0.28, 0.08))
    L.emis += rgb((1.0, 0.62, 0.16)) * (status * 1.8)[..., None]
    _wear(cv, L, salt + 10, 0.6, (0.20, 0.23, 0.27), (0.62, 0.63, 0.63), 0.55, edge_w=0.010, scratches=60, grime=0.25)
    return _done(cv, L, [(0.003, 0.0005, 0.5), (0.012, 0.0012, 0.3)])


def _screen(cv: Canvas, salt: int):
    """Écran du bloc-données (V = haut de l'écran) : verre sombre, interface générique en
    lignes et en blocs (courbe, barres, jauge, rangées de tirets), lignes de balayage."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.025, 0.035, 0.04), rough=0.55)
    d = _rim(cv)
    L.h += -0.0003 * (1.0 - smoothstep(0.0, 0.002, d))
    col = cv.zeros(3)
    cyan, green, amber = rgb((0.45, 0.95, 0.85)), rgb((0.55, 1.0, 0.65)), rgb((1.0, 0.66, 0.22))

    def add(mask, color, lum):
        nonlocal col
        col = col + color * (mask * lum)[..., None]

    def frame_rect(x0, y0, x1, y1):
        sd = _rect_sd(cv, x0, y0, x1, y1, 0.0012)
        return np.clip(1.0 - np.abs(sd) / 0.00035, 0, 1), tg.fill(sd, cv.px)

    # Fond : lueur très faible, plus claire au centre.
    vign = 1.0 - 0.5 * np.clip(((cv.x / w - 0.5) * 2) ** 2 + ((cv.y / hh - 0.5) * 2) ** 2, 0, 1)
    add(np.ones_like(cv.x) * vign, cyan, 0.10)
    # Bandeau d'en-tête et ses blocs.
    hdr = tg.fill(_rect_sd(cv, 0.006, hh - 0.013, w - 0.006, hh - 0.0045, 0.001), cv.px)
    add(hdr, cyan, 0.45)
    blocks = cv.zeros()
    for x0, x1 in ((0.009, 0.030), (0.033, 0.041), (0.118, 0.141)):
        blocks = np.maximum(blocks, tg.fill(_rect_sd(cv, x0, hh - 0.0105, x1, hh - 0.007, 0.0006), cv.px))
    add(blocks, cyan, 0.9)
    # Panneau gauche : grille de points et courbe.
    fr, inside = frame_rect(0.006, 0.030, 0.081, 0.078)
    add(fr, cyan, 0.55)
    gx = (np.abs(((cv.x - 0.006) % 0.0075) - 0.00375) < 0.00025) & (np.abs(((cv.y - 0.030) % 0.006) - 0.003) < 0.00025)
    add(gx * inside, cyan, 0.35)
    xs = np.linspace(0.008, 0.079, 60)
    rng = cv.rng(salt + 1)
    ys = 0.054 + 0.011 * np.sin(xs * 160) * np.exp(-((xs - 0.045) / 0.03) ** 2) + np.cumsum(rng.normal(0, 0.0012, len(xs)))
    ys = np.clip(ys - (ys.mean() - 0.054), 0.034, 0.075)
    curve = cv.zeros()
    for (ax, ay), (bx, by) in zip(zip(xs[:-1], ys[:-1]), zip(xs[1:], ys[1:])):
        curve = np.maximum(curve, np.clip(1.0 - tg.sd_segment(cv.x, cv.y, ax, ay, bx, by) / 0.0006, 0, 1))
    add(curve, green, 1.5)
    add(cv.blur(curve, 0.0012), green, 0.5)
    # Barres (en haut à droite).
    fr2, _ = frame_rect(0.088, 0.050, 0.144, 0.078)
    add(fr2, cyan, 0.45)
    bars = cv.zeros()
    for k, lvl in enumerate((0.55, 0.8, 0.4, 0.95, 0.7, 0.3, 0.6)):
        x0 = 0.092 + k * 0.0073
        bars = np.maximum(bars, tg.fill(_rect_sd(cv, x0, 0.053, x0 + 0.0048, 0.053 + 0.021 * lvl), cv.px))
    add(bars, amber, 1.15)
    # Rangées de tirets (en bas à droite) : pseudo-texte illisible.
    rows = cv.zeros()
    for r in range(5):
        y = 0.040 - r * 0.0068
        x = 0.090
        while x < 0.140:
            ln = float(rng.uniform(0.003, 0.012))
            x1 = min(x + ln, 0.142)
            rows = np.maximum(rows, tg.fill(_rect_sd(cv, x, y - 0.0011, x1, y + 0.0011, 0.0004), cv.px))
            x = x1 + float(rng.uniform(0.0015, 0.003))
    add(rows, cyan, 0.75)
    # Jauge circulaire et barre de progression (en bas à gauche).
    rr = np.hypot(cv.x - 0.019, cv.y - 0.0165)
    ang = (np.degrees(np.arctan2(cv.y - 0.0165, cv.x - 0.019)) + 360.0) % 360.0
    ring = np.clip(1.0 - np.abs(rr - 0.0075) / 0.0008, 0, 1)
    add(ring * 0.35, cyan, 1.0)
    add(ring * ((ang > 200) | (ang < 60)), green, 1.3)
    prog_fr, _ = frame_rect(0.032, 0.012, 0.079, 0.021)
    add(prog_fr, cyan, 0.5)
    add(tg.fill(_rect_sd(cv, 0.0335, 0.0135, 0.0335 + 0.044 * 0.64, 0.0195, 0.0005), cv.px), green, 1.1)
    # Lignes de balayage, bord sombre (cadre de la dalle).
    scan = 0.88 + 0.12 * np.sin(cv.y / 0.0005 * np.pi) ** 2
    col = col * (scan * smoothstep(0.0008, 0.003, d))[..., None]
    L.emis = col
    # Le verre reflète un peu la lumière émise (couleur de base un peu teintée), rayures.
    L.albedo = L.albedo + 0.06 * np.clip(col, 0, 1)
    scr = cv.scratches(salt + 2, 30, (0.006, 0.04), 0.00025)
    L.paint(scr * 0.35, (0.20, 0.22, 0.23), rough=0.3)
    L.paint(stains(cv, salt + 3, 0.03, 0.12, edge=0.6) * 0.5, (0.05, 0.06, 0.065), rough=0.35)  # traces de doigts
    return _done(cv, L, [(0.002, 0.0002, 0.3)], edge_light=0.0)


def _metal(cv: Canvas, salt: int):
    """Acier brossé usé : brossage le long de U, rayures, huile, arêtes polies."""
    L = Layers(cv, (0.56, 0.56, 0.54), rough=0.42, metal=0.55)
    brushed = cv.noise(salt, 0.12, 0.0012, stretch=(0.08, 1.0))
    L.albedo = L.albedo * (tg.tone(cv, salt + 1, 0.06, 0.06) * (1.0 + 0.05 * brushed))[..., None]
    L.h += 0.00003 * brushed + 0.00002 * cv.noise(salt + 2, 0.02, 0.003)
    d = _rim(cv)
    L.h += -0.0004 * (1.0 - smoothstep(0.0, 0.004, d))
    edge = 1.0 - smoothstep(0.0008, 0.006, d)
    L.paint(edge * 0.5, (0.72, 0.72, 0.70), rough=0.25)
    scr = cv.scratches(salt + 3, 90, (0.005, 0.04), 0.0003, curve=0.1)
    L.paint(scr * 0.5, (0.74, 0.74, 0.72), rough=0.3)
    oil = stains(cv, salt + 4, 0.06, 0.15, edge=1.2, breakup=0.15)
    L.paint(oil * 0.35, (0.26, 0.25, 0.23), rough=0.6, metal=0.3)
    rust = cut(cv.noise(salt + 5, 0.01, 0.0015), 0.03, 0.2) * (1.0 - edge)
    L.paint(rust * 0.7, tg.RUST, rough=0.85, metal=0.05)
    return _done(cv, L, [(0.002, 0.0003, 0.4), (0.01, 0.001, 0.2)], edge_light=0.08)


def _copper(cv: Canvas, salt: int):
    """Spires de fil de cuivre le long de U (le fil fait le tour de la bobine), oxydation."""
    pitch = cv.tile_h / 32
    t = (cv.y % pitch) / pitch * 2.0 - 1.0
    wire = np.sqrt(np.clip(1.0 - t * t, 0.0, 1.0))
    L = Layers(cv, (0.78, 0.45, 0.25), rough=0.32, metal=0.55)
    L.albedo = L.albedo * tg.tone(cv, salt + 2, 0.03, 0.08)[..., None]
    L.h += 0.0008 * wire
    ox = cut(cv.noise(salt, 0.05, 0.003), 0.08, 0.4)
    burn = cut(cv.noise(salt + 1, 0.08, 0.01), 0.06, 0.5)
    L.paint(ox * 0.55, (0.35, 0.45, 0.36), rough=0.6, metal=0.25)
    L.paint(burn * 0.3, (0.30, 0.16, 0.10), rough=0.55)
    return _done(cv, L, [(0.0015, 0.0006, 0.7)], edge_light=0.15)


def _cables(cv: Canvas, salt: int):
    """Quatre colonnes de gaine (rouge, jaune, bleu, noir), U = tour du câble, V = longueur."""
    w = cv.tile
    colors = [(0.55, 0.10, 0.07), (0.78, 0.60, 0.12), (0.13, 0.27, 0.52), (0.09, 0.09, 0.09)]
    col_idx = np.clip((cv.x / (w / 4)).astype(np.int32), 0, 3)
    base = np.stack([np.choose(col_idx, [c[i] for c in colors]) for i in range(3)], -1).astype(np.float32)
    L = Layers(cv, (0.5, 0.5, 0.5), rough=0.45)
    L.albedo = base * tg.tone(cv, salt, 0.05, 0.05)[..., None]
    local = (cv.x % (w / 4)) / (w / 4)
    tracer = np.clip(1.0 - np.abs(local - 0.5) / 0.05, 0, 1)
    L.paint(tracer * 0.35, (0.85, 0.85, 0.80))
    L.h += 0.00005 * cv.noise(salt + 1, 0.02, 0.002, stretch=(1.0, 0.2))
    scuff = cv.scratches(salt + 2, 60, (0.005, 0.03), 0.0005, angle=(np.pi / 2, 0.3))
    L.paint(scuff * 0.4, (0.40, 0.38, 0.35), rough=0.7)
    L.paint(stains(cv, salt + 3, 0.06, 0.15, edge=0.6) * 0.35, tg.GRIME, rough=0.75)
    return _done(cv, L, [(0.002, 0.0002, 0.3)], edge_light=0.0)


def _webbing(cv: Canvas, salt: int):
    w = cv.tile
    L = Layers(cv, (0.19, 0.20, 0.17), rough=0.88)
    a = np.sin((cv.x + cv.y) / 0.0016 * np.pi)
    b = np.sin((cv.x - cv.y * 0.5) / 0.0011 * np.pi)
    L.h += 0.00009 * (0.6 * a + 0.4 * b)
    L.albedo = L.albedo * (tg.tone(cv, salt, 0.04, 0.08) * (0.95 + 0.05 * a))[..., None]
    stitch = cv.zeros()
    for u in (0.0035, w - 0.0035):
        zig = u + 0.0012 * np.sign(np.sin(cv.y / 0.0025 * np.pi))
        stitch = np.maximum(stitch, np.clip(1.0 - np.abs(cv.x - zig) / 0.0005, 0, 1))
    L.h += 0.00015 * stitch
    L.paint(stitch * 0.6, (0.36, 0.36, 0.30))
    d = _rim(cv)
    L.h += -0.0003 * (1.0 - smoothstep(0.0, 0.0015, d))
    fuzz = cut(cv.noise(salt + 1, 0.03, 0.002), 0.2, 0.4)
    L.paint(fuzz * 0.3, (0.30, 0.30, 0.26))
    L.paint(stains(cv, salt + 2, 0.06, 0.12, edge=0.6) * 0.3, tg.GRIME)
    return _done(cv, L, [(0.0015, 0.0002, 0.4)], edge_light=0.0)


def _leds(cv: Canvas, salt: int):
    """Quadrants : voyant vert (bas gauche), ambre (bas droite), rouge (haut gauche), touche
    rétroéclairée (haut droite) ; boîtier anthracite."""
    L = Layers(cv, (0.13, 0.13, 0.135), rough=0.5)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.04, 0.06)[..., None]
    q = cv.tile / 2
    for (cx, cy), color in (((q / 2, q / 2), (0.25, 1.0, 0.35)), ((1.5 * q, q / 2), (1.0, 0.60, 0.12)),
                            ((q / 2, 1.5 * q), (1.0, 0.16, 0.10))):
        dd = np.hypot(cv.x - cx, cv.y - cy)
        r = 0.0105
        core = np.clip(1.0 - dd / r, 0.0, 1.0) ** 0.5
        bezel = np.clip(1.0 - np.abs(dd - r * 1.25) / 0.0018, 0.0, 1.0)
        L.h += 0.0010 * core - 0.0003 * bezel
        L.paint((core > 0.02) * 1.0, tuple(np.array(color) * 0.55), rough=0.2)
        L.paint(bezel * 0.85, (0.05, 0.05, 0.05))
        L.emis += rgb(color) * (core * 4.0)[..., None]
    cx, cy = 1.5 * q, 1.5 * q
    key_sd = tg.sd_box(cv.x - cx, cv.y - cy, 0.0135, 0.0135, 0.003)
    key = tg.fill(key_sd, cv.px)
    L.paint(key, (0.20, 0.21, 0.22), rough=0.55, h=0.0008)
    glow = np.clip(1.0 - np.abs(key_sd + 0.0006) / 0.0009, 0, 1)
    L.emis += rgb((0.35, 1.0, 0.75)) * (glow * 2.0)[..., None]
    sym = tg.fill(tg.sd_box(cv.x - cx, cv.y - cy, 0.0065, 0.0012, 0.0006), cv.px)
    L.paint(sym, (0.25, 0.6, 0.45))
    L.emis += rgb((0.35, 1.0, 0.75)) * (sym * 2.5)[..., None]
    scr = cv.scratches(salt + 2, 30, (0.004, 0.02), 0.0003)
    L.paint(scr * 0.4, (0.40, 0.40, 0.40))
    return _done(cv, L, [(0.002, 0.0004, 0.5)], edge_light=0.0)


def _hazard(cv: Canvas, salt: int):
    L = Layers(cv, (0.80, 0.60, 0.12), rough=0.55)
    band = (np.floor((cv.x + cv.y) / 0.012) % 2 == 0).astype(np.float32)
    band = cv.blur(band, 0.0003)
    L.paint(band, (0.08, 0.08, 0.075))
    L.albedo = L.albedo * tg.tone(cv, salt, 0.05, 0.06)[..., None]
    L.h += 0.00004 * cv.noise(salt + 1, 0.02, 0.003)
    _wear(cv, L, salt + 10, 1.1, (0.40, 0.40, 0.37), tg.STEEL, 0.5, edge_w=0.012, scratches=50, grime=0.35)
    return _done(cv, L, [(0.003, 0.0005, 0.5)])


def _generators():
    return {
        "shell": lambda cv: _shell(cv, 100),
        "rubber": lambda cv: _rubber(cv, 200),
        "case_ochre": lambda cv: _case(cv, 300, (0.76, 0.54, 0.14), True, 0.9, tg.PRIMER, 0.35, 0.6),
        "case_white": lambda cv: _case(cv, 400, (0.75, 0.76, 0.73), False, 0.6, (0.48, 0.49, 0.48), 0.4, 0.0),
        "med_cross": lambda cv: _med_cross(cv, 500),
        "foil": lambda cv: _foil(cv, 600),
        "chip": lambda cv: _chip(cv, 700),
        "manifest": lambda cv: _manifest(cv, 800),
        "screen": lambda cv: _screen(cv, 900),
        "metal": lambda cv: _metal(cv, 1000),
        "copper": lambda cv: _copper(cv, 1100),
        "cables": lambda cv: _cables(cv, 1200),
        "webbing": lambda cv: _webbing(cv, 1300),
        "leds": lambda cv: _leds(cv, 1400),
        "paint_olive": lambda cv: _case(cv, 1500, (0.38, 0.40, 0.28), True, 1.1, tg.PRIMER, 0.45, 0.9),
        "hazard": lambda cv: _hazard(cv, 1600),
    }


def generate() -> dict[str, Path]:
    maps = {role: np.zeros((RES, RES, 3), np.float32) for role in ("color", "normal", "orm", "emissive")}
    for name, gen in _generators().items():
        col, row = CELLS[name]
        w, h = SIZES[name]
        # Une case = un canevas à part, à la taille de la face qu'elle habille.
        cv = Canvas(CELL_PX, w, seed=9100 + col + GRID * row, tile_h=h)
        out = gen(cv)
        ys, xs = slice(row * CELL_PX, (row + 1) * CELL_PX), slice(col * CELL_PX, (col + 1) * CELL_PX)
        for role in ("color", "normal", "orm"):
            maps[role][ys, xs] = out[role]
        maps["emissive"][ys, xs] = linear_to_srgb(out["emissive"] / EMIT_STRENGTH)
    written = {}
    for role, img in maps.items():
        path = OUT_DIR / f"{NAME}_{role}.png"
        tg.write_png(path, img)
        written[role] = path
    PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
    for role in ("color", "emissive", "normal", "orm"):
        tg.write_png(PREVIEW_DIR / f"atlas_{role}.png", maps[role])
    print(f"atlas {NAME} : {len(written)} cartes {RES} px dans {OUT_DIR}")
    return written


if __name__ == "__main__":
    generate()
