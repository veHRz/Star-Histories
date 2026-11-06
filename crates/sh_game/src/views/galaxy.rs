//! Vue Galaxie : la carte d'Astréa (M3, refaite en M9).
//!
//! Tout ce qui s'affiche vient de `sh_data::public_map` : rien de secret, ni élément ni texte
//! (règle d'or n°3 ; test `la_carte_ne_montre_aucun_secret`).
//!
//! Composition (M9, direction « futur usé » de l'interface) :
//! - **fond** : la galaxie vue de dessus (`ui/map/galaxy.png`, généré par `tools/map/map_art.py`,
//!   bulbe au noyau, bras du ciel de P17), sur un noir de la même teinte que le bord de l'image ;
//!   les nébuleuses publiques en nappes (`ui/map/nebula.png`) à leur position et à leur rayon ;
//! - **zones** en aplats translucides (la Zone Inexplorée en voile sombre) et contours marqués
//!   avec un halo ; grille A–T × 1–13 discrète, repères collés aux bords de l'écran ;
//! - **icônes** par type (système, monde isolé, planète errante, monde détruit, station, hub de
//!   Portails, noyau), teintées par l'affiliation ; flottes connues en couronne autour de leur
//!   dernière position, d'autant plus pâles que l'information est ancienne ; position du joueur
//!   et destination ; itinéraire par Portail (arcs cyan, de hub en hub) et par hyperespace
//!   (tirets ambre, contournant les nébuleuses) ;
//! - **étiquettes** placées sans chevauchement (`sh_data::map::layout_labels`) : à grande
//!   échelle, seules les plus importantes restent ; les icônes trop serrées s'éclaircissent ;
//! - **légende**, infobulle au survol (champs publics seulement), panneau d'itinéraire.
//!
//! Depuis M10 :
//! - **détail des systèmes** au plus près : astres, stations et points de livraison de chaque
//!   système public (`sh_data::map::system_detail`, mêmes identifiants que l'autopilote), en
//!   icônes et noms, orbites en traits fins ; fondu enchaîné avec l'icône du système selon le
//!   zoom (`sh_data::map::detail_fade`). Rien de secret : la disposition publique de la vue
//!   Globale, et les structures de la mise en scène ;
//! - **contrats en cours** : un panneau sous la date (émetteur, trajet, échéance) et un losange
//!   sur chaque destination (sur ses points de livraison au plus près) ;
//! - **autopilote** : une cible du lieu courant, cliquée, se propose dans le panneau
//!   d'itinéraire (`voyage::MapTarget`) ; `1` ou le bouton l'engage et referme la carte ;
//! - les touches ne sont plus écrites dans les panneaux : l'encart des touches
//!   (`ui::keys_panel::KeyHelp`) les donne, selon ce qui est choisi.
//!
//! Les icônes et les étiquettes sont des nœuds d'interface (taille constante à l'écran, nettes
//! à l'échelle de rendu réduite) dans une couche sous les panneaux ; le fond, les zones et les
//! traits sont dans la scène 3D vue de dessus. Une poignée d'entités en tout : la carte reste
//! légère sur la machine de référence.
//!
//! Repère : 1 unité de carte = 1 unité du monde, x vers l'est, y de la carte vers le sud (+z),
//! nord en haut de l'écran.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::ui_widgets::Activate;
use bevy::window::PrimaryWindow;
use bevy_enhanced_input::prelude::*;
use sh_core::MapPos;
use sh_core::grid::{CELL_SIZE, COLUMNS, MAP_HEIGHT, MAP_WIDTH, ROWS};
use sh_data::map::{
    DETAIL_RADIUS, DetailPoint, LabelRequest, ScreenRect, TargetClick, detail_fade, layout_labels,
    system_detail, target_click, thin_markers, triangulate,
};
use sh_data::scene::{NavKind, NavPoint, nav_points};
use sh_data::travel::endpoint;
use sh_data::{Faction, Galaxy, MapEntry, MapKind, Place, PlacedKind, StructureKind, public_map};
use sh_sim::ContractState;
use sh_sim::travel::{blocking_nebulae, hyperspace_path};

use crate::career::{Career, contract_title};
use crate::clock::{GameClock, HudCorner, format_date};
use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::input::{
    Pan, Pause, Select, SkipTutorial, ToggleMap, TravelHyperspace, TravelPortal, Zoom,
};
use crate::living::{PlayerKnowledge, display_name};
use crate::options::UserSettings;
use crate::state::{ControlMode, ViewLevel};
use crate::tutorial::Tutorial;
use crate::ui::keys_panel::{KeyHelp, KeyHint};
use crate::ui::theme::{self, Frame, MenuButton, UiFonts};
use crate::views::view_banner;
use crate::voyage::{
    HoveredEntry, HoveredTarget, Location, MapTarget, NavChoice, ResumeFlight, RoutePlan, ShipFuel,
};

pub struct GalaxyViewPlugin;

impl Plugin for GalaxyViewPlugin {
    fn build(&self, app: &mut App) {
        let in_map = in_state(ViewLevel::Galaxy);
        app.init_gizmo_group::<RouteGizmos>()
            .init_gizmo_group::<DashedRouteGizmos>()
            .init_gizmo_group::<RouteGlowGizmos>()
            .init_gizmo_group::<OrbitGizmos>()
            .init_resource::<MapScreen>()
            .add_observer(on_autopilot_button)
            .add_systems(Startup, configure_route_gizmos)
            .add_systems(OnEnter(ViewLevel::Galaxy), spawn_galaxy)
            .add_systems(
                Update,
                // Une condition par système (AGENTS.md §4, groupes chaînés).
                (
                    pan_and_zoom.run_if(in_map.clone()),
                    project_map.run_if(in_map.clone()),
                    hover.run_if(in_map.clone()),
                    place_markers.run_if(in_map.clone()),
                    place_map_labels.run_if(in_map.clone()),
                    show_tooltip.run_if(in_map.clone()),
                    show_route.run_if(in_map.clone()),
                    give_test_contract.run_if(in_map.clone()),
                    engage_on_request.run_if(in_map.clone()),
                    show_contracts.run_if(in_map.clone()),
                    update_map_keys.run_if(in_map.clone()),
                    animate_dashes.run_if(in_map.clone()),
                    draw_route.run_if(in_map.clone()),
                    draw_orbits.run_if(in_map),
                )
                    .chain(),
            );
    }
}

const SCOPE: DespawnOnExit<ViewLevel> = DespawnOnExit(ViewLevel::Galaxy);
const CENTER: Vec2 = Vec2::new(500.0, 330.0);
/// Au plus près (M10), le disque d'un système (`DETAIL_RADIUS`) remplit presque l'écran.
const MIN_HEIGHT: f32 = 16.0;
const MAX_HEIGHT: f32 = 1400.0;
/// Le fond de la caméra : exactement le bord de `galaxy.png` (`BACKGROUND` de
/// `tools/map/map_art.py`), pour qu'aucune couture ne se voie.
const BACKGROUND: Color = Color::srgb(4.0 / 255.0, 5.0 / 255.0, 10.0 / 255.0);
/// `galaxy.png` couvre ± 1,35 rayon de l'ellipse autour du centre (`MARGIN` du générateur) ;
/// `nebula.png`, ± 1,45 rayon de la nébuleuse (`NEBULA_EXTENT`).
const GALAXY_IMAGE_MARGIN: f32 = 1.35;
const NEBULA_IMAGE_EXTENT: f32 = 1.45;
/// Hors de ce rectangle (unités du monde autour du centre), la caméra ne s'égare pas.
const PAN_LIMIT: Vec2 = Vec2::new(650.0, 450.0);
/// Au-delà de ce zoom (pixels par unité de carte), les noms des zones s'effacent.
const ZONE_LABEL_MAX_SCALE: f32 = 3.5;
/// Âge (jours) au-delà duquel une flotte connue est au plus pâle.
const FLEET_STALE_DAYS: f32 = 120.0;
/// Les noms du détail d'un système n'apparaissent qu'au-delà de cette opacité.
const DETAIL_LABEL_MIN: f32 = 0.35;
/// Le détail se survole (et l'icône du système ne se survole plus) au-delà de cette opacité.
const DETAIL_HOVER_MIN: f32 = 0.5;
/// Contrats listés au plus dans le panneau (les autres se comptent).
const CONTRACTS_SHOWN: usize = 4;

/// Les traits de l'itinéraire : Portail (trait plein), hyperespace (tirets), halo dessous.
#[derive(Default, Reflect, GizmoConfigGroup)]
struct RouteGizmos;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct DashedRouteGizmos;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct RouteGlowGizmos;

/// Les orbites du détail des systèmes, en traits fins.
#[derive(Default, Reflect, GizmoConfigGroup)]
struct OrbitGizmos;

/// Un point du détail d'un système, à sa place sur la carte.
struct DetailMarker {
    /// L'entrée de la carte du système qui le contient.
    entry: usize,
    owner: sh_core::Id<Place>,
    point: DetailPoint,
    pos: MapPos,
    /// Le centre de son orbite sur la carte (son parent, ou le centre du système).
    orbit_center: MapPos,
}

/// Le détail des systèmes (calculé une fois à l'entrée de la vue) et les cibles du lieu courant.
#[derive(Resource)]
struct SystemDetails {
    points: Vec<DetailMarker>,
    /// L'ordre d'importance des points (icônes gardées et noms placés en premier).
    order: Vec<usize>,
    /// Le point le plus proche du vaisseau, quand la carte a été ouverte en vol dans ce système :
    /// le repère « vous êtes ici » s'y pose au plus près.
    here: Option<usize>,
    /// Les cibles du lieu courant (système ou monde isolé), pour l'autopilote.
    local: Vec<NavPoint>,
}

/// Survol imposé d'un point du détail (autopilote : `SH_HOVER=<cible>`).
#[derive(Resource, Default)]
struct ForcedTarget(Option<usize>);

/// Les entrées de la carte (calculées une fois à l'entrée de la vue), et l'ordre d'importance
/// de leurs icônes et étiquettes.
#[derive(Resource)]
struct MapEntries {
    entries: Vec<MapEntry>,
    order: Vec<usize>,
}

/// Une flotte connue du joueur, à sa dernière position connue (brouillard d'information).
#[derive(Clone, Debug, PartialEq)]
struct FleetMarker {
    name: String,
    /// L'entrée de la carte autour de laquelle elle s'affiche (le système de la planète où elle
    /// stationne, par exemple), et sa place dans la couronne.
    anchor: usize,
    slot: usize,
    at: String,
    faction: Option<sh_core::Id<Faction>>,
    as_of: sh_core::StellarDate,
    /// Âge de l'information, en jours.
    age_days: i64,
}

#[derive(Resource)]
struct FleetMarkers(Vec<FleetMarker>);

/// Un nom de zone, au centre de son contour.
struct ZoneLabel {
    anchor: MapPos,
}

#[derive(Resource)]
struct ZoneLabels(Vec<ZoneLabel>);

/// La flotte connue sous le curseur, s'il y en a une (elle passe avant les lieux).
#[derive(Resource, Default)]
struct HoveredFleet(Option<FleetMarker>);

/// Survol imposé par l'autopilote (`SH_HOVER`) : il remplace le curseur.
#[derive(Resource)]
struct ForcedHover(Option<MapEntry>);

/// Les repères de la carte à l'écran, recalculés à chaque frame (pixels logiques de la
/// fenêtre) : les icônes, les étiquettes et le survol les lisent.
#[derive(Resource, Default)]
struct MapScreen {
    window: Vec2,
    /// Pixels d'écran par unité de carte.
    scale: f32,
    /// Position de chaque entrée (`None` : hors de l'écran), et si son icône est montrée.
    entries: Vec<Option<Vec2>>,
    shown: Vec<bool>,
    fleets: Vec<Option<Vec2>>,
    zones: Vec<Option<Vec2>>,
    /// Repères de la grille : abscisse de chaque colonne, ordonnée de chaque ligne, et la
    /// position des bords de la grille (pour coller les repères à l'écran).
    columns: Vec<Option<f32>>,
    rows: Vec<Option<f32>>,
    grid_top: f32,
    grid_left: f32,
    here: Option<Vec2>,
    dest: Option<Vec2>,
    /// Opacité du détail des systèmes (0 : la galaxie seule ; 1 : le détail seul).
    detail: f32,
    /// Position de chaque point du détail (`None` : hors de l'écran, ou détail invisible), et
    /// si son icône est montrée.
    details: Vec<Option<Vec2>>,
    detail_shown: Vec<bool>,
    /// Les entrées et les points du détail qui sont la destination d'un contrat en cours.
    contract_entries: Vec<bool>,
    contract_points: Vec<bool>,
}

/// La couche des icônes et des étiquettes, sous tous les panneaux.
#[derive(Component)]
struct MapLayer;

/// Ce que représente une icône de la carte.
#[derive(Component, Clone, Copy, Debug)]
enum MapIcon {
    Entry(usize),
    /// Le badge « hub de Portails » d'une entrée, accroché à son icône.
    Hub(usize),
    Fleet(usize),
    Here,
    Destination,
    /// Un point du détail d'un système.
    Detail(usize),
    /// Le losange d'une destination de contrat : sur l'icône d'un système, ou sur un point de
    /// livraison de son détail.
    ContractEntry(usize),
    ContractPoint(usize),
}

/// La couleur d'origine d'un repère qui s'efface en fondu (icône ou nom).
#[derive(Component, Clone, Copy)]
struct BaseColor(Color);

/// Le bouton « Autopilote vers … » du panneau d'itinéraire.
#[derive(Component)]
struct AutopilotButton;

#[derive(Component)]
struct AutopilotButtonText;

/// Le panneau des contrats en cours, et la liste qu'il reconstruit quand elle change.
#[derive(Component)]
struct ContractsPanel;

/// Porte l'encart des touches de la carte (`KeyHelp`), tenu à jour selon ce qui est choisi.
#[derive(Component)]
struct MapKeys;

/// Ce que nomme une étiquette de la carte.
#[derive(Component, Clone, Copy, Debug)]
struct MapLabel {
    target: LabelTarget,
    /// Taille du texte (estimation de la largeur avant la première mise en page).
    font_size: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelTarget {
    Entry(usize),
    Zone(usize),
    Column(usize),
    Row(usize),
    Detail(usize),
}

/// Les panneaux de la carte que les étiquettes évitent.
#[derive(Component)]
struct MapPanel;

/// L'infobulle (plaquette sous le curseur) : le nom en titre, les champs publics dessous.
#[derive(Component)]
struct Tooltip;

#[derive(Component)]
struct TooltipTitle;

#[derive(Component)]
struct TooltipBody;

/// Les lignes du panneau d'itinéraire.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum RouteLine {
    /// Position et carburant (relevé à chasse fixe).
    Status,
    /// « Route vers … », ou la consigne sans route.
    Title,
    /// Portail et hyperespace (cachée sans route).
    Options,
}

/// Position du monde d'un point de la carte.
fn world(pos: MapPos) -> Vec3 {
    Vec3::new(pos.x as f32 - CENTER.x, 0.0, pos.y as f32 - CENTER.y)
}

/// Couleur d'affiliation (lisibilité de la carte, pas une charte).
fn faction_color(faction: Option<&sh_core::Id<Faction>>) -> Color {
    match faction.map(|f| f.as_str()) {
        Some("ugl" | "fou" | "fdu") => Color::srgb(0.40, 0.66, 1.0),
        Some("empire") => Color::srgb(0.96, 0.36, 0.30),
        Some("hexa_corp") => Color::srgb(0.98, 0.80, 0.34),
        Some("federation") => Color::srgb(0.74, 0.74, 0.82),
        Some("royaume_grothul") => Color::srgb(0.56, 0.86, 0.42),
        Some("ligue_des_pirates" | "pirates_independants") => Color::srgb(0.88, 0.46, 0.86),
        Some(_) => Color::srgb(0.80, 0.74, 0.62),
        None => Color::srgb(0.64, 0.66, 0.70),
    }
}

fn entry_color(entry: &MapEntry) -> Color {
    match entry.kind {
        _ if entry.destroyed => Color::srgb(0.52, 0.52, 0.55),
        MapKind::Phenomenon => Color::srgb(0.88, 0.84, 0.76),
        _ => faction_color(entry.controller.as_ref()),
    }
}

/// L'icône d'une entrée (image et taille à l'écran) ; `None` pour une nébuleuse, dessinée en
/// nappe dans la scène.
fn entry_icon(entry: &MapEntry) -> Option<(&'static str, f32)> {
    Some(match entry.kind {
        MapKind::System => ("ui/map/system.png", 28.0),
        MapKind::Body if entry.destroyed => ("ui/map/ruin.png", 22.0),
        MapKind::Body if entry.rogue => ("ui/map/rogue.png", 22.0),
        MapKind::Body if entry.parent.is_some() => ("ui/map/world.png", 14.0),
        MapKind::Body => ("ui/map/world.png", 22.0),
        MapKind::Station => ("ui/map/station.png", 20.0),
        MapKind::Phenomenon if entry.radius.is_some() => return None,
        MapKind::Phenomenon => ("ui/map/core.png", 20.0),
    })
}

fn icon_size(entry: &MapEntry) -> f32 {
    entry_icon(entry).map_or(0.0, |(_, size)| size)
}

/// L'icône d'un point du détail d'un système : image, taille à l'écran, couleur. Les points de
/// livraison (structures) en ambre, le hub de Portails et le port en cyan des écrans.
fn detail_icon(kind: NavKind) -> (&'static str, f32, Color) {
    match kind {
        NavKind::Body(PlacedKind::Star) => ("ui/map/world.png", 24.0, Color::srgb(1.0, 0.84, 0.50)),
        NavKind::Body(PlacedKind::GasGiant) => {
            ("ui/map/world.png", 22.0, Color::srgb(0.86, 0.76, 0.60))
        }
        NavKind::Body(PlacedKind::Planet) => ("ui/map/world.png", 18.0, theme::TEXT),
        NavKind::Body(PlacedKind::Moon) => ("ui/map/world.png", 12.0, theme::TEXT_DIM),
        NavKind::Body(PlacedKind::Station) => ("ui/map/station.png", 16.0, theme::TEXT),
        NavKind::Structure(StructureKind::PortalHub) => ("ui/map/hub.png", 16.0, theme::SCREEN),
        NavKind::Structure(_) => ("ui/map/station.png", 16.0, theme::ACCENT),
        NavKind::Dock => ("ui/map/station.png", 14.0, theme::SCREEN),
    }
}

/// Le genre d'un point du détail, en clair (infobulle, panneau de la cible).
fn detail_kind_key(kind: NavKind) -> &'static str {
    match kind {
        NavKind::Body(PlacedKind::Star) => "map.detail.kind.star",
        NavKind::Body(PlacedKind::Planet) => "map.detail.kind.planet",
        NavKind::Body(PlacedKind::GasGiant) => "map.detail.kind.gas_giant",
        NavKind::Body(PlacedKind::Moon) => "map.detail.kind.moon",
        NavKind::Body(PlacedKind::Station) => "map.detail.kind.station",
        NavKind::Structure(StructureKind::PortalHub) => "map.detail.kind.portal_hub",
        NavKind::Structure(StructureKind::Shipyard) => "map.detail.kind.shipyard",
        NavKind::Structure(StructureKind::Depot) => "map.detail.kind.depot",
        NavKind::Dock => "map.detail.kind.dock",
    }
}

/// L'ordre d'importance des points du détail : étoiles, planètes, points de livraison, ports,
/// stations, lunes.
fn detail_rank(kind: NavKind) -> u8 {
    match kind {
        NavKind::Body(PlacedKind::Star) => 0,
        NavKind::Body(PlacedKind::Planet | PlacedKind::GasGiant) => 1,
        NavKind::Structure(_) => 2,
        NavKind::Dock => 3,
        NavKind::Body(PlacedKind::Station) => 4,
        NavKind::Body(PlacedKind::Moon) => 5,
    }
}

/// Le détail de tous les systèmes publics de la carte, chaque point à sa place.
fn build_details(data: &Data, texts: &Texts, entries: &[MapEntry]) -> Vec<DetailMarker> {
    let mut out = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        if entry.kind != MapKind::System {
            continue;
        }
        let Some(system) = data.galaxy.system(&entry.id) else {
            continue;
        };
        let points = system_detail(data, texts, system);
        let positions: Vec<MapPos> = points.iter().map(|p| p.map_pos(entry.pos)).collect();
        for point in points {
            let pos = point.map_pos(entry.pos);
            let orbit_center = point
                .parent
                .and_then(|p| positions.get(p).copied())
                .unwrap_or(entry.pos);
            out.push(DetailMarker {
                entry: i,
                owner: entry.id.clone(),
                point,
                pos,
                orbit_center,
            });
        }
    }
    out
}

/// Une distance de l'espace, lisible : en kilomètres, sans décimale.
fn distance_text(texts: &Texts, meters: f64) -> String {
    texts.format(
        "map.target.distance",
        &[("km", &texts.number((meters / 1000.0).max(0.0), 0))],
    )
}

/// L'ordre d'importance des entrées (icônes gardées et étiquettes placées en premier) :
/// systèmes, mondes isolés, stations, phénomènes, puis les lunes ; les plus peuplés d'abord.
fn importance(entries: &[MapEntry]) -> Vec<usize> {
    let rank = |e: &MapEntry| match e.kind {
        MapKind::System => 0,
        MapKind::Body if e.parent.is_none() => 1,
        MapKind::Station => 2,
        MapKind::Phenomenon => 3,
        MapKind::Body => 4,
    };
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by(|&a, &b| {
        let (ea, eb) = (&entries[a], &entries[b]);
        rank(ea)
            .cmp(&rank(eb))
            .then(eb.population.cmp(&ea.population))
            .then(ea.id.cmp(&eb.id))
    });
    order
}

/// L'entrée de la carte qui représente un lieu : lui-même, ou le système qui le contient.
fn entry_for_place(
    galaxy: &Galaxy,
    entries: &[MapEntry],
    at: &sh_core::Id<Place>,
) -> Option<usize> {
    let find = |id: &sh_core::Id<Place>| entries.iter().position(|e| &e.id == id);
    find(at).or_else(|| {
        let system = galaxy
            .body(at)
            .and_then(|b| b.system.clone())
            .or_else(|| galaxy.station(at).and_then(|s| s.system.clone()))?;
        find(&system)
    })
}

/// Le système qui contient un lieu (un astre ou une station d'un système), sinon le lieu même.
fn system_of(galaxy: &Galaxy, id: &sh_core::Id<Place>) -> sh_core::Id<Place> {
    galaxy
        .body(id)
        .and_then(|b| b.system.clone())
        .or_else(|| galaxy.station(id).and_then(|s| s.system.clone()))
        .unwrap_or_else(|| id.clone())
}

/// Population lisible : « 3,4 milliard(s) », « 250 million(s) ».
fn population_text(texts: &Texts, n: u64) -> String {
    let n = n as f64;
    if n >= 1e9 {
        texts.format("map.unit.billion", &[("value", &texts.number(n / 1e9, 1))])
    } else if n >= 1e6 {
        texts.format("map.unit.million", &[("value", &format!("{:.0}", n / 1e6))])
    } else {
        format!("{n:.0}")
    }
}

fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        // Les aplats se voient d'en haut quel que soit le sens de leur contour.
        cull_mode: None,
        alpha_mode: if color.alpha() < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        ..default()
    }
}

/// Un quadrilatère posé à plat sur le plan de la carte (face vers le haut, nord en haut).
fn flat_quad(width: f32, height: f32) -> Rectangle {
    Rectangle::new(width, height)
}

fn flat(translation: Vec3) -> Transform {
    Transform::from_translation(translation)
        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
}

/// L'aplat d'une zone : son contour découpé en triangles, à plat.
fn zone_mesh(outline: &[MapPos], height: f32) -> Mesh {
    let positions: Vec<[f32; 3]> = outline
        .iter()
        .map(|&p| {
            let w = world(p);
            [w.x, height, w.z]
        })
        .collect();
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let indices: Vec<u32> = triangulate(outline)
        .into_iter()
        .flatten()
        .map(|i| i as u32)
        .collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

/// Centre (barycentre des surfaces) d'un contour fermé.
fn centroid(outline: &[MapPos]) -> MapPos {
    let n = outline.len();
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (outline[i], outline[(i + 1) % n]);
        let cross = p.x * q.y - q.x * p.y;
        a += cross;
        cx += (p.x + q.x) * cross;
        cy += (p.y + q.y) * cross;
    }
    if a.abs() < 1e-9 {
        let k = n.max(1) as f64;
        return MapPos::new(
            outline.iter().map(|p| p.x).sum::<f64>() / k,
            outline.iter().map(|p| p.y).sum::<f64>() / k,
        );
    }
    MapPos::new(cx / (3.0 * a), cy / (3.0 * a))
}

/// `SH_MAP_CENTER=x,y` (autopilote) : le point de la carte au centre de l'écran.
fn map_center_from_env() -> Option<MapPos> {
    let value = std::env::var("SH_MAP_CENTER").ok()?;
    let (x, y) = value.split_once(',')?;
    Some(MapPos::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
}

fn configure_route_gizmos(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<RouteGizmos>();
    config.line.width = 3.0;
    config.line.joints = GizmoLineJoint::Round(4);
    let (config, _) = store.config_mut::<DashedRouteGizmos>();
    config.line.width = 3.0;
    config.line.style = GizmoLineStyle::Dashed {
        gap_scale: 2.0,
        line_scale: 3.5,
    };
    let (config, _) = store.config_mut::<RouteGlowGizmos>();
    config.line.width = 10.0;
    config.line.joints = GizmoLineJoint::Round(4);
    let (config, _) = store.config_mut::<OrbitGizmos>();
    config.line.width = 1.2;
}

#[allow(clippy::too_many_arguments)]
fn spawn_galaxy(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    asset_server: Res<AssetServer>,
    (location, knowledge, clock, resume, corner): (
        Option<Res<Location>>,
        Option<Res<PlayerKnowledge>>,
        Option<Res<GameClock>>,
        Option<Res<ResumeFlight>>,
        Option<Single<Entity, With<HudCorner>>>,
    ),
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    mut mode: ResMut<NextState<ControlMode>>,
) {
    mode.set(ControlMode::Map);
    let g = &data.galaxy;
    let entries = public_map(g);
    let order = importance(&entries);

    // Caméra centrée sur le vaisseau (un système ou un monde isolé).
    let location_id = location.as_ref().map(|l| l.system.clone());
    let here = location
        .as_ref()
        .and_then(|l| l.endpoint(&data))
        .map_or(Vec3::ZERO, |e| world(e.pos()));
    // Autopilote (captures) : `SH_MAP_CENTER=x,y` (coordonnées de la carte) et
    // `SH_MAP_HEIGHT=<altitude>` (de 60 à 1400 ; 1100 montre toute la galaxie) cadrent la carte.
    let here = map_center_from_env().map_or(here, world);
    let height = std::env::var("SH_MAP_HEIGHT")
        .ok()
        .and_then(|v| v.trim().parse::<f32>().ok())
        .map_or(420.0, |h| h.clamp(MIN_HEIGHT, MAX_HEIGHT));
    commands.spawn((
        Name::new("Caméra de la carte"),
        SCOPE,
        WorldCamera,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(BACKGROUND),
            ..default()
        },
        Transform::from_translation(here + Vec3::Y * height).looking_at(here, Vec3::NEG_Z),
    ));

    let zones = spawn_backdrop(
        &mut commands,
        g,
        &entries,
        &asset_server,
        (&mut meshes, &mut materials, &mut gizmo_assets),
    );
    let (zone_labels, zone_names): (Vec<ZoneLabel>, Vec<(String, Color)>) =
        zones.into_iter().unzip();

    // Les flottes connues, en couronne autour de leur dernière position connue.
    let fleets = known_fleets(g, &entries, knowledge.as_deref(), clock.as_deref());

    // La couche des icônes et des étiquettes : sous tous les panneaux de l'interface.
    let layer = commands
        .spawn((
            Name::new("Repères de la carte"),
            SCOPE,
            MapLayer,
            Pickable::IGNORE,
            GlobalZIndex(-1),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
        ))
        .id();
    let icon = |path: &'static str, size: f32, color: Color, target: MapIcon| {
        (
            target,
            BaseColor(color),
            ChildOf(layer),
            Pickable::IGNORE,
            ImageNode::new(asset_server.load(path)).with_color(color),
            Node {
                position_type: PositionType::Absolute,
                width: px(size),
                height: px(size),
                ..default()
            },
            UiTransform::IDENTITY,
            Visibility::Hidden,
        )
    };
    let label = |text: String, font: TextFont, color: Color, target: LabelTarget, size: f32| {
        (
            MapLabel {
                target,
                font_size: size,
            },
            BaseColor(color),
            ChildOf(layer),
            Pickable::IGNORE,
            Text::new(text),
            font,
            TextColor(color),
            // Ombre courte : le nom reste lisible sur le bulbe et les nébuleuses.
            theme::label_shadow(),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Visibility::Hidden,
        )
    };
    for (i, entry) in entries.iter().enumerate() {
        if let Some((path, size)) = entry_icon(entry) {
            commands.spawn(icon(path, size, entry_color(entry), MapIcon::Entry(i)));
        }
        if entry.has_portal_hub {
            commands.spawn(icon("ui/map/hub.png", 14.0, theme::SCREEN, MapIcon::Hub(i)));
        }
        let (font, color, size) = match entry.kind {
            MapKind::System => (fonts.title(14.0), theme::TEXT, 14.0),
            MapKind::Body if entry.parent.is_none() => (fonts.body(13.0), theme::TEXT, 13.0),
            _ => (fonts.body(12.0), theme::TEXT_DIM, 12.0),
        };
        commands.spawn(label(
            entry.name.clone(),
            font,
            color,
            LabelTarget::Entry(i),
            size,
        ));
    }
    for (i, (name, color)) in zone_names.into_iter().enumerate() {
        let (font, spacing) = fonts.heading(12.0);
        commands.spawn((
            label(name, font, color, LabelTarget::Zone(i), 12.0),
            spacing,
        ));
    }
    for col in 0..usize::from(COLUMNS) {
        let letter = char::from(b'A' + col as u8).to_string();
        commands.spawn(label(
            letter,
            fonts.mono(12.0),
            theme::TEXT_DIM.with_alpha(0.7),
            LabelTarget::Column(col),
            12.0,
        ));
    }
    for row in 0..usize::from(ROWS) {
        commands.spawn(label(
            (row + 1).to_string(),
            fonts.mono(12.0),
            theme::TEXT_DIM.with_alpha(0.7),
            LabelTarget::Row(row),
            12.0,
        ));
    }
    for (i, fleet) in fleets.iter().enumerate() {
        // Plus l'information est vieille, plus la flotte est pâle.
        let age = (fleet.age_days as f32 / FLEET_STALE_DAYS).clamp(0.0, 1.0);
        let color = faction_color(fleet.faction.as_ref()).with_alpha(1.0 - 0.65 * age);
        commands.spawn(icon("ui/map/fleet.png", 18.0, color, MapIcon::Fleet(i)));
    }
    // Le détail des systèmes : une icône et un nom par point, cachés jusqu'au zoom.
    let details = build_details(&data, &texts, &entries);
    for (k, marker) in details.iter().enumerate() {
        let (path, size, color) = detail_icon(marker.point.kind);
        commands.spawn(icon(path, size, color, MapIcon::Detail(k)));
        if let Some(name) = &marker.point.name {
            let (font, color, size) = match marker.point.kind {
                NavKind::Body(PlacedKind::Planet | PlacedKind::GasGiant | PlacedKind::Star) => {
                    (fonts.body(13.0), theme::TEXT, 13.0)
                }
                NavKind::Structure(_) => (fonts.body(12.0), theme::ACCENT, 12.0),
                _ => (fonts.body(12.0), theme::TEXT_DIM, 12.0),
            };
            commands.spawn(label(
                name.clone(),
                font,
                color,
                LabelTarget::Detail(k),
                size,
            ));
        }
    }
    // Les losanges des destinations de contrat : un par système, un par point de livraison ;
    // montrés selon les contrats en cours (`project_map`).
    let diamond = |size: f32, target: MapIcon| {
        (
            target,
            BaseColor(theme::ACCENT),
            ChildOf(layer),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                width: px(size),
                height: px(size),
                border: UiRect::all(px(2)),
                ..default()
            },
            BorderColor::all(theme::ACCENT),
            UiTransform::from_rotation(Rot2::degrees(45.0)),
            Visibility::Hidden,
        )
    };
    for (i, entry) in entries.iter().enumerate() {
        if entry.kind == MapKind::System {
            commands.spawn(diamond(30.0, MapIcon::ContractEntry(i)));
        }
    }
    for (k, marker) in details.iter().enumerate() {
        if marker.point.delivery {
            commands.spawn(diamond(22.0, MapIcon::ContractPoint(k)));
        }
    }
    commands.spawn(icon(
        "ui/map/target.png",
        40.0,
        theme::TEXT,
        MapIcon::Destination,
    ));
    commands.spawn(icon("ui/map/here.png", 40.0, theme::ACCENT, MapIcon::Here));

    spawn_panels(&mut commands, &texts, &fonts, &asset_server);
    spawn_contracts_panel(&mut commands, &fonts, corner.as_deref().copied());
    commands.spawn((
        Name::new("Touches de la carte"),
        SCOPE,
        MapKeys,
        KeyHelp::default(),
    ));

    // Les cibles du lieu courant, et le point du détail le plus proche du vaisseau quand la
    // carte a été ouverte en vol dans ce système.
    let local = location_id
        .as_ref()
        .map(|here| nav_points(&data, &texts, here))
        .unwrap_or_default();
    let here_point = match (&location_id, resume.as_deref()) {
        (Some(here), Some(resume)) if &resume.system == here => details
            .iter()
            .enumerate()
            .filter(|(_, m)| &m.owner == here && m.point.is_target())
            .min_by(|a, b| {
                let da = (a.1.point.real - resume.pos).length();
                let db = (b.1.point.real - resume.pos).length();
                da.total_cmp(&db)
            })
            .map(|(k, _)| k),
        _ => None,
    };

    // Autopilote (captures) : `SH_HOVER=<lieu ou cible>` simule le survol, `SH_ROUTE=<lieu>`
    // pose un itinéraire (système ou monde isolé), `SH_MAP_TARGET=<cible>` propose une cible du
    // lieu courant à l'autopilote (panneau d'itinéraire).
    let pick = |var: &str| {
        let id = std::env::var(var).ok()?;
        entries.iter().find(|e| e.id.as_str() == id).cloned()
    };
    let pick_detail = |var: &str| {
        let id = std::env::var(var).ok()?;
        let id = id.trim();
        details
            .iter()
            .position(|m| m.point.id.as_deref() == Some(id))
    };
    let forced_hover = pick("SH_HOVER");
    let forced_target = forced_hover
        .is_none()
        .then(|| pick_detail("SH_HOVER"))
        .flatten();
    if let (Some(dest), Some(here)) = (pick("SH_ROUTE"), location_id.clone())
        && dest.destination
        && dest.id != here
    {
        commands.insert_resource(crate::voyage::plan_route(&data, &here, &dest.id));
    }
    let mut map_target = MapTarget::default();
    if let (Ok(id), Some(here)) = (std::env::var("SH_MAP_TARGET"), location_id.as_ref()) {
        match local.iter().find(|p| p.id == id.trim()) {
            Some(point) => {
                map_target.0 = Some(NavChoice {
                    id: point.id.clone(),
                    name: point.name.clone(),
                    owner: here.clone(),
                    click: TargetClick::Autopilot,
                });
            }
            None => warn!("SH_MAP_TARGET : « {id} » n'est pas une cible du lieu courant"),
        }
    }
    commands.insert_resource(map_target);
    commands.insert_resource(ForcedTarget(forced_target));
    commands.insert_resource(SystemDetails {
        order: detail_order(&details),
        points: details,
        here: here_point,
        local,
    });
    commands.insert_resource(ForcedHover(forced_hover.clone()));
    commands.insert_resource(HoveredEntry(forced_hover));
    commands.insert_resource(FleetMarkers(fleets));
    commands.insert_resource(HoveredFleet::default());
    commands.insert_resource(ZoneLabels(zone_labels));
    commands.insert_resource(MapEntries { entries, order });
    commands.insert_resource(MapScreen::default());

    commands.spawn(view_banner(
        &texts,
        &fonts,
        texts.get("view.galaxy.title"),
        SCOPE,
    ));
}

/// Le décor de la scène 3D : la galaxie vue de dessus (cadrée sur l'ellipse canon, §3.1), les
/// zones (aplat, contour et halo ; la Zone Inexplorée en voile sombre et, comme le No Man's
/// Land, sans maître, en tirets), la grille de repérage et l'ellipse, les nébuleuses. Rend le
/// centre, le nom et la couleur de chaque zone, pour les étiquettes.
fn spawn_backdrop(
    commands: &mut Commands,
    g: &Galaxy,
    entries: &[MapEntry],
    asset_server: &AssetServer,
    (meshes, materials, gizmo_assets): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<GizmoAsset>,
    ),
) -> Vec<(ZoneLabel, (String, Color))> {
    let frame = &g.frame;
    let galaxy_center = world(MapPos::new(frame.center.0, frame.center.1));
    commands.spawn((
        Name::new("Galaxie"),
        SCOPE,
        Mesh3d(meshes.add(flat_quad(
            2.0 * GALAXY_IMAGE_MARGIN * frame.radius_x as f32,
            2.0 * GALAXY_IMAGE_MARGIN * frame.radius_y as f32,
        ))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("ui/map/galaxy.png")),
            ..unlit(Color::WHITE)
        })),
        flat(galaxy_center - Vec3::Y * 3.0),
    ));

    let mut solid = GizmoAsset::default();
    let mut dashed = GizmoAsset::default();
    let mut glow = GizmoAsset::default();
    let mut zones = Vec::new();
    for zone in g.zones.iter().filter(|z| !z.secret && z.outline.len() >= 3) {
        let unexplored = zone.id.as_str() == "zone_inexploree";
        let color = faction_color(zone.controller.as_ref());
        let fill = if unexplored {
            Color::srgba(0.0, 0.0, 0.0, 0.45)
        } else if zone.controller.is_some() {
            color.with_alpha(0.08)
        } else {
            color.with_alpha(0.035)
        };
        commands.spawn((
            Name::new(zone.name.clone()),
            SCOPE,
            Mesh3d(meshes.add(zone_mesh(&zone.outline, -2.0))),
            MeshMaterial3d(materials.add(unlit(fill))),
        ));
        let points: Vec<Vec3> = zone
            .outline
            .iter()
            .chain(zone.outline.first())
            .map(|&p| world(p) + Vec3::Y * 0.2)
            .collect();
        if zone.controller.is_some() {
            solid.linestrip(points.clone(), color.with_alpha(0.85));
            glow.linestrip(points, color.with_alpha(0.16));
        } else {
            dashed.linestrip(
                points,
                color.with_alpha(if unexplored { 0.45 } else { 0.6 }),
            );
        }
        let text_color = if unexplored {
            theme::TEXT_DIM.with_alpha(0.55)
        } else {
            color.with_alpha(0.6)
        };
        zones.push((
            ZoneLabel {
                anchor: centroid(&zone.outline),
            },
            (theme::caps(&zone.name), text_color),
        ));
    }

    // La grille de repérage et l'ellipse canon de la galaxie, en traits fins.
    let mut grid = GizmoAsset::default();
    let line = Color::srgba(0.55, 0.62, 0.80, 0.07);
    for col in 0..=COLUMNS {
        let x = f64::from(col) * CELL_SIZE;
        grid.line(
            world(MapPos::new(x, 0.0)),
            world(MapPos::new(x, MAP_HEIGHT)),
            line,
        );
    }
    for row in 0..=ROWS {
        let y = f64::from(row) * CELL_SIZE;
        grid.line(
            world(MapPos::new(0.0, y)),
            world(MapPos::new(MAP_WIDTH, y)),
            line,
        );
    }
    let ellipse: Vec<Vec3> = (0..=128)
        .map(|k| {
            let a = std::f64::consts::TAU * f64::from(k) / 128.0;
            world(MapPos::new(
                frame.center.0 + frame.radius_x * a.cos(),
                frame.center.1 + frame.radius_y * a.sin(),
            )) + Vec3::Y * 0.1
        })
        .collect();
    grid.linestrip(ellipse, Color::srgba(0.70, 0.76, 0.95, 0.10));
    let dash = GizmoLineStyle::Dashed {
        gap_scale: 2.5,
        line_scale: 4.0,
    };
    for (name, asset, width, style) in [
        ("Grille", grid, 1.0, GizmoLineStyle::Solid),
        ("Halo des zones", glow, 8.0, GizmoLineStyle::Solid),
        ("Contours", solid, 2.0, GizmoLineStyle::Solid),
        ("Contours sans maître", dashed, 1.5, dash),
    ] {
        commands.spawn((
            Name::new(name),
            SCOPE,
            Gizmo {
                handle: gizmo_assets.add(asset),
                line_config: GizmoLineConfig {
                    width,
                    style,
                    joints: GizmoLineJoint::Round(4),
                    ..default()
                },
                ..default()
            },
        ));
    }

    // Les nébuleuses : une nappe à leur position et à leur rayon (données publiques).
    let nebula_material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("ui/map/nebula.png")),
        ..unlit(Color::srgba(1.0, 1.0, 1.0, 0.95))
    });
    for entry in entries.iter().filter(|e| e.kind == MapKind::Phenomenon) {
        let Some(radius) = entry.radius else { continue };
        let side = 2.0 * NEBULA_IMAGE_EXTENT * radius as f32;
        commands.spawn((
            Name::new(entry.name.clone()),
            SCOPE,
            Mesh3d(meshes.add(flat_quad(side, side))),
            MeshMaterial3d(nebula_material.clone()),
            flat(world(entry.pos) - Vec3::Y * 1.0),
        ));
    }
    zones
}

/// Les flottes connues du joueur (jamais une flotte secrète), chacune rattachée à l'entrée de
/// la carte où elle a été vue, à sa place dans la couronne, avec l'âge de l'information.
fn known_fleets(
    g: &Galaxy,
    entries: &[MapEntry],
    knowledge: Option<&PlayerKnowledge>,
    clock: Option<&GameClock>,
) -> Vec<FleetMarker> {
    let today = clock.map(|c| c.date);
    let mut fleets = Vec::new();
    let Some(knowledge) = knowledge else {
        return fleets;
    };
    let mut per_anchor: std::collections::BTreeMap<usize, usize> = Default::default();
    for (id, known) in &knowledge.fleets {
        let Some(fleet) = g.fleets.iter().find(|f| &f.id == id).filter(|f| !f.secret) else {
            continue;
        };
        let Some(anchor) = entry_for_place(g, entries, &known.at) else {
            continue;
        };
        let slot = per_anchor.entry(anchor).or_default();
        fleets.push(FleetMarker {
            name: fleet.name.clone(),
            anchor,
            slot: *slot,
            at: known.at.to_string(),
            faction: fleet.faction.clone(),
            as_of: known.as_of,
            age_days: today.map_or(0, |t| known.as_of.days_until(t).max(0)),
        });
        *slot += 1;
    }
    fleets
}

/// L'infobulle (cachée), le panneau d'itinéraire (en bas à gauche) et la légende.
fn spawn_panels(
    commands: &mut Commands,
    texts: &Texts,
    fonts: &UiFonts,
    asset_server: &AssetServer,
) {
    // Infobulle, panneau d'itinéraire, légende.
    commands.spawn((
        SCOPE,
        Tooltip,
        Pickable::IGNORE,
        // Au-dessus de la couche des repères et des panneaux de la carte, sur un fond presque
        // opaque : les étiquettes ne se lisent pas au travers.
        GlobalZIndex(10),
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            // Large à gauche : la barre ambre de la plaquette.
            padding: UiRect::new(px(14), px(12), px(7), px(9)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.020, 0.025, 0.030, 0.94)),
        fonts.frame(Frame::Plate),
        Visibility::Hidden,
        children![
            (
                TooltipTitle,
                Text::new(""),
                TextLayout::no_wrap(),
                fonts.title(15.0),
                TextColor(theme::TEXT),
                Pickable::IGNORE,
            ),
            (
                TooltipBody,
                Text::new(""),
                TextLayout::no_wrap(),
                fonts.body(14.0),
                TextColor(theme::TEXT_DIM),
                Pickable::IGNORE,
            ),
        ],
    ));
    // Le panneau d'itinéraire : un relevé d'instrument. En haut la position et le carburant (à
    // chasse fixe), puis la route choisie (titre ambre) et ses deux façons d'y aller. Ses
    // lignes ne passent jamais à la ligne d'elles-mêmes : un retour automatique, décidé après
    // la mesure du nœud, faisait déborder le texte du cadre ; le cadre prend la largeur de la
    // plus longue ligne et la hauteur de toutes.
    commands.spawn((
        SCOPE,
        MapPanel,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(20),
            bottom: px(20),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            // Même marge en haut et en bas.
            padding: UiRect::new(px(18), px(18), px(13), px(13)),
            ..default()
        },
        fonts.frame(Frame::Readout),
        children![
            (
                RouteLine::Status,
                Text::new(""),
                TextLayout::no_wrap(),
                fonts.mono(14.0),
                TextColor(theme::SCREEN),
                Pickable::IGNORE,
            ),
            (
                Pickable::IGNORE,
                Node {
                    width: percent(100),
                    height: px(1),
                    margin: UiRect::vertical(px(4)),
                    ..default()
                },
                BackgroundColor(theme::SCREEN_DIM),
            ),
            (
                RouteLine::Title,
                Text::new(""),
                TextLayout::no_wrap(),
                fonts.title(16.0),
                TextColor(theme::ACCENT),
                Pickable::IGNORE,
            ),
            (
                RouteLine::Options,
                Text::new(""),
                TextLayout::no_wrap(),
                fonts.body(15.0),
                TextColor(theme::TEXT),
                Pickable::IGNORE,
            ),
            // « Autopilote vers … » : montré quand une cible du lieu courant est choisie. Le
            // seul élément de la carte qui retient un clic.
            (
                AutopilotButton,
                MenuButton,
                bevy::ui_widgets::Button,
                bevy::ui_widgets::ActivateOnPress,
                Node {
                    display: Display::None,
                    align_self: AlignSelf::FlexStart,
                    margin: UiRect::top(px(6)),
                    padding: UiRect::axes(px(18), px(8)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                fonts.frame(Frame::Button),
                children![(
                    AutopilotButtonText,
                    Text::new(""),
                    TextLayout::no_wrap(),
                    fonts.body(15.0),
                    TextColor(theme::TEXT),
                    Pickable::IGNORE,
                )],
            ),
        ],
    ));
    spawn_legend(commands, texts, fonts, asset_server);
}

/// Le panneau des contrats en cours : sous la date et les nouvelles, dans la colonne du coin
/// haut droit (`clock::HudCorner`), qui les empile sans chevauchement ; à sa place si la
/// colonne n'existe pas. Caché sans contrat ; son contenu suit `show_contracts`.
fn spawn_contracts_panel(commands: &mut Commands, fonts: &UiFonts, corner: Option<Entity>) {
    let mut panel = commands.spawn((
        Name::new("Contrats en cours"),
        SCOPE,
        ContractsPanel,
        MapPanel,
        Pickable::IGNORE,
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            padding: UiRect::new(px(18), px(18), px(11), px(12)),
            ..default()
        },
        fonts.frame(Frame::Readout),
    ));
    match corner {
        Some(corner) => {
            panel.insert(ChildOf(corner));
        }
        None => {
            panel.insert(Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                right: px(20),
                top: px(70),
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: UiRect::new(px(18), px(18), px(11), px(12)),
                ..default()
            });
        }
    }
}

/// L'ordre d'importance des points du détail (`detail_rank`, puis l'ordre des données).
fn detail_order(details: &[DetailMarker]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..details.len()).collect();
    order.sort_by_key(|&k| (detail_rank(details[k].point.kind), k));
    order
}

/// La légende, en bas à droite : une icône (ou un trait) et son sens par ligne.
fn spawn_legend(
    commands: &mut Commands,
    texts: &Texts,
    fonts: &UiFonts,
    asset_server: &AssetServer,
) {
    enum Mark {
        Icon(&'static str, Color),
        Line(Color, bool),
        /// Le losange d'une destination de contrat.
        Diamond,
    }
    let rows = [
        (
            Mark::Icon("ui/map/system.png", theme::TEXT),
            "map.legend.system",
        ),
        (
            Mark::Icon("ui/map/world.png", theme::TEXT),
            "map.legend.world",
        ),
        (
            Mark::Icon("ui/map/rogue.png", theme::TEXT),
            "map.legend.rogue",
        ),
        (
            Mark::Icon("ui/map/ruin.png", Color::srgb(0.52, 0.52, 0.55)),
            "map.legend.ruin",
        ),
        (
            Mark::Icon("ui/map/station.png", theme::TEXT),
            "map.legend.station",
        ),
        (
            Mark::Icon("ui/map/hub.png", theme::SCREEN),
            "map.legend.hub",
        ),
        (
            Mark::Icon("ui/map/nebula_icon.png", Color::srgb(0.85, 0.42, 0.75)),
            "map.legend.nebula",
        ),
        (
            Mark::Icon("ui/map/fleet.png", theme::TEXT),
            "map.legend.fleet",
        ),
        (
            Mark::Icon("ui/map/here.png", theme::ACCENT),
            "map.legend.here",
        ),
        (
            Mark::Icon("ui/map/target.png", theme::TEXT),
            "map.legend.destination",
        ),
        (
            Mark::Icon("ui/map/station.png", theme::ACCENT),
            "map.legend.delivery",
        ),
        (Mark::Diamond, "map.legend.contract"),
        (Mark::Line(theme::SCREEN, false), "map.legend.portal"),
        (Mark::Line(theme::ACCENT, true), "map.legend.hyperspace"),
    ];
    let panel = commands
        .spawn((
            Name::new("Légende de la carte"),
            SCOPE,
            MapPanel,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                right: px(20),
                bottom: px(20),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                padding: UiRect::new(px(16), px(16), px(10), px(12)),
                ..default()
            },
            fonts.frame(Frame::Readout),
        ))
        .id();
    commands.spawn((
        ChildOf(panel),
        Pickable::IGNORE,
        theme::heading(fonts, &texts.get("map.legend.title"), 12.0, theme::SCREEN),
        Node {
            margin: UiRect::bottom(px(4)),
            ..default()
        },
    ));
    for (mark, key) in rows {
        let row = commands
            .spawn((
                ChildOf(panel),
                Pickable::IGNORE,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    ..default()
                },
            ))
            .id();
        let slot = commands
            .spawn((
                ChildOf(row),
                Pickable::IGNORE,
                Node {
                    width: px(20),
                    height: px(18),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: px(2),
                    ..default()
                },
            ))
            .id();
        match mark {
            Mark::Icon(path, color) => {
                commands.spawn((
                    ChildOf(slot),
                    Pickable::IGNORE,
                    ImageNode::new(asset_server.load(path)).with_color(color),
                    Node {
                        width: px(18),
                        height: px(18),
                        ..default()
                    },
                ));
            }
            Mark::Diamond => {
                commands.spawn((
                    ChildOf(slot),
                    Pickable::IGNORE,
                    Node {
                        width: px(11),
                        height: px(11),
                        border: UiRect::all(px(2)),
                        ..default()
                    },
                    BorderColor::all(theme::ACCENT),
                    UiTransform::from_rotation(Rot2::degrees(45.0)),
                ));
            }
            // Un trait plein (Portail) ou trois tirets (hyperespace), comme sur la carte.
            Mark::Line(color, dashed) => {
                let (count, width) = if dashed { (3, 4.0) } else { (1, 20.0) };
                for _ in 0..count {
                    commands.spawn((
                        ChildOf(slot),
                        Pickable::IGNORE,
                        Node {
                            width: px(width),
                            height: px(3),
                            ..default()
                        },
                        BackgroundColor(color),
                    ));
                }
            }
        }
        commands.spawn((
            ChildOf(row),
            Pickable::IGNORE,
            Text::new(texts.get(key)),
            fonts.body(13.0),
            TextColor(theme::TEXT_DIM),
        ));
    }
}

fn pan_and_zoom(
    time: Res<Time>,
    pan: Single<&Action<Pan>>,
    zoom: Single<&Action<Zoom>>,
    mut camera: Single<&mut Transform, With<WorldCamera>>,
) {
    let height = camera.translation.y;
    // Vitesse proportionnelle à l'altitude : la carte défile à la même vitesse apparente.
    let speed = height * 0.9;
    let pan: Vec2 = ***pan;
    if pan != Vec2::ZERO {
        let x = camera.translation.x + pan.x * speed * time.delta_secs();
        let z = camera.translation.z - pan.y * speed * time.delta_secs();
        camera.translation.x = x.clamp(-PAN_LIMIT.x, PAN_LIMIT.x);
        camera.translation.z = z.clamp(-PAN_LIMIT.y, PAN_LIMIT.y);
    }
    let zoom: f32 = ***zoom;
    if zoom != 0.0 {
        camera.translation.y =
            (height * (1.0 - 0.12 * zoom.clamp(-3.0, 3.0))).clamp(MIN_HEIGHT, MAX_HEIGHT);
    }
}

/// Projette les repères de la carte à l'écran. La caméra vient peut-être de bouger
/// (`pan_and_zoom`) : on part de son `Transform` de ce frame, pas du `GlobalTransform` du
/// frame précédent, pour que les icônes ne flottent pas au-dessus du fond pendant un
/// défilement.
#[allow(clippy::too_many_arguments)]
fn project_map(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &Transform), With<WorldCamera>>,
    data: Res<Data>,
    entries: Res<MapEntries>,
    fleets: Res<FleetMarkers>,
    zones: Res<ZoneLabels>,
    (location, route, details, career, target): (
        Option<Res<Location>>,
        Option<Res<RoutePlan>>,
        Res<SystemDetails>,
        Option<Res<Career>>,
        Res<MapTarget>,
    ),
    mut screen: ResMut<MapScreen>,
) {
    let (camera, transform) = *camera;
    let view = GlobalTransform::from(*transform);
    let size = window.size();
    // Avec l'échelle de rendu, la caméra dessine dans une image plus petite que la fenêtre.
    let to_window = camera
        .logical_viewport_size()
        .map_or(Vec2::ONE, |s| size / s.max(Vec2::ONE));
    let project = |p: Vec3| {
        camera
            .world_to_viewport(&view, p)
            .ok()
            .map(|v| v * to_window)
    };
    let on_screen = |p: Vec2, margin: f32| {
        p.x >= -margin && p.y >= -margin && p.x <= size.x + margin && p.y <= size.y + margin
    };
    let screen = &mut *screen;
    screen.window = size;
    screen.scale = match (project(Vec3::ZERO), project(Vec3::X * 100.0)) {
        (Some(a), Some(b)) => a.distance(b) / 100.0,
        _ => 0.0,
    };
    let list = &entries.entries;
    screen.entries = list
        .iter()
        .map(|e| project(world(e.pos)).filter(|p| on_screen(*p, 60.0)))
        .collect();
    // Les icônes trop serrées s'éclaircissent, les plus importantes d'abord.
    let mut markers = Vec::new();
    let mut indices = Vec::new();
    for &i in &entries.order {
        let icon = icon_size(&list[i]);
        if let (Some(p), true) = (screen.entries[i], icon > 0.0) {
            markers.push((p.x, p.y, icon / 2.0));
            indices.push(i);
        }
    }
    screen.shown = vec![false; list.len()];
    for (i, keep) in indices.into_iter().zip(thin_markers(&markers)) {
        screen.shown[i] = keep;
    }
    // Une nébuleuse n'a pas d'icône : son nom se pose au nord de sa nappe.
    for (i, e) in list.iter().enumerate() {
        if let (MapKind::Phenomenon, Some(radius)) = (e.kind, e.radius) {
            let north = MapPos::new(e.pos.x, e.pos.y - radius * 0.95);
            screen.entries[i] = project(world(north)).filter(|p| on_screen(*p, 60.0));
            screen.shown[i] = screen.entries[i].is_some();
        }
    }
    screen.fleets = fleets
        .0
        .iter()
        .map(|f| {
            let center = screen.entries.get(f.anchor).copied().flatten()?;
            // En couronne, à partir du haut, dans le sens des aiguilles d'une montre.
            let radius = icon_size(&list[f.anchor]).max(14.0) / 2.0 + 11.0;
            let angle = -std::f32::consts::FRAC_PI_2 + 0.68 * f.slot as f32;
            Some(center + Vec2::new(angle.cos(), angle.sin()) * radius)
        })
        .collect();
    screen.zones = zones
        .0
        .iter()
        .map(|z| project(world(z.anchor)).filter(|p| on_screen(*p, 0.0)))
        .collect();
    let origin = project(world(MapPos::new(0.0, 0.0)));
    screen.grid_top = origin.map_or(0.0, |p| p.y);
    screen.grid_left = origin.map_or(0.0, |p| p.x);
    screen.columns = (0..usize::from(COLUMNS))
        .map(|c| {
            let x = (c as f64 + 0.5) * CELL_SIZE;
            project(world(MapPos::new(x, 0.0)))
                .map(|p| p.x)
                .filter(|x| (8.0..=size.x - 8.0).contains(x))
        })
        .collect();
    screen.rows = (0..usize::from(ROWS))
        .map(|r| {
            let y = (r as f64 + 0.5) * CELL_SIZE;
            project(world(MapPos::new(0.0, y)))
                .map(|p| p.y)
                .filter(|y| (8.0..=size.y - 8.0).contains(y))
        })
        .collect();
    let place = |id: &sh_core::Id<Place>| {
        endpoint(&data.galaxy, id)
            .and_then(|e| project(world(e.pos())))
            .filter(|p| on_screen(*p, 40.0))
    };
    screen.here = location.as_ref().and_then(|l| place(&l.system));
    screen.dest = route.as_ref().and_then(|r| place(&r.dest));

    // Le détail des systèmes, en fondu selon la taille à l'écran du disque d'un système.
    screen.detail = detail_fade(DETAIL_RADIUS as f32 * screen.scale);
    let detail = screen.detail;
    screen.details = details
        .points
        .iter()
        .map(|m| {
            (detail > 0.0)
                .then(|| project(world(m.pos)))
                .flatten()
                .filter(|p| on_screen(*p, 40.0))
        })
        .collect();
    let mut markers = Vec::new();
    let mut indices = Vec::new();
    for &k in &details.order {
        if let Some(p) = screen.details[k] {
            let (_, size, _) = detail_icon(details.points[k].point.kind);
            markers.push((p.x, p.y, size / 2.0));
            indices.push(k);
        }
    }
    screen.detail_shown = vec![false; details.points.len()];
    for (k, keep) in indices.into_iter().zip(thin_markers(&markers)) {
        screen.detail_shown[k] = keep;
    }
    // Au plus près, le repère du vaisseau glisse du centre du système vers la cible la plus
    // proche de lui.
    if let (Some(center), Some(k)) = (screen.here, details.here)
        && let Some(near) = screen.details.get(k).copied().flatten()
    {
        screen.here = Some(center.lerp(near, detail));
    }
    // La cible proposée à l'autopilote porte le repère de la destination : un point du détail,
    // ou l'entrée d'un monde isolé.
    if let Some(choice) = &target.0 {
        let point = details
            .points
            .iter()
            .position(|m| m.owner == choice.owner && m.point.id.as_deref() == Some(&choice.id))
            .and_then(|k| screen.details.get(k).copied().flatten());
        let entry = list
            .iter()
            .position(|e| e.id.as_str() == choice.id)
            .and_then(|i| screen.entries.get(i).copied().flatten());
        screen.dest = point.or(entry);
    }

    // Les destinations des contrats en cours.
    screen.contract_entries = vec![false; list.len()];
    screen.contract_points = vec![false; details.points.len()];
    let active = career
        .as_ref()
        .map(|c| {
            c.captain
                .contracts
                .iter()
                .filter(|c| c.state == ContractState::Active)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for contract in active {
        if let Some(i) = entry_for_place(&data.galaxy, list, &contract.to) {
            screen.contract_entries[i] = true;
            for (k, m) in details.points.iter().enumerate() {
                if m.entry == i && m.point.delivery {
                    screen.contract_points[k] = true;
                }
            }
        }
    }
}

/// L'entrée, la flotte ou la cible du détail sous le curseur, à l'écran (à quelques pixels de
/// son icône). Rien au-dessus d'un panneau de la carte (un clic y vise le panneau).
#[allow(clippy::too_many_arguments)]
fn hover(
    window: Single<&Window, With<PrimaryWindow>>,
    screen: Res<MapScreen>,
    entries: Res<MapEntries>,
    fleets: Res<FleetMarkers>,
    (forced, forced_target, details, location): (
        Res<ForcedHover>,
        Res<ForcedTarget>,
        Res<SystemDetails>,
        Option<Res<Location>>,
    ),
    panels: Query<(&ComputedNode, &UiGlobalTransform), With<MapPanel>>,
    mut hovered: ResMut<HoveredEntry>,
    mut hovered_fleet: ResMut<HoveredFleet>,
    mut hovered_target: ResMut<HoveredTarget>,
) {
    let here = location.as_ref().map(|l| &l.system);
    let choice_for = |k: usize| {
        let m = &details.points[k];
        let id = m.point.id.clone()?;
        let destination = entries.entries.get(m.entry).is_some_and(|e| e.destination);
        Some(NavChoice {
            name: m.point.name.clone().unwrap_or_else(|| id.clone()),
            id,
            owner: m.owner.clone(),
            click: target_click(here, &m.owner, destination),
        })
    };
    if forced.0.is_some() {
        return;
    }
    if let Some(k) = forced_target.0 {
        let choice = choice_for(k);
        if hovered_target.0 != choice {
            hovered_target.0 = choice;
        }
        return;
    }
    let over_panel = |cursor: Vec2| {
        panels.iter().any(|(computed, transform)| {
            let scale = computed.inverse_scale_factor();
            let half = computed.size() * scale / 2.0;
            let center = transform.translation * scale;
            computed.size().x > 0.0
                && (cursor.x - center.x).abs() <= half.x
                && (cursor.y - center.y).abs() <= half.y
        })
    };
    let (entry, fleet, target) = match window.cursor_position() {
        Some(cursor) if !over_panel(cursor) => {
            let list = &entries.entries;
            let detail_on = screen.detail >= DETAIL_HOVER_MIN;
            let nearest_entry = list
                .iter()
                .enumerate()
                .filter(|(i, _)| screen.shown.get(*i).copied().unwrap_or(false))
                .filter_map(|(i, e)| {
                    if let (MapKind::Phenomenon, Some(radius)) = (e.kind, e.radius) {
                        // Une nébuleuse se survole sur toute sa nappe, après les icônes.
                        let center = screen.entries.get(i).copied().flatten()?;
                        let r = radius as f32 * screen.scale;
                        let center = center + Vec2::Y * r * 0.95;
                        return (cursor.distance(center) <= r).then_some((20.0, e));
                    }
                    let p = screen.entries.get(i).copied().flatten()?;
                    let d = cursor.distance(p);
                    (d <= icon_size(e) / 2.0 + 5.0).then_some((d, e))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            let nearest_fleet = fleets
                .0
                .iter()
                .zip(&screen.fleets)
                .filter_map(|(f, p)| {
                    let d = cursor.distance((*p)?);
                    (d <= 11.0).then_some((d, f))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            // Au plus près, les cibles du détail passent avant l'icône du système (effacée) ;
            // l'étoile sans nom, qui n'est pas une cible, laisse voir le système.
            let nearest_detail = details
                .points
                .iter()
                .enumerate()
                .filter(|_| detail_on)
                .filter(|(k, m)| {
                    m.point.is_target() && screen.detail_shown.get(*k).copied().unwrap_or(false)
                })
                .filter_map(|(k, m)| {
                    let p = screen.details.get(k).copied().flatten()?;
                    let d = cursor.distance(p);
                    let (_, size, _) = detail_icon(m.point.kind);
                    (d <= size / 2.0 + 5.0).then_some((d, k))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            match (nearest_detail, nearest_entry, nearest_fleet) {
                (Some(d), e, _) if e.is_none_or(|e| d.0 <= e.0) => (None, None, choice_for(d.1)),
                (_, Some(e), Some(f)) if f.0 < e.0 => (None, Some(f.1.clone()), None),
                (_, None, Some(f)) => (None, Some(f.1.clone()), None),
                (_, e, _) => (e.map(|e| e.1.clone()), None, None),
            }
        }
        _ => (None, None, None),
    };
    // Une entrée de la carte qui est une cible du lieu courant (un monde isolé et ses lunes) :
    // un clic la propose à l'autopilote.
    let target = target.or_else(|| {
        let entry = entry.as_ref()?;
        let point = details.local.iter().find(|p| p.id == entry.id.as_str())?;
        Some(NavChoice {
            id: point.id.clone(),
            name: point.name.clone(),
            owner: here?.clone(),
            click: TargetClick::Autopilot,
        })
    });
    if hovered.0 != entry {
        hovered.0 = entry;
    }
    if hovered_fleet.0 != fleet {
        hovered_fleet.0 = fleet;
    }
    if hovered_target.0 != target {
        hovered_target.0 = target;
    }
}

/// Pose un nœud à `(left, top)` seulement s'il a bougé : un nœud modifié est remis en page (la
/// lecture passe par `Deref`, seule l'écriture marque le changement).
fn set_position(node: &mut Mut<Node>, left: f32, top: f32) {
    let (left, top) = (px(left.round()), px(top.round()));
    if node.left != left || node.top != top {
        node.left = left;
        node.top = top;
    }
}

fn set_visible(visibility: &mut Mut<Visibility>, visible: bool) {
    visibility.set_if_neq(if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
}

/// Place les icônes sur leurs repères ; la position du joueur bat doucement, la destination
/// tourne lentement, les losanges des contrats battent. Au zoom, l'icône d'un système s'efface
/// pendant que son détail apparaît (même fondu, `MapScreen::detail`).
#[allow(clippy::type_complexity)]
fn place_markers(
    time: Res<Time>,
    screen: Res<MapScreen>,
    entries: Res<MapEntries>,
    mut icons: Query<(
        &MapIcon,
        &BaseColor,
        &ComputedNode,
        &mut Node,
        &mut Visibility,
        &mut UiTransform,
        Option<&mut ImageNode>,
        Option<&mut BorderColor>,
    )>,
) {
    let t = time.elapsed_secs();
    let detail = screen.detail;
    let galaxy = 1.0 - detail;
    let is_system = |i: usize| {
        entries
            .entries
            .get(i)
            .is_some_and(|e| e.kind == MapKind::System)
    };
    let pulse = 0.75 + 0.25 * (t * 2.4).sin();
    for (icon, base, computed, mut node, mut visibility, mut transform, image, border) in &mut icons
    {
        let size = computed.size() * computed.inverse_scale_factor();
        let entry_at = |i: usize| {
            screen
                .entries
                .get(i)
                .copied()
                .flatten()
                .filter(|_| screen.shown.get(i).copied().unwrap_or(false))
        };
        let (at, alpha) = match *icon {
            MapIcon::Entry(i) => (entry_at(i), if is_system(i) { galaxy } else { 1.0 }),
            // Le badge du hub, au coin haut-droit de l'icône du système.
            MapIcon::Hub(i) => (
                entry_at(i).map(|p| {
                    let r = entries.entries.get(i).map_or(14.0, icon_size) / 2.0;
                    p + Vec2::new(r * 0.8, -r * 0.8)
                }),
                if is_system(i) { galaxy } else { 1.0 },
            ),
            MapIcon::Fleet(i) => (screen.fleets.get(i).copied().flatten(), 1.0),
            MapIcon::Here => (screen.here, 1.0),
            MapIcon::Destination => (screen.dest, 1.0),
            MapIcon::Detail(k) => (
                screen
                    .details
                    .get(k)
                    .copied()
                    .flatten()
                    .filter(|_| screen.detail_shown.get(k).copied().unwrap_or(false)),
                detail,
            ),
            MapIcon::ContractEntry(i) => (
                entry_at(i).filter(|_| screen.contract_entries.get(i).copied().unwrap_or(false)),
                galaxy * pulse,
            ),
            MapIcon::ContractPoint(k) => (
                screen
                    .details
                    .get(k)
                    .copied()
                    .flatten()
                    .filter(|_| screen.contract_points.get(k).copied().unwrap_or(false)),
                detail * pulse,
            ),
        };
        let at = at.filter(|_| alpha > 0.02);
        set_visible(&mut visibility, at.is_some());
        let Some(at) = at else { continue };
        set_position(&mut node, at.x - size.x / 2.0, at.y - size.y / 2.0);
        let color = base.0.with_alpha(base.0.alpha() * alpha.min(1.0));
        if let Some(mut image) = image
            && image.color != color
        {
            image.color = color;
        }
        if let Some(mut border) = border
            && border.top != color
        {
            *border = BorderColor::all(color);
        }
        transform.set_if_neq(match *icon {
            MapIcon::Here => UiTransform {
                scale: Vec2::splat(1.0 + 0.08 * (t * 3.0).sin()),
                ..UiTransform::IDENTITY
            },
            MapIcon::Destination => UiTransform::from_rotation(Rot2::radians(0.35 * t)),
            MapIcon::ContractEntry(_) | MapIcon::ContractPoint(_) => {
                UiTransform::from_rotation(Rot2::degrees(45.0))
            }
            _ => UiTransform::IDENTITY,
        });
    }
}

/// Place les étiquettes sans chevauchement (`sh_data::map::layout_labels`), les plus
/// importantes d'abord : la position du joueur, la destination, l'entrée survolée, puis l'ordre
/// d'importance, puis les noms des zones. Les repères de la grille collent aux bords.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn place_map_labels(
    screen: Res<MapScreen>,
    entries: Res<MapEntries>,
    hovered: Res<HoveredEntry>,
    (hovered_target, details): (Res<HoveredTarget>, Res<SystemDetails>),
    location: Option<Res<Location>>,
    route: Option<Res<RoutePlan>>,
    mut labels: Query<(
        Entity,
        &MapLabel,
        &Text,
        &ComputedNode,
        &mut Node,
        &mut Visibility,
        &BaseColor,
        &mut TextColor,
    )>,
    panels: Query<(&ComputedNode, &UiGlobalTransform), (With<MapPanel>, Without<MapLabel>)>,
) {
    let list = &entries.entries;
    let size_of = |text: &Text, label: &MapLabel, computed: &ComputedNode| {
        let size = computed.size() * computed.inverse_scale_factor();
        if size.x > 0.0 {
            size
        } else {
            // Avant la première mise en page : une estimation suffit pour un frame.
            Vec2::new(
                text.0.chars().count() as f32 * label.font_size * 0.56,
                label.font_size * 1.25,
            )
        }
    };
    let mut entry_labels: Vec<Option<(Entity, Vec2)>> = vec![None; list.len()];
    let mut zone_labels: Vec<Option<(Entity, Vec2)>> = vec![None; screen.zones.len()];
    let mut detail_labels: Vec<Option<(Entity, Vec2)>> = vec![None; details.points.len()];
    for (entity, label, text, computed, mut node, mut visibility, _, _) in &mut labels {
        let size = size_of(text, label, computed);
        match label.target {
            LabelTarget::Detail(k) => {
                if let Some(slot) = detail_labels.get_mut(k) {
                    *slot = Some((entity, size));
                }
            }
            LabelTarget::Entry(i) => {
                if let Some(slot) = entry_labels.get_mut(i) {
                    *slot = Some((entity, size));
                }
            }
            LabelTarget::Zone(i) => {
                if let Some(slot) = zone_labels.get_mut(i) {
                    *slot = Some((entity, size));
                }
            }
            // Lettres en haut, chiffres à gauche : sur le bord de la grille quand il est à
            // l'écran, sinon collés au bord de la fenêtre.
            LabelTarget::Column(c) => {
                let x = screen.columns.get(c).copied().flatten();
                set_visible(&mut visibility, x.is_some());
                if let Some(x) = x {
                    let top = (screen.grid_top - size.y - 4.0).max(4.0);
                    set_position(&mut node, x - size.x / 2.0, top);
                }
            }
            LabelTarget::Row(r) => {
                let y = screen.rows.get(r).copied().flatten();
                set_visible(&mut visibility, y.is_some());
                if let Some(y) = y {
                    let left = (screen.grid_left - size.x - 6.0).max(6.0);
                    set_position(&mut node, left, y - size.y / 2.0);
                }
            }
        }
    }

    // Obstacles : les icônes montrées, les flottes, les repères du joueur, les panneaux.
    let mut obstacles: Vec<ScreenRect> = Vec::new();
    for (i, e) in list.iter().enumerate() {
        if let (Some(p), true, Some(_)) = (
            screen.entries.get(i).copied().flatten(),
            screen.shown.get(i).copied().unwrap_or(false),
            entry_icon(e),
        ) {
            obstacles.push(ScreenRect::around(p.x, p.y, icon_size(e) / 2.0));
        }
    }
    obstacles.extend(
        screen
            .fleets
            .iter()
            .flatten()
            .map(|p| ScreenRect::around(p.x, p.y, 8.0)),
    );
    if screen.detail > 0.02 {
        for (k, m) in details.points.iter().enumerate() {
            if let (Some(p), true) = (
                screen.details.get(k).copied().flatten(),
                screen.detail_shown.get(k).copied().unwrap_or(false),
            ) {
                let (_, size, _) = detail_icon(m.point.kind);
                obstacles.push(ScreenRect::around(p.x, p.y, size / 2.0));
            }
        }
    }
    obstacles.extend(
        [screen.here, screen.dest]
            .into_iter()
            .flatten()
            .map(|p| ScreenRect::around(p.x, p.y, 6.0)),
    );
    for (computed, transform) in &panels {
        let scale = computed.inverse_scale_factor();
        let size = computed.size() * scale;
        let center = transform.translation * scale;
        obstacles.push(ScreenRect::new(
            center.x - size.x / 2.0,
            center.y - size.y / 2.0,
            size.x,
            size.y,
        ));
    }

    // L'ordre de placement : la position du joueur, la destination, ce qui est survolé, puis
    // au plus près le détail des systèmes, puis le reste de la carte.
    let find = |id: &sh_core::Id<Place>| list.iter().position(|e| &e.id == id);
    let mut requests = Vec::new();
    let mut owners = Vec::new();
    let mut done_entries = vec![false; list.len()];
    let mut push_entry = |i: usize, requests: &mut Vec<LabelRequest>, owners: &mut Vec<Entity>| {
        if done_entries.get(i).copied().unwrap_or(true) {
            return;
        }
        done_entries[i] = true;
        let (Some(anchor), true, Some((entity, size))) = (
            screen.entries.get(i).copied().flatten(),
            screen.shown.get(i).copied().unwrap_or(false),
            entry_labels.get(i).copied().flatten(),
        ) else {
            return;
        };
        let clearance = if list[i].kind == MapKind::Phenomenon && list[i].radius.is_some() {
            0.0
        } else if list[i].kind == MapKind::System {
            // L'icône du système s'efface au profit de l'étoile de son détail (plus petite) ;
            // le losange d'une destination de contrat déborde de l'icône.
            let icon = if screen.contract_entries.get(i).copied().unwrap_or(false) {
                21.0
            } else {
                icon_size(&list[i]) / 2.0
            };
            icon * (1.0 - screen.detail) + 13.0 * screen.detail
        } else {
            icon_size(&list[i]) / 2.0
        };
        requests.push(LabelRequest {
            anchor: (anchor.x, anchor.y),
            size: (size.x, size.y),
            clearance,
            centered: clearance == 0.0,
        });
        owners.push(entity);
    };
    for i in [
        location.as_ref().and_then(|l| find(&l.system)),
        route.as_ref().and_then(|r| find(&r.dest)),
        hovered.0.as_ref().and_then(|h| find(&h.id)),
    ]
    .into_iter()
    .flatten()
    {
        push_entry(i, &mut requests, &mut owners);
    }
    if screen.detail >= DETAIL_LABEL_MIN {
        let hovered_detail = hovered_target.0.as_ref().and_then(|t| {
            details
                .points
                .iter()
                .position(|m| m.owner == t.owner && m.point.id.as_deref() == Some(t.id.as_str()))
        });
        let mut done = vec![false; details.points.len()];
        for k in hovered_detail
            .into_iter()
            .chain(details.order.iter().copied())
        {
            if std::mem::replace(&mut done[k], true) {
                continue;
            }
            let (Some(anchor), true, Some((entity, size))) = (
                screen.details.get(k).copied().flatten(),
                screen.detail_shown.get(k).copied().unwrap_or(false),
                detail_labels.get(k).copied().flatten(),
            ) else {
                continue;
            };
            let (_, icon, _) = detail_icon(details.points[k].point.kind);
            // Le losange d'une destination de contrat déborde de l'icône.
            let clearance = if screen.contract_points.get(k).copied().unwrap_or(false) {
                (icon / 2.0).max(15.0)
            } else {
                icon / 2.0
            };
            requests.push(LabelRequest {
                anchor: (anchor.x, anchor.y),
                size: (size.x, size.y),
                clearance,
                centered: false,
            });
            owners.push(entity);
        }
    }
    for &i in &entries.order {
        push_entry(i, &mut requests, &mut owners);
    }
    if screen.scale < ZONE_LABEL_MAX_SCALE {
        for (anchor, label) in screen.zones.iter().zip(&zone_labels) {
            if let (Some(anchor), Some((entity, size))) = (anchor, label) {
                requests.push(LabelRequest {
                    anchor: (anchor.x, anchor.y),
                    size: (size.x, size.y),
                    clearance: 0.0,
                    centered: true,
                });
                owners.push(*entity);
            }
        }
    }
    let bounds = ScreenRect::new(2.0, 2.0, screen.window.x - 4.0, screen.window.y - 4.0);
    let placed = layout_labels(&requests, &obstacles, bounds);
    let mut positions: Vec<(Entity, (f32, f32))> = Vec::new();
    for (entity, at) in owners.into_iter().zip(placed) {
        if let Some(at) = at {
            positions.push((entity, at));
        }
    }
    for (entity, label, _, _, mut node, mut visibility, base, mut color) in &mut labels {
        if matches!(label.target, LabelTarget::Column(_) | LabelTarget::Row(_)) {
            continue;
        }
        match positions.iter().find(|(e, _)| *e == entity) {
            Some((_, (x, y))) => {
                set_position(&mut node, *x, *y);
                set_visible(&mut visibility, true);
            }
            None => set_visible(&mut visibility, false),
        }
        // Les noms du détail apparaissent avec lui.
        if matches!(label.target, LabelTarget::Detail(_)) {
            let wanted = base.0.with_alpha(base.0.alpha() * screen.detail);
            if color.0 != wanted {
                color.0 = wanted;
            }
        }
    }
}

/// Remplace un texte seulement s'il a changé : un texte réécrit est remis en page.
fn set_text(text: &mut Text, wanted: String) {
    if text.0 != wanted {
        text.0 = wanted;
    }
}

/// Les lignes d'une infobulle de lieu : le nom en titre, puis les champs publics.
fn entry_lines(
    data: &Data,
    texts: &Texts,
    entry: &MapEntry,
    here: Option<&sh_core::Id<Place>>,
) -> Vec<String> {
    let kind = match entry.kind {
        MapKind::System => texts.get("map.tip.kind.system"),
        MapKind::Body => match entry.parent.as_ref() {
            Some(parent) => texts.format(
                "map.tip.kind.moon",
                &[(
                    "parent",
                    data.galaxy.place_name(parent).unwrap_or(parent.as_str()),
                )],
            ),
            None if entry.rogue => texts.get("map.tip.kind.rogue"),
            None => texts.get("map.tip.kind.body"),
        },
        MapKind::Station => texts.get("map.tip.kind.station"),
        MapKind::Phenomenon => texts.get("map.tip.kind.phenomenon"),
    };
    let mut lines = vec![entry.name.clone(), kind];
    if let Some(cell) = entry.cell {
        lines.push(texts.format("map.tip.cell", &[("cell", &cell.to_string())]));
    }
    match entry.controller.as_ref().and_then(|f| data.faction(f)) {
        Some(faction) => {
            lines.push(texts.format("map.tip.controller", &[("faction", &faction.name)]));
        }
        None if entry.kind != MapKind::Phenomenon => lines.push(texts.get("map.tip.no_controller")),
        None => {}
    }
    if entry.population > 0 {
        let value = population_text(texts, entry.population);
        lines.push(texts.format("map.tip.population", &[("value", &value)]));
    }
    if entry.has_portal_hub {
        lines.push(texts.get("map.tip.hub"));
    }
    if entry.destroyed {
        lines.push(texts.get("map.tip.destroyed"));
    }
    if entry.blocks_hyperspace {
        lines.push(texts.get("map.tip.blocks_hyperspace"));
    }
    if entry.kind == MapKind::Body && entry.destination {
        lines.push(texts.get("map.tip.orbit_only"));
    }
    if here == Some(&entry.id) {
        lines.push(texts.get("map.tip.here"));
    } else if entry.destination {
        lines.push(texts.get("map.tip.click"));
    }
    lines
}

/// Les lignes d'une infobulle de cible du détail : le nom, son genre et son système, puis si
/// c'est un point de livraison (et s'il est attendu par un contrat), et ce que fait un clic.
fn target_lines(
    data: &Data,
    texts: &Texts,
    details: &SystemDetails,
    career: Option<&Career>,
    target: &NavChoice,
) -> Vec<String> {
    let marker = details
        .points
        .iter()
        .find(|m| m.owner == target.owner && m.point.id.as_deref() == Some(target.id.as_str()));
    let owner = data
        .galaxy
        .place_name(&target.owner)
        .map_or_else(|| target.owner.to_string(), str::to_owned);
    let mut lines = vec![target.name.clone()];
    if let Some(marker) = marker {
        lines.push(texts.format(
            "map.detail.kind_in",
            &[
                ("kind", &texts.get(detail_kind_key(marker.point.kind))),
                ("place", &owner),
            ],
        ));
        if marker.point.delivery {
            lines.push(texts.get("map.detail.delivery"));
            let expected = career.is_some_and(|c| {
                c.captain.contracts.iter().any(|c| {
                    c.state == ContractState::Active
                        && system_of(&data.galaxy, &c.to) == target.owner
                })
            });
            if expected {
                lines.push(texts.get("map.detail.contract"));
            }
        }
    }
    lines.push(match target.click {
        TargetClick::Autopilot => texts.get("map.detail.click_autopilot"),
        TargetClick::Route => texts.format("map.detail.click_route", &[("place", &owner)]),
        TargetClick::Nothing => texts.get("map.detail.elsewhere"),
    });
    lines
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn show_tooltip(
    data: Res<Data>,
    texts: Res<Texts>,
    hovered: Res<HoveredEntry>,
    hovered_fleet: Res<HoveredFleet>,
    forced: Res<ForcedHover>,
    (screen, entries, location): (Res<MapScreen>, Res<MapEntries>, Option<Res<Location>>),
    (hovered_target, details, forced_target, career): (
        Res<HoveredTarget>,
        Res<SystemDetails>,
        Res<ForcedTarget>,
        Option<Res<Career>>,
    ),
    window: Single<&Window, With<PrimaryWindow>>,
    mut tooltip: Single<(&mut Node, &mut Visibility, &ComputedNode), With<Tooltip>>,
    mut title: Single<&mut Text, (With<TooltipTitle>, Without<TooltipBody>)>,
    mut body: Single<&mut Text, (With<TooltipBody>, Without<TooltipTitle>)>,
) {
    let (node, visibility, computed) = &mut *tooltip;
    let size = computed.size() * computed.inverse_scale_factor();
    // Le nom en titre, les autres lignes dessous ; sans entrée survolée, rien.
    let lines = if let Some(fleet) = &hovered_fleet.0 {
        let mut lines = vec![fleet.name.clone(), texts.get("map.tip.kind.fleet")];
        if let Some(faction) = fleet.faction.as_ref().and_then(|f| data.faction(f)) {
            lines.push(texts.format("map.tip.controller", &[("faction", &faction.name)]));
        }
        lines.push(texts.format(
            "map.tip.reported",
            &[
                ("date", &crate::clock::format_date(&texts, fleet.as_of)),
                ("place", &crate::living::display_name(&data, &fleet.at)),
            ],
        ));
        lines.push(texts.format("map.tip.age", &[("days", &fleet.age_days.to_string())]));
        lines
    } else if let Some(entry) = &hovered.0 {
        let mut lines = entry_lines(&data, &texts, entry, location.as_ref().map(|l| &l.system));
        // Un monde isolé où l'on est (ou l'une de ses lunes) : un clic le propose à
        // l'autopilote.
        if hovered_target
            .0
            .as_ref()
            .is_some_and(|t| t.click == TargetClick::Autopilot)
        {
            lines.push(texts.get("map.detail.click_autopilot"));
        }
        lines
    } else if let Some(target) = &hovered_target.0 {
        target_lines(&data, &texts, &details, career.as_deref(), target)
    } else {
        set_visible(visibility, false);
        return;
    };
    // Sous le curseur ; survol imposé par l'autopilote : à côté de l'icône survolée.
    let forced_at = forced
        .0
        .as_ref()
        .and_then(|f| {
            let i = entries.entries.iter().position(|e| e.id == f.id)?;
            screen.entries.get(i).copied().flatten()
        })
        .or_else(|| screen.details.get(forced_target.0?).copied().flatten());
    let cursor = forced_at
        .or_else(|| window.cursor_position())
        .unwrap_or(window.size() / 2.0);
    // À droite et sous le curseur ; près d'un bord de la fenêtre, de l'autre côté.
    let window_size = window.size();
    let left = if cursor.x + 18.0 + size.x > window_size.x - 8.0 {
        cursor.x - 18.0 - size.x
    } else {
        cursor.x + 18.0
    };
    let top = if cursor.y + 14.0 + size.y > window_size.y - 8.0 {
        cursor.y - 14.0 - size.y
    } else {
        cursor.y + 14.0
    };
    set_position(node, left.max(8.0), top.max(8.0));
    let mut lines = lines.into_iter();
    set_text(&mut title, lines.next().unwrap_or_default());
    set_text(&mut body, lines.collect::<Vec<_>>().join("\n"));
    set_visible(visibility, true);
}

/// Le panneau d'itinéraire : la position et le carburant, puis la route choisie et ses deux
/// façons d'y aller, ou la cible proposée à l'autopilote et son bouton. Les touches ne s'y
/// écrivent pas : l'encart des touches les donne (`update_map_keys`).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn show_route(
    data: Res<Data>,
    texts: Res<Texts>,
    route: Option<Res<RoutePlan>>,
    fuel: Option<Res<ShipFuel>>,
    location: Option<Res<Location>>,
    (target, details, resume): (
        Res<MapTarget>,
        Res<SystemDetails>,
        Option<Res<ResumeFlight>>,
    ),
    mut lines: Query<(&RouteLine, &mut Text, &mut TextColor, &mut Node), Without<AutopilotButton>>,
    mut button: Query<&mut Node, (With<AutopilotButton>, Without<RouteLine>)>,
    mut button_text: Query<&mut Text, (With<AutopilotButtonText>, Without<RouteLine>)>,
) {
    let here_id = location.as_ref().map(|l| l.system.clone());
    let here = location
        .and_then(|l| l.endpoint(&data))
        .map_or_else(String::new, |e| e.name().to_owned());
    let have = fuel.as_ref().map_or(0.0, |f| f.units);
    let status = [
        texts.format("map.here", &[("place", &here)]),
        texts.format("map.fuel", &[("fuel", &format!("{have:.0}"))]),
    ]
    .join("\n");
    let choice = target
        .0
        .as_ref()
        .filter(|t| t.click == TargetClick::Autopilot);
    let (title, title_color, options) = match (choice, route) {
        (Some(choice), _) => {
            let point = details.local.iter().find(|p| p.id == choice.id);
            let mut options = Vec::new();
            if let Some(point) = point {
                let kind = texts.get(detail_kind_key(point.kind));
                options.push(if point.is_delivery() {
                    texts.format("map.target.kind_delivery", &[("kind", &kind)])
                } else {
                    kind
                });
                // La distance, quand on sait où est le vaisseau (carte ouverte en vol).
                if let Some(resume) = resume
                    .as_ref()
                    .filter(|r| here_id.as_ref() == Some(&r.system))
                {
                    options.push(distance_text(&texts, (point.pos - resume.pos).length()));
                }
            }
            (
                texts.format("map.target.title", &[("name", &choice.name)]),
                theme::ACCENT,
                options,
            )
        }
        (None, None) => (texts.get("map.plan.hint"), theme::TEXT_DIM, Vec::new()),
        (None, Some(route)) => {
            let dest = endpoint(&data.galaxy, &route.dest);
            let name = dest.map_or_else(String::new, |e| e.name().to_owned());
            let mut options = vec![match &route.portal {
                Some(p) => texts.format(
                    "map.plan.portal",
                    &[
                        ("hours", &format!("{:.0}", p.hours)),
                        ("jumps", &p.hubs.len().saturating_sub(1).to_string()),
                        ("toll", &(p.toll.cents / 100).to_string()),
                    ],
                ),
                None => texts.get("map.plan.no_portal"),
            }];
            if let Some(h) = &route.hyperspace {
                let mut line = texts.format(
                    "map.plan.hyperspace",
                    &[
                        ("days", &format!("{:.0}", h.days().ceil())),
                        ("fuel", &format!("{:.0}", h.fuel)),
                    ],
                );
                if h.detour {
                    line.push_str(&texts.get("map.plan.detour"));
                }
                if h.fuel > have {
                    line.push_str(&texts.get("map.plan.no_fuel"));
                }
                options.push(line);
            } else if route.hyperspace_blocked {
                options.push(texts.get("map.plan.blocked"));
            }
            // Un monde isolé n'a pas de spatioport (Q18) : on n'y fait qu'orbiter.
            if dest.is_some_and(|e| e.is_isolated()) {
                options.push(texts.get("map.plan.orbit"));
            }
            (
                texts.format("map.plan.title", &[("name", &name)]),
                theme::ACCENT,
                options,
            )
        }
    };
    for (line, mut text, mut color, mut node) in &mut lines {
        match line {
            RouteLine::Status => set_text(&mut text, status.clone()),
            RouteLine::Title => {
                set_text(&mut text, title.clone());
                if color.0 != title_color {
                    color.0 = title_color;
                }
            }
            RouteLine::Options => {
                // Sans route, la ligne se retire : pas de vide au bas du panneau.
                let display = if options.is_empty() {
                    Display::None
                } else {
                    Display::Flex
                };
                if node.display != display {
                    node.display = display;
                }
                set_text(&mut text, options.join("\n"));
            }
        }
    }
    let display = if choice.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut button {
        if node.display != display {
            node.display = display;
        }
    }
    if let Some(choice) = choice {
        let wanted = texts.format("map.target.engage", &[("name", &choice.name)]);
        for mut text in &mut button_text {
            set_text(&mut text, wanted.clone());
        }
    }
}

/// Le bouton « Autopilote vers … » : comme la touche du panneau (`voyage::engage_autopilot`).
fn on_autopilot_button(
    event: On<Activate>,
    buttons: Query<(), With<AutopilotButton>>,
    mut commands: Commands,
) {
    if buttons.contains(event.entity) {
        commands.queue(crate::voyage::engage_autopilot);
    }
}

/// Une ligne du panneau des contrats (comparée d'un frame à l'autre : la liste n'est refaite
/// qu'au changement).
#[derive(Clone, Debug, PartialEq)]
enum ContractLine {
    Title(String),
    Detail(String),
    Deadline(String, Color),
    More(String),
}

/// Le panneau des contrats en cours : pour chacun, son titre, son émetteur et son trajet, son
/// échéance (ambre à deux jours, rouge en retard). Caché sans contrat.
#[allow(clippy::too_many_arguments)]
fn show_contracts(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    career: Option<Res<Career>>,
    clock: Option<Res<GameClock>>,
    mut panel: Query<(Entity, &mut Node), With<ContractsPanel>>,
    mut built: Local<Option<(Entity, Vec<ContractLine>)>>,
) {
    let Ok((entity, mut node)) = panel.single_mut() else {
        return;
    };
    let mut lines = Vec::new();
    if let (Some(career), Some(clock)) = (career, clock) {
        let active: Vec<_> = career
            .captain
            .contracts
            .iter()
            .filter(|c| c.state == ContractState::Active)
            .collect();
        for c in active.iter().take(CONTRACTS_SHOWN) {
            lines.push(ContractLine::Title(contract_title(&texts, &data, c)));
            lines.push(ContractLine::Detail(texts.format(
                "map.contract.route",
                &[
                    (
                        "issuer",
                        &crate::codex::organization_name(&texts, &data, &c.issuer),
                    ),
                    ("from", &display_name(&data, c.from.as_str())),
                    ("to", &display_name(&data, c.to.as_str())),
                ],
            )));
            let days = clock.date.days_until(c.deadline);
            let (left, color) = match days {
                ..0 => (texts.get("map.contract.overdue"), theme::DANGER),
                0 => (texts.get("map.contract.today"), theme::ACCENT),
                1..=2 => (
                    texts.format("map.contract.days", &[("days", &days.to_string())]),
                    theme::ACCENT,
                ),
                _ => (
                    texts.format("map.contract.days", &[("days", &days.to_string())]),
                    theme::TEXT_DIM,
                ),
            };
            lines.push(ContractLine::Deadline(
                texts.format(
                    "map.contract.deadline",
                    &[("date", &format_date(&texts, c.deadline)), ("left", &left)],
                ),
                color,
            ));
        }
        if active.len() > CONTRACTS_SHOWN {
            lines.push(ContractLine::More(texts.format(
                "map.contract.more",
                &[("count", &(active.len() - CONTRACTS_SHOWN).to_string())],
            )));
        }
    }
    if built
        .as_ref()
        .is_some_and(|(e, l)| *e == entity && *l == lines)
    {
        return;
    }
    let display = if lines.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    if node.display != display {
        node.display = display;
    }
    // Reconstruit la liste : l'en-tête, puis les lignes ; un filet entre deux contrats.
    commands.entity(entity).despawn_children();
    if !lines.is_empty() {
        commands.spawn((
            ChildOf(entity),
            Pickable::IGNORE,
            theme::heading(
                &fonts,
                &texts.get("map.contract.header"),
                12.0,
                theme::SCREEN,
            ),
            Node {
                margin: UiRect::bottom(px(4)),
                ..default()
            },
        ));
    }
    for (n, line) in lines.iter().enumerate() {
        let (text, font, color, top) = match line {
            ContractLine::Title(t) => (
                t.clone(),
                fonts.title(14.0),
                theme::TEXT,
                if n == 0 { 0.0 } else { 7.0 },
            ),
            ContractLine::Detail(t) => (t.clone(), fonts.body(13.0), theme::TEXT_DIM, 0.0),
            ContractLine::Deadline(t, c) => (t.clone(), fonts.body(13.0), *c, 0.0),
            ContractLine::More(t) => (t.clone(), fonts.body(13.0), theme::TEXT_DIM, 6.0),
        };
        commands.spawn((
            ChildOf(entity),
            Pickable::IGNORE,
            Text::new(text),
            font,
            TextColor(color),
            // Les dates et les noms ne passent jamais à la ligne (AGENTS.md §4).
            TextLayout::no_wrap(),
            Node {
                margin: UiRect::top(px(top)),
                ..default()
            },
        ));
    }
    *built = Some((entity, lines));
}

/// L'encart des touches de la carte : déplacer, zoomer, choisir, puis selon ce qui est choisi
/// partir (Portail, hyperespace) ou engager l'autopilote, et revenir aux commandes. Réécrit
/// seulement quand il change (l'encart se reconstruit alors).
fn update_map_keys(
    texts: Res<Texts>,
    route: Option<Res<RoutePlan>>,
    target: Res<MapTarget>,
    settings: Res<UserSettings>,
    tutorial: Option<Res<Tutorial>>,
    mut help: Query<&mut KeyHelp, With<MapKeys>>,
) {
    let Ok(mut help) = help.single_mut() else {
        return;
    };
    let mut hints = vec![
        KeyHint::action::<Pan>(texts.get("map.keys.pan")),
        KeyHint::action::<Zoom>(texts.get("map.keys.zoom")),
        KeyHint::action::<Select>(texts.get("map.keys.select")),
    ];
    let autopilot = target
        .0
        .as_ref()
        .is_some_and(|t| t.click == TargetClick::Autopilot);
    if autopilot {
        hints.push(KeyHint::action::<TravelPortal>(
            texts.get("map.keys.autopilot"),
        ));
    } else if let Some(route) = route {
        if route.portal.is_some() {
            hints.push(KeyHint::action::<TravelPortal>(
                texts.get("map.keys.portal"),
            ));
        }
        if route.hyperspace.is_some() {
            hints.push(KeyHint::action::<TravelHyperspace>(
                texts.get("map.keys.hyperspace"),
            ));
        }
    }
    hints.push(KeyHint::action::<ToggleMap>(texts.get("map.keys.back")));
    hints.push(KeyHint::action::<Pause>(texts.get("map.keys.pause")));
    if crate::tutorial::is_active(&settings, tutorial.as_deref()) {
        hints.push(KeyHint::action::<SkipTutorial>(
            texts.get("map.keys.skip_tutorial"),
        ));
    }
    let wanted = KeyHelp::new(hints);
    if *help != wanted {
        *help = wanted;
    }
}

/// Les tirets de l'hyperespace défilent vers la destination.
fn animate_dashes(time: Res<Time>, mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DashedRouteGizmos>();
    config.line.animation_offset = -time.elapsed_secs() * 1.5;
}

/// Un arc de Portail entre deux hubs : une courbe qui s'écarte vers la gauche du trajet (un
/// saut, pas un trajet en ligne droite).
fn portal_arc(a: Vec3, b: Vec3) -> impl Iterator<Item = Vec3> {
    let along = b - a;
    let side = Vec3::new(along.z, 0.0, -along.x) * 0.18;
    let control = (a + b) / 2.0 + side;
    (0..=24).map(move |k| {
        let t = k as f32 / 24.0;
        a * (1.0 - t) * (1.0 - t) + control * 2.0 * t * (1.0 - t) + b * t * t
    })
}

/// L'itinéraire choisi : arcs de Portail (cyan) de hub en hub, et tracé d'hyperespace (tirets
/// ambre) contournant les nébuleuses ; un halo sous chacun.
fn draw_route(
    data: Res<Data>,
    location: Option<Res<Location>>,
    route: Option<Res<RoutePlan>>,
    mut solid: Gizmos<RouteGizmos>,
    mut dashed: Gizmos<DashedRouteGizmos>,
    mut glow: Gizmos<RouteGlowGizmos>,
) {
    let (Some(location), Some(route)) = (location, route) else {
        return;
    };
    let g = &data.galaxy;
    let (Some(from), Some(to)) = (location.endpoint(&data), endpoint(g, &route.dest)) else {
        return;
    };
    let lift = Vec3::Y * 0.4;
    if route.hyperspace.is_some() {
        let points: Vec<Vec3> =
            hyperspace_path(from.pos(), to.pos(), &blocking_nebulae(g, &data.travel))
                .into_iter()
                .map(|p| world(p) + lift)
                .collect();
        glow.linestrip(points.clone(), theme::ACCENT.with_alpha(0.14));
        dashed.linestrip(points, theme::ACCENT);
    }
    if let Some(portal) = &route.portal {
        // Départ, chaque hub (au centre de son système, ou sur son monde), arrivée.
        let hub_pos = |id: &sh_core::Id<Place>| {
            let body = g.body(id)?;
            match &body.system {
                Some(system) => g.system(system).map(|s| s.center),
                None => body.pos,
            }
        };
        let mut stops: Vec<Vec3> = vec![world(from.pos())];
        stops.extend(portal.hubs.iter().filter_map(hub_pos).map(world));
        stops.push(world(to.pos()));
        stops.dedup_by(|a, b| a.distance(*b) < 2.0);
        for pair in stops.windows(2) {
            let arc: Vec<Vec3> = portal_arc(pair[0], pair[1]).map(|p| p + lift).collect();
            glow.linestrip(arc.clone(), theme::SCREEN.with_alpha(0.14));
            solid.linestrip(arc, theme::SCREEN);
        }
    }
}

/// Les orbites du détail des systèmes, en fondu avec lui : l'orbite de chaque planète autour
/// du centre, plus pâle celle de chaque satellite autour de son astre. Une cinquantaine de
/// cercles au plus, seulement au zoom.
fn draw_orbits(
    screen: Res<MapScreen>,
    details: Res<SystemDetails>,
    mut gizmos: Gizmos<OrbitGizmos>,
) {
    let detail = screen.detail;
    if detail <= 0.02 {
        return;
    }
    let flat = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    for (k, m) in details.points.iter().enumerate() {
        // Seulement les systèmes dont un point est à l'écran.
        if screen.details.get(k).copied().flatten().is_none()
            && screen
                .details
                .get(m.point.parent.unwrap_or(k))
                .copied()
                .flatten()
                .is_none()
        {
            continue;
        }
        let center = world(m.orbit_center) + Vec3::Y * 0.05;
        let radius = (m.pos.x - m.orbit_center.x).hypot(m.pos.y - m.orbit_center.y) as f32;
        if radius < 1e-3 {
            continue;
        }
        let alpha = if m.point.parent.is_none() { 0.22 } else { 0.12 };
        gizmos
            .circle(
                Isometry3d::new(center, flat),
                radius,
                theme::SCREEN.with_alpha(alpha * detail),
            )
            .resolution(96);
    }
}

/// Autopilote (captures) : `SH_MAP_CONTRACT=<système>` accepte, dès que la partie est prête sur
/// la carte (une fois), un contrat de transport du lieu courant vers ce système (celui du bureau s'il en
/// propose un, sinon le premier transport offert, détourné), pour montrer le panneau des
/// contrats et les losanges de destination.
fn give_test_contract(
    data: Res<Data>,
    location: Option<Res<Location>>,
    clock: Option<Res<GameClock>>,
    living: Option<Res<crate::living::LivingWorld>>,
    career: Option<ResMut<Career>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Ok(value) = std::env::var("SH_MAP_CONTRACT") else {
        *done = true;
        return;
    };
    // La partie s'installe pendant les premiers frames : on attend qu'elle soit prête.
    let (Some(location), Some(clock), Some(living), Some(mut career)) =
        (location, clock, living, career)
    else {
        return;
    };
    *done = true;
    let Ok(to) = sh_core::Id::<Place>::new(value.trim()) else {
        warn!("SH_MAP_CONTRACT : identifiant invalide « {value} »");
        return;
    };
    let today = clock.date;
    let offers = sh_sim::contract_offers(&data, &living.sim, &location.system, today);
    let contract = offers.iter().find(|c| c.to == to).cloned().or_else(|| {
        offers
            .iter()
            .find(|c| c.kind != sh_sim::ContractKind::Towing)
            .cloned()
            .map(|c| sh_sim::Contract {
                id: format!("CT-{}-{to}-carte-{}", location.system, today.day_index()),
                to: to.clone(),
                deadline: today.add_days(30),
                ..c
            })
    });
    let Some(contract) = contract else {
        warn!(
            "SH_MAP_CONTRACT : aucun transport proposé à {}",
            location.system
        );
        return;
    };
    match career.captain.accept(&data, contract) {
        Ok(_) => info!("SH_MAP_CONTRACT : contrat vers {to} accepté"),
        Err(err) => warn!("SH_MAP_CONTRACT : contrat refusé ({err:?})"),
    }
}

/// Autopilote (vérification) : `SH_MAP_ENGAGE=<secondes>` engage, une fois, l'autopilote vers
/// la cible proposée (`SH_MAP_TARGET`) après ce délai sur la carte, comme la touche du
/// panneau : la carte se referme et l'autopilote vole en vue Globale.
fn engage_on_request(
    time: Res<Time>,
    target: Res<MapTarget>,
    mut commands: Commands,
    mut waited: Local<f32>,
    mut done: Local<bool>,
) {
    if *done || target.0.is_none() {
        return;
    }
    let Some(delay) = std::env::var("SH_MAP_ENGAGE")
        .ok()
        .and_then(|v| v.trim().parse::<f32>().ok())
    else {
        *done = true;
        return;
    };
    *waited += time.delta_secs();
    if *waited >= delay {
        *done = true;
        commands.queue(crate::voyage::engage_autopilot);
    }
}
