//! Interactions à la première personne : un rayon de visée et une touche.
//!
//! Le rayon part de l'œil du joueur ; il est arrêté par les murs et les portes, si bien qu'on
//! n'interagit jamais à travers une cloison. La cible visée affiche une invite ; la touche
//! d'interaction (ou l'autopilote, via [`InteractRequest`]) déclenche l'action.

use std::collections::BTreeMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::data::Texts;
use crate::input::Interact;
use crate::player::{Player, PlayerCamera, PlayerController, PlayerMode};
use crate::sector::doors::Door;
use crate::sector::gltf::{Interactable, Socket};
use crate::sector::{ActiveSector, Docked, Layer};
use crate::state::ControlMode;
use crate::ui::hud::{Notice, Prompt};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Focus>()
            .init_resource::<Inventory>()
            .add_message::<InteractRequest>()
            .add_observer(on_interact_key)
            .add_systems(
                Update,
                (find_focus, update_prompt, perform)
                    .chain()
                    .after(crate::player::PlayerSystems)
                    .run_if(resource_exists::<ActiveSector>),
            );
    }
}

/// Portée de la main, en mètres.
pub const REACH: f32 = 2.2;
/// On ne ferme pas une porte sur soi.
const DOOR_CLEARANCE: f32 = 0.9;

/// La cible visée, s'il y en a une.
#[derive(Resource, Default, Debug)]
pub struct Focus(pub Option<Target>);

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    /// Entité qui porte l'[`Interactable`] ou la [`Door`].
    pub entity: Entity,
    pub kind: Interactable,
}

/// Demande d'interaction avec la cible visée (touche ou autopilote).
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct InteractRequest;

/// Objets ramassés (l'inventaire complet arrive en M5).
#[derive(Resource, Default, Debug)]
pub struct Inventory {
    pub items: BTreeMap<String, u32>,
}

fn on_interact_key(_: On<Start<Interact>>, mut requests: MessageWriter<InteractRequest>) {
    requests.write(InteractRequest);
}

/// Remonte la hiérarchie depuis une entité touchée jusqu'à ce qui est interactif.
fn interactive_ancestor(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    interactables: &Query<&Interactable>,
    doors: &Query<&Door>,
) -> Option<Target> {
    for _ in 0..8 {
        if let Ok(kind) = interactables.get(entity) {
            return Some(Target {
                entity,
                kind: kind.clone(),
            });
        }
        if doors.contains(entity) {
            return Some(Target {
                entity,
                kind: Interactable::Door,
            });
        }
        entity = parents.get(entity).ok()?.parent();
    }
    None
}

fn find_focus(
    spatial: SpatialQuery,
    camera: Query<&GlobalTransform, With<PlayerCamera>>,
    player: Query<&PlayerController, With<Player>>,
    parents: Query<&ChildOf>,
    interactables: Query<&Interactable>,
    doors: Query<&Door>,
    mut focus: ResMut<Focus>,
) {
    let (Ok(eye), Ok(controller)) = (camera.single(), player.single()) else {
        focus.0 = None;
        return;
    };
    // Assis, la seule action possible est de se lever.
    if let PlayerMode::Seated { seat, .. } = controller.mode {
        let kind = interactables
            .get(seat)
            .cloned()
            .unwrap_or(Interactable::Seat {
                role: String::new(),
            });
        focus.0 = Some(Target { entity: seat, kind });
        return;
    }
    let filter = SpatialQueryFilter::from_mask([Layer::Static, Layer::Door, Layer::Interact]);
    let hit = spatial.cast_ray(eye.translation(), eye.forward(), REACH, true, &filter);
    let target = hit.and_then(|h| interactive_ancestor(h.entity, &parents, &interactables, &doors));
    if focus.0 != target {
        focus.0 = target;
    }
}

fn update_prompt(
    focus: Res<Focus>,
    texts: Res<Texts>,
    mode: Res<State<ControlMode>>,
    docked: Option<Res<Docked>>,
    player: Query<&PlayerController, With<Player>>,
    doors: Query<&Door>,
    mut prompt: ResMut<Prompt>,
) {
    let docked = docked.is_some();
    let seated = player
        .single()
        .is_ok_and(|p| matches!(p.mode, PlayerMode::Seated { .. }));
    let text = focus.0.as_ref().map(|target| match &target.kind {
        Interactable::Door => {
            let open = doors.get(target.entity).is_ok_and(Door::is_open);
            texts.get(if open {
                "interact.door.close"
            } else {
                "interact.door.open"
            })
        }
        Interactable::Console { label } => texts.format(
            "interact.console",
            &[("console", &texts.get(&format!("console.{label}.name")))],
        ),
        // Aux commandes, E fait rouler le vaisseau : on se lève avec F (contexte Pilotage).
        Interactable::Seat { .. } if seated && *mode.get() == ControlMode::Piloting => {
            texts.get(if docked {
                "interact.seat.leave_helm_docked"
            } else {
                "interact.seat.leave_helm"
            })
        }
        Interactable::Seat { .. } if seated => texts.get("interact.seat.leave"),
        Interactable::Seat { .. } => texts.get("interact.seat.sit"),
        Interactable::Pickup { item } => texts.format(
            "interact.pickup",
            &[("item", &texts.get(&format!("item.{item}")))],
        ),
        Interactable::Ladder => texts.get("interact.ladder"),
    });
    if prompt.0 != text {
        prompt.0 = text;
    }
}

/// L'œil d'un siège : le point d'attache `SOCKET_<nom>_eye`, voisin du repère `INTERACT_<nom>`
/// (convention de tools/blender/kit/ship_interior.py).
pub fn seat_eye(
    seat: Entity,
    names: &Query<&Name>,
    parents: &Query<&ChildOf>,
    children: &Query<&Children>,
    sockets: &Query<(&Socket, &GlobalTransform)>,
) -> Option<Transform> {
    let wanted = names
        .get(seat)
        .ok()
        .and_then(|n| n.as_str().strip_prefix("INTERACT_"))
        .map(|n| format!("{n}_eye"))?;
    let parent = parents.get(seat).map(ChildOf::parent).ok()?;
    children
        .get(parent)
        .into_iter()
        .flatten()
        .filter_map(|e| sockets.get(*e).ok())
        .find(|(socket, _)| socket.usage == "seat_eye" && socket.name == wanted)
        .map(|(_, t)| t.compute_transform())
}

/// S'asseoir : le joueur s'attache à l'œil du siège.
pub fn sit(controller: &mut PlayerController, seat: Entity, eye: Transform) {
    let facing = eye.rotation.to_euler(EulerRot::YXZ).0;
    controller.mode = PlayerMode::Seated {
        seat,
        eye: eye.translation,
        facing,
    };
    controller.yaw = facing;
    controller.pitch = -0.1;
}

/// S'asseoir sur le siège de ce rôle (autopilote) ; faux si le siège n'est pas chargé.
pub fn sit_at(world: &mut World, role: &str) -> bool {
    let mut state: bevy::ecs::system::SystemState<(
        Query<(Entity, &Interactable)>,
        Query<&Name>,
        Query<&ChildOf>,
        Query<&Children>,
        Query<(&Socket, &GlobalTransform)>,
        Query<&mut PlayerController, With<Player>>,
    )> = bevy::ecs::system::SystemState::new(world);
    let Ok((seats, names, parents, children, sockets, mut player)) = state.get_mut(world) else {
        return false;
    };
    let Some((seat, _)) = seats
        .iter()
        .find(|(_, i)| matches!(i, Interactable::Seat { role: r } if r == role))
    else {
        return false;
    };
    let Some(eye) = seat_eye(seat, &names, &parents, &children, &sockets) else {
        return false;
    };
    let Ok(mut controller) = player.single_mut() else {
        return false;
    };
    sit(&mut controller, seat, eye);
    true
}

/// Se lever : un pas devant le siège.
pub fn stand_up(world: &mut World) {
    let mut query = world.query_filtered::<(&mut Transform, &mut PlayerController), With<Player>>();
    let Ok((mut transform, mut controller)) = query.single_mut(world) else {
        return;
    };
    let PlayerMode::Seated { facing, eye, .. } = controller.mode else {
        return;
    };
    let forward = Quat::from_rotation_y(facing) * Vec3::NEG_Z;
    let feet = eye - Vec3::Y * controller.eye_height() + forward * 0.75;
    controller.mode = PlayerMode::Walking;
    controller.velocity = Vec3::ZERO;
    transform.translation = feet + Vec3::Y * 0.05;
}

fn perform(
    mut commands: Commands,
    mut requests: MessageReader<InteractRequest>,
    focus: Res<Focus>,
    texts: Res<Texts>,
    mut inventory: ResMut<Inventory>,
    mut notices: MessageWriter<Notice>,
    mut doors: Query<(&GlobalTransform, &mut Door)>,
    sockets: Query<(&Socket, &GlobalTransform)>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
    mut player: Single<(&GlobalTransform, &mut PlayerController), With<Player>>,
    (data, living, location): (
        Res<crate::data::Data>,
        Option<(
            Res<crate::living::LivingWorld>,
            Res<crate::living::PlayerKnowledge>,
        )>,
        Option<Res<crate::voyage::Location>>,
    ),
) {
    let living = living.as_ref().map(|(w, k)| (&**w, &**k));
    if requests.read().count() == 0 {
        return;
    }
    let Some(target) = focus.0.clone() else {
        return;
    };
    let (player_transform, controller) = &mut *player;
    match &target.kind {
        Interactable::Door => {
            if let Ok((door_transform, mut door)) = doors.get_mut(target.entity) {
                let inside = player_transform
                    .translation()
                    .distance(door_transform.translation())
                    < DOOR_CLEARANCE;
                if !(door.is_open() && inside) {
                    door.toggle();
                }
            }
        }
        Interactable::Console { label } => {
            // Les communications donnent les nouvelles ; le terminal du marché, les prix du
            // monde vivant ; les autres consoles, leur message.
            let text = match (label.as_str(), &living) {
                ("comms", Some((_, knowledge))) => {
                    crate::living::latest_news(&texts, &data, knowledge, 3)
                }
                ("market", Some((world, _))) => match location.as_ref() {
                    Some(l) => crate::living::market_board(&texts, &data, world, &l.system),
                    None => texts.get("console.market.message"),
                },
                _ => texts.get(&format!("console.{label}.message")),
            };
            notices.write(Notice(text));
        }
        Interactable::Seat { .. } => match controller.mode {
            PlayerMode::Seated { .. } => {
                commands.queue(stand_up);
            }
            PlayerMode::Walking => {
                match seat_eye(target.entity, &names, &parents, &children, &sockets) {
                    Some(eye) => sit(controller, target.entity, eye),
                    None => warn!("siège sans point d'attache SOCKET_*_eye"),
                }
            }
        },
        Interactable::Pickup { item } => {
            *inventory.items.entry(item.clone()).or_default() += 1;
            notices.write(Notice(texts.format(
                "notice.picked_up",
                &[("item", &texts.get(&format!("item.{item}")))],
            )));
            commands.entity(target.entity).despawn();
        }
        Interactable::Ladder => {
            notices.write(Notice(texts.get("notice.ladder")));
        }
    }
}
