//! Disposition de jeu d'un système stellaire en vue Globale (proposition P2).
//!
//! Le canon ne donne pas d'orbites : la carte est schématique et l'ordre orbital n'est connu que
//! lorsqu'il est indiqué (§0 de l'export, point 7). Cette règle, générique et déterministe,
//! produit une disposition jouable sans rien affirmer :
//! - étoile au centre ;
//! - planètes sur les rangs orbitaux de l'échelle ([`SystemScale::orbit_radius`]) : le rang
//!   canon quand il est connu, sinon l'ordre de distance au centre sur la carte ;
//! - satellites (lunes, et tout astre dont le parent n'est pas une étoile) autour de leur parent ;
//! - stations en orbite de stationnement autour de l'astre le plus proche sur la carte ;
//! - **direction** de chaque astre reprise de la carte, pour que le système reste reconnaissable.
//!
//! Plan du système = plan XZ du jeu ; X = est de la carte, Z = sud de la carte (y croissant).
//!
//! Un **monde isolé** (M9, Q18) n'a pas de système : [`isolated_layout`] le pose au centre, ses
//! lunes publiques autour par la même règle que les satellites, et aucune étoile (le canon n'en
//! nomme pas ; la lumière vient de loin, voir la vue Globale).

use std::collections::BTreeSet;
use std::f64::consts::TAU;

use sh_core::{Id, MapPos, SpacePos};

use crate::common::Place;
use crate::galaxy::{Body, BodyKind, Galaxy, OrbitOrder, StarSystem};
use crate::settings::SystemScale;
use crate::travel::isolated_root;

/// Genre d'un objet disposé (détermine sa taille et son rendu).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacedKind {
    Star,
    Planet,
    GasGiant,
    Moon,
    Station,
}

/// Un astre ou une station, à sa place de jeu.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    /// `None` pour une étoile que le canon ne nomme ni ne décrit.
    pub id: Option<Id<Place>>,
    pub kind: PlacedKind,
    pub pos: SpacePos,
    /// Rayon (m) ; pour une station, celui de son astre parent n'est pas compté.
    pub radius: f64,
    /// L'astre autour duquel il orbite (`None` : l'étoile).
    pub around: Option<Id<Place>>,
}

/// Angle d'or : répartit des directions de secours sans alignement.
const GOLDEN_ANGLE: f64 = TAU * 0.381_966_011_250_105;

fn direction(from: MapPos, to: Option<MapPos>, fallback_index: usize) -> f64 {
    match to {
        Some(p) if (p.x - from.x).hypot(p.y - from.y) > 1e-6 => (p.y - from.y).atan2(p.x - from.x),
        _ => GOLDEN_ANGLE * fallback_index as f64,
    }
}

fn on_plane(center: SpacePos, radius: f64, angle: f64) -> SpacePos {
    center + SpacePos::new(radius * angle.cos(), 0.0, radius * angle.sin())
}

fn is_gas_giant(body: &Body) -> bool {
    body.kind_note.to_lowercase().contains("gazeuse")
}

fn radius_of(body: &Body, scale: &SystemScale) -> (PlacedKind, f64) {
    match body.kind {
        BodyKind::Star => (PlacedKind::Star, scale.star_radius),
        BodyKind::Moon => (PlacedKind::Moon, scale.moon_radius),
        _ if is_gas_giant(body) => (PlacedKind::GasGiant, scale.gas_giant_radius),
        _ => (PlacedKind::Planet, scale.planet_radius),
    }
}

/// Rangs (1 = le plus proche) d'un groupe d'astres frères : rangs canon d'abord, puis les
/// autres par distance croissante sur la carte, puis « le plus extérieur ».
fn ranks<'a>(bodies: &[&'a Body], from: MapPos) -> Vec<(&'a Body, u8)> {
    let mut used: BTreeSet<u8> = bodies
        .iter()
        .filter_map(|b| match b.orbit_order {
            Some(OrbitOrder::Rank(r)) => Some(r),
            _ => None,
        })
        .collect();
    let mut unknown: Vec<&Body> = bodies
        .iter()
        .copied()
        .filter(|b| b.orbit_order.is_none())
        .collect();
    let distance = |b: &Body| {
        b.pos
            .map_or(f64::MAX, |p| (p.x - from.x).hypot(p.y - from.y))
    };
    unknown.sort_by(|a, b| distance(a).total_cmp(&distance(b)).then(a.id.cmp(&b.id)));

    let mut out: Vec<(&Body, u8)> = Vec::new();
    for b in bodies {
        if let Some(OrbitOrder::Rank(r)) = b.orbit_order {
            out.push((b, r));
        }
    }
    let mut next = 1;
    for b in unknown {
        while used.contains(&next) {
            next += 1;
        }
        used.insert(next);
        out.push((b, next));
    }
    let mut outer = used.iter().max().copied().unwrap_or(0);
    for b in bodies {
        if b.orbit_order == Some(OrbitOrder::Outermost) {
            outer += 1;
            out.push((b, outer));
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.id.cmp(&b.0.id)));
    out
}

/// Disposition de jeu des astres et stations **publics** d'un système.
pub fn system_layout(galaxy: &Galaxy, system: &StarSystem, scale: &SystemScale) -> Vec<Placed> {
    let center = system.center;
    let members: Vec<&Body> = system
        .bodies
        .iter()
        .chain(&system.stars)
        .filter_map(|id| galaxy.body(id))
        .filter(|b| !b.secret)
        .collect();
    let is_star = |id: &Option<Id<Place>>| {
        id.as_ref()
            .and_then(|id| galaxy.body(id))
            .is_some_and(|b| b.kind == BodyKind::Star)
    };

    let mut out = Vec::new();
    let mut stars: Vec<&Body> = members
        .iter()
        .copied()
        .filter(|b| b.kind == BodyKind::Star)
        .collect();
    stars.sort_by(|a, b| a.id.cmp(&b.id));
    stars.dedup_by(|a, b| a.id == b.id);
    if stars.is_empty() {
        out.push(Placed {
            id: None,
            kind: PlacedKind::Star,
            pos: SpacePos::ZERO,
            radius: scale.star_radius,
            around: None,
        });
    }
    for (i, star) in stars.iter().enumerate() {
        // Compagnons éventuels : écartés au plus près, sous le premier rang planétaire.
        let pos = if i == 0 {
            SpacePos::ZERO
        } else {
            on_plane(
                SpacePos::ZERO,
                0.5 * scale.orbit_radius(1),
                direction(center, star.pos, i),
            )
        };
        out.push(Placed {
            id: Some(star.id.clone()),
            kind: PlacedKind::Star,
            pos,
            radius: scale.star_radius,
            around: None,
        });
    }

    // Planètes : astres sans parent, ou dont le parent est une étoile.
    let planets: Vec<&Body> = members
        .iter()
        .copied()
        .filter(|b| b.kind != BodyKind::Star && b.kind != BodyKind::Belt)
        .filter(|b| b.parent.is_none() || is_star(&b.parent))
        .collect();
    for (i, (body, rank)) in ranks(&planets, center).into_iter().enumerate() {
        let (kind, radius) = radius_of(body, scale);
        let angle = direction(center, body.pos, i);
        out.push(Placed {
            id: Some(body.id.clone()),
            kind,
            pos: on_plane(SpacePos::ZERO, scale.orbit_radius(rank), angle),
            radius,
            around: None,
        });
    }

    // Satellites, parent par parent (un parent est placé avant ses satellites).
    place_satellites(galaxy, &members, scale, center, &mut out);

    // Stations : en orbite de stationnement autour de l'astre le plus proche sur la carte.
    let orbitable: Vec<(Id<Place>, MapPos, f64, SpacePos)> = out
        .iter()
        .filter(|p| p.kind != PlacedKind::Star)
        .filter_map(|p| {
            let id = p.id.clone()?;
            let map = galaxy.body(&id)?.pos?;
            Some((id, map, p.radius, p.pos))
        })
        .collect();
    let mut stations: Vec<_> = system
        .stations
        .iter()
        .filter_map(|id| galaxy.station(id))
        .filter(|s| !s.secret)
        .collect();
    stations.sort_by(|a, b| a.id.cmp(&b.id));
    for (i, station) in stations.into_iter().enumerate() {
        let near = orbitable.iter().min_by(|a, b| {
            let da = (a.1.x - station.pos.x).hypot(a.1.y - station.pos.y);
            let db = (b.1.x - station.pos.x).hypot(b.1.y - station.pos.y);
            da.total_cmp(&db)
        });
        let (pos, around) = match near {
            Some((id, map, radius, pos)) => (
                on_plane(
                    *pos,
                    radius * (1.0 + scale.parking_orbit_radii),
                    direction(*map, Some(station.pos), i),
                ),
                Some(id.clone()),
            ),
            None => (
                on_plane(
                    SpacePos::ZERO,
                    scale.orbit_radius(1),
                    direction(center, Some(station.pos), i),
                ),
                None,
            ),
        };
        out.push(Placed {
            id: Some(station.id.clone()),
            kind: PlacedKind::Station,
            pos,
            radius: 0.0,
            around,
        });
    }
    out
}

/// Place les satellites (parmi `members`) autour des astres déjà placés dans `out`, parent par
/// parent : un parent est placé avant ses satellites. Les planètes d'une étoile, déjà sur leurs
/// rangs, ne sont pas reprises.
fn place_satellites(
    galaxy: &Galaxy,
    members: &[&Body],
    scale: &SystemScale,
    center: MapPos,
    out: &mut Vec<Placed>,
) {
    let mut placed_ids: Vec<usize> = (0..out.len()).collect();
    while let Some(index) = placed_ids.pop() {
        let parent = out[index].clone();
        let Some(parent_id) = parent
            .id
            .clone()
            .filter(|_| parent.kind != PlacedKind::Star)
        else {
            continue;
        };
        let parent_map = galaxy
            .body(&parent_id)
            .and_then(|b| b.pos)
            .unwrap_or(center);
        let children: Vec<&Body> = members
            .iter()
            .copied()
            .filter(|b| b.parent.as_ref() == Some(&parent_id) && b.kind != BodyKind::Belt)
            .collect();
        for (i, (body, rank)) in ranks(&children, parent_map).into_iter().enumerate() {
            let (kind, radius) = radius_of(body, scale);
            let base = scale.moon_orbit_radii * parent.radius.max(radius);
            let distance = base * (1.0 + 0.6 * f64::from(rank - 1));
            out.push(Placed {
                id: Some(body.id.clone()),
                kind,
                pos: on_plane(parent.pos, distance, direction(parent_map, body.pos, i)),
                radius,
                around: Some(parent_id.clone()),
            });
            placed_ids.push(out.len() - 1);
        }
    }
}

/// Disposition de jeu autour d'un monde isolé public (M9, Q18) : le monde au centre (pour une
/// lune, sa planète : on arrive au même endroit), ses lunes publiques autour, sans étoile. Vide
/// pour un astre secret ou d'un système (il n'a pas de scène isolée).
pub fn isolated_layout(galaxy: &Galaxy, body: &Body, scale: &SystemScale) -> Vec<Placed> {
    if body.secret || body.system.is_some() {
        return Vec::new();
    }
    let root = isolated_root(galaxy, body);
    let center = root.pos.unwrap_or(MapPos::new(0.0, 0.0));
    let (kind, radius) = radius_of(root, scale);
    let mut out = vec![Placed {
        id: Some(root.id.clone()),
        kind,
        pos: SpacePos::ZERO,
        radius,
        around: None,
    }];
    let members: Vec<&Body> = galaxy
        .bodies
        .iter()
        .filter(|b| !b.secret && b.system.is_none() && b.kind != BodyKind::Star)
        .collect();
    place_satellites(galaxy, &members, scale, center, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn un_monde_isole_est_seul_au_centre_avec_ses_lunes() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
        let data = crate::GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"));
        let g = &data.galaxy;
        let scale = &data.space.scale;
        let id = |s: &str| Id::<Place>::new(s).unwrap_or_else(|e| panic!("{e}"));
        let body = |s: &str| g.body(&id(s)).unwrap_or_else(|| panic!("{s} absent"));

        // Calderis, planète errante : seule, au centre, sans étoile.
        let calderis = isolated_layout(g, body("calderis"), scale);
        assert_eq!(calderis.len(), 1);
        assert_eq!(calderis[0].id, Some(id("calderis")));
        assert_eq!(calderis[0].pos, SpacePos::ZERO);
        assert!(calderis.iter().all(|p| p.kind != PlacedKind::Star));

        // Viser Maïa, c'est arriver à Alcyone : la planète au centre, ses deux lunes autour,
        // sur des orbites distinctes, hors de la planète.
        let alcyone = isolated_layout(g, body("maia"), scale);
        assert_eq!(alcyone[0].id, Some(id("alcyone")));
        assert_eq!(alcyone.len(), 3, "{alcyone:?}");
        let planet = alcyone[0].radius;
        for moon in &alcyone[1..] {
            assert_eq!(moon.around, Some(id("alcyone")));
            assert!(moon.pos.length() > 2.0 * planet, "{moon:?}");
        }
        assert!(alcyone[1].pos.distance(alcyone[2].pos) > 2.0 * alcyone[1].radius);

        // Un astre d'un système n'a pas de scène isolée.
        assert!(isolated_layout(g, body("hexis"), scale).is_empty());
    }
}
