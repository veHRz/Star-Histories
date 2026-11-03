"""Le spatioport d'Hexis, un quartier en version brute (proposition P7).

Le canon dit seulement qu'Hexis a des spatioports (vide connu, §11.1 : « quartiers, villes et
spatioports […] d'Hexis ») : tout ici est proposé. Cinq cellules, géométrie du kit d'intérieur :

    passerelle (le vaisseau s'y amarre, point d'attache « @ship » à son extrémité est)
    hall (salle des pas perdus, tableau des départs)
      ├── bureau (nord) : terminal des contrats Hexa-Corp
      ├── taverne (sud) : comptoir, tables
      └── marche (ouest) : étals

Sorties : assets/models/places/hexis_spaceport/<cellule>{,_lod}.glb, lightmaps dans
assets/textures/lightmaps/hexis_spaceport/, descripteur
assets/data/propositions/sectors/hexis_spaceport.ron, art/places/hexis_spaceport.blend.
Repères Blender : X = est, Y = nord, Z = haut.
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "kit"))

import common as c  # noqa: E402
import sector_tools as st  # noqa: E402
import ship_interior as kit  # noqa: E402

SECTOR = "hexis_spaceport"

CELLS = {
    "passerelle": (-12.0, -0.1, -1.2, 1.2, 2.6),
    "hall": (-26.0, -12.4, -7.0, 7.0, 5.0),
    "bureau": (-24.0, -16.0, 7.4, 13.4, 3.0),
    "taverne": (-24.0, -16.0, -13.4, -7.4, 3.2),
    "marche": (-40.0, -26.4, -6.0, 6.0, 4.5),
}

PORTALS = [
    # Le vaisseau amarré : son sas s'ouvre ici (voir sh_data::dock).
    ("passerelle", "@ship", "Door", 0.0, 0.0, "x"),
    ("passerelle", "hall", "Opening", -12.2, 0.0, "x"),
    ("hall", "bureau", "Door", -20.0, 7.2, "y"),
    ("hall", "taverne", "Door", -20.0, -7.2, "y"),
    ("hall", "marche", "Door", -26.2, 0.0, "x"),
]

SPAWNS = [
    ("quai", "passerelle", -2.6, 0.0, 0.0, 90.0),
    ("hall", "hall", -19.0, 0.0, 0.0, -90.0),
]

WARM = [1.0, 0.82, 0.62]
NEUTRAL = [1.0, 0.94, 0.86]


def light(name, x, y, h, m, coll, lumens, rng, color=NEUTRAL, facing=0.0):
    props = {"lumens": lumens, "range": rng, "color": color, "shadow_candidate": 0}
    return kit.ceiling_light(name, (x, y, h), facing, m, coll, props)


def build_cell(cell: str, m: dict) -> dict:
    x0, x1, y0, y1, h = CELLS[cell]
    coll = c.collection(f"CELL_{cell}")
    shell = kit.room_shell(cell, x0, x1, y0, y1, 0.0, h, st.openings_for(cell, CELLS, PORTALS, {}), m, coll, floor_strip=(cell == "passerelle"))
    out = {"visual": list(shell["visual"]), "structure": list(shell["structure"]), "colliders": list(shell["colliders"]), "marks": [], "pickups": []}

    def add(result):
        vis, col, marks = result
        out["visual"].extend(vis)
        out["colliders"].extend(col)
        out["marks"].extend(marks)

    if cell == "passerelle":
        for i, x in enumerate((-10.0, -6.0, -2.0)):
            add(light(f"{cell}_l{i}", x, 0.0, h, m, coll, 15000.0, 7.0, facing=math.pi / 2))
        for i, x in enumerate((-9.0, -5.0)):
            out["visual"].append(c.box(f"{cell}_stripe{i}", (0.3, 2.2, 0.01), (x, 0.0, 0.006), m["hazard"], coll))
    elif cell == "hall":
        add(kit.console(f"{cell}_departures", (-13.4, 4.5, 0.0), -math.pi / 2, m, coll, label="departures"))
        for row, y in enumerate((-2.2, 2.2)):
            for i, x in enumerate((-23.0, -21.6, -18.4, -17.0)):
                add(kit.seat(f"{cell}_bench{row}_{i}", (x, y, 0.0), 0.0 if row == 0 else math.pi, m, coll))
        add(kit.table(f"{cell}_kiosk", (-20.0, 0.0, 0.0), 0.0, m, coll, size=(2.0, 1.0)))
        for i, (x, y) in enumerate(((-14.0, -5.6), (-14.0, 5.8), (-25.0, 5.8))):
            add(kit.crate(f"{cell}_crate{i}", (x, y, 0.0), 0.2 * i, m, coll))
        for i, (x, y) in enumerate(((-23.0, -3.5), (-23.0, 3.5), (-17.0, -3.5), (-17.0, 3.5))):
            add(light(f"{cell}_l{i}", x, y, h, m, coll, 40000.0, 12.0))
    elif cell == "bureau":
        add(kit.console(f"{cell}_contracts", (-20.0, 12.9, 0.0), math.pi, m, coll, label="contracts"))
        add(kit.table(f"{cell}_desk", (-21.8, 10.6, 0.0), 0.0, m, coll, size=(1.8, 0.9)))
        add(kit.seat(f"{cell}_seat0", (-21.8, 9.8, 0.0), 0.0, m, coll))
        add(kit.locker(f"{cell}_locker0", (-16.45, 12.0, 0.0), -math.pi / 2, m, coll))
        add(kit.locker(f"{cell}_locker1", (-16.45, 11.3, 0.0), -math.pi / 2, m, coll))
        for i, x in enumerate((-22.0, -18.0)):
            add(light(f"{cell}_l{i}", x, 10.4, h, m, coll, 18000.0, 8.0))
    elif cell == "taverne":
        # Comptoir le long du mur sud, tables et tabourets.
        add(kit.table(f"{cell}_bar", (-20.0, -12.6, 0.0), 0.0, m, coll, size=(5.0, 0.8)))
        add(kit.console(f"{cell}_tap", (-17.6, -12.9, 0.0), 0.0, m, coll, label="bar"))
        for i, (x, y) in enumerate(((-22.4, -9.6), (-19.0, -9.6))):
            add(kit.table(f"{cell}_table{i}", (x, y, 0.0), 0.0, m, coll, size=(1.2, 1.2)))
            add(kit.seat(f"{cell}_seat{i}a", (x - 1.0, y, 0.0), -math.pi / 2, m, coll))
            add(kit.seat(f"{cell}_seat{i}b", (x + 1.0, y, 0.0), math.pi / 2, m, coll))
        for i, y in enumerate((-12.6, -11.6)):
            add(kit.crate(f"{cell}_keg{i}", (-23.4, y, 0.0), 0.1, m, coll, size=(0.7, 0.7, 0.9)))
        for i, x in enumerate((-22.0, -18.0)):
            add(light(f"{cell}_l{i}", x, -10.4, h, m, coll, 12000.0, 7.0, color=WARM))
    elif cell == "marche":
        for i, (x, y) in enumerate(((-37.0, -3.5), (-32.0, -3.5), (-37.0, 3.5), (-32.0, 3.5))):
            add(kit.table(f"{cell}_stall{i}", (x, y, 0.0), 0.0, m, coll, size=(2.4, 1.0)))
            add(kit.crate(f"{cell}_stall{i}_crate", (x - 0.6, y, 0.8), 0.3 * i, m, coll, size=(0.6, 0.5, 0.4)))
            add(kit.crate(f"{cell}_stall{i}_stock", (x + 0.4, y + 1.1, 0.0), -0.2 * i, m, coll))
        add(kit.console(f"{cell}_terminal", (-28.4, 5.5, 0.0), math.pi, m, coll, label="market"))
        for i, (x, y) in enumerate(((-36.0, -3.0), (-30.0, -3.0), (-36.0, 3.0), (-30.0, 3.0))):
            add(light(f"{cell}_l{i}", x, y, h, m, coll, 30000.0, 10.0))
    return out


def main() -> None:
    c.reset_scene()
    m = kit.palette()
    built = {cell: build_cell(cell, m) for cell in CELLS}
    meta = {
        "script": "tools/blender/places/hexis_spaceport.py",
        "comment": "Le spatioport d'Hexis, quartier brut (proposition P7) : cellules, portails, apparitions.",
        "id": SECTOR,
        "kind": "District",
        "name_key": f"sector.{SECTOR}",
        "status": 'Proposition("P7")',
        "door_model": "models/kit/ship_interior/door_sliding.glb",
        "models": f"models/places/{SECTOR}",
        "lightmaps": f"textures/lightmaps/{SECTOR}",
    }
    st.export_sector(meta, CELLS, PORTALS, SPAWNS, built, f"assets/data/propositions/sectors/{SECTOR}.ron", f"art/places/{SECTOR}.blend")
    print(f"{SECTOR} : {len(CELLS)} cellules, {len(PORTALS)} portails exportés")


if __name__ == "__main__":
    main()
