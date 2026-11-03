//! Puissances, organisations et sous-organisations (§4 de l'export).
//!
//! Transcription manuelle : `assets/data/canon/factions.ron`. Une sous-organisation
//! (division d'Hexa-Corp, FOU et FDU de l'UGL…) pointe vers son organisation par `parent` :
//! les réputations et les grades (M5) s'appuieront sur cet arbre.

use serde::{Deserialize, Serialize};
use sh_core::{Id, Year};

use crate::common::{Place, Status, is_false};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FactionsFile {
    pub factions: Vec<Faction>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Faction {
    pub id: Id<Faction>,
    /// Nom canon, exact (français, accents et articles compris).
    pub name: String,
    /// Nom anglais fourni par l'auteur, s'il en donne un un jour (`docs/QUESTIONS.md` Q11).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    /// Sigle ou nom court canon (« UGL », « FOU »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    pub kind: FactionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id<Faction>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capital: Option<Id<Place>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub founded: Option<Year>,
    /// Fin de l'organisation (✝), si elle a disparu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<Year>,
    /// Préfixe de ses vaisseaux (§2.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ship_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leaders_note: Option<String>,
    /// Résumé canon (interne, jamais affiché).
    pub summary_note: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_notes: Vec<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactionKind {
    /// Grande puissance politique (Empire, UGL, Fédération).
    Power,
    /// Corporation (Hexa-Corp).
    Corporation,
    /// Division ou service d'une organisation.
    Division,
    /// Branche armée d'une puissance (FOU, FDU…).
    MilitaryBranch,
    /// Crime organisé (Syndicat Tosear).
    Syndicate,
    /// Royaume (Royaume Grothul, Royaume de Primaterre).
    Kingdom,
    /// Piraterie (Ligue, pirates indépendants, gangs).
    Pirates,
    /// Ordre religieux ou mystique.
    Order,
    /// Guilde marchande.
    Guild,
    /// Compagnie de mercenaires.
    Mercenaries,
    /// Station-nation (Franchise).
    StationNation,
    /// Puissance ou parti historique.
    Historical,
}
