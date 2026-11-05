//! Ce que montre la carte galactique (vue Galaxie) : uniquement des informations **publiques**.
//!
//! La carte, ses étiquettes et ses infobulles ne lisent rien d'autre que [`public_map`] : un
//! système, astre, station, phénomène ou zone secret n'y entre pas, et aucune note secrète non
//! plus (règle d'or n°3, AGENTS.md). Les infobulles sont faites de champs structurés (case,
//! faction, population, hub), traduits par la langue de l'interface ; la prose du canon n'y
//! figure pas.
//!
//! Le reste du module est la géométrie pure de la carte (M9), testée sans moteur : placement des
//! étiquettes sans chevauchement ([`layout_labels`]), éclaircissement des icônes trop serrées
//! ([`thin_markers`]) et découpe des zones en triangles ([`triangulate`]).
//!
//! **Détail d'un système** (M10) : au plus près, la carte montre les astres, les stations et les
//! points de livraison d'un système ([`system_detail`]). Ils viennent de la disposition publique
//! de la vue Globale (`layout::system_layout`, qui écarte tout astre ou station secret) et des
//! cibles de navigation (`scene::nav_points_in`) : les mêmes identifiants que l'autopilote. Le
//! système tient dans un disque de [`DETAIL_RADIUS`] unités de carte ; les distances réelles
//! (des centaines de kilomètres aux milliards de mètres) y sont rendues lisibles par une échelle
//! logarithmique **par anneau** (les planètes autour de l'étoile, les satellites autour de leur
//! astre), et les satellites d'un même astre s'écartent d'un angle minimal pour que leurs
//! icônes et leurs noms ne s'empilent pas.

use std::collections::BTreeMap;
use std::f64::consts::TAU;

use sh_core::{GridCell, Id, MapPos, SpacePos};

use crate::GameData;
use crate::common::Place;
use crate::factions::Faction;
use crate::galaxy::{BodyKind, Galaxy, StarSystem};
use crate::lang::Lang;
use crate::layout::system_layout;
use crate::scene::{NavKind, nav_points_in, system_structures};
use crate::travel::endpoint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapKind {
    System,
    /// Astre hors système (planète isolée, errante).
    Body,
    /// Station hors système.
    Station,
    Phenomenon,
}

/// Une entrée de la carte.
#[derive(Clone, Debug, PartialEq)]
pub struct MapEntry {
    pub kind: MapKind,
    pub id: Id<Place>,
    pub name: String,
    pub pos: MapPos,
    pub cell: Option<GridCell>,
    /// Rayon sur la carte (phénomènes étendus).
    pub radius: Option<f64>,
    pub controller: Option<Id<Faction>>,
    /// Population totale connue (0 : inconnue ou nulle).
    pub population: u64,
    pub has_portal_hub: bool,
    pub destroyed: bool,
    /// Planète errante, sans étoile (§3.4).
    pub rogue: bool,
    /// L'astre autour duquel tourne une lune isolée (Maïa et Mérope autour d'Alcyone).
    pub parent: Option<Id<Place>>,
    /// Phénomène qui bloque l'hyperespace (le Croissant, §3.6 : fait public).
    pub blocks_hyperspace: bool,
    /// On peut y aller (système, ou monde isolé depuis M9 : [`crate::travel::endpoint`]).
    pub destination: bool,
}

/// Les entrées publiques de la carte, dans l'ordre des données.
pub fn public_map(galaxy: &Galaxy) -> Vec<MapEntry> {
    let hub_bodies: Vec<&Id<Place>> = galaxy.portal_hubs.iter().map(|h| &h.body).collect();
    let mut out = Vec::new();
    for s in galaxy.systems.iter().filter(|s| !s.secret) {
        out.push(MapEntry {
            kind: MapKind::System,
            id: s.id.clone(),
            name: s.name.clone(),
            pos: s.center,
            cell: Some(s.cell),
            radius: None,
            controller: s.controller.clone(),
            population: s.population_total,
            has_portal_hub: s.bodies.iter().any(|b| hub_bodies.contains(&b)),
            destroyed: false,
            rogue: false,
            parent: None,
            blocks_hyperspace: false,
            destination: true,
        });
    }
    for b in galaxy
        .bodies
        .iter()
        .filter(|b| !b.secret && b.system.is_none() && b.kind != BodyKind::Star)
    {
        let Some(pos) = b.pos else { continue };
        // Une lune d'un astre secret ne se montre pas non plus (elle le trahirait).
        let parent = match &b.parent {
            Some(p) => match galaxy.body(p) {
                Some(parent) if !parent.secret => Some(p.clone()),
                _ => continue,
            },
            None => None,
        };
        out.push(MapEntry {
            kind: MapKind::Body,
            id: b.id.clone(),
            name: b.name.clone(),
            pos,
            cell: b.cell,
            radius: None,
            controller: b.controller.clone(),
            population: b.population.as_ref().map_or(0, |p| p.total),
            has_portal_hub: hub_bodies.contains(&&b.id),
            destroyed: b.destroyed,
            rogue: b.kind == BodyKind::RoguePlanet,
            parent,
            blocks_hyperspace: false,
            destination: endpoint(galaxy, &b.id).is_some(),
        });
    }
    for st in galaxy
        .stations
        .iter()
        .filter(|s| !s.secret && s.system.is_none())
    {
        out.push(MapEntry {
            kind: MapKind::Station,
            id: st.id.clone(),
            name: st.name.clone(),
            pos: st.pos,
            cell: Some(st.cell),
            radius: None,
            controller: st.controller.clone(),
            population: st.population.total,
            has_portal_hub: false,
            destroyed: false,
            rogue: false,
            parent: None,
            blocks_hyperspace: false,
            destination: false,
        });
    }
    for p in galaxy.phenomena.iter().filter(|p| !p.secret) {
        let Some(pos) = p.pos else { continue };
        out.push(MapEntry {
            kind: MapKind::Phenomenon,
            id: p.id.clone(),
            name: p.name.clone(),
            pos,
            cell: p.cell,
            radius: p.radius,
            controller: None,
            population: 0,
            has_portal_hub: false,
            destroyed: false,
            rogue: false,
            parent: None,
            blocks_hyperspace: p.blocks_hyperspace,
            destination: false,
        });
    }
    out
}

/// L'entrée la plus proche d'un point de la carte, à moins de `reach` unités (un phénomène
/// étendu se survole sur tout son disque).
pub fn entry_at(entries: &[MapEntry], at: MapPos, reach: f64) -> Option<&MapEntry> {
    entries
        .iter()
        .map(|e| {
            let d = (e.pos.x - at.x).hypot(e.pos.y - at.y);
            let d = if e.kind == MapKind::Phenomenon {
                (d - e.radius.unwrap_or(0.0)).max(0.0) + reach * 0.5
            } else {
                d
            };
            (d, e)
        })
        .filter(|(d, _)| *d <= reach)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, e)| e)
}

// --- Géométrie de l'écran de la carte --------------------------------------------------------

/// Un rectangle d'écran, en pixels logiques : coin haut-gauche, largeur, hauteur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ScreenRect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Carré centré sur un point (une icône).
    pub fn around(cx: f32, cy: f32, half: f32) -> Self {
        Self::new(cx - half, cy - half, 2.0 * half, 2.0 * half)
    }

    /// Recouvrement strict : deux rectangles qui se touchent ne se chevauchent pas.
    pub fn overlaps(&self, other: &ScreenRect) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }

    pub fn contains(&self, other: &ScreenRect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.w <= self.x + self.w
            && other.y + other.h <= self.y + self.h
    }
}

/// Une étiquette à placer près de son repère.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelRequest {
    /// Le repère (centre de l'icône), en pixels.
    pub anchor: (f32, f32),
    /// Taille du texte, en pixels.
    pub size: (f32, f32),
    /// Rayon de l'icône à ne pas recouvrir (0 : pas d'icône).
    pub clearance: f32,
    /// Étiquette centrée sur son repère (noms de zones, nébuleuses) : centrée, sinon un peu
    /// au-dessus ou au-dessous.
    pub centered: bool,
}

/// Écart entre une icône et son étiquette, en pixels.
const LABEL_GAP: f32 = 3.0;

/// Places candidates d'une étiquette, dans l'ordre de préférence : à droite, à gauche, dessous,
/// dessus, puis en diagonale (coin haut-gauche de chaque place).
fn candidates(r: &LabelRequest) -> Vec<(f32, f32)> {
    let (ax, ay) = r.anchor;
    let (w, h) = r.size;
    if r.centered {
        let x = ax - w / 2.0;
        return vec![
            (x, ay - h / 2.0),
            (x, ay - 1.6 * h - h / 2.0),
            (x, ay + 1.6 * h - h / 2.0),
        ];
    }
    let c = r.clearance + LABEL_GAP;
    // Les diagonales partent du coin de l'icône (à 45°), pas de son côté.
    let d = r.clearance * std::f32::consts::FRAC_1_SQRT_2 + LABEL_GAP;
    vec![
        (ax + c, ay - h / 2.0),
        (ax - c - w, ay - h / 2.0),
        (ax - w / 2.0, ay + c),
        (ax - w / 2.0, ay - c - h),
        (ax + d, ay - d - h),
        (ax + d, ay + d),
        (ax - d - w, ay - d - h),
        (ax - d - w, ay + d),
    ]
}

/// Place les étiquettes **dans l'ordre donné** (la plus importante d'abord), chacune à la
/// première place libre : dans `bounds`, sans recouvrir une étiquette déjà placée ni un
/// obstacle (les icônes, les panneaux). Sans place libre, l'étiquette est cachée (`None`) : à
/// grande échelle, seuls les noms importants restent, et les autres reviennent au zoom.
pub fn layout_labels(
    requests: &[LabelRequest],
    obstacles: &[ScreenRect],
    bounds: ScreenRect,
) -> Vec<Option<(f32, f32)>> {
    let mut placed: Vec<ScreenRect> = Vec::new();
    let mut out = Vec::with_capacity(requests.len());
    for r in requests {
        let found = candidates(r).into_iter().find_map(|(x, y)| {
            let rect = ScreenRect::new(x, y, r.size.0, r.size.1);
            let free = bounds.contains(&rect)
                && !placed.iter().any(|p| p.overlaps(&rect))
                && !obstacles.iter().any(|o| o.overlaps(&rect));
            free.then_some(rect)
        });
        out.push(found.map(|rect| {
            placed.push(rect);
            (rect.x, rect.y)
        }));
    }
    out
}

/// Éclaircit des icônes trop serrées : dans l'ordre donné (la plus importante d'abord), une
/// icône `(x, y, rayon)` reste si elle ne recouvre pas de plus de la moitié une icône déjà
/// gardée. Une lune collée à sa planète disparaît à grande échelle et revient au zoom.
pub fn thin_markers(markers: &[(f32, f32, f32)]) -> Vec<bool> {
    let mut kept: Vec<(f32, f32, f32)> = Vec::new();
    markers
        .iter()
        .map(|&(x, y, r)| {
            let crowded = kept.iter().any(|&(kx, ky, kr)| {
                let d = (x - kx).hypot(y - ky);
                d < (r + kr) - 0.5 * r.min(kr)
            });
            if !crowded {
                kept.push((x, y, r));
            }
            !crowded
        })
        .collect()
}

/// Découpe un polygone simple (convexe ou non, sans trou ni croisement) en triangles, par
/// « oreilles » successives. Indices dans `points`. Les contours des zones de la carte sont
/// concaves : un éventail depuis un sommet déborderait.
pub fn triangulate(points: &[MapPos]) -> Vec<[usize; 3]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    // Aire signée : le sens du contour décide de quel côté est l'intérieur.
    let area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let cross =
        |a: MapPos, b: MapPos, c: MapPos| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut remaining: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity(n - 2);
    // Au plus n² essais : un contour dégénéré (points alignés) ne boucle pas sans fin.
    let mut guard = n * n;
    while remaining.len() > 3 && guard > 0 {
        guard -= 1;
        let m = remaining.len();
        let ear = (0..m).find(|&i| {
            let (ia, ib, ic) = (
                remaining[(i + m - 1) % m],
                remaining[i],
                remaining[(i + 1) % m],
            );
            let (a, b, c) = (points[ia], points[ib], points[ic]);
            if sign * cross(a, b, c) <= 1e-12 {
                return false;
            }
            // Aucun autre sommet dans le triangle (bord compris).
            !remaining.iter().any(|&j| {
                if j == ia || j == ib || j == ic {
                    return false;
                }
                let p = points[j];
                sign * cross(a, b, p) >= 0.0
                    && sign * cross(b, c, p) >= 0.0
                    && sign * cross(c, a, p) >= 0.0
            })
        });
        let Some(i) = ear else { break };
        out.push([
            remaining[(i + m - 1) % m],
            remaining[i],
            remaining[(i + 1) % m],
        ]);
        remaining.remove(i);
    }
    if remaining.len() == 3 {
        out.push([remaining[0], remaining[1], remaining[2]]);
    }
    out
}

// --- Détail d'un système (M10) ---------------------------------------------------------------

/// Rayon du disque où se dessine le détail d'un système, en unités de carte. Les deux systèmes
/// publics les plus proches (Vorrh et Ferrok) sont à ≈ 42 unités : deux détails ne se touchent
/// jamais.
pub const DETAIL_RADIUS: f64 = 8.0;
/// Le détail apparaît quand son disque mesure ce rayon à l'écran (pixels), et l'emporte tout à
/// fait sur l'icône du système à partir de celui-ci.
pub const DETAIL_FADE_START: f32 = 140.0;
pub const DETAIL_FADE_END: f32 = 240.0;
/// Les planètes : l'orbite la plus proche à 30 % du rayon, la plus lointaine au bord ; une
/// planète seule aux 62 %.
const PLANET_INNER: f64 = 0.30;
const PLANET_SINGLE: f64 = 0.62;
/// L'anneau des satellites d'un astre : au plus 45 % de l'anneau de son parent, et au plus la
/// moitié de l'écart à son voisin le plus proche (deux anneaux voisins ne se recouvrent pas).
const SATELLITE_SHARE: f64 = 0.45;
const SATELLITE_GAP: f64 = 0.5;
/// Les satellites : le plus proche à 40 % de l'anneau, le plus lointain au bord ; seul, aux 75 %.
const SATELLITE_INNER: f64 = 0.40;
const SATELLITE_SINGLE: f64 = 0.75;
/// Écart angulaire minimal entre deux satellites d'un même astre (degrés). Les structures
/// d'Hexis sont à quelques degrés l'une de l'autre dans la vue Globale : sur la carte, elles se
/// confondraient.
const SATELLITE_SPREAD_DEG: f64 = 55.0;

/// Un point du détail d'un système : un astre, une station, une structure (point de livraison)
/// ou un port, à sa place sur la carte.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailPoint {
    /// L'identifiant de navigation (`scene::NavPoint::id`, celui qu'accepte l'autopilote) ;
    /// `None` pour une étoile que le canon ne nomme pas (décor, pas une cible).
    pub id: Option<String>,
    /// Nom affiché (`None` : rien à écrire).
    pub name: Option<String>,
    pub kind: NavKind,
    /// Un point de livraison des contrats.
    pub delivery: bool,
    /// Position réelle dans le système (m), celle de la vue Globale.
    pub real: SpacePos,
    /// L'astre autour duquel il tourne (indice dans la liste ; `None` : l'étoile, au centre).
    pub parent: Option<usize>,
    /// Décalage sur la carte depuis le centre du système (unités de carte ; x vers l'est, y vers
    /// le sud, comme la carte).
    pub offset: (f64, f64),
}

impl DetailPoint {
    /// Une cible que l'on peut choisir (pour l'autopilote ou un itinéraire).
    pub fn is_target(&self) -> bool {
        self.id.is_some()
    }

    /// Sa position sur la carte, dans un système centré en `center`.
    pub fn map_pos(&self, center: MapPos) -> MapPos {
        MapPos::new(center.x + self.offset.0, center.y + self.offset.1)
    }
}

/// Le détail d'un système public, dans l'ordre de la disposition (étoiles, planètes,
/// satellites, stations), puis les structures, puis les ports. Vide pour un système secret.
pub fn system_detail(data: &GameData, lang: &Lang, system: &StarSystem) -> Vec<DetailPoint> {
    if system.secret {
        return Vec::new();
    }
    let g = &data.galaxy;
    let placed = system_layout(g, system, &data.space.scale);
    let nav = nav_points_in(data, lang, Some(system), &placed);
    let scene = data.scene(&system.id);

    // Autour de quoi tourne chaque cible : la disposition le dit des astres et des stations,
    // la mise en scène des structures et des ports.
    let mut around: BTreeMap<String, String> = BTreeMap::new();
    for p in &placed {
        if let (Some(id), Some(parent)) = (&p.id, &p.around) {
            around.insert(id.to_string(), parent.to_string());
        }
    }
    for st in system_structures(g, scene, &system.id) {
        around.insert(st.id, st.around.to_string());
    }
    for dock in scene.map(|s| s.docks.as_slice()).unwrap_or_default() {
        around.insert(dock.id.clone(), dock.around.to_string());
    }

    let mut points: Vec<DetailPoint> = Vec::new();
    // Une étoile que le canon ne nomme pas : au centre, en décor ; la mise en scène peut lui
    // donner un nom d'affichage (« Étoile d'Hexis », P11).
    for p in placed.iter().filter(|p| p.id.is_none()) {
        points.push(DetailPoint {
            id: None,
            name: scene.and_then(|s| s.star_name.clone()),
            kind: NavKind::Body(p.kind),
            delivery: false,
            real: p.pos,
            parent: None,
            offset: (0.0, 0.0),
        });
    }
    for n in &nav {
        points.push(DetailPoint {
            id: Some(n.id.clone()),
            name: Some(n.name.clone()),
            kind: n.kind,
            delivery: n.is_delivery(),
            real: n.pos,
            parent: None,
            offset: (0.0, 0.0),
        });
    }
    let index: BTreeMap<String, usize> = points
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.id.clone().map(|id| (id, i)))
        .collect();
    for p in &mut points {
        p.parent =
            p.id.as_ref()
                .and_then(|id| around.get(id))
                .and_then(|parent| index.get(parent))
                .copied();
    }
    let geometry: Vec<(SpacePos, Option<usize>)> =
        points.iter().map(|p| (p.real, p.parent)).collect();
    for (p, offset) in points
        .iter_mut()
        .zip(detail_offsets(&geometry, DETAIL_RADIUS))
    {
        p.offset = offset;
    }
    points
}

/// Le décalage sur la carte de chaque point `(position réelle, parent)` d'un système, dans un
/// disque de rayon `radius` (unités de carte). Anneau par anneau, du centre vers l'extérieur :
/// la direction réelle de chaque point depuis son parent est gardée (celle de la carte, pour les
/// planètes : `layout`), sa distance passe à l'échelle logarithmique de son anneau, et les
/// satellites d'un même parent s'écartent d'un angle minimal. Un parent inconnu ou circulaire
/// compte pour l'étoile.
pub fn detail_offsets(points: &[(SpacePos, Option<usize>)], radius: f64) -> Vec<(f64, f64)> {
    let n = points.len();
    let parent = |i: usize| points[i].1.filter(|&p| p < n && p != i);
    // Profondeur de chaque point (0 : autour de l'étoile), bornée contre les boucles.
    let depth: Vec<usize> = (0..n)
        .map(|i| {
            let (mut d, mut at) = (0, i);
            while let Some(p) = parent(at) {
                d += 1;
                at = p;
                if d > n {
                    return 0;
                }
            }
            d
        })
        .collect();
    let effective_parent = |i: usize| parent(i).filter(|_| depth[i] > 0);
    let mut out = vec![(0.0, 0.0); n];
    // Le rayon de l'anneau où se rangent les satellites de chaque point.
    let mut ring = vec![0.0; n];
    let max_depth = depth.iter().copied().max().unwrap_or(0);
    for level in 0..=max_depth {
        let mut groups: BTreeMap<Option<usize>, Vec<usize>> = BTreeMap::new();
        for i in (0..n).filter(|&i| depth[i] == level) {
            groups.entry(effective_parent(i)).or_default().push(i);
        }
        for (owner, members) in &groups {
            let (origin_real, origin, size, inner, single) = match owner {
                None => (
                    SpacePos::ZERO,
                    (0.0, 0.0),
                    radius,
                    PLANET_INNER,
                    PLANET_SINGLE,
                ),
                Some(p) => (
                    points[*p].0,
                    out[*p],
                    ring[*p],
                    SATELLITE_INNER,
                    SATELLITE_SINGLE,
                ),
            };
            let polar: Vec<(f64, f64)> = members
                .iter()
                .map(|&i| {
                    let v = points[i].0 - origin_real;
                    (v.x.hypot(v.z), v.z.atan2(v.x))
                })
                .collect();
            let distances: Vec<f64> = polar.iter().map(|p| p.0).collect();
            let fractions = ring_fractions(&distances, inner, single);
            let mut angles: Vec<f64> = polar.iter().map(|p| p.1).collect();
            if owner.is_some() {
                // Seuls les satellites qui ne sont pas au centre de leur parent s'écartent.
                let moving: Vec<usize> =
                    (0..members.len()).filter(|&k| fractions[k] > 0.0).collect();
                let spread = spread_angles(
                    &moving.iter().map(|&k| angles[k]).collect::<Vec<_>>(),
                    SATELLITE_SPREAD_DEG.to_radians(),
                );
                for (&k, a) in moving.iter().zip(spread) {
                    angles[k] = a;
                }
            }
            for (k, &i) in members.iter().enumerate() {
                let r = size * fractions[k];
                out[i] = (
                    origin.0 + r * angles[k].cos(),
                    origin.1 + r * angles[k].sin(),
                );
            }
        }
        // L'anneau des satellites de chaque point de ce niveau : une part de l'anneau de son
        // parent, et moins de la moitié de l'écart à son plus proche voisin (le parent, ou un
        // frère).
        for (owner, members) in &groups {
            let (origin, size) = match owner {
                None => ((0.0, 0.0), radius),
                Some(p) => (out[*p], ring[*p]),
            };
            for &i in members {
                let at = out[i];
                let dist = |q: (f64, f64)| (q.0 - at.0).hypot(q.1 - at.1);
                let mut nearest = f64::INFINITY;
                if dist(origin) > 1e-9 {
                    nearest = dist(origin);
                }
                for &j in members.iter().filter(|&&j| j != i) {
                    let d = dist(out[j]);
                    if d > 1e-9 {
                        nearest = nearest.min(d);
                    }
                }
                ring[i] = (SATELLITE_SHARE * size).min(SATELLITE_GAP * nearest);
            }
        }
    }
    out
}

/// La place de chaque distance sur son anneau (fraction du rayon) : 0 au centre, puis de
/// `inner` (la plus proche) à 1 (la plus lointaine) en échelle logarithmique ; `single` si
/// toutes sont égales.
fn ring_fractions(distances: &[f64], inner: f64, single: f64) -> Vec<f64> {
    const EPS: f64 = 1e-6;
    let logs: Vec<f64> = distances
        .iter()
        .filter(|&&d| d > EPS)
        .map(|d| d.ln())
        .collect();
    let lo = logs.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    distances
        .iter()
        .map(|&d| {
            if d <= EPS {
                0.0
            } else if hi - lo < 1e-9 {
                single
            } else {
                inner + (1.0 - inner) * (d.ln() - lo) / (hi - lo)
            }
        })
        .collect()
}

/// Écarte des angles (radians) d'au moins `min_sep` (réduit si le cercle ne suffit pas), sans
/// changer leur ordre autour du cercle : deux voisins trop proches se repoussent également,
/// jusqu'à ce que tout écart suffise.
pub fn spread_angles(angles: &[f64], min_sep: f64) -> Vec<f64> {
    let n = angles.len();
    if n < 2 {
        return angles.to_vec();
    }
    let min_sep = min_sep.min(0.9 * TAU / n as f64);
    let mut order: Vec<usize> = (0..n).collect();
    let norm: Vec<f64> = angles.iter().map(|a| a.rem_euclid(TAU)).collect();
    order.sort_by(|&a, &b| norm[a].total_cmp(&norm[b]).then(a.cmp(&b)));
    let mut a: Vec<f64> = order.iter().map(|&i| norm[i]).collect();
    for _ in 0..500 {
        let mut moved = false;
        for k in 0..n {
            let next = (k + 1) % n;
            let gap = if next == 0 {
                a[0] + TAU - a[k]
            } else {
                a[next] - a[k]
            };
            if gap < min_sep - 1e-9 {
                let push = (min_sep - gap) / 2.0 + 1e-12;
                a[k] -= push;
                a[next] += push;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    let mut out = vec![0.0; n];
    for (k, &i) in order.iter().enumerate() {
        out[i] = a[k];
    }
    out
}

/// L'opacité du détail des systèmes (0 : la carte de la galaxie ; 1 : le détail seul), selon
/// le rayon à l'écran du disque d'un système (pixels). Fondu doux entre les deux seuils.
pub fn detail_fade(radius_px: f32) -> f32 {
    let t =
        ((radius_px - DETAIL_FADE_START) / (DETAIL_FADE_END - DETAIL_FADE_START)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Ce que fait un clic sur une cible de la carte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetClick {
    /// La proposer à l'autopilote : elle est dans le lieu où est le vaisseau.
    Autopilot,
    /// Tracer un itinéraire vers le lieu qui la contient.
    Route,
    Nothing,
}

/// L'autopilote ne vise que le lieu courant (système ou monde isolé) : ailleurs, une cible mène
/// à un itinéraire vers son lieu, s'il est une destination. Sans lieu connu, rien.
pub fn target_click(
    here: Option<&Id<Place>>,
    owner: &Id<Place>,
    owner_is_destination: bool,
) -> TargetClick {
    match here {
        None => TargetClick::Nothing,
        Some(here) if here == owner => TargetClick::Autopilot,
        Some(_) if owner_is_destination => TargetClick::Route,
        Some(_) => TargetClick::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(x: f32, y: f32) -> LabelRequest {
        LabelRequest {
            anchor: (x, y),
            size: (80.0, 16.0),
            clearance: 10.0,
            centered: false,
        }
    }

    #[test]
    fn les_etiquettes_ne_se_chevauchent_pas() {
        let bounds = ScreenRect::new(0.0, 0.0, 1600.0, 900.0);
        // Des repères serrés, comme Alcyone et ses lunes, ou Karza près de Rade-Noire.
        let requests: Vec<LabelRequest> = [
            (400.0, 300.0),
            (412.0, 302.0),
            (424.0, 300.0),
            (418.0, 318.0),
            (800.0, 450.0),
            (805.0, 455.0),
        ]
        .iter()
        .map(|&(x, y)| label(x, y))
        .collect();
        let icons: Vec<ScreenRect> = requests
            .iter()
            .map(|r| ScreenRect::around(r.anchor.0, r.anchor.1, r.clearance))
            .collect();
        let placed = layout_labels(&requests, &icons, bounds);
        let rects: Vec<ScreenRect> = placed
            .iter()
            .zip(&requests)
            .filter_map(|(p, r)| p.map(|(x, y)| ScreenRect::new(x, y, r.size.0, r.size.1)))
            .collect();
        assert!(rects.len() >= 4, "trop d'étiquettes cachées : {placed:?}");
        for (i, a) in rects.iter().enumerate() {
            assert!(bounds.contains(a));
            for b in &rects[i + 1..] {
                assert!(!a.overlaps(b), "{a:?} recouvre {b:?}");
            }
            for icon in &icons {
                assert!(!a.overlaps(icon), "{a:?} recouvre l'icône {icon:?}");
            }
        }
        // La plus importante est toujours placée (ici à gauche : ses voisines bouchent la droite).
        assert_eq!(
            placed[0],
            Some((400.0 - 10.0 - LABEL_GAP - 80.0, 300.0 - 8.0))
        );
    }

    #[test]
    fn une_etiquette_sans_place_est_cachee() {
        let bounds = ScreenRect::new(0.0, 0.0, 200.0, 100.0);
        // Place libre : à droite de son icône, centrée en hauteur.
        let placed = layout_labels(&[label(50.0, 50.0)], &[], bounds);
        assert_eq!(placed, vec![Some((50.0 + 10.0 + LABEL_GAP, 42.0))]);
        // Un panneau couvre tout l'écran : aucune place.
        let panel = ScreenRect::new(0.0, 0.0, 200.0, 100.0);
        assert_eq!(
            layout_labels(&[label(100.0, 50.0)], &[panel], bounds),
            vec![None]
        );
        // Au bord droit de l'écran, l'étiquette passe à gauche de son icône.
        let placed = layout_labels(&[label(190.0, 50.0)], &[], bounds);
        assert_eq!(placed, vec![Some((190.0 - 10.0 - LABEL_GAP - 80.0, 42.0))]);
    }

    #[test]
    fn les_icones_serrees_s_eclaircissent() {
        // Une planète (rayon 11) et deux lunes à 4 pixels : elles disparaissent ; une troisième
        // icône bien à l'écart reste.
        let kept = thin_markers(&[
            (100.0, 100.0, 11.0),
            (104.0, 100.0, 7.0),
            (96.0, 100.0, 7.0),
            (200.0, 100.0, 11.0),
        ]);
        assert_eq!(kept, vec![true, false, false, true]);
        // Au zoom, à 30 pixels l'une de l'autre, tout le monde reste.
        let kept = thin_markers(&[(100.0, 100.0, 11.0), (130.0, 100.0, 7.0)]);
        assert_eq!(kept, vec![true, true]);
    }

    #[test]
    fn un_contour_concave_se_decoupe_sans_deborder() {
        // Un « U » : l'éventail depuis le premier sommet couvrirait le creux.
        let p = |x: f64, y: f64| MapPos::new(x, y);
        let points = vec![
            p(0.0, 0.0),
            p(3.0, 0.0),
            p(3.0, 3.0),
            p(2.0, 3.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 3.0),
            p(0.0, 3.0),
        ];
        let triangles = triangulate(&points);
        assert_eq!(triangles.len(), points.len() - 2);
        let area = |t: &[usize; 3]| {
            let (a, b, c) = (points[t[0]], points[t[1]], points[t[2]]);
            ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() / 2.0
        };
        let total: f64 = triangles.iter().map(area).sum();
        // Aire du « U » : 3 × 3 moins le creux 1 × 2.
        assert!((total - 7.0).abs() < 1e-9, "{total}");
        // Le centre du creux n'est dans aucun triangle.
        let hole = p(1.5, 2.0);
        for t in &triangles {
            let (a, b, c) = (points[t[0]], points[t[1]], points[t[2]]);
            let s =
                |u: MapPos, v: MapPos| (v.x - u.x) * (hole.y - u.y) - (v.y - u.y) * (hole.x - u.x);
            let (d1, d2, d3) = (s(a, b), s(b, c), s(c, a));
            let inside = (d1 > 0.0 && d2 > 0.0 && d3 > 0.0) || (d1 < 0.0 && d2 < 0.0 && d3 < 0.0);
            assert!(!inside, "le creux est couvert par {t:?}");
        }
    }

    // --- Détail d'un système ---

    fn game_data() -> GameData {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
        GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"))
    }

    fn place(s: &str) -> Id<Place> {
        Id::new(s).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn les_satellites_s_ecartent_sans_changer_d_ordre() {
        // Trois structures à quelques degrés l'une de l'autre (Hexis), et une quatrième.
        let deg = |d: f64| d.to_radians();
        let input = [deg(50.7), deg(54.5), deg(49.0), deg(200.0)];
        let out = spread_angles(&input, deg(55.0));
        let mut sorted: Vec<(f64, usize)> = out
            .iter()
            .enumerate()
            .map(|(i, a)| (a.rem_euclid(TAU), i))
            .collect();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
        for k in 0..sorted.len() {
            let (a, b) = (sorted[k].0, sorted[(k + 1) % sorted.len()].0);
            let gap = (b - a).rem_euclid(TAU);
            assert!(
                gap >= deg(55.0) - 1e-6,
                "écart {} : {out:?}",
                gap.to_degrees()
            );
        }
        // L'ordre autour du cercle reste celui des entrées (49 < 50,7 < 54,5 < 200).
        let order: Vec<usize> = sorted.iter().map(|s| s.1).collect();
        let start = order.iter().position(|&i| i == 2).unwrap_or(0);
        let rotated: Vec<usize> = order[start..]
            .iter()
            .chain(&order[..start])
            .copied()
            .collect();
        assert_eq!(rotated, vec![2, 0, 1, 3]);
        // Trop de monde pour l'écart demandé : il se réduit, et tout reste distinct.
        let crowd = spread_angles(&[0.0; 12], deg(55.0));
        for (i, a) in crowd.iter().enumerate() {
            for b in &crowd[i + 1..] {
                assert!((a - b).abs() > deg(20.0));
            }
        }
    }

    #[test]
    fn l_echelle_du_detail_est_logarithmique_par_anneau() {
        let p = |x: f64, z: f64| SpacePos::new(x, 0.0, z);
        // L'étoile au centre ; trois planètes de 6e10 à 6e11 m ; une lune et une station autour
        // de la première, à 2e6 et 1e7 m.
        let points = [
            (p(0.0, 0.0), None),
            (p(6e10, 0.0), None),
            (p(0.0, 2e11), None),
            (p(-6e11, 0.0), None),
            (p(6e10 + 2e6, 0.0), Some(1)),
            (p(6e10, 1e7), Some(1)),
        ];
        let out = detail_offsets(&points, 8.0);
        let r = |i: usize| out[i].0.hypot(out[i].1);
        assert_eq!(out[0], (0.0, 0.0));
        // La plus proche à 30 % du rayon, la plus lointaine au bord, la direction gardée.
        assert!((r(1) - 2.4).abs() < 1e-9 && out[1].1.abs() < 1e-9);
        assert!((r(3) - 8.0).abs() < 1e-9 && out[3].0 < 0.0);
        assert!(r(2) > r(1) && r(2) < r(3) && out[2].1 > 0.0);
        // Les satellites restent près de leur astre, hors de lui, chacun à sa place.
        for i in [4, 5] {
            let d = (out[i].0 - out[1].0).hypot(out[i].1 - out[1].1);
            assert!(d > 0.1 && d < 0.5 * 2.4 + 1e-9, "satellite {i} à {d}");
        }
        let (d4, d5) = (
            (out[4].0 - out[1].0).hypot(out[4].1 - out[1].1),
            (out[5].0 - out[1].0).hypot(out[5].1 - out[1].1),
        );
        assert!(
            d4 < d5,
            "la lune (2e6 m) est plus près que la station (1e7 m)"
        );
        // Un parent qui boucle ne fait pas tourner la fonction sans fin.
        let looped = detail_offsets(&[(p(1e9, 0.0), Some(1)), (p(2e9, 0.0), Some(0))], 8.0);
        assert_eq!(looped.len(), 2);
    }

    #[test]
    fn le_detail_d_hexis_montre_ses_cibles_sans_secret() {
        let data = game_data();
        let g = &data.galaxy;
        let hexis = g
            .system(&place("sys_hexis"))
            .unwrap_or_else(|| panic!("Hexis absent"));
        let detail = system_detail(&data, &data.lang, hexis);
        let ids: Vec<&str> = detail.iter().filter_map(|p| p.id.as_deref()).collect();
        for wanted in [
            "hexis",
            "l_enclume",
            "hexis_shipyards",
            "hexis_portal_hub",
            "hexis_spaceport_approach",
        ] {
            assert!(ids.contains(&wanted), "{wanted} absent : {ids:?}");
        }
        // Les points de livraison sont les structures ; l'étoile sans nom n'est pas une cible.
        let delivery: Vec<&str> = detail
            .iter()
            .filter(|p| p.delivery)
            .filter_map(|p| p.id.as_deref())
            .collect();
        assert_eq!(delivery, vec!["hexis_shipyards", "hexis_portal_hub"]);
        assert!(
            detail
                .iter()
                .any(|p| !p.is_target() && p.offset == (0.0, 0.0))
        );
        // Les satellites d'Hexis tournent autour d'elle.
        let at = |id: &str| {
            detail
                .iter()
                .position(|p| p.id.as_deref() == Some(id))
                .unwrap_or_else(|| panic!("{id}"))
        };
        assert_eq!(detail[at("hexis_portal_hub")].parent, Some(at("hexis")));
        assert_eq!(detail[at("l_enclume")].parent, Some(at("hexis")));
    }

    #[test]
    fn le_detail_de_chaque_systeme_tient_dans_son_disque() {
        let data = game_data();
        let g = &data.galaxy;
        for system in &g.systems {
            let detail = system_detail(&data, &data.lang, system);
            if system.secret {
                assert!(detail.is_empty(), "{} est secret", system.id);
                continue;
            }
            for (i, p) in detail.iter().enumerate() {
                let r = p.offset.0.hypot(p.offset.1);
                // Un satellite peut dépasser un peu le bord, jamais au point d'approcher un
                // autre système.
                assert!(r <= 1.5 * DETAIL_RADIUS, "{} : {:?} à {r}", system.id, p.id);
                // Aucun astre ou station secret.
                if let Some(id) = &p.id {
                    let id = place(id);
                    assert!(!g.body(&id).is_some_and(|b| b.secret), "{id} secret");
                    assert!(!g.station(&id).is_some_and(|s| s.secret), "{id} secret");
                }
                // Deux cibles ne tombent jamais au même endroit.
                for q in &detail[i + 1..] {
                    let d = (p.offset.0 - q.offset.0).hypot(p.offset.1 - q.offset.1);
                    assert!(
                        d > 0.02 * DETAIL_RADIUS,
                        "{} : {:?} et {:?} confondus ({d})",
                        system.id,
                        p.id,
                        q.id
                    );
                }
            }
        }
    }

    #[test]
    fn le_detail_apparait_en_fondu() {
        assert_eq!(detail_fade(0.0), 0.0);
        assert_eq!(detail_fade(DETAIL_FADE_START), 0.0);
        assert_eq!(detail_fade(DETAIL_FADE_END), 1.0);
        assert_eq!(detail_fade(10_000.0), 1.0);
        let mid = detail_fade(0.5 * (DETAIL_FADE_START + DETAIL_FADE_END));
        assert!((mid - 0.5).abs() < 1e-6);
        assert!(detail_fade(150.0) < detail_fade(160.0));
    }

    #[test]
    fn l_autopilote_ne_vise_que_le_lieu_courant() {
        let (hexis, cygnara) = (place("sys_hexis"), place("sys_cygnara"));
        assert_eq!(
            target_click(Some(&hexis), &hexis, true),
            TargetClick::Autopilot
        );
        assert_eq!(
            target_click(Some(&hexis), &cygnara, true),
            TargetClick::Route
        );
        assert_eq!(
            target_click(Some(&hexis), &cygnara, false),
            TargetClick::Nothing
        );
        assert_eq!(target_click(None, &hexis, true), TargetClick::Nothing);
    }
}
