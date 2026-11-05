"""Coque extérieure de l'intérieur type `corvette` (proposition P12 ; M9, lot C, docs/m9-retours.md
§8.0).

Un petit marchand indépendant de taille corvette (P12), vu de dehors : utilitaire, robuste,
rapiécé, peinture sable passée, plaques de réparation olive et apprêt, traces de chauffe à
l'arrière, pochoirs génériques de la trim sheet, aucun insigne ni inscription. Silhouette
originale : un corps large et plat (« brique » de cargo), une proue en coin qui porte la
verrière, une taille de guêpe à l'arrière flanquée de deux nacelles de propulsion, une troisième
tuyère au centre, une dorsale basse avec radiateurs et parabole. Pas de bouclier, pas d'arme.

Proue (M10, lot C, retour 11 de l'auteur : « trop carrée ») : un seul volume convexe à facettes.
Les épaules se resserrent en coin (≈ 28° par flanc, bouchains sombres, prises d'air sur les
joues) jusqu'aux joues chanfreinées du cockpit ; le toit descend vers une visière blindée qui
déborde en pointe au-dessus d'une verrière inclinée vers l'arrière (≈ 17°) ; le ventre remonte vers
une étrave basse (phares, capteurs). Les ouvertures sont des trapèzes (verrière plus étroite en
haut, vitres latérales au bord avant incliné) ; embrasures et cadres épousent la peau (`Skin`,
lancers de rayons). Le vitrage reste celui de l'intérieur (plan vertical à y = 20,6) : la coque ne
fait que l'encadrer.

Repère : celui du secteur `corvette` (Blender : X tribord, Y avant, Z haut ; pont à z = 0). Le jeu
pose la coque au même repère que l'intérieur (dans le berceau du hangar). Cotes tenues (vérifiées
par le script, qui s'arrête en erreur sinon) :
- enveloppe : x ∈ [−9,3 ; 9,3], y ∈ [−27,7 ; 22,1], z ∈ [−1,7 ; 5,1] (jeu : y ∈ [−1,7 ; 5,1],
  z ∈ [−22,1 ; 27,7]) ; le point le plus bas (patins repliés, à plat) est à z = −1,7 ;
- aucune face à moins de 0,2 m des cellules (cloisons comprises), sauf les embrasures de la
  verrière et le tunnel du sas, qui viennent à 1 cm de la paroi (faces perpendiculaires) ;
- coque pleine autour des cellules (`check_enclosed` : rien ne perce la peau) ;
- coque creuse : le corps et la proue sont évidés (booléen) de l'enveloppe des cellules + 0,2 m,
  et percés aux fenêtres du cockpit et au tunnel : de dehors, on ne voit l'intérieur que par le
  sas et la verrière ;
- tunnel du sas : 1,4 × 2,2 m, de la peau du module d'embarquement (x = −9,3) à la porte
  `sas__@dock` (x = −7,8), centré y = −3,0 ; sol laissé vide (la passerelle du hangar le fournit ;
  caniveau à z = −0,5 sous ses longerons) ;
- vue du pilote (`SOCKET_cockpit_pilot_eye`) dégagée : embrasures évasées, étrave sous la ligne
  de visée, visière au-dessus du champ vertical (75°) ; seuls les rayons rasants restent arrêtés
  (bord gauche de la verrière à ≈ 51° de lacet, haut et coin arrière de la vitre bâbord, dont
  l'embrasure est profonde : les flancs en coin s'écartent du vitrage), voir `check_pilot_view`.

Matériaux (8) : peinture de coque (`hull_corvette`, texture neuve cuite par ce script : 2K,
tuile de 4 m, plaques sable, olive et apprêt, écaillures, coulures), tôle de réparation olive
(`panel_painted` teinté), `metal_dark`, `metal_bare`, `trim_ship` (bandes de danger, pochoirs,
grilles, tuyaux, câbles), feux (pastilles `warship_lights` du lot D de M8, émission ≈ 4), lueur
des tuyères (mêmes pastilles, ≈ 1,5), métal chauffé (`metal_bare` bruni). Géométrie empruntée à
`space/warships.py` (fuseaux à pans coupés, tours, plaques rapiécées), en lecture seule.

Décor par niveau (`detail`, docs/m9-retours.md §3.2) : 0 = la coque complète et sobre ; 1 =
antennes fines, tuyaux, pochoirs, poignées ; 2 = plaques rapiécées en relief, câbles, petits
boîtiers. Fusion par matériau et par niveau (`common.join_by_material`). Pas de lightmap :
éclairage dynamique.

Sorties :
- assets/models/ships/corvette_hull/corvette_hull.gltf (+ .bin) : maillages fusionnés, colliders
  `corvette_hull_<partie>_col` (pavés de l'enveloppe, tunnel libre ; étrave `bow` et joues
  `cheek_port` / `cheek_starboard` en morceaux convexes de la proue) et
  `corvette_hull_tunnel_<partie>_col` (parois et plafond du tunnel) ;
- assets/models/ships/corvette_hull/corvette_hull_lod.gltf : version lointaine (≤ 3 000 triangles) ;
- assets/textures/materials/hull_corvette_{color,normal,orm}.png (cuites si absentes, ou
  `--textures`) ;
- art/ships/corvette_hull.blend.

    blender --background --factory-startup --python tools/blender/ships/corvette_hull.py -- [options]

Options : `--textures` (recuire la texture de coque), `--previews` (aperçus Cycles dans
target/art-previews/C/), `--only a,b` (prises de vue), `--samples N`, `--res LxH`. Les aperçus
importent l'intérieur depuis la copie figée `target/art-previews/m9-lotC/ref/corvette/` si elle
existe, sinon depuis `assets/models/ships/corvette/`.
"""

from __future__ import annotations

import contextlib
import io
import math
import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "space"))
sys.path.insert(0, str(HERE.parent / "materials"))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

import common as c  # noqa: E402
import warships as ws  # noqa: E402
from warships import IDENTITY, Rot, Sec, T  # noqa: E402

NAME = "corvette_hull"
OUT = "assets/models/ships/corvette_hull"
BLEND = "art/ships/corvette_hull.blend"
PREVIEWS = c.ROOT / "target" / "art-previews" / "C"
REF_DIRS = (c.ROOT / "target" / "art-previews" / "m9-lotC" / "ref" / "corvette", c.ROOT / "assets" / "models" / "ships" / "corvette")

# --- Cotes de l'intérieur (lot B, `ships/corvette.py`), repère Blender, cloisons comprises -------

CELL_BOXES = {
    "cockpit": ((-4.2, 14.2, -0.2), (4.2, 20.6, 3.2)),
    "chambre": ((-7.8, 7.8, -0.2), (-1.4, 14.2, 3.0)),
    "quartiers": ((-7.8, 1.4, -0.2), (-1.4, 7.8, 3.0)),
    "carre": ((1.4, 1.4, -0.2), (7.8, 9.8, 3.0)),
    "couloirs": ((-1.4, -14.2, -0.2), (1.4, 14.2, 2.8)),
    "sas": ((-7.8, -5.0, -0.2), (-1.4, -1.0, 2.8)),
    "soute": ((1.4, -13.6, -0.2), (7.8, -1.4, 3.6)),
    "machines": ((-5.2, -26.2, -0.2), (5.2, -14.2, 3.6)),
}
CLEARANCE = 0.2  # jeu minimal entre la coque et les cloisons des cellules
ENVELOPE = ((-9.3, -27.7, -1.7), (9.3, 22.1, 5.1))  # HULL_MARGIN = 1,5 m du hangar (lot D)
PILOT_EYE = (-1.0, 19.18, 1.25)  # SOCKET_cockpit_pilot_eye (intérieur)

# Porte extérieure du sas (`sas__@dock`, 1,4 × 2,2 m, paroi x ∈ [−7,8 ; −7,6]) et tunnel.
HATCH_Y = -3.0
HATCH_W, HATCH_H = 1.4, 2.2
HATCH_WALL = -7.8
SKIN_X = -9.3  # peau du module d'embarquement
CHANNEL_Z = -0.5  # fond du caniveau sous la passerelle (longerons jusqu'à −0,26)
WALL_GAP = 0.01  # les embrasures s'arrêtent à 1 cm des cloisons

# Verrière : (nom, axe de la normale, sens de la normale, plan du vitrage + 1 cm, rectangle du
# vitrage (u0, u1, v0, v1), trapèze de l'ouverture dans la peau (u0 bas, u1 bas, u0 haut, u1 haut,
# v0, v1 : évasé là où regarde le pilote, resserré en haut pour la ligne de la verrière), coupures
# des bords haut et bas, en fraction de la largeur : le pli du V de la verrière).
# Axe "y" : u = x, v = z ; axe "x" : u = y, v = z. Le vitrage est celui de l'intérieur (lot B).
WINDOWS = [
    ("front", "y", 1.0, 20.6 + WALL_GAP, (-3.5, 3.5, 0.85, 2.7), (-4.2, 4.2, -3.6, 3.6, 0.42, 3.26), (0.25, 0.5, 0.75)),
    ("port", "x", -1.0, -4.2 - WALL_GAP, (18.6, 20.0, 0.85, 2.7), (17.85, 20.45, 18.0, 20.2, 0.62, 3.4), (0.5,)),
    ("starboard", "x", 1.0, 4.2 + WALL_GAP, (18.6, 20.0, 0.85, 2.7), (17.85, 20.45, 18.0, 20.2, 0.62, 3.4), (0.5,)),
]
FRAME_W = 0.12  # largeur des cadres de la verrière, sur la peau

# --- Formes (sections à pans coupés : Sec(y, demi-largeur, haut, bas, chanfrein haut, cx, cz,
# chanfrein bas)) ; chaque section est vérifiée contre les cellules par `check_clearance`. --------

BODY = [
    Sec(-26.6, 6.0, 4.4, 1.1, 0.9, 0.0, 0.0, 0.8),  # poupe de la taille (salle des machines)
    Sec(-15.2, 6.0, 4.4, 1.1, 0.9, 0.0, 0.0, 0.8),
    Sec(-13.9, 9.0, 4.6, 1.3, 1.5, 0.0, 0.0, 1.6),  # épaules : soute, sas ; carène biseautée
    Sec(10.0, 9.0, 4.6, 1.3, 1.5, 0.0, 0.0, 1.6),
]
FORE_Y = 10.0  # le corps s'arrête ici ; la proue commence (raccord exact sur sa section)

# Proue : un seul volume convexe à facettes (enveloppe des points ci-dessous, x ≥ 0, miroir
# automatique), vérifié contre les cellules par `check_enclosed`.
# - épaules : la section du corps (18 m) jusqu'à y = 12,9, puis les flancs se resserrent en coin
#   (≈ 28°) jusqu'aux joues (9,4 m) : la chambre (bâbord, jusqu'à y = 14,2) reste dedans ;
# - toit qui descend doucement, puis visière (brow) qui plonge vers la verrière ;
# - verrière plane inclinée vers l'arrière (`WS_*`), sous une visière blindée en V ;
# - étrave sous la verrière, ventre qui remonte ;
# - le cockpit (x ±4,4, y ≤ 20,8, z −0,4 → 3,4, jeu compris) tient dans les joues.
SHOULDER = [(0.0, 12.9, 4.6), (7.45, 12.9, 4.56), (9.0, 12.9, 3.52), (9.0, 12.9, -0.6), (7.7, 12.9, -1.28), (0.0, 12.9, -1.28)]
CHEEK = ((4.7, 20.95, 3.55), (4.7, 20.95, -0.6))  # arêtes avant des flancs (haut, bas)
BROW = ((0.0, 19.4, 3.98), (3.7, 19.3, 3.95))  # début de la visière (faîte, arête)
# Verrière plane, inclinée de ≈ 17° : son arête haute frôle l'angle avant-haut du cockpit, ce qui
# interdit un V (la pointe est reportée sur la visière blindée, `VISOR_TIP`).
WS_TOP = ((0.0, 20.98, 3.56), (4.4, 20.98, 3.54))  # arête haute de la verrière (milieu, coin)
WS_BOT = (0.0, 22.0, 0.22)  # arête basse, au milieu ; le coin s'en déduit (verrière plane)
VISOR_TIP = (22.0, 3.47)  # pointe de la visière blindée (y, z)
CHIN = (0.0, 21.98, -0.22)  # bas de l'étrave, au milieu
CHIN_W = 3.0  # demi-largeur du bas de l'étrave : grandes joues basses chanfreinées de part et d'autre
BELLY = ((0.0, 20.2, -0.88), (3.7, 20.35, -0.84))  # avant du ventre


def ws_corners() -> tuple[Vector, Vector, Vector, Vector]:
    """Coins de la moitié tribord de la verrière : haut (faîte, coin), bas (faîte, coin)."""
    t0, t1 = Vector(WS_TOP[0]), Vector(WS_TOP[1])
    b0 = Vector(WS_BOT)
    return t0, t1, b0, t1 + (b0 - t0)  # parallélogramme : plan


def fore_points() -> list[tuple]:
    """Demi-proue (x ≥ 0)."""
    t0, t1, b0, b1 = ws_corners()
    chin0 = Vector(CHIN)
    chin1 = chin0 + (b1 - b0) * (CHIN_W / b1.x)  # étrave plane sous la verrière, plus étroite
    body = [(0.0, FORE_Y, 4.6), (7.5, FORE_Y, 4.6), (9.0, FORE_Y, 3.1), (9.0, FORE_Y, 0.3), (7.4, FORE_Y, -1.3), (0.0, FORE_Y, -1.3)]
    pts = body + SHOULDER + list(CHEEK) + list(BROW) + list(BELLY)
    pts += [tuple(t0), tuple(t1), tuple(b0), tuple(b1), tuple(chin0), tuple(chin1)]
    return pts


SPINE = [  # dorsale basse
    Sec(-25.8, 1.4, 0.3, 0.3, 0.25, 0.0, 4.5),
    Sec(-23.5, 2.3, 0.4, 0.4, 0.35, 0.0, 4.5),
    Sec(4.0, 2.3, 0.4, 0.4, 0.35, 0.0, 4.5),
    Sec(7.5, 1.2, 0.2, 0.4, 0.15, 0.0, 4.5),
]
POD_X, POD_Z, POD_R = 7.45, 1.5, 1.75  # nacelles de propulsion
POD = [(0.0, -13.85), (1.5, -13.85), (POD_R, -14.6), (POD_R, -24.8), (1.62, -26.2), (1.5, -26.6), (0.0, -26.6)]
STERN = -26.6
SKID_X = 4.6
SKID_STRUTS = (-18.5, -8.0, 2.4, 12.0)  # au droit des sabots du berceau (lot D)

# --- Texture de coque (neuve) --------------------------------------------------------------------

TEXTURE = "hull_corvette"
TEX_RES = 2048
TEX_TILE = 6.0  # mètres par tuile (≈ 340 px/m) : plaques de 1,5 m

# Densité des textures par matériau (mètres par tuile).
TILE = {"paint": TEX_TILE, "accent": 2.0, "dark": 2.0, "bare": 2.0, "heat": 2.0}
EMISSIVE = ("lights", "glow")
LIGHT_STRENGTH = 4.0  # feux de navigation (AGENTS.md §4 : ≈ 4 pour un feu)
GLOW_STRENGTH = 1.5  # lueur des tuyères (≈ 1 à 2)


def write_hull_texture() -> None:
    """Cuit `hull_corvette_{color,normal,orm}.png` (couleur et normale 2K, ORM 1K ; tuile de 4 m) :
    plaques de bordé décalées en rangées de 1 m, rivets, trappes vissées ; peinture sable passée,
    quelques plaques remplacées (olive, apprêt gris, sable clair), écaillures en deux couches
    (apprêt puis métal) près des joints, coulures sous les rivets, crasse, rayures. Couleurs
    cuites (matériau non teinté). Déterministe (graines fixes) ; ≈ 30 s."""
    import numpy as np
    import texgen as tg

    res, tile = TEX_RES, TEX_TILE
    cv = tg.Canvas(res, tile, seed=9127)
    rng = cv.rng(1)
    rects = []
    offsets = [0.0, 0.6, 1.3, 0.25]
    widths = [[1.5, 1.25, 1.25], [1.0, 2.0, 1.0], [1.5, 1.0, 1.5], [1.25, 1.5, 1.25]]
    for j in range(4):
        x = offsets[j]
        for w in widths[j]:
            rects.append((x, float(j), x + w, float(j + 1)))
            x += w
    d, ident = tg.panel_field(cv, rects, corner=0.03)
    n = len(rects)
    h = tg.seam_profile(d, 0.005, 0.012, 0.006)
    tilt = rng.normal(0, 0.0012, (n, 2))
    centers = np.array([((r[0] + r[2]) / 2, (r[1] + r[3]) / 2) for r in rects], dtype=np.float32)
    inside = tg.smoothstep(0.004, 0.02, d)
    h += inside * (tilt[ident, 0] * cv.wrap_dx(cv.x, centers[ident, 0]) + tilt[ident, 1] * cv.wrap_dy(cv.y, centers[ident, 1]))
    riv = cv.zeros()
    for j in range(4):
        for side in (-1, 1):
            riv = np.maximum(riv, tg.rivets_along_x(cv, j + side * 0.045, 0.1, 0.012, 0.003, x_off=0.05))
    riv *= tg.smoothstep(0.025, 0.035, d)
    h += riv
    hatch = cv.zeros()
    for hx, hy, hw, hh in ((0.8, 0.5, 0.22, 0.16), (3.1, 2.45, 0.16, 0.26), (1.9, 3.5, 0.3, 0.13), (2.6, 1.5, 0.12, 0.12)):
        hd = tg.sd_box(cv.wrap_dx(cv.x, hx), cv.wrap_dy(cv.y, hy), hw, hh, 0.03)
        hatch = np.maximum(hatch, 1.0 - tg.smoothstep(0.0, 0.006, np.abs(hd)))
    h -= 0.003 * hatch

    # Peinture par plaque : sable passé, plaques remplacées (olive, apprêt, sable clair, sable sombre).
    sand = (0.63, 0.57, 0.46)
    paints = [sand] * n
    for i, col in ((2, (0.50, 0.50, 0.40)), (5, (0.52, 0.51, 0.48)), (7, (0.69, 0.65, 0.55)), (10, (0.57, 0.51, 0.41))):
        paints[i] = col
    paints = np.array(paints, dtype=np.float32)
    shade = (1.0 + rng.normal(0, 0.035, n)).astype(np.float32)
    base = paints[ident] * shade[ident][..., None]
    base = base * (tg.tone(cv, 7, 2.0, 0.06) * tg.tone(cv, 8, 0.15, 0.025))[..., None]
    fade = tg.smoothstep(-0.3, 1.4, cv.noise(11, 1.5, 0.05))  # plages délavées
    base = tg.lerp(base, base * 0.6 + tg.rgb((0.80, 0.78, 0.72)) * 0.4, (0.45 * fade)[..., None])
    # Écaillures : apprêt, puis métal, près des joints et des trappes.
    edge = 1.0 - tg.smoothstep(0.006, 0.07, d)
    chips = tg.chip_field(cv, 3, np.maximum(edge, hatch * 0.8), 0.015, edge_weight=3.0)
    primer = tg.cut(chips, 0.08, 0.06)
    bare = tg.cut(chips, 0.03, 0.06)
    h -= 0.0002 * primer
    col = tg.lerp(base, tg.rgb(tg.PRIMER), primer[..., None])
    col = tg.lerp(col, tg.rgb(tg.STEEL) * 0.85, bare[..., None])
    # Coulures sous les rivets, les trappes et les joints ; crasse ; rayures ; impacts.
    seam = 1.0 - tg.smoothstep(0.0, 0.01, d)
    src = np.clip((riv > 0.0005).astype(np.float32) + 0.6 * hatch + 0.4 * seam, 0.0, 1.0) * (cv.noise(4, 0.6, 0.1) > -0.3)
    streak = tg.drips(cv, src, 0.7, 5)
    col = tg.lerp(col, tg.rgb(tg.GRIME), (0.35 * streak)[..., None])
    grime = tg.stains(cv, 12, 0.8, 0.07)
    col = tg.lerp(col, tg.rgb((0.30, 0.27, 0.22)), (0.18 * grime)[..., None])
    scratch = cv.scratches(13, 500, (0.05, 0.4), 0.002)
    col = tg.lerp(col, tg.rgb(tg.STEEL) * 0.8, (0.45 * scratch * (1.0 - bare))[..., None])
    pits = tg.cut(-cv.noise(2, 0.02, cv.px * 1.5) + 0.8 * cv.noise(21, 0.5, 0.1), 0.004, 0.3)
    h -= 0.0008 * pits
    col = tg.lerp(col, tg.rgb((0.2, 0.2, 0.2)), (pits * 0.6)[..., None])
    rough = 0.62 + 0.05 * cv.noise(9, 0.12, 0.01) + 0.1 * streak + 0.08 * grime - 0.25 * bare - 0.1 * scratch
    metal = 0.05 + 0.45 * bare + 0.2 * scratch
    maps = tg.finish(cv, h, col, rough, metal, [(0.01, 0.003, 0.45), (0.05, 0.006, 0.3)], edge_light=0.1)
    tg.write_png(c.TEXTURES_DIR / f"{TEXTURE}_color.png", maps["color"].astype(np.float32))
    tg.write_png(c.TEXTURES_DIR / f"{TEXTURE}_normal.png", maps["normal"].astype(np.float32))
    tg.write_png(c.TEXTURES_DIR / f"{TEXTURE}_orm.png", tg.downsample(maps["orm"], 2).astype(np.float32))


# --- Matériaux (8) -------------------------------------------------------------------------------


def palette() -> dict:
    return {
        "paint": c.textured_material("corvette_paint", TEXTURE),
        # Tôles de réparation et module d'embarquement : panneaux peints olive passé.
        "accent": c.textured_material("corvette_accent", "panel_painted", tint=(0.52, 0.55, 0.40)),
        "dark": c.textured_material("corvette_dark", "metal_dark"),
        "bare": c.textured_material("corvette_bare", "metal_bare", tint=(0.72, 0.70, 0.67)),
        "trim": c.textured_material("corvette_trim", "trim_ship"),
        # Métal bruni par la chaleur (tuyères, plaque de poupe).
        "heat": c.textured_material("corvette_heat", "metal_bare", tint=(0.50, 0.42, 0.38)),
        "lights": c.textured_material("corvette_lights", ws.LIGHT_SHEET, emissive=(1.0, 1.0, 1.0),
                                      emissive_strength=LIGHT_STRENGTH),
        "glow": c.textured_material("corvette_glow", ws.LIGHT_SHEET, emissive=(1.0, 1.0, 1.0),
                                    emissive_strength=GLOW_STRENGTH),
    }


# --- Assemblage ----------------------------------------------------------------------------------


def apply_uv(obj, mat: str, trim, mpt: float, swatch, tile, offset) -> None:
    if mat in EMISSIVE:
        u, v = ws.swatch_uv(swatch or "nav_white")
        layer = obj.data.uv_layers[0] if obj.data.uv_layers else obj.data.uv_layers.new(name=c.UV_TEXTURE)
        layer.data.foreach_set("uv", [u, v] * len(obj.data.loops))
    elif mat == "trim":
        c.uv_trim(obj, trim or "edge", meters_per_tile=mpt)
    else:
        c.uv_box(obj, tile or TILE[mat], offset=offset)


class Hull:
    """Pièces de la coque : maillages dans le repère du vaisseau, matériau, UV, niveau de décor
    (`level`), pièces à évider (`cut`), pièces autorisées près des cloisons (`near_wall`)."""

    def __init__(self, mats: dict, coll, seed: int, prefix: str = "ch"):
        self.mats = mats
        self.coll = coll
        self.prefix = prefix
        self.rng = random.Random(seed)
        self.parts: list[bpy.types.Object] = []
        self.detail = 0
        self.cut: list[bpy.types.Object] = []
        self.near_wall: set[str] = set()
        self._uv: dict[str, tuple] = {}

    @contextlib.contextmanager
    def level(self, detail: int):
        old, self.detail = self.detail, detail
        try:
            yield
        finally:
            self.detail = old

    def add(self, bm, mat: str, M: Matrix = IDENTITY, trim=None, mpt: float = 2.0, swatch=None, tile=None,
            cut: bool = False, near_wall: bool = False):
        bmesh.ops.transform(bm, matrix=M, verts=bm.verts)
        if M.determinant() < 0:
            bmesh.ops.reverse_faces(bm, faces=bm.faces)
        name = f"{self.prefix}_{mat}_{len(self.parts):04d}"
        mesh = bpy.data.meshes.new(name)
        bm.to_mesh(mesh)
        bm.free()
        mesh.materials.append(self.mats[mat])
        obj = bpy.data.objects.new(name, mesh)
        self.coll.objects.link(obj)
        uv = (mat, trim, mpt, swatch, tile, (self.rng.random(), self.rng.random()))
        if cut:
            self.cut.append(obj)
            self._uv[obj.name] = uv
        else:
            apply_uv(obj, *uv)
        if self.detail:
            c.set_detail([obj], self.detail)
        if near_wall:
            self.near_wall.add(obj.name)
        self.parts.append(obj)
        return obj

    def split(self, obj, pred, mat: str):
        """Sépare les faces de `obj` dont la normale vérifie `pred` dans une nouvelle pièce de
        matériau `mat` (livrée : biseaux hauts olive, ventre sombre)."""
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bm.normal_update()
        picked = [f for f in bm.faces if pred(f.normal)]
        if not picked:
            bm.free()
            return None
        other = bm.copy()
        bmesh.ops.delete(bm, geom=picked, context="FACES")
        bm.to_mesh(obj.data)
        bm.free()
        other.normal_update()
        bmesh.ops.delete(other, geom=[f for f in other.faces if not pred(f.normal)], context="FACES")
        return self.add(other, mat)

    def finish_cut(self) -> None:
        for obj in self.cut:
            apply_uv(obj, *self._uv[obj.name])

    # Raccourcis -------------------------------------------------------------------------------

    def box(self, mat, size, at, M: Matrix = IDENTITY, bevel=0.0, rot: Matrix = IDENTITY, **kw):
        return self.add(ws.bm_box(*size, bevel=bevel), mat, M @ T(*at) @ rot, **kw)

    def aabox(self, mat, lo, hi, bevel=0.0, **kw):
        """Pavé aligné sur les axes, donné par ses coins."""
        size = tuple(b - a for a, b in zip(lo, hi))
        at = tuple((a + b) / 2 for a, b in zip(lo, hi))
        return self.box(mat, size, at, bevel=bevel, **kw)

    def loft(self, mat, secs, M: Matrix = IDENTITY, **kw):
        return self.add(ws.bm_loft(secs), mat, M, **kw)

    def lathe(self, mat, profile, M: Matrix = IDENTITY, seg=12, closed=False, smooth=0.0, caps=True, **kw):
        return self.add(ws.bm_lathe(profile, seg, closed, smooth, caps), mat, M, **kw)

    def band(self, mat, r, y0, y1, M: Matrix = IDENTITY, seg=12, **kw):
        return self.lathe(mat, [(r, y0), (r, y1)], M, seg=seg, caps=False, **kw)

    def cyl(self, mat, r, length, M: Matrix = IDENTITY, seg=12, **kw):
        return self.add(ws.bm_cyl(r, length, seg), mat, M, **kw)

    def rod(self, mat, p0, p1, r, seg=6, **kw):
        """Cylindre de p0 à p1."""
        p0, p1 = Vector(p0), Vector(p1)
        d = p1 - p0
        rot = Vector((0, 1, 0)).rotation_difference(d.normalized()).to_matrix().to_4x4()
        return self.cyl(mat, r, d.length, T(*((p0 + p1) / 2)) @ rot, seg=seg, **kw)

    def light(self, swatch, at, size=0.24, M: Matrix = IDENTITY, mat="lights"):
        r = size / 2
        prof = [(0, -r), (r * 0.7, -r * 0.7), (r, 0), (r * 0.7, r * 0.7), (0, r)]
        return self.lathe(mat, prof, M @ T(*at), seg=8, swatch=swatch)

    def sleeve(self, mat, inner, outer, axis=None, **kw):
        """Embrasure ouverte : un quadrilatère par côté, de la boucle `inner` à la boucle `outer`
        (même nombre de points, dans le même ordre), faces tournées vers l'axe de l'ouverture
        (vues de dehors comme du dedans de l'ouverture). `axis` : axe de l'ouverture (par défaut,
        du centre de `inner` vers celui de `outer`)."""
        bm = bmesh.new()
        n = len(inner)
        vi = [bm.verts.new(p) for p in inner]
        vo = [bm.verts.new(p) for p in outer]
        ci = sum((Vector(p) for p in inner), Vector()) / n
        co = sum((Vector(p) for p in outer), Vector()) / n
        axis = Vector(axis).normalized() if axis is not None else (co - ci).normalized()
        for i in range(n):
            j = (i + 1) % n
            f = bm.faces.new((vi[i], vi[j], vo[j], vo[i]))
            f.normal_update()
            cen = f.calc_center_median()
            on_axis = ci + axis * (cen - ci).dot(axis)
            if f.normal.dot(on_axis - cen) < 0:
                f.normal_flip()
        return self.add(bm, mat, near_wall=True, **kw)


# --- Volumes à facettes et peau ------------------------------------------------------------------


def mirrored(half) -> list[tuple]:
    """Points d'une demi-coque (x ≥ 0) et leurs symétriques (x < 0)."""
    out = []
    for p in half:
        out.append(tuple(p))
        if abs(p[0]) > 1e-6:
            out.append((-p[0], p[1], p[2]))
    return out


def bm_hull(points) -> bmesh.types.BMesh:
    """Enveloppe convexe des points : facettes planes (triangles coplanaires fusionnés), normales
    vers l'extérieur."""
    bm = bmesh.new()
    verts = [bm.verts.new(p) for p in points]
    bmesh.ops.remove_doubles(bm, verts=verts, dist=1e-5)
    res = bmesh.ops.convex_hull(bm, input=bm.verts[:], use_existing_faces=False)
    junk = [g for g in list(res["geom_interior"]) + list(res["geom_unused"]) if isinstance(g, bmesh.types.BMVert)]
    if junk:
        bmesh.ops.delete(bm, geom=list(set(junk)), context="VERTS")
    bmesh.ops.dissolve_limit(bm, angle_limit=math.radians(0.3), verts=bm.verts[:], edges=bm.edges[:])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def hull_points_clipped(points, planes) -> list[tuple]:
    """Sommets de l'enveloppe convexe de `points` coupée par des demi-espaces (point du plan,
    normale sortante : on garde ce qui est derrière) ; l'enveloppe de ces sommets est le morceau
    coupé (colliders, membrures)."""
    bm = bm_hull(points)
    for co, no in planes:
        geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
        bmesh.ops.bisect_plane(bm, geom=geom, plane_co=tuple(co), plane_no=tuple(no), dist=1e-6, clear_outer=True)
    pts = [tuple(v.co) for v in bm.verts]
    bm.free()
    return pts


def hull_points_between(points, lo: Vector, hi: Vector) -> list[tuple]:
    """Morceau de l'enveloppe convexe de `points` dans le pavé [lo, hi]."""
    planes = []
    for axis in range(3):
        for value, sign in ((hi[axis], 1.0), (lo[axis], -1.0)):
            co = [0.0, 0.0, 0.0]
            co[axis] = value
            no = [0.0, 0.0, 0.0]
            no[axis] = sign
            planes.append((co, no))
    return hull_points_clipped(points, planes)


def grown(points, grow: float, zc: float) -> list[tuple]:
    """Points écartés de `grow` dans leur section (autour de l'axe x = 0, z = zc) : une membrure
    qui ceinture un volume convexe."""
    out = []
    for x, y, z in points:
        d = Vector((x, 0.0, z - zc))
        d = d.normalized() if d.length > 1e-6 else Vector((0.0, 0.0, 1.0))
        out.append((x + d.x * grow, y, z + d.z * grow))
    return out


class Skin:
    """Peau extérieure de la coque (avant l'évidement) : points et normales par lancer de rayon,
    repères tangents pour poser du décor sur des facettes inclinées."""

    def __init__(self, objs):
        self.tree = _bvh(objs)

    def hit(self, origin, direction, dist: float = 40.0) -> tuple[Vector, Vector]:
        o, d = Vector(origin), Vector(direction).normalized()
        loc, nor, _, _ = self.tree.ray_cast(o, d, dist)
        if loc is None:
            raise RuntimeError(f"peau introuvable depuis {tuple(round(v, 2) for v in o)} vers {tuple(round(v, 2) for v in d)}")
        return loc, nor.normalized()

    @staticmethod
    def frame(p: Vector, n: Vector) -> Matrix:
        """Repère local : Z = normale, X horizontal le long de la facette (ou X du vaisseau sur une
        facette presque horizontale), Y = Z × X."""
        n = Vector(n).normalized()
        if abs(n.z) > 0.92:
            u = Vector((1.0, 0.0, 0.0))
            u = (u - n * u.dot(n)).normalized()
        else:
            u = Vector((0.0, 0.0, 1.0)).cross(n).normalized()
        v = n.cross(u)
        M = Matrix((u, v, n)).transposed().to_4x4()
        M.translation = p
        return M

    def at(self, origin, direction) -> Matrix:
        return self.frame(*self.hit(origin, direction))


# --- Sous-ensembles ------------------------------------------------------------------------------


def nozzle(h: Hull, M: Matrix, r_throat: float, r_exit: float, length: float, wall: float = 0.12) -> None:
    """Tuyère en cloche vers −Y (y = 0 au col, contre la poupe) : cloche en métal chauffé, collier,
    disque bleu au col, cœur plus clair, doublure lumineuse le long de la paroi intérieure."""
    prof = [(r_throat + wall, 0.0), (r_exit + wall * 0.6, -length), (r_exit, -length),
            (r_throat * 1.02, -0.25), (r_throat * 0.98, 0.0)]
    h.lathe("heat", prof, M, seg=24, closed=True, smooth=30.0)
    h.lathe("dark", [(r_throat + wall, 0.18), (r_throat + wall + 0.1, 0.18), (r_throat + wall + 0.1, -0.2),
                     (r_throat + wall * 0.9, -0.2)], M, seg=24, closed=True)
    h.lathe("glow", [(0, -0.2), (r_throat, -0.2), (r_throat, -0.12), (0, -0.12)], M, seg=24, swatch="engine_blue")
    h.lathe("glow", [(0, -0.3), (r_throat * 0.55, -0.3), (r_throat * 0.55, -0.22), (0, -0.22)], M, seg=16,
            swatch="core_blue")

    def wall_r(y: float) -> float:
        t = (-y - 0.25) / (length - 0.25)
        return r_throat * 1.02 + (r_exit - r_throat * 1.02) * t

    y0, y1 = -0.3, -length * 0.8
    prof = [(wall_r(y0) - 0.02, y0), (wall_r(y1) - 0.02, y1), (wall_r(y1) - 0.06, y1), (wall_r(y0) - 0.06, y0)]
    h.lathe("glow", prof, M, seg=24, closed=True, swatch="liner_blue")


def nav_light(h: Hull, swatch: str, at, normal, size=0.2) -> None:
    """Feu sur un petit socle sombre posé sur la coque (`normal` : axe de la paroi)."""
    p, n = Vector(at), Vector(normal).normalized()
    base = p - n * (size * 0.55)
    h.box("dark", (0.22, 0.22, 0.22), tuple(base))
    h.light(swatch, tuple(p), size)


def quad_loop(q, breaks) -> list[tuple[float, float]]:
    """Contour d'un trapèze (u0 bas, u1 bas, u0 haut, u1 haut, v0, v1) : bas (gauche → droite, avec
    les coupures en fraction de la largeur), droite, haut (droite → gauche), gauche. Les coupures
    tombent sur les plis de la peau (V de la verrière) : embrasure et cadre les suivent."""
    u0b, u1b, u0t, u1t, v0, v1 = q
    ts = [0.0] + list(breaks)
    out = [(u0b + (u1b - u0b) * t, v0) for t in ts]
    out += [(u1b, v0), (u1t, v1)]
    out += [(u0t + (u1t - u0t) * t, v1) for t in reversed(ts[1:])]
    out.append((u0t, v1))
    return out


def rect_quad(r) -> tuple:
    u0, u1, v0, v1 = r
    return (u0, u1, u0, u1, v0, v1)


def grow_quad(q, d: float) -> tuple:
    """Trapèze agrandi de `d` sur chaque bord (bords obliques décalés perpendiculairement)."""
    u0b, u1b, u0t, u1t, v0, v1 = q
    h = v1 - v0

    def side(ub, ut, sign):  # bord oblique décalé vers l'extérieur (sign : −1 gauche, +1 droite)
        du = ut - ub
        shift = d * math.hypot(du, h) / h  # décalage horizontal d'une droite décalée de d
        u_at = lambda v: ub + du * (v - v0) / h + sign * shift  # noqa: E731
        return u_at(v0 - d), u_at(v1 + d)

    lb, lt = side(u0b, u0t, -1.0)
    rb, rt = side(u1b, u1t, 1.0)
    return (lb, rb, lt, rt, v0 - d, v1 + d)


def window_loops(skin: Skin) -> list[dict]:
    """Pour chaque vitrage : contour du vitrage (plan intérieur), contour de l'ouverture et contour
    extérieur du cadre, posés sur la peau par des rayons tirés le long de l'axe du vitrage."""
    out = []
    for name, axis, sgn, w_in, inner, outer, breaks in WINDOWS:
        d = Vector((0.0, sgn, 0.0)) if axis == "y" else Vector((sgn, 0.0, 0.0))

        def P(w, u, v, axis=axis):
            return Vector((u, w, v)) if axis == "y" else Vector((w, u, v))

        def on_skin(q, P=P, d=d, w_in=w_in, sgn=sgn, breaks=breaks):
            hits = [skin.hit(P(w_in + sgn * 0.05, u, v), d) for u, v in quad_loop(q, breaks)]
            return [p for p, _ in hits], [n for _, n in hits]

        a, na = on_skin(outer)
        b, nb = on_skin(grow_quad(outer, FRAME_W))
        out.append({"name": name, "axis": axis, "sgn": sgn, "d": d, "w_in": w_in, "outer": outer, "P": P,
                    "inner": [P(w_in, u, v) for u, v in quad_loop(rect_quad(inner), breaks)], "a": a, "na": na, "b": b, "nb": nb})
    return out


def conform_frame(h: Hull, a, na, b, nb, axis: Vector, mat: str = "dark", rise: float = 0.06, sink: float = 0.04,
                  lip: float = 0.06):
    """Cadre posé sur la peau entre le contour de l'ouverture `a` et le contour extérieur `b`
    (normales de la peau `na`, `nb`) : dessus à `rise` de la peau, rebord extérieur enfoncé de
    `sink`, rebord intérieur qui plonge de `lip` dans l'ouverture (il cache la coupe)."""
    bm = bmesh.new()
    at = [bm.verts.new(p + n * rise) for p, n in zip(a, na)]
    bt = [bm.verts.new(p + n * rise) for p, n in zip(b, nb)]
    bb = [bm.verts.new(p - n * sink) for p, n in zip(b, nb)]
    ai = [bm.verts.new(p - axis * lip) for p in a]
    center = sum(a, Vector()) / len(a)
    count = len(a)

    def radial(m: Vector) -> Vector:
        r = m - center
        return r - axis * r.dot(axis)

    for i in range(count):
        j = (i + 1) % count
        n_skin = (na[i] + na[j] + nb[i] + nb[j]).normalized()
        for quad, want in (((at[i], at[j], bt[j], bt[i]), "skin"), ((bt[i], bt[j], bb[j], bb[i]), "out"),
                           ((ai[i], ai[j], at[j], at[i]), "in")):
            f = bm.faces.new(quad)
            f.normal_update()
            m = f.calc_center_median()
            ref = n_skin if want == "skin" else (radial(m) if want == "out" else -radial(m))
            if f.normal.dot(ref) < 0:
                f.normal_flip()
    return h.add(bm, mat, near_wall=True)


def windows(h: Hull, cutters, skin: Skin) -> None:
    """Embrasures évasées (métal nu devant, sombre sur les flancs, plus profonds), cadres et coupes
    des trois vitrages du cockpit, posés sur la peau."""
    for w in window_loops(skin):
        d, sgn, P = w["d"], w["sgn"], w["P"]
        h.sleeve("bare" if w["axis"] == "y" else "dark", w["inner"], w["a"], axis=d)
        conform_frame(h, w["a"], w["na"], w["b"], w["nb"], d)
        # Coupe : prisme du trapèze (agrandi de 2 cm), du cœur (évidé) jusqu'au-delà de la peau.
        far = max(p.dot(d) for p in w["a"] + w["b"]) + 0.4
        q = grow_quad(w["outer"], 0.02)
        corners = [(q[0], q[4]), (q[1], q[4]), (q[3], q[5]), (q[2], q[5])]
        pts = [tuple(P(w["w_in"] + sgn * 0.05, u, v)) for u, v in corners] + [tuple(P(sgn * far, u, v)) for u, v in corners]
        bm = bm_hull(pts)
        me = bpy.data.meshes.new(f"cut_window_{w['name']}")
        bm.to_mesh(me)
        bm.free()
        cutters.objects.link(bpy.data.objects.new(me.name, me))


def boarding_module(h: Hull, cutters, cols) -> None:
    """Module d'embarquement à bâbord : bloc rapporté (peau à x = −9,3), tunnel 1,4 × 2,2 m jusqu'à
    la porte du sas, encadrement, bandes de danger, feux ambre, auvent ; colliders du tunnel."""
    y0, y1 = HATCH_Y - HATCH_W / 2, HATCH_Y + HATCH_W / 2
    h.aabox("accent", (-9.22, -6.0, -0.8), (-8.5, 0.0, 3.3), bevel=0.08, cut=True)
    e = 0.02
    c.box("cut_tunnel", (1.7, HATCH_W + 2 * e, HATCH_H - CHANNEL_Z + 2 * e),
          ((-9.6 + -7.9) / 2, HATCH_Y, (CHANNEL_Z + HATCH_H) / 2), None, cutters)
    # Doublure du tunnel (plafond, parois, fond du caniveau), de la peau à 1 cm de la paroi du sas.
    xi, xo = HATCH_WALL - WALL_GAP, SKIN_X
    rect = [(y0, CHANNEL_Z), (y1, CHANNEL_Z), (y1, HATCH_H), (y0, HATCH_H)]
    h.sleeve("dark", [(xi, y, z) for y, z in rect], [(xo, y, z) for y, z in rect])
    # Encadrement (x −9,28 → −9,18), plaques de danger en saillie à −9,3.
    fw = 0.24
    xa, xb = -9.28, -9.18
    h.aabox("dark", (xa, y0 - fw, CHANNEL_Z), (xb, y0, HATCH_H + fw), bevel=0.02)
    h.aabox("dark", (xa, y1, CHANNEL_Z), (xb, y1 + fw, HATCH_H + fw), bevel=0.02)
    h.aabox("dark", (xa, y0 - fw, HATCH_H), (xb, y1 + fw, HATCH_H + fw), bevel=0.02)
    h.aabox("dark", (xa, y0 - fw, CHANNEL_Z - 0.25), (xb, y1 + fw, CHANNEL_Z), bevel=0.02)
    for ya, yb in ((y0 - fw + 0.03, y0 - 0.03), (y1 + 0.03, y1 + fw - 0.03)):
        h.aabox("trim", (-9.3, ya, -0.3), (-9.27, yb, HATCH_H + 0.1), trim="hazard", mpt=1.0)
    h.aabox("trim", (-9.3, y0 - fw + 0.03, HATCH_H + 0.05), (-9.27, y1 + fw - 0.03, HATCH_H + fw - 0.04),
            trim="hazard", mpt=1.0)
    # Auvent, feux ambre de part et d'autre, rampe lumineuse au plafond du tunnel.
    h.aabox("dark", (-9.3, y0 - 0.6, 2.62), (-9.05, y1 + 0.6, 2.78), bevel=0.02)
    for y in (y0 - 0.45, y1 + 0.45):
        h.aabox("dark", (-9.3, y - 0.11, 2.3), (-9.16, y + 0.11, 2.5))
        h.light("beacon_amber", (-9.19, y, 2.4), 0.16)
    h.aabox("lights", (-9.0, HATCH_Y - 0.05, HATCH_H - 0.03), (-8.2, HATCH_Y + 0.05, HATCH_H), swatch="beacon_amber")
    # Habillage du module : nervures, grilles, poignées et pochoir (niveau 1).
    for y in (-5.6, -0.4):
        h.aabox("dark", (-9.26, y - 0.12, -0.75), (-9.12, y + 0.12, 3.25))
    h.aabox("trim", (-9.24, -5.3, 2.9), (-9.2, -4.3, 3.15), trim="vent", mpt=2.0)
    with h.level(1):
        for y in (y0 - 0.7, y1 + 0.7):
            h.rod("bare", (-9.24, y, 0.6), (-9.24, y, 1.9), 0.025)
            for z in (0.6, 1.9):
                h.aabox("dark", (-9.24, y - 0.03, z - 0.03), (-9.2, y + 0.03, z + 0.03))
        h.aabox("trim", (-9.235, -1.6, 2.85), (-9.215, -0.7, 3.15), trim="stencils", mpt=2.0)
    # Colliders du tunnel : parois et plafond (le sol est la passerelle du hangar).
    xs = (SKIN_X + HATCH_WALL) / 2
    lx = HATCH_WALL - SKIN_X
    cols.append(c.collider_box(f"{NAME}_tunnel_aft_col", (lx, 0.5, HATCH_H - CHANNEL_Z), (xs, y0 - 0.25, (HATCH_H + CHANNEL_Z) / 2), h.coll))
    cols.append(c.collider_box(f"{NAME}_tunnel_fwd_col", (lx, 0.5, HATCH_H - CHANNEL_Z), (xs, y1 + 0.25, (HATCH_H + CHANNEL_Z) / 2), h.coll))
    cols.append(c.collider_box(f"{NAME}_tunnel_top_col", (lx, HATCH_W + 1.0, 1.2), (xs, HATCH_Y, HATCH_H + 0.6), h.coll))


def cargo_door(h: Hull) -> None:
    """Porte de soute à tribord, en relief : deux battants, nervures, charnières en haut, bandes de
    danger au seuil, chevrons."""
    y0, y1, z0, z1 = -11.6, -3.6, 0.45, 3.0
    h.aabox("accent", (8.94, y0, z0), (9.12, y1, z1), bevel=0.03)
    fw = 0.16
    for lo, hi in (((8.92, y0 - fw, z1), (9.2, y1 + fw, z1 + fw)), ((8.92, y0 - fw, z0 - fw), (9.2, y1 + fw, z0)),
                   ((8.92, y0 - fw, z0), (9.2, y0, z1)), ((8.92, y1, z0), (9.2, y1 + fw, z1))):
        h.aabox("dark", lo, hi, bevel=0.02)
    h.aabox("dark", (9.1, (y0 + y1) / 2 - 0.05, z0), (9.16, (y0 + y1) / 2 + 0.05, z1))  # jointure des battants
    for z in (1.1, 1.9, 2.6):
        for ya, yb in ((y0 + 0.25, (y0 + y1) / 2 - 0.25), ((y0 + y1) / 2 + 0.25, y1 - 0.25)):
            h.aabox("accent", (9.1, ya, z - 0.07), (9.18, yb, z + 0.07), bevel=0.02)
    for y in (y0 + 0.8, (y0 + y1) / 2 - 1.2, (y0 + y1) / 2 + 1.2, y1 - 0.8):
        h.aabox("bare", (9.12, y - 0.2, z1 + 0.02), (9.24, y + 0.2, z1 + 0.14))  # charnières
    h.aabox("trim", (9.2, y0 - fw, z0 - fw + 0.02), (9.22, y1 + fw, z0 - 0.02), trim="hazard", mpt=1.0)
    h.aabox("trim", (9.1, y0 + 0.6, z0 + 0.15), (9.14, y0 + 2.6, z0 + 0.35), trim="chevrons", mpt=2.0)
    h.aabox("trim", (9.1, y1 - 2.6, z0 + 0.15), (9.14, y1 - 0.6, z0 + 0.35), trim="chevrons", mpt=2.0)
    with h.level(1):
        h.aabox("trim", (9.1, (y0 + y1) / 2 + 0.6, 2.7), (9.14, (y0 + y1) / 2 + 1.6, 2.95), trim="stencils", mpt=2.0)
        for y in ((y0 + y1) / 2 - 0.35, (y0 + y1) / 2 + 0.35):  # poignées
            h.rod("bare", (9.17, y, 1.25), (9.17, y, 1.75), 0.025)
            for z in (1.25, 1.75):
                h.aabox("dark", (9.1, y - 0.03, z - 0.03), (9.18, y + 0.03, z + 0.03))


def engines(h: Hull) -> None:
    """Nacelles de propulsion de part et d'autre de la taille, trois tuyères, plaque de poupe
    bruneie, pylônes."""
    for sx in (-1, 1):
        M = T(sx * POD_X, 0.0, POD_Z)
        h.lathe("paint", POD, M, seg=32, smooth=35.0)
        for y in (-16.8, -20.6):
            h.band("dark", POD_R + 0.07, y - 0.25, y + 0.25, M, seg=32)
        h.lathe("heat", [(POD_R + 0.03, -24.9), (1.66, -26.25)], M, seg=32, caps=False)
        nozzle(h, M @ T(0.0, STERN, 0.0), 1.0, 1.36, 1.1)
        # Pylônes haut et bas entre la nacelle et la taille.
        for z in (POD_Z + 1.2, POD_Z - 1.15):
            h.aabox("dark", (min(sx * 5.8, sx * 6.9), -24.0, z - 0.18), (max(sx * 5.8, sx * 6.9), -15.8, z + 0.18), bevel=0.04)
        # Grilles d'admission et trappes sur le dessus de la nacelle.
        h.box("trim", (0.9, 3.0, 0.06), (sx * POD_X, -18.7, POD_Z + POD_R - 0.01), trim="vent", mpt=2.0)
    # Poupe de la taille : plaque bruneie, tuyère centrale.
    h.aabox("heat", (-5.2, STERN - 0.09, -0.6), (5.2, STERN + 0.05, 3.8), bevel=0.03)
    nozzle(h, T(0.0, STERN - 0.09, 1.7), 0.72, 0.95, 0.95)
    for x in (-3.4, 3.4):  # ventouses de manœuvre
        h.cyl("dark", 0.28, 0.3, T(x, STERN - 0.2, 3.2) @ Rot(0, "X"), seg=12)
        h.lathe("glow", [(0, -0.36), (0.18, -0.36), (0.18, -0.34), (0, -0.34)], T(x, STERN, 3.2), seg=12, swatch="liner_blue")


def dorsal(h: Hull) -> None:
    """Dorsale, radiateurs, parabole, mât, antennes (fines : niveau 1)."""
    h.loft("paint", SPINE)
    for y in (-20.0, -9.0, 1.5):
        h.loft("dark", ws.shell(SPINE, y - 0.2, y + 0.2, 0.05))
    h.box("trim", (1.6, 4.0, 0.05), (0.0, -14.5, 4.9), trim="vent", mpt=2.0)
    # Radiateurs sur la taille, de part et d'autre de la dorsale.
    for sx in (-1, 1):
        x = sx * 3.9
        h.aabox("dark", (x - 0.55, -24.4, 4.38), (x + 0.55, -16.6, 4.52))
        for y in (-24.4, -16.6):
            h.aabox("dark", (x - 0.55, y - 0.12, 4.4), (x + 0.55, y + 0.12, 5.02))
        for k in range(14):
            y = -24.2 + (k + 0.5) * 7.4 / 14
            h.aabox("bare", (x - 0.5, y - 0.05, 4.5), (x + 0.5, y + 0.05, 4.98))
        h.aabox("dark", (x - 0.55, -24.4, 4.96), (x - 0.45, -16.6, 5.04))
        h.aabox("dark", (x + 0.45, -24.4, 4.96), (x + 0.55, -16.6, 5.04))
    # Parabole sur le pont avant, tournée vers l'avant et le haut.
    dish_at = Vector((-4.4, 7.0, 4.6))
    h.cyl("dark", 0.12, 0.3, T(*(dish_at + Vector((0, 0, 0.12)))) @ Rot(90, "X"), seg=8)
    r = 0.3
    prof = [(0.0, 0.0), (r * 0.5, r * 0.08), (r, r * 0.3), (r * 0.94, r * 0.36), (r * 0.46, r * 0.16), (0.0, 0.1 * r)]
    M = T(*(dish_at + Vector((0, 0, 0.24)))) @ Rot(60, "X")
    h.lathe("bare", prof, M, seg=16, closed=True, smooth=40.0)
    h.add(ws.bm_cyl(0.03, r * 0.8, 4), "dark", M @ T(0, r * 0.4, 0))
    # Mât court et feux du dessus.
    h.aabox("dark", (-0.2, -8.2, 4.86), (0.2, -7.8, 4.96))
    h.light("beacon_amber", (0.0, -8.0, 5.0), 0.2)
    h.light("nav_white", (0.0, -25.95, 4.72), 0.24)
    with h.level(1):
        for x, y, z0, z1 in ((-1.0, -24.6, 4.75, 5.08), (1.0, -24.6, 4.75, 5.06), (2.6, 3.0, 4.6, 5.08)):
            h.rod("bare", (x, y, z0), (x, y, z1), 0.02, seg=4)
            h.aabox("dark", (x - 0.06, y - 0.06, z0 - 0.04), (x + 0.06, y + 0.06, z0 + 0.06))
        h.rod("bare", (-0.9, 9.0, 4.4), (-0.9, 9.0, 5.05), 0.025, seg=4)
        h.rod("bare", (-1.2, 9.0, 4.85), (-0.6, 9.0, 4.85), 0.015, seg=4)


def skids(h: Hull) -> None:
    """Patins repliés sous le ventre : deux longerons posés sur les sabots du berceau (dessous à
    z = −1,7), jambes repliées dans des logements, vérins."""
    for sx in (-1, 1):
        x = sx * SKID_X
        h.aabox("dark", (x - 0.3, -20.0, -1.7), (x + 0.3, 13.0, -1.48), bevel=0.03)
        h.aabox("bare", (x - 0.24, -19.6, -1.5), (x + 0.24, 12.6, -1.42))
        for y, tilt in ((-20.3, -1), (13.3, 1)):  # bouts relevés (dessous à −1,7)
            zc = -1.7 + 0.45 * math.sin(math.radians(25)) + 0.1 * math.cos(math.radians(25))
            h.box("dark", (0.6, 0.9, 0.2), (x, y, zc), rot=Rot(tilt * 25, "X"))
        for y in SKID_STRUTS:
            h.aabox("dark", (x - 0.45, y - 0.9, -1.36), (x + 0.45, y + 0.9, -1.0))  # logement
            h.box("dark", (0.34, 1.4, 0.16), (x, y, -1.42), rot=Rot(-8, "X"))  # jambe repliée
            h.rod("bare", (x + sx * 0.32, y - 0.6, -1.15), (x + sx * 0.32, y + 0.5, -1.4), 0.05)
    # Trappes et feu ambre du ventre.
    for x, y, w, l in ((0.0, -10.0, 2.0, 2.6), (-5.6, 5.5, 1.6, 1.6), (5.6, 5.5, 1.6, 1.6), (0.0, 9.0, 1.4, 2.0)):
        h.aabox("dark", (x - w / 2, y - l / 2, -1.36), (x + w / 2, y + l / 2, -1.28))
    h.light("beacon_amber", (0.0, -2.0, -1.4), 0.2)


def side_details(h: Hull, skin: Skin) -> None:
    """Membrures, feux de navigation, pochoirs, tuyaux, câbles, trappes et grilles."""
    for y0, y1 in ((-13.6, -13.1), (0.6, 1.2), (7.2, 7.7), (-25.2, -24.7), (-19.4, -18.9)):
        h.loft("dark", ws.shell(BODY, y0, y1, 0.1), cut=True)  # anneau évidé : ses bouts traversent les cellules
    # Feux de navigation : rouge à bâbord, vert à tribord (épaules et nacelles), blanc à la poupe.
    for sx, sw in ((-1, "nav_red"), (1, "nav_green")):
        p, n = skin.hit((sx * 12.0, 11.2, 2.0), (-sx, 0, 0))
        nav_light(h, sw, tuple(p + n * 0.1), tuple(n))
        nav_light(h, sw, (sx * POD_X, -25.6, POD_Z + POD_R + 0.12), (0, 0, 1))
    # Grilles et trappes du dessus (niveau 0).
    for x, y in ((-6.2, -9.5), (6.2, -9.5), (-6.2, 3.5), (6.0, 4.5)):
        h.box("trim", (1.2, 2.0, 0.05), (x, y, 4.62), trim="vent", mpt=2.0)
    for x, y in ((-4.8, -2.0), (4.8, -1.0), (3.4, 8.6)):
        h.aabox("dark", (x - 0.6, y - 0.6, 4.58), (x + 0.6, y + 0.6, 4.64))
    # Bandes de danger le long des épaules (garde-corps de maintenance), niveau 0.
    for sx in (-1, 1):
        h.box("trim", (0.08, 3.0, 0.2), (sx * 9.03, 8.4, 3.1), trim="warning_red", mpt=1.0)
    with h.level(1):
        # Tuyaux le long des flancs (brides), supports.
        for sx, spans in ((-1, ((0.6, 9.5), (-13.2, -6.4))), (1, ((-2.8, 9.5), (-13.2, -12.2)))):
            for ya, yb in spans:
                if yb - ya < 1.5:
                    continue
                x = sx * 9.12
                h.add(ws.bm_cyl(0.11, yb - ya, 10), "trim", T(x, (ya + yb) / 2, 0.6), trim="flange", mpt=2.0)
                k = int((yb - ya) // 2.0)
                for i in range(k + 1):
                    y = ya + 0.3 + i * (yb - ya - 0.6) / max(k, 1)
                    h.aabox("dark", (min(x, sx * 8.95), y - 0.06, 0.43), (max(x, sx * 8.95), y + 0.06, 0.75))
        # Pochoirs génériques (bande de la trim sheet).
        for lo, hi in (((-9.03, 2.0, 2.3), (-8.98, 3.0, 2.6)), ((8.98, 0.5, 2.3), (9.03, 1.5, 2.6))):
            h.aabox("trim", lo, hi, trim="stencils", mpt=2.0)
        h.box("trim", (1.0, 0.04, 0.28), (0.0, STERN - 0.1, 3.35), trim="stencils", mpt=2.0)
    with h.level(2):
        # Câbles le long de la dorsale, boîtiers, valves.
        for sx in (-1, 1):
            h.add(ws.bm_cyl(0.07, 15.0, 8), "trim", T(sx * 2.45, -4.5, 4.67), trim="cables", mpt=2.0)
        rng = h.rng
        for _ in range(26):
            sx = rng.choice((-1, 1))
            y = rng.uniform(-12.8, 9.5)
            if sx < 0 and -6.6 < y < 0.6:
                continue
            if sx > 0 and -12.2 < y < -3.0:
                continue
            z = rng.uniform(0.75, 2.6)
            w, l, t = rng.uniform(0.25, 0.6), rng.uniform(0.3, 0.9), rng.uniform(0.06, 0.16)
            mat = rng.choice(("dark", "dark", "bare"))
            h.aabox(mat, (min(sx * 8.98, sx * (9.0 + t)), y - l / 2, z - w / 2), (max(sx * 8.98, sx * (9.0 + t)), y + l / 2, z + w / 2))
        for _ in range(18):
            x = rng.uniform(-6.6, 6.6)
            y = rng.uniform(-12.8, 8.5)
            if abs(x) < 2.8 or (abs(x) > 3.3 and -24.5 < y < -16.4):
                continue
            w, l = rng.uniform(0.3, 0.8), rng.uniform(0.3, 1.0)
            h.aabox(rng.choice(("dark", "bare")), (x - w / 2, y - l / 2, 4.58), (x + w / 2, y + l / 2, 4.58 + rng.uniform(0.08, 0.2)))


def extra_details(h: Hull) -> None:
    """Trappes de visite, points d'amarrage, blocs de manœuvre arrière (niveau 0) ; rambardes du
    pont (niveau 1) ; réservoirs, conduites et boîtiers (niveau 2). La proue a les siens
    (`nose_details`)."""
    # Trappes de visite sur les flancs (cadre sombre, tôle olive, poignée).
    for sx, ys in ((-1, (-11.0, 5.0)), (1, (-1.0, 6.5))):
        for y in ys:
            x = sx * 9.0
            h.aabox("dark", (min(x - sx * 0.04, x + sx * 0.06), y - 0.75, 1.0), (max(x - sx * 0.04, x + sx * 0.06), y + 0.75, 2.3), bevel=0.02)
            h.aabox("accent", (min(x, x + sx * 0.09), y - 0.62, 1.12), (max(x, x + sx * 0.09), y + 0.62, 2.18))
            h.aabox("trim", (min(x, x + sx * 0.1), y - 0.4, 1.9), (max(x, x + sx * 0.1), y + 0.4, 2.05), trim="warning_red", mpt=1.0)
    # Points d'amarrage du dessus (blocs sombres, bande de danger).
    for x, y in ((-6.0, -12.5), (6.0, -12.5), (-6.0, 8.8), (6.0, 8.8)):
        h.aabox("dark", (x - 0.45, y - 0.45, 4.56), (x + 0.45, y + 0.45, 4.82), bevel=0.04)
        h.aabox("bare", (x - 0.18, y - 0.18, 4.8), (x + 0.18, y + 0.18, 4.92))
        h.aabox("trim", (x - 0.45, y - 0.47, 4.6), (x + 0.45, y - 0.44, 4.78), trim="hazard", mpt=1.0)
    # Blocs de manœuvre aux angles arrière (tuyères de contrôle d'attitude, éteintes).
    for sx in (-1, 1):
        for y, z in ((-25.4, 3.9),):
            x = sx * 5.85
            h.aabox("dark", (x - 0.35, y - 0.35, z - 0.3), (x + 0.35, y + 0.35, z + 0.3), bevel=0.03)
            h.cyl("bare", 0.09, 0.14, T(x + sx * 0.38, y, z) @ Rot(90, "Z"), seg=8)
            h.cyl("bare", 0.09, 0.14, T(x, y, z + 0.37) @ Rot(90, "X"), seg=8)
    with h.level(1):
        # Rambardes de maintenance le long du pont.
        for sx in (-1, 1):
            x = sx * 7.2
            h.rod("bare", (x, -12.6, 4.95), (x, 9.4, 4.95), 0.025, seg=6)
            for k in range(12):
                y = -12.6 + k * 22.0 / 11
                h.rod("bare", (x, y, 4.58), (x, y, 4.97), 0.022, seg=4)
    with h.level(2):
        # Réservoirs couchés sur le pont avant, à tribord.
        for x in (5.6, 6.5):
            M = T(x, 4.8, 4.83)  # dessus des sangles à 5,09 m (enveloppe : 5,1)
            r, L = 0.24, 5.4
            prof = [(0, -L / 2), (r * 0.6, -L / 2 + 0.05), (r, -L / 2 + 0.3), (r, L / 2 - 0.3), (r * 0.6, L / 2 - 0.05), (0, L / 2)]
            h.lathe("bare", prof, M, seg=12, smooth=40.0)
            for y in (-1.6, 0.0, 1.6):
                h.band("dark", r + 0.02, y - 0.08, y + 0.08, M, seg=12)
            for y in (-2.0, 2.0):
                h.aabox("dark", (x - 0.3, 4.8 + y - 0.1, 4.58), (x + 0.3, 4.8 + y + 0.1, 4.68))
        # Conduites sur la taille, entre radiateurs et dorsale.
        for sx in (-1, 1):
            for dx in (2.6, 2.85):
                h.add(ws.bm_cyl(0.07, 9.0, 8), "trim", T(sx * dx, -20.0, 4.47), trim="flange", mpt=2.0)
        # Boîtiers et capots sur les nacelles.
        for sx in (-1, 1):
            for y in (-15.5, -22.5):
                h.aabox("dark", (sx * POD_X - 0.4, y - 0.6, POD_Z + POD_R - 0.08), (sx * POD_X + 0.4, y + 0.6, POD_Z + POD_R + 0.14), bevel=0.03)
        rng = h.rng
        for _ in range(40):
            x = rng.uniform(-6.8, 6.8)
            y = rng.uniform(-12.8, 9.0)
            if abs(x) < 2.8 or (x > 5.1 and 1.6 < y < 8.2) or (x < -3.6 and 6.0 < y < 8.0):
                continue
            w, l, t = rng.uniform(0.15, 0.5), rng.uniform(0.2, 0.7), rng.uniform(0.05, 0.18)
            h.aabox(rng.choice(("dark", "bare", "dark")), (x - w / 2, y - l / 2, 4.58), (x + w / 2, y + l / 2, 4.6 + t))


def repair_patches(h: Hull) -> None:
    """Plaques de réparation : quelques grandes tôles olive au niveau 0, beaucoup de tôles en relief
    soudées au niveau 2 (`warships.patches`)."""

    def occupied(p: Vector) -> bool:
        if p.x < -8.0 and -6.6 < p.y < 0.6:
            return True  # module d'embarquement
        if p.x > 8.0 and -12.3 < p.y < -2.9:
            return True  # porte de soute
        if p.z > 4.3 and abs(p.x) < 2.9:
            return True  # dorsale
        if p.x < -8.0 and 10.6 < p.y < 12.4:
            return True  # feux
        if p.x > 8.0 and 10.6 < p.y < 12.4:
            return True
        return False

    sides = [s._replace(ch=max(s.ch, s.cb)) for s in BODY]  # flanc plat : entre les deux biseaux
    ws.patches(h, sides, -13.0, 9.8, 4, [("accent", 0.7), ("dark", 0.3)], skip=occupied, weld=0.0, faces=("side_l", "side_r"))
    ws.patches(h, BODY, -13.0, 9.8, 3, [("accent", 0.7), ("dark", 0.3)], skip=occupied, weld=0.0, faces=("top",))
    with h.level(2):
        ws.patches(h, sides, -13.0, 9.8, 12, [("accent", 0.35), ("paint", 0.4), ("dark", 0.25)], skip=occupied, weld=0.6,
                   faces=("side_l", "side_r"))
        ws.patches(h, BODY, -13.0, 9.8, 10, [("accent", 0.35), ("paint", 0.4), ("dark", 0.25)], skip=occupied, weld=0.6,
                   faces=("top", "ch_tl", "ch_tr"))
        ws.patches(h, BODY, -26.0, -15.8, 6, [("accent", 0.5), ("paint", 0.5)], skip=occupied, weld=0.5, faces=("top",))


def skin_box(h: Hull, skin: Skin, mat: str, origin, direction, size, at=(0.0, 0.0), sink: float = 0.015,
             spin: float = 0.0, **kw):
    """Pavé posé à plat sur la peau (dans le repère de la facette touchée par le rayon) : `size` =
    (le long de la facette, en travers, épaisseur), `at` décale le long de la facette."""
    M = skin.at(origin, direction)
    return h.box(mat, size, (at[0], at[1], size[2] / 2 - sink), M=M, rot=Rot(spin, "Z"), **kw)


def slab(h: Hull, mat: str, pts, normal: Vector, top: float, bottom: float = 0.02, **kw):
    """Plaque plane épaisse : polygone `pts` (plan de normale `normal`), de −`bottom` à +`top`."""
    n = Vector(normal).normalized()
    cloud = [tuple(Vector(p) + n * top) for p in pts] + [tuple(Vector(p) - n * bottom) for p in pts]
    return h.add(bm_hull(cloud), mat, **kw)


def intake(h: Hull, skin: Skin, origin, direction, length: float = 1.7, height: float = 0.75) -> None:
    """Prise d'air plaquée sur une joue : caisson sombre, capot en visière, lames inclinées."""
    M = skin.at(origin, direction)
    h.box("dark", (length, height, 0.3), (0.0, 0.0, 0.13), M=M, bevel=0.03)
    h.box("accent", (length + 0.16, 0.1, 0.42), (0.0, height / 2 + 0.03, 0.19), M=M, bevel=0.02)  # capot
    h.box("bare", (length - 0.2, height - 0.16, 0.02), (0.0, 0.0, 0.26), M=M)  # fond clair (grille vue de biais)
    for k in range(5):
        v = -height / 2 + 0.14 + k * (height - 0.28) / 4
        h.box("dark", (length - 0.22, 0.05, 0.16), (0.0, v, 0.33), M=M, rot=Rot(35, "X"))


def nose_details(h: Hull, skin: Skin) -> None:
    """Proue : visière blindée, membrures, prises d'air, capteurs, phares, blocs de manœuvre
    (niveau 0) ; antennes, sondes, poignées (niveau 1) ; plaques soudées, boîtiers (niveau 2)."""
    fore = mirrored(fore_points())
    # Membrures : à l'épaulement et au milieu du coin.
    big = 50.0
    for y0, y1, grow, zc in ((12.45, 12.85, 0.08, 1.6), (16.9, 17.25, 0.07, 1.5)):
        ring = hull_points_between(fore, Vector((-big, y0, -big)), Vector((big, y1, big)))
        h.add(bm_hull(grown(ring, grow, zc)), "dark", cut=True)

    # Bouchains : corniche sombre le long de l'arête basse des flancs en coin.
    for sx in (-1, 1):
        a = Vector((sx * SHOULDER[3][0], SHOULDER[3][1], SHOULDER[3][2]))
        b = Vector((sx * CHEEK[1][0], CHEEK[1][1], CHEEK[1][2]))
        along = (b - a).normalized()
        out = Vector((along.y, -along.x, 0.0)).normalized()
        if out.x * sx < 0:
            out = -out
        a, b = a + along * 0.2, b - along * 0.3
        slab(h, "dark", [a - out * 0.06, b - out * 0.06, b + out * 0.18, a + out * 0.18], Vector((0.0, 0.0, 1.0)), 0.06, 0.06)

    # Visière blindée en V : plaque posée sur le haut de la visière, dont la pointe déborde de
    # 0,75 m au-dessus de la verrière (au-dessus du champ vertical du pilote, 37,5°).
    t0, t1, _, _ = ws_corners()
    tip = Vector((0.0, VISOR_TIP[0], VISOR_TIP[1]))

    def back(x: float) -> Vector:
        return skin.hit((x, 19.8, 9.0), (0.0, 0.0, -1.0))[0]

    def front(x: float) -> Vector:  # bord avant de la visière (pointe → coin)
        corner = Vector((math.copysign(t1.x - 0.2, x) if x else t1.x - 0.2, t1.y + 0.14, t1.z + 0.03))
        k = abs(x) / (t1.x - 0.2)
        return Vector((x, tip.y + (corner.y - tip.y) * k, tip.z + (corner.z - tip.z) * k))

    for sx in (-1, 1):
        xe = sx * (t1.x - 0.2)
        quad = [back(0.0), back(xe), front(xe), front(0.0)]
        n = (quad[1] - quad[0]).cross(quad[3] - quad[0]).normalized()
        if n.z < 0:
            n = -n
        slab(h, "dark", quad, n, 0.09, 0.03)
        for x in (1.2, 2.5):  # nervures le long de la pente
            bk, fr = back(sx * x), front(sx * x)
            w = Vector((0.025, 0.0, 0.0))
            slab(h, "bare", [bk - w + n * 0.08, bk + w + n * 0.08, fr + w + n * 0.08, fr - w + n * 0.08], n, 0.05, 0.0)
        # Gousset sous la pointe, contre la verrière (il porte le débord).
        g0 = front(sx * 0.9) - n * 0.03
        slab(h, "dark", [g0, g0 + Vector((0.0, -0.6, 0.0)), g0 + Vector((0.0, -0.6, -0.22))], Vector((1.0, 0.0, 0.0)), 0.03, 0.03)
    bk, fr = back(0.0), front(0.0)  # couvre-joint du faîte
    w = Vector((0.07, 0.0, 0.0))
    n0 = (fr - bk).cross(Vector((1.0, 0.0, 0.0))).normalized()
    if n0.z < 0:
        n0 = -n0
    slab(h, "bare", [bk - w, bk + w, fr + w, fr - w], n0, 0.14, 0.02)

    # Arête dorsale de la proue : couvre-joint sombre du faîte, de l'épaulement à la visière.
    ridge = [skin.hit((0.0, y, 9.0), (0.0, 0.0, -1.0))[0] for y in (12.95, 15.0, 17.2, 19.75)]
    for p0, p1 in zip(ridge, ridge[1:]):
        w = Vector((0.28, 0.0, 0.0))
        n = (p1 - p0).cross(Vector((1.0, 0.0, 0.0))).normalized()
        if n.z < 0:
            n = -n
        slab(h, "dark", [p0 - w, p0 + w, p1 + w, p1 - w], n, 0.1, 0.03)

    # Prises d'air sur les joues du cou (dièdre entre les épaules et le module du cockpit).
    for sx in (-1, 1):
        intake(h, skin, (sx * 12.0, 15.9, 1.55), (-sx, 0.0, 0.0))
        # Grille basse et trappe sur la même joue.
        skin_box(h, skin, "dark", (sx * 12.0, 15.6, -0.15), (-sx, 0.0, 0.0), (1.3, 0.42, 0.05), sink=0.02)
        skin_box(h, skin, "trim", (sx * 12.0, 15.6, -0.15), (-sx, 0.0, 0.0), (1.1, 0.26, 0.07), trim="vent", mpt=2.0)

    # Blocs de manœuvre sur les arêtes hautes du cou (tuyères de contrôle d'attitude, éteintes).
    for sx in (-1, 1):
        M = skin.at((sx * 12.0, 12.2, 7.0), (-sx, 0.0, -1.0))
        h.box("dark", (0.75, 0.75, 0.42), (0.0, 0.0, 0.18), M=M, bevel=0.04)
        h.cyl("bare", 0.1, 0.16, M @ T(0.0, 0.0, 0.46) @ Rot(90, "X"), seg=8)
        h.cyl("bare", 0.1, 0.16, M @ T(0.0, 0.43, 0.2), seg=8)
        h.cyl("bare", 0.1, 0.16, M @ T(0.0, -0.43, 0.2), seg=8)

    # Toit du module : dôme de capteurs, lame d'antenne, trappe d'accès, grilles du cou.
    M = skin.at((2.3, 18.5, 9.0), (0.0, 0.0, -1.0))
    h.box("dark", (1.0, 1.0, 0.12), (0.0, 0.0, 0.04), M=M, bevel=0.02)
    h.lathe("bare", [(0.0, 0.0), (0.42, 0.0), (0.4, 0.12), (0.3, 0.3), (0.16, 0.4), (0.0, 0.43)],
            M @ T(0.0, 0.0, 0.1) @ Rot(90, "X"), seg=16, smooth=40.0)
    skin_box(h, skin, "dark", (-1.3, 18.4, 9.0), (0.0, 0.0, -1.0), (0.06, 0.9, 0.36), sink=0.02)  # lame d'antenne
    skin_box(h, skin, "accent", (-2.6, 18.2, 9.0), (0.0, 0.0, -1.0), (1.0, 1.2, 0.05), sink=0.02, spin=3.0)  # trappe
    for x in (-3.0, 3.0):
        skin_box(h, skin, "trim", (x, 11.6, 9.0), (0.0, 0.0, -1.0), (1.2, 1.6, 0.05), sink=0.02, trim="vent", mpt=2.0)

    # Étrave : phares d'atterrissage et fenêtre de capteur dans la facette sous la verrière ;
    # tourelle de capteurs sous le ventre.
    for x in (-2.3, 2.3):
        M = skin.at((x, 25.0, -0.02), (0.0, -1.0, 0.0))
        h.box("dark", (0.5, 0.3, 0.08), (0.0, 0.0, 0.025), M=M)
        h.box("lights", (0.36, 0.18, 0.02), (0.0, 0.0, 0.07), M=M, swatch="nav_white")
    M = skin.at((0.0, 25.0, -0.02), (0.0, -1.0, 0.0))
    h.box("dark", (1.3, 0.28, 0.06), (0.0, 0.0, 0.02), M=M)
    h.box("lights", (1.1, 0.14, 0.02), (0.0, 0.0, 0.055), M=M, swatch="window_dark")
    M = skin.at((0.0, 20.9, -6.0), (0.0, 0.0, 1.0))
    h.box("dark", (0.8, 0.8, 0.14), (0.0, 0.0, 0.05), M=M, bevel=0.02)
    h.lathe("bare", [(0.0, 0.0), (0.34, 0.0), (0.32, 0.1), (0.24, 0.24), (0.12, 0.32), (0.0, 0.34)],
            M @ T(0.0, 0.0, 0.1) @ Rot(90, "X"), seg=12, smooth=40.0)
    h.box("lights", (0.16, 0.05, 0.1), (0.0, 0.29, 0.3), M=M, swatch="window_dark")

    # Plaques de réparation (niveau 0 : quelques grandes tôles olive et à nu).
    for mat, origin, direction, size, spin in (
        ("accent", (-12.0, 14.4, 2.7), (1.0, 0.0, 0.0), (1.6, 0.9, 0.06), 4.0),
        ("bare", (12.0, 16.7, 0.4), (-1.0, 0.0, 0.0), (1.1, 0.7, 0.05), -6.0),
        ("accent", (7.0, 19.0, -0.05), (-1.0, 0.0, 0.0), (1.5, 0.5, 0.05), 2.0),
        ("accent", (-3.0, 16.4, 9.0), (0.0, 0.0, -1.0), (1.4, 1.1, 0.05), -5.0),
        ("bare", (2.8, 18.6, -6.0), (0.0, 0.0, 1.0), (1.3, 1.0, 0.05), 7.0),
    ):
        skin_box(h, skin, mat, origin, direction, size, sink=0.02, spin=spin)

    with h.level(1):
        # Antennes fouet sur le toit, sondes aux coins bas de la verrière, poignées de la joue.
        for x, y, height in ((-2.0, 17.9, 0.8), (-3.0, 17.9, 0.55)):
            p, n = skin.hit((x, y, 9.0), (0.0, 0.0, -1.0))
            h.rod("bare", tuple(p - n * 0.02), tuple(p + n * height), 0.018, seg=4)
            h.box("dark", (0.14, 0.14, 0.1), tuple(p + n * 0.03))
        for sx in (-1, 1):
            p, n = skin.hit((sx * 4.62, 25.0, 0.15), (0.0, -1.0, 0.0))
            h.rod("bare", tuple(p - Vector((0, 0.05, 0))), (p.x, min(p.y + 0.45, 22.08), p.z), 0.025, seg=6)
            h.box("dark", (0.12, 0.12, 0.12), tuple(p))
        for sx in (-1, 1):
            M = skin.at((sx * 12.0, 17.1, 2.6), (-sx, 0.0, 0.0))
            h.rod("bare", tuple(M @ Vector((-0.3, 0.0, 0.09))), tuple(M @ Vector((0.3, 0.0, 0.09))), 0.022, seg=4)
            for u in (-0.3, 0.3):
                h.box("dark", (0.06, 0.06, 0.1), (u, 0.0, 0.04), M=M)

    with h.level(2):
        # Tôles soudées en relief, cordons, boîtiers sur la proue.
        for mat, origin, direction, size, spin in (
            ("paint", (-12.0, 12.0, 0.6), (1.0, 0.0, 0.0), (1.3, 0.8, 0.08), -3.0),
            ("accent", (12.0, 13.0, 2.2), (-1.0, 0.0, 0.0), (1.0, 0.6, 0.08), 5.0),
            ("dark", (-7.0, 18.0, -0.1), (1.0, 0.0, 0.0), (0.6, 0.4, 0.1), 0.0),
            ("paint", (1.5, 15.5, 9.0), (0.0, 0.0, -1.0), (1.2, 0.9, 0.07), 8.0),
            ("accent", (-1.6, 19.6, -6.0), (0.0, 0.0, 1.0), (1.0, 0.8, 0.07), -4.0),
            ("dark", (12.0, 18.0, 2.9), (-1.0, 0.0, -0.3), (0.5, 0.35, 0.12), 0.0),
            ("bare", (-3.4, 12.6, 9.0), (0.0, 0.0, -1.0), (0.7, 0.5, 0.12), 0.0),
        ):
            skin_box(h, skin, mat, origin, direction, size, sink=0.02, spin=spin)
        for origin, direction, length in (((-12.0, 14.4, 2.25), (1.0, 0.0, 0.0), 1.5), ((-3.0, 15.85, 9.0), (0.0, 0.0, -1.0), 1.2)):
            skin_box(h, skin, "bare", origin, direction, (length, 0.07, 0.1), sink=0.03)


def build(h: Hull, cutters, cols) -> tuple[list, Skin]:
    """Assemble la coque ; renvoie les volumes pleins (corps, cou, module du cockpit, avant
    l'évidement) et la peau."""
    body = h.loft("paint", BODY, cut=True)
    fore = h.add(bm_hull(mirrored(fore_points())), "paint", cut=True)
    shells = [body, fore]
    skin = Skin(shells)
    enclosed = check_enclosed(shells)
    if enclosed:
        raise RuntimeError("cellules hors de la coque pleine (la peau serait percée) :\n  " + "\n  ".join(enclosed[:20]))
    windows(h, cutters, skin)
    nose_details(h, skin)
    boarding_module(h, cutters, cols)
    cargo_door(h)
    engines(h)
    dorsal(h)
    skids(h)
    side_details(h, skin)
    extra_details(h)
    repair_patches(h)
    return shells, skin


def core_cutters(cutters) -> None:
    """Évidement : les cellules agrandies du jeu (0,2 m)."""
    for name, (lo, hi) in CELL_BOXES.items():
        lo = Vector(lo) - Vector((CLEARANCE,) * 3)
        hi = Vector(hi) + Vector((CLEARANCE,) * 3)
        c.box(f"core_{name}", tuple(hi - lo), tuple((lo + hi) / 2), None, cutters)


def hollow(objs, cutters) -> None:
    """Retire des pièces `objs` l'évidement et les coupes (booléen exact, collection)."""
    for obj in objs:
        mod = obj.modifiers.new("sh_hollow", "BOOLEAN")
        mod.operation = "DIFFERENCE"
        mod.operand_type = "COLLECTION"
        mod.collection = cutters
        mod.solver = "EXACT"
    dg = bpy.context.evaluated_depsgraph_get()
    for obj in objs:
        mesh = bpy.data.meshes.new_from_object(obj.evaluated_get(dg))
        old = obj.data
        obj.modifiers.clear()
        obj.data = mesh
        bpy.data.meshes.remove(old)
        mesh.name = obj.name


def collider_hull(name: str, points, coll) -> bpy.types.Object:
    """Collider convexe (le jeu prend l'enveloppe convexe du maillage)."""
    bm = bm_hull(points)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    obj = bpy.data.objects.new(name, me)
    coll.objects.link(obj)
    obj.display_type = "WIRE"
    obj.hide_render = True
    return obj


def envelope_colliders(coll, cols) -> None:
    """Pavés de l'enveloppe (couche de coque entre l'enveloppe et les cellules + 0,2 m), tunnel
    libre, et morceaux convexes de la proue (étrave, joues du cou et du module) : aucun n'entre
    dans une cellule, le joueur à bord n'en touche aucun."""
    (x0, y0, z0), (x1, y1, z1) = ENVELOPE
    k = HATCH_W / 2 + 0.5
    neck_y = SHOULDER[0][1]  # épaulement : la proue commence à se resserrer
    boxes = {
        "port_aft": ((x0, y0, -1.3), (-8.0, HATCH_Y - k, 4.0)),
        "port_fwd": ((x0, HATCH_Y + k, -1.3), (-8.0, neck_y, 4.0)),
        "starboard": ((8.0, y0, -1.3), (x1, neck_y, 4.0)),
        "stern": ((-8.0, y0, -1.3), (8.0, -26.4, 4.6)),
        "bottom": ((x0, y0, z0), (x1, y1, -0.4)),
    }
    for part, (lo, hi) in boxes.items():
        cols.append(c.collider_box(f"{NAME}_{part}_col", tuple(b - a for a, b in zip(lo, hi)),
                                   tuple((a + b) / 2 for a, b in zip(lo, hi)), coll))
    big = 50.0
    fore = mirrored(fore_points())
    bow_y = 20.85  # vitrage frontal (20,6) + 0,25 m
    cols.append(collider_hull(f"{NAME}_bow_col", hull_points_between(fore, Vector((-big, bow_y, -big)), Vector((big, big, big))), coll))
    # Joues : une tranche de 0,22 m sous la peau des flancs en coin, de l'épaulement aux vitrages.
    sh, ck = Vector(SHOULDER[2]), Vector(CHEEK[0])
    for sx, side in ((-1, "port"), (1, "starboard")):
        a2, b2 = Vector((sx * sh.x, sh.y, 0.0)), Vector((sx * ck.x, ck.y, 0.0))
        out = Vector((b2.y - a2.y, -(b2.x - a2.x), 0.0)).normalized()
        if out.x * sx < 0:
            out = -out
        planes = [((0.0, sh.y, 0.0), (0.0, -1.0, 0.0)), ((0.0, bow_y, 0.0), (0.0, 1.0, 0.0)), (a2 - out * 0.22, -out)]
        cols.append(collider_hull(f"{NAME}_cheek_{side}_col", hull_points_clipped(fore, planes), coll))


# --- Version lointaine ---------------------------------------------------------------------------


def build_lod(h: Hull) -> None:
    """≤ 3 000 triangles : volumes (dont la proue à facettes), nacelles, tuyères allumées, vitrages
    (éventails posés sur la peau), feux ; aucun évidement."""
    body = h.loft("paint", BODY)
    fore = h.add(bm_hull(mirrored(fore_points())), "paint")
    skin = Skin([body, fore])
    h.loft("paint", SPINE)
    h.aabox("accent", (-9.22, -6.0, -0.8), (-8.5, 0.0, 3.3))
    h.aabox("dark", (-9.28, HATCH_Y - 0.94, CHANNEL_Z), (-9.18, HATCH_Y + 0.94, HATCH_H + 0.24))
    h.aabox("accent", (8.94, -11.6, -0.1), (9.14, -3.6, 3.1))
    for sx in (-1, 1):
        M = T(sx * POD_X, 0.0, POD_Z)
        h.lathe("paint", POD, M, seg=12)
        h.lathe("glow", [(0, -0.05), (1.05, -0.05), (1.05, 0.0), (0, 0.0)], M @ T(0, STERN - 0.4, 0), seg=12,
                swatch="engine_blue")
        h.lathe("heat", [(1.12, 0.0), (1.4, -1.1), (1.28, -1.1), (1.0, 0.0)], M @ T(0, STERN, 0), seg=12, closed=True)
        h.aabox("bare", (sx * 3.9 - 0.55, -24.4, 4.38), (sx * 3.9 + 0.55, -16.6, 5.0))
        h.aabox("dark", (sx * SKID_X - 0.3, -20.0, -1.7), (sx * SKID_X + 0.3, 13.0, -1.3))
    h.aabox("heat", (-5.2, STERN - 0.09, -0.6), (5.2, STERN + 0.05, 3.8))
    h.lathe("glow", [(0, -0.05), (0.75, -0.05), (0.75, 0.0), (0, 0.0)], T(0, STERN - 0.4, 1.7), seg=12, swatch="engine_blue")
    # Visière : une plaque sombre par moitié, de la visière à la pointe.
    tip = Vector((0.0, VISOR_TIP[0], VISOR_TIP[1]))
    t1 = ws_corners()[1]
    for sx in (-1, 1):
        b0 = skin.hit((0.0, 19.8, 9.0), (0.0, 0.0, -1.0))[0]
        b1 = skin.hit((sx * (t1.x - 0.2), 19.8, 9.0), (0.0, 0.0, -1.0))[0]
        f1 = Vector((sx * (t1.x - 0.2), t1.y + 0.14, t1.z + 0.03))
        slab(h, "dark", [b0, b1, f1, tip], Vector((0.0, 0.3, 1.0)), 0.09, 0.03)
    # Vitrages : éventail posé sur la peau, au contour du cadre (lueur froide du cockpit, ≈ 1,5).
    for w in window_loops(skin):
        bm = bmesh.new()
        ring = [bm.verts.new(p + n * 0.03) for p, n in zip(w["b"], w["nb"])]
        mid = bm.verts.new(sum((v.co for v in ring), Vector()) / len(ring) + w["d"] * 0.02)
        for i in range(len(ring)):
            f = bm.faces.new((ring[i], ring[(i + 1) % len(ring)], mid))
            f.normal_update()
            if f.normal.dot(w["d"]) < 0:
                f.normal_flip()
        h.add(bm, "glow", swatch="window_cool")
    for sx, sw in ((-1, "nav_red"), (1, "nav_green")):
        p, n = skin.hit((sx * 12.0, 11.2, 2.0), (-sx, 0, 0))
        h.box("lights", (0.2, 0.2, 0.2), tuple(p + n * 0.05), swatch=sw)
    h.aabox("lights", (-0.1, -26.05, 4.62), (0.1, -25.85, 4.82), swatch="nav_white")


# --- Contrôles -----------------------------------------------------------------------------------


def _sample_points(obj):
    M = obj.matrix_world
    me = obj.data
    pts = [M @ v.co for v in me.vertices]
    me.calc_loop_triangles()
    pts += [M @ ((me.vertices[t.vertices[0]].co + me.vertices[t.vertices[1]].co + me.vertices[t.vertices[2]].co) / 3)
            for t in me.loop_triangles]
    for e in me.edges:
        a, b = me.vertices[e.vertices[0]].co, me.vertices[e.vertices[1]].co
        pts.append(M @ ((a + b) / 2))
    return pts


def _box_distance(p: Vector, lo, hi) -> float:
    """Distance signée à un pavé (négative dedans)."""
    d = [max(lo[i] - p[i], 0.0, p[i] - hi[i]) for i in range(3)]
    out = math.sqrt(sum(v * v for v in d))
    if out > 0.0:
        return out
    return -min(min(p[i] - lo[i], hi[i] - p[i]) for i in range(3))


def check_clearance(parts, near_wall: set[str]) -> list[str]:
    """Aucune pièce à moins de CLEARANCE des cellules (sommets, centres de faces et d'arêtes) ; les
    embrasures (`near_wall`) peuvent approcher mais jamais entrer."""
    bad = []
    boxes = list(CELL_BOXES.values())
    for obj in parts:
        limit = 0.004 if obj.name in near_wall else CLEARANCE - 1e-3
        worst = min((_box_distance(p, lo, hi), tuple(round(v, 3) for v in p)) for p in _sample_points(obj) for lo, hi in boxes)
        if worst[0] < limit:
            bad.append(f"{obj.name} : {worst[0]:.3f} m en {worst[1]}")
    return bad


def check_envelope(objs) -> tuple[list[float], list[float], list[str]]:
    lo = [1e9] * 3
    hi = [-1e9] * 3
    bad = []
    (ex0, ey0, ez0), (ex1, ey1, ez1) = ENVELOPE
    for obj in objs:
        for v in obj.data.vertices:
            p = obj.matrix_world @ v.co
            for i in range(3):
                lo[i] = min(lo[i], p[i])
                hi[i] = max(hi[i], p[i])
            if not (ex0 - 1e-4 <= p.x <= ex1 + 1e-4 and ey0 - 1e-4 <= p.y <= ey1 + 1e-4 and ez0 - 1e-4 <= p.z <= ez1 + 1e-4):
                bad.append(f"{obj.name} : {tuple(round(a, 3) for a in p)}")
                break
    return lo, hi, bad


def _bvh(objs) -> BVHTree:
    bm = bmesh.new()
    for obj in objs:
        tmp = obj.data.copy()
        tmp.transform(obj.matrix_world)
        bm.from_mesh(tmp)
        bpy.data.meshes.remove(tmp)
    tree = BVHTree.FromBMesh(bm)
    bm.free()
    return tree


def check_enclosed(shells, margin: float = 0.06, step: float = 0.3) -> list[str]:
    """Les cellules agrandies du jeu (`CLEARANCE`) + `margin` restent dans la coque pleine (union
    des volumes `shells`), sauf aux vitrages et au tunnel : sinon l'évidement percerait la peau
    (un trou que `check_clearance` ne voit pas)."""
    trees = [_bvh([o]) for o in shells]

    def inside(p: Vector) -> bool:
        for tree in trees:
            loc, nor, _, _ = tree.find_nearest(p)
            if loc is not None and (p - loc).dot(nor) < 0.0:
                return True
        return False

    def exempt(p: Vector) -> bool:
        # Vitrages : dans le rectangle de l'ouverture (élargi), du côté de la peau.
        for _, axis, sgn, w_in, _, q, _ in WINDOWS:
            u0, u1, v0, v1 = min(q[0], q[2]), max(q[1], q[3]), q[4], q[5]
            w, u = (p.y, p.x) if axis == "y" else (p.x, p.y)
            if sgn * (w - w_in) > -0.3 and u0 - 0.2 <= u <= u1 + 0.2 and v0 - 0.2 <= p.z <= v1 + 0.2:
                return True
        # Tunnel du sas.
        return p.x < HATCH_WALL and abs(p.y - HATCH_Y) <= HATCH_W / 2 + 0.3 and p.z <= HATCH_H + 0.3

    bad = []
    grow = CLEARANCE + margin
    for name, (lo, hi) in CELL_BOXES.items():
        lo = [v - grow for v in lo]
        hi = [v + grow for v in hi]
        n = [max(1, int(math.ceil((hi[i] - lo[i]) / step))) for i in range(3)]
        for i in range(n[0] + 1):
            for j in range(n[1] + 1):
                for k in range(n[2] + 1):
                    on_face = i in (0, n[0]) or j in (0, n[1]) or k in (0, n[2])
                    if not on_face:
                        continue
                    p = Vector((lo[0] + (hi[0] - lo[0]) * i / n[0], lo[1] + (hi[1] - lo[1]) * j / n[1],
                                lo[2] + (hi[2] - lo[2]) * k / n[2]))
                    if not exempt(p) and not inside(p):
                        bad.append(f"{name} : {tuple(round(v, 2) for v in p)}")
    return bad


def check_pilot_view(objs) -> list[str]:
    """Rayons de l'œil du pilote vers une grille de points du vitrage : aucun ne doit toucher la
    coque avant d'avoir franchi la peau (on accepte les rayons qui sortent du champ du jeu :
    75° vertical, ≈ 107° horizontal en 16:9, en tournant la tête de 30° au plus)."""
    tree = _bvh(objs)
    eye = Vector(PILOT_EYE)
    out = []
    for name, axis, sgn, w_in, (iu0, iu1, iv0, iv1), _, _ in WINDOWS:
        normal = Vector((0, sgn, 0)) if axis == "y" else Vector((sgn, 0, 0))
        total = blocked = in_field = 0
        where = []
        for i in range(21):
            for j in range(11):
                u = iu0 + 0.02 + (iu1 - iu0 - 0.04) * i / 20
                v = iv0 + 0.02 + (iv1 - iv0 - 0.04) * j / 10
                p = Vector((u, w_in - 0.005 * sgn, v)) if axis == "y" else Vector((w_in - 0.005 * sgn, u, v))
                d = (p - eye).normalized()
                total += 1
                hit = tree.ray_cast(eye + d * ((p - eye).length + 0.001), d, 6.0)
                if hit[0] is None:
                    continue
                blocked += 1
                # Angles par rapport à la normale du vitrage (le pilote tourné vers lui).
                flat = d - Vector((0, 0, d.z))
                yaw = math.degrees(flat.angle(normal)) if flat.length > 1e-6 else 0.0
                pitch = math.degrees(math.asin(max(-1.0, min(1.0, d.z))))
                if yaw <= 55.0 and abs(pitch) <= 37.5:
                    in_field += 1
                    where.append(f"({u:.2f}, {v:.2f}) → {tuple(round(q, 2) for q in hit[0])}")
        out.append(f"{name} : {blocked}/{total} rayons arrêtés par la coque dans les 6 m, dont {in_field} dans le champ "
                   f"du jeu (lacet ≤ 55°, tangage ≤ 37,5° autour de la normale du vitrage)"
                   + (" ; vitrage (u, v) → impact : " + ", ".join(where[:6]) if where else ""))
    return out


def tri_count(objs) -> int:
    return sum(len(p.vertices) - 2 for o in objs for p in o.data.polygons)


# --- Aperçus -------------------------------------------------------------------------------------


def import_interior() -> list[bpy.types.Object]:
    """Cellules de l'intérieur (copie figée de préférence), images rattachées à la bibliothèque."""
    src = next((d for d in REF_DIRS if d.exists() and any(d.glob("*.gltf"))), None)
    if src is None:
        print("aperçus : intérieur introuvable")
        return []
    coll = c.collection("PREVIEW_INTERIOR")
    before = set(bpy.data.objects)
    quiet = io.StringIO()
    for f in sorted(src.glob("*.gltf")):
        if f.stem.endswith("_lod"):
            continue
        with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
            bpy.ops.import_scene.gltf(filepath=str(f))
    objs = [o for o in bpy.data.objects if o not in before]
    for o in objs:
        for u in o.users_collection:
            u.objects.unlink(o)
        coll.objects.link(o)
        if o.name.endswith("_col") or "_col." in o.name or (o.type == "MESH" and "detail" in o.keys() and int(o["detail"]) > 2):
            o.hide_render = True
    for img in bpy.data.images:
        if img.filepath and not Path(bpy.path.abspath(img.filepath)).exists():
            alt = c.TEXTURES_DIR / Path(img.filepath).name
            if alt.exists():
                img.filepath = str(alt)
                img.reload()
    print(f"aperçus : intérieur importé de {src} ({len(objs)} objets)")
    return [o for o in objs if o.type == "MESH" and not o.hide_render]


def preview_props() -> list[bpy.types.Object]:
    """Quai, fosse, sabots et passerelle (aperçu « du quai » seulement, non exportés)."""
    coll = c.collection("PREVIEW_PROPS")
    floor = c.material("pv_floor", (0.32, 0.31, 0.29), roughness=0.8)
    pit = c.material("pv_pit", (0.18, 0.18, 0.18), roughness=0.8)
    yellow = c.material("pv_yellow", (0.8, 0.6, 0.08), roughness=0.5)
    grate = c.material("pv_grate", (0.22, 0.22, 0.22), metallic=0.6, roughness=0.5)
    out = [
        c.box("pv_quay", (20.0, 80.0, 0.2), (-20.5, -3.0, -0.1), floor, coll),
        c.box("pv_pit", (24.0, 54.0, 0.2), (0.0, -2.8, -3.1), pit, coll),
        c.box("pv_gangway", (3.5, 1.4, 0.26), (-9.55, HATCH_Y, -0.13), grate, coll),
        c.box("pv_gangway_out", (1.5, 1.6, 0.26), (-11.8, HATCH_Y, -0.13), grate, coll),
    ]
    for y in (-18.46, -8.02, 2.42, 12.86):
        out.append(c.box(f"pv_saddle{y}", (19.4, 0.8, 1.3), (0.0, y, -2.35), pit, coll))
    for s in (-1, 1):
        out.append(c.box(f"pv_rail{s}", (1.2, 0.05, 0.05), (-10.6, HATCH_Y + s * 0.8, 1.1), yellow, coll))
    return out


def sight_lines() -> list[bpy.types.Object]:
    """Lignes de visée du pilote (bas et haut du vitrage frontal), aperçu de coupe seulement."""
    coll = c.collection("PREVIEW_SIGHT")
    red = c.material("pv_sight", (1.0, 0.1, 0.05), emission=(1.0, 0.1, 0.05), emission_strength=8.0)
    eye = Vector(PILOT_EYE)
    out = []
    for z in (0.85, 2.7):
        p = Vector((PILOT_EYE[0], 20.6, z))
        d = (p - eye).normalized()
        end = eye + d * 4.5
        mid = (eye + end) / 2
        o = c.cylinder(f"pv_sight_{z}", 0.012, (end - eye).length, tuple(mid), red, coll, axis="Y", segments=6)
        o.rotation_mode = "QUATERNION"
        o.rotation_quaternion = Vector((0, 1, 0)).rotation_difference(d)
        out.append(o)
    return out


def _camera(eye, target, fov=None, lens=35.0, ortho=None, clip=0.05, roll=None):
    data = bpy.data.cameras.get("pv_cam") or bpy.data.cameras.new("pv_cam")
    cam = bpy.data.objects.get("pv_cam") or bpy.data.objects.new("pv_cam", data)
    if cam.name not in bpy.context.scene.collection.objects:
        bpy.context.scene.collection.objects.link(cam)
    data.clip_start, data.clip_end = clip, 500.0
    if ortho:
        data.type = "ORTHO"
        data.sensor_fit = "AUTO"  # sinon le cadrage dépend de la prise de vue précédente (champ vertical)
        data.ortho_scale = ortho
    else:
        data.type = "PERSP"
        if fov:
            data.sensor_fit = "VERTICAL"
            data.angle = math.radians(fov)
        else:
            data.sensor_fit = "AUTO"
            data.lens = lens
    cam.location = eye
    if roll is not None:
        cam.rotation_euler = roll
    else:
        cam.rotation_euler = (Vector(target) - Vector(eye)).to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = cam


def shots() -> dict:
    """Nom → (réglages de caméra, niveaux, groupes montrés : hull, lod, interior, props, sight)."""
    ex, ey, ez = PILOT_EYE
    top = (0.0, 0.0, math.pi / 2)
    return {
        "avant34": (dict(eye=(-40.0, 38.0, 22.0), target=(0.0, -1.0, 1.0), lens=40.0), (0, 2), ("hull", "interior")),
        "arriere34": (dict(eye=(38.0, -52.0, 18.0), target=(0.0, -6.0, 1.0), lens=40.0), (0, 2), ("hull", "interior")),
        "profil": (dict(eye=(-80.0, -2.8, 3.0), target=(0.0, -2.8, 1.6), lens=50.0), (0, 2), ("hull", "interior")),
        "dessus": (dict(eye=(0.0, -2.8, 60.0), target=None, ortho=56.0, roll=top), (0, 2), ("hull", "interior")),
        "dessous": (dict(eye=(-14.0, 30.0, -16.0), target=(0.0, -3.0, -1.0), lens=35.0), (2,), ("hull", "interior")),
        "pres_sas": (dict(eye=(-19.0, 4.0, 4.5), target=(-9.0, -3.0, 1.0), lens=35.0), (0, 2), ("hull", "interior")),
        "pres_nez": (dict(eye=(-9.0, 30.0, 4.5), target=(0.0, 20.0, 1.2), lens=32.0), (2,), ("hull", "interior")),
        "face": (dict(eye=(0.0, 45.0, 3.0), target=(0.0, 15.0, 1.6), lens=50.0), (2,), ("hull", "interior")),
        "quai_nez": (dict(eye=(-11.5, 26.0, 1.9), target=(-2.0, 18.0, 1.2), fov=70.0), (2,), ("hull", "interior", "props")),
        "verriere": (dict(eye=(-4.5, 27.0, 2.6), target=(0.0, 20.0, 1.8), lens=30.0), (2,), ("hull", "interior")),
        "nez_dessous": (dict(eye=(9.0, 31.0, -6.0), target=(0.0, 17.0, 0.5), lens=35.0), (2,), ("hull", "interior")),
        "quai": (dict(eye=(-16.0, -0.6, 1.7), target=(-8.0, -3.0, 1.1), fov=75.0), (0, 2), ("hull", "interior", "props")),
        "pilote": (dict(eye=(ex, ey, ez), target=(ex, ey + 5.0, ez - 0.25), fov=75.0, clip=0.02), (2,), ("hull", "interior")),
        "pilote_gauche": (dict(eye=(ex, ey, ez), target=(ex - 4.0, ey + 1.2, ez - 0.2), fov=75.0, clip=0.02), (2,),
                          ("hull", "interior")),
        "coupe_dessus": (dict(eye=(0.0, -2.8, 40.0), target=None, ortho=54.0, roll=top, clip=38.6), (2,), ("hull", "interior")),
        "coupe_tunnel": (dict(eye=(-6.0, 30.0, 1.3), target=(-6.0, 0.0, 1.3), ortho=11.0, clip=33.0), (2,), ("hull", "interior")),
        "coupe_nez": (dict(eye=(-30.0, 19.4, 1.6), target=(0.0, 19.4, 1.6), ortho=9.0, clip=29.0), (2,),
                      ("hull", "interior", "sight")),
        "coupe_travers": (dict(eye=(0.0, 30.0, 1.6), target=(0.0, 0.0, 1.6), ortho=21.0, clip=30.0 + 8.0), (2,), ("hull", "interior")),
        "lod": (dict(eye=(-40.0, 38.0, 22.0), target=(0.0, -1.0, 1.0), lens=40.0), (0,), ("lod",)),
    }


def render_previews(groups: dict, samples: int, res: str, only) -> None:
    import preview as pv

    w, hgt = (int(v) for v in res.split("x"))
    pv.setup_cycles(samples, w, hgt, exposure=0.0)
    pv.world((0.10, 0.105, 0.12), 1.0)
    sun_data = bpy.data.lights.new("pv_sun", "SUN")
    sun_data.energy = 3.2
    sun_data.angle = math.radians(1.5)
    sun = bpy.data.objects.new("pv_sun", sun_data)
    bpy.context.scene.collection.objects.link(sun)
    ws.look_at(sun, Vector((-0.45, 0.35, 0.82)), Vector((0, 0, 1)))
    pv.area_light("pv_fill", (40.0, -30.0, 12.0), (0.0, 0.0, 0.0), 30000.0, size=20.0, color=(0.8, 0.85, 1.0))
    all_meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    for name, (cam, levels, shown) in shots().items():
        if only and name not in only:
            continue
        _camera(cam["eye"], cam.get("target"), fov=cam.get("fov"), lens=cam.get("lens", 35.0), ortho=cam.get("ortho"),
                clip=cam.get("clip", 0.05), roll=cam.get("roll"))
        visible = {o for g in shown for o in groups.get(g, [])}
        for o in groups.get("hull", []):
            o.visible_shadow = not name.startswith("coupe")  # coupes : cellules éclairées par le soleil
        for lv in levels:
            for o in all_meshes:
                o.hide_render = o not in visible or c.detail_of(o) > lv
            suffix = f"_n{lv}" if len(levels) > 1 or name in ("avant34",) else ""
            path = PREVIEWS / f"{name}{suffix}.png"
            pv.render(path)
            print(f"aperçu : {path.name}")


# --- Programme -----------------------------------------------------------------------------------


def main(argv) -> None:
    samples = int(argv[argv.index("--samples") + 1]) if "--samples" in argv else 32
    res = argv[argv.index("--res") + 1] if "--res" in argv else "1280x720"
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    c.reset_scene()
    if "--textures" in argv or not (c.TEXTURES_DIR / f"{TEXTURE}_color.png").exists():
        write_hull_texture()
        print(f"texture {TEXTURE} cuite")
    mats = palette()
    parts_coll = c.collection("PARTS")
    cutters = c.collection("CUTTERS")
    export_coll = c.collection("EXPORT")
    lod_coll = c.collection("LOD_PARTS")
    cols: list[bpy.types.Object] = []
    h = Hull(mats, parts_coll, seed=7731)
    core_cutters(cutters)
    shells, _skin = build(h, cutters, cols)
    envelope_colliders(export_coll, cols)
    for o in cutters.objects:
        o.display_type = "WIRE"
        o.hide_render = True
    hollow(h.cut, cutters)
    # Livrée : biseaux hauts du corps, du cou et du module du cockpit en olive, ventre en métal sombre.
    for obj in shells:
        h.split(obj, lambda n: 0.3 < n.z < 0.95 and abs(n.x) > 0.3, "accent")
        h.split(obj, lambda n: n.z < -0.95, "dark")
        h.split(obj, lambda n: n.y > 0.55, "accent")  # façade de la proue : verrière, joues, étrave
    h.finish_cut()
    bad = ws.inverted_parts(h.parts)
    if bad:
        raise RuntimeError(f"normales inversées sur {len(bad)} pièces : {bad[:8]}")
    too_close = check_clearance(h.parts, h.near_wall)
    if too_close:
        raise RuntimeError("coque trop près des cellules :\n  " + "\n  ".join(too_close[:20]))
    lo, hi, outside = check_envelope(h.parts)
    if outside:
        raise RuntimeError("coque hors de l'enveloppe :\n  " + "\n  ".join(outside[:20]))
    for line in check_pilot_view(h.parts):
        print("vue du pilote : " + line)

    quiet = io.StringIO()
    with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
        merged = c.join_by_material(h.parts, NAME, export_coll)
    for o in h.parts:
        o.hide_set(True)
        o.hide_render = True
    lod = Hull(mats, lod_coll, seed=7731, prefix="lod")
    build_lod(lod)
    with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
        lod_merged = c.join_by_material(lod.parts, f"{NAME}_lod", export_coll)
        c.export_gltf(f"{OUT}/{NAME}.gltf", merged + cols)
        c.export_gltf(f"{OUT}/{NAME}_lod.gltf", lod_merged)
    for o in lod.parts:
        o.hide_set(True)
        o.hide_render = True
    c.save_blend(BLEND)

    by_level: dict[int, int] = {}
    for o in merged:
        by_level[c.detail_of(o)] = by_level.get(c.detail_of(o), 0) + tri_count([o])
    mats_used = sorted({o.data.materials[0].name for o in merged})
    print(f"{NAME} : {len(h.parts)} pièces → {len(merged)} maillages dessinés, {len(mats_used)} matériaux {mats_used}")
    print("triangles par niveau : " + ", ".join(f"n{k} {v}" for k, v in sorted(by_level.items())) +
          f" ; cumul n0+n1 {by_level.get(0, 0) + by_level.get(1, 0)}, n0+n1+n2 {sum(by_level.values())}")
    print(f"LOD : {len(lod_merged)} maillages, {tri_count(lod_merged)} triangles")
    print(f"enveloppe (Blender) : min {tuple(round(v, 3) for v in lo)}, max {tuple(round(v, 3) for v in hi)}")
    print(f"enveloppe (jeu) : min {c.to_bevy((lo[0], hi[1], lo[2]))}, max {c.to_bevy((hi[0], lo[1], hi[2]))}")
    print("colliders : " + ", ".join(o.name for o in cols))
    print("maillages : " + ", ".join(f"{o.name}{'' if not c.detail_of(o) else ' (detail ' + str(c.detail_of(o)) + ')'}" for o in merged))

    if "--previews" in argv:
        PREVIEWS.mkdir(parents=True, exist_ok=True)
        groups = {"hull": merged, "lod": lod_merged, "interior": import_interior(), "props": preview_props(),
                  "sight": sight_lines()}
        render_previews(groups, samples, res, only)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
