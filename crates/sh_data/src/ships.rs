//! Classes de vaisseaux, préfixes (§2.4) et formations (§2.5).

use serde::{Deserialize, Serialize};
use sh_core::Id;

use crate::common::{Status, is_false};
use crate::factions::Faction;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ShipClassesFile {
    pub classes: Vec<ShipClass>,
    pub categories: Vec<ShipCategory>,
    pub prefixes: Vec<ShipPrefix>,
}

/// Fourchette canon : minimum et maximum, `None` pour « et plus ».
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub min: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

impl Range {
    pub fn contains(self, value: u32) -> bool {
        value >= self.min && self.max.is_none_or(|max| value <= max)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hyperdrive {
    Yes,
    No,
    /// « non (en général) » : quelques exceptions existent (chasseurs long-courriers).
    RarelyYes,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipClass {
    pub id: Id<ShipClass>,
    pub name: String,
    /// Longueur en mètres.
    pub length_m: Range,
    pub crew: Range,
    pub hyperdrive: Hyperdrive,
    /// Classes normalisées par les conventions des Nations Unies (corvette → cuirassé).
    #[serde(default, skip_serializing_if = "is_false")]
    pub conventioned: bool,
    /// Faux pour le vaisseau capitale : la classe est théorique, aucun n'a jamais existé.
    pub ever_built: bool,
    /// Porte toujours un nom (cuirassés, PV, croiseurs…).
    #[serde(default, skip_serializing_if = "is_false")]
    pub always_named: bool,
    pub role_note: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Catégories hors classes (spécialisés, paramilitaires, civils) : décrites, pas chiffrées.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipCategory {
    pub id: Id<ShipCategory>,
    pub name: String,
    pub note: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipPrefix {
    pub prefix: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meaning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<Id<Faction>>,
    pub entity_note: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FormationsFile {
    pub formations: Vec<Formation>,
    /// Règle de décompte commune (chasseurs et bombardiers exclus).
    pub counting_note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Formation {
    pub id: Id<Formation>,
    pub name: String,
    /// Nombre de vaisseaux (chasseurs et bombardiers non comptés).
    pub ships: Range,
    pub composition_note: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}
