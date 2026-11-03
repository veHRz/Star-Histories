//! Vue Galaxie — version de test (M0).
//!
//! Affiche la carte canon d'Astréa depuis les données : ellipse de la galaxie, grille A–T × 1–13,
//! contours des zones, systèmes, astres isolés, stations, phénomènes. Les entrées secrètes ne sont
//! jamais dessinées. Repère : 1 unité de carte = 1 unité du monde, x vers l'est, y de la carte
//! vers le sud (+z), nord en haut de l'écran.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use sh_core::MapPos;
use sh_core::grid::{CELL_SIZE, COLUMNS, MAP_HEIGHT, MAP_WIDTH, ROWS};
use sh_data::{BodyKind, Faction};

use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::input::{Pan, Zoom};
use crate::state::{ControlMode, ViewLevel};
use crate::ui::theme::{self, UiFonts};
use crate::views::{WorldLabel, view_banner};

pub struct GalaxyViewPlugin;

impl Plugin for GalaxyViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Galaxy), spawn_galaxy)
            .add_systems(
                Update,
                (pan_and_zoom, draw_map_lines).run_if(in_state(ViewLevel::Galaxy)),
            );
    }
}

const SCOPE: DespawnOnExit<ViewLevel> = DespawnOnExit(ViewLevel::Galaxy);
const CENTER: Vec2 = Vec2::new(500.0, 330.0);
const MIN_HEIGHT: f32 = 60.0;
const MAX_HEIGHT: f32 = 1400.0;

/// Position du monde d'un point de la carte.
fn world(pos: MapPos) -> Vec3 {
    Vec3::new(pos.x as f32 - CENTER.x, 0.0, pos.y as f32 - CENTER.y)
}

/// Couleur d'affiliation (lisibilité de la vue de test, pas une charte).
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

fn spawn_galaxy(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mode: ResMut<NextState<ControlMode>>,
) {
    mode.set(ControlMode::Map);
    let g = &data.galaxy;
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

    commands.spawn((
        Name::new("Caméra de la carte"),
        SCOPE,
        WorldCamera,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.01, 0.012, 0.02)),
            ..default()
        },
        Transform::from_xyz(0.0, 780.0, 0.0).looking_at(Vec3::ZERO, Vec3::NEG_Z),
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

    for system in g.systems.iter().filter(|s| !s.secret) {
        let color = faction_color(system.controller.as_ref());
        let pos = world(system.center);
        commands.spawn((
            Name::new(system.name.clone()),
            SCOPE,
            Mesh3d(sphere.clone()),
            MeshMaterial3d(materials.add(unlit(color))),
            Transform::from_translation(pos).with_scale(Vec3::splat(4.0)),
        ));
        commands.spawn(label(&fonts, &system.name, pos, theme::TEXT));
    }

    for body in g
        .bodies
        .iter()
        .filter(|b| !b.secret && b.system.is_none() && b.kind != BodyKind::Star)
    {
        let Some(pos) = body.pos else { continue };
        let color = if body.destroyed {
            Color::srgb(0.30, 0.30, 0.32)
        } else {
            faction_color(body.controller.as_ref())
        };
        let pos = world(pos);
        commands.spawn((
            Name::new(body.name.clone()),
            SCOPE,
            Mesh3d(sphere.clone()),
            MeshMaterial3d(materials.add(unlit(color))),
            Transform::from_translation(pos).with_scale(Vec3::splat(2.6)),
        ));
        commands.spawn(label(&fonts, &body.name, pos, theme::TEXT_DIM));
    }

    for station in g
        .stations
        .iter()
        .filter(|s| !s.secret && s.system.is_none())
    {
        let pos = world(station.pos);
        commands.spawn((
            Name::new(station.name.clone()),
            SCOPE,
            Mesh3d(cube.clone()),
            MeshMaterial3d(materials.add(unlit(faction_color(station.controller.as_ref())))),
            Transform::from_translation(pos).with_scale(Vec3::splat(3.4)),
        ));
        commands.spawn(label(&fonts, &station.name, pos, theme::TEXT_DIM));
    }

    for phenomenon in g.phenomena.iter().filter(|p| !p.secret) {
        let (Some(pos), Some(radius)) = (phenomenon.pos, phenomenon.radius) else {
            continue;
        };
        commands.spawn((
            Name::new(phenomenon.name.clone()),
            SCOPE,
            Mesh3d(meshes.add(Circle::new(radius as f32))),
            MeshMaterial3d(materials.add(unlit(Color::srgba(0.55, 0.25, 0.70, 0.35)))),
            Transform::from_translation(world(pos) - Vec3::Y * 0.5)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
    }

    commands.spawn(view_banner(
        &texts,
        &fonts,
        texts.get("view.galaxy.title"),
        "view.galaxy.hint",
        SCOPE,
    ));
}

fn label(fonts: &UiFonts, name: &str, anchor: Vec3, color: Color) -> impl Bundle {
    (
        SCOPE,
        WorldLabel::At(anchor),
        Text::new(name),
        fonts.body(13.0),
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

/// Grille A–T × 1–13, contours des zones et bord du repère (gizmos, redessinés chaque frame).
fn draw_map_lines(mut gizmos: Gizmos, data: Res<Data>) {
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
}
