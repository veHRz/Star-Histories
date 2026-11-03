"""Outils communs aux scripts Blender de Star Histories (AGENTS.md §9).

Chaque asset est produit par un script versionné, rejouable, sans clic manuel. Un script :
1. construit l'asset à partir de zéro (reset_scene),
2. enregistre la source dans art/<catégorie>/<asset>.blend,
3. exporte assets/models/<catégorie>/<asset>.glb.

Conventions (vérifiées de bout en bout par le jeu) :
- 1 unité = 1 mètre ; Blender Z-haut, l'export glTF convertit en Y-haut ;
- l'avant d'un objet pointe vers +Y dans Blender (−Z dans Bevy) ;
- noms relus par le jeu : `*_col` (collision, non rendue), `*_lod1`, `CELL_<id>`,
  `PORTAL_<a>_<b>`, `SPAWN_<id>`, `SOCKET_<usage>`, `INTERACT_<type>`, `LIGHT_<id>` ;
- les propriétés personnalisées deviennent des extras glTF (métadonnées).

Lancement :
- en arrière-plan : blender --background --factory-startup --python tools/blender/<…>.py
- dans le Blender ouvert (MCP) : run_in_open_blender("tools/blender/<…>.py")
"""

from __future__ import annotations

import json
import math
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[2]

_MATERIALS: dict[str, bpy.types.Material] = {}


# --- Scène ------------------------------------------------------------------------------------


def reset_scene() -> None:
    """Scène vide : supprime objets, collections, maillages et matériaux orphelins."""
    if bpy.context.object and bpy.context.object.mode != "OBJECT":
        bpy.ops.object.mode_set(mode="OBJECT")
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)
    for coll in list(bpy.data.collections):
        bpy.data.collections.remove(coll)
    for block in (bpy.data.meshes, bpy.data.materials, bpy.data.lights, bpy.data.cameras):
        for item in list(block):
            if item.users == 0:
                block.remove(item)
    _MATERIALS.clear()
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"
    scene.unit_settings.scale_length = 1.0


def collection(name: str, parent: bpy.types.Collection | None = None) -> bpy.types.Collection:
    coll = bpy.data.collections.get(name) or bpy.data.collections.new(name)
    target = parent or bpy.context.scene.collection
    if coll.name not in target.children:
        target.children.link(coll)
    return coll


# --- Matériaux --------------------------------------------------------------------------------


def material(
    name: str,
    color: tuple[float, float, float],
    metallic: float = 0.0,
    roughness: float = 0.6,
    emission: tuple[float, float, float] | None = None,
    emission_strength: float = 0.0,
) -> bpy.types.Material:
    """Matériau PBR partagé (peu de matériaux par scène : AGENTS.md §9.3)."""
    if name in _MATERIALS:
        return _MATERIALS[name]
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.use_nodes = True
    # Nœuds cherchés par type, jamais par nom (les noms sont traduits selon la langue de Blender).
    bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    if emission is not None:
        bsdf.inputs["Emission Color"].default_value = (*emission, 1.0)
        bsdf.inputs["Emission Strength"].default_value = emission_strength
    mat.diffuse_color = (*color, 1.0)
    _MATERIALS[name] = mat
    return mat


# --- Géométrie --------------------------------------------------------------------------------


def _link(obj: bpy.types.Object, coll: bpy.types.Collection | None) -> bpy.types.Object:
    (coll or bpy.context.scene.collection).objects.link(obj)
    return obj


def box(
    name: str,
    size: tuple[float, float, float],
    center: tuple[float, float, float],
    mat: bpy.types.Material | None,
    coll: bpy.types.Collection | None = None,
    bevel: float = 0.0,
) -> bpy.types.Object:
    """Pavé de dimensions `size` (x, y, z) centré en `center`, origine au centre."""
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * size[0], v.co.y * size[1], v.co.z * size[2]))
    if bevel > 0.0:
        bmesh.ops.bevel(
            bm, geom=list(bm.edges), offset=bevel, segments=1, affect="EDGES", profile=0.5
        )
    bm.to_mesh(mesh)
    bm.free()
    if mat is not None:
        mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    obj.location = center
    return _link(obj, coll)


def cylinder(
    name: str,
    radius: float,
    depth: float,
    center: tuple[float, float, float],
    mat: bpy.types.Material | None,
    coll: bpy.types.Collection | None = None,
    axis: str = "Z",
    segments: int = 16,
) -> bpy.types.Object:
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    bmesh.ops.create_cone(
        bm, cap_ends=True, segments=segments, radius1=radius, radius2=radius, depth=depth
    )
    if axis == "X":
        bmesh.ops.rotate(bm, verts=bm.verts, cent=(0, 0, 0), matrix=_rot("Y"))
    elif axis == "Y":
        bmesh.ops.rotate(bm, verts=bm.verts, cent=(0, 0, 0), matrix=_rot("X"))
    bm.to_mesh(mesh)
    bm.free()
    if mat is not None:
        mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    obj.location = center
    return _link(obj, coll)


def _rot(axis: str):
    from mathutils import Matrix

    return Matrix.Rotation(math.radians(90), 3, axis)


def collider_box(
    name: str,
    size: tuple[float, float, float],
    center: tuple[float, float, float],
    coll: bpy.types.Collection | None = None,
) -> bpy.types.Object:
    """Collision simplifiée : nom terminé par `_col`, jamais rendue par le jeu."""
    if not name.endswith("_col"):
        name = f"{name}_col"
    obj = box(name, size, center, None, coll)
    obj.display_type = "WIRE"
    obj.hide_render = True
    return obj


def empty(
    name: str,
    location: tuple[float, float, float],
    coll: bpy.types.Collection | None = None,
    props: dict | None = None,
    size: float = 0.3,
    rotation_z: float = 0.0,
) -> bpy.types.Object:
    """Repère nommé (CELL_, PORTAL_, SPAWN_, INTERACT_, LIGHT_…) ; `props` → extras glTF."""
    obj = bpy.data.objects.new(name, None)
    obj.empty_display_type = "ARROWS"
    obj.empty_display_size = size
    obj.location = location
    obj.rotation_euler = (0.0, 0.0, rotation_z)
    for key, value in (props or {}).items():
        obj[key] = value
    return _link(obj, coll)


def join_by_material(objects: list[bpy.types.Object], prefix: str, coll) -> list[bpy.types.Object]:
    """Fusionne des copies des objets, un maillage par matériau : peu d'appels de rendu."""
    groups: dict[str, list[bpy.types.Object]] = {}
    for obj in objects:
        if obj.type != "MESH" or not obj.data.materials:
            continue
        groups.setdefault(obj.data.materials[0].name, []).append(obj)
    merged = []
    for mat_name, objs in sorted(groups.items()):
        copies = []
        for obj in objs:
            copy = obj.copy()
            copy.data = obj.data.copy()
            coll.objects.link(copy)
            copies.append(copy)
        bpy.ops.object.select_all(action="DESELECT")
        for c in copies:
            c.select_set(True)
        bpy.context.view_layer.objects.active = copies[0]
        if len(copies) > 1:
            bpy.ops.object.join()
        joined = bpy.context.view_layer.objects.active
        joined.name = f"{prefix}_{mat_name}"
        joined.data.name = joined.name
        merged.append(joined)
    return merged


# --- Fichiers ---------------------------------------------------------------------------------


def save_blend(rel_path: str) -> Path:
    path = ROOT / rel_path
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(path), compress=True, copy=True)
    return path


def export_glb(rel_path: str, objects: list[bpy.types.Object]) -> Path:
    """Exporte exactement `objects` en glTF binaire (Y-haut, extras, modificateurs appliqués)."""
    path = ROOT / rel_path
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_extras=True,
        export_yup=True,
        export_apply=True,
        export_cameras=False,
        export_lights=False,
        export_materials="EXPORT",
    )
    return path


def write_text(rel_path: str, text: str) -> Path:
    path = ROOT / rel_path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")
    return path


def run_in_open_blender(rel_path: str) -> None:
    """Exécute un script d'asset dans le Blender ouvert (via le MCP), comme en arrière-plan."""
    path = ROOT / rel_path
    code = compile(path.read_text(encoding="utf-8"), str(path), "exec")
    exec(code, {"__name__": "__main__", "__file__": str(path)})


def to_bevy(v: tuple[float, float, float]) -> tuple[float, float, float]:
    """Blender (x, y, z) Z-haut → Bevy (x, z, −y) Y-haut, comme l'export glTF."""
    x, y, z = v
    return (round(x, 4), round(z, 4), round(-y, 4))


def props_json(obj: bpy.types.Object) -> str:
    return json.dumps({k: obj[k] for k in obj.keys() if not k.startswith("_")}, sort_keys=True)
