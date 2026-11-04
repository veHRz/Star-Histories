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

use bevy::math::DVec3;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use sh_core::SpacePos;
use sh_data::{Handling, SystemScale};

use crate::data::Data;
use crate::input::{Boost, LeaveSeat, Roll, Steer, Thrust, ToggleFlightAssist};
use crate::interaction::{seat_eye, sit};
use crate::player::{Player, PlayerController, PlayerMode, PlayerSystems, ScriptedControl};
use crate::sector::gltf::{Interactable, Socket};
use crate::sector::{Aboard, ActiveSector};
use crate::space::{PositionDriver, SpacePosition, SpaceRotation};
use crate::state::{ControlMode, ViewLevel};
use crate::transition::Sequence;

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PilotIntent>()
            .add_observer(on_toggle_assist)
            .add_observer(on_leave_seat)
            .add_systems(
                Update,
                (
                    (seat_at_start, sync_control_mode),
                    // En vol seulement, et pas pendant une approche automatique.
                    (read_pilot_input, fly)
                        .run_if(in_state(ViewLevel::Global))
                        .run_if(not(resource_exists::<Sequence>)),
                )
                    .chain()
                    .in_set(ShipSystems)
                    .after(PlayerSystems)
                    .run_if(resource_exists::<ActiveSector>),
            );
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
) {
    if scripted.0 {
        return;
    }
    if *mode.get() != ControlMode::Piloting {
        *intent = PilotIntent::default();
        return;
    }
    let steer: Vec2 = ***steer * settings.look_scale();
    *intent = PilotIntent {
        thrust: ***thrust,
        turn: Vec2::new(steer.x, -steer.y) / MOUSE_FULL_TURN,
        roll: ***roll,
        cruise: ***boost,
    };
}

fn on_toggle_assist(_: On<Start<ToggleFlightAssist>>, mut ships: Query<&mut ShipMotion>) {
    for mut motion in &mut ships {
        motion.mode = match motion.mode {
            FlightMode::Assisted => FlightMode::Decoupled,
            FlightMode::Decoupled => FlightMode::Assisted,
        };
        info!("vol : {:?}", motion.mode);
    }
}

fn on_leave_seat(_: On<Start<LeaveSeat>>, mut commands: Commands) {
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
