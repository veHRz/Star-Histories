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
//!
//! **Spatioport vu de l'espace** (M10, P31) : un port sans modèle propre a celui de la station
//! orbitale par défaut ([`DEFAULT_SPACEPORT_MODEL`]) et son niveau de détail éloigné
//! ([`lod_model`]).
//!
//! **Points de livraison** (M10, P32) : toute structure (hub de Portails, chantiers, dépôt de
//! fret) reçoit la cargaison des contrats. Un système qui a un hub de Portails canon
//! (`galaxy.portal_hubs`) sans hub mis en scène en reçoit un **générique** ([`generic_hub`]),
//! et l'on y arrive ([`arrival_of`]). [`system_structures`] rassemble les structures d'un
//! système, [`has_delivery_point`] dit si un contrat peut y finir. [`nav_points`] liste tout ce
//! qu'on peut viser dans un système (astres, stations, structures, ports), sans `World` : la vue
//! Globale (autopilote) et la carte s'en servent.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sh_core::{Id, SpacePos};

use crate::GameData;
use crate::common::{Place, Status};
use crate::galaxy::{Galaxy, StarSystem};
use crate::lang::Lang;
use crate::layout::{Placed, PlacedKind, isolated_layout, system_layout};
use crate::travel::{Endpoint, endpoint};

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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

/// Le modèle, vu de l'espace, d'un port d'amarrage qui ne déclare pas le sien (P31) : la station
/// orbitale à hangars (≈ 600 m, bouche du hangar vers −Z en jeu).
pub const DEFAULT_SPACEPORT_MODEL: &str = "models/space/spaceport_station/spaceport_station.gltf";

/// Le modèle d'un hub de Portails générique (P32).
pub const GENERIC_HUB_MODEL: &str = "models/space/portal_hub.gltf";
/// Clé de langue du nom d'un hub de Portails générique ; `{place}` : l'astre qui le porte.
pub const GENERIC_HUB_NAME_KEY: &str = "structure.generic_portal_hub";
/// Altitude d'un hub générique, en rayons de son astre (comme ceux d'Hexis et de Cygnara).
const GENERIC_HUB_ALTITUDE_RADII: f64 = 1.5;
/// Arrivée près d'un hub générique : distance (m), azimut et élévation (comme à Cygnara).
const GENERIC_ARRIVAL: (f64, f64, f64) = (25_000.0, -50.0, 12.0);

/// Où l'autopilote s'arrête (m, depuis le centre de la cible). Un astre : en rayons, hors de
/// sa surface (une étoile, plus loin : sa lumière et sa chaleur). Une station canon, une
/// structure, un port : quelques kilomètres, la taille de leurs modèles (≈ 0,6 à 3 km).
const STOP_BODY_RADII: f64 = 1.6;
const STOP_STAR_RADII: f64 = 3.0;
const STOP_STATION: f64 = 5_000.0;
const STOP_STRUCTURE: f64 = 8_000.0;
const STOP_DOCK: f64 = 3_000.0;

/// Le niveau de détail éloigné d'un modèle : `<nom>_lod.gltf` à côté de `<nom>.gltf`
/// (convention de l'atelier : `spaceport_station_lod.gltf`).
pub fn lod_model(model: &str) -> Option<String> {
    model
        .strip_suffix(".gltf")
        .map(|stem| format!("{stem}_lod.gltf"))
}

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
    /// Modèle de la station vue de l'espace (sous `assets/`) ; absent : la station orbitale par
    /// défaut ([`DEFAULT_SPACEPORT_MODEL`], P31).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl DockPort {
    /// Le modèle de la station du port, vue de l'espace.
    pub fn model_path(&self) -> &str {
        self.model.as_deref().unwrap_or(DEFAULT_SPACEPORT_MODEL)
    }
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
    /// Dépôt de fret orbital (M10, P32) : où les remorqueurs d'Hexa-Corp prennent la cargaison
    /// dans un système qui n'a ni hub de Portails ni chantiers.
    Depot,
}

/// Point d'arrivée : à `distance` mètres d'un objet (station, structure ou astre), à
/// `azimuth_deg` de la direction de l'étoile dans le plan (0 : l'étoile dans le dos, 90 : de
/// côté) et à `elevation_deg` au-dessus du plan, en regardant l'objet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arrival {
    pub near: String,
    pub distance: f64,
    #[serde(default)]
    pub azimuth_deg: f64,
    #[serde(default)]
    pub elevation_deg: f64,
}

/// Le hub de Portails générique d'un système (P32) : le premier hub canon porté par un astre
/// public du système, quand la mise en scène n'en place aucun. Direction tirée du nom de
/// l'astre : stable d'une visite à l'autre, sans rien affirmer de plus.
pub fn generic_hub(
    galaxy: &Galaxy,
    scene: Option<&SystemScene>,
    system: &Id<Place>,
) -> Option<Structure> {
    let staged = scene.is_some_and(|s| {
        s.structures
            .iter()
            .any(|st| st.kind == StructureKind::PortalHub)
    });
    if staged {
        return None;
    }
    let hub = galaxy.portal_hubs.iter().find(|h| {
        galaxy
            .body(&h.body)
            .is_some_and(|b| !b.secret && !b.destroyed && b.system.as_ref() == Some(system))
    })?;
    let angle = (sh_core::rng::fnv1a(hub.body.as_str().as_bytes()) % 360) as f64;
    Some(Structure {
        id: format!("{}_portal_hub", hub.body),
        kind: StructureKind::PortalHub,
        name_key: GENERIC_HUB_NAME_KEY.to_owned(),
        around: hub.body.clone(),
        altitude_radii: GENERIC_HUB_ALTITUDE_RADII,
        angle_deg: angle,
        model: GENERIC_HUB_MODEL.to_owned(),
    })
}

/// Les structures d'un système : celles de sa mise en scène, puis son hub générique s'il en a
/// un. Toutes sont des points de livraison.
pub fn system_structures(
    galaxy: &Galaxy,
    scene: Option<&SystemScene>,
    system: &Id<Place>,
) -> Vec<Structure> {
    let mut out: Vec<Structure> = scene.map(|s| s.structures.clone()).unwrap_or_default();
    out.extend(generic_hub(galaxy, scene, system));
    out
}

/// Un contrat peut-il finir dans ce système (une structure où livrer) ?
pub fn has_delivery_point(data: &GameData, system: &Id<Place>) -> bool {
    data.galaxy.system(system).is_some_and(|s| !s.secret)
        && !system_structures(&data.galaxy, data.scene(system), system).is_empty()
}

/// L'arrivée habituelle dans un système : celle de sa mise en scène, sinon près de son hub
/// générique (on sort d'un saut là où l'on vient livrer).
pub fn arrival_of(
    galaxy: &Galaxy,
    scene: Option<&SystemScene>,
    system: &Id<Place>,
) -> Option<Arrival> {
    if let Some(scene) = scene {
        return Some(scene.arrival.clone());
    }
    let (distance, azimuth_deg, elevation_deg) = GENERIC_ARRIVAL;
    generic_hub(galaxy, scene, system).map(|hub| Arrival {
        near: hub.id,
        distance,
        azimuth_deg,
        elevation_deg,
    })
}

/// Le nom affiché d'une structure ; un hub générique porte celui de son astre.
pub fn structure_name(lang: &Lang, galaxy: &Galaxy, st: &Structure) -> String {
    if st.name_key == GENERIC_HUB_NAME_KEY {
        let place = galaxy
            .body(&st.around)
            .map_or_else(|| st.around.to_string(), |b| b.name.clone());
        lang.format(&st.name_key, &[("place", &place)])
    } else {
        lang.get(&st.name_key)
    }
}

/// Un point à `altitude_radii` rayons au-dessus de l'astre `around`, dans la direction
/// `angle_deg` du plan du système (structures et ports).
pub fn place_around(
    placed: &[Placed],
    around: &Id<Place>,
    altitude_radii: f64,
    angle_deg: f64,
) -> Option<SpacePos> {
    let body = placed.iter().find(|p| p.id.as_ref() == Some(around))?;
    let radius = body.radius * (1.0 + altitude_radii);
    let a = angle_deg.to_radians();
    Some(body.pos + SpacePos::new(radius * a.cos(), 0.0, radius * a.sin()))
}

/// Ce qu'on peut viser dans un système.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavKind {
    /// Un astre (étoile, planète, géante, lune) ou une station canon.
    Body(PlacedKind),
    /// Une structure : un point de livraison.
    Structure(StructureKind),
    /// La balise d'approche d'un port d'amarrage.
    Dock,
}

/// Une cible de navigation (autopilote, carte).
#[derive(Clone, Debug, PartialEq)]
pub struct NavPoint {
    /// Identifiant de l'astre, de la station, de la structure ou du port.
    pub id: String,
    pub kind: NavKind,
    /// Nom affiché.
    pub name: String,
    /// Position absolue dans le système (m).
    pub pos: SpacePos,
    /// Rayon de l'astre (m) ; 0 pour une station, une structure, un port.
    pub radius: f64,
    /// Distance au centre (m) où l'autopilote s'arrête : hors de la surface d'un astre, à
    /// portée de livraison d'une structure, à portée d'amarrage d'un port.
    pub stop: f64,
}

impl NavPoint {
    /// Un point de livraison des contrats ?
    pub fn is_delivery(&self) -> bool {
        matches!(self.kind, NavKind::Structure(_))
    }
}

/// Où s'arrêter devant une cible : proportionnel à sa taille, jamais au-delà de la moitié de la
/// portée qui compte (livraison pour une structure, amarrage pour un port).
pub fn stop_distance(kind: NavKind, radius: f64, delivery_range: f64, dock_range: f64) -> f64 {
    match kind {
        NavKind::Body(PlacedKind::Star) => radius * STOP_STAR_RADII,
        NavKind::Body(PlacedKind::Station) => STOP_STATION,
        NavKind::Body(_) => radius * STOP_BODY_RADII,
        NavKind::Structure(_) => STOP_STRUCTURE.min(0.5 * delivery_range),
        NavKind::Dock => STOP_DOCK.min(0.5 * dock_range),
    }
}

/// Les cibles d'un lieu de la vue Globale (un système, ou un monde isolé) : astres et stations
/// publics nommés, structures (avec le hub générique), ports. Ordre stable : celui de la
/// disposition, puis les structures, puis les ports.
pub fn nav_points(data: &GameData, lang: &Lang, place: &Id<Place>) -> Vec<NavPoint> {
    let scale = &data.space.scale;
    match endpoint(&data.galaxy, place) {
        Some(Endpoint::System(system)) => {
            let placed = system_layout(&data.galaxy, system, scale);
            nav_points_in(data, lang, Some(system), &placed)
        }
        Some(Endpoint::Isolated(body)) => {
            let placed = isolated_layout(&data.galaxy, body, scale);
            nav_points_in(data, lang, None, &placed)
        }
        None => Vec::new(),
    }
}

/// [`nav_points`] sur une disposition déjà calculée (la vue Globale a la sienne).
pub fn nav_points_in(
    data: &GameData,
    lang: &Lang,
    system: Option<&StarSystem>,
    placed: &[Placed],
) -> Vec<NavPoint> {
    let g = &data.galaxy;
    let delivery = data.career.contracts.delivery_range;
    let mut out = Vec::new();
    for p in placed {
        let Some(id) = &p.id else {
            continue;
        };
        let name = match p.kind {
            PlacedKind::Station => g.station(id).map(|s| s.name.clone()),
            _ => g.body(id).map(|b| b.name.clone()),
        };
        let Some(name) = name else {
            continue;
        };
        let kind = NavKind::Body(p.kind);
        let radius = if p.kind == PlacedKind::Station {
            0.0
        } else {
            p.radius
        };
        out.push(NavPoint {
            id: id.to_string(),
            kind,
            name,
            pos: p.pos,
            radius,
            stop: stop_distance(kind, radius, delivery, 0.0),
        });
    }
    let Some(system) = system else {
        return out;
    };
    let scene = data.scene(&system.id);
    for st in system_structures(g, scene, &system.id) {
        let Some(pos) = place_around(placed, &st.around, st.altitude_radii, st.angle_deg) else {
            continue;
        };
        let kind = NavKind::Structure(st.kind);
        out.push(NavPoint {
            name: structure_name(lang, g, &st),
            id: st.id,
            kind,
            pos,
            radius: 0.0,
            stop: stop_distance(kind, 0.0, delivery, 0.0),
        });
    }
    for dock in scene.map(|s| s.docks.as_slice()).unwrap_or_default() {
        let Some(pos) = place_around(placed, &dock.around, dock.altitude_radii, dock.angle_deg)
        else {
            continue;
        };
        out.push(NavPoint {
            id: dock.id.clone(),
            kind: NavKind::Dock,
            name: lang.get(&dock.name_key),
            pos,
            radius: 0.0,
            stop: stop_distance(NavKind::Dock, 0.0, delivery, dock.range),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::Path;

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

    fn game_data() -> GameData {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
        GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"))
    }

    fn id(s: &str) -> Id<Place> {
        Id::new(s).unwrap_or_else(|e| panic!("{e}"))
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

    #[test]
    fn un_port_sans_modele_prend_la_station_par_defaut() {
        let scenes = [scene("sys_a", &dock("port_a", None))];
        assert_eq!(scenes[0].docks[0].model_path(), DEFAULT_SPACEPORT_MODEL);
        assert_eq!(
            lod_model(DEFAULT_SPACEPORT_MODEL).as_deref(),
            Some("models/space/spaceport_station/spaceport_station_lod.gltf")
        );
        assert_eq!(lod_model("models/space/x.glb"), None);
    }

    #[test]
    fn un_systeme_sans_scene_recoit_le_hub_canon() {
        let data = game_data();
        let g = &data.galaxy;
        // Concordia : un hub canon, aucune mise en scène.
        let concordia = id("sys_concordia");
        assert!(data.scene(&concordia).is_none());
        let hub = generic_hub(g, None, &concordia).unwrap_or_else(|| panic!("pas de hub"));
        assert_eq!(hub.kind, StructureKind::PortalHub);
        assert_eq!(hub.around, id("concordia"));
        assert_eq!(hub.id, "concordia_portal_hub");
        // Le même d'une visite à l'autre.
        assert_eq!(generic_hub(g, None, &concordia), Some(hub.clone()));
        // L'arrivée se fait près de lui.
        let arrival = arrival_of(g, None, &concordia).unwrap_or_else(|| panic!("pas d'arrivée"));
        assert_eq!(arrival.near, hub.id);
        // Son nom est celui de son astre.
        let name = structure_name(&data.lang, g, &hub);
        assert!(name.contains("Concordia"), "{name}");
        // Un système mis en scène avec son propre hub n'en reçoit pas un second.
        let hexis = id("sys_hexis");
        assert!(generic_hub(g, data.scene(&hexis), &hexis).is_none());
        // Ni un système sans hub canon (Ferrok).
        assert!(generic_hub(g, None, &id("sys_ferrok")).is_none());
    }

    #[test]
    fn toute_economie_a_un_point_de_livraison() {
        let data = game_data();
        for system in data.sim.economies.keys() {
            assert!(has_delivery_point(&data, system), "{system}");
        }
        // Vorrh : hub de Portails autour de Vorrhal, chantiers (P32).
        let vorrh = id("sys_vorrh");
        let points = nav_points(&data, &data.lang, &vorrh);
        let hub = points
            .iter()
            .find(|p| p.kind == NavKind::Structure(StructureKind::PortalHub))
            .unwrap_or_else(|| panic!("pas de hub à Vorrh : {points:?}"));
        let vorrhal = points
            .iter()
            .find(|p| p.id == "vorrhal")
            .unwrap_or_else(|| panic!("pas de Vorrhal"));
        // En orbite de Vorrhal, à 1,5 rayon de sa surface.
        let altitude = hub.pos.distance(vorrhal.pos) / vorrhal.radius;
        assert!((altitude - 2.5).abs() < 0.2, "{altitude}");
        assert!(
            points
                .iter()
                .any(|p| p.kind == NavKind::Structure(StructureKind::Shipyard))
        );
    }

    #[test]
    fn l_autopilote_s_arrete_hors_de_la_surface_et_a_portee() {
        let data = game_data();
        let delivery = data.career.contracts.delivery_range;
        for place in ["sys_hexis", "sys_vorrh", "sys_concordia", "calderis"] {
            let points = nav_points(&data, &data.lang, &id(place));
            assert!(!points.is_empty(), "{place}");
            for p in &points {
                assert!(p.stop > p.radius, "{place} : {p:?}");
                if p.is_delivery() {
                    assert!(p.stop < delivery, "{place} : {p:?}");
                }
            }
        }
        // Le port d'Hexis : l'arrêt est à portée d'amarrage.
        let hexis = nav_points(&data, &data.lang, &id("sys_hexis"));
        let port = hexis
            .iter()
            .find(|p| p.kind == NavKind::Dock)
            .unwrap_or_else(|| panic!("pas de port à Hexis"));
        let range = data
            .scene(&id("sys_hexis"))
            .and_then(|s| s.docks.first())
            .map_or(0.0, |d| d.range);
        assert!(port.stop < range, "{port:?}");
    }
}
