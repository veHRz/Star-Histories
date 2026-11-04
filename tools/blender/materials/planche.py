"""Planche de démonstration de la bibliothèque de matériaux (lot 0 de M8).

Chaque matériau sur un panneau de 2 × 2 tuiles et un cylindre à l'échelle réelle (projection
boîte du catalogue), lumière de studio neutre, nom en légende ; une rangée montre les teintes
d'identité (P6) sur les matériaux teintables. `--details` rend aussi un gros plan rasant de
chaque matériau à l'échelle réelle.

    blender --background --factory-startup --python tools/blender/materials/planche.py -- [--details] [--samples N]

Sorties : target/art-previews/lot0/planche.png (et details/<mat>.png).
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))

import bmesh  # noqa: E402
import bpy  # noqa: E402

import common as c  # noqa: E402
import library  # noqa: E402
import preview as pv  # noqa: E402

# Émission des matériaux qui en ont (couleur, intensité) ; le caillebotis montre son émissif
# facultatif (lumière sous les trous).
EMISSIVE = {
    "screen": ((1.0, 1.0, 1.0), 2.0),
    "buttons": ((1.0, 1.0, 1.0), 2.5),
    "light_band": ((1.0, 0.97, 0.92), 1.2),
    "trim_ship": ((1.0, 0.97, 0.92), 1.5),
    "grating": ((0.55, 0.75, 1.0), 0.06),
}
TINTED = [
    ("panel_painted", "hexa_yellow"),
    ("panel_painted", "ugl_bluegray"),
    ("hull_plates", "pirate_rust"),
    ("panel_painted", "empire_anthracite"),
    ("container_ribbed", "olive"),
]


def material_for(name: str, tint_name: str = "neutral"):
    tint = library.TINTS[tint_name]
    emis = EMISSIVE.get(name)
    label = name if tint_name == "neutral" else f"{name}__{tint_name}"
    if emis:
        return c.textured_material(f"lib_{label}", name, tint=tint, emissive=emis[0], emissive_strength=emis[1])
    return c.textured_material(f"lib_{label}", name, tint=tint)


def quad(name: str, size: float, center, mat, tiles: float = 2.0) -> bpy.types.Object:
    """Panneau vertical (plan XZ) tourné vers −Y, UV de 0 à `tiles`."""
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    h = size / 2
    verts = [bm.verts.new(v) for v in ((-h, 0, -h), (h, 0, -h), (h, 0, h), (-h, 0, h))]
    face = bm.faces.new(verts)
    uv = bm.loops.layers.uv.new(c.UV_TEXTURE)
    for loop, (u, v) in zip(face.loops, ((0, 0), (tiles, 0), (tiles, tiles), (0, tiles))):
        loop[uv].uv = (u, v)
    face.normal_update()
    if face.normal.y > 0:
        bmesh.ops.reverse_faces(bm, faces=[face])
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    obj.location = center
    bpy.context.scene.collection.objects.link(obj)
    return obj


def label(text: str, location, size: float = 0.13) -> bpy.types.Object:
    curve = bpy.data.curves.new(f"label_{text}", type="FONT")
    curve.body = text
    curve.size = size
    curve.align_x = "CENTER"
    obj = bpy.data.objects.new(f"label_{text}", curve)
    obj.location = location
    obj.rotation_euler = (math.pi / 2, 0, 0)
    obj.visible_shadow = False  # pas de légende fantôme sur le fond
    obj.data.materials.append(c.material("label_ink", (0.85, 0.85, 0.85), roughness=0.8))
    bpy.context.scene.collection.objects.link(obj)
    return obj


def build_board():
    c.reset_scene()
    cells = [(name, "neutral") for name in library.MATERIALS] + TINTED
    cols = 5
    pitch_x, pitch_z = 2.25, 2.35
    backdrop = c.box("backdrop", (cols * pitch_x + 2, 0.05, 12), (0, 0.4, 0), c.material("backdrop", (0.18, 0.18, 0.19), roughness=0.9))
    _ = backdrop
    for i, (name, tint) in enumerate(cells):
        col, row = i % cols, i // cols
        x = (col - (cols - 1) / 2) * pitch_x
        z = 3.4 - row * pitch_z
        mat = material_for(name, tint)
        quad(f"panel_{i}", 1.6, (x, 0.0, z), mat)
        cyl = c.cylinder(f"cyl_{i}", 0.22, 0.75, (x + 0.62, -0.35, z - 0.42), mat, segments=32)
        c.uv_box(cyl)
        tile = library.MATERIALS[name]["tile_m"]
        text = f"{name}  ({tile:g} m)" if tint == "neutral" else f"{name} + {tint}"
        label(text, (x, -0.02, z - 0.98))
    return cells


def light_board():
    pv.world((0.6, 0.62, 0.66), 0.25)
    sun = bpy.data.lights.new("key", type="SUN")
    sun.energy = 3.2
    sun.angle = math.radians(4)
    obj = bpy.data.objects.new("key", sun)
    bpy.context.scene.collection.objects.link(obj)
    obj.rotation_euler = (math.radians(58), math.radians(-28), math.radians(-35))


def render_details(samples: int):
    """Gros plan rasant de chaque matériau à l'échelle réelle (pavé de 1,2 m, projection boîte)."""
    for obj in list(bpy.context.scene.objects):
        if obj.type in {"MESH", "FONT"}:
            obj.hide_render = True
    pv.setup_cycles(samples, 900, 700, exposure=0.0)
    for name in library.MATERIALS:
        mat = material_for(name)
        tile = library.MATERIALS[name]["tile_m"]
        side = min(1.2, tile)
        slab = c.box(f"detail_{name}", (side, 0.04, side), (0, -6.0, 0.0), mat, bevel=0.004)
        c.uv_box(slab)
        pv.camera("cam_detail", (side * 0.18, -6.0 - side * 0.95, side * 0.12), (0, -6.0, 0.0), lens=50)
        pv.render(pv.PREVIEWS / "details" / f"{name}.png")
        slab.hide_render = True


def main(argv):
    samples = int(argv[argv.index("--samples") + 1]) if "--samples" in argv else 64
    build_board()
    light_board()
    pv.setup_cycles(samples, 2400, 2100, exposure=0.0)
    pv.camera("cam_board", (0, -20, 0.0), (0, 0, 0.0), ortho=11.9)
    out = pv.render(pv.PREVIEWS / "planche.png")
    print(f"planche : {out}")
    if "--details" in argv:
        render_details(max(16, samples // 2))
    c.save_blend("art/materials/planche.blend")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
