"""Outils communs aux scripts de secteurs (cellules, portails, apparitions) : AGENTS.md §6.2, §9.

Un secteur est décrit par des dictionnaires simples :
- `cells` : id → (x0, x1, y0, y1, hauteur sous plafond), intérieurs, en mètres (repère Blender) ;
- `portals` : (cellule a, cellule b, genre, x, y, axe) ; b peut être un point d'attache externe
  « @… » (amarrage) ; « axis » = mur traversé ("x" : mur perpendiculaire à X) ;
- `windows` : id → [(côté, position, largeur, hauteur, allège)] ;
- `spawns` : (id, cellule, x, y, z, cap en degrés autour de Z, 0 = +Y).

`export_sector` construit, déplie, fusionne, exporte chaque cellule en deux modèles glTF séparés
(Locale et Sectorielle, textures de la bibliothèque citées par chemin, M8), cuit les lightmaps
et écrit le descripteur RON. Le script du vaisseau du joueur (`ships/player_ship.py`,
antérieur) a encore sa propre copie de ces fonctions.

Aperçus (M8, lot C) : `preview_lit` (Cycles, une lampe par repère `LIGHT_` et les surfaces
émissives) et `preview_game` (ce que le jeu affiche en Bas et Moyen dans une cellule à
lightmap : couleur × (1 − métal) × lightmap × exposition, plus l'émissif).
"""

from __future__ import annotations

import contextlib
import io
import math
import os

import bpy

import common as c
import lightmap as lm
import ship_interior as kit

# Extension des modèles de cellule (glTF séparé depuis M8 : images partagées, une seule copie
# en mémoire vidéo quel que soit le nombre de cellules qui les citent).
MODEL_EXT = ".gltf"


def openings_for(cell: str, cells: dict, portals: list, windows: dict) -> list:
    """Ouvertures de la coque d'une cellule, déduites des portails (et ses fenêtres)."""
    x0, x1, y0, y1, h = cells[cell]
    out = []
    for a, b, kind, x, y, axis in portals:
        if cell not in (a, b):
            continue
        if kind == "Opening":
            # La section de l'ouverture est celle de la cellule « a » (la passerelle, pas le
            # hall dans lequel elle débouche), comme dans le descripteur (`portal_record`).
            ax0, ax1, ay0, ay1, ah = cells[a]
            width, height = (ax1 - ax0) if axis == "y" else (ay1 - ay0), ah
        else:
            width, height = kit.DOOR_W, kit.DOOR_H
        if axis == "y":
            side = "N" if abs(y - y1) < abs(y - y0) else "S"
            out.append((side, x, width, height))
        else:
            side = "E" if abs(x - x1) < abs(x - x0) else "W"
            out.append((side, y, width, height))
    out.extend(windows.get(cell, []))
    return out


def portal_record(cells: dict, a, b, kind, x, y, axis) -> dict:
    x0, x1, y0, y1, h = cells[a]
    if kind == "Opening":
        width = (x1 - x0) if axis == "y" else (y1 - y0)
        height = h
    else:
        width, height = kit.DOOR_W, kit.DOOR_H
    # Normale : de a vers b (vers l'extérieur de a pour un point d'attache « @… »).
    if b.startswith("@"):
        ac = ((x0 + x1) / 2, (y0 + y1) / 2)
        bc = (2 * x - ac[0], 2 * y - ac[1])
    else:
        bx0, bx1, by0, by1, _ = cells[b]
        bc = ((bx0 + bx1) / 2, (by0 + by1) / 2)
    if axis == "y":
        normal = (0.0, 1.0 if bc[1] > y else -1.0, 0.0)
    else:
        normal = (1.0 if bc[0] > x else -1.0, 0.0, 0.0)
    return {
        "id": f"{a}__{b.lstrip('@')}",
        "a": a,
        "b": b,
        "kind": kind,
        "center": c.to_bevy((x, y, height / 2)),
        "normal": c.to_bevy(normal),
        "width": round(width, 3),
        "height": round(height, 3),
    }


def fmt(v) -> str:
    return "(" + ", ".join(f"{x:.3f}" for x in v) + ")"


def sector_ron(meta: dict, cells: dict, portals: list, spawns: list, lightmaps: dict) -> str:
    """Descripteur RON. `meta` : id, kind, name_key, status, door_model, models, comment ;
    `model_ext` facultatif (défaut `MODEL_EXT`, glTF séparé)."""
    ext = meta.get("model_ext", MODEL_EXT)
    lines = [
        f"// Généré par {meta['script']} : NE PAS ÉDITER À LA MAIN.",
        f"// {meta['comment']}",
        "// Coordonnées dans le repère du jeu (Y-haut), en mètres.",
        "(",
        f'    id: "{meta["id"]}",',
        f"    kind: {meta['kind']},",
        f'    name_key: "{meta["name_key"]}",',
        f"    status: {meta['status']},",
        f'    door_model: "{meta["door_model"]}",',
        "    cells: [",
    ]
    for cell, (x0, x1, y0, y1, h) in cells.items():
        lo = c.to_bevy((x0 - kit.WALL, y1 + kit.WALL, -kit.WALL))
        hi = c.to_bevy((x1 + kit.WALL, y0 - kit.WALL, h + kit.WALL))
        model = f"{meta['models']}/{cell}"
        lines += [
            "        (",
            f'            id: "{cell}",',
            f'            model: "{model}{ext}",',
            f'            lod: "{model}_lod{ext}",',
            f"            min: {fmt(lo)},",
            f"            max: {fmt(hi)},",
        ]
        if cell in lightmaps:
            lmap = lightmaps[cell]
            lines.append(f'            lightmap: (image: "{lmap["image"]}", exposure: {lmap["exposure"]:.3f}),')
        lines.append("        ),")
    lines.append("    ],")
    lines.append("    portals: [")
    for p in portals:
        r = portal_record(cells, *p)
        lines += [
            "        (",
            f'            id: "{r["id"]}",',
            f'            a: "{r["a"]}",',
            f'            b: "{r["b"]}",',
            f"            kind: {r['kind']},",
            f"            center: {fmt(r['center'])},",
            f"            normal: {fmt(r['normal'])},",
            f"            width: {r['width']:.3f},",
            f"            height: {r['height']:.3f},",
            "        ),",
        ]
    lines.append("    ],")
    lines.append("    spawns: [")
    for sid, cell, x, y, z, heading in spawns:
        lines.append(
            f'        (id: "{sid}", cell: "{cell}", pos: {fmt(c.to_bevy((x, y, z)))}, heading_deg: {heading:.1f}),'
        )
    lines += ["    ],", ")", ""]
    return "\n".join(lines)


def _tris(objects) -> int:
    return sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in objects if o.type == "MESH")


def unique_collider_names(colliders) -> int:
    """Rend uniques les noms des colliders, en gardant le suffixe `_col` relu par le jeu.

    Le kit (`room_shell` : pilastres, montants de porte, colonnes, bordures) nomme tous les
    colliders d'un même genre `<préfixe>_<genre>_col` ; Blender renomme les doublons
    `…_col.001`, que le jeu ne reconnaît plus (`name.ends_with("_col")`) : il les dessinait
    comme des pavés blancs sans texture et sans collision. Renvoie le nombre de renommages."""
    import re

    pattern = re.compile(r"^(.*?)(_col)?(\.\d+)?$")
    fixed = 0
    originals = [o.name for o in colliders]
    for i, obj in enumerate(colliders):
        obj.name = f"__sh_tmp_col_{i}"
    used = {o.name for o in bpy.data.objects}
    for obj, name in zip(colliders, originals):
        base = pattern.match(name).group(1)
        new, k = f"{base}_col", 1
        while new in used:
            new = f"{base}{k}_col"
            k += 1
        obj.name = new
        used.add(new)
        fixed += new != name
    return fixed


def _check_gltf_names(rel_path: str) -> None:
    """Aucun nœud du modèle exporté ne doit porter un suffixe de doublon Blender (`.001`) : un
    `_col`, `LIGHT_`, `INTERACT_` ou `SOCKET_` ainsi suffixé ne serait plus lu comme tel."""
    import json
    import re

    data = json.loads((c.ROOT / rel_path).read_text(encoding="utf-8"))
    bad = [n["name"] for n in data.get("nodes", []) if re.search(r"\.\d{3}$", n.get("name", ""))]
    if bad:
        raise RuntimeError(f"{rel_path} : noms de nœuds suffixés par Blender : {bad[:6]}")


def _remove_stale(models_dir: str, cells: dict) -> list[str]:
    """Supprime les modèles d'avant M8 (`<cellule>.glb`, `<cellule>_lod.glb`) quand leur
    remplaçant glTF existe : le jeu ne doit plus les trouver."""
    removed = []
    folder = c.ROOT / "assets" / models_dir
    for cell in cells:
        for stem in (cell, f"{cell}_lod"):
            old, new = folder / f"{stem}.glb", folder / f"{stem}{MODEL_EXT}"
            if old.exists() and new.exists():
                old.unlink()
                removed.append(old.name)
    return removed


def export_sector(
    meta: dict,
    cells: dict,
    portals: list,
    spawns: list,
    built: dict,
    ron_path: str,
    blend_path: str,
    *,
    bake: bool = True,
) -> dict:
    """Déplie, fusionne, exporte chaque cellule, cuit les lightmaps, écrit le RON et le .blend.

    `built` : id → {"visual", "structure", "colliders", "marks", "pickups"} (listes d'objets).
    `bake` = False : pas de lightmap (itération rapide ; le descripteur n'en cite aucune).
    Renvoie {"stats": {cellule: {"tris", "lod_tris", "materials", "res", "merged", "lod"}},
    "lightmaps": {cellule: {"image", "exposure"}}}.
    """
    exports = c.collection("EXPORT")
    # Matrices à jour avant dépliage et fusion (les pavés orientés posent matrix_basis).
    bpy.context.view_layer.update()
    renamed = sum(unique_collider_names(p["colliders"]) for p in built.values())
    if renamed:
        print(f"colliders renommés (noms en double du kit) : {renamed}")
    resolutions = {cell: lm.resolution_for(p["visual"] + p["pickups"]) for cell, p in built.items()}
    bake_input = {}
    stats = {}
    quiet = io.StringIO()
    with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
        for cell, parts in built.items():
            lm.unwrap(parts["visual"] + parts["pickups"], resolutions[cell])
            merged = c.join_by_material(parts["visual"], cell, exports)
            full = merged + parts["colliders"] + parts["marks"] + parts["pickups"]
            c.export_gltf(f"assets/{meta['models']}/{cell}{MODEL_EXT}", full)
            lod = c.join_by_material(parts["structure"], f"{cell}_lod1", exports)
            c.export_gltf(f"assets/{meta['models']}/{cell}_lod{MODEL_EXT}", lod)
            for suffix in ("", "_lod"):
                _check_gltf_names(f"assets/{meta['models']}/{cell}{suffix}{MODEL_EXT}")
            for obj in merged + lod:
                obj.hide_set(True)
            stats[cell] = {
                "tris": _tris(merged + parts["pickups"]),
                "lod_tris": _tris(lod),
                "materials": sorted({o.data.materials[0].name for o in merged}),
                "res": resolutions[cell],
                "colliders": len(parts["colliders"]),
                "merged": merged,
                "lod": lod,
            }
            bake_input[cell] = {"bake": merged + parts["pickups"], "res": resolutions[cell], "marks": parts["marks"]}
    for cell, s in stats.items():
        print(
            f"cellule {cell} : {s['tris']} triangles, {len(s['materials'])} matériaux, LOD {s['lod_tris']} "
            f"triangles, {s['colliders']} colliders, lightmap {s['res']} px"
        )
        print(f"  matériaux : {', '.join(s['materials'])}")
    removed = _remove_stale(meta["models"], cells)
    if removed:
        print(f"anciens modèles supprimés : {', '.join(removed)}")
    lightmaps = lm.bake_cells(bake_input, meta["lightmaps"], c.collection("BAKE_LAMPS")) if bake else {}
    c.write_text(ron_path, sector_ron(meta, cells, portals, spawns, lightmaps))
    c.save_blend(blend_path)
    return {"stats": stats, "lightmaps": lightmaps}


# --- Aperçus -------------------------------------------------------------------------------------


def door_panels(portals: list, cells: dict, m: dict, coll, style: str = "station") -> list:
    """Panneaux de porte fermés aux portails de type porte, posés comme le jeu
    (`sector/doors.rs` : à la base du portail, l'avant du modèle vers la normale). Aperçus
    seulement : jamais exportés avec les cellules."""
    out = []
    for i, p in enumerate(portals):
        if p[2] != "Door":
            continue
        r = portal_record(cells, *p)
        nx, nz = r["normal"][0], r["normal"][2]
        # Bevy (x, −y Blender) → Blender : normale (nx, −nz).
        bx, by = nx, -nz
        facing = math.atan2(-bx, by)
        vis, col, _ = kit.door_panel(f"preview_door{i}", m, coll, style=style)
        for o in col:
            bpy.data.objects.remove(o, do_unlink=True)
        x, y = p[3], p[4]
        for o in vis:
            ox, oy = o.location.x, o.location.y
            cs, sn = math.cos(facing), math.sin(facing)
            o.location = (x + ox * cs - oy * sn, y + ox * sn + oy * cs, o.location.z)
            o.rotation_euler[2] += facing
        out += vis
    return out


def _hide_lenses() -> None:
    """Les lentilles des plafonniers (`sh_light*`) restent visibles mais n'éclairent pas : leurs
    lampes le font (comme au bake)."""
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH" and obj.data.materials and obj.data.materials[0] and obj.data.materials[0].name.startswith("sh_light"):
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = False


def preview_lit(shots: dict, marks: list, out_dir, prefix: str, samples: int = 32, res=(1280, 720), only=None) -> None:
    """Rendus Cycles « éclairés » (lampes des repères LIGHT_ + émissifs), sans lightmap.
    `shots` : nom → (œil, cible, focale)."""
    import preview as pv

    exposure = float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2"))
    pv.setup_cycles(samples, res[0], res[1], exposure=exposure)
    pv.world((0, 0, 0), 0.0)
    for lamp in lm.spawn_lamps(marks, c.collection("PREVIEW_LAMPS")):
        lamp.data.shadow_soft_size = 0.06
    _hide_lenses()
    for name, (eye, target, lens) in shots.items():
        if only and name not in only:
            continue
        pv.camera("cam", eye, target, lens=lens)
        pv.render(out_dir / f"{prefix}{name}.png")
        print(f"aperçu : {prefix}{name}.png")


def game_material(mat, image, scale: float, tag: str, exposure: float):
    """Copie d'un matériau qui rend ce que le jeu affiche dans une cellule à lightmap (Bas et
    Moyen, sans lumière dynamique) : émission = couleur × (1 − métal) × lightmap × échelle +
    émissif (sans l'exposition de la caméra, comme Bevy). Transparence gardée (vitrages)."""
    g = mat.copy()
    g.name = f"game_{tag}_{mat.name}"
    nodes, links = g.node_tree.nodes, g.node_tree.links
    bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
    out = next(n for n in nodes if n.type == "OUTPUT_MATERIAL")

    def source(name):
        s = bsdf.inputs[name]
        return s.links[0].from_socket if s.is_linked else None, s.default_value

    def feed(socket, src, value):
        if src is not None:
            links.new(src, socket)
        else:
            socket.default_value = value if not hasattr(value, "__len__") else tuple(value)[: len(socket.default_value)]

    base_src, base_val = source("Base Color")
    met_src, met_val = source("Metallic")
    em_src, em_val = source("Emission Color")
    alpha = bsdf.inputs["Alpha"].default_value
    strength = bsdf.inputs["Emission Strength"].default_value
    mul = nodes.new("ShaderNodeVectorMath")
    mul.operation = "MULTIPLY"
    feed(mul.inputs[0], base_src, base_val)
    if image is not None:
        uv = nodes.new("ShaderNodeUVMap")
        uv.uv_map = lm.UV_NAME
        tex = nodes.new("ShaderNodeTexImage")
        tex.image = image
        links.new(uv.outputs["UV"], tex.inputs["Vector"])
        links.new(tex.outputs["Color"], mul.inputs[1])
    else:
        mul.inputs[1].default_value = (1.0, 1.0, 1.0)
    inv = nodes.new("ShaderNodeMath")
    inv.operation = "SUBTRACT"
    inv.inputs[0].default_value = 1.0
    feed(inv.inputs[1], met_src, met_val)
    k = nodes.new("ShaderNodeMath")
    k.operation = "MULTIPLY"
    links.new(inv.outputs[0], k.inputs[0])
    k.inputs[1].default_value = scale
    diffuse = nodes.new("ShaderNodeVectorMath")
    diffuse.operation = "SCALE"
    links.new(mul.outputs[0], diffuse.inputs[0])
    links.new(k.outputs[0], next(s for s in diffuse.inputs if s.name == "Scale"))
    glow = nodes.new("ShaderNodeVectorMath")
    glow.operation = "SCALE"
    feed(glow.inputs[0], em_src, em_val)
    next(s for s in glow.inputs if s.name == "Scale").default_value = strength / (2.0**exposure)
    add = nodes.new("ShaderNodeVectorMath")
    add.operation = "ADD"
    links.new(diffuse.outputs[0], add.inputs[0])
    links.new(glow.outputs[0], add.inputs[1])
    emit = nodes.new("ShaderNodeEmission")
    emit.inputs["Strength"].default_value = 1.0
    links.new(add.outputs[0], emit.inputs["Color"])
    if alpha < 1.0:
        transp = nodes.new("ShaderNodeBsdfTransparent")
        mix = nodes.new("ShaderNodeMixShader")
        mix.inputs[0].default_value = alpha
        links.new(transp.outputs[0], mix.inputs[1])
        links.new(emit.outputs[0], mix.inputs[2])
        links.new(mix.outputs[0], out.inputs["Surface"])
    else:
        links.new(emit.outputs["Emission"], out.inputs["Surface"])
    return g


def preview_game(shots: dict, stats: dict, lightmaps: dict, out_dir, prefix: str, extra=(), ambient: float = 0.35,
                 samples: int = 16, res=(1280, 720), only=None) -> None:
    """Aperçus « comme dans le jeu » : cellules à lightmap en émission seule, sans lampe ni
    rebond ; `extra` (panneaux de porte, objets mobiles sans lightmap) : couleur × `ambient`."""
    import preview as pv

    exposure = float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2"))
    pv.setup_cycles(samples, res[0], res[1], exposure=exposure)
    bpy.context.scene.cycles.max_bounces = 0
    pv.world((0, 0, 0), 0.0)
    for obj in list(bpy.context.scene.objects):
        if obj.type == "LIGHT":
            obj.hide_render = True
    shown = set()
    for cid, s in stats.items():
        info = lightmaps.get(cid)
        image, scale = None, ambient
        if info is not None:
            image = bpy.data.images.load(str(c.ROOT / "assets" / info["image"]), check_existing=True)
            image.colorspace_settings.name = "sRGB"
            scale = info["exposure"] / lm.WATTS_TO_LUMENS
        for obj in s["merged"]:
            obj.hide_render = False
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = True
            mats = obj.data.materials
            for i, mat in enumerate(mats):
                mats[i] = game_material(mat, image, scale, cid, exposure)
            shown.add(obj)
    cache = {}
    for obj in extra:
        if obj.type != "MESH":
            continue
        mats = obj.data.materials
        for i, mat in enumerate(mats):
            if mat.name not in cache:
                cache[mat.name] = game_material(mat, None, ambient, "extra", exposure)
            mats[i] = cache[mat.name]
        obj.hide_render = False
        shown.add(obj)
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH" and obj not in shown:
            obj.hide_render = True
    for name, (eye, target, lens) in shots.items():
        if only and name not in only:
            continue
        pv.camera("cam", eye, target, lens=lens)
        pv.render(out_dir / f"{prefix}{name}.png")
        print(f"aperçu jeu : {prefix}{name}.png")
