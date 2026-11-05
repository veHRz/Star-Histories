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
use bevy::window::PrimaryWindow;
use bevy_enhanced_input::prelude::*;
use sh_core::MapPos;
use sh_core::grid::{CELL_SIZE, COLUMNS, MAP_HEIGHT, MAP_WIDTH, ROWS};
use sh_data::map::{LabelRequest, ScreenRect, layout_labels, thin_markers, triangulate};
use sh_data::travel::endpoint;
use sh_data::{Faction, Galaxy, MapEntry, MapKind, Place, public_map};
use sh_sim::travel::{blocking_nebulae, hyperspace_path};

use crate::clock::GameClock;
use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::input::{Pan, Zoom};
use crate::living::PlayerKnowledge;
use crate::state::{ControlMode, ViewLevel};
use crate::ui::theme::{self, Frame, UiFonts};
use crate::views::view_banner;
use crate::voyage::{HoveredEntry, Location, RoutePlan, ShipFuel};

pub struct GalaxyViewPlugin;

impl Plugin for GalaxyViewPlugin {
    fn build(&self, app: &mut App) {
        let in_map = in_state(ViewLevel::Galaxy);
        app.init_gizmo_group::<RouteGizmos>()
            .init_gizmo_group::<DashedRouteGizmos>()
            .init_gizmo_group::<RouteGlowGizmos>()
            .init_resource::<MapScreen>()
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
                    animate_dashes.run_if(in_map.clone()),
                    draw_route.run_if(in_map),
                )
                    .chain(),
            );
    }
}

const SCOPE: DespawnOnExit<ViewLevel> = DespawnOnExit(ViewLevel::Galaxy);
const CENTER: Vec2 = Vec2::new(500.0, 330.0);
const MIN_HEIGHT: f32 = 60.0;
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

/// Les traits de l'itinéraire : Portail (trait plein), hyperespace (tirets), halo dessous.
#[derive(Default, Reflect, GizmoConfigGroup)]
struct RouteGizmos;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct DashedRouteGizmos;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct RouteGlowGizmos;

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
}

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
}

#[allow(clippy::too_many_arguments)]
fn spawn_galaxy(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    asset_server: Res<AssetServer>,
    (location, knowledge, clock): (
        Option<Res<Location>>,
        Option<Res<PlayerKnowledge>>,
        Option<Res<GameClock>>,
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
    commands.spawn(icon(
        "ui/map/target.png",
        40.0,
        theme::TEXT,
        MapIcon::Destination,
    ));
    commands.spawn(icon("ui/map/here.png", 40.0, theme::ACCENT, MapIcon::Here));

    spawn_panels(&mut commands, &texts, &fonts, &asset_server);

    // Autopilote (captures) : `SH_HOVER=<lieu>` simule le survol, `SH_ROUTE=<lieu>` pose un
    // itinéraire (système ou monde isolé).
    let pick = |var: &str| {
        let id = std::env::var(var).ok()?;
        entries.iter().find(|e| e.id.as_str() == id).cloned()
    };
    let forced_hover = pick("SH_HOVER");
    if let (Some(dest), Some(here)) = (pick("SH_ROUTE"), location_id)
        && dest.destination
        && dest.id != here
    {
        commands.insert_resource(crate::voyage::plan_route(&data, &here, &dest.id));
    }
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
        "view.galaxy.hint",
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
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            max_width: px(360),
            // Large à gauche : la barre ambre de la plaquette.
            padding: UiRect::new(px(14), px(12), px(7), px(9)),
            ..default()
        },
        fonts.frame(Frame::Plate),
        Visibility::Hidden,
        children![
            (
                TooltipTitle,
                Text::new(""),
                fonts.title(15.0),
                TextColor(theme::TEXT),
                Pickable::IGNORE,
            ),
            (
                TooltipBody,
                Text::new(""),
                fonts.body(14.0),
                TextColor(theme::TEXT_DIM),
                Pickable::IGNORE,
            ),
        ],
    ));
    // Le panneau d'itinéraire : un relevé d'instrument. En haut la position et le carburant (à
    // chasse fixe), puis la route choisie (titre ambre) et ses deux façons d'y aller.
    commands.spawn((
        SCOPE,
        MapPanel,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(20),
            bottom: px(20),
            max_width: px(560),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            padding: UiRect::new(px(18), px(18), px(12), px(14)),
            ..default()
        },
        fonts.frame(Frame::Readout),
        children![
            (
                RouteLine::Status,
                Text::new(""),
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
                fonts.title(16.0),
                TextColor(theme::ACCENT),
                Pickable::IGNORE,
            ),
            (
                RouteLine::Options,
                Text::new(""),
                fonts.body(15.0),
                TextColor(theme::TEXT),
                Pickable::IGNORE,
            ),
        ],
    ));
    spawn_legend(commands, texts, fonts, asset_server);
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
    (location, route): (Option<Res<Location>>, Option<Res<RoutePlan>>),
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
}

/// L'entrée ou la flotte sous le curseur, à l'écran (à quelques pixels de son icône).
fn hover(
    window: Single<&Window, With<PrimaryWindow>>,
    screen: Res<MapScreen>,
    entries: Res<MapEntries>,
    fleets: Res<FleetMarkers>,
    forced: Res<ForcedHover>,
    mut hovered: ResMut<HoveredEntry>,
    mut hovered_fleet: ResMut<HoveredFleet>,
) {
    if forced.0.is_some() {
        return;
    }
    let (entry, fleet) = match window.cursor_position() {
        None => (None, None),
        Some(cursor) => {
            let list = &entries.entries;
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
            match (nearest_entry, nearest_fleet) {
                (Some(e), Some(f)) if f.0 < e.0 => (None, Some(f.1.clone())),
                (None, Some(f)) => (None, Some(f.1.clone())),
                (e, _) => (e.map(|e| e.1.clone()), None),
            }
        }
    };
    if hovered.0 != entry {
        hovered.0 = entry;
    }
    if hovered_fleet.0 != fleet {
        hovered_fleet.0 = fleet;
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
/// tourne lentement.
fn place_markers(
    time: Res<Time>,
    screen: Res<MapScreen>,
    entries: Res<MapEntries>,
    mut icons: Query<(
        &MapIcon,
        &ComputedNode,
        &mut Node,
        &mut Visibility,
        &mut UiTransform,
    )>,
) {
    let t = time.elapsed_secs();
    for (icon, computed, mut node, mut visibility, mut transform) in &mut icons {
        let size = computed.size() * computed.inverse_scale_factor();
        let at = match *icon {
            MapIcon::Entry(i) => screen
                .entries
                .get(i)
                .copied()
                .flatten()
                .filter(|_| screen.shown.get(i).copied().unwrap_or(false)),
            // Le badge du hub, au coin haut-droit de l'icône du système.
            MapIcon::Hub(i) => screen
                .entries
                .get(i)
                .copied()
                .flatten()
                .filter(|_| screen.shown.get(i).copied().unwrap_or(false))
                .map(|p| {
                    let r = entries.entries.get(i).map_or(14.0, icon_size) / 2.0;
                    p + Vec2::new(r * 0.8, -r * 0.8)
                }),
            MapIcon::Fleet(i) => screen.fleets.get(i).copied().flatten(),
            MapIcon::Here => screen.here,
            MapIcon::Destination => screen.dest,
        };
        set_visible(&mut visibility, at.is_some());
        let Some(at) = at else { continue };
        set_position(&mut node, at.x - size.x / 2.0, at.y - size.y / 2.0);
        transform.set_if_neq(match *icon {
            MapIcon::Here => UiTransform {
                scale: Vec2::splat(1.0 + 0.08 * (t * 3.0).sin()),
                ..UiTransform::IDENTITY
            },
            MapIcon::Destination => UiTransform::from_rotation(Rot2::radians(0.35 * t)),
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
    location: Option<Res<Location>>,
    route: Option<Res<RoutePlan>>,
    mut labels: Query<(
        Entity,
        &MapLabel,
        &Text,
        &ComputedNode,
        &mut Node,
        &mut Visibility,
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
    for (entity, label, text, computed, mut node, mut visibility) in &mut labels {
        let size = size_of(text, label, computed);
        match label.target {
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

    // L'ordre de placement.
    let find = |id: &sh_core::Id<Place>| list.iter().position(|e| &e.id == id);
    let mut wanted: Vec<usize> = Vec::new();
    for i in [
        location.as_ref().and_then(|l| find(&l.system)),
        route.as_ref().and_then(|r| find(&r.dest)),
        hovered.0.as_ref().and_then(|h| find(&h.id)),
    ]
    .into_iter()
    .flatten()
    .chain(entries.order.iter().copied())
    {
        if !wanted.contains(&i) {
            wanted.push(i);
        }
    }
    let mut requests = Vec::new();
    let mut owners = Vec::new();
    for i in wanted {
        let (Some(anchor), true, Some((entity, size))) = (
            screen.entries.get(i).copied().flatten(),
            screen.shown.get(i).copied().unwrap_or(false),
            entry_labels.get(i).copied().flatten(),
        ) else {
            continue;
        };
        let clearance = if list[i].kind == MapKind::Phenomenon && list[i].radius.is_some() {
            0.0
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
    for (entity, label, _, _, mut node, mut visibility) in &mut labels {
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

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn show_tooltip(
    data: Res<Data>,
    texts: Res<Texts>,
    hovered: Res<HoveredEntry>,
    hovered_fleet: Res<HoveredFleet>,
    forced: Res<ForcedHover>,
    (screen, entries, location): (Res<MapScreen>, Res<MapEntries>, Option<Res<Location>>),
    window: Single<&Window, With<PrimaryWindow>>,
    mut tooltip: Single<(&mut Node, &mut Visibility), With<Tooltip>>,
    mut title: Single<&mut Text, (With<TooltipTitle>, Without<TooltipBody>)>,
    mut body: Single<&mut Text, (With<TooltipBody>, Without<TooltipTitle>)>,
) {
    let (node, visibility) = &mut *tooltip;
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
        entry_lines(&data, &texts, entry, location.as_ref().map(|l| &l.system))
    } else {
        set_visible(visibility, false);
        return;
    };
    // Sous le curseur ; survol imposé par l'autopilote : à côté de l'icône survolée.
    let forced_at = forced.0.as_ref().and_then(|f| {
        let i = entries.entries.iter().position(|e| e.id == f.id)?;
        screen.entries.get(i).copied().flatten()
    });
    let cursor = forced_at
        .or_else(|| window.cursor_position())
        .unwrap_or(window.size() / 2.0);
    set_position(node, cursor.x + 18.0, cursor.y + 14.0);
    let mut lines = lines.into_iter();
    set_text(&mut title, lines.next().unwrap_or_default());
    set_text(&mut body, lines.collect::<Vec<_>>().join("\n"));
    set_visible(visibility, true);
}

fn show_route(
    data: Res<Data>,
    texts: Res<Texts>,
    route: Option<Res<RoutePlan>>,
    fuel: Option<Res<ShipFuel>>,
    location: Option<Res<Location>>,
    mut lines: Query<(&RouteLine, &mut Text, &mut TextColor, &mut Node)>,
) {
    let here = location
        .and_then(|l| l.endpoint(&data))
        .map_or_else(String::new, |e| e.name().to_owned());
    let have = fuel.as_ref().map_or(0.0, |f| f.units);
    let status = [
        texts.format("map.here", &[("place", &here)]),
        texts.format("map.fuel", &[("fuel", &format!("{have:.0}"))]),
    ]
    .join("\n");
    let (title, title_color, options) = match route {
        None => (texts.get("map.plan.hint"), theme::TEXT_DIM, Vec::new()),
        Some(route) => {
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
