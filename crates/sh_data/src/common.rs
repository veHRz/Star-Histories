//! Types communs à tous les schémas de données.

use serde::{Deserialize, Serialize};
use sh_core::Id;

/// Marqueur des identifiants de lieux : systèmes, astres, stations et phénomènes partagent un
/// même espace de noms, car une flotte ou un trajet peut viser n'importe lequel d'entre eux.
pub enum Place {}

/// Statut d'une donnée vis-à-vis du canon (AGENTS.md §1, règle 2).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// Fait canon : l'entrée doit citer sa section source (`source: "§3.3"`).
    #[default]
    Canon,
    /// Invention du jeu, consignée dans `docs/propositions.md` sous ce numéro (« P12 »).
    Proposition(String),
    /// Le canon lui-même marque l'élément comme une proposition non validée de l'auteur
    /// (ex. le préfixe UNS, le nom « Jask »). On le garde, mais sans l'affirmer.
    AuthorDraft,
}

impl Status {
    pub fn is_canon(&self) -> bool {
        matches!(self, Status::Canon)
    }
}

/// Population d'un lieu, telle que le canon la donne (ordres de grandeur, §2.9).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Population {
    /// Civile (population carcérale dans un bagne), en texte canon (« 175 Md »).
    pub civil: String,
    /// Militaire et forces de l'ordre.
    pub military: String,
    /// Ouvriers de chantiers.
    pub shipyard: String,
    /// Total approximatif (calcul de l'export, pas un recensement).
    pub total: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Où se trouve une chose : un lieu connu des données, ou une description libre quand le canon
/// ne donne pas de lieu précis (« chaque système impérial », « mobile »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Location {
    Place(Id<Place>),
    Unplaced(String),
    Unknown,
}

impl Location {
    pub fn place(&self) -> Option<&Id<Place>> {
        match self {
            Location::Place(id) => Some(id),
            _ => None,
        }
    }
}

pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}
