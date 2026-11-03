"""Outils communs aux scripts de secteurs (cellules, portails, apparitions) : AGENTS.md §6.2, §9.

Un secteur est décrit par des dictionnaires simples :
- `cells` : id → (x0, x1, y0, y1, hauteur sous plafond), intérieurs, en mètres (repère Blender) ;
- `portals` : (cellule a, cellule b, genre, x, y, axe) ; b peut être un point d'attache externe
  « @… » (amarrage) ; « axis » = mur traversé ("x" : mur perpendiculaire à X) ;
- `windows` : id → [(côté, position, largeur, hauteur, allège)] ;
- `spawns` : (id, cellule, x, y, z, cap en degrés autour de Z, 0 = +Y).

`export_sector` construit, déplie, fusionne, exporte chaque cellule en deux modèles (Locale et
Sectorielle), cuit les lightmaps et écrit le descripteur RON. Le script du vaisseau du joueur
(`ships/player_ship.py`, antérieur) a encore sa propre copie de ces fonctions.
"""

from __future__ import annotations

import contextlib
import io

import common as c
import lightmap as lm
import ship_interior as kit


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
    """Descripteur RON. `meta` : id, kind, name_key, status, door_model, models, comment."""
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
        model = f"{meta['models']}/{cell}.glb"
        lines += [
            "        (",
            f'            id: "{cell}",',
            f'            model: "{model}",',
            f'            lod: "{model[:-4]}_lod.glb",',
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


def export_sector(meta: dict, cells: dict, portals: list, spawns: list, built: dict, ron_path: str, blend_path: str) -> None:
    """Déplie, fusionne, exporte chaque cellule, cuit les lightmaps, écrit le RON et le .blend.

    `built` : id → {"visual", "structure", "colliders", "marks", "pickups"} (listes d'objets).
    """
    exports = c.collection("EXPORT")
    resolutions = {cell: lm.resolution_for(p["visual"] + p["pickups"]) for cell, p in built.items()}
    bake = {}
    quiet = io.StringIO()
    with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
        for cell, parts in built.items():
            lm.unwrap(parts["visual"] + parts["pickups"], resolutions[cell])
            merged = c.join_by_material(parts["visual"], cell, exports)
            full = merged + parts["colliders"] + parts["marks"] + parts["pickups"]
            c.export_glb(f"assets/{meta['models']}/{cell}.glb", full)
            lod = c.join_by_material(parts["structure"], f"{cell}_lod1", exports)
            c.export_glb(f"assets/{meta['models']}/{cell}_lod.glb", lod)
            for obj in merged + lod:
                obj.hide_set(True)
            bake[cell] = {"bake": merged + parts["pickups"], "res": resolutions[cell], "marks": parts["marks"]}
    lightmaps = lm.bake_cells(bake, meta["lightmaps"], c.collection("BAKE_LAMPS"))
    c.write_text(ron_path, sector_ron(meta, cells, portals, spawns, lightmaps))
    c.save_blend(blend_path)
