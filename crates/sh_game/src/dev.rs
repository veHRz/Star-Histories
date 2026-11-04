//! Autopilote de développement, piloté par variables d'environnement.
//!
//! Permet à un agent IA (ou à un script) de lancer le jeu, de vérifier qu'il tourne sans
//! panique, d'en récupérer une capture et des mesures, sans intervention humaine :
//!
//! - `SH_START=<vue>[:<lieu>]` : démarre directement dans une vue (`galaxy`, `global:hexis`,
//!   `sector:test`), dans une nouvelle partie comme le bouton du menu (`new`), dans une
//!   sauvegarde du dossier du joueur (`load:<fichier>`) ou sur le menu (`menu`).
//! - `SH_USER_DIR=<dossier>` : dossier des sauvegardes et réglages (jamais ceux du joueur).
//! - `SH_EXIT_AFTER_FRAMES=N` : quitte proprement après N frames.
//! - `SH_SCREENSHOT=chemin.png` : enregistre une capture puis quitte.
//! - `SH_SCREENSHOT_FRAME=N` : frame de la capture (défaut : 90, le temps que la scène se charge).
//! - `SH_PRESET=low|medium|high` : préréglage graphique (lu par `graphics`).
//! - `SH_WINDOW=<L>x<H>|fullscreen` : taille de la fenêtre, ou plein écran sans bordure (les
//!   mesures de la machine de référence se font en 1080p).
//!
//! À la sortie, le temps de frame moyen et maximal est toujours journalisé (hors démarrage).

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Capturing, Screenshot, save_to_disk};
use bevy::window::{MonitorSelection, WindowMode};
use bevy::winit::WinitSettings;

use crate::data::Data;
use crate::loading::StartRequest;
use crate::state::{AppState, EnterGame, ViewLevel};

/// Frames d'attente maximale après la demande de capture avant d'abandonner.
const SCREENSHOT_TIMEOUT_FRAMES: u32 = 600;
/// Frames ignorées par les mesures après le chargement (compilation des pipelines, etc.).
const WARMUP_FRAMES: u32 = 20;

pub struct AutopilotPlugin;

impl Plugin for AutopilotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameStats>()
            .add_systems(Last, (record_frame_time, report_on_exit).chain());
        // `SH_PAUSE=1|load` : le menu de pause (ou sa liste de sauvegardes), pour les captures.
        if let Ok(value) = std::env::var("SH_PAUSE") {
            app.insert_resource(PauseShot {
                load: value.trim() == "load",
            })
            .add_systems(
                Update,
                pause_shot.run_if(resource_exists::<crate::sector::ActiveSector>),
            );
        }
        if let Ok(value) = std::env::var("SH_TERMINAL") {
            use crate::ui::terminal::Screen;
            let screen = match value.trim() {
                "fiche" => Screen::Sheet,
                "journal" => Screen::Journal,
                _ => Screen::Codex,
            };
            app.insert_resource(TerminalShot(screen)).add_systems(
                Update,
                terminal_shot.run_if(resource_exists::<crate::career::Career>),
            );
        }
        // `SH_OPTIONS=1|keys` : l'écran des options (ou sa page des touches) au menu principal.
        if let Ok(value) = std::env::var("SH_OPTIONS") {
            let keys = value.trim() == "keys";
            app.add_systems(
                OnEnter(AppState::MainMenu),
                move |mut commands: Commands| {
                    if keys {
                        crate::ui::options::open_keys(&mut commands);
                    } else {
                        crate::ui::options::open(&mut commands);
                    }
                },
            );
        }
        if std::env::var_os("SH_GPU_TIMINGS").is_some() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
                .init_resource::<GpuTimings>()
                .add_systems(
                    Last,
                    (record_gpu_timings, report_gpu_timings)
                        .chain()
                        .after(record_frame_time),
                );
        }

        // Bevy bride à 60 Hz une fenêtre sans focus (`WinitSettings::game`) ; une fenêtre lancée
        // par l'autopilote n'a souvent pas le focus : ses frames rapides seraient comptées 16,7 ms.
        if ["SH_EXIT_AFTER_FRAMES", "SH_SCREENSHOT", "SH_SCENARIO"]
            .iter()
            .any(|name| std::env::var_os(name).is_some())
        {
            app.insert_resource(WinitSettings::continuous());
        }

        match std::env::var("SH_START").ok().as_deref().map(parse_start) {
            Some(Ok(request)) => {
                app.insert_resource(request);
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

/// `galaxy`, `global:hexis`, `sector:test`, `new`, `menu` → demande de démarrage.
pub fn parse_start(value: &str) -> Result<StartRequest, String> {
    let value = value.trim();
    if value.is_empty() || value == "menu" {
        return Ok(StartRequest::Menu);
    }
    if value == "new" {
        return Ok(StartRequest::NewGame);
    }
    if let Some(name) = value.strip_prefix("load:") {
        return Ok(StartRequest::Load(name.trim().to_owned()));
    }
    let (view, place) = match value.split_once(':') {
        Some((view, place)) => (
            view,
            Some(place.trim().to_owned()).filter(|p| !p.is_empty()),
        ),
        None => (value, None),
    };
    let view = ViewLevel::from_key(view.trim()).ok_or_else(|| {
        format!(
            "SH_START : vue inconnue « {view} » (galaxy, global:<lieu>, sector:<lieu>, new, menu)"
        )
    })?;
    Ok(StartRequest::Direct(EnterGame { view, place }))
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

#[derive(Resource)]
struct PauseShot {
    load: bool,
}

/// `SH_TERMINAL=codex|fiche|journal` : ouvre cet écran du terminal (captures).
#[derive(Resource)]
struct TerminalShot(crate::ui::terminal::Screen);

fn terminal_shot(
    time: Res<Time<Real>>,
    shot: Res<TerminalShot>,
    mut open: MessageWriter<crate::ui::terminal::OpenTerminal>,
    mut since: Local<f32>,
) {
    let before = *since;
    *since += time.delta_secs();
    if before <= 3.0 && *since > 3.0 {
        open.write(crate::ui::terminal::OpenTerminal(shot.0.clone()));
    }
}

/// Ouvre le menu de pause trois secondes après l'entrée à bord.
fn pause_shot(
    mut commands: Commands,
    time: Res<Time<Real>>,
    shot: Res<PauseShot>,
    mut since: Local<f32>,
) {
    let before = *since;
    *since += time.delta_secs();
    if before <= 3.0 && *since > 3.0 {
        let load = shot.load;
        commands.queue(move |world: &mut World| crate::ui::pause::open(world, load));
    }
}

/// `SH_GPU_TIMINGS=1` : temps GPU moyen de chaque passe de rendu (requêtes d'horodatage,
/// Vulkan et DX12), cumulé depuis la fin du chargement et journalisé à la sortie. Sur l'UHD 770,
/// certaines passes (le bloom) comptent aussi des attentes : comparer, pas additionner.
#[derive(Resource, Default)]
struct GpuTimings(std::collections::BTreeMap<String, (f64, u32)>);

fn record_gpu_timings(
    store: Res<bevy::diagnostic::DiagnosticsStore>,
    stats: Res<FrameStats>,
    mut timings: ResMut<GpuTimings>,
) {
    if stats.since_loaded <= WARMUP_FRAMES {
        return;
    }
    for diagnostic in store.iter() {
        let path = diagnostic.path().as_str();
        if !path.ends_with("elapsed_gpu") {
            continue;
        }
        if let Some(value) = diagnostic.measurement().map(|m| m.value) {
            let entry = timings.0.entry(path.to_owned()).or_default();
            entry.0 += value;
            entry.1 += 1;
        }
    }
}

fn report_gpu_timings(mut exits: MessageReader<AppExit>, timings: Res<GpuTimings>) {
    if exits.read().next().is_none() {
        return;
    }
    let mut passes: Vec<(&String, f64)> = timings
        .0
        .iter()
        .map(|(path, (total, n))| (path, total / f64::from((*n).max(1))))
        .collect();
    passes.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (path, ms) in passes {
        info!("temps GPU : {ms:6.2} ms  {path}");
    }
}

/// L'autopilote pilote-t-il cette partie (démarrage direct, scénario, capture, sortie) ?
pub fn autopilot_active() -> bool {
    [
        "SH_START",
        "SH_SCENARIO",
        "SH_SCREENSHOT",
        "SH_EXIT_AFTER_FRAMES",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
}

fn env_u32(name: &str) -> Option<u32> {
    std::env::var(name).ok()?.trim().parse().ok()
}

/// Applique `SH_WINDOW` à la fenêtre principale. Appelée avant l'installation du journal :
/// une valeur illisible est ignorée et renvoyée en erreur, à journaliser par l'appelant.
pub fn apply_window_env(window: &mut Window) -> Result<(), String> {
    let Ok(value) = std::env::var("SH_WINDOW") else {
        return Ok(());
    };
    match parse_window(&value) {
        Some(WindowSetting::Fullscreen) => {
            window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Primary);
        }
        Some(WindowSetting::Size(width, height)) => window.resolution.set(width, height),
        None => {
            return Err(format!(
                "SH_WINDOW : « {value} » illisible (attendu <L>x<H> ou fullscreen), ignoré"
            ));
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
enum WindowSetting {
    Fullscreen,
    Size(f32, f32),
}

fn parse_window(value: &str) -> Option<WindowSetting> {
    let value = value.trim();
    if value == "fullscreen" {
        return Some(WindowSetting::Fullscreen);
    }
    let (width, height) = value.split_once('x')?;
    let width: u16 = width.trim().parse().ok()?;
    let height: u16 = height.trim().parse().ok()?;
    (width > 0 && height > 0).then(|| WindowSetting::Size(f32::from(width), f32::from(height)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_de_sh_window() {
        assert_eq!(parse_window("fullscreen"), Some(WindowSetting::Fullscreen));
        assert_eq!(
            parse_window(" 1920x1080 "),
            Some(WindowSetting::Size(1920.0, 1080.0))
        );
        assert_eq!(parse_window("1920"), None);
        assert_eq!(parse_window("0x1080"), None);
    }

    #[test]
    fn lecture_de_sh_start() {
        assert_eq!(parse_start("menu"), Ok(StartRequest::Menu));
        assert_eq!(parse_start(""), Ok(StartRequest::Menu));
        assert_eq!(parse_start(" new "), Ok(StartRequest::NewGame));
        assert_eq!(
            parse_start("galaxy"),
            Ok(StartRequest::Direct(EnterGame {
                view: ViewLevel::Galaxy,
                place: None
            }))
        );
        assert_eq!(
            parse_start("global:hexis"),
            Ok(StartRequest::Direct(EnterGame {
                view: ViewLevel::Global,
                place: Some("hexis".into())
            }))
        );
        assert_eq!(
            parse_start(" sector:test "),
            Ok(StartRequest::Direct(EnterGame {
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
    /// Ouvrir le terminal (raccourci de l'autopilote : comme si l'on avait utilisé la console).
    Open(Terminal),
    /// Prendre l'option du terminal qui correspond au motif ([`choice_matches`]).
    Choose(&'static str),
    /// Touche d'amarrage aux commandes, une fois (livrer, accrocher une épave).
    Hail,
    /// Vérifier la fiche du capitaine.
    Assert(
        &'static str,
        fn(&sh_sim::Captain, &sh_data::GameData) -> bool,
    ),
    /// Capture d'écran dans `target/shots/<nom>.png`, puis une seconde d'attente.
    Shot(&'static str),
    /// Attendre tant de secondes (chargement des modèles avant une capture).
    Wait(f32),
    /// Jouer le raid en cours (combattre, fuir, rester passif) jusqu'à l'issue attendue, en
    /// au plus tant de secondes ; capture facultative après quelques secondes de combat.
    Raid(RaidPlay, RaidEnd, f32, Option<(&'static str, f32)>),
    /// Combattre à pied (ou rester passif) jusqu'à ce que les hostiles tombent (`true`) ou que
    /// le joueur tombe (`false`), en au plus tant de secondes ; capture facultative.
    Firefight(bool, bool, f32, Option<(&'static str, f32)>),
    /// Placer le vaisseau, à l'arrêt, à 1 km d'une cible (`wreck` : l'épave d'un remorquage ;
    /// `shipyard` : les chantiers du système). Raccourci de l'autopilote : le vol est testé
    /// par `fly_ship`.
    JumpNear(&'static str),
}

/// La conduite de l'autopilote pendant un raid.
#[derive(Clone, Copy, Debug)]
enum RaidPlay {
    Fight,
    Flee,
    Idle,
}

/// L'issue attendue d'un raid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RaidEnd {
    Won,
    Fled,
    Lost,
}

/// Un écran du terminal à ouvrir.
#[derive(Clone, Copy, Debug)]
enum Terminal {
    Dialogue(&'static str),
    Market,
    Sheet,
    Journal,
}

/// Les étapes de terminal et de carrière : elles se jouent aux commandes, sans marcher.
fn is_career_step(step: &Step) -> bool {
    matches!(
        step,
        Step::Open(_)
            | Step::Choose(_)
            | Step::Hail
            | Step::Assert(..)
            | Step::Shot(_)
            | Step::Wait(_)
            | Step::JumpNear(_)
    )
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

/// `contract_loop` : contrat de transport Hexis → Cygnara, livraison, paiement, réputation,
/// échéance de la Banque, puis trahison d'un second contrat et ses conséquences (critère de M5).
const CONTRACT_LOOP: &[Step] = &[
    Step::Wait(4.0),
    Step::Open(Terminal::Dialogue("hexis_contract_office")),
    Step::Shot("m5-bureau"),
    Step::Choose("say:dlg.office.ask_work"),
    Step::Choose("say:dlg.office.show_contracts"),
    Step::Shot("m5-contrats"),
    Step::Choose("accept:sys_cygnara"),
    Step::Choose("close"),
    Step::Assert("contrat pris", |c, _| {
        c.contracts
            .iter()
            .any(|k| k.state == sh_sim::ContractState::Active && k.to.as_str() == "sys_cygnara")
    }),
    Step::OpenMap,
    Step::Route("sys_cygnara"),
    Step::Depart(sh_sim::travel::TravelMode::Portal),
    Step::Hail,
    Step::Assert("payé, réputation en hausse", |c, _| {
        let id = |s: &str| sh_core::Id::new(s).ok();
        c.contracts
            .iter()
            .any(|k| k.state == sh_sim::ContractState::Done)
            && c.credits.cents > 300_000
            && id("hc_logistique").is_some_and(|f| c.reputation(&f) > 0)
    }),
    Step::Open(Terminal::Journal),
    Step::Shot("m5-journal"),
    Step::Choose("close"),
    Step::OpenMap,
    Step::Route("sys_hexis"),
    Step::Depart(sh_sim::travel::TravelMode::Portal),
    Step::Open(Terminal::Dialogue("hexis_contract_office")),
    Step::Choose("say:dlg.office.ask_bank"),
    Step::Choose("say:dlg.office.pay"),
    Step::Choose("close"),
    Step::Assert("échéance payée", |c, _| {
        c.loan
            .as_ref()
            .is_some_and(|l| l.balance.cents == 4_250_000)
    }),
    // Un remorquage (CRS) : l'épave près des chantiers d'Hexis, ramenée aux chantiers.
    Step::Open(Terminal::Dialogue("hexis_contract_office")),
    Step::Choose("say:dlg.office.ask_work"),
    Step::Choose("say:dlg.office.show_contracts"),
    Step::Choose("accept:sys_hexis"),
    Step::Choose("close"),
    Step::JumpNear("wreck"),
    Step::Wait(2.0),
    Step::Shot("m5-epave"),
    Step::Hail,
    Step::Assert("épave accrochée", |c, _| {
        c.contracts
            .iter()
            .any(|k| k.kind == sh_sim::ContractKind::Towing && k.towed)
    }),
    Step::JumpNear("shipyard"),
    Step::Hail,
    Step::Assert("remorquage payé", |c, _| {
        c.contracts.iter().any(|k| {
            k.kind == sh_sim::ContractKind::Towing && k.state == sh_sim::ContractState::Done
        })
    }),
    // La trahison : un second contrat, dont la cargaison est vendue au marché.
    Step::Open(Terminal::Dialogue("hexis_contract_office")),
    Step::Choose("say:dlg.office.ask_work"),
    Step::Choose("say:dlg.office.show_contracts"),
    Step::Choose("accept:any"),
    Step::Choose("close"),
    Step::Open(Terminal::Market),
    Step::Choose("betray"),
    Step::Shot("m5-trahison"),
    Step::Choose("close"),
    Step::Assert("trahison : exclu, traître, dette alourdie", |c, data| {
        let hc = sh_core::Id::new("hexa_corp").ok();
        hc.is_some_and(|f| c.refused_by(data, &f))
            && c.traits.contains(sh_sim::career::NOTORIOUS_TRAITOR)
            && c.loan.as_ref().is_some_and(|l| l.balance.cents > 4_250_000)
    }),
    Step::Open(Terminal::Dialogue("hexis_contract_office")),
    Step::Shot("m5-porte-close"),
    Step::Choose("close"),
    Step::Open(Terminal::Sheet),
    Step::Shot("m5-fiche"),
    Step::Choose("close"),
];

/// `raid_win` (avec `SH_START=global:cygnara SH_RAID=1`) : l'embuscade de la Ligue, combattue
/// aux côtés de la Flotte de défense jusqu'à la victoire (critère de M6).
const RAID_WIN: &[Step] = &[
    Step::Wait(3.0),
    Step::Shot("m6-contact"),
    Step::Raid(
        RaidPlay::Fight,
        RaidEnd::Won,
        400.0,
        Some(("m6-combat", 50.0)),
    ),
    Step::Shot("m6-victoire"),
    // Les dégâts persistent : retour à Hexis, puis le chantier.
    Step::OpenMap,
    Step::Route("sys_hexis"),
    Step::Depart(sh_sim::travel::TravelMode::Portal),
    Step::Assert("dégâts toujours là", |c, _| {
        c.hull.as_ref().is_some_and(|h| h.integrity() < 1.0)
    }),
    Step::JumpNear("shipyard"),
    Step::Hail,
    Step::Shot("m6-chantier"),
    Step::Choose("repair"),
    Step::Choose("close"),
    Step::Assert("réparé autant que payé", |c, _| {
        c.credits.cents < 300_000
            && (c.credits.cents == 0 || c.hull.as_ref().is_some_and(|h| h.integrity() >= 1.0))
    }),
];

/// `raid_flee` : le joueur fait demi-tour et file en croisière.
const RAID_FLEE: &[Step] = &[
    Step::Wait(3.0),
    Step::Raid(RaidPlay::Flee, RaidEnd::Fled, 180.0, None),
];

/// `raid_lose` (avec `SH_RAID=alone`) : seul et passif, le vaisseau est perdu.
const RAID_LOSE: &[Step] = &[
    Step::Wait(3.0),
    Step::Raid(
        RaidPlay::Idle,
        RaidEnd::Lost,
        240.0,
        Some(("m6-alarme", 12.0)),
    ),
    Step::Wait(1.5),
    Step::Shot("m6-mort"),
];

/// `firefight` (avec `SH_START=sector:player_ship SH_SPAWN=soute SH_BOARDING=3`) : un abordage
/// repoussé dans la soute (critère de M6, à pied).
const FIREFIGHT: &[Step] = &[
    Step::Wait(2.0),
    Step::Firefight(true, true, 150.0, Some(("m6-abordage", 1.2))),
    Step::Wait(1.0),
    Step::Shot("m6-abordage-fin"),
];

/// `firefight_lose` (mêmes variables) : passif, le joueur tombe.
const FIREFIGHT_LOSE: &[Step] = &[
    Step::Wait(2.0),
    Step::Firefight(false, false, 150.0, None),
    Step::Wait(1.0),
    Step::Shot("m6-abordage-mort"),
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
    /// La page du terminal au moment d'un choix (on attend qu'elle change).
    page_seen: Option<crate::ui::terminal::Page>,
    /// Pendant un raid : coques en jeu au plus fort du combat.
    max_ships: usize,
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
        // Ces scénarios passent par le menu principal, où les étapes ordinaires ne jouent pas.
        if name == "menu_return" {
            app.add_systems(Update, menu_return);
            return;
        }
        if name == "save_load" {
            app.init_resource::<SaveLoadCheck>()
                .add_systems(Update, save_load);
            return;
        }
        if name == "pause_escape" {
            app.init_resource::<PauseCheck>()
                .add_systems(PreUpdate, pause_escape);
            return;
        }
        let (steps, flight) = match name.as_str() {
            "walk_ship" => (WALK_SHIP, None),
            "fly_ship" => (FLY_SHIP_WALK, Some(FlightCheck::default())),
            "port_loop" => (PORT_LOOP, None),
            "voyage" => (VOYAGE, None),
            "contract_loop" => (CONTRACT_LOOP, None),
            "raid_win" => (RAID_WIN, None),
            "raid_flee" => (RAID_FLEE, None),
            "raid_lose" => (RAID_LOSE, None),
            "firefight" => (FIREFIGHT, None),
            "firefight_lose" => (FIREFIGHT_LOSE, None),
            _ => (WALK_SHIP, None),
        };
        if !matches!(
            name.as_str(),
            "walk_ship"
                | "fly_ship"
                | "port_loop"
                | "voyage"
                | "contract_loop"
                | "raid_win"
                | "raid_flee"
                | "raid_lose"
                | "firefight"
                | "firefight_lose"
        ) {
            error!(
                "SH_SCENARIO inconnu : « {name} » (walk_ship, fly_ship, port_loop, voyage, contract_loop, raid_win, raid_flee, raid_lose, firefight, firefight_lose, menu_return, save_load, pause_escape)"
            );
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
            page_seen: None,
            max_ships: 0,
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

/// `menu_return` (avec `SH_START=new`) : Échap ramène au menu principal, puis « Nouvelle partie »
/// repart de zéro. Au menu, le monde vivant ne doit plus tourner (il plantait, faute de vue).
#[allow(clippy::too_many_arguments)]
fn menu_return(
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    data: Option<Res<Data>>,
    mut career: Option<ResMut<crate::career::Career>>,
    mut app_state: ResMut<NextState<AppState>>,
    mut view: ResMut<NextState<ViewLevel>>,
    mut place: ResMut<crate::state::CurrentPlace>,
    mut phase: Local<(u8, f32, f32)>,
    mut exit: MessageWriter<AppExit>,
) {
    let (step, since, total) = &mut *phase;
    *since += time.delta_secs();
    *total += time.delta_secs();
    let in_game = *state.get() == AppState::InGame;
    match *step {
        // En jeu : on vide le compte, puis Échap (comme `views::on_pause`).
        0 if in_game && *since > 3.0 => {
            let Some(career) = career.as_mut() else {
                return;
            };
            career.credits.cents = 0;
            app_state.set(AppState::MainMenu);
            info!("scénario menu_return : Échap, retour au menu principal");
            (*step, *since) = (1, 0.0);
        }
        // Au menu, quelques secondes, puis comme le bouton « Nouvelle partie ».
        1 if !in_game && *since > 2.0 => {
            let Some(data) = data else {
                return;
            };
            EnterGame::new_game(&data).apply(&mut app_state, &mut view, &mut place);
            info!("scénario menu_return : nouvelle partie");
            (*step, *since) = (2, 0.0);
        }
        // De nouveau en jeu : le compte est celui du départ, pas celui de la partie quittée.
        2 if in_game && *since > 3.0 => {
            let expected = data
                .as_ref()
                .and_then(|d| d.start())
                .map(|s| s.credits * 100);
            let credits = career.as_ref().map(|c| c.credits.cents);
            if credits.is_some() && credits == expected {
                info!("scénario menu_return terminé : la nouvelle partie repart de zéro");
            } else {
                error!(
                    "scénario menu_return : la nouvelle partie garde l'ancienne (compte {credits:?}, attendu {expected:?})"
                );
                exit.write(AppExit::error());
                return;
            }
            exit.write(AppExit::Success);
            *step = 3;
        }
        _ if *total > 90.0 => {
            error!("scénario menu_return : bloqué à l'étape {step}");
            exit.write(AppExit::error());
        }
        _ => {}
    }
}

/// `pause_escape` : le menu de pause et les options, au vrai clavier et aux vrais boutons.
#[derive(Resource, Default)]
struct PauseCheck {
    step: usize,
    since: f32,
    total: f32,
    /// Échap enfoncé au frame précédent : le relâcher maintenant.
    releasing: bool,
}

/// Envoie un appui ou un relâchement d'Échap, comme le clavier (contextes d'entrée compris).
fn escape_key(world: &mut World, pressed: bool) {
    use bevy::input::ButtonState;
    use bevy::input::keyboard::{Key, KeyboardInput};
    let mut windows = world.query_filtered::<Entity, With<bevy::window::PrimaryWindow>>();
    let Ok(window) = windows.single(world) else {
        return;
    };
    world.write_message(KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: Key::Escape,
        state: if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
        text: None,
        repeat: false,
        window,
    });
}

fn press_button(world: &mut World, button: Option<Entity>) -> bool {
    match button {
        Some(entity) => {
            world.trigger(bevy::ui_widgets::Activate { entity });
            true
        }
        None => false,
    }
}

/// `pause_escape` (avec `SH_START=new`) : Échap ouvre la pause ; « Reprendre » et Échap la
/// ferment, écran compris ; Échap dans les options ne ferme que les options ; « Retour » aussi.
/// Le double Échap plantait (une condition de groupe évaluée une fois, la pause retirée entre
/// deux systèmes) et l'écran restait affiché après « Reprendre ».
fn pause_escape(world: &mut World) {
    use crate::ui::{options, pause};
    let dt = world.resource::<Time<Real>>().delta_secs();
    let ready = world.contains_resource::<crate::career::Career>()
        && world.contains_resource::<crate::sector::ActiveSector>();
    let mut check = world.resource_mut::<PauseCheck>();
    check.since += dt;
    check.total += dt;
    if check.releasing {
        check.releasing = false;
        escape_key(world, false);
        return;
    }
    let (step, since, total) = (check.step, check.since, check.total);
    if total > 90.0 {
        error!("scénario pause_escape : bloqué à l'étape {step}");
        world.write_message(AppExit::error());
        return;
    }
    let wait = if step == 0 { 3.0 } else { 0.5 };
    if !ready || since < wait {
        return;
    }
    let paused = world.contains_resource::<pause::Paused>();
    let pause_shown = pause::is_shown(world);
    let options_open = world.contains_resource::<options::OptionsScreen>();
    let options_shown = options::is_shown(world);
    // Ce qu'on attend avant l'étape, puis l'action de l'étape.
    let (expected, action): (bool, &str) = match step {
        0 => (!paused, "escape"),
        1 => (paused && pause_shown, "resume"),
        2 => (!paused && !pause_shown, "escape"),
        3 => (paused && pause_shown, "escape"),
        4 => (!paused && !pause_shown, "escape"),
        5 => (paused, "options"),
        6 => (options_open && options_shown, "escape"),
        7 => (
            !options_open && !options_shown && paused && pause_shown,
            "options",
        ),
        8 => (options_open && options_shown, "back"),
        9 => (!options_open && !options_shown && paused, "escape"),
        10 => (!paused && !pause_shown, "done"),
        _ => return,
    };
    if !expected {
        error!(
            "scénario pause_escape : état inattendu à l'étape {step} (pause {paused}, écran {pause_shown}, options {options_open}, écran des options {options_shown})"
        );
        world.write_message(AppExit::error());
        return;
    }
    let done = match action {
        "escape" => {
            escape_key(world, true);
            world.resource_mut::<PauseCheck>().releasing = true;
            true
        }
        "resume" => {
            let button = pause::find_button(world, false);
            press_button(world, button)
        }
        "options" => {
            let button = pause::find_button(world, true);
            press_button(world, button)
        }
        "back" => {
            let button = options::find_back_button(world);
            press_button(world, button)
        }
        _ => {
            info!(
                "scénario pause_escape terminé : pause, reprise, Échap et options sans écran fantôme"
            );
            world.write_message(AppExit::Success);
            true
        }
    };
    if !done {
        error!("scénario pause_escape : bouton introuvable à l'étape {step}");
        world.write_message(AppExit::error());
        return;
    }
    let mut check = world.resource_mut::<PauseCheck>();
    check.step += 1;
    check.since = 0.0;
}

/// `save_load` : ce qui a été sauvegardé revient au chargement.
#[derive(Resource, Default)]
struct SaveLoadCheck {
    step: u8,
    since: f32,
    total: f32,
    /// Argent, date, vue et pose du vaisseau (en vol) au moment de la sauvegarde.
    expected: Option<(
        i64,
        sh_core::StellarDate,
        ViewLevel,
        Option<sh_core::SpacePos>,
    )>,
}

/// La pose du vaisseau du joueur en vue Globale.
fn ship_pose(world: &World) -> Option<sh_core::SpacePos> {
    let root = world.get_resource::<crate::sector::ActiveSector>()?.root;
    world.get::<crate::space::SpacePosition>(root).map(|p| p.0)
}

/// `save_load` (avec `SH_USER_DIR` et `SH_START=new` ou `sector:hexis_spaceport`) : sauvegarder,
/// tout changer, recharger, retrouver l'argent, la date, la vue et la pose du vaisseau.
fn save_load(world: &mut World) {
    let dt = world.resource::<Time<Real>>().delta_secs();
    let in_game = *world.resource::<State<AppState>>().get() == AppState::InGame;
    let ready = in_game
        && world.contains_resource::<crate::career::Career>()
        && world.contains_resource::<crate::sector::ActiveSector>();
    let mut check = world.resource_mut::<SaveLoadCheck>();
    check.since += dt;
    check.total += dt;
    let (step, since, total) = (check.step, check.since, check.total);
    if total > 120.0 {
        error!("scénario save_load : bloqué à l'étape {step}");
        world.write_message(AppExit::error());
        return;
    }
    match step {
        0 if ready && since > 4.0 => {
            if let Some(mut career) = world.get_resource_mut::<crate::career::Career>() {
                career.credits.cents = 432_100;
            }
            let date = world.resource::<crate::clock::GameClock>().date;
            let view = *world.resource::<State<ViewLevel>>().get();
            // En vol, le vaisseau s'écarte de son point d'arrivée : le chargement doit le
            // remettre là où il a été sauvegardé, pas là où une arrivée le placerait.
            if view == ViewLevel::Global
                && let Some(root) = world
                    .get_resource::<crate::sector::ActiveSector>()
                    .map(|s| s.root)
                && let Some(mut pos) = world.get_mut::<crate::space::SpacePosition>(root)
            {
                pos.0 += sh_core::SpacePos::new(1500.0, 0.0, 0.0);
            }
            let pose = ship_pose(world).filter(|_| view == ViewLevel::Global);
            crate::save::save_now(world, crate::save::SaveKind::Manual);
            let Some(latest) = crate::save::list_saves().into_iter().next() else {
                error!("scénario save_load : aucune sauvegarde écrite");
                world.write_message(AppExit::error());
                return;
            };
            info!(
                "scénario save_load : sauvegarde {} ({view:?})",
                latest.path.display()
            );
            // Tout change avant de recharger : la sauvegarde doit tout rendre.
            if let Some(mut career) = world.get_resource_mut::<crate::career::Career>() {
                career.credits.cents = 0;
            }
            crate::save::request_load(world, &latest.path);
            let mut check = world.resource_mut::<SaveLoadCheck>();
            check.expected = Some((432_100, date, view, pose));
            (check.step, check.since) = (1, 0.0);
        }
        // Le chargement quitte la partie : on attend d'y être revenu.
        1 if !in_game => {
            let mut check = world.resource_mut::<SaveLoadCheck>();
            (check.step, check.since) = (2, 0.0);
        }
        2 if ready && since > 3.0 => {
            let Some((credits, date, view, pose)) = world.resource::<SaveLoadCheck>().expected
            else {
                return;
            };
            let got_credits = world.resource::<crate::career::Career>().credits.cents;
            let got_date = world.resource::<crate::clock::GameClock>().date;
            let got_view = *world.resource::<State<ViewLevel>>().get();
            let drift = match (pose, ship_pose(world)) {
                (Some(a), Some(b)) => a.distance(b),
                _ => 0.0,
            };
            if got_credits == credits && got_date == date && got_view == view && drift < 1.0 {
                info!(
                    "scénario save_load terminé : argent, date et vue retrouvés ({got_view:?}), vaisseau à {drift:.2} m"
                );
                world.write_message(AppExit::Success);
            } else {
                error!(
                    "scénario save_load : rechargé {got_credits} centimes le {got_date:?} en {got_view:?} (écart {drift:.1} m), attendu {credits} le {date:?} en {view:?}"
                );
                world.write_message(AppExit::error());
            }
            world.resource_mut::<SaveLoadCheck>().step = 3;
        }
        _ => {}
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
    if let Some(&step) = scenario.steps.get(scenario.index)
        && let Step::Raid(_, _, max, shot) = step
    {
        scenario.step_time += time.delta_secs();
        // Mesures pendant le combat (critère de performance de M6), hors capture d'écran
        // (elle coûte à elle seule plusieurs dizaines de millisecondes).
        let capturing = shot.is_some_and(|(_, at)| (scenario.step_time - at).abs() < 0.3);
        let ms = real.delta_secs_f64() * 1000.0;
        if !capturing {
            scenario.frames += 1;
            scenario.total_ms += ms;
            scenario.worst_ms = scenario.worst_ms.max(ms);
            if ms > 33.0 {
                scenario.over_33ms += 1;
            }
        }
        if scenario.step_time > max {
            error!(
                "scénario {} : le raid n'a pas fini en {max} s ({step:?})",
                scenario.name
            );
            exit.write(AppExit::error());
            return;
        }
        commands.queue(move |world: &mut World| raid_step(world, step));
        return;
    }
    if let Some(&step) = scenario.steps.get(scenario.index)
        && let Step::Firefight(_, _, max, shot) = step
    {
        scenario.step_time += time.delta_secs();
        // Mesures hors capture d'écran (elle coûte à elle seule des dizaines de ms).
        let capturing = shot.is_some_and(|(_, at)| (scenario.step_time - at).abs() < 0.3);
        let ms = real.delta_secs_f64() * 1000.0;
        if !capturing {
            scenario.frames += 1;
            scenario.total_ms += ms;
            scenario.worst_ms = scenario.worst_ms.max(ms);
            if ms > 33.0 {
                scenario.over_33ms += 1;
            }
        }
        if scenario.step_time > max {
            error!(
                "scénario {} : le combat n'a pas fini en {max} s",
                scenario.name
            );
            exit.write(AppExit::error());
            return;
        }
        commands.queue(move |world: &mut World| firefight_step(world, step));
        return;
    }
    if let Some(&step) = scenario.steps.get(scenario.index)
        && is_career_step(&step)
    {
        scenario.step_time += time.delta_secs();
        if scenario.step_time > TRANSITION_SECONDS {
            error!(
                "scénario {} : bloqué à l'étape {} ({step:?})",
                scenario.name, scenario.index
            );
            exit.write(AppExit::error());
            return;
        }
        commands.queue(move |world: &mut World| career_step(world, step));
        return;
    }
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
        Step::OpenMap
        | Step::Route(_)
        | Step::Depart(_)
        | Step::Mark
        | Step::Expect(..)
        | Step::Open(_)
        | Step::Choose(_)
        | Step::Hail
        | Step::Assert(..)
        | Step::Shot(_)
        | Step::Wait(_)
        | Step::JumpNear(_)
        | Step::Raid(..)
        | Step::Firefight(..) => return,
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

/// Une option du terminal correspond-elle au motif ? `say:<clé>` (réplique), `accept:<lieu>`
/// ou `accept:any`, `betray`, `close`.
fn choice_matches(choice: &crate::ui::terminal::Choice, pattern: &str) -> bool {
    use crate::ui::terminal::Action;
    match (&choice.action, pattern.split_once(':')) {
        (Action::Say { key, .. }, Some(("say", k))) => key == k,
        (Action::Accept(c), Some(("accept", to))) => to == "any" || c.to.as_str() == to,
        (Action::Betray(_), None) => pattern == "betray",
        (Action::Repair, None) => pattern == "repair",
        (action, None) if pattern == "close" => action.closes(),
        _ => false,
    }
}

/// Une étape de terminal ou de carrière, jouée avec le `World` (chaque frame jusqu'à la fin).
fn career_step(world: &mut World, step: Step) {
    use crate::ui::terminal::{OpenTerminal, Screen, Terminal as Term, TerminalChoice};
    let next = |world: &mut World| {
        let mut scenario = world.resource_mut::<Scenario>();
        scenario.index += 1;
        scenario.issued = None;
        scenario.step_time = 0.0;
        scenario.page_seen = None;
    };
    let fail = |world: &mut World, message: String| {
        let name = world.resource::<Scenario>().name.clone();
        error!("scénario {name} : {message}");
        world.write_message(AppExit::error());
    };
    let issued = world.resource::<Scenario>().issued.is_some();
    match step {
        Step::Open(which) => {
            let open = world.get_resource::<Term>().map(|t| t.page.is_some());
            match (issued, open) {
                (true, Some(true)) => next(world),
                (false, None) => {
                    let screen = match which {
                        Terminal::Dialogue(id) => Screen::Dialogue {
                            id: id.to_owned(),
                            node: String::new(),
                        },
                        Terminal::Market => Screen::Market,
                        Terminal::Sheet => Screen::Sheet,
                        Terminal::Journal => Screen::Journal,
                    };
                    world.write_message(OpenTerminal(screen));
                    world.resource_mut::<Scenario>().issued = Some(None);
                }
                _ => {}
            }
        }
        Step::Choose(pattern) => {
            let page = world.get_resource::<Term>().and_then(|t| t.page.clone());
            if issued {
                // Fait quand la page a changé (ou que le terminal s'est fermé).
                let seen = world.resource::<Scenario>().page_seen.clone();
                let closed = !world.contains_resource::<Term>();
                if closed || (page.is_some() && page != seen) {
                    next(world);
                }
                return;
            }
            let Some(page) = page else {
                return;
            };
            match page
                .choices
                .iter()
                .position(|c| c.enabled && choice_matches(c, pattern))
            {
                Some(index) => {
                    info!("scénario : choix « {} »", page.choices[index].label);
                    world.write_message(TerminalChoice(index));
                    let mut scenario = world.resource_mut::<Scenario>();
                    scenario.issued = Some(None);
                    scenario.page_seen = Some(page);
                }
                None => {
                    let labels: Vec<&str> = page.choices.iter().map(|c| c.label.as_str()).collect();
                    fail(
                        world,
                        format!("aucune option « {pattern} » parmi {labels:?}"),
                    );
                }
            }
        }
        Step::Wait(seconds) => {
            if world.resource::<Scenario>().step_time > seconds {
                next(world);
            }
        }
        Step::JumpNear(target) => {
            let pos = match target {
                "wreck" => world
                    .query_filtered::<&crate::space::SpacePosition, With<crate::career::Wreck>>()
                    .iter(world)
                    .next()
                    .map(|p| p.0),
                _ => world
                    .query::<(&crate::career::CargoPoint, &crate::space::SpacePosition)>()
                    .iter(world)
                    .find(|(c, _)| c.0 == sh_data::StructureKind::Shipyard)
                    .map(|(_, p)| p.0),
            };
            // L'épave apparaît un frame après l'acceptation : on attend.
            let Some(pos) = pos else {
                return;
            };
            let Some(root) = world
                .get_resource::<crate::sector::ActiveSector>()
                .map(|s| s.root)
            else {
                return;
            };
            if let Some(mut ship) = world.get_mut::<crate::space::SpacePosition>(root) {
                ship.0 = pos + sh_core::SpacePos::new(0.0, 300.0, 1000.0);
            }
            if let Some(mut motion) = world.get_mut::<crate::ship::ShipMotion>(root) {
                motion.velocity = Default::default();
            }
            next(world);
        }
        Step::Hail => {
            crate::transition::request_dock(world);
            next(world);
        }
        Step::Assert(what, check) => {
            let data = world.resource::<crate::data::Data>().clone();
            let ok = world
                .get_resource::<crate::career::Career>()
                .is_some_and(|c| check(&c.captain, &data));
            if ok {
                info!("scénario : vérifié — {what}");
                next(world);
            } else {
                let sheet = world
                    .get_resource::<crate::career::Career>()
                    .map(|c| format!("{:?}", c.captain))
                    .unwrap_or_default();
                fail(
                    world,
                    format!("échec de la vérification « {what} » : {sheet}"),
                );
            }
        }
        Step::Shot(name) => {
            if !issued {
                let path = format!("target/shots/{name}.png");
                world
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path));
                world.resource_mut::<Scenario>().issued = Some(None);
            } else if world.resource::<Scenario>().step_time > 1.0 {
                next(world);
            }
        }
        _ => next(world),
    }
}

/// Une étape de raid, jouée avec le `World` à chaque frame : l'autopilote pilote, tire, lance
/// ses leurres ; l'étape finit à l'issue du raid (erreur si ce n'est pas celle attendue).
fn raid_step(world: &mut World, step: Step) {
    use crate::combat::space::{AiPilot, Combatant, FireIntent, PlayerTarget, Shot, Side};
    use crate::combat::{Raid, RaidOutcome};
    use bevy::math::DVec3;
    let Step::Raid(play, end, _, shot) = step else {
        return;
    };
    let elapsed = world.resource::<Scenario>().step_time;
    let next = |world: &mut World| {
        let mut scenario = world.resource_mut::<Scenario>();
        scenario.index += 1;
        scenario.issued = None;
        scenario.step_time = 0.0;
    };
    // La capture en cours de combat.
    if let Some((name, at)) = shot
        && elapsed >= at
        && world.resource::<Scenario>().issued.is_none()
    {
        world
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("target/shots/{name}.png")));
        world.resource_mut::<Scenario>().issued = Some(None);
    }
    match world.get_resource::<Raid>().map(|r| r.outcome) {
        None => return,
        Some(Some(outcome)) => {
            let got = match outcome {
                RaidOutcome::Won => RaidEnd::Won,
                RaidOutcome::Fled => RaidEnd::Fled,
                RaidOutcome::Lost => RaidEnd::Lost,
            };
            let name = world.resource::<Scenario>().name.clone();
            let kills = world.get_resource::<Raid>().map_or(0, |r| r.kills);
            if got == end {
                let s = world.resource::<Scenario>();
                info!(
                    "scénario {name} : raid fini ({got:?}, {kills} pillard(s) détruit(s)) après {elapsed:.0} s ; \
                     pendant le combat : {} coques au plus, temps de frame moyen {:.2} ms, pire {:.2} ms, {} frame(s) > 33 ms",
                    s.max_ships,
                    s.total_ms / f64::from(s.frames.max(1)),
                    s.worst_ms,
                    s.over_33ms
                );
                next(world);
            } else {
                error!("scénario {name} : raid fini en {got:?}, attendu {end:?}");
                world.write_message(AppExit::error());
            }
            return;
        }
        Some(None) => {}
    }
    let Some(root) = world
        .get_resource::<crate::sector::ActiveSector>()
        .map(|s| s.root)
    else {
        return;
    };
    let Some((pos, rot)) = world
        .get::<crate::space::SpacePosition>(root)
        .zip(world.get::<crate::space::SpaceRotation>(root))
        .map(|(p, r)| (p.0, r.0))
    else {
        return;
    };
    let me = DVec3::new(pos.x, pos.y, pos.z);
    let to_world = rot.as_dquat();
    let mut q = world.query::<(Entity, &Combatant, &AiPilot, &crate::space::SpacePosition)>();
    let raiders: Vec<(Entity, DVec3, DVec3)> = q
        .iter(world)
        .filter(|(_, c, ai, _)| c.side == Side::Raiders && ai.delay <= 0.0 && !c.hull.destroyed())
        .map(|(e, _, ai, p)| (e, DVec3::new(p.0.x, p.0.y, p.0.z), ai.velocity))
        .collect();
    let defenders: Vec<DVec3> = q
        .iter(world)
        .filter(|(_, c, ai, _)| c.side == Side::Defenders && ai.delay <= 0.0 && !c.hull.destroyed())
        .map(|(_, _, _, p)| DVec3::new(p.0.x, p.0.y, p.0.z))
        .collect();
    let in_play = {
        let mut all = world.query::<(&Combatant, Option<&AiPilot>)>();
        all.iter(world)
            .filter(|(c, ai)| !c.hull.destroyed() && ai.is_none_or(|a| a.delay <= 0.0))
            .count()
    };
    {
        let mut s = world.resource_mut::<Scenario>();
        s.max_ships = s.max_ships.max(in_play);
    }
    let locked = world.resource::<PlayerTarget>().0;
    let incoming = {
        let mut shots = world.query::<(&Shot, &crate::space::SpacePosition)>();
        shots.iter(world).any(|(s, p)| {
            s.target == Some(root) && DVec3::new(p.0.x, p.0.y, p.0.z).distance(me) < 1500.0
        })
    };
    // Tourner vers une direction absolue : erreurs de lacet et de tangage dans le repère du
    // vaisseau, commande proportionnelle.
    let turn_to = |dir: DVec3| {
        let local = to_world.inverse() * dir.normalize_or(DVec3::NEG_Z);
        let yaw = local.x.atan2(-local.z);
        let pitch = local.y.atan2(-local.z);
        Vec2::new(
            (yaw * 3.0).clamp(-1.0, 1.0) as f32,
            (pitch * 3.0).clamp(-1.0, 1.0) as f32,
        )
    };
    let mut intent = crate::ship::PilotIntent::default();
    let mut fire = FireIntent::default();
    let nearest = raiders
        .iter()
        .min_by(|a, b| a.1.distance(me).total_cmp(&b.1.distance(me)))
        .copied();
    let forward = to_world * DVec3::NEG_Z;
    match (play, nearest) {
        (RaidPlay::Fight, Some(near)) => {
            // Un cargo ne tient pas la ligne : il frappe les pillards déjà aux prises avec la
            // Flotte de défense, et garde ses distances avec les autres.
            let engaged = |p: DVec3| defenders.iter().any(|d| d.distance(p) < 3500.0);
            let prey = raiders
                .iter()
                .filter(|r| engaged(r.1))
                .min_by(|a, b| a.1.distance(me).total_cmp(&b.1.distance(me)))
                .copied();
            let threat = me.distance(near.1) < 2800.0 && !engaged(near.1);
            if let (Some((_, tp, tv)), false) = (prey, threat) {
                let aim = crate::combat::space::lead(me, DVec3::ZERO, tp, tv, 2500.0);
                intent.turn = turn_to(aim - me);
                let dist = me.distance(tp);
                intent.thrust = if dist > 1500.0 {
                    Vec3::new(0.0, 0.0, -0.8)
                } else if dist < 700.0 {
                    Vec3::new(0.8, 0.3, 0.5)
                } else {
                    Vec3::new(0.3, 0.0, -0.2)
                };
                fire.guns = forward.angle_between(aim - me).to_degrees() < 2.5;
                fire.missile = locked.is_some() && (1200.0..5000.0).contains(&dist);
            } else {
                // Dégagement : à l'opposé des pillards, en zigzag, vers la rescousse.
                let centre = raiders.iter().map(|r| r.1).sum::<DVec3>() / raiders.len() as f64;
                let mut away = (me - centre).normalize_or(DVec3::X);
                if let Some(d) = defenders.first() {
                    away = (away + (*d - me).normalize_or(away)).normalize_or(away);
                }
                let weave = (elapsed * 0.7).sin() as f64;
                let side = away.cross(DVec3::Y).normalize_or(DVec3::X) * weave * 0.5;
                intent.turn = turn_to(away + side);
                intent.thrust = Vec3::new(0.0, 0.0, -1.0);
            }
            fire.decoy = incoming;
        }
        (RaidPlay::Flee, Some(_)) => {
            let centre = raiders.iter().map(|r| r.1).sum::<DVec3>() / raiders.len() as f64;
            let away = me - centre;
            intent.turn = turn_to(away);
            intent.cruise = forward.angle_between(away).to_degrees() < 25.0;
            intent.thrust = Vec3::NEG_Z;
            fire.decoy = incoming;
        }
        _ => {}
    }
    *world.resource_mut::<crate::ship::PilotIntent>() = intent;
    *world.resource_mut::<FireIntent>() = fire;
}

/// Une étape de combat à pied, jouée avec le `World` à chaque frame : l'autopilote vise le
/// pirate visible le plus proche et tire (ou ne fait rien) ; fin quand un camp est tombé.
fn firefight_step(world: &mut World, step: Step) {
    use crate::combat::foot::{FighterAi, FootFireIntent, PlayerArms};
    let Step::Firefight(fight, expect_win, _, shot) = step else {
        return;
    };
    let elapsed = world.resource::<Scenario>().step_time;
    if let Some((name, at)) = shot
        && elapsed >= at
        && world.resource::<Scenario>().issued.is_none()
    {
        world
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("target/shots/{name}.png")));
        world.resource_mut::<Scenario>().issued = Some(None);
    }
    let mut q = world.query::<(&crate::combat::foot::Fighter, &Transform, &FighterAi)>();
    let fighters: Vec<(Vec3, bool, bool)> = q
        .iter(world)
        .map(|(f, t, ai)| (t.translation, f.alive(), ai.crouched))
        .collect();
    if fighters.is_empty() {
        return;
    }
    let health = world.get_resource::<PlayerArms>().map_or(0.0, |a| a.health);
    let alive = fighters.iter().filter(|f| f.1).count();
    let name = world.resource::<Scenario>().name.clone();
    if alive == 0 || health <= 0.0 {
        let won = alive == 0;
        if won == expect_win {
            let s = world.resource::<Scenario>();
            info!(
                "scénario {name} : combat à pied fini ({}, santé {health:.0}, {} hostile(s) debout) après {elapsed:.0} s ; temps de frame moyen {:.2} ms, pire {:.2} ms, {} frame(s) > 33 ms",
                if won { "gagné" } else { "perdu" },
                alive,
                s.total_ms / f64::from(s.frames.max(1)),
                s.worst_ms,
                s.over_33ms
            );
            let mut scenario = world.resource_mut::<Scenario>();
            scenario.index += 1;
            scenario.issued = None;
            scenario.step_time = 0.0;
        } else {
            error!(
                "scénario {name} : combat à pied {} contre toute attente",
                if won { "gagné" } else { "perdu" }
            );
            world.write_message(AppExit::error());
        }
        return;
    }
    if !fight {
        return;
    }
    let mut pq = world.query_filtered::<(&GlobalTransform, &crate::player::PlayerController), With<crate::player::Player>>();
    let Ok((pt, controller)) = pq.single(world) else {
        return;
    };
    let eye = pt.translation() + Vec3::Y * controller.eye_height();
    let (yaw, pitch) = (controller.yaw, controller.pitch);
    // La cible : le pirate debout le plus proche dont on voit le torse.
    let mut spatial = bevy::ecs::system::SystemState::<avian3d::prelude::SpatialQuery>::new(world);
    let spatial = spatial.get(world);
    let Ok(spatial) = spatial else {
        return;
    };
    let target = fighters
        .iter()
        .filter(|f| f.1)
        .map(|f| f.0 + Vec3::Y * if f.2 { 0.8 } else { 1.3 })
        .filter(|c| {
            let d = *c - eye;
            Dir3::new(d).ok().is_none_or(|dir| {
                spatial
                    .cast_ray(
                        eye,
                        dir,
                        d.length(),
                        true,
                        &avian3d::prelude::SpatialQueryFilter::from_mask([
                            crate::sector::Layer::Static,
                            crate::sector::Layer::Door,
                        ]),
                    )
                    .is_none()
            })
        })
        .min_by(|a, b| a.distance(eye).total_cmp(&b.distance(eye)));
    let mut intent = crate::player::PlayerIntent::default();
    let mut fire = FootFireIntent::default();
    if let Some(c) = target {
        let d = (c - eye).normalize_or(Vec3::NEG_Z);
        let want_yaw = (-d.x).atan2(-d.z);
        let want_pitch = d.y.clamp(-1.0, 1.0).asin();
        let wrap = |a: f32| {
            (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
        };
        let dy = wrap(yaw - want_yaw);
        let dp = pitch - want_pitch;
        // Une main humaine : 2,5 rad/s au plus, et on ne tire qu'une fois la mire posée.
        let max = 2.5 * world.resource::<Time>().delta_secs();
        intent.look = Vec2::new(dy.clamp(-max, max), dp.clamp(-max, max));
        fire.fire = dy.abs() < 0.03 && dp.abs() < 0.03;
        // On se baisse pour recharger.
        intent.crouch = world
            .get_resource::<PlayerArms>()
            .is_some_and(|a| a.reload > 0.0 || a.mag == 0);
    }
    *world.resource_mut::<crate::player::PlayerIntent>() = intent;
    *world.resource_mut::<FootFireIntent>() = fire;
}
