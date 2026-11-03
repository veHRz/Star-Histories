//! Le joueur à pied : contrôleur à la première personne (avian3d).
//!
//! Contrôleur **cinématique** : le joueur n'est pas un corps simulé, il se déplace par
//! « move and slide » (`avian3d::MoveAndSlide`), glisse le long des murs, monte les marches
//! (jusqu'à [`STEP_HEIGHT`]) et les pentes (jusqu'à [`MAX_SLOPE_DEG`]), saute, court, s'accroupit.
//! Les entrées passent par [`PlayerIntent`], que remplissent soit les actions du contexte
//! À pied, soit un scénario de l'autopilote : le contrôleur ne sait pas d'où elles viennent.
//!
//! La gravité à bord est une proposition (P14) : 1 g artificielle dans les vaisseaux.

use avian3d::prelude::*;
use bevy::camera::Exposure;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_enhanced_input::prelude::*;

use crate::graphics::WorldCamera;
use crate::input::{Crouch, Jump, Look, Move, Sprint};
use crate::sector::streaming::{CellReady, CellStates};
use crate::sector::{Aboard, ActiveSector, Layer, v3};
use crate::state::{ControlMode, ViewLevel};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerIntent>()
            .init_resource::<ScriptedControl>()
            .add_systems(Update, spawn_player.run_if(resource_added::<ActiveSector>))
            .add_systems(
                Update,
                (read_input, move_player, update_head)
                    .chain()
                    .in_set(PlayerSystems)
                    .run_if(resource_exists::<ActiveSector>),
            )
            .add_systems(OnEnter(ViewLevel::Sector), grab_cursor)
            .add_systems(OnExit(ViewLevel::Sector), release_cursor)
            .add_systems(OnEnter(ViewLevel::Global), grab_cursor)
            .add_systems(OnExit(ViewLevel::Global), release_cursor);
    }
}

/// Systèmes du joueur (entrées → déplacement → caméra).
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerSystems;

pub const RADIUS: f32 = 0.3;
pub const STAND_HEIGHT: f32 = 1.75;
pub const CROUCH_HEIGHT: f32 = 1.15;
/// Distance entre le sommet de la tête et les yeux.
const EYE_FROM_TOP: f32 = 0.12;
pub const WALK_SPEED: f32 = 3.2;
pub const RUN_SPEED: f32 = 5.6;
pub const CROUCH_SPEED: f32 = 1.6;
const GROUND_ACCEL: f32 = 14.0;
const AIR_ACCEL: f32 = 2.5;
const JUMP_SPEED: f32 = 4.6;
const GRAVITY: f32 = 9.81;
pub const STEP_HEIGHT: f32 = 0.36;
pub const MAX_SLOPE_DEG: f32 = 46.0;
const GROUND_PROBE: f32 = 0.08;
/// Avancée minimale d'une montée de marche, en mètres. Posée sur le nez de la marche avec moins
/// de `RADIUS · (1 − sin(pente max.))` ≈ 8 cm de débord, la capsule touche l'arête sous une
/// normale trop inclinée et glisse en arrière à la frame suivante : à haut taux de
/// rafraîchissement (6 mm d'avancée par frame à 500 i/s), on ne montait plus. On sonde donc
/// toujours au moins cette distance (marge × 1,5), quelle que soit la durée de la frame.
const STEP_PROBE: f32 = 0.13;
const MOUSE_SENSITIVITY: f32 = 0.0022;
/// Délai après que la cellule du joueur est prête avant d'activer la physique (les collisions
/// sont construites au frame suivant, puis entrent dans l'arbre des requêtes).
const SETTLE_TIME: f32 = 0.2;

#[derive(Component)]
pub struct Player;

/// La caméra à la première personne, enfant du joueur.
#[derive(Component)]
pub struct PlayerCamera;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerMode {
    Walking,
    /// Assis : le joueur est attaché à l'œil d'un siège.
    Seated {
        seat: Entity,
        eye: Vec3,
        facing: f32,
    },
}

#[derive(Component, Debug)]
pub struct PlayerController {
    pub velocity: Vec3,
    pub grounded: bool,
    pub height: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub mode: PlayerMode,
    settle: f32,
}

impl PlayerController {
    pub fn eye_height(&self) -> f32 {
        self.height - EYE_FROM_TOP
    }
}

/// Ce que le joueur veut faire ce frame, quelle que soit la source.
#[derive(Resource, Default, Debug, Clone)]
pub struct PlayerIntent {
    /// x = droite, y = avant, longueur ≤ 1.
    pub movement: Vec2,
    /// Rotation de visée en radians (lacet, tangage).
    pub look: Vec2,
    pub jump: bool,
    pub sprint: bool,
    pub crouch: bool,
}

/// Quand il est actif, l'autopilote remplit [`PlayerIntent`] et les entrées sont ignorées.
#[derive(Resource, Default)]
pub struct ScriptedControl(pub bool);

fn spawn_player(
    mut commands: Commands,
    sector: Res<ActiveSector>,
    aboard: Option<Res<Aboard>>,
    mut mode: ResMut<NextState<ControlMode>>,
) {
    mode.set(ControlMode::OnFoot);
    let s = &sector.sector;
    // `SH_SPAWN=<id>` (autopilote) choisit le point d'apparition ; sinon celui du départ quand
    // on est à bord en vol, sinon le premier du secteur.
    let wanted = std::env::var("SH_SPAWN")
        .ok()
        .or_else(|| aboard.map(|a| a.spawn.clone()));
    let spawn = wanted
        .as_deref()
        .and_then(|id| s.spawns.iter().find(|sp| sp.id == id))
        .or_else(|| s.spawns.first());
    let Some(spawn) = spawn else {
        warn!("secteur {} sans point d'apparition", s.id);
        return;
    };
    let yaw = spawn.heading_deg.to_radians();
    let player = commands
        .spawn((
            Name::new("Joueur"),
            Player,
            PlayerController {
                velocity: Vec3::ZERO,
                grounded: false,
                height: STAND_HEIGHT,
                yaw,
                pitch: 0.0,
                mode: PlayerMode::Walking,
                settle: 0.0,
            },
            Transform::from_translation(v3(spawn.pos)).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            sector.scope(),
        ))
        .id();
    commands.spawn((
        Name::new("Caméra du joueur"),
        PlayerCamera,
        WorldCamera,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 75f32.to_radians(),
            near: 0.05,
            ..default()
        }),
        Exposure::INDOOR,
        Transform::from_xyz(0.0, STAND_HEIGHT - EYE_FROM_TOP, 0.0),
        ChildOf(player),
    ));
    info!(
        "joueur placé à l'apparition « {} » ({})",
        spawn.id, spawn.cell
    );
    // `SH_SPAWN_AT=x,y,z,cap,tangage` (autopilote, degrés) : position libre pour les captures.
    if let Some(values) = std::env::var("SH_SPAWN_AT").ok().map(|v| {
        v.split(',')
            .filter_map(|n| n.trim().parse::<f32>().ok())
            .collect::<Vec<_>>()
    }) && values.len() == 5
    {
        let yaw = values[3].to_radians();
        commands.entity(player).insert((
            Transform::from_xyz(values[0], values[1], values[2])
                .with_rotation(Quat::from_rotation_y(yaw)),
            PlayerController {
                velocity: Vec3::ZERO,
                grounded: false,
                height: STAND_HEIGHT,
                yaw,
                pitch: values[4].to_radians(),
                mode: PlayerMode::Walking,
                settle: 0.0,
            },
        ));
    }
}

fn read_input(
    scripted: Res<ScriptedControl>,
    mode: Res<State<ControlMode>>,
    mut intent: ResMut<PlayerIntent>,
    movement: Query<&Action<Move>>,
    look: Query<&Action<Look>>,
    jump: Query<&Action<Jump>>,
    sprint: Query<&Action<Sprint>>,
    crouch: Query<&Action<Crouch>>,
) {
    if scripted.0 {
        return;
    }
    if *mode.get() != ControlMode::OnFoot {
        *intent = PlayerIntent::default();
        return;
    }
    let get_vec2 = |q: &Query<&Action<Move>>| q.iter().next().map_or(Vec2::ZERO, |a| **a);
    intent.movement = get_vec2(&movement).clamp_length_max(1.0);
    let look: Vec2 = look.iter().next().map_or(Vec2::ZERO, |a| **a);
    intent.look = look * MOUSE_SENSITIVITY;
    intent.jump = jump.iter().next().is_some_and(|a| **a);
    intent.sprint = sprint.iter().next().is_some_and(|a| **a);
    intent.crouch = crouch.iter().next().is_some_and(|a| **a);
}

fn capsule(height: f32) -> Collider {
    Collider::capsule(RADIUS, (height - 2.0 * RADIUS).max(0.01))
}

fn walkable(normal: Vec3) -> bool {
    normal.y >= MAX_SLOPE_DEG.to_radians().cos()
}

/// Le contact d'un lancer de capsule vers le bas repose-t-il sur un sol praticable ?
/// Sur une arête (nez de marche), la normale du contact suit l'arrondi de la capsule : elle est
/// d'autant plus inclinée que la capsule déborde peu de l'arête, donc que la frame est courte
/// (refus des marches au-delà de ≈ 150 i/s). On lit alors la normale de la face elle-même, juste
/// au-delà du point de contact.
fn walkable_contact(
    spatial: &SpatialQuery,
    filter: &SpatialQueryFilter,
    center: Vec3,
    hit: &ShapeHitData,
) -> bool {
    if walkable(hit.normal1) {
        return true;
    }
    let outward = Vec2::new(hit.point1.x - center.x, hit.point1.z - center.z).normalize_or_zero();
    if outward == Vec2::ZERO {
        return false;
    }
    let origin = hit.point1 + Vec3::new(outward.x, 0.0, outward.y) * 0.02 + Vec3::Y * 0.05;
    spatial
        .cast_ray(origin, Dir3::NEG_Y, 0.1, true, filter)
        .is_some_and(|ray| ray.distance > 0.0 && walkable(ray.normal))
}

fn move_player(
    time: Res<Time>,
    intent: Res<PlayerIntent>,
    states: Res<CellStates>,
    ready: Query<(), With<CellReady>>,
    spatial: SpatialQuery,
    move_and_slide: MoveAndSlide,
    mut player: Single<(&mut Transform, &mut PlayerController), With<Player>>,
) {
    let dt = time.delta_secs().min(1.0 / 20.0);
    let (transform, controller) = &mut *player;
    controller.yaw -= intent.look.x;
    controller.pitch = (controller.pitch - intent.look.y).clamp(-1.5, 1.5);

    if let PlayerMode::Seated { eye, facing, .. } = controller.mode {
        // Assis : les yeux à ceux du siège ; on regarde librement autour du cap du siège.
        let feet = eye - Vec3::Y * controller.eye_height();
        transform.translation = feet;
        controller.yaw = controller.yaw.clamp(facing - 1.6, facing + 1.6);
        transform.rotation = Quat::from_rotation_y(controller.yaw);
        controller.velocity = Vec3::ZERO;
        return;
    }
    transform.rotation = Quat::from_rotation_y(controller.yaw);

    // Pas de physique tant que la cellule du joueur n'a pas ses collisions.
    let cell_ready = states
        .current
        .as_deref()
        .is_some_and(|c| states.is_local_ready(c, &ready));
    if !cell_ready {
        controller.settle = 0.0;
        return;
    }
    if controller.settle < SETTLE_TIME {
        controller.settle += dt;
        return;
    }

    let filter = SpatialQueryFilter::from_mask([Layer::Static, Layer::Door]);
    let config = MoveAndSlideConfig::default();

    // Accroupi : on ne se relève que s'il y a la place au-dessus.
    let target_height = if intent.crouch {
        CROUCH_HEIGHT
    } else {
        STAND_HEIGHT
    };
    if target_height > controller.height {
        let shape = capsule(controller.height);
        let center = transform.translation + Vec3::Y * controller.height / 2.0;
        let room = spatial
            .cast_shape(
                &shape,
                center,
                Quat::IDENTITY,
                Dir3::Y,
                &ShapeCastConfig::from_max_distance(target_height - controller.height),
                &filter,
            )
            .map_or(target_height - controller.height, |hit| hit.distance);
        controller.height += room.min(dt * 4.0);
    } else {
        controller.height = (controller.height - dt * 4.0).max(target_height);
    }

    // Vitesse horizontale voulue, dans le repère du joueur.
    let speed = if intent.crouch {
        CROUCH_SPEED
    } else if intent.sprint {
        RUN_SPEED
    } else {
        WALK_SPEED
    };
    let forward = Quat::from_rotation_y(controller.yaw) * Vec3::NEG_Z;
    let right = Quat::from_rotation_y(controller.yaw) * Vec3::X;
    let wish = (forward * intent.movement.y + right * intent.movement.x) * speed;
    let accel = if controller.grounded {
        GROUND_ACCEL
    } else {
        AIR_ACCEL
    };
    let horizontal = Vec2::new(controller.velocity.x, controller.velocity.z)
        .move_towards(Vec2::new(wish.x, wish.z), accel * speed.max(1.0) * dt);
    controller.velocity.x = horizontal.x;
    controller.velocity.z = horizontal.y;

    let was_grounded = controller.grounded;
    if was_grounded && intent.jump {
        controller.velocity.y = JUMP_SPEED;
        controller.grounded = false;
    } else if was_grounded {
        controller.velocity.y = 0.0;
    } else {
        controller.velocity.y -= GRAVITY * dt;
    }

    let height = controller.height;
    let shape = capsule(height);
    let center = transform.translation + Vec3::Y * height / 2.0;
    let out = move_and_slide.move_and_slide(
        &shape,
        center,
        Quat::IDENTITY,
        controller.velocity,
        time.delta().min(std::time::Duration::from_millis(50)),
        &config,
        &filter,
        |_| MoveAndSlideHitResponse::Accept,
    );
    let mut new_center = out.position;
    if !controller.grounded || controller.velocity.y > 0.0 {
        controller.velocity.y = out.projected_velocity.y;
    }

    // Marches : si l'avancée horizontale est bloquée au sol, on retente surélevé.
    let desired = Vec2::new(controller.velocity.x, controller.velocity.z) * dt;
    let achieved = Vec2::new(new_center.x - center.x, new_center.z - center.z);
    if was_grounded && desired.length() > 1e-4 && achieved.length() < desired.length() * 0.6 {
        let lift = spatial
            .cast_shape(
                &shape,
                center,
                Quat::IDENTITY,
                Dir3::Y,
                &ShapeCastConfig::from_max_distance(STEP_HEIGHT),
                &filter,
            )
            .map_or(STEP_HEIGHT, |hit| (hit.distance - 0.01).max(0.0));
        if lift > 0.05 {
            let raised = center + Vec3::Y * lift;
            let flat = Vec3::new(controller.velocity.x, 0.0, controller.velocity.z);
            let step_dt = time.delta().min(std::time::Duration::from_millis(50));
            // Sonde surélevée, sur au moins STEP_PROBE : y a-t-il un sol praticable au-delà ?
            let probe_scale = (STEP_PROBE / desired.length()).max(1.0);
            let probe = move_and_slide.move_and_slide(
                &shape,
                raised,
                Quat::IDENTITY,
                flat * probe_scale,
                step_dt,
                &config,
                &filter,
                |_| MoveAndSlideHitResponse::Accept,
            );
            let probed = Vec2::new(probe.position.x - raised.x, probe.position.z - raised.z);
            let landing = spatial.cast_shape(
                &shape,
                probe.position,
                Quat::IDENTITY,
                Dir3::NEG_Y,
                &ShapeCastConfig::from_max_distance(lift + GROUND_PROBE),
                &filter,
            );
            if probed.length() > achieved.length() + STEP_PROBE * 0.25
                && let Some(hit) =
                    landing.filter(|h| walkable_contact(&spatial, &filter, probe.position, h))
            {
                // Toute l'avancée sondée : la capsule se pose franchement sur la marche.
                new_center = probe.position - Vec3::Y * hit.distance;
            }
        }
    }

    // Sol : accroche au sol (y compris en descendant des marches) si l'on n'est pas en saut.
    let probe = if was_grounded && controller.velocity.y <= 0.0 {
        STEP_HEIGHT + GROUND_PROBE
    } else {
        GROUND_PROBE
    };
    let ground = spatial.cast_shape(
        &shape,
        new_center,
        Quat::IDENTITY,
        Dir3::NEG_Y,
        &ShapeCastConfig::from_max_distance(probe),
        &filter,
    );
    match ground.filter(|h| walkable_contact(&spatial, &filter, new_center, h)) {
        Some(hit) if controller.velocity.y <= 0.1 => {
            new_center.y -= (hit.distance - 0.005).max(0.0);
            controller.grounded = true;
            controller.velocity.y = 0.0;
        }
        _ => controller.grounded = false,
    }

    transform.translation = new_center - Vec3::Y * height / 2.0;
}

fn update_head(
    player: Single<&PlayerController, With<Player>>,
    mut camera: Single<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    camera.translation = Vec3::Y * player.eye_height();
    camera.rotation = Quat::from_rotation_x(player.pitch);
}

fn grab_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

fn release_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}
