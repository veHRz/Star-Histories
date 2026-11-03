//! Planification des voyages interstellaires (AGENTS.md §2.2 de l'export, §6.3) : logique pure.
//!
//! - **Portail** : chemin le plus court en nombre de passages dans le graphe des hubs
//!   (`portal_links`, P19) ; chaque passage coûte une durée et un péage (P20).
//! - **Hyperespace** : ligne droite sur la carte, contournée autour des nébuleuses qui bloquent
//!   l'hyperespace ; durée proportionnelle à la longueur (P3), allongée hors des routes
//!   cartographiées.
//!
//! Les systèmes secrets ne sont jamais des destinations : aucun plan ne les atteint.

use std::collections::{BTreeMap, VecDeque};

use sh_core::{Credits, Id, MapPos};
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

fn public<'a>(galaxy: &'a Galaxy, id: &Id<Place>) -> Option<&'a StarSystem> {
    galaxy.system(id).filter(|s| !s.secret)
}

fn fuel_check(hours: f64, ship: &ShipLogistics) -> (f64, bool) {
    let fuel = hours / 24.0 * ship.fuel_per_day;
    (fuel, fuel <= ship.fuel_capacity)
}

/// Itinéraire par Portails, s'il existe (les deux systèmes ont un hub relié au réseau).
pub fn portal_plan(
    galaxy: &Galaxy,
    links: &PortalLinks,
    settings: &TravelSettings,
    ship: &ShipLogistics,
    from: &Id<Place>,
    to: &Id<Place>,
) -> Option<TravelPlan> {
    let start = hub_of(galaxy, public(galaxy, from)?)?;
    let goal = hub_of(galaxy, public(galaxy, to)?)?;
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

/// Longueur d'un trajet en ligne droite de `a` à `b`, contourné autour des disques interdits
/// (`(centre, rayon)`) : la corde qui traverse un disque est remplacée par le plus court des
/// deux arcs du cercle de contournement.
pub fn detour_length(a: MapPos, b: MapPos, obstacles: &[(MapPos, f64)]) -> (f64, bool) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx.hypot(dy);
    if length < 1e-9 {
        return (0.0, false);
    }
    let (ux, uy) = (dx / length, dy / length);
    let mut total = length;
    let mut detour = false;
    for &(c, r) in obstacles {
        // Projection du centre sur le segment, distance au segment.
        let t = ((c.x - a.x) * ux + (c.y - a.y) * uy).clamp(0.0, length);
        let (px, py) = (a.x + ux * t, a.y + uy * t);
        let d = (c.x - px).hypot(c.y - py);
        if d >= r {
            continue;
        }
        let half_chord = (r * r - d * d).sqrt();
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

/// Itinéraire par hyperespace entre deux systèmes publics.
pub fn hyperspace_plan(
    galaxy: &Galaxy,
    settings: &TravelSettings,
    ship: &ShipLogistics,
    from: &Id<Place>,
    to: &Id<Place>,
) -> Option<TravelPlan> {
    let (a, b) = (public(galaxy, from)?, public(galaxy, to)?);
    if a.id == b.id {
        return None;
    }
    let obstacles: Vec<(MapPos, f64)> = galaxy
        .phenomena
        .iter()
        .filter(|p| p.blocks_hyperspace)
        .filter_map(|p| Some((p.pos?, p.radius? * settings.nebula_margin)))
        .collect();
    let (map_length, detour) = detour_length(a.center, b.center, &obstacles);
    let charted = a.controller.is_some() && b.controller.is_some();
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
