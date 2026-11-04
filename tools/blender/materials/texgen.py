"""Boîte à outils des textures procédurales carrelables (lot 0 de M8, docs/refonte-visuelle.md).

Tout est calculé en numpy (le Python de Blender l'embarque) sur une grille **torique** : bruit
spectral (FFT), flous, décalages et tampons bouclent sur les bords de la tuile, si bien que les
raccords sont parfaits par construction, sans bake Cycles ni retouche. Graines fixes : relancer
donne des images identiques.

Repères : un tableau `[ligne, colonne]` a sa ligne 0 en BAS de l'image, comme les pixels de
Blender et l'UV v = 0 ; `write_png` retourne l'image à l'écriture. Colonnes = U (x), lignes =
V (y). Le canevas connaît la taille physique de sa tuile (`tile_m`) : positions et hauteurs sont
en mètres, ce qui donne des normales à la bonne échelle quelle que soit la résolution.
"""

from __future__ import annotations

import struct
import zlib
from pathlib import Path

import numpy as np

# Métal plafonné dans les ORM : dans une cellule à lightmap, le jeu n'éclaire que le diffus
# (lightmap × couleur × (1 − métal), sans lumière ponctuelle ni carte d'environnement en
# Moyen). Un métal pur y serait noir ; à 0,55 il garde de la matière.
METAL_MAX = 0.55
# Part de l'occlusion recopiée dans la couleur : la lightmap ne lit pas la texture
# d'occlusion, les creux doivent donc être sombres dans la couleur elle-même.
ALBEDO_AO = 0.7


def smoothstep(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def lerp(a, b, t):
    t = np.asarray(t)
    if np.ndim(a) == 3 or np.ndim(b) == 3:
        if t.ndim == 2:
            t = t[..., None]
    return a + (b - a) * t


def rgb(c) -> np.ndarray:
    return np.asarray(c, dtype=np.float32).reshape(1, 1, 3)


# --- Formes (distances signées, coordonnées relatives en mètres) --------------------------------


def sd_box(dx, dy, hw, hh, r=0.0):
    """Rectangle aux coins arrondis (demi-côtés hw, hh) ; négatif à l'intérieur."""
    qx = np.abs(dx) - (hw - r)
    qy = np.abs(dy) - (hh - r)
    out = np.hypot(np.maximum(qx, 0.0), np.maximum(qy, 0.0))
    return out + np.minimum(np.maximum(qx, qy), 0.0) - r


def sd_circle(dx, dy, r):
    return np.hypot(dx, dy) - r


def sd_segment(dx, dy, ax, ay, bx, by):
    """Distance au segment [a, b]."""
    px, py = dx - ax, dy - ay
    ex, ey = bx - ax, by - ay
    t = np.clip((px * ex + py * ey) / max(ex * ex + ey * ey, 1e-12), 0.0, 1.0)
    return np.hypot(px - ex * t, py - ey * t)


def sd_poly(dx, dy, pts):
    """Polygone convexe (sommets dans le sens trigonométrique) ; négatif à l'intérieur."""
    d = None
    n = len(pts)
    for i in range(n):
        ax, ay = pts[i]
        bx, by = pts[(i + 1) % n]
        ex, ey = bx - ax, by - ay
        ln = max(np.hypot(ex, ey), 1e-12)
        # Normale sortante d'un polygone trigonométrique : (ey, −ex).
        s = ((dx - ax) * ey - (dy - ay) * ex) / ln
        d = s if d is None else np.maximum(d, s)
    return d


def fill(sd, px):
    """Masque anticrénelé d'une distance signée (1 à l'intérieur), sur un pixel."""
    return np.clip(0.5 - sd / px, 0.0, 1.0)


# --- Canevas -----------------------------------------------------------------------------------


class Canvas:
    """Tuile carrée de `res` pixels couvrant `tile_m` mètres, graine `seed`."""

    def __init__(self, res: int = 1024, tile_m: float = 2.0, seed: int = 0, tile_h: float | None = None):
        self.res = res
        self.tile = tile_m
        self.tile_h = tile_h or tile_m
        self.px = tile_m / res
        self.py = self.tile_h / res
        self.seed = seed
        cx = ((np.arange(res, dtype=np.float32) + 0.5) * self.px)
        cy = ((np.arange(res, dtype=np.float32) + 0.5) * self.py)
        self.x = np.broadcast_to(cx[None, :], (res, res))
        self.y = np.broadcast_to(cy[:, None], (res, res))

    # Hasard -------------------------------------------------------------------------------------

    def rng(self, salt: int) -> np.random.Generator:
        return np.random.default_rng([self.seed, salt])

    def zeros(self, channels: int | None = None) -> np.ndarray:
        shape = (self.res, self.res) if channels is None else (self.res, self.res, channels)
        return np.zeros(shape, dtype=np.float32)

    def full(self, value) -> np.ndarray:
        if np.ndim(value) == 0:
            return np.full((self.res, self.res), value, dtype=np.float32)
        return np.broadcast_to(rgb(value), (self.res, self.res, 3)).astype(np.float32).copy()

    # Bruits -------------------------------------------------------------------------------------

    def noise(self, salt: int, big_m: float, small_m: float | None = None, beta: float = 1.0, stretch=(1.0, 1.0)) -> np.ndarray:
        """Bruit spectral périodique, centré réduit (moyenne 0, écart type 1).

        Détails entre `small_m` et `big_m` mètres, spectre d'amplitude en f^-beta (1 : nuages
        naturels). `stretch` = (sx, sy) : un sx > 1 étire les motifs le long de x (brossé).
        """
        res = self.res
        white = self.rng(salt).standard_normal((res, res)).astype(np.float32)
        spec = np.fft.rfft2(white)
        fy = np.fft.fftfreq(res)[:, None] * res / self.tile_h
        fx = np.fft.rfftfreq(res)[None, :] * res / self.tile
        f = np.sqrt((fx * stretch[0]) ** 2 + (fy * stretch[1]) ** 2)
        f_lo = 1.0 / big_m
        f_hi = 1.0 / small_m if small_m else res / (2.0 * min(self.tile, self.tile_h))
        safe = np.maximum(f, 1e-9)
        amp = safe ** (-beta)
        lo = (safe / f_lo) ** 4
        amp *= lo / (1.0 + lo) * np.exp(-((safe / f_hi) ** 2))
        amp[0, 0] = 0.0
        out = np.fft.irfft2(spec * amp, s=(res, res)).astype(np.float32)
        out -= out.mean()
        out /= out.std() + 1e-9
        return out

    def voronoi(self, salt: int, cells: int, jitter: float = 0.9):
        """Cellules de Voronoï périodiques (grille `cells` × `cells` jitterée).

        Renvoie (F1, F2, identifiant) ; distances en mètres.
        """
        res = self.res
        rng = self.rng(salt)
        jx = (rng.random((cells, cells)) - 0.5) * jitter + 0.5
        jy = (rng.random((cells, cells)) - 0.5) * jitter + 0.5
        gx = (np.arange(res) + 0.5) / res * cells
        gx = np.broadcast_to(gx[None, :], (res, res))
        gy = np.broadcast_to(((np.arange(res) + 0.5) / res * cells)[:, None], (res, res))
        ix = np.floor(gx).astype(np.int32)
        iy = np.floor(gy).astype(np.int32)
        fx = gx - ix
        fy = gy - iy
        f1 = np.full((res, res), 1e9, dtype=np.float32)
        f2 = np.full((res, res), 1e9, dtype=np.float32)
        ident = np.zeros((res, res), dtype=np.int32)
        sx, sy = self.tile / cells, self.tile_h / cells
        for oy in (-1, 0, 1):
            for ox in (-1, 0, 1):
                cxi = (ix + ox) % cells
                cyi = (iy + oy) % cells
                px = ox + jx[cyi, cxi] - fx
                py = oy + jy[cyi, cxi] - fy
                d = np.hypot(px * sx, py * sy).astype(np.float32)
                cid = cyi * cells + cxi
                closer = d < f1
                f2 = np.where(closer, f1, np.minimum(f2, d))
                ident = np.where(closer, cid, ident)
                f1 = np.where(closer, d, f1)
        return f1, f2, ident

    # Filtres ------------------------------------------------------------------------------------

    def blur(self, img: np.ndarray, sigma_m: float, sigma_y_m: float | None = None) -> np.ndarray:
        """Flou gaussien périodique (rayons en mètres)."""
        if img.ndim == 3:
            return np.stack([self.blur(img[..., i], sigma_m, sigma_y_m) for i in range(img.shape[2])], -1)
        sx = sigma_m / self.px
        sy = (sigma_y_m if sigma_y_m is not None else sigma_m) / self.py
        if sx <= 0.0 and sy <= 0.0:
            return img
        res = self.res
        fy = np.fft.fftfreq(res)[:, None]
        fx = np.fft.rfftfreq(res)[None, :]
        g = np.exp(-2.0 * np.pi**2 * ((sx * fx) ** 2 + (sy * fy) ** 2))
        return np.fft.irfft2(np.fft.rfft2(img) * g, s=img.shape).astype(np.float32)

    def smear(self, img: np.ndarray, length_m: float, dx: int = 0, dy: int = -1, decay: float | None = None) -> np.ndarray:
        """Traînées : chaque valeur s'étire dans la direction (dx, dy) en pixels (défaut : vers
        le bas, coulures), avec une décroissance exponentielle sur `length_m` mètres."""
        steps = max(1, int(length_m / self.py))
        k = decay if decay is not None else float(np.exp(-3.0 / steps))
        out = img.copy()
        cur = img.copy()
        for _ in range(steps):
            cur = np.roll(cur, (dy, dx), axis=(0, 1)) * k
            np.maximum(out, cur, out=out)
        return out

    def shift(self, img: np.ndarray, dx_m: float, dy_m: float) -> np.ndarray:
        return np.roll(img, (int(round(dy_m / self.py)), int(round(dx_m / self.px))), axis=(0, 1))

    def wrap_dx(self, x, cx):
        t = self.tile
        return (x - cx + t / 2.0) % t - t / 2.0

    def wrap_dy(self, y, cy):
        t = self.tile_h
        return (y - cy + t / 2.0) % t - t / 2.0

    # Tampons ------------------------------------------------------------------------------------

    def stamp(self, dst: np.ndarray, cx: float, cy: float, rx: float, ry: float, fn, op: str = "max") -> None:
        """Applique `fn(dx, dy)` (mètres relatifs au centre) dans la fenêtre ±(rx, ry) autour de
        (cx, cy), en bouclant sur les bords. `op` : max, min, add, mul, over (fn renvoie
        (valeur, alpha)), set."""
        res = self.res
        x0 = int(np.floor((cx - rx) / self.px)) - 1
        x1 = int(np.ceil((cx + rx) / self.px)) + 1
        y0 = int(np.floor((cy - ry) / self.py)) - 1
        y1 = int(np.ceil((cy + ry) / self.py)) + 1
        xs = np.arange(x0, x1 + 1)
        ys = np.arange(y0, y1 + 1)
        if len(xs) >= res:
            xs = np.arange(0, res)
        if len(ys) >= res:
            ys = np.arange(0, res)
        ddx = ((xs + 0.5) * self.px - cx).astype(np.float32)
        ddy = ((ys + 0.5) * self.py - cy).astype(np.float32)
        if len(xs) == res:
            ddx = self.wrap_dx((xs + 0.5) * self.px, cx).astype(np.float32)
        if len(ys) == res:
            ddy = self.wrap_dy((ys + 0.5) * self.py, cy).astype(np.float32)
        gx, gy = np.meshgrid(ddx, ddy)
        idx = np.ix_(ys % res, xs % res)
        region = dst[idx]
        val = fn(gx, gy)
        if op == "max":
            dst[idx] = np.maximum(region, val)
        elif op == "min":
            dst[idx] = np.minimum(region, val)
        elif op == "add":
            dst[idx] = region + val
        elif op == "mul":
            dst[idx] = region * val
        elif op == "set":
            dst[idx] = val
        elif op == "over":
            v, a = val
            if region.ndim == 3 and np.ndim(a) == 2:
                a = a[..., None]
            dst[idx] = region * (1.0 - a) + v * a
        else:
            raise ValueError(op)

    # Relief -------------------------------------------------------------------------------------

    def normal(self, h: np.ndarray, strength: float = 1.0) -> np.ndarray:
        """Carte de normales en espace tangent, convention OpenGL / glTF (vert = +V), depuis
        une hauteur en mètres ; renvoyée dans [0, 1]."""
        hx = (np.roll(h, -1, axis=1) - np.roll(h, 1, axis=1)) / (2.0 * self.px)
        hy = (np.roll(h, -1, axis=0) - np.roll(h, 1, axis=0)) / (2.0 * self.py)
        n = np.stack([-hx * strength, -hy * strength, np.ones_like(h)], -1)
        n /= np.linalg.norm(n, axis=-1, keepdims=True)
        return (n * 0.5 + 0.5).astype(np.float32)

    def cavity(self, h: np.ndarray, radius_m: float, depth_m: float) -> np.ndarray:
        """Creux relatif (0 à 1) : de combien un point est sous la moyenne de son voisinage."""
        return np.clip((self.blur(h, radius_m) - h) / depth_m, 0.0, 1.0)

    def convexity(self, h: np.ndarray, radius_m: float, depth_m: float) -> np.ndarray:
        return np.clip((h - self.blur(h, radius_m)) / depth_m, 0.0, 1.0)

    def occlusion(self, h: np.ndarray, layers) -> np.ndarray:
        """Occlusion ambiante approchée : somme pondérée de creux à plusieurs rayons.
        `layers` : [(rayon m, profondeur m, poids)]."""
        ao = np.ones_like(h)
        for radius, depth, weight in layers:
            ao -= weight * self.cavity(h, radius, depth)
        return np.clip(ao, 0.0, 1.0)

    # Rayures ------------------------------------------------------------------------------------

    def scratches(self, salt: int, count: int, length_m=(0.03, 0.2), width_m: float = 0.0015, angle=None, density=None, curve: float = 0.0) -> np.ndarray:
        """Rayures fines (masque 0 à 1) : segments au hasard, plus nombreux là où `density`
        (0 à 1) est forte ; `angle` : angle moyen (radians) et dispersion, ou None (toutes
        directions)."""
        out = self.zeros()
        rng = self.rng(salt)
        tries = 0
        made = 0
        while made < count and tries < count * 20:
            tries += 1
            cx, cy = rng.random() * self.tile, rng.random() * self.tile_h
            if density is not None:
                ix = int(cx / self.px) % self.res
                iy = int(cy / self.py) % self.res
                if rng.random() > density[iy, ix]:
                    continue
            made += 1
            ln = rng.uniform(*length_m)
            a = rng.uniform(0, np.pi) if angle is None else rng.normal(angle[0], angle[1])
            ax, ay = -0.5 * ln * np.cos(a), -0.5 * ln * np.sin(a)
            bx, by = -ax, -ay
            w = width_m * rng.uniform(0.6, 1.6)
            strength = rng.uniform(0.35, 1.0)
            bend = rng.normal(0, curve)

            def fn(gx, gy, ax=ax, ay=ay, bx=bx, by=by, w=w, s=strength, bend=bend, ln=ln):
                if bend:
                    # Légère courbure : décalage perpendiculaire parabolique.
                    t = np.clip(((gx - ax) * (bx - ax) + (gy - ay) * (by - ay)) / (ln * ln), 0, 1)
                    off = bend * ln * 4 * t * (1 - t)
                    nx, ny = -(by - ay) / ln, (bx - ax) / ln
                    gx = gx - nx * off
                    gy = gy - ny * off
                d = sd_segment(gx, gy, ax, ay, bx, by)
                t = np.clip(((gx - ax) * (bx - ax) + (gy - ay) * (by - ay)) / (ln * ln), 0, 1)
                taper = np.sin(np.pi * t) ** 0.5
                return s * taper * np.clip(1.0 - d / (w + self.px * 0.5), 0.0, 1.0)

            r = 0.5 * ln + w + 0.01 + abs(bend) * ln
            self.stamp(out, cx, cy, r, r, fn, "max")
        return out

    # Sorties ------------------------------------------------------------------------------------

    def tile3(self, img: np.ndarray, factor: int = 2) -> np.ndarray:
        """Aperçu 3 × 3 (réduit) pour vérifier les raccords à l'œil."""
        small = downsample(img, factor) if factor > 1 else img
        return np.tile(small, (3, 3, 1) if small.ndim == 3 else (3, 3))


def downsample(img: np.ndarray, factor: int = 2) -> np.ndarray:
    h, w = img.shape[:2]
    if img.ndim == 3:
        return img.reshape(h // factor, factor, w // factor, factor, img.shape[2]).mean(axis=(1, 3))
    return img.reshape(h // factor, factor, w // factor, factor).mean(axis=(1, 3))


def encode_png(img: np.ndarray) -> bytes:
    """PNG 8 bits RGB (ou niveaux de gris si 2D), ligne 0 du tableau = bas de l'image.

    Écrit à la main (zlib) : octets reproductibles, sans canal alpha, filtres PNG choisis
    ligne par ligne (Sub, Up, Paeth) pour des fichiers compacts.
    """
    arr = np.clip(img, 0.0, 1.0)
    data = (arr * 255.0 + 0.5).astype(np.uint8)[::-1]
    if data.ndim == 2:
        data = data[..., None]
    h, w, ch = data.shape
    color_type = 2 if ch == 3 else 0
    raw = data.reshape(h, w * ch).astype(np.int16)
    left = np.zeros_like(raw)
    left[:, ch:] = raw[:, :-ch]
    up = np.zeros_like(raw)
    up[1:] = raw[:-1]
    upleft = np.zeros_like(raw)
    upleft[1:, ch:] = raw[:-1, :-ch]
    p = left + up - upleft
    pa, pb, pc = np.abs(p - left), np.abs(p - up), np.abs(p - upleft)
    pred = np.where((pa <= pb) & (pa <= pc), left, np.where(pb <= pc, up, upleft))
    candidates = [raw % 256, (raw - left) % 256, (raw - up) % 256, (raw - pred) % 256]
    kinds = [0, 1, 2, 4]
    scores = np.stack([np.minimum(c, 256 - c).sum(axis=1) for c in candidates])
    best = scores.argmin(axis=0)
    rows = np.empty((h, w * ch + 1), dtype=np.uint8)
    for i, cand in enumerate(candidates):
        sel = best == i
        rows[sel, 0] = kinds[i]
        rows[sel, 1:] = cand[sel].astype(np.uint8)
    payload = zlib.compress(rows.tobytes(), 9)

    def chunk(tag: bytes, body: bytes) -> bytes:
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", w, h, 8, color_type, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", payload) + chunk(b"IEND", b"")


def write_png(path: Path, img: np.ndarray) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    blob = encode_png(img)
    path.write_bytes(blob)
    return len(blob)


def srgb_to_linear(c):
    c = np.asarray(c, dtype=np.float32)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


# --- Usure : briques communes aux matériaux -----------------------------------------------------
# Leçon de la première passe : un seuil dur sur un bruit large donne des taches « camouflage ».
# L'usure crédible est rare, regroupée, concentrée sur les arêtes ; les variations de teinte sont
# lentes et douces.

# Couleurs (sRGB) communes à l'usure.
STEEL = (0.58, 0.575, 0.56)
PRIMER = (0.42, 0.42, 0.39)  # apprêt gris (deux couches qui s'écaillent, pas de « rougeole » rouille)
GRIME = (0.16, 0.14, 0.12)
DUST = (0.55, 0.52, 0.47)
RUST = (0.40, 0.22, 0.12)


def cut(field: np.ndarray, frac: float, sharp: float = 0.08) -> np.ndarray:
    """Masque couvrant ≈ `frac` de la tuile (seuil au centile), bord de largeur `sharp`."""
    t = float(np.percentile(field, 100.0 * (1.0 - frac)))
    return smoothstep(t, t + sharp, field)


def tone(cv: Canvas, salt: int, big_m: float, amount: float) -> np.ndarray:
    """Variation multiplicative (1 ± amount) : fBm complet jusqu'à quelques pixels (un bruit
    limité à deux octaves fait des « nuages » de retouche photo)."""
    return 1.0 + amount * cv.noise(salt, big_m, max(big_m / 60.0, 2.5 * cv.px), beta=1.1)


def stains(cv: Canvas, salt: int, big_m: float, frac: float, edge: float = 0.35, breakup: float = 0.25) -> np.ndarray:
    """Taches rares (huile, crasse) de taille ≈ big_m : forme lente, bord cassé par un bruit fin."""
    base = cv.noise(salt, big_m, big_m / 5.0)
    fine = cv.noise(salt + 1, big_m / 6.0, big_m / 40.0)
    return cut(base + breakup * fine, frac, edge)


def chip_field(cv: Canvas, salt: int, edge: np.ndarray, flake_m: float = 0.01, edge_weight: float = 2.0) -> np.ndarray:
    """Champ d'écaillures : fort près des arêtes (`edge`, 0 à 1), en flocons de ≈ flake_m,
    regroupés par un bruit plus large. À couper avec `cut` (deux seuils : apprêt puis métal)."""
    cluster = cv.noise(salt, flake_m * 15.0, flake_m * 3.0)
    flake = cv.noise(salt + 1, flake_m * 2.0, flake_m * 0.3, beta=1.5)
    return edge_weight * edge + 0.7 * cluster + 0.5 * flake


def drips(cv: Canvas, src: np.ndarray, length_m: float, salt: int) -> np.ndarray:
    """Coulures verticales sous les sources (rivets, joints, fentes), striées et irrégulières."""
    s = cv.smear(src.astype(np.float32), length_m)
    mod = np.clip(0.5 + 0.5 * cv.noise(salt, 0.3, 0.003, stretch=(1.0, 12.0)), 0.0, 1.0)
    return np.clip(s * mod, 0.0, 1.0)


def finish(cv: Canvas, h, albedo, rough, metal, ao_layers, ao_extra=None, emissive=None,
           normal_strength=1.0, edge_light=0.08, albedo_ao=ALBEDO_AO):
    """Assemble les cartes finales : normale depuis la hauteur, occlusion depuis les creux
    (recopiée en partie dans la couleur, que la lightmap multiplie), arêtes vives éclaircies
    (le relief doit se lire même sans la carte de normales, ignorée par la lightmap), ORM plafonné."""
    ao = cv.occlusion(h, ao_layers)
    if ao_extra is not None:
        ao = ao * ao_extra
    shade = 1.0 - albedo_ao * (1.0 - ao)
    color = albedo * shade[..., None]
    if edge_light:
        color = color * (1.0 + edge_light * cv.convexity(h, 3 * cv.px, 0.0004))[..., None]
    out = {
        "color": np.clip(color, 0.0, 1.0),
        "normal": cv.normal(h, normal_strength),
        "orm": np.stack([ao, np.clip(rough, 0.05, 1.0), np.clip(metal, 0.0, METAL_MAX)], -1),
    }
    if emissive is not None:
        out["emissive"] = np.clip(emissive, 0.0, 1.0)
    return out


def even_spacing(length: float, target: float) -> float:
    """Pas le plus proche de `target` qui divise exactement `length` (périodicité)."""
    return length / max(1, round(length / target))


def rivets_along_x(cv: Canvas, y_line: float, spacing: float, radius: float, height: float, x_off: float = 0.0):
    """Rangée de rivets en dôme le long de x (hauteur en mètres)."""
    sp = even_spacing(cv.tile, spacing)
    dx = ((cv.x - x_off) % sp) - sp / 2.0
    dy = cv.wrap_dy(cv.y, y_line)
    d = np.hypot(dx, dy)
    return height * np.sqrt(np.clip(1.0 - (d / radius) ** 2, 0.0, 1.0))


def panel_field(cv: Canvas, rects, corner: float = 0.015):
    """Distance intérieure au bord du panneau (m) et indice du panneau, pour une partition de
    la tuile en rectangles (x0, y0, x1, y1)."""
    dist = np.full((cv.res, cv.res), -1.0, dtype=np.float32)
    ident = np.zeros((cv.res, cv.res), dtype=np.int32)
    for i, (x0, y0, x1, y1) in enumerate(rects):
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        d = -sd_box(cv.wrap_dx(cv.x, cx), cv.wrap_dy(cv.y, cy), (x1 - x0) / 2, (y1 - y0) / 2, corner)
        better = d > dist
        dist = np.where(better, d, dist)
        ident = np.where(better, i, ident)
    return dist, ident


def seam_profile(d, half: float, bevel: float, depth: float):
    """Profil d'un joint : creux de `depth` sur `half` de part et d'autre, arête arrondie."""
    return -depth * (1.0 - smoothstep(half * 0.4, half + bevel, d))


def screw(gx, gy, r, slot_angle=0.6, height=0.0008):
    """Tête de vis bombée fendue (hauteur relative)."""
    d = np.hypot(gx, gy)
    dome = height * np.sqrt(np.clip(1.0 - (d / r) ** 2, 0.0, 1.0))
    ca, sa = np.cos(slot_angle), np.sin(slot_angle)
    along = gx * ca + gy * sa
    across = -gx * sa + gy * ca
    slot = (np.abs(across) < r * 0.16) & (np.abs(along) < r * 0.8)
    return np.where(slot, dome * 0.2, dome)


# --- Pochoirs génériques ------------------------------------------------------------------------
# Police « pochoir » originale faite de traits et d'arcs (ponts entre les traits), sans lettre
# reconnaissable d'une police existante. Boîte de glyphe : largeur 1, hauteur 1,6.

_GLYPHS = {
    "0": [("arc", 0.5, 1.1, 0.38, 100, 180), ("seg", 0.12, 1.1, 0.12, 0.5), ("arc", 0.5, 0.5, 0.38, 180, 260),
          ("arc", 0.5, 1.1, 0.38, 0, 80), ("seg", 0.88, 1.1, 0.88, 0.5), ("arc", 0.5, 0.5, 0.38, 280, 360)],
    "1": [("seg", 0.58, 0.0, 0.58, 1.6), ("seg", 0.42, 1.52, 0.2, 1.3)],
    "2": [("arc", 0.5, 1.13, 0.37, -35, 165), ("seg", 0.8, 0.92, 0.17, 0.25), ("seg", 0.12, 0.08, 0.9, 0.08)],
    "3": [("arc", 0.47, 1.2, 0.33, -90, 160), ("arc", 0.47, 0.44, 0.37, -160, 90)],
    "4": [("seg", 0.58, 1.6, 0.08, 0.55), ("seg", 0.08, 0.45, 0.95, 0.45), ("seg", 0.7, 1.15, 0.7, 0.0)],
    "5": [("seg", 0.88, 1.52, 0.28, 1.52), ("seg", 0.15, 1.45, 0.15, 0.98), ("arc", 0.47, 0.52, 0.4, -150, 125)],
    "7": [("seg", 0.08, 1.52, 0.92, 1.52), ("seg", 0.86, 1.34, 0.36, 0.0)],
    "A": [("seg", 0.05, 0.0, 0.45, 1.5), ("seg", 0.95, 0.0, 0.55, 1.5), ("seg", 0.3, 0.55, 0.7, 0.55)],
    "B": [("seg", 0.12, 0.0, 0.12, 1.6), ("arc", 0.5, 1.2, 0.36, -90, 90), ("arc", 0.5, 0.42, 0.4, -90, 90),
          ("seg", 0.3, 1.56, 0.5, 1.56), ("seg", 0.3, 0.8, 0.5, 0.8), ("seg", 0.3, 0.03, 0.5, 0.03)],
}


def _sd_arc(gx, gy, cx, cy, r, a0, a1):
    px, py = gx - cx, gy - cy
    ang = np.degrees(np.arctan2(py, px)) % 360.0
    a0m, a1m = a0 % 360.0, a1 % 360.0
    if a1 - a0 >= 360:
        inside = np.ones_like(ang, dtype=bool)
    elif a0m <= a1m:
        inside = (ang >= a0m) & (ang <= a1m)
    else:
        inside = (ang >= a0m) | (ang <= a1m)
    ring = np.abs(np.hypot(px, py) - r)
    e0 = np.hypot(px - r * np.cos(np.radians(a0)), py - r * np.sin(np.radians(a0)))
    e1 = np.hypot(px - r * np.cos(np.radians(a1)), py - r * np.sin(np.radians(a1)))
    return np.where(inside, ring, np.minimum(e0, e1))


def glyph_sd(ch: str, gx, gy, size: float, stroke: float = 0.2):
    """Distance signée d'un glyphe de hauteur `size` (origine en bas à gauche)."""
    s = size / 1.6
    u, v = gx / s, gy / s
    d = np.full(np.shape(gx), 1e3, dtype=np.float32)
    for prim in _GLYPHS[ch]:
        dd = sd_segment(u, v, *prim[1:]) if prim[0] == "seg" else _sd_arc(u, v, *prim[1:])
        d = np.minimum(d, dd)
    return (d - stroke / 2) * s


def text_sd(text: str, gx, gy, size: float, spacing: float = 0.28, stroke: float = 0.2):
    s = size / 1.6
    d = np.full(np.shape(gx), 1e3, dtype=np.float32)
    x = 0.0
    for ch in text:
        if ch != " ":
            d = np.minimum(d, glyph_sd(ch, gx - x, gy, size, stroke))
        x += (1.0 + spacing) * s
    return d


def text_width(text: str, size: float, spacing: float = 0.28) -> float:
    s = size / 1.6
    return len(text) * (1.0 + spacing) * s - spacing * s
