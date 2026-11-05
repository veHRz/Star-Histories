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
- les propriétés personnalisées deviennent des extras glTF (métadonnées) ;
- décor par niveau de qualité (M9, docs/m9-retours.md §3.2) : propriété `detail` (entier,
  `set_detail`) : absente ou 0 = toujours affiché, 1 = Moyen et Haut, 2 = Haut seulement ;
  `join_by_material` ne mélange jamais deux niveaux ;
- volume d'interaction (M9, §3.3) : extra `half_extents` d'un `INTERACT_*` (`half_extents`,
  `fit_interact`), demi-tailles en mètres dans le repère local du nœud converti au jeu (Y-haut).

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
    alpha: float = 1.0,
) -> bpy.types.Material:
    """Matériau PBR partagé (peu de matériaux par scène : AGENTS.md §9.3). `alpha` < 1 : matériau
    transparent mélangé (vitrages), exporté en `alphaMode: BLEND`."""
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
    if alpha < 1.0:
        bsdf.inputs["Alpha"].default_value = alpha
        mat.surface_render_method = "BLENDED"
    mat.diffuse_color = (*color, alpha)
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
    """Fusionne des copies des objets, un maillage par matériau **et par niveau de décor**
    (`detail`) : peu d'appels de rendu, et le jeu peut cacher un niveau sans toucher aux autres.

    Niveau 0 : `<prefix>_<matériau>` (nom inchangé depuis M1) ; niveau n > 0 :
    `<prefix>_<matériau>_d<n>`, qui porte la propriété `detail` = n (extra glTF)."""
    groups: dict[tuple[str, int], list[bpy.types.Object]] = {}
    for obj in objects:
        if obj.type != "MESH" or not obj.data.materials:
            continue
        groups.setdefault((obj.data.materials[0].name, detail_of(obj)), []).append(obj)
    merged = []
    for (mat_name, level), objs in sorted(groups.items()):
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
        joined.name = f"{prefix}_{mat_name}" if level == 0 else f"{prefix}_{mat_name}_d{level}"
        joined.data.name = joined.name
        if DETAIL_KEY in joined.keys():
            del joined[DETAIL_KEY]
        if level > 0:
            joined[DETAIL_KEY] = level
        merged.append(joined)
    return merged


# --- Décor par niveau de qualité (M9) -----------------------------------------------------------
# Convention (docs/m9-retours.md §3.2) : `detail` absent ou 0 = toujours ; 1 = Moyen et Haut ;
# 2 = Haut seulement. Le jeu cache un nœud (et ses enfants) dont `detail` dépasse le niveau du
# préréglage (`decor_detail`). Un décor optionnel n'a pas de collision, n'est jamais fusionné
# avec la géométrie de base et ne projette pas d'ombre dans le bake (lightmap.py) ; il a ses UV
# de lightmap comme le reste.

DETAIL_KEY = "detail"
DETAIL_MAX = 2
# Repères qui portent du gameplay : jamais optionnels (le jeu doit se jouer pareil en Bas).
_GAMEPLAY_PREFIXES = ("INTERACT_", "SOCKET_", "PORTAL_", "SPAWN_", "CELL_")


def detail_of(obj: bpy.types.Object) -> int:
    """Niveau de décor d'un objet (0 si la propriété est absente)."""
    return int(obj.get(DETAIL_KEY, 0))


def set_detail(objects, level: int) -> None:
    """Marque des objets de décor au niveau `level` (0, 1 ou 2), propriété personnalisée
    `detail` exportée en extra glTF ; 0 retire la marque. Refuse un collider (`_col`) ou un
    repère de gameplay (`INTERACT_`, `SOCKET_`…) à un niveau > 0. Un `LIGHT_` peut l'être : sa
    lampe n'est alors pas cuite (`lightmap.bake_cells`), elle n'existe qu'en dynamique."""
    level = int(level)
    if not 0 <= level <= DETAIL_MAX:
        raise ValueError(f"niveau de décor hors bornes : {level} (0 à {DETAIL_MAX})")
    for obj in objects:
        if level > 0 and (obj.name.endswith("_col") or obj.name.startswith(_GAMEPLAY_PREFIXES)):
            raise ValueError(f"{obj.name} : un collider ou un repère de gameplay ne peut pas être un décor optionnel")
        if level > 0:
            obj[DETAIL_KEY] = level
        elif DETAIL_KEY in obj.keys():
            del obj[DETAIL_KEY]


# --- Volumes d'interaction (M9) ---------------------------------------------------------------
# Convention (docs/m9-retours.md §3.3) : extra `half_extents: [x, y, z]` sur un `INTERACT_*` →
# capteur en pavé centré sur le nœud ; absent → sphère de 0,35 m. Les demi-tailles sont
# exprimées dans le repère **local** du nœud (elles tournent avec l'objet), converti comme
# l'export glTF : x Blender → x, hauteur (z Blender) → y, profondeur (y Blender) → z.


def half_extents(size: tuple[float, float, float], margin: float = 0.0) -> list[float]:
    """Tailles complètes Blender (x, y, z) → demi-tailles du jeu [x, y (hauteur), z], en
    mètres, agrandies de `margin` de chaque côté."""
    sx, sy, sz = size
    return [round(sx / 2 + margin, 4), round(sz / 2 + margin, 4), round(sy / 2 + margin, 4)]


def fit_interact(mark: bpy.types.Object, objects, margin: float = 0.04, recenter: bool = True) -> list[float]:
    """Ajuste un repère `INTERACT_*` pour que son pavé couvre `objects` (boîtes englobantes
    dans le repère de rotation du repère) : `half_extents` écrit (marge `margin` comprise,
    pour que le rayon de visée touche le capteur avant le collider de l'objet) et, si
    `recenter`, le repère est déplacé au centre de la boîte (sa rotation ne change pas).
    Renvoie les demi-tailles du jeu."""
    bpy.context.view_layer.update()
    rot = mark.matrix_world.to_quaternion().to_matrix()
    inv = rot.inverted()
    lo = Vector((1e9, 1e9, 1e9))
    hi = Vector((-1e9, -1e9, -1e9))
    for obj in objects:
        corners = [obj.matrix_world @ Vector(b) for b in obj.bound_box] if obj.type == "MESH" else [obj.matrix_world.translation]
        for p in corners:
            q = inv @ p
            lo = Vector(map(min, lo, q))
            hi = Vector(map(max, hi, q))
    size = hi - lo
    if recenter:
        center = rot @ ((lo + hi) / 2)
        if mark.parent is not None:
            center = mark.parent.matrix_world.inverted() @ center
        mark.location = center
    he = half_extents(tuple(size), margin)
    mark["half_extents"] = he
    return he


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


# --- Matériaux texturés « futur usé » (M8, lot 0) ------------------------------------------------
# Bibliothèque : tools/blender/materials/library.py (catalogue MATERIALS, bandes TRIMS, teintes
# TINTS), textures cuites dans assets/textures/materials/. Les modèles qui s'en servent sont
# exportés en glTF séparé (`export_gltf`) : leurs images pointent vers ces fichiers par chemin
# relatif, et Bevy ne charge chaque image qu'une fois, quel que soit le nombre de modèles.

TEXTURES_DIR = ROOT / "assets" / "textures" / "materials"
UV_TEXTURE = "UVMap"  # 1er jeu d'UV (TEXCOORD_0) : les textures ; le 2e (`lightmap`) est aux lightmaps
GLTF_OUTPUT_GROUP = "glTF Material Output"  # nom relu par l'exportateur glTF (sortie Occlusion)
_COLORSPACE = {"color": "sRGB", "emissive": "sRGB", "normal": "Non-Color", "orm": "Non-Color"}


def material_library():
    """Module `library` de la bibliothèque (catalogue, bandes de trim), importé à la demande."""
    import importlib
    import sys

    path = str(Path(__file__).resolve().parent / "materials")
    if path not in sys.path:
        sys.path.insert(0, path)
    return importlib.import_module("library")


def _socket(sockets, identifier: str):
    return next(s for s in sockets if s.identifier == identifier)


def _gltf_output_group() -> bpy.types.NodeTree:
    """Groupe de nœuds « glTF Material Output » : l'exportateur glTF de Blender y lit l'occlusion
    (`io_scene_gltf2/blender/com/material_helpers.py`, `get_socket_from_gltf_material_node`)."""
    group = bpy.data.node_groups.get(GLTF_OUTPUT_GROUP)
    if group is not None:
        return group
    try:
        from io_scene_gltf2.blender.com.material_helpers import create_settings_group

        return create_settings_group(GLTF_OUTPUT_GROUP)
    except ImportError:
        group = bpy.data.node_groups.new(GLTF_OUTPUT_GROUP, "ShaderNodeTree")
        group.interface.new_socket("Occlusion", in_out="INPUT", socket_type="NodeSocketFloat")
        group.nodes.new("NodeGroupInput")
        group.nodes.new("NodeGroupOutput")
        return group


def _texture_files(maps) -> dict[str, Path]:
    """Rôle → fichier : `maps` est un nom de la bibliothèque (« panel_painted ») ou un
    dictionnaire {rôle: chemin} (absolu ou relatif à la racine du dépôt)."""
    if isinstance(maps, str):
        files = {role: TEXTURES_DIR / f"{maps}_{role}.png" for role in _COLORSPACE}
        files = {role: path for role, path in files.items() if path.exists()}
    else:
        files = {role: (Path(p) if Path(p).is_absolute() else ROOT / p) for role, p in maps.items()}
    if "color" not in files or not files["color"].exists():
        raise FileNotFoundError(f"texture de couleur introuvable pour {maps!r} (lancer materials/library.py)")
    return files


def textured_material(
    name: str,
    maps,
    tint: tuple[float, float, float] = (1.0, 1.0, 1.0),
    emissive: tuple[float, float, float] | None = None,
    emissive_strength: float = 1.0,
    normal_strength: float = 1.0,
) -> bpy.types.Material:
    """Matériau PBR alimenté par les textures cuites de la bibliothèque, partagé par nom.

    - `maps` : nom d'un matériau de la bibliothèque (« panel_painted ») ou {rôle: chemin} ;
    - `tint` : facteur de couleur de base (glTF `baseColorFactor`) ; teintes d'identité dans
      `library.TINTS` ;
    - `emissive` : couleur d'émission (facteur glTF), multipliée par `<maps>_emissive.png` s'il
      existe ; None : pas d'émission. `emissive_strength` > 1 sort en
      KHR_materials_emissive_strength.

    Faces arrière masquées (`doubleSided: false`) : réserver aux volumes fermés.

    Graphe : nœuds Image sur l'UV `UVMap` (explicite : `lightmap.py` ajoute un 2e jeu), couleur ×
    teinte (Mix Multiplier : l'exportateur en fait le facteur), Normal Map en espace tangent,
    ORM → Separate Color : G rugosité, B métal, R occlusion vers le groupe « glTF Material
    Output ». Les trois canaux viennent de la même image : l'exportateur la garde telle quelle
    (pas d'ORM recomposé). Nœuds cherchés par type ou par identifiant de prise, jamais par nom.
    La taille de tuile du catalogue est notée sur le matériau (`sh_tile_m`) pour `uv_box`.
    """
    if name in _MATERIALS:
        return _MATERIALS[name]
    files = _texture_files(maps)
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
    out = next(n for n in nodes if n.type == "OUTPUT_MATERIAL")
    for node in list(nodes):
        if node not in (bsdf, out):
            nodes.remove(node)
    bsdf.location, out.location = (300, 0), (600, 0)
    uv = nodes.new("ShaderNodeUVMap")
    uv.uv_map = UV_TEXTURE
    uv.location = (-900, 0)

    def image(role: str, y: float):
        node = nodes.new("ShaderNodeTexImage")
        img = bpy.data.images.load(str(files[role]), check_existing=True)
        img.colorspace_settings.name = _COLORSPACE[role]
        node.image = img
        node.extension = "REPEAT"
        node.location = (-600, y)
        links.new(uv.outputs["UV"], node.inputs["Vector"])
        return node

    def multiply(color_out, factor, y: float):
        mix = nodes.new("ShaderNodeMix")
        mix.data_type = "RGBA"
        mix.blend_type = "MULTIPLY"
        mix.location = (-250, y)
        _socket(mix.inputs, "Factor_Float").default_value = 1.0
        links.new(color_out, _socket(mix.inputs, "A_Color"))
        _socket(mix.inputs, "B_Color").default_value = (*factor, 1.0)
        return _socket(mix.outputs, "Result_Color")

    color = image("color", 300)
    links.new(multiply(color.outputs["Color"], tint, 300), bsdf.inputs["Base Color"])
    if "normal" in files:
        normal = image("normal", -300)
        nmap = nodes.new("ShaderNodeNormalMap")
        nmap.space = "TANGENT"
        nmap.uv_map = UV_TEXTURE
        nmap.inputs["Strength"].default_value = normal_strength
        nmap.location = (-250, -300)
        links.new(normal.outputs["Color"], nmap.inputs["Color"])
        links.new(nmap.outputs["Normal"], bsdf.inputs["Normal"])
    if "orm" in files:
        orm = image("orm", 0)
        sep = nodes.new("ShaderNodeSeparateColor")
        sep.mode = "RGB"
        sep.location = (-250, 0)
        links.new(orm.outputs["Color"], sep.inputs["Color"])
        links.new(sep.outputs["Green"], bsdf.inputs["Roughness"])
        links.new(sep.outputs["Blue"], bsdf.inputs["Metallic"])
        group = nodes.new("ShaderNodeGroup")
        group.node_tree = _gltf_output_group()
        group.location = (300, -400)
        links.new(sep.outputs["Red"], group.inputs["Occlusion"])
    if emissive is not None:
        if "emissive" in files:
            emis = image("emissive", -650)
            links.new(multiply(emis.outputs["Color"], emissive, -650), bsdf.inputs["Emission Color"])
        else:
            bsdf.inputs["Emission Color"].default_value = (*emissive, 1.0)
        bsdf.inputs["Emission Strength"].default_value = emissive_strength
    mat.diffuse_color = (*tint, 1.0)
    # Faces arrière masquées (glTF doubleSided: false) : les pièces sont des volumes fermés,
    # le GPU n'ombre pas leurs faces cachées.
    mat.use_backface_culling = True
    if isinstance(maps, str):
        spec = material_library().MATERIALS.get(maps, {})
        mat["sh_library"] = maps
        mat["sh_tile_m"] = float(spec.get("tile_m", 2.0))
    _MATERIALS[name] = mat
    return mat


def _uv_layer(mesh: bpy.types.Mesh):
    """Le 1er jeu d'UV (`UVMap`), créé au besoin, toujours en tête."""
    if len(mesh.uv_layers) == 0:
        mesh.uv_layers.new(name=UV_TEXTURE)
    return mesh.uv_layers[0]


def _mesh_arrays(obj: bpy.types.Object):
    """Sommets par boucle dans le repère « local × échelle + position ramenée dans la rotation
    de l'objet » : la texture suit la rotation de l'objet (props tournés par le kit) et deux
    objets de même rotation posés côte à côte raccordent (murs en plusieurs pavés). Normales par
    face dans le même repère, polygone de chaque boucle."""
    import numpy as np

    mesh = obj.data
    loc, rot, scale = obj.matrix_basis.decompose()
    offset = rot.inverted() @ loc
    s = np.array(scale, dtype=np.float64)
    nv, nl, np_ = len(mesh.vertices), len(mesh.loops), len(mesh.polygons)
    co = np.empty(nv * 3)
    mesh.vertices.foreach_get("co", co)
    co = co.reshape(-1, 3) * s + np.array(offset)
    loop_v = np.empty(nl, dtype=np.int64)
    mesh.loops.foreach_get("vertex_index", loop_v)
    normals = np.empty(np_ * 3)
    mesh.polygons.foreach_get("normal", normals)
    normals = normals.reshape(-1, 3) / np.where(s == 0, 1, s)
    normals /= np.maximum(np.linalg.norm(normals, axis=1, keepdims=True), 1e-12)
    totals = np.empty(np_, dtype=np.int64)
    mesh.polygons.foreach_get("loop_total", totals)
    loop_poly = np.repeat(np.arange(np_), totals)
    return co[loop_v], normals, loop_poly


def _tile_of(obj: bpy.types.Object) -> float:
    mats = [m for m in obj.data.materials if m is not None]
    return float(mats[0].get("sh_tile_m", 2.0)) if mats else 2.0


def uv_box(obj: bpy.types.Object, meters_per_tile: float | None = None, offset: tuple[float, float] = (0.0, 0.0)) -> None:
    """Projection boîte du 1er jeu d'UV, densité constante (tuile de `meters_per_tile` mètres ;
    None : la taille du catalogue notée sur le matériau, sinon 2 m, soit 512 px/m en 1K).

    Chaque face prend l'axe dominant de sa normale ; les murs gardent V vers le haut (Z), et
    aucune face n'est vue en miroir (pochoirs lisibles). Marche sur `box` (biseauté ou non),
    `cylinder` et un maillage fusionné ; à appeler avant `join_by_material` (les UV suivent la
    fusion) ou après (repère de l'objet fusionné). `offset` (en tuiles) décale la texture :
    deux travées séparées par une nervure ne montrent plus le même motif au même endroit.
    """
    import numpy as np

    mpt = meters_per_tile or _tile_of(obj)
    p, normals, loop_poly = _mesh_arrays(obj)
    n = normals[loop_poly]
    axis = np.argmax(np.abs(n), axis=1)
    sign = np.sign(n[np.arange(len(n)), axis])
    u = np.where(axis == 0, sign * p[:, 1], np.where(axis == 1, -sign * p[:, 0], p[:, 0]))
    v = np.where(axis == 2, sign * p[:, 1], p[:, 2])
    layer = _uv_layer(obj.data)
    uv = np.stack([u, v], 1) / mpt + np.array(offset, dtype=np.float64)
    layer.data.foreach_set("uv", uv.ravel())


def trim_band(trim) -> tuple[float, float]:
    """(v0, v1) d'une bande de la trim sheet : nom de `library.TRIMS` ou couple explicite."""
    if isinstance(trim, str):
        return material_library().TRIMS[trim]
    return float(trim[0]), float(trim[1])


def uv_trim(obj: bpy.types.Object, trim, meters_per_tile: float = 2.0, pad_px: float = 2.0, res: int = 1024,
            stencil_start: int | None = None) -> None:
    """Pose une bande de trim sheet : U le long de la plus grande dimension de l'objet (à
    l'échelle : `meters_per_tile` mètres par répétition), V = la bande (`trim` : nom de
    `library.TRIMS` ou (v0, v1)), rognée de `pad_px` texels pour que le filtrage ne déborde pas
    sur la bande voisine.

    Faces le long de l'axe : chaque face montre toute la hauteur de la bande, haut de la bande
    vers le haut (Z) quand c'est possible, jamais en miroir. Cylindre (plus de 8 faces
    latérales) : la bande fait le tour (V = angle). Bouts : la bande, en projection plane.

    M10 : la bande « stencils » (pochoirs) est calée sur un groupe entier de glyphes
    (`stencil_group`, `_align_stencils`), choisi d'après le nom de l'objet ou par
    `stencil_start` (première case, 0 à 7 : « 07 », flèche, « A2 », « ! », « 12 », chevrons,
    « B4 », « 03 »).
    """
    import numpy as np

    v0, v1 = trim_band(trim)
    pad = pad_px / res
    v0, v1 = v0 + pad, v1 - pad
    p, normals, loop_poly = _mesh_arrays(obj)
    span = p.max(axis=0) - p.min(axis=0)
    ax = int(np.argmax(span))
    a = np.zeros(3)
    a[ax] = 1.0
    others = [i for i in range(3) if i != ax]
    along = normals @ a
    side = np.abs(along) <= 0.7
    zup = np.array([0.0, 0.0, 1.0]) if ax != 2 else np.array([0.0, 1.0, 0.0])
    uvs = np.zeros((len(p), 2))
    center = (p.max(axis=0) + p.min(axis=0)) / 2
    round_ = int(side.sum()) > 8
    if round_:
        # Origine des angles sur un sommet : la face qui ferme le tour finit exactement à v1.
        first = p[np.nonzero(side[loop_poly])[0][0]] - center
        ang0 = np.arctan2(first[others[1]], first[others[0]])
    for poly in range(len(normals)):
        idx = np.nonzero(loop_poly == poly)[0]
        if idx.size == 0:
            continue
        pts = p[idx]
        nrm = normals[poly]
        if not side[poly]:
            b, c = pts[:, others[0]], pts[:, others[1]]
            lo, hi = c.min(), c.max()
            uvs[idx, 0] = b / meters_per_tile
            uvs[idx, 1] = v0 + (c - lo) / max(hi - lo, 1e-9) * (v1 - v0)
            continue
        if round_:
            rel = pts - center
            ang = (np.arctan2(rel[:, others[1]], rel[:, others[0]]) - ang0 + 1e-6) % (2 * np.pi) - 1e-6
            if ang.max() - ang.min() > np.pi:
                ang = np.where(ang < np.pi, ang + 2 * np.pi, ang)
            uvs[idx, 0] = pts[:, ax] / meters_per_tile
            uvs[idx, 1] = v0 + np.clip(ang / (2 * np.pi), 0.0, 1.0) * (v1 - v0)
            continue
        up = np.cross(nrm, a)
        up /= max(np.linalg.norm(up), 1e-12)
        if up @ zup < -1e-6:
            up = -up
        right = np.cross(up, nrm)
        across = pts @ up
        lo, hi = across.min(), across.max()
        uvs[idx, 0] = (pts @ right) / meters_per_tile
        uvs[idx, 1] = v0 + (across - lo) / max(hi - lo, 1e-9) * (v1 - v0)
    if trim == "stencils" and not round_:
        _align_stencils(obj, uvs, side, loop_poly, stencil_start)
    layer = _uv_layer(obj.data)
    layer.data.foreach_set("uv", uvs.ravel())


# --- Pochoirs (M10) -------------------------------------------------------------------------------
# La bande `stencils` de la trim sheet (materials/trim.py) porte huit glyphes, un tous les 25 cm
# de texture : « 07 », flèche, « A2 », « ! », « 12 », chevrons, « B4 », « 03 ». Une étiquette posée
# au hasard le long de U en montrait des morceaux, ou « 03 » et « 07 » de part et d'autre de la
# couture de la tuile (retour 6 de M10). `uv_trim(obj, "stencils")` cale désormais chaque face sur
# un groupe entier de glyphes voisins, sans franchir la couture : `STENCIL_GROUPS`.

STENCIL_SLOT_M = 0.25  # largeur d'un glyphe et de ses marges, en mètres de texture (tuile de 2 m)
STENCIL_SLOTS = 8
# Marge qu'on peut rogner ou ajouter de chaque côté d'un groupe sans entamer un glyphe (le plus
# large, la flèche, laisse ≈ 5 cm de vide de chaque côté de sa case).
STENCIL_MARGIN_M = 0.04
# Premières cases des groupes « voulus », par nombre de glyphes : un code seul, un code et son
# pictogramme (« 07 → », « A2 ! », « 12 » » : jamais deux codes accolés comme « 03 07 »), un code,
# un pictogramme et un code…
STENCIL_GROUPS = {
    1: (0, 2, 4, 6, 7, 3),
    2: (0, 2, 4),
    3: (0, 2, 4),
}


def stencil_group(key: str, width_m: float, start: int | None = None) -> tuple[int, int]:
    """(première case, nombre de cases) du groupe de glyphes d'une étiquette de `width_m` mètres
    de texture : autant de glyphes que la largeur en contient (au plus les huit), groupe choisi
    par `start` ou, sinon, de façon stable d'après `key` (le nom de l'objet)."""
    import zlib

    k = int(min(STENCIL_SLOTS, max(1, round(width_m / STENCIL_SLOT_M))))
    starts = STENCIL_GROUPS.get(k, tuple(range(0, STENCIL_SLOTS - k + 1)))
    if start is not None:
        s = int(start) % STENCIL_SLOTS
        return (min(s, STENCIL_SLOTS - k), k)
    return (starts[zlib.crc32(key.encode("utf-8")) % len(starts)], k)


def _align_stencils(obj, uvs, side, loop_poly, start) -> None:
    """Recale U de chaque face latérale d'une étiquette `stencils` sur un groupe entier de
    glyphes (`stencil_group`), à l'échelle d'origine à ± `STENCIL_MARGIN_M` près : jamais un
    glyphe coupé, jamais la couture « 03 | 07 »."""
    import numpy as np

    faces = [np.nonzero(loop_poly == poly)[0] for poly in np.nonzero(side)[0]]
    faces = [idx for idx in faces if idx.size]
    if not faces:
        return
    tile_m = STENCIL_SLOT_M * STENCIL_SLOTS
    width = max(float(np.ptp(uvs[idx, 0])) for idx in faces) * tile_m
    s0, k = stencil_group(obj.name, width, start)
    target = float(np.clip(width, k * STENCIL_SLOT_M - 2 * STENCIL_MARGIN_M, k * STENCIL_SLOT_M + 2 * STENCIL_MARGIN_M))
    center = (s0 + k / 2) / STENCIL_SLOTS
    half = target / tile_m / 2
    for idx in faces:
        u = uvs[idx, 0]
        lo, span = float(u.min()), float(np.ptp(u))
        if span < 1e-9:
            uvs[idx, 0] = center
            continue
        uvs[idx, 0] = center - half + (u - lo) / span * 2 * half


def export_gltf(rel_path: str, objects: list[bpy.types.Object]) -> Path:
    """Comme `export_glb`, mais en glTF séparé (`.gltf` + `.bin` du même nom), tangentes
    comprises, images **non copiées** : chaque `images[].uri` est relative au `.gltf` et pointe
    vers la bibliothèque (`assets/textures/materials/`). Lève une erreur si une image est
    embarquée, copiée à côté du modèle, recomposée par l'exportateur ou introuvable."""
    import json as _json
    from urllib.parse import unquote

    path = ROOT / rel_path
    if path.suffix != ".gltf":
        raise ValueError(f"export_gltf attend un chemin en .gltf : {rel_path}")
    # Un collider renommé par Blender (`…_col.001`, nom pris deux fois) ne finit plus par `_col` :
    # le jeu le dessinerait comme un pavé sans matériau, et sans collision.
    import re

    renamed = sorted(o.name for o in objects if re.search(r"_col\.\d+$", o.name))
    if renamed:
        raise ValueError(f"export_gltf : colliders au nom en double ({len(renamed)}) : {renamed[:8]}")
    path.parent.mkdir(parents=True, exist_ok=True)
    before = set(path.parent.iterdir())
    bpy.ops.object.select_all(action="DESELECT")
    # Blender ne calcule pas d'espace tangent sur un n-gone (bouchons de `cylinder`) et
    # l'exportateur abandonne alors les tangentes de tout le maillage : triangulation des
    # seuls n-gones, le temps de l'export (modificateur appliqué par export_apply, puis retiré).
    triangulated = []
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
        if obj.type == "MESH" and any(len(p.vertices) > 4 for p in obj.data.polygons):
            mod = obj.modifiers.new("sh_triangulate_ngons", "TRIANGULATE")
            mod.min_vertices = 5
            triangulated.append((obj, mod))
    try:
        bpy.ops.export_scene.gltf(
            filepath=str(path),
            export_format="GLTF_SEPARATE",
            use_selection=True,
            export_extras=True,
            export_yup=True,
            export_apply=True,
            export_cameras=False,
            export_lights=False,
            export_materials="EXPORT",
            export_keep_originals=True,
            export_tangents=True,
        )
    finally:
        for obj, mod in triangulated:
            obj.modifiers.remove(mod)
    data = _json.loads(path.read_text(encoding="utf-8"))
    normal_mapped = {i for i, mat in enumerate(data.get("materials", [])) if "normalTexture" in mat}
    for mesh in data.get("meshes", []):
        for prim in mesh.get("primitives", []):
            if prim.get("material") in normal_mapped and "TANGENT" not in prim.get("attributes", {}):
                raise RuntimeError(f"{rel_path} : {mesh.get('name')} sans tangentes malgré une carte de normales")
    library = TEXTURES_DIR.resolve()
    for image in data.get("images", []):
        uri = image.get("uri")
        if not uri or "bufferView" in image:
            raise RuntimeError(f"{rel_path} : image embarquée ({image.get('name')}) au lieu d'un lien vers la bibliothèque")
        target = (path.parent / unquote(uri)).resolve()
        if not target.exists() or target.parent != library:
            raise RuntimeError(f"{rel_path} : image hors bibliothèque ou introuvable : {uri}")
    # L'occlusion et le métal-rugosité doivent citer la même image `_orm` (pas de recomposition).
    textures = data.get("textures", [])
    for mat in data.get("materials", []):
        occ = mat.get("occlusionTexture")
        mr = mat.get("pbrMetallicRoughness", {}).get("metallicRoughnessTexture")
        if occ and mr and textures[occ["index"]]["source"] != textures[mr["index"]]["source"]:
            raise RuntimeError(f"{rel_path} : ORM recomposé pour {mat.get('name')}")
        for key in ("baseColorTexture", "metallicRoughnessTexture"):
            info = mat.get("pbrMetallicRoughness", {}).get(key)
            if info and info.get("texCoord", 0) != 0:
                raise RuntimeError(f"{rel_path} : {key} de {mat.get('name')} sur TEXCOORD_{info['texCoord']}")
    stray = sorted(p.name for p in set(path.parent.iterdir()) - before if p.name not in (path.name, path.with_suffix(".bin").name))
    if stray:
        raise RuntimeError(f"{rel_path} : fichiers inattendus écrits à côté du modèle : {stray}")
    return path
