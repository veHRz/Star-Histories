"""Atlas des PNJ de comptoir (lot E de M9) : `char_clerk`, 1K.

Pourquoi un atlas à part de `char_gear` (pirate) : les employés montrent leur visage, leurs mains
et des matières douces (cuir, linge, toile de tablier) que l'atlas du pirate n'a pas. Une seule
texture de 1K porte les trois visages et ces matières : un seul matériau par personnage pour
la peau, les cheveux, le cuir et le linge (budget de matériaux du personnage).

Disposition (pixels, ligne 0 en BAS, comme les UV de Blender) :
- trois cases de visage de 512 px (`face_a`, `face_b`, `face_c`) : peau, yeux, lèvres, sourcils,
  cheveux et barbe peints dans le repère (azimut, élévation) de `clerk_face`, en évaluant la même
  surface que le maillage : chaque trait tombe sur son relief. Occlusion calculée sur la SDF de
  la tête (orbites, sous le nez, sous le menton), recopiée en partie dans la couleur (le jeu
  n'éclaire un personnage que par l'ambiante dans une cellule à lightmap, en Bas et Moyen) ;
- `leather` (cuir brun : tablier, gants, chaussures, sacoche, ceinture), `towel` (torchon rayé),
  `apron` (toile de tablier bordeaux, poche, taches), 256 px ;
- `skin_a`, `skin_b`, `skin_c` (peau unie des mains, du cou et des avant-bras), `screen`
  (écran de tablette, émissif, sans texte), 128 px.

Cartes (contrat du jeu) dans assets/textures/materials/ : `char_clerk_color.png` (sRGB),
`char_clerk_normal.png` (OpenGL, linéaire), `char_clerk_orm.png` (R occlusion, G rugosité,
B métal, linéaire), `char_clerk_emissive.png` (sRGB). Plus la toile des vêtements
`char_clerk_cloth_{color,normal,orm}.png` (1K, tuile de 0,4 m, teintée par vêtement).

Les visages dépendent de la forme des têtes (`clerk_face`) : toute retouche d'une tête impose de
régénérer l'atlas (`clerks.py -- --atlas` le fait avant les modèles). Aperçus à plat dans
target/art-previews/m9-lotE/ (`atlas_*.png`, `cloth_color_3x3.png`).

    blender --background --factory-startup --threads 4 --python tools/blender/characters/clerk_atlas.py
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "materials"))
sys.path.insert(0, str(HERE))

import clerk_face as cf  # noqa: E402
import texgen as tg  # noqa: E402
from texgen import Canvas, cut, rgb, smoothstep, stains  # noqa: E402

ROOT = HERE.parents[2]
OUT_DIR = ROOT / "assets" / "textures" / "materials"
PREVIEW_DIR = ROOT / "target" / "art-previews" / "m9-lotE"
NAME = "char_clerk"
RES = 1024
FACE_PAD = 2

# (x0, y0, largeur, hauteur) en pixels, y0 depuis le bas.
RECTS: dict[str, tuple[int, int, int, int]] = {
    "face_a": (0, 0, 512, 512),
    "face_b": (512, 0, 512, 512),
    "face_c": (0, 512, 512, 512),
    "leather": (512, 512, 256, 256),
    "towel": (768, 512, 256, 256),
    "apron": (512, 768, 256, 256),
    "skin_a": (768, 768, 128, 128),
    "skin_b": (896, 768, 128, 128),
    "skin_c": (768, 896, 128, 128),
    "screen": (896, 896, 128, 128),
}


def rect_uv(name: str, pad_px: float = 4.0) -> tuple[float, float, float, float]:
    """(u0, v0, u1, v1) d'une case, rognée de `pad_px` texels."""
    x0, y0, w, h = RECTS[name]
    return ((x0 + pad_px) / RES, (y0 + pad_px) / RES, (x0 + w - pad_px) / RES, (y0 + h - pad_px) / RES)


def face_uv(key: str) -> tuple[float, float, float, float]:
    return rect_uv(f"face_{key}", FACE_PAD)


# --- Têtes des trois employés (forme et couleurs) ---------------------------------------------
# Peaux, cheveux et visages variés ; aucune ressemblance voulue avec quiconque.

HEADS: dict[str, cf.Head] = {
    # Guichetière : peau mate, cheveux bruns tirés en chignon bas, visage fin.
    "a": cf.Head(skin=(0.70, 0.52, 0.40), flush=(0.74, 0.44, 0.37), lip=(0.58, 0.33, 0.31), hair=(0.11, 0.08, 0.065),
                 iris=(0.30, 0.19, 0.11), hair_style="bun", female=True, width=0.95, jaw=0.9, chin=0.9, nose_w=0.84,
                 nose_len=0.92, lips=1.08, brow=0.5, neck=0.93, age=0.12, brow_hair=0.72, face_len=0.95,
                 taper=0.20, jowl=0.6, cheeks=1.0),
    # Marchand : peau foncée, crâne ras, barbe courte, visage large.
    "b": cf.Head(skin=(0.38, 0.255, 0.18), flush=(0.41, 0.23, 0.165), lip=(0.27, 0.16, 0.14), hair=(0.085, 0.07, 0.06),
                 iris=(0.13, 0.08, 0.05), hair_style="shaved", beard="full", width=1.05, jaw=1.12, chin=1.08,
                 nose_w=1.25, nose_bridge=0.8, lips=1.18, brow=1.05, neck=1.15, age=0.2, brow_hair=1.1),
    # Tenancier : peau claire et rougeaude, cheveux châtain clair grisonnants, barbe de trois jours.
    "c": cf.Head(skin=(0.80, 0.62, 0.53), flush=(0.84, 0.47, 0.41), lip=(0.68, 0.41, 0.39), hair=(0.38, 0.26, 0.17),
                 iris=(0.36, 0.46, 0.50), hair_style="short", beard="stubble", width=1.0, jaw=1.02, chin=1.05,
                 face_len=1.02, nose_len=1.12, nose_w=0.95, lips=0.85, brow=1.15, age=0.7, gray=0.45, recede=0.6,
                 brow_hair=1.0, taper=0.24, jowl=0.8),
}


# --- Bruits dans l'espace ------------------------------------------------------------------------


def _hash(ix, iy, iz, seed: int):
    h = (ix.astype(np.int64) * 73856093) ^ (iy.astype(np.int64) * 19349663) ^ (iz.astype(np.int64) * 83492791) ^ (seed * 2654435761)
    h = (h ^ (h >> 13)) * 1274126177
    h = h ^ (h >> 16)
    return (h & 0xFFFF).astype(np.float64) / 65535.0


def vnoise3(x, y, z, scale: float, seed: int, octaves: int = 3) -> np.ndarray:
    """Bruit de valeur 3D (centré, ≈ [−1, 1]) évalué sur la surface : échelle en mètres, sans
    raccord ni étirement dû au dépliage."""
    out = np.zeros_like(x, dtype=np.float64)
    amp, total = 1.0, 0.0
    for o in range(octaves):
        s = scale / (2 ** o)
        gx, gy, gz = x / s, y / s, z / s
        ix, iy, iz = np.floor(gx), np.floor(gy), np.floor(gz)
        fx, fy, fz = gx - ix, gy - iy, gz - iz
        fx, fy, fz = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy), fz * fz * (3 - 2 * fz)
        acc = 0.0
        for dx in (0, 1):
            for dy in (0, 1):
                for dz in (0, 1):
                    w = (fx if dx else 1 - fx) * (fy if dy else 1 - fy) * (fz if dz else 1 - fz)
                    acc = acc + w * _hash(ix + dx, iy + dy, iz + dz, seed + o * 31)
        out += amp * (acc * 2.0 - 1.0)
        total += amp
        amp *= 0.5
    return out / total


def strands(a, e, freq_a: int, freq_e: float, seed: int, octaves: int = 3) -> np.ndarray:
    """Mèches : bruit de valeur 2D périodique en azimut, fin en travers (a), lâche le long (e)."""
    out = np.zeros_like(a, dtype=np.float64)
    amp, total = 1.0, 0.0
    for o in range(octaves):
        fa = freq_a * (2 ** o)
        fe = freq_e * (2 ** o)
        ga = (a / (2 * math.pi) + 0.5) * fa
        ge = e * fe
        ia, ie = np.floor(ga), np.floor(ge)
        ta, te = ga - ia, ge - ie
        ta, te = ta * ta * (3 - 2 * ta), te * te * (3 - 2 * te)
        acc = 0.0
        for da in (0, 1):
            for de in (0, 1):
                w = (ta if da else 1 - ta) * (te if de else 1 - te)
                acc = acc + w * _hash((ia + da) % fa, ie + de, np.zeros_like(ia), seed + o * 17)
        out += amp * (acc * 2.0 - 1.0)
        total += amp
        amp *= 0.55
    return out / total


def _ss(e0, e1, x):
    return smoothstep(e0, e1, x)


def _gauss(dx, dz, sx, sz):
    return np.exp(-((dx / sx) ** 2 + (dz / sz) ** 2))


def _mix(col, target, t):
    t = np.clip(t, 0.0, 1.0)[..., None]
    return col + (np.asarray(target, dtype=np.float64).reshape(1, 1, 3) - col) * t


# --- Visages -------------------------------------------------------------------------------------


def paint_face(h: cf.Head, res: int = 512, pad: int = FACE_PAD) -> dict[str, np.ndarray]:
    t = (np.arange(res) + 0.5 - pad) / (res - 2 * pad)
    U, V = np.meshgrid(t, t)
    a, e = cf.angles_from_uv(U, V)
    x, y, z, r = cf.surface(h, a, e)
    nx, ny, nz = cf.gradient(h, x, y, z)
    ao = cf.occlusion(h, x, y, z, nx, ny, nz)
    X = x / h.width
    ax = np.abs(X)
    mz = cf.mouth_z(h)
    tx, ty, tz = cf.nose_tip(h)
    front = _ss(0.02, 0.06, y)
    # Taille d'un texel (m) pour l'anticrénelage des traits.
    px = 0.0006

    hair_cov = cf.hair_cover(h, x, y, z) if h.hair_style != "none" else np.zeros_like(x)
    region = cf.beard_region(h, x, y, z, soft=3.0)
    beard = cf.beard_cover(h, x, y, z, soft=1.2)
    height = np.zeros_like(x)
    rough = np.full_like(x, 0.56)

    # Peau : teinte, marbrures lentes et fines.
    col = np.broadcast_to(np.asarray(h.skin, dtype=np.float64).reshape(1, 1, 3), x.shape + (3,)).copy()
    mott = vnoise3(x, y, z, 0.03, 11)
    fine = vnoise3(x, y, z, 0.004, 12, octaves=2)
    col *= (1.0 + 0.05 * mott + 0.025 * fine)[..., None]
    height += 0.00003 * vnoise3(x, y, z, 0.0012, 13, octaves=2)
    # Rougeurs : joues, nez, menton ; front un peu plus clair.
    flush = (0.55 * _gauss(ax - 0.044, z + 0.028, 0.020, 0.016) + 0.55 * _gauss(X, z - tz, 0.012, 0.012)
             + 0.25 * _gauss(X, z + 0.108, 0.018, 0.010)) * front
    col = _mix(col, h.flush, 0.42 * flush)
    col = _mix(col, np.asarray(h.skin) * 1.06, 0.4 * _gauss(X, z - 0.05, 0.035, 0.02) * front)
    # Oreilles : la zone cachée sous l'oreille lui sert de texture (projection depuis le centre de
    # la tête, `clerk_kit.ear`) : rougeur, conque plus sombre, bord de l'hélix plus clair.
    ea = np.abs(a)
    col = _mix(col, h.flush, 0.35 * _gauss(ea - 1.74, z, 0.22, 0.03))
    concha = _gauss(ea - 1.70, z + 0.004, 0.075, 0.011)
    col = _mix(col, np.asarray(h.skin) * 0.55, 0.45 * concha)
    height -= 0.0004 * concha
    # Cernes et paupières un peu plus sombres.
    ex = ax - cf.EYE_X
    ez = z - cf.EYE_Z
    under = _gauss(ex + 0.001, ez + 0.0105, 0.011, 0.0055) * front
    col = _mix(col, np.asarray(h.skin) * np.array([0.80, 0.74, 0.78]), 0.35 * under)
    lids = _gauss(ex, ez - 0.0065, 0.014, 0.0045) * front
    col = _mix(col, np.asarray(h.skin) * np.array([0.86, 0.80, 0.80]), 0.35 * lids)

    # Rides (âge) : front, pattes d'oie, sillons nasogéniens.
    if h.age > 0:
        lines = np.zeros_like(x)
        for zz in (0.044, 0.053, 0.061):
            wob = 0.0012 * vnoise3(x, y, z, 0.02, 21 + int(zz * 1000), octaves=1)
            lines += np.exp(-((z - zz - wob) / 0.0008) ** 2) * (1.0 - _ss(0.025, 0.045, ax))
        for k in range(3):
            ang = -0.45 + 0.45 * k
            dxl = ax - (cf.EYE_X + 0.019)
            along = dxl * math.cos(ang) + ez * math.sin(ang)
            acr = -dxl * math.sin(ang) + ez * math.cos(ang)
            lines += np.exp(-(acr / 0.0006) ** 2) * _ss(0.0, 0.003, along) * (1.0 - _ss(0.008, 0.013, along))
        # Sillon : de l'aile du nez à la commissure.
        sx0, sz0 = 0.016 * h.nose_w, tz - 0.004
        sx1, sz1 = 0.030, mz - 0.006
        d = tg.sd_segment(ax, z, sx0, sz0, sx1, sz1)
        lines += 0.8 * np.exp(-(d / 0.0011) ** 2) * front
        lines *= h.age
        height -= 0.00022 * lines
        col = _mix(col, np.asarray(h.skin) * 0.78, 0.45 * lines)

    # Lèvres : couleur dans la zone des ellipsoïdes de lèvres, fente sombre.
    dl = cf.lip_distance(h, x, y, z)
    lipm = (1.0 - _ss(0.0002, 0.0018, dl)) * front
    upper = _ss(mz - 0.0005, mz + 0.0015, z)
    col = _mix(col, h.lip, 0.9 * lipm)
    col = _mix(col, np.asarray(h.lip) * 0.82, 0.5 * lipm * upper)
    slit = np.exp(-((z - mz) / 0.0009) ** 2) * (1.0 - _ss(0.017, 0.024, ax)) * front
    col = _mix(col, np.asarray(h.lip) * 0.35, 0.85 * slit)
    rough = np.where(lipm > 0.5, 0.40, rough)
    height -= 0.00008 * lipm * (0.5 + 0.5 * np.sin(X / 0.0012 * np.pi))

    # Narines : ombres sous le bout du nez.
    nos = _gauss((ax - 0.0072 * h.nose_w) / 0.0034, (z - (tz - 0.0090)) / 0.0019, 1.0, 1.0) * _ss(-0.2, -0.6, nz) * front
    col = _mix(col, np.asarray(h.skin) * 0.3, 0.75 * np.clip(nos * 1.4, 0, 1))

    # Barbe de trois jours (peinte) ou ombre de barbe sous une barbe courte.
    if h.beard in ("stubble", "full") or not h.female:
        grain = _ss(0.1, 0.6, vnoise3(x, y, z, 0.0011, 31, octaves=1))
        amount = {"stubble": 0.32, "full": 0.5}.get(h.beard, 0.15)
        st = region * amount * (0.55 + 0.45 * grain)
        col = _mix(col, np.asarray(h.hair) * 0.9 + np.asarray(h.skin) * 0.1, st)
        height += 0.00004 * region * grain

    # Sourcils.
    bdx = ex - 0.002
    arch = 0.0175 + 0.0035 * (1.0 - ((bdx - 0.004) / 0.020) ** 2)
    thick = (0.0026 - 0.0011 * _ss(-0.016, 0.019, bdx)) * h.brow_hair
    brow_d = np.abs(ez - arch) - thick
    browm = (1.0 - _ss(-px, px, brow_d)) * _ss(-0.019, -0.014, bdx) * (1.0 - _ss(0.017, 0.022, bdx)) * front
    hairs = 0.5 + 0.5 * strands(a, e, 900, 60, 41)
    browm = browm * (0.55 + 0.45 * hairs)
    col = _mix(col, np.asarray(h.hair) * 0.85, 0.9 * browm)
    height += 0.00012 * browm

    # Yeux : fente en amande, sclère, iris, pupille, ligne des cils, pli de paupière.
    W = 0.0136 if h.female else 0.0133
    q = np.clip(1.0 - (ex / W) ** 2, 0.0, 1.0)
    tilt = 0.0009 * (ex / W)
    zu = (0.0054 if h.female else 0.0049) * q ** 0.75 + tilt + 0.0004
    zl = -0.0041 * q ** 0.9 + tilt * 0.6
    inside = _ss(-px, px, zu - ez) * _ss(-px, px, ez - zl) * (np.abs(ex) < W) * front
    scl = np.asarray((0.80, 0.76, 0.72)) * 0.95
    eye = np.broadcast_to(scl.reshape(1, 1, 3), x.shape + (3,)).copy()
    corner = _ss(0.5, 1.0, np.abs(ex) / W)
    eye = _mix(eye, scl * 0.62, 0.7 * corner)
    lidshadow = _ss(zu - 0.0028, zu, ez)
    eye = _mix(eye, scl * 0.45, 0.65 * lidshadow)
    ir = 0.0057
    di = np.hypot(ex - 0.0002, ez - 0.0006)
    iris = (1.0 - _ss(ir - px, ir + px, di))
    ang = np.arctan2(ez - 0.0006, ex)
    streak = 0.85 + 0.25 * np.sin(ang * 23.0 + 3.0 * np.sin(ang * 7.0))
    icol = np.asarray(h.iris) * (0.75 + 0.45 * (di / ir))[..., None] * streak[..., None]
    eye = eye * (1 - iris[..., None]) + icol * iris[..., None]
    eye = _mix(eye, np.asarray(h.iris) * 0.35, 0.8 * np.exp(-((di - ir) / 0.0007) ** 2))
    pupil = 1.0 - _ss(0.0021 - px, 0.0021 + px, di)
    eye = _mix(eye, (0.025, 0.022, 0.02), pupil)
    eye = _mix(eye, scl * 0.4, 0.55 * lidshadow * iris)
    col = col * (1 - inside[..., None]) + eye * inside[..., None]
    rough = np.where(inside > 0.5, 0.12, rough)
    # Caroncule rose au coin interne.
    car = _gauss(ex + W * 0.92, ez - 0.0001, 0.0016, 0.0014) * front
    col = _mix(col, np.asarray(h.flush) * 0.9, 0.6 * car)
    # Ligne des cils (paupière supérieure, plus épaisse vers l'extérieur), paupière inférieure.
    lash_w = (0.0007 + 0.0006 * _ss(-0.3, 1.0, ex / W)) * (1.45 if h.female else 1.0)
    lash = np.exp(-((ez - zu - 0.0003) / lash_w) ** 2) * (np.abs(ex) < W * 1.06) * front
    dark = (0.06, 0.045, 0.04) if h.hair[0] < 0.2 else np.asarray(h.hair) * 0.35
    col = _mix(col, dark, 0.9 * lash)
    low = np.exp(-((ez - zl + 0.0002) / 0.0004) ** 2) * (np.abs(ex) < W) * front
    col = _mix(col, np.asarray(h.skin) * 0.62, 0.5 * low)
    crease = np.exp(-((ez - zu * 1.05 - 0.0052) / 0.0007) ** 2) * (np.abs(ex) < W * 1.05) * front
    col = _mix(col, np.asarray(h.skin) * 0.72, 0.45 * crease)
    height -= 0.00015 * crease

    # Cheveux : couleur, mèches, racines, gris ; crâne ras : repousse sur la peau.
    if h.hair_style == "shaved":
        grain = _ss(0.0, 0.7, vnoise3(x, y, z, 0.0010, 51, octaves=1))
        col = _mix(col, np.asarray(h.hair), hair_cov * (0.45 + 0.35 * grain))
        rough = np.where(hair_cov > 0.5, 0.62, rough)
        height += 0.00003 * hair_cov * grain
    else:
        sa = strands(a, e, 420, 14, 61)
        sb = strands(a, e, 160, 5, 62)
        hcol = np.asarray(h.hair) * (0.78 + 0.32 * sa + 0.18 * sb)[..., None]
        # Reflets sur le dessus (direction de la lumière du plafond), racines sombres au bord.
        hcol *= (1.0 + 0.18 * _ss(0.3, 0.9, nz))[..., None]
        edge = np.clip(hair_cov * (1.0 - _ss(0.0, 0.006, z - cf.hairline_z(h, cf.azimuth(x, y)))), 0, 1)
        if h.gray > 0:
            temple = np.exp(-((np.abs(a) - 1.35) / 0.45) ** 2) + 0.35
            gmask = np.clip(h.gray * temple * (0.6 + 0.6 * sa), 0, 1)
            hcol = _mix(hcol, (0.60, 0.58, 0.56), gmask)
        # Bord : quelques mèches laissent voir la peau.
        thin = _ss(-0.2, 0.4, sa) * edge
        cov = np.clip(hair_cov - 0.5 * thin * (1.0 - hair_cov * 0.5), 0, 1)
        col = col * (1 - cov[..., None]) + hcol * cov[..., None]
        rough = np.where(cov > 0.5, 0.50 + 0.12 * sb, rough)
        height += 0.00022 * cov * sa
    # Barbe courte en relief : bord clairsemé (poils isolés), teinte variée.
    if h.beard == "full":
        curl = vnoise3(x, y, z, 0.0016, 71, octaves=2)
        speck = _ss(0.0, 0.5, vnoise3(x, y, z, 0.0009, 72, octaves=1))
        bcol = np.asarray(h.hair) * (0.9 + 0.55 * curl)[..., None]
        soft = cf.beard_cover(h, x, y, z, soft=2.2)
        cov = np.clip(beard * (0.75 + 0.25 * speck) + (soft - beard) * speck * 0.8, 0.0, 1.0)
        col = col * (1 - cov[..., None]) + bcol * cov[..., None]
        rough = np.where(cov > 0.5, 0.68, rough)
        height += 0.00035 * beard * curl

    # Occlusion dans la couleur.
    col *= (1.0 - 0.55 * (1.0 - ao))[..., None]

    normal = _surface_normal(height, x, y, z)
    orm = np.stack([ao, np.clip(rough, 0.05, 1.0), np.zeros_like(ao)], -1)
    return {"color": np.clip(col, 0, 1), "normal": normal, "orm": orm, "emissive": np.zeros_like(col)}


def _surface_normal(height, x, y, z) -> np.ndarray:
    """Carte de normales (espace tangent, OpenGL) d'une hauteur peinte sur la case du visage :
    pentes rapportées à la taille réelle d'un texel le long de U et de V."""
    du = np.sqrt(np.gradient(x, axis=1) ** 2 + np.gradient(y, axis=1) ** 2 + np.gradient(z, axis=1) ** 2)
    dv = np.sqrt(np.gradient(x, axis=0) ** 2 + np.gradient(y, axis=0) ** 2 + np.gradient(z, axis=0) ** 2)
    hu = np.gradient(height, axis=1) / np.maximum(du, 1e-4)
    hv = np.gradient(height, axis=0) / np.maximum(dv, 1e-4)
    n = np.stack([-hu, -hv, np.ones_like(height)], -1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    return n * 0.5 + 0.5


# --- Petites cases ------------------------------------------------------------------------------


def _rim(cv: Canvas) -> np.ndarray:
    return np.minimum(np.minimum(cv.x, cv.tile - cv.x), np.minimum(cv.y, cv.tile - cv.y))


def _stitch(cv: Canvas, d, pitch=0.005):
    dash = (np.sin((cv.x + cv.y) / pitch * 2 * np.pi) > -0.1).astype(np.float32)
    return np.clip(1.0 - np.abs(d) / 0.0008, 0, 1) * dash


def _leather(cv: Canvas, salt: int):
    """Cuir brun patiné : grain fin, plis, éraflures claires sur les bords, couture au bord."""
    d = _rim(cv)
    f1, f2, _ = cv.voronoi(salt, 90, 0.9)
    pebble = np.clip((f2 - f1) / 0.0015, 0, 1)
    h = 0.00003 * pebble + 0.00012 * cv.noise(salt + 1, 0.08, 0.01)
    crease = np.abs(cv.noise(salt + 2, 0.12, 0.012, stretch=(1.0, 2.5)))
    cr = 1.0 - _ss(0.0, 0.06, crease)
    h -= 0.0001 * cr
    h -= 0.0010 * (1.0 - _ss(0.0, 0.008, d))
    st = _stitch(cv, d - 0.007)
    h += 0.00015 * st
    scuff = cut(cv.noise(salt + 3, 0.05, 0.004) + 1.8 * (1 - _ss(0.0, 0.02, d)), 0.12, 0.4)
    base = rgb((0.25, 0.165, 0.11)) * tg.tone(cv, salt + 4, 0.12, 0.06)[..., None]
    albedo = tg.lerp(base, rgb((0.40, 0.27, 0.18)), scuff * 0.40)
    albedo = tg.lerp(albedo, rgb((0.15, 0.09, 0.06)), cr * 0.30)
    albedo = tg.lerp(albedo, rgb((0.50, 0.43, 0.32)), st * 0.6)
    albedo = tg.lerp(albedo, rgb(tg.GRIME), stains(cv, salt + 5, 0.1, 0.10, edge=0.6) * 0.25)
    rough = 0.48 + 0.22 * scuff + 0.05 * pebble
    return tg.finish(cv, h, albedo, rough, np.zeros_like(rough), [(0.003, 0.0005, 0.4), (0.012, 0.0012, 0.3)], edge_light=0.03)


def _towel(cv: Canvas, salt: int):
    k = 0.0028
    wx = 0.5 + 0.5 * np.cos(cv.x / k * 2 * np.pi)
    wy = 0.5 + 0.5 * np.cos(cv.y / k * 2 * np.pi)
    waffle = np.maximum(wx, wy)
    h = 0.00025 * waffle + 0.0006 * cv.noise(salt, 0.1, 0.01, stretch=(1.0, 2.0))
    v = cv.y / cv.tile
    stripe = (_ss(0.70, 0.705, v) * (1 - _ss(0.76, 0.765, v)) + _ss(0.79, 0.795, v) * (1 - _ss(0.81, 0.815, v)))
    base = rgb((0.78, 0.76, 0.70)) * tg.tone(cv, salt + 1, 0.1, 0.06)[..., None]
    albedo = tg.lerp(base, rgb((0.52, 0.19, 0.16)), stripe * 0.85)
    albedo = albedo * (0.9 + 0.1 * waffle)[..., None]
    dirt = stains(cv, salt + 2, 0.08, 0.25, edge=0.7)
    albedo = tg.lerp(albedo, rgb((0.50, 0.44, 0.36)), dirt * 0.45)
    wine = stains(cv, salt + 3, 0.05, 0.06, edge=0.5)
    albedo = tg.lerp(albedo, rgb((0.45, 0.22, 0.22)), wine * 0.35)
    return tg.finish(cv, h, albedo, 0.92 - 0.05 * waffle, np.zeros_like(h), [(0.002, 0.0003, 0.4), (0.01, 0.001, 0.3)], edge_light=0.0)


def _apron(cv: Canvas, salt: int):
    """Toile de tablier : sergé épais, poche cousue au milieu, taches, bords roulés."""
    d = _rim(cv)
    twill = 0.5 + 0.5 * np.sin((cv.x + cv.y) / 0.0022 * np.pi)
    h = 0.00008 * twill + 0.0006 * cv.noise(salt, 0.2, 0.01, beta=1.5, stretch=(1.0, 3.0))
    h -= 0.001 * (1.0 - _ss(0.0, 0.012, d))
    T = cv.tile
    pd = tg.sd_box(cv.x - T * 0.5, cv.y - T * 0.62, T * 0.2, T * 0.12, 0.006)
    pocket = tg.fill(pd, cv.px)
    h += 0.0008 * pocket
    st = _stitch(cv, pd + 0.006, 0.006) + _stitch(cv, d - 0.012, 0.006)
    h += 0.00015 * np.clip(st, 0, 1)
    base = rgb((0.33, 0.17, 0.14)) * tg.tone(cv, salt + 1, 0.2, 0.08)[..., None]
    base = base * (0.93 + 0.07 * twill)[..., None]
    wear = cut(cv.noise(salt + 2, 0.1, 0.006) + 2.0 * (1 - _ss(0.0, 0.03, d)) + 1.5 * (1 - _ss(0.0, 0.01, np.abs(pd))), 0.2, 0.4)
    albedo = tg.lerp(base, rgb((0.44, 0.29, 0.25)), wear * 0.35)
    albedo = tg.lerp(albedo, rgb((0.22, 0.10, 0.09)), stains(cv, salt + 3, 0.06, 0.08, edge=0.5) * 0.35)
    albedo = tg.lerp(albedo, rgb((0.50, 0.42, 0.34)), stains(cv, salt + 4, 0.08, 0.04, edge=0.7) * 0.22)
    albedo = tg.lerp(albedo, rgb((0.20, 0.12, 0.10)), np.clip(st, 0, 1) * 0.5)
    return tg.finish(cv, h, albedo, 0.86 - 0.1 * wear, np.zeros_like(h), [(0.004, 0.0008, 0.4), (0.02, 0.002, 0.3)], edge_light=0.0)


def _skin(cv: Canvas, h: cf.Head, salt: int):
    base = rgb(np.asarray(h.skin) * 0.9) * tg.tone(cv, salt, 0.05, 0.04)[..., None]
    flush = cut(cv.noise(salt + 1, 0.05, 0.01), 0.3, 0.6)
    albedo = tg.lerp(base, rgb(h.flush), flush * 0.2)
    height = 0.00003 * cv.noise(salt + 2, 0.004, 0.0012)
    return tg.finish(cv, height, albedo, np.full_like(height, 0.58), np.zeros_like(height), [(0.002, 0.0002, 0.2)], edge_light=0.0)


def _screen(cv: Canvas, salt: int):
    """Écran de tablette : verre sombre, interface générique (bandeau, lignes, histogramme)."""
    T = cv.tile
    d = _rim(cv)
    albedo = cv.full((0.035, 0.04, 0.045))
    emis = cv.zeros(3)
    u, v = cv.x / T, cv.y / T

    def block(x0, y0, x1, y1, color, level=1.0):
        nonlocal emis
        m = _ss(x0 - 0.004, x0, u) * (1 - _ss(x1, x1 + 0.004, u)) * _ss(y0 - 0.004, y0, v) * (1 - _ss(y1, y1 + 0.004, v))
        emis = emis + rgb(color) * (m * level)[..., None]
        return m

    block(0.08, 0.84, 0.92, 0.92, (0.25, 0.75, 0.85))
    for i in range(4):
        y0 = 0.64 - i * 0.12
        block(0.08, y0, 0.14, y0 + 0.06, (0.95, 0.62, 0.18), 0.9)
        block(0.18, y0, 0.18 + 0.5 - 0.09 * i, y0 + 0.06, (0.30, 0.70, 0.80), 0.55)
    for i in range(6):
        hgt = 0.06 + 0.035 * ((i * 7) % 5)
        block(0.66 + i * 0.045, 0.10, 0.69 + i * 0.045, 0.10 + hgt, (0.35, 0.85, 0.55), 0.8)
    emis *= (0.92 + 0.08 * np.sin(cv.y / 0.0015 * np.pi) ** 2)[..., None]
    emis *= _ss(0.0, 0.01, d)[..., None]
    h = -0.0006 * (1.0 - _ss(0.0, 0.006, d))
    return tg.finish(cv, h, albedo + emis * 0.25, np.full_like(h, 0.18), np.zeros_like(h), [(0.002, 0.0004, 0.3)],
                     emissive=emis, edge_light=0.0)


# --- Assemblage -----------------------------------------------------------------------------------


def generate(only: str | None = None) -> dict[str, Path]:
    maps = {role: np.zeros((RES, RES, 3), np.float32) for role in ("color", "normal", "orm", "emissive")}
    maps["normal"][..., :] = (0.5, 0.5, 1.0)
    maps["orm"][..., :] = (1.0, 0.8, 0.0)

    def put(name, out):
        x0, y0, w, h = RECTS[name]
        for role in maps:
            if role in out:
                maps[role][y0:y0 + h, x0:x0 + w] = out[role]

    for key, head in HEADS.items():
        put(f"face_{key}", paint_face(head))
        put(f"skin_{key}", _skin(Canvas(128, 0.12, seed=900 + ord(key)), head, 10))
    put("leather", _leather(Canvas(256, 0.25, seed=501), 20))
    put("towel", _towel(Canvas(256, 0.25, seed=502), 30))
    put("apron", _apron(Canvas(256, 0.5, seed=503), 40))
    put("screen", _screen(Canvas(128, 0.12, seed=504), 50))
    written = {}
    for role, img in maps.items():
        path = OUT_DIR / f"{NAME}_{role}.png"
        tg.write_png(path, img)
        written[role] = path
    PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
    tg.write_png(PREVIEW_DIR / "atlas_color.png", maps["color"])
    tg.write_png(PREVIEW_DIR / "atlas_normal.png", maps["normal"])
    tg.write_png(PREVIEW_DIR / "atlas_orm.png", maps["orm"])
    print(f"atlas {NAME} : {len(written)} cartes {RES} px dans {OUT_DIR}")
    written.update({f"cloth_{r}": p for r, p in generate_cloth().items()})
    return written


# --- Toile des vêtements -----------------------------------------------------------------------
# Pourquoi pas `char_cloth` (pirate) : sa toile de combinaison porte de grandes taches de
# poussière et de graisse, juste pour un abordeur, mais qui marbrent une chemise ou un gilet de
# guichet. Ici : toile serrée, plis de tombé verticaux, usure discrète sur les plis, rares taches.

CLOTH = "char_clerk_cloth"
CLOTH_TILE = 0.4


def generate_cloth() -> dict[str, Path]:
    cv = Canvas(1024, CLOTH_TILE, seed=8123)
    p = 0.0012
    weave = np.sin(cv.x / p * np.pi) * np.sin(cv.y / p * np.pi)
    h = 0.00003 * weave
    folds = cv.noise(1, 0.12, 0.012, beta=1.6, stretch=(1.0, 3.5))
    h += 0.00022 * folds
    crease = np.zeros_like(folds)
    h += 0.00004 * cv.noise(9, 0.01, 0.002)
    ridge = cv.convexity(h, 0.006, 0.0005)
    wear = np.clip(ridge * 0.6 + 0.2 * cut(cv.noise(3, 0.06, 0.004), 0.12, 0.4), 0.0, 1.0)
    grease = stains(cv, 4, 0.10, 0.04, edge=0.8)
    dust = stains(cv, 5, 0.12, 0.12, edge=0.7)
    base = rgb((0.74, 0.73, 0.71)) * tg.tone(cv, 6, 0.15, 0.035)[..., None]
    base = base * (0.97 + 0.03 * weave)[..., None]
    albedo = tg.lerp(base, rgb((0.84, 0.83, 0.80)), wear * 0.25)
    albedo = albedo * (1.0 - 0.04 * folds)[..., None]
    albedo = tg.lerp(albedo, rgb((0.45, 0.42, 0.38)), grease * 0.18)
    albedo = tg.lerp(albedo, rgb((0.78, 0.75, 0.68)), dust * 0.10)
    rough = np.clip(0.86 - 0.18 * grease + 0.03 * cv.noise(7, 0.05, 0.003), 0.4, 1.0)
    maps = tg.finish(cv, h, albedo, rough, np.zeros_like(rough), [(0.004, 0.0006, 0.15), (0.03, 0.002, 0.12)],
                     edge_light=0.0)
    written = {}
    for role in ("color", "normal", "orm"):
        path = OUT_DIR / f"{CLOTH}_{role}.png"
        tg.write_png(path, maps[role])
        written[role] = path
    PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
    tg.write_png(PREVIEW_DIR / "cloth_color_3x3.png", cv.tile3(maps["color"], 2))
    print(f"toile {CLOTH} : 3 cartes 1024 px dans {OUT_DIR}")
    return written


if __name__ == "__main__":
    generate()
