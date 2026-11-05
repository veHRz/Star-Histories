//! Réglages des voyages interstellaires (`gameplay/travel.ron`), réseau de Portails proposé
//! (`propositions/portal_links.ron`) et extrémités possibles d'un voyage ([`endpoint`]).
//!
//! Le canon fixe le principe (§2.2) : Portails quasi instantanés, en **hubs** et **paires**,
//! régulés par la faction du système ; hyperespace lent (≈ 3 jours entre voisins), bloqué par
//! les nébuleuses. Il ne dit ni quelles liaisons existent (sauf : seul le hub d'Hexis relie les
//! deux blocs), ni les péages, ni la consommation : tout cela est proposé.
//!
//! Depuis M9 (décision de l'auteur, Q18), les **mondes isolés** (astres hors système, §3.4) sont
//! aussi des destinations : par l'hyperespace, arrivée en orbite seulement (aucun n'a de
//! spatioport). Celui qui porte un hub de Portails canon relié au réseau se rejoint aussi par
//! Portail (Karza, « porte d'entrée de l'Empire », §3.4 et §3.7). Un astre secret n'est jamais
//! une destination.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sh_core::{GridCell, Id, MapPos};

use crate::common::{Place, Status};
use crate::factions::Faction;
use crate::galaxy::{Body, BodyKind, Galaxy, StarSystem};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TravelSettings {
    /// Jours d'hyperespace par unité de carte (P3 : « ≈ 10 unités ≈ 3 jours », déduction non
    /// canon de l'export, utilisée pour l'équilibrage, jamais affichée comme un fait).
    pub hyperspace_days_per_unit: f64,
    /// Allongement d'un trajet hors des routes cartographiées (un des deux lieux sans maître).
    pub improvised_factor: f64,
    /// Marge de contournement d'une nébuleuse, en fraction de son rayon.
    pub nebula_margin: f64,
    /// Durée d'un passage de Portail (file, contrôles), en heures.
    pub portal_transit_hours: f64,
    /// Péage d'un passage, en crédits.
    pub portal_toll_credits: i64,
    /// Arrivée en orbite d'un monde isolé : distance au centre de l'astre, en rayons de l'astre.
    #[serde(default = "default_isolated_arrival_radii")]
    pub isolated_arrival_radii: f64,
    /// Carburant lourd des vaisseaux pilotables, par secteur.
    pub ships: BTreeMap<String, ShipLogistics>,
}

fn default_isolated_arrival_radii() -> f64 {
    3.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShipLogistics {
    /// Réserve de carburant lourd (unités).
    pub fuel_capacity: f64,
    /// Consommation de bord par jour (l'hyperpropulsion, elle, est nucléaire : §2.3).
    pub fuel_per_day: f64,
}

/// Liaisons entre hubs de Portails (identifiants des astres qui portent les hubs).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortalLinks {
    #[serde(default)]
    pub status: Status,
    pub links: Vec<(Id<Place>, Id<Place>)>,
}

/// Une extrémité de voyage : là où le vaisseau du joueur peut se trouver, d'où il peut partir
/// et où il peut aller.
#[derive(Clone, Copy, Debug)]
pub enum Endpoint<'a> {
    /// Un système stellaire public.
    System(&'a StarSystem),
    /// Un monde isolé public (astre hors système, §3.4) : planète, planète errante ou lune
    /// d'un monde isolé.
    Isolated(&'a Body),
}

impl<'a> Endpoint<'a> {
    pub fn id(&self) -> &'a Id<Place> {
        match self {
            Endpoint::System(s) => &s.id,
            Endpoint::Isolated(b) => &b.id,
        }
    }

    pub fn name(&self) -> &'a str {
        match self {
            Endpoint::System(s) => &s.name,
            Endpoint::Isolated(b) => &b.name,
        }
    }

    /// Position sur la carte (centre du système, ou position de l'astre).
    pub fn pos(&self) -> MapPos {
        match self {
            Endpoint::System(s) => s.center,
            // `endpoint` n'accepte que les astres placés : la valeur de secours ne sert pas.
            Endpoint::Isolated(b) => b.pos.unwrap_or(MapPos::new(0.0, 0.0)),
        }
    }

    pub fn cell(&self) -> Option<GridCell> {
        match self {
            Endpoint::System(s) => Some(s.cell),
            Endpoint::Isolated(b) => b.cell,
        }
    }

    pub fn controller(&self) -> Option<&'a Id<Faction>> {
        match self {
            Endpoint::System(s) => s.controller.as_ref(),
            Endpoint::Isolated(b) => b.controller.as_ref(),
        }
    }

    pub fn is_isolated(&self) -> bool {
        matches!(self, Endpoint::Isolated(_))
    }
}

/// L'extrémité de voyage `id`, si c'en est une : un système public, ou un monde isolé public
/// placé sur la carte (et dont la planète, pour une lune, est publique). Jamais un secret, ni un
/// astre d'un système (on va au système), ni une station ou un phénomène.
pub fn endpoint<'a>(galaxy: &'a Galaxy, id: &Id<Place>) -> Option<Endpoint<'a>> {
    if let Some(system) = galaxy.system(id) {
        return (!system.secret).then_some(Endpoint::System(system));
    }
    let body = galaxy.body(id)?;
    let isolated = !body.secret
        && body.system.is_none()
        && !matches!(body.kind, BodyKind::Star | BodyKind::Belt)
        && body.pos.is_some()
        && ancestors_public(galaxy, body);
    isolated.then_some(Endpoint::Isolated(body))
}

/// Les parents d'un astre sont tous publics (une lune d'un monde secret reste secrète).
fn ancestors_public(galaxy: &Galaxy, body: &Body) -> bool {
    let mut at = body;
    // Borne : une chaîne de parents plus longue que la liste des astres serait une boucle.
    for _ in 0..galaxy.bodies.len() {
        let Some(parent) = at.parent.as_ref() else {
            return true;
        };
        match galaxy.body(parent) {
            Some(p) if !p.secret => at = p,
            _ => return false,
        }
    }
    false
}

/// Le monde au centre de la scène d'un monde isolé : l'astre lui-même, ou la planète dont il est
/// la lune (en remontant les parents publics et hors système).
pub fn isolated_root<'a>(galaxy: &'a Galaxy, body: &'a Body) -> &'a Body {
    let mut at = body;
    for _ in 0..galaxy.bodies.len() {
        match at
            .parent
            .as_ref()
            .and_then(|p| galaxy.body(p))
            .filter(|p| !p.secret && p.system.is_none() && p.kind != BodyKind::Star)
        {
            Some(parent) => at = parent,
            None => break,
        }
    }
    at
}
