"""Toile de combinaison de travail (lot F de M8) : `char_cloth`, texture répétable 1K.

Pourquoi pas `fabric` de la bibliothèque : c'est un similicuir matelassé de siège (bourrelets
horizontaux) ; posé sur un corps, il en fait un bibendum. Ici : sergé épais, plis lâches plutôt
verticaux, coutures doubles (une verticale, une horizontale par tuile), une pièce cousue de
réparation, usure claire et taches. Couleur neutre claire, teintée par le facteur glTF
(`pirate.SUIT_TINT`), comme les matériaux teintables de la bibliothèque.

Tuile de 0,5 m (`TILE_M`) : 2 px/mm. Cartes : `char_cloth_{color,normal,orm}.png` dans
assets/textures/materials/ (contrat du jeu : couleur sRGB, normale OpenGL et ORM linéaires).

    blender --background --factory-startup --threads 4 --python tools/blender/characters/char_cloth.py
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
PREVIEW_DIR = ROOT / "target" / "art-previews" / "lotF"
NAME = "char_cloth"
RES = 1024
TILE_M = 0.5


def _stitch_line(cv: Canvas, d: np.ndarray, along: np.ndarray, pitch: float = 0.004) -> np.ndarray:
    """Points de couture en tirets le long d'une ligne (distance `d` à la ligne)."""
    dash = (np.sin(along / pitch * 2 * np.pi) > -0.2).astype(np.float32)
    return np.clip(1.0 - np.abs(d) / 0.0006, 0.0, 1.0) * dash


def generate() -> dict[str, Path]:
    cv = Canvas(RES, TILE_M, seed=7311)
    # Sergé : côtes diagonales fines + trame.
    twill = 0.5 + 0.5 * np.sin((cv.x * 1.0 + cv.y * 1.0) / 0.0016 * np.pi)
    weft = 0.5 + 0.5 * np.sin(cv.y / 0.0011 * np.pi)
    h = 0.00006 * twill + 0.00003 * weft
    # Plis : bruit étiré verticalement (le tissu tombe), quelques plis marqués.
    folds = cv.noise(1, 0.25, 0.01, beta=1.6, stretch=(1.0, 3.0))
    h += 0.0007 * folds
    creases = np.abs(cv.noise(2, 0.2, 0.02, stretch=(1.0, 3.0)))
    crease = 1.0 - smoothstep(0.0, 0.12, creases)
    h -= 0.0003 * crease

    # Coutures doubles : une verticale (x = 0,12 m), une horizontale (y = 0,31 m), bourrelet.
    seams = cv.zeros()
    stitches = cv.zeros()
    for pos, vertical in ((0.12, True), (0.31, False)):
        d = cv.wrap_dx(cv.x, pos) if vertical else cv.wrap_dy(cv.y, pos)
        along = cv.y if vertical else cv.x
        seams = np.maximum(seams, np.clip(1.0 - np.abs(d) / 0.0012, 0.0, 1.0))
        h += 0.0006 * np.exp(-((d / 0.004) ** 2)) - 0.0004 * np.clip(1.0 - np.abs(d) / 0.0012, 0.0, 1.0)
        for off in (-0.0045, 0.0045):
            stitches = np.maximum(stitches, _stitch_line(cv, d - off, along))
    # Pièce de réparation cousue (rectangle un peu de travers).
    pcx, pcy, ang = 0.34, 0.12, 0.08
    dx, dy = cv.wrap_dx(cv.x, pcx), cv.wrap_dy(cv.y, pcy)
    rx = dx * np.cos(ang) + dy * np.sin(ang)
    ry = -dx * np.sin(ang) + dy * np.cos(ang)
    pd = tg.sd_box(rx, ry, 0.055, 0.04, 0.004)
    patch = tg.fill(pd, cv.px)
    h += 0.0005 * patch
    stitches = np.maximum(stitches, _stitch_line(cv, pd + 0.004, rx + ry))
    h += 0.00012 * stitches

    # Usure : arêtes de plis éclaircies, taches de graisse, poussière.
    ridge = cv.convexity(h, 0.006, 0.0006)
    wear = np.clip(ridge * 0.5 + 0.25 * cut(cv.noise(3, 0.08, 0.004), 0.15, 0.4), 0.0, 1.0)
    grease = stains(cv, 4, 0.25, 0.06, edge=0.8)
    dust = stains(cv, 5, 0.15, 0.2, edge=0.7)

    base = rgb((0.66, 0.64, 0.60)) * tg.tone(cv, 6, 0.2, 0.06)[..., None]
    base = base * (0.95 + 0.05 * twill)[..., None]
    albedo = tg.lerp(base, rgb((0.60, 0.61, 0.60)), patch * 0.6)  # pièce d'une autre toile, plus froide
    albedo = tg.lerp(albedo, rgb((0.80, 0.78, 0.73)), wear * 0.35)
    albedo = tg.lerp(albedo, rgb((0.45, 0.43, 0.40)), seams * 0.4)
    albedo = tg.lerp(albedo, rgb((0.38, 0.36, 0.33)), stitches * 0.5)
    albedo = tg.lerp(albedo, rgb((0.30, 0.27, 0.23)), grease * 0.3)
    albedo = tg.lerp(albedo, rgb((0.72, 0.68, 0.60)), dust * 0.25)
    rough = np.clip(0.88 - 0.25 * grease + 0.04 * cv.noise(7, 0.05, 0.003), 0.4, 1.0)
    metal = np.zeros_like(rough)
    maps = tg.finish(cv, h, albedo, rough, metal, [(0.004, 0.0008, 0.3), (0.03, 0.003, 0.3)], edge_light=0.0)

    written = {}
    for role in ("color", "normal", "orm"):
        path = OUT_DIR / f"{NAME}_{role}.png"
        tg.write_png(path, maps[role])
        written[role] = path
    PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
    tg.write_png(PREVIEW_DIR / "cloth_color_3x3.png", cv.tile3(maps["color"], 2))
    print(f"toile {NAME} : 3 cartes {RES} px dans {OUT_DIR}")
    return written


if __name__ == "__main__":
    generate()
