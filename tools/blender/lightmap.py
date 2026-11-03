"""Éclairage précalculé (lightmaps) des intérieurs : dépliage, bake Cycles, export PNG.

Le meilleur levier de performance sur GPU intégré (AGENTS.md §9.3, §10) : l'éclairage diffus
des cellules (direct, rebonds, ombres douces) est calculé une fois par Cycles et lu par Bevy
(`Lightmap`). En jeu, les lumières ponctuelles n'ajoutent plus que le spéculaire et l'éclairage
des objets mobiles (portes), sans carte d'ombre.

Étapes, appelées par le script d'un secteur :
1. `unwrap(objets)` : 2e jeu d'UV (`lightmap`, TEXCOORD_1 en glTF) commun à tous les objets
   d'une cellule, avant leur fusion par matériau : la version Locale et la version Sectorielle
   d'une cellule partagent ainsi la même lightmap ;
2. `bake_cells(...)` : une vraie lampe Blender par repère `LIGHT_`, puis un bake diffus par
   cellule, avec toutes les cellules dans la scène (les voisines occultent et renvoient la
   lumière) ;
3. chaque lightmap est écrite en PNG sRGB 8 bits, normalisée par un facteur d'échelle :
   l'exposition à donner au matériau dans le jeu (`lightmap_exposure`), écrite dans le
   descripteur du secteur. Le sRGB garde la précision dans les ombres, le GPU le décode
   gratuitement, et le PNG est léger (pas d'EXR 32 bits, non filtrable partout).

Unités : la puissance d'une lampe Blender (W) vaut lumens / 683, comme l'exporteur glTF ; une
valeur de bake (W/m²/sr) vaut donc 683 nits dans Bevy.
"""

from __future__ import annotations

import math
import os
import time

import bpy
import numpy as np

import common as c

WATTS_TO_LUMENS = 683.0
# Densité visée (texels par mètre) et bornes de résolution d'une lightmap.
TEXELS_PER_METER = 20.0
MIN_RES, MAX_RES = 256, 1024
# Marge autour des îlots UV et débord du bake, en texels.
PADDING = 3
# Échantillons Cycles (SH_BAKE_SAMPLES pour itérer vite, au prix du bruit).
SAMPLES = int(os.environ.get("SH_BAKE_SAMPLES", "512"))
# Part des texels sous le facteur d'échelle (le reste, sous les plafonniers, sature).
SCALE_PERCENTILE = 99.7
UV_NAME = "lightmap"


def _meshes(objects):
    return [o for o in objects if o.type == "MESH"]


def surface_area(objects) -> float:
    total = 0.0
    for obj in _meshes(objects):
        scale = obj.matrix_world.to_scale()
        factor = abs(scale.x * scale.y * scale.z) ** (2.0 / 3.0)
        total += sum(p.area for p in obj.data.polygons) * factor
    return total


def resolution_for(objects) -> int:
    """Résolution (puissance de 2) qui tient la densité visée, avec 70 % de remplissage."""
    texels = surface_area(objects) * TEXELS_PER_METER**2 / 0.7
    res = 2 ** math.ceil(math.log2(max(1.0, math.sqrt(texels))))
    return int(min(MAX_RES, max(MIN_RES, res)))


def unwrap(objects, res: int) -> None:
    """Déplie un 2e jeu d'UV commun à `objects`, empaqueté dans un seul carré."""
    meshes = _meshes(objects)
    for obj in meshes:
        uvs = obj.data.uv_layers
        # TEXCOORD_0 doit exister pour que la lightmap soit TEXCOORD_1 (UV_1 dans Bevy).
        if len(uvs) == 0:
            uvs.new(name="UVMap")
        layer = uvs.get(UV_NAME) or uvs.new(name=UV_NAME)
        uvs.active = layer
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    margin = 2.0 * PADDING / res
    bpy.ops.uv.smart_project(
        angle_limit=math.radians(66.0), island_margin=margin, area_weight=0.0, scale_to_bounds=False
    )
    bpy.ops.uv.pack_islands(rotate=True, margin_method="FRACTION", margin=margin)
    bpy.ops.object.mode_set(mode="OBJECT")
    for obj in meshes:
        # Le premier jeu reste le jeu actif au rendu (textures éventuelles du matériau).
        obj.data.uv_layers.active = obj.data.uv_layers[0]


def _setup_cycles(scene) -> None:
    try:
        scene.render.engine = "CYCLES"
    except TypeError as err:
        raise RuntimeError(f"Cycles indisponible : {err}") from err
    scene.cycles.device = "CPU"
    scene.cycles.samples = SAMPLES
    scene.cycles.use_denoising = False  # le bake n'est pas débruité : voir _smooth
    # Les chemins spéculaires des métaux font des « lucioles » : on les écrête et on borne les
    # rebonds (la lumière diffuse indirecte, l'essentiel, reste juste).
    scene.cycles.sample_clamp_indirect = 1.0
    scene.cycles.caustics_reflective = False
    scene.cycles.caustics_refractive = False
    scene.cycles.blur_glossy = 1.0
    scene.cycles.max_bounces = 6
    scene.cycles.diffuse_bounces = 4
    scene.cycles.glossy_bounces = 1
    bake = scene.render.bake
    bake.margin = PADDING
    bake.use_clear = True
    # Intérieur clos : aucun éclairage de fond.
    world = scene.world or bpy.data.worlds.new("World")
    scene.world = world
    world.use_nodes = True
    bg = next((n for n in world.node_tree.nodes if n.type == "BACKGROUND"), None)
    if bg is not None:
        bg.inputs["Strength"].default_value = 0.0


def spawn_lamps(marks, coll) -> list[bpy.types.Object]:
    """Une lampe ponctuelle Blender par repère `LIGHT_` (mêmes réglages que dans le jeu)."""
    lamps = []
    for mark in marks:
        if not mark.name.startswith("LIGHT_"):
            continue
        data = bpy.data.lights.new(f"bake_{mark.name}", type="POINT")
        data.energy = float(mark.get("lumens", 20000.0)) / WATTS_TO_LUMENS
        data.color = tuple(mark.get("color", (1.0, 1.0, 1.0)))[:3]
        data.shadow_soft_size = 0.1
        lamp = bpy.data.objects.new(f"bake_{mark.name}", data)
        lamp.matrix_world = mark.matrix_world.copy()
        coll.objects.link(lamp)
        lamps.append(lamp)
    return lamps


def _bake_image(name: str, res: int) -> bpy.types.Image:
    old = bpy.data.images.get(name)
    if old is not None:
        bpy.data.images.remove(old)
    return bpy.data.images.new(name, res, res, alpha=True, float_buffer=True)


def _target_nodes(objects, image):
    """Nœud image actif dans chaque matériau des objets : la cible du bake."""
    nodes = []
    for mat in {m for o in _meshes(objects) for m in o.data.materials if m}:
        node = mat.node_tree.nodes.new("ShaderNodeTexImage")
        node.image = image
        node.interpolation = "Linear"
        mat.node_tree.nodes.active = node
        nodes.append((mat, node))
    return nodes


def _smooth(rgb, mask):
    """Flou gaussien 3 × 3 restreint aux texels cuits : atténue le bruit résiduel du bake.

    Son rayon (1 texel) reste dans le débord de chaque îlot (PADDING texels) : il ne mélange
    jamais deux surfaces voisines dans l'atlas.
    """
    weights = np.zeros(rgb.shape[:2], dtype=np.float32)
    total = np.zeros_like(rgb)
    w = mask.astype(np.float32)
    for dy, dx, k in [(y, x, (2 - abs(y)) * (2 - abs(x))) for y in (-1, 0, 1) for x in (-1, 0, 1)]:
        total += k * np.roll(rgb * w[..., None], (dy, dx), axis=(0, 1))
        weights += k * np.roll(w, (dy, dx), axis=(0, 1))
    return np.where(mask[..., None], total / np.maximum(weights, 1e-6)[..., None], rgb)


def _write_png(image, path) -> float:
    """Normalise, encode en sRGB 8 bits et écrit ; renvoie le facteur d'échelle (W/m²/sr)."""
    res = image.size[0]
    px = np.empty(res * res * 4, dtype=np.float32)
    image.pixels.foreach_get(px)
    px = px.reshape(res, res, 4)
    rgb = _smooth(np.maximum(px[..., :3], 0.0), px[..., 3] > 0.0)
    lit = rgb.max(axis=2)
    covered = lit[px[..., 3] > 0.0]
    scale = float(np.percentile(covered, SCALE_PERCENTILE)) if covered.size else 1.0
    scale = max(scale, 1e-6)
    lin = np.clip(rgb / scale, 0.0, 1.0)
    srgb = np.where(lin <= 0.0031308, lin * 12.92, 1.055 * np.power(lin, 1.0 / 2.4) - 0.055)

    out = bpy.data.images.new(f"{image.name}_png", res, res, alpha=False)
    out.colorspace_settings.name = "Non-Color"  # octets écrits tels quels : déjà en sRGB
    data = np.ones((res, res, 4), dtype=np.float32)
    data[..., :3] = srgb
    out.pixels.foreach_set(data.ravel())
    path.parent.mkdir(parents=True, exist_ok=True)
    out.filepath_raw = str(path)
    out.file_format = "PNG"
    out.save()
    bpy.data.images.remove(out)
    return scale


def bake_cells(cells: dict, out_dir: str, lamps_coll) -> dict:
    """Bake chaque cellule. `cells` : id → {"bake": objets, "res": int, "marks": repères}.

    Seuls les objets « bake » des cellules sont rendus pendant le bake ; renvoie, par cellule,
    {"image": chemin relatif à assets/, "exposure": exposition du matériau dans le jeu}.
    """
    scene = bpy.context.scene
    _setup_cycles(scene)
    bakeable = {o for cell in cells.values() for o in _meshes(cell["bake"])}
    hidden = []
    for obj in scene.objects:
        if obj.type == "MESH" and obj not in bakeable and not obj.hide_render:
            obj.hide_render = True
            hidden.append(obj)
    # Les plafonniers sont remplacés par leurs lampes (comme dans le jeu) : leur surface
    # émissive ne doit pas éclairer une seconde fois.
    emitters = []
    for obj in bakeable:
        if any(m and m.name == "sh_light" for m in obj.data.materials):
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = False
            obj.visible_volume_scatter = False
            emitters.append(obj)
    lamps = spawn_lamps([m for cell in cells.values() for m in cell["marks"]], lamps_coll)

    results = {}
    for cell_id, cell in cells.items():
        start = time.monotonic()
        objs = _meshes(cell["bake"])
        image = _bake_image(f"lightmap_{cell_id}", cell["res"])
        nodes = _target_nodes(objs, image)
        bpy.ops.object.select_all(action="DESELECT")
        for obj in objs:
            obj.hide_set(False)
            obj.select_set(True)
        bpy.context.view_layer.objects.active = objs[0]
        bpy.ops.object.bake(
            type="DIFFUSE",
            pass_filter={"DIRECT", "INDIRECT"},
            margin=PADDING,
            margin_type="EXTEND",
            use_clear=True,
            uv_layer=UV_NAME,
        )
        for mat, node in nodes:
            mat.node_tree.nodes.remove(node)
        rel = f"{out_dir}/{cell_id}.png"
        scale = _write_png(image, c.ROOT / "assets" / rel)
        bpy.data.images.remove(image)
        results[cell_id] = {"image": rel, "exposure": round(scale * WATTS_TO_LUMENS, 3)}
        print(
            f"lightmap {cell_id} : {cell['res']} px, exposition {results[cell_id]['exposure']}, "
            f"{SAMPLES} échantillons, {time.monotonic() - start:.0f} s"
        )

    for obj in hidden:
        obj.hide_render = False
    for obj in emitters:
        obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = True
        obj.visible_volume_scatter = True
    for lamp in lamps:
        data = lamp.data
        bpy.data.objects.remove(lamp, do_unlink=True)
        bpy.data.lights.remove(data)
    return results
