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
const WALK_SHIP: &[(f32, f32, Option<&str>)] = &[
    (-2.4, -3.0, Some("couloir_arriere__sas")),
    (0.0, -3.0, None),
    (0.0, -12.6, None),
    (0.0, -15.6, Some("couloir_arriere__machines")),
    (0.0, -17.0, None),
    (0.0, -12.8, Some("couloir_arriere__machines")),
    (0.0, -7.0, None),
    (2.6, -7.0, Some("couloir_arriere__soute")),
    (5.2, -7.5, None),
    (2.6, -7.0, None),
    (0.0, -7.0, Some("couloir_arriere__soute")),
    (0.0, 6.0, None),
    (-2.6, 6.0, Some("couloir_avant__quartiers")),
    (-4.5, 5.5, None),
    (-2.6, 6.0, None),
    (0.0, 6.0, Some("couloir_avant__quartiers")),
    (2.6, 6.0, Some("couloir_avant__carre")),
    (3.2, 8.5, None),
    (2.6, 6.0, None),
    (0.0, 6.0, Some("couloir_avant__carre")),
    (0.0, 12.8, None),
    (0.0, 15.0, Some("couloir_avant__cockpit")),
    (0.0, 16.4, None),
    (0.0, 17.4, None),
    (0.0, 18.4, None),
];

/// Délai sans progrès au-delà duquel un scénario est déclaré bloqué.
const STUCK_SECONDS: f32 = 8.0;

#[derive(Resource)]
struct Scenario {
    name: String,
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
        if name != "walk_ship" {
            error!("SH_SCENARIO inconnu : « {name} » (walk_ship)");
            app.add_systems(Startup, |mut exit: MessageWriter<AppExit>| {
                exit.write(AppExit::error());
            });
            return;
        }
        app.insert_resource(Scenario {
            name,
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
                .run_if(resource_exists::<crate::sector::ActiveSector>),
        );
    }
}

fn run_scenario(
    time: Res<Time>,
    real: Res<Time<Real>>,
    mut scenario: ResMut<Scenario>,
    mut scripted: ResMut<crate::player::ScriptedControl>,
    mut intent: ResMut<crate::player::PlayerIntent>,
    cells: Res<crate::sector::streaming::CellStates>,
    player: Query<
        (&GlobalTransform, &crate::player::PlayerController),
        With<crate::player::Player>,
    >,
    mut doors: Query<&mut crate::sector::doors::Door>,
    mut exit: MessageWriter<AppExit>,
) {
    use crate::sector::doors::DoorState;

    scripted.0 = true;
    *intent = crate::player::PlayerIntent::default();
    let Ok((transform, controller)) = player.single() else {
        return;
    };
    if !controller.grounded && !scenario.started {
        return; // attend que le joueur soit posé (cellule prête, physique active)
    }
    if !scenario.started {
        scenario.started = true;
        info!(
            "scénario {} : départ ({} étapes)",
            scenario.name,
            WALK_SHIP.len()
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

    let Some(&(x, y, door)) = WALK_SHIP.get(scenario.index) else {
        let mean = scenario.total_ms / f64::from(scenario.frames.max(1));
        info!(
            "scénario {} terminé : {} transitions de cellule, temps de frame moyen {mean:.2} ms, pire {:.2} ms, {} frame(s) > 33 ms",
            scenario.name, cells.transitions, scenario.worst_ms, scenario.over_33ms
        );
        exit.write(AppExit::Success);
        return;
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

    let target = Vec3::new(x, 0.0, -y);
    let pos = transform.translation();
    let to = Vec2::new(target.x - pos.x, target.z - pos.z);
    let distance = to.length();
    if distance < 0.35 {
        scenario.index += 1;
        scenario.best_distance = f32::MAX;
        scenario.since_progress = 0.0;
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
