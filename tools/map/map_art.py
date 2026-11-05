"""Images de la carte galactique (M9, lot L8) : fond de la galaxie, nébuleuse, icônes.

Tout est calculé ici, sans image extérieure, de façon déterministe (graine fixe) :

- `galaxy.png` : Astréa vue de dessus, cadrée sur l'ellipse canon (§3.1 : centre (500, 330),
  rayons 310 × 248) avec une marge (`MARGIN`). Bulbe doré au centre (le noyau, trou noir
  ordinaire), deux bras spiraux bleutés, disque, poussières et semis d'étoiles. Le dessin des
  bras reprend la règle du ciel de la vue Globale (`crates/sh_game/src/views/sky.rs`, P17 :
  spirale logarithmique de 15°, phase 0,6) pour que la carte et le ciel se répondent. Le bord de
  l'image a exactement la couleur `BACKGROUND`, que la caméra de la carte reprend comme couleur
  de fond (`views/galaxy.rs`) : aucune couture.
- `nebula.png` : la nappe d'une nébuleuse en arc, ouverte vers l'est comme dans le ciel (P17),
  rouge et violette (couleur inventée), posée par le jeu à la position et au rayon des données.
- icônes (blanches, teintées en jeu par la couleur d'affiliation) : système, monde isolé,
  planète errante, monde détruit, station, hub de Portails, noyau, nébuleuse (légende), flotte,
  position du joueur, destination. Chaque icône est dessinée au double de sa taille d'affichage
  (nette à l'échelle 2, réduite de moitié à l'échelle 1), avec un liseré sombre qui la détache
  d'un fond clair (le bulbe).

Bibliothèque standard seulement :
    uv run --python 3.11 tools/map/map_art.py
"""

import math
import random
import struct
import sys
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "assets" / "ui" / "map"

# Ellipse canon de la galaxie (§3.1), en unités de carte.
RADIUS_X = 310.0
RADIUS_Y = 248.0
# L'image couvre ± MARGIN rayons autour du centre (la lueur s'éteint à 1,25 rayon).
MARGIN = 1.35
GALAXY_W = 1024
GALAXY_H = round(GALAXY_W * RADIUS_Y / RADIUS_X)  # 819 : même rapport que l'ellipse

# Couleur de fond (sRGB 8 bits), reprise par `views/galaxy.rs` (`BACKGROUND`).
BACKGROUND = (4, 5, 10)

# Bras spiraux : mêmes réglages que `sky.rs`.
ARM_PITCH_DEG = 15.0
ARM_WIDTH = 0.45
ARM_PHASE = 0.6


# --- PNG ---------------------------------------------------------------------------------------


def png(w: int, h: int, raw: bytes, channels: int) -> bytes:
    def chunk(kind: bytes, data: bytes) -> bytes:
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    color_type = 6 if channels == 4 else 2
    header = struct.pack(">IIBBBBB", w, h, 8, color_type, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def save(name: str, w: int, h: int, rows, channels: int) -> None:
    raw = bytearray()
    for row in rows:
        raw.append(0)  # filtre « aucun »
        raw.extend(row)
    path = OUT / name
    path.write_bytes(png(w, h, bytes(raw), channels))
    print(f"{path.relative_to(OUT.parents[2])} ({w} × {h})")


def to8(v: float) -> int:
    return max(0, min(255, round(v * 255.0)))


def srgb(linear: float) -> float:
    linear = max(0.0, linear)
    if linear <= 0.0031308:
        return 12.92 * linear
    return 1.055 * linear ** (1.0 / 2.4) - 0.055


def linear(s: float) -> float:
    if s <= 0.04045:
        return s / 12.92
    return ((s + 0.055) / 1.055) ** 2.4


# --- Bruit -------------------------------------------------------------------------------------


def hash3(i: int, j: int, k: int) -> float:
    h = (i * 73856093) ^ (j * 19349663) ^ (k * 83492791)
    h &= 0xFFFFFFFFFFFFFFFF
    h ^= h >> 13
    h = (h * 0x5BD1E995) & 0xFFFFFFFFFFFFFFFF
    h ^= h >> 15
    return (h & 0xFFFF) / 65536.0


def value_noise(x: float, y: float, z: float) -> float:
    xi, yi, zi = math.floor(x), math.floor(y), math.floor(z)
    fx, fy, fz = x - xi, y - yi, z - zi
    fx = fx * fx * (3 - 2 * fx)
    fy = fy * fy * (3 - 2 * fy)
    fz = fz * fz * (3 - 2 * fz)
    out = []
    for dk in (0, 1):
        k = zi + dk
        a = hash3(xi, yi, k) + (hash3(xi + 1, yi, k) - hash3(xi, yi, k)) * fx
        b = hash3(xi, yi + 1, k) + (hash3(xi + 1, yi + 1, k) - hash3(xi, yi + 1, k)) * fx
        out.append(a + (b - a) * fy)
    return out[0] + (out[1] - out[0]) * fz


def fbm(x: float, y: float, z: float, octaves: int = 3) -> float:
    total, norm = 0.0, 0.0
    for o in range(octaves):
        f = float(1 << o)
        total += value_noise(x * f, y * f, z * f) / f
        norm += 1.0 / f
    return total / norm


# --- Fond de la galaxie -------------------------------------------------------------------------


def surface(dx: float, dz: float):
    """Brillance de surface (bulbe, bras, disque) en coordonnées normalisées par les rayons."""
    rho = math.hypot(dx, dz)
    if rho > 1.25:
        return 0.0, 0.0, 0.0, rho
    edge = min(1.0, max(0.0, (1.25 - rho) / 0.3))
    bulge = 6.0 * math.exp(-(rho * rho) / (0.11 * 0.11))
    disk = math.exp(-rho / 0.38) * edge
    phi = math.atan2(dz, dx)
    psi = phi - math.log(rho + 0.04) / math.tan(math.radians(ARM_PITCH_DEG)) - ARM_PHASE
    # `%` de Python sur un flottant à diviseur positif : le `rem_euclid` de Rust.
    wrapped = abs((psi % math.pi) - math.pi / 2)
    arm_shape = math.exp(-((math.pi / 2 - wrapped) ** 2) / (2 * ARM_WIDTH * ARM_WIDTH))
    arms = 1.6 * arm_shape * disk * min(1.0, max(0.0, (rho - 0.12) / 0.1))
    return bulge, arms, disk, rho


def galaxy() -> None:
    w, h = GALAXY_W, GALAXY_H
    rng = random.Random(425)
    bg = [linear(c / 255.0) for c in BACKGROUND]
    # Radiance linéaire, ajoutée au fond puis comprimée en douceur.
    img = [[0.0, 0.0, 0.0] for _ in range(w * h)]
    for j in range(h):
        dz = (2.0 * (j + 0.5) / h - 1.0) * MARGIN
        for i in range(w):
            dx = (2.0 * (i + 0.5) / w - 1.0) * MARGIN
            bulge, arms, disk, rho = surface(dx, dz)
            if rho > 1.25:
                continue
            # Poussières : nappes sombres qui hachent le disque et les bras (pas le bulbe).
            x, z = dx * RADIUS_X, dz * RADIUS_Y
            dust = max(0.0, fbm(x * 0.035, 3.1, z * 0.035) - 0.48) * 3.2
            dust = min(1.0, dust) * min(1.0, rho / 0.18)
            # Grain du disque : la lumière n'est jamais lisse.
            grain = 0.75 + 0.5 * fbm(x * 0.09, 7.7, z * 0.09, 2)
            a = arms * grain * (1.0 - 0.75 * dust)
            d = disk * (0.8 + 0.4 * grain) * (1.0 - 0.55 * dust)
            b = bulge
            # Couleurs du ciel (P17) : bulbe chaud, bras bleutés, disque neutre ; réglées sombres
            # pour que la carte reste lisible par-dessus.
            px = img[j * w + i]
            px[0] = 0.042 * b + 0.022 * a + 0.013 * d
            px[1] = 0.030 * b + 0.036 * a + 0.012 * d
            px[2] = 0.016 * b + 0.068 * a + 0.014 * d

    # Étoiles : tirées selon la brillance du disque (rejet), éclat discret.
    def splat(x: float, y: float, rgb, radius: float) -> None:
        r = int(math.ceil(radius * 2))
        for yy in range(int(y) - r, int(y) + r + 1):
            if not 0 <= yy < h:
                continue
            for xx in range(int(x) - r, int(x) + r + 1):
                if not 0 <= xx < w:
                    continue
                d2 = (xx + 0.5 - x) ** 2 + (yy + 0.5 - y) ** 2
                k = math.exp(-d2 / (2 * radius * radius))
                px = img[yy * w + xx]
                for c in range(3):
                    px[c] += rgb[c] * k

    placed = 0
    while placed < 9000:
        dx = rng.uniform(-MARGIN, MARGIN)
        dz = rng.uniform(-MARGIN, MARGIN)
        bulge, arms, disk, rho = surface(dx, dz)
        density = min(1.0, 0.04 + 0.5 * disk + 0.9 * arms + 0.05 * bulge)
        if rho > 1.3 or rng.random() > density:
            continue
        x = (dx / MARGIN + 1.0) * 0.5 * w
        y = (dz / MARGIN + 1.0) * 0.5 * h
        t = rng.random()
        tint = (1.0, 0.80, 0.60) if t < 0.35 else (1.0, 0.95, 0.88) if t < 0.75 else (0.75, 0.85, 1.0)
        bright = 0.012 + 0.05 * rng.random() ** 3
        if rng.random() < 0.02:
            bright *= 4.0
        splat(x, y, [c * bright for c in tint], 0.55)
        placed += 1
    # Régions d'étoiles jeunes le long des bras : petites nappes rosées (génériques, sans nom).
    knots = 0
    while knots < 140:
        dx = rng.uniform(-1.1, 1.1)
        dz = rng.uniform(-1.1, 1.1)
        bulge, arms, disk, rho = surface(dx, dz)
        if rho < 0.25 or rho > 1.05 or arms < 0.35 * disk or rng.random() > arms * 3.0:
            continue
        x = (dx / MARGIN + 1.0) * 0.5 * w
        y = (dz / MARGIN + 1.0) * 0.5 * h
        k = 0.010 + 0.018 * rng.random()
        splat(x, y, (k * 1.0, k * 0.30, k * 0.55), 1.2 + 1.6 * rng.random())
        knots += 1
    # Quelques étoiles d'avant-plan hors du disque (la carte n'est pas un trou noir).
    for _ in range(500):
        x, y = rng.uniform(0, w), rng.uniform(0, h)
        dx = (2.0 * x / w - 1.0) * MARGIN
        dz = (2.0 * y / h - 1.0) * MARGIN
        if math.hypot(dx, dz) < 1.2:
            continue
        # Le bord doit rester au fond exact : pas d'étoile à moins de 6 pixels du cadre.
        if min(x, y, w - x, h - y) < 6:
            continue
        splat(x, y, [0.006 + 0.012 * rng.random()] * 3, 0.5)

    rows = []
    for j in range(h):
        row = bytearray()
        for i in range(w):
            px = img[j * w + i]
            for c in range(3):
                # Compression douce : le bulbe sature sans écrêter, le reste est linéaire.
                v = px[c]
                v = 0.85 * (1.0 - math.exp(-v / 0.85))
                row.append(to8(srgb(bg[c] + v)))
        rows.append(row)
    # Le cadre au fond exact (aucune couture avec la couleur de fond de la caméra).
    for j in (0, h - 1):
        rows[j] = bytearray(BACKGROUND * w)
    for row in rows:
        row[0:3] = bytes(BACKGROUND)
        row[-3:] = bytes(BACKGROUND)
    save("galaxy.png", w, h, rows, 3)


# --- Nébuleuse ---------------------------------------------------------------------------------

NEBULA_SIZE = 256
# Demi-côté de l'image, en rayons de la nébuleuse (le jeu pose le quadrilatère à cette taille).
NEBULA_EXTENT = 1.45


def nebula() -> None:
    n = NEBULA_SIZE
    rows = []
    for j in range(n):
        row = bytearray()
        for i in range(n):
            x = (2.0 * (i + 0.5) / n - 1.0) * NEBULA_EXTENT
            y = (2.0 * (j + 0.5) / n - 1.0) * NEBULA_EXTENT
            r = math.hypot(x, y)
            ring = math.exp(-(((r - 0.72) / 0.21) ** 2))
            # Arc ouvert vers l'est (angle 0), comme dans le ciel ; les pointes s'effilent.
            angle = abs(math.atan2(y, x))
            t = min(1.0, max(0.0, (angle - 0.55) / 1.4))
            arc = t * t * (3 - 2 * t)
            t = min(1.0, max(0.0, (angle - 0.2) / 2.2))
            haze_arc = t * t * (3 - 2 * t)
            # Filaments à deux échelles.
            fil = max(0.0, fbm(x * 5.0, 1.7, y * 5.0) - 0.28) * 2.2
            fine = 0.6 + 0.8 * fbm(x * 13.0, 4.3, y * 13.0, 2)
            haze = 0.16 * math.exp(-((r / 0.85) ** 2)) * haze_arc
            density = ring * arc * (0.25 + fil * fine) + haze * (0.5 + fil)
            # Bord de l'image transparent.
            fade = min(1.0, max(0.0, (NEBULA_EXTENT - r) / 0.25))
            alpha = min(0.85, 1.6 * density) * fade
            mix = min(1.0, max(0.0, fbm(x * 1.4, 9.1, y * 1.4) * 1.6 - 0.3))
            red = (0.95, 0.30, 0.42)
            violet = (0.60, 0.32, 0.95)
            col = [red[c] * (1 - mix) + violet[c] * mix for c in range(3)]
            # Cœur des filaments plus clair.
            glow = min(1.0, density * 1.2)
            col = [c + (1.0 - c) * 0.35 * glow for c in col]
            row.extend((to8(col[0]), to8(col[1]), to8(col[2]), to8(alpha)))
        rows.append(row)
    save("nebula.png", n, n, rows, 4)


# --- Icônes ------------------------------------------------------------------------------------
# Une icône = des calques peints l'un sur l'autre. Chaque calque : une distance signée (en pixels
# de l'image, négative à l'intérieur), une luminance et une opacité ; anticrénelage par la distance.


def circle(cx: float, cy: float, r: float):
    return lambda x, y: math.hypot(x - cx, y - cy) - r


def ring(cx: float, cy: float, r: float, width: float):
    return lambda x, y: abs(math.hypot(x - cx, y - cy) - r) - width / 2


def segment(ax: float, ay: float, bx: float, by: float, width: float):
    def d(x, y):
        vx, vy = bx - ax, by - ay
        t = max(0.0, min(1.0, ((x - ax) * vx + (y - ay) * vy) / (vx * vx + vy * vy)))
        return math.hypot(x - (ax + vx * t), y - (ay + vy * t)) - width / 2

    return d


def polygon(points):
    """Distance signée à un polygone (formule d'Inigo Quilez)."""

    def d(x, y):
        n = len(points)
        best = (x - points[0][0]) ** 2 + (y - points[0][1]) ** 2
        sign = 1.0
        j = n - 1
        for i in range(n):
            ex, ey = points[j][0] - points[i][0], points[j][1] - points[i][1]
            wx, wy = x - points[i][0], y - points[i][1]
            t = max(0.0, min(1.0, (wx * ex + wy * ey) / (ex * ex + ey * ey)))
            bx, by = wx - ex * t, wy - ey * t
            best = min(best, bx * bx + by * by)
            c1 = y >= points[i][1]
            c2 = y < points[j][1]
            c3 = ex * wy > ey * wx
            if (c1 and c2 and c3) or (not c1 and not c2 and not c3):
                sign = -sign
            j = i
        return sign * math.sqrt(best)

    return d


def union(*fs):
    return lambda x, y: min(f(x, y) for f in fs)


def subtract(a, b):
    return lambda x, y: max(a(x, y), -b(x, y))


def grow(f, amount: float):
    return lambda x, y: f(x, y) - amount


def icon(name: str, size: int, layers) -> None:
    """`layers` : liste de (distance, luminance, opacité) ; luminance et opacité peuvent être
    des fonctions de (x, y)."""
    rows = []
    for j in range(size):
        row = bytearray()
        for i in range(size):
            x, y = i + 0.5, j + 0.5
            c_acc, a_acc = 0.0, 0.0
            for dist, lum, alpha in layers:
                cov = min(1.0, max(0.0, 0.5 - dist(x, y)))
                if cov <= 0.0:
                    continue
                l = lum(x, y) if callable(lum) else lum
                a = (alpha(x, y) if callable(alpha) else alpha) * cov
                c_acc = l * a + c_acc * (1.0 - a)
                a_acc = a + a_acc * (1.0 - a)
            c = c_acc / a_acc if a_acc > 1e-6 else 0.0
            v = to8(c)
            row.extend((v, v, v, to8(a_acc)))
        rows.append(row)
    save(f"{name}.png", size, size, rows, 4)


DARK = 0.0  # luminance du liseré (reste noir une fois teinté)
OUTLINE = 0.55  # opacité du liseré


def icons() -> None:
    # Système stellaire (56 → 28) : étoile, orbite brisée, une planète sur l'orbite.
    s, c = 56, 28.0
    orbit = subtract(ring(c, c, 20.0, 3.2), union(circle(c + 14.1, c + 14.1, 4.0), circle(c - 14.1, c - 14.1, 4.0)))
    planet_x, planet_y = c + 20.0 * math.cos(-1.1), c + 20.0 * math.sin(-1.1)
    icon(
        "system",
        s,
        [
            (circle(c, c, 26.0), 1.0, lambda x, y: 0.22 * max(0.0, 1.0 - math.hypot(x - c, y - c) / 26.0) ** 2),
            (grow(orbit, 1.6), DARK, OUTLINE),
            (orbit, 0.9, 0.9),
            (circle(c, c, 11.5), DARK, OUTLINE),
            (circle(c, c, 9.0), 1.0, 1.0),
            (circle(planet_x, planet_y, 5.0), DARK, OUTLINE),
            (circle(planet_x, planet_y, 3.4), 1.0, 1.0),
        ],
    )

    # Monde isolé (44 → 22) : disque, terminateur (lumière en haut à gauche), liseré d'atmosphère.
    s, c = 44, 22.0

    def shade(x, y):
        nx, ny = (x - c) / 15.0, (y - c) / 15.0
        lit = -0.62 * nx - 0.62 * ny + 0.5 * math.sqrt(max(0.0, 1.0 - nx * nx - ny * ny))
        return 0.38 + 0.62 * min(1.0, max(0.0, lit * 1.6 + 0.35))

    icon(
        "world",
        s,
        [
            (circle(c, c, 18.0), DARK, OUTLINE),
            (circle(c, c, 15.0), shade, 1.0),
            (ring(c, c, 15.2, 1.4), 1.0, 0.55),
        ],
    )

    # Planète errante (44 → 22) : disque sombre, liseré clair, orbite absente en tirets.
    dashes = subtract(ring(c, c, 19.5, 2.2), union(*[
        segment(c, c, c + 30 * math.cos(a), c + 30 * math.sin(a), 3.6)
        for a in [k * math.pi / 4 + math.pi / 8 for k in range(8)]
    ]))
    icon(
        "rogue",
        s,
        [
            (grow(dashes, 1.4), DARK, OUTLINE),
            (dashes, 0.85, 0.85),
            (circle(c, c, 16.5), DARK, OUTLINE),
            (circle(c, c, 13.5), 0.42, 1.0),
            (ring(c, c, 13.0, 2.0), 1.0, 0.9),
        ],
    )

    # Monde détruit (44 → 22) : disque fendu, un éclat arraché.
    cracks = union(
        segment(c - 6, c - 15, c + 1, c - 2, 2.4),
        segment(c + 1, c - 2, c - 4, c + 15, 2.4),
        segment(c + 1, c - 2, c + 15, c + 3, 2.4),
    )
    chunk = polygon([(c + 6, c - 16), (c + 17, c - 9), (c + 9, c - 5)])
    body = subtract(subtract(circle(c, c, 15.0), cracks), chunk)
    icon(
        "ruin",
        s,
        [
            (grow(body, 2.6), DARK, OUTLINE),
            (body, 0.72, 1.0),
            (polygon([(c + 12, c - 18), (c + 19, c - 14), (c + 15, c - 12)]), 0.72, 0.9),
        ],
    )

    # Station (40 → 20) : hexagone, moyeu, trois rayons.
    s, c = 40, 20.0
    hexa = [(c + 15 * math.cos(math.pi / 6 + k * math.pi / 3), c + 15 * math.sin(math.pi / 6 + k * math.pi / 3)) for k in range(6)]
    frame = subtract(polygon(hexa), grow(polygon(hexa), -3.4))
    spokes = union(*[segment(c, c, c + 13 * math.cos(a), c + 13 * math.sin(a), 2.6) for a in (-math.pi / 2, math.pi / 6, 5 * math.pi / 6)])
    shape = union(frame, spokes, circle(c, c, 4.6))
    icon("station", s, [(grow(shape, 2.0), DARK, OUTLINE), (shape, 1.0, 1.0)])

    # Hub de Portails (32 → 16) : anneau épais à quatre encoches et un anneau intérieur.
    s, c = 32, 16.0
    notches = union(*[segment(c, c, c + 20 * math.cos(a), c + 20 * math.sin(a), 3.0) for a in (0, math.pi / 2, math.pi, 3 * math.pi / 2)])
    gate = union(subtract(ring(c, c, 11.5, 4.0), notches), ring(c, c, 5.0, 2.4))
    icon("hub", s, [(grow(gate, 1.8), DARK, OUTLINE), (gate, 1.0, 1.0)])

    # Noyau galactique (40 → 20) : disque d'accrétion aplati, horizon noir, anneau de photons.
    s, c = 40, 20.0

    def disk_alpha(x, y):
        e = ((x - c) / 18.0) ** 2 + ((y - c) / 6.0) ** 2
        return 0.75 * max(0.0, 1.0 - e)

    disk_shape = lambda x, y: (math.hypot((x - c) / 18.0, (y - c) / 6.0) - 1.0) * 6.0
    icon(
        "core",
        s,
        [
            (disk_shape, 1.0, disk_alpha),
            (circle(c, c, 9.0), 1.0, 0.95),
            (circle(c, c, 6.8), DARK, 1.0),
        ],
    )

    # Nébuleuse (légende, 40 → 20) : petit croissant filamenteux.
    def neb_alpha(x, y):
        nx, ny = (x - c) / 15.0, (y - c) / 15.0
        r = math.hypot(nx, ny)
        ring_v = math.exp(-(((r - 0.72) / 0.26) ** 2))
        arc = min(1.0, max(0.0, (abs(math.atan2(ny, nx)) - 0.8) / 0.9))
        fil = 0.55 + 0.45 * fbm(nx * 4.0, 2.2, ny * 4.0)
        return min(1.0, 1.5 * ring_v * arc * fil)

    icon("nebula_icon", s, [(circle(c, c, 19.0), 1.0, neb_alpha)])

    # Flotte (36 → 18) : chevron pointé vers le haut.
    s, c = 36, 18.0
    chevron = polygon([(c, c - 13), (c + 11, c + 11), (c, c + 5), (c - 11, c + 11)])
    icon("fleet", s, [(grow(chevron, 2.6), DARK, OUTLINE + 0.15), (chevron, 1.0, 1.0)])

    # Position du joueur (80 → 40) : réticule, anneau, quatre traits, point central.
    s, c = 80, 40.0
    ticks = union(
        segment(c, c - 34, c, c - 22, 3.6),
        segment(c, c + 22, c, c + 34, 3.6),
        segment(c - 34, c, c - 22, c, 3.6),
        segment(c + 22, c, c + 34, c, 3.6),
    )
    reticle = union(ring(c, c, 17.0, 3.2), ticks, circle(c, c, 3.4))
    icon("here", s, [(grow(reticle, 2.0), DARK, OUTLINE), (reticle, 1.0, 1.0)])

    # Destination (80 → 40) : quatre équerres et un losange.
    brackets = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            x0, y0 = c + sx * 26, c + sy * 26
            brackets.append(segment(x0, y0, x0 - sx * 11, y0, 3.4))
            brackets.append(segment(x0, y0, x0, y0 - sy * 11, 3.4))
    diamond = polygon([(c, c - 7), (c + 7, c), (c, c + 7), (c - 7, c)])
    target = union(*brackets, diamond)
    icon("target", s, [(grow(target, 2.0), DARK, OUTLINE), (target, 1.0, 1.0)])


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    icons()
    nebula()
    galaxy()
    return 0


if __name__ == "__main__":
    sys.exit(main())
