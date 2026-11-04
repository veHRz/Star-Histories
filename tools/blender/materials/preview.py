"""Aides au rendu des aperçus du lot 0 (planche, salle témoin) : Cycles sur CPU, en arrière-plan.

Les aperçus vont dans target/art-previews/lot0/ (non versionné).
"""

from __future__ import annotations

import math
from pathlib import Path

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[3]
PREVIEWS = ROOT / "target" / "art-previews" / "lot0"


def setup_cycles(samples: int = 64, width: int = 1600, height: int = 900, exposure: float = 0.0, look: str | None = None) -> None:
    scene = bpy.context.scene
    try:
        scene.render.engine = "CYCLES"
    except TypeError as err:
        raise RuntimeError(f"Cycles indisponible : {err}") from err
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    scene.cycles.use_denoising = True
    scene.cycles.use_adaptive_sampling = True
    scene.cycles.max_bounces = 6
    scene.cycles.diffuse_bounces = 3
    scene.cycles.glossy_bounces = 2
    scene.cycles.sample_clamp_indirect = 4.0
    scene.cycles.caustics_reflective = False
    scene.cycles.caustics_refractive = False
    scene.render.resolution_x = width
    scene.render.resolution_y = height
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGB"
    vs = scene.view_settings
    for name in ("AgX", "Filmic", "Standard"):
        try:
            vs.view_transform = name
            break
        except TypeError:
            continue
    if look:
        try:
            vs.look = look
        except TypeError:
            pass
    vs.exposure = exposure


def world(color=(0.0, 0.0, 0.0), strength: float = 0.0) -> None:
    scene = bpy.context.scene
    w = scene.world or bpy.data.worlds.new("World")
    scene.world = w
    w.use_nodes = True
    bg = next((n for n in w.node_tree.nodes if n.type == "BACKGROUND"), None)
    if bg is not None:
        bg.inputs["Color"].default_value = (*color, 1.0)
        bg.inputs["Strength"].default_value = strength


def camera(name: str, location, target, lens: float = 24.0, ortho: float | None = None) -> bpy.types.Object:
    data = bpy.data.cameras.get(name) or bpy.data.cameras.new(name)
    data.lens = lens
    data.clip_start = 0.05
    data.clip_end = 200.0
    if ortho:
        data.type = "ORTHO"
        data.ortho_scale = ortho
    cam = bpy.data.objects.get(name) or bpy.data.objects.new(name, data)
    if cam.name not in bpy.context.scene.collection.objects:
        bpy.context.scene.collection.objects.link(cam)
    cam.location = location
    direction = Vector(target) - Vector(location)
    cam.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = cam
    return cam


def area_light(name: str, location, target, power: float, size: float = 1.0, color=(1.0, 1.0, 1.0)) -> bpy.types.Object:
    data = bpy.data.lights.new(name, type="AREA")
    data.energy = power
    data.size = size
    data.color = color
    obj = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(obj)
    obj.location = location
    obj.rotation_euler = (Vector(target) - Vector(location)).to_track_quat("-Z", "Y").to_euler()
    return obj


def render(path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    return path


def look_from(eye, yaw_deg: float, pitch_deg: float = 0.0, dist: float = 10.0):
    """Cible à `dist` mètres d'un œil, cap `yaw_deg` (0 = +Y, positif vers −X) et tangage."""
    yaw, pitch = math.radians(yaw_deg), math.radians(pitch_deg)
    d = Vector((-math.sin(yaw) * math.cos(pitch), math.cos(yaw) * math.cos(pitch), math.sin(pitch)))
    return tuple(Vector(eye) + d * dist)
