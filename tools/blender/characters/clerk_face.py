"""Têtes des PNJ de comptoir (lot E de M9) : forme, cheveux, barbe et correspondance UV.

Une tête est une surface étoilée autour d'un centre (entre les oreilles, à hauteur des yeux),
décrite par une distance signée (SDF) : crâne, masse du visage, mâchoire, menton, pommettes,
arcades, orbites, globes oculaires, nez, lèvres et cou, plus une épaisseur ajoutée pour les
cheveux et la barbe. Chaque rayon (azimut a, élévation e) parti du centre donne le point le plus
extérieur de la surface. Le maillage (`clerk_kit.head_mesh`) et la peinture de la texture
(`clerk_atlas.paint_face`) échantillonnent la MÊME fonction aux mêmes (a, e) : les traits peints
(yeux, sourcils, lèvres, barbe, implantation des cheveux) tombent exactement sur leur relief,
sans dépliage manuel.

Repère local de la tête : origine au centre, +Y devant, +X à droite du personnage, +Z en haut,
en mètres. Azimut a = atan2(−x, y) : 0 devant, positif vers la gauche du personnage, si bien que
U croît vers la droite de qui regarde le visage (texture vue de l'extérieur, jamais en miroir).
Correspondance non linéaire (tanh) : le visage prend l'essentiel de la case, l'arrière du crâne
et les pôles (sommet sous les cheveux, bas du cou dans le col) sont comprimés.

Pur numpy (sans bpy) : importé par le maillage et par la texture.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

import numpy as np

# --- Correspondance (a, e) <-> (U, V) de la case du visage -----------------------------------

K_U = 2.0        # compression de l'arrière du crâne (azimut)
K_V = 1.3        # compression des pôles (élévation)
E_MID = -0.25    # élévation au centre de la case (entre les yeux et la bouche)
E_LO, E_HI = -math.pi / 2, math.pi / 2
R_MAX = 0.215    # rayon de départ de la recherche : le cou y est tranché (caché dans le col)
R_MIN = 0.02


def _tmap(s, k):
    return np.tanh(k * s) / math.tanh(k)


def _tinv(y, k):
    return np.arctanh(np.clip(y, -0.9999999, 0.9999999) * math.tanh(k)) / k


def uv_from_angles(a, e):
    a = np.asarray(a, dtype=np.float64)
    e = np.asarray(e, dtype=np.float64)
    u = 0.5 + 0.5 * _tmap(a / math.pi, K_U)
    s = np.where(e < E_MID, (e - E_MID) / (E_MID - E_LO), (e - E_MID) / (E_HI - E_MID))
    v = 0.5 + 0.5 * _tmap(s, K_V)
    return u, v


def angles_from_uv(u, v):
    """Inverse de `uv_from_angles`. U est périodique (raccord à l'arrière du crâne) ; V est
    borné aux pôles."""
    u = np.asarray(u, dtype=np.float64) % 1.0
    v = np.clip(np.asarray(v, dtype=np.float64), 0.0, 1.0)
    a = math.pi * _tinv(2.0 * u - 1.0, K_U)
    s = _tinv(2.0 * v - 1.0, K_V)
    e = np.where(s < 0, E_MID + s * (E_MID - E_LO), E_MID + s * (E_HI - E_MID))
    return a, e


def direction(a, e):
    ce = np.cos(e)
    return -np.sin(a) * ce, np.cos(a) * ce, np.sin(e)


# --- Description d'une tête ------------------------------------------------------------------


@dataclass(frozen=True)
class Head:
    """Paramètres d'une tête (facteurs relatifs à une tête moyenne, 1 = moyen)."""

    skin: tuple            # albédo de base de la peau (sRGB)
    flush: tuple           # rougeur des joues, du nez, des oreilles (sRGB)
    lip: tuple             # teinte des lèvres (sRGB)
    hair: tuple            # cheveux, sourcils, barbe (sRGB)
    iris: tuple            # iris (sRGB)
    hair_style: str        # "bun" (chignon bas), "short" (court, coiffé en arrière), "shaved" (ras)
    beard: str = "none"    # "none", "stubble" (peinte), "full" (barbe courte en relief)
    female: bool = False
    width: float = 1.0
    jaw: float = 1.0
    chin: float = 1.0
    face_len: float = 1.0
    nose_len: float = 1.0
    nose_w: float = 1.0
    nose_bridge: float = 1.0
    lips: float = 1.0
    brow: float = 1.0
    neck: float = 1.0
    age: float = 0.3       # rides (0 à 1)
    gray: float = 0.0      # part de gris (tempes d'abord)
    recede: float = 0.0    # golfes des tempes (0 à 1)
    brow_hair: float = 1.0 # épaisseur des sourcils
    taper: float = 0.27    # rétrécissement du bas du visage vers le menton
    jowl: float = 1.0      # joues sous les pommettes (0 : aucune)
    muzzle: float = 1.0    # avancée de la bouche (arcade dentaire)
    cheeks: float = 0.6    # joues pleines entre pommette et bouche (0 : creusées)


# Positions caractéristiques (moyennes) : partagées par la forme et la peinture.
EYE_X, EYE_Z = 0.0315, 0.0
EYE_BALL_Y = 0.0655
EYE_R = 0.0122
# Oreille : point d'attache (côté droit ; miroir pour le gauche).
EAR_X, EAR_Y, EAR_Z = 0.0715, -0.004, -0.002


def eye_center(h: Head):
    return EYE_X * h.width, EYE_BALL_Y + EYE_R, EYE_Z


def mouth_z(h: Head) -> float:
    return -0.0685 * h.face_len


def nose_tip(h: Head):
    return 0.0, 0.110 + 0.012 * (h.nose_len - 1.0), -0.0405 * h.nose_len * h.face_len


# --- Distances signées -----------------------------------------------------------------------


def _ellipsoid(x, y, z, c, r):
    """Ellipsoïde (approximation de distance d'I. Quilez, suffisante pour la recherche)."""
    px, py, pz = (x - c[0]) / r[0], (y - c[1]) / r[1], (z - c[2]) / r[2]
    k0 = np.sqrt(px * px + py * py + pz * pz)
    qx, qy, qz = px / r[0], py / r[1], pz / r[2]
    k1 = np.sqrt(qx * qx + qy * qy + qz * qz)
    return k0 * (k0 - 1.0) / np.maximum(k1, 1e-9)


def _sphere(x, y, z, c, r):
    return np.sqrt((x - c[0]) ** 2 + (y - c[1]) ** 2 + (z - c[2]) ** 2) - r


def _capsule(x, y, z, a, b, r0, r1=None):
    """Capsule de a à b, rayon r0 → r1 (cône arrondi approché)."""
    pax, pay, paz = x - a[0], y - a[1], z - a[2]
    bax, bay, baz = b[0] - a[0], b[1] - a[1], b[2] - a[2]
    t = np.clip((pax * bax + pay * bay + paz * baz) / (bax * bax + bay * bay + baz * baz), 0.0, 1.0)
    r = r0 if r1 is None else r0 + (r1 - r0) * t
    return np.sqrt((pax - bax * t) ** 2 + (pay - bay * t) ** 2 + (paz - baz * t) ** 2) - r


def smin(a, b, k):
    h = np.clip(0.5 + 0.5 * (b - a) / k, 0.0, 1.0)
    return b + (a - b) * h - k * h * (1.0 - h)


def smax(a, b, k):
    return -smin(-a, -b, k)


def _ss(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def _lips(h: Head, x, y, z):
    """Lèvres (x déjà divisé par la largeur) : ellipsoïdes courbés qui suivent l'arcade
    dentaire, commissures en retrait."""
    mz = mouth_z(h)
    lp = h.lips
    yb = y + 9.0 * x * x
    ul = _ellipsoid(x, yb, z, (0.0, 0.0965, mz + 0.0052), (0.0235, 0.0075 * lp, 0.0062 * lp))
    ll = _ellipsoid(x, yb, z, (0.0, 0.0935, mz - 0.0062 * lp), (0.0200, 0.0080 * lp, 0.0070 * lp))
    return smin(ul, ll, 0.003)


def lip_distance(h: Head, x, y, z):
    """Distance aux lèvres seules (peinture de leur couleur)."""
    return _lips(h, x / h.width, y, z)


def skin_sdf(h: Head, x, y, z):
    """Peau seule (sans cheveux ni barbe)."""
    x = x / h.width
    ax = np.abs(x)
    fl = h.face_len
    # Crâne, plus long derrière.
    d = _ellipsoid(x, y, z, (0.0, -0.012, 0.016), (0.0745, 0.0975, 0.1005))
    # Bas du visage : ellipsoïde rétréci vers le menton (mâchoire en V vue de face).
    wz = 1.0 - (h.taper - 0.10 * (h.jaw - 1.0)) * _ss(-0.058, -0.124 * fl, z)
    face = _ellipsoid(x / wz, y, z, (0.0, 0.024, -0.024 * fl), (0.0645 * h.jaw, 0.072, 0.090 * fl))
    d = smin(d, face, 0.03)
    # Angles de la mâchoire, sous les oreilles.
    gon = _ellipsoid(ax, y, z, (0.045 * h.jaw, 0.002, -0.066 * fl), (0.015 * h.jaw, 0.028, 0.021))
    d = smin(d, gon, 0.026)
    # Museau (arcade dentaire sous le nez) et menton.
    mz = mouth_z(h)
    mu = h.muzzle
    muzzle = _ellipsoid(x, y, z, (0.0, 0.070 - 0.012 * (1.0 - mu), mz + 0.002),
                        (0.031 * min(1.0, h.jaw) ** 0.5 * (0.6 + 0.4 * mu), 0.031, 0.026 * (0.7 + 0.3 * mu)))
    d = smin(d, muzzle, 0.02)
    chin_z = -0.1025 * fl * (0.97 + 0.03 * h.chin)
    chin = _ellipsoid(x, y, z, (0.0, 0.0745, chin_z), (0.027 * h.chin, 0.020, 0.018))
    d = smin(d, chin, 0.022)
    # Ligne de la mâchoire (de l'angle au menton) et joues pleines sous les pommettes.
    jaw_line = _capsule(ax, y, z, (0.047 * h.jaw, 0.000, -0.068 * fl), (0.020 * h.chin, 0.066, chin_z + 0.004),
                        0.012 * h.jaw * (0.75 if h.female else 1.0))
    d = smin(d, jaw_line, 0.022)
    if h.jowl > 0:
        jowl = _ellipsoid(ax, y, z, (0.037 * h.jaw, 0.050, -0.050 * fl),
                          (0.019 * h.jowl, 0.027, 0.025 * min(1.0, 0.5 + 0.5 * h.jowl)))
        d = smin(d, jowl, 0.025)
    if h.cheeks > 0:
        pad = _ellipsoid(ax, y, z, (0.035 * h.width ** 0.5, 0.060, -0.030 * fl), (0.020 * h.cheeks + 0.004, 0.022, 0.024))
        d = smin(d, pad, 0.022)
    # Pommettes, arcades sourcilières.
    cheek = _ellipsoid(ax, y, z, (0.046, 0.054, -0.013), (0.019, 0.022, 0.013))
    d = smin(d, cheek, 0.028)
    brow = _capsule(ax, y, z, (0.0, 0.0805, 0.0245), (0.040, 0.070, 0.021), 0.0082 * h.brow)
    d = smin(d, brow, 0.016)
    # Orbites creusées, globes oculaires (paupières par le raccord doux).
    sock = _ellipsoid(ax, y, z, (EYE_X, 0.090, EYE_Z + 0.002), (0.017, 0.016, 0.0122))
    d = smax(d, -sock, 0.007)
    eye = _sphere(ax, y, z, (EYE_X, EYE_BALL_Y, EYE_Z), EYE_R)
    d = smin(d, eye, 0.003)
    # Nez : arête, pointe, ailes.
    tx, ty, tz = nose_tip(h)
    nw = h.nose_w
    bridge_y = 0.084 + 0.004 * (h.nose_bridge - 1.0)
    ridge = _capsule(x, y, z, (0.0, bridge_y, 0.012), (0.0, ty - 0.004, tz + 0.004), 0.0052 * nw, 0.0080 * nw)
    tip = _sphere(x, y, z, (0.0, ty - 0.0045, tz), 0.0095 * (0.6 + 0.4 * nw))
    alae = _ellipsoid(ax, y, z, (0.0122 * nw, ty - 0.0145, tz - 0.004), (0.0088 * nw, 0.0100, 0.0074))
    nose = smin(smin(ridge, tip, 0.006), alae, 0.006)
    d = smin(d, nose, 0.0065)
    # Lèvres : ellipsoïdes courbés qui suivent l'arcade (commissures en retrait), fente fine.
    d = smin(d, _lips(h, x, y, z), 0.006)
    slit = _ellipsoid(x, y + 9.0 * x * x, z, (0.0, 0.1050, mz), (0.0225, 0.0085, 0.0009))
    d = smax(d, -slit, 0.0012)
    # Cou, légèrement penché en avant.
    nr = 0.0525 * h.neck
    neck = _capsule(x, y * 0.97, z, (0.0, -0.024, -0.05), (0.0, -0.004, -0.30), nr)
    d = smin(d, neck, 0.024)
    if h.hair_style == "bun":
        bun = _ellipsoid(x, y, z, (0.0, -0.104, -0.020), (0.036, 0.030, 0.031))
        d = smin(d, bun, 0.012)
    return d


# --- Cheveux et barbe ------------------------------------------------------------------------

# Implantation des cheveux : hauteur (z) de la limite en fonction de |azimut| (rad).
_HAIRLINES = {
    "bun": [(0.0, 0.074), (0.45, 0.069), (0.85, 0.058), (1.15, 0.040), (1.33, 0.020), (1.45, 0.024),
            (1.58, 0.042), (1.80, 0.040), (2.02, -0.008), (2.35, -0.050), (2.8, -0.072), (math.pi, -0.080)],
    "short": [(0.0, 0.076), (0.40, 0.072), (0.75, 0.063), (1.05, 0.046), (1.26, 0.020), (1.36, -0.022),
              (1.47, -0.024), (1.555, 0.038), (1.78, 0.040), (2.0, -0.008), (2.4, -0.052), (2.8, -0.068),
              (math.pi, -0.072)],
}
_HAIRLINES["shaved"] = _HAIRLINES["short"]


def hairline_z(h: Head, a):
    pts = _HAIRLINES[h.hair_style]
    aa = np.abs(a)
    z = np.interp(aa, [p[0] for p in pts], [p[1] for p in pts])
    if h.recede > 0:
        # Golfes : la limite remonte aux tempes (|a| ≈ 0,6), pas au milieu du front.
        z = z + h.recede * 0.018 * np.exp(-((aa - 0.62) / 0.22) ** 2)
    return z


def azimuth(x, y):
    return np.arctan2(-x, y)


def hair_cover(h: Head, x, y, z):
    """Part couverte de cheveux (0 à 1, bord net) au point (sur ou près de la surface)."""
    a = azimuth(x, y)
    zl = hairline_z(h, a)
    return _ss(zl - 0.0025, zl + 0.0025, z)


def hair_thickness(h: Head, x, y, z):
    a = azimuth(x, y)
    zl = hairline_z(h, a)
    ramp = _ss(zl - 0.002, zl + 0.013, z)
    if h.hair_style == "shaved":
        return 0.0012 * ramp
    if h.hair_style == "bun":
        # Tirés en arrière, serrés : peu de volume, un peu plus au sommet.
        t = 0.0065 + 0.0045 * _ss(0.0, 0.10, z)
        return t * ramp
    # Court, coiffé en arrière : volume sur le dessus, dégradé sur les côtés et la nuque.
    top = _ss(0.02, 0.09, z) * (0.6 + 0.4 * np.cos(np.clip(a, -math.pi, math.pi) / 2) ** 2)
    t = 0.004 + 0.0075 * top * (1.0 - 0.4 * _ss(0.10, 0.125, z))
    return t * ramp


def beard_cover(h: Head, x, y, z, soft: float = 1.0):
    """Barbe en relief (0 à 1) : la zone de barbe pour une barbe courte, rien sinon."""
    if h.beard != "full":
        return np.zeros_like(np.asarray(x, dtype=np.float64))
    return beard_region(h, x, y, z, soft)


def beard_region(h: Head, x, y, z, soft: float = 1.0):
    """Zone de barbe (0 à 1) : joues sous une ligne (de la patte à la commissure), menton,
    moustache ; lèvres exclues. `soft` élargit les transitions (le relief s'amincit sur les
    bords, la peinture reste nette)."""
    w = 0.0025 * soft
    ax = np.abs(x / h.width)
    mz = mouth_z(h)
    tz = nose_tip(h)[2]
    line = mz + 0.013 + (-0.012 - (mz + 0.013)) * _ss(0.028, 0.068, ax)
    cheeks = 1.0 - _ss(line - w, line + w, z)
    stache = (1.0 - _ss(0.029 - w, 0.029 + w, ax)) * _ss(mz - w, mz + w, z) * (1.0 - _ss(tz - 0.013 - w, tz - 0.013 + w, z))
    # Devant l'oreille et l'angle de la mâchoire seulement ; s'arrête sur le haut du cou.
    back = _ss(-0.010 - w, -0.010 + w, y)
    neck = _ss(-0.150 - w, -0.132 + w, z)
    lips = 1.0 - _ss(0.0012, 0.0012 + 1.5 * w, lip_distance(h, x, y, z))
    return np.clip(np.maximum(cheeks, stache * _ss(0.07, 0.085, y)) * back * neck * (1.0 - lips), 0.0, 1.0)


def beard_thickness(h: Head, x, y, z):
    cov = beard_cover(h, x, y, z, soft=3.5)
    return cov * (0.0035 + 0.0045 * _ss(-0.06, -0.11, z))


def full_sdf(h: Head, x, y, z):
    return skin_sdf(h, x, y, z) - hair_thickness(h, x, y, z) - beard_thickness(h, x, y, z)


# --- Recherche de la surface -----------------------------------------------------------------


def surface(h: Head, a, e, steps: int = 72, bisect: int = 14):
    """Point le plus extérieur de la surface (peau + cheveux + barbe) le long de chaque rayon
    (a, e). Renvoie (x, y, z, r)."""
    a = np.asarray(a, dtype=np.float64)
    e = np.asarray(e, dtype=np.float64)
    dx, dy, dz = direction(a, e)
    shape = np.broadcast(a, e).shape
    lo = np.full(shape, R_MIN)
    hi = np.full(shape, R_MAX)
    found = np.zeros(shape, dtype=bool)
    rs = np.linspace(R_MAX, R_MIN, steps)
    prev = np.full(shape, R_MAX)
    for r in rs:
        d = full_sdf(h, dx * r, dy * r, dz * r)
        hit = (d <= 0.0) & ~found
        lo = np.where(hit, r, lo)
        hi = np.where(hit, prev, hi)
        found |= hit
        prev = np.full(shape, r)
        if found.all():
            break
    # Rayon déjà dans la matière au départ (bas du cou) : la surface est la sphère de départ.
    inside0 = full_sdf(h, dx * R_MAX, dy * R_MAX, dz * R_MAX) <= 0.0
    for _ in range(bisect):
        mid = 0.5 * (lo + hi)
        d = full_sdf(h, dx * mid, dy * mid, dz * mid)
        inside = d <= 0.0
        lo = np.where(inside, mid, lo)
        hi = np.where(inside, hi, mid)
    r = np.where(inside0, R_MAX, 0.5 * (lo + hi))
    return dx * r, dy * r, dz * r, r


def gradient(h: Head, x, y, z, eps: float = 0.0004, fn=None):
    """Normale (gradient normalisé) de la SDF complète."""
    fn = fn or full_sdf
    gx = fn(h, x + eps, y, z) - fn(h, x - eps, y, z)
    gy = fn(h, x, y + eps, z) - fn(h, x, y - eps, z)
    gz = fn(h, x, y, z + eps) - fn(h, x, y, z - eps)
    n = np.sqrt(gx * gx + gy * gy + gz * gz) + 1e-12
    return gx / n, gy / n, gz / n


def occlusion(h: Head, x, y, z, nx, ny, nz):
    """Occlusion ambiante approchée par la SDF (creux des orbites, sous le nez, sous le menton)."""
    occ = np.zeros_like(x)
    total = 0.0
    for dist, w in ((0.003, 1.0), (0.007, 0.8), (0.014, 0.6), (0.026, 0.4)):
        d = full_sdf(h, x + nx * dist, y + ny * dist, z + nz * dist)
        occ += w * np.clip((dist - d) / dist, 0.0, 1.0)
        total += w
    return np.clip(1.0 - 0.9 * occ / total, 0.0, 1.0)
