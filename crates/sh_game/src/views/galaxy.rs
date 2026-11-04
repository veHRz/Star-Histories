//! Vue Galaxie : la carte d'Astréa (M3).
//!
//! Tout ce qui s'affiche vient de `sh_data::public_map` : rien de secret, ni élément ni texte
//! (règle d'or n°3 ; test `la_carte_ne_montre_aucun_secret`). Ellipse de la galaxie, grille
//! A–T × 1–13 et ses repères, contours des zones, systèmes, astres isolés, stations,
//! phénomènes ; la position du vaisseau ; une infobulle au survol (champs publics seulement) ;
//! un clic choisit une destination et affiche ses itinéraires ([`crate::voyage`]).
//!
//! Habillage (M8) : l'infobulle est une plaquette à barre ambre (nom en titre), l'itinéraire un
//! relevé d'instrument (position et carburant à chasse fixe, route en ambre).
//!
//! Repère : 1 unité de carte = 1 unité du monde, x vers l'est, y de la carte vers le sud (+z),
//! nord en haut de l'écran.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_enhanced_input::prelude::*;
use sh_core::MapPos;
use sh_core::grid::{CELL_SIZE, COLUMNS, MAP_HEIGHT, MAP_WIDTH, ROWS};
use sh_data::{Faction, MapEntry, MapKind, entry_at, public_map};

use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::input::{Pan, Zoom};
use crate::living::PlayerKnowledge;
use crate::state::{ControlMode, ViewLevel};
use crate::ui::theme::{self, Frame, UiFonts};
use crate::views::{WorldLabel, view_banner};
use crate::voyage::{HoveredEntry, Location, RoutePlan, ShipFuel};

pub struct GalaxyViewPlugin;

impl Plugin for GalaxyViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Galaxy), spawn_galaxy)
            .add_systems(
                Update,
                (
                    pan_and_zoom,
                    hover,
                    show_tooltip,
                    show_route,
                    draw_map_lines,
                )
                    .chain()
                    .run_if(in_state(ViewLevel::Galaxy)),
            );
    }
}

const SCOPE: DespawnOnExit<ViewLevel> = DespawnOnExit(ViewLevel::Galaxy);
const CENTER: Vec2 = Vec2::new(500.0, 330.0);
const MIN_HEIGHT: f32 = 60.0;
const MAX_HEIGHT: f32 = 1400.0;

/// Les entrées de la carte (calculées une fois à l'entrée de la vue).
#[derive(Resource)]
struct MapEntries(Vec<MapEntry>);

/// Une flotte connue du joueur, à sa dernière position connue (brouillard d'information).
#[derive(Clone, Debug, PartialEq)]
struct FleetMarker {
    name: String,
    pos: MapPos,
    at: String,
    faction: Option<sh_core::Id<Faction>>,
    as_of: sh_core::StellarDate,
}

#[derive(Resource)]
struct FleetMarkers(Vec<FleetMarker>);

/// La flotte connue sous le curseur, s'il y en a une (elle passe avant les lieux).
#[derive(Resource, Default)]
struct HoveredFleet(Option<FleetMarker>);

/// Survol imposé par l'autopilote (`SH_HOVER`) : il remplace le curseur.
#[derive(Resource)]
struct ForcedHover(Option<MapEntry>);

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

fn map_pos(world: Vec3) -> MapPos {
    MapPos::new(f64::from(world.x + CENTER.x), f64::from(world.z + CENTER.y))
}

/// Couleur d'affiliation (lisibilité de la carte, pas une charte).
fn faction_color(faction: Option<&sh_core::Id<Faction>>) -> Color {
    match faction.map(|f| f.as_str()) {
        Some("ugl" | "fou" | "fdu") => Color::srgb(0.35, 0.62, 1.0),
        Some("empire") => Color::srgb(0.95, 0.32, 0.28),
        Some("hexa_corp") => Color::srgb(0.95, 0.78, 0.30),
        Some("federation") => Color::srgb(0.70, 0.70, 0.75),
        Some("royaume_grothul") => Color::srgb(0.55, 0.85, 0.40),
        Some("ligue_des_pirates" | "pirates_independants") => Color::srgb(0.85, 0.45, 0.85),
        Some(_) => Color::srgb(0.75, 0.70, 0.60),
        None => Color::srgb(0.60, 0.62, 0.66),
    }
}

/// Position sur la carte d'un lieu (système, astre, station).
fn place_pos(g: &sh_data::Galaxy, id: &sh_core::Id<sh_data::Place>) -> Option<MapPos> {
    g.system(id)
        .map(|s| s.center)
        .or_else(|| g.body(id).and_then(|b| b.pos))
        .or_else(|| g.station(id).map(|s| s.pos))
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

#[allow(clippy::too_many_arguments)]
fn spawn_galaxy(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    location: Option<Res<Location>>,
    knowledge: Option<Res<PlayerKnowledge>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mode: ResMut<NextState<ControlMode>>,
) {
    mode.set(ControlMode::Map);
    let g = &data.galaxy;
    let entries = public_map(g);
    let unlit = |color: Color| StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: if color.alpha() < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        ..default()
    };

    // Caméra centrée sur le vaisseau.
    let location_id = location.as_ref().map(|l| l.system.clone());
    let here = location
        .and_then(|l| g.system(&l.system).map(|s| world(s.center)))
        .unwrap_or(Vec3::ZERO);
    commands.spawn((
        Name::new("Caméra de la carte"),
        SCOPE,
        WorldCamera,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.01, 0.012, 0.02)),
            ..default()
        },
        Transform::from_translation(here + Vec3::Y * 420.0).looking_at(here, Vec3::NEG_Z),
    ));

    // La galaxie : une ellipse (§3.1).
    commands.spawn((
        Name::new("Galaxie"),
        SCOPE,
        Mesh3d(meshes.add(Ellipse::new(
            g.frame.radius_x as f32,
            g.frame.radius_y as f32,
        ))),
        MeshMaterial3d(materials.add(unlit(Color::srgba(0.16, 0.20, 0.34, 0.55)))),
        Transform::from_xyz(0.0, -1.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(24, 12));
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for entry in &entries {
        let pos = world(entry.pos);
        let color = if entry.destroyed {
            Color::srgb(0.30, 0.30, 0.32)
        } else {
            faction_color(entry.controller.as_ref())
        };
        let (mesh, scale, label_color, transform) = match entry.kind {
            MapKind::System => (sphere.clone(), 4.0, theme::TEXT, None),
            MapKind::Body => (sphere.clone(), 2.6, theme::TEXT_DIM, None),
            MapKind::Station => (cube.clone(), 3.4, theme::TEXT_DIM, None),
            MapKind::Phenomenon => {
                let Some(radius) = entry.radius else { continue };
                (
                    meshes.add(Circle::new(radius as f32)),
                    1.0,
                    theme::TEXT_DIM,
                    Some(
                        Transform::from_translation(pos - Vec3::Y * 0.5)
                            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                    ),
                )
            }
        };
        let material = if entry.kind == MapKind::Phenomenon {
            materials.add(unlit(Color::srgba(0.55, 0.25, 0.70, 0.35)))
        } else {
            materials.add(unlit(color))
        };
        commands.spawn((
            Name::new(entry.name.clone()),
            SCOPE,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform.unwrap_or(Transform::from_translation(pos).with_scale(Vec3::splat(scale))),
        ));
        commands.spawn(label(&fonts, &entry.name, pos, label_color, 13.0));
    }

    // Repères de la grille : lettres en haut, chiffres à gauche.
    for col in 0..COLUMNS {
        let x = (f64::from(col) + 0.5) * CELL_SIZE;
        let letter = char::from(b'A' + col).to_string();
        commands.spawn(label(
            &fonts,
            &letter,
            world(MapPos::new(x, 4.0)),
            theme::TEXT_DIM,
            12.0,
        ));
    }
    for row in 0..ROWS {
        let y = (f64::from(row) + 0.5) * CELL_SIZE;
        let number = (row + 1).to_string();
        commands.spawn(label(
            &fonts,
            &number,
            world(MapPos::new(4.0, y)),
            theme::TEXT_DIM,
            12.0,
        ));
    }

    // Infobulle et panneau d'itinéraire.
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
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(20),
            bottom: px(20),
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
    // Autopilote (captures) : `SH_HOVER=<système>` simule le survol, `SH_ROUTE=<système>` pose
    // un itinéraire.
    let pick = |var: &str| {
        let id = std::env::var(var).ok()?;
        entries.iter().find(|e| e.id.as_str() == id).cloned()
    };
    let forced_hover = pick("SH_HOVER");
    if let (Some(dest), Some(here)) = (pick("SH_ROUTE"), location_id) {
        commands.insert_resource(crate::voyage::plan_route(&data, &here, &dest.id));
    }
    commands.insert_resource(ForcedHover(forced_hover.clone()));
    commands.insert_resource(HoveredEntry(forced_hover));
    // Les flottes connues du joueur, en couronne autour de leur dernière position connue.
    let mut markers = Vec::new();
    let diamond = meshes.add(Cuboid::new(2.6, 0.4, 2.6));
    if let Some(knowledge) = knowledge {
        let mut per_place: std::collections::BTreeMap<String, usize> = Default::default();
        for (id, known) in &knowledge.fleets {
            let Some(base) = place_pos(g, &known.at) else {
                continue;
            };
            let Some(fleet) = g.fleets.iter().find(|f| &f.id == id).filter(|f| !f.secret) else {
                continue;
            };
            let slot = per_place.entry(known.at.to_string()).or_default();
            let angle = std::f64::consts::TAU * (*slot as f64) / 6.0 + 0.4;
            *slot += 1;
            let pos = MapPos::new(base.x + 7.0 * angle.cos(), base.y + 7.0 * angle.sin());
            commands.spawn((
                Name::new(fleet.name.clone()),
                SCOPE,
                Mesh3d(diamond.clone()),
                MeshMaterial3d(materials.add(unlit(faction_color(fleet.faction.as_ref())))),
                Transform::from_translation(world(pos) + Vec3::Y * 0.6)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)),
            ));
            markers.push(FleetMarker {
                name: fleet.name.clone(),
                pos,
                at: known.at.to_string(),
                faction: fleet.faction.clone(),
                as_of: known.as_of,
            });
        }
    }
    commands.insert_resource(FleetMarkers(markers));
    commands.insert_resource(HoveredFleet::default());
    commands.insert_resource(MapEntries(entries));

    commands.spawn(view_banner(
        &texts,
        &fonts,
        texts.get("view.galaxy.title"),
        "view.galaxy.hint",
        SCOPE,
    ));
}

fn label(fonts: &UiFonts, name: &str, anchor: Vec3, color: Color, size: f32) -> impl Bundle {
    (
        SCOPE,
        WorldLabel::At(anchor),
        Text::new(name),
        fonts.body(size),
        TextColor(color),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
    )
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
    camera.translation.x += pan.x * speed * time.delta_secs();
    camera.translation.z -= pan.y * speed * time.delta_secs();
    let zoom: f32 = ***zoom;
    if zoom != 0.0 {
        camera.translation.y =
            (height * (1.0 - 0.12 * zoom.clamp(-3.0, 3.0))).clamp(MIN_HEIGHT, MAX_HEIGHT);
    }
}

/// L'entrée sous le curseur : rayon de la caméra jusqu'au plan de la carte.
fn hover(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform, &Transform), With<WorldCamera>>,
    entries: Res<MapEntries>,
    markers: Res<FleetMarkers>,
    forced: Res<ForcedHover>,
    mut hovered: ResMut<HoveredEntry>,
    mut hovered_fleet: ResMut<HoveredFleet>,
) {
    if forced.0.is_some() {
        return;
    }
    let (camera, camera_transform, transform) = *camera;
    let found = window.cursor_position().and_then(|cursor| {
        // Avec l'échelle de rendu, la caméra dessine dans une image plus petite que la fenêtre.
        let scale = camera
            .logical_viewport_size()
            .map_or(Vec2::ONE, |size| size / window.size().max(Vec2::ONE));
        let ray = camera
            .viewport_to_world(camera_transform, cursor * scale)
            .ok()?;
        let t = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
        // Portée de survol proportionnelle à l'altitude : ≈ 12 pixels à l'écran.
        let reach = f64::from(transform.translation.y) * 0.025;
        let at = map_pos(ray.get_point(t));
        let fleet = markers
            .0
            .iter()
            .map(|m| ((m.pos.x - at.x).hypot(m.pos.y - at.y), m))
            .filter(|(d, _)| *d <= reach * 0.6)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, m)| m.clone());
        Some((entry_at(&entries.0, at, reach).cloned(), fleet))
    });
    let (entry, fleet) = found.unwrap_or((None, None));
    if hovered.0 != entry {
        hovered.0 = entry;
    }
    if hovered_fleet.0 != fleet {
        hovered_fleet.0 = fleet;
    }
}

/// Remplace un texte seulement s'il a changé : un texte réécrit est remis en page.
fn set_text(text: &mut Text, wanted: String) {
    if text.0 != wanted {
        text.0 = wanted;
    }
}

#[allow(clippy::type_complexity)]
fn show_tooltip(
    data: Res<Data>,
    texts: Res<Texts>,
    hovered: Res<HoveredEntry>,
    hovered_fleet: Res<HoveredFleet>,
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
        lines
    } else if let Some(entry) = &hovered.0 {
        let kind = match entry.kind {
            MapKind::System => "map.tip.kind.system",
            MapKind::Body => "map.tip.kind.body",
            MapKind::Station => "map.tip.kind.station",
            MapKind::Phenomenon => "map.tip.kind.phenomenon",
        };
        let mut lines = vec![entry.name.clone(), texts.get(kind)];
        if let Some(cell) = entry.cell {
            lines.push(texts.format("map.tip.cell", &[("cell", &cell.to_string())]));
        }
        if let Some(faction) = entry.controller.as_ref().and_then(|f| data.faction(f)) {
            lines.push(texts.format("map.tip.controller", &[("faction", &faction.name)]));
        }
        if entry.population > 0 {
            let value = population_text(&texts, entry.population);
            lines.push(texts.format("map.tip.population", &[("value", &value)]));
        }
        if entry.has_portal_hub {
            lines.push(texts.get("map.tip.hub"));
        }
        if entry.destroyed {
            lines.push(texts.get("map.tip.destroyed"));
        }
        lines
    } else {
        **visibility = Visibility::Hidden;
        return;
    };
    // Sous le curseur ; sans curseur (autopilote), au centre de l'écran.
    let cursor = window.cursor_position().unwrap_or(window.size() / 2.0);
    node.left = px(cursor.x + 16.0);
    node.top = px(cursor.y + 12.0);
    let mut lines = lines.into_iter();
    set_text(&mut title, lines.next().unwrap_or_default());
    set_text(&mut body, lines.collect::<Vec<_>>().join("\n"));
    **visibility = Visibility::Inherited;
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
        .and_then(|l| data.galaxy.system(&l.system))
        .map_or_else(String::new, |s| s.name.clone());
    let have = fuel.as_ref().map_or(0.0, |f| f.units);
    let status = [
        texts.format("map.here", &[("place", &here)]),
        texts.format("map.fuel", &[("fuel", &format!("{have:.0}"))]),
    ]
    .join("\n");
    let (title, title_color, options) = match route {
        None => (texts.get("map.plan.hint"), theme::TEXT_DIM, Vec::new()),
        Some(route) => {
            let name = data
                .galaxy
                .system(&route.dest)
                .map_or_else(String::new, |s| s.name.clone());
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

/// Grille, contours des zones, position du vaisseau, destination (gizmos, redessinés chaque frame).
fn draw_map_lines(
    mut gizmos: Gizmos,
    data: Res<Data>,
    location: Option<Res<Location>>,
    route: Option<Res<RoutePlan>>,
) {
    let grid = Color::srgba(0.55, 0.60, 0.75, 0.10);
    for col in 0..=COLUMNS {
        let x = f64::from(col) * CELL_SIZE;
        gizmos.line(
            world(MapPos::new(x, 0.0)),
            world(MapPos::new(x, MAP_HEIGHT)),
            grid,
        );
    }
    for row in 0..=ROWS {
        let y = f64::from(row) * CELL_SIZE;
        gizmos.line(
            world(MapPos::new(0.0, y)),
            world(MapPos::new(MAP_WIDTH, y)),
            grid,
        );
    }
    for zone in data
        .galaxy
        .zones
        .iter()
        .filter(|z| !z.secret && !z.outline.is_empty())
    {
        let color = faction_color(zone.controller.as_ref()).with_alpha(0.55);
        let points = zone
            .outline
            .iter()
            .chain(zone.outline.first())
            .map(|&p| world(p) + Vec3::Y * 0.2);
        gizmos.linestrip(points, color);
    }
    let flat = Isometry3d::new(
        Vec3::ZERO,
        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
    );
    let here = location.and_then(|l| data.galaxy.system(&l.system).map(|s| world(s.center)));
    if let Some(here) = here {
        let ring = Isometry3d::new(here + Vec3::Y * 0.5, flat.rotation);
        gizmos.circle(ring, 9.0, theme::ACCENT);
        if let Some(dest) = route
            .as_ref()
            .and_then(|r| data.galaxy.system(&r.dest).map(|s| world(s.center)))
        {
            gizmos.circle(
                Isometry3d::new(dest + Vec3::Y * 0.5, flat.rotation),
                9.0,
                theme::TEXT,
            );
            gizmos.line(
                here + Vec3::Y * 0.5,
                dest + Vec3::Y * 0.5,
                theme::ACCENT.with_alpha(0.6),
            );
        }
    }
}
