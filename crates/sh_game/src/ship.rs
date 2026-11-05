//! Le vaisseau du joueur en vol (vue Globale) : pilotage depuis le siège, modèle de vol 6 axes.
//!
//! - **À bord, le vaisseau est le référentiel** : la racine du secteur porte la position et
//!   l'orientation absolues du vaisseau ([`PositionDriver::Frame`]) ; l'intérieur et le joueur ne
//!   bougent pas dans le rendu, c'est l'extérieur qui défile ([`crate::space`]).
//! - **S'asseoir au poste de pilotage** passe au contexte Pilotage (en vol comme à quai, où
//!   `G` demande le départ) ; se lever (`F`) rend le contexte À pied, et l'on marche dans le
//!   vaisseau en plein vol.
//! - **Modèle de vol** ([`step`], logique pure) : vol **assisté** (la vitesse suit la commande,
//!   le vaisseau s'arrête seul), **découplé** (newtonien : la poussée accélère, rien ne freine),
//!   **croisière** (`Boost` : la vitesse suit la limite de l'échelle P2, qui croît avec la
//!   distance à la surface la plus proche). La rotation suit toujours la commande, dans les
//!   limites du maniement (`gameplay/flight.ron`).
//!
//! Les commandes passent par [`PilotIntent`], que remplissent les actions du contexte Pilotage
//! ou un scénario de l'autopilote.
//!
//! **Autopilote** (M10) : [`AutopilotRequest`] désigne une cible du système courant (astre,
//! station, structure ou port : une [`NavTarget`] posée par la vue Globale). Aux commandes, le
//! vaisseau s'aligne, prend la croisière tant qu'il a la place de ralentir, puis freine et
//! s'arrête à la distance d'arrêt de la cible ([`approach`], logique pure, la même que celle du
//! pilote des scénarios). Toute commande du joueur, la bascule du vol assisté ou le lever du
//! siège le désengagent ; la croisière reste interdite près des hostiles (`CruiseLock`).

use bevy::math::DVec3;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use sh_core::SpacePos;
use sh_data::{Handling, SystemScale};

use crate::data::{Data, Texts};
use crate::input::{Boost, LeaveSeat, Roll, Steer, Thrust, ToggleFlightAssist};
use crate::interaction::{seat_eye, sit};
use crate::player::{Player, PlayerController, PlayerMode, PlayerSystems, ScriptedControl};
use crate::sector::gltf::{Interactable, Socket};
use crate::sector::{Aboard, ActiveSector};
use crate::space::{PositionDriver, SpacePosition, SpaceRotation};
use crate::state::{AppState, ControlMode, ViewLevel};
use crate::transition::Sequence;
use crate::ui::hud::Notice;

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PilotIntent>()
            .init_resource::<AutopilotRequest>()
            .add_observer(on_toggle_assist)
            .add_observer(on_leave_seat)
            // L'autopilote est propre à une partie : une partie quittée ne le garde pas.
            .add_systems(
                OnExit(AppState::InGame),
                |mut request: ResMut<AutopilotRequest>| request.0 = None,
            )
            .add_systems(
                Update,
                (
                    (seat_at_start, sync_control_mode),
                    // En vol seulement, et pas pendant une approche automatique (conditions sur
                    // chaque système d'un groupe chaîné, AGENTS.md §4).
                    (
                        read_pilot_input
                            .run_if(in_state(ViewLevel::Global))
                            .run_if(not(resource_exists::<Sequence>)),
                        run_autopilot
                            .run_if(in_state(ViewLevel::Global))
                            .run_if(not(resource_exists::<Sequence>)),
                        fly.run_if(in_state(ViewLevel::Global))
                            .run_if(not(resource_exists::<Sequence>)),
                    )
                        .chain(),
                )
                    .chain()
                    .in_set(ShipSystems)
                    .after(PlayerSystems)
                    .run_if(resource_exists::<ActiveSector>),
            );
    }
}

/// La cible de l'autopilote : l'identifiant d'un astre, d'une station, d'une structure ou d'un
/// port du système courant (`sh_data::scene::nav_points`), ou rien. La carte (ou
/// `SH_AUTOPILOT`) la pose ; le vol la lit en vue Globale et l'efface à l'arrivée.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct AutopilotRequest(pub Option<String>);

/// Une cible de navigation de la vue Globale (posée par `views::global` sur les astres,
/// stations, structures et balises de ports).
#[derive(Component, Clone, Debug)]
pub struct NavTarget {
    pub id: String,
    /// Distance au centre (m) où l'autopilote s'arrête (`sh_data::scene::stop_distance`).
    pub stop: f64,
    /// Un point de livraison des contrats.
    pub delivery: bool,
}

/// Une commande du joueur plus forte que ce seuil désengage l'autopilote (souris : pixels par
/// frame ; un tremblement de la main ne compte pas).
const STEER_DEADZONE: f32 = 1.5;
/// Frames consécutives de souris au-delà du seuil pour reprendre la main.
const STEER_TAKEOVER_FRAMES: u32 = 4;
/// Hors de cet angle (degrés) entre l'avant et la cible, on vire sans pousser.
const ALIGN_DEGREES: f64 = 10.0;
/// Approche de l'autopilote : en croisière, la vitesse visée est le reste du chemin divisé par
/// ce temps (s), sous la limite de croisière. La croisière suit sa consigne au taux 2/s
/// ([`step`]) : le reste du chemin r obéit à r'' + 2 r' + (2 / T) r = 0, sans dépassement dès
/// que T ≥ 2 (amortissement critique) ; 2,5 laisse de la marge aux frames longs.
const APPROACH_SECONDS: f64 = 2.5;
/// Vitesse plancher de la fin d'approche (m/s) : la décroissance exponentielle ne finirait
/// jamais ; à cette vitesse, le vol assisté s'arrête en ≈ 20 m.
const FINAL_SPEED: f64 = 30.0;
/// Arrivé : à moins de cette vitesse (m/s), dans la distance d'arrêt (à cette marge près, m).
const ARRIVED_SPEED: f64 = 2.0;
const ARRIVED_SLACK: f64 = 250.0;
/// Période (s) du relevé de progression de l'autopilote dans le journal.
const AUTOPILOT_LOG_SECONDS: f32 = 10.0;

/// Ce que l'autopilote commande pour ce frame.
#[derive(Clone, Debug, Default)]
pub struct Approach {
    pub intent: PilotIntent,
    /// Arrêté à la distance d'arrêt : l'autopilote a fini.
    pub arrived: bool,
}

/// Distance (m) pour s'arrêter en vol assisté depuis `speed` : la décroissance de la croisière
/// jusqu'à la vitesse de manœuvre, puis le freinage (marche arrière).
pub fn braking_distance(speed: f64, handling: &Handling) -> f64 {
    let brake = |v: f64| v * v / (2.0 * handling.accel_backward.max(1e-3));
    if speed > handling.max_speed {
        handling.cruise_spool_down * (speed - handling.max_speed) + brake(handling.max_speed)
    } else {
        brake(speed)
    }
}

/// Un frame d'approche vers `target`, arrêt à `stop` mètres de lui : virer vers lui ; une fois
/// aligné, la croisière avec une consigne de vitesse qui décroît avec le reste du chemin
/// ([`APPROACH_SECONDS`], [`PilotIntent::cruise_cap`]) : loin, la limite de croisière ; près,
/// un ralentissement continu jusqu'à l'arrêt, sans dépasser la cible. Croisière interdite
/// (`cruise_allowed` faux, hostiles proches) : poussée de manœuvre, coupée quand il ne reste
/// que la distance de freinage. Logique pure (jeu et scénarios).
pub fn approach(
    pos: SpacePos,
    rotation: Quat,
    velocity: DVec3,
    target: SpacePos,
    stop: f64,
    handling: &Handling,
    cruise_allowed: bool,
) -> Approach {
    let to = target - pos;
    let distance = to.length();
    let speed = velocity.length();
    let remaining = distance - stop;
    if remaining <= ARRIVED_SLACK && speed < ARRIVED_SPEED {
        return Approach {
            intent: PilotIntent::default(),
            arrived: true,
        };
    }
    let mut intent = PilotIntent::default();
    if remaining <= 0.0 {
        // Dans la distance d'arrêt : on laisse le vol assisté freiner.
        return Approach {
            intent,
            arrived: false,
        };
    }
    let dir = DVec3::new(to.x, to.y, to.z) / distance.max(1e-6);
    let to_world = rotation.as_dquat();
    let local = to_world.inverse() * dir;
    let yaw = local.x.atan2(-local.z);
    let pitch = local.y.atan2(-local.z);
    intent.turn = Vec2::new(
        (yaw * 3.0).clamp(-1.0, 1.0) as f32,
        (pitch * 3.0).clamp(-1.0, 1.0) as f32,
    );
    let aligned = (to_world * DVec3::NEG_Z).angle_between(dir).to_degrees() < ALIGN_DEGREES;
    if aligned && cruise_allowed {
        intent.cruise = true;
        intent.cruise_cap = Some((remaining / APPROACH_SECONDS).max(FINAL_SPEED));
    } else if aligned && remaining > braking_distance(speed, handling) {
        intent.thrust = Vec3::NEG_Z;
    }
    Approach {
        intent,
        arrived: false,
    }
}

/// Pilotage et vol du vaisseau du joueur.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShipSystems;

/// Mouvement du vaisseau : vitesse absolue (m/s), vitesse de rotation dans ses axes (rad/s).
#[derive(Component, Clone, Debug, Default)]
pub struct ShipMotion {
    pub velocity: DVec3,
    pub angular: Vec3,
    pub mode: FlightMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlightMode {
    /// La vitesse suit la commande ; sans commande, le vaisseau s'arrête.
    #[default]
    Assisted,
    /// Newtonien : la poussée accélère, rien ne freine.
    Decoupled,
}

/// Les commandes de pilotage de ce frame, quelle que soit la source.
#[derive(Resource, Default, Debug, Clone)]
pub struct PilotIntent {
    /// Poussée dans les axes du vaisseau (x droite, y haut, −z avant), composantes dans [−1, 1].
    pub thrust: Vec3,
    /// Tangage et lacet demandés, dans [−1, 1] (fraction des vitesses maximales).
    pub turn: Vec2,
    /// Roulis demandé, dans [−1, 1] (positif : vers la droite).
    pub roll: f32,
    /// Croisière.
    pub cruise: bool,
    /// Consigne de vitesse de croisière plus basse que la limite (m/s) : l'autopilote ralentit
    /// ainsi à l'approche. Jamais posée par le pilotage manuel.
    pub cruise_cap: Option<f64>,
}

/// Sensibilité de la souris en pilotage : pixels par frame pour une rotation à pleine vitesse.
const MOUSE_FULL_TURN: f32 = 12.0;

/// Un pas du modèle de vol. Renvoie la nouvelle orientation ; met à jour `motion`.
/// `nearest_surface` : distance (m) à la surface la plus proche, pour la croisière.
pub fn step(
    motion: &mut ShipMotion,
    orientation: Quat,
    intent: &PilotIntent,
    handling: &Handling,
    scale: &SystemScale,
    nearest_surface: f64,
    dt: f64,
) -> Quat {
    // Rotation : la vitesse angulaire suit la commande (réponse du premier ordre).
    let rates = Vec3::new(
        handling.pitch_rate_deg.to_radians() as f32,
        handling.yaw_rate_deg.to_radians() as f32,
        handling.roll_rate_deg.to_radians() as f32,
    );
    let wanted = Vec3::new(
        intent.turn.y.clamp(-1.0, 1.0) * rates.x,
        -intent.turn.x.clamp(-1.0, 1.0) * rates.y,
        -intent.roll.clamp(-1.0, 1.0) * rates.z,
    );
    let k = (1.0 - (-handling.turn_response * dt).exp()) as f32;
    motion.angular += (wanted - motion.angular) * k;
    let orientation =
        (orientation * Quat::from_scaled_axis(motion.angular * dt as f32)).normalize();
    let to_world = orientation.as_dquat();
    let forward = to_world * DVec3::NEG_Z;

    if intent.cruise {
        // Croisière : vers l'avant seulement, à la vitesse limite de l'échelle ; la vitesse
        // latérale s'éteint.
        // Sous la limite de l'échelle ; l'autopilote peut viser moins (approche).
        let target = intent
            .cruise_cap
            .map_or(f64::MAX, |cap| cap.max(0.0))
            .min(scale.cruise_speed_limit(nearest_surface));
        let along = motion.velocity.dot(forward);
        let blend = 1.0 - (-2.0 * dt).exp();
        let speed = along + (target - along) * blend;
        motion.velocity = forward * speed;
        return orientation;
    }

    let speed = motion.velocity.length();
    if speed > handling.max_speed * 1.01 {
        // Retour de croisière : décélération exponentielle jusqu'à la vitesse de manœuvre.
        let decay = (-dt / handling.cruise_spool_down).exp();
        motion.velocity *= (decay).max(handling.max_speed / speed);
        return orientation;
    }

    let local = to_world.inverse() * motion.velocity;
    let thrust = intent.thrust.clamp(Vec3::splat(-1.0), Vec3::ONE).as_dvec3();
    // Accélération maximale par axe et par sens (−z : avant).
    let accel = |axis: usize, sign: f64| match axis {
        0 => handling.accel_lateral,
        1 => handling.accel_vertical,
        _ if sign < 0.0 => handling.accel_forward,
        _ => handling.accel_backward,
    };
    let mut next = local;
    for axis in 0..3 {
        match motion.mode {
            FlightMode::Assisted => {
                let target = thrust[axis] * handling.max_speed;
                let delta = target - local[axis];
                let limit = accel(axis, delta) * dt;
                next[axis] = local[axis] + delta.clamp(-limit, limit);
            }
            FlightMode::Decoupled => {
                next[axis] = local[axis] + thrust[axis] * accel(axis, thrust[axis]) * dt;
            }
        }
    }
    motion.velocity = to_world * next;
    if motion.velocity.length() > handling.max_speed {
        motion.velocity = motion.velocity.normalize() * handling.max_speed;
    }
    orientation
}

/// Un astre : sa surface compte pour la vitesse de croisière.
#[derive(Component)]
pub struct SurfaceBody {
    pub radius: f64,
}

/// Le départ de partie place le joueur sur un siège (`seat_role`) dès que le siège et son
/// point d'attache sont chargés.
#[allow(clippy::too_many_arguments)]
fn seat_at_start(
    aboard: Option<Res<Aboard>>,
    mut done: Local<bool>,
    seats: Query<(Entity, &Interactable)>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    sockets: Query<(&Socket, &GlobalTransform)>,
    mut player: Query<&mut PlayerController, With<Player>>,
) {
    let Some(role) = aboard.as_ref().and_then(|a| a.seat_role.as_ref()) else {
        return;
    };
    if aboard.as_ref().is_some_and(|a| a.is_added()) {
        *done = false;
    }
    if *done {
        return;
    }
    let Ok(mut controller) = player.single_mut() else {
        return;
    };
    let seat = seats
        .iter()
        .find(|(_, i)| matches!(i, Interactable::Seat { role: r } if r == role));
    let Some((seat, _)) = seat else {
        return;
    };
    if let Some(eye) = seat_eye(seat, &names, &parents, &children, &sockets) {
        sit(&mut controller, seat, eye);
        *done = true;
        info!("à bord : assis au poste « {role} »");
    }
}

/// Assis au poste de pilotage en vol : contexte Pilotage ; sinon À pied.
fn sync_control_mode(
    player: Query<&PlayerController, With<Player>>,
    interactables: Query<&Interactable>,
    mode: Res<State<ControlMode>>,
    mut next: ResMut<NextState<ControlMode>>,
) {
    let Ok(controller) = player.single() else {
        return;
    };
    let piloting = match controller.mode {
        PlayerMode::Seated { seat, .. } => interactables
            .get(seat)
            .is_ok_and(|i| matches!(i, Interactable::Seat { role } if role == "pilot")),
        PlayerMode::Walking => false,
    };
    let wanted = if piloting {
        ControlMode::Piloting
    } else {
        ControlMode::OnFoot
    };
    if *mode.get() != wanted && *mode.get() != ControlMode::Menu {
        next.set(wanted);
    }
}

#[allow(clippy::too_many_arguments)]
fn read_pilot_input(
    scripted: Res<ScriptedControl>,
    mode: Res<State<ControlMode>>,
    thrust: Single<&Action<Thrust>>,
    steer: Single<&Action<Steer>>,
    roll: Single<&Action<Roll>>,
    boost: Single<&Action<Boost>>,
    mut intent: ResMut<PilotIntent>,
    settings: Res<crate::options::UserSettings>,
    (mut autopilot, texts, mut notices): (
        ResMut<AutopilotRequest>,
        Res<Texts>,
        MessageWriter<Notice>,
    ),
    mut steer_streak: Local<u32>,
) {
    if scripted.0 {
        return;
    }
    if *mode.get() != ControlMode::Piloting {
        *intent = PilotIntent::default();
        *steer_streak = 0;
        return;
    }
    let raw_steer: Vec2 = ***steer;
    let steer = raw_steer * settings.look_scale();
    *intent = PilotIntent {
        thrust: ***thrust,
        turn: Vec2::new(steer.x, -steer.y) / MOUSE_FULL_TURN,
        roll: ***roll,
        cruise: ***boost,
        cruise_cap: None,
    };
    // Le joueur reprend la main : l'autopilote se désengage. Une touche suffit ; la souris doit
    // bouger plusieurs frames de suite (un saut isolé du curseur, à la prise de la souris par la
    // fenêtre, l'a désengagé à tort, M10).
    *steer_streak = if raw_steer.length() > STEER_DEADZONE {
        steer_streak.saturating_add(1)
    } else {
        0
    };
    let keys = intent.thrust != Vec3::ZERO || intent.roll != 0.0 || intent.cruise;
    let mouse = *steer_streak >= STEER_TAKEOVER_FRAMES;
    if (keys || mouse) && autopilot.0.is_some() {
        info!(
            "autopilote : le joueur reprend la main ({})",
            if keys { "touches" } else { "souris" }
        );
        disengage(&mut autopilot, &texts, &mut notices);
    }
}

/// Coupe l'autopilote, et le dit.
fn disengage(request: &mut AutopilotRequest, texts: &Texts, notices: &mut MessageWriter<Notice>) {
    if request.0.take().is_some() {
        info!("autopilote désengagé");
        notices.write(Notice(texts.get("nav.autopilot.off")));
    }
}

/// L'autopilote, aux commandes : il remplace les commandes du frame par celles de l'approche
/// de la cible. Pas aux commandes (arrivée, pas encore assis) : il attend.
#[allow(clippy::too_many_arguments)]
fn run_autopilot(
    time: Res<Time>,
    mut request: ResMut<AutopilotRequest>,
    mode: Res<State<ControlMode>>,
    data: Res<Data>,
    texts: Res<Texts>,
    sector: Res<ActiveSector>,
    targets: Query<(&NavTarget, &SpacePosition, &Name)>,
    bodies: Query<(&SpacePosition, &SurfaceBody)>,
    mut ships: Query<(&SpacePosition, &SpaceRotation, &mut ShipMotion), Without<NavTarget>>,
    mut intent: ResMut<PilotIntent>,
    mut notices: MessageWriter<Notice>,
    mut since_log: Local<f32>,
    lock: Option<Res<crate::combat::CruiseLock>>,
) {
    let Some(id) = request.0.clone() else {
        *since_log = 0.0;
        return;
    };
    // Progression toutes les ~10 s : un trajet qui n'aboutit pas se lit dans le journal.
    *since_log += time.delta_secs();
    let report = *since_log >= AUTOPILOT_LOG_SECONDS;
    if report {
        *since_log = 0.0;
    }
    if *mode.get() != ControlMode::Piloting {
        if report {
            info!(
                "autopilote vers {id} : en attente (contrôle {:?})",
                mode.get()
            );
        }
        return;
    }
    let Ok((pos, rotation, mut motion)) = ships.get_mut(sector.root) else {
        return;
    };
    let Some(handling) = data.flight.ships.get(sector.sector.id.as_str()) else {
        return;
    };
    let Some((target, target_pos, name)) = targets.iter().find(|(t, _, _)| t.id == id) else {
        warn!("autopilote : cible « {id} » absente du système");
        request.0 = None;
        notices.write(Notice(texts.get("nav.autopilot.unknown")));
        return;
    };
    // Le freinage compte sur le vol assisté.
    if motion.mode != FlightMode::Assisted {
        motion.mode = FlightMode::Assisted;
    }
    let obstacles: Vec<(SpacePos, f64)> = bodies.iter().map(|(p, b)| (p.0, b.radius)).collect();
    let surface = nearest_surface(pos.0, obstacles.iter().copied());
    // Hostiles proches : pas de croisière (`fly` la couperait de toute façon).
    let cruise_allowed = !lock.is_some_and(|l| l.0);
    let (step, aim) = autopilot_frame(
        pos.0,
        rotation.0,
        motion.velocity,
        (target_pos.0, target.stop),
        &obstacles,
        handling,
        cruise_allowed,
    );
    if report {
        let forward = rotation.0 * Vec3::NEG_Z;
        let to = aim - pos.0;
        let off = forward.as_dvec3().angle_between(dvec(to)).to_degrees();
        info!(
            "autopilote vers {id} : {:.0} m (arrêt {:.0} m), {:.0} m/s, surface à {:.0} km, \
             écart {off:.1}°, croisière {}, détour {}",
            pos.0.distance(target_pos.0),
            target.stop,
            motion.velocity.length(),
            surface / 1000.0,
            step.intent.cruise,
            aim != target_pos.0,
        );
    }
    *intent = step.intent;
    if step.arrived {
        info!(
            "autopilote : arrivé à {:.0} m de {id}",
            pos.0.distance(target_pos.0)
        );
        request.0 = None;
        notices.write(Notice(
            texts.format("nav.autopilot.arrived", &[("target", name.as_str())]),
        ));
    }
}

fn on_toggle_assist(
    _: On<Start<ToggleFlightAssist>>,
    mut ships: Query<&mut ShipMotion>,
    mut autopilot: ResMut<AutopilotRequest>,
    texts: Res<Texts>,
    mut notices: MessageWriter<Notice>,
) {
    // L'autopilote vole en assisté : changer de mode, c'est reprendre la main.
    disengage(&mut autopilot, &texts, &mut notices);
    for mut motion in &mut ships {
        motion.mode = match motion.mode {
            FlightMode::Assisted => FlightMode::Decoupled,
            FlightMode::Decoupled => FlightMode::Assisted,
        };
        info!("vol : {:?}", motion.mode);
    }
}

fn on_leave_seat(
    _: On<Start<LeaveSeat>>,
    mut commands: Commands,
    mut autopilot: ResMut<AutopilotRequest>,
    texts: Res<Texts>,
    mut notices: MessageWriter<Notice>,
) {
    disengage(&mut autopilot, &texts, &mut notices);
    commands.queue(|world: &mut World| crate::interaction::stand_up(world));
}

/// Intègre le vol du vaisseau du joueur (même quand personne n'est aux commandes : il dérive,
/// ou s'arrête en vol assisté).
fn fly(
    time: Res<Time>,
    data: Res<Data>,
    sector: Res<ActiveSector>,
    intent: Res<PilotIntent>,
    career: Option<Res<crate::career::Career>>,
    lock: Option<Res<crate::combat::CruiseLock>>,
    bodies: Query<(&SpacePosition, &SurfaceBody)>,
    mut ship: Query<
        (
            &mut SpacePosition,
            &mut SpaceRotation,
            &mut ShipMotion,
            &PositionDriver,
        ),
        Without<SurfaceBody>,
    >,
) {
    let Ok((mut pos, mut rotation, mut motion, driver)) = ship.get_mut(sector.root) else {
        return;
    };
    if *driver != PositionDriver::Frame {
        return;
    }
    let Some(handling) = data.flight.ships.get(sector.sector.id.as_str()) else {
        return;
    };
    let nearest = nearest_surface(pos.0, bodies.iter().map(|(p, b)| (p.0, b.radius)));
    let dt = f64::from(time.delta_secs().min(0.1));
    // Moteurs touchés (poupe ouverte) : il ne reste que les verniers.
    let thrust = career
        .as_ref()
        .and_then(|c| c.captain.hull.as_ref())
        .map_or(1.0, |h| h.thrust_factor()) as f32;
    let intent = PilotIntent {
        thrust: intent.thrust * thrust,
        // Hostiles trop proches : pas de croisière.
        cruise: intent.cruise && !lock.is_some_and(|l| l.0),
        ..intent.clone()
    };
    rotation.0 = step(
        &mut motion,
        rotation.0,
        &intent,
        handling,
        &data.space.scale,
        nearest,
        dt,
    );
    let d = motion.velocity * dt;
    pos.0 += SpacePos::new(d.x, d.y, d.z);
}

/// Un astre dont la surface passe à moins de cette marge (en rayons) de la ligne droite vers la
/// cible fait obstacle ; on le contourne par un point à `DETOUR_RADII` rayons de son centre.
/// Au large : sous 1 rayon d'altitude, la croisière (0,15 × la distance à la surface) devient
/// lente ; le détour garde ≈ 1,4 rayon d'altitude.
const OBSTACLE_RADII: f64 = 2.0;
const DETOUR_RADII: f64 = 2.4;

fn dvec(p: SpacePos) -> DVec3 {
    DVec3::new(p.x, p.y, p.z)
}

/// Le détour de l'autopilote : si la ligne droite de `pos` à `target` passe trop près d'un
/// astre (`bodies` : centre, rayon), un point de passage à côté de lui, du côté où la ligne
/// passe (le premier obstacle rencontré). Un astre visé lui-même (centre sur la cible) n'est
/// pas un obstacle. Logique pure.
pub fn detour(pos: SpacePos, target: SpacePos, bodies: &[(SpacePos, f64)]) -> Option<SpacePos> {
    let (a, b) = (dvec(pos), dvec(target));
    let path = b - a;
    let length = path.length();
    if length < 1.0 {
        return None;
    }
    let along_dir = path / length;
    bodies
        .iter()
        .filter(|(c, _)| c.distance(target) > 1.0)
        .filter_map(|&(c, r)| {
            let c = dvec(c);
            let along = (c - a).dot(along_dir).clamp(0.0, length);
            let closest = a + along_dir * along;
            let offset = closest - c;
            (offset.length() < OBSTACLE_RADII * r).then(|| {
                // Ligne en plein centre : contourner par le haut du plan.
                let side = offset
                    .try_normalize()
                    .or_else(|| along_dir.cross(DVec3::Y).try_normalize())
                    .unwrap_or(DVec3::X);
                let p = c + side * (DETOUR_RADII * r);
                (along, SpacePos::new(p.x, p.y, p.z))
            })
        })
        .min_by(|x, y| x.0.total_cmp(&y.0))
        .map(|(_, p)| p)
}

/// Un frame d'autopilote : [`approach`] de la cible (`goal` : position, distance d'arrêt), ou
/// d'un point de passage quand un astre barre la route ([`detour`]) ; la distance d'arrêt
/// compte alors le reste du chemin (pas de freinage au point de passage). Rend aussi le point
/// visé. Logique pure (jeu et tests).
pub fn autopilot_frame(
    pos: SpacePos,
    rotation: Quat,
    velocity: DVec3,
    goal: (SpacePos, f64),
    obstacles: &[(SpacePos, f64)],
    handling: &Handling,
    cruise_allowed: bool,
) -> (Approach, SpacePos) {
    let (target, stop) = goal;
    let (aim, stop) = match detour(pos, target, obstacles) {
        Some(waypoint) => (waypoint, stop - waypoint.distance(target)),
        None => (target, stop),
    };
    let step = approach(pos, rotation, velocity, aim, stop, handling, cruise_allowed);
    (step, aim)
}

/// Distance à la surface la plus proche (m).
pub fn nearest_surface(at: SpacePos, bodies: impl Iterator<Item = (SpacePos, f64)>) -> f64 {
    bodies
        .map(|(p, r)| at.distance(p) - r)
        .fold(f64::MAX, f64::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handling() -> Handling {
        Handling {
            max_speed: 250.0,
            accel_forward: 45.0,
            accel_backward: 25.0,
            accel_lateral: 20.0,
            accel_vertical: 20.0,
            pitch_rate_deg: 40.0,
            yaw_rate_deg: 30.0,
            roll_rate_deg: 80.0,
            turn_response: 3.0,
            cruise_spool_down: 1.5,
        }
    }

    fn scale() -> SystemScale {
        SystemScale {
            au: 1.496e9,
            first_orbit_au: 0.4,
            orbit_ratio: 1.7,
            star_radius: 7.0e7,
            planet_radius: 6.4e5,
            gas_giant_radius: 7.0e6,
            moon_radius: 1.7e5,
            moon_orbit_radii: 12.0,
            parking_orbit_radii: 1.5,
            cruise_rate: 0.15,
            cruise_min_speed: 2000.0,
            cruise_max_speed: 3.0e7,
            manoeuvre_max_speed: 300.0,
        }
    }

    fn run(motion: &mut ShipMotion, intent: &PilotIntent, seconds: f64, surface: f64) -> Quat {
        let mut q = Quat::IDENTITY;
        for _ in 0..(seconds * 60.0) as usize {
            q = step(
                motion,
                q,
                intent,
                &handling(),
                &scale(),
                surface,
                1.0 / 60.0,
            );
        }
        q
    }

    #[test]
    fn vol_assiste_atteint_la_vitesse_puis_s_arrete() {
        let mut m = ShipMotion::default();
        let forward = PilotIntent {
            thrust: Vec3::NEG_Z,
            ..default()
        };
        run(&mut m, &forward, 7.0, 1e6);
        assert!((m.velocity.z + 250.0).abs() < 1.0, "{:?}", m.velocity);
        // 250 m/s à 25 m/s² en marche arrière : 10 s pour s'arrêter.
        run(&mut m, &PilotIntent::default(), 11.0, 1e6);
        assert!(m.velocity.length() < 0.5, "{:?}", m.velocity);
    }

    #[test]
    fn vol_decouple_garde_sa_vitesse() {
        let mut m = ShipMotion {
            mode: FlightMode::Decoupled,
            ..default()
        };
        let side = PilotIntent {
            thrust: Vec3::X,
            ..default()
        };
        run(&mut m, &side, 2.0, 1e6);
        let v = m.velocity;
        assert!((v.x - 40.0).abs() < 0.5, "{v:?}");
        run(&mut m, &PilotIntent::default(), 5.0, 1e6);
        assert!(
            (m.velocity - v).length() < 1e-9,
            "rien ne freine en découplé"
        );
    }

    #[test]
    fn la_rotation_respecte_le_maniement() {
        let mut m = ShipMotion::default();
        let yaw_left = PilotIntent {
            turn: Vec2::new(-1.0, 0.0),
            ..default()
        };
        let q = run(&mut m, &yaw_left, 3.0, 1e6);
        // Lacet à gauche ≈ 30°/s pendant 3 s (≈ 80°) : l'avant tourne vers −X.
        let forward = q * Vec3::NEG_Z;
        assert!(forward.x < -0.3, "{forward:?}");
        assert!((m.angular.y - 30f32.to_radians()).abs() < 1e-3);
    }

    /// Vole avec l'autopilote vers `target` (arrêt à `stop`) depuis `start`, l'avant tourné vers
    /// `heading` ; une planète de rayon `radius` centrée en `planet` borne la croisière. Rend
    /// le temps d'arrivée (s), la distance finale et la plus courte distance atteinte.
    fn autopilot_flight(
        start: SpacePos,
        heading: Quat,
        target: SpacePos,
        stop: f64,
        planet: SpacePos,
        radius: f64,
    ) -> (Option<f64>, f64, f64) {
        autopilot_flight_with(
            start,
            heading,
            (target, stop),
            (planet, radius),
            true,
            1.0 / 60.0,
        )
    }

    /// [`autopilot_flight`], croisière permise ou non, au pas `dt` (s).
    fn autopilot_flight_with(
        start: SpacePos,
        heading: Quat,
        goal: (SpacePos, f64),
        planet: (SpacePos, f64),
        cruise_allowed: bool,
        dt: f64,
    ) -> (Option<f64>, f64, f64) {
        let (h, s) = (handling(), scale());
        let (target, stop) = goal;
        let mut motion = ShipMotion::default();
        let (mut pos, mut rot) = (start, heading);
        let mut closest = f64::MAX;
        for frame in 0..(900.0 / dt) as usize {
            let surface = pos.distance(planet.0) - planet.1;
            let (mut a, _) = autopilot_frame(
                pos,
                rot,
                motion.velocity,
                (target, stop),
                &[planet],
                &h,
                cruise_allowed,
            );
            if a.arrived {
                return (Some(frame as f64 * dt), pos.distance(target), closest);
            }
            assert!(surface > 0.0, "dans la planète à {pos:?}");
            // Comme `fly` sous `CruiseLock`.
            a.intent.cruise &= cruise_allowed;
            rot = step(&mut motion, rot, &a.intent, &h, &s, surface, dt);
            let d = motion.velocity * dt;
            pos += SpacePos::new(d.x, d.y, d.z);
            closest = closest.min(pos.distance(target));
        }
        (None, pos.distance(target), closest)
    }

    #[test]
    fn l_autopilote_arrive_et_s_arrete_sans_depasser() {
        // Un point de livraison à 1,5 rayon d'une planète, à 500 000 km de là : croisière,
        // ralentissement, freinage, arrêt à la distance d'arrêt.
        let radius = 6.4e5;
        let planet = SpacePos::new(0.0, 0.0, -2.5 * radius);
        let stop = 8_000.0;
        let (time, distance, closest) = autopilot_flight(
            SpacePos::new(0.0, 0.0, 5.0e8),
            Quat::IDENTITY,
            SpacePos::ZERO,
            stop,
            planet,
            radius,
        );
        let time = time.unwrap_or_else(|| panic!("jamais arrivé ({distance:.0} m)"));
        assert!(time < 300.0, "{time} s");
        assert!(
            (distance - stop).abs() <= ARRIVED_SLACK + 50.0,
            "{distance}"
        );
        assert!(closest > stop - 1_000.0, "dépassé : {closest}");
    }

    #[test]
    fn l_autopilote_vire_vers_une_cible_de_cote() {
        // La cible sur le côté, à 40 km : virer d'abord, puis approcher.
        let radius = 6.4e5;
        let (time, distance, _) = autopilot_flight(
            SpacePos::ZERO,
            Quat::IDENTITY,
            SpacePos::new(40_000.0, 0.0, 0.0),
            3_000.0,
            SpacePos::new(0.0, -5.0e6, 0.0),
            radius,
        );
        assert!(time.is_some(), "jamais arrivé ({distance:.0} m)");
        assert!(
            (distance - 3_000.0).abs() <= ARRIVED_SLACK + 50.0,
            "{distance}"
        );
    }

    #[test]
    fn loin_de_toute_surface_la_croisiere_ne_fait_pas_depasser() {
        // Croisière permise jusqu'à 30 000 km/s : une cible à 60 km ne doit pas être dépassée.
        let (time, distance, closest) = autopilot_flight(
            SpacePos::ZERO,
            Quat::IDENTITY,
            SpacePos::new(0.0, 0.0, -60_000.0),
            8_000.0,
            SpacePos::new(0.0, 0.0, 1.0e10),
            6.4e5,
        );
        assert!(time.is_some(), "jamais arrivé ({distance:.0} m)");
        assert!(closest > 7_000.0, "dépassé : {closest}");
    }

    #[test]
    fn l_autopilote_contourne_un_astre_sur_sa_route() {
        // Une planète entre le vaisseau et la cible : un point de passage à côté d'elle, hors
        // de sa surface ; route dégagée ou planète visée : pas de détour.
        let r = 6.4e5;
        let planet = (SpacePos::ZERO, r);
        let (from, to) = (
            SpacePos::new(0.0, 0.0, 2.5 * r),
            SpacePos::new(1.0e3, 0.0, -1.0e8),
        );
        let waypoint = detour(from, to, &[planet]).unwrap_or_else(|| panic!("pas de détour"));
        // Au large : hors de la zone d'obstacle, à la distance de détour.
        assert!(waypoint.length() > OBSTACLE_RADII * r, "{waypoint:?}");
        assert!(
            (waypoint.length() - DETOUR_RADII * r).abs() < 1.0,
            "{waypoint:?}"
        );
        // Route dégagée : vers le large, la planète dans le dos ; ou une ligne qui la longe
        // au-delà de la zone d'obstacle.
        let away = SpacePos::new(0.0, 0.0, 1.0e8);
        assert!(detour(from, away, &[planet]).is_none());
        let side = (
            SpacePos::new(2.2 * r, 0.0, 2.5 * r),
            SpacePos::new(2.2 * r, 0.0, -1.0e8),
        );
        assert!(detour(side.0, side.1, &[planet]).is_none());
        // La même ligne, plus près : détour.
        assert!(
            detour(
                SpacePos::new(1.8 * r, 0.0, 2.5 * r),
                SpacePos::new(1.8 * r, 0.0, -1.0e8),
                &[planet]
            )
            .is_some()
        );
        assert!(detour(from, SpacePos::ZERO, &[planet]).is_none());
        // En vol : on n'entre jamais dans la planète (`autopilot_flight` le vérifie).
        let (time, distance, _) =
            autopilot_flight(from, Quat::IDENTITY, to, 8_000.0, SpacePos::ZERO, r);
        assert!(time.is_some(), "jamais arrivé ({distance:.0} m)");
    }

    #[test]
    fn deja_arrete_devant_la_cible() {
        let a = approach(
            SpacePos::new(0.0, 0.0, 5_000.0),
            Quat::IDENTITY,
            DVec3::ZERO,
            SpacePos::ZERO,
            5_100.0,
            &handling(),
            true,
        );
        assert!(a.arrived);
        assert_eq!(a.intent.thrust, Vec3::ZERO);
    }

    #[test]
    fn approche_courte_rapide_et_sans_depasser() {
        // Hexis : 15 km de la cible, arrêt à 8 km, en orbite (surface à ≈ 970 km) ; à 30 et
        // 144 i/s.
        for dt in [1.0 / 30.0, 1.0 / 144.0] {
            let (time, distance, closest) = autopilot_flight_with(
                SpacePos::new(0.0, 0.0, 15_000.0),
                Quat::IDENTITY,
                (SpacePos::ZERO, 8_000.0),
                (SpacePos::new(0.0, -1.6e6, 0.0), 6.4e5),
                true,
                dt,
            );
            let time = time.unwrap_or_else(|| panic!("jamais arrivé ({distance:.0} m)"));
            assert!(time < 20.0, "{time} s");
            assert!(closest > 8_000.0 - ARRIVED_SLACK, "dépassé : {closest}");
        }
    }

    #[test]
    fn sans_croisiere_l_autopilote_arrive_quand_meme() {
        // Hostiles proches (`CruiseLock`) : en vol de manœuvre, sans dépasser.
        let (time, distance, closest) = autopilot_flight_with(
            SpacePos::new(0.0, 0.0, 20_000.0),
            Quat::IDENTITY,
            (SpacePos::ZERO, 8_000.0),
            (SpacePos::new(0.0, -1.6e6, 0.0), 6.4e5),
            false,
            1.0 / 60.0,
        );
        assert!(time.is_some(), "jamais arrivé ({distance:.0} m)");
        assert!(closest > 8_000.0 - ARRIVED_SLACK, "dépassé : {closest}");
    }

    #[test]
    fn croisiere_puis_retour_a_la_vitesse_de_manoeuvre() {
        let mut m = ShipMotion::default();
        let cruise = PilotIntent {
            cruise: true,
            ..default()
        };
        // À 10 000 km d'une surface : limite à 1 500 km/s.
        run(&mut m, &cruise, 5.0, 1.0e7);
        let speed = m.velocity.length();
        assert!(speed > 1.4e6, "{speed}");
        run(&mut m, &PilotIntent::default(), 30.0, 1.0e7);
        assert!(
            m.velocity.length() <= 250.0 + 1e-6,
            "{}",
            m.velocity.length()
        );
    }
}
