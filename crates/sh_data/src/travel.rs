//! Réglages des voyages interstellaires (`gameplay/travel.ron`) et réseau de Portails proposé
//! (`propositions/portal_links.ron`).
//!
//! Le canon fixe le principe (§2.2) : Portails quasi instantanés, en **hubs** et **paires**,
//! régulés par la faction du système ; hyperespace lent (≈ 3 jours entre voisins), bloqué par
//! les nébuleuses. Il ne dit ni quelles liaisons existent (sauf : seul le hub d'Hexis relie les
//! deux blocs), ni les péages, ni la consommation : tout cela est proposé.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sh_core::Id;

use crate::common::{Place, Status};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TravelSettings {
    /// Jours d'hyperespace par unité de carte (P3 : « ≈ 10 unités ≈ 3 jours », déduction non
    /// canon de l'export, utilisée pour l'équilibrage, jamais affichée comme un fait).
    pub hyperspace_days_per_unit: f64,
    /// Allongement d'un trajet hors des routes cartographiées (un des deux systèmes sans maître).
    pub improvised_factor: f64,
    /// Marge de contournement d'une nébuleuse, en fraction de son rayon.
    pub nebula_margin: f64,
    /// Durée d'un passage de Portail (file, contrôles), en heures.
    pub portal_transit_hours: f64,
    /// Péage d'un passage, en crédits.
    pub portal_toll_credits: i64,
    /// Carburant lourd des vaisseaux pilotables, par secteur.
    pub ships: BTreeMap<String, ShipLogistics>,
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
