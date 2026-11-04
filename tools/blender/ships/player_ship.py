"""Le vaisseau du joueur (proposition P12) : petit marchand de taille corvette, ≈ 60 m.

Assemblé avec le kit « futur usé » (`tools/blender/kit/ship_interior.py`, M8 lot A) et habillé
pièce par pièce (M8 lot B, direction : docs/refonte-visuelle.md §2). Un seul pont, une épine
dorsale de couloirs, des salles de part et d'autre :

    cockpit (avant, plate-forme surélevée, verrière) : lumière froide, blanche et bleue
    couloir_avant ── quartiers (bâbord) / carre (tribord) : plus chaleureux, usés
    couloir_arriere ── sas (bâbord, massif) / soute (tribord, ambre)
    machines (arrière, ambre, réacteur)

Gameplay intact depuis M1 : mêmes cellules, portails, apparitions, repères `INTERACT_` /
`SOCKET_` aux mêmes positions, mêmes colliders des props ; l'habillage ajouté reste hors des
trajets de l'autopilote (`crates/sh_game/src/dev.rs` : walk_ship, port_loop, firefight,
tutorial) et n'a de collider que contre une paroi ou en hauteur.

Sorties :
- art/ships/player_ship.blend ;
- assets/models/ships/player_ship/<cellule>.gltf (+ .bin ; qualité Locale : tout, avec
  collisions) et <cellule>_lod.gltf (qualité Sectorielle : coque seule), glTF séparé, textures
  citées par chemin dans assets/textures/materials/ ;
- assets/textures/lightmaps/player_ship/<cellule>.png (éclairage précalculé, `lightmap.py`) ;
- assets/data/propositions/sectors/player_ship.ron (cellules, portails, apparitions), généré.

    blender --background --factory-startup --threads 8 --python tools/blender/ships/player_ship.py -- [options]

Bake final : 512 échantillons (défaut de `lightmap.py`, ≈ 1 h sur le poste Linux) ;
`SH_BAKE_SAMPLES=64` pour itérer (≈ 5 min, bruité). Avant le bake, les faces jamais vues sont
retirées et l'atlas est déplié avec des marges serrées (`prepare_lightmap`) : ≈ 25 à 40 texels/m.

Options : `--no-bake` (exporte sans cuire ni écrire le descripteur), `--no-export` (ni export ni
bake : aperçus seulement), `--lit` (aperçus éclairés par les lampes, Cycles), `--game` (aperçus
« comme dans le jeu » : couleur × lightmap + émissif, après un bake), `--only a,b` (prises de vue),
`--samples N`, `--res LxH`. Aperçus dans target/art-previews/lotB/.

Repères Blender : X = tribord, Y = avant, Z = haut. Le jeu reçoit X, Z, −Y (export glTF).
"""

from __future__ import annotations

import contextlib
import io
import math
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "kit"))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "materials"))

import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import common as c  # noqa: E402
import lightmap as lm  # noqa: E402
import ship_interior as kit  # noqa: E402

SECTOR = "player_ship"
MODELS = f"assets/models/ships/{SECTOR}"
LIGHTMAPS = f"textures/lightmaps/{SECTOR}"
DOOR_MODEL = "models/kit/ship_interior/door_sliding.gltf"
PREVIEWS = c.ROOT / "target" / "art-previews" / "lotB"
CORRIDOR_H = 2.6
PI = math.pi

# Cellules : id → (x0, x1, y0, y1, hauteur sous plafond). Intérieurs ; les cloisons (0,2 m)
# sont à l'extérieur de ces bornes, deux salles voisines laissent 0,4 m entre elles.
CELLS = {
    "machines": (-5.0, 5.0, -26.0, -14.4, 3.4),
    "couloir_arriere": (-1.2, 1.2, -14.0, 0.0, CORRIDOR_H),
    "sas": (-5.6, -1.6, -5.0, -1.0, CORRIDOR_H),
    "soute": (1.6, 10.0, -12.0, -2.0, 3.4),
    "couloir_avant": (-1.2, 1.2, 0.0, 14.0, CORRIDOR_H),
    "quartiers": (-8.4, -1.6, 2.0, 10.0, 2.8),
    "carre": (1.6, 8.4, 2.0, 10.0, 2.8),
    "cockpit": (-4.0, 4.0, 14.4, 22.0, 3.4),
}

# Portails : (cellule a, cellule b, genre, x, y, axe). « axis » = mur traversé ("x" : mur
# perpendiculaire à X, donc ouverture vers ±X ; "y" : vers ±Y).
PORTALS = [
    ("couloir_arriere", "machines", "Door", 0.0, -14.2, "y"),
    ("couloir_arriere", "soute", "Door", 1.4, -7.0, "x"),
    ("couloir_arriere", "sas", "Door", -1.4, -3.0, "x"),
    ("couloir_arriere", "couloir_avant", "Opening", 0.0, 0.0, "y"),
    ("couloir_avant", "quartiers", "Door", -1.4, 6.0, "x"),
    ("couloir_avant", "carre", "Door", 1.4, 6.0, "x"),
    ("couloir_avant", "cockpit", "Door", 0.0, 14.2, "y"),
    # Écoutille extérieure : point d'attache « @dock », verrouillé en vol, porte du sas à quai.
    ("sas", "@dock", "Door", -5.7, -3.0, "x"),
]

# Fenêtres : (côté, position, largeur, hauteur, allège) ; la verrière du cockpit, au-dessus
# des consoles de pilotage (plate-forme surélevée de 0,8 m).
WINDOWS = {
    "cockpit": [("N", 0.0, 6.4, 1.45, 1.75)],
}

SPAWNS = [
    # (id, cellule, x, y, z, cap en degrés autour de Z : 0 = vers l'avant +Y)
    ("sas", "sas", -3.8, -3.0, 0.0, -90.0),
    ("cockpit", "cockpit", 0.0, 15.2, 0.0, 0.0),
    ("soute", "soute", 5.8, -10.5, 0.0, 0.0),
]

# Lampes (repères LIGHT_ : lumière dynamique du jeu, lampe Blender au bake). Intensités calées
# sur la salle témoin validée (coursive 2 500 lm, atelier sodium 12 000 lm) : la lightmap est
# normalisée, mais son exposition est absolue dans le jeu.
COOL = [0.78, 0.88, 1.0]
COCKPIT_WHITE = [0.86, 0.93, 1.0]
WARM = [1.0, 0.8, 0.58]
SODIUM = [1.0, 0.55, 0.22]
GREEN = [0.2, 1.0, 0.35]


def palette() -> dict:
    """Matériaux du kit, plus des variantes propres au vaisseau (aucune texture nouvelle) :
    lentille chaude (`sh_light_warm`, masquée au bake comme tout `sh_light*`), bandeaux bleu
    (cockpit) et ambré doux (salles de vie), cuits ; tôle peinte crème des salles de vie, voyant
    vert, tapis des quartiers."""
    m = kit.palette()
    m["light_warm"] = c.material("sh_light_warm", (1.0, 0.9, 0.75), emission=(1.0, 0.78, 0.5), emission_strength=4.0)
    m["band_blue"] = c.textured_material("sh_band_blue", "light_band", emissive=(0.22, 0.5, 1.0), emissive_strength=2.6)
    m["band_hearth"] = c.textured_material("sh_band_hearth", "light_band", emissive=(1.0, 0.6, 0.3), emissive_strength=3.0)
    m["panel_warm"] = c.textured_material("sh_panel_warm", "panel_painted", tint=(0.96, 0.86, 0.72))
    m["beacon_green"] = c.material("sh_beacon_green", (0.04, 0.45, 0.1), emission=(0.12, 1.0, 0.3), emission_strength=6.0)
    m["rug"] = c.textured_material("sh_fabric_rug", "fabric", tint=(0.36, 0.2, 0.15))
    return m


# --- Briques -------------------------------------------------------------------------------------


def _uv(obj, uv="box", tile=None):
    """UV de texture (1er jeu) : "box", ("box", (du, dv)), ("strip", m), nom de bande de trim."""
    if uv is None:
        return obj
    if uv == "box":
        c.uv_box(obj, tile)
    elif isinstance(uv, tuple) and uv[0] == "box":
        c.uv_box(obj, tile, offset=uv[1])
    elif isinstance(uv, tuple) and uv[0] == "strip":
        c.uv_trim(obj, (0.0, 1.0), meters_per_tile=uv[1], pad_px=0.0)
    else:
        c.uv_trim(obj, uv, meters_per_tile=tile or 2.0)
    return obj


def _cuts(lo, hi, gaps):
    out, cur = [], lo
    for a, b in sorted(gaps):
        if b <= cur or a >= hi:
            continue
        if a - cur > 0.05:
            out.append((cur, a))
        cur = max(cur, b)
    if hi - cur > 0.05:
        out.append((cur, hi))
    return out


def place(objs, center, facing):
    """Pose des objets construits autour de l'origine (face utile vers +Y) : rotation `facing`
    autour de Z, puis translation."""
    cs, sn = math.cos(facing), math.sin(facing)
    for obj in objs:
        x, y = obj.location.x, obj.location.y
        obj.location = (center[0] + x * cs - y * sn, center[1] + x * sn + y * cs, center[2] + obj.location.z)
        obj.rotation_euler[2] += facing


class Cell:
    """Collecte des objets d'une cellule : visibles, structure (version simplifiée),
    collisions, repères, objets ramassables."""

    def __init__(self, cid: str, m: dict):
        self.id = cid
        self.m = m
        self.x0, self.x1, self.y0, self.y1, self.h = CELLS[cid]
        self.coll = c.collection(f"CELL_{cid}")
        self.visual, self.structure, self.colliders, self.marks, self.pickups = [], [], [], [], []
        self._n = 0

    def nm(self, tag: str) -> str:
        self._n += 1
        return f"{self.id}_{tag}{self._n}"

    def mat(self, key):
        return self.m[key] if isinstance(key, str) else key

    def add(self, result, structure=False):
        """Triplet du kit (visibles, colliders, repères) ; `structure` : aussi dans la version
        simplifiée de la cellule."""
        vis, col, marks = result
        self.visual += vis
        if structure:
            self.structure += vis
        self.colliders += col
        self.marks += marks
        return vis

    def shell(self, **kw):
        openings = openings_for(self.id)
        parts = kit.room_shell(self.id, self.x0, self.x1, self.y0, self.y1, 0.0, self.h, openings, self.m, self.coll, **kw)
        self.visual += parts["visual"]
        self.structure += parts["structure"]
        self.colliders += parts["colliders"]

    def obj(self, o, uv="box", tile=None, structure=False):
        _uv(o, uv, tile)
        self.visual.append(o)
        if structure:
            self.structure.append(o)
        return o

    def box(self, tag, size, center, mat, uv="box", bevel=0.0, tile=None, structure=False):
        return self.obj(c.box(self.nm(tag), size, center, self.mat(mat), self.coll, bevel=bevel), uv, tile, structure)

    def cyl(self, tag, radius, depth, center, mat, axis="Z", segments=12, uv="box", tile=None):
        o = c.cylinder(self.nm(tag), radius, depth, center, self.mat(mat), self.coll, axis=axis, segments=segments)
        return self.obj(o, uv, tile)

    def ori(self, tag, size, center, right, up, mat, uv="box", bevel=0.0, tile=None, structure=False):
        """Pavé dont les axes locaux sont (right, up, right × up) : la face +Z regarde vers
        right × up. Face vers +Y : right = (−1, 0, 0), up = (0, 0, 1)."""
        o = c.box(self.nm(tag), size, (0, 0, 0), self.mat(mat), self.coll, bevel=bevel)
        r, u = Vector(right).normalized(), Vector(up).normalized()
        n = r.cross(u).normalized()
        u = n.cross(r).normalized()
        o.matrix_basis = Matrix.Translation(Vector(center)) @ Matrix((r, u, n)).transposed().to_4x4()
        return self.obj(o, uv, tile)

    def col(self, tag, size, center):
        self.colliders.append(c.collider_box(f"{self.id}_{tag}", size, center, self.coll))

    def lamp(self, tag, x, y, lumens, rng, color, lens="light", facing=0.0, drop=0.0, shadow=False):
        props = {"lumens": lumens, "range": rng, "color": list(color), "shadow_candidate": int(shadow)}
        self.add(kit.ceiling_light(f"{self.id}_{tag}", (x, y, self.h), facing, self.m, self.coll, props, lens=lens, drop=drop))

    def pickup(self, result):
        """Objet ramassable : ses pièces fusionnées en un seul objet (plusieurs matériaux),
        enfant de son repère INTERACT_ : le jeu le retire d'un bloc quand on le ramasse."""
        vis, _, marks = result
        root = marks[0]
        bpy.context.view_layer.update()
        bpy.ops.object.select_all(action="DESELECT")
        for o in vis:
            o.select_set(True)
        bpy.context.view_layer.objects.active = vis[0]
        bpy.ops.object.join()
        joined = bpy.context.view_layer.objects.active
        joined.name = f"{root.name[len('INTERACT_'):]}_mesh"
        bpy.context.view_layer.update()
        joined.parent = root
        joined.matrix_parent_inverse = root.matrix_world.inverted()
        self.pickups.append(joined)
        self.marks += marks

    # Repère des murs : point (s le long du mur, d vers l'intérieur, z), angle d'un prop dont
    # la face +Y regarde la pièce depuis ce mur.
    def wall(self, side, s, d=0.0, z=0.0):
        if side == "N":
            return (s, self.y1 - d, z)
        if side == "S":
            return (s, self.y0 + d, z)
        if side == "E":
            return (self.x1 - d, s, z)
        return (self.x0 + d, s, z)

    @staticmethod
    def facing(side):
        return {"N": PI, "S": 0.0, "E": PI / 2, "W": -PI / 2}[side]

    @staticmethod
    def inward(side):
        return {"N": (0, -1, 0), "S": (0, 1, 0), "E": (-1, 0, 0), "W": (1, 0, 0)}[side]


def openings_for(cell: str) -> list[tuple]:
    """Ouvertures de la coque d'une cellule, déduites des portails."""
    x0, x1, y0, y1, h = CELLS[cell]
    out = []
    for a, b, kind, x, y, axis in PORTALS:
        if cell not in (a, b):
            continue
        if kind == "Opening":
            width, height = (x1 - x0) if axis == "y" else (y1 - y0), h
        else:
            width, height = kit.DOOR_W, kit.DOOR_H
        if axis == "y":
            side = "N" if abs(y - y1) < abs(y - y0) else "S"
            out.append((side, x, width, height))
        else:
            side = "E" if abs(x - x1) < abs(x - x0) else "W"
            out.append((side, y, width, height))
    out.extend(WINDOWS.get(cell, []))
    return out


# --- Pièces d'habillage propres au vaisseau -----------------------------------------------------


def chamfer_bands(k: Cell, tone: str, tc: float = 0.35):
    """Bandeaux d'angle sous les pans coupés hauts d'une salle `ship` (comme ceux du kit, dans
    une autre teinte : `room_shell(bands=False)`), interrompus au droit des portes."""
    nrm_of = {s: (Vector(Cell.inward(s)) * 0.92 + Vector((0, 0, -0.38))).normalized() for s in "NSEW"}
    for side in "NSEW":
        lo, hi = (k.x0, k.x1) if side in "NS" else (k.y0, k.y1)
        gaps = [(p - 1.0, p + 1.0) for sd, p, w, oh, *rest in openings_for(k.id) if sd == side]
        for sa, sb in _cuts(lo + 0.45, hi - 0.45, gaps):
            if sb - sa < 0.5:
                continue
            p0, p1 = k.wall(side, sa, 0.0, k.h - tc), k.wall(side, sb, 0.0, k.h - tc)
            k.add(kit.light_band(k.nm("band"), p0, p1, k.m, k.coll, normal=tuple(nrm_of[side]), tone=tone))


def prism(k: Cell, tag, section_yz, xa, xb, mat):
    """Plaque à profil quelconque dans le plan YZ (points (y, z)), extrudée de x = xa à xb."""
    import bmesh

    mesh = bpy.data.meshes.new(k.nm(tag))
    bm = bmesh.new()
    lo = [bm.verts.new((min(xa, xb), y, z)) for y, z in section_yz]
    hi = [bm.verts.new((max(xa, xb), y, z)) for y, z in section_yz]
    bm.faces.new(lo)
    bm.faces.new(list(reversed(hi)))
    n = len(section_yz)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()
    mesh.materials.append(k.mat(mat))
    obj = bpy.data.objects.new(mesh.name, mesh)
    k.coll.objects.link(obj)
    return k.obj(obj)


def status_lamp(k: Cell, tag, center, facing, mat, color, lumens=300.0, rng=3.0, light=True):
    """Voyant mural (gyrophare fixe) : embase sombre, globe émissif, cage ; repère LIGHT_."""
    vis = [
        c.cylinder(k.nm(f"{tag}_base"), 0.07, 0.06, (0, 0.03, 0), k.m["dark"], k.coll, axis="Y", segments=12),
        c.cylinder(k.nm(tag), 0.05, 0.1, (0, 0.11, 0), k.mat(mat), k.coll, axis="Y", segments=12),
    ]
    for i in range(2):
        a = i * PI / 2 + PI / 4
        o = c.box(k.nm(f"{tag}_cage"), (0.014, 0.13, 0.014), (0, 0.0, 0), k.m["dark"], k.coll)
        r, u = Vector((math.cos(a), 0, math.sin(a))), Vector((0, 1, 0))
        n = r.cross(u).normalized()
        o.matrix_basis = Matrix.Translation(Vector((0, 0.12, 0))) @ Matrix((r, u, n)).transposed().to_4x4()
        vis.append(o)
    marks = []
    if light:
        marks.append(c.empty(f"LIGHT_{k.id}_{tag}", (0, 0.3, 0), k.coll, {"lumens": lumens, "range": rng, "color": list(color), "shadow_candidate": 0}))
    bpy.context.view_layer.update()
    place(vis + marks, center, facing)
    for o in vis:
        _uv(o)
    k.visual += vis
    k.marks += marks


def header_lamps(k: Cell, side, s, colors):
    """Voyants sur le linteau d'une porte (face du linteau à 0,16 m du mur) : `colors` =
    [(décalage le long du mur, clé du matériau)]."""
    for ds, key in colors:
        x, y, z = k.wall(side, s + ds, 0.17, kit.DOOR_H + 0.15)
        n = Vector(Cell.inward(side))
        right = n.cross(Vector((0, 0, 1)))
        k.ori("door_lamp", (0.14, 0.07, 0.02), (x, y, z), tuple(-right), (0, 0, 1), key)


def cargo_net(k: Cell, center, rot, size=1.2):
    """Filet d'arrimage sur une caisse (posé dans son repère, tourné avec elle) : sangles de
    caoutchouc croisées sur la face −X et par-dessus, crochets au sol. Collé à la caisse : pas
    de collider propre."""
    h = size / 2
    objs = []

    def strap(tag, sz, at, mat="rubber"):
        objs.append(c.box(k.nm(tag), sz, at, k.m[mat], k.coll))

    for t in (-0.42, -0.14, 0.14, 0.42):
        y = t * size
        strap("net_v", (0.012, 0.045, size), (-h - 0.008, y, h))
        strap("net_t", (size - 0.08, 0.045, 0.012), (0.0, y, size + 0.008))
        strap("net_hook", (0.08, 0.06, 0.05), (-h - 0.05, y, 0.025), "dark")
    for z in (0.3, 0.72, size - 0.14):
        strap("net_h", (0.012, size - 0.04, 0.04), (-h - 0.014, 0.0, z))
    strap("net_x", (size - 0.04, 0.04, 0.012), (0.0, 0.0, size + 0.014))
    for o in objs:
        _uv(o)
    bpy.context.view_layer.update()
    place(objs, center, rot)
    k.visual += objs


# --- Cellules -----------------------------------------------------------------------------------


def build_couloir_avant(k: Cell):
    lamps = [(0.0, 3.0), (0.0, 9.0), (0.0, 13.0)]
    k.shell(floor_strip=True, style="ship", avoid=lamps)
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 2500.0, 6.0, COOL, facing=PI / 2, shadow=(i == 1))
    m, kc = k.m, k.coll
    # Parois : entre les nervures (y pairs), hors des cadres de portes (y 4,8 → 8,2).
    k.add(kit.junction_box(k.nm("jbox"), k.wall("W", 3.0, 0, 1.35), Cell.facing("W"), m, kc))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("W", 9.0, 0, 1.55), Cell.facing("W"), m, kc))
    k.add(kit.vent(k.nm("vent"), k.wall("W", 11.0, 0, 0.75), Cell.facing("W"), m, kc, size=(0.7, 0.22)))
    k.add(kit.stencil(k.nm("stencil"), k.wall("W", 13.0, 0, 1.95), Cell.facing("W"), m, kc, length=0.9))
    k.add(kit.vent(k.nm("vent"), k.wall("E", 1.0, 0, 0.75), Cell.facing("E"), m, kc, size=(0.7, 0.22)))
    k.add(kit.stencil(k.nm("stencil"), k.wall("E", 3.0, 0, 1.95), Cell.facing("E"), m, kc, length=1.2))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("E", 9.0, 0, 1.35), Cell.facing("E"), m, kc, size=(0.5, 0.14, 0.62)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("E", 11.0, 0, 1.6), Cell.facing("E"), m, kc, size=(0.5, 0.34)))
    # Conduite verticale dans l'angle du fond, à côté du cadre de la porte du cockpit.
    k.add(kit.pipe(k.nm("riser"), (1.08, 13.88, 0.32), (1.08, 13.88, 2.15), m, kc, radius=0.045))


def build_couloir_arriere(k: Cell):
    lamps = [(0.0, -13.0), (0.0, -9.0), (0.0, -5.0), (0.0, -1.0)]
    # Les casiers sont adossés au mur E : le pan bas s'y interrompt.
    k.shell(floor_strip=True, style="ship", avoid=lamps, low_gaps=[("E", -10.65, 1.5)])
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 2500.0, 6.0, COOL, facing=PI / 2, shadow=(i == 1))
    m, kc = k.m, k.coll
    k.add(kit.locker("couloir_arriere_locker0", (0.85, -11.0, 0.0), -PI / 2, m, kc))
    k.add(kit.locker("couloir_arriere_locker1", (0.85, -10.3, 0.0), -PI / 2, m, kc))
    k.add(kit.stencil(k.nm("stencil"), k.wall("E", -10.65, 0, 2.06), Cell.facing("E"), m, kc, length=1.2))
    # Paroi W : écran, boîtier, grille, pochoir ; paroi E : boîtier près de la jonction.
    k.add(kit.vent(k.nm("vent"), k.wall("W", -13.0, 0, 0.75), Cell.facing("W"), m, kc, size=(0.7, 0.22)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("W", -11.0, 0, 1.35), Cell.facing("W"), m, kc))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("W", -9.0, 0, 1.55), Cell.facing("W"), m, kc))
    k.add(kit.stencil(k.nm("stencil"), k.wall("W", -5.0, 0, 1.95), Cell.facing("W"), m, kc, length=1.2))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("E", -3.0, 0, 1.35), Cell.facing("E"), m, kc, size=(0.5, 0.14, 0.62)))
    k.add(kit.vent(k.nm("vent"), k.wall("E", -5.0, 0, 0.75), Cell.facing("E"), m, kc, size=(0.7, 0.22)))
    k.add(kit.stencil(k.nm("stencil"), k.wall("E", -1.0, 0, 1.95), Cell.facing("E"), m, kc, length=0.9))
    # Fond (porte des machines) : conduite verticale dans l'angle.
    k.add(kit.pipe(k.nm("riser"), (-1.08, -13.88, 0.32), (-1.08, -13.88, 2.15), m, kc, radius=0.045))


def build_sas(k: Cell):
    lamps = [(-3.6, -4.2), (-3.6, -1.8)]
    k.shell(style="ship", avoid=lamps)
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 2000.0, 6.0, COOL, shadow=(i == 0))
    m, kc = k.m, k.coll
    # Combinaisons de part et d'autre de l'écoutille (positions de M1).
    k.add(kit.locker("sas_suit0", (-5.15, -4.4, 0.0), PI / 2, m, kc))
    k.add(kit.locker("sas_suit1", (-5.15, -1.6, 0.0), PI / 2, m, kc))
    # Écoutille extérieure : zone de danger au sol, voyants rouges, gyrophare.
    k.add(kit.hazard_frame(k.nm("hatch_zone"), (-4.8, -3.0, 0.0), (0.72, 1.4), m, kc, width=0.12))
    header_lamps(k, "W", -3.0, [(-0.55, "beacon"), (0.55, "beacon")])
    k.add(kit.beacon(k.nm("beacon"), k.wall("W", -4.4, 0, 2.12), Cell.facing("W"), m, kc, lumens=600.0, rng=3.0))
    status_lamp(k, "go", k.wall("W", -1.6, 0, 2.12), Cell.facing("W"), "beacon_green", GREEN, lumens=400.0)
    # Porte intérieure : voyants verts côté sas.
    header_lamps(k, "E", -3.0, [(-0.55, "beacon_green"), (0.55, "beacon_green")])
    # Grille d'évacuation au centre (purge), sans relief.
    k.add(kit.floor_grating(k.nm("drain"), (-3.4, -3.0, 0.0), (0.8, 1.2), 0.0, m, kc))
    # Paroi N : banc d'équipement, écran de cycle ; paroi S : armoire de pressurisation.
    bx, by = -3.0, -1.0 - 0.09 - 0.23
    k.box("bench_top", (1.6, 0.46, 0.06), (bx, by, 0.47), "dark", bevel=0.01)
    k.box("bench_pad", (1.5, 0.4, 0.05), (bx, by, 0.525), "rubber")
    for sx in (-1, 1):
        k.box("bench_leg", (0.06, 0.4, 0.44), (bx + sx * 0.7, by, 0.22), "dark")
    k.col("bench", (1.6, 0.48, 0.55), (bx, by, 0.275))
    k.add(kit.wall_screen(k.nm("cycle"), k.wall("N", -2.6, 0, 1.6), Cell.facing("N"), m, kc, size=(0.7, 0.4)))
    k.add(kit.stencil(k.nm("stencil"), k.wall("N", -4.45, 0, 2.08), Cell.facing("N"), m, kc, length=1.0))
    k.add(kit.cabinet(k.nm("press"), k.wall("S", -3.4), Cell.facing("S"), m, kc, size=(1.2, 0.32, 1.6)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("S", -2.25, 0, 1.4), Cell.facing("S"), m, kc, size=(0.4, 0.14, 0.5)))
    # Conduites de pressurisation au plafond, de l'écoutille à la porte intérieure.
    for y, r in ((-4.55, 0.06), (-1.45, 0.045)):
        k.add(kit.pipe(k.nm("pipe"), (-5.3, y, 2.38), (-1.9, y, 2.38), m, kc, radius=r, hang_to=k.h))


def build_soute(k: Cell):
    lamps = [(4.4, -5.0), (7.2, -5.0), (4.4, -9.0), (7.2, -9.0)]
    k.shell(style="industrial", avoid=lamps)
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 16000.0, 10.0, SODIUM, lens="light_amber", drop=0.3, shadow=(i == 3))
    m, kc = k.m, k.coll
    # Caisses de M1 (mêmes positions, tailles et colliders), teintes variées.
    stacks = [(3.2, -4.0), (3.2, -5.2), (4.4, -4.0), (8.4, -4.2), (8.4, -5.5), (8.4, -10.4), (7.1, -10.4)]
    variants = ["olive", "slate", "olive", "yellow", "rust", "slate", "olive"]
    for i, ((x, y), v) in enumerate(zip(stacks, variants)):
        k.add(kit.crate(f"soute_crate{i}", (x, y, 0.0), 0.05 * i, m, kc, size=(1.2, 1.2, 1.2), variant=v))
    k.add(kit.crate("soute_crate_top", (3.2, -4.0, 1.2), 0.3, m, kc, variant="rust"))
    k.pickup(kit.pickup("soute_pickup_parts", (6.0, -7.0, 0.0), 0.4, m, kc, item="spare_parts"))
    for i in (3, 4, 5):
        x, y = stacks[i]
        cargo_net(k, (x, y, 0.0), 0.05 * i)
    # Zone de levage sous le palan, allée balisée depuis la porte.
    k.add(kit.hazard_frame(k.nm("lift_zone"), (5.9, -7.0, 0.0), (1.7, 1.7), m, kc))
    for y in (-6.25, -7.75):
        k.add(kit.hazard_stripe(k.nm("lane"), (1.9, y, 0.0), (4.85, y, 0.0), m, kc, width=0.1))
    k.ori("chevrons", (2.4, 0.36, 0.006), (3.3, -7.0, 0.003), (1, 0, 0), (0, 1, 0), "trim_sheet", uv="chevrons", tile=0.8)
    # Pont roulant : deux chemins de roulement sous les poutres, une poutre transversale et
    # son palan au-dessus de la zone de levage (tout au-dessus de 2,1 m).
    zr = k.h - 0.26
    for x in (2.55, 9.05):
        k.add(kit.ceiling_beam(k.nm("runway"), (x, -11.9, zr), (x, -2.1, zr), m, kc, depth=0.2, width=0.18))
    zb = zr - 0.2
    k.add(kit.ceiling_beam(k.nm("bridge"), (2.55, -7.0, zb), (9.05, -7.0, zb), m, kc, depth=0.24, width=0.22, mat=m["paint_yellow"]))
    for x in (2.55, 9.05):
        k.box("truck", (0.3, 0.5, 0.14), (x, -7.0, zb + 0.02), "dark", bevel=0.01)
    zh = zb - 0.24
    k.box("hoist", (0.42, 0.5, 0.34), (5.9, -7.0, zh - 0.17), "dark", bevel=0.02)
    k.ori("hoist_label", (0.3, 0.1, 0.004), (5.9, -7.252, zh - 0.12), (1, 0, 0), (0, 0, 1), "trim_sheet", uv="hazard")
    k.cyl("hoist_cable", 0.012, zh - 0.34 - 2.3, (5.9, -7.0, (zh - 0.34 + 2.3) / 2), "dark", segments=6)
    k.box("hook_block", (0.16, 0.1, 0.12), (5.9, -7.0, 2.24), "paint_yellow", bevel=0.01)
    # Parois : armoire de manifeste (N), écrans et boîtiers au-dessus des caisses, gaine (E).
    k.add(kit.cabinet(k.nm("manifest"), k.wall("N", 6.6, 0.04), Cell.facing("N"), m, kc, size=(1.1, 0.32, 1.7)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("N", 4.6, 0.04, 1.75), Cell.facing("N"), m, kc))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("N", 8.6, 0.04, 1.8), Cell.facing("N"), m, kc))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("S", 4.6, 0.04, 1.75), Cell.facing("S"), m, kc, size=(0.8, 0.46)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("S", 2.6, 0.04, 1.6), Cell.facing("S"), m, kc))
    k.add(kit.pipe(k.nm("riser"), (6.6, -11.85, 0.0), (6.6, -11.85, k.h), m, kc, radius=0.07))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("E", -7.0, 0.04, 1.95), Cell.facing("E"), m, kc, size=(0.5, 0.16, 0.5)))
    k.add(kit.duct(k.nm("duct"), (9.45, -11.95, 2.62), (9.45, -2.05, 2.62), m, kc, size=(0.5, 0.32), hang_to=k.h))
    k.add(kit.beacon(k.nm("beacon"), k.wall("W", -7.0, 0, 2.72), Cell.facing("W"), m, kc))
    k.add(kit.stencil(k.nm("stencil"), k.wall("W", -9.0, 0.04, 2.3), Cell.facing("W"), m, kc, length=1.4))
    # Tuyauterie le long du mur W, chemin de câbles le long du mur N (sous les chemins de
    # roulement).
    k.add(kit.pipe(k.nm("pipe"), (2.08, -11.95, 2.62), (2.08, -2.05, 2.62), m, kc, radius=0.08, hang_to=k.h))
    k.add(kit.cable_tray(k.nm("tray"), (1.85, -2.45, 2.72), (9.75, -2.45, 2.72), m, kc, hang_to=zr))


def build_machines(k: Cell):
    lamps = [(-2.2, -17.0), (2.2, -17.0), (-2.2, -25.0), (2.2, -25.0)]
    k.shell(style="industrial", avoid=lamps)
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 13000.0, 10.0, SODIUM, lens="light_amber", drop=0.3, shadow=(i == 0))
    m, kc = k.m, k.coll
    # Props de M1 (mêmes positions et colliders).
    k.add(kit.reactor("machines_reactor", (0.0, -21.0, 0.0), m, kc, ceiling=k.h))
    k.add(kit.console("machines_console", (3.4, -16.0, 0.0), PI, m, kc, label="engineering"))
    k.add(kit.crate("machines_crate0", (-3.6, -16.2, 0.0), 0.2, m, kc, variant="slate"))
    k.add(kit.crate("machines_crate1", (-3.8, -17.4, 0.0), -0.1, m, kc, size=(1.2, 1.2, 1.0), variant="rust"))
    k.add(kit.hazard_frame(k.nm("reactor_zone"), (0.0, -21.0, 0.0), (2.7, 2.7), m, kc))
    k.add(kit.floor_grating(k.nm("grating"), (0.0, -17.9, 0.0), (1.4, 0.8), 0.0, m, kc))
    k.add(kit.floor_grating(k.nm("grating"), (0.0, -24.7, 0.0), (1.4, 0.9), 0.0, m, kc))
    # Pompes et armoires contre les parois, entre les colonnes.
    k.add(kit.pump(k.nm("pump"), (4.3, -20.95, 0.0), PI / 2, m, kc, height=k.h))
    k.add(kit.pump(k.nm("pump"), (-4.3, -25.0, 0.0), -PI / 2, m, kc, height=k.h))
    k.add(kit.cabinet(k.nm("cabinet"), k.wall("W", -21.0, 0.04), Cell.facing("W"), m, kc, size=(1.2, 0.32, 1.7)))
    k.add(kit.cabinet(k.nm("cabinet"), k.wall("E", -25.0, 0.04), Cell.facing("E"), m, kc, size=(1.1, 0.32, 1.7)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("N", 4.0, 0.04, 2.15), Cell.facing("N"), m, kc, size=(0.9, 0.46)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("N", -2.0, 0.04, 1.7), Cell.facing("N"), m, kc))
    for x in (-2.0, 2.0):
        k.add(kit.junction_box(k.nm("jbox"), k.wall("S", x, 0.04, 1.6), Cell.facing("S"), m, kc, size=(0.5, 0.16, 0.62)))
    k.add(kit.pipe(k.nm("riser"), (0.0, -25.86, 0.0), (0.0, -25.86, k.h), m, kc, radius=0.1))
    k.add(kit.pipe(k.nm("riser"), (-4.86, -19.0, 0.0), (-4.86, -19.0, k.h), m, kc, radius=0.06))
    k.add(kit.beacon(k.nm("beacon"), k.wall("W", -21.0, 0.04, 2.45), Cell.facing("W"), m, kc))
    k.add(kit.beacon(k.nm("beacon"), k.wall("E", -17.0, 0.04, 2.45), Cell.facing("E"), m, kc))
    # Plafond : gaine (W), tuyaux (E), chemin de câbles, tous sous les poutres.
    zt = k.h - 0.27
    k.add(kit.duct(k.nm("duct"), (-3.6, -25.95, zt - 0.2), (-3.6, -14.45, zt - 0.2), m, kc, size=(0.55, 0.34), hang_to=k.h))
    k.add(kit.pipe(k.nm("pipe"), (3.55, -25.95, zt - 0.12), (3.55, -14.45, zt - 0.12), m, kc, radius=0.09, hang_to=k.h))
    k.add(kit.pipe(k.nm("pipe"), (3.85, -25.95, zt - 0.08), (3.85, -14.45, zt - 0.08), m, kc, radius=0.05, hang_to=k.h))
    k.add(kit.cable_tray(k.nm("tray"), (1.2, -25.95, zt - 0.05), (1.2, -14.45, zt - 0.05), m, kc, hang_to=k.h))


def warm_palette(k: Cell, floor=None):
    """Salles de vie : tôle peinte crème à la place de la tôle claire (coque, casiers, tiroirs)."""
    warm = k.m["panel_warm"]
    k.m = {**k.m, "panel": warm, "wall": warm, "ceiling": warm, "band_warm": k.m["band_hearth"]}
    if floor:
        k.m["floor_plate"] = k.m["floor"] = k.m[floor]


def build_quartiers(k: Cell):
    lamps = [(-5.0, 3.0), (-5.0, 7.0)]
    warm_palette(k)
    k.shell(style="ship", bands=False, avoid=lamps)
    chamfer_bands(k, "hearth")
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 4200.0, 8.0, WARM, lens="light_warm", shadow=(i == 0))
    m, kc = k.m, k.coll
    # Props de M1.
    k.add(kit.bunk("quartiers_bunk0", (-7.85, 4.0, 0.0), PI / 2, m, kc))
    k.add(kit.bunk("quartiers_bunk1", (-7.85, 8.0, 0.0), PI / 2, m, kc))
    k.add(kit.locker("quartiers_locker0", (-4.0, 9.65, 0.0), PI, m, kc))
    k.add(kit.locker("quartiers_locker1", (-4.7, 9.65, 0.0), PI, m, kc))
    k.add(kit.table("quartiers_desk", (-4.6, 2.6, 0.0), 0.0, m, kc, size=(1.6, 0.8)))
    k.pickup(kit.pickup("quartiers_pickup_datapad", (-4.9, 2.6, 0.79), 0.3, m, kc, item="datapad"))
    # Coin bureau : écran mural, étagère et ses boîtes, tabouret ; lampe de bureau (cuite).
    k.add(kit.wall_screen(k.nm("screen"), k.wall("S", -5.4, 0, 1.5), Cell.facing("S"), m, kc, size=(0.7, 0.42)))
    k.box("shelf", (1.3, 0.26, 0.04), (-7.4, 2.21, 1.62), "dark", bevel=0.005)
    for i, (dx, sz, mat) in enumerate(((-0.4, (0.28, 0.2, 0.22), "crate_slate"), (-0.05, (0.22, 0.18, 0.16), "rubber"), (0.35, (0.34, 0.2, 0.2), "crate"))):
        k.box("shelf_box", sz, (-7.4 + dx, 2.23, 1.64 + sz[2] / 2), mat, bevel=0.008)
    k.add(kit.light_band(k.nm("desk_lamp"), (-6.2, 2.06, 1.98), (-4.6, 2.06, 1.98), m, kc, normal=(0, 0.5, -0.87), tone="warm"))
    k.cyl("stool", 0.19, 0.06, (-4.6, 3.45, 0.45), "rubber", segments=12)
    k.cyl("stool_post", 0.035, 0.42, (-4.6, 3.45, 0.21), "bare", segments=8)
    k.cyl("stool_foot", 0.17, 0.03, (-4.6, 3.45, 0.015), "dark", segments=12)
    k.col("stool", (0.4, 0.4, 0.48), (-4.6, 3.45, 0.24))
    # Entre les couchettes : armoire de rangement ; tapis au centre.
    k.add(kit.cabinet(k.nm("cabinet"), k.wall("W", 6.0), Cell.facing("W"), m, kc, size=(0.9, 0.3, 1.3)))
    k.box("rug", (1.6, 2.2, 0.012), (-5.0, 6.0, 0.006), "rug", tile=1.0)
    # Paroi N (à côté des casiers) et paroi E (côté couloir).
    k.add(kit.wall_screen(k.nm("screen"), k.wall("N", -5.6, 0, 1.55), Cell.facing("N"), m, kc, size=(0.6, 0.4)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("E", 3.2, 0, 1.35), Cell.facing("E"), m, kc, size=(0.4, 0.14, 0.5)))
    k.add(kit.vent(k.nm("vent"), k.wall("E", 8.8, 0, 0.4), Cell.facing("E"), m, kc, size=(0.6, 0.2)))
    k.add(kit.vent(k.nm("vent"), (-3.0, 5.0, k.h), 0.0, m, kc, size=(0.5, 0.5), surface="ceiling"))
    k.add(kit.pipe(k.nm("pipe"), (-2.0, 2.05, 2.25), (-2.0, 9.95, 2.25), m, kc, radius=0.04, hang_to=k.h))


def build_carre(k: Cell):
    lamps = [(5.0, 6.0), (6.6, 3.0), (3.6, 9.0)]
    warm_palette(k, floor="tiles")
    k.shell(style="ship", bands=False, avoid=lamps, beams=[4.0, 8.0])
    chamfer_bands(k, "hearth")
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 3400.0, 7.0, WARM, lens="light_warm", shadow=(i == 0))
    m, kc = k.m, k.coll
    # Props de M1 ; la console de l'office regarde maintenant la pièce (π de plus).
    k.add(kit.table("carre_table", (5.0, 6.0, 0.0), PI / 2, m, kc, size=(2.4, 1.0)))
    k.add(kit.seat("carre_seat0", (4.1, 5.2, 0.0), -PI / 2, m, kc))
    k.add(kit.seat("carre_seat1", (5.9, 6.8, 0.0), PI / 2, m, kc))
    k.add(kit.console("carre_galley", (7.85, 3.0, 0.0), -PI / 2 + PI, m, kc, label="galley"))
    # Sur la table : plateau, gobelets, bloc de rations.
    k.box("tray", (0.34, 0.5, 0.02), (5.0, 6.35, 0.8), "bare", bevel=0.005)
    for i, (x, y) in enumerate(((4.82, 5.3), (5.2, 5.55), (4.86, 6.85), (5.18, 7.05))):
        k.cyl("mug", 0.04, 0.1, (x, y, 0.84), "crate_slate" if i % 2 else "paint_yellow", segments=10)
    k.box("ration", (0.16, 0.24, 0.07), (5.05, 6.3, 0.845), "crate", bevel=0.006)
    # Office le long du mur E : comptoir, placards hauts, distributeur, bidons.
    k.add(kit.table(k.nm("counter"), (8.1, 5.0, 0.0), PI / 2, m, kc, size=(1.6, 0.6)))
    for y, sz in ((4.6, (0.24, 0.3, 0.42)), (5.15, (0.18, 0.18, 0.26)), (5.45, (0.18, 0.18, 0.2))):
        k.box("canister", (sz[1], sz[0], sz[2]), (8.12, y, 0.79 + sz[2] / 2), "bare" if sz[0] > 0.2 else "crate_slate", bevel=0.01)
    k.box("cupboard", (0.36, 1.6, 0.6), (8.22, 5.0, 1.82), "dark", bevel=0.01)
    k.ori("cupboard_door", (0.74, 0.54, 0.012), (8.035, 4.6, 1.82), (0, -1, 0), (0, 0, 1), "panel", uv=("box", (0.3, 0.1)))
    k.ori("cupboard_door", (0.74, 0.54, 0.012), (8.035, 5.4, 1.82), (0, -1, 0), (0, 0, 1), "panel", uv=("box", (0.6, 0.4)))
    k.ori("cupboard_lamp", (1.4, 0.05, 0.01), (8.08, 5.0, 1.515), (0, 1, 0), (1, 0, 0), "trim_sheet", uv="light_strip")
    # Parois : écran (N), buffet (S), boîtier (W, côté couloir), grille, tuyau au plafond.
    k.add(kit.wall_screen(k.nm("screen"), k.wall("N", 6.6, 0, 1.55), Cell.facing("N"), m, kc, size=(0.9, 0.5)))
    k.add(kit.cabinet(k.nm("sideboard"), k.wall("S", 4.6), Cell.facing("S"), m, kc, size=(1.3, 0.32, 1.1)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("W", 3.2, 0, 1.35), Cell.facing("W"), m, kc, size=(0.4, 0.14, 0.5)))
    k.add(kit.vent(k.nm("vent"), k.wall("W", 8.8, 0, 0.4), Cell.facing("W"), m, kc, size=(0.6, 0.2)))
    k.add(kit.vent(k.nm("vent"), (5.0, 3.6, k.h), 0.0, m, kc, size=(0.5, 0.5), surface="ceiling"))
    k.add(kit.pipe(k.nm("pipe"), (8.0, 2.05, 2.25), (8.0, 9.95, 2.25), m, kc, radius=0.04, hang_to=k.h))


def build_cockpit(k: Cell):
    rise, py0 = 0.8, 16.8
    lamps = [(0.0, 15.5, 0.0), (-1.0, 17.7, PI / 2), (1.0, 17.7, PI / 2)]
    k.shell(style="ship", beams=[-2.0, 2.0], avoid=[(x, y) for x, y, _ in lamps])
    for i, (x, y, f) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 2800.0, 8.0, COCKPIT_WHITE, facing=f, shadow=(i == 1))
    m, kc = k.m, k.coll
    x0, x1, y1 = k.x0, k.x1, k.y1

    # --- Plate-forme (collider de M1), escalier, garde-corps ---------------------------------
    k.box("platform", (x1 - x0, y1 - py0, rise - 0.02), (0, (py0 + y1) / 2, (rise - 0.02) / 2), "dark", structure=True)
    k.box("platform_top", (x1 - x0, y1 - py0, 0.02), (0, (py0 + y1) / 2, rise - 0.01), "floor_plate", structure=True)
    k.colliders.append(c.collider_box("cockpit_platform", (x1 - x0, y1 - py0, rise), (0, (py0 + y1) / 2, rise / 2), kc))
    k.add(kit.stairs("cockpit_stairs", -1.0, 1.0, py0 - 1.2, rise, 4, m, kc))
    for sx in (-1, 1):
        xr = sx * 2.5
        # Face avant de la plate-forme : nez sombre, bandeau bleu dessous, grille, plinthe rivée.
        k.box("nosing", (3.0, 0.06, 0.06), (xr, py0 - 0.03, rise - 0.03), "dark", bevel=0.01)
        k.add(kit.light_band(k.nm("nose_band"), (sx * 1.05, py0 - 0.02, rise - 0.13), (sx * 3.95, py0 - 0.02, rise - 0.13), m, kc, normal=(0, -0.75, -0.66), tone="blue"))
        k.ori("nose_vent", (1.4, 0.24, 0.006), (sx * 2.6, py0 - 0.003, 0.36), (1, 0, 0), (0, 0, 1), "trim_sheet", uv="vent")
        k.ori("nose_kick", (2.96, 0.12, 0.012), (xr, py0 - 0.006, 0.065), (1, 0, 0), (0, 0, 1), "trim_sheet", uv="rivets")
        k.add(kit.hazard_stripe(k.nm("edge"), (sx * 1.0, py0 + 0.16, rise), (sx * 4.0, py0 + 0.16, rise), m, kc, width=0.12))
        # Garde-corps : mêmes colliders qu'en M1 ; main courante, lisse, poteaux, plinthe.
        k.colliders.append(c.collider_box(f"cockpit_rail{sx}", (3.0, 0.1, 1.0), (xr, py0 + 0.05, rise + 0.5), kc))
        k.cyl("rail_top", 0.028, 3.0, (xr, py0 + 0.05, rise + 1.0), "bare", axis="X", segments=8)
        k.cyl("rail_mid", 0.018, 3.0, (xr, py0 + 0.05, rise + 0.55), "bare", axis="X", segments=8)
        for px in (1.04, 2.5, 3.96):
            k.cyl("rail_post", 0.03, 1.0, (sx * px, py0 + 0.05, rise + 0.5), "dark", segments=8)
        k.ori("rail_kick", (3.0, 0.1, 0.012), (xr, py0 + 0.05, rise + 0.05), (1, 0, 0), (0, 0, 1), "trim_sheet", uv="hazard")
        # Limons de l'escalier.
        prism(k, "stringer", [(py0 - 1.2, 0.0), (py0, 0.0), (py0, rise + 0.06), (py0 - 1.2, 0.26)], sx * 1.005, sx * 1.045, "dark")

    k.add(kit.floor_grating(k.nm("aisle"), (0.0, 18.0, rise), (1.0, 1.7), 0.0, m, kc))

    # --- Postes de pilotage (M1) et pupitre central ------------------------------------------
    k.add(kit.seat("cockpit_pilot", (-1.0, 19.6, rise), 0.0, m, kc, role="pilot"))
    k.add(kit.seat("cockpit_copilot", (1.0, 19.6, rise), 0.0, m, kc, role="copilot"))
    k.add(kit.console("cockpit_helm", (-1.0, 21.2, rise), PI, m, kc, label="helm", low=True))
    k.add(kit.console("cockpit_nav", (1.0, 21.2, rise), PI, m, kc, label="navigation", low=True))
    k.add(kit.console("cockpit_comms", (3.4, 18.5, rise), PI / 2, m, kc, label="comms"))
    # Pupitre des gaz entre les deux consoles (hors du trajet : y > 21).
    k.box("pedestal", (0.56, 0.56, 0.78), (0, 21.36, rise + 0.39), "dark", bevel=0.015)
    a = math.radians(25)
    k.ori("pedestal_top", (0.5, 0.44, 0.012), (0, 21.32, rise + 0.79), (1, 0, 0), (0, math.cos(a), math.sin(a)), "buttons")
    for sx in (-1, 1):
        k.cyl("throttle", 0.014, 0.2, (sx * 0.12, 21.3, rise + 0.9), "bare", segments=8)
        k.box("throttle_knob", (0.07, 0.05, 0.05), (sx * 0.12, 21.3, rise + 1.01), "rubber", bevel=0.01)
    k.col("pedestal", (0.6, 0.6, 0.95), (0, 21.36, rise + 0.475))

    # --- Verrière : meneaux, bandeau bleu sous le linteau ------------------------------------
    for x in (-2.2, 0.0, 2.2):
        k.box("mullion", (0.1, 0.22, 1.45), (x, y1 + 0.0, 1.75 + 0.725), "dark", bevel=0.01)
    k.add(kit.light_band(k.nm("canopy_band"), (-3.0, y1 - 0.15, 3.17), (3.0, y1 - 0.15, 3.17), m, kc, normal=(0, -0.35, -0.94), tone="blue"))

    # --- Panneau de plafond au-dessus des sièges (face aux yeux du pilote, > 2,9 m) ---------
    # Face tournée vers l'œil du pilote (n), « haut » de la face (up) tel que r × up = n.
    n = Vector((0, -0.45, -0.89)).normalized()
    r = Vector((1, 0, 0))
    up = Vector((0, n.z, -n.y))
    ctr = Vector((0, 20.05, 3.13))
    k.ori("overhead", (3.2, 1.0, 0.08), tuple(ctr), tuple(r), tuple(up), "dark", bevel=0.015)
    for sx in (-1, 1):
        k.ori("overhead_buttons", (1.05, 0.62, 0.012), tuple(ctr + n * 0.046 + r * (sx * 0.98) + up * 0.06), tuple(r), tuple(up), "buttons")
    k.ori("overhead_screen", (0.72, 0.42, 0.012), tuple(ctr + n * 0.046 + up * 0.12), tuple(r), tuple(up), "screen", tile=0.75)
    k.ori("overhead_strip", (2.9, 0.05, 0.01), tuple(ctr + n * 0.046 - up * 0.4), tuple(r), tuple(up), "trim_sheet", uv="light_strip")
    for sx in (-1, 1):
        k.box("overhead_rod", (0.04, 0.04, 0.5), (sx * 1.45, 20.45, 3.15), "dark")

    # --- Parois de la plate-forme : armoire, écrans, boîtiers ---------------------------------
    k.add(kit.cabinet(k.nm("cabinet"), k.wall("W", 19.4, 0, rise), Cell.facing("W"), m, kc, size=(1.4, 0.32, 1.5)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("W", 21.2, 0, rise + 1.45), Cell.facing("W"), m, kc, size=(0.7, 0.45)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("E", 21.2, 0, rise + 1.45), Cell.facing("E"), m, kc, size=(0.7, 0.45)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("W", 17.4, 0, rise + 1.5), Cell.facing("W"), m, kc))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("E", 17.4, 0, rise + 1.5), Cell.facing("E"), m, kc))
    # Plafond : chemins de câbles et tuyaux le long des parois.
    for sx in (-1, 1):
        k.add(kit.cable_tray(k.nm("tray"), (sx * 3.2, 14.7, 3.2), (sx * 3.2, 21.75, 3.2), m, kc, width=0.3, hang_to=k.h))
        k.add(kit.pipe(k.nm("pipe"), (sx * 2.6, 14.6, 3.27), (sx * 2.6, 21.8, 3.27), m, kc, radius=0.045, hang_to=k.h))

    # --- Partie basse (entrée) : casiers, armoire, écran, caillebotis --------------------------
    k.add(kit.locker(k.nm("locker"), (-3.4, 14.68, 0.0), 0.0, m, kc))
    k.add(kit.locker(k.nm("locker"), (-2.77, 14.68, 0.0), 0.0, m, kc))
    k.add(kit.cabinet(k.nm("cabinet"), k.wall("S", 3.0), Cell.facing("S"), m, kc, size=(1.3, 0.32, 1.6)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("W", 15.4, 0, 1.4), Cell.facing("W"), m, kc))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("E", 15.4, 0, 1.55), Cell.facing("E"), m, kc, size=(0.6, 0.4)))
    k.add(kit.floor_grating(k.nm("grating"), (0.0, 15.1, 0.0), (1.2, 0.66), 0.0, m, kc))


BUILDERS = {
    "machines": build_machines,
    "couloir_arriere": build_couloir_arriere,
    "sas": build_sas,
    "soute": build_soute,
    "couloir_avant": build_couloir_avant,
    "quartiers": build_quartiers,
    "carre": build_carre,
    "cockpit": build_cockpit,
}


def build_all(m) -> dict[str, Cell]:
    cells = {}
    for cid in CELLS:
        k = Cell(cid, m)
        BUILDERS[cid](k)
        # Toute pièce posée à la main a ses UV de texture (projection boîte à défaut).
        kit.apply_uvs(k.visual)
        cells[cid] = k
    bpy.context.view_layer.update()
    return cells


# --- Descripteur du secteur ---------------------------------------------------------------------


def portal_record(a, b, kind, x, y, axis):
    x0, x1, y0, y1, h = CELLS[a]
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
        bx0, bx1, by0, by1, _ = CELLS[b]
        bc = ((bx0 + bx1) / 2, (by0 + by1) / 2)
    if axis == "y":
        normal = (0.0, 1.0 if bc[1] > y else -1.0, 0.0)
    else:
        normal = (1.0 if bc[0] > x else -1.0, 0.0, 0.0)
    return {
        "id": f"{a}__{b}",
        "a": a,
        "b": b,
        "kind": kind,
        "center": c.to_bevy((x, y, height / 2)),
        "normal": c.to_bevy(normal),
        "normal_blender": normal,
        "width": round(width, 3),
        "height": round(height, 3),
    }


def fmt(v) -> str:
    return "(" + ", ".join(f"{x:.3f}" for x in v) + ")"


def sector_ron(lightmaps: dict) -> str:
    lines = [
        "// Généré par tools/blender/ships/player_ship.py : NE PAS ÉDITER À LA MAIN.",
        "// Le vaisseau du joueur (proposition P12) : cellules, portails, apparitions.",
        "// Coordonnées dans le repère du jeu (Y-haut, −Z = avant du vaisseau), en mètres.",
        "(",
        f'    id: "{SECTOR}",',
        "    kind: Ship,",
        '    name_key: "sector.player_ship",',
        '    status: Proposition("P12"),',
        f'    door_model: "{DOOR_MODEL}",',
        "    cells: [",
    ]
    for cell, (x0, x1, y0, y1, h) in CELLS.items():
        lo = c.to_bevy((x0 - kit.WALL, y1 + kit.WALL, -kit.WALL))
        hi = c.to_bevy((x1 + kit.WALL, y0 - kit.WALL, h + kit.WALL))
        model = f"models/ships/{SECTOR}/{cell}"
        lines += [
            "        (",
            f'            id: "{cell}",',
            f'            model: "{model}.gltf",',
            f'            lod: "{model}_lod.gltf",',
            f"            min: {fmt(lo)},",
            f"            max: {fmt(hi)},",
        ]
        if cell in lightmaps:
            lmap = lightmaps[cell]
            lines.append(f'            lightmap: (image: "{lmap["image"]}", exposure: {lmap["exposure"]:.3f}),')
        lines.append("        ),")
    lines.append("    ],")
    lines.append("    portals: [")
    for p in PORTALS:
        r = portal_record(*p)
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
    for sid, cell, x, y, z, heading in SPAWNS:
        lines.append(
            f'        (id: "{sid}", cell: "{cell}", pos: {fmt(c.to_bevy((x, y, z)))}, heading_deg: {heading:.1f}),'
        )
    lines += ["    ],", ")", ""]
    return "\n".join(lines)


# --- Lightmap : faces cachées, dépliage ---------------------------------------------------------
# Écart de 2,5 texels entre îlots, densité minimale de 18 texels/m, faces cachées retirées :
# introduits ici (lot B de M8), désormais communs à tous les secteurs (`lightmap.py`).


def cull_hidden_faces(k: Cell) -> int:
    """Faces que le joueur ne voit jamais retirées (`lightmap.cull_hidden_faces`) : bornes
    intérieures de la cellule, sol, et l'estrade du cockpit (0,8 m)."""
    floors = [0.0] + ([0.8] if k.id == "cockpit" else [])
    keep, removed = lm.cull_hidden_faces(k.visual, (k.x0, k.y0, 0.0), (k.x1, k.y1, k.h), floors=floors)
    alive = set(keep)
    k.visual = keep
    k.structure = [o for o in k.structure if o in alive]
    return removed


def prepare_lightmap(k: Cell) -> tuple[int, float]:
    """Faces cachées retirées, résolution de la lightmap, dépliage ; renvoie (résolution,
    densité en texels par mètre)."""
    cull_hidden_faces(k)
    return lm.prepare(k.visual + k.pickups)


# --- Export et bake -----------------------------------------------------------------------------


def _tris(objs) -> int:
    return sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in objs if o.type == "MESH")


def export(cells: dict[str, Cell], bake: bool) -> dict:
    exports = c.collection("EXPORT")
    stats, bake_input = {}, {}
    quiet = io.StringIO()
    with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
        for cid, k in cells.items():
            # UV de lightmap avant la fusion : la version Locale et la version Sectorielle
            # (sous-ensemble des mêmes pièces) partagent la même lightmap.
            res, density = prepare_lightmap(k)
            merged = c.join_by_material(k.visual, cid, exports)
            c.export_gltf(f"{MODELS}/{cid}.gltf", merged + k.colliders + k.marks + k.pickups)
            lod = c.join_by_material(k.structure, f"{cid}_lod1", exports)
            c.export_gltf(f"{MODELS}/{cid}_lod.gltf", lod)
            verify_gltf(f"{MODELS}/{cid}.gltf")
            verify_gltf(f"{MODELS}/{cid}_lod.gltf")
            for obj in merged + lod:
                obj.hide_set(True)
            mats = sorted({mt.name for o in merged + k.pickups for mt in o.data.materials if mt})
            stats[cid] = {"tris": _tris(merged + k.pickups), "lod": _tris(lod), "mats": mats, "res": res,
                          "density": density, "cols": len(k.colliders), "merged": merged}
            bake_input[cid] = {"bake": merged + k.pickups, "res": res, "marks": k.marks}
    for cid, s in stats.items():
        print(f"cellule {cid} : {s['tris']} triangles, {len(s['mats'])} matériaux, LOD {s['lod']} triangles, "
              f"{s['cols']} colliders, lightmap {s['res']} px ({s['density']:.1f} texels/m)")
        print(f"  matériaux : {', '.join(s['mats'])}")
    lightmaps = lm.bake_cells(bake_input, LIGHTMAPS, c.collection("BAKE_LAMPS")) if bake else {}
    return {"stats": stats, "lightmaps": lightmaps}


def verify_gltf(rel_path: str) -> None:
    """Contrôle d'un modèle exporté : aucun nœud renommé par Blender (`….001` : un collider
    `…_col.001` ne serait plus reconnu par le jeu), et chaque maillage rendu (hors `_col`) a un
    matériau, TEXCOORD_0 (textures) et TEXCOORD_1 (lightmap)."""
    import json
    import re

    data = json.loads((c.ROOT / rel_path).read_text(encoding="utf-8"))
    renamed = [n["name"] for n in data.get("nodes", []) if re.search(r"\.\d{3}$", n.get("name", ""))]
    if renamed:
        raise RuntimeError(f"{rel_path} : nœuds renommés par Blender : {renamed}")
    for node in data.get("nodes", []):
        if "mesh" not in node or node["name"].endswith("_col"):
            continue
        for prim in data["meshes"][node["mesh"]]["primitives"]:
            attrs = prim.get("attributes", {})
            missing = [a for a in ("TEXCOORD_0", "TEXCOORD_1") if a not in attrs]
            if "material" not in prim or missing:
                raise RuntimeError(f"{rel_path} : {node['name']} sans matériau ou sans {missing}")


def remove_old_glb() -> None:
    for path in (c.ROOT / MODELS).glob("*.glb"):
        path.unlink()
        print(f"supprimé : {path.relative_to(c.ROOT)}")


# --- Aperçus ------------------------------------------------------------------------------------

EYE = 1.7
# Prises de vue : nom → (pieds (x, y, z du sol), cap en degrés (0 = +Y, positif vers −X),
# tangage). Les six premières reprennent les captures de l'avant (tools/captures.sh).
SHOTS = {
    "cockpit": ((0.0, 15.2, 0.0), 0.0, 0.0),
    "cockpit_poste": ((2.6, 15.0, 0.0), 30.0, -10.0),
    "couloir": ((0.0, 1.0, 0.0), 0.0, 0.0),
    "soute": ((9.4, -11.6, 0.0), 35.0, -8.0),
    "sas": ((-2.0, -3.0, 0.0), 90.0, 0.0),
    "machines": ((0.0, -14.8, 0.0), 180.0, -5.0),
    "abordage": ((5.8, -10.5, 0.0), 0.0, 0.0),
    "couloir_arriere": ((0.0, -0.6, 0.0), 180.0, 0.0),
    "quartiers": ((-2.1, 5.0, 0.0), 72.0, -10.0),
    "carre": ((2.1, 5.6, 0.0), -62.0, -10.0),
    "cockpit_siege": ((-1.0, 19.55, 0.35), 0.0, -12.0),
    "cockpit_haut": ((2.6, 21.3, 0.8), 145.0, -14.0),
    "cockpit_console": ((-0.1, 20.35, 0.65), 28.0, -32.0),
    "sas_retour": ((-4.8, -3.0, 0.0), -90.0, -4.0),
    "soute_porte": ((6.4, -5.0, 0.0), 120.0, -6.0),
    "machines_retour": ((-2.4, -25.2, 0.0), -20.0, -6.0),
}


def _camera(feet, yaw, pitch, fov_deg=75.0):
    data = bpy.data.cameras.get("preview") or bpy.data.cameras.new("preview")
    data.sensor_fit = "VERTICAL"
    data.angle = math.radians(fov_deg)
    data.clip_start = 0.05
    data.clip_end = 200.0
    cam = bpy.data.objects.get("preview") or bpy.data.objects.new("preview", data)
    if cam.name not in bpy.context.scene.collection.objects:
        bpy.context.scene.collection.objects.link(cam)
    eye = Vector(feet) + Vector((0, 0, EYE))
    yw, pt = math.radians(yaw), math.radians(pitch)
    d = Vector((-math.sin(yw) * math.cos(pt), math.cos(yw) * math.cos(pt), math.sin(pt)))
    cam.location = eye
    cam.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = cam


def _setup_render(samples, res):
    import preview as pv

    w, h = (int(v) for v in res.split("x"))
    pv.setup_cycles(samples, w, h, exposure=float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2")))
    pv.world((0, 0, 0), 0.0)
    return pv


def door_panels(m) -> list:
    """Panneaux de porte fermés, posés comme le jeu (`doors.rs`) : à la base du portail, +Y du
    modèle vers la normale du portail (de a vers b)."""
    coll = c.collection("PREVIEW_DOORS")
    objs = []
    for i, p in enumerate(PORTALS):
        if p[2] != "Door":
            continue
        r = portal_record(*p)
        nx, ny, _ = r["normal_blender"]
        facing = math.atan2(-nx, ny)
        vis, col, _ = kit.door_panel(f"preview_door{i}", m, coll)
        for o in col:
            bpy.data.objects.remove(o, do_unlink=True)
        place(vis, (p[3], p[4], 0.0), facing)
        objs += vis
    return objs


def _shots(only):
    return [(n, s) for n, s in SHOTS.items() if not only or n in only]


def lit_previews(cells, m, samples, res, only):
    """Aperçus Cycles éclairés par les lampes (une par repère LIGHT_) et les surfaces
    émissives : la scène avant cuisson (pièces non fusionnées)."""
    pv = _setup_render(samples, res)
    doors = door_panels(m)
    shown = {o for k in cells.values() for o in k.visual + k.pickups} | set(doors)
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            obj.hide_render = obj not in shown
            if obj.data.materials and obj.data.materials[0] and obj.data.materials[0].name.startswith("sh_light"):
                obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = False
    lamps = lm.spawn_lamps([mk for k in cells.values() for mk in k.marks], c.collection("PREVIEW_LAMPS"))
    for lamp in lamps:
        lamp.data.shadow_soft_size = 0.06
    for name, (feet, yaw, pitch) in _shots(only):
        _camera(feet, yaw, pitch)
        pv.render(PREVIEWS / f"lit_{name}.png")
        print(f"aperçu : lit_{name}.png")
    for lamp in lamps:
        bpy.data.objects.remove(lamp, do_unlink=True)


def _game_material(mat, image, scale: float, tag: str):
    """Copie d'un matériau qui rend ce que le jeu affiche dans une cellule à lightmap, en Bas et
    Moyen : émission = couleur × (1 − métal) × lightmap × échelle + émissif (repris de la salle
    témoin, lot 0). `image` None : objet mobile (porte), éclairé par une ambiante constante."""
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
            socket.default_value = tuple(value)[: len(socket.default_value)] if hasattr(value, "__len__") else value

    base_src, base_val = source("Base Color")
    met_src, met_val = source("Metallic")
    em_src, em_val = source("Emission Color")
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
    kk = nodes.new("ShaderNodeMath")
    kk.operation = "MULTIPLY"
    links.new(inv.outputs[0], kk.inputs[0])
    kk.inputs[1].default_value = scale
    diffuse = nodes.new("ShaderNodeVectorMath")
    diffuse.operation = "SCALE"
    links.new(mul.outputs[0], diffuse.inputs[0])
    links.new(kk.outputs[0], next(s for s in diffuse.inputs if s.name == "Scale"))
    glow = nodes.new("ShaderNodeVectorMath")
    glow.operation = "SCALE"
    feed(glow.inputs[0], em_src, em_val)
    # Bevy affiche l'émissif sans l'exposition de la caméra : on la retire ici.
    next(s for s in glow.inputs if s.name == "Scale").default_value = strength / (2.0 ** float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2")))
    add = nodes.new("ShaderNodeVectorMath")
    add.operation = "ADD"
    links.new(diffuse.outputs[0], add.inputs[0])
    links.new(glow.outputs[0], add.inputs[1])
    emit = nodes.new("ShaderNodeEmission")
    emit.inputs["Strength"].default_value = 1.0
    links.new(add.outputs[0], emit.inputs["Color"])
    links.new(emit.outputs["Emission"], out.inputs["Surface"])
    return g


def game_previews(cells, result, m, samples, res, only):
    """Aperçus « comme dans le jeu » (Bas et Moyen, cellules à lightmap) : émission seule, sans
    lampe ; portes éclairées par une ambiante constante."""
    pv = _setup_render(samples, res)
    bpy.context.scene.cycles.max_bounces = 0
    for obj in list(bpy.context.scene.objects):
        if obj.type == "LIGHT":
            obj.hide_render = True
    doors = door_panels(m)
    shown = set(doors)
    for cid, s in result["stats"].items():
        info = result["lightmaps"][cid]
        image = bpy.data.images.load(str(c.ROOT / "assets" / info["image"]), check_existing=True)
        image.colorspace_settings.name = "sRGB"
        scale = info["exposure"] / lm.WATTS_TO_LUMENS
        for obj in s["merged"] + cells[cid].pickups:
            shown.add(obj)
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = True
            mats = obj.data.materials
            for i, mat in enumerate(mats):
                mats[i] = _game_material(mat, image, scale, cid)
    ambient = float(os.environ.get("SH_PREVIEW_DOOR_AMBIENT", "0.35"))
    for obj in doors:
        mats = obj.data.materials
        for i, mat in enumerate(mats):
            mats[i] = _game_material(mat, None, ambient, "door")
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            obj.hide_render = obj not in shown
    for name, (feet, yaw, pitch) in _shots(only):
        _camera(feet, yaw, pitch)
        pv.render(PREVIEWS / f"jeu_{name}.png")
        print(f"aperçu jeu : jeu_{name}.png")


def main(argv) -> None:
    do_export = "--no-export" not in argv
    bake = do_export and "--no-bake" not in argv
    samples = int(argv[argv.index("--samples") + 1]) if "--samples" in argv else 48
    res = argv[argv.index("--res") + 1] if "--res" in argv else "1280x720"
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    c.reset_scene()
    m = palette()
    cells = build_all(m)
    for cid, k in cells.items():
        print(f"construit {cid} : {_tris(k.visual + k.pickups)} triangles, {len(k.colliders)} colliders, {len(k.marks)} repères")
    result = {"stats": {}, "lightmaps": {}}
    if do_export:
        result = export(cells, bake)
    if bake:
        c.write_text(f"assets/data/propositions/sectors/{SECTOR}.ron", sector_ron(result["lightmaps"]))
        remove_old_glb()
    c.save_blend(f"art/ships/{SECTOR}.blend")
    PREVIEWS.mkdir(parents=True, exist_ok=True)
    if "--lit" in argv:
        lit_previews(cells, m, samples, res, only)
    if "--game" in argv and result["lightmaps"]:
        game_previews(cells, result, m, max(8, samples // 2), res, only)
    print(f"{SECTOR} : {len(CELLS)} cellules, {len(PORTALS)} portails")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
