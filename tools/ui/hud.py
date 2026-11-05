"""Schéma du vaisseau du HUD (M9, lot L2) : silhouette vue de dessus et plaques de blindage.

Le schéma (`crates/sh_game/src/ui/combat_hud.rs`) superpose, dans une zone de `AREA_W` × `AREA_H`
unités logiques :
- la silhouette du vaisseau, nez en haut, découpée en quatre régions (proue, poupe, bâbord,
  tribord) selon la règle des impacts (`sh_sim::combat::section_at` : diagonales de la boîte de
  la coque) ; le jeu teinte chaque région selon la **structure** de sa section ;
- quatre arcs de plaques autour d'elle, un par section, teintés selon le **blindage** : le jeu
  les raccourcit vers leur milieu quand le blindage s'use (`ImageNode::rect`) ;
- le cœur (réacteur, quille) au centre, un losange teinté selon son état.

Pas de bouclier, jamais (AGENTS.md §13, Q16) : seul le blindage protège.

Les images sont **blanches** (le jeu les teinte : `ImageNode::color`) et dessinées au double de
leur taille logique, comme les cadres (`frames.py`). Silhouette générique, sans emprunt : elle ne
représente aucun vaisseau connu. Les rectangles de placement sont imprimés à la fin : ce sont les
constantes `SCHEMATIC_*` de `combat_hud.rs` (mêmes valeurs des deux côtés).

Bibliothèque standard seulement :
    uv run --python 3.11 tools/ui/hud.py
"""

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from frames import SCALE, png  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "assets" / "ui" / "hud"

# Suréchantillonnage par axe, par pixel d'image : R sous-échantillons par unité logique.
SS = 4
R = SCALE * SS

# Zone du schéma (unités logiques) et son centre.
AREA_W, AREA_H = 184, 168
CX, CY = AREA_W / 2, AREA_H / 2

# Anneau des plaques : ellipse centrée, demi-axes, demi-épaisseur de la bande.
RING_RX, RING_RY = 56.0, 72.0
BAND = 3.6
# Ouverture angulaire (degrés, de part et d'autre de l'axe) et nombre de plaques par arc.
BOW_SPAN, SIDE_SPAN = 37.0, 31.0
PLATES = 5
GAP = 3.0


# --- Formes de la silhouette (unités logiques de la zone, côté tribord ; symétrie en x). -----


def mirror(points):
    return [(2 * CX - x, y) for x, y in points]


# Coque : demi-profil du nez à la poupe (bloc moteur carré), puis son reflet.
HULL_RIGHT = [
    (92, 24),
    (96.5, 32),
    (100.5, 45),
    (103.5, 58),
    (105, 74),
    (105, 118),
    (103, 128),
    (103, 140),
]
HULL = HULL_RIGHT + list(reversed(mirror(HULL_RIGHT)))
# Pylônes en flèche : ils tiennent les nacelles à distance de la coque (le vide entre les deux
# se voit : la silhouette se lit comme un vaisseau, pas comme un bloc).
WING_R = [(104.5, 88), (111, 97), (111, 111), (104.5, 116)]
WING_L = list(reversed(mirror(WING_R)))
# Nacelles des moteurs : rectangles arrondis à l'avant, plus courtes que la coque.
NACELLE_R = (110.0, 119.0, 88.0, 134.0)  # x0, x1, y0, y1
NACELLE_L = (2 * CX - NACELLE_R[1], 2 * CX - NACELLE_R[0], NACELLE_R[2], NACELLE_R[3])
# Verrière du cockpit : un trapèze sombre près du nez.
WINDOW = [(88.5, 44), (95.5, 44), (97, 52), (87, 52)]
# Tuyères (sombres) : au bout de la coque et des nacelles.
NOZZLES = [(86.0, 98.0, 136.5, 140.0), (111.5, 117.5, 130.5, 134.0), (66.5, 72.5, 130.5, 134.0)]
# Joints de panneaux (lignes horizontales de la coque, épine centrale).
SEAMS_Y = [64.0, 104.0, 124.0]


def in_nozzle(x, y):
    return any(x0 <= x <= x1 and y0 <= y <= y1 for x0, x1, y0, y1 in NOZZLES)


def in_poly(x, y, poly):
    inside = False
    n = len(poly)
    j = n - 1
    for i in range(n):
        xi, yi = poly[i]
        xj, yj = poly[j]
        if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi:
            inside = not inside
        j = i
    return inside


def in_nacelle(x, y, n):
    x0, x1, y0, y1 = n
    if not (x0 <= x <= x1 and y0 <= y <= y1):
        return False
    r = (x1 - x0) / 2
    if y < y0 + r:
        cx = (x0 + x1) / 2
        return (x - cx) ** 2 + (y - (y0 + r)) ** 2 <= r * r
    return True


def in_ship(x, y):
    return (
        in_poly(x, y, HULL)
        or in_poly(x, y, WING_R)
        or in_poly(x, y, WING_L)
        or in_nacelle(x, y, NACELLE_R)
        or in_nacelle(x, y, NACELLE_L)
    )


# Boîte de la coque pour la règle des sections (rayon et demi-longueur, comme `section_at`).
SHIP_X0, SHIP_X1 = 62, 122
SHIP_Y0, SHIP_Y1 = 22, 146
HALF_W = (SHIP_X1 - SHIP_X0) / 2
HALF_L = (SHIP_Y1 - SHIP_Y0) / 2
SHIP_CY = (SHIP_Y0 + SHIP_Y1) / 2


def section(x, y):
    nz = (y - SHIP_CY) / HALF_L  # avant (proue) : y petit
    nx = (x - CX) / HALF_W
    if abs(nz) >= abs(nx):
        return "bow" if nz < 0 else "stern"
    return "starboard" if nx > 0 else "port"


def ship_images():
    """Les quatre régions de la silhouette, même rectangle : remplissage léger (hologramme),
    contour plein, joints et verrière ; chaque image ne garde que sa région."""
    w, h = SHIP_X1 - SHIP_X0, SHIP_Y1 - SHIP_Y0
    gw, gh = w * R, h * R

    def at(i, j):
        return SHIP_X0 + (i + 0.5) / R, SHIP_Y0 + (j + 0.5) / R

    mask = [[in_ship(*at(i, j)) for i in range(gw)] for j in range(gh)]
    # Contour : un sous-échantillon dedans dont un voisin à 1,1 unité est dehors.
    d = round(1.1 * R)
    dd = round(1.1 * R / math.sqrt(2))
    offsets = [(d, 0), (-d, 0), (0, d), (0, -d), (dd, dd), (dd, -dd), (-dd, dd), (-dd, -dd)]

    def outside(i, j):
        return not (0 <= i < gw and 0 <= j < gh) or not mask[j][i]

    images = {}
    for name in ("bow", "stern", "port", "starboard"):
        rows = []
        for py in range(h * SCALE):
            row = bytearray([0])
            for px in range(w * SCALE):
                acc_a = 0.0
                acc_c = 0.0
                for sy in range(SS):
                    for sx in range(SS):
                        i, j = px * SS + sx, py * SS + sy
                        if not mask[j][i]:
                            continue
                        x, y = at(i, j)
                        if section(x, y) != name:
                            continue
                        edge = any(outside(i + oi, j + oj) for oi, oj in offsets)
                        if edge:
                            a, c = 1.0, 1.0
                        elif in_poly(x, y, WINDOW) or in_nozzle(x, y):
                            a, c = 0.85, 0.55
                        elif any(abs(y - s) < 0.45 for s in SEAMS_Y) or (
                            abs(x - CX) < 0.4 and 50 < y < 132
                        ):
                            a, c = 0.70, 0.80
                        else:
                            # Remplissage : un léger dégradé vers l'arrière, la tôle s'assombrit.
                            a = 0.36 - 0.10 * (y - SHIP_Y0) / h
                            c = 1.0
                        acc_a += a
                        acc_c += a * c
                n = SS * SS
                alpha = acc_a / n
                if alpha <= 1e-6:
                    row += bytes(4)
                    continue
                v = round(min(1.0, acc_c / acc_a) * 255)
                row += bytes((v, v, v, round(alpha * 255)))
            rows.append(bytes(row))
        images[f"hull_{name}.png"] = (w, h, png(w * SCALE, h * SCALE, b"".join(rows)))
    return images


# --- Arcs de plaques. -----------------------------------------------------------------------


def ring_angle(x, y):
    """Angle paramétrique sur l'ellipse (degrés, 0 = tribord, 90 = proue)."""
    return math.degrees(math.atan2((CY - y) / RING_RY, (x - CX) / RING_RX)) % 360.0


def in_band(x, y):
    dx, dy = x - CX, y - CY
    outer = (dx / (RING_RX + BAND)) ** 2 + (dy / (RING_RY + BAND)) ** 2 <= 1.0
    inner = (dx / (RING_RX - BAND)) ** 2 + (dy / (RING_RY - BAND)) ** 2 < 1.0
    return outer and not inner


def plate_shade(theta, center, span):
    """Opacité et teinte d'un point de l'arc : plaques séparées par des joints, filet clair au
    bord extérieur (rendu par l'appelant)."""
    start = center - span
    step = 2 * span / PLATES
    t = theta - start
    if t < 0 or t > 2 * span:
        return None
    k = min(PLATES - 1, int(t / step))
    local = t - k * step
    if local < GAP / 2 or local > step - GAP / 2:
        return None
    return k


def arc_rect(center, span):
    """Rectangle (logique, entier) qui contient l'arc, marge d'une unité."""
    xs, ys = [], []
    for k in range(0, 201):
        theta = math.radians(center - span + 2 * span * k / 200)
        for rx, ry in ((RING_RX + BAND, RING_RY + BAND), (RING_RX - BAND, RING_RY - BAND)):
            xs.append(CX + rx * math.cos(theta))
            ys.append(CY - ry * math.sin(theta))
    x0, y0 = math.floor(min(xs)) - 1, math.floor(min(ys)) - 1
    x1, y1 = math.ceil(max(xs)) + 1, math.ceil(max(ys)) + 1
    return x0, y0, x1 - x0, y1 - y0


def arc_image(center, span):
    x0, y0, w, h = arc_rect(center, span)
    rows = []
    for py in range(h * SCALE):
        row = bytearray([0])
        for px in range(w * SCALE):
            acc_a = 0.0
            acc_c = 0.0
            for sy in range(SS):
                for sx in range(SS):
                    x = x0 + (px * SS + sx + 0.5) / R
                    y = y0 + (py * SS + sy + 0.5) / R
                    if not in_band(x, y):
                        continue
                    theta = ring_angle(x, y)
                    # Ramène l'angle autour du centre de l'arc (-180..180).
                    rel = (theta - center + 180.0) % 360.0 - 180.0
                    if plate_shade(rel + center, center, span) is None:
                        continue
                    dx, dy = x - CX, y - CY
                    rim = (dx / (RING_RX + BAND - 0.9)) ** 2 + (dy / (RING_RY + BAND - 0.9)) ** 2 > 1.0
                    a, c = (1.0, 1.0) if rim else (0.92, 0.78)
                    acc_a += a
                    acc_c += a * c
            n = SS * SS
            alpha = acc_a / n
            if alpha <= 1e-6:
                row += bytes(4)
                continue
            v = round(min(1.0, acc_c / acc_a) * 255)
            row += bytes((v, v, v, round(alpha * 255)))
        rows.append(bytes(row))
    return (x0, y0, w, h), png(w * SCALE, h * SCALE, b"".join(rows))


# --- Le cœur : un losange à cœur clair, dans un anneau sombre qui le détache de la coque. -----
CORE = 14


def core_image():
    c = CORE / 2
    rows = []
    for py in range(CORE * SCALE):
        row = bytearray([0])
        for px in range(CORE * SCALE):
            acc_a = 0.0
            acc_c = 0.0
            for sy in range(SS):
                for sx in range(SS):
                    x = (px * SS + sx + 0.5) / R - c
                    y = (py * SS + sy + 0.5) / R - c
                    m = abs(x) + abs(y)
                    if m <= 4.6:
                        a, v = 1.0, (1.0 if m < 2.2 else 0.82)
                    elif m <= 5.6:
                        a, v = 0.9, 0.0  # liseré sombre
                    elif m <= 6.6:
                        a, v = 0.95, 0.9  # anneau extérieur
                    else:
                        continue
                    acc_a += a
                    acc_c += a * v
            n = SS * SS
            alpha = acc_a / n
            if alpha <= 1e-6:
                row += bytes(4)
                continue
            v = round(min(1.0, acc_c / acc_a) * 255)
            row += bytes((v, v, v, round(alpha * 255)))
        rows.append(bytes(row))
    return png(CORE * SCALE, CORE * SCALE, b"".join(rows))


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    rects = {}
    for name, (w, h, data) in ship_images().items():
        (OUT / name).write_bytes(data)
        print(f"écrit {OUT / name} ({w * SCALE} × {h * SCALE})")
    rects["SHIP"] = (SHIP_X0, SHIP_Y0, SHIP_X1 - SHIP_X0, SHIP_Y1 - SHIP_Y0)
    for name, center, span, key in (
        ("arc_bow.png", 90.0, BOW_SPAN, "BOW_ARC"),
        ("arc_port.png", 180.0, SIDE_SPAN, "PORT_ARC"),
    ):
        rect, data = arc_image(center, span)
        (OUT / name).write_bytes(data)
        rects[key] = rect
        print(f"écrit {OUT / name} ({rect[2] * SCALE} × {rect[3] * SCALE})")
    (OUT / "core.png").write_bytes(core_image())
    rects["CORE"] = (CX - CORE / 2, CY - CORE / 2 + 4, CORE, CORE)
    print(f"écrit {OUT / 'core.png'} ({CORE * SCALE} × {CORE * SCALE})")
    print(f"\nZone : {AREA_W} × {AREA_H} ; rectangles (x, y, l, h) pour combat_hud.rs :")
    for key, (x, y, w, h) in rects.items():
        print(f"    SCHEMATIC_{key} = ({x}, {y}, {w}, {h})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
