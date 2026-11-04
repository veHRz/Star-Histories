"""Bibliothèque de matériaux « futur usé » (lot 0 de M8, docs/refonte-visuelle.md §2-§3).

Chaque matériau est un petit graphe procédural (hauteur, masques d'usure, couleurs) écrit en
numpy (`texgen`, `surfaces`, `devices`, `trim`) : périodique par construction (raccords
parfaits), déterministe (graines fixes), cuit en PNG dans `assets/textures/materials/` selon le
contrat relu par le jeu (`crates/sh_game/src/materials.rs`) :

- `<mat>_color.png`    couleur de base, sRGB ;
- `<mat>_normal.png`   normale en espace tangent, convention OpenGL / glTF, linéaire ;
- `<mat>_orm.png`      R = occlusion, G = rugosité, B = métal, linéaire ;
- `<mat>_emissive.png` émission, sRGB (seulement les matériaux qui en ont besoin).

Pourquoi numpy plutôt que des nœuds Blender cuits par Cycles : bruit spectral, flous et
tampons sont toriques, donc les tuiles raccordent sans retouche ; le calcul est rapide (≈ 2 min
pour toute la bibliothèque) et reproductible à l'octet près, sans échantillonnage.

Lancer (régénère tout, ou seulement les matériaux nommés ; `--preview` écrit aussi des
aperçus carrelés 3 × 3 dans target/art-previews/lot0/tiles/) :

    blender --background --factory-startup --python tools/blender/materials/library.py -- [noms…] [--preview]

Dans les scripts d'assets : `common.textured_material("sh_panel", "panel_painted", tint=…)`,
puis `common.uv_box(obj)` (densité lue dans `MATERIALS`) ou `common.uv_trim(obj, "hazard")`
pour une bande de la trim sheet (`TRIMS`).

Éclairage du jeu : dans une cellule à lightmap, seul le diffus est éclairé (lightmap ×
couleur × (1 − métal)) et la texture d'occlusion n'y joue pas. D'où un métal plafonné
(`texgen.METAL_MAX`) et une part de l'occlusion recopiée dans la couleur (`texgen.ALBEDO_AO`).
Les couleurs « peintes » sont neutres et claires : on les teinte par le facteur de couleur
glTF (`tint`), voir `TINTS`.
"""

from __future__ import annotations

import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

import devices  # noqa: E402
import surfaces  # noqa: E402
import texgen as tg  # noqa: E402
import trim  # noqa: E402

ROOT = Path(__file__).resolve().parents[3]
OUT_DIR = ROOT / "assets" / "textures" / "materials"
PREVIEW_DIR = ROOT / "target" / "art-previews" / "lot0" / "tiles"

# --- Catalogue ---------------------------------------------------------------------------------
# tile_m : taille réelle d'une tuile (mètres), lue par `common.uv_box` ; sizes : résolution de
# chaque carte écrite (ORM peu détaillés en 512 : indiscernable, quatre fois moins de pixels).
# tintable : couleur neutre claire, prévue pour un facteur de couleur (TINTS).

MATERIALS: dict[str, dict] = {
    "panel_painted": {"tile_m": 2.0, "tintable": True, "sizes": {"color": 1024, "normal": 1024, "orm": 1024},
                      "doc": "Tôle peinte claire : panneaux 1 × 0,5 m, rivets, vis, trappe, plaque, grille, écaillures sur les arêtes, coulures"},
    "metal_dark": {"tile_m": 2.0, "tintable": False, "sizes": {"color": 1024, "normal": 1024, "orm": 1024},
                   "doc": "Anthracite usé (encadrements, nervures, poutres) : frottements, rayures, rares éclats, huile"},
    "metal_bare": {"tile_m": 2.0, "tintable": False, "sizes": {"color": 1024, "normal": 1024, "orm": 1024},
                   "doc": "Métal brut brossé le long de U, rayé, taché de graisse (machines, rails, tuyaux)"},
    "floor_plate": {"tile_m": 2.0, "tintable": False, "sizes": {"color": 1024, "normal": 1024, "orm": 1024},
                    "doc": "Tôle larmée antidérapante en plaques de 1 m boulonnées, sommets polis"},
    "grating": {"tile_m": 2.0, "tintable": False, "sizes": {"color": 1024, "normal": 1024, "orm": 1024, "emissive": 512},
                "doc": "Caillebotis opaque, barres le long de V ; émissif facultatif = lumière par les trous"},
    "rubber": {"tile_m": 1.0, "tintable": False, "sizes": {"color": 1024, "normal": 1024, "orm": 512},
               "doc": "Caoutchouc noir mat, poussiéreux, frotté (gaines, joints, tapis)"},
    "fabric": {"tile_m": 1.0, "tintable": True, "sizes": {"color": 1024, "normal": 1024, "orm": 512},
               "doc": "Similicuir matelassé (bourrelets horizontaux de 12,5 cm, coutures), sièges et couchettes"},
    "container_ribbed": {"tile_m": 2.0, "tintable": True, "sizes": {"color": 1024, "normal": 1024, "orm": 512},
                         "doc": "Tôle nervurée de caisses et conteneurs, nervures de 12,5 cm le long de V"},
    "ferrocrete": {"tile_m": 4.0, "tintable": False, "sizes": {"color": 1024, "normal": 1024, "orm": 512},
                   "doc": "Béton taché du spatioport, joints tous les 2 m, fissures, traces de roues"},
    "floor_tiles": {"tile_m": 2.0, "tintable": True, "sizes": {"color": 1024, "normal": 1024, "orm": 512},
                    "doc": "Dalles de 50 cm usées (hall, bureau)"},
    "hull_plates": {"tile_m": 4.0, "tintable": True, "sizes": {"color": 1024, "normal": 1024, "orm": 512},
                    "doc": "Blindage de coque extérieure : plaques décalées, rivets, trappes, suie le long de +U"},
    "screen": {"tile_m": 1.0, "tintable": False, "sizes": {"color": 1024, "normal": 512, "orm": 512, "emissive": 1024},
               "doc": "Écrans segmentés 4 × 4 (chiffres, barres, oscilloscope, radar, voyants) ; émissif"},
    "buttons": {"tile_m": 0.5, "tintable": False, "sizes": {"color": 1024, "normal": 1024, "orm": 512, "emissive": 1024},
                "doc": "Panneau de boutons rétroéclairés, interrupteurs à garde, molettes, voyants ; émissif"},
    "light_band": {"tile_m": 1.0, "tintable": False, "sizes": {"color": 512, "normal": 512, "orm": 512, "emissive": 512},
                   "doc": "Diffuseur de bandeau lumineux (U le long de la bande, V en travers : uv_trim(obj, (0, 1))) ; émissif"},
    "trim_ship": {"tile_m": 2.0, "tintable": False, "sizes": {"color": 1024, "normal": 1024, "orm": 1024, "emissive": 512},
                  "doc": "Trim sheet : bandes horizontales (TRIMS), à poser avec common.uv_trim"},
}

# Bandes de la trim sheet `trim_ship` : (v0, v1) en UV de Blender (v = 0 en bas de l'image).
# Toutes répètent le long de U avec une période de 2 m (1024 px à 512 px/m).
_TRIM_PX = [
    # (nom, hauteur en pixels), de bas en haut ; somme = 1024
    ("warning_red", 64),   # bandes d'avertissement rouge / blanc usées (12,5 cm)
    ("stencils", 128),     # plaque grise à pochoirs génériques : 07, A2, 12, B4, 03, flèche, chevrons, « ! »
    ("light_strip", 64),   # bandeau lumineux segmenté tous les 25 cm (seule bande émissive)
    ("gasket", 64),        # joint caoutchouc à trois lèvres (portes, trappes, hublots)
    ("flange", 96),        # tuyau brut, bride boulonnée tous les 1 m, collier jaune (U = axe du tuyau)
    ("cables", 128),       # faisceau de sept câbles gainés, colliers tous les 50 cm (U = axe)
    ("vent", 128),         # grille d'aération à fentes, crasse sous les fentes
    ("chevrons", 96),      # chevrons de direction clairs sur fond sombre (pointent vers +U)
    ("hazard", 128),       # bandes de danger jaune / noir à 45°, usées
    ("edge", 64),          # chanfrein de bord de panneau anthracite, arête écaillée (chanfrein en bas)
    ("rivets", 64),        # plat rivé anthracite, rivets tous les 12,5 cm ; sert de plinthe (bas usé)
]
TRIMS: dict[str, tuple[float, float]] = {}
_acc = 0
for _name, _px in _TRIM_PX:
    TRIMS[_name] = (_acc / 1024.0, (_acc + _px) / 1024.0)
    _acc += _px
assert _acc == 1024

# Teintes d'identité proposées (P6) pour les matériaux teintables (facteur de couleur glTF).
TINTS: dict[str, tuple[float, float, float]] = {
    "neutral": (1.0, 1.0, 1.0),
    "hexa_yellow": (1.0, 0.74, 0.16),
    "hexa_black": (0.22, 0.22, 0.22),
    "ugl_bluegray": (0.70, 0.79, 0.90),
    "pirate_rust": (0.78, 0.44, 0.30),
    "pirate_red": (0.70, 0.16, 0.12),
    "empire_anthracite": (0.34, 0.35, 0.38),
    "olive": (0.62, 0.66, 0.46),
    "warm_gray": (0.92, 0.88, 0.80),
}

GENERATORS = {
    "panel_painted": surfaces.panel_painted,
    "metal_dark": surfaces.metal_dark,
    "metal_bare": surfaces.metal_bare,
    "floor_plate": surfaces.floor_plate,
    "grating": surfaces.grating,
    "rubber": surfaces.rubber,
    "fabric": surfaces.fabric,
    "container_ribbed": surfaces.container_ribbed,
    "ferrocrete": surfaces.ferrocrete,
    "floor_tiles": surfaces.floor_tiles,
    "hull_plates": surfaces.hull_plates,
    "screen": devices.screen,
    "buttons": devices.buttons,
    "light_band": devices.light_band,
    "trim_ship": lambda res, tile_m: trim.trim_ship(res, tile_m, TRIMS),
}


def texture_path(name: str, role: str) -> Path:
    """Chemin d'une carte de la bibliothèque (`role` : color, normal, orm, emissive)."""
    return OUT_DIR / f"{name}_{role}.png"


def _resize(img: np.ndarray, size: int) -> np.ndarray:
    res = img.shape[0]
    if size == res:
        return img
    if size < res:
        return tg.downsample(img, res // size)
    raise ValueError(f"agrandissement non prévu ({res} → {size})")


def seam_error(img: np.ndarray, vertical: bool = True) -> float:
    """Raccord : saut moyen entre la dernière et la première colonne (ligne), rapporté au plus
    fort saut moyen entre deux colonnes (lignes) voisines de l'intérieur. ≤ 1 : le bord ne se
    distingue pas d'un joint intérieur ; nettement plus : motif coupé au bord. `vertical` :
    vérifier aussi le haut et le bas (pas pour une trim sheet, dont les bandes s'arrêtent)."""
    def ratio(a: np.ndarray) -> float:
        inner = np.abs(np.diff(a, axis=1)).mean(axis=tuple(i for i in range(a.ndim) if i != 1))
        edge = np.abs(a[:, 0] - a[:, -1]).mean()
        return float(edge / max(inner.max(), 1e-9))

    r = ratio(img)
    if vertical:
        r = max(r, ratio(np.swapaxes(img, 0, 1)))
    return r


def generate(name: str, preview: bool = False) -> dict:
    """Calcule et écrit les cartes d'un matériau ; renvoie {rôle: (chemin, octets)}."""
    spec = MATERIALS[name]
    work = max(spec["sizes"].values())
    start = time.monotonic()
    maps = GENERATORS[name](work, spec["tile_m"])
    written = {}
    seams = []
    for role, size in spec["sizes"].items():
        img = maps[role]
        if role == "normal" and size < img.shape[0]:
            v = _resize(img * 2 - 1, size)
            img = v / np.maximum(np.linalg.norm(v, axis=-1, keepdims=True), 1e-6) * 0.5 + 0.5
        else:
            img = _resize(img, size)
        seams.append(seam_error(img, vertical=(name != "trim_ship")))
        path = texture_path(name, role)
        written[role] = (path, tg.write_png(path, img))
    if preview:
        factor = 2 if work >= 1024 else 1
        cv = tg.Canvas(work, 1.0)
        for role in maps:
            tg.write_png(PREVIEW_DIR / f"{name}_{role}_3x3.png", cv.tile3(maps[role], factor))
    sizes = ", ".join(f"{r} {spec['sizes'][r]}" for r in written)
    print(f"matériau {name} : {sizes} px ; raccord {max(seams):.2f} ({time.monotonic() - start:.1f} s)")
    return written


def main(argv: list[str]) -> None:
    preview = "--preview" in argv
    names = [a for a in argv if not a.startswith("--")] or list(GENERATORS)
    total = 0
    for name in names:
        written = generate(name, preview)
        total += sum(b for _, b in written.values())
    print(f"bibliothèque : {len(names)} matériaux, {total / 1e6:.1f} Mo écrits dans {OUT_DIR}")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
