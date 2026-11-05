"""Atlas `trim_utility` (lot A de M10) : les faces dessinées des appareils du kit d'intérieur.

Pourquoi un atlas : le tableau électrique (`kit.cabinet`), le frigo (`kit.fridge`) et la plaque
de nom des casiers (`kit.locker_bank`) ont besoin de faces précises (compteur à cadran et à
disque, disjoncteurs, pictogramme de danger électrique, afficheur, porte émaillée embuée) que
les matériaux carrelables de la bibliothèque ne savent pas dessiner. Une seule texture de 1K
les porte toutes : **un seul matériau** (`sh_utility`) de plus dans une cellule, quel que soit
le nombre d'appareils.

Découpage (u, v de Blender, v = 0 en bas) :
- moitié gauche (512 × 1024 px) : `fridge_door`, la face de la porte du frigo (0,78 × 1,55 m,
  ≈ 660 px/m dans les deux sens) : émail blanc froid, nervures embouties, perles et coulures de
  condensation, givre le long du joint, éraflures près du loquet ;
- moitié droite : huit cases de 256 px (colonnes 2 et 3 de la grille 4 × 4), chacune à la
  taille physique de la face qu'elle habille (reliefs et usure à la bonne échelle une fois la
  case étirée sur la pièce, comme l'atlas `item_gear` du lot F de M9).

Émission : un seul facteur pour tout le matériau (`EMIT_STRENGTH`) ; chaque case écrit sa
luminance visée **en linéaire** (repères du jeu, AGENTS.md §4 : voyant ≈ 4, afficheur ≈ 1,5).

Aucun texte lisible, aucun logo ni insigne : tambours du compteur en traits, plaques en tirets,
pictogrammes génériques (éclair dans un triangle, flocon). La plaque de nom est **vierge** : le
jeu y projette le nom choisi par le joueur (docs/m10.md §2).

Cartes écrites (contrat du jeu) dans assets/textures/materials/ : `trim_utility_color.png`
(sRGB), `_normal.png` (OpenGL, linéaire), `_orm.png` (R occlusion, G rugosité, B métal,
linéaire), `_emissive.png` (sRGB). Aperçus dans target/art-previews/A/atlas_*.png.

    blender --background --factory-startup --python tools/blender/materials/utility_atlas.py
"""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import texgen as tg  # noqa: E402
from texgen import Canvas, cut, fill, rgb, sd_box, sd_circle, sd_poly, sd_segment, smoothstep, stains  # noqa: E402

ROOT = HERE.parents[2]
OUT_DIR = ROOT / "assets" / "textures" / "materials"
PREVIEW_DIR = ROOT / "target" / "art-previews" / "A"
NAME = "trim_utility"
RES = 1024
CELL_PX = 256
EMIT_STRENGTH = 4.0  # facteur d'émission du matériau (luminance linéaire d'un pixel blanc)

# Cases : (u0, v0, u1, v1) en pixels de l'atlas (origine en bas à gauche) et taille physique
# (largeur U, hauteur V) en mètres de la face habillée.
CELLS: dict[str, tuple[tuple[int, int, int, int], tuple[float, float]]] = {
    "fridge_door": ((0, 0, 512, 1024), (0.78, 1.55)),
    "meter": ((512, 768, 768, 1024), (0.30, 0.30)),       # compteur électrique sous verre
    "gauge": ((768, 768, 1024, 1024), (0.15, 0.15)),      # cadran rond (ampèremètre)
    "breakers": ((512, 512, 768, 768), (0.40, 0.16)),     # rangée de disjoncteurs sur rail
    "danger": ((768, 512, 1024, 768), (0.14, 0.14)),      # triangle jaune à éclair
    "nameplate": ((512, 256, 768, 512), (0.26, 0.09)),    # plaque de nom vierge, rivetée
    "display": ((768, 256, 1024, 512), (0.16, 0.07)),     # afficheur du frigo (émissif)
    "leds": ((512, 0, 768, 256), (0.06, 0.06)),           # quadrants : vert, ambre, rouge, scellé
    "vent_round": ((768, 0, 1024, 256), (0.60, 0.15)),    # grille perforée inox
}
# Sous-cases : parent et fraction (a0, b0, a1, b1) de la case.
SUBCELLS: dict[str, tuple[str, tuple[float, float, float, float]]] = {
    "led_green": ("leds", (0.0, 0.0, 0.5, 0.5)),
    "led_amber": ("leds", (0.5, 0.0, 1.0, 0.5)),
    "led_red": ("leds", (0.0, 0.5, 0.5, 1.0)),
    "seal": ("leds", (0.5, 0.5, 1.0, 1.0)),
}


def cell_rect(name: str, pad_px: float = 3.0) -> tuple[float, float, float, float]:
    """(u0, v0, u1, v1) d'une case ou d'une sous-case en UV, rognée de `pad_px` texels (le
    filtrage et les premières mipmaps ne débordent pas sur la voisine)."""
    if name in SUBCELLS:
        parent, (a0, b0, a1, b1) = SUBCELLS[name]
        (x0, y0, x1, y1), _ = CELLS[parent]
        x0, x1 = x0 + (x1 - x0) * a0, x0 + (x1 - x0) * a1
        y0, y1 = y0 + (y1 - y0) * b0, y0 + (y1 - y0) * b1
    else:
        (x0, y0, x1, y1), _ = CELLS[name]
    return ((x0 + pad_px) / RES, (y0 + pad_px) / RES, (x1 - pad_px) / RES, (y1 - pad_px) / RES)


def cell_size(name: str) -> tuple[float, float]:
    """Taille physique (m) d'une case ou d'une sous-case : (largeur U, hauteur V)."""
    if name in SUBCELLS:
        parent, (a0, b0, a1, b1) = SUBCELLS[name]
        w, h = CELLS[parent][1]
        return (w * (a1 - a0), h * (b1 - b0))
    return CELLS[name][1]


def linear_to_srgb(c):
    c = np.clip(np.asarray(c, dtype=np.float32), 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * np.power(c, 1.0 / 2.4) - 0.055).astype(np.float32)


# --- Briques -----------------------------------------------------------------------------------


class Layers:
    """Couches d'une case : hauteur (m), couleur sRGB, rugosité, métal, émission linéaire."""

    def __init__(self, cv: Canvas, color, rough: float = 0.6, metal: float = 0.0):
        self.cv = cv
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

    def glow(self, mask, color, lum: float):
        self.emis = self.emis + rgb(color) * (mask * lum)[..., None]


def _rim(cv: Canvas) -> np.ndarray:
    """Distance (m) au bord de la case : les arêtes de la pièce."""
    return np.minimum(np.minimum(cv.x, cv.tile - cv.x), np.minimum(cv.y, cv.tile_h - cv.y))


def _rect(cv: Canvas, x0, y0, x1, y1, r=0.0):
    return sd_box(cv.x - (x0 + x1) / 2, cv.y - (y0 + y1) / 2, (x1 - x0) / 2, (y1 - y0) / 2, r)


def _dome(cv: Canvas, h: np.ndarray, x: float, y: float, r: float, height: float) -> None:
    def fn(gx, gy):
        d = np.hypot(gx, gy)
        return height * np.sqrt(np.clip(1.0 - (d / r) ** 2, 0.0, 1.0))

    cv.stamp(h, x, y, r * 1.5, r * 1.5, fn)


def _screws(cv: Canvas, pts, r: float = 0.003) -> tuple[np.ndarray, np.ndarray]:
    h = cv.zeros()
    for i, (x, y) in enumerate(pts):
        cv.stamp(h, x, y, r * 1.6, r * 1.6, lambda gx, gy, a=0.4 + i * 0.85: tg.screw(gx, gy, r, a, 0.0007))
    return h, np.clip(cv.blur((h > 0.00005).astype(np.float32), cv.px) * 1.5, 0.0, 1.0)


def _wear(cv: Canvas, L: Layers, salt: int, wear: float, under, bare, bare_metal: float, edge_w: float = 0.012,
          scratches: int = 40, grime: float = 0.3):
    """Usure d'une peinture : écaillures sur les arêtes (sous-couche puis matière nue), rayures,
    crasse dans les coins."""
    d = _rim(cv)
    edge = 1.0 - smoothstep(0.0015, edge_w, d)
    chips = tg.chip_field(cv, salt, edge * wear, 0.004, edge_weight=3.0)
    under_m = cut(chips, 0.08 * wear)
    bare_m = cut(chips, 0.035 * wear)
    L.h = L.h - 0.00018 * under_m - 0.00008 * bare_m
    L.paint(under_m, under, rough=0.75)
    L.paint(bare_m, bare, rough=0.38, metal=bare_metal)
    scr = cv.scratches(salt + 5, scratches, (0.006, 0.04), 0.0004, curve=0.05)
    L.paint(scr * 0.3, tuple((np.asarray(under) + np.asarray(bare)) / 2))
    g = stains(cv, salt + 7, min(cv.tile, cv.tile_h) * 0.5, 0.12, edge=1.2, breakup=0.15) * 0.5
    corner = (1.0 - smoothstep(0.0, 0.01, d)) * 0.5
    L.paint(np.clip(g * grime + corner * grime, 0.0, 0.8), tg.GRIME, rough=0.85)


def _glass_glare(cv: Canvas, L: Layers, inside, strength: float = 0.16):
    """Reflet de vitre cuit (bande diagonale douce) : la face se lit « sous verre » même sans
    verre transparent (pas de tri de transparence, rien à charger derrière)."""
    t = (cv.x / cv.tile - cv.y / cv.tile_h)
    band = np.exp(-((t - 0.25) / 0.09) ** 2) + 0.5 * np.exp(-((t - 0.05) / 0.035) ** 2)
    L.albedo = L.albedo + strength * (band * inside)[..., None]
    L.rough = tg.lerp(L.rough, 0.12, inside * 0.8)


def _done(cv: Canvas, L: Layers, ao_layers, edge_light: float = 0.06):
    out = tg.finish(cv, L.h, np.clip(L.albedo, 0.0, 1.0), L.rough, L.metal, ao_layers, edge_light=edge_light)
    out["emissive"] = L.emis
    return out


# --- Cases -------------------------------------------------------------------------------------

_BAKELITE = (0.07, 0.068, 0.065)
_DIAL = (0.80, 0.78, 0.71)


def _meter(cv: Canvas, salt: int):
    """Compteur électrique sous verre : boîtier de bakélite noire, plaque crème, tambours (traits
    blancs, pas de chiffres), fenêtre du disque d'aluminium et son repère, plaque signalétique en
    tirets, voyant d'impulsion rouge, vis de scellé ; reflet de vitre cuit."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, _BAKELITE, rough=0.42)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.05, 0.08)[..., None]
    d = _rim(cv)
    L.h += 0.0025 * smoothstep(0.0, 0.012, d)  # boîtier bombé
    # Plaque crème en retrait (fenêtre du boîtier).
    plate_sd = _rect(cv, 0.035, 0.035, w - 0.035, hh - 0.035, 0.012)
    plate = fill(plate_sd, cv.px)
    L.paint(plate, _DIAL, rough=0.6, h=-0.003)
    L.albedo = L.albedo * (1.0 - 0.06 * plate * np.clip(cv.noise(salt + 1, 0.06, 0.004), -1, 1))[..., None]
    lip = np.clip(1.0 - np.abs(plate_sd) / 0.0015, 0.0, 1.0)
    L.paint(lip, (0.02, 0.02, 0.02), rough=0.4)
    # Tambours du totalisateur : six fenêtres noires, traits blancs (tambours entre deux crans).
    y_c = 0.205
    x0 = 0.075
    for i in range(6):
        cx = x0 + i * 0.025
        win = fill(_rect(cv, cx - 0.0105, y_c - 0.015, cx + 0.0105, y_c + 0.015, 0.0015), cv.px)
        drum = (0.95, 0.94, 0.9) if i < 5 else (0.85, 0.16, 0.10)
        L.paint(win, (0.03, 0.03, 0.03), rough=0.35, h=-0.0006)
        off = ((i * 0.37 + salt * 0.01) % 1.0) * 0.012 - 0.006
        for k in (-1, 0, 1):
            ty = y_c + off + k * 0.012
            bar = fill(sd_box(cv.x - cx, cv.y - ty, 0.006, 0.0018, 0.0006), cv.px) * win
            L.paint(bar, drum, rough=0.4)
    frame = np.clip(1.0 - np.abs(_rect(cv, x0 - 0.016, y_c - 0.02, x0 + 5 * 0.025 + 0.016, y_c + 0.02, 0.003)) / 0.0012, 0, 1)
    L.paint(frame, (0.15, 0.15, 0.15))
    # Fenêtre du disque : fente noire, tranche d'aluminium brossé, repère noir et rouge.
    slot_sd = _rect(cv, 0.06, 0.125, w - 0.06, 0.15, 0.004)
    slot = fill(slot_sd, cv.px)
    L.paint(slot, (0.025, 0.025, 0.025), rough=0.4, h=-0.0015)
    disc = fill(_rect(cv, 0.065, 0.131, w - 0.065, 0.144, 0.002), cv.px)
    brushed = 0.5 + 0.5 * cv.noise(salt + 3, 0.05, 0.0008, stretch=(20, 1))
    L.albedo = tg.lerp(L.albedo, rgb((0.62, 0.62, 0.60)) * (0.85 + 0.25 * brushed)[..., None], disc)
    L.metal = tg.lerp(L.metal, 0.5, disc)
    L.rough = tg.lerp(L.rough, 0.3, disc)
    mark = fill(sd_box(cv.x - 0.19, cv.y - 0.1375, 0.006, 0.0068), cv.px) * disc
    L.paint(mark, (0.55, 0.06, 0.04))
    # Flèche de sens de rotation (pictogramme générique).
    arrow = fill(np.minimum(sd_box(cv.x - 0.115, cv.y - 0.163, 0.018, 0.0012), sd_poly(cv.x - 0.135, cv.y - 0.163, [(0.0, -0.004), (0.007, 0.0), (0.0, 0.004)])), cv.px)
    L.paint(arrow * plate, (0.12, 0.12, 0.12))
    # Plaque signalétique : rangées de tirets (pseudo-texte illisible), cadre fin.
    for r_, (y, n) in enumerate(((0.098, 7), (0.086, 5), (0.074, 8), (0.062, 4))):
        rng = cv.rng(salt + 10 + r_)
        x = 0.06
        for _ in range(n):
            ln = float(rng.uniform(0.006, 0.02))
            dash = fill(sd_box(cv.x - (x + ln / 2), cv.y - y, ln / 2, 0.0014, 0.0005), cv.px)
            L.paint(dash, (0.18, 0.18, 0.17))
            x += ln + 0.005
            if x > 0.2:
                break
    box_sd = _rect(cv, 0.05, 0.052, 0.21, 0.108, 0.002)
    L.paint(np.clip(1.0 - np.abs(box_sd) / 0.0009, 0, 1), (0.25, 0.25, 0.24))
    # Voyant d'impulsion (rouge, émissif) et pastille de classe (verte, mate).
    led_d = np.hypot(cv.x - 0.24, cv.y - 0.083)
    core = np.clip(1.0 - led_d / 0.0055, 0.0, 1.0) ** 0.5
    L.paint((led_d < 0.0075) * 1.0, (0.04, 0.04, 0.04))
    L.paint(core, (0.6, 0.1, 0.06), rough=0.2)
    L.glow(core, (1.0, 0.16, 0.08), 3.0)
    L.paint(fill(sd_circle(cv.x - 0.24, cv.y - 0.058, 0.007), cv.px), (0.22, 0.42, 0.22))
    # Vis de scellé (deux), bombées, dans le boîtier.
    sh, smask = _screws(cv, [(0.02, hh - 0.02), (w - 0.02, 0.02)], r=0.0045)
    L.h += sh
    L.paint(smask, (0.55, 0.53, 0.48), rough=0.4, metal=0.45)
    _glass_glare(cv, L, plate, 0.14)
    L.paint(np.clip(cv.scratches(salt + 20, 25, (0.01, 0.05), 0.0004) * (1 - plate), 0, 1) * 0.4, (0.3, 0.3, 0.3))
    return _done(cv, L, [(0.004, 0.0008, 0.5), (0.012, 0.002, 0.3)])


def _gauge(cv: Canvas, salt: int):
    """Cadran rond : lunette noire, cadran blanc, graduations sur 240°, arc vert / jaune / rouge,
    aiguille noire, cache central ; reflet de vitre."""
    w, hh = cv.tile, cv.tile_h
    cx, cy = w / 2, hh / 2
    L = Layers(cv, (0.16, 0.165, 0.17), rough=0.5)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.05, 0.06)[..., None]
    dd = np.hypot(cv.x - cx, cv.y - cy)
    ang = np.degrees(np.arctan2(cv.y - cy, cv.x - cx))
    R = 0.062
    bezel = fill(dd - R, cv.px)
    L.paint(bezel, _BAKELITE, rough=0.35, h=0.002)
    face = fill(dd - (R - 0.008), cv.px)
    L.paint(face, (0.86, 0.85, 0.80), rough=0.55, h=-0.001)
    # Arc de 240° (de 210° à −30°), sens horaire : t = 0 à gauche en bas, 1 à droite en bas.
    t = ((210.0 - ang) % 360.0) / 240.0
    in_arc = (t >= 0.0) & (t <= 1.0)
    ring = (np.abs(dd - 0.043) < 0.0035) & in_arc
    col = np.where((t < 0.6)[..., None], rgb((0.18, 0.55, 0.22)), np.where((t < 0.82)[..., None], rgb((0.85, 0.66, 0.12)), rgb((0.75, 0.12, 0.08))))
    L.albedo = tg.lerp(L.albedo, col, ring * face)
    for k in range(25):
        a = np.radians(210.0 - 240.0 * k / 24)
        major = k % 4 == 0
        r0, r1 = (0.031, 0.047) if major else (0.036, 0.047)
        tick = np.clip(1.0 - sd_segment(cv.x - cx, cv.y - cy, r0 * np.cos(a), r0 * np.sin(a), r1 * np.cos(a), r1 * np.sin(a)) / (0.0011 if major else 0.0006), 0, 1)
        L.paint(tick * face, (0.05, 0.05, 0.05))
    a = np.radians(210.0 - 240.0 * 0.47)
    needle = np.clip(1.0 - sd_segment(cv.x - cx, cv.y - cy, -0.008 * np.cos(a), -0.008 * np.sin(a), 0.048 * np.cos(a), 0.048 * np.sin(a)) / 0.0012, 0, 1)
    L.paint(needle * face, (0.05, 0.05, 0.05), h=0.0003)
    L.paint(fill(dd - 0.006, cv.px), (0.1, 0.1, 0.1), rough=0.3, h=0.0008)
    _glass_glare(cv, L, face, 0.16)
    _wear(cv, L, salt + 30, 0.6, (0.3, 0.3, 0.3), tg.STEEL, 0.4)
    return _done(cv, L, [(0.003, 0.0006, 0.5)])


def _breakers(cv: Canvas, salt: int):
    """Rangée de disjoncteurs modulaires sur rail, sous un capot gris : modules gris clair,
    leviers noirs (quelques-uns abaissés), voyants d'état rouges et verts, interrupteur général à
    levier rouge, porte-étiquettes en tirets sous la rangée."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.36, 0.37, 0.36), rough=0.6)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.08, 0.05)[..., None]
    rng = cv.rng(salt)
    # Ouverture du capot (les modules dépassent du capot).
    win_sd = _rect(cv, 0.02, 0.045, w - 0.02, 0.135, 0.003)
    win = fill(win_sd, cv.px)
    L.paint(win, (0.06, 0.06, 0.06), h=-0.002)
    L.paint(np.clip(1.0 - np.abs(win_sd) / 0.0012, 0, 1), (0.2, 0.2, 0.2))
    x = 0.028
    widths = [0.054, 0.018, 0.018, 0.018, 0.018, 0.018, 0.018, 0.036, 0.018, 0.018, 0.018, 0.018, 0.018, 0.018, 0.018, 0.018]
    k = 0
    for wd in widths:
        if x + wd > w - 0.026:
            break
        x0, x1 = x + 0.0006, x + wd - 0.0006
        body_sd = _rect(cv, x0, 0.048, x1, 0.132, 0.0012)
        body = fill(body_sd, cv.px)
        main = wd > 0.05
        tone_c = (0.80, 0.80, 0.77) if not main else (0.72, 0.72, 0.70)
        L.paint(body, tone_c, rough=0.55, h=0.003)
        mx = (x0 + x1) / 2
        # Fenêtre du levier et levier (haut = enclenché, bas = déclenché).
        lw = (x1 - x0) * (0.55 if not main else 0.6)
        slot = fill(_rect(cv, mx - lw / 2, 0.072, mx + lw / 2, 0.108, 0.001), cv.px)
        L.paint(slot, (0.12, 0.12, 0.12), h=-0.0015)
        up = rng.random() > 0.18 or main
        ly0, ly1 = (0.090, 0.106) if up else (0.074, 0.090)
        lever = fill(_rect(cv, mx - lw / 2 + 0.0012, ly0, mx + lw / 2 - 0.0012, ly1, 0.001), cv.px)
        L.paint(lever, (0.65, 0.1, 0.07) if main else (0.08, 0.08, 0.08), rough=0.4, h=0.0025)
        # Voyant d'état (vert enclenché, rouge déclenché), émissif faible.
        ind = fill(_rect(cv, mx - 0.004, 0.115, mx + 0.004, 0.121, 0.0008), cv.px)
        c = (0.25, 0.9, 0.35) if up else (1.0, 0.18, 0.1)
        L.paint(ind, tuple(np.array(c) * 0.5), rough=0.25)
        L.glow(ind, c, 1.6)
        # Repère de calibre : deux tirets sombres (pas de chiffres).
        for j in range(2):
            dash = fill(sd_box(cv.x - (mx - 0.003 + j * 0.006), cv.y - 0.06, 0.002, 0.0012), cv.px)
            L.paint(dash * body, (0.25, 0.25, 0.25))
        L.paint(np.clip(1.0 - np.abs(body_sd) / 0.0006, 0, 1) * 0.6, (0.3, 0.3, 0.3))
        x += wd
        k += 1
    # Porte-étiquettes sous la rangée : bande claire à tirets, et vis du capot.
    strip = fill(_rect(cv, 0.03, 0.018, w - 0.03, 0.036, 0.001), cv.px)
    L.paint(strip, (0.72, 0.72, 0.68), rough=0.5, h=0.0004)
    xs = 0.036
    while xs < w - 0.05:
        ln = float(rng.uniform(0.008, 0.018))
        L.paint(fill(sd_box(cv.x - (xs + ln / 2), cv.y - 0.027, ln / 2, 0.0013, 0.0005), cv.px), (0.2, 0.2, 0.2))
        xs += ln + float(rng.uniform(0.008, 0.02))
    sh, smask = _screws(cv, [(0.012, 0.012), (w - 0.012, 0.012), (0.012, hh - 0.012), (w - 0.012, hh - 0.012)], r=0.0035)
    L.h += sh
    L.paint(smask, (0.5, 0.5, 0.48), metal=0.45)
    _wear(cv, L, salt + 40, 0.9, tg.PRIMER, tg.STEEL, 0.45)
    return _done(cv, L, [(0.003, 0.0008, 0.5), (0.01, 0.002, 0.3)])


def _danger(cv: Canvas, salt: int):
    """Pictogramme de danger électrique : triangle jaune à bord noir et éclair noir, sur plaque
    grise ; usé (aucun texte)."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.32, 0.32, 0.31), rough=0.6)
    cx, cy = w / 2, hh / 2 - 0.004
    s = 0.058
    tri = [(-s, -0.86 * s * 0.66), (s, -0.86 * s * 0.66), (0.0, 0.86 * s * 1.34)]
    outer = sd_poly(cv.x - cx, cv.y - cy, tri)
    L.paint(fill(outer + 0.002, cv.px), (0.04, 0.04, 0.04), rough=0.5, h=0.0004)
    inner = fill(outer + 0.0085, cv.px)
    L.paint(inner, (0.92, 0.70, 0.08), rough=0.5)
    # Éclair : deux parallélogrammes et une pointe.
    bolt = [
        [(0.004, 0.044), (-0.012, 0.004), (0.002, 0.004), (0.016, 0.044)],
        [(-0.004, 0.012), (0.012, 0.012), (-0.002, -0.024), (-0.014, -0.024)],
    ]
    bsd = None
    for poly in bolt:
        sd = sd_poly(cv.x - cx, cv.y - cy, _ccw(poly))
        bsd = sd if bsd is None else np.minimum(bsd, sd)
    tip = sd_poly(cv.x - cx, cv.y - cy, _ccw([(-0.022, -0.012), (-0.002, -0.022), (-0.018, -0.04)]))
    bsd = np.minimum(bsd, tip)
    L.paint(fill(bsd, cv.px) * inner, (0.04, 0.04, 0.04))
    L.albedo = L.albedo * tg.tone(cv, salt, 0.04, 0.05)[..., None]
    _wear(cv, L, salt + 50, 1.0, (0.45, 0.44, 0.40), tg.STEEL, 0.4, scratches=30, grime=0.35)
    sh, smask = _screws(cv, [(0.012, 0.012), (w - 0.012, 0.012)], r=0.0035)
    L.h += sh
    L.paint(smask, (0.5, 0.5, 0.48), metal=0.45)
    return _done(cv, L, [(0.003, 0.0006, 0.5)])


def _ccw(poly):
    """Sommets dans le sens trigonométrique (aire signée positive), comme le veut `sd_poly`."""
    a = sum(x0 * y1 - x1 * y0 for (x0, y0), (x1, y1) in zip(poly, poly[1:] + poly[:1]))
    return poly if a > 0 else poly[::-1]


def _nameplate(cv: Canvas, salt: int):
    """Plaque de nom vierge : aluminium anodisé clair, chant biseauté, quatre rivets, filet
    embouti autour du champ d'écriture ; rayures fines, crasse dans les angles. Aucun glyphe :
    le jeu y projette le nom choisi par le joueur."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.84, 0.83, 0.78), rough=0.42, metal=0.25)
    brushed = cv.noise(salt, 0.3, 0.002, stretch=(30, 1))
    L.albedo = L.albedo * (1.0 + 0.012 * brushed)[..., None] * tg.tone(cv, salt + 1, 0.1, 0.015)[..., None]
    d = _rim(cv)
    L.h += 0.0012 * smoothstep(0.0, 0.004, d)
    L.paint(1.0 - smoothstep(0.0, 0.0035, d), (0.62, 0.61, 0.57), rough=0.35)
    groove_sd = _rect(cv, 0.018, 0.012, w - 0.018, hh - 0.012, 0.004)
    groove = np.clip(1.0 - np.abs(groove_sd) / 0.0008, 0, 1)
    L.h -= 0.0003 * groove
    L.paint(groove * 0.5, (0.55, 0.54, 0.5))
    for x, y in ((0.009, 0.009), (w - 0.009, 0.009), (0.009, hh - 0.009), (w - 0.009, hh - 0.009)):
        _dome(cv, L.h, x, y, 0.0042, 0.0012)
        rv = fill(np.hypot(cv.x - x, cv.y - y) - 0.0042, cv.px)
        L.paint(rv, (0.58, 0.57, 0.53), rough=0.3, metal=0.5)
    scr = cv.scratches(salt + 3, 40, (0.005, 0.03), 0.00025, angle=(0.1, 0.4))
    L.paint(scr * 0.35, (0.95, 0.94, 0.9))
    L.paint(np.clip((1.0 - smoothstep(0.0, 0.008, d)) * 0.25 * (0.6 + 0.4 * cv.noise(salt + 4, 0.03, 0.003)), 0, 0.35), tg.GRIME, rough=0.8)
    return _done(cv, L, [(0.002, 0.0004, 0.4)], edge_light=0.04)


def _display(cv: Canvas, salt: int):
    """Afficheur du frigo : verre noir sous lunette, pictogramme flocon, jauge à barres et deux
    pavés d'état en cyan (aucun chiffre)."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.12, 0.12, 0.125), rough=0.5)
    glass_sd = _rect(cv, 0.008, 0.008, w - 0.008, hh - 0.008, 0.003)
    glass = fill(glass_sd, cv.px)
    L.paint(glass, (0.015, 0.02, 0.025), rough=0.12, h=-0.0008)
    cyan = (0.45, 0.88, 1.0)
    # Fond très faible.
    L.glow(glass, cyan, 0.05)
    # Flocon (trois traits, petites branches).
    fx, fy, r = 0.03, hh / 2, 0.017
    flake = cv.zeros()
    for k in range(3):
        a = np.pi / 2 + k * np.pi / 3
        ca, sa = np.cos(a), np.sin(a)
        flake = np.maximum(flake, np.clip(1.0 - sd_segment(cv.x - fx, cv.y - fy, -r * ca, -r * sa, r * ca, r * sa) / 0.0011, 0, 1))
        for s in (-1, 1):
            bx, by = fx + s * 0.6 * r * ca, fy + s * 0.6 * r * sa
            for b in (-1, 1):
                a2 = a + b * np.radians(40) + (0 if s > 0 else np.pi)
                flake = np.maximum(flake, np.clip(1.0 - sd_segment(cv.x - bx, cv.y - by, 0, 0, 0.006 * np.cos(a2), 0.006 * np.sin(a2)) / 0.0009, 0, 1))
    L.paint(flake * glass, (0.3, 0.55, 0.65))
    L.glow(flake * glass, cyan, 1.6)
    # Jauge à barres (4 allumées sur 6, hauteur croissante).
    for i in range(6):
        x = 0.06 + i * 0.011
        hgt = 0.012 + i * 0.004
        bar = fill(_rect(cv, x, 0.017, x + 0.007, 0.017 + hgt, 0.0006), cv.px)
        on = i < 4
        L.paint(bar, (0.25, 0.5, 0.6) if on else (0.06, 0.08, 0.09))
        L.glow(bar, cyan, 1.5 if on else 0.12)
    # Deux pavés d'état : vert (froid atteint) et éteint.
    for i, (c, lum) in enumerate((((0.4, 1.0, 0.55), 1.4), ((1.0, 0.5, 0.15), 0.1))):
        x = 0.135 + i * 0.0
        y = 0.044 - i * 0.02
        pad = fill(_rect(cv, x - 0.006, y - 0.004, x + 0.006, y + 0.004, 0.001), cv.px)
        L.paint(pad, tuple(np.array(c) * 0.4))
        L.glow(pad, c, lum)
    _glass_glare(cv, L, glass, 0.05)
    return _done(cv, L, [(0.002, 0.0004, 0.5)], edge_light=0.0)


def _leds(cv: Canvas, salt: int):
    """Quadrants : voyant vert (bas gauche), ambre (bas droite), rouge (haut gauche) ; scellé de
    plomb sur son fil (haut droite), qu'on pose sur la fenêtre du compteur."""
    L = Layers(cv, (0.12, 0.12, 0.125), rough=0.5)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.04, 0.06)[..., None]
    q = cv.tile / 2
    for (cx, cy), color in (((q / 2, q / 2), (0.25, 1.0, 0.35)), ((1.5 * q, q / 2), (1.0, 0.60, 0.12)), ((q / 2, 1.5 * q), (1.0, 0.16, 0.10))):
        dd = np.hypot(cv.x - cx, cv.y - cy)
        r = 0.0095
        core = np.clip(1.0 - dd / r, 0.0, 1.0) ** 0.5
        bezel = np.clip(1.0 - np.abs(dd - r * 1.25) / 0.0018, 0.0, 1.0)
        L.h += 0.0010 * core - 0.0003 * bezel
        L.paint((core > 0.02) * 1.0, tuple(np.array(color) * 0.55), rough=0.2)
        L.paint(bezel * 0.85, (0.05, 0.05, 0.05))
        L.glow(core, color, 4.0)
    # Scellé : pastille de plomb gris mat, frappée d'un anneau, fil torsadé qui la traverse.
    cx, cy = 1.5 * q, 1.5 * q
    dd = np.hypot(cv.x - cx, cv.y - cy)
    seal = fill(dd - 0.0085, cv.px)
    wire = np.clip(1.0 - np.abs(cv.y - cy - 0.0015 * np.sin((cv.x - cx) / 0.0025)) / 0.0007, 0, 1) * (1 - seal) * (cv.x > q + 0.002)
    L.paint(wire, (0.62, 0.58, 0.45), rough=0.35, metal=0.5, h=0.0003)
    L.paint(seal, (0.42, 0.43, 0.44), rough=0.7, metal=0.3, h=0.0012)
    ring = np.clip(1.0 - np.abs(dd - 0.0055) / 0.0006, 0, 1) * seal
    L.h -= 0.0002 * ring
    L.paint(ring * 0.6, (0.3, 0.3, 0.31))
    return _done(cv, L, [(0.002, 0.0004, 0.5)], edge_light=0.0)


def _vent_round(cv: Canvas, salt: int):
    """Grille perforée inox (trous ronds en quinconce), cadre lisse, deux vis ; poussière dans
    les trous du bas, givre léger sur le haut (grille du groupe froid)."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.56, 0.56, 0.55), rough=0.38, metal=0.45)
    brushed = cv.noise(salt, 0.2, 0.001, stretch=(25, 1))
    L.albedo = L.albedo * (1.0 + 0.05 * brushed)[..., None]
    field = fill(_rect(cv, 0.03, 0.022, w - 0.03, hh - 0.022, 0.004), cv.px)
    pitch = 0.012
    gy = cv.y / pitch
    row = np.floor(gy)
    gx = cv.x / pitch + 0.5 * (row % 2)
    dx = (gx - np.floor(gx) - 0.5) * pitch
    dy = (gy - row - 0.5) * pitch
    holes = fill(np.hypot(dx, dy) - 0.0036, cv.px) * field
    L.paint(holes, (0.02, 0.02, 0.02), rough=0.9, metal=0.0, h=-0.004)
    rim = np.clip(1.0 - np.abs(np.hypot(dx, dy) - 0.0038) / 0.0006, 0, 1) * field
    L.paint(rim * 0.4, (0.75, 0.75, 0.73))
    dust = (1.0 - smoothstep(0.0, hh * 0.6, cv.y)) * (1 - holes)
    L.paint(np.clip(dust * 0.25 * (0.6 + 0.4 * cv.noise(salt + 2, 0.05, 0.003)), 0, 0.4), tg.DUST, rough=0.8)
    frost = smoothstep(hh * 0.55, hh, cv.y) * np.clip(0.5 + 0.5 * cv.noise(salt + 3, 0.04, 0.002), 0, 1)
    L.paint(frost * 0.25 * (1 - holes), (0.86, 0.9, 0.93), rough=0.75, metal=0.0)
    sh, smask = _screws(cv, [(0.015, hh / 2), (w - 0.015, hh / 2)], r=0.004)
    L.h += sh
    L.paint(smask, (0.6, 0.6, 0.58))
    _wear(cv, L, salt + 60, 0.5, (0.45, 0.45, 0.44), tg.STEEL, 0.5, scratches=30, grime=0.25)
    return _done(cv, L, [(0.003, 0.001, 0.6), (0.01, 0.002, 0.3)])


# Vitre de la porte du frigo, en fractions de la face (x0, y0, x1, y1) : `kit.fridge` pose son
# cadre au même endroit.
FRIDGE_WINDOW = (0.13, 0.5, 0.87, 0.93)


def _fridge_window(cv: Canvas, L: Layers, salt: int):
    """Vitre givrée de la porte (armoire réfrigérée vitrée) : derrière, trois clayettes éclairées
    d'en haut en blanc froid, des rangées de rations (barquettes olive, sachets, boîtes) et de
    bouteilles, floues (verre embué) ; givre et buée sur le pourtour du verre, reflet. Émission
    faible (≈ 0,05 à 0,5) : l'intérieur se lit allumé dans la pénombre sans éblouir."""
    w, hh = cv.tile, cv.tile_h
    fx0, fy0, fx1, fy1 = FRIDGE_WINDOW
    x0, y0, x1, y1 = fx0 * w, fy0 * hh, fx1 * w, fy1 * hh
    win_sd = _rect(cv, x0, y0, x1, y1, 0.012)
    win = fill(win_sd, cv.px)
    rng = cv.rng(salt)
    # Intérieur : fond bleu acier, éclairé du haut.
    gy = np.clip((cv.y - y0) / (y1 - y0), 0, 1)
    inside = rgb((0.10, 0.13, 0.16)) * (0.6 + 0.6 * gy)[..., None]
    lum = 0.05 + 0.16 * gy ** 2
    shelves = [y0 + (y1 - y0) * f for f in (0.05, 0.36, 0.67)]
    items = cv.zeros()
    item_col = cv.zeros(3)
    palette_items = [(0.42, 0.45, 0.30), (0.62, 0.55, 0.36), (0.55, 0.57, 0.58), (0.35, 0.38, 0.27), (0.70, 0.66, 0.55), (0.55, 0.16, 0.10)]
    for k, sy_ in enumerate(shelves):
        bar = np.clip(1.0 - np.abs(cv.y - sy_) / 0.004, 0, 1) * (cv.x > x0) * (cv.x < x1)
        inside = tg.lerp(inside, rgb((0.75, 0.8, 0.84)), bar)
        lum = lum + 0.35 * bar
        x = x0 + 0.015
        top = (shelves[k + 1] - 0.02) if k + 1 < len(shelves) else y1 - 0.05
        while x < x1 - 0.03:
            kind = rng.random()
            if kind < 0.3:  # bouteille
                bw_, bh_ = 0.045, min(top - sy_ - 0.01, float(rng.uniform(0.16, 0.22)))
            else:  # barquette, boîte, sachet
                bw_, bh_ = float(rng.uniform(0.06, 0.12)), min(top - sy_ - 0.01, float(rng.uniform(0.05, 0.14)))
            if x + bw_ > x1 - 0.015:
                break
            m_ = fill(_rect(cv, x, sy_ + 0.004, x + bw_, sy_ + 0.004 + bh_, 0.012 if kind < 0.3 else 0.004), cv.px)
            colr = palette_items[int(rng.integers(0, len(palette_items)))]
            if kind < 0.3:
                colr = (0.35, 0.55, 0.6) if rng.random() < 0.6 else (0.6, 0.62, 0.6)
            shade = 0.7 + 0.3 * np.clip((cv.y - sy_) / max(bh_, 1e-3), 0, 1)
            item_col = tg.lerp(item_col, rgb(colr) * shade[..., None], m_)
            items = np.maximum(items, m_)
            x += bw_ + float(rng.uniform(0.005, 0.02))
    inside = tg.lerp(inside, item_col, items)
    lum = lum * (1.0 - 0.5 * items) + 0.08 * items
    # Verre embué : tout l'intérieur flou, voile clair.
    inside = cv.blur(inside, 0.003)
    lum = cv.blur(lum, 0.003)
    fog = 0.22 + 0.06 * cv.noise(salt + 1, 0.4, 0.06)
    inside = tg.lerp(inside, rgb((0.55, 0.62, 0.68)), fog)
    # Givre et buée au bord du verre.
    d_in = -win_sd
    frost = (1.0 - smoothstep(0.0, 0.045 + 0.012 * cv.noise(salt + 2, 0.1, 0.01), d_in)) * np.clip(0.9 + 0.1 * cv.noise(salt + 3, 0.05, 0.003), 0, 1)
    frost = cv.blur(np.clip(frost, 0, 1), 0.002)
    inside = tg.lerp(inside, rgb((0.9, 0.94, 0.97)), np.clip(frost, 0, 1) * 0.85)
    lum = lum * (1.0 - 0.6 * frost)
    L.albedo = tg.lerp(L.albedo, inside, win)
    L.rough = tg.lerp(L.rough, 0.1 + 0.5 * np.clip(frost, 0, 1), win)
    L.metal = tg.lerp(L.metal, 0.0, win)
    L.h = L.h * (1.0 - win) - 0.001 * win
    L.glow(win * lum, (0.82, 0.92, 1.0), 1.0)
    _glass_glare(cv, L, win * (1.0 - np.clip(frost, 0, 1)), 0.08)


def _fridge_door(cv: Canvas, salt: int):
    """Face de la porte du frigo (U = largeur, charnières à gauche, loquet à droite ; V = haut) :
    émail blanc froid, vitre givrée sur l'intérieur éclairé (`_fridge_window`), deux nervures
    embouties, perles de condensation et coulures depuis le haut, givre le long du joint,
    éraflures et mains sales près du loquet, crasse en bas."""
    w, hh = cv.tile, cv.tile_h
    L = Layers(cv, (0.80, 0.82, 0.83), rough=0.38)
    L.albedo = L.albedo * tg.tone(cv, salt, 0.4, 0.035)[..., None]
    L.h += 0.00003 * cv.noise(salt + 1, 0.05, 0.004)
    d = _rim(cv)
    # Bord tombé de la porte (arrondi) et deux nervures embouties horizontales.
    L.h += 0.003 * smoothstep(0.0, 0.02, d)
    for y in (hh * 0.2, hh * 0.38):
        rib = np.exp(-((cv.y - y) / 0.012) ** 2) * fill(_rect(cv, 0.06, 0, w - 0.06, hh, 0.0), cv.px)
        L.h += 0.0025 * rib
    # Cadre embouti du champ central.
    frame_sd = _rect(cv, 0.05, 0.08, w - 0.05, hh - 0.08, 0.03)
    L.h -= 0.0008 * np.clip(1.0 - np.abs(frame_sd) / 0.004, 0, 1)
    # Givre le long du joint (tout le pourtour), plus marqué en haut.
    frost_w = 0.035 + 0.02 * smoothstep(hh * 0.6, hh, cv.y)
    frost = (1.0 - smoothstep(0.0, frost_w, d)) * np.clip(0.55 + 0.45 * cv.noise(salt + 2, 0.06, 0.002), 0, 1)
    frost = np.clip(frost * 1.3, 0, 1)
    L.paint(frost * 0.75, (0.93, 0.96, 0.98), rough=0.8)
    L.h += 0.0002 * frost * (0.5 + 0.5 * cv.noise(salt + 3, 0.01, 0.0015))
    # Perles de condensation (plus nombreuses en haut et près du joint).
    rng = cv.rng(salt + 4)
    beads = cv.zeros()
    bh_ = cv.zeros()
    for _ in range(900):
        x = float(rng.uniform(0.01, w - 0.01))
        y = float(hh - abs(rng.normal(0.0, hh * 0.28)))
        if rng.random() < 0.35:
            # Près des bords latéraux.
            x = float(rng.choice([rng.uniform(0.01, 0.07), rng.uniform(w - 0.07, w - 0.01)]))
            y = float(rng.uniform(0.05, hh - 0.02))
        r = float(rng.uniform(0.0008, 0.0024))
        _dome(cv, bh_, x, y, r, r * 0.5)
    # Les perles ne comptent pas dans l'occlusion (elles l'assombriraient en mouchetures) :
    # leur relief n'entre que dans la carte de normales, à la fin.
    beads = np.clip(bh_ / 0.0005, 0, 1)
    L.paint(beads * 0.25, (0.97, 0.98, 1.0), rough=0.08)
    L.rough = tg.lerp(L.rough, 0.06, beads)
    # Coulures de condensation depuis le haut et sous les nervures.
    src = cv.zeros()
    for _ in range(40):
        x = float(rng.uniform(0.04, w - 0.04))
        y = float(rng.choice([hh - 0.03, hh * FRIDGE_WINDOW[1] - 0.03, hh * 0.38 - 0.012]))
        cv.stamp(src, x, y, 0.004, 0.004, lambda gx, gy: np.clip(1.0 - np.hypot(gx / 0.0016, gy / 0.003), 0, 1))
    trails = cv.smear(src, float(rng.uniform(0.2, 0.4)), decay=0.995)
    trails = np.clip(trails * np.clip(0.4 + 0.6 * cv.noise(salt + 5, 0.3, 0.003, stretch=(1, 12)), 0, 1), 0, 1)
    L.paint(trails * 0.22, (0.66, 0.68, 0.68), rough=0.12)
    L.paint(trails * 0.15, (0.55, 0.52, 0.45))  # calcaire laissé par les gouttes séchées
    # Zone du loquet (droite, à mi-hauteur) : traces de mains grises, éraflures.
    hand = np.exp(-(((cv.x - (w - 0.07)) / 0.06) ** 2 + ((cv.y - hh * 0.55) / 0.22) ** 2))
    L.paint(np.clip(hand * 0.35 * (0.6 + 0.4 * cv.noise(salt + 6, 0.05, 0.003)), 0, 0.45), (0.52, 0.52, 0.5), rough=0.55)
    scr = cv.scratches(salt + 7, 70, (0.01, 0.06), 0.0005, angle=(0.0, 0.6), density=np.clip(hand * 2 + 0.05, 0, 1))
    L.paint(scr * 0.45, (0.62, 0.62, 0.6))
    # Crasse en bas (coups de pied), écaillures seulement sur les arêtes (sur une grande face,
    # `_wear` sèmerait des éclats partout).
    kick = 1.0 - smoothstep(0.0, 0.18, cv.y)
    L.paint(np.clip(kick * 0.4 * (0.5 + 0.5 * cv.noise(salt + 8, 0.08, 0.004)), 0, 0.5), tg.GRIME, rough=0.85)
    edge = 1.0 - smoothstep(0.002, 0.012, d)
    chips = cut(tg.chip_field(cv, salt + 70, edge, 0.004, edge_weight=3.0) * edge, 0.012, 0.08)
    L.paint(chips, (0.5, 0.52, 0.53), rough=0.6)
    L.paint(cut(chips, 0.4) * chips, tg.STEEL, rough=0.38, metal=0.45)
    _fridge_window(cv, L, salt + 90)
    out = tg.finish(cv, L.h, np.clip(L.albedo, 0.0, 1.0), L.rough, L.metal, [(0.004, 0.0008, 0.5), (0.02, 0.002, 0.3)], edge_light=0.05)
    out["normal"] = cv.normal(L.h + bh_ * 0.6)
    out["emissive"] = L.emis
    return out


def _generators():
    return {
        "fridge_door": lambda cv: _fridge_door(cv, 100),
        "meter": lambda cv: _meter(cv, 200),
        "gauge": lambda cv: _gauge(cv, 300),
        "breakers": lambda cv: _breakers(cv, 400),
        "danger": lambda cv: _danger(cv, 500),
        "nameplate": lambda cv: _nameplate(cv, 600),
        "display": lambda cv: _display(cv, 700),
        "leds": lambda cv: _leds(cv, 800),
        "vent_round": lambda cv: _vent_round(cv, 900),
    }


def generate() -> dict[str, Path]:
    maps = {role: np.zeros((RES, RES, 3), np.float32) for role in ("color", "normal", "orm", "emissive")}
    maps["normal"][...] = (0.5, 0.5, 1.0)
    maps["orm"][...] = (1.0, 0.6, 0.0)
    for name, gen in _generators().items():
        (x0, y0, x1, y1), (w, h) = CELLS[name]
        pw, ph = x1 - x0, y1 - y0
        # Une case = un canevas carré à part, à la taille de la face qu'elle habille ; une case
        # deux fois plus haute que large est calculée à pleine hauteur puis réduite en largeur.
        res = max(pw, ph)
        cv = Canvas(res, w, seed=7300 + x0 + y0, tile_h=h)
        out = gen(cv)
        fx = res // pw
        for role in ("color", "normal", "orm", "emissive"):
            img = out[role]
            if fx > 1:
                img = img.reshape(res, pw, fx, 3).mean(axis=2)
            if role == "normal":
                v = img * 2 - 1
                img = v / np.maximum(np.linalg.norm(v, axis=-1, keepdims=True), 1e-6) * 0.5 + 0.5
            if role == "emissive":
                img = linear_to_srgb(img / EMIT_STRENGTH)
            maps[role][y0:y1, x0:x1] = img
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
