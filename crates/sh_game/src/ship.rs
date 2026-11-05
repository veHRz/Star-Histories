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
/// Hors de cet angle (degrés) entre l'avant et la cible, on vire sans pousser.
const ALIGN_DEGREES: f64 = 10.0;
/// Marge (m) gardée avant la distance d'arrêt quand on coupe la croisière.
const CRUISE_MARGIN: f64 = 3_000.0;
/// En vol de manœuvre, on ne relance la croisière qu'au-delà de cette distance (m) : plus près,
/// la croisière (au moins 2 km/s) ne ferait qu'osciller.
const CRUISE_ENGAGE: f64 = 20_000.0;
/// Arrivé : à moins de cette vitesse (m/s), dans la distance d'arrêt (à cette marge près, m).
const ARRIVED_SPEED: f64 = 2.0;
const ARRIVED_SLACK: f64 = 250.0;

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
/// aligné, pousser, et rester en croisière tant que la distance laisse le temps de ralentir (la
/// croisière coupée, la vitesse décroît en ≈ `cruise_spool_down` s : ≈ 1,5 × v mètres) ; couper
/// la poussée quand il ne reste que la distance de freinage. Logique pure (jeu et scénarios).
///
/// `cruise_limit` : la vitesse de croisière permise ici (m/s), `dt` : la durée du frame (s). La
/// vitesse qu'aurait le vaisseau au frame suivant en croisière (même montée que [`step`]) doit
/// encore laisser la place de ralentir : un seul frame de croisière loin de toute surface
/// (jusqu'à 30 000 km/s) dépasserait sinon une cible proche.
#[allow(clippy::too_many_arguments)]
pub fn approach(
    pos: SpacePos,
    rotation: Quat,
    velocity: DVec3,
    target: SpacePos,
    stop: f64,
    handling: &Handling,
    cruise_limit: f64,
    dt: f64,
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
    if aligned && remaining > braking_distance(speed, handling) {
        intent.thrust = Vec3::NEG_Z;
        // Déjà lancé (au-dessus de la vitesse de manœuvre) ou loin : la croisière, tant qu'elle
        // laisse la place de ralentir.
        let next = speed + (cruise_limit - speed).max(0.0) * (1.0 - (-2.0 * dt).exp());
        let room = remaining > (handling.cruise_spool_down + 0.2) * next + CRUISE_MARGIN;
        let launched = speed > handling.max_speed * 1.05 || remaining > CRUISE_ENGAGE;
        intent.cruise = room && launched;
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
        let target = scale.cruise_speed_limit(nearest_surface);
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
) {
    if scripted.0 {
        return;
    }
    if *mode.get() != ControlMode::Piloting {
        *intent = PilotIntent::default();
        return;
    }
    let raw_steer: Vec2 = ***steer;
    let steer = raw_steer * settings.look_scale();
    *intent = PilotIntent {
        thrust: ***thrust,
        turn: Vec2::new(steer.x, -steer.y) / MOUSE_FULL_TURN,
        roll: ***roll,
        cruise: ***boost,
    };
    // Le joueur reprend la main : l'autopilote se désengage.
    let touched = intent.thrust != Vec3::ZERO
        || intent.roll != 0.0
        || intent.cruise
        || raw_steer.length() > STEER_DEADZONE;
    if touched && autopilot.0.is_some() {
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
) {
    let Some(id) = request.0.clone() else {
        return;
    };
    if *mode.get() != ControlMode::Piloting {
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
    // La même vitesse limite et le même pas que `fly`.
    let surface = nearest_surface(pos.0, bodies.iter().map(|(p, b)| (p.0, b.radius)));
    let step = approach(
        pos.0,
        rotation.0,
        motion.velocity,
        target_pos.0,
        target.stop,
        handling,
        data.space.scale.cruise_speed_limit(surface),
        f64::from(time.delta_secs().min(0.1)),
    );
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
        let (h, s) = (handling(), scale());
        let mut motion = ShipMotion::default();
        let (mut pos, mut rot) = (start, heading);
        let mut closest = f64::MAX;
        let dt = 1.0 / 60.0;
        for frame in 0..(900.0 / dt) as usize {
            let surface = pos.distance(planet) - radius;
            let limit = s.cruise_speed_limit(surface);
            let a = approach(pos, rot, motion.velocity, target, stop, &h, limit, dt);
            if a.arrived {
                return (Some(frame as f64 * dt), pos.distance(target), closest);
            }
            assert!(surface > 0.0, "dans la planète à {pos:?}");
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
    fn deja_arrete_devant_la_cible() {
        let a = approach(
            SpacePos::new(0.0, 0.0, 5_000.0),
            Quat::IDENTITY,
            DVec3::ZERO,
            SpacePos::ZERO,
            5_100.0,
            &handling(),
            2_000.0,
            1.0 / 60.0,
        );
        assert!(a.arrived);
        assert_eq!(a.intent.thrust, Vec3::ZERO);
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
