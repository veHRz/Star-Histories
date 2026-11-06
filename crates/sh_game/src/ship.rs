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
//! - **Montée de la croisière** (M11, réponse de l'auteur à Q19) : la vitesse ne saute plus
//!   d'un coup vers la limite ; elle monte doucement puis de plus en plus vite
//!   ([`SystemScale::cruise_climb`], niveau de montée dans [`ShipMotion::cruise_spool`]),
//!   jusqu'à 0,1 c en ≈ 20 s. La descente (limite qui baisse près d'une surface, consigne de
//!   l'autopilote) suit toujours sa consigne en 0,5 s : ni planète ni cible dépassée.
//! - **Vitesses de M11** (P33, Q19) : la poussée va jusqu'à 1 km/s, avec des accélérations
//!   renforcées au-delà de la vitesse de combat (la finesse à basse vitesse ne change pas) ;
//!   tant que des hostiles proches interdisent la croisière (`CruiseLock`), la poussée reste
//!   bornée à la **vitesse de combat** ([`PilotIntent::speed_cap`]) : plus vite, le vaisseau
//!   ralentit comme au sortir de la croisière. Les combats de M6 restent ceux qui ont été réglés.
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
//! L'approche finale se fait en croisière bridée ([`PilotIntent::cruise_cap`]), plus rapide que
//! toute poussée : la poussée ne sert qu'au combat, sous la vitesse de combat.

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
/// ce temps (s), sous la limite de croisière. La croisière descend vers sa consigne au taux 2/s
/// ([`step`]) : le reste du chemin r obéit à r'' + 2 r' + (2 / T) r = 0, sans dépassement dès
/// que T ≥ 2 (amortissement critique) ; 2,5 laisse de la marge aux frames longs. La montée
/// progressive (M11) ne fait qu'arriver plus tard sur cette consigne : elle reste sous elle.
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

/// Distance (m) pour s'arrêter en vol assisté depuis `speed`, la poussée bornée à `limit`
/// (m/s) : la décroissance jusqu'à `limit` (sortie de croisière, vitesse de combat), puis le
/// freinage en marche arrière ([`Handling::brake_distance`]).
pub fn braking_distance(speed: f64, handling: &Handling, limit: f64) -> f64 {
    if speed > limit {
        handling.cruise_spool_down * (speed - limit) + handling.brake_distance(limit)
    } else {
        handling.brake_distance(speed)
    }
}

/// Un frame d'approche vers `target`, arrêt à `stop` mètres de lui : virer vers lui ; une fois
/// aligné, la croisière avec une consigne de vitesse qui décroît avec le reste du chemin
/// ([`APPROACH_SECONDS`], [`PilotIntent::cruise_cap`]) : loin, la limite de croisière ; près,
/// un ralentissement continu jusqu'à l'arrêt, sans dépasser la cible. Croisière interdite
/// (`cruise_allowed` faux, hostiles proches) : poussée, bornée à la vitesse de combat, coupée
/// quand il ne reste que la distance de freinage. Logique pure (jeu et scénarios).
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
    } else if aligned
        && remaining > braking_distance(speed, handling, handling.thrust_limit(!cruise_allowed))
    {
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
    /// Niveau de montée de la croisière (s, [`SystemScale::cruise_climb`]) ; `None` hors
    /// croisière. À l'engagement, il part de celui de la vitesse courante
    /// ([`SystemScale::cruise_spool_level`]) : 0 depuis la poussée, plus haut si l'on reprend la
    /// croisière lancé.
    pub cruise_spool: Option<f64>,
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
    /// Plafond de la poussée plus bas que la vitesse maximale du maniement (m/s) : la vitesse
    /// de combat, posée par le vol ([`fly`]) tant que des hostiles interdisent la croisière ; un
    /// scénario peut en poser un plus bas (le pilotage manuel, jamais). Au-delà, le vaisseau
    /// ralentit comme au sortir de la croisière (`cruise_spool_down`).
    pub speed_cap: Option<f64>,
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
        let spool = motion
            .cruise_spool
            .unwrap_or_else(|| scale.cruise_spool_level(along));
        let (speed, spool) = if target > along {
            // Montée : douce au départ, de plus en plus vive (M11, pas de saut d'un coup).
            scale.cruise_climb(along, spool, target, dt)
        } else {
            // Descente : la consigne en 0,5 s (près d'une surface, à l'approche d'une cible) ;
            // la montée se recale sur la vitesse, pour reprendre de là sans à-coup.
            let blend = 1.0 - (-2.0 * dt).exp();
            let speed = along + (target - along) * blend;
            (speed, (spool + dt).min(scale.cruise_spool_level(speed)))
        };
        motion.cruise_spool = Some(spool);
        motion.velocity = forward * speed;
        return orientation;
    }
    motion.cruise_spool = None;

    // La vitesse permise en poussée : celle du maniement, ou la vitesse de combat.
    let cap = intent
        .speed_cap
        .map_or(handling.max_speed, |c| c.clamp(0.0, handling.max_speed));
    let speed = motion.velocity.length();
    if speed > cap * 1.01 {
        // Retour de croisière, ou vitesse de combat imposée : décélération exponentielle jusqu'à
        // la vitesse permise (en douceur : pas de cassure de la vitesse d'un frame à l'autre).
        let decay = (-dt / handling.cruise_spool_down).exp();
        motion.velocity *= (decay).max(cap / speed);
        return orientation;
    }

    let local = to_world.inverse() * motion.velocity;
    let thrust = intent.thrust.clamp(Vec3::splat(-1.0), Vec3::ONE).as_dvec3();
    // Accélération maximale par axe et par sens (−z : avant), renforcée au-delà de la vitesse
    // de combat.
    let factor = handling.accel_factor(speed);
    let accel = |axis: usize, sign: f64| {
        factor
            * match axis {
                0 => handling.accel_lateral,
                1 => handling.accel_vertical,
                _ if sign < 0.0 => handling.accel_forward,
                _ => handling.accel_backward,
            }
    };
    let mut next = local;
    for axis in 0..3 {
        match motion.mode {
            FlightMode::Assisted => {
                let target = thrust[axis] * cap;
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
    if motion.velocity.length() > cap {
        motion.velocity = motion.velocity.normalize_or_zero() * cap;
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
    (mut steer_streak, mut guard, real): (Local<u32>, Local<MouseGuard>, Res<Time<Real>>),
) {
    if scripted.0 {
        return;
    }
    if *mode.get() != ControlMode::Piloting {
        *intent = PilotIntent::default();
        *steer_streak = 0;
        return;
    }
    let since_guard = guard.update(real.elapsed_secs_f64(), autopilot.0.as_deref());
    let raw_steer: Vec2 = ***steer;
    let steer = raw_steer * settings.look_scale();
    *intent = PilotIntent {
        thrust: ***thrust,
        turn: Vec2::new(steer.x, -steer.y) / MOUSE_FULL_TURN,
        roll: ***roll,
        cruise: ***boost,
        cruise_cap: None,
        speed_cap: None,
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
    let mouse = mouse_takes_over(*steer_streak, since_guard);
    if (keys || mouse) && autopilot.0.is_some() {
        info!(
            "autopilote : le joueur reprend la main ({})",
            if keys { "touches" } else { "souris" }
        );
        disengage(&mut autopilot, &texts, &mut notices);
    }
}

/// Délai de grâce de la souris (s) : le joueur engage l'autopilote d'un clic sur la carte, et sa
/// main bouge encore quand la carte se ferme ; de même en reprenant les commandes.
const MOUSE_GRACE_SECONDS: f64 = 0.5;
/// Au-delà de cet écart entre deux frames lus (s), les commandes ont été quittées (carte,
/// séquence, siège) : la reprise rouvre le délai de grâce.
const PILOT_GAP_SECONDS: f64 = 0.2;

/// Depuis quand la souris ne peut pas désengager l'autopilote.
#[derive(Default)]
pub struct MouseGuard {
    request: Option<String>,
    last_seen: Option<f64>,
    since: f64,
}

impl MouseGuard {
    /// Un frame aux commandes à l'instant `now` (s), avec cette demande d'autopilote ; rend le
    /// temps écoulé depuis l'engagement ou la reprise des commandes (le plus récent).
    pub fn update(&mut self, now: f64, request: Option<&str>) -> f64 {
        let resumed = self
            .last_seen
            .is_none_or(|last| now - last > PILOT_GAP_SECONDS);
        let engaged = request.is_some() && self.request.as_deref() != request;
        if resumed || engaged {
            self.since = now;
        }
        self.last_seen = Some(now);
        self.request = request.map(str::to_owned);
        now - self.since
    }
}

/// La souris reprend-elle la main ? Plusieurs frames de suite au-delà du seuil, et hors du
/// délai de grâce. Les touches, elles, désengagent toujours tout de suite.
pub fn mouse_takes_over(streak: u32, since_guard: f64) -> bool {
    streak >= STEER_TAKEOVER_FRAMES && since_guard >= MOUSE_GRACE_SECONDS
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
    // Hostiles trop proches : pas de croisière, et la poussée bornée à la vitesse de combat (sous
    // un plafond déjà posé par la commande, s'il est plus bas : un scénario peut en poser un).
    let combat = lock.is_some_and(|l| l.0);
    let speed_cap = match (
        intent.speed_cap,
        combat.then_some(handling.combat_max_speed),
    ) {
        (Some(asked), Some(combat)) => Some(asked.min(combat)),
        (asked, combat) => asked.or(combat),
    };
    let intent = PilotIntent {
        thrust: intent.thrust * thrust,
        cruise: intent.cruise && !combat,
        speed_cap,
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
/// Au large : sous 1 rayon d'altitude, la croisière (`cruise_rate` × la distance à la surface)
/// devient lente ; le détour garde ≈ 1,4 rayon d'altitude. Un détour plus large ne gagne presque
/// rien (simulation du trajet de `deliver_vorrh`, M11 : 4 rayons au lieu de 2,4, ≈ 1 %), et
/// `OBSTACLE_RADII` doit rester sous l'altitude des cibles (2,5 rayons du centre).
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

    /// Le maniement de la corvette (`flight.ron`, M11).
    fn handling() -> Handling {
        Handling {
            max_speed: 1000.0,
            combat_max_speed: 250.0,
            accel_forward: 45.0,
            accel_backward: 25.0,
            accel_lateral: 20.0,
            accel_vertical: 20.0,
            fast_accel_factor: 3.0,
            pitch_rate_deg: 40.0,
            yaw_rate_deg: 30.0,
            roll_rate_deg: 80.0,
            turn_response: 3.0,
            cruise_spool_down: 1.5,
        }
    }

    /// L'échelle de `space.ron` (croisière de P2, montée progressive de M11).
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
            cruise_spool_accel: 30.0,
            cruise_spool_time: 1.5,
            manoeuvre_max_speed: 1000.0,
        }
    }

    /// Vole `seconds` secondes à 60 i/s (au frame près), l'avant tourné vers −Z au départ.
    fn run(motion: &mut ShipMotion, intent: &PilotIntent, seconds: f64, surface: f64) -> Quat {
        let mut q = Quat::IDENTITY;
        for _ in 0..(seconds * 60.0).round() as usize {
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
        // Sous la vitesse de combat, l'accélération de toujours (45 m/s²) : la finesse ne
        // change pas.
        run(&mut m, &forward, 5.0, 1e6);
        assert!((m.velocity.z + 225.0).abs() < 1.0, "{:?}", m.velocity);
        // Puis ×3 : 1 km/s en ≈ 11 s en tout.
        run(&mut m, &forward, 6.5, 1e6);
        assert!((m.velocity.z + 1000.0).abs() < 1.0, "{:?}", m.velocity);
        // Freinage : 1 000 → 250 m/s à 75 m/s², puis 250 → 0 à 25 m/s² ; ≈ 20 s.
        run(&mut m, &PilotIntent::default(), 9.0, 1e6);
        assert!(m.velocity.length() > 250.0, "{:?}", m.velocity);
        run(&mut m, &PilotIntent::default(), 12.0, 1e6);
        assert!(m.velocity.length() < 0.5, "{:?}", m.velocity);
    }

    #[test]
    fn la_vitesse_de_combat_ralentit_en_douceur_et_borne_la_poussee() {
        let mut m = ShipMotion::default();
        let forward = PilotIntent {
            thrust: Vec3::NEG_Z,
            ..default()
        };
        run(&mut m, &forward, 12.0, 1e6);
        assert!((m.velocity.length() - 1000.0).abs() < 1.0);
        // Les hostiles arrivent : la poussée est bornée à 250 m/s (comme `fly`).
        let combat = PilotIntent {
            speed_cap: Some(handling().combat_max_speed),
            ..forward.clone()
        };
        let speeds: Vec<f64> = (0..240)
            .map(|_| {
                run(&mut m, &combat, 1.0 / 60.0, 1e6);
                m.velocity.length()
            })
            .collect();
        // Pas de cassure : au plus ≈ 1 % de moins par frame ; à la vitesse de combat en ≈ 2 s.
        let mut previous = 1000.0;
        for &v in &speeds[..30] {
            assert!(
                v >= previous * 0.985 && v <= previous + 1e-9,
                "{previous} → {v}"
            );
            previous = v;
        }
        assert!(speeds[150] <= 250.0 + 1e-6, "{}", speeds[150]);
        // Pleine poussée sous le verrou : elle reste à la vitesse de combat.
        run(&mut m, &combat, 5.0, 1e6);
        let v = m.velocity.length();
        assert!((245.0..=250.0 + 1e-6).contains(&v), "{v}");
        // Verrou levé : la poussée repart vers 1 km/s.
        run(&mut m, &forward, 3.0, 1e6);
        assert!(m.velocity.length() > 600.0, "{}", m.velocity.length());
    }

    #[test]
    fn la_distance_de_freinage_compte_la_vitesse_permise() {
        let h = handling();
        // Sous la vitesse permise : le freinage seul.
        assert!((braking_distance(250.0, &h, 1000.0) - 1250.0).abs() < 1e-6);
        // Au-dessus (sortie de croisière ou vitesse de combat) : la décroissance d'abord.
        let combat = braking_distance(600.0, &h, 250.0);
        assert!((combat - (1.5 * 350.0 + 1250.0)).abs() < 1e-6, "{combat}");
        assert!(braking_distance(600.0, &h, 1000.0) > combat);
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
            // Comme `fly` sous `CruiseLock` : ni croisière, ni plus que la vitesse de combat.
            a.intent.cruise &= cruise_allowed;
            a.intent.speed_cap = (!cruise_allowed).then_some(h.combat_max_speed);
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
    fn la_souris_ne_coupe_pas_l_autopilote_juste_apres_l_engagement() {
        let frame = 1.0 / 60.0;
        let mut guard = MouseGuard::default();
        // Aux commandes depuis un moment, sans autopilote.
        let mut t = 10.0;
        while t < 12.0 {
            guard.update(t, None);
            t += frame;
        }
        // Engagé depuis la carte : la souris qui bouge encore ne compte pas pendant 0,5 s.
        let since = guard.update(t, Some("vorrh_shipyards"));
        assert!(!mouse_takes_over(STEER_TAKEOVER_FRAMES + 10, since));
        let mut since = since;
        while since < MOUSE_GRACE_SECONDS + frame {
            t += frame;
            since = guard.update(t, Some("vorrh_shipyards"));
        }
        assert!(mouse_takes_over(STEER_TAKEOVER_FRAMES, since));
        assert!(!mouse_takes_over(STEER_TAKEOVER_FRAMES - 1, since));
        // Retour de la carte (les commandes quittées une seconde) : de nouveau le délai.
        t += 1.0;
        let since = guard.update(t, Some("vorrh_shipyards"));
        assert!(since < 1e-9);
        assert!(!mouse_takes_over(STEER_TAKEOVER_FRAMES + 10, since));
        // Une autre cible : nouvel engagement, nouveau délai.
        let mut since = 0.0;
        for _ in 0..60 {
            t += frame;
            since = guard.update(t, Some("vorrh_shipyards"));
        }
        assert!(since > MOUSE_GRACE_SECONDS);
        t += frame;
        assert!(guard.update(t, Some("vorrh_portal_hub")) < 1e-9);
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
        // À 10 000 km d'une surface : limite à 1 500 km/s, atteinte en ≈ 16 s depuis l'arrêt.
        run(&mut m, &cruise, 20.0, 1.0e7);
        let speed = m.velocity.length();
        assert!(speed > 1.49e6, "{speed}");
        // Retour à la poussée maximale en ≈ 11 s (1,5 × ln 1 500), puis le freinage (≈ 20 s).
        run(&mut m, &PilotIntent::default(), 12.0, 1.0e7);
        assert!(
            m.velocity.length() <= handling().max_speed + 1e-6,
            "{}",
            m.velocity.length()
        );
        assert!(m.cruise_spool.is_none());
        run(&mut m, &PilotIntent::default(), 22.0, 1.0e7);
        assert!(m.velocity.length() < 0.5, "{}", m.velocity.length());
    }

    /// Engage la croisière à `from` m/s (vers −Z), loin de toute surface, et rend la vitesse de
    /// chaque frame à 60 i/s pendant `seconds` secondes.
    fn cruise_profile(from: f64, seconds: f64, intent: &PilotIntent) -> Vec<f64> {
        let mut m = ShipMotion {
            velocity: DVec3::new(0.0, 0.0, -from),
            ..default()
        };
        (0..(seconds * 60.0).round() as usize)
            .map(|_| {
                run(&mut m, intent, 1.0 / 60.0, 1.0e12);
                m.velocity.length()
            })
            .collect()
    }

    #[test]
    fn la_croisiere_monte_en_douceur_puis_de_plus_en_plus_vite() {
        // Réponse de l'auteur à Q19 : pas de saut d'un coup depuis la vitesse de poussée.
        let cruise = PilotIntent {
            cruise: true,
            ..default()
        };
        let speeds = cruise_profile(1000.0, 30.0, &cruise);
        let accels: Vec<f64> = std::iter::once(1000.0)
            .chain(speeds.iter().copied())
            .collect::<Vec<_>>()
            .windows(2)
            .map(|w| (w[1] - w[0]) * 60.0)
            .collect();
        // Quelques g la première seconde (≈ 3 à 6 g), à peine plus vite au bout de 3 s.
        assert!(
            accels[..60].iter().all(|a| *a <= 10.0 * 9.81),
            "{:?}",
            &accels[..60]
        );
        assert!(speeds[179] < 1500.0, "{}", speeds[179]);
        // Puis de plus en plus vite, sans jamais ralentir, jusqu'à 0,1 c en 15 à 25 s.
        assert!(speeds.windows(2).all(|w| w[1] >= w[0]));
        assert!(accels[600] > 10.0 * accels[60]);
        let top = speeds
            .iter()
            .position(|v| *v >= 0.99 * scale().cruise_max_speed);
        let top = top.map(|i| i as f64 / 60.0);
        assert!(top.is_some_and(|t| (15.0..=25.0).contains(&t)), "{top:?}");
    }

    #[test]
    fn la_croisiere_redescend_puis_reprend_sans_saut() {
        // L'autopilote bride la croisière (approche), puis la relâche : la descente suit la
        // consigne en 0,5 s, la reprise repart de la vitesse atteinte, sans bond.
        let mut m = ShipMotion::default();
        let capped = |cap: f64| PilotIntent {
            cruise: true,
            cruise_cap: Some(cap),
            ..default()
        };
        run(&mut m, &capped(1.0e6), 30.0, 1.0e12);
        assert!((m.velocity.length() - 1.0e6).abs() < 1.0);
        run(&mut m, &capped(1.0e5), 5.0, 1.0e12);
        assert!((m.velocity.length() - 1.0e5).abs() < 100.0);
        let free = PilotIntent {
            cruise: true,
            ..default()
        };
        run(&mut m, &free, 0.1, 1.0e12);
        assert!(m.velocity.length() < 1.1e5, "{}", m.velocity.length());
        run(&mut m, &free, 10.0, 1.0e12);
        assert!(m.velocity.length() > 1.0e6, "{}", m.velocity.length());
    }

    #[test]
    fn la_croisiere_finit_au_dessus_de_la_poussee() {
        // Le tutoriel reconnaît la croisière au-delà d'une fois et demie la poussée maximale :
        // même au ras d'une surface, elle y arrive, en quelques secondes depuis l'arrêt.
        let (h, s) = (handling(), scale());
        assert!(s.cruise_speed_limit(0.0) > 1.5 * h.max_speed);
        let mut m = ShipMotion::default();
        let cruise = PilotIntent {
            cruise: true,
            ..default()
        };
        run(&mut m, &cruise, 8.0, 100.0);
        assert!(m.velocity.length() > 1.5 * h.max_speed);
    }
}
