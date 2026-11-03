//! Autopilote de développement, piloté par variables d'environnement.
//!
//! Permet à un agent IA (ou à un script) de lancer le jeu, de vérifier qu'il tourne sans
//! panique, d'en récupérer une capture et des mesures, sans intervention humaine :
//!
//! - `SH_START=<vue>[:<lieu>]` : démarre directement dans une vue (`galaxy`, `global:hexis`,
//!   `sector:test`) ou sur le menu (`menu`), sans passer par le menu principal.
//! - `SH_EXIT_AFTER_FRAMES=N` : quitte proprement après N frames.
//! - `SH_SCREENSHOT=chemin.png` : enregistre une capture puis quitte.
//! - `SH_SCREENSHOT_FRAME=N` : frame de la capture (défaut : 90, le temps que la scène se charge).
//! - `SH_PRESET=low|medium|high` : préréglage graphique (lu par `graphics`).
//!
//! À la sortie, le temps de frame moyen et maximal est toujours journalisé (hors démarrage).

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Capturing, Screenshot, save_to_disk};

use crate::data::Data;
use crate::loading::StartRequest;
use crate::state::{EnterGame, ViewLevel};

/// Frames d'attente maximale après la demande de capture avant d'abandonner.
const SCREENSHOT_TIMEOUT_FRAMES: u32 = 600;
/// Frames ignorées par les mesures après le chargement (compilation des pipelines, etc.).
const WARMUP_FRAMES: u32 = 20;

pub struct AutopilotPlugin;

impl Plugin for AutopilotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameStats>()
            .add_systems(Last, (record_frame_time, report_on_exit).chain());

        match std::env::var("SH_START").ok().as_deref().map(parse_start) {
            Some(Ok(request)) => {
                app.insert_resource(StartRequest(request));
            }
            Some(Err(message)) => {
                // L'erreur sera visible au démarrage : on refuse plutôt que de deviner.
                error!("{message}");
                app.add_systems(Startup, |mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::error());
                });
            }
            None => {}
        }

        let exit_after = env_u32("SH_EXIT_AFTER_FRAMES");
        let screenshot = std::env::var_os("SH_SCREENSHOT").map(|path| PendingScreenshot {
            path: PathBuf::from(path),
            at_frame: env_u32("SH_SCREENSHOT_FRAME").unwrap_or(90),
            requested: false,
        });
        if exit_after.is_none() && screenshot.is_none() {
            return;
        }
        if let Some(shot) = &screenshot {
            // Une ancienne capture ferait croire que la nouvelle est déjà écrite.
            let _ = std::fs::remove_file(&shot.path);
            if let Some(dir) = shot.path.parent().filter(|dir| !dir.as_os_str().is_empty())
                && let Err(err) = std::fs::create_dir_all(dir)
            {
                error!("autopilote : impossible de créer {} : {err}", dir.display());
            }
        }
        app.insert_resource(Autopilot {
            frame: 0,
            exit_after,
            screenshot,
        })
        .add_systems(Last, run_autopilot.before(record_frame_time));
    }
}

/// `galaxy`, `global:hexis`, `sector:test`, `menu` → demande de démarrage.
pub fn parse_start(value: &str) -> Result<Option<EnterGame>, String> {
    let value = value.trim();
    if value.is_empty() || value == "menu" {
        return Ok(None);
    }
    let (view, place) = match value.split_once(':') {
        Some((view, place)) => (
            view,
            Some(place.trim().to_owned()).filter(|p| !p.is_empty()),
        ),
        None => (value, None),
    };
    let view = ViewLevel::from_key(view.trim()).ok_or_else(|| {
        format!("SH_START : vue inconnue « {view} » (galaxy, global:<lieu>, sector:<lieu>, menu)")
    })?;
    Ok(Some(EnterGame { view, place }))
}

#[derive(Resource)]
struct Autopilot {
    frame: u32,
    exit_after: Option<u32>,
    screenshot: Option<PendingScreenshot>,
}

struct PendingScreenshot {
    path: PathBuf,
    at_frame: u32,
    requested: bool,
}

fn run_autopilot(
    mut commands: Commands,
    mut autopilot: ResMut<Autopilot>,
    capturing: Query<(), With<Capturing>>,
    mut exit: MessageWriter<AppExit>,
) {
    autopilot.frame += 1;
    let frame = autopilot.frame;

    if let Some(shot) = autopilot.screenshot.as_mut() {
        if !shot.requested && frame >= shot.at_frame {
            info!("autopilote : capture d'écran vers {}", shot.path.display());
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(shot.path.clone()));
            shot.requested = true;
        } else if shot.requested {
            let written = shot.path.exists() && capturing.is_empty();
            if written && frame >= shot.at_frame + 5 {
                info!("autopilote : capture écrite, fermeture");
                exit.write(AppExit::Success);
                return;
            }
            if frame >= shot.at_frame + SCREENSHOT_TIMEOUT_FRAMES {
                error!("autopilote : la capture n'a pas été écrite à temps");
                exit.write(AppExit::error());
                return;
            }
        }
    }

    if autopilot.exit_after.is_some_and(|n| frame >= n) {
        info!("autopilote : {frame} frames atteintes, fermeture");
        exit.write(AppExit::Success);
    }
}

/// Mesures de temps de frame depuis la fin du chargement.
#[derive(Resource, Default)]
struct FrameStats {
    since_loaded: u32,
    count: u32,
    total_ms: f64,
    worst_ms: f64,
    over_33ms: u32,
}

fn record_frame_time(
    time: Res<Time<Real>>,
    data: Option<Res<Data>>,
    mut stats: ResMut<FrameStats>,
) {
    if data.is_none() {
        return;
    }
    stats.since_loaded += 1;
    if stats.since_loaded <= WARMUP_FRAMES {
        return;
    }
    let ms = time.delta_secs_f64() * 1000.0;
    stats.count += 1;
    stats.total_ms += ms;
    stats.worst_ms = stats.worst_ms.max(ms);
    if ms > 33.0 {
        stats.over_33ms += 1;
    }
}

fn report_on_exit(mut exits: MessageReader<AppExit>, stats: Res<FrameStats>) {
    if exits.read().next().is_none() {
        return;
    }
    if stats.count == 0 {
        info!(
            "temps de frame : aucune mesure (sortie avant la fin du chargement + {WARMUP_FRAMES} frames)"
        );
        return;
    }
    let mean = stats.total_ms / f64::from(stats.count);
    info!(
        "temps de frame : moyenne {mean:.2} ms ({:.0} i/s), pire {:.2} ms, {} frame(s) > 33 ms, sur {} frames",
        1000.0 / mean,
        stats.worst_ms,
        stats.over_33ms,
        stats.count
    );
}

fn env_u32(name: &str) -> Option<u32> {
    std::env::var(name).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_de_sh_start() {
        assert_eq!(parse_start("menu"), Ok(None));
        assert_eq!(parse_start(""), Ok(None));
        assert_eq!(
            parse_start("galaxy"),
            Ok(Some(EnterGame {
                view: ViewLevel::Galaxy,
                place: None
            }))
        );
        assert_eq!(
            parse_start("global:hexis"),
            Ok(Some(EnterGame {
                view: ViewLevel::Global,
                place: Some("hexis".into())
            }))
        );
        assert_eq!(
            parse_start(" sector:test "),
            Ok(Some(EnterGame {
                view: ViewLevel::Sector,
                place: Some("test".into())
            }))
        );
        assert!(parse_start("cockpit").is_err());
    }
}

// --- Scénarios ----------------------------------------------------------------------------------

/// Parcours scripté du vaisseau du joueur, du sas au cockpit, en passant par toutes les
/// cellules et toutes les portes. Coordonnées Blender (x = tribord, y = avant) : le jeu reçoit
/// (x, −y). `Some(portail)` : porte à ouvrir avant de rejoindre le point.
const WALK_SHIP: &[Step] = &[
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(0.0, -3.0, None),
    w(0.0, -12.6, None),
    w(0.0, -15.6, Some("couloir_arriere__machines")),
    w(0.0, -17.0, None),
    w(0.0, -12.8, Some("couloir_arriere__machines")),
    w(0.0, -7.0, None),
    w(2.6, -7.0, Some("couloir_arriere__soute")),
    w(5.2, -7.5, None),
    w(2.6, -7.0, None),
    w(0.0, -7.0, Some("couloir_arriere__soute")),
    w(0.0, 6.0, None),
    w(-2.6, 6.0, Some("couloir_avant__quartiers")),
    w(-4.5, 5.5, None),
    w(-2.6, 6.0, None),
    w(0.0, 6.0, Some("couloir_avant__quartiers")),
    w(2.6, 6.0, Some("couloir_avant__carre")),
    w(3.2, 8.5, None),
    w(2.6, 6.0, None),
    w(0.0, 6.0, Some("couloir_avant__carre")),
    w(0.0, 12.8, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 16.4, None),
    w(0.0, 17.4, None),
    w(0.0, 18.4, None),
];

/// Fin du scénario `fly_ship` : du poste de pilotage au couloir avant, à pied, en plein vol.
const FLY_SHIP_WALK: &[Step] = &[
    w(0.0, 20.3, None),
    w(0.0, 18.0, None),
    w(0.0, 16.4, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 12.8, None),
    w(0.0, 10.0, None),
];

/// Une étape de scénario.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// Marcher jusqu'au point (x, y) du repère Blender du vaisseau — transformé par son placement
    /// quand il est amarré — après avoir ouvert la porte du portail donné.
    Walk(f32, f32, Option<&'static str>),
    /// S'asseoir au poste de pilotage (le joueur est à côté).
    SitAtHelm,
    /// Se lever.
    StandUp,
    /// Poussée vers l'avant pendant tant de secondes.
    Thrust(f32),
    /// Demander l'amarrage (en vol) ou le départ (à quai), puis attendre l'arrivée.
    DockOrDepart,
    /// Ouvrir la carte galactique (aux commandes) et attendre la vue Galaxie.
    OpenMap,
    /// Choisir ce système comme destination, sur la carte.
    Route(&'static str),
    /// Partir par ce mode, puis attendre l'arrivée aux commandes en vue Globale.
    Depart(sh_sim::travel::TravelMode),
    /// Noter la date (référence de [`Step::Expect`]).
    Mark,
    /// Vérifier qu'on est dans ce système et que la durée depuis la marque est dans
    /// [min, max] heures.
    Expect(&'static str, f64, f64),
}

const fn w(x: f32, y: f32, door: Option<&'static str>) -> Step {
    Step::Walk(x, y, door)
}

/// `port_loop` : du quai au cockpit, départ, vol, amarrage, et retour au quai (critère de M2).
/// Le quai est à 2,6 m de l'écoutille, côté passerelle (x < −5,8 dans le repère du vaisseau).
const PORT_LOOP: &[Step] = &[
    w(-6.4, -3.0, Some("sas__passerelle")),
    w(-3.8, -3.0, None),
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(0.0, -3.0, None),
    w(0.0, 12.8, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 16.4, None),
    w(0.0, 17.4, None),
    w(0.0, 18.6, None),
    Step::SitAtHelm,
    Step::DockOrDepart,
    Step::Thrust(4.0),
    Step::DockOrDepart,
    Step::StandUp,
    w(0.0, 20.3, None),
    w(0.0, 18.0, None),
    w(0.0, 16.4, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 12.8, None),
    w(0.0, -3.0, None),
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(-3.8, -3.0, None),
    w(-6.4, -3.0, Some("sas__passerelle")),
    w(-8.3, -3.0, None),
];

/// `voyage` : Hexis → Cygnara par Portail, puis retour par l'hyperespace (critère de M3).
const VOYAGE: &[Step] = &[
    Step::Mark,
    Step::OpenMap,
    Step::Route("sys_cygnara"),
    Step::Depart(sh_sim::travel::TravelMode::Portal),
    Step::Expect("sys_cygnara", 2.0, 12.0),
    Step::Mark,
    Step::OpenMap,
    Step::Route("sys_hexis"),
    Step::Depart(sh_sim::travel::TravelMode::Hyperspace),
    Step::Expect("sys_hexis", 15.0 * 24.0, 35.0 * 24.0),
];

/// Délai maximal d'une transition de vue (amarrage, départ) dans un scénario.
const TRANSITION_SECONDS: f32 = 30.0;

/// Les phases de vol du scénario `fly_ship`, chronométrées.
#[derive(Default)]
struct FlightCheck {
    step: u8,
    t: f32,
    start: Option<(sh_core::SpacePos, Quat)>,
    walk_start: Option<sh_core::SpacePos>,
}

/// Délai sans progrès au-delà duquel un scénario est déclaré bloqué.
const STUCK_SECONDS: f32 = 8.0;

#[derive(Resource)]
struct Scenario {
    name: String,
    steps: &'static [Step],
    /// Une étape non marchée a été lancée (siège, transition) ; vue au moment du lancement.
    issued: Option<Option<ViewLevel>>,
    step_time: f32,
    views_seen: Vec<ViewLevel>,
    mark: Option<crate::clock::GameClock>,
    last_issue: f32,
    /// `fly_ship` : les phases de vol précèdent la marche.
    flight: Option<FlightCheck>,
    index: usize,
    best_distance: f32,
    since_progress: f32,
    started: bool,
    frames: u32,
    worst_ms: f64,
    over_33ms: u32,
    total_ms: f64,
}

pub struct ScenarioPlugin;

impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        let Ok(name) = std::env::var("SH_SCENARIO") else {
            return;
        };
        let (steps, flight) = match name.as_str() {
            "walk_ship" => (WALK_SHIP, None),
            "fly_ship" => (FLY_SHIP_WALK, Some(FlightCheck::default())),
            "port_loop" => (PORT_LOOP, None),
            "voyage" => (VOYAGE, None),
            _ => (WALK_SHIP, None),
        };
        if !matches!(
            name.as_str(),
            "walk_ship" | "fly_ship" | "port_loop" | "voyage"
        ) {
            error!("SH_SCENARIO inconnu : « {name} » (walk_ship, fly_ship, port_loop, voyage)");
            app.add_systems(Startup, |mut exit: MessageWriter<AppExit>| {
                exit.write(AppExit::error());
            });
            return;
        }
        app.insert_resource(Scenario {
            name,
            steps,
            issued: None,
            step_time: 0.0,
            views_seen: Vec::new(),
            mark: None,
            last_issue: 0.0,
            flight,
            index: 0,
            best_distance: f32::MAX,
            since_progress: 0.0,
            started: false,
            frames: 0,
            worst_ms: 0.0,
            over_33ms: 0,
            total_ms: 0.0,
        })
        .add_systems(
            Update,
            run_scenario
                .before(crate::player::PlayerSystems)
                .run_if(scenario_can_run),
        );
    }
}

/// Un scénario joue à bord (un secteur est chargé) ou sur la carte.
fn scenario_can_run(
    sector: Option<Res<crate::sector::ActiveSector>>,
    view: Option<Res<State<ViewLevel>>>,
) -> bool {
    sector.is_some() || view.is_some_and(|v| *v.get() == ViewLevel::Galaxy)
}

#[allow(clippy::too_many_arguments)]
fn run_scenario(
    mut commands: Commands,
    time: Res<Time>,
    real: Res<Time<Real>>,
    mut scenario: ResMut<Scenario>,
    mut scripted: ResMut<crate::player::ScriptedControl>,
    mut intent: ResMut<crate::player::PlayerIntent>,
    mut pilot: ResMut<crate::ship::PilotIntent>,
    cells: Res<crate::sector::streaming::CellStates>,
    player: Query<
        (&GlobalTransform, &crate::player::PlayerController),
        With<crate::player::Player>,
    >,
    mut ships: Query<(
        &crate::space::SpacePosition,
        &crate::space::SpaceRotation,
        &mut crate::ship::ShipMotion,
    )>,
    mut doors: Query<&mut crate::sector::doors::Door>,
    (sector, docked, view, sequence): (
        Option<Res<crate::sector::ActiveSector>>,
        Option<Res<crate::sector::Docked>>,
        Res<State<ViewLevel>>,
        Option<Res<crate::transition::Sequence>>,
    ),
    travel: (
        Option<Res<crate::clock::GameClock>>,
        Option<Res<crate::voyage::Location>>,
    ),
    mut exit: MessageWriter<AppExit>,
) {
    use crate::sector::doors::DoorState;

    scripted.0 = true;
    *intent = crate::player::PlayerIntent::default();
    *pilot = crate::ship::PilotIntent::default();
    let current_view = *view.get();
    // Étapes de la carte et des voyages : elles se jouent aussi sans joueur (vue Galaxie).
    if let Some(&step) = scenario.steps.get(scenario.index)
        && matches!(
            step,
            Step::OpenMap | Step::Route(_) | Step::Depart(_) | Step::Mark | Step::Expect(..)
        )
    {
        let seated = player
            .single()
            .is_ok_and(|(_, c)| matches!(c.mode, crate::player::PlayerMode::Seated { .. }));
        match voyage_step(
            &mut scenario,
            step,
            current_view,
            seated,
            sequence.is_some(),
            &mut commands,
            time.delta_secs(),
            &travel,
        ) {
            Ok(()) => {}
            Err(message) => {
                error!("scénario {} : {message}", scenario.name);
                exit.write(AppExit::error());
            }
        }
        return;
    }
    let Ok((transform, controller)) = player.single() else {
        return;
    };
    if let Some(flight) = scenario.flight.as_mut() {
        let seated = matches!(controller.mode, crate::player::PlayerMode::Seated { .. });
        if flight.step < 5 {
            let Ok((pos, rot, mut motion)) = ships.single_mut() else {
                return;
            };
            if !seated {
                return; // attend d'être assis au poste de pilotage
            }
            match fly_phase(
                flight,
                pos.0,
                rot.0,
                &mut motion,
                &mut pilot,
                time.delta_secs(),
            ) {
                Ok(true) => commands.queue(crate::interaction::stand_up),
                Ok(false) => {}
                Err(message) => {
                    error!("scénario {} : {message}", scenario.name);
                    exit.write(AppExit::error());
                }
            }
            return;
        }
        if flight.walk_start.is_none()
            && let Ok((pos, _, _)) = ships.single()
        {
            flight.walk_start = Some(pos.0);
        }
    }
    let seated_now = matches!(controller.mode, crate::player::PlayerMode::Seated { .. });
    if !(controller.grounded || seated_now) && !scenario.started {
        return; // attend que le joueur soit posé (cellule prête, physique active)
    }
    if !scenario.started {
        scenario.started = true;
        info!(
            "scénario {} : départ ({} étapes)",
            scenario.name,
            scenario.steps.len()
        );
        info!(
            "scénario : joueur posé en {:?}, hauteur {:.2}",
            transform.translation(),
            controller.height
        );
    }
    // Mesures pendant le parcours seulement (transitions de cellules comprises).
    let ms = real.delta_secs_f64() * 1000.0;
    scenario.frames += 1;
    scenario.total_ms += ms;
    scenario.worst_ms = scenario.worst_ms.max(ms);
    if ms > 33.0 {
        scenario.over_33ms += 1;
    }

    if scenario.views_seen.last() != Some(&current_view) {
        scenario.views_seen.push(current_view);
    }
    let Some(&step) = scenario.steps.get(scenario.index) else {
        let mean = scenario.total_ms / f64::from(scenario.frames.max(1));
        // `fly_ship` : le vaisseau a continué sur sa lancée pendant la marche.
        if let Some(start) = scenario.flight.as_ref().and_then(|f| f.walk_start)
            && let Ok((pos, _, _)) = ships.single()
        {
            let drift = pos.0.distance(start);
            info!(
                "scénario {} : le vaisseau a parcouru {drift:.0} m pendant la marche",
                scenario.name
            );
            if drift < 100.0 {
                error!(
                    "scénario {} : le vaisseau ne volait pas pendant la marche",
                    scenario.name
                );
                exit.write(AppExit::error());
                return;
            }
        }
        info!(
            "scénario {} terminé : {} transitions de cellule, vues {:?}, temps de frame moyen {mean:.2} ms, pire {:.2} ms, {} frame(s) > 33 ms",
            scenario.name,
            cells.transitions,
            scenario.views_seen,
            scenario.worst_ms,
            scenario.over_33ms
        );
        exit.write(AppExit::Success);
        return;
    };

    let seated = matches!(controller.mode, crate::player::PlayerMode::Seated { .. });
    let dt = time.delta_secs();
    let advance = |scenario: &mut Scenario| {
        scenario.index += 1;
        scenario.issued = None;
        scenario.step_time = 0.0;
        scenario.best_distance = f32::MAX;
        scenario.since_progress = 0.0;
    };
    let (x, y, door) = match step {
        Step::Walk(x, y, door) => (x, y, door),
        Step::Thrust(seconds) => {
            pilot.thrust = Vec3::NEG_Z;
            scenario.step_time += dt;
            if scenario.step_time >= seconds {
                advance(&mut scenario);
            }
            return;
        }
        Step::OpenMap | Step::Route(_) | Step::Depart(_) | Step::Mark | Step::Expect(..) => return,
        Step::SitAtHelm | Step::StandUp | Step::DockOrDepart => {
            let done = match step {
                Step::SitAtHelm => seated,
                Step::StandUp => !seated,
                _ => {
                    scenario.issued.is_some_and(|v| v != Some(current_view))
                        && sequence.is_none()
                        && seated
                }
            };
            if done {
                info!(
                    "scénario {} : étape {:?} faite ({current_view:?})",
                    scenario.name, step
                );
                advance(&mut scenario);
                return;
            }
            if scenario.issued.is_none() {
                scenario.issued = Some(Some(current_view));
                match step {
                    Step::SitAtHelm => commands.queue(|world: &mut World| {
                        crate::interaction::sit_at(world, "pilot");
                    }),
                    Step::StandUp => commands.queue(crate::interaction::stand_up),
                    _ => commands.queue(crate::transition::request_dock),
                }
            }
            scenario.step_time += dt;
            if scenario.step_time > TRANSITION_SECONDS {
                error!(
                    "scénario {} bloqué à l'étape {} ({step:?}, vue {current_view:?})",
                    scenario.name, scenario.index
                );
                exit.write(AppExit::error());
            }
            return;
        }
    };

    // Porte à ouvrir d'abord : on attend qu'elle soit grande ouverte.
    if let Some(portal) = door
        && let Some(mut d) = doors.iter_mut().find(|d| d.portal == portal)
        && d.state != DoorState::Open
    {
        d.request_open();
        scenario.since_progress = 0.0;
        return;
    }

    // Repère du vaisseau ; amarré, ses cellules sont placées dans celui du quartier.
    let placement = docked
        .is_some()
        .then(|| {
            sector
                .as_ref()
                .and_then(|s| s.sector.cells.iter().find_map(|c| c.placement))
        })
        .flatten();
    let target = placement.map_or(Vec3::new(x, 0.0, -y), |p| {
        crate::sector::v3(p.apply((x, 0.0, -y)))
    });
    let pos = transform.translation();
    let to = Vec2::new(target.x - pos.x, target.z - pos.z);
    let distance = to.length();
    if distance < 0.35 {
        advance(&mut scenario);
        return;
    }
    if distance < scenario.best_distance - 0.05 {
        scenario.best_distance = distance;
        scenario.since_progress = 0.0;
    } else {
        scenario.since_progress += time.delta_secs();
        if scenario.since_progress > STUCK_SECONDS {
            error!(
                "scénario {} bloqué à l'étape {} (cible {:?}, position {:?}, cellule {:?})",
                scenario.name, scenario.index, target, pos, cells.current
            );
            exit.write(AppExit::error());
            return;
        }
    }
    // Se tourner vers la cible (le contrôleur soustrait `look.x` au lacet), puis avancer.
    let desired_yaw = (-to.x).atan2(-to.y);
    let mut delta = controller.yaw - desired_yaw;
    delta = (delta + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    intent.look = Vec2::new(
        delta * (time.delta_secs() * 10.0).min(1.0),
        controller.pitch * 0.1,
    );
    let aligned = delta.abs() < 0.5;
    intent.movement = Vec2::new(0.0, if aligned { 1.0 } else { 0.25 });
}

/// Les phases de vol de `fly_ship`. Renvoie `Ok(true)` quand il est temps de se lever.
fn fly_phase(
    flight: &mut FlightCheck,
    pos: sh_core::SpacePos,
    rot: Quat,
    motion: &mut crate::ship::ShipMotion,
    pilot: &mut crate::ship::PilotIntent,
    dt: f32,
) -> Result<bool, String> {
    use crate::ship::FlightMode;

    let (start, start_rot) = *flight.start.get_or_insert((pos, rot));
    flight.t += dt;
    let speed = motion.velocity.length();
    // (durée, commande) de chaque phase ; la vérification se fait à la fin de la phase.
    let duration = [6.0, 4.0, 22.0, 5.0, 2.5][usize::from(flight.step)];
    match flight.step {
        0 => pilot.thrust = Vec3::NEG_Z,
        1 => pilot.cruise = true,
        3 => pilot.turn = Vec2::new(if flight.t < 3.0 { -1.0 } else { 0.0 }, 0.0),
        4 => {
            motion.mode = FlightMode::Decoupled;
            pilot.thrust = Vec3::NEG_Z;
        }
        _ => {}
    }
    if flight.t < duration {
        return Ok(false);
    }
    let check = match flight.step {
        0 => (
            (240.0..=251.0).contains(&speed),
            format!(
                "vol assisté : {speed:.1} m/s, {:.0} m parcourus",
                pos.distance(start)
            ),
        ),
        1 => (speed > 2000.0, format!("croisière : {speed:.0} m/s")),
        2 => (speed < 5.0, format!("retour au calme : {speed:.2} m/s")),
        3 => {
            let turned = (rot * Vec3::NEG_Z)
                .angle_between(start_rot * Vec3::NEG_Z)
                .to_degrees();
            (turned > 45.0, format!("lacet : {turned:.0}°"))
        }
        _ => (
            speed > 60.0,
            format!("vol découplé avant de se lever : {speed:.0} m/s"),
        ),
    };
    if !check.0 {
        return Err(format!("échec de la phase {} — {}", flight.step, check.1));
    }
    info!("scénario fly_ship : {}", check.1);
    flight.step += 1;
    flight.t = 0.0;
    Ok(flight.step == 5)
}

/// Une étape de carte ou de voyage. `Ok` : on continue (étape faite ou en cours).
#[allow(clippy::too_many_arguments)]
fn voyage_step(
    scenario: &mut Scenario,
    step: Step,
    view: ViewLevel,
    seated: bool,
    in_transition: bool,
    commands: &mut Commands,
    dt: f32,
    travel: &(
        Option<Res<crate::clock::GameClock>>,
        Option<Res<crate::voyage::Location>>,
    ),
) -> Result<(), String> {
    let (clock, location) = travel;
    let next = |scenario: &mut Scenario| {
        scenario.index += 1;
        scenario.issued = None;
        scenario.step_time = 0.0;
        scenario.last_issue = 0.0;
    };
    scenario.step_time += dt;
    if scenario.step_time > TRANSITION_SECONDS {
        return Err(format!(
            "bloqué à l'étape {} ({step:?}, vue {view:?})",
            scenario.index
        ));
    }
    match step {
        Step::Mark => {
            scenario.mark = clock.as_ref().map(|c| (**c).clone());
            next(scenario);
        }
        Step::OpenMap => {
            // On réessaie chaque seconde : le contexte Pilotage s'active un frame après
            // l'arrivée au siège, et la carte ne s'ouvre qu'aux commandes.
            let retry = scenario.step_time - scenario.last_issue > 1.0;
            if view == ViewLevel::Galaxy && !in_transition {
                next(scenario);
            } else if (scenario.issued.is_none() || retry)
                && view == ViewLevel::Global
                && seated
                && !in_transition
            {
                scenario.issued = Some(Some(view));
                scenario.last_issue = scenario.step_time;
                commands.queue(crate::voyage::toggle_map);
            }
        }
        Step::Route(dest) => {
            let dest = sh_core::Id::<sh_data::Place>::new(dest).map_err(|e| e.to_string())?;
            commands.queue(move |world: &mut World| {
                let data = world.resource::<crate::data::Data>().clone();
                if let Some(from) = world
                    .get_resource::<crate::voyage::Location>()
                    .map(|l| l.system.clone())
                {
                    world.insert_resource(crate::voyage::plan_route(&data, &from, &dest));
                }
            });
            next(scenario);
        }
        Step::Depart(mode) => {
            let arrived =
                scenario.issued.is_some() && view == ViewLevel::Global && seated && !in_transition;
            if arrived {
                info!("scénario {} : arrivée après {mode:?}", scenario.name);
                next(scenario);
            } else if scenario.issued.is_none() && view == ViewLevel::Galaxy {
                scenario.issued = Some(Some(view));
                commands.queue(move |world: &mut World| crate::voyage::depart(world, mode));
            }
        }
        Step::Expect(system, min, max) => {
            let here = location.as_ref().map(|l| l.system.to_string());
            let hours = match (clock, &scenario.mark) {
                (Some(now), Some(mark)) => now.hours_since(mark),
                _ => f64::NAN,
            };
            if here.as_deref() != Some(system) || !(min..=max).contains(&hours) {
                return Err(format!(
                    "attendu {system} en {min}–{max} h ; on est à {here:?} après {hours:.1} h"
                ));
            }
            info!(
                "scénario {} : {system} atteint en {hours:.1} h ({:.1} j)",
                scenario.name,
                hours / 24.0
            );
            next(scenario);
        }
        _ => next(scenario),
    }
    Ok(())
}
