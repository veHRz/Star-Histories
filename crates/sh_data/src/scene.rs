//! Mise en scène d'un système en vue Globale : ce que la disposition générique
//! ([`crate::layout`]) ne sait pas placer, parce que le canon ne le place pas.
//!
//! Hub de Portails, chantiers orbitaux, trafic ambiant, point d'arrivée, nom affiché d'une étoile
//! sans nom : tout cela est **proposé** (`status: Proposition`), un fichier par système dans
//! `assets/data/propositions/systems/`.

use serde::{Deserialize, Serialize};
use sh_core::Id;

use crate::common::{Place, Status};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemScene {
    pub system: Id<Place>,
    #[serde(default)]
    pub status: Status,
    /// Nom affiché de l'étoile quand le canon ne la nomme pas (« Étoile d'Hexis », P11).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub star_name: Option<String>,
    #[serde(default)]
    pub structures: Vec<Structure>,
    /// Vaisseaux du trafic ambiant (figurants, sans simulation en M2).
    #[serde(default)]
    pub traffic: u32,
    /// Où apparaît le joueur en arrivant dans le système.
    pub arrival: Arrival,
}

/// Une installation orbitale sans fiche canon propre.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Structure {
    pub id: String,
    pub kind: StructureKind,
    /// Clé de langue de son nom affiché.
    pub name_key: String,
    /// L'astre autour duquel elle orbite.
    pub around: Id<Place>,
    /// Altitude, en rayons de l'astre.
    pub altitude_radii: f64,
    /// Direction autour de l'astre dans le plan du système, en degrés (0 = est, 90 = sud).
    pub angle_deg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureKind {
    /// Hub de Portails (anneaux et station de contrôle).
    PortalHub,
    /// Chantiers orbitaux (cales, grues, coques en construction).
    Shipyard,
}

/// Point d'arrivée : à `distance` mètres d'un objet (station, structure ou astre), du côté de
/// l'étoile, à `elevation_deg` au-dessus du plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Arrival {
    pub near: String,
    pub distance: f64,
    #[serde(default)]
    pub elevation_deg: f64,
}
