"""Icônes d'inventaire des objets (lot F de M9) : rendues depuis les modèles exportés.

Pour chaque objet, le script recharge `assets/models/items/<id>.gltf` (ce que le jeu affichera,
textures comprises) et le rend en EEVEE, en arrière-plan, sur film transparent :

- **même cadrage pour tous** : caméra orthographique de trois-quarts plongeante (azimut 35° vers
  +X depuis l'avant +Y, site 38°), objet centré, sa plus grande dimension à l'écran couvrant
  `FILL` (80 %) du cadre (calcul sur les sommets projetés, pas sur la boîte englobante) ;
- **même éclairage studio**, mis à l'échelle de l'objet (positions ∝ taille, énergies ∝ taille²,
  donc même éclairement pour une puce de 5 cm et une caisse de 40 cm) : clé chaude au-dessus à
  gauche, contre-jour froid derrière (détache la silhouette sur l'interface sombre), débouchage
  faible, ambiante sombre ;
- rendu à 512 px puis réduit à 256 px (moyenne en linéaire, alpha prémultiplié) : bords nets et
  sans frange ; PNG RGBA 8 bits, alpha non prémultiplié.

Sorties : assets/ui/icons/items/<id>.png (256 × 256), planche de contrôle sur fond d'interface
(#0d1117) dans target/art-previews/m9-lotF/planche_icones.png (icônes à 256 px dans des cases,
puis à 64 et 32 px), rendus 512 px dans target/art-previews/m9-lotF/icons_512/.

    blender --background --factory-startup --python tools/blender/items/icons.py -- [--only datapad,medkit] [--samples 64]
"""

from __future__ import annotations

import math
import struct
import sys
import zlib
from pathlib import Path

import bpy
import numpy as np
from mathutils import Vector

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))

import common as c  # noqa: E402
from items import ITEMS  # noqa: E402

ICON_DIR = c.ROOT / "assets" / "ui" / "icons" / "items"
PREVIEW = c.ROOT / "target" / "art-previews" / "m9-lotF"
SIZE = 256
SUPER = 2
FILL = 0.80
AZIMUTH = 35.0
ELEVATION = 38.0
UI_BG = (13, 17, 23)  # #0d1117
SLOT_BG = (22, 28, 37)
SLOT_EDGE = (44, 54, 68)
# Énergies des lampes (W) pour un objet de 1 m (× taille²).
KEY, RIM, FILL_LIGHT = 120.0, 190.0, 30.0


# --- Scène ---------------------------------------------------------------------------------------


def _clean() -> None:
    c.reset_scene()
    for block in (bpy.data.lights, bpy.data.cameras, bpy.data.images, bpy.data.materials, bpy.data.meshes):
        for item in list(block):
            if item.users == 0:
                block.remove(item)


def _engine(scene) -> None:
    try:
        scene.render.engine = "BLENDER_EEVEE"
    except TypeError:
        scene.render.engine = "BLENDER_EEVEE_NEXT"


def _world(color, strength: float) -> None:
    world = bpy.data.worlds.get("icon_world") or bpy.data.worlds.new("icon_world")
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (*color, 1.0)
    bg.inputs["Strength"].default_value = strength
    bpy.context.scene.world = world


def _direction(az_deg: float, el_deg: float) -> Vector:
    """Direction de la cible vers un point : azimut 0 = devant (+Y), 90 = droite de l'objet (+X)."""
    a, e = math.radians(az_deg), math.radians(el_deg)
    return Vector((math.sin(a) * math.cos(e), math.cos(a) * math.cos(e), math.sin(e)))


def _light(name, kind, loc, target, energy, color, size):
    data = bpy.data.lights.new(name, kind)
    data.energy = energy
    data.color = color
    if kind == "AREA":
        data.shape = "DISK"
        data.size = size
    obj = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(obj)
    obj.location = loc
    obj.rotation_euler = (Vector(target) - Vector(loc)).to_track_quat("-Z", "Y").to_euler()
    return obj


def _import(item: str) -> list:
    path = c.ROOT / "assets" / "models" / "items" / f"{item}.gltf"
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    return [o for o in bpy.data.objects if o not in before and o.type == "MESH"]


def _world_points(objs) -> np.ndarray:
    dg = bpy.context.evaluated_depsgraph_get()
    pts = []
    for obj in objs:
        ev = obj.evaluated_get(dg)
        mesh = ev.to_mesh()
        co = np.empty(len(mesh.vertices) * 3)
        mesh.vertices.foreach_get("co", co)
        co = co.reshape(-1, 3)
        mw = np.array(obj.matrix_world)
        pts.append(co @ mw[:3, :3].T + mw[:3, 3])
        ev.to_mesh_clear()
    return np.concatenate(pts)


def setup_render(samples: int) -> None:
    scene = bpy.context.scene
    _engine(scene)
    scene.render.film_transparent = True
    scene.render.resolution_x = scene.render.resolution_y = SIZE * SUPER
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.image_settings.color_depth = "8"
    scene.view_settings.view_transform = "AgX"
    try:
        scene.view_settings.look = "AgX - Medium High Contrast"
    except TypeError:
        scene.view_settings.look = "None"
    scene.view_settings.exposure = 0.0
    try:
        scene.eevee.taa_render_samples = samples
    except AttributeError:
        pass


def frame_and_light(objs) -> None:
    """Caméra orthographique et éclairage studio, à l'échelle de l'objet."""
    scene = bpy.context.scene
    pts = _world_points(objs)
    center = Vector(((pts.min(axis=0) + pts.max(axis=0)) / 2).tolist())
    size = float(np.linalg.norm(pts.max(axis=0) - pts.min(axis=0)))
    d = _direction(AZIMUTH, ELEVATION)
    rot = (-d).to_track_quat("-Z", "Y")
    right = rot @ Vector((1, 0, 0))
    up = rot @ Vector((0, 1, 0))
    pr = pts @ np.array(right)
    pu = pts @ np.array(up)
    extent = max(np.ptp(pr), np.ptp(pu))
    cr, cu = (pr.min() + pr.max()) / 2, (pu.min() + pu.max()) / 2
    depth = float((pts @ np.array(d)).max()) + size * 2 + 0.1
    cam_data = bpy.data.cameras.new("icon_cam")
    cam_data.type = "ORTHO"
    cam_data.ortho_scale = extent / FILL
    cam_data.clip_start = 0.001
    cam_data.clip_end = depth + size * 4 + 1.0
    cam = bpy.data.objects.new("icon_cam", cam_data)
    scene.collection.objects.link(cam)
    cam.location = right * cr + up * cu + d * depth
    cam.rotation_euler = rot.to_euler()
    scene.camera = cam
    s = size
    _world((0.045, 0.05, 0.06), 1.0)
    _light("key", "AREA", center + _direction(-25, 52) * 2.6 * s, center, KEY * s * s, (1.0, 0.90, 0.78), 2.0 * s)
    _light("rim", "AREA", center + _direction(195, 30) * 2.4 * s, center, RIM * s * s, (0.62, 0.78, 1.0), 0.9 * s)
    _light("fill", "AREA", center + _direction(95, 8) * 2.8 * s, center, FILL_LIGHT * s * s, (0.92, 0.95, 1.0), 2.0 * s)


# --- Images --------------------------------------------------------------------------------------


def _to_linear(c_):
    return np.where(c_ <= 0.04045, c_ / 12.92, ((c_ + 0.055) / 1.055) ** 2.4)


def _to_srgb(c_):
    c_ = np.clip(c_, 0.0, 1.0)
    return np.where(c_ <= 0.0031308, c_ * 12.92, 1.055 * np.power(c_, 1.0 / 2.4) - 0.055)


def load_rgba(path: Path) -> np.ndarray:
    """Image RGBA (ligne 0 en haut), couleurs sRGB, alpha non prémultiplié, flottants 0-1."""
    img = bpy.data.images.load(str(path), check_existing=False)
    w, h = img.size
    px = np.empty(w * h * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    bpy.data.images.remove(img)
    return px.reshape(h, w, 4)[::-1].copy()


def downsample(rgba: np.ndarray, factor: int) -> np.ndarray:
    """Réduction en linéaire, alpha prémultiplié (pas de frange sombre ni claire)."""
    h, w = rgba.shape[:2]
    lin = _to_linear(rgba[..., :3]) * rgba[..., 3:4]
    a = rgba[..., 3:4]
    lin = lin.reshape(h // factor, factor, w // factor, factor, 3).mean(axis=(1, 3))
    a = a.reshape(h // factor, factor, w // factor, factor, 1).mean(axis=(1, 3))
    col = np.where(a > 1e-6, lin / np.maximum(a, 1e-6), 0.0)
    return np.concatenate([_to_srgb(col), a], -1)


def write_png(path: Path, img: np.ndarray) -> None:
    """PNG 8 bits RGB ou RGBA (ligne 0 en haut), filtre « Up » : octets reproductibles."""
    data = (np.clip(img, 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)
    h, w, ch = data.shape
    raw = data.reshape(h, w * ch).astype(np.int16)
    up = np.zeros_like(raw)
    up[1:] = raw[:-1]
    rows = np.empty((h, w * ch + 1), dtype=np.uint8)
    rows[:, 0] = 2
    rows[:, 1:] = ((raw - up) % 256).astype(np.uint8)

    def chunk(tag: bytes, body: bytes) -> bytes:
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", w, h, 8, 6 if ch == 4 else 2, 0, 0, 0)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows.tobytes(), 9))
                     + chunk(b"IEND", b""))


def render_icon(item: str, samples: int) -> Path:
    _clean()
    setup_render(samples)
    objs = _import(item)
    frame_and_light(objs)
    big = PREVIEW / "icons_512" / f"{item}.png"
    big.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.scene.render.filepath = str(big)
    bpy.ops.render.render(write_still=True)
    icon = downsample(load_rgba(big), SUPER)
    out = ICON_DIR / f"{item}.png"
    write_png(out, icon)
    cover = icon[..., 3] > 0.5
    ys, xs = np.nonzero(cover)
    frac = (max(np.ptp(xs), np.ptp(ys)) + 1) / SIZE if len(xs) else 0.0
    print(f"icône {item} : {out.relative_to(c.ROOT)} ({SIZE} px, objet sur {frac:.0%} du cadre)")
    return out


# --- Planche -------------------------------------------------------------------------------------


def _over(dst_lin: np.ndarray, icon: np.ndarray, x: int, y: int) -> None:
    """Compose une icône (sRGB, alpha droit) sur un fond linéaire, à (x, y) = coin haut gauche."""
    h, w = icon.shape[:2]
    a = icon[..., 3:4]
    region = dst_lin[y:y + h, x:x + w]
    dst_lin[y:y + h, x:x + w] = region * (1 - a) + _to_linear(icon[..., :3]) * a


def _slot(dst_lin: np.ndarray, x: int, y: int, size: int, border: int = 2) -> None:
    edge = _to_linear(np.array(SLOT_EDGE) / 255.0)
    fill = _to_linear(np.array(SLOT_BG) / 255.0)
    dst_lin[y:y + size, x:x + size] = edge
    dst_lin[y + border:y + size - border, x + border:x + size - border] = fill


def planche(items: list[str]) -> Path:
    pad, cols = 24, 3
    rows = math.ceil(len(items) / cols)
    small = [64, 32]
    strip_h = sum(s + 2 * 8 for s in small) + pad
    width = pad + cols * (SIZE + pad)
    height = pad + rows * (SIZE + pad) + strip_h
    canvas = np.empty((height, width, 3), dtype=np.float64)
    canvas[:] = _to_linear(np.array(UI_BG) / 255.0)
    icons = {}
    for i, item in enumerate(items):
        path = ICON_DIR / f"{item}.png"
        if not path.exists():
            continue
        icons[item] = load_rgba(path)
        r, col = divmod(i, cols)
        x, y = pad + col * (SIZE + pad), pad + r * (SIZE + pad)
        _slot(canvas, x, y, SIZE)
        _over(canvas, icons[item], x, y)
    y = pad + rows * (SIZE + pad)
    for s in small:
        x = pad
        for item in items:
            if item not in icons:
                continue
            f = SIZE // s
            ic = downsample(icons[item], f)
            _slot(canvas, x, y, s + 8, border=1)
            _over(canvas, ic, x + 4, y + 4)
            x += s + 8 + 10
        y += s + 16
    out = PREVIEW / "planche_icones.png"
    write_png(out, _to_srgb(canvas))
    print(f"planche : {out.relative_to(c.ROOT)} ({width} × {height})")
    return out


def main(argv: list[str]) -> None:
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    samples = int(argv[argv.index("--samples") + 1]) if "--samples" in argv else 64
    for item in ITEMS:
        if only and item not in only:
            continue
        render_icon(item, samples)
    planche(ITEMS)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
