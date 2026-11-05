//! Mise en scène d'un système en vue Globale : ce que la disposition générique
//! ([`crate::layout`]) ne sait pas placer, parce que le canon ne le place pas.
//!
//! Hub de Portails, chantiers orbitaux, trafic ambiant, point d'arrivée, nom affiché d'une étoile
//! sans nom : tout cela est **proposé** (`status: Proposition`), un fichier par système dans
//! `assets/data/propositions/systems/`.
//!
//! **Spatioport par défaut** (M9, P7) : un port d'amarrage qui ne déclare pas son secteur
//! accueille le vaisseau dans [`DEFAULT_SPACEPORT`] (hangar et salles). Plusieurs lieux peuvent
//! donc partager le même secteur : le port lui-même (son identifiant, son système) dit où l'on
//! est ([`find_dock`]), son nom affiché à quai vient de `place_key`.

use std::collections::BTreeMap;

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
    /// Modèle (sous `assets/`) des stations canon du système, par identifiant.
    #[serde(default)]
    pub station_models: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traffic: Option<Traffic>,
    /// Ports d'amarrage : la balise d'approche d'un secteur qui accueille le vaisseau.
    #[serde(default)]
    pub docks: Vec<DockPort>,
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
    /// Modèle, sous `assets/`.
    pub model: String,
}

/// Le secteur d'un port d'amarrage qui ne déclare pas le sien (P7).
pub const DEFAULT_SPACEPORT: &str = "spaceport";

fn default_spaceport() -> String {
    DEFAULT_SPACEPORT.to_owned()
}

/// Balise d'approche d'un port : à portée, on demande l'amarrage et le vaisseau rejoint le
/// secteur `sector` (son point d'attache « @ship »).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DockPort {
    pub id: String,
    /// Le secteur qui accueille le vaisseau ; absent : le spatioport par défaut.
    #[serde(default = "default_spaceport")]
    pub sector: String,
    /// Clé de langue du nom de la balise d'approche (vue Globale).
    pub name_key: String,
    /// Clé de langue du nom du lieu, affiché à quai (« Spatioport d'Hexis ») ; absente : le nom
    /// du secteur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_key: Option<String>,
    pub around: Id<Place>,
    pub altitude_radii: f64,
    pub angle_deg: f64,
    /// Portée de la demande d'amarrage (m).
    pub range: f64,
}

/// Le port où un quartier accueille le vaisseau : celui du système `system` qui a ce secteur
/// s'il y en a un, sinon le premier de tous (un départ direct dans le secteur, sans système
/// connu). Un spatioport par défaut, partagé, est ainsi rattaché au lieu où l'on s'est amarré.
pub fn find_dock<'a>(
    scenes: &'a [SystemScene],
    district: &str,
    system: Option<&str>,
) -> Option<(&'a SystemScene, &'a DockPort)> {
    let docks = || {
        scenes.iter().flat_map(|scene| {
            scene
                .docks
                .iter()
                .filter(move |d| d.sector == district)
                .map(move |d| (scene, d))
        })
    };
    docks()
        .find(|(scene, _)| Some(scene.system.as_str()) == system)
        .or_else(|| docks().next())
}

/// Trafic ambiant : des figurants qui font la navette entre les stations et les structures du
/// système (sans simulation en M2).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Traffic {
    pub ships: u32,
    pub model: String,
    /// Vitesse de croisière des figurants (m/s).
    pub speed: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureKind {
    /// Hub de Portails (anneaux et station de contrôle).
    PortalHub,
    /// Chantiers orbitaux (cales, grues, coques en construction).
    Shipyard,
}

/// Point d'arrivée : à `distance` mètres d'un objet (station, structure ou astre), à
/// `azimuth_deg` de la direction de l'étoile dans le plan (0 : l'étoile dans le dos, 90 : de
/// côté) et à `elevation_deg` au-dessus du plan, en regardant l'objet.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Arrival {
    pub near: String,
    pub distance: f64,
    #[serde(default)]
    pub azimuth_deg: f64,
    #[serde(default)]
    pub elevation_deg: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(system: &str, docks: &str) -> SystemScene {
        let text = format!(
            "(system: \"{system}\", docks: [{docks}],
              arrival: (near: \"x\", distance: 1.0))"
        );
        crate::ron_options()
            .from_str(&text)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn dock(id: &str, sector: Option<&str>) -> String {
        let sector = sector.map_or(String::new(), |s| format!("sector: \"{s}\","));
        format!(
            "(id: \"{id}\", {sector} name_key: \"k\", around: \"a\", altitude_radii: 1.0,
              angle_deg: 0.0, range: 1.0)"
        )
    }

    #[test]
    fn un_port_sans_secteur_prend_le_spatioport_par_defaut() {
        let scenes = vec![
            scene("sys_a", &dock("port_a", None)),
            scene(
                "sys_b",
                &format!("{}, {}", dock("port_b", None), dock("quai_b", Some("quai"))),
            ),
        ];
        assert_eq!(scenes[0].docks[0].sector, DEFAULT_SPACEPORT);
        assert_eq!(scenes[1].docks[1].sector, "quai");
        assert!(scenes[0].docks[0].place_key.is_none());
        let id =
            |district, system| find_dock(&scenes, district, system).map(|(_, d)| d.id.as_str());
        // Le spatioport par défaut, partagé : le port du système où l'on est.
        assert_eq!(id(DEFAULT_SPACEPORT, Some("sys_b")), Some("port_b"));
        assert_eq!(id(DEFAULT_SPACEPORT, Some("sys_a")), Some("port_a"));
        // Système inconnu ou absent : le premier port de ce secteur.
        assert_eq!(id(DEFAULT_SPACEPORT, None), Some("port_a"));
        assert_eq!(id(DEFAULT_SPACEPORT, Some("sys_z")), Some("port_a"));
        // Un lieu qui a son propre spatioport.
        assert_eq!(id("quai", Some("sys_a")), Some("quai_b"));
        assert_eq!(id("ailleurs", None), None);
    }
}
