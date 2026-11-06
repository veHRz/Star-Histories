"""Intérieur type des corvettes (proposition P12 ; M9, lot B, docs/m9-retours.md).

Le vaisseau du joueur, « petit marchand de taille corvette », utilise cet intérieur ; d'autres
vaisseaux de la classe pourront le réutiliser. Il reprend le vaisseau de M8
(`ships/player_ship.py`, gardé tel quel tant que le jeu s'en sert) et répond aux retours de
l'auteur :
- cockpit : sièges près d'une grande verrière panoramique (vitrage frontal de 7 × 1,85 m et deux
  vitrages latéraux, cadres seulement sur les bords, aucun meneau), planche de bord basse sous la
  vitre, sans estrade ;
- sas : une vraie salle de passage, porte intérieure (couloir) et porte extérieure (`sas__@dock`,
  sur le flanc bâbord de l'enveloppe), éclairage d'alerte, bandes de danger, râtelier de
  combinaisons ;
- chambre du capitaine (départ du tutoriel) : couchette, trois casiers fonctionnels
  (`corvette_chambre_casier_1` à `_3`), bureau mural et la tablette (`datapad`), étagère, hublots ;
- salle des machines : console de commande plaquée au mur ;
- noms de salle (`meta["rooms"]`) et décor par niveau de qualité (`detail` : 0 sobre, 1 habillage
  moyen, 2 très riche ; aucun décor optionnel n'a de collision ni de lampe cuite).

M10 (lot B, docs/m10.md §1.2) : plus de sacs sur les couchettes ni sur la malle, plus de linge
plié sur le lit ; frigo du kit (`corvette_carre_frigo`) à la place de la borne de la cuisine et de
la chambre froide ; porte blindée du kit au sas (`meta["door_models"]`, portail `sas__@dock`
seulement) ; casiers décoratifs « verrouillés » (volume `INTERACT_` du kit, noms stables) et
plaques de nom sur les casiers de la chambre ; tableaux électriques du kit (gaines au plafond).

Plan (repère Blender : X = tribord, Y = avant, Z = haut ; pont à z = 0) :

    cockpit                      x −4 → 4,     y 14,4 → 20,4  (verrière au nord)
    couloir_avant                x −1,2 → 1,2, y 0 → 14
      chambre (bâbord)           x −7,6 → −1,6, y 8 → 14
      quartiers (bâbord)         x −7,6 → −1,6, y 1,6 → 7,6
      carre (tribord)            x 1,6 → 7,6,  y 1,6 → 9,6
    couloir_arriere              x −1,2 → 1,2, y −14 → 0
      sas (bâbord)               x −7,6 → −1,6, y −4,8 → −1,2  (porte extérieure à x = −7,7)
      soute (tribord)            x 1,6 → 7,6,  y −13,4 → −1,6
    machines                     x −5 → 5,     y −26 → −14,4

Gameplay : identifiants de cellules et de portails repris de `player_ship` (scénarios, abordage
dans `couloir_arriere`) ; apparitions `cockpit`, `soute`, `sas`, `chambre` ; grille de 2 m,
portes de 1,4 × 2,2 m ; `_col` en pavés ; volumes d'interaction (`half_extents`) sur sièges,
consoles et casiers.

Sorties (glTF séparé, textures de la bibliothèque citées par chemin) :
- assets/models/ships/corvette/<cellule>.gltf (+ .bin) et <cellule>_lod.gltf ;
- assets/textures/lightmaps/corvette/<cellule>.png ;
- assets/data/propositions/sectors/corvette.ron (généré) ; art/ships/corvette.blend.

    blender --background --factory-startup --python tools/blender/ships/corvette.py -- [options]

Options : `--no-bake` (exporte sans cuire), `--no-export` (ni export ni bake : aperçus
seulement), `--lit` (aperçus Cycles éclairés par les lampes), `--game` (aperçus « comme dans le
jeu » après un bake), `--only a,b` (prises de vue), `--samples N`, `--res LxH`. Aperçus dans
target/art-previews/m9-lotB/ ; `<vue>_n0.png` / `_n1` / `_n2` : la même vue au niveau de décor
0 (Bas), 1 (Moyen) ou 2 (Haut). Bake final : 512 échantillons (défaut de `lightmap.py`) ;
`SH_BAKE_SAMPLES=64` pour un aperçu. `SH_PREVIEW_DIR=<dossier>` et `SH_PREVIEW_PREFIX=<préfixe>`
(M10) : dossier et préfixe des aperçus (avant / après sous les mêmes angles).
"""

from __future__ import annotations

import math
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "kit"))
sys.path.insert(0, str(HERE.parent / "materials"))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import common as c  # noqa: E402
import lightmap as lm  # noqa: E402
import sector_tools as st  # noqa: E402
import ship_interior as kit  # noqa: E402

SECTOR = "corvette"
MODELS = f"models/ships/{SECTOR}"
LIGHTMAPS = f"textures/lightmaps/{SECTOR}"
DOOR_MODEL = "models/kit/ship_interior/door_sliding.gltf"
RON_PATH = f"assets/data/propositions/sectors/{SECTOR}.ron"
BLEND_PATH = f"art/ships/{SECTOR}.blend"
PREVIEWS = c.ROOT / "target" / "art-previews" / "m9-lotB"
CORRIDOR_H = 2.6
PI = math.pi

# Cellules : id → (x0, x1, y0, y1, hauteur sous plafond), intérieurs ; cloisons de 0,2 m à
# l'extérieur de ces bornes.
CELLS = {
    "machines": (-5.0, 5.0, -26.0, -14.4, 3.4),
    "couloir_arriere": (-1.2, 1.2, -14.0, 0.0, CORRIDOR_H),
    "sas": (-7.6, -1.6, -4.8, -1.2, CORRIDOR_H),
    "soute": (1.6, 7.6, -13.4, -1.6, 3.4),
    "couloir_avant": (-1.2, 1.2, 0.0, 14.0, CORRIDOR_H),
    "quartiers": (-7.6, -1.6, 1.6, 7.6, 2.8),
    "chambre": (-7.6, -1.6, 8.0, 14.0, 2.8),
    "carre": (1.6, 7.6, 1.6, 9.6, 2.8),
    "cockpit": (-4.0, 4.0, 14.4, 20.4, 3.0),
}

# Noms de salle (docs/m9-retours.md §3.5).
ROOMS = {
    "machines": "engine_room",
    "couloir_arriere": "corridor",
    "sas": "airlock",
    "soute": "cargo_hold",
    "couloir_avant": "corridor",
    "quartiers": "crew_quarters",
    "chambre": "cabin",
    "carre": "mess",
    "cockpit": "cockpit",
}

# Portails : (cellule a, cellule b, genre, x, y, axe) ; axe = mur traversé ("x" : mur
# perpendiculaire à X).
PORTALS = [
    ("couloir_arriere", "machines", "Door", 0.0, -14.2, "y"),
    ("couloir_arriere", "soute", "Door", 1.4, -7.0, "x"),
    ("couloir_arriere", "sas", "Door", -1.4, -3.0, "x"),
    ("couloir_arriere", "couloir_avant", "Opening", 0.0, 0.0, "y"),
    ("couloir_avant", "quartiers", "Door", -1.4, 4.6, "x"),
    ("couloir_avant", "carre", "Door", 1.4, 5.6, "x"),
    ("couloir_avant", "chambre", "Door", -1.4, 11.0, "x"),
    ("couloir_avant", "cockpit", "Door", 0.0, 14.2, "y"),
    # Porte extérieure du sas, sur le flanc bâbord : point d'attache « @dock » (verrouillée en
    # vol ; à quai, elle s'ouvre sur la passerelle du hangar).
    ("sas", "@dock", "Door", -7.7, -3.0, "x"),
]

# Verrière du cockpit : (côté, position, largeur, hauteur, allège). Vitrage frontal de 7 m sur
# presque toute la largeur, deux vitrages latéraux à l'avant des flancs ; même allège (0,85 m,
# juste au-dessus de la planche de bord) et même linteau (2,7 m).
WINDOWS = {
    "cockpit": [("N", 0.0, 7.0, 1.85, 0.85), ("E", 19.3, 1.4, 1.85, 0.85), ("W", 19.3, 1.4, 1.85, 0.85)],
}

SPAWNS = [
    # (id, cellule, x, y, z, cap en degrés autour de Z : 0 = vers l'avant +Y, −90 = vers +X)
    ("sas", "sas", -5.4, -3.0, 0.0, -90.0),
    ("cockpit", "cockpit", 0.0, 15.3, 0.0, 0.0),
    ("soute", "soute", 5.2, -10.8, 0.0, 0.0),
    # Départ du tutoriel : près du lit, tourné vers la porte et les casiers.
    ("chambre", "chambre", -5.3, 11.6, 0.0, -120.0),
]

META = {
    "script": "tools/blender/ships/corvette.py",
    "comment": "Intérieur type des corvettes (proposition P12) : cellules, portails, apparitions.",
    "id": SECTOR,
    "kind": "Ship",
    "name_key": f"sector.{SECTOR}",
    "status": 'Proposition("P12")',
    "door_model": DOOR_MODEL,
    "models": MODELS,
    "lightmaps": LIGHTMAPS,
    "rooms": ROOMS,
    # M10 (lot B) : la porte extérieure du sas (donne sur le vide en vol) est la porte blindée du
    # kit ; les autres portes gardent `door_model` du secteur. Id calculé par `portal_record`
    # (« @ » retiré), réécrit « sas__@dock » par `write_ron`.
    "door_models": {"sas__dock": st.AIRLOCK_DOOR},
    # M11 (streaming) : ce qui se voit par une vitre (`sector_tools.sector_ron`, champ `windows`).
    # Seule la verrière du cockpit (WINDOWS : vitrages nord, est, ouest, tous sur la coque) donne
    # sur une autre zone, l'extérieur ; les hublots de la chambre sont opaques (verre fumé du
    # kit), le verre du carré (carafe) et les visières restent dans leur cellule.
    "windows": {"cockpit": ["@outside"]},
}

COOL = [0.78, 0.88, 1.0]
COCKPIT_WHITE = [0.86, 0.93, 1.0]
WARM = [1.0, 0.8, 0.58]
SODIUM = [1.0, 0.55, 0.22]
GREEN = [0.2, 1.0, 0.35]
AMBER = [1.0, 0.6, 0.15]
RED = list(kit.RED)


def palette() -> dict:
    """Matériaux du kit et variantes du vaisseau (aucune texture nouvelle) : lentille chaude,
    bandeaux bleu (cockpit), ambré doux (salles de vie) et rouge (alerte du sas), cuits ; tôle
    peinte crème des salles de vie, voyants vert et ambre, tapis, toile des combinaisons,
    peinture rouge (extincteurs), feuillage (plante du capitaine)."""
    lib = c.material_library()
    m = kit.palette()
    m["light_warm"] = c.material("sh_light_warm", (1.0, 0.9, 0.75), emission=(1.0, 0.78, 0.5), emission_strength=4.0)
    m["band_blue"] = c.textured_material("sh_band_blue", "light_band", emissive=(0.22, 0.5, 1.0), emissive_strength=2.6)
    m["band_hearth"] = c.textured_material("sh_band_hearth", "light_band", emissive=(1.0, 0.6, 0.3), emissive_strength=3.0)
    m["band_red"] = c.textured_material("sh_band_red", "light_band", emissive=(1.0, 0.14, 0.07), emissive_strength=3.2)
    m["panel_warm"] = c.textured_material("sh_panel_warm", "panel_painted", tint=(0.96, 0.86, 0.72))
    m["beacon_green"] = c.material("sh_beacon_green", (0.04, 0.45, 0.1), emission=(0.12, 1.0, 0.3), emission_strength=6.0)
    m["beacon_amber"] = c.material("sh_beacon_amber", (0.6, 0.32, 0.04), emission=(1.0, 0.55, 0.12), emission_strength=6.0)
    m["rug"] = c.textured_material("sh_fabric_rug", "fabric", tint=(0.36, 0.2, 0.15))
    m["suit"] = c.textured_material("sh_suit", "fabric", tint=(0.88, 0.5, 0.2))
    m["paint_red"] = c.textured_material("sh_paint_red", "panel_painted", tint=lib.TINTS["pirate_red"])
    m["plant"] = c.material("sh_plant", (0.10, 0.26, 0.08), roughness=0.75)
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


def _frame(right, up):
    r, u = Vector(right).normalized(), Vector(up).normalized()
    n = r.cross(u).normalized()
    u = n.cross(r).normalized()
    return Matrix((r, u, n)).transposed().to_4x4()


class Cell:
    """Collecte des objets d'une cellule : visibles, structure (version simplifiée),
    collisions, repères, objets ramassables. `detail` (M9) : niveau de décor des objets ajoutés
    (0 : toujours ; 1 : Moyen et Haut ; 2 : Haut) ; un décor optionnel n'a jamais de collider et
    ne va pas dans la version simplifiée."""

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

    def add(self, result, structure=False, detail=0):
        """Triplet du kit (visibles, colliders, repères). `detail` > 0 : décor optionnel (marqué,
        colliders supprimés ; seul un repère `LIGHT_` peut l'accompagner)."""
        vis, col, marks = result
        if detail:
            bad = [mk.name for mk in marks if not mk.name.startswith("LIGHT_")]
            if bad:
                raise ValueError(f"{self.id} : repères de gameplay dans un décor optionnel : {bad}")
            c.set_detail(vis + list(marks), detail)
            for o in col:
                bpy.data.objects.remove(o, do_unlink=True)
            col = []
        self.visual += vis
        if structure and not detail:
            self.structure += vis
        self.colliders += col
        self.marks += marks
        return vis

    def shell(self, **kw):
        parts = kit.room_shell(self.id, self.x0, self.x1, self.y0, self.y1, 0.0, self.h, openings_for(self.id), self.m, self.coll, **kw)
        self.visual += parts["visual"]
        self.structure += parts["structure"]
        self.colliders += parts["colliders"]

    def obj(self, o, uv="box", tile=None, structure=False, detail=0):
        _uv(o, uv, tile)
        if detail:
            c.set_detail([o], detail)
        self.visual.append(o)
        if structure and not detail:
            self.structure.append(o)
        return o

    def box(self, tag, size, center, mat, uv="box", bevel=0.0, tile=None, structure=False, detail=0):
        return self.obj(c.box(self.nm(tag), size, center, self.mat(mat), self.coll, bevel=bevel), uv, tile, structure, detail)

    def cyl(self, tag, radius, depth, center, mat, axis="Z", segments=12, uv="box", tile=None, detail=0):
        o = c.cylinder(self.nm(tag), radius, depth, center, self.mat(mat), self.coll, axis=axis, segments=segments)
        return self.obj(o, uv, tile, detail=detail)

    def ori(self, tag, size, center, right, up, mat, uv="box", bevel=0.0, tile=None, structure=False, detail=0):
        """Pavé d'axes locaux (right, up, right × up) : la face +Z regarde vers right × up.
        Face vers +Y : right = (−1, 0, 0), up = (0, 0, 1) ; vers −Y : right = (1, 0, 0)."""
        o = c.box(self.nm(tag), size, (0, 0, 0), self.mat(mat), self.coll, bevel=bevel)
        o.matrix_basis = Matrix.Translation(Vector(center)) @ _frame(right, up)
        return self.obj(o, uv, tile, structure, detail)

    def rod(self, tag, p0, p1, radius, mat, segments=8, uv="box", detail=0):
        """Cylindre de p0 à p1 (câbles, tiges, barres de maintien)."""
        p0, p1 = Vector(p0), Vector(p1)
        d = p1 - p0
        o = c.cylinder(self.nm(tag), radius, d.length, (0, 0, 0), self.mat(mat), self.coll, segments=segments)
        t = d.normalized()
        side = t.cross(Vector((0, 0, 1)))
        if side.length < 1e-4:
            side = Vector((1, 0, 0))
        side.normalize()
        up = t.cross(side).normalized()
        o.matrix_basis = Matrix.Translation((p0 + p1) / 2) @ Matrix((side, up, t)).transposed().to_4x4()
        return self.obj(o, uv, detail=detail)

    def sphere(self, tag, radius, center, mat, scale=(1.0, 1.0, 1.0), segments=12, rings=8, detail=0):
        mesh = bpy.data.meshes.new(self.nm(tag))
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=segments, v_segments=rings, radius=radius)
        for v in bm.verts:
            v.co = Vector((v.co.x * scale[0], v.co.y * scale[1], v.co.z * scale[2]))
        bm.to_mesh(mesh)
        bm.free()
        mesh.materials.append(self.mat(mat))
        o = bpy.data.objects.new(mesh.name, mesh)
        o.location = center
        self.coll.objects.link(o)
        return self.obj(o, detail=detail)

    def extrude(self, tag, section, axis, s0, s1, mat, structure=False, detail=0):
        """Prisme : polygone `section` (points 3D pris à s = 0) extrudé le long de `axis`."""
        axis = Vector(axis)
        mesh = bpy.data.meshes.new(self.nm(tag))
        bm = bmesh.new()
        lo = [bm.verts.new(Vector(p) + axis * s0) for p in section]
        hi = [bm.verts.new(Vector(p) + axis * s1) for p in section]
        bm.faces.new(lo)
        bm.faces.new(list(reversed(hi)))
        n = len(section)
        for i in range(n):
            j = (i + 1) % n
            bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        bm.to_mesh(mesh)
        bm.free()
        mesh.materials.append(self.mat(mat))
        o = bpy.data.objects.new(mesh.name, mesh)
        self.coll.objects.link(o)
        return self.obj(o, structure=structure, detail=detail)

    def col(self, tag, size, center):
        self.colliders.append(c.collider_box(f"{self.id}_{tag}", size, center, self.coll))

    def lamp(self, tag, x, y, lumens, rng, color, lens="light", facing=0.0, drop=0.0, shadow=False):
        props = {"lumens": lumens, "range": rng, "color": list(color), "shadow_candidate": int(shadow)}
        self.add(kit.ceiling_light(f"{self.id}_{tag}", (x, y, self.h), facing, self.m, self.coll, props, lens=lens, drop=drop))

    def pickup(self, result):
        """Objet ramassable rattaché à son repère (`attach_pickup`) : jamais fusionné avec la
        cellule, sans ombre au bake, remplacé en jeu par le modèle de l'objet."""
        joined, marks = kit.attach_pickup(result)
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

    @staticmethod
    def right(side):
        """« Droite » d'un observateur qui regarde ce mur depuis la pièce (pour `ori`, face vers
        la pièce : right = Cell.right(side), up = (0, 0, 1))."""
        n = Vector(Cell.inward(side))
        return tuple((-n).cross(Vector((0, 0, 1))))


def openings_for(cell: str) -> list[tuple]:
    """Ouvertures de la coque d'une cellule, déduites des portails (et ses fenêtres)."""
    return st.openings_for(cell, CELLS, PORTALS, WINDOWS)


# --- Pièces d'habillage propres au vaisseau -----------------------------------------------------


def chamfer_bands(k: Cell, tone: str, tc: float = 0.35, detail: int = 0):
    """Bandeaux d'angle sous les pans coupés hauts d'une salle `ship`, dans une autre teinte que
    ceux du kit (`room_shell(bands=False)`), interrompus au droit des portes et des fenêtres."""
    nrm_of = {s: (Vector(Cell.inward(s)) * 0.92 + Vector((0, 0, -0.38))).normalized() for s in "NSEW"}
    for side in "NSEW":
        lo, hi = (k.x0, k.x1) if side in "NS" else (k.y0, k.y1)
        gaps = [(o[1] - o[2] / 2 - 0.3, o[1] + o[2] / 2 + 0.3) for o in openings_for(k.id) if o[0] == side]
        for sa, sb in _cuts(lo + 0.45, hi - 0.45, gaps):
            if sb - sa < 0.5:
                continue
            p0, p1 = k.wall(side, sa, 0.0, k.h - tc), k.wall(side, sb, 0.0, k.h - tc)
            k.add(kit.light_band(k.nm("band"), p0, p1, k.m, k.coll, normal=tuple(nrm_of[side]), tone=tone, detail=detail))


def status_lamp(k: Cell, tag, center, facing, mat, color, lumens=300.0, rng=3.0, light=True):
    """Voyant mural (gyrophare fixe) : embase sombre, globe émissif, cage ; repère LIGHT_."""
    vis = [
        c.cylinder(k.nm(f"{tag}_base"), 0.07, 0.06, (0, 0.03, 0), k.m["dark"], k.coll, axis="Y", segments=12),
        c.cylinder(k.nm(tag), 0.05, 0.1, (0, 0.11, 0), k.mat(mat), k.coll, axis="Y", segments=12),
    ]
    for i in range(2):
        a = i * PI / 2 + PI / 4
        o = c.box(k.nm(f"{tag}_cage"), (0.014, 0.13, 0.014), (0, 0.0, 0), k.m["dark"], k.coll)
        o.matrix_basis = Matrix.Translation(Vector((0, 0.12, 0))) @ _frame((math.cos(a), 0, math.sin(a)), (0, 1, 0))
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


def cargo_net(k: Cell, center, rot, size=1.2, detail=1):
    """Filet d'arrimage sur une caisse : sangles de caoutchouc croisées sur la face −X et
    par-dessus, crochets au sol. Collé à la caisse : pas de collider propre."""
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
    bpy.context.view_layer.update()
    place(objs, center, rot)
    for o in objs:
        k.obj(o, detail=detail)


def wall_conduits(k: Cell, z, d=0.2, sides="NSEW", skip=(), detail=2, radius=0.022):
    """Deux gaines fines à colliers le long des murs, sous les pans coupés (décor optionnel) :
    interrompues au droit des portes et des fenêtres (`openings_for`) et sur `skip` =
    [(côté, a, b)] ; à `d` du mur, en avant des nervures et des linteaux."""
    for side in sides:
        lo, hi = (k.x0, k.x1) if side in "NS" else (k.y0, k.y1)
        gaps = [(o[1] - o[2] / 2 - 0.3, o[1] + o[2] / 2 + 0.3) for o in openings_for(k.id) if o[0] == side]
        gaps += [(a, b) for sd, a, b in skip if sd == side]
        for sa, sb in _cuts(lo + 0.25, hi - 0.25, gaps):
            if sb - sa < 0.6:
                continue
            for dz, dd, mat, r in ((0.0, d, "bare", radius), (-0.07, d - 0.035, "rubber", radius * 0.8)):
                k.add(kit.pipe(k.nm("conduit"), k.wall(side, sa, dd, z + dz), k.wall(side, sb, dd, z + dz), k.m, k.coll,
                               radius=r, clamps=0.8, mat=mat, detail=detail))


def hanging_cable(k: Cell, p0, p1, sag=0.25, radius=0.014, mat="rubber", n=6, detail=2):
    """Câble qui pend entre deux points (chaînette approchée par `n` segments)."""
    p0, p1 = Vector(p0), Vector(p1)
    pts = [p0.lerp(p1, i / n) - Vector((0, 0, 4 * sag * (i / n) * (1 - i / n))) for i in range(n + 1)]
    for a, b in zip(pts[:-1], pts[1:]):
        k.rod("cable", a, b, radius, mat, segments=6, detail=detail)


def mug(k: Cell, x, y, z, mat="crate_slate", rot=0.0, detail=2):
    k.cyl("mug", 0.04, 0.095, (x, y, z + 0.0475), mat, segments=10, detail=detail)
    hx, hy = 0.05 * math.cos(rot), 0.05 * math.sin(rot)
    k.box("mug_handle", (0.02, 0.02, 0.05), (x + hx, y + hy, z + 0.05), mat, detail=detail)


def bottle(k: Cell, x, y, z, mat="crate_slate", h=0.24, detail=2):
    k.cyl("bottle", 0.036, h * 0.7, (x, y, z + h * 0.35), mat, segments=10, detail=detail)
    k.cyl("bottle_neck", 0.015, h * 0.3, (x, y, z + h * 0.85), mat, segments=8, detail=detail)
    k.cyl("bottle_cap", 0.017, 0.02, (x, y, z + h + 0.01), "dark", segments=8, detail=detail)


def slates(k: Cell, x, y, z, rot=0.0, count=3, detail=2):
    """Pile de tablettes de données et de cartes (sans inscription)."""
    for i in range(count):
        o = k.box("slate", (0.2, 0.14, 0.012), (x, y, z + 0.006 + i * 0.013), "dark" if i % 2 else "crate_slate", bevel=0.003, detail=detail)
        o.rotation_euler[2] = rot + 0.18 * (i - 1)
    k.ori("slate_screen", (0.15, 0.09, 0.002), (x, y, z + 0.013 * count + 0.001), (math.cos(rot), math.sin(rot), 0), (-math.sin(rot), math.cos(rot), 0), "screen", tile=0.4, detail=detail)


def boots(k: Cell, x, y, rot=0.0, detail=2):
    """Paire de bottes posées au sol."""
    for i, dx in enumerate((-0.08, 0.08)):
        objs = [
            c.box(k.nm("boot_sole"), (0.11, 0.29, 0.04), (dx, 0.02, 0.02), k.m["rubber"], k.coll),
            c.box(k.nm("boot_toe"), (0.1, 0.14, 0.08), (dx, 0.08, 0.07), k.m["fabric_dark"], k.coll, bevel=0.02),
            c.box(k.nm("boot_shaft"), (0.1, 0.13, 0.26), (dx, -0.05, 0.16), k.m["fabric_dark"], k.coll, bevel=0.02),
        ]
        bpy.context.view_layer.update()
        place(objs, (x, y, 0.0), rot + 0.12 * (i - 0.5))
        for o in objs:
            k.obj(o, detail=detail)


def plant(k: Cell, x, y, z, detail=2):
    """Plante en pot du capitaine : pot sombre, terre, feuillage en touffes."""
    k.cyl("pot", 0.075, 0.13, (x, y, z + 0.065), "dark", segments=12, detail=detail)
    k.cyl("soil", 0.068, 0.01, (x, y, z + 0.128), "rubber", segments=12, detail=detail)
    for i, (dx, dy, dz, s) in enumerate(((0, 0, 0.2, 0.09), (0.05, 0.03, 0.17, 0.07), (-0.05, -0.02, 0.16, 0.07), (0.01, -0.05, 0.25, 0.06))):
        k.sphere("leaves", s, (x + dx, y + dy, z + dz), "plant", scale=(1.0, 0.8, 1.3), segments=8, rings=6, detail=detail)


def extinguisher(k: Cell, side, s, z=0.55, detail=2):
    """Extincteur mural rouge sur son étrier (sans inscription)."""
    x, y, _ = k.wall(side, s, 0.0, 0.0)
    n = Vector(Cell.inward(side))
    p = Vector((x, y, 0)) + n * 0.1
    k.box("ext_bracket", (0.12, 0.12, 0.04) if side in "EW" else (0.12, 0.12, 0.04), tuple(Vector((x, y, z + 0.15)) + n * 0.05), "dark", detail=detail)
    k.cyl("ext_body", 0.065, 0.42, (p.x, p.y, z + 0.21), "paint_red", segments=12, detail=detail)
    k.cyl("ext_head", 0.03, 0.08, (p.x, p.y, z + 0.46), "dark", segments=8, detail=detail)
    k.box("ext_handle", (0.1, 0.03, 0.02), (p.x, p.y, z + 0.51), "bare", detail=detail)
    k.rod("ext_hose", (p.x, p.y, z + 0.46), tuple(p + n * 0.07 + Vector((0, 0, z + 0.18))), 0.01, "rubber", detail=detail)


def jacket(k: Cell, side, s, z=1.85, detail=2):
    """Veste suspendue à une patère murale."""
    x, y, _ = k.wall(side, s, 0.0, 0.0)
    n = Vector(Cell.inward(side))
    r = Vector(Cell.right(side))
    base = Vector((x, y, 0))
    k.box("hook", (0.04, 0.04, 0.04) if side in "NS" else (0.04, 0.04, 0.04), tuple(base + n * 0.03 + Vector((0, 0, z))), "bare", detail=detail)
    k.ori("jacket", (0.42, 0.62, 0.07), tuple(base + n * 0.06 + Vector((0, 0, z - 0.36))), tuple(r), (0, 0, 1), "fabric_dark", bevel=0.025, detail=detail)
    k.ori("jacket_collar", (0.24, 0.08, 0.08), tuple(base + n * 0.07 + Vector((0, 0, z - 0.04))), tuple(r), (0, 0, 1), "fabric", bevel=0.02, detail=detail)
    for sx in (-1, 1):
        k.ori("jacket_sleeve", (0.1, 0.55, 0.07), tuple(base + n * 0.07 + r * (sx * 0.24) + Vector((0, 0, z - 0.4))), tuple(r), (0, 0, 1), "fabric_dark", bevel=0.02, detail=detail)


def gauge(k: Cell, side, s, z, d=0.0, radius=0.07, detail=1):
    """Manomètre mural : boîtier rond, cadran lumineux."""
    x, y, _ = k.wall(side, s, d, 0.0)
    n = Vector(Cell.inward(side))
    base = Vector((x, y, z))
    ax = "Y" if side in "NS" else "X"
    k.cyl("gauge", radius, 0.05, tuple(base + n * 0.025), "dark", axis=ax, segments=12, detail=detail)
    k.cyl("gauge_dial", radius * 0.8, 0.012, tuple(base + n * 0.052), "buttons", axis=ax, segments=12, tile=0.3, detail=detail)


def valve_wheel(k: Cell, center, axis="Y", radius=0.14, detail=2):
    """Volant de vanne : jante en huit segments, rayons, moyeu (rouge)."""
    ctr = Vector(center)
    if axis == "Y":
        u, v = Vector((1, 0, 0)), Vector((0, 0, 1))
    elif axis == "X":
        u, v = Vector((0, 1, 0)), Vector((0, 0, 1))
    else:
        u, v = Vector((1, 0, 0)), Vector((0, 1, 0))
    pts = [ctr + (u * math.cos(2 * PI * i / 8) + v * math.sin(2 * PI * i / 8)) * radius for i in range(9)]
    for a, b in zip(pts[:-1], pts[1:]):
        k.rod("wheel_rim", a, b, 0.014, "paint_red", segments=6, detail=detail)
    for i in range(0, 8, 2):
        k.rod("wheel_spoke", ctr, pts[i], 0.009, "dark", segments=6, detail=detail)


def gas_tank(k: Cell, x, y, z, radius=0.11, h=0.8, mat="paint_yellow", detail=1):
    """Bouteille de gaz (oxygène, azote) debout : corps, épaule, robinet."""
    k.cyl("tank", radius, h, (x, y, z + h / 2), mat, segments=12, detail=detail)
    k.cyl("tank_top", radius * 0.7, 0.06, (x, y, z + h + 0.03), "bare", segments=12, detail=detail)
    k.cyl("tank_valve", 0.025, 0.08, (x, y, z + h + 0.1), "dark", segments=8, detail=detail)


def barrel(k: Cell, x, y, z, mat="crate_slate", radius=0.29, h=0.88, detail=0):
    k.cyl("barrel", radius, h, (x, y, z + h / 2), mat, segments=14, detail=detail)
    for zr in (0.15, h - 0.15):
        k.cyl("barrel_ring", radius + 0.012, 0.04, (x, y, z + zr), "dark", segments=14, detail=detail)
    k.cyl("barrel_cap", 0.04, 0.02, (x + radius * 0.5, y, z + h + 0.01), "dark", segments=8, detail=detail)


def sideboard(k: Cell, tag, center, facing, size=(1.2, 0.42, 0.85), drawers=3, front="panel", detail=0):
    """Commode / buffet contre un mur (`center` au pied du mur, la pièce vers +Y) : caisson
    sombre, façades peintes en tiroirs, poignées de métal brut, plateau à chant ; collider."""
    W, D, H = size
    objs = [
        c.box(k.nm(f"{tag}_body"), (W, D - 0.02, H - 0.08), (0, D / 2 - 0.01, 0.08 + (H - 0.08) / 2), k.m["dark"], k.coll, bevel=0.01),
        c.box(k.nm(f"{tag}_kick"), (W - 0.06, D - 0.08, 0.08), (0, D / 2 - 0.04, 0.04), k.m["rubber"], k.coll),
        c.box(k.nm(f"{tag}_top"), (W + 0.02, D + 0.01, 0.03), (0, D / 2, H + 0.015), k.m["dark"], k.coll, bevel=0.006),
        c.box(k.nm(f"{tag}_edge"), (W + 0.03, 0.02, 0.035), (0, D + 0.005, H + 0.012), k.m["bare"], k.coll),
    ]
    dh = (H - 0.14) / drawers
    for i in range(drawers):
        z = 0.1 + dh * (i + 0.5)
        o = c.box(k.nm(f"{tag}_front"), (W - 0.06, 0.016, dh - 0.03), (0, D - 0.002, z), k.m[front], k.coll, bevel=0.003)
        c.uv_box(o, None, offset=(0.31 * i, 0.17 * i))
        objs.append(o)
        objs.append(c.box(k.nm(f"{tag}_pull"), (0.18, 0.02, 0.025), (0, D + 0.016, z + dh * 0.2), k.m["bare"], k.coll))
    cols = [] if detail else [c.collider_box(k.nm(f"{tag}"), (W, D, H + 0.03), (0, D / 2, (H + 0.03) / 2), k.coll)]
    bpy.context.view_layer.update()
    place(objs + cols, center, facing)
    for o in objs:
        k.obj(o, uv=None if o.data.uv_layers else "box", detail=detail)
    k.colliders += cols


def suit_rack(k: Cell, xc, yw, count=3, pitch=0.8):
    """Râtelier de combinaisons contre le mur nord (`yw` : face du mur, la pièce vers −Y) : dos,
    montants, tablette des casques, barre, bac à bottes ; combinaisons orange suspendues (torse,
    plastron, dos de survie, bras, gants, jambes, bottes) et leurs casques à visière fumée. Un
    collider pour tout le râtelier."""
    W = count * pitch + 0.2
    yb = yw - 0.1  # dos du râtelier, en avant des nervures du mur
    depth = 0.46
    yf = yb - depth
    k.box("rack_back", (W, 0.02, 2.1), (xc, yb - 0.01, 1.15), "dark")
    for sx in (-1, 1):
        k.box("rack_post", (0.05, depth, 2.15), (xc + sx * (W / 2 - 0.025), yb - depth / 2, 1.075), "dark", bevel=0.008)
    k.box("rack_shelf", (W, depth, 0.03), (xc, yb - depth / 2, 1.95), "dark", bevel=0.006)
    k.box("rack_lip", (W, 0.015, 0.05), (xc, yf + 0.008, 1.965), "bare")
    k.ori("rack_label", (0.6, 0.08, 0.004), (xc - W / 4, yf - 0.001, 1.93), (1, 0, 0), (0, 0, 1), "trim_sheet", uv="stencils")
    k.rod("rack_rail", (xc - W / 2 + 0.05, yb - 0.24, 1.8), (xc + W / 2 - 0.05, yb - 0.24, 1.8), 0.016, "bare")
    k.box("rack_tray", (W - 0.1, depth - 0.04, 0.05), (xc, yb - depth / 2, 0.025), "rubber")
    k.ori("rack_tray_edge", (W - 0.1, 0.05, 0.006), (xc, yf + 0.02 - 0.001, 0.04), (1, 0, 0), (0, 0, 1), "trim_sheet", uv="hazard")
    ys = yb - 0.24
    for i in range(count):
        x = xc + (i - (count - 1) / 2) * pitch
        k.box("suit_hook", (0.012, 0.012, 0.05), (x, ys, 1.775), "bare")
        k.box("suit_hanger", (0.42, 0.02, 0.02), (x, ys, 1.74), "bare")
        k.cyl("suit_collar", 0.12, 0.05, (x, ys, 1.66), "bare", segments=14)
        k.box("suit_torso", (0.46, 0.24, 0.6), (x, ys, 1.33), "suit", bevel=0.07)
        k.box("suit_chest", (0.3, 0.05, 0.2), (x, ys - 0.13, 1.42), "dark", bevel=0.012)
        k.ori("suit_chest_btn", (0.2, 0.09, 0.006), (x, ys - 0.158, 1.42), (1, 0, 0), (0, 0, 1), "buttons", tile=0.3)
        k.box("suit_pack", (0.38, 0.12, 0.46), (x, ys + 0.17, 1.36), "panel", bevel=0.02)
        k.box("suit_belt", (0.48, 0.26, 0.06), (x, ys, 1.02), "dark", bevel=0.01)
        for sx in (-1, 1):
            k.box("suit_arm", (0.13, 0.15, 0.56), (x + sx * 0.29, ys, 1.28), "suit", bevel=0.04)
            k.cyl("suit_cuff", 0.075, 0.04, (x + sx * 0.29, ys, 0.99), "bare", segments=10)
            k.box("suit_glove", (0.1, 0.12, 0.13), (x + sx * 0.29, ys - 0.01, 0.91), "rubber", bevel=0.03)
            k.box("suit_leg", (0.18, 0.2, 0.76), (x + sx * 0.11, ys, 0.61), "suit", bevel=0.05)
            k.box("suit_knee", (0.14, 0.04, 0.12), (x + sx * 0.11, ys - 0.11, 0.55), "dark", bevel=0.01)
            k.box("suit_boot", (0.17, 0.3, 0.2), (x + sx * 0.11, ys - 0.04, 0.15), "rubber", bevel=0.03)
        k.cyl("helmet_ring", 0.12, 0.04, (x, ys - 0.02, 1.985), "bare", segments=14)
        k.sphere("helmet", 0.155, (x, ys - 0.02, 2.12), "panel", segments=14, rings=9)
        k.sphere("visor", 0.12, (x, ys - 0.13, 2.12), "glass_dark", scale=(1.0, 0.42, 0.7), segments=12, rings=8)
    k.col("rack", (W, depth + 0.04, 2.2), (xc, yb - depth / 2, 1.1))


def pressure_collar(k: Cell, side, s):
    """Bride de pression de la porte extérieure (côté pièce) : plats boulonnés sur les montants et
    le linteau du cadre du kit (épaisseur portée à 0,22 m), bandes de danger, bande rouge
    d'avertissement au-dessus. Ne gêne ni la porte, ni son boîtier de commande."""
    a, b = s - kit.DOOR_W / 2, s + kit.DOOR_W / 2
    r = Cell.right(side)
    up = (0, 0, 1)
    for s0 in (a - 0.24, b):
        sc = s0 + 0.12
        k.ori("collar_post", (0.26, kit.DOOR_H, 0.06), k.wall(side, sc, 0.19, kit.DOOR_H / 2), r, up, "dark", bevel=0.012)
        k.ori("collar_hazard", (0.18, 1.0, 0.006), k.wall(side, sc, 0.223, 0.6), r, up, "trim_sheet", uv="hazard")
        for zb in (0.25, 0.75, 1.25, 1.75, 2.1):
            k.cyl("collar_bolt", 0.018, 0.02, k.wall(side, sc - 0.09, 0.225, zb), "bare", axis="X" if side in "EW" else "Y", segments=6)
            k.cyl("collar_bolt", 0.018, 0.02, k.wall(side, sc + 0.09, 0.225, zb), "bare", axis="X" if side in "EW" else "Y", segments=6)
    k.ori("collar_head", (b - a + 0.52, 0.3, 0.06), k.wall(side, s, 0.19, kit.DOOR_H + 0.15), r, up, "dark", bevel=0.012)
    k.ori("collar_warning", (b - a + 0.4, 0.1, 0.006), k.wall(side, s, 0.223, kit.DOOR_H + 0.08), r, up, "trim_sheet", uv="warning_red")


# --- Cellules -----------------------------------------------------------------------------------


def warm_palette(k: Cell, floor=None):
    """Salles de vie : tôle peinte crème à la place de la tôle claire (coque, casiers, tiroirs)."""
    warm = k.m["panel_warm"]
    k.m = {**k.m, "panel": warm, "wall": warm, "ceiling": warm, "band_warm": k.m["band_hearth"]}
    if floor:
        k.m["floor_plate"] = k.m["floor"] = k.m[floor]


def build_cockpit(k: Cell):
    """Cockpit : planche de bord basse sous une verrière panoramique, deux sièges à 1,2 m de la
    vitre, pupitre des gaz entre eux, panneau de plafond au-dessus des têtes, poste de
    communications et armoires à l'arrière. Lumière froide, blanche et bleue."""
    y1 = k.y1
    m, kc = k.m, k.coll
    lamps = [(-1.4, 16.1, PI / 2), (1.4, 16.1, PI / 2), (0.0, 17.9, 0.0)]
    k.shell(style="ship", beams=[-2.0, 2.0], avoid=[(x, y) for x, y, _ in lamps])
    for i, (x, y, f) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 3200.0, 8.0, COCKPIT_WHITE, facing=f, shadow=(i == 2))

    # --- Planche de bord : un seul profil sous toute la verrière ------------------------------
    # Profil (y, z) : contre le mur, dessus plat à 0,84 m (juste sous l'allège), pente vers le
    # pilote, nez, face inclinée, retrait pour les pieds.
    sec = [(y1, 0.0), (y1, 0.84), (y1 - 0.26, 0.84), (y1 - 0.60, 0.70), (y1 - 0.64, 0.64), (y1 - 0.58, 0.16), (y1 - 0.52, 0.10), (y1 - 0.52, 0.0)]
    k.extrude("dash", [(0.0, y, z) for y, z in sec], (1, 0, 0), k.x0 + 0.02, k.x1 - 0.02, "dark", structure=True)
    k.col("dash", (k.x1 - k.x0 - 0.04, 0.66, 0.86), (0.0, y1 - 0.33, 0.43))
    # Repère de la pente : centre, « haut » le long de la pente, normale vers le pilote.
    sy, sz = y1 - 0.43, 0.77
    u = Vector((0, 0.925, 0.381))
    nrm = Vector((0, -0.381, 0.925))
    r = (1, 0, 0)

    def slope(tag, x, v, w, hh, mat, uv="box", tile=None, detail=0):
        ctr = Vector((x, sy, sz)) + u * v + nrm * 0.007
        k.ori(tag, (w, hh, 0.012), tuple(ctr), r, tuple(u), mat, uv=uv, tile=tile, detail=detail)

    for sx in (-1, 1):
        x = sx * 1.0
        slope("dash_screen", x, 0.05, 0.56, 0.2, "screen", tile=0.75)
        slope("dash_screen_side", x - 0.47, 0.05, 0.28, 0.17, "screen", tile=0.6)
        slope("dash_screen_side", x + 0.47, 0.05, 0.28, 0.17, "screen", tile=0.6)
        slope("dash_buttons", x, -0.115, 1.2, 0.08, "buttons")
        slope("dash_outer_buttons", sx * 2.4, -0.02, 1.0, 0.26, "buttons")
        slope("dash_outer_screen", sx * 3.3, 0.04, 0.42, 0.2, "screen", tile=0.7)
        slope("dash_outer_strip", sx * 3.3, -0.11, 0.5, 0.06, "buttons", detail=1)
        # Manche : colonne sortant de la face, guidon à deux poignées.
        k.box("yoke_col", (0.07, 0.24, 0.07), (x, y1 - 0.74, 0.6), "dark", bevel=0.01)
        k.box("yoke_bar", (0.34, 0.05, 0.05), (x, y1 - 0.87, 0.6), "dark", bevel=0.01)
        for gx in (-1, 1):
            k.box("yoke_grip", (0.045, 0.05, 0.15), (x + gx * 0.17, y1 - 0.87, 0.66), "rubber", bevel=0.012)
            k.box("yoke_btn", (0.03, 0.03, 0.012), (x + gx * 0.17, y1 - 0.87, 0.74), "beacon" if gx < 0 else "buttons")
    slope("dash_center", 0.0, 0.0, 0.5, 0.3, "buttons")
    # Liseré lumineux au fond du dessus, bandeaux bleus (cuits) sous le nez et au pied.
    k.ori("dash_strip", (7.4, 0.03, 0.006), (0.0, y1 - 0.245, 0.843), (1, 0, 0), (0, 1, 0), "trim_sheet", uv="light_strip")
    k.ori("dash_glow", (7.7, 0.03, 0.008), (0.0, y1 - 0.645, 0.6), (1, 0, 0), (0, -0.124, 0.992), "band_blue", uv=("strip", 1.0))
    k.ori("toe_glow", (7.7, 0.03, 0.008), (0.0, y1 - 0.524, 0.06), (1, 0, 0), (0, 0, 1), "band_blue", uv=("strip", 1.0))

    # Consoles basses sous les vitrages latéraux, qui prolongent la planche de bord.
    for sx in (-1, 1):
        xw = sx * k.x1
        side = [(xw, 0.0, 0.0), (xw, 0.0, 0.82), (xw - sx * 0.24, 0.0, 0.82), (xw - sx * 0.44, 0.0, 0.72), (xw - sx * 0.46, 0.0, 0.66),
                (xw - sx * 0.42, 0.0, 0.12), (xw - sx * 0.38, 0.0, 0.0)]
        k.extrude("side_desk", side, (0, 1, 0), 18.42, y1 - 0.64, "dark")
        k.col(f"side_desk{sx}", (0.46, y1 - 0.64 - 18.42, 0.84), (xw - sx * 0.23, (18.42 + y1 - 0.64) / 2, 0.42))
        su = Vector((sx * 0.894, 0, 0.447))
        sn = Vector((-sx * 0.447, 0, 0.894))
        for y, w, hh, mat, tile in ((19.35, 0.42, 0.14, "screen", 0.6), (18.8, 0.5, 0.13, "buttons", None)):
            ctr = Vector((xw - sx * 0.34, y, 0.77)) + sn * 0.007
            k.ori("side_plate", (w, hh, 0.012), tuple(ctr), (0, -sx, 0), tuple(su), mat, tile=tile)

    # --- Postes de pilotage -------------------------------------------------------------------
    seat_y = y1 - 1.17
    k.add(kit.seat("cockpit_pilot", (-1.0, seat_y, 0.0), 0.0, m, kc, role="pilot"))
    k.add(kit.seat("cockpit_copilot", (1.0, seat_y, 0.0), 0.0, m, kc, role="copilot"))
    for name, x, label in (("cockpit_helm", -1.0, "helm"), ("cockpit_nav", 1.0, "navigation")):
        k.marks.append(c.empty(f"INTERACT_{name}", (x, y1 - 0.33, 0.45), kc,
                               {"interact": "console", "label": label, "half_extents": c.half_extents((1.4, 0.66, 0.9), 0.04)}))
    # Pupitre des gaz entre les sièges, soudé à la planche de bord.
    k.box("pedestal", (0.4, 0.74, 0.62), (0.0, y1 - 0.97, 0.31), "dark", bevel=0.015)
    k.ori("pedestal_top", (0.34, 0.64, 0.012), (0.0, y1 - 0.97, 0.626), (1, 0, 0), (0, 1, 0), "buttons")
    k.ori("pedestal_screen", (0.28, 0.18, 0.01), (0.0, y1 - 1.345, 0.42), (1, 0, 0), (0, 0, 1), "screen", tile=0.5)
    for sx in (-1, 1):
        k.cyl("throttle", 0.014, 0.2, (sx * 0.08, y1 - 1.1, 0.72), "bare", segments=8)
        k.box("throttle_knob", (0.06, 0.05, 0.05), (sx * 0.08, y1 - 1.1, 0.83), "rubber", bevel=0.01)
    k.col("pedestal", (0.44, 0.78, 0.86), (0.0, y1 - 0.99, 0.43))

    # --- Panneau de plafond au-dessus des sièges (hors du champ du pilote, au-dessus de la ligne
    # de vue de l'entrée vers la vitre) ---------------------------------------------------------
    n = Vector((0, -0.45, -0.89)).normalized()
    rr = Vector((1, 0, 0))
    up = Vector((0, n.z, -n.y))
    ctr = Vector((0.0, seat_y - 0.03, 2.8))
    k.ori("overhead", (3.0, 0.8, 0.08), tuple(ctr), tuple(rr), tuple(up), "dark", bevel=0.015)
    for sx in (-1, 1):
        k.ori("overhead_buttons", (1.0, 0.5, 0.012), tuple(ctr + n * 0.046 + rr * (sx * 0.92) + up * 0.05), tuple(rr), tuple(up), "buttons")
        k.box("overhead_rod", (0.04, 0.04, 0.4), (sx * 1.4, seat_y + 0.22, 2.8), "dark")
    k.ori("overhead_screen", (0.66, 0.36, 0.012), tuple(ctr + n * 0.046 + up * 0.08), tuple(rr), tuple(up), "screen", tile=0.75)
    k.ori("overhead_strip", (2.8, 0.05, 0.01), tuple(ctr + n * 0.046 - up * 0.34), tuple(rr), tuple(up), "trim_sheet", uv="light_strip")

    # --- Arrière : communications, armoire, casiers --------------------------------------------
    k.add(kit.console("cockpit_comms", (k.x1 - 0.32, 17.4, 0.0), PI / 2, m, kc, label="comms"))
    k.add(kit.cabinet(k.nm("cabinet"), k.wall("W", 17.4), Cell.facing("W"), m, kc, size=(1.4, 0.32, 1.5)))
    # Casiers décoratifs (M10 : « Casier verrouillé », volume INTERACT_ du kit).
    k.add(kit.locker("cockpit_locker0", (-3.4, k.y0 + 0.28, 0.0), 0.0, m, kc))
    k.add(kit.locker("cockpit_locker1", (-2.74, k.y0 + 0.28, 0.0), 0.0, m, kc))  # 0,66 m : volumes jointifs
    k.add(kit.cabinet(k.nm("cabinet"), k.wall("S", 3.0), Cell.facing("S"), m, kc, size=(1.3, 0.32, 1.6)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("E", 15.4, 0, 1.78), Cell.facing("E"), m, kc, size=(0.6, 0.4)))
    k.add(kit.floor_grating(k.nm("grating"), (0.0, 16.4, 0.0), (1.0, 1.6), 0.0, m, kc, glow=False))
    # Strapontins le long des flancs, à l'entrée (passagers).
    k.add(kit.seat("cockpit_jump_w", (k.x0 + 0.42, 15.55, 0.0), -PI / 2, m, kc))
    k.add(kit.seat("cockpit_jump_e", (k.x1 - 0.42, 15.55, 0.0), PI / 2, m, kc))

    # --- Niveau 1 : habillage moyen ------------------------------------------------------------
    # M10 : plus au-dessus du tableau électrique (ses gaines montent jusqu'à 2,05 m) : paroi sud,
    # entre les casiers et la porte.
    k.add(kit.wall_screen(k.nm("screen"), k.wall("S", -1.4, 0, 1.6), Cell.facing("S"), m, kc, size=(0.5, 0.34), detail=1))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("W", 15.0, 0, 2.12), Cell.facing("W"), m, kc, size=(0.42, 0.14, 0.46), detail=1))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("E", 17.4, 0, 2.2), Cell.facing("E"), m, kc, size=(0.5, 0.14, 0.42), detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("S", -2.0 - 0.9, 0, 2.35), Cell.facing("S"), m, kc, length=0.9, detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("E", 16.0, 0, 2.3), Cell.facing("E"), m, kc, length=0.8, detail=1))
    for sx in (-1, 1):
        k.add(kit.cable_tray(k.nm("tray"), (sx * 3.3, 14.6, 2.82), (sx * 3.3, 18.2, 2.82), m, kc, width=0.28, hang_to=k.h, detail=1))
    k.add(kit.vent(k.nm("vent"), (0.0, 15.2, k.h), 0.0, m, kc, size=(0.5, 0.5), surface="ceiling", detail=1))
    gauge(k, "W", 15.95, 1.85, detail=1)
    gauge(k, "W", 16.15, 1.85, detail=1)
    # Chemins de câbles du plafond jusqu'au panneau de plafond (au-dessus de la ligne de vue).
    for sx in (-1, 1):
        k.add(kit.cable_tray(k.nm("tray"), (sx * 0.75, 14.7, 2.86), (sx * 0.75, seat_y - 0.45, 2.86), m, kc, width=0.22, hang_to=k.h, detail=1))
        # Trappes d'accès sur la face de la planche de bord.
        zf = 0.34
        yf = y1 - 0.64 + 0.06 * (0.64 - zf) / 0.48 - 0.004
        k.ori("dash_access", (1.1, 0.3, 0.006), (sx * 2.55, yf, zf), (1, 0, 0), (0, -0.124, 0.992), "trim_sheet", uv="vent", detail=1)
        k.ori("dash_access", (0.42, 0.22, 0.006), (sx * 0.5, yf, zf), (1, 0, 0), (0, -0.124, 0.992), "trim_sheet", uv="vent", detail=1)

    # --- Niveau 2 : très riche ------------------------------------------------------------------
    for sx in (-1, 1):
        k.add(kit.pipe(k.nm("pipe"), (sx * 2.65, 14.6, 2.9), (sx * 2.65, 18.4, 2.9), m, kc, radius=0.04, hang_to=k.h, detail=2))
        hanging_cable(k, (sx * 1.4, seat_y + 0.22, 2.62), (sx * 3.3, 18.2, 2.8), sag=0.18, detail=2)
    # Casque audio pendu à la tige du panneau de plafond (côté pilote).
    k.box("headset_band", (0.03, 0.18, 0.03), (-1.4, seat_y + 0.22, 2.585), "rubber", detail=2)
    for dy in (-0.09, 0.09):
        k.cyl("headset_cup", 0.045, 0.035, (-1.4, seat_y + 0.22 + dy, 2.53), "dark", axis="Y", segments=10, detail=2)
    mug(k, 3.88, 18.62, 0.82, "paint_yellow", detail=2)
    slates(k, -3.0, y1 - 0.12, 0.84, rot=0.25, detail=2)
    # Figurine fétiche sur la planche (formes simples, aucune ressemblance).
    k.cyl("figurine_base", 0.035, 0.015, (3.1, y1 - 0.12, 0.848), "bare", segments=10, detail=2)
    k.sphere("figurine_body", 0.03, (3.1, y1 - 0.12, 0.885), "paint_yellow", scale=(1, 1, 1.3), segments=8, rings=6, detail=2)
    k.sphere("figurine_head", 0.022, (3.1, y1 - 0.12, 0.935), "panel", segments=8, rings=6, detail=2)
    # Notes collées sur les consoles latérales.
    for sx, y in ((-1, 18.7), (-1, 18.9), (1, 19.15)):
        su = Vector((sx * 0.894, 0, 0.447))
        sn = Vector((-sx * 0.447, 0, 0.894))
        ctr = Vector((sx * (k.x1 - 0.34), y, 0.77)) + sn * 0.015 + su * 0.03
        k.ori("note", (0.07, 0.07, 0.003), tuple(ctr), (0, -sx, 0), tuple(su), "fabric_light", detail=2)
    extinguisher(k, "S", 1.3, detail=2)
    for sx in (-1, 1):
        hanging_cable(k, (sx * 0.75, 15.3, 2.8), (sx * 0.75, 16.9, 2.8), sag=0.2, detail=2)
    # Veste jetée sur le dossier du copilote, tasse au pied du pupitre des gaz.
    k.ori("seat_jacket", (0.5, 0.5, 0.05), (1.0, seat_y - 0.38, 0.95), (-1, 0, 0), (0, -0.139, 0.99), "fabric_dark", bevel=0.015, detail=2)
    k.ori("seat_jacket_fold", (0.5, 0.12, 0.08), (1.0, seat_y - 0.31, 1.2), (-1, 0, 0), (0, 0.6, 0.8), "fabric_dark", bevel=0.02, detail=2)
    mug(k, -0.1, y1 - 0.72, 0.632, "crate_slate", rot=PI, detail=2)
    wall_conduits(k, 2.5, sides="SEW", skip=[("E", 18.2, 20.4), ("W", 18.2, 20.4)])
    # Faisceaux de câbles qui sortent de la planche de bord et filent sous le caillebotis.
    for sx in (-1, 1):
        k.rod("dash_cable", (sx * 2.0, y1 - 0.55, 0.12), (sx * 2.0, y1 - 0.8, 0.02), 0.02, "rubber", detail=2)
    k.ori("cable_cover", (0.18, 3.0, 0.01), (-2.0, y1 - 2.35, 0.005), (1, 0, 0), (0, 1, 0), "trim_sheet", uv="rivets", detail=2)


def build_chambre(k: Cell):
    """Chambre du capitaine, départ du tutoriel : couchette sous deux hublots (paroi de coque),
    casiers fonctionnels, bureau et sa tablette, étagère, commode ; lumière chaude."""
    warm_palette(k)
    lamps = [(-6.0, 11.0), (-3.2, 11.0)]
    k.shell(style="ship", bands=False, avoid=lamps)
    chamfer_bands(k, "hearth")
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 3600.0, 7.0, WARM, lens="light_warm", shadow=(i == 1))
    m, kc = k.m, k.coll
    # Couchette contre la paroi de coque (ouest), tête au nord ; nervure du mur à y = 12.
    bed_y = 12.8
    k.add(kit.bed("chambre_bed", (k.x0 + 0.5, bed_y, 0.0), -PI / 2, m, kc, pad_gaps=[bed_y - 12.0]))
    for y in (13.0, 10.9):
        k.add(kit.cabin_porthole(k.nm("porthole"), k.wall("W", y, 0, 1.55), Cell.facing("W"), m, kc))
    # Casiers fonctionnels (contenus de départ dans gameplay/items.ron), mur sud.
    k.add(kit.locker_bank("chambre_lockers", (-4.6, k.y0 + 0.285, 0.0), 0.0, m, kc, 3,
                          containers=["corvette_chambre_casier_1", "corvette_chambre_casier_2", "corvette_chambre_casier_3"]))
    # Bureau mural (nord), chaise, et la tablette du tutoriel posée dessus.
    k.add(kit.wall_desk("chambre_desk", (-4.5, k.y1, 0.0), PI, m, kc, width=1.3, items_detail=1))
    k.add(kit.seat("chambre_chair", (-4.5, 12.95, 0.0), 0.0, m, kc, role="passenger"))
    k.pickup(kit.pickup("chambre_pickup_datapad", (-4.92, k.y1 - 0.33, 0.78), 0.25, m, kc, item="datapad"))
    # Étagère (sud-est), commode et malle au pied du lit.
    k.add(kit.shelf(k.nm("shelf"), (-2.55, k.y0, 0.0), 0.0, m, kc, width=1.0, items_detail=1))
    sideboard(k, "dresser", k.wall("W", 9.0), Cell.facing("W"), size=(1.0, 0.42, 0.82), drawers=3)
    k.add(kit.crate(k.nm("footlocker"), (k.x0 + 0.5, 11.4, 0.0), 0.0, m, kc, size=(0.9, 0.45, 0.45), variant="slate"))
    k.add(kit.vent(k.nm("vent"), (-4.6, 9.3, k.h), 0.0, m, kc, size=(0.5, 0.5), surface="ceiling"))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("E", 9.0, 0, 1.5), Cell.facing("E"), m, kc, size=(0.5, 0.34)))
    # Penderie dans l'angle nord-est.
    k.add(kit.locker("chambre_wardrobe", (-2.3, k.y1 - 0.28, 0.0), PI, m, kc, both_sides=False))

    # --- Niveau 1 ----------------------------------------------------------------------------
    k.box("rug", (1.8, 1.3, 0.012), (-4.4, 10.9, 0.006), "rug", tile=1.0, detail=1)
    k.add(kit.wall_screen(k.nm("screen"), k.wall("N", -3.2, 0, 1.5), Cell.facing("N"), m, kc, size=(0.36, 0.26), detail=1))
    # Miroir (verre fumé) dans son cadre, près de la porte.
    k.ori("mirror_frame", (0.5, 0.8, 0.03), k.wall("E", 13.45, 0.015, 1.5), Cell.right("E"), (0, 0, 1), "dark", bevel=0.01, detail=1)
    k.ori("mirror", (0.42, 0.72, 0.006), k.wall("E", 13.45, 0.033, 1.5), Cell.right("E"), (0, 0, 1), "glass_dark", detail=1)
    k.add(kit.junction_box(k.nm("jbox"), k.wall("S", -6.6, 0, 1.5), Cell.facing("S"), m, kc, size=(0.4, 0.14, 0.5), detail=1))
    k.add(kit.vent(k.nm("vent"), k.wall("S", -6.6, 0, 0.45), Cell.facing("S"), m, kc, size=(0.6, 0.2), detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("N", -2.6, 0, 2.2), Cell.facing("N"), m, kc, length=0.8, detail=1))
    k.add(kit.pipe(k.nm("pipe"), (-1.82, 8.05, 2.3), (-1.82, 13.95, 2.3), m, kc, radius=0.04, hang_to=k.h, detail=1))
    k.box("dresser_lamp_base", (0.1, 0.1, 0.03), (-7.35, 8.7, 0.865), "dark", detail=1)
    k.cyl("dresser_lamp", 0.05, 0.16, (-7.35, 8.7, 0.96), "light_warm", segments=10, detail=1)

    # --- Niveau 2 : objets personnels ----------------------------------------------------------
    plant(k, -6.75, 13.8, 1.09, detail=2)
    mug(k, -4.05, k.y1 - 0.35, 0.78, "paint_yellow", rot=PI, detail=2)
    slates(k, -4.35, k.y1 - 0.42, 0.78, rot=-0.3, count=2, detail=2)
    bottle(k, -7.36, 9.22, 0.85, "crate_slate", detail=2)
    k.box("frame", (0.03, 0.2, 0.15), (-7.48, 9.3, 0.925), "dark", detail=2)
    k.ori("frame_screen", (0.16, 0.11, 0.004), (-7.462, 9.3, 0.925), (0, 1, 0), (0, 0, 1), "screen", tile=0.35, detail=2)
    boots(k, -4.25, k.y1 - 0.38, rot=PI, detail=2)
    jacket(k, "E", 12.7, detail=2)
    # M10 : plus de sac sur la malle ni de linge plié sur le lit (retour 5 de l'auteur).
    hanging_cable(k, (-4.1, k.y1 - 0.03, 1.0), (-3.7, k.y1 - 0.4, 0.79), sag=0.08, radius=0.008, detail=2)
    wall_conduits(k, 2.33, sides="NSW")
    # Caisses rangées dans l'angle sud-est.
    k.add(kit.crate(k.nm("small_crate"), (-1.85, 8.3, 0.0), 0.1, m, kc, size=(0.42, 0.42, 0.36), variant="olive", detail=2))
    k.add(kit.crate(k.nm("small_crate"), (-1.86, 8.32, 0.36), -0.15, m, kc, size=(0.36, 0.36, 0.3), variant="slate", detail=2))
    slates(k, -7.3, 8.98, 0.85, rot=1.2, count=2, detail=2)
    for i in range(3):
        k.box("book", (0.04, 0.16, 0.22 - 0.02 * i), (-6.95 + 0.05 * i, 13.82, 1.09 + 0.11 - 0.01 * i), "fabric_dark" if i % 2 else "crate_rust", detail=2)


def build_quartiers(k: Cell):
    """Quartiers de l'équipage : deux couchettes doubles contre la coque, penderie, casiers,
    coin bureau ; lumière chaude."""
    warm_palette(k)
    lamps = [(-4.4, 2.6), (-4.4, 6.6)]
    k.shell(style="ship", bands=False, avoid=lamps)
    chamfer_bands(k, "hearth")
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 3600.0, 8.0, WARM, lens="light_warm", shadow=(i == 0))
    m, kc = k.m, k.coll
    k.add(kit.bunk("quartiers_bunk0", (k.x0 + 0.55, 2.75, 0.0), PI / 2, m, kc))
    k.add(kit.bunk("quartiers_bunk1", (k.x0 + 0.55, 6.45, 0.0), PI / 2, m, kc))
    k.add(kit.locker("quartiers_wardrobe", (k.x0 + 0.28, 4.6, 0.0), -PI / 2, m, kc))
    k.add(kit.locker("quartiers_locker0", (-3.0, k.y1 - 0.28, 0.0), PI, m, kc))
    k.add(kit.locker("quartiers_locker1", (-2.34, k.y1 - 0.28, 0.0), PI, m, kc))  # 0,66 m : volumes jointifs
    k.add(kit.table("quartiers_desk", (-4.6, k.y0 + 0.4, 0.0), 0.0, m, kc, size=(1.4, 0.7)))
    k.cyl("stool", 0.19, 0.06, (-4.6, 2.55, 0.45), "rubber", segments=12)
    k.cyl("stool_post", 0.035, 0.42, (-4.6, 2.55, 0.21), "bare", segments=8)
    k.cyl("stool_foot", 0.17, 0.03, (-4.6, 2.55, 0.015), "dark", segments=12)
    k.col("stool", (0.4, 0.4, 0.48), (-4.6, 2.55, 0.24))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("S", -4.6, 0, 1.45), Cell.facing("S"), m, kc, size=(0.7, 0.42)))
    k.add(kit.vent(k.nm("vent"), (-4.4, 4.6, k.h), 0.0, m, kc, size=(0.5, 0.5), surface="ceiling"))

    # --- Niveau 1 ----------------------------------------------------------------------------
    k.box("rug", (1.4, 2.0, 0.012), (-4.6, 4.9, 0.006), "rug", tile=1.0, detail=1)
    # Rideaux tirés à moitié sur le côté ouvert des couchettes.
    for y in (2.75, 6.45):
        for zc, hh, zr in ((0.8, 0.8, 1.22), (1.68, 0.36, 1.88)):
            k.box("curtain", (0.02, 0.8, hh), (k.x0 + 1.02, y + 0.55, zc), "fabric_dark", detail=1)
            k.rod("curtain_rod", (k.x0 + 1.02, y - 0.98, zr), (k.x0 + 1.02, y + 0.98, zr), 0.01, "bare", detail=1)
    k.add(kit.junction_box(k.nm("jbox"), k.wall("E", 2.6, 0, 1.35), Cell.facing("E"), m, kc, size=(0.4, 0.14, 0.5), detail=1))
    k.add(kit.vent(k.nm("vent"), k.wall("E", 7.0, 0, 0.4), Cell.facing("E"), m, kc, size=(0.6, 0.2), detail=1))
    k.add(kit.pipe(k.nm("pipe"), (-1.82, 1.65, 2.3), (-1.82, 7.55, 2.3), m, kc, radius=0.04, hang_to=k.h, detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("N", -5.2, 0, 2.2), Cell.facing("N"), m, kc, length=1.0, detail=1))
    k.add(kit.light_band(k.nm("desk_lamp"), (-5.2, k.y0 + 0.06, 1.92), (-4.0, k.y0 + 0.06, 1.92), m, kc, normal=(0, 0.5, -0.87), tone="warm", detail=1))

    # --- Niveau 2 ----------------------------------------------------------------------------
    # M10 : plus de sacs sur les couchettes du haut (retour 2 : ils ressemblaient à des moteurs).
    boots(k, k.x0 + 1.15, 2.3, rot=-PI / 2, detail=2)
    k.box("towel", (0.03, 0.3, 0.4), (k.x0 + 1.0, 5.8, 1.42), "fabric_light", detail=2)
    mug(k, -4.9, k.y0 + 0.35, 0.79, "paint_yellow", detail=2)
    mug(k, -4.2, k.y0 + 0.5, 0.79, "crate_slate", rot=1.0, detail=2)
    slates(k, -4.5, k.y0 + 0.3, 0.79, rot=0.4, count=3, detail=2)
    jacket(k, "E", 7.1, detail=2)
    k.add(kit.shelf(k.nm("shelf"), (-6.1, k.y1, 0.0), PI, m, kc, width=0.8, height=0.9, depth=0.3, levels=2, items_detail=2))
    hanging_cable(k, (-1.85, 2.2, 2.3), (-1.85, 4.0, 2.3), sag=0.22, detail=2)
    wall_conduits(k, 2.33, sides="NSW")
    # Petits écrans personnels à la tête des couchettes (M10 : plus de linge pendu aux montants,
    # un pavé qui se lisait comme une planche devant la couchette du haut).
    for y in (2.75, 6.45):
        for z in (0.8, 1.75):
            k.ori("bunk_screen", (0.16, 0.11, 0.006), (k.x0 + 0.06, y - 0.7, z), (0, 1, 0), (0, 0, 1), "screen", tile=0.35, detail=2)
    k.add(kit.crate(k.nm("small_crate"), (-5.1, k.y0 + 0.35, 0.0), 0.05, m, kc, size=(0.45, 0.4, 0.35), variant="slate", detail=2))


def build_carre(k: Cell):
    """Carré : table et sièges, office le long de la coque (frigo, plan de travail, placards),
    étagère, buffet ; lumière chaude, carrelage. M10 : le frigo du kit (`corvette_carre_frigo`,
    nourriture seulement) remplace la borne de la cuisine et la chambre froide décorative."""
    warm_palette(k, floor="tiles")
    lamps = [(4.4, 5.6), (6.0, 2.6), (4.4, 8.6)]
    k.shell(style="ship", bands=False, avoid=lamps, beams=[3.6, 7.6])
    chamfer_bands(k, "hearth")
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 3200.0, 7.0, WARM, lens="light_warm", shadow=(i == 0))
    m, kc = k.m, k.coll
    k.add(kit.table("carre_table", (4.4, 7.0, 0.0), PI / 2, m, kc, size=(2.2, 0.9)))
    for i, (x, y, f) in enumerate(((3.55, 6.4, -PI / 2), (3.55, 7.6, -PI / 2), (5.25, 6.4, PI / 2), (5.25, 7.6, PI / 2))):
        k.add(kit.seat(f"carre_seat{i}", (x, y, 0.0), f, m, kc))
    # Office contre la coque (est) : frigo dans la première travée (entre le mur sud et la
    # nervure de y = 3,6), porte vers la salle ; plan de travail et placards dans la suivante.
    k.add(kit.fridge("carre_frigo", (k.x1 - 0.35, 2.6, 0.0), PI / 2, m, kc, container="corvette_carre_frigo"))
    k.add(kit.table(k.nm("counter"), (k.x1 - 0.32, 4.6, 0.0), PI / 2, m, kc, size=(1.6, 0.6)))
    k.box("cupboard", (0.36, 1.6, 0.6), (k.x1 - 0.18, 4.6, 1.82), "dark", bevel=0.01)
    k.ori("cupboard_door", (0.74, 0.54, 0.012), (k.x1 - 0.365, 4.2, 1.82), (0, -1, 0), (0, 0, 1), "panel", uv=("box", (0.3, 0.1)))
    k.ori("cupboard_door", (0.74, 0.54, 0.012), (k.x1 - 0.365, 5.0, 1.82), (0, -1, 0), (0, 0, 1), "panel", uv=("box", (0.6, 0.4)))
    k.ori("cupboard_lamp", (1.4, 0.05, 0.01), (k.x1 - 0.32, 4.6, 1.515), (0, 1, 0), (1, 0, 0), "trim_sheet", uv="light_strip")
    # Étagère de provisions dans la travée de l'ancienne chambre froide (garnie dès le Bas).
    k.add(kit.shelf(k.nm("pantry"), k.wall("E", 6.6), Cell.facing("E"), m, kc, width=1.2, height=1.8, depth=0.34, items_detail=0))
    sideboard(k, "sideboard", k.wall("S", 4.6), Cell.facing("S"), size=(1.3, 0.42, 0.9), drawers=2)
    sideboard(k, "buffet", k.wall("N", 4.6), Cell.facing("N"), size=(1.6, 0.42, 0.8), drawers=2)
    k.add(kit.wall_screen(k.nm("screen"), k.wall("N", 4.6, 0, 1.55), Cell.facing("N"), m, kc, size=(0.9, 0.5)))
    # Fontaine à eau (sud-ouest).
    k.box("fountain", (0.36, 0.34, 1.0), (2.4, k.y0 + 0.19, 0.5), "dark", bevel=0.012)
    k.cyl("fountain_jug", 0.12, 0.34, (2.4, k.y0 + 0.19, 1.17), "glass", segments=12)
    k.ori("fountain_tap", (0.14, 0.1, 0.006), (2.4, k.y0 + 0.363, 0.72), (-1, 0, 0), (0, 0, 1), "buttons", tile=0.3)
    k.col("fountain", (0.38, 0.36, 1.34), (2.4, k.y0 + 0.19, 0.67))
    k.add(kit.vent(k.nm("vent"), (4.4, 3.6 - 0.8, k.h), 0.0, m, kc, size=(0.5, 0.5), surface="ceiling"))

    # --- Niveau 1 ----------------------------------------------------------------------------
    k.box("tray", (0.34, 0.5, 0.02), (4.4, 7.0, 0.8), "bare", bevel=0.005, detail=1)
    for i, (x, y) in enumerate(((4.22, 6.2), (4.6, 6.45), (4.26, 7.75), (4.58, 7.95))):
        mug(k, x, y, 0.79, "crate_slate" if i % 2 else "paint_yellow", rot=i, detail=1)
    k.add(kit.junction_box(k.nm("jbox"), k.wall("W", 3.2, 0, 1.35), Cell.facing("W"), m, kc, size=(0.4, 0.14, 0.5), detail=1))
    k.add(kit.vent(k.nm("vent"), k.wall("W", 8.8, 0, 0.4), Cell.facing("W"), m, kc, size=(0.6, 0.2), detail=1))
    k.add(kit.pipe(k.nm("pipe"), (7.38, 1.65, 2.3), (7.38, 9.55, 2.3), m, kc, radius=0.04, hang_to=k.h, detail=1))
    # Suspension au-dessus de la table (décor émissif, sans lampe cuite).
    k.rod("pendant_rod", (4.4, 7.0, k.h), (4.4, 7.0, 2.2), 0.008, "dark", detail=1)
    k.cyl("pendant", 0.16, 0.08, (4.4, 7.0, 2.16), "dark", segments=14, detail=1)
    k.cyl("pendant_glow", 0.13, 0.012, (4.4, 7.0, 2.115), "light_warm", segments=14, detail=1)
    for y, sz in ((4.0, (0.24, 0.3, 0.42)), (4.45, (0.18, 0.18, 0.26)), (4.75, (0.18, 0.18, 0.2))):
        k.box("canister", (sz[1], sz[0], sz[2]), (k.x1 - 0.3, y, 0.79 + sz[2] / 2), "bare" if sz[0] > 0.2 else "crate_slate", bevel=0.01, detail=1)

    # --- Niveau 2 ----------------------------------------------------------------------------
    k.box("ration", (0.16, 0.24, 0.07), (4.45, 6.95, 0.845), "crate", bevel=0.006, detail=2)
    bottle(k, 4.3, 7.4, 0.79, "crate_slate", detail=2)
    # Machine à boisson chaude sur le plan de travail, ustensiles pendus sous les placards.
    k.box("brewer", (0.3, 0.26, 0.36), (k.x1 - 0.3, 5.25, 0.97), "dark", bevel=0.015, detail=2)
    k.ori("brewer_panel", (0.16, 0.12, 0.006), (k.x1 - 0.454, 5.25, 1.04), (0, -1, 0), (0, 0, 1), "buttons", tile=0.3, detail=2)
    k.rod("utensil_rail", (k.x1 - 0.32, 3.95, 1.45), (k.x1 - 0.32, 5.25, 1.45), 0.01, "bare", detail=2)
    for i, y in enumerate((4.1, 4.3, 4.5, 4.75, 5.0)):
        k.box("utensil", (0.02, 0.05 if i % 2 else 0.12, 0.22 if i % 2 else 0.12), (k.x1 - 0.32, y, 1.32), "bare" if i % 2 else "dark", detail=2)
    plant(k, 5.4, k.y1 - 0.2, 0.83, detail=2)
    slates(k, 3.9, k.y1 - 0.22, 0.83, rot=0.2, count=2, detail=2)
    for i in range(3):
        k.cyl("can", 0.035, 0.12, (4.85 + i * 0.09, k.y0 + 0.22, 0.99), "paint_red" if i == 1 else "crate_slate", segments=10, detail=2)
    hanging_cable(k, (7.4, 6.0, 2.3), (7.4, 8.0, 2.3), sag=0.2, detail=2)
    wall_conduits(k, 2.33, sides="NSW")
    # Assiettes et couverts sur la table, étagère à épices et bocaux au-dessus du buffet.
    for x, y in ((4.15, 6.4), (4.65, 6.4), (4.15, 7.6), (4.65, 7.6)):
        k.cyl("plate", 0.11, 0.012, (x, y, 0.796), "bare", segments=14, detail=2)
    k.box("spice_shelf", (1.2, 0.14, 0.025), (4.6, k.y1 - 0.07, 1.98), "dark", detail=2)
    for i in range(7):
        k.cyl("jar", 0.035, 0.11 + 0.02 * (i % 3), (4.1 + i * 0.16, k.y1 - 0.07, 2.0 + (0.11 + 0.02 * (i % 3)) / 2),
              ("crate_slate", "paint_yellow", "crate_rust", "glass")[i % 4], segments=10, detail=2)
    k.add(kit.wall_screen(k.nm("screen"), k.wall("E", 8.6, 0, 1.6), Cell.facing("E"), m, kc, size=(0.4, 0.3), detail=2))


def build_sas(k: Cell):
    """Sas : salle de passage entre la porte intérieure (couloir, est) et la porte extérieure
    (`sas__@dock`, ouest, bride de pression), éclairage d'alerte rouge, bandes de danger,
    râtelier de combinaisons au nord, banc et casiers au sud."""
    lamps = [(-6.2, -3.0), (-3.0, -3.0)]
    k.shell(style="ship", bands=False, avoid=lamps, beams=[-4.6])
    chamfer_bands(k, "red")
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 1500.0, 6.0, COOL, shadow=(i == 0))
    m, kc = k.m, k.coll
    yo = -3.0
    # Porte extérieure : bride de pression, gyrophare rouge, voyants rouges, zone de danger.
    pressure_collar(k, "W", yo)
    for ds in (-0.55, 0.55):
        k.ori("door_lamp", (0.14, 0.07, 0.02), k.wall("W", yo + ds, 0.23, 2.42), Cell.right("W"), (0, 0, 1), "beacon")
    k.add(kit.beacon(k.nm("beacon"), k.wall("W", -4.35, 0, 2.0), Cell.facing("W"), m, kc, lumens=700.0, rng=3.5))
    status_lamp(k, "amber", k.wall("W", -1.65, 0, 2.0), Cell.facing("W"), "beacon_amber", AMBER, lumens=350.0)
    k.add(kit.hazard_frame(k.nm("hatch_zone"), (k.x0 + 0.55, yo, 0.0), (0.8, 1.4), m, kc, width=0.12))
    k.ori("exit_chevrons", (1.8, 0.36, 0.006), (-5.0, yo, 0.003), (-1, 0, 0), (0, -1, 0), "trim_sheet", uv="chevrons", tile=0.8)
    # Porte intérieure : voyants verts, seuil.
    header_lamps(k, "E", yo, [(-0.55, "beacon_green"), (0.55, "beacon_green")])
    status_lamp(k, "go", k.wall("E", -4.35, 0, 2.0), Cell.facing("E"), "beacon_green", GREEN, lumens=300.0)
    # Bandes de danger au sol le long des parois, grille de purge au centre.
    for y in (k.y1 - 0.62, k.y0 + 0.62):
        k.add(kit.hazard_stripe(k.nm("edge"), (k.x0 + 1.15, y, 0.0), (k.x1 - 0.35, y, 0.0), m, kc, width=0.1))
    k.add(kit.floor_grating(k.nm("drain"), (-3.4, yo, 0.0), (0.8, 1.2), 0.0, m, kc, glow=None))
    # Râtelier de combinaisons (nord).
    suit_rack(k, -4.6, k.y1, count=3)
    # Sud : casier, banc, armoire de pressurisation ; écrans de cycle près des deux portes.
    k.add(kit.locker("sas_locker", (-6.9, k.y0 + 0.28, 0.0), 0.0, m, kc))
    bx, by = -4.6, k.y0 + 0.09 + 0.23
    k.box("bench_top", (1.6, 0.46, 0.06), (bx, by, 0.47), "dark", bevel=0.01)
    k.box("bench_pad", (1.5, 0.4, 0.05), (bx, by, 0.525), "rubber")
    for sx in (-1, 1):
        k.box("bench_leg", (0.06, 0.4, 0.44), (bx + sx * 0.7, by, 0.22), "dark")
    k.col("bench", (1.6, 0.48, 0.55), (bx, by, 0.275))
    k.add(kit.cabinet(k.nm("press"), k.wall("S", -2.5), Cell.facing("S"), m, kc, size=(1.0, 0.32, 1.6)))
    k.add(kit.wall_screen(k.nm("cycle"), k.wall("N", -7.0, 0, 1.5), Cell.facing("N"), m, kc, size=(0.5, 0.36)))
    k.add(kit.wall_screen(k.nm("cycle"), k.wall("N", -2.4, 0, 1.5), Cell.facing("N"), m, kc, size=(0.5, 0.36)))
    # Conduites de pressurisation au plafond, bouches de purge.
    k.add(kit.pipe(k.nm("pipe"), (-7.3, k.y1 - 0.32, 2.4), (-1.9, k.y1 - 0.32, 2.4), m, kc, radius=0.06, hang_to=k.h))
    for x in (-5.4, -3.8):
        k.add(kit.vent(k.nm("purge"), (x, yo, k.h), 0.0, m, kc, size=(0.5, 0.4), surface="ceiling"))

    # --- Niveau 1 ----------------------------------------------------------------------------
    k.add(kit.pipe(k.nm("pipe"), (-7.3, k.y0 + 0.3, 2.42), (-1.9, k.y0 + 0.3, 2.42), m, kc, radius=0.045, hang_to=k.h, detail=1))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("S", -6.2, 0, 1.5), Cell.facing("S"), m, kc, size=(0.4, 0.14, 0.5), detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("N", -7.0, 0, 2.0), Cell.facing("N"), m, kc, length=0.8, detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("S", -3.6, 0, 1.9), Cell.facing("S"), m, kc, length=0.9, detail=1))
    gauge(k, "N", -2.85, 1.55, detail=1)
    gauge(k, "N", -6.55, 1.55, detail=1)
    # Râtelier de bouteilles d'air au mur sud, au-dessus du banc.
    k.box("tank_rack", (1.2, 0.04, 0.3), (bx, k.y0 + 0.02, 1.25), "dark", detail=1)
    for i in range(4):
        gas_tank(k, bx - 0.45 + i * 0.3, k.y0 + 0.15, 0.92, radius=0.1, h=0.62, mat="paint_yellow" if i % 2 else "crate_slate", detail=1)
    # Flexibles d'air des combinaisons vers la paroi.
    for i in range(3):
        x = -4.6 + (i - 1) * 0.8
        hanging_cable(k, (x, k.y1 - 0.24, 1.6), (x + 0.25, k.y1 - 0.02, 2.1), sag=0.12, radius=0.02, detail=1)

    # --- Niveau 2 ----------------------------------------------------------------------------
    for i, x in enumerate((-5.1, -4.15)):
        k.sphere("bench_helmet", 0.15, (x, by + 0.02, 0.7), "panel", segments=12, rings=8, detail=2)
        k.sphere("bench_visor", 0.115, (x + 0.02, by + 0.13, 0.7), "glass_dark", scale=(1.0, 0.42, 0.7), segments=10, rings=6, detail=2)
    boots(k, -4.6, by, rot=0.0, detail=2)
    # Trousse de secours murale, longe enroulée sur une patère, plaques d'avertissement.
    k.box("medbox", (0.4, 0.14, 0.3), (-1.95, k.y1 - 0.07, 1.0), "paint_red", bevel=0.01, detail=2)
    k.ori("medbox_label", (0.22, 0.06, 0.004), (-1.95, k.y1 - 0.142, 1.05), (1, 0, 0), (0, 0, 1), "trim_sheet", uv="stencils", detail=2)
    for i in range(5):
        a = 2 * PI * i / 5
        k.rod("tether", (-7.4 + 0.1 * math.cos(a), k.y0 + 0.06, 1.75 + 0.1 * math.sin(a)),
              (-7.4 + 0.1 * math.cos(a + 2 * PI / 5), k.y0 + 0.06, 1.75 + 0.1 * math.sin(a + 2 * PI / 5)), 0.012, "paint_yellow", detail=2)
    for x in (-6.35, -2.85):
        k.ori("warning", (0.4, 0.1, 0.004), k.wall("N", x, 0.002, 2.08), (1, 0, 0), (0, 0, 1), "trim_sheet", uv="warning_red", detail=2)
    extinguisher(k, "E", -1.55, detail=2)
    hanging_cable(k, (-6.8, k.y1 - 0.32, 2.4), (-5.9, k.y1 - 0.32, 2.4), sag=0.15, detail=2)
    wall_conduits(k, 2.13, sides="S")


def build_couloir_avant(k: Cell):
    lamps = [(0.0, 2.2), (0.0, 8.2), (0.0, 12.9)]
    k.shell(floor_strip=True, style="ship", avoid=lamps)
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 2500.0, 6.0, COOL, facing=PI / 2, shadow=(i == 1))
    m, kc = k.m, k.coll
    k.add(kit.vent(k.nm("vent"), k.wall("W", 1.0, 0, 0.75), Cell.facing("W"), m, kc, size=(0.7, 0.22)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("W", 7.0, 0, 1.55), Cell.facing("W"), m, kc))
    k.add(kit.vent(k.nm("vent"), k.wall("E", 3.0, 0, 0.75), Cell.facing("E"), m, kc, size=(0.7, 0.22)))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("E", 9.0, 0, 1.35), Cell.facing("E"), m, kc, size=(0.5, 0.14, 0.62)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("E", 11.0, 0, 1.6), Cell.facing("E"), m, kc, size=(0.5, 0.34)))
    k.add(kit.pipe(k.nm("riser"), (1.08, 13.88, 0.32), (1.08, 13.88, 2.15), m, kc, radius=0.045))
    # Niveau 1.
    k.add(kit.junction_box(k.nm("jbox"), k.wall("W", 9.0, 0, 1.35), Cell.facing("W"), m, kc, detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("W", 13.0, 0, 1.95), Cell.facing("W"), m, kc, length=0.9, detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("E", 3.0, 0, 1.95), Cell.facing("E"), m, kc, length=1.2, detail=1))
    k.add(kit.vent(k.nm("vent"), k.wall("E", 13.0, 0, 0.75), Cell.facing("E"), m, kc, size=(0.7, 0.22), detail=1))
    k.add(kit.pipe(k.nm("conduit"), (1.08, 6.6, 0.5), (1.08, 13.6, 0.5), m, kc, radius=0.03, mat="bare", detail=1))
    k.add(kit.pipe(k.nm("riser"), (-1.08, 0.12, 0.32), (-1.08, 0.12, 2.15), m, kc, radius=0.04, detail=1))
    gauge(k, "E", 7.4, 1.5, detail=1)
    # Niveau 2.
    for y0 in (3.0, 9.4):
        hanging_cable(k, (0.42, y0, 2.48), (0.42, y0 + 1.6, 2.48), sag=0.16, detail=2)
    k.box("board", (0.02, 0.7, 0.5), (1.19, 12.6, 1.7), "dark", detail=2)
    for i, (dy, dz) in enumerate(((-0.2, 0.08), (0.05, 0.12), (0.22, -0.05), (-0.1, -0.12))):
        k.ori("sheet", (0.16, 0.2, 0.002), (1.178, 12.6 + dy, 1.7 + dz), (0, -1, 0), (0, 0, 1), "fabric_light", detail=2)
    extinguisher(k, "E", 7.0, detail=2)
    for y in (3.4, 9.6):
        k.ori("warning", (0.4, 0.1, 0.004), k.wall("W" if y > 5 else "E", y, 0.002, 2.0),
              Cell.right("W" if y > 5 else "E"), (0, 0, 1), "trim_sheet", uv="warning_red", detail=2)
    wall_conduits(k, 2.03, sides="EW")


def build_couloir_arriere(k: Cell):
    lamps = [(0.0, -13.0), (0.0, -9.0), (0.0, -5.0), (0.0, -1.0)]
    k.shell(floor_strip=True, style="ship", avoid=lamps, low_gaps=[("E", -10.65, 1.5)])
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 2500.0, 6.0, COOL, facing=PI / 2, shadow=(i == 1))
    m, kc = k.m, k.coll
    k.add(kit.locker("couloir_arriere_locker0", (0.85, -11.0, 0.0), -PI / 2, m, kc))
    k.add(kit.locker("couloir_arriere_locker1", (0.85, -10.3, 0.0), -PI / 2, m, kc))
    k.add(kit.stencil(k.nm("stencil"), k.wall("E", -10.65, 0, 2.06), Cell.facing("E"), m, kc, length=1.2))
    k.add(kit.vent(k.nm("vent"), k.wall("W", -13.0, 0, 0.75), Cell.facing("W"), m, kc, size=(0.7, 0.22)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("W", -9.0, 0, 1.55), Cell.facing("W"), m, kc))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("E", -3.0, 0, 1.35), Cell.facing("E"), m, kc, size=(0.5, 0.14, 0.62)))
    k.add(kit.pipe(k.nm("riser"), (-1.08, -13.88, 0.32), (-1.08, -13.88, 2.15), m, kc, radius=0.045))
    # Niveau 1.
    k.add(kit.junction_box(k.nm("jbox"), k.wall("W", -11.0, 0, 1.35), Cell.facing("W"), m, kc, detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("W", -5.0, 0, 1.95), Cell.facing("W"), m, kc, length=1.2, detail=1))
    k.add(kit.vent(k.nm("vent"), k.wall("E", -5.0, 0, 0.75), Cell.facing("E"), m, kc, size=(0.7, 0.22), detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("E", -1.0, 0, 1.95), Cell.facing("E"), m, kc, length=0.9, detail=1))
    k.add(kit.pipe(k.nm("conduit"), (-1.08, -12.6, 0.5), (-1.08, -4.2, 0.5), m, kc, radius=0.03, mat="bare", detail=1))
    gauge(k, "W", -7.0, 1.5, detail=1)
    gauge(k, "W", -6.8, 1.5, detail=1)
    # Niveau 2.
    for y0 in (-12.4, -6.6, -2.8):
        hanging_cable(k, (0.42, y0, 2.48), (0.42, y0 + 1.5, 2.48), sag=0.16, detail=2)
    extinguisher(k, "W", -12.2, z=0.62, detail=2)
    valve_wheel(k, (-1.12, -7.0, 1.2), axis="X", radius=0.12, detail=2)
    k.rod("valve_stem", (-1.2, -7.0, 1.2), (-1.12, -7.0, 1.2), 0.02, "dark", detail=2)
    k.ori("warning", (0.4, 0.1, 0.004), k.wall("E", -12.4, 0.002, 2.0), Cell.right("E"), (0, 0, 1), "trim_sheet", uv="warning_red", detail=2)
    wall_conduits(k, 2.03, sides="EW")


def build_soute(k: Cell):
    """Soute : caisses (abris de l'abordage), palette de fûts, pont roulant, zone de levage."""
    lamps = [(3.4, -4.4), (5.8, -4.4), (3.4, -10.4), (5.8, -10.4)]
    k.shell(style="industrial", avoid=lamps)
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 14000.0, 10.0, SODIUM, lens="light_amber", drop=0.3, shadow=(i == 3))
    m, kc = k.m, k.coll
    stacks = [(2.5, -2.35), (2.5, -3.6), (6.7, -2.6), (6.7, -3.9), (6.7, -12.3), (5.45, -12.3), (2.5, -12.4)]
    variants = ["olive", "slate", "yellow", "rust", "slate", "olive", "rust"]
    for i, ((x, y), v) in enumerate(zip(stacks, variants)):
        k.add(kit.crate(f"soute_crate{i}", (x, y, 0.0), 0.04 * (i % 3), m, kc, size=(1.2, 1.2, 1.2), variant=v))
    k.add(kit.crate("soute_crate_top", (2.5, -2.35, 1.2), 0.3, m, kc, variant="rust"))
    k.add(kit.crate("soute_crate_cover", (3.7, -9.6, 0.0), 0.15, m, kc, size=(0.9, 0.9, 0.9), variant="olive"))
    # Palette de fûts entre deux colonnes (est).
    k.box("pallet", (1.0, 1.0, 0.12), (6.75, -8.4, 0.06), "crate", bevel=0.005)
    for dx, dy, mat in ((-0.22, -0.22, "crate_slate"), (0.22, -0.22, "crate_rust"), (-0.22, 0.22, "crate_rust"), (0.22, 0.22, "crate_slate")):
        barrel(k, 6.75 + dx, -8.4 + dy, 0.12, mat, radius=0.2, h=0.8)
    k.col("pallet", (1.0, 1.0, 0.94), (6.75, -8.4, 0.47))
    k.pickup(kit.pickup("soute_pickup_parts", (4.6, -6.6, 0.0), 0.4, m, kc, item="spare_parts"))
    k.add(kit.hazard_frame(k.nm("lift_zone"), (4.6, -6.8, 0.0), (1.6, 1.6), m, kc))
    k.ori("chevrons", (1.4, 0.36, 0.006), (2.7, -7.0, 0.003), (1, 0, 0), (0, 1, 0), "trim_sheet", uv="chevrons", tile=0.8)
    # Pont roulant : chemins de roulement, poutre, palan au-dessus de la zone de levage.
    zr = k.h - 0.26
    for x in (2.2, 7.0):
        k.add(kit.ceiling_beam(k.nm("runway"), (x, -13.3, zr), (x, -1.7, zr), m, kc, depth=0.2, width=0.18))
    zb = zr - 0.2
    k.add(kit.ceiling_beam(k.nm("bridge"), (2.2, -6.8, zb), (7.0, -6.8, zb), m, kc, depth=0.24, width=0.22, mat=m["paint_yellow"]))
    for x in (2.2, 7.0):
        k.box("truck", (0.3, 0.5, 0.14), (x, -6.8, zb + 0.02), "dark", bevel=0.01)
    zh = zb - 0.24
    k.box("hoist", (0.42, 0.5, 0.34), (4.6, -6.8, zh - 0.17), "dark", bevel=0.02)
    k.ori("hoist_label", (0.3, 0.1, 0.004), (4.6, -7.052, zh - 0.12), (1, 0, 0), (0, 0, 1), "trim_sheet", uv="hazard")
    k.cyl("hoist_cable", 0.012, zh - 0.34 - 2.3, (4.6, -6.8, (zh - 0.34 + 2.3) / 2), "dark", segments=6)
    k.box("hook_block", (0.16, 0.1, 0.12), (4.6, -6.8, 2.24), "paint_yellow", bevel=0.01)
    k.add(kit.cabinet(k.nm("manifest"), k.wall("N", 4.6, 0.04), Cell.facing("N"), m, kc, size=(1.1, 0.32, 1.7)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("S", 4.6, 0.04, 1.75), Cell.facing("S"), m, kc, size=(0.8, 0.46)))
    k.add(kit.beacon(k.nm("beacon"), k.wall("W", -9.4 + 1.0, 0.04, 2.72), Cell.facing("W"), m, kc))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("E", -6.4, 0.04, 1.8), Cell.facing("E"), m, kc))

    # --- Niveau 1 ----------------------------------------------------------------------------
    for i in (2, 3, 4):
        x, y = stacks[i]
        cargo_net(k, (x, y, 0.0), 0.04 * (i % 3) + PI, detail=1)
    for y in (-6.1, -7.5):
        k.add(kit.hazard_stripe(k.nm("lane"), (1.9, y, 0.0), (3.7, y, 0.0), m, kc, width=0.1, detail=1))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("N", 6.6, 0.04, 1.75), Cell.facing("N"), m, kc, detail=1))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("S", 2.6, 0.04, 1.6), Cell.facing("S"), m, kc, detail=1))
    k.add(kit.duct(k.nm("duct"), (7.05, -13.35, 2.62), (7.05, -1.65, 2.62), m, kc, size=(0.46, 0.3), hang_to=k.h, detail=1))
    k.add(kit.pipe(k.nm("pipe"), (2.08, -13.35, 2.62), (2.08, -1.65, 2.62), m, kc, radius=0.08, hang_to=k.h, detail=1))
    k.add(kit.cable_tray(k.nm("tray"), (1.85, -2.05, 2.72), (7.35, -2.05, 2.72), m, kc, hang_to=zr, detail=1))
    k.add(kit.stencil(k.nm("stencil"), k.wall("W", -10.4, 0.04, 2.3), Cell.facing("W"), m, kc, length=1.4, detail=1))
    k.add(kit.pipe(k.nm("riser"), (5.0, -13.25, 0.0), (5.0, -13.25, k.h), m, kc, radius=0.07, detail=1))

    # --- Niveau 2 ----------------------------------------------------------------------------
    # Petites caisses et boîtes sur les piles, bâche, outils, chaîne du palan.
    for (x, y, z, s, v, rot) in ((6.7, -2.6, 1.2, 0.6, "slate", 0.2), (6.6, -12.25, 1.2, 0.5, "yellow", -0.3), (2.45, -3.6, 1.2, 0.7, "olive", 0.1)):
        k.add(kit.crate(k.nm("small_crate"), (x, y, z), rot, m, kc, size=(s, s, s * 0.8), variant=v, detail=2))
    k.box("tarp", (1.26, 1.26, 0.04), (5.45, -12.3, 1.22), "fabric_dark", bevel=0.015, detail=2)
    k.box("tarp_fold", (1.26, 0.06, 0.5), (5.45, -11.66, 0.98), "fabric_dark", detail=2)
    k.box("toolbox", (0.42, 0.2, 0.18), (3.7, -9.6, 0.99), "paint_red", bevel=0.01, detail=2)
    k.box("wrench", (0.3, 0.04, 0.015), (3.55, -9.4, 0.908), "bare", detail=2)
    hanging_cable(k, (4.6, -6.75, 2.32), (5.2, -6.6, 2.6), sag=0.22, radius=0.01, mat="dark", detail=2)
    for i, (sx, sy, x, y, a) in enumerate(((0.6, 0.35, 5.3, -9.3, 0.5), (0.3, 0.22, 5.62, -9.5, -0.3), (0.45, 0.2, 5.9, -10.0, 1.1))):
        o = k.box("oil", (sx, sy, 0.002), (x, y, 0.001 + 0.0004 * i), "rubber", bevel=0.0009, detail=2)
        o.rotation_euler[2] = a
    gas_tank(k, 7.3, -4.95, 0.0, radius=0.11, h=0.9, mat="paint_red", detail=2)
    gas_tank(k, 7.3, -4.66, 0.0, radius=0.11, h=0.9, mat="crate_slate", detail=2)
    k.ori("warning", (0.6, 0.1, 0.004), k.wall("E", -10.4, 0.044, 2.3), Cell.right("E"), (0, 0, 1), "trim_sheet", uv="warning_red", detail=2)
    wall_conduits(k, 2.8, d=0.27, sides="SEW", skip=[("W", -8.8, -8.0)])
    for x, y, z, rot in ((6.7, -3.9, 1.2, 0.5), (2.5, -12.4, 1.2, -0.2)):
        for dx, dy in ((-0.2, -0.15), (0.2, 0.15)):
            barrel(k, x + dx, y + dy, z, "crate_rust" if dx < 0 else "crate_slate", radius=0.16, h=0.5, detail=2)
    for (x, y) in ((6.7, -12.3), (2.5, -3.6)):
        cargo_net(k, (x, y, 0.0), PI, detail=2)


def build_machines(k: Cell):
    """Salle des machines : réacteur au centre ; console de commande plaquée au mur nord, à côté
    de la porte, reliée à la paroi par ses gaines ; pompes, armoires, ambre."""
    lamps = [(-2.2, -17.0), (2.2, -17.0), (-2.2, -25.0), (2.2, -25.0)]
    k.shell(style="industrial", avoid=lamps)
    for i, (x, y) in enumerate(lamps):
        k.lamp(f"l{i}", x, y, 13000.0, 10.0, SODIUM, lens="light_amber", drop=0.3, shadow=(i == 0))
    m, kc = k.m, k.coll
    k.add(kit.reactor("machines_reactor", (0.0, -21.0, 0.0), m, kc, ceiling=k.h), structure=True)
    # Console contre le mur nord : dos du caisson (0,3 m derrière son centre) contre le
    # soubassement (0,04 m), entre la nervure de x = 3 et le mur est.
    cx, cy = 3.95, k.y1 - 0.04 - 0.31
    k.add(kit.console("machines_console", (cx, cy, 0.0), PI, m, kc, label="engineering"))
    k.add(kit.junction_box(k.nm("jbox"), k.wall("N", cx, 0.04, 2.15), Cell.facing("N"), m, kc, size=(0.6, 0.16, 0.5)))
    for dx in (-0.45, 0.45):
        k.add(kit.pipe(k.nm("feed"), (cx + dx, k.y1 - 0.12, 1.6), (cx + dx, k.y1 - 0.12, 1.92), m, kc, radius=0.03, mat="rubber"))
    k.add(kit.crate("machines_crate0", (-3.6, -16.2, 0.0), 0.2, m, kc, variant="slate"))
    k.add(kit.crate("machines_crate1", (-3.8, -17.4, 0.0), -0.1, m, kc, size=(1.2, 1.2, 1.0), variant="rust"))
    k.add(kit.hazard_frame(k.nm("reactor_zone"), (0.0, -21.0, 0.0), (2.7, 2.7), m, kc))
    k.add(kit.floor_grating(k.nm("grating"), (0.0, -17.9, 0.0), (1.4, 0.8), 0.0, m, kc))
    k.add(kit.floor_grating(k.nm("grating"), (0.0, -24.7, 0.0), (1.4, 0.9), 0.0, m, kc))
    k.add(kit.pump(k.nm("pump"), (4.3, -20.95, 0.0), PI / 2, m, kc, height=k.h))
    k.add(kit.pump(k.nm("pump"), (-4.3, -25.0, 0.0), -PI / 2, m, kc, height=k.h))
    k.add(kit.cabinet(k.nm("cabinet"), k.wall("W", -21.0, 0.04), Cell.facing("W"), m, kc, size=(1.2, 0.32, 1.7)))
    k.add(kit.cabinet(k.nm("cabinet"), k.wall("E", -25.0, 0.04), Cell.facing("E"), m, kc, size=(1.1, 0.32, 1.7)))
    k.add(kit.wall_screen(k.nm("screen"), k.wall("N", -3.9, 0.04, 1.8), Cell.facing("N"), m, kc, size=(0.9, 0.46)))
    k.add(kit.beacon(k.nm("beacon"), k.wall("W", -21.0, 0.04, 2.45), Cell.facing("W"), m, kc))
    k.add(kit.pipe(k.nm("riser"), (0.0, -25.86, 0.0), (0.0, -25.86, k.h), m, kc, radius=0.1))
    zt = k.h - 0.27
    k.add(kit.pipe(k.nm("pipe"), (3.55, -25.95, zt - 0.12), (3.55, -14.45, zt - 0.12), m, kc, radius=0.09, hang_to=k.h))
    # Étagère de pièces détachées (paroi ouest) : meuble de base, objets au niveau 2.
    k.add(kit.shelf(k.nm("shelf"), k.wall("W", -23.0, 0.04), Cell.facing("W"), m, kc, width=1.2, height=1.8, depth=0.36, items_detail=2))

    # --- Niveau 1 ----------------------------------------------------------------------------
    k.add(kit.junction_box(k.nm("jbox"), k.wall("N", -2.0, 0.04, 1.7), Cell.facing("N"), m, kc, detail=1))
    for x in (-2.0, 2.0):
        k.add(kit.junction_box(k.nm("jbox"), k.wall("S", x, 0.04, 1.6), Cell.facing("S"), m, kc, size=(0.5, 0.16, 0.62), detail=1))
    k.add(kit.pipe(k.nm("riser"), (-4.86, -19.0, 0.0), (-4.86, -19.0, k.h), m, kc, radius=0.06, detail=1))
    k.add(kit.beacon(k.nm("beacon"), k.wall("E", -17.0, 0.04, 2.45), Cell.facing("E"), m, kc, detail=1))
    k.add(kit.duct(k.nm("duct"), (-3.6, -25.95, zt - 0.2), (-3.6, -14.45, zt - 0.2), m, kc, size=(0.55, 0.34), hang_to=k.h, detail=1))
    k.add(kit.cable_tray(k.nm("tray"), (1.2, -25.95, zt - 0.05), (1.2, -14.45, zt - 0.05), m, kc, hang_to=k.h, detail=1))
    gauge(k, "N", 2.4, 1.6, d=0.04, detail=1)
    gauge(k, "N", 2.6, 1.6, d=0.04, detail=1)
    k.add(kit.stencil(k.nm("stencil"), k.wall("N", cx, 0.04, 2.55), Cell.facing("N"), m, kc, length=0.9, detail=1))

    # --- Niveau 2 ----------------------------------------------------------------------------
    k.add(kit.pipe(k.nm("pipe"), (3.85, -25.95, zt - 0.08), (3.85, -14.45, zt - 0.08), m, kc, radius=0.05, hang_to=k.h, detail=2))
    for y in (-18.0, -23.5):
        valve_wheel(k, (3.55, y, zt - 0.32), axis="Z", radius=0.13, detail=2)
        k.rod("valve_stem", (3.55, y, zt - 0.21), (3.55, y, zt - 0.32), 0.02, "dark", detail=2)
    for y0 in (-24.6, -19.6, -16.0):
        hanging_cable(k, (1.2, y0, zt - 0.05), (1.2, y0 + 1.4, zt - 0.05), sag=0.25, detail=2)
    # Panneau à outils (paroi est) et taches d'huile au sol.
    k.box("toolboard", (0.03, 1.0, 0.7), (k.x1 - 0.055, -22.9, 1.55), "dark", detail=2)
    for i, (dy, dz, sz) in enumerate(((-0.35, 0.15, (0.03, 0.05, 0.3)), (-0.15, 0.12, (0.03, 0.04, 0.24)), (0.08, 0.18, (0.03, 0.18, 0.05)),
                                      (0.3, 0.1, (0.03, 0.06, 0.28)), (-0.2, -0.18, (0.03, 0.3, 0.04)), (0.25, -0.15, (0.03, 0.12, 0.12)))):
        k.box("tool", sz, (k.x1 - 0.085, -22.9 + dy, 1.55 + dz), "bare" if i % 2 else "paint_red", detail=2)
    for i, (x, y, a) in enumerate(((-1.8, -18.6, 0.4), (2.4, -23.6, -0.6), (-2.6, -23.0, 1.0), (-2.3, -23.3, 0.2))):
        o = k.box("oil", (0.6 - 0.12 * i, 0.4 - 0.06 * i, 0.002), (x, y, 0.001 + 0.0004 * i), "rubber", bevel=0.0009, detail=2)
        o.rotation_euler[2] = a
    barrel(k, -3.7, -17.4, 1.0, "crate_slate", radius=0.2, h=0.6, detail=2)
    k.add(kit.crate(k.nm("small_crate"), (-3.55, -16.25, 0.9), 0.4, m, kc, size=(0.5, 0.5, 0.4), variant="yellow", detail=2))
    extinguisher(k, "N", -1.6, detail=2)
    wall_conduits(k, 2.8, d=0.27, skip=[("S", -0.4, 0.4)])
    for y in (-16.6, -19.4):
        gauge(k, "E", y, 1.7, d=0.04, detail=2)
        gauge(k, "W", y - 0.6, 1.7, d=0.04, detail=2)


BUILDERS = {
    "machines": build_machines,
    "couloir_arriere": build_couloir_arriere,
    "sas": build_sas,
    "soute": build_soute,
    "couloir_avant": build_couloir_avant,
    "quartiers": build_quartiers,
    "chambre": build_chambre,
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


# --- Statistiques, contrôles ----------------------------------------------------------------------


def _tris(objs) -> int:
    return sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in objs if o.type == "MESH")


def cell_stats(k: Cell) -> str:
    by_level = {}
    for o in k.visual:
        if o.type == "MESH":
            by_level[c.detail_of(o)] = by_level.get(c.detail_of(o), 0) + _tris([o])
    levels = ", ".join(f"n{lv} {n}" for lv, n in sorted(by_level.items()))
    mats = sorted({mt.name for o in k.visual + k.pickups if o.type == "MESH" for mt in o.data.materials if mt})
    return (f"{k.id} : {sum(by_level.values())} triangles ({levels}), {len(mats)} matériaux, {len(k.colliders)} colliders, "
            f"{len(k.marks)} repères")


def check_gameplay(cells: dict[str, Cell]) -> None:
    """Gameplay : jamais de collider ni de repère de gameplay optionnel ; chaque siège a son œil,
    chaque siège, console et casier son volume ; les casiers de la chambre sont là."""
    names = {mk.name for k in cells.values() for mk in k.marks}
    for k in cells.values():
        bad = [o.name for o in k.colliders if c.detail_of(o) > 0]
        if bad:
            raise RuntimeError(f"{k.id} : colliders optionnels {bad[:5]}")
        for mk in k.marks:
            kind = mk.get("interact")
            if kind in ("seat", "console", "locker") and "half_extents" not in mk.keys():
                raise RuntimeError(f"{mk.name} ({kind}) sans half_extents")
            if kind == "seat" and f"SOCKET_{mk.name[len('INTERACT_'):]}_eye" not in names:
                raise RuntimeError(f"{mk.name} sans œil")
    lockers = [mk for k in cells.values() for mk in k.marks if mk.get("interact") == "locker"]
    containers = sorted(mk["container"] for mk in lockers if mk.get("container"))
    locked = sorted(mk.name for mk in lockers if not mk.get("container"))
    print(f"rangements fonctionnels : {containers}")
    print(f"casiers verrouillés (décor) : {locked}")
    if len(containers) != len(set(containers)):
        raise RuntimeError(f"contenus en double : {containers}")
    for need in FUNCTIONAL_CONTAINERS:
        if need not in containers:
            raise RuntimeError(f"rangement {need} absent")
    # M10 (docs/m10.md §3.3) : chaque casier de la chambre a sa plaque de nom (volume et ancre).
    for i in (1, 2, 3):
        cid = f"corvette_chambre_casier_{i}"
        for node, key in ((f"INTERACT_chambre_lockers_{i}_label", "interact"), (f"SOCKET_chambre_lockers_{i}_label", "usage")):
            mk = bpy.data.objects.get(node)
            if mk is None or mk.get("container") != cid or mk.get(key) not in ("locker_label", "label"):
                raise RuntimeError(f"plaque de nom de {cid} absente ou fausse ({node})")
    check_interact_volumes(cells)


# Rangements fonctionnels de la corvette (contenus de départ et règles dans gameplay/items.ron).
FUNCTIONAL_CONTAINERS = ("corvette_chambre_casier_1", "corvette_chambre_casier_2", "corvette_chambre_casier_3", "corvette_carre_frigo")


def _interact_box(mk):
    """Boîte englobante (Blender, monde) du capteur d'un `INTERACT_*` : pavé `half_extents`
    ([x, hauteur, profondeur] dans le repère local Y-haut du nœud), sinon sphère de 0,35 m."""
    he = mk.get("half_extents")
    hx, hz, hy = (he[0], he[1], he[2]) if he is not None else (0.35, 0.35, 0.35)
    pts = [mk.matrix_world @ Vector((sx * hx, sy * hy, sz * hz)) for sx in (-1, 1) for sy in (-1, 1) for sz in (-1, 1)]
    return tuple(min(p[i] for p in pts) for i in range(3)), tuple(max(p[i] for p in pts) for i in range(3))


def _overlap(a, b, eps=0.0):
    return all(a[0][i] < b[1][i] - eps and b[0][i] < a[1][i] - eps for i in range(3))


def check_interact_volumes(cells: dict[str, Cell]) -> None:
    """Volumes d'interaction : aucun ne doit en chevaucher un autre (sauf une plaque de nom, posée
    exprès devant son casier, et le siège et sa console de pilotage) ni empiéter sur le passage
    d'une porte (1,4 m de large, 0,6 m de part et d'autre de la cloison). Avertissements
    seulement : la visée choisit le capteur le plus proche."""
    bpy.context.view_layer.update()
    boxes = {mk.name: _interact_box(mk) for k in cells.values() for mk in k.marks if mk.name.startswith("INTERACT_")}
    names = sorted(boxes)
    warn = []
    allowed = {("INTERACT_cockpit_helm", "INTERACT_cockpit_pilot"), ("INTERACT_cockpit_copilot", "INTERACT_cockpit_nav")}
    for i, a in enumerate(names):
        for b in names[i + 1:]:
            if b == f"{a}_label" or a == f"{b}_label" or (a, b) in allowed:
                continue
            if _overlap(boxes[a], boxes[b], eps=0.005):
                warn.append(f"{a} chevauche {b}")
    for p in PORTALS:
        a_, b_, kind, x, y, axis = p[:6]
        if kind != "Door":
            continue
        hw = kit.DOOR_W / 2
        lane = ((x - 0.7, y - hw, 0.0), (x + 0.7, y + hw, kit.DOOR_H)) if axis == "x" else ((x - hw, y - 0.7, 0.0), (x + hw, y + 0.7, kit.DOOR_H))
        for n in names:
            if _overlap(boxes[n], lane, eps=0.005):
                warn.append(f"{n} empiète sur le passage de la porte {a_}/{b_}")
    for w in warn:
        print(f"AVERTISSEMENT volume : {w}")
    print(f"volumes d'interaction : {len(names)} ({len(warn)} avertissements)")


def verify_gltf(rel_path: str) -> None:
    """Chaque maillage rendu (hors `_col`) a un matériau, TEXCOORD_0 et TEXCOORD_1 (lightmap)."""
    import json

    data = json.loads((c.ROOT / rel_path).read_text(encoding="utf-8"))
    for node in data.get("nodes", []):
        if "mesh" not in node or node["name"].endswith("_col"):
            continue
        for prim in data["meshes"][node["mesh"]]["primitives"]:
            attrs = prim.get("attributes", {})
            missing = [a for a in ("TEXCOORD_0", "TEXCOORD_1") if a not in attrs]
            if "material" not in prim or missing:
                raise RuntimeError(f"{rel_path} : {node['name']} sans matériau ou sans {missing}")


def write_ron(lightmaps: dict) -> None:
    """Descripteur du secteur (`sector_tools.sector_ron`), porte extérieure nommée `sas__@dock`
    comme celle de `player_ship`."""
    text = st.sector_ron(META, CELLS, PORTALS, SPAWNS, lightmaps)
    text = text.replace('id: "sas__dock"', 'id: "sas__@dock"')
    c.write_text(RON_PATH, text)


def report_envelope() -> None:
    """Cotes pour la coque (lot C) et le hangar (lot D), dans le repère du jeu."""
    lo = [1e9, 1e9, 1e9]
    hi = [-1e9, -1e9, -1e9]
    for x0, x1, y0, y1, h in CELLS.values():
        a = c.to_bevy((x0 - kit.WALL, y1 + kit.WALL, -kit.WALL))
        b = c.to_bevy((x1 + kit.WALL, y0 - kit.WALL, h + kit.WALL))
        for i in range(3):
            lo[i] = min(lo[i], a[i], b[i])
            hi[i] = max(hi[i], a[i], b[i])
    print(f"enveloppe (jeu, cloisons comprises) : min {tuple(round(v, 3) for v in lo)}, max {tuple(round(v, 3) for v in hi)}")
    dock = st.portal_record(CELLS, *PORTALS[-1])
    print(f"porte extérieure sas__@dock (jeu) : centre {dock['center']}, normale {dock['normal']}, {dock['width']} × {dock['height']} m")


# --- Aperçus ------------------------------------------------------------------------------------

EYE = 1.7


def _pilot_eye():
    sock = bpy.data.objects.get("SOCKET_cockpit_pilot_eye")
    bpy.context.view_layer.update()
    return tuple(sock.matrix_world.translation) if sock else (-1.0, 19.18, 1.25)


def shots() -> dict:
    """Prises de vue : nom → (œil, cible, champ vertical en degrés (75 : celui du jeu), niveaux de
    décor à rendre). Une vue « ortho » : vue de dessus en coupe."""
    ex, ey, ez = _pilot_eye()
    all3, both = (0, 1, 2), (0, 2)
    return {
        "ensemble": ("ortho", None, None, (2,)),
        "cockpit_entree": ((0.0, 14.9, EYE), (0.0, 20.4, 1.45), 75.0, all3),
        "cockpit_pilote": ((ex, ey, ez), (ex, ey + 5.0, ez - 0.25), 75.0, (2,)),
        "cockpit_pilote_gauche": ((ex, ey, ez), (ex - 4.0, ey + 2.6, ez - 0.3), 75.0, (2,)),
        "cockpit_cote": ((-3.3, 15.1, EYE), (1.6, 20.2, 1.0), 75.0, (2,)),
        "cockpit_arriere": ((2.6, 18.6, EYE), (-1.0, 14.4, 1.2), 75.0, both),
        "chambre": ((-2.2, 9.6, EYE), (-7.0, 12.6, 0.9), 75.0, all3),
        "chambre_depart": ((-5.3, 11.6, EYE), (-5.3 + 3.0 * 0.866, 11.6 - 3.0 * 0.5, 1.45), 75.0, all3),
        "chambre_casiers": ((-5.6, 12.6, EYE), (-4.2, 8.0, 1.0), 75.0, both),
        "chambre_bureau": ((-3.1, 11.3, EYE), (-4.7, 14.0, 0.95), 75.0, both),
        "sas": ((-2.0, -3.0, EYE), (-7.6, -3.0, 1.2), 75.0, all3),
        "sas_retour": ((-7.0, -3.4, EYE), (-1.6, -3.0, 1.2), 75.0, both),
        "sas_combinaisons": ((-3.6, -4.2, EYE), (-4.8, -1.2, 1.2), 75.0, (2,)),
        "machines": ((0.0, -14.9, EYE), (0.0, -21.0, 1.1), 75.0, both),
        "machines_console": ((1.5, -17.3, EYE), (3.95, -14.4, 1.0), 75.0, both),
        "soute": ((2.2, -6.6, EYE), (6.4, -11.6, 1.0), 75.0, both),
        "soute_fond": ((5.2, -10.8, EYE), (3.0, -3.0, 1.2), 75.0, (2,)),
        "couloir_avant": ((0.0, 0.6, EYE), (0.0, 14.0, 1.4), 75.0, both),
        "couloir_arriere": ((0.0, -0.6, EYE), (0.0, -14.0, 1.4), 75.0, (2,)),
        "quartiers": ((-2.2, 4.0, EYE), (-7.2, 5.6, 0.9), 75.0, both),
        "carre": ((2.2, 5.0, EYE), (7.2, 7.4, 0.9), 75.0, both),
        # M10 (lot B) : retours de l'auteur, avant / après sous les mêmes angles.
        "quartiers_couchettes": ((-3.4, 4.6, 1.95), (-7.2, 6.6, 1.35), 75.0, (2,)),
        "quartiers_couchette_haut": ((-5.0, 4.9, 1.9), (-7.2, 6.4, 1.45), 75.0, (2,)),
        "chambre_lit": ((-4.4, 10.4, EYE), (-7.0, 12.5, 0.55), 75.0, both),
        "carre_cuisine": ((3.2, 5.6, EYE), (7.4, 3.4, 1.1), 75.0, both),
        "sas_porte": ((-4.4, -3.0, EYE), (-7.7, -3.0, 1.25), 75.0, (2,)),
        "chambre_plaques": ((-4.6, 10.1, 1.6), (-4.6, 8.0, 1.25), 60.0, (2,)),
        "soute_compteur": ((4.6, -4.0, EYE), (4.6, -1.6, 1.45), 75.0, (2,)),
        "sas_compteur": ((-3.0, -2.2, EYE), (-2.5, -4.8, 1.45), 75.0, (2,)),
        "cockpit_compteur": ((-1.2, 17.0, EYE), (-4.0, 17.4, 1.6), 75.0, (1, 2)),
        "cockpit_compteur_s": ((2.2, 16.8, EYE), (3.0, 14.4, 1.45), 75.0, (2,)),
        "machines_compteur": ((-2.0, -21.0, EYE), (-5.0, -21.0, 1.6), 75.0, (2,)),
    }


def _camera(eye, target, fov):
    data = bpy.data.cameras.get("preview") or bpy.data.cameras.new("preview")
    cam = bpy.data.objects.get("preview") or bpy.data.objects.new("preview", data)
    if cam.name not in bpy.context.scene.collection.objects:
        bpy.context.scene.collection.objects.link(cam)
    data.clip_start, data.clip_end = 0.03, 200.0
    if eye == "ortho":
        data.type = "ORTHO"
        data.ortho_scale = 50.0
        cam.location = (0.0, -2.8, 40.0)
        cam.rotation_euler = (0.0, 0.0, PI / 2)
    else:
        data.type = "PERSP"
        data.sensor_fit = "VERTICAL"
        data.angle = math.radians(fov)
        cam.location = eye
        cam.rotation_euler = (Vector(target) - Vector(eye)).to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = cam


def _render_set(objs, lamps, out_prefix, only):
    """Rend chaque prise de vue à chaque niveau de décor demandé : un objet (ou une lampe) de
    niveau supérieur est masqué. `objs` : maillages montrés."""
    import preview as pv

    for name, (eye, target, fov, levels) in shots().items():
        if only and name not in only:
            continue
        if eye == "ortho" and out_prefix == "jeu_":
            continue  # maillages fusionnés par matériau : on ne peut pas ôter les plafonds
        _camera(eye, target, fov)
        cut = []
        if eye == "ortho":
            # Vue de dessus en coupe : plafonds et tout ce qui est en hauteur masqués.
            for o in objs:
                if not o.hide_render and min((o.matrix_world @ Vector(b)).z for b in o.bound_box) > 2.0:
                    o.hide_render = True
                    cut.append(o)
            pv.world((0.6, 0.65, 0.7), 0.6)
        for lv in levels:
            for o in objs:
                if o not in cut:
                    o.hide_render = c.detail_of(o) > lv
            for lamp, d in lamps:
                lamp.hide_render = d > lv
            suffix = "" if levels == (2,) else f"_n{lv}"
            out = Path(os.environ.get("SH_PREVIEW_DIR", str(PREVIEWS)))
            out.mkdir(parents=True, exist_ok=True)
            pv.render(out / f"{os.environ.get('SH_PREVIEW_PREFIX', '')}{out_prefix}{name}{suffix}.png")
            print(f"aperçu : {out_prefix}{name}{suffix}.png")
        for o in cut:
            o.hide_render = False
        if eye == "ortho":
            pv.world((0.012, 0.02, 0.04), 1.0)


def lit_previews(cells, m, samples, res, only):
    """Aperçus Cycles éclairés par les lampes (une par repère LIGHT_) et les surfaces émissives ;
    fond bleu nuit derrière la verrière (aperçu seulement)."""
    import preview as pv

    w, h = (int(v) for v in res.split("x"))
    pv.setup_cycles(samples, w, h, exposure=float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2")))
    pv.world((0.012, 0.02, 0.04), 1.0)
    doors = st.door_panels(PORTALS, CELLS, m, c.collection("PREVIEW_DOORS"), style="ship", door_models=META["door_models"])
    shown = [o for k in cells.values() for o in k.visual + k.pickups] + doors
    keep = set(shown)
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            obj.hide_render = obj not in keep
            if obj.data.materials and obj.data.materials[0] and obj.data.materials[0].name.startswith("sh_light"):
                obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = False
    marks = [mk for k in cells.values() for mk in k.marks]
    lamps = []
    for mk, lamp in zip([mk for mk in marks if mk.name.startswith("LIGHT_")], lm.spawn_lamps(marks, c.collection("PREVIEW_LAMPS"))):
        lamp.data.shadow_soft_size = 0.06
        lamps.append((lamp, c.detail_of(mk)))
    _render_set(shown, lamps, "lit_", only)


def game_previews(result, m, samples, res, only):
    """Aperçus « comme dans le jeu » (Bas et Moyen, cellules à lightmap) : émission seule (couleur
    × (1 − métal) × lightmap × exposition + émissif), sans lampe ni rebond ; portes éclairées par
    une ambiante constante."""
    import preview as pv

    w, h = (int(v) for v in res.split("x"))
    exposure = float(os.environ.get("SH_PREVIEW_EXPOSURE", "2.2"))
    pv.setup_cycles(samples, w, h, exposure=exposure)
    bpy.context.scene.cycles.max_bounces = 0
    pv.world((0.012, 0.02, 0.04), 1.0)
    for obj in list(bpy.context.scene.objects):
        if obj.type == "LIGHT":
            obj.hide_render = True
    shown = []
    for cid, s in result["stats"].items():
        info = result["lightmaps"].get(cid)
        image = bpy.data.images.load(str(c.ROOT / "assets" / info["image"]), check_existing=True)
        image.colorspace_settings.name = "sRGB"
        scale = info["exposure"] / lm.WATTS_TO_LUMENS
        for obj in s["merged"]:
            obj.visible_diffuse = obj.visible_glossy = obj.visible_transmission = True
            mats = obj.data.materials
            for i, mat in enumerate(mats):
                mats[i] = st.game_material(mat, image, scale, cid, exposure)
            shown.append(obj)
    doors = st.door_panels(PORTALS, CELLS, m, c.collection("PREVIEW_DOORS_GAME"), style="ship", door_models=META["door_models"])
    ambient = float(os.environ.get("SH_PREVIEW_DOOR_AMBIENT", "0.35"))
    cache = {}
    for obj in doors:
        mats = obj.data.materials
        for i, mat in enumerate(mats):
            if mat.name not in cache:
                cache[mat.name] = st.game_material(mat, None, ambient, "door", exposure)
            mats[i] = cache[mat.name]
    shown += doors
    keep = set(shown)
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            obj.hide_render = obj not in keep
    _render_set(shown, [], "jeu_", only)


def load_lightmaps() -> dict:
    """Lightmaps déjà cuites (descripteur existant) : pour `--game` après un `--no-bake`."""
    import re

    path = c.ROOT / RON_PATH
    if not path.exists():
        return {}
    out = {}
    text = path.read_text(encoding="utf-8")
    for cid, img, expo in re.findall(r'id: "(\w+)",.*?lightmap: \(image: "([^"]+)", exposure: ([\d.]+)\)', text, re.S):
        out[cid] = {"image": img, "exposure": float(expo)}
    return out


def main(argv) -> None:
    do_export = "--no-export" not in argv
    bake = do_export and "--no-bake" not in argv
    samples = int(argv[argv.index("--samples") + 1]) if "--samples" in argv else 32
    res = argv[argv.index("--res") + 1] if "--res" in argv else "1280x720"
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    c.reset_scene()
    m = palette()
    cells = build_all(m)
    for k in cells.values():
        print("construit " + cell_stats(k))
    check_gameplay(cells)
    report_envelope()
    PREVIEWS.mkdir(parents=True, exist_ok=True)
    result = None
    if do_export:
        built = {cid: {"visual": k.visual, "structure": k.structure, "colliders": k.colliders, "marks": k.marks, "pickups": k.pickups}
                 for cid, k in cells.items()}
        result = st.export_sector(META, CELLS, PORTALS, SPAWNS, built, RON_PATH, BLEND_PATH, bake=bake)
        lightmaps = result["lightmaps"] or ({} if bake else {})
        write_ron(lightmaps)
        for cid in CELLS:
            verify_gltf(f"assets/{MODELS}/{cid}.gltf")
            verify_gltf(f"assets/{MODELS}/{cid}_lod.gltf")
        for cid, s in result["stats"].items():
            levels = ", ".join(f"n{lv} {n}" for lv, n in sorted(s["tris_by_detail"].items()))
            print(f"exporté {cid} : {s['tris']} triangles ({levels}), LOD {s['lod_tris']}, {len(s['materials'])} matériaux, "
                  f"{len(s['merged'])} maillages, lightmap {s['res']} px")
    else:
        # Aperçus seuls : la source `art/` reste celle du dernier export.
        c.save_blend(str((PREVIEWS / f"{SECTOR}_apercus.blend").relative_to(c.ROOT)))
    if "--lit" in argv and not do_export:
        lit_previews(cells, m, samples, res, only)
    if "--game" in argv and result is not None:
        if not result["lightmaps"]:
            result["lightmaps"] = load_lightmaps()
        if len(result["lightmaps"]) == len(CELLS):
            game_previews(result, m, max(8, samples // 2), res, only)
        else:
            print("aperçus jeu : lightmaps absentes (lancer d'abord le bake)")
    print(f"{SECTOR} : {len(CELLS)} cellules, {len(PORTALS)} portails")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
