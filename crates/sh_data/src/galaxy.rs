//! La galaxie d'Astréa : zones, systèmes, astres, stations, phénomènes, hubs et flottes.
//!
//! Ce fichier de données (`assets/data/canon/galaxy.ron`) est **généré** par `tools/sh_import`
//! à partir de `_context/star-histories-carte.json` : on ne l'édite jamais à la main.
//!
//! Les champs `*_note` sont les textes canon d'origine (en français). Ils servent à la
//! conception et à la simulation, **jamais à l'affichage** : les textes visibles du joueur
//! passent par les fichiers de langue. Les passages marqués 🔒 dans le canon sont isolés dans
//! `secret_notes` ; la validation vérifie qu'aucun autre champ n'en contient.

use serde::{Deserialize, Serialize};
use sh_core::{GridCell, Id, MapPos};

use crate::common::{Location, Place, Population, Status, is_false};
use crate::factions::Faction;

/// Toutes les données géographiques canon.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Galaxy {
    /// Provenance du fichier (source et date de l'export).
    pub origin: String,
    pub frame: MapFrame,
    pub zones: Vec<Zone>,
    pub systems: Vec<StarSystem>,
    pub bodies: Vec<Body>,
    pub stations: Vec<Station>,
    pub phenomena: Vec<Phenomenon>,
    pub portal_hubs: Vec<PortalHub>,
    pub fleets: Vec<Fleet>,
}

impl Galaxy {
    pub fn system(&self, id: &Id<Place>) -> Option<&StarSystem> {
        self.systems.iter().find(|s| &s.id == id)
    }

    pub fn body(&self, id: &Id<Place>) -> Option<&Body> {
        self.bodies.iter().find(|b| &b.id == id)
    }

    pub fn station(&self, id: &Id<Place>) -> Option<&Station> {
        self.stations.iter().find(|s| &s.id == id)
    }

    /// Nom canon d'un lieu, quel que soit son genre.
    pub fn place_name(&self, id: &Id<Place>) -> Option<&str> {
        self.system(id)
            .map(|s| s.name.as_str())
            .or_else(|| self.body(id).map(|b| b.name.as_str()))
            .or_else(|| self.station(id).map(|s| s.name.as_str()))
            .or_else(|| {
                self.phenomena
                    .iter()
                    .find(|p| &p.id == id)
                    .map(|p| p.name.as_str())
            })
    }

    /// Position sur la carte d'un lieu, si le canon en donne une.
    pub fn place_pos(&self, id: &Id<Place>) -> Option<MapPos> {
        self.system(id)
            .map(|s| s.center)
            .or_else(|| self.body(id).and_then(|b| b.pos))
            .or_else(|| self.station(id).map(|s| s.pos))
            .or_else(|| {
                self.phenomena
                    .iter()
                    .find(|p| &p.id == id)
                    .and_then(|p| p.pos)
            })
    }
}

/// Le repère de la carte canonique (§3.1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MapFrame {
    pub width: f64,
    pub height: f64,
    /// Centre de l'ellipse de la galaxie (et position du noyau).
    pub center: (f64, f64),
    pub radius_x: f64,
    pub radius_y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub id: Id<Zone>,
    pub name: String,
    /// Faction qui contrôle la zone, si une seule la tient.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller: Option<Id<Faction>>,
    pub control_note: String,
    pub cells_note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_note: Option<String>,
    pub description_note: String,
    /// Contour sur la carte (polygone fermé implicitement). Vide quand la carte n'en trace pas
    /// (les marches sont volontairement vides).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outline: Vec<MapPos>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline_note: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_notes: Vec<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StarSystem {
    pub id: Id<Place>,
    pub name: String,
    pub cell: GridCell,
    pub center: MapPos,
    /// Étoiles du système (astres de genre `Star`). Vide si l'étoile n'est pas nommée (❓).
    pub stars: Vec<Id<Place>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub star_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller: Option<Id<Faction>>,
    pub affiliation_note: String,
    pub note: String,
    /// Corps du système, dans l'ordre de l'export (pas un ordre orbital).
    pub bodies: Vec<Id<Place>>,
    pub stations: Vec<Id<Place>>,
    pub population_total: u64,
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
pub enum BodyKind {
    Star,
    Planet,
    Moon,
    Belt,
    /// Planète errante, sans étoile.
    RoguePlanet,
}

/// Rang orbital connu d'un astre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrbitOrder {
    /// 1 = le plus proche de son parent.
    Rank(u8),
    /// Le plus extérieur du système, sans rang chiffré.
    Outermost,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Body {
    pub id: Id<Place>,
    pub name: String,
    pub kind: BodyKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<Id<Place>>,
    /// Corps autour duquel il orbite (étoile ou planète).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id<Place>>,
    /// Rang orbital, seulement quand le canon le précise (lune n° 2, 1re planète, la plus
    /// extérieure…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbit_order: Option<OrbitOrder>,
    /// Case canon. `None` quand le canon ne place pas l'astre (Jask).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<GridCell>,
    /// Case où la carte dessine l'astre, quand elle diffère de la case canon (§12, point 7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawn_in: Option<GridCell>,
    /// Position sur la carte. Dans un système, la disposition est schématique (§0, point 7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<MapPos>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos_note: Option<String>,
    /// Détruit ou mort (✝).
    #[serde(default, skip_serializing_if = "is_false")]
    pub destroyed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population: Option<Population>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller: Option<Id<Faction>>,
    pub kind_note: String,
    pub affiliation_note: String,
    pub state_note: String,
    pub role_note: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_notes: Vec<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Station {
    pub id: Id<Place>,
    pub name: String,
    pub kind_note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<Id<Place>>,
    pub cell: GridCell,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawn_in: Option<GridCell>,
    pub pos: MapPos,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_note: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub under_construction: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller: Option<Id<Faction>>,
    pub affiliation_note: String,
    pub state_note: String,
    pub population: Population,
    pub role_note: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_notes: Vec<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Phenomenon {
    pub id: Id<Place>,
    pub name: String,
    pub kind_note: String,
    /// Case principale, quand le canon en donne une seule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<GridCell>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<MapPos>,
    /// Rayon approximatif sur la carte, en unités de carte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    /// Système auquel le phénomène se rattache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<Id<Place>>,
    /// Bloque l'hyperespace (§2.2).
    #[serde(default, skip_serializing_if = "is_false")]
    pub blocks_hyperspace: bool,
    pub role_note: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_notes: Vec<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Un grand hub de Portails (§3.7).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortalHub {
    /// Astre qui porte le hub.
    pub body: Id<Place>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller: Option<Id<Faction>>,
    pub camp_note: String,
    pub role_note: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Une flotte, avec sa position en 425at (§6.1).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fleet {
    pub id: Id<Fleet>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<Id<Faction>>,
    pub entity_note: String,
    pub base: Location,
    pub position: Location,
    pub state_note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flagship_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commander_note: Option<String>,
    pub composition_note: String,
    pub role_note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motto: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_notes: Vec<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}
