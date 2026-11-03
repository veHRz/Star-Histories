//! Vue Sectorielle — version de test (M0).
//!
//! Une salle de test en boîtes simples : elle sert à vérifier l'état, les entrées À pied, la
//! capture de la souris, les lumières et les préréglages graphiques. Le vrai secteur (le vaisseau
//! du joueur, ses cellules et ses portails) arrive en M1. Déplacement sans physique (avian3d en M1).

use bevy::camera::Exposure;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::data::Texts;
use crate::graphics::{GraphicsSettings, WorldCamera};
use crate::input::{Look, Move, Sprint};
use crate::state::{ControlMode, CurrentPlace, ViewLevel};
use crate::ui::theme::UiFonts;
use crate::views::view_banner;

pub struct SectorViewPlugin;

impl Plugin for SectorViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Sector), spawn_test_room)
            .add_systems(Update, walk.run_if(in_state(ViewLevel::Sector)));
    }
}

const SCOPE: DespawnOnExit<ViewLevel> = DespawnOnExit(ViewLevel::Sector);
const EYE_HEIGHT: f32 = 1.7;
const WALK_SPEED: f32 = 3.5;
const SPRINT_FACTOR: f32 = 2.0;
const MOUSE_SENSITIVITY: f32 = 0.0022;

/// Orientation de la tête (lacet, tangage), séparée de la transformation pour éviter le roulis.
#[derive(Component, Default)]
struct Head {
    yaw: f32,
    pitch: f32,
}

fn spawn_test_room(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    settings: Option<Res<GraphicsSettings>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mode: ResMut<NextState<ControlMode>>,
    place: Res<CurrentPlace>,
) {
    // La salle de test n'existe que pour `sector:test` ; les vrais secteurs viennent des données.
    if place.0.as_deref() != Some("test") {
        return;
    }
    mode.set(ControlMode::OnFoot);
    let wall = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.44, 0.47),
        perceptual_roughness: 0.85,
        metallic: 0.1,
        ..default()
    });
    let floor = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.23, 0.25),
        perceptual_roughness: 0.7,
        metallic: 0.3,
        ..default()
    });
    let crate_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.40, 0.18),
        perceptual_roughness: 0.6,
        ..default()
    });
    let panel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.06, 0.07),
        emissive: LinearRgba::rgb(0.2, 1.4, 1.8),
        ..default()
    });
    let lamp = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::rgb(14.0, 11.0, 8.0),
        ..default()
    });

    // Salle de 12 × 16 m, 4 m sous plafond.
    let (w, d, h) = (12.0_f32, 16.0_f32, 4.0_f32);
    let boxes = [
        // (taille, position, matériau)
        (
            Vec3::new(w, 0.2, d),
            Vec3::new(0.0, -0.1, 0.0),
            floor.clone(),
        ),
        (
            Vec3::new(w, 0.2, d),
            Vec3::new(0.0, h + 0.1, 0.0),
            wall.clone(),
        ),
        (
            Vec3::new(0.2, h, d),
            Vec3::new(-w / 2.0, h / 2.0, 0.0),
            wall.clone(),
        ),
        (
            Vec3::new(0.2, h, d),
            Vec3::new(w / 2.0, h / 2.0, 0.0),
            wall.clone(),
        ),
        (
            Vec3::new(w, h, 0.2),
            Vec3::new(0.0, h / 2.0, -d / 2.0),
            wall.clone(),
        ),
        (
            Vec3::new(w, h, 0.2),
            Vec3::new(0.0, h / 2.0, d / 2.0),
            wall.clone(),
        ),
        // Caisses.
        (
            Vec3::splat(1.2),
            Vec3::new(-3.5, 0.6, -4.0),
            crate_mat.clone(),
        ),
        (
            Vec3::splat(1.2),
            Vec3::new(-3.5, 1.8, -4.0),
            crate_mat.clone(),
        ),
        (
            Vec3::splat(1.2),
            Vec3::new(-2.2, 0.6, -4.3),
            crate_mat.clone(),
        ),
        (
            Vec3::new(2.0, 1.0, 1.0),
            Vec3::new(3.6, 0.5, 2.0),
            crate_mat,
        ),
        // Console.
        (
            Vec3::new(2.4, 1.1, 0.8),
            Vec3::new(0.0, 0.55, -7.2),
            wall.clone(),
        ),
        (Vec3::new(2.2, 0.7, 0.05), Vec3::new(0.0, 1.5, -7.55), panel),
    ];
    for (size, pos, material) in boxes {
        commands.spawn((
            SCOPE,
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(material),
            Transform::from_translation(pos),
        ));
    }

    // Plafonniers : les premiers projettent des ombres, dans la limite du préréglage.
    let shadowed = settings.map_or(2, |s| usize::from(s.preset.max_shadowed_point_lights));
    let lamp_mesh = meshes.add(Cuboid::new(1.2, 0.06, 0.3));
    for (i, z) in [-5.0_f32, 0.0, 5.0].into_iter().enumerate() {
        commands.spawn((
            SCOPE,
            Mesh3d(lamp_mesh.clone()),
            MeshMaterial3d(lamp.clone()),
            Transform::from_xyz(0.0, h - 0.03, z),
        ));
        commands.spawn((
            SCOPE,
            PointLight {
                intensity: 150_000.0,
                range: 14.0,
                color: Color::srgb(1.0, 0.92, 0.82),
                shadow_maps_enabled: i < shadowed,
                ..default()
            },
            Transform::from_xyz(0.0, h - 0.3, z),
        ));
    }
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.6, 0.7, 0.9),
        brightness: 40.0,
        ..default()
    });

    commands.spawn((
        Name::new("Caméra à la première personne"),
        SCOPE,
        WorldCamera,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 75f32.to_radians(),
            ..default()
        }),
        // Exposition d'intérieur : l'exposition par défaut de Bevy vise un extérieur ensoleillé.
        Exposure::INDOOR,
        Head::default(),
        Transform::from_xyz(0.0, EYE_HEIGHT, 6.0),
    ));

    commands.spawn(view_banner(
        &texts,
        &fonts,
        texts.get("view.sector.title"),
        "view.sector.hint",
        SCOPE,
    ));
}

fn walk(
    time: Res<Time>,
    movement: Single<&Action<Move>>,
    look: Single<&Action<Look>>,
    sprint: Single<&Action<Sprint>>,
    mut head: Single<(&mut Transform, &mut Head), With<WorldCamera>>,
) {
    let (transform, head_angles) = &mut *head;
    let look: Vec2 = ***look;
    head_angles.yaw -= look.x * MOUSE_SENSITIVITY;
    head_angles.pitch = (head_angles.pitch - look.y * MOUSE_SENSITIVITY).clamp(-1.5, 1.5);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, head_angles.yaw, head_angles.pitch, 0.0);

    let input: Vec2 = ***movement;
    let speed = if ***sprint {
        WALK_SPEED * SPRINT_FACTOR
    } else {
        WALK_SPEED
    };
    let forward = Quat::from_rotation_y(head_angles.yaw) * Vec3::NEG_Z;
    let right = Quat::from_rotation_y(head_angles.yaw) * Vec3::X;
    transform.translation += (forward * input.y + right * input.x) * speed * time.delta_secs();
    transform.translation.y = EYE_HEIGHT;
}
