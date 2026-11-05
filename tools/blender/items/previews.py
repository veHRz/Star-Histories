"""Aperçus des objets d'inventaire (lot F de M9) : rendus EEVEE en arrière-plan dans
target/art-previews/m9-lotF/. Pas d'effet sur les exports.

Les objets sont rechargés depuis leurs glTF exportés (ce que le jeu chargera) et posés sur un
établi (plateau à 0,78 m, matériaux de la bibliothèque) ; la caisse de pièces est au sol, comme
dans la soute du vaisseau.

- `table_overview` : vue à hauteur d'œil (1,7 m), éclairage d'intérieur (lampe chaude au-dessus,
  bandeau froid, ambiante) ;
- `table_game_fov` : même point de vue, champ du jeu (75° vertical) ;
- `table_close_a`, `table_close_b`, `floor_parts` : gros plans ;
- `table_ambient` : ambiante seule, faible et froide : un objet mobile dans une cellule à
  lightmap n'est éclairé que par l'ambiante du jeu en Bas et Moyen ; il doit encore se lire ;
- `models_angles` : chaque objet sous quatre angles (avant, côté, arrière, dessus).

    blender --background --factory-startup --python tools/blender/items/previews.py -- [--only overview,angles] [--samples 32]
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

import bpy
import numpy as np
from mathutils import Vector

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))

import common as c  # noqa: E402
import icons  # noqa: E402
from items import ITEMS  # noqa: E402

OUT = c.ROOT / "target" / "art-previews" / "m9-lotF"
TABLE_Z = 0.78

# Disposition sur l'établi : (x, y, cap en degrés) ; la caisse de pièces au sol.
LAYOUT = {
    "datapad": (-0.52, 0.16, 12.0),
    "data_chip": (-0.30, 0.22, -25.0),
    "ration": (-0.14, 0.18, 8.0),
    "cargo_manifest": (0.10, 0.16, -6.0),
    "mag_pistol": (0.42, 0.14, 28.0),
    "toolkit": (-0.46, -0.16, 4.0),
    "medkit": (-0.04, -0.15, -8.0),
    "flight_suit": (0.38, -0.14, 10.0),
}
FLOOR = {"spare_parts": (1.05, 0.25, -20.0)}


def _engine(scene) -> None:
    try:
        scene.render.engine = "BLENDER_EEVEE"
    except TypeError:
        scene.render.engine = "BLENDER_EEVEE_NEXT"


def _world(color, strength: float) -> None:
    world = bpy.data.worlds.get("prev_world") or bpy.data.worlds.new("prev_world")
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (*color, 1.0)
    bg.inputs["Strength"].default_value = strength
    bpy.context.scene.world = world


def _place(objs, x, y, z, yaw_deg):
    for obj in objs:
        obj.location = (x, y, z)
        obj.rotation_mode = "XYZ"
        obj.rotation_euler = (0.0, 0.0, math.radians(yaw_deg))


def _textured_box(name, size, center, lib, tile=None):
    mat = c.textured_material(f"prev_{lib}", lib)
    obj = c.box(name, size, center, mat, bevel=0.004 if min(size) > 0.02 else 0.0)
    c.uv_box(obj, tile)
    return obj


def build_set() -> None:
    """Établi (plateau en métal sombre, pieds), sol en tôle larmée, cloison peinte derrière."""
    _textured_box("floor", (6.0, 6.0, 0.02), (0.0, 0.0, -0.01), "floor_plate")
    _textured_box("wall", (6.0, 0.05, 3.0), (0.0, -0.75, 1.5), "panel_painted")
    _textured_box("table_top", (1.5, 0.75, 0.04), (0.0, 0.0, TABLE_Z - 0.02), "metal_dark", 1.0)
    for sx in (-1, 1):
        for sy in (-1, 1):
            _textured_box(f"leg_{sx}_{sy}", (0.05, 0.05, TABLE_Z - 0.04), (sx * 0.68, sy * 0.32, (TABLE_Z - 0.04) / 2),
                          "metal_bare", 1.0)
    _textured_box("shelf", (1.4, 0.65, 0.02), (0.0, 0.0, 0.18), "metal_dark", 1.0)


def load_items() -> dict:
    placed = {}
    for item, (x, y, yaw) in {**LAYOUT, **FLOOR}.items():
        objs = icons._import(item)
        _place(objs, x, y, TABLE_Z if item in LAYOUT else 0.0, yaw)
        placed[item] = objs
    return placed


def _clear_lights():
    for obj in list(bpy.data.objects):
        if obj.type == "LIGHT":
            bpy.data.objects.remove(obj, do_unlink=True)


def light_interior():
    _clear_lights()
    _world((0.55, 0.62, 0.78), 0.12)
    lamp = bpy.data.lights.new("lamp", "POINT")
    lamp.energy = 140.0
    lamp.color = (1.0, 0.82, 0.62)
    lamp.shadow_soft_size = 0.15
    obj = bpy.data.objects.new("lamp", lamp)
    bpy.context.scene.collection.objects.link(obj)
    obj.location = (0.1, 0.25, 2.3)
    icons._light("band", "AREA", (-1.6, 1.2, 2.2), (0.0, 0.0, TABLE_Z), 120.0, (0.70, 0.82, 1.0), 1.2)


def light_ambient():
    _clear_lights()
    _world((0.55, 0.62, 0.78), 0.35)


def render(name, cam_loc, target, lens=35.0, res=(1600, 900), samples=32, fov_v=None) -> Path:
    scene = bpy.context.scene
    _engine(scene)
    scene.render.film_transparent = False
    scene.view_settings.view_transform = "AgX"
    scene.view_settings.look = "None"
    cam_data = bpy.data.cameras.get("prev_cam") or bpy.data.cameras.new("prev_cam")
    cam_data.type = "PERSP"
    cam_data.clip_start = 0.01
    cam_data.sensor_fit = "VERTICAL" if fov_v else "AUTO"
    if fov_v:
        cam_data.angle_y = math.radians(fov_v)
    else:
        cam_data.lens = lens
    cam = bpy.data.objects.get("prev_cam") or bpy.data.objects.new("prev_cam", cam_data)
    if cam.name not in scene.collection.objects:
        scene.collection.objects.link(cam)
    cam.location = cam_loc
    cam.rotation_euler = (Vector(target) - Vector(cam_loc)).to_track_quat("-Z", "Y").to_euler()
    scene.camera = cam
    scene.render.resolution_x, scene.render.resolution_y = res
    scene.render.resolution_percentage = 100
    scene.render.image_settings.color_mode = "RGB"
    try:
        scene.eevee.taa_render_samples = samples
    except AttributeError:
        pass
    path = OUT / f"{name}.png"
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    print(f"aperçu : {path.relative_to(c.ROOT)}")
    return path


def table_views(samples: int) -> None:
    icons._clean()
    build_set()
    load_items()
    eye = (0.15, 1.25, 1.70)
    light_interior()
    render("table_overview", eye, (-0.02, -0.02, TABLE_Z), lens=30, samples=samples)
    render("table_game_fov", eye, (0.0, -0.1, TABLE_Z + 0.1), fov_v=75.0, samples=samples)
    render("table_close_a", (-0.12, 0.85, 1.30), (-0.12, 0.17, TABLE_Z), lens=40, samples=samples)
    render("table_close_b", (0.05, 0.70, 1.35), (-0.04, -0.15, TABLE_Z + 0.05), lens=32, samples=samples)
    render("floor_parts", (1.25, 1.10, 1.30), (1.05, 0.25, 0.08), lens=40, samples=samples)
    light_ambient()
    render("table_ambient", eye, (-0.02, -0.02, TABLE_Z), lens=30, samples=samples)


def angles_sheet(samples: int, cell: int = 240) -> Path:
    """Chaque objet sous quatre angles, fond gris neutre, éclairage studio des icônes."""
    views = [(35.0, 30.0), (90.0, 20.0), (200.0, 30.0), (0.0, 89.0)]
    rows = []
    tmp = OUT / "angles_tmp.png"
    for item in ITEMS:
        icons._clean()
        objs = icons._import(item)
        row = []
        for az, el in views:
            icons.AZIMUTH, icons.ELEVATION = az, el
            for obj in list(bpy.data.objects):
                if obj.type in ("LIGHT", "CAMERA"):
                    bpy.data.objects.remove(obj, do_unlink=True)
            scene = bpy.context.scene
            icons.setup_render(samples)
            scene.render.film_transparent = False
            scene.render.resolution_x = scene.render.resolution_y = cell
            icons.frame_and_light(objs)
            icons._world((0.16, 0.17, 0.19), 1.0)
            scene.render.filepath = str(tmp)
            bpy.ops.render.render(write_still=True)
            row.append(icons.load_rgba(tmp)[..., :3])
        rows.append(np.concatenate(row, 1))
    sheet = np.concatenate(rows, 0)
    out = OUT / "models_angles.png"
    icons.write_png(out, sheet)
    tmp.unlink(missing_ok=True)
    print(f"aperçu : {out.relative_to(c.ROOT)}")
    return out


def main(argv: list[str]) -> None:
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else ["overview", "angles"]
    samples = int(argv[argv.index("--samples") + 1]) if "--samples" in argv else 32
    if "overview" in only:
        table_views(samples)
    if "angles" in only:
        angles_sheet(samples)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
