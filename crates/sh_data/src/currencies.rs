//! Monnaies du monde (§2.8). Les valeurs et les taux ne sont pas canon (§11.1) : ils vivront
//! dans les données d'équilibrage, pas ici.

use serde::{Deserialize, Serialize};
use sh_core::{Id, Year};

use crate::common::Status;
use crate::factions::Faction;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CurrenciesFile {
    pub currencies: Vec<Currency>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Currency {
    pub id: Id<Currency>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Id<Faction>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<Year>,
    /// Fin de la monnaie, si elle a disparu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Year>,
    /// En circulation en 425at.
    pub in_circulation: bool,
    /// Imposée par la loi dans ces juridictions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imposed_in: Vec<Id<Faction>>,
    /// Interdite dans ces juridictions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub banned_in: Vec<Id<Faction>>,
    pub status_note: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}
