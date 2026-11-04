"""Cadres d'interface « futur usé » (M8, lot U) : images découpées en 9 tranches pour Bevy.

Chaque image est un petit gabarit (panneau, bouton, relevé, plaquette) : pans coupés, filets
fins, coins marqués, rivets discrets, accents ambre et cyan comme les voyants et les écrans du
décor. Bevy l'étire par `NodeImageMode::Sliced` : les coins restent nets, les côtés et le centre
s'étirent (`crates/sh_game/src/ui/theme.rs` lit les mêmes bordures, `SLICES` ci-dessous).

Les images sont dessinées au double de leur taille logique : nettes sur un écran à facteur
d'échelle 2, réduites d'un facteur 2 (lecture bilinéaire) à l'échelle 1 ; le jeu borne
`max_corner_scale` à 0,5 pour cela. Tout est calculé ici, sans image extérieure : les traits
obliques sont lissés par suréchantillonnage 4 × 4.

Bibliothèque standard seulement (zlib, struct) :
    uv run --python 3.11 tools/ui/frames.py
"""

import math
import struct
import sys
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "assets" / "ui"

# Pixels d'image par unité logique (pixel d'interface à l'échelle 1).
SCALE = 2
# Suréchantillonnage par axe (lissage des pans coupés et des rivets).
SS = 4
SQRT2 = math.sqrt(2.0)

# Couleurs sRGB (r, g, b, a), alignées sur `theme.rs`.
STEEL = (0.42, 0.45, 0.49)
STEEL_LIGHT = (0.62, 0.65, 0.69)
AMBER = (0.95, 0.66, 0.24)
CYAN = (0.45, 0.82, 0.90)


class Shape:
    """Rectangle w × h (unités logiques) aux coins coupés à 45° : `cuts` = (haut-gauche,
    haut-droit, bas-droit, bas-gauche), longueur de la coupe sur chaque côté."""

    def __init__(self, w: float, h: float, cuts: tuple[float, float, float, float]):
        self.w, self.h, self.cuts = w, h, cuts

    def inside(self, x: float, y: float, inset: float = 0.0) -> bool:
        """Le point est-il dans la forme rentrée de `inset` (bord parallèle, coupes comprises) ?"""
        w, h = self.w, self.h
        if x < inset or y < inset or x > w - inset or y > h - inset:
            return False
        tl, tr, br, bl = self.cuts
        k = inset * SQRT2
        return (
            (tl <= 0 or x + y >= tl + k)
            and (tr <= 0 or (w - x) + y >= tr + k)
            and (br <= 0 or (w - x) + (h - y) >= br + k)
            and (bl <= 0 or x + (h - y) >= bl + k)
        )

    def band(self, x: float, y: float, outer: float, inner: float) -> bool:
        """Entre deux retraits : un filet qui suit le contour."""
        return self.inside(x, y, outer) and not self.inside(x, y, inner)


def over(dst, color, alpha):
    """Peint `color` (r, g, b) d'opacité `alpha` sur `dst` (prémultiplié)."""
    r, g, b, a = dst
    return (
        color[0] * alpha + r * (1 - alpha),
        color[1] * alpha + g * (1 - alpha),
        color[2] * alpha + b * (1 - alpha),
        alpha + a * (1 - alpha),
    )


def render(w: int, h: int, paint) -> bytes:
    """Rend `paint(x, y) -> (r, g, b, a) prémultiplié` en RGBA8 non prémultiplié."""
    pw, ph = w * SCALE, h * SCALE
    rows = []
    for py in range(ph):
        row = bytearray([0])  # filtre PNG « aucun »
        for px in range(pw):
            acc = [0.0, 0.0, 0.0, 0.0]
            for sy in range(SS):
                for sx in range(SS):
                    x = (px + (sx + 0.5) / SS) / SCALE
                    y = (py + (sy + 0.5) / SS) / SCALE
                    c = paint(x, y)
                    for i in range(4):
                        acc[i] += c[i]
            n = SS * SS
            a = acc[3] / n
            if a <= 1e-6:
                row += bytes(4)
                continue
            rgb = [min(1.0, acc[i] / n / a) for i in range(3)]
            row += bytes(round(v * 255) for v in (*rgb, a))
        rows.append(bytes(row))
    return png(pw, ph, b"".join(rows))


def png(w: int, h: int, raw: bytes) -> bytes:
    def chunk(kind: bytes, data: bytes) -> bytes:
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)  # RGBA 8 bits
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def rivet(x: float, y: float, cx: float, cy: float, dst):
    """Une tête de vis : ombre décalée, tête, reflet."""
    if (x - cx - 0.4) ** 2 + (y - cy - 0.4) ** 2 <= 1.6**2:
        dst = over(dst, (0.0, 0.0, 0.0), 0.55)
    if (x - cx) ** 2 + (y - cy) ** 2 <= 1.3**2:
        dst = over(dst, STEEL, 0.95)
    if (x - cx + 0.4) ** 2 + (y - cy + 0.4) ** 2 <= 0.5**2:
        dst = over(dst, STEEL_LIGHT, 0.9)
    return dst


# --- Panneau : coins haut-gauche et bas-droit coupés, coupe haut-gauche en ambre. ------------
PANEL = Shape(48, 48, (12, 0, 12, 0))


def panel(x: float, y: float):
    s = PANEL
    c = (0.0, 0.0, 0.0, 0.0)
    if not s.inside(x, y):
        return c
    c = over(c, (0.050, 0.057, 0.068), 0.94)
    # Bande un peu plus claire le long du haut : la tôle accroche la lumière.
    if s.inside(x, y, 1) and y < 5:
        c = over(c, (1.0, 1.0, 1.0), 0.025)
    if s.band(x, y, 0, 1):
        c = over(c, STEEL, 0.85)
    if s.band(x, y, 3, 3.5):
        c = over(c, STEEL, 0.22)
    # L'ambre souligne la coupe et déborde un peu sur les deux côtés voisins.
    near_cut = x + y < 12 + 2 * SQRT2 + 0.01 and x < 14 and y < 14
    along_top = y < 2 and 12 <= x < 18
    along_left = x < 2 and 12 <= y < 18
    if s.band(x, y, 0, 2) and (near_cut or along_top or along_left):
        c = over(c, AMBER, 0.95)
    c = rivet(x, y, 48 - 6, 6, c)
    c = rivet(x, y, 6, 48 - 6, c)
    return c


# --- Boutons : petits pans coupés, languette à gauche (voyant éteint ou allumé). -------------
BUTTON = Shape(24, 24, (6, 0, 6, 0))


def button(hover: bool):
    fill = (0.25, 0.17, 0.065) if hover else (0.115, 0.125, 0.14)
    line, line_a = (AMBER, 0.9) if hover else (STEEL, 0.6)
    tab, tab_a = (AMBER, 1.0) if hover else (STEEL, 0.5)

    def paint(x: float, y: float):
        s = BUTTON
        c = (0.0, 0.0, 0.0, 0.0)
        if not s.inside(x, y):
            return c
        c = over(c, fill, 0.92)
        if s.band(x, y, 0, 1):
            c = over(c, line, line_a)
        if 2.5 <= x < 4.5 and 7 <= y < 17:
            c = over(c, tab, tab_a)
        return c

    return paint


# --- Relevé (HUD) : verre sombre, quatre coins coupés, équerres cyan. -----------------------
READOUT = Shape(32, 32, (5, 5, 5, 5))


def readout(x: float, y: float):
    s = READOUT
    c = (0.0, 0.0, 0.0, 0.0)
    if not s.inside(x, y):
        return c
    c = over(c, (0.025, 0.040, 0.050), 0.80)
    if s.band(x, y, 0, 1):
        c = over(c, CYAN, 0.28)
    corner = (x < 10 or x > 22) and (y < 10 or y > 22)
    if corner and s.band(x, y, 0, 1.5):
        c = over(c, CYAN, 0.95)
    return c


# --- Plaquette (invites, messages) : fond discret, barre ambre à gauche. --------------------
PLATE = Shape(16, 16, (0, 0, 3, 0))


def plate(x: float, y: float):
    s = PLATE
    c = (0.0, 0.0, 0.0, 0.0)
    if not s.inside(x, y):
        return c
    c = over(c, (0.020, 0.025, 0.030), 0.66)
    if s.band(x, y, 0, 1):
        c = over(c, STEEL, 0.25)
    if x < 2:
        c = over(c, AMBER, 0.95)
    return c


# Nom de fichier -> (largeur, hauteur logiques, peinture). Bordures de découpe : `theme.rs`.
IMAGES = {
    "panel.png": (48, 48, panel),
    "button.png": (24, 24, button(False)),
    "button_hover.png": (24, 24, button(True)),
    "readout.png": (32, 32, readout),
    "plate.png": (16, 16, plate),
}


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (w, h, paint) in IMAGES.items():
        target = OUT / name
        target.write_bytes(render(w, h, paint))
        print(f"écrit {target} ({w * SCALE} × {h * SCALE})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
