//! Portes coulissantes des portails de genre `Door`.
//!
//! Une porte est aussi une **porte de chargement** : elle ne s'ouvre que lorsque la cellule de
//! l'autre côté est prête en qualité Locale ; l'animation d'ouverture couvre la fin d'un
//! chargement si nécessaire. Elle se referme seule quand plus personne n'est à proximité.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;

use crate::player::Player;
use crate::sector::gltf::ColliderLayer;
use crate::sector::streaming::{CellReady, CellStates};
use crate::sector::{ActiveSector, Layer, v3};

pub struct DoorsPlugin;

impl Plugin for DoorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_doors.run_if(resource_added::<ActiveSector>))
            .add_systems(
                Update,
                (animate_doors, auto_close).run_if(resource_exists::<ActiveSector>),
            );
    }
}

/// Durée d'ouverture ou de fermeture, en secondes.
const SLIDE_TIME: f32 = 0.55;
/// Délai avant fermeture automatique quand plus personne n'est près.
const AUTO_CLOSE_AFTER: f32 = 4.0;
/// Distance sous laquelle une porte reste ouverte (et ne se referme pas sur le joueur).
const KEEP_OPEN_DISTANCE: f32 = 2.2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DoorState {
    Closed,
    /// En attente que la cellule d'en face soit prête.
    Unlocking,
    Opening,
    Open,
    Closing,
}

#[derive(Component, Debug)]
pub struct Door {
    pub portal: String,
    pub a: String,
    pub b: String,
    pub state: DoorState,
    /// Ouverture de 0 (fermée) à 1 (ouverte).
    pub openness: f32,
    pub width: f32,
    pub idle: f32,
    pub panel: Option<Entity>,
}

impl Door {
    pub fn is_open(&self) -> bool {
        matches!(self.state, DoorState::Open | DoorState::Opening)
    }

    pub fn request_open(&mut self) {
        if matches!(self.state, DoorState::Closed | DoorState::Closing) {
            self.state = DoorState::Unlocking;
        }
    }

    pub fn request_close(&mut self) {
        if matches!(
            self.state,
            DoorState::Open | DoorState::Opening | DoorState::Unlocking
        ) {
            self.state = DoorState::Closing;
        }
    }

    pub fn toggle(&mut self) {
        if self.is_open() || self.state == DoorState::Unlocking {
            self.request_close();
        } else {
            self.request_open();
        }
    }
}

/// Le panneau mobile d'une porte.
#[derive(Component)]
struct DoorPanel;

fn spawn_doors(mut commands: Commands, sector: Res<ActiveSector>, asset_server: Res<AssetServer>) {
    let s = &sector.sector;
    let model = asset_server.load(GltfAssetLabel::Scene(0).from_asset(s.door_model.clone()));
    for portal in s
        .portals
        .iter()
        .filter(|p| p.kind == sh_data::PortalKind::Door)
    {
        let base = v3(portal.center) - Vec3::Y * (portal.height / 2.0);
        let normal = v3(portal.normal).normalize_or(Vec3::Z);
        let door = commands
            .spawn((
                Name::new(format!("Porte {}", portal.id)),
                Transform::from_translation(base).looking_to(normal, Vec3::Y),
                Visibility::default(),
                ChildOf(sector.root),
            ))
            .id();
        let panel = commands
            .spawn((
                Name::new("Panneau"),
                DoorPanel,
                WorldAssetRoot(model.clone()),
                ColliderLayer(Layer::Door),
                RigidBody::Kinematic,
                ChildOf(door),
            ))
            .id();
        commands.entity(door).insert(Door {
            portal: portal.id.clone(),
            a: portal.a.clone(),
            b: portal.b.clone(),
            state: DoorState::Closed,
            openness: 0.0,
            width: portal.width,
            idle: 0.0,
            panel: Some(panel),
        });
    }
}

fn animate_doors(
    time: Res<Time>,
    states: Res<CellStates>,
    ready: Query<(), With<CellReady>>,
    mut doors: Query<&mut Door>,
    mut panels: Query<&mut Transform, With<DoorPanel>>,
) {
    let step = time.delta_secs() / SLIDE_TIME;
    for mut door in &mut doors {
        match door.state {
            DoorState::Unlocking => {
                // Porte de chargement : les deux côtés doivent être en qualité Locale.
                let both_ready = states.is_local_ready(&door.a, &ready)
                    && states.is_local_ready(&door.b, &ready);
                if both_ready {
                    door.state = DoorState::Opening;
                }
            }
            DoorState::Opening => {
                door.openness = (door.openness + step).min(1.0);
                if door.openness >= 1.0 {
                    door.state = DoorState::Open;
                    door.idle = 0.0;
                }
            }
            DoorState::Closing => {
                door.openness = (door.openness - step).max(0.0);
                if door.openness <= 0.0 {
                    door.state = DoorState::Closed;
                }
            }
            DoorState::Open | DoorState::Closed => {}
        }
        if let Some(panel) = door.panel
            && let Ok(mut t) = panels.get_mut(panel)
        {
            // Le panneau coulisse dans la cloison ; courbe douce aux extrémités.
            let eased = door.openness * door.openness * (3.0 - 2.0 * door.openness);
            t.translation.x = eased * (door.width + 0.05);
        }
    }
}

fn auto_close(
    time: Res<Time>,
    player: Query<&GlobalTransform, With<Player>>,
    mut doors: Query<(&GlobalTransform, &mut Door)>,
) {
    let player = player.single().map(|p| p.translation()).ok();
    for (transform, mut door) in &mut doors {
        if door.state != DoorState::Open {
            continue;
        }
        let near = player.is_some_and(|p| p.distance(transform.translation()) < KEEP_OPEN_DISTANCE);
        if near {
            door.idle = 0.0;
        } else {
            door.idle += time.delta_secs();
            if door.idle >= AUTO_CLOSE_AFTER {
                door.request_close();
            }
        }
    }
}
