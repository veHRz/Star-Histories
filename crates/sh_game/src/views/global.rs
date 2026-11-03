//! Vue Globale — version de test (M0).
//!
//! Charge le système d'un lieu (`global:hexis`, `global:sys_cygnara`…) et en dispose les corps
//! d'après la carte canon, agrandie. La disposition canon à l'intérieur d'un système est
//! schématique (§0 de l'export) : cette vue ne prétend rien sur les orbites. L'échelle et la
//! mise en scène réelles des systèmes viendront en M2 (P2). Les corps secrets ne sont pas dessinés.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use sh_data::{BodyKind, Galaxy, Place};

use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::input::{Roll, Steer, Thrust};
use crate::state::{ControlMode, CurrentPlace, ViewLevel};
use crate::ui::theme::{self, UiFonts};
use crate::views::{WorldLabel, view_banner};

pub struct GlobalViewPlugin;

impl Plugin for GlobalViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Global), spawn_global)
            .add_systems(Update, fly.run_if(in_state(ViewLevel::Global)));
    }
}

const SCOPE: DespawnOnExit<ViewLevel> = DespawnOnExit(ViewLevel::Global);
/// Agrandissement de la disposition schématique de la carte, pour la vue de test.
const LAYOUT_SCALE: f32 = 40.0;

/// Le système à afficher pour un lieu : le lieu lui-même s'il est un système, sinon le système
/// de l'astre ou de la station.
fn system_of<'a>(galaxy: &'a Galaxy, place: &str) -> Option<&'a sh_data::StarSystem> {
    let id = sh_core::Id::<Place>::new(place).ok()?;
    galaxy.system(&id).or_else(|| {
        let system = galaxy
            .body(&id)
            .and_then(|b| b.system.clone())
            .or_else(|| galaxy.station(&id).and_then(|s| s.system.clone()))?;
        galaxy.system(&system)
    })
}

fn spawn_global(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    place: Res<CurrentPlace>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mode: ResMut<NextState<ControlMode>>,
) {
    mode.set(ControlMode::Piloting);
    let g = &data.galaxy;
    let place_id = place.0.clone().unwrap_or_else(|| "test".into());
    let system = system_of(g, &place_id).filter(|s| !s.secret);

    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(48, 24));
    let station_mesh = meshes.add(Cuboid::new(1.0, 0.4, 1.0));

    // L'étoile, au centre (sans nom quand le canon ne la nomme pas).
    commands.spawn((
        Name::new("Étoile"),
        SCOPE,
        Mesh3d(sphere.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.85, 0.6),
            emissive: LinearRgba::rgb(40.0, 30.0, 18.0),
            ..default()
        })),
        Transform::from_scale(Vec3::splat(30.0)),
    ));

    let mut focus = Vec3::new(0.0, 0.0, 0.0);
    let mut title_place = place_id.clone();
    if let Some(system) = system {
        title_place = system.name.clone();
        let center = system.center;
        for id in &system.bodies {
            let Some(body) = g.body(id).filter(|b| !b.secret && b.kind != BodyKind::Star) else {
                continue;
            };
            let Some(pos) = body.pos else { continue };
            let offset =
                Vec3::new((pos.x - center.x) as f32, 0.0, (pos.y - center.y) as f32) * LAYOUT_SCALE;
            // Un corps posé sur l'étoile dans la carte schématique est écarté pour rester visible.
            let offset = if offset.length() < 60.0 {
                offset + Vec3::new(90.0, 0.0, 0.0)
            } else {
                offset
            };
            let radius = match body.kind {
                BodyKind::Moon => 3.0,
                BodyKind::Belt => continue,
                _ => 8.0,
            };
            commands.spawn((
                Name::new(body.name.clone()),
                SCOPE,
                Mesh3d(sphere.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.45, 0.52, 0.60),
                    perceptual_roughness: 0.9,
                    ..default()
                })),
                Transform::from_translation(offset).with_scale(Vec3::splat(radius)),
            ));
            commands.spawn(label(&fonts, &body.name, offset));
            if id.as_str() == place_id {
                focus = offset;
            }
        }
        for id in &system.stations {
            let Some(station) = g.station(id).filter(|s| !s.secret) else {
                continue;
            };
            let pos = station.pos;
            let offset =
                Vec3::new((pos.x - center.x) as f32, 0.0, (pos.y - center.y) as f32) * LAYOUT_SCALE;
            commands.spawn((
                Name::new(station.name.clone()),
                SCOPE,
                Mesh3d(station_mesh.clone()),
                MeshMaterial3d(materials.add(Color::srgb(0.75, 0.66, 0.45))),
                Transform::from_translation(offset).with_scale(Vec3::splat(10.0)),
            ));
            commands.spawn(label(&fonts, &station.name, offset));
        }
    } else if place_id != "test" {
        warn!("vue Globale : aucun système public pour « {place_id} »");
    }

    // La lumière de l'étoile, orientée vers le corps visé (une lumière directionnelle éclaire
    // uniformément tout le système de test, sans portée ni atténuation à régler).
    commands.spawn((
        SCOPE,
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            ..default()
        },
        Transform::default().looking_to(focus.normalize_or(Vec3::NEG_Z), Vec3::Y),
    ));

    commands.spawn((
        Name::new("Caméra de la vue Globale"),
        SCOPE,
        WorldCamera,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Transform::from_translation(focus + Vec3::new(-40.0, 60.0, 160.0))
            .looking_at(focus, Vec3::Y),
    ));

    commands.spawn(view_banner(
        &texts,
        &fonts,
        texts.format("view.global.title", &[("place", &title_place)]),
        "view.global.hint",
        SCOPE,
    ));
}

fn label(fonts: &UiFonts, name: &str, anchor: Vec3) -> impl Bundle {
    (
        SCOPE,
        WorldLabel(anchor),
        Text::new(name),
        fonts.body(14.0),
        TextColor(theme::TEXT),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
    )
}

/// Vol libre de test : poussée 3 axes, souris pour orienter, Q/E pour le roulis.
fn fly(
    time: Res<Time>,
    thrust: Single<&Action<Thrust>>,
    steer: Single<&Action<Steer>>,
    roll: Single<&Action<Roll>>,
    mut camera: Single<&mut Transform, With<WorldCamera>>,
) {
    let dt = time.delta_secs();
    let steer: Vec2 = ***steer;
    let roll: f32 = ***roll;
    let rotation = Quat::from_euler(
        EulerRot::YXZ,
        -steer.x * 0.002,
        -steer.y * 0.002,
        -roll * 1.2 * dt,
    );
    camera.rotation *= rotation;
    let thrust: Vec3 = ***thrust;
    let movement = camera.rotation * thrust * 120.0 * dt;
    camera.translation += movement;
}
