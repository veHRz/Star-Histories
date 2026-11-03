//! Plugins Bevy de Star Histories.
//!
//! Squelette de départ : une scène de test qui prouve que la chaîne de compilation,
//! le rendu et les outils d'autopilote (`dev`) fonctionnent. Le jalon M0 d'AGENTS.md
//! remplace cette scène par la vraie architecture (états, vues, données canon).

use bevy::prelude::*;

mod dev;

pub struct StarHistoriesPlugin;

impl Plugin for StarHistoriesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Star Histories".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(dev::DevToolsPlugin)
        .add_systems(Startup, spawn_smoke_scene);
    }
}

fn spawn_smoke_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(8.0, 8.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.12, 0.13, 0.16))),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(196, 152, 72))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
