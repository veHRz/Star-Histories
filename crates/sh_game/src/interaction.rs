//! Interactions à la première personne : un rayon de visée et une touche.
//!
//! Le rayon part de l'œil du joueur ; il est arrêté par les murs et les portes, si bien qu'on
//! n'interagit jamais à travers une cloison. La cible visée (le capteur d'un `INTERACT_*`, un pavé
//! qui couvre l'objet quand le modèle donne ses `half_extents` : on s'assoit en visant la chaise)
//! affiche une invite ; la touche d'interaction (ou l'autopilote, via [`InteractRequest`])
//! déclenche l'action : porte, console, siège, ramassage (l'objet entre dans l'[`Inventory`] et
//! son point de ramassage dans `items::PickedUp` : il ne revient plus), casier
//! (`items::OpenContainer`) et sa plaque de nom (`items::NameContainer`), PNJ de comptoir
//! (`items::TalkTo`), échelle.
//!
//! Refus sans effet (M10) : un casier décoratif ou qui n'est pas au joueur se dit verrouillé,
//! la porte d'un sas qui donne sur le vide (vaisseau en vol) le dit aussi ; la touche n'y fait
//! rien. L'invite d'un PNJ nomme d'abord son rôle, puis l'action sur une seconde ligne.

use std::collections::BTreeMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::data::Texts;
use crate::input::Interact;
use crate::items::{
    Containers, ItemCatalog, NameContainer, OpenContainer, PLAYER_SHIP, PickedUp, TalkTo,
    container_name,
};
use crate::player::{Player, PlayerCamera, PlayerController, PlayerMode};
use crate::sector::doors::Door;
use crate::sector::gltf::{Interactable, PickupId, Socket};
use crate::sector::{ActiveSector, Docked, Layer};
use crate::state::ControlMode;
use crate::ui::character::logic::capitalize;
use crate::ui::hud::{Notice, Prompt};
use crate::ui::terminal::{OpenTerminal, Screen};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        // Une condition par système (AGENTS.md §4) : celle d'un groupe chaîné n'est évaluée
        // qu'une fois, avant le premier.
        let in_sector = resource_exists::<ActiveSector>;
        app.init_resource::<Focus>()
            .init_resource::<Inventory>()
            .add_message::<InteractRequest>()
            .add_observer(on_interact_key)
            .add_systems(
                Update,
                (
                    find_focus.run_if(in_sector),
                    update_prompt.run_if(in_sector),
                    perform.run_if(in_sector),
                )
                    .chain()
                    .after(crate::player::PlayerSystems),
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

/// Objets portés par le joueur, ramassés à pied (montrés dans la fiche du capitaine).
#[derive(Resource, Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
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

/// Une porte dont un côté donne sur l'extérieur (`@dock` : le sas d'un vaisseau qui n'est pas
/// amarré) n'ouvre que sur le vide : elle ne s'ouvre pas, l'invite le dit (M10). À quai,
/// l'assemblage du vaisseau et du quartier a remplacé ce côté par une vraie cellule.
pub fn opens_on_void(a: &str, b: &str) -> bool {
    sh_data::is_external(a) || sh_data::is_external(b)
}

/// Un casier s'ouvre (ou se nomme) s'il a un contenu (pas un casier décoratif) et qu'il est au
/// joueur (`items::owner_of_cell`) ; sinon il est verrouillé.
pub fn usable_locker(container: &str, owner: Option<&str>) -> bool {
    !container.is_empty() && owner == Some(PLAYER_SHIP)
}

/// L'invite d'un casier ou du frigo : son nom s'il en a un, sinon celle de son genre
/// (`<kind_key>.open` : « ouvrir le frigo »), à défaut celle du casier.
fn locker_prompt(
    texts: &Texts,
    catalog: Option<&ItemCatalog>,
    containers: Option<&Containers>,
    container: &str,
    owner: Option<&str>,
) -> String {
    if !usable_locker(container, owner) {
        return texts.get("interact.locker.locked");
    }
    if let Some(name) = container_name(containers, PLAYER_SHIP, container) {
        return texts.format("interact.locker.named", &[("name", name)]);
    }
    let kind = catalog.map_or(sh_data::items::LOCKER_KIND_KEY, |c| c.kind_key(container));
    let key = format!("{kind}.open");
    texts.get(if texts.has(&key) {
        &key
    } else {
        "container.locker.open"
    })
}

/// Ce que l'invite lit des rangements (limite de paramètres d'un système).
#[derive(bevy::ecs::system::SystemParam)]
struct Stores<'w> {
    catalog: Option<Res<'w, ItemCatalog>>,
    containers: Option<Res<'w, Containers>>,
}

fn update_prompt(
    focus: Res<Focus>,
    texts: Res<Texts>,
    mode: Res<State<ControlMode>>,
    docked: Option<Res<Docked>>,
    stores: Stores,
    player: Query<&PlayerController, With<Player>>,
    doors: Query<&Door>,
    mut prompt: ResMut<Prompt>,
) {
    let docked = docked.is_some();
    let seated = player
        .single()
        .is_ok_and(|p| matches!(p.mode, PlayerMode::Seated { .. }));
    let catalog = stores.catalog.as_deref();
    let containers = stores.containers.as_deref();
    let text = focus.0.as_ref().map(|target| match &target.kind {
        Interactable::Door => match doors.get(target.entity) {
            Ok(door) if opens_on_void(&door.a, &door.b) => texts.get("interact.door.void"),
            door => texts.get(if door.is_ok_and(Door::is_open) {
                "interact.door.close"
            } else {
                "interact.door.open"
            }),
        },
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
        Interactable::Locker { container, owner } => {
            locker_prompt(&texts, catalog, containers, container, owner.as_deref())
        }
        Interactable::LockerLabel { container, owner } => {
            if !usable_locker(container, owner.as_deref()) {
                texts.get("interact.locker.locked")
            } else if let Some(name) = container_name(containers, PLAYER_SHIP, container) {
                texts.format("interact.locker_label.named", &[("name", name)])
            } else {
                texts.get("interact.locker_label")
            }
        }
        Interactable::Npc { role, .. } => {
            // Qui l'on vise d'abord, avec sa majuscule, puis l'action sur une seconde ligne ;
            // un rôle sans texte n'affiche pas sa clé.
            let role_key = format!("npc.role.{role}");
            if texts.has(&role_key) {
                texts.format(
                    "interact.npc.with",
                    &[("role", &capitalize(&texts.get(&role_key)))],
                )
            } else {
                texts.get("interact.npc")
            }
        }
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

/// Se lever : un pas devant le siège, ou derrière, ou à côté, là où il y a la place debout. Un
/// siège face à une planche de bord (le pilote de la corvette, M9) n'a pas de place devant : s'y
/// lever coinçait le joueur entre le siège, la planche et le pupitre des gaz.
pub fn stand_up(world: &mut World) {
    let mut query = world.query_filtered::<&PlayerController, With<Player>>();
    let Ok(controller) = query.single(world) else {
        return;
    };
    let PlayerMode::Seated { facing, eye, .. } = controller.mode else {
        return;
    };
    let turn = Quat::from_rotation_y(facing);
    let offsets = [
        Vec3::NEG_Z * 0.75,
        Vec3::Z * 0.8,
        Vec3::NEG_X * 0.75,
        Vec3::X * 0.75,
    ]
    .map(|offset| turn * offset);
    // Repli : comme avant, un pas devant, les yeux à hauteur debout.
    let fallback = eye - Vec3::Y * controller.eye_height() + offsets[0] + Vec3::Y * 0.05;
    let free = {
        let mut state = bevy::ecs::system::SystemState::<SpatialQuery>::new(world);
        let Ok(spatial) = state.get(world) else {
            return;
        };
        let body = Collider::capsule(
            crate::player::RADIUS,
            crate::player::STAND_HEIGHT - 2.0 * crate::player::RADIUS,
        );
        let filter = SpatialQueryFilter::from_mask([Layer::Static, Layer::Door]);
        offsets.into_iter().find_map(|offset| {
            // Le sol sous le pas, sous les yeux du siège ; plus haut qu'un genou, c'est un
            // meuble (la planche de bord), pas un sol.
            let above = eye + offset;
            let floor = spatial.cast_ray(above, Dir3::NEG_Y, 2.5, true, &filter)?;
            let ground = above.y - floor.distance;
            if ground > eye.y - 0.9 {
                return None;
            }
            let feet = Vec3::new(above.x, ground + 0.03, above.z);
            let center = feet + Vec3::Y * (crate::player::STAND_HEIGHT / 2.0 + 0.02);
            spatial
                .shape_intersections(&body, center, Quat::IDENTITY, &filter)
                .is_empty()
                .then_some(feet)
        })
    };
    let mut query = world.query_filtered::<(&mut Transform, &mut PlayerController), With<Player>>();
    let Ok((mut transform, mut controller)) = query.single_mut(world) else {
        return;
    };
    controller.mode = PlayerMode::Walking;
    controller.velocity = Vec3::ZERO;
    transform.translation = free.unwrap_or(fallback);
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
    (data, living, location, active, docked, mut terminals): (
        Res<crate::data::Data>,
        Option<(
            Res<crate::living::LivingWorld>,
            Res<crate::living::PlayerKnowledge>,
        )>,
        Option<Res<crate::voyage::Location>>,
        Option<Res<ActiveSector>>,
        Option<Res<Docked>>,
        MessageWriter<OpenTerminal>,
    ),
    (mut picked, pickup_ids, mut containers, mut namings, mut talks): (
        Option<ResMut<PickedUp>>,
        Query<&PickupId>,
        MessageWriter<OpenContainer>,
        MessageWriter<NameContainer>,
        MessageWriter<TalkTo>,
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
                // Le sas d'un vaisseau en vol : rien derrière, la porte n'est pas sollicitée.
                if opens_on_void(&door.a, &door.b) {
                    return;
                }
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
            // Une console qui ouvre un dialogue (bureau des contrats, taverne) ou le marché
            // ouvre le terminal.
            // À quai, le secteur chargé assemble le vaisseau et le quartier : le quartier compte.
            let sector = docked
                .as_ref()
                .map(|d| d.district.as_str())
                .or(active.as_ref().map(|a| a.sector.id.as_str()));
            let dialogue = data
                .dialogues
                .iter()
                .find(|d| &d.console == label && Some(d.sector.as_str()) == sector);
            if let Some(d) = dialogue {
                terminals.write(OpenTerminal(Screen::Dialogue {
                    id: d.id.clone(),
                    node: String::new(),
                }));
                return;
            }
            if label == "market" && location.is_some() {
                terminals.write(OpenTerminal(Screen::Market));
                return;
            }
            // Les communications donnent les nouvelles ; les autres consoles, leur message.
            let text = match (label.as_str(), &living) {
                ("comms", Some((_, knowledge))) => {
                    crate::living::latest_news(&texts, &data, knowledge, 3)
                }
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
            // Pris pour de bon : son point de ramassage reste vide, même après un chargement.
            if let (Ok(id), Some(picked)) = (pickup_ids.get(target.entity), picked.as_mut()) {
                picked.insert(id.0.clone());
            }
            notices.write(Notice(texts.format(
                "notice.picked_up",
                &[("item", &texts.get(&format!("item.{item}")))],
            )));
            commands.entity(target.entity).despawn();
        }
        // Un casier verrouillé (décoratif, ou qui n'est pas au joueur) ne fait rien : l'invite
        // l'a déjà dit.
        Interactable::Locker { container, owner } => {
            if usable_locker(container, owner.as_deref()) {
                containers.write(OpenContainer(container.clone()));
            }
        }
        Interactable::LockerLabel { container, owner } => {
            if usable_locker(container, owner.as_deref()) {
                namings.write(NameContainer(container.clone()));
            }
        }
        Interactable::Npc { role, variant } => {
            talks.write(TalkTo {
                role: role.clone(),
                variant: variant.clone(),
            });
        }
        Interactable::Ladder => {
            notices.write(Notice(texts.get("notice.ladder")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sas_sur_le_vide() {
        // En vol, le sas donne sur le point d'attache « @dock » : pas de porte à ouvrir.
        assert!(opens_on_void("sas", "@dock"));
        assert!(opens_on_void("@dock", "sas"));
        // À quai, le point d'attache est devenu le quai du hangar.
        assert!(!opens_on_void("sas", "quai"));
        assert!(!opens_on_void("chambre", "couloir"));
    }

    #[test]
    fn casiers_verrouilles() {
        assert!(usable_locker(
            "corvette_chambre_casier_1",
            Some(PLAYER_SHIP)
        ));
        // Décoratif : aucun contenu.
        assert!(!usable_locker("", Some(PLAYER_SHIP)));
        // Celui d'un autre vaisseau, ou d'un lieu qui n'est pas au joueur.
        assert!(!usable_locker("corvette_chambre_casier_1", None));
        assert!(!usable_locker("corvette_chambre_casier_1", Some("autre")));
    }
}
