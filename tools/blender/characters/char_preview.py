"""Aperçus des personnages et des armes (lot F de M8) : rendus EEVEE en arrière-plan dans
target/art-previews/lotF/. Pas d'effet sur les exports.

Deux éclairages :
- `studio` : clé chaude, contre-jour froid, débouchage, fond gris (lire les formes) ;
- `ambient` : rien qu'une ambiante uniforme et faible, comme un objet mobile dans une cellule à
  lightmap en Bas et Moyen (le jeu ne l'éclaire que par `GlobalAmbientLight`) : le personnage
  doit encore s'y détacher (teintes pas trop sombres, émissifs).
"""

from __future__ import annotations

import math
from pathlib import Path

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "target" / "art-previews" / "lotF"


def _engine(scene) -> None:
    for name in ("BLENDER_EEVEE", "BLENDER_EEVEE_NEXT"):
        try:
            scene.render.engine = name
            return
        except TypeError:
            continue


def _world(color, strength: float) -> None:
    world = bpy.data.worlds.get("preview") or bpy.data.worlds.new("preview")
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (*color, 1.0)
    bg.inputs["Strength"].default_value = strength
    bpy.context.scene.world = world


def _clear_preview() -> None:
    for obj in list(bpy.data.objects):
        if obj.get("sh_preview"):
            bpy.data.objects.remove(obj, do_unlink=True)


def _light(name, kind, loc, target, energy, color, size=1.0):
    data = bpy.data.lights.new(name, kind)
    data.energy = energy
    data.color = color
    if kind == "AREA":
        data.size = size
    obj = bpy.data.objects.new(name, data)
    obj["sh_preview"] = True
    bpy.context.scene.collection.objects.link(obj)
    obj.location = loc
    d = Vector(target) - Vector(loc)
    obj.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    return obj


def _floor(z: float = 0.0, size: float = 6.0) -> None:
    mesh = bpy.data.meshes.new("preview_floor")
    h = size / 2
    mesh.from_pydata([(-h, -h, z), (h, -h, z), (h, h, z), (-h, h, z)], [], [(0, 1, 2, 3)])
    mat = bpy.data.materials.get("preview_floor") or bpy.data.materials.new("preview_floor")
    mat.use_nodes = True
    bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (0.08, 0.08, 0.085, 1.0)
    bsdf.inputs["Roughness"].default_value = 0.8
    mesh.materials.append(mat)
    obj = bpy.data.objects.new("preview_floor", mesh)
    obj["sh_preview"] = True
    bpy.context.scene.collection.objects.link(obj)


def setup(lighting: str = "studio", center=(0.0, 0.0, 0.9), scale: float = 1.0, floor: bool = True) -> None:
    """Éclairage et fond. `scale` : taille du sujet (1 = personnage, 0,2 = arme)."""
    _clear_preview()
    scene = bpy.context.scene
    _engine(scene)
    scene.render.film_transparent = False
    scene.view_settings.view_transform = "AgX"
    scene.view_settings.look = "None"
    cx, cy, cz = center
    s = scale
    if lighting == "studio":
        _world((0.23, 0.24, 0.26), 0.6)
        _light("key", "AREA", (cx + 2.2 * s, cy + 2.4 * s, cz + 1.6 * s), center, 260 * s * s, (1.0, 0.9, 0.78), 1.5 * s)
        _light("rim", "AREA", (cx - 1.8 * s, cy - 2.2 * s, cz + 1.4 * s), center, 320 * s * s, (0.7, 0.82, 1.0), 1.0 * s)
        _light("fill", "AREA", (cx - 2.5 * s, cy + 1.5 * s, cz + 0.2 * s), center, 70 * s * s, (0.9, 0.95, 1.0), 2.0 * s)
    elif lighting == "ambient":
        # Ambiante seule, faible, légèrement froide (couloir du vaisseau) : pas de lampe.
        _world((0.12, 0.125, 0.14), 1.0)
    elif lighting == "dim":
        # Couloir sombre : faible ambiante + une lampe de plafond ambrée lointaine.
        _world((0.04, 0.042, 0.05), 1.0)
        _light("ceiling", "POINT", (cx + 0.5 * s, cy + 1.5 * s, cz + 1.6 * s), center, 60 * s * s, (1.0, 0.75, 0.45))
    if floor:
        _floor()


def render(name: str, cam_loc, target, lens: float = 50.0, res=(900, 1200), samples: int = 32) -> Path:
    scene = bpy.context.scene
    cam_data = bpy.data.cameras.get("preview_cam") or bpy.data.cameras.new("preview_cam")
    cam_data.lens = lens
    cam_data.clip_start = 0.01
    cam = bpy.data.objects.get("preview_cam")
    if cam is None:
        cam = bpy.data.objects.new("preview_cam", cam_data)
        cam["sh_preview"] = True
        scene.collection.objects.link(cam)
    cam.location = cam_loc
    d = Vector(target) - Vector(cam_loc)
    cam.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    scene.camera = cam
    scene.render.resolution_x, scene.render.resolution_y = res
    scene.render.resolution_percentage = 100
    try:
        scene.eevee.taa_render_samples = samples
    except AttributeError:
        pass
    path = OUT / f"{name}.png"
    path.parent.mkdir(parents=True, exist_ok=True)
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    return path


def orbit(target, distance: float, yaw_deg: float, height: float):
    """Position de caméra autour de `target` : yaw 0 = devant (+Y), 90 = côté droit (+X)."""
    a = math.radians(yaw_deg)
    tx, ty, tz = target
    return (tx + distance * math.sin(a), ty + distance * math.cos(a), tz + height)
