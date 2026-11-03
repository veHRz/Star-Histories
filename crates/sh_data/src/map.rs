//! Ce que montre la carte galactique (vue Galaxie) : uniquement des informations **publiques**.
//!
//! La carte, ses étiquettes et ses infobulles ne lisent rien d'autre que [`public_map`] : un
//! système, astre, station, phénomène ou zone secret n'y entre pas, et aucune note secrète non
//! plus (règle d'or n°3, AGENTS.md). Les infobulles sont faites de champs structurés (case,
//! faction, population, hub), traduits par la langue de l'interface ; la prose du canon n'y
//! figure pas.

use sh_core::{GridCell, Id, MapPos};

use crate::common::Place;
use crate::factions::Faction;
use crate::galaxy::{BodyKind, Galaxy};

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
        });
    }
    for b in galaxy
        .bodies
        .iter()
        .filter(|b| !b.secret && b.system.is_none() && b.kind != BodyKind::Star)
    {
        let Some(pos) = b.pos else { continue };
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
