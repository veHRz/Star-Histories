"""Installations orbitales du système d'Hexis (propositions P15 et P18), vues de loin en vue Globale.

- `l_enclume` : la station-chantier de cuirassés, **en construction** (canon §3.5) : une épine de
  4 km, une cale à demi bâtie autour d'une coque de cuirassé inachevée, des modules et radiateurs ;
- `portal_hub` : le hub de Portails d'Hexis (canon §3.7) : quatre anneaux autour d'une station de
  contrôle. L'apparence d'un Portail n'est pas décrite par le canon : anneau de 700 m, lueur bleue ;
- `shipyard` : chantiers orbitaux (canon §3.3) : trois cales de frégates ;
- `freighter` : cargo générique du trafic ambiant (≈ 90 m, taille corvette).

Géométrie simple et lisible (DA « réaliste stylisé »), palette industrielle d'Hexa-Corp (P6).
Une unité = 1 m ; l'avant des vaisseaux vers +Y. Sorties : assets/models/space/<nom>.glb et
art/space/hexis_orbitals.blend.
"""

from __future__ import annotations

import contextlib
import io
import math
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import common as c  # noqa: E402

OUT = "assets/models/space"


def palette() -> dict:
    return {
        "hull": c.material("hc_hull", (0.24, 0.25, 0.27), metallic=0.7, roughness=0.5),
        "dark": c.material("hc_dark", (0.07, 0.075, 0.08), metallic=0.6, roughness=0.5),
        "plate": c.material("hc_plate", (0.22, 0.23, 0.24), metallic=0.75, roughness=0.4),
        "accent": c.material("hc_accent", (0.85, 0.45, 0.08), metallic=0.2, roughness=0.5),
        "light": c.material(
            "hc_light", (1.0, 0.95, 0.85), emission=(1.0, 0.9, 0.75), emission_strength=25.0
        ),
        "portal": c.material(
            "hc_portal", (0.2, 0.5, 1.0), emission=(0.35, 0.65, 1.0), emission_strength=12.0
        ),
        "engine": c.material(
            "hc_engine", (0.3, 0.6, 1.0), emission=(0.45, 0.7, 1.0), emission_strength=18.0
        ),
    }


def beam(name, a, b, thickness, mat, coll):
    """Poutre droite de section carrée entre deux points alignés sur un axe."""
    size = [abs(b[i] - a[i]) or thickness for i in range(3)]
    center = [(a[i] + b[i]) / 2 for i in range(3)]
    return c.box(name, tuple(size), tuple(center), mat, coll)


def torus(name, major, minor, center, mat, coll, axis="Y", segments=64):
    with contextlib.redirect_stdout(io.StringIO()):
        bpy.ops.mesh.primitive_torus_add(
            major_radius=major, minor_radius=minor, major_segments=segments, minor_segments=10
        )
    obj = bpy.context.active_object
    obj.name = name
    obj.data.name = name
    obj.data.materials.append(mat)
    if axis == "Y":
        obj.rotation_euler = (math.pi / 2, 0.0, 0.0)
    elif axis == "X":
        obj.rotation_euler = (0.0, math.pi / 2, 0.0)
    obj.location = center
    for col in list(obj.users_collection):
        col.objects.unlink(obj)
    coll.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    with contextlib.redirect_stdout(io.StringIO()):
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    return obj


def nav_lights(prefix, points, size, m, coll):
    return [c.box(f"{prefix}_nav{i}", (size, size, size), p, m["light"], coll) for i, p in enumerate(points)]


def l_enclume(m, coll):
    """Station-chantier de cuirassés en construction (≈ 4 km)."""
    objs = []
    # Épine dorsale et module de commandement à l'arrière.
    objs.append(c.box("enclume_spine", (140, 4200, 140), (0, 0, 420), m["hull"], coll))
    objs.append(c.box("enclume_command", (520, 420, 320), (0, -2250, 420), m["hull"], coll, bevel=20))
    objs.append(c.box("enclume_command_band", (540, 60, 60), (0, -2050, 560), m["accent"], coll))
    for i, x in enumerate((-420, 420)):
        objs.append(c.cylinder(f"enclume_tank{i}", 120, 600, (x, -2250, 420), m["plate"], coll, axis="Y", segments=24))
    # Radiateurs, de part et d'autre de l'épine.
    for i, y in enumerate((-1500, -900, -300)):
        for side in (-1, 1):
            objs.append(c.box(f"enclume_radiator{i}_{side}", (700, 120, 8), (side * 500, y, 420), m["dark"], coll))
    # Cale : cadres transversaux tous les 300 m ; la moitié avant n'est pas encore bâtie.
    half_w, z0, z1, t = 520, -330, 330, 36
    for i, y in enumerate(range(-1800, 1801, 300)):
        built = y < 700
        corners = [(-half_w, z0), (half_w, z0), (half_w, z1), (-half_w, z1)]
        if not built and i % 2:
            continue
        for k in range(4):
            (x0, za), (x1, zb) = corners[k], corners[(k + 1) % 4]
            if not built and k in (1, 2):
                continue  # cadres inachevés : seulement deux côtés posés
            objs.append(beam(f"enclume_rib{i}_{k}", (x0, y, za), (x1, y, zb), t, m["hull"], coll))
    # Longerons de la cale (les deux du haut s'arrêtent : chantier en cours).
    for k, (x, z, y_end) in enumerate(((-half_w, z0, 1800), (half_w, z0, 1800), (-half_w, z1, 700), (half_w, z1, 700))):
        objs.append(beam(f"enclume_rail{k}", (x, -1800, z), (x, y_end, z), t, m["hull"], coll))
    # Grues sur les longerons supérieurs.
    for i, y in enumerate((-1200, -200, 600)):
        objs.append(c.box(f"enclume_crane{i}", (60, 60, 260), (half_w, y, z1 + 130), m["accent"], coll))
        objs.append(c.box(f"enclume_crane{i}_arm", (420, 40, 40), (half_w - 200, y, z1 + 250), m["accent"], coll))
    # Coque de cuirassé inachevée : bordé posé à l'arrière, membrures nues à l'avant.
    objs.append(c.box("enclume_hull_keel", (220, 2600, 40), (0, -100, -200), m["plate"], coll))
    objs.append(c.box("enclume_hull_aft", (300, 1100, 240), (0, -900, -60), m["plate"], coll, bevel=25))
    objs.append(c.box("enclume_hull_mid", (300, 500, 120), (0, -100, -120), m["plate"], coll))
    for i, y in enumerate(range(200, 1201, 125)):
        objs.append(c.box(f"enclume_frame{i}", (300, 18, 240), (0, y, -60), m["dark"], coll))
    # Feux de position aux extrémités et sur les cadres.
    points = [(x, y, z) for x in (-half_w, half_w) for y in (-1800, -600, 600) for z in (z1,)]
    points += [(0, 2100, 420), (0, -2460, 600)]
    objs += nav_lights("enclume", points, 14, m, coll)
    return objs


def portal_hub(m, coll):
    """Quatre anneaux de Portail autour d'une station de contrôle."""
    objs = [
        c.cylinder("hub_core", 160, 360, (0, 0, 0), m["hull"], coll, segments=32),
        c.box("hub_core_band", (340, 340, 40), (0, 0, 120), m["accent"], coll),
        c.cylinder("hub_core_top", 90, 160, (0, 0, 260), m["plate"], coll, segments=24),
    ]
    for i, angle in enumerate((0.0, 90.0, 180.0, 270.0)):
        a = math.radians(angle)
        dx, dy = math.cos(a), math.sin(a)
        center = (dx * 1500, dy * 1500, 0)
        ring = torus(f"hub_ring{i}", 350, 32, center, m["plate"], coll, axis="Y")
        ring.rotation_euler = (0.0, 0.0, a - math.pi / 2)
        glow = torus(f"hub_ring{i}_glow", 318, 6, center, m["portal"], coll, axis="Y")
        glow.rotation_euler = (0.0, 0.0, a - math.pi / 2)
        objs += [ring, glow]
        # Pylône de liaison vers la station centrale.
        objs.append(beam(f"hub_pylon{i}", (dx * 160, dy * 160, -20), (dx * 1150, dy * 1150, 20), 40, m["hull"], coll))
        objs += nav_lights(f"hub_ring{i}", [(dx * 1500, dy * 1500, 390)], 18, m, coll)
    with contextlib.redirect_stdout(io.StringIO()):
        bpy.context.view_layer.update()
        for o in objs:
            if o.name.startswith("hub_ring"):
                bpy.ops.object.select_all(action="DESELECT")
                o.select_set(True)
                bpy.context.view_layer.objects.active = o
                bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    return objs


def shipyard(m, coll):
    """Trois cales de frégates côte à côte, dont deux occupées."""
    objs = []
    for i, x in enumerate((-700, 0, 700)):
        w, l, h, t = 260, 520, 200, 20
        for y in (-l / 2, 0, l / 2):
            for (xa, za), (xb, zb) in (((-w, -h), (w, -h)), ((w, -h), (w, h)), ((-w, h), (w, h)), ((-w, -h), (-w, h))):
                objs.append(beam(f"yard{i}_rib{int(y)}_{xa}_{za}_{zb}", (x + xa, y, za), (x + xb, y, zb), t, m["hull"], coll))
        for xa in (-w, w):
            for za in (-h, h):
                objs.append(beam(f"yard{i}_rail{xa}_{za}", (x + xa, -l / 2, za), (x + xa, l / 2, za), t, m["hull"], coll))
        if i != 2:
            objs.append(c.box(f"yard{i}_frigate", (110, 330, 80), (x, 0, 0), m["plate"], coll, bevel=10))
            objs.append(c.box(f"yard{i}_frigate_bridge", (50, 60, 40), (x, -110, 60), m["hull"], coll))
        objs.append(c.box(f"yard{i}_crane", (30, 30, 160), (x + w, l / 2, h + 80), m["accent"], coll))
        objs += nav_lights(f"yard{i}", [(x - w, -l / 2, h), (x + w, -l / 2, h), (x - w, l / 2, h)], 8, m, coll)
    objs.append(beam("yard_spine", (-1000, 0, -260), (1000, 0, -260), 60, m["hull"], coll))
    objs.append(c.box("yard_office", (300, 160, 140), (0, 0, -380), m["hull"], coll, bevel=10))
    return objs


def freighter(m, coll):
    """Cargo générique, ≈ 90 m, avant vers +Y."""
    objs = [
        c.box("freighter_hull", (16, 70, 12), (0, 0, 0), m["hull"], coll, bevel=1.5),
        c.box("freighter_bridge", (10, 10, 6), (0, 30, 8), m["plate"], coll, bevel=1.0),
        c.box("freighter_stripe", (16.4, 4, 12.4), (0, 22, 0), m["accent"], coll),
        c.box("freighter_engine_block", (20, 12, 14), (0, -38, 0), m["plate"], coll, bevel=1.0),
    ]
    for i, x in enumerate((-6, 6)):
        objs.append(c.cylinder(f"freighter_nozzle{i}", 3.5, 4, (x, -46, 0), m["dark"], coll, axis="Y"))
        objs.append(c.cylinder(f"freighter_glow{i}", 2.8, 0.6, (x, -48.2, 0), m["engine"], coll, axis="Y"))
    for i, y in enumerate((-20, -6, 8)):
        for side in (-1, 1):
            objs.append(c.box(f"freighter_container{i}_{side}", (8, 12, 8), (side * 12, y, 0), m["dark" if i % 2 else "accent"], coll))
    objs += nav_lights("freighter", [(0, 35, 11), (-10, -30, 7), (10, -30, 7)], 0.8, m, coll)
    return objs


def main() -> None:
    c.reset_scene()
    m = palette()
    exports = c.collection("EXPORT")
    assets = {
        "l_enclume": l_enclume,
        "portal_hub": portal_hub,
        "shipyard": shipyard,
        "freighter": freighter,
    }
    quiet = io.StringIO()
    for name, builder in assets.items():
        coll = c.collection(f"ASSET_{name}")
        parts = builder(m, coll)
        with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
            merged = c.join_by_material(parts, name, exports)
            c.export_glb(f"{OUT}/{name}.glb", merged)
            for obj in merged:
                obj.hide_set(True)
        tris = sum(len(p.vertices) - 2 for o in merged for p in o.data.polygons)
        print(f"{name} : {len(merged)} maillages, {tris} triangles")
    c.save_blend("art/space/hexis_orbitals.blend")


if __name__ == "__main__":
    main()
