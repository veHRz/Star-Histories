"""Textures de la planète Hexis (M9, lot L6) : un monde-usine très urbanisé (Q17).

Le canon (§3.3) dit « monde-usine et place forte », ancien monde industriel ravagé, ≈ 1,5 Md
d'habitants. L'auteur a retenu (Q17) : nappes de villes et de complexes industriels visibles de
jour, lumières des villes côté nuit, zones ravagées encore visibles. Rien n'est nommé ici :
aucune ville, aucun lieu, seulement l'aspect général.

Produit, dans `assets/textures/planets/` :

- `hexis_color_4k.png`, `hexis_color_2k.png` : couleur de la surface, équirectangulaire, sRGB
  (RGB) ; 4096 × 2048 pour le préréglage Haut, 2048 × 1024 pour Bas et Moyen ;
- `hexis_surface_2k.png` : données de surface, équirectangulaire, linéaire (RGBA) :
  R lumières de nuit, G densité urbaine, B industrie, A eau ;
- `urban_detail.png` : détail carrelable 1024 × 1024, linéaire (RGBA) : R trame urbaine (rues,
  îlots, toits), G éclairage nocturne des rues, B toits industriels (halles, cuves, faisceaux de
  voies), A relief (cratères, gravats, fosses). Le jeu le plaque en projection triplanaire à deux
  échelles et divise chaque canal par sa moyenne (dernier niveau de mipmap) : le détail module
  la carte de couleur sans la décaler.

Repère des cartes (contrat avec `assets/shaders/planet.wesl`) : la ligne j a pour latitude
π/2 − π (j + ½) / H, la colonne i pour longitude 2π (i + ½) / W, et la direction du point est
(cos lat cos lon, sin lat, cos lat sin lon). Le bruit est échantillonné en 3D sur la sphère : pas
de couture en longitude ni aux pôles. Le spatioport d'Hexis (P7, orbite basse au-dessus de la
latitude 49°, longitude 0 de ce repère) et L'Enclume regardent la même région : c'est la vitrine
(mégapole, ceintures d'usines, golfe pollué, zone ravagée à l'horizon).

Python de Blender (numpy), en arrière-plan, sans scène :

    blender --background --factory-startup --python tools/planets/hexis.py -- [options]

Options : `--size 4096` (largeur de la grande carte ; 1024 pour un essai rapide, qui n'écrit que
les aperçus), `--previews` (aperçus « comme en jeu » dans `target/art-previews/m9-lotL6/`),
`--no-write` (aperçus seulement).
"""

import math
import struct
import sys
import time
import zlib
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "textures" / "planets"
PREVIEWS = ROOT / "target" / "art-previews" / "m9-lotL6"

F32 = np.float32
TAU = 2.0 * math.pi

# Vitrine, en (latitude, longitude) en degrés du repère des cartes.
SPACEPORT = (49.0, 0.0)
GULF = (39.0, -15.0)
RAVAGED = (63.0, 27.0)
BASIN = (30.0, 21.0)


# --------------------------------------------------------------------------------------------
# Bruit
# --------------------------------------------------------------------------------------------

_M1 = np.uint32(0x9E3779B1)
_M2 = np.uint32(0x85EBCA77)
_M3 = np.uint32(0xC2B2AE3D)


def _hash3(ix, iy, iz, seed):
    h = ix * _M1
    h ^= iy * _M2
    h ^= iz * _M3
    h ^= np.uint32((seed * 0x27D4EB2F + 0x165667B1) & 0xFFFFFFFF)
    h ^= h >> np.uint32(15)
    h *= _M2
    h ^= h >> np.uint32(13)
    h *= _M3
    h ^= h >> np.uint32(16)
    return h


def _to_u32(a):
    return a.astype(np.int64).astype(np.uint32)


def gnoise(x, y, z, seed):
    """Bruit de gradient 3D, environ dans [−1, 1] (écart type ≈ 0,3)."""
    fx, fy, fz = np.floor(x), np.floor(y), np.floor(z)
    tx, ty, tz = (x - fx).astype(F32), (y - fy).astype(F32), (z - fz).astype(F32)
    ix, iy, iz = _to_u32(fx), _to_u32(fy), _to_u32(fz)
    one = np.uint32(1)

    def fade(t):
        return t * t * t * (t * (t * F32(6.0) - F32(15.0)) + F32(10.0))

    u, v, w = fade(tx), fade(ty), fade(tz)
    scale = F32(2.0 / 1023.0)

    def corner(ox, oy, oz):
        h = _hash3(ix + one * ox, iy + one * oy, iz + one * oz, seed)
        gx = (h & np.uint32(1023)).astype(F32) * scale - F32(1.0)
        gy = ((h >> np.uint32(10)) & np.uint32(1023)).astype(F32) * scale - F32(1.0)
        gz = ((h >> np.uint32(20)) & np.uint32(1023)).astype(F32) * scale - F32(1.0)
        return gx * (tx - F32(ox)) + gy * (ty - F32(oy)) + gz * (tz - F32(oz))

    x00 = corner(0, 0, 0) + u * (corner(1, 0, 0) - corner(0, 0, 0))
    x10 = corner(0, 1, 0) + u * (corner(1, 1, 0) - corner(0, 1, 0))
    x01 = corner(0, 0, 1) + u * (corner(1, 0, 1) - corner(0, 0, 1))
    x11 = corner(0, 1, 1) + u * (corner(1, 1, 1) - corner(0, 1, 1))
    y0 = x00 + v * (x10 - x00)
    y1 = x01 + v * (x11 - x01)
    return (y0 + w * (y1 - y0)) * F32(1.5)


def smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0.0, 1.0).astype(F32)
    return t * t * (F32(3.0) - F32(2.0) * t)


def mix(a, b, t):
    """Mélange de couleurs (a, b : triplets ou tableaux …×3 ; t : tableau H×W)."""
    a = np.asarray(a, dtype=F32)
    b = np.asarray(b, dtype=F32)
    t = np.asarray(t, dtype=F32)[..., None]
    return a + (b - a) * t


# --------------------------------------------------------------------------------------------
# Champs sur la sphère, multirésolution
# --------------------------------------------------------------------------------------------


class Sphere:
    """Directions des texels d'une carte équirectangulaire W × H, et grilles grossières : chaque
    octave de bruit est calculée à la résolution qu'exige sa fréquence, puis interpolée. Les
    opérations non linéaires (seuils, lignes) se font à pleine résolution : bords nets."""

    def __init__(self, width):
        self.W = width
        self.H = width // 2
        self._grids = {}
        self._warps = {}
        j = (np.arange(self.H, dtype=np.float64) + 0.5) / self.H
        self.lat = (math.pi / 2 - math.pi * j).astype(F32)
        x, y, z = self.grid(self.W)
        self.X, self.Y, self.Z = x, y, z
        # Poids d'aire des texels (quantiles pondérés).
        self.area = np.broadcast_to(np.cos(self.lat)[:, None], (self.H, self.W))

    def grid(self, w):
        if w not in self._grids:
            h = w // 2
            lat = math.pi / 2 - math.pi * (np.arange(h) + 0.5) / h
            lon = TAU * (np.arange(w) + 0.5) / w
            cl = np.cos(lat)[:, None]
            x = (cl * np.cos(lon)[None, :]).astype(F32)
            y = np.broadcast_to(np.sin(lat)[:, None], (h, w)).astype(F32)
            z = (cl * np.sin(lon)[None, :]).astype(F32)
            self._grids[w] = (x, y, z)
        return self._grids[w]

    def upsample(self, a):
        """Interpolation bilinéaire d'une carte h × w vers H × W (raccord en longitude)."""
        h, w = a.shape
        if w == self.W:
            return a
        sx = (np.arange(self.W) + 0.5) * w / self.W - 0.5
        x0 = np.floor(sx).astype(np.int64)
        wx = (sx - x0).astype(F32)
        x1 = (x0 + 1) % w
        x0 %= w
        rows = a[:, x0] * (F32(1.0) - wx) + a[:, x1] * wx
        sy = np.clip((np.arange(self.H) + 0.5) * h / self.H - 0.5, 0, h - 1)
        y0 = np.floor(sy).astype(np.int64)
        y1 = np.minimum(y0 + 1, h - 1)
        wy = (sy - y0).astype(F32)[:, None]
        return rows[y0] * (F32(1.0) - wy) + rows[y1] * wy

    def _warp(self, w, amp, freq, seed):
        key = (w, amp, freq, seed)
        if key not in self._warps:
            x, y, z = self.grid(w)
            out = []
            for k in range(3):
                n = gnoise(x * freq, y * freq, z * freq, seed + 7 * k)
                n += 0.5 * gnoise(x * freq * 2.1, y * freq * 2.1, z * freq * 2.1, seed + 7 * k + 3)
                out.append(n * F32(amp))
            self._warps[key] = out
        return self._warps[key]

    def field(self, freq, octaves, seed, gain=0.5, lac=2.03, warp=None, stretch=(1.0, 1.0, 1.0)):
        """Bruit fractal (fbm) normalisé : moyenne 0, écart type 1 (pondéré par l'aire).
        `stretch` multiplie la fréquence par axe (traînées le long des parallèles si y > 1)."""
        total = np.zeros((self.H, self.W), F32)
        amp = 1.0
        sx, sy, sz = (F32(s) for s in stretch)
        for o in range(octaves):
            f = freq * lac**o
            w = 64
            while w < min(self.W, 28 * f * max(stretch)):
                w *= 2
            w = min(w, self.W)
            x, y, z = self.grid(w)
            if warp is not None:
                wx, wy, wz = self._warp(w, *warp)
                x, y, z = x + wx, y + wy, z + wz
            n = gnoise(x * F32(f) * sx, y * F32(f) * sy, z * F32(f) * sz, seed * 131 + o)
            total += F32(amp) * self.upsample(n)
            amp *= gain
        return self.normalize(total)

    def normalize(self, a):
        weights = self.area
        mean = float((a * weights).sum() / weights.sum())
        std = float(np.sqrt((((a - mean) ** 2) * weights).sum() / weights.sum()))
        return ((a - mean) / max(std, 1e-6)).astype(F32)

    def quantile(self, a, q, mask=None):
        """Quantile pondéré par l'aire (et par un masque éventuel)."""
        step = max(1, self.W // 512)
        sub = a[::step, ::step].ravel()
        wts = (self.area[::step, ::step] * (1.0 if mask is None else mask[::step, ::step])).ravel()
        order = np.argsort(sub)
        cum = np.cumsum(wts[order])
        idx = np.searchsorted(cum, q * cum[-1])
        return float(sub[order[min(idx, len(sub) - 1)]])

    def direction(self, lat_deg, lon_deg):
        lat, lon = math.radians(lat_deg), math.radians(lon_deg)
        return (math.cos(lat) * math.cos(lon), math.sin(lat), math.cos(lat) * math.sin(lon))

    def angle_to(self, lat_deg, lon_deg):
        cx, cy, cz = self.direction(lat_deg, lon_deg)
        dot = self.X * F32(cx) + self.Y * F32(cy) + self.Z * F32(cz)
        return np.arccos(np.clip(dot, -1.0, 1.0)).astype(F32)

    def bump(self, center, radius_deg):
        a = self.angle_to(*center) / F32(math.radians(radius_deg))
        return np.exp(-a * a).astype(F32)


def line(n, width):
    """Lignes fines là où un champ lisse s'annule (réseaux d'axes, cicatrices, fissures)."""
    return (F32(1.0) - smoothstep(0.0, width, np.abs(n))).astype(F32)


# --------------------------------------------------------------------------------------------
# Cratères
# --------------------------------------------------------------------------------------------


def stamp_craters(sp, centers, radii):
    """Masques des cratères : fond (vitrifié), rebord (cendres claires), éjectas. Chaque cratère
    n'est calculé que sur son voisinage (boîte en latitude et longitude, raccord en longitude)."""
    H, W = sp.H, sp.W
    floor = np.zeros((H, W), F32)
    rim = np.zeros((H, W), F32)
    ejecta = np.zeros((H, W), F32)
    for (clat, clon), r in zip(centers, radii):
        reach = 2.2 * r
        lat0, lat1 = clat + math.degrees(reach), clat - math.degrees(reach)
        j0 = max(0, int((90.0 - min(lat0, 90.0)) / 180.0 * H) - 1)
        j1 = min(H, int((90.0 - max(lat1, -90.0)) / 180.0 * H) + 2)
        coslat = max(math.cos(math.radians(clat)), 1e-3)
        span = math.degrees(reach) / coslat
        if span >= 180.0 or lat0 >= 89.0 or lat1 <= -89.0:
            cols = np.arange(W)
        else:
            c = clon % 360.0 / 360.0 * W
            half = span / 360.0 * W + 2
            cols = np.arange(int(c - half), int(c + half) + 1) % W
        cx, cy, cz = sp.direction(clat, clon)
        px = sp.X[j0:j1][:, cols]
        py = sp.Y[j0:j1][:, cols]
        pz = sp.Z[j0:j1][:, cols]
        ang = np.arccos(np.clip(px * cx + py * cy + pz * cz, -1.0, 1.0)) / r
        f = F32(1.0) - smoothstep(0.72, 0.86, ang)
        rm = np.exp(-(((ang - 0.93) / 0.09) ** 2)).astype(F32)
        ej = ((F32(1.0) - smoothstep(1.0, 2.1, ang)) * (ang > 0.95)).astype(F32)
        sl = (slice(j0, j1), cols)
        floor[sl] = np.maximum(floor[sl], f)
        rim[sl] = np.maximum(rim[sl], rm)
        ejecta[sl] = np.maximum(ejecta[sl], ej)
    return floor, rim, ejecta


def crater_list(sp, mask, count, rmin_km, rmax_km, rng, planet_km=640.0):
    """Cratères tirés selon un masque (zones ravagées), rayons en loi de puissance."""
    step = max(1, sp.W // 1024)
    m = (mask[::step, ::step] * sp.area[::step, ::step]).ravel().astype(np.float64)
    p = m / m.sum()
    picks = rng.choice(len(p), size=count, p=p)
    h, w = mask[::step, ::step].shape
    centers = []
    for k in picks:
        j, i = divmod(int(k), w)
        lat = 90.0 - 180.0 * (j + rng.random()) / h
        lon = 360.0 * (i + rng.random()) / w
        centers.append((lat, lon))
    u = rng.random(count)
    # Loi de puissance : beaucoup de petits, quelques grands.
    radii_km = rmin_km * (rmax_km / rmin_km) ** (u**3.5)
    return centers, radii_km / planet_km


# --------------------------------------------------------------------------------------------
# Grande carte d'Hexis
# --------------------------------------------------------------------------------------------

# Couleurs linéaires (albédo). Terres nues ocre et brûlées, villes gris froid, usines sombres et
# rouillées, zones ravagées charbonneuses, mers presque noires, sel terne des bassins asséchés.
WASTE_A = (0.150, 0.118, 0.082)
WASTE_B = (0.092, 0.078, 0.064)
TAILINGS = (0.165, 0.115, 0.075)
URBAN_DARK = (0.088, 0.088, 0.090)
URBAN_MID = (0.138, 0.136, 0.134)
URBAN_LIGHT = (0.215, 0.210, 0.200)
IND_DARK = (0.055, 0.054, 0.054)
IND_RUST = (0.175, 0.090, 0.045)
IND_METAL = (0.240, 0.250, 0.262)
CHAR = (0.034, 0.030, 0.027)
ASH = (0.150, 0.142, 0.132)
GLASS = (0.020, 0.024, 0.028)
SEA_DEEP = (0.007, 0.010, 0.013)
SEA_OIL = (0.016, 0.017, 0.013)
SEA_SHALLOW = (0.024, 0.030, 0.026)
SEA_PLUME = (0.050, 0.038, 0.022)
SALT = (0.170, 0.158, 0.136)
SALT_DARK = (0.105, 0.097, 0.084)
MUD = (0.120, 0.100, 0.078)
SMOG = (0.280, 0.255, 0.190)


def hexis_maps(width, seed=425):
    """Renvoie (couleur linéaire H×W×3, données H×W×4 dans [0, 1])."""
    t0 = time.time()
    sp = Sphere(width)
    rng = np.random.default_rng(seed)

    def log(msg):
        print(f"  [{time.time() - t0:5.1f} s] {msg}", flush=True)

    # Continents : bruit déformé, côtes découpées par les octaves fines.
    cont = sp.field(1.05, 6, 1, warp=(0.42, 1.4, 11))
    cont += F32(0.30) * sp.field(22.0, 3, 2)
    cont += F32(1.5) * sp.bump(SPACEPORT, 10.0)
    cont -= F32(2.4) * sp.bump(GULF, 6.5)
    cont -= F32(1.6) * sp.bump(BASIN, 6.0)
    cont += F32(0.8) * sp.bump(RAVAGED, 9.0)
    sea_level = sp.quantile(cont, 0.36)
    relief = cont - F32(sea_level)
    land = smoothstep(-0.015, 0.015, relief)
    coast = np.exp(-np.abs(relief) / F32(0.25)).astype(F32)
    log("continents")

    # Bassins asséchés : d'anciennes mers (une sur cinq), croûte de sel, rivages fossiles.
    # Peu nombreux mais grands : de petites plaques claires dans les mers ressembleraient à de
    # la banquise.
    basin_f = sp.field(0.9, 3, 3) + F32(3.0) * sp.bump(BASIN, 6.5)
    sea = F32(1.0) - land
    dried = sea * smoothstep(0.0, 0.3, basin_f - F32(sp.quantile(basin_f, 0.86, sea)))
    log("bassins")

    # Densité urbaine : mégarégions, centres, villes secondaires, et la mégapole du spatioport.
    mega = sp.field(2.0, 5, 4, warp=(0.30, 2.0, 21))
    cores_f = sp.field(8.5, 4, 5)
    urban_raw = (
        F32(0.6) * mega
        + F32(0.45) * cores_f
        + F32(0.35) * coast
        + F32(3.0) * sp.bump(SPACEPORT, 5.0)
        + F32(1.0) * sp.bump(SPACEPORT, 14.0)
    )
    q_u = sp.quantile(urban_raw, 0.42, land)
    # Bord assez net : de loin, les nappes urbaines sont des formes, pas un camaïeu.
    urban = land * smoothstep(q_u - 0.12, q_u + 0.30, urban_raw)
    cores = smoothstep(1.0, 2.0, cores_f + F32(2.5) * sp.bump(SPACEPORT, 2.2)) * urban
    towns = smoothstep(0.7, 1.9, sp.field(38.0, 2, 20)) * urban
    log("villes")

    # Axes de transport : réseaux de lignes (lieux où des champs lisses s'annulent).
    a1 = sp.field(4.0, 2, 6)
    a2 = sp.field(11.0, 2, 7)
    a3 = sp.field(26.0, 2, 8)
    reach = smoothstep(-0.2, 0.9, urban_raw - F32(q_u))
    axes = np.maximum.reduce(
        [line(a1, 0.030), F32(0.75) * line(a2, 0.040), F32(0.45) * line(a3, 0.060)]
    )
    axes = axes * land * reach
    log("axes")

    # Industrie : près des côtes, des axes, en ceinture autour de la mégapole.
    ring = sp.bump(SPACEPORT, 9.0) - sp.bump(SPACEPORT, 3.5)
    ind_f = sp.field(5.5, 5, 9) + F32(0.6) * coast + F32(0.5) * axes + F32(1.6) * ring
    q_i = sp.quantile(ind_f, 0.74, land)
    industry = land * smoothstep(q_i - 0.15, q_i + 0.35, ind_f) * (F32(1.0) - F32(0.7) * cores)
    log("industrie")

    # Zones ravagées : grandes plaies de l'ancienne guerre, cratères, cicatrices vitrifiées.
    rav_f = sp.field(1.5, 5, 10, warp=(0.35, 1.8, 31)) + F32(3.0) * sp.bump(RAVAGED, 8.0)
    q_r = sp.quantile(rav_f, 0.87, land)
    ravaged = (
        land
        * smoothstep(q_r - 0.2, q_r + 0.35, rav_f)
        * (F32(1.0) - F32(0.8) * sp.bump(SPACEPORT, 6.0))
    )
    scars = line(sp.field(6.5, 3, 12), 0.05) * ravaged
    centers, radii = crater_list(sp, ravaged**3 + F32(0.0005) * land, 700, 1.0, 40.0, rng)
    c_floor, c_rim, c_ejecta = stamp_craters(sp, centers, radii)
    destroyed = np.maximum(ravaged, c_floor)
    urban = urban * (F32(1.0) - destroyed)
    industry = industry * (F32(1.0) - destroyed)
    cores = cores * (F32(1.0) - destroyed)
    towns = towns * (F32(1.0) - destroyed)
    axes = axes * (F32(1.0) - F32(0.85) * destroyed)
    log(f"zones ravagées ({len(radii)} cratères)")

    # Smog : nappes jaunâtres au-dessus des villes et des usines.
    # Traînées étirées le long des parallèles du repère (vents zonaux) plutôt que des taches.
    smog_f = sp.field(2.2, 5, 13, warp=(0.5, 1.6, 41), stretch=(1.0, 3.5, 1.0))
    # Surtout au-dessus des terres : les mers restent sombres.
    smog = smoothstep(0.2, 1.6, smog_f) * (
        F32(0.12) + F32(0.25) * land + F32(0.63) * np.maximum(urban, industry)
    )
    log("smog")

    # Textures fines de la carte (mouchetures, rouille, toits, terrils).
    n_fine = sp.field(90.0, 2, 14)
    n_mid = sp.field(30.0, 3, 15)
    n_rust = sp.field(14.0, 3, 16)

    # --- Couleur ---
    # Terres nues : variation douce ; la rouille (terrils, scories) se tient autour des usines.
    halo = smoothstep(q_i - 0.9, q_i, ind_f)
    waste = mix(WASTE_A, WASTE_B, smoothstep(-2.0, 2.0, F32(0.6) * n_mid + F32(0.6) * n_rust))
    waste = mix(waste, TAILINGS, smoothstep(0.2, 1.4, n_rust) * halo * F32(0.75))
    # Gris froid, plus sombre que les terres ocre : les villes se lisent par la teinte, sans
    # ressembler à du givre ; les centres et les villes secondaires sont plus clairs.
    urban_col = mix(URBAN_DARK, URBAN_MID, smoothstep(-1.4, 1.0, F32(0.5) * n_fine + F32(0.7) * n_mid))
    dense = np.clip(cores * F32(0.8) + towns * F32(0.4) + F32(0.05) * n_fine, 0, 1)
    urban_col = mix(urban_col, URBAN_LIGHT, dense)
    ind_col = mix(IND_DARK, IND_RUST, smoothstep(-0.3, 1.5, n_rust))
    ind_col = mix(ind_col, IND_METAL, smoothstep(0.8, 2.0, n_fine) * F32(0.6))
    rav_col = mix(CHAR, ASH, smoothstep(-0.4, 2.0, n_mid + F32(0.6) * n_fine))
    rav_col = mix(rav_col, GLASS, scars)

    land_col = mix(waste, urban_col, urban)
    land_col = mix(land_col, ind_col, industry)
    land_col = mix(land_col, rav_col, ravaged * F32(0.92))
    ejecta_grain = F32(0.6) + F32(0.4) * smoothstep(-1, 1, n_fine)
    land_col = mix(land_col, ASH, c_ejecta * F32(0.30) * ejecta_grain)
    land_col = mix(land_col, ASH, c_rim * F32(0.50))
    land_col = mix(land_col, GLASS, c_floor * F32(0.45))
    land_col = mix(land_col, URBAN_LIGHT, axes * F32(0.5))

    swirl = sp.field(9.0, 4, 17, warp=(0.25, 6.0, 51))
    sea_col = mix(SEA_DEEP, SEA_OIL, smoothstep(-0.5, 1.5, swirl))
    sea_col = mix(sea_col, SEA_SHALLOW, np.exp(-np.abs(relief) / F32(0.06)) * F32(0.8))
    # Rejets des usines côtières : panaches brunâtres le long des côtes industrielles.
    plume = smoothstep(q_i - 0.6, q_i + 0.4, ind_f)
    sea_col = mix(sea_col, SEA_PLUME, plume * coast * F32(0.7))

    shores = F32(0.5) + F32(0.5) * np.sin(relief * F32(70.0)).astype(F32)
    cracks = line(sp.field(40.0, 2, 18), 0.06)
    # Croûte terne et craquelée, vasières vers les anciens rivages, saumure au fond.
    salt_col = mix(SALT, SALT_DARK, np.clip(shores * F32(0.35) + cracks * F32(0.5) + F32(0.25) * smoothstep(-1.0, 1.5, n_mid), 0, 1))
    salt_col = mix(salt_col, MUD, np.exp(-np.abs(relief) / F32(0.15)) * F32(0.8))
    salt_col = mix(salt_col, SEA_OIL, smoothstep(-0.9, -1.4, relief))

    color = mix(sea_col, land_col, land)
    color = mix(color, salt_col, dried)
    color = mix(color, SMOG, smog * F32(0.30))
    log("couleur")

    # --- Lumières de nuit : centres éclatants, semis de villes, axes, usines ; banlieues
    # faibles et quartiers éteints, pour que le réseau se lise au lieu d'une nappe uniforme.
    sites = smoothstep(0.6, 1.6, n_fine) * industry
    lights = (
        urban ** F32(1.6) * (F32(0.06) + F32(0.5) * towns + F32(0.9) * cores)
        + axes * F32(0.45)
        + sites * F32(0.6)
        + industry * F32(0.05)
    )
    blackout = smoothstep(0.9, 1.3, sp.field(10.0, 2, 19))
    lights *= (F32(1.0) - F32(0.85) * blackout) * (F32(1.0) - F32(0.3) * smog)
    lights *= F32(0.6) + F32(0.4) * smoothstep(-1.5, 1.5, n_fine)
    lights *= F32(1.0) - destroyed
    lights *= land * (F32(1.0) - dried)
    lights = np.clip(lights, 0.0, 1.0)
    log("lumières")

    water = sea * (F32(1.0) - dried)
    data = np.stack([lights, urban, industry, water], axis=-1).astype(F32)
    return color.astype(F32), np.clip(data, 0.0, 1.0)


# --------------------------------------------------------------------------------------------
# Détail carrelable
# --------------------------------------------------------------------------------------------


def tile_noise(px, py, period, seed):
    """Bruit de valeur périodique (tore) : px, py dans [0, 1)."""
    x, y = px * period, py * period
    x0, y0 = np.floor(x), np.floor(y)
    tx, ty = (x - x0).astype(F32), (y - y0).astype(F32)
    tx = tx * tx * (3 - 2 * tx)
    ty = ty * ty * (3 - 2 * ty)
    i0 = (x0.astype(np.int64) % period).astype(np.uint32)
    j0 = (y0.astype(np.int64) % period).astype(np.uint32)
    i1 = ((x0.astype(np.int64) + 1) % period).astype(np.uint32)
    j1 = ((y0.astype(np.int64) + 1) % period).astype(np.uint32)
    z = np.zeros_like(i0)

    def val(i, j):
        return (_hash3(i, j, z, seed) & np.uint32(0xFFFF)).astype(F32) / F32(65535.0)

    a = val(i0, j0) + tx * (val(i1, j0) - val(i0, j0))
    b = val(i0, j1) + tx * (val(i1, j1) - val(i0, j1))
    return a + ty * (b - a)


def tile_fbm(px, py, period, octaves, seed):
    total = np.zeros_like(px, dtype=F32)
    amp, norm = 1.0, 0.0
    for o in range(octaves):
        total += F32(amp) * tile_noise(px, py, period * 2**o, seed + o)
        norm += amp
        amp *= 0.5
    return total / F32(norm)


def cell_hash(*ints, seed=0):
    """Hachage entier → [0, 1) (identifiants d'îlots, de parcelles)."""
    arrays = np.broadcast_arrays(*[np.asarray(v).astype(np.int64) for v in ints])
    h = np.full(arrays[0].shape, seed * 2654435761 & 0xFFFFFFFF, np.uint32)
    for k, v in enumerate(arrays):
        h = _hash3(v.astype(np.uint32), np.full(v.shape, k, np.uint32), h, seed)
    return (h & np.uint32(0xFFFFFF)).astype(F32) / F32(16777216.0)


def torus_germs(rng, count, min_dist):
    """Germes sur le tore, écartés d'au moins `min_dist` : sans paire trop proche, les grands
    axes (frontières de Voronoï) ne s'épaississent pas en bandes."""
    germs = []
    while len(germs) < count:
        g = rng.random(2)
        if all(np.hypot(*((g - h + 0.5) % 1.0 - 0.5)) >= min_dist for h in germs):
            germs.append(g)
    return np.array(germs)


def voronoi_torus(px, py, seeds):
    """Cellule la plus proche sur le tore, décalage vers son germe, écart aux deux plus proches."""
    best = np.full(px.shape, 9.0, F32)
    second = np.full(px.shape, 9.0, F32)
    idx = np.zeros(px.shape, np.int32)
    offx = np.zeros(px.shape, F32)
    offy = np.zeros(px.shape, F32)
    for k, (sx, sy) in enumerate(seeds):
        dx = (px - sx + 0.5) % 1.0 - 0.5
        dy = (py - sy + 0.5) % 1.0 - 0.5
        d = np.sqrt(dx * dx + dy * dy).astype(F32)
        closer = d < best
        second = np.where(closer, best, np.minimum(second, d))
        best = np.where(closer, d, best)
        idx = np.where(closer, k, idx)
        offx = np.where(closer, dx, offx)
        offy = np.where(closer, dy, offy)
    return idx, offx, offy, second - best


def grid_layer(offx, offy, angle, size, n):
    """Coordonnées locales (pixels) dans la trame tournée d'un quartier."""
    ca, sa = np.cos(angle), np.sin(angle)
    a = (offx * ca - offy * sa) * n
    b = (offx * sa + offy * ca) * n
    ia, ib = np.floor(a / size), np.floor(b / size)
    return a - ia * size, b - ib * size, ia, ib


def urban_detail(n=1024, ss=2, seed=7):
    """Détail carrelable RGBA (linéaire, [0, 1]) : trame urbaine, éclairage, industrie, relief."""
    N = n * ss
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[0:N, 0:N]
    px = ((xx + 0.5) / N).astype(F32)
    py = ((yy + 0.5) / N).astype(F32)
    # Quartiers aux limites incurvées : les grands axes suivent leurs frontières.
    wx = (tile_fbm(px, py, 4, 3, seed + 1) - 0.5) * F32(0.10)
    wy = (tile_fbm(px, py, 4, 3, seed + 2) - 0.5) * F32(0.10)
    qx, qy = (px + wx) % 1.0, (py + wy) % 1.0

    K = 18
    germs = torus_germs(rng, K, 0.14)
    did, offx, offy, gap = voronoi_torus(qx, qy, germs)
    # Type de quartier : 0 habitat en damier, 1 centre, 2 entrepôts, 3 friche, 4 vieux tissu
    # irrégulier (sans damier) : la variété casse l'effet de papier quadrillé.
    kinds = rng.choice(5, size=K, p=[0.35, 0.15, 0.17, 0.08, 0.25])
    angles = np.where(rng.random(K) < 0.5, 0.0, rng.uniform(-0.6, 0.6, K)) + rng.choice([0, math.pi / 2], K)
    sizes = np.select(
        [kinds == 0, kinds == 1, kinds == 2, kinds == 3],
        [rng.uniform(16, 24, K), rng.uniform(26, 38, K), rng.uniform(40, 60, K), rng.uniform(30, 50, K)],
        default=rng.uniform(20, 30, K),
    )
    kind = kinds[did]
    size = sizes[did].astype(F32)
    ang = angles[did].astype(F32)
    fa, fb, ia, ib = grid_layer(offx, offy, ang, size, N)
    street_w = np.select([kind == 1, kind == 4], [3.0, 0.0], default=2.0).astype(F32)
    street = np.maximum(
        F32(1.0) - smoothstep(street_w - 1.0, street_w + 0.6, np.minimum(fa, size - fa) * 2),
        F32(1.0) - smoothstep(street_w - 1.0, street_w + 0.6, np.minimum(fb, size - fb) * 2),
    )
    street = street * (kind != 4)
    major = F32(1.0) - smoothstep(4.0 / N, 9.0 / N, gap)
    # Parcelles : chaque îlot partagé en 2 × 2 (ou 1 × 3) bâtiments de teintes différentes.
    h_block = cell_hash(did, ia, ib, seed=seed + 3)
    split_a = np.where(h_block < 0.5, 0.5, 0.33 + 0.34 * cell_hash(did, ia, ib, seed=seed + 4))
    sub_a = (fa / size > split_a).astype(np.int32)
    sub_b = (fb / size > 0.5).astype(np.int32)
    parcel = cell_hash(did, ia * 2 + sub_a, ib * 2 + sub_b, seed=seed + 5)
    edge = np.minimum.reduce([np.abs(fa / size - split_a) * size, np.abs(fb / size - 0.5) * size])
    courtyard = smoothstep(1.2, 0.2, edge) * F32(0.35)
    grain = tile_fbm(px, py, 64, 3, seed + 6)

    old_town = smoothstep(0.40, 0.60, tile_fbm(px, py, 96, 3, seed + 8))
    roof = np.select(
        [kind == 0, kind == 1, kind == 2, kind == 3],
        [
            F32(0.30) + F32(0.55) * parcel,
            F32(0.42) + F32(0.50) * parcel,
            F32(0.28) + F32(0.58) * h_block,
            F32(0.22) + F32(0.25) * grain,
        ],
        default=F32(0.28) + F32(0.40) * old_town,
    ).astype(F32)
    roof = roof * (F32(1.0) - courtyard * (kind != 4)) + F32(0.14) * (grain - F32(0.5))
    vacant = (cell_hash(did, ia, ib, seed=seed + 7) < np.select([kind == 3, kind == 4], [0.6, 0.0], 0.06)).astype(F32)
    roof = roof * (F32(1.0) - vacant) + vacant * (F32(0.20) + F32(0.25) * grain)
    # Rues à peine plus sombres que la moyenne des toits : elles se devinent, sans quadriller.
    r = roof * (F32(1.0) - street) + F32(0.36) * street
    r = r * (F32(1.0) - major) + F32(0.22) * major

    # Éclairage : rues, grands axes, fenêtres et enseignes des îlots, projecteurs des entrepôts.
    lamp = F32(0.75) + F32(0.25) * np.cos((fa + fb) * F32(1.3))
    windows = np.select(
        [kind == 0, kind == 1, kind == 2, kind == 3],
        [
            F32(0.05) + F32(0.20) * parcel,
            F32(0.20) + F32(0.45) * parcel,
            F32(0.02) + F32(0.7) * (parcel > 0.95),
            F32(0.0) * parcel,
        ],
        default=F32(0.06) + F32(0.30) * old_town * grain,
    ).astype(F32)
    windows = windows * (F32(1.0) - vacant)
    g = windows * (F32(1.0) - street) + street * F32(0.45) * lamp * np.where(kind == 3, 0.3, 1.0)
    g = g * (F32(1.0) - major) + major * F32(0.95)
    g = np.clip(g, 0.0, 1.0)

    # Industrie : trame large, halles nervurées, parcs de cuves, faisceaux de voies, crassiers.
    jx = (tile_fbm(px, py, 3, 2, seed + 11) - 0.5) * F32(0.06)
    K2 = 10
    germs2 = torus_germs(rng, K2, 0.18)
    did2, ox2, oy2, gap2 = voronoi_torus((px + jx) % 1.0, (py + jx) % 1.0, germs2)
    ang2 = rng.choice([0.0, math.pi / 2, math.pi / 4], K2)[did2].astype(F32)
    size2 = rng.uniform(56, 110, K2)[did2].astype(F32)
    fa2, fb2, ia2, ib2 = grid_layer(ox2, oy2, ang2, size2, N)
    lot = cell_hash(did2, ia2, ib2, seed=seed + 12)
    road2 = np.maximum(
        F32(1.0) - smoothstep(3.0, 4.5, np.minimum(fa2, size2 - fa2) * 2),
        F32(1.0) - smoothstep(3.0, 4.5, np.minimum(fb2, size2 - fb2) * 2),
    )
    u2, v2 = fa2 / size2, fb2 / size2
    hall = (F32(0.35) + F32(0.5) * cell_hash(did2, ia2, ib2, seed=seed + 13)) * (
        F32(0.85) + F32(0.15) * np.cos(fa2 * F32(2.2))
    )
    cu, cv = (u2 * 3) % 1.0 - 0.5, (v2 * 3) % 1.0 - 0.5
    rr = np.sqrt(cu * cu + cv * cv)
    tanks = np.where(rr < 0.36, F32(0.75) - F32(0.4) * smoothstep(0.30, 0.36, rr), F32(0.18))
    rails = F32(0.30) + F32(0.25) * (np.cos(fb2 * F32(2.0)) > 0.6)
    slag = F32(0.12) + F32(0.25) * grain
    b = np.select(
        [lot < 0.45, lot < 0.62, lot < 0.78, lot < 0.9],
        [hall, tanks, rails, slag],
        default=F32(0.08) + F32(0.1) * grain,
    ).astype(F32)
    b = b * (F32(1.0) - road2) + F32(0.25) * road2
    b = b * (F32(1.0) - (F32(1.0) - smoothstep(3.0 / N, 7.0 / N, gap2))) + F32(0.22) * (
        F32(1.0) - smoothstep(3.0 / N, 7.0 / N, gap2)
    )

    # Relief : terrain bosselé, cratères (rebord clair, fond sombre), fosses en gradins.
    a = F32(0.35) + F32(0.3) * tile_fbm(px, py, 16, 5, seed + 21)
    for k in range(70):
        cx, cy = rng.random(2)
        rad = (4.0 + 60.0 * rng.random() ** 3) / N
        dx = (px - cx + 0.5) % 1.0 - 0.5
        dy = (py - cy + 0.5) % 1.0 - 0.5
        rho = np.sqrt(dx * dx + dy * dy) / F32(rad)
        if k % 9 == 0:
            terr = np.where(rho < 1.0, F32(0.25) + F32(0.25) * (np.cos(rho * F32(18.0)) > 0), F32(0.0))
            a = np.where(rho < 1.0, terr, a)
        else:
            a = np.where(rho < 0.8, a * F32(0.45), a)
            a = a + F32(0.35) * np.exp(-(((rho - 0.92) / 0.1) ** 2)).astype(F32)
    a = np.clip(a, 0.0, 1.0)

    img = np.stack([r, g, b, a], axis=-1).astype(F32)
    img = np.clip(img, 0.0, 1.0)
    # Suréchantillonnage : lignes et bords lissés.
    img = img.reshape(n, ss, n, ss, 4).mean(axis=(1, 3))
    return img.astype(F32)


# --------------------------------------------------------------------------------------------
# PNG
# --------------------------------------------------------------------------------------------


def write_png(path, arr):
    """PNG 8 bits (1 à 4 canaux) ; filtre Paeth (calculé sur les octets bruts, vectorisé)."""
    arr = np.ascontiguousarray(arr)
    if arr.ndim == 2:
        arr = arr[:, :, None]
    h, w, c = arr.shape
    color_type = {1: 0, 2: 4, 3: 2, 4: 6}[c]
    raw = arr.reshape(h, w * c).astype(np.int16)
    left = np.zeros_like(raw)
    left[:, c:] = raw[:, :-c]
    up = np.zeros_like(raw)
    up[1:] = raw[:-1]
    upleft = np.zeros_like(raw)
    upleft[1:, c:] = raw[:-1, :-c]
    p = left + up - upleft
    pa, pb, pc = np.abs(p - left), np.abs(p - up), np.abs(p - upleft)
    pred = np.where((pa <= pb) & (pa <= pc), left, np.where(pb <= pc, up, upleft))
    filt = ((raw - pred) & 0xFF).astype(np.uint8)
    body = np.concatenate([np.full((h, 1), 4, np.uint8), filt], axis=1).tobytes()

    def chunk(kind, data):
        return (
            struct.pack(">I", len(data))
            + kind
            + data
            + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
        )

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, color_type, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(body, 9))
    png += chunk(b"IEND", b"")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(png)
    print(f"  écrit {path.relative_to(ROOT)} ({w} × {h}, {len(png) / 1e6:.1f} Mo)", flush=True)


def to_srgb8(lin):
    v = np.clip(lin, 0.0, 1.0)
    s = np.where(v <= 0.0031308, v * 12.92, 1.055 * np.power(v, 1 / 2.4) - 0.055)
    return (s * 255.0 + 0.5).astype(np.uint8)


def to_u8(v):
    return (np.clip(v, 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)


def half(a):
    """Réduction 2 × (moyenne, en lumière linéaire)."""
    h, w = a.shape[:2]
    return a.reshape(h // 2, 2, w // 2, 2, *a.shape[2:]).mean(axis=(1, 3)).astype(F32)


# --------------------------------------------------------------------------------------------
# Aperçus « comme en jeu » (approximation du shader du jeu, assets/shaders/planet.wesl)
# --------------------------------------------------------------------------------------------

# Réglages repris de assets/data/gameplay/planets.ron (à tenir ensemble).
LOOK = {
    "detail_tiles_km": (160.0, 25.0),
    "detail_weights": (0.45, 0.5),
    "detail_blur": (1.0, 0.0),
    "urban": 1.2,
    "industry": 1.0,
    "rough": 0.7,
    "lights_detail": 1.6,
    "night_color": (1.0, 0.56, 0.24),
    "night_core": (1.0, 0.86, 0.62),
    "night_intensity": 1.4,
    "lights_on": (-0.04, 0.16),
    "haze_color": (0.78, 0.70, 0.52),
    "haze_strength": 0.6,
    "haze_power": 5.0,
    "haze_floor": 0.03,
    "haze_lit": (-0.25, 0.35),
}
SUN_OUT = 400.0 / (1.2 * 2**7) / math.pi  # 400 lux, exposition INDOOR (EV100 7), ÷ π


def mip_pyramid(img):
    levels = [img]
    while levels[-1].shape[0] > 1:
        levels.append(half(levels[-1]))
    return levels


def sample_tile(levels, u, v, lod):
    """Échantillonnage trilinéaire d'une texture carrelable (u, v en carreaux)."""
    out = np.zeros(u.shape + (levels[0].shape[2],), F32)
    lod = np.clip(lod, 0.0, len(levels) - 1.001)
    l0 = np.floor(lod).astype(np.int32)
    t = (lod - l0).astype(F32)[..., None]
    for k in range(len(levels) - 1):
        m = l0 == k
        if not m.any():
            continue
        acc = None
        for lev, wgt in ((k, F32(1.0) - t[m]), (k + 1, t[m])):
            img = levels[lev]
            n = img.shape[0]
            x = u[m] * n - 0.5
            y = v[m] * n - 0.5
            x0, y0 = np.floor(x).astype(np.int64), np.floor(y).astype(np.int64)
            fx, fy = (x - x0).astype(F32)[:, None], (y - y0).astype(F32)[:, None]
            x0 %= n
            y0 %= n
            x1, y1 = (x0 + 1) % n, (y0 + 1) % n
            s = (img[y0, x0] * (1 - fx) + img[y0, x1] * fx) * (1 - fy) + (
                img[y1, x0] * (1 - fx) + img[y1, x1] * fx
            ) * fy
            acc = s * wgt if acc is None else acc + s * wgt
        out[m] = acc
    return out


def sample_equirect(img, u, v):
    h, w = img.shape[:2]
    x = u * w - 0.5
    y = np.clip(v * h - 0.5, 0, h - 1)
    x0 = np.floor(x).astype(np.int64)
    y0 = np.floor(y).astype(np.int64)
    fx = (x - x0).astype(F32)[:, None]
    fy = (y - y0).astype(F32)[:, None]
    x0 %= w
    x1 = (x0 + 1) % w
    y1 = np.minimum(y0 + 1, h - 1)
    return (img[y0, x0] * (1 - fx) + img[y0, x1] * fx) * (1 - fy) + (
        img[y1, x0] * (1 - fx) + img[y1, x1] * fx
    ) * fy


def tonemap(c):
    # Courbe filmique (ACES ajustée) : proche du rendu du jeu, sans prétention d'exactitude.
    return np.clip((c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14), 0.0, 1.0)


def render_view(
    color, data, detail, eye, target, sun, size=(960, 540), fov_deg=75.0, ss=2, up=(0.0, 0.0, 1.0)
):
    """Rendu d'une vue (repère des cartes, rayon 1) : surface, détail, nuit, voile. Le haut de
    l'image est l'axe Y du jeu (axe z du repère des cartes), comme dans le cockpit."""
    W, H = size[0] * ss, size[1] * ss
    eye = np.asarray(eye, np.float64)
    fwd = np.asarray(target, np.float64) - eye
    fwd /= np.linalg.norm(fwd)
    up0 = np.asarray(up, np.float64)
    right = np.cross(fwd, up0)
    right /= np.linalg.norm(right)
    up = np.cross(right, fwd)
    tan = math.tan(math.radians(fov_deg) / 2)
    ys, xs = np.mgrid[0:H, 0:W]
    sx = ((xs + 0.5) / W * 2 - 1) * tan * W / H
    sy = (1 - (ys + 0.5) / H * 2) * tan
    rays = fwd[None, None] + sx[..., None] * right[None, None] + sy[..., None] * up[None, None]
    rays /= np.linalg.norm(rays, axis=-1, keepdims=True)
    b = (rays * eye).sum(-1)
    c = (eye * eye).sum() - 1.0
    disc = b * b - c
    hit = disc > 0
    t = -b - np.sqrt(np.maximum(disc, 0))
    hit &= t > 0
    img = np.zeros((H, W, 3), F32)
    r = rays[hit]
    th = t[hit]
    p = eye[None] + r * th[:, None]
    p /= np.linalg.norm(p, axis=-1, keepdims=True)
    lat = np.arcsin(np.clip(p[:, 1], -1, 1))
    lon = np.arctan2(p[:, 2], p[:, 0]) % TAU
    u = (lon / TAU).astype(F32)
    v = ((math.pi / 2 - lat) / math.pi).astype(F32)
    base = sample_equirect(color, u, v)
    dat = sample_equirect(data, u, v)
    levels = mip_pyramid(detail)
    mean = levels[-1][0, 0]
    ndv = np.clip(-(r * p).sum(-1), 0.02, 1.0)
    pix = math.radians(fov_deg) / H
    weights = np.abs(p) ** 6
    weights /= weights.sum(-1, keepdims=True)
    mod = np.zeros(len(p), F32)
    lights_mod = np.ones(len(p), F32)
    urban, industry, water = dat[:, 1], dat[:, 2], dat[:, 3]
    rough = (1 - water) * np.clip(1 - urban - industry, 0, 1)
    for tile_km, wgt, blur in zip(LOOK["detail_tiles_km"], LOOK["detail_weights"], LOOK["detail_blur"]):
        k = 640.0 / tile_km
        footprint = th * pix / np.maximum(ndv, 0.08) * k * detail.shape[0]
        lod = (np.log2(np.maximum(footprint, 1e-6)) + blur).astype(F32)
        q = p * k
        dsum = np.zeros((len(p), 4), F32)
        for axis, (a1, a2) in enumerate(((1, 2), (2, 0), (0, 1))):
            dsum += weights[:, axis : axis + 1] * sample_tile(levels, q[:, a1], q[:, a2], lod)
        rel = dsum / np.maximum(mean, 0.05)
        mod += wgt * (
            urban * LOOK["urban"] * (rel[:, 0] - 1)
            + industry * LOOK["industry"] * (rel[:, 2] - 1)
            + rough * LOOK["rough"] * (rel[:, 3] - 1)
        )
        lights_mod *= np.maximum(1 + LOOK["lights_detail"] * wgt * (rel[:, 1] - 1), 0)
    albedo = base * np.maximum(1 + mod, 0)[:, None]
    sun = np.asarray(sun, np.float64)
    sun /= np.linalg.norm(sun)
    ndl = (p * sun).sum(-1)
    col = albedo * (SUN_OUT * np.maximum(ndl, 0))[:, None]
    night = smoothstep(LOOK["lights_on"][0], LOOK["lights_on"][1], -ndl)
    lum = dat[:, 0] * np.clip(lights_mod, 0, 3)
    tint = mix(LOOK["night_color"], LOOK["night_core"], smoothstep(0.5, 1.2, lum))
    col = col + tint * (LOOK["night_intensity"] * lum * night)[:, None]
    f = (1 - ndv) ** LOOK["haze_power"]
    lit = smoothstep(LOOK["haze_lit"][0], LOOK["haze_lit"][1], ndl)
    veil = np.clip((LOOK["haze_floor"] + (1 - LOOK["haze_floor"]) * f) * LOOK["haze_strength"], 0, 1)
    haze = np.asarray(LOOK["haze_color"], F32)[None] * (SUN_OUT * lit)[:, None]
    col = col + (haze - col) * veil[:, None]
    img[hit] = col
    out = to_srgb8(tonemap(img))
    return out.reshape(size[1], ss, size[0], ss, 3).mean(axis=(1, 3)).astype(np.uint8)


def absolute_to_maps(v):
    """Repère du jeu (astre sans rotation, sphère UV de Bevy aux pôles ±Z) → repère des cartes."""
    return (v[0], v[2], v[1])


def previews(color, data, detail):
    PREVIEWS.mkdir(parents=True, exist_ok=True)
    print("aperçus :", flush=True)
    # Directions vues du centre d'Hexis, repère absolu du jeu (P2, P15 : étoile à l'origine).
    sun = absolute_to_maps((0.6, 0.0, 0.8))
    offset = (-0.222, 0.208, 0.9525)  # arrivée de sys_hexis.ron (azimut −50°, élévation 12°)
    enclume = (0.640 * 2.5, 0.0, 0.768 * 2.5)
    views = {
        # Arrivée près de L'Enclume, Hexis au centre (SH_LOOK) : face éclairée.
        "loin-jour": (absolute_to_maps(enclume), (0, 0, 0), 75.0),
        # SH_ARRIVAL=hexis,2000000 : terminateur au bord.
        "terminateur": (absolute_to_maps(tuple(3.125 * c for c in offset)), (0, 0, 0), 75.0),
        # SH_ARRIVAL=hexis,-2000000 + SH_LOOK : côté nuit.
        "nuit": (absolute_to_maps(tuple(-3.125 * c for c in offset)), (0, 0, 0), 75.0),
    }
    for name, (eye, target, fov) in views.items():
        write_png(PREVIEWS / f"{name}.png", render_view(color, data, detail, eye, target, sun, fov_deg=fov))
    # Orbite basse : au-dessus du spatioport (0,08 rayon), vers l'horizon puis à la verticale.
    radial = np.array(absolute_to_maps((0.656, 0.0, 0.755)))
    eye = radial * 1.08
    east = np.cross(np.array([0.0, 1.0, 0.0]), radial)
    east /= np.linalg.norm(east)
    for name, look in (
        ("orbite-basse-horizon", eye + east * 1.0 - radial * 0.45),
        ("orbite-basse-verticale", eye - radial),
        ("orbite-basse-nord", eye + np.array(absolute_to_maps((0.0, 1.0, 0.0))) - radial * 0.4),
    ):
        # Vaisseau à l'horizontale locale (le joueur peut rouler) : le haut est la verticale
        # (le nord, pour la vue à la verticale).
        up = east if name.endswith("verticale") else radial
        write_png(PREVIEWS / f"{name}.png", render_view(color, data, detail, eye, look, sun, up=up))
    small = half(half(color)) if color.shape[1] >= 4096 else half(color)
    write_png(PREVIEWS / "carte-couleur.png", to_srgb8(small))
    write_png(PREVIEWS / "carte-lumieres.png", to_srgb8(np.clip(half(data)[..., 0:1] * np.array([1.0, 0.56, 0.24], F32), 0, 1)))
    write_png(PREVIEWS / "detail.png", to_u8(np.concatenate([detail[..., k] for k in range(4)], axis=1)))


def main():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    size = 4096
    if "--size" in argv:
        size = int(argv[argv.index("--size") + 1])
    write = "--no-write" not in argv and size == 4096
    t0 = time.time()
    print(f"Hexis : carte {size} × {size // 2}", flush=True)
    color, data = hexis_maps(size)
    print("détail carrelable", flush=True)
    detail = urban_detail()
    print(f"  moyennes du détail (R, G, B, A) : {np.round(detail.mean(axis=(0, 1)), 3)}", flush=True)
    if write:
        write_png(OUT / "hexis_color_4k.png", to_srgb8(color))
        color2 = half(color)
        write_png(OUT / "hexis_color_2k.png", to_srgb8(color2))
        write_png(OUT / "hexis_surface_2k.png", to_u8(half(data)))
        write_png(OUT / "urban_detail.png", to_u8(detail))
    if "--previews" in argv:
        c2 = half(color) if size >= 4096 else color
        d2 = half(data) if size >= 4096 else data
        previews(c2, d2, detail)
    print(f"terminé en {time.time() - t0:.0f} s", flush=True)


main()
