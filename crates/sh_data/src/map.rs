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

use sh_core::{GridCell, Id, MapPos};

use crate::common::Place;
use crate::factions::Faction;
use crate::galaxy::{BodyKind, Galaxy};
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
}
