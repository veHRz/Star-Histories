//! Planification des voyages interstellaires (AGENTS.md §2.2 de l'export, §6.3) : logique pure.
//!
//! - **Portail** : chemin le plus court en nombre de passages dans le graphe des hubs
//!   (`portal_links`, P19) ; chaque passage coûte une durée et un péage (P20).
//! - **Hyperespace** : ligne droite sur la carte, contournée autour des nébuleuses qui bloquent
//!   l'hyperespace ; durée proportionnelle à la longueur (P3), allongée hors des routes
//!   cartographiées. Un lieu au cœur d'une nébuleuse bloquante (Rade-Noire, dans le Croissant :
//!   « on ne la traverse qu'en vitesse normale », §3.6) ne se rejoint pas par l'hyperespace.
//!
//! Les extrémités d'un voyage sont les systèmes publics et, depuis M9 (Q18), les mondes isolés
//! publics (`sh_data::travel::endpoint`) : on s'y rend par l'hyperespace, arrivée en orbite ; et
//! par Portail s'ils portent un hub canon relié au réseau (Karza).
//! Les lieux secrets ne sont jamais des destinations : aucun plan ne les atteint.

use std::collections::{BTreeMap, VecDeque};

use sh_core::{Credits, Id, MapPos};
use sh_data::travel::{Endpoint, endpoint};
use sh_data::{Galaxy, Place, PortalLinks, ShipLogistics, StarSystem, TravelSettings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TravelMode {
    Portal,
    Hyperspace,
}

/// Un itinéraire chiffré.
#[derive(Clone, Debug, PartialEq)]
pub struct TravelPlan {
    pub mode: TravelMode,
    /// Durée totale, en heures.
    pub hours: f64,
    /// Carburant lourd consommé.
    pub fuel: f64,
    pub toll: Credits,
    /// Hubs franchis (Portail), du départ à l'arrivée.
    pub hubs: Vec<Id<Place>>,
    /// Longueur parcourue sur la carte (hyperespace), en unités.
    pub map_length: f64,
    /// Le trajet contourne au moins une nébuleuse.
    pub detour: bool,
    /// Le carburant suffit.
    pub feasible: bool,
}

impl TravelPlan {
    pub fn days(&self) -> f64 {
        self.hours / 24.0
    }
}

/// L'astre qui porte le hub de Portails d'un système, s'il en a un.
pub fn hub_of<'a>(galaxy: &'a Galaxy, system: &StarSystem) -> Option<&'a Id<Place>> {
    galaxy
        .portal_hubs
        .iter()
        .map(|h| &h.body)
        .find(|b| system.bodies.contains(b))
}

/// Le hub de Portails d'une extrémité : celui de son système, ou celui que porte un monde isolé
/// d'après le canon (Karza, « grand hub de Portails », §3.4 et §3.7). Un hub proposé ne suffit
/// pas à ouvrir un monde isolé aux Portails (Q18 : par défaut, l'hyperespace).
fn endpoint_hub<'a>(galaxy: &'a Galaxy, end: Endpoint<'a>) -> Option<&'a Id<Place>> {
    match end {
        Endpoint::System(system) => hub_of(galaxy, system),
        Endpoint::Isolated(body) => galaxy
            .portal_hubs
            .iter()
            .find(|h| h.body == body.id && h.status.is_canon())
            .map(|h| &h.body),
    }
}

fn fuel_check(hours: f64, ship: &ShipLogistics) -> (f64, bool) {
    let fuel = hours / 24.0 * ship.fuel_per_day;
    (fuel, fuel <= ship.fuel_capacity)
}

/// Itinéraire par Portails, s'il existe (les deux extrémités ont un hub relié au réseau).
pub fn portal_plan(
    galaxy: &Galaxy,
    links: &PortalLinks,
    settings: &TravelSettings,
    ship: &ShipLogistics,
    from: &Id<Place>,
    to: &Id<Place>,
) -> Option<TravelPlan> {
    let start = endpoint_hub(galaxy, endpoint(galaxy, from)?)?;
    let goal = endpoint_hub(galaxy, endpoint(galaxy, to)?)?;
    if start == goal {
        return None;
    }
    // Parcours en largeur : le moins de passages possible ; voisins triés (déterminisme).
    let mut graph: BTreeMap<&Id<Place>, Vec<&Id<Place>>> = BTreeMap::new();
    for (a, b) in &links.links {
        graph.entry(a).or_default().push(b);
        graph.entry(b).or_default().push(a);
    }
    for next in graph.values_mut() {
        next.sort();
    }
    let mut previous: BTreeMap<&Id<Place>, &Id<Place>> = BTreeMap::new();
    let mut queue = VecDeque::from([start]);
    while let Some(hub) = queue.pop_front() {
        if hub == goal {
            break;
        }
        for next in graph.get(hub).into_iter().flatten() {
            if *next != start && !previous.contains_key(next) {
                previous.insert(next, hub);
                queue.push_back(next);
            }
        }
    }
    let mut path = vec![goal.clone()];
    let mut at = goal;
    while at != start {
        at = previous.get(at)?;
        path.push(at.clone());
    }
    path.reverse();
    let jumps = (path.len() - 1) as f64;
    let hours = jumps * settings.portal_transit_hours;
    let (fuel, feasible) = fuel_check(hours, ship);
    Some(TravelPlan {
        mode: TravelMode::Portal,
        hours,
        fuel,
        toll: Credits::units(settings.portal_toll_credits * (path.len() as i64 - 1)),
        hubs: path,
        map_length: 0.0,
        detour: false,
        feasible,
    })
}

/// Les disques que l'hyperespace contourne : nébuleuses bloquantes, rayon élargi de la marge.
pub fn blocking_nebulae(galaxy: &Galaxy, settings: &TravelSettings) -> Vec<(MapPos, f64)> {
    galaxy
        .phenomena
        .iter()
        .filter(|p| p.blocks_hyperspace)
        .filter_map(|p| Some((p.pos?, p.radius? * settings.nebula_margin)))
        .collect()
}

/// Le point est-il au cœur d'un disque interdit ?
pub fn inside_obstacle(pos: MapPos, obstacles: &[(MapPos, f64)]) -> bool {
    obstacles
        .iter()
        .any(|&(c, r)| (pos.x - c.x).hypot(pos.y - c.y) < r)
}

/// Pour un segment `a → b` et un disque `(c, r)` qu'il traverse : distance le long du segment
/// jusqu'au point le plus proche du centre, distance du centre au segment, demi-corde.
fn crossing(a: MapPos, u: (f64, f64), length: f64, c: MapPos, r: f64) -> Option<(f64, f64, f64)> {
    let t = ((c.x - a.x) * u.0 + (c.y - a.y) * u.1).clamp(0.0, length);
    let (px, py) = (a.x + u.0 * t, a.y + u.1 * t);
    let d = (c.x - px).hypot(c.y - py);
    (d < r).then(|| (t, d, (r * r - d * d).sqrt()))
}

/// Longueur d'un trajet en ligne droite de `a` à `b`, contourné autour des disques interdits
/// (`(centre, rayon)`) : la corde qui traverse un disque est remplacée par le plus court des
/// deux arcs du cercle de contournement.
pub fn detour_length(a: MapPos, b: MapPos, obstacles: &[(MapPos, f64)]) -> (f64, bool) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx.hypot(dy);
    if length < 1e-9 {
        return (0.0, false);
    }
    let u = (dx / length, dy / length);
    let mut total = length;
    let mut detour = false;
    for &(c, r) in obstacles {
        let Some((_, d, half_chord)) = crossing(a, u, length, c, r) else {
            continue;
        };
        let half_angle = (half_chord / r).asin();
        // L'arc le plus court entre les deux points d'entrée et de sortie.
        let arc = 2.0
            * r
            * if d > 0.0 {
                half_angle
            } else {
                std::f64::consts::FRAC_PI_2
            };
        total += arc - 2.0 * half_chord;
        detour = true;
    }
    (total, detour)
}

/// Le tracé d'un trajet d'hyperespace sur la carte (pour le dessiner) : la ligne droite, où
/// chaque traversée de disque interdit est remplacée par l'arc le plus court, échantillonné
/// tous les 10° environ. Même règle que [`detour_length`].
pub fn hyperspace_path(a: MapPos, b: MapPos, obstacles: &[(MapPos, f64)]) -> Vec<MapPos> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx.hypot(dy);
    if length < 1e-9 {
        return vec![a, b];
    }
    let u = (dx / length, dy / length);
    // Les traversées dans l'ordre du trajet.
    let mut crossings: Vec<(f64, f64, f64, MapPos, f64)> = obstacles
        .iter()
        .filter_map(|&(c, r)| crossing(a, u, length, c, r).map(|(t, d, h)| (t, d, h, c, r)))
        .collect();
    crossings.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut path = vec![a];
    for (t, d, half_chord, c, r) in crossings {
        let entry = MapPos::new(a.x + u.0 * (t - half_chord), a.y + u.1 * (t - half_chord));
        let exit = MapPos::new(a.x + u.0 * (t + half_chord), a.y + u.1 * (t + half_chord));
        // L'arc court passe du côté du segment opposé au centre ; un trajet qui passe par le
        // centre contourne par la gauche (choix fixe : déterminisme).
        let side = if d > 1e-9 {
            let (px, py) = (a.x + u.0 * t, a.y + u.1 * t);
            ((px - c.x) / d, (py - c.y) / d)
        } else {
            (-u.1, u.0)
        };
        let angle_of = |p: MapPos| (p.y - c.y).atan2(p.x - c.x);
        let (start, end, mid) = (angle_of(entry), angle_of(exit), side.1.atan2(side.0));
        // Balayage d'`start` à `end` en passant par `mid` : sens et amplitude.
        let wrap = |x: f64| x.rem_euclid(std::f64::consts::TAU);
        let ccw = wrap(end - start);
        let sweep = if wrap(mid - start) <= ccw {
            ccw
        } else {
            ccw - std::f64::consts::TAU
        };
        let steps = ((sweep.abs() / 10f64.to_radians()).ceil() as usize).max(2);
        path.push(entry);
        for k in 1..steps {
            let angle = start + sweep * k as f64 / steps as f64;
            path.push(MapPos::new(c.x + r * angle.cos(), c.y + r * angle.sin()));
        }
        path.push(exit);
    }
    path.push(b);
    path
}

/// Itinéraire par hyperespace entre deux extrémités publiques (systèmes ou mondes isolés).
/// Aucun si l'une des deux est au cœur d'une nébuleuse qui bloque l'hyperespace.
pub fn hyperspace_plan(
    galaxy: &Galaxy,
    settings: &TravelSettings,
    ship: &ShipLogistics,
    from: &Id<Place>,
    to: &Id<Place>,
) -> Option<TravelPlan> {
    let (a, b) = (endpoint(galaxy, from)?, endpoint(galaxy, to)?);
    if a.id() == b.id() {
        return None;
    }
    let obstacles = blocking_nebulae(galaxy, settings);
    if inside_obstacle(a.pos(), &obstacles) || inside_obstacle(b.pos(), &obstacles) {
        return None;
    }
    let (map_length, detour) = detour_length(a.pos(), b.pos(), &obstacles);
    let charted = a.controller().is_some() && b.controller().is_some();
    let factor = if charted {
        1.0
    } else {
        settings.improvised_factor
    };
    let hours = map_length * settings.hyperspace_days_per_unit * 24.0 * factor;
    let (fuel, feasible) = fuel_check(hours, ship);
    Some(TravelPlan {
        mode: TravelMode::Hyperspace,
        hours,
        fuel,
        toll: Credits::ZERO,
        hubs: Vec::new(),
        map_length,
        detour,
        feasible,
    })
}

/// La destination est-elle hors de portée de l'hyperespace parce qu'elle est au cœur d'une
/// nébuleuse bloquante (et non parce qu'elle n'est pas une destination) ?
pub fn hyperspace_blocked(galaxy: &Galaxy, settings: &TravelSettings, to: &Id<Place>) -> bool {
    endpoint(galaxy, to)
        .is_some_and(|end| inside_obstacle(end.pos(), &blocking_nebulae(galaxy, settings)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn length_of(path: &[MapPos]) -> f64 {
        path.windows(2)
            .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
            .sum()
    }

    #[test]
    fn la_corde_est_remplacee_par_l_arc() {
        let a = MapPos::new(0.0, 0.0);
        let b = MapPos::new(100.0, 0.0);
        // Un disque centré sur le trajet : demi-cercle de rayon 10 au lieu de 20 unités.
        let (length, detour) = detour_length(a, b, &[(MapPos::new(50.0, 0.0), 10.0)]);
        assert!(detour);
        assert!(
            (length - (80.0 + std::f64::consts::PI * 10.0)).abs() < 1e-9,
            "{length}"
        );
        // Hors du trajet : ligne droite.
        let (length, detour) = detour_length(a, b, &[(MapPos::new(50.0, 30.0), 10.0)]);
        assert!(!detour);
        assert!((length - 100.0).abs() < 1e-9);
    }

    #[test]
    fn le_trace_contourne_par_l_arc_court() {
        let a = MapPos::new(0.0, 0.0);
        let b = MapPos::new(100.0, 0.0);
        // Le disque déborde un peu au-dessus du trajet (y < 0 sur la carte, vers le nord) : on
        // passe du côté sud, à l'opposé du centre.
        let obstacles = [(MapPos::new(50.0, -4.0), 10.0)];
        let path = hyperspace_path(a, b, &obstacles);
        assert_eq!(path.first(), Some(&a));
        assert_eq!(path.last(), Some(&b));
        let (length, _) = detour_length(a, b, &obstacles);
        // L'arc échantillonné est un peu plus court que l'arc vrai (cordes de 10°).
        let drawn = length_of(&path);
        assert!((drawn - length).abs() < 0.2, "{drawn} contre {length}");
        for p in &path {
            assert!(
                !inside_obstacle(*p, &[(obstacles[0].0, 10.0 - 1e-6)]),
                "{p:?}"
            );
            assert!(p.y >= -1e-9, "le tracé passe au nord : {p:?}");
        }
        // Sans obstacle, deux points.
        assert_eq!(hyperspace_path(a, b, &[]), vec![a, b]);
    }
}
