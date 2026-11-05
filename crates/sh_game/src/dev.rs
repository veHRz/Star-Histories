//! Autopilote de développement, piloté par variables d'environnement.
//!
//! Permet à un agent IA (ou à un script) de lancer le jeu, de vérifier qu'il tourne sans
//! panique, d'en récupérer une capture et des mesures, sans intervention humaine :
//!
//! - `SH_START=<vue>[:<lieu>]` : démarre directement dans une vue (`galaxy`, `global:hexis`,
//!   `sector:test`), dans une nouvelle partie comme le bouton du menu (`new`), dans une
//!   sauvegarde du dossier du joueur (`load:<fichier>`) ou sur le menu (`menu`). M9 : `global:`
//!   place le joueur aux commandes, `new` debout dans la chambre (le départ) ; le vaisseau du
//!   joueur est `sector:corvette` (intérieur type), le spatioport `sector:spaceport` (le
//!   vaisseau y est amarré dans le hangar).
//! - `SH_USER_DIR=<dossier>` : dossier des sauvegardes et réglages (jamais ceux du joueur).
//! - `SH_EXIT_AFTER_FRAMES=N` : quitte proprement après N frames.
//! - `SH_SCREENSHOT=chemin.png` : enregistre une capture puis quitte.
//! - `SH_SCREENSHOT_FRAME=N` : frame de la capture (défaut : 90, le temps que la scène se charge).
//! - `SH_PRESET=low|medium|high` : préréglage graphique (lu par `graphics`).
//! - `SH_WINDOW=<L>x<H>|fullscreen` : taille de la fenêtre, ou plein écran sans bordure (les
//!   mesures de la machine de référence se font en 1080p).
//! - `SH_TUTORIAL=1` : le tutoriel guidé tourne (coupé sinon, comme les conseils) ;
//!   `SH_TUTORIAL_STEP=<n>` le fait commencer à l'étape n, de 1 (la chambre) à 10 (captures du
//!   panneau). Le scénario `SH_SCENARIO=tutorial` le joue en entier, de la chambre et du tour du
//!   vaisseau à la sauvegarde.
//! - `SH_SCREEN=inventory|skills` : ouvre l'écran du personnage sur cet onglet, 3 s après
//!   l'entrée en partie (comme `SH_TERMINAL` pour le terminal ; captures).
//! - `SH_TERMINAL=<écran>` : ouvre cet écran du terminal 3 s après l'entrée en partie (captures) :
//!   `codex`, `fiche`, `journal`, `contrats`, `marche`, `banque`, `chantier`, `guichet` (le
//!   bureau des contrats), `taverne`.
//! - `SH_TALK=<rôle>[:<variante>]` : parle au PNJ de comptoir de ce rôle (`contracts`, `bank`,
//!   `market`, `shipyard`, `bar`) 3 s après l'entrée en partie, comme `E` en le visant (captures
//!   de la conversation).
//! - `SH_SCENARIO=terminal_tour` (avec `SH_START=sector:spaceport`) : chaque PNJ de
//!   comptoir, le service qu'il ouvre, puis le journal, la fiche et le codex ; captures
//!   `target/shots/m9-borne-*.png`.
//! - `SH_KEYHELP=0|1` : force l'encart des touches replié (0) ou déplié (1).
//! - `SH_AUTOPILOT=<cible>` (M10) : à la première arrivée en vue Globale, l'autopilote est
//!   engagé vers cette cible du système (astre, station, structure ou port :
//!   `vorrh_shipyards`, `hexis_spaceport_approach`, `concordia_portal_hub`…).
//! - `SH_CONTRACT=<système>` (M10) : un contrat de transport d'Hexis vers ce système (`sys_vorrh`),
//!   accepté dès l'entrée en partie (captures du repère de destination).
//! - `SH_SCENARIO=deliver_vorrh` (avec `SH_START=global:hexis SH_RAID=0`) : contrat vers Vorrh,
//!   voyage par Portail, autopilote jusqu'aux chantiers de Vorrh-II, livraison et paiement ;
//!   captures `target/shots/m10-vorrh-*.png`.
//! - `SH_SCENARIO=pickup_persist` (avec `SH_USER_DIR` et `SH_START=sector:spaceport`) :
//!   ramasser la tablette de la chambre du vaisseau, sauvegarder, recharger : elle ne revient
//!   pas sur le bureau et reste dans l'inventaire.
//! - `SH_SCENARIO=voyage_isolated` (avec `SH_START=global:hexis SH_RAID=0`) : Hexis → Calderis
//!   (monde isolé, sans Portail) par l'hyperespace, capture en orbite, retour ; durées vérifiées.
//! - `SH_SCENARIO=key_help` (avec `SH_START=sector:spaceport SH_SPAWN=hall`) : F1 tenue
//!   comme par un joueur bascule l'encart des touches, une fois par appui.
//! - `SH_SCENARIO=character` (avec `SH_START=sector:corvette SH_SPAWN=soute`) : I, O, Échap
//!   dans l'écran du personnage (sans pause derrière), puis un casier : Entrée, E.
//! - `SH_SCENARIO=fridge` (M10, avec `SH_START=sector:corvette SH_GIVE=spare_parts:5
//!   SH_CONTAINER=corvette_carre_frigo`) : le frigo compte ses rations, refuse les pièces,
//!   rend et reprend une ration ; capture `target/shots/m10-frigo.png`.
//! - `SH_SCENARIO=locker_name` (M10, avec `SH_USER_DIR` et `SH_START=sector:spaceport`) : nommer
//!   un casier de la chambre, le nom sur sa plaque, sauvegarder, recharger : il est resté ;
//!   captures `m10-saisie-nom.png`, `m10-plaque.png`, `m10-plaque-rechargee.png`.
//! - `SH_SCENARIO=airlock_space` (M10, avec `SH_START=global:hexis SH_RAID=0`) : en vol, au
//!   sas, la porte extérieure annonce le vide et ne bouge pas ; capture `m10-sas-vide.png`.
//! - Avec `SH_START=new`, la partie commence debout dans la chambre (M9) : `map_key` prend
//!   d'abord les commandes ; `save_load` sauvegarde debout et recharge aux commandes (une
//!   sauvegarde en vol reprend au poste de pilotage) ; `menu_return` et `pause_escape` n'ont
//!   pas besoin du siège.
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
            let dialogue = |id: &str| Screen::Dialogue {
                id: id.to_owned(),
                node: String::new(),
            };
            let screen = match value.trim() {
                "fiche" => Screen::Sheet,
                "journal" => Screen::Journal,
                "contrats" => Screen::Contracts,
                "marche" => Screen::Market,
                "banque" => Screen::Bank,
                "chantier" => Screen::Repair,
                "guichet" => dialogue("hexis_contract_office"),
                "taverne" => dialogue("hexis_tavern"),
                _ => Screen::Codex,
            };
            app.insert_resource(TerminalShot(screen)).add_systems(
                Update,
                terminal_shot.run_if(resource_exists::<crate::career::Career>),
            );
        }
        // `SH_TALK=<rôle>[:<variante>]` : la conversation d'un PNJ de comptoir (captures).
        if let Ok(value) = std::env::var("SH_TALK") {
            let (role, variant) = value.trim().split_once(':').unwrap_or((value.trim(), "a"));
            app.insert_resource(TalkShot(crate::items::TalkTo {
                role: role.to_owned(),
                variant: variant.to_owned(),
            }))
            .add_systems(
                Update,
                talk_shot.run_if(resource_exists::<crate::career::Career>),
            );
        }
        // `SH_SCREEN=inventory|skills` : l'écran du personnage sur cet onglet (captures).
        if let Ok(value) = std::env::var("SH_SCREEN") {
            match crate::items::CharacterTab::from_key(value.trim()) {
                Some(tab) => {
                    app.insert_resource(CharacterShot(tab)).add_systems(
                        Update,
                        character_shot.run_if(resource_exists::<crate::career::Career>),
                    );
                }
                None => warn!("SH_SCREEN inconnu : « {value} » (inventory, skills)"),
            }
        }
        // `SH_AUTOPILOT=<cible>` : l'autopilote engagé vers cette cible du système à la première
        // arrivée en vue Globale, comme si la carte l'avait demandé (M10).
        if let Ok(value) = std::env::var("SH_AUTOPILOT") {
            let target = value.trim().to_owned();
            app.add_systems(
                OnEnter(ViewLevel::Global),
                move |mut request: ResMut<crate::ship::AutopilotRequest>, mut done: Local<bool>| {
                    if !*done {
                        *done = true;
                        info!("SH_AUTOPILOT : autopilote vers {target}");
                        request.0 = Some(target.clone());
                    }
                },
            );
        }
        // `SH_CONTRACT=<système>` : un contrat de transport vers ce système, accepté dès que la
        // partie est prête (captures du repère de destination, M10).
        if let Ok(value) = std::env::var("SH_CONTRACT") {
            let to = value.trim().to_owned();
            app.add_systems(
                Update,
                (move |world: &mut World, mut done: Local<bool>| {
                    if *done || !world.contains_resource::<crate::living::LivingWorld>() {
                        return;
                    }
                    match give_contract_to(world, &to) {
                        Ok(true) => *done = true,
                        Ok(false) => {}
                        Err(message) => {
                            *done = true;
                            error!("SH_CONTRACT : {message}");
                        }
                    }
                })
                .run_if(resource_exists::<crate::career::Career>),
            );
        }
        // `SH_KEYHELP=0|1` : l'encart des touches replié ou déplié, quoi qu'en dise le défaut.
        if let Ok(value) = std::env::var("SH_KEYHELP") {
            app.insert_resource(crate::ui::keys_panel::KeyHelpVisible(value.trim() != "0"));
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

/// `SH_TALK=<rôle>[:<variante>]` : parle au PNJ de comptoir de ce rôle (captures).
#[derive(Resource)]
struct TalkShot(crate::items::TalkTo);

fn talk_shot(
    time: Res<Time<Real>>,
    shot: Res<TalkShot>,
    mut talk: MessageWriter<crate::items::TalkTo>,
    mut since: Local<f32>,
) {
    let before = *since;
    *since += time.delta_secs();
    if before <= 3.0 && *since > 3.0 {
        talk.write(shot.0.clone());
    }
}

/// `SH_TERMINAL=<écran>` : ouvre cet écran du terminal (captures).
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

/// `SH_SCREEN=inventory|skills` : ouvre l'écran du personnage sur cet onglet (captures).
#[derive(Resource)]
struct CharacterShot(crate::items::CharacterTab);

fn character_shot(
    time: Res<Time<Real>>,
    shot: Res<CharacterShot>,
    mut open: MessageWriter<crate::items::OpenCharacterScreen>,
    mut since: Local<f32>,
) {
    let before = *since;
    *since += time.delta_secs();
    if before <= 3.0 && *since > 3.0 {
        open.write(crate::items::OpenCharacterScreen(shot.0));
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

/// Parcours scripté du vaisseau du joueur (intérieur type `corvette`, M9), du sas au cockpit, en
/// passant par toutes les cellules et toutes les portes, jusqu'au siège du pilote. Coordonnées
/// Blender (x = tribord, y = avant) : le jeu reçoit (x, −y). `Some(portail)` : porte à ouvrir
/// avant de rejoindre le point. Portes du couloir avant : quartiers (−1,4 ; 4,6), carré
/// (1,4 ; 5,6), chambre (−1,4 ; 11,0) ; du couloir arrière : sas (−1,4 ; −3), soute (1,4 ; −7),
/// machines (0 ; −14,2) ; cockpit (0 ; 14,2), sans estrade.
const WALK_SHIP: &[Step] = &[
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(0.0, -3.0, None),
    w(0.0, -12.6, None),
    w(0.0, -15.6, Some("couloir_arriere__machines")),
    w(0.0, -17.0, None),
    w(0.0, -12.8, Some("couloir_arriere__machines")),
    w(0.0, -7.0, None),
    w(2.6, -7.0, Some("couloir_arriere__soute")),
    w(4.4, -7.8, None),
    w(2.6, -7.0, None),
    w(0.0, -7.0, Some("couloir_arriere__soute")),
    w(0.0, 4.6, None),
    w(-2.6, 4.6, Some("couloir_avant__quartiers")),
    w(-3.8, 4.6, None),
    w(-2.6, 4.6, None),
    w(0.0, 4.6, Some("couloir_avant__quartiers")),
    w(0.0, 5.6, None),
    w(2.6, 5.6, Some("couloir_avant__carre")),
    w(3.0, 4.0, None),
    w(2.6, 5.6, None),
    w(0.0, 5.6, Some("couloir_avant__carre")),
    w(0.0, 11.0, None),
    w(-2.6, 11.0, Some("couloir_avant__chambre")),
    w(-4.0, 11.0, None),
    w(-2.6, 11.0, None),
    w(0.0, 11.0, Some("couloir_avant__chambre")),
    w(0.0, 12.8, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 16.4, None),
    w(0.0, 17.4, None),
    w(0.0, 18.4, None),
    Step::SitAtHelm,
];

/// Fin du scénario `fly_ship` : du poste de pilotage au couloir avant, à pied, en plein vol.
/// Le joueur se lève devant les sièges (y ≈ 18,6 ; la planche de bord commence à 19,7).
const FLY_SHIP_WALK: &[Step] = &[
    w(0.0, 18.6, None),
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
    /// Parler au PNJ de comptoir de ce rôle (comme `E` en le visant) ; fini quand le terminal
    /// montre sa conversation.
    Talk(&'static str),
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
    /// Tourner (lacet, aux commandes) pendant tant de secondes.
    Turn(f32),
    /// Croisière vers l'avant pendant tant de secondes.
    Cruise(f32),
    /// Placer le vaisseau, à l'arrêt, à tant de mètres au-dessus de la balise d'approche du
    /// port, à la verticale de son astre. Raccourci : le trajet depuis l'arrivée pourrait
    /// traverser la planète ; l'approche elle-même se vole ([`Step::FlyToBeacon`]).
    JumpAboveBeacon(f64),
    /// Voler jusqu'à portée de la balise d'approche : virer vers elle, pousser, croisière
    /// tant qu'on a la place de ralentir.
    FlyToBeacon,
    /// Engager l'autopilote du jeu vers cette cible du système (M10, comme la carte) et
    /// attendre qu'il s'arrête devant elle, à sa distance d'arrêt.
    Autopilot(&'static str),
    /// Comme [`Step::Walk`], mais en coordonnées du quartier (repère du jeu : x, z), à quai.
    WalkDistrict(f32, f32, Option<&'static str>),
    /// Marcher jusqu'à la console de cette étiquette, la viser et l'utiliser (comme `E`) ;
    /// fini quand le terminal s'ouvre. Plus jouée depuis M9 (le tutoriel parle au guichetier,
    /// un PNJ de comptoir) ; gardée pour un scénario qui viserait une console.
    #[allow(dead_code)]
    UseConsole(&'static str),
    /// Vérifier que le tutoriel est arrivé à l'étape n (11 : terminé).
    Tutorial(usize),
    /// Sauvegarde rapide (comme `F5`).
    QuickSave,
    /// Une action propre à un scénario, jouée avec le `World` à chaque frame jusqu'à ce qu'elle
    /// rende `Ok(true)` ; `Err` : échec du scénario. Le texte la nomme dans le journal.
    Script(&'static str, fn(&mut World) -> Result<bool, String>),
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
    Repair,
    Sheet,
    Journal,
    Codex,
}

/// Les étapes de terminal et de carrière : elles se jouent aux commandes, sans marcher.
fn is_career_step(step: &Step) -> bool {
    matches!(
        step,
        Step::Open(_)
            | Step::Talk(_)
            | Step::Choose(_)
            | Step::Hail
            | Step::Assert(..)
            | Step::Shot(_)
            | Step::Wait(_)
            | Step::JumpNear(_)
            | Step::JumpAboveBeacon(_)
            | Step::FlyToBeacon
            | Step::Autopilot(_)
            | Step::Tutorial(_)
            | Step::QuickSave
            | Step::Script(..)
    )
}

const fn w(x: f32, y: f32, door: Option<&'static str>) -> Step {
    Step::Walk(x, y, door)
}

/// `port_loop` (avec `SH_START=sector:spaceport`) : du quai du hangar au cockpit, départ, vol,
/// amarrage, et retour au quai (critère de M2). Le vaisseau repose dans son berceau (M9) : on
/// longe la passerelle jusqu'à la porte extérieure du sas (x = −7,7 dans le repère du vaisseau,
/// portail `sas__hangar` une fois amarré), le sas de 6 m, le couloir, le cockpit ; au retour, le
/// quai est au-delà de la coque (x < −7,8), au pied de la passerelle.
const PORT_LOOP: &[Step] = &[
    w(-6.4, -3.0, Some("sas__hangar")),
    w(-5.4, -3.0, None),
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(0.0, -3.0, None),
    w(0.0, 12.8, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 16.4, None),
    w(0.0, 17.4, None),
    w(0.0, 18.4, None),
    Step::SitAtHelm,
    Step::DockOrDepart,
    Step::Thrust(4.0),
    Step::DockOrDepart,
    Step::StandUp,
    w(0.0, 18.6, None),
    w(0.0, 18.0, None),
    w(0.0, 16.4, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 12.8, None),
    w(0.0, -3.0, None),
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(-5.4, -3.0, None),
    w(-6.4, -3.0, Some("sas__hangar")),
    w(-8.3, -3.0, None),
    w(-11.5, -3.0, None),
    // Le long du quai vers la proue (en dedans des bornes de carburant) : la coque posée dans son
    // berceau, éclairée comme en arrivée directe (l'ambiante du secteur ne dépend pas du chemin).
    w(-13.5, -3.0, None),
    w(-13.5, 4.0, None),
    Step::Shot("m9-port-hangar"),
];

/// `voyage` : Hexis → Cygnara par Portail, puis retour par l'hyperespace (critère de M3) ; enfin
/// le ravitaillement en carburant lourd au chantier d'Hexis (M9 : le saut a entamé la réserve).
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
    Step::Open(Terminal::Repair),
    Step::Shot("m9-chantier-carburant"),
    Step::Choose("refuel"),
    Step::Shot("m9-chantier-ravitaille"),
    Step::Choose("close"),
];

/// `voyage_isolated` (avec `SH_START=global:hexis SH_RAID=0`) : un monde isolé (M9, Q18),
/// Calderis, planète errante sans Portail ; aller par l'hyperespace jusqu'à son orbite, puis
/// retour à Hexis. ≈ 19,4 jours par sens : les deux sens tiennent dans le réservoir (60 jours).
const VOYAGE_ISOLATED: &[Step] = &[
    Step::Mark,
    Step::OpenMap,
    Step::Route("calderis"),
    Step::Depart(sh_sim::travel::TravelMode::Hyperspace),
    Step::Expect("calderis", 15.0 * 24.0, 25.0 * 24.0),
    // Le temps que l'astre et ses textures (calculées en tâche de fond) soient prêts.
    Step::Wait(2.0),
    Step::Shot("m9-isole-orbite"),
    Step::Mark,
    Step::OpenMap,
    Step::Route("sys_hexis"),
    Step::Depart(sh_sim::travel::TravelMode::Hyperspace),
    Step::Expect("sys_hexis", 15.0 * 24.0, 25.0 * 24.0),
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
    Step::Open(Terminal::Dialogue("hexis_bank")),
    Step::Choose("say:dlg.bank.pay"),
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

/// `deliver_vorrh` (avec `SH_START=global:hexis SH_RAID=0`) : un contrat vers Vorrh (M10, le
/// retour n° 21 de l'auteur), Hexis → Karza → Vorrhal par Portail, arrivée près du hub de
/// Vorrhal avec le repère de destination, autopilote jusqu'aux chantiers de Vorrh-II, livraison
/// (touche d'amarrage) et paiement ; captures `target/shots/m10-vorrh-*.png`.
const DELIVER_VORRH: &[Step] = &[
    Step::Wait(3.0),
    Step::Script("contrat vers Vorrh", give_vorrh_contract),
    Step::Assert("contrat vers Vorrh pris", |c, _| {
        c.contracts
            .iter()
            .any(|k| k.state == sh_sim::ContractState::Active && k.to.as_str() == "sys_vorrh")
    }),
    Step::OpenMap,
    Step::Route("sys_vorrh"),
    Step::Depart(sh_sim::travel::TravelMode::Portal),
    // Le temps que le hub et le trafic soient prêts.
    Step::Wait(3.0),
    Step::Shot("m10-vorrh-arrivee"),
    Step::Autopilot("vorrh_shipyards"),
    Step::Wait(2.0),
    Step::Shot("m10-vorrh-chantiers"),
    Step::Hail,
    Step::Assert("livré à Vorrh et payé", |c, _| {
        c.contracts
            .iter()
            .any(|k| k.state == sh_sim::ContractState::Done && k.to.as_str() == "sys_vorrh")
    }),
    Step::Wait(1.0),
    Step::Shot("m10-vorrh-livre"),
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

/// `firefight` (avec `SH_START=sector:corvette SH_SPAWN=soute SH_BOARDING=3`) : un abordage
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

/// `tutorial` (avec `SH_START=new SH_TUTORIAL=1 SH_RAID=0`, et `SH_USER_DIR`) : tout le
/// tutoriel guidé, par les mêmes actions qu'un joueur (critères de M8 et de M9). Chaque étape
/// franchie est vérifiée ([`Step::Tutorial`]) ; captures `target/shots/m9-tuto-chambre.png`,
/// `m9-tuto-tour.png` et `m8-tuto-*.png` (noms de M8 gardés, pour comparer). La partie commence
/// debout dans la chambre (apparition `chambre`, Blender (−5,3 ; 11,6)) : un casier, la
/// tablette, l'inventaire ([`tutorial_script`] : viser et la touche d'interaction, vraies
/// touches I et Échap), puis le tour du vaisseau de porte en porte (portes du couloir avant :
/// chambre (−1,4 ; 11,0), quartiers (−1,4 ; 4,6), carré (1,4 ; 5,6) ; du couloir arrière : sas
/// (−1,4 ; −3), soute (1,4 ; −7), machines (0 ; −14,2) ; cockpit (0 ; 14,2)), jusqu'au siège du
/// pilote. À quai, le chemin de `port_loop` jusqu'au hangar, puis le couloir, le hall et le
/// bureau, où l'on vise le guichetier.
const TUTORIAL: &[Step] = &[
    Step::Wait(2.0),
    Step::Tutorial(1),
    Step::Shot("m9-tuto-chambre"),
    // 1. La chambre : un casier, la tablette du bureau, l'inventaire.
    Step::Script("ouvrir un casier", tutorial_script::open_locker),
    Step::Wait(0.5),
    Step::Script("refermer le casier (Échap)", tutorial_script::close_screen),
    Step::Script("ramasser la tablette", tutorial_script::take_pickup),
    Step::Script("ouvrir l'inventaire (I)", tutorial_script::open_inventory),
    Step::Wait(0.5),
    Step::Script(
        "refermer l'inventaire (Échap)",
        tutorial_script::close_screen,
    ),
    Step::Tutorial(2),
    // 2. Le tour : quartiers et carré à l'avant, sas, soute et machines à l'arrière, cockpit.
    w(-4.0, 11.0, None),
    w(-2.6, 11.0, None),
    w(0.0, 11.0, Some("couloir_avant__chambre")),
    w(0.0, 4.6, None),
    w(-2.6, 4.6, Some("couloir_avant__quartiers")),
    w(-3.8, 4.6, None),
    w(-2.6, 4.6, None),
    w(0.0, 4.6, Some("couloir_avant__quartiers")),
    w(0.0, 5.6, None),
    w(2.6, 5.6, Some("couloir_avant__carre")),
    w(3.0, 4.0, None),
    w(2.6, 5.6, None),
    w(0.0, 5.6, Some("couloir_avant__carre")),
    // Face à l'arrière, le repère sur la porte du sas.
    w(0.0, 3.0, None),
    Step::Shot("m9-tuto-tour"),
    w(0.0, -3.0, None),
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(-3.8, -3.0, None),
    w(-2.4, -3.0, None),
    w(0.0, -3.0, Some("couloir_arriere__sas")),
    w(0.0, -7.0, None),
    w(2.6, -7.0, Some("couloir_arriere__soute")),
    w(4.4, -7.8, None),
    w(2.6, -7.0, None),
    w(0.0, -7.0, Some("couloir_arriere__soute")),
    w(0.0, -12.6, None),
    w(0.0, -15.6, Some("couloir_arriere__machines")),
    w(0.0, -17.0, None),
    w(0.0, -12.8, Some("couloir_arriere__machines")),
    w(0.0, 12.8, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 16.4, None),
    w(0.0, 17.4, None),
    w(0.0, 18.4, None),
    Step::Tutorial(3),
    // 3. S'asseoir, regarder, pousser, la croisière.
    Step::SitAtHelm,
    Step::Wait(1.0),
    Step::Shot("m8-tuto-1-commandes"),
    Step::Turn(3.0),
    Step::Thrust(4.0),
    Step::Cruise(2.0),
    Step::Tutorial(4),
    // 4. La balise du spatioport, l'amarrage.
    Step::JumpAboveBeacon(25_000.0),
    Step::Wait(1.0),
    Step::Shot("m8-tuto-2-balise"),
    Step::FlyToBeacon,
    Step::DockOrDepart,
    Step::Tutorial(5),
    // 5. Se lever, sortir par le sas et la passerelle (le chemin de `port_loop`), le hangar, le
    // couloir, le hall (par l'allée centrale), le bureau.
    Step::StandUp,
    Step::Wait(1.0),
    Step::Shot("m8-tuto-3-sas"),
    w(0.0, 18.6, None),
    w(0.0, 18.0, None),
    w(0.0, 16.4, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 12.8, None),
    w(0.0, -3.0, None),
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(-5.4, -3.0, None),
    w(-6.4, -3.0, Some("sas__hangar")),
    w(-8.3, -3.0, None),
    w(-11.5, -3.0, None),
    Step::WalkDistrict(-3.0, 0.0, None),
    Step::WalkDistrict(-6.0, 0.0, Some("couloir__hangar")),
    // Le hall refait (M10) : l'allée centrale (z = 0) est dégagée jusqu'au panneau du carrefour
    // (x = −20), entre les banquettes, puis tout droit vers la porte du bureau.
    Step::WalkDistrict(-15.0, 0.0, None),
    Step::WalkDistrict(-20.0, 0.0, None),
    Step::WalkDistrict(-20.0, -8.6, Some("hall__bureau")),
    Step::Shot("m8-tuto-3-bureau"),
    Step::Tutorial(6),
    // 6. Le guichetier, un contrat de transport.
    Step::Script("parler au guichetier", tutorial_script::talk_to_clerk),
    Step::Choose("say:dlg.office.ask_work"),
    Step::Choose("say:dlg.office.show_contracts"),
    Step::Shot("m8-tuto-4-contrats"),
    Step::Choose("accept:sys_cygnara"),
    Step::Tutorial(7),
    Step::Choose("close"),
    Step::Wait(0.5),
    // 7. Le journal, l'écran du personnage (vraie touche I), le codex.
    Step::Open(Terminal::Journal),
    Step::Choose("close"),
    Step::Wait(0.5),
    Step::Script("ouvrir l'inventaire (I)", tutorial_script::open_inventory),
    Step::Wait(0.5),
    Step::Script(
        "refermer l'inventaire (Échap)",
        tutorial_script::close_screen,
    ),
    Step::Open(Terminal::Codex),
    Step::Choose("close"),
    Step::Wait(0.5),
    Step::Tutorial(8),
    Step::Shot("m8-tuto-6-depart"),
    // 8. Retour au cockpit, départ, la carte, la destination du contrat.
    Step::WalkDistrict(-20.0, -8.6, Some("hall__bureau")),
    Step::WalkDistrict(-20.0, 0.0, None),
    Step::WalkDistrict(-15.0, 0.0, None),
    Step::WalkDistrict(-6.0, 0.0, None),
    Step::WalkDistrict(-3.0, 0.0, Some("couloir__hangar")),
    Step::WalkDistrict(5.0, 0.0, None),
    w(-6.4, -3.0, Some("sas__hangar")),
    w(-5.4, -3.0, None),
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(0.0, -3.0, None),
    w(0.0, 12.8, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 16.4, None),
    w(0.0, 17.4, None),
    w(0.0, 18.4, None),
    Step::SitAtHelm,
    Step::DockOrDepart,
    Step::OpenMap,
    Step::Route("sys_cygnara"),
    Step::Wait(1.0),
    Step::Shot("m8-tuto-6-carte"),
    Step::Tutorial(9),
    // 9. Par Portail, livraison en orbite près du hub, paiement.
    Step::Depart(sh_sim::travel::TravelMode::Portal),
    Step::Wait(1.0),
    Step::Shot("m8-tuto-7-hub"),
    Step::Hail,
    Step::Tutorial(10),
    Step::Shot("m8-tuto-8-sauvegarde"),
    // 10. Sauvegarder : fin.
    Step::QuickSave,
    Step::Tutorial(11),
    Step::Wait(1.0),
    Step::Shot("m8-tuto-fin"),
];

/// Les actions propres au scénario `tutorial` ([`Step::Script`]) : ce qu'un joueur fait dans la
/// chambre et au bureau, que les étapes ordinaires ne savent pas faire. Viser se fait d'une
/// place d'où le rayon de visée touche l'objet ([`aim_spot`], raccourci : le joueur y est posé,
/// dans la salle où il se trouve), puis la touche d'interaction comme `E` ; l'inventaire et la
/// fermeture des écrans passent par de vrais messages clavier (I, Échap), tenus ≈ 150 ms.
mod tutorial_script {
    use bevy::prelude::*;

    use super::{
        KEY_HOLD_SECONDS, KEY_SETTLE_SECONDS, Scenario, aim_spot, cell_local_ready, logical_key,
        place_player, send_key,
    };
    use crate::items::CharacterTab;
    use crate::sector::gltf::Interactable;
    use crate::ui::character::{CharacterScreen, ScreenView};

    /// Délai (s) au-delà duquel une action sans effet fait échouer le scénario.
    const SCRIPT_SECONDS: f32 = 12.0;

    /// L'état de l'action en cours : l'étape du scénario où elle a commencé, sa cible, la touche
    /// tenue (et depuis quand), le temps écoulé.
    #[derive(Resource, Default)]
    struct ScriptState {
        index: Option<usize>,
        since: f32,
        target: Option<Entity>,
        pressed: Option<f32>,
        released: bool,
    }

    /// Le temps écoulé depuis le début de l'action ; une nouvelle étape repart de zéro.
    fn begin(world: &mut World) -> f32 {
        let dt = world.resource::<Time<Real>>().delta_secs();
        let index = world.resource::<Scenario>().index;
        let mut state = world.get_resource_or_insert_with(ScriptState::default);
        if state.index != Some(index) {
            *state = ScriptState {
                index: Some(index),
                ..default()
            };
        } else {
            state.since += dt;
        }
        state.since
    }

    /// Appuie sur la touche, la tient ≈ 150 ms, la relâche ; vrai une fois relâchée depuis
    /// assez longtemps pour que l'écran ait réagi.
    fn tap(world: &mut World, code: KeyCode, now: f32) -> bool {
        let state = world.resource::<ScriptState>();
        match (state.pressed, state.released) {
            (None, _) => {
                send_key(world, code, logical_key(code), true);
                world.resource_mut::<ScriptState>().pressed = Some(now);
                false
            }
            (Some(at), false) if now - at >= KEY_HOLD_SECONDS => {
                send_key(world, code, logical_key(code), false);
                world.resource_mut::<ScriptState>().released = true;
                false
            }
            (Some(at), released) => released && now - at >= KEY_HOLD_SECONDS + KEY_SETTLE_SECONDS,
        }
    }

    fn character_view(world: &World) -> Option<ScreenView> {
        world
            .get_resource::<CharacterScreen>()
            .map(|s| s.view.clone())
    }

    /// Vise l'interactif voulu le plus proche du joueur, appuie sur la touche d'interaction (une
    /// fois par seconde tant que rien ne se passe) ; fini quand `done` le constate.
    fn interact_with(
        world: &mut World,
        wanted: fn(&Interactable) -> bool,
        done: fn(&mut World, Entity) -> bool,
    ) -> Result<bool, String> {
        let now = begin(world);
        let Some(entity) = world.resource::<ScriptState>().target else {
            // La salle du joueur prête (ses collisions servent à viser), le joueur au sol.
            let cell = world
                .get_resource::<crate::sector::streaming::CellStates>()
                .and_then(|c| c.current.clone());
            let player = world
                .query_filtered::<&Transform, With<crate::player::Player>>()
                .single(world)
                .ok()
                .map(|t| t.translation);
            let (Some(cell), Some(player)) = (cell, player) else {
                return Ok(false);
            };
            if !cell_local_ready(world, &cell) {
                return Ok(false);
            }
            let found = world
                .query::<(Entity, &Interactable, &GlobalTransform)>()
                .iter(world)
                .filter(|(_, i, _)| wanted(i))
                .map(|(e, _, t)| (e, t.translation()))
                .min_by(|a, b| {
                    a.1.distance_squared(player)
                        .total_cmp(&b.1.distance_squared(player))
                });
            let Some((entity, at)) = found else {
                return if now > SCRIPT_SECONDS {
                    Err(format!("rien à viser dans la cellule {cell}"))
                } else {
                    Ok(false)
                };
            };
            // D'où viser : du côté du joueur d'abord, puis tout autour.
            let side = Vec2::new(player.x - at.x, player.z - at.z).normalize_or(Vec2::Y);
            let Some(stand) = aim_spot(world, entity, at, player.y, side) else {
                return Err(format!("aucun point d'où viser {entity:?} ({at:?})"));
            };
            place_player(world, stand, Some(at));
            world.resource_mut::<ScriptState>().target = Some(entity);
            return Ok(false);
        };
        if done(world, entity) {
            return Ok(true);
        }
        let aimed = world
            .resource::<crate::interaction::Focus>()
            .0
            .as_ref()
            .is_some_and(|t| t.entity == entity);
        let pressed = world.resource::<ScriptState>().pressed;
        if aimed && pressed.is_none_or(|at| now - at > 1.0) {
            world.write_message(crate::interaction::InteractRequest);
            world.resource_mut::<ScriptState>().pressed = Some(now);
        }
        if now > SCRIPT_SECONDS {
            return Err(format!(
                "sans effet (visée {})",
                if aimed { "posée" } else { "manquée" }
            ));
        }
        Ok(false)
    }

    /// Un casier de la chambre : l'écran de rangement s'ouvre.
    pub(super) fn open_locker(world: &mut World) -> Result<bool, String> {
        interact_with(
            world,
            |i| matches!(i, Interactable::Locker { .. }),
            |world, _| {
                matches!(character_view(world), Some(ScreenView::Container(_)))
                    && crate::ui::character::is_shown(world)
            },
        )
    }

    /// La tablette posée sur le bureau : prise, son point de ramassage disparaît.
    pub(super) fn take_pickup(world: &mut World) -> Result<bool, String> {
        interact_with(
            world,
            |i| matches!(i, Interactable::Pickup { .. }),
            |world, entity| world.get::<Interactable>(entity).is_none(),
        )
    }

    /// Le guichetier du bureau des contrats (PNJ de comptoir) : sa conversation s'ouvre.
    pub(super) fn talk_to_clerk(world: &mut World) -> Result<bool, String> {
        interact_with(
            world,
            |i| matches!(i, Interactable::Npc { role, .. } if role == "contracts"),
            |world, _| {
                world
                    .get_resource::<crate::ui::terminal::Terminal>()
                    .is_some_and(|t| t.page.is_some())
            },
        )
    }

    /// La touche de l'inventaire (I) : l'écran du personnage, sur l'inventaire.
    pub(super) fn open_inventory(world: &mut World) -> Result<bool, String> {
        let now = begin(world);
        if !tap(world, KeyCode::KeyI, now) {
            return Ok(false);
        }
        let view = character_view(world);
        if view == Some(ScreenView::Tab(CharacterTab::Inventory))
            && crate::ui::character::is_shown(world)
        {
            return Ok(true);
        }
        if now > SCRIPT_SECONDS / 2.0 {
            return Err(format!("I n'ouvre pas l'inventaire (écran {view:?})"));
        }
        Ok(false)
    }

    /// Échap : l'écran du personnage (inventaire ou casier) se ferme, sans pause derrière, et
    /// le joueur reprend la marche.
    pub(super) fn close_screen(world: &mut World) -> Result<bool, String> {
        let now = begin(world);
        if !tap(world, KeyCode::Escape, now) {
            return Ok(false);
        }
        let paused = world.contains_resource::<crate::ui::pause::Paused>();
        if paused {
            return Err("Échap a ouvert la pause".into());
        }
        let open = character_view(world).is_some() || crate::ui::character::is_shown(world);
        let on_foot = world
            .get_resource::<State<crate::state::ControlMode>>()
            .is_some_and(|m| *m.get() == crate::state::ControlMode::OnFoot);
        if !open && on_foot {
            return Ok(true);
        }
        if now > SCRIPT_SECONDS / 2.0 {
            return Err(format!(
                "Échap ne referme pas l'écran (ouvert {open}, à pied {on_foot})"
            ));
        }
        Ok(false)
    }
}

/// `terminal_tour` (avec `SH_START=sector:spaceport`) : chaque PNJ de comptoir (M9), le
/// service que sa conversation ouvre, le retour à la conversation, puis les onglets de la
/// tablette ; captures `target/shots/m9-borne-*.png` des écrans refaits.
const TERMINAL_TOUR: &[Step] = &[
    Step::Wait(4.0),
    Step::Talk("contracts"),
    Step::Shot("m9-borne-pnj-contrats"),
    Step::Choose("say:dlg.office.ask_work"),
    Step::Choose("say:dlg.office.show_contracts"),
    Step::Shot("m9-borne-contrats"),
    Step::Choose("back"),
    Step::Shot("m9-borne-retour"),
    Step::Choose("close"),
    Step::Talk("bank"),
    Step::Shot("m9-borne-pnj-banque"),
    Step::Choose("say:dlg.bank.show_account"),
    Step::Shot("m9-borne-banque"),
    Step::Choose("close"),
    Step::Talk("market"),
    Step::Shot("m9-borne-pnj-marche"),
    Step::Choose("say:dlg.market.show_prices"),
    Step::Shot("m9-borne-marche"),
    Step::Choose("close"),
    Step::Talk("shipyard"),
    Step::Shot("m9-borne-pnj-chantier"),
    Step::Choose("say:dlg.shipyard.ask_repair"),
    Step::Shot("m9-borne-chantier"),
    Step::Choose("close"),
    Step::Talk("bar"),
    Step::Shot("m9-borne-pnj-taverne"),
    Step::Choose("say:dlg.tavern.ask_news"),
    Step::Shot("m9-borne-taverne-nouvelles"),
    Step::Choose("close"),
    Step::Open(Terminal::Journal),
    Step::Shot("m9-borne-journal"),
    Step::Choose("close"),
    Step::Open(Terminal::Sheet),
    Step::Shot("m9-borne-fiche"),
    Step::Choose("close"),
    Step::Open(Terminal::Codex),
    Step::Shot("m9-borne-codex"),
    Step::Choose("close"),
];

/// `fridge` (avec `SH_START=sector:corvette SH_GIVE=spare_parts:5
/// SH_CONTAINER=corvette_carre_frigo`) : le frigo de la cuisine (M10) affiche son stock de
/// rations, refuse des pièces détachées (rien ne bouge), rend une ration et la reprend ; capture
/// `target/shots/m10-frigo.png`. Les transferts passent par les fonctions de l'écran de
/// rangement (`ui::character::logic::{store, retrieve}`), sur les vraies ressources.
const FRIDGE: &[Step] = &[
    Step::Script("le frigo s'ouvre et compte 12 rations", |w| {
        m10_script::fridge_shows(w, 12)
    }),
    Step::Wait(0.5),
    Step::Shot("m10-frigo"),
    Step::Script(
        "ranger des pièces au frigo : refusé",
        m10_script::fridge_refuses_parts,
    ),
    Step::Script("prendre une ration", m10_script::fridge_take_ration),
    Step::Script("le frigo compte 11 rations", |w| {
        m10_script::fridge_shows(w, 11)
    }),
    Step::Script("remettre la ration", m10_script::fridge_put_back_ration),
    Step::Script("le frigo compte de nouveau 12 rations", |w| {
        m10_script::fridge_shows(w, 12)
    }),
];

/// `locker_name` (avec `SH_USER_DIR=… SH_START=sector:spaceport`) : à quai (une sauvegarde se
/// fait à quai ou en vol, pas dans l'intérieur type `sector:corvette`), nommer le premier casier
/// de la chambre comme le ferait la plaque (`items::NameContainer`), taper « Mes affaires »,
/// valider ; la plaque l'affiche ; sauvegarder, recharger : le nom est resté, sur la plaque
/// aussi. Captures `target/shots/m10-saisie-nom.png`, `m10-plaque.png`, `m10-plaque-rechargee.png`.
const LOCKER_NAME: &[Step] = &[
    Step::Wait(3.0),
    Step::Script("aller dans la chambre", m10_script::go_to_chambre),
    Step::Script("ouvrir la saisie du nom", m10_script::open_naming),
    Step::Wait(0.5),
    Step::Script("taper le nom", m10_script::type_locker_name),
    Step::Wait(0.5),
    Step::Shot("m10-saisie-nom"),
    Step::Script("valider le nom", m10_script::submit_locker_name),
    Step::Script("la plaque porte le nom", m10_script::plate_shows_name),
    Step::Wait(0.5),
    Step::Shot("m10-plaque"),
    Step::Script("sauvegarder et recharger", m10_script::save_and_reload),
    Step::Script("retourner dans la chambre", m10_script::go_to_chambre),
    Step::Script(
        "le nom est resté après le chargement",
        m10_script::name_kept,
    ),
    Step::Script(
        "la plaque porte toujours le nom",
        m10_script::plate_shows_name,
    ),
    Step::Wait(0.5),
    Step::Shot("m10-plaque-rechargee"),
];

/// `airlock_space` (avec `SH_START=global:hexis SH_RAID=0`) : en vol, se lever, aller au sas,
/// viser sa porte extérieure (le vide derrière, M10) : l'invite le dit (`interact.door.void`) et
/// la touche d'interaction ne la sollicite pas (elle reste `Closed`, jamais `Unlocking`, sans
/// son) ; capture `target/shots/m10-sas-vide.png`.
const AIRLOCK_SPACE: &[Step] = &[
    Step::Wait(1.0),
    Step::StandUp,
    w(0.0, 18.6, None),
    w(0.0, 18.0, None),
    w(0.0, 16.4, None),
    w(0.0, 15.0, Some("couloir_avant__cockpit")),
    w(0.0, 12.8, None),
    w(0.0, -3.0, None),
    w(-2.4, -3.0, Some("couloir_arriere__sas")),
    w(-5.4, -3.0, None),
    Step::Script(
        "viser la porte extérieure du sas",
        m10_script::aim_void_door,
    ),
    Step::Wait(0.5),
    Step::Shot("m10-sas-vide"),
    Step::Script(
        "la touche d'interaction : la porte reste fermée",
        m10_script::press_void_door,
    ),
];

/// Les actions propres aux scénarios de M10 ([`Step::Script`]) : le frigo, le nom d'un casier
/// sur sa plaque, la porte du sas sur le vide. Chacune journalise ce qu'elle constate quand elle
/// échoue ; le temps de l'étape est celui du scénario (`Scenario::step_time`).
mod m10_script {
    use bevy::prelude::*;

    use super::{CHARACTER_LOCKER, Scenario, aim_spot, cell_local_ready, go_to_cell, place_player};
    use crate::items::{Containers, ItemCatalog, PLAYER_SHIP};
    use crate::ui::character::logic::{self, Place};

    /// Le frigo de la cuisine de la corvette (`gameplay/items.ron`).
    const FRIDGE: &str = "corvette_carre_frigo";
    /// Un objet que le frigo refuse (pièces détachées, `SH_GIVE=spare_parts:5`).
    const PARTS: &str = "spare_parts";
    const RATION: &str = "ration";
    /// Le nom donné au casier.
    const LOCKER_NAME: &str = "Mes affaires";
    const CHAMBRE: &str = "chambre";

    /// Ce qu'une action a fait d'une étape à l'autre : l'étape où elle a agi (une seule fois par
    /// étape), et la racine du secteur avant un chargement.
    #[derive(Resource, Default)]
    struct M10State {
        done_at: Option<usize>,
        root: Option<Entity>,
    }

    fn elapsed(world: &World) -> f32 {
        world.resource::<Scenario>().step_time
    }

    /// Vrai la première fois qu'on l'appelle à cette étape du scénario (l'action à faire une fois).
    fn first_time(world: &mut World) -> bool {
        let index = world.resource::<Scenario>().index;
        let mut state = world.get_resource_or_insert_with(M10State::default);
        if state.done_at == Some(index) {
            return false;
        }
        state.done_at = Some(index);
        true
    }

    /// L'action de cette étape a-t-elle déjà été faite ([`first_time`]) ?
    fn done_here(world: &World) -> bool {
        let index = world.resource::<Scenario>().index;
        world
            .get_resource::<M10State>()
            .is_some_and(|s| s.done_at == Some(index))
    }

    fn texts(world: &World) -> Option<&sh_data::Lang> {
        world.get_resource::<crate::data::Texts>().map(|t| &t.0)
    }

    fn held(world: &World, item: &str) -> u32 {
        super::held(world, item)
    }

    fn fridge_content(world: &World) -> std::collections::BTreeMap<String, u32> {
        world
            .get_resource::<Containers>()
            .and_then(|c| c.contents(FRIDGE))
            .cloned()
            .unwrap_or_default()
    }

    fn stock(world: &World) -> Option<u32> {
        let catalog = world.get_resource::<ItemCatalog>()?;
        Some(
            world
                .get_resource::<Containers>()?
                .stock(PLAYER_SHIP, FRIDGE, catalog),
        )
    }

    /// Range ou prend au frigo par les fonctions de l'écran de rangement, sur les vraies
    /// ressources. `store` : ranger (sinon prendre) ; rend ce que la fonction rend (rangé :
    /// `None` si refusé).
    fn transfer(world: &mut World, store: bool, item: &str, count: u32) -> Option<Option<u32>> {
        let catalog = world.get_resource::<ItemCatalog>()?.0.clone();
        // `resource_scope` et `resource_mut` paniquent sans leur ressource.
        (world.contains_resource::<crate::interaction::Inventory>()
            && world.contains_resource::<Containers>())
        .then_some(())?;
        Some(
            world.resource_scope(|world, mut containers: Mut<Containers>| {
                let mut inventory = world.resource_mut::<crate::interaction::Inventory>();
                let at = Place {
                    ship: PLAYER_SHIP,
                    container: FRIDGE,
                };
                if store {
                    logic::store(
                        &mut inventory.items,
                        &mut containers,
                        &catalog,
                        at,
                        item,
                        count,
                    )
                } else {
                    Some(logic::retrieve(
                        &mut inventory.items,
                        &mut containers,
                        at,
                        item,
                        count,
                    ))
                }
            }),
        )
    }

    /// Le frigo est ouvert (`SH_CONTAINER`) et son écran affiche « Rations : `count` », qui est
    /// bien son stock.
    pub(super) fn fridge_shows(world: &mut World, count: u32) -> Result<bool, String> {
        use crate::ui::character::{CharacterScreen, ScreenView};
        let t = elapsed(world);
        let Some(lang) = texts(world) else {
            return Ok(false);
        };
        let wanted = lang
            .format(
                "container.fridge.stock",
                &[("count", &lang.number(f64::from(count), 0))],
            )
            .to_lowercase();
        let view = world
            .get_resource::<CharacterScreen>()
            .map(|s| s.view.clone());
        let open = view == Some(ScreenView::Container(FRIDGE.to_owned()))
            && crate::ui::character::is_shown(world);
        let stock = stock(world);
        let shown = open && {
            let mut texts = world.query::<&Text>();
            texts
                .iter(world)
                .any(|t| t.0.to_lowercase().contains(&wanted))
        };
        if shown && stock == Some(count) {
            info!("scénario fridge : l'écran du frigo affiche « {wanted} »");
            return Ok(true);
        }
        if t > 15.0 {
            return Err(format!(
                "écran {view:?} (ouvert {open}), stock {stock:?}, « {wanted} » affiché {shown} ; lancer avec SH_CONTAINER={FRIDGE}"
            ));
        }
        Ok(false)
    }

    /// Ranger les pièces détachées au frigo : refusé, ni le frigo ni l'inventaire ne changent.
    pub(super) fn fridge_refuses_parts(world: &mut World) -> Result<bool, String> {
        let parts = held(world, PARTS);
        if parts == 0 {
            return Err(format!(
                "aucune pièce détachée sur soi : lancer avec SH_GIVE={PARTS}:5"
            ));
        }
        let before = fridge_content(world);
        let stored = transfer(world, true, PARTS, parts).ok_or("frigo ou inventaire absents")?;
        let after = fridge_content(world);
        let left = held(world, PARTS);
        if stored.is_some() || after != before || left != parts {
            return Err(format!(
                "le frigo a pris des pièces : rangé {stored:?}, frigo {before:?} → {after:?}, inventaire {parts} → {left}"
            ));
        }
        info!("scénario fridge : {parts} × {PARTS} refusés, frigo inchangé {after:?}");
        Ok(true)
    }

    /// Prendre une ration du frigo : elle passe dans l'inventaire.
    pub(super) fn fridge_take_ration(world: &mut World) -> Result<bool, String> {
        let before = held(world, RATION);
        let taken = transfer(world, false, RATION, 1).ok_or("frigo ou inventaire absents")?;
        let after = held(world, RATION);
        if taken != Some(1) || after != before + 1 {
            return Err(format!(
                "ration non prise : {taken:?}, inventaire {before} → {after}"
            ));
        }
        Ok(true)
    }

    /// Remettre la ration au frigo : acceptée (c'est de la nourriture).
    pub(super) fn fridge_put_back_ration(world: &mut World) -> Result<bool, String> {
        let before = held(world, RATION);
        let stored = transfer(world, true, RATION, 1).ok_or("frigo ou inventaire absents")?;
        let after = held(world, RATION);
        if stored != Some(1) || after + 1 != before {
            return Err(format!(
                "ration refusée par le frigo : {stored:?}, inventaire {before} → {after}"
            ));
        }
        Ok(true)
    }

    /// Le joueur est posé dans la chambre du vaisseau, la salle chargée en qualité Locale.
    pub(super) fn go_to_chambre(world: &mut World) -> Result<bool, String> {
        let t = elapsed(world);
        // Après un chargement, le joueur peut n'apparaître qu'un peu après le secteur : on
        // réessaie jusqu'à ce qu'il soit posé dans la chambre.
        if !done_here(world) {
            if go_to_cell(world, CHAMBRE) {
                first_time(world);
            } else if t > 10.0 {
                return Err(format!("cellule {CHAMBRE} ou joueur introuvables"));
            }
            return Ok(false);
        }
        let current = world
            .get_resource::<crate::sector::streaming::CellStates>()
            .and_then(|c| c.current.clone());
        if cell_local_ready(world, CHAMBRE) && current.as_deref() == Some(CHAMBRE) && t > 2.0 {
            return Ok(true);
        }
        if t > 20.0 {
            return Err(format!(
                "la chambre ne se charge pas autour du joueur (cellule courante {current:?})"
            ));
        }
        Ok(false)
    }

    /// La saisie du nom du premier casier de la chambre s'ouvre (comme sa plaque visée et `E`).
    pub(super) fn open_naming(world: &mut World) -> Result<bool, String> {
        if first_time(world) {
            world.write_message(crate::items::NameContainer(CHARACTER_LOCKER.to_owned()));
        }
        if crate::ui::character::naming::is_naming(world) {
            return Ok(true);
        }
        if elapsed(world) > 6.0 {
            let mode = super::control_mode(world);
            return Err(format!("la saisie ne s'ouvre pas (mode {mode:?})"));
        }
        Ok(false)
    }

    pub(super) fn type_locker_name(world: &mut World) -> Result<bool, String> {
        if crate::ui::character::naming::type_name(world, LOCKER_NAME) {
            return Ok(true);
        }
        if elapsed(world) > 5.0 {
            return Err("aucun champ de saisie".into());
        }
        Ok(false)
    }

    /// Valider (comme Entrée) : la saisie se ferme et le casier porte le nom.
    pub(super) fn submit_locker_name(world: &mut World) -> Result<bool, String> {
        if first_time(world) {
            crate::ui::character::naming::submit_name(world);
        }
        let open = world.contains_resource::<crate::ui::character::naming::NamingScreen>();
        let name = world
            .get_resource::<Containers>()
            .and_then(|c| c.name(PLAYER_SHIP, CHARACTER_LOCKER))
            .map(str::to_owned);
        if !open && name.as_deref() == Some(LOCKER_NAME) {
            info!("scénario locker_name : {CHARACTER_LOCKER} s'appelle « {LOCKER_NAME} »");
            return Ok(true);
        }
        if elapsed(world) > 5.0 {
            return Err(format!("saisie ouverte {open}, nom {name:?}"));
        }
        Ok(false)
    }

    /// Le nom est toujours là (après le chargement).
    pub(super) fn name_kept(world: &mut World) -> Result<bool, String> {
        let name = world
            .get_resource::<Containers>()
            .and_then(|c| c.name(PLAYER_SHIP, CHARACTER_LOCKER))
            .map(str::to_owned);
        if name.as_deref() == Some(LOCKER_NAME) {
            return Ok(true);
        }
        Err(format!(
            "après le chargement, {CHARACTER_LOCKER} s'appelle {name:?}"
        ))
    }

    /// La plaque du casier (`SOCKET_<n>_label`) porte le nom ; le joueur se place devant pour la
    /// lire (la capture), visant la plaque : l'invite propose de la renommer.
    pub(super) fn plate_shows_name(world: &mut World) -> Result<bool, String> {
        use crate::sector::gltf::{ContainerLabel, Interactable};
        use crate::views::FootLabel;
        let t = elapsed(world);
        let plate = world
            .query::<(&ContainerLabel, Option<&FootLabel>, &GlobalTransform)>()
            .iter(world)
            .find(|(l, ..)| l.container == CHARACTER_LOCKER)
            .map(|(_, f, g)| (f.map(|f| f.text.clone()), g.translation(), g.forward()));
        let Some((text, anchor, out)) = plate else {
            if t > 6.0 {
                return Err(format!(
                    "aucune plaque pour {CHARACTER_LOCKER} (le modèle de la chambre a-t-il SOCKET_*_label ?)"
                ));
            }
            return Ok(false);
        };
        if text.as_deref() != Some(LOCKER_NAME) {
            if t > 6.0 {
                return Err(format!("la plaque affiche {text:?}"));
            }
            return Ok(false);
        }
        if first_time(world) {
            // Viser la plaque (son volume d'interaction), du côté où elle se lit.
            let target = world
                .query::<(Entity, &Interactable, &GlobalTransform)>()
                .iter(world)
                .find(|(_, i, _)| {
                    matches!(i, Interactable::LockerLabel { container, .. } if container == CHARACTER_LOCKER)
                })
                .map(|(e, _, g)| (e, g.translation()));
            let feet_y = world
                .query_filtered::<&Transform, With<crate::player::Player>>()
                .single(world)
                .map_or(anchor.y - 1.5, |t| t.translation.y);
            let side = Vec2::new(out.x, out.z).normalize_or(Vec2::Y);
            let stand = target
                .and_then(|(entity, at)| aim_spot(world, entity, at, feet_y, side))
                .unwrap_or(
                    Vec3::new(anchor.x, feet_y, anchor.z) + Vec3::new(out.x, 0.0, out.z) * 1.2,
                );
            place_player(world, stand, Some(anchor));
            info!(
                "scénario locker_name : la plaque de {CHARACTER_LOCKER} affiche « {LOCKER_NAME} »"
            );
        }
        if t < 1.5 {
            return Ok(false);
        }
        let prompt = world
            .get_resource::<crate::ui::hud::Prompt>()
            .and_then(|p| p.0.clone());
        if prompt.as_deref().is_some_and(|p| p.contains(LOCKER_NAME)) {
            return Ok(true);
        }
        if t > 6.0 {
            return Err(format!("devant la plaque, invite {prompt:?}"));
        }
        Ok(false)
    }

    /// Sauvegarde manuelle, puis chargement de ce fichier ; fini quand la partie est revenue (un
    /// nouveau secteur chargé).
    pub(super) fn save_and_reload(world: &mut World) -> Result<bool, String> {
        if first_time(world) {
            let root = world
                .get_resource::<crate::sector::ActiveSector>()
                .map(|a| a.root);
            let Some(path) = crate::save::write_save(world, crate::save::SaveKind::Manual) else {
                return Err(
                    "sauvegarde refusée (lancer à quai : SH_START=sector:spaceport)".into(),
                );
            };
            info!("scénario locker_name : sauvegarde {}", path.display());
            crate::save::request_load(world, &path);
            world.resource_mut::<M10State>().root = root;
            return Ok(false);
        }
        let before = world.resource::<M10State>().root;
        let now = world
            .get_resource::<crate::sector::ActiveSector>()
            .map(|a| a.root);
        let in_game = *world.resource::<State<crate::state::AppState>>().get()
            == crate::state::AppState::InGame;
        Ok(in_game && now.is_some() && now != before && world.contains_resource::<Containers>())
    }

    /// La porte du sas qui donne sur le vide (vaisseau en vol), et son repère.
    fn void_door(world: &mut World) -> Option<(Entity, GlobalTransform, f32)> {
        use crate::sector::doors::Door;
        world
            .query::<(Entity, &Door, &GlobalTransform)>()
            .iter(world)
            .find(|(_, d, _)| Door::leads_to_void(&d.a, &d.b))
            .map(|(e, d, g)| (e, *g, d.height))
    }

    /// Se placer dans le sas devant sa porte extérieure et la viser : l'invite dit le vide.
    pub(super) fn aim_void_door(world: &mut World) -> Result<bool, String> {
        let Some((entity, at, height)) = void_door(world) else {
            return Err("aucune porte sur le vide (le vaisseau est-il amarré ?)".into());
        };
        if first_time(world) {
            // La porte regarde vers le vide (son avant) ; on se tient côté sas, à 1,5 m.
            let base = at.translation();
            let inside = -at.forward().as_vec3();
            let feet_y = world
                .query_filtered::<&Transform, With<crate::player::Player>>()
                .single(world)
                .map_or(base.y, |t| t.translation.y);
            let stand = Vec3::new(base.x + inside.x * 1.5, feet_y, base.z + inside.z * 1.5);
            place_player(world, stand, Some(base + Vec3::Y * (height / 2.0)));
        }
        let aimed = world
            .resource::<crate::interaction::Focus>()
            .0
            .as_ref()
            .is_some_and(|t| t.entity == entity);
        let prompt = world
            .get_resource::<crate::ui::hud::Prompt>()
            .and_then(|p| p.0.clone());
        let wanted = texts(world).map(|l| l.get("interact.door.void"));
        if aimed && prompt.is_some() && prompt == wanted {
            info!(
                "scénario airlock_space : porte visée, invite « {} »",
                prompt.unwrap_or_default()
            );
            return Ok(true);
        }
        if elapsed(world) > 6.0 {
            return Err(format!(
                "porte visée {aimed}, invite {prompt:?} (attendue {wanted:?})"
            ));
        }
        Ok(false)
    }

    /// La touche d'interaction, trois fois en 3 s, sur la porte visée : elle reste fermée, sans
    /// jamais passer en attente (`Unlocking`) ni bouger.
    pub(super) fn press_void_door(world: &mut World) -> Result<bool, String> {
        use crate::sector::doors::{Door, DoorState};
        let Some((entity, ..)) = void_door(world) else {
            return Err("la porte sur le vide a disparu".into());
        };
        let Some((state, openness)) = world.get::<Door>(entity).map(|d| (d.state, d.openness))
        else {
            return Err("la porte sur le vide a disparu".into());
        };
        if state != DoorState::Closed || openness > 0.0 {
            return Err(format!(
                "la porte a réagi : {state:?}, ouverture {openness:.2}"
            ));
        }
        let t = elapsed(world);
        let aimed = world
            .resource::<crate::interaction::Focus>()
            .0
            .as_ref()
            .is_some_and(|f| f.entity == entity);
        if !aimed {
            return Err("la visée a quitté la porte".into());
        }
        // Un appui vers 0,2 s, 1,2 s et 2,2 s.
        let due = if t < 0.2 { 0 } else { 1 + (t - 0.2) as u32 };
        let mut count = world.get_resource_or_insert_with(PressCount::default);
        if due > count.0 && count.0 < 3 {
            count.0 += 1;
            world.write_message(crate::interaction::InteractRequest);
        }
        if t > 3.2 {
            let presses = world.resource::<PressCount>().0;
            info!(
                "scénario airlock_space : {presses} appuis sur la porte du vide, elle est restée fermée"
            );
            return Ok(true);
        }
        Ok(false)
    }

    /// Appuis faits sur la porte du sas.
    #[derive(Resource, Default)]
    struct PressCount(u32);
}

/// Distance horizontale (m) à laquelle l'autopilote s'arrête devant une console pour l'utiliser.
const CONSOLE_DISTANCE: f32 = 1.4;

/// Délai maximal d'une transition de vue (amarrage, départ) dans un scénario.
const TRANSITION_SECONDS: f32 = 30.0;
/// Durée maximale d'un trajet de l'autopilote dans un système (s). D'une planète à l'autre de
/// Vorrh (≈ 3,7 millions de km) : ≈ 4 min mesurées (05/11/2026), dont ≈ 2 min de croisière
/// plafonnée à 30 000 km/s et ≈ 1 min à chaque bout (limite de croisière proportionnelle à la
/// distance à la surface) ; marge de 50 %.
const AUTOPILOT_SECONDS: f32 = 360.0;

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
        if name == "map_key" {
            app.init_resource::<MapKeyCheck>()
                .add_systems(PreUpdate, map_key);
            return;
        }
        if name == "pickup_persist" {
            app.init_resource::<PickupCheck>()
                .add_systems(Update, pickup_persist);
            return;
        }
        // Touches tenues comme par un joueur : la touche est relâchée avant que le scénario
        // regarde la suite.
        if name == "key_help" {
            app.init_resource::<HeldKey>()
                .init_resource::<KeyHelpCheck>()
                .add_systems(PreUpdate, (release_key, key_help).chain());
            return;
        }
        if name == "character" {
            app.init_resource::<HeldKey>()
                .init_resource::<CharacterCheck>()
                .add_systems(PreUpdate, (release_key, character_screen).chain());
            return;
        }
        let (steps, flight) = match name.as_str() {
            "walk_ship" => (WALK_SHIP, None),
            "fly_ship" => (FLY_SHIP_WALK, Some(FlightCheck::default())),
            "port_loop" => (PORT_LOOP, None),
            "voyage" => (VOYAGE, None),
            "voyage_isolated" => (VOYAGE_ISOLATED, None),
            "contract_loop" => (CONTRACT_LOOP, None),
            "deliver_vorrh" => (DELIVER_VORRH, None),
            "raid_win" => (RAID_WIN, None),
            "raid_flee" => (RAID_FLEE, None),
            "raid_lose" => (RAID_LOSE, None),
            "firefight" => (FIREFIGHT, None),
            "firefight_lose" => (FIREFIGHT_LOSE, None),
            "tutorial" => (TUTORIAL, None),
            "terminal_tour" => (TERMINAL_TOUR, None),
            "fridge" => (FRIDGE, None),
            "locker_name" => (LOCKER_NAME, None),
            "airlock_space" => (AIRLOCK_SPACE, None),
            _ => (WALK_SHIP, None),
        };
        if !matches!(
            name.as_str(),
            "walk_ship"
                | "fly_ship"
                | "port_loop"
                | "voyage"
                | "voyage_isolated"
                | "contract_loop"
                | "deliver_vorrh"
                | "raid_win"
                | "raid_flee"
                | "raid_lose"
                | "firefight"
                | "firefight_lose"
                | "tutorial"
                | "terminal_tour"
                | "fridge"
                | "locker_name"
                | "airlock_space"
        ) {
            error!(
                "SH_SCENARIO inconnu : « {name} » (walk_ship, fly_ship, port_loop, voyage, voyage_isolated, contract_loop, deliver_vorrh, raid_win, raid_flee, raid_lose, firefight, firefight_lose, tutorial, terminal_tour, fridge, locker_name, airlock_space, menu_return, save_load, pause_escape, map_key, pickup_persist, key_help, character)"
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

/// Envoie un appui ou un relâchement d'une touche, comme le clavier (contextes d'entrée compris).
fn send_key(
    world: &mut World,
    key_code: KeyCode,
    logical_key: bevy::input::keyboard::Key,
    pressed: bool,
) {
    use bevy::input::ButtonState;
    use bevy::input::keyboard::KeyboardInput;
    let mut windows = world.query_filtered::<Entity, With<bevy::window::PrimaryWindow>>();
    let Ok(window) = windows.single(world) else {
        return;
    };
    world.write_message(KeyboardInput {
        key_code,
        logical_key,
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

fn escape_key(world: &mut World, pressed: bool) {
    send_key(
        world,
        KeyCode::Escape,
        bevy::input::keyboard::Key::Escape,
        pressed,
    );
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

/// `map_key` : la carte, à la vraie touche.
#[derive(Resource, Default)]
struct MapKeyCheck {
    step: u8,
    since: f32,
    total: f32,
    /// M enfoncé : le relâcher quand ce délai tombe à zéro (un appui humain dure ≈ 150 ms,
    /// bien plus qu'un frame : la carte s'ouvre touche encore enfoncée).
    holding: Option<f32>,
}

const KEY_HOLD_SECONDS: f32 = 0.15;

/// `map_key` (avec `SH_START=new`) : aux commandes ([`take_helm`] : la partie commence debout),
/// M ouvre la carte, qui reste ouverte ; M la referme, retour aux commandes. La carte se
/// refermait d'elle-même une demi-seconde après son ouverture.
fn map_key(world: &mut World) {
    use crate::state::ControlMode;
    let dt = world.resource::<Time<Real>>().delta_secs();
    let mut check = world.resource_mut::<MapKeyCheck>();
    check.since += dt;
    check.total += dt;
    if let Some(left) = check.holding {
        if left > dt {
            check.holding = Some(left - dt);
        } else {
            check.holding = None;
            send_key(
                world,
                KeyCode::KeyM,
                bevy::input::keyboard::Key::Character("m".into()),
                false,
            );
        }
        return;
    }
    let (step, since, total) = (check.step, check.since, check.total);
    if total > 90.0 {
        error!("scénario map_key : bloqué à l'étape {step}");
        world.write_message(AppExit::error());
        return;
    }
    let view = world.get_resource::<State<ViewLevel>>().map(|s| *s.get());
    let mode = world.get_resource::<State<ControlMode>>().map(|s| *s.get());
    let busy = world.contains_resource::<crate::transition::Sequence>();
    let seated = view == Some(ViewLevel::Global) && mode == Some(ControlMode::Piloting) && !busy;
    match step {
        // Aux commandes depuis un moment : M.
        0 if seated && since > 3.0 => {}
        // Une nouvelle partie commence debout dans la chambre (M9) : d'abord les commandes, puis
        // trois secondes assis.
        0 => {
            if view == Some(ViewLevel::Global) && !busy && mode != Some(ControlMode::Piloting) {
                take_helm(world);
                world.resource_mut::<MapKeyCheck>().since = 0.0;
            }
            return;
        }
        // Trois secondes plus tard, la carte doit toujours être là.
        1 if since < 3.0 => {
            if view != Some(ViewLevel::Galaxy) && since > 1.0 {
                error!("scénario map_key : la carte s'est refermée seule (vue {view:?})");
                world.write_message(AppExit::error());
            }
            return;
        }
        1 => {}
        // Refermée : retour aux commandes.
        2 if seated && since > 1.0 => {
            info!(
                "scénario map_key terminé : la carte s'ouvre, reste ouverte et se referme à la touche"
            );
            world.write_message(AppExit::Success);
            return;
        }
        _ => return,
    }
    send_key(
        world,
        KeyCode::KeyM,
        bevy::input::keyboard::Key::Character("m".into()),
        true,
    );
    let mut check = world.resource_mut::<MapKeyCheck>();
    check.holding = Some(KEY_HOLD_SECONDS);
    check.step += 1;
    check.since = 0.0;
}

/// La touche que tient un scénario (`key_help`, `character`) et le temps qu'il reste avant de
/// la relâcher : un appui humain dure ≈ 150 ms ([`KEY_HOLD_SECONDS`]), bien plus qu'un frame,
/// et l'écran qui s'ouvre voit la touche encore enfoncée (AGENTS.md §4, touche tenue d'un
/// contexte à l'autre).
#[derive(Resource, Default)]
struct HeldKey(Option<(KeyCode, f32)>);

/// Délai après le relâchement d'une touche avant d'en regarder l'effet (écran ouvert ou fermé,
/// mode de commande changé : quelques frames suffisent, on laisse de la marge).
const KEY_SETTLE_SECONDS: f32 = 0.6;

/// La touche logique d'un message clavier simulé : le jeu lit les touches physiques, la
/// logique ne sert qu'à la saisie de texte.
fn logical_key(code: KeyCode) -> bevy::input::keyboard::Key {
    use bevy::input::keyboard::{Key, NativeKey};
    match code {
        KeyCode::Escape => Key::Escape,
        KeyCode::Enter => Key::Enter,
        KeyCode::F1 => Key::F1,
        KeyCode::KeyE => Key::Character("e".into()),
        KeyCode::KeyI => Key::Character("i".into()),
        KeyCode::KeyO => Key::Character("o".into()),
        _ => Key::Unidentified(NativeKey::Unidentified),
    }
}

/// Enfonce une touche (vrai message clavier : contextes d'entrée et écrans la voient comme
/// celle d'un joueur) ; [`release_key`] la relâche après [`KEY_HOLD_SECONDS`].
fn hold_key(world: &mut World, code: KeyCode) {
    send_key(world, code, logical_key(code), true);
    world.resource_mut::<HeldKey>().0 = Some((code, KEY_HOLD_SECONDS));
}

/// Relâche la touche tenue quand son temps est écoulé.
fn release_key(world: &mut World) {
    let dt = world.resource::<Time<Real>>().delta_secs();
    let mut held = world.resource_mut::<HeldKey>();
    let Some((code, left)) = held.0 else {
        return;
    };
    if left > dt {
        held.0 = Some((code, left - dt));
        return;
    }
    held.0 = None;
    send_key(world, code, logical_key(code), false);
}

/// Une touche est-elle encore tenue (le scénario attend son relâchement) ?
fn key_held(world: &World) -> bool {
    world.resource::<HeldKey>().0.is_some()
}

/// Le mode de commande courant.
fn control_mode(world: &World) -> Option<crate::state::ControlMode> {
    world
        .get_resource::<State<crate::state::ControlMode>>()
        .map(|s| *s.get())
}

/// `key_help` : l'encart des touches, à la vraie touche.
#[derive(Resource, Default)]
struct KeyHelpCheck {
    step: u8,
    since: f32,
    total: f32,
    /// L'encart était-il déplié au départ ?
    initial: bool,
}

/// `key_help` (avec `SH_START=sector:spaceport SH_SPAWN=hall`) : à pied, F1 tenue comme
/// par un joueur bascule l'encart des touches (`ui::keys_panel::KeyHelpVisible`) une seule
/// fois malgré la touche tenue ; un second appui le remet comme au départ.
fn key_help(world: &mut World) {
    use crate::state::ControlMode;
    use crate::ui::keys_panel::KeyHelpVisible;
    const DONE: u8 = u8::MAX;
    let dt = world.resource::<Time<Real>>().delta_secs();
    let mut check = world.resource_mut::<KeyHelpCheck>();
    check.since += dt;
    check.total += dt;
    let (step, since, total, initial) = (check.step, check.since, check.total, check.initial);
    if step == DONE {
        return;
    }
    let fail = |world: &mut World, message: String| {
        error!("scénario key_help : {message}");
        world.write_message(AppExit::error());
        world.resource_mut::<KeyHelpCheck>().step = DONE;
    };
    if total > 90.0 {
        fail(world, format!("bloqué à l'étape {step}"));
        return;
    }
    if key_held(world) {
        return;
    }
    let on_foot = world.contains_resource::<crate::sector::ActiveSector>()
        && control_mode(world) == Some(ControlMode::OnFoot);
    let Some(visible) = world.get_resource::<KeyHelpVisible>().map(|v| v.0) else {
        return;
    };
    let settled = since > KEY_HOLD_SECONDS + KEY_SETTLE_SECONDS;
    match step {
        // À pied depuis un moment : on note l'état de l'encart, puis F1.
        0 if on_foot && since > 3.0 => {
            world.resource_mut::<KeyHelpCheck>().initial = visible;
        }
        // F1 relâchée : l'encart a basculé une fois (puis il est revenu).
        1 | 2 if settled => {
            let expected = if step == 1 { !initial } else { initial };
            if visible != expected {
                fail(
                    world,
                    format!(
                        "après l'appui {step} sur F1, encart déplié {visible} (attendu {expected})"
                    ),
                );
                return;
            }
            if step == 2 {
                let (start, other) = if initial {
                    ("déplié", "replié")
                } else {
                    ("replié", "déplié")
                };
                info!(
                    "scénario key_help terminé : F1 tenue bascule l'encart des touches une fois par appui ({start} → {other} → {start})"
                );
                world.write_message(AppExit::Success);
                world.resource_mut::<KeyHelpCheck>().step = DONE;
                return;
            }
        }
        _ => return,
    }
    hold_key(world, KeyCode::F1);
    let mut check = world.resource_mut::<KeyHelpCheck>();
    check.step += 1;
    check.since = 0.0;
}

/// Le casier ouvert par le scénario `character` (contenu de départ : `gameplay/items.ron`).
const CHARACTER_LOCKER: &str = "corvette_chambre_casier_1";

/// `character` : l'écran du personnage et le rangement d'un casier, aux vraies touches.
#[derive(Resource, Default)]
struct CharacterCheck {
    step: u8,
    since: f32,
    total: f32,
    /// Le casier et l'inventaire avant le transfert.
    before: Option<(
        std::collections::BTreeMap<String, u32>,
        std::collections::BTreeMap<String, u32>,
    )>,
}

/// Le contenu du casier du scénario et l'inventaire du joueur.
fn locker_and_pack(
    world: &World,
) -> Option<(
    std::collections::BTreeMap<String, u32>,
    std::collections::BTreeMap<String, u32>,
)> {
    let locker = world
        .get_resource::<crate::items::Containers>()?
        .contents(CHARACTER_LOCKER)
        .cloned()
        .unwrap_or_default();
    let pack = world
        .get_resource::<crate::interaction::Inventory>()?
        .items
        .clone();
    Some((locker, pack))
}

/// `character` (avec `SH_START=sector:corvette SH_SPAWN=soute`) : I tenue ouvre l'écran du
/// personnage (onglet Inventaire), O passe aux Compétences, Échap le ferme sans ouvrir la
/// pause derrière, retour à pied ; puis un casier ([`CHARACTER_LOCKER`]) : Entrée prend la case
/// choisie (comme au clavier dans l'écran), E referme le rangement.
fn character_screen(world: &mut World) {
    use crate::state::ControlMode;
    use crate::ui::character::{CharacterScreen, ScreenView};
    use crate::ui::{character, pause};
    const DONE: u8 = u8::MAX;
    let dt = world.resource::<Time<Real>>().delta_secs();
    let mut check = world.resource_mut::<CharacterCheck>();
    check.since += dt;
    check.total += dt;
    let (step, since, total) = (check.step, check.since, check.total);
    if step == DONE {
        return;
    }
    let fail = |world: &mut World, message: String| {
        error!("scénario character : {message}");
        world.write_message(AppExit::error());
        world.resource_mut::<CharacterCheck>().step = DONE;
    };
    if total > 120.0 {
        fail(world, format!("bloqué à l'étape {step}"));
        return;
    }
    if key_held(world) {
        return;
    }
    let settled = since > KEY_HOLD_SECONDS + KEY_SETTLE_SECONDS;
    let mode = control_mode(world);
    let view = world
        .get_resource::<CharacterScreen>()
        .map(|s| s.view.clone());
    let shown = character::is_shown(world);
    let paused = world.contains_resource::<pause::Paused>() || pause::is_shown(world);
    // L'écran est fermé pour de bon : ni ressource, ni entités, ni pause, et à pied.
    let closed = view.is_none() && !shown && !paused && mode == Some(ControlMode::OnFoot);
    let state = format!("(écran {view:?}, affiché {shown}, pause {paused}, mode {mode:?})");
    let key = match step {
        // À pied depuis un moment, la partie prête : I.
        0 => {
            let ready = world.contains_resource::<crate::sector::ActiveSector>()
                && world.contains_resource::<crate::career::Career>()
                && locker_and_pack(world).is_some()
                && mode == Some(ControlMode::OnFoot);
            if !(ready && since > 3.0) {
                return;
            }
            KeyCode::KeyI
        }
        // L'inventaire est affiché, en mode Menu (curseur libre) : O.
        1 if settled => {
            let expected = ScreenView::Tab(crate::items::CharacterTab::Inventory);
            if view.as_ref() != Some(&expected) || !shown || mode != Some(ControlMode::Menu) {
                fail(world, format!("I n'ouvre pas l'inventaire {state}"));
                return;
            }
            KeyCode::KeyO
        }
        // Les compétences : Échap.
        2 if settled => {
            let expected = ScreenView::Tab(crate::items::CharacterTab::Skills);
            if view.as_ref() != Some(&expected) || !shown {
                fail(world, format!("O ne passe pas aux compétences {state}"));
                return;
            }
            KeyCode::Escape
        }
        // Échap a fermé l'écran, sans pause derrière : le casier (comme le ferait la touche
        // d'interaction en le visant ; il est dans la chambre, le scénario reste dans la soute).
        3 if settled => {
            if !closed {
                fail(
                    world,
                    format!("Échap ne referme pas proprement l'écran {state}"),
                );
                return;
            }
            let before = locker_and_pack(world);
            world.write_message(crate::items::OpenContainer(CHARACTER_LOCKER.to_owned()));
            let mut check = world.resource_mut::<CharacterCheck>();
            check.before = before;
            (check.step, check.since) = (4, 0.0);
            return;
        }
        // Le rangement est affiché, le casier n'est pas vide : Entrée prend la case choisie.
        4 if since > 1.0 => {
            let expected = ScreenView::Container(CHARACTER_LOCKER.to_owned());
            if view.as_ref() != Some(&expected) || !shown {
                fail(world, format!("le casier ne s'ouvre pas {state}"));
                return;
            }
            let empty = world
                .resource::<CharacterCheck>()
                .before
                .as_ref()
                .is_none_or(|(locker, _)| locker.is_empty());
            if empty {
                fail(world, format!("casier {CHARACTER_LOCKER} vide ou inconnu"));
                return;
            }
            KeyCode::Enter
        }
        // Un objet est passé du casier à l'inventaire, en entier : E referme le rangement.
        5 if settled => {
            let before = world.resource::<CharacterCheck>().before.clone();
            let (Some((locker_before, pack_before)), Some((locker_after, pack_after))) =
                (before, locker_and_pack(world))
            else {
                fail(world, "casier ou inventaire disparus".into());
                return;
            };
            let moved = locker_before.iter().find_map(|(item, n)| {
                let left = locker_after.get(item).copied().unwrap_or(0);
                (left < *n).then(|| (item.clone(), n - left))
            });
            let Some((item, count)) = moved else {
                fail(world, format!("Entrée n'a rien pris au casier {state}"));
                return;
            };
            let had = pack_before.get(&item).copied().unwrap_or(0);
            let has = pack_after.get(&item).copied().unwrap_or(0);
            if has != had + count {
                fail(
                    world,
                    format!("{count} × {item} pris au casier, inventaire {had} → {has}"),
                );
                return;
            }
            info!("scénario character : {count} × {item} pris au casier {CHARACTER_LOCKER}");
            KeyCode::KeyE
        }
        // E relâchée : le rangement est fermé, retour à pied.
        6 if settled => {
            if !closed {
                fail(world, format!("E ne referme pas le casier {state}"));
                return;
            }
            info!(
                "scénario character terminé : I ouvre l'inventaire, O les compétences, Échap ferme sans pause ; au casier, Entrée prend un objet et E referme"
            );
            world.write_message(AppExit::Success);
            world.resource_mut::<CharacterCheck>().step = DONE;
            return;
        }
        _ => return,
    };
    hold_key(world, key);
    let mut check = world.resource_mut::<CharacterCheck>();
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

/// `save_load` (avec `SH_USER_DIR` et `SH_START=new` ou `sector:spaceport`) : sauvegarder,
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

/// `pickup_persist` : l'objet ramassé, son point de ramassage et sa cellule (la tablette posée
/// sur le bureau de la chambre du vaisseau du joueur, M9).
const PICKUP_NODE: &str = "INTERACT_chambre_pickup_datapad";
const PICKUP_CELL: &str = "chambre";
const PICKUP_ITEM: &str = "datapad";
/// Distances horizontales (m) d'où l'autopilote essaie de viser l'objet, à portée de main.
const PICKUP_STANDS: [f32; 3] = [1.1, 1.4, 0.8];

/// Un point d'où viser l'objet `entity` (en `at`) : à l'une des [`PICKUP_STANDS`], dans l'une de
/// seize directions en partant de `side`, là où le joueur debout ne touche rien et où le rayon de
/// visée (celui de `interaction`, depuis l'œil) touche l'objet avant tout obstacle.
fn aim_spot(world: &mut World, entity: Entity, at: Vec3, feet_y: f32, side: Vec2) -> Option<Vec3> {
    use crate::player::{RADIUS, STAND_HEIGHT};
    use crate::sector::Layer;
    use avian3d::prelude::*;
    let eye_height = world
        .query_filtered::<&crate::player::PlayerController, With<crate::player::Player>>()
        .single(world)
        .map_or(STAND_HEIGHT - 0.12, |c| c.eye_height());
    let mut state = bevy::ecs::system::SystemState::<(SpatialQuery, Query<&ChildOf>)>::new(world);
    let Ok((spatial, parents)) = state.get(world) else {
        return None;
    };
    let body = Collider::capsule(RADIUS, STAND_HEIGHT - 2.0 * RADIUS);
    let solid = SpatialQueryFilter::from_mask([Layer::Static, Layer::Door]);
    let sight = SpatialQueryFilter::from_mask([Layer::Static, Layer::Door, Layer::Interact]);
    // L'objet touché, ou l'un de ses descendants.
    let is_target = |mut hit: Entity| loop {
        if hit == entity {
            return true;
        }
        match parents.get(hit) {
            Ok(parent) => hit = parent.parent(),
            Err(_) => return false,
        }
    };
    let base = side.y.atan2(side.x);
    for dist in PICKUP_STANDS {
        for k in 0..16 {
            // 0, +1, −1, +2, −2… : d'abord les directions proches du côté de la salle.
            let step = if k % 2 == 0 { k / 2 } else { -(k + 1) / 2 };
            let angle = base + step as f32 * std::f32::consts::TAU / 16.0;
            let feet = Vec3::new(at.x + angle.cos() * dist, feet_y, at.z + angle.sin() * dist);
            let center = feet + Vec3::Y * (STAND_HEIGHT / 2.0 + 0.03);
            if !spatial
                .shape_intersections(&body, center, Quat::IDENTITY, &solid)
                .is_empty()
            {
                continue;
            }
            let eye = feet + Vec3::Y * eye_height;
            let Ok(dir) = Dir3::new(at - eye) else {
                continue;
            };
            let hit = spatial.cast_ray(eye, dir, crate::interaction::REACH, true, &sight);
            if hit.is_some_and(|h| is_target(h.entity)) {
                return Some(feet);
            }
        }
    }
    None
}

/// `pickup_persist` : un objet pris ne revient pas, même après un chargement.
#[derive(Resource, Default)]
struct PickupCheck {
    step: u8,
    since: f32,
    total: f32,
    /// Le nœud de ramassage visé, et son identifiant stable.
    target: Option<(Entity, String)>,
    /// Exemplaires de l'objet dans l'inventaire avant de le ramasser.
    held_before: u32,
}

/// La cellule est-elle chargée en qualité Locale (interactions comprises) ?
fn cell_local_ready(world: &World, cell: &str) -> bool {
    world
        .get_resource::<crate::sector::streaming::CellStates>()
        .and_then(|states| states.slots.get(cell))
        .and_then(|slot| slot.full)
        .is_some_and(|e| {
            world
                .get::<crate::sector::streaming::CellReady>(e)
                .is_some()
        })
}

fn held(world: &World, item: &str) -> u32 {
    world
        .get_resource::<crate::interaction::Inventory>()
        .and_then(|inventory| inventory.items.get(item).copied())
        .unwrap_or(0)
}

/// Place le joueur debout, pieds en `feet`, et tourne son regard vers `look_at` (comme le ferait
/// la souris). Faux sans joueur.
fn place_player(world: &mut World, feet: Vec3, look_at: Option<Vec3>) -> bool {
    use crate::player::{Player, PlayerController, PlayerMode};
    let mut query = world.query_filtered::<(&mut Transform, &mut PlayerController), With<Player>>();
    let Ok((mut transform, mut controller)) = query.single_mut(world) else {
        return false;
    };
    transform.translation = feet;
    controller.mode = PlayerMode::Walking;
    controller.velocity = Vec3::ZERO;
    if let Some(target) = look_at {
        let eye = feet + Vec3::Y * controller.eye_height();
        let d = (target - eye).normalize_or(Vec3::NEG_Z);
        controller.yaw = (-d.x).atan2(-d.z);
        controller.pitch = d.y.clamp(-1.0, 1.0).asin();
    }
    true
}

/// Prend les commandes de n'importe où à bord (raccourci de l'autopilote : la marche jusqu'au
/// cockpit est éprouvée par `walk_ship`) : le joueur va au point d'apparition des commandes du
/// départ, puis s'assoit au poste de pilotage dès que sa cellule est prête. Vrai une fois assis.
fn take_helm(world: &mut World) -> bool {
    use crate::player::{Player, PlayerController, PlayerMode};
    let mut players = world.query_filtered::<&PlayerController, With<Player>>();
    if players
        .single(world)
        .is_ok_and(|c| matches!(c.mode, PlayerMode::Seated { .. }))
    {
        return true;
    }
    let spawn = world
        .resource::<Data>()
        .start()
        .map(|s| s.aboard_spawn.clone())
        .and_then(|id| {
            let active = world.get_resource::<crate::sector::ActiveSector>()?;
            let spawn = active.sector.spawns.iter().find(|sp| sp.id == id)?;
            Some((crate::sector::v3(spawn.pos), spawn.cell.clone()))
        });
    let Some((feet, cell)) = spawn else {
        return false;
    };
    if !cell_local_ready(world, &cell) {
        // Tenu sur place tant que le sol de la cellule n'est pas chargé (sinon il tomberait).
        place_player(world, feet, None);
        return false;
    }
    crate::interaction::sit_at(world, "pilot")
}

/// Place le joueur au centre d'une cellule du secteur chargé, au ras du sol : la cellule se
/// charge autour de lui. Faux si la cellule ou le joueur manquent.
fn go_to_cell(world: &mut World, cell: &str) -> bool {
    let feet = world
        .get_resource::<crate::sector::ActiveSector>()
        .and_then(|active| active.sector.cell(cell))
        .map(|c| {
            let (x, _, z) = c.center();
            Vec3::new(x, c.min.1 + 0.25, z)
        });
    feet.is_some_and(|feet| place_player(world, feet, None))
}

/// `pickup_persist` (avec `SH_USER_DIR` et `SH_START=sector:spaceport`) : à quai, dans la
/// chambre du vaisseau, viser la tablette posée (son modèle du catalogue s'il existe) et la
/// ramasser, sauvegarder, recharger ; la tablette n'est plus sur le bureau, elle est dans
/// l'inventaire, son point de ramassage dans `items::PickedUp`.
fn pickup_persist(world: &mut World) {
    use crate::sector::gltf::{Interactable, PickupId, PickupModel};
    let dt = world.resource::<Time<Real>>().delta_secs();
    let in_game = *world.resource::<State<AppState>>().get() == AppState::InGame;
    let ready = in_game
        && world.contains_resource::<crate::career::Career>()
        && world.contains_resource::<crate::sector::ActiveSector>();
    if ready {
        // La souris ne doit pas déranger la visée.
        world.resource_mut::<crate::player::ScriptedControl>().0 = true;
        *world.resource_mut::<crate::player::PlayerIntent>() = default();
    }
    let mut check = world.resource_mut::<PickupCheck>();
    check.since += dt;
    check.total += dt;
    let (step, since, total) = (check.step, check.since, check.total);
    let fail = |world: &mut World, message: String| {
        error!("scénario pickup_persist : {message}");
        world.write_message(AppExit::error());
        world.resource_mut::<PickupCheck>().step = u8::MAX;
    };
    if total > 150.0 && step != u8::MAX {
        fail(world, format!("bloqué à l'étape {step}"));
        return;
    }
    let next = |world: &mut World| {
        let mut check = world.resource_mut::<PickupCheck>();
        (check.step, check.since) = (check.step + 1, 0.0);
    };
    match step {
        // À quai : dans la salle de l'objet (avant, puis après le chargement).
        0 | 5 if ready && since > 3.0 => {
            if !go_to_cell(world, PICKUP_CELL) {
                fail(world, format!("cellule {PICKUP_CELL} introuvable"));
                return;
            }
            next(world);
        }
        // La salle chargée et le joueur au sol : viser l'objet depuis le côté de la salle.
        1 if cell_local_ready(world, PICKUP_CELL) && since > 1.5 => {
            let mut nodes = world.query::<(
                Entity,
                &Name,
                &Interactable,
                Option<&PickupId>,
                Option<&Children>,
                &GlobalTransform,
            )>();
            let found = nodes
                .iter(world)
                .find(|(_, name, kind, ..)| {
                    name.as_str() == PICKUP_NODE && matches!(kind, Interactable::Pickup { .. })
                })
                .map(|(entity, _, _, id, children, at)| {
                    let children: Vec<Entity> = children.into_iter().flatten().copied().collect();
                    (entity, id.map(|i| i.0.clone()), children, at.translation())
                });
            let Some((entity, id, children, at)) = found else {
                fail(world, format!("{PICKUP_NODE} absent de {PICKUP_CELL}"));
                return;
            };
            let Some(id) = id else {
                fail(
                    world,
                    format!("{PICKUP_NODE} sans identifiant de ramassage"),
                );
                return;
            };
            // Le modèle du catalogue remplace l'apparence du kit quand il existe.
            let model = world
                .resource::<Data>()
                .items
                .get(PICKUP_ITEM)
                .map(|item| item.model_path());
            let produced = model.is_some_and(|m| crate::data::assets_dir().join(m).is_file());
            let shown = children
                .iter()
                .any(|c| world.get::<PickupModel>(*c).is_some());
            if produced != shown {
                fail(
                    world,
                    format!("modèle de l'objet : fichier {produced}, posé {shown}"),
                );
                return;
            }
            let center = world
                .get_resource::<crate::sector::ActiveSector>()
                .and_then(|a| crate::sector::streaming::cell_center(&a.sector, PICKUP_CELL));
            let feet_y = world
                .query_filtered::<&Transform, With<crate::player::Player>>()
                .single(world)
                .map_or(at.y, |t| t.translation.y);
            let (Some(center), true) = (center, feet_y.is_finite()) else {
                fail(world, "joueur ou salle introuvable".into());
                return;
            };
            // D'où viser : côté salle d'abord, puis tout autour, là où le joueur tient debout et
            // où le rayon de visée touche bien l'objet (la chaise du bureau peut le masquer).
            let side = Vec2::new(center.x - at.x, center.z - at.z).normalize_or(Vec2::Y);
            let Some(stand) = aim_spot(world, entity, at, feet_y, side) else {
                fail(world, format!("aucun point d'où viser {PICKUP_NODE}"));
                return;
            };
            place_player(world, stand, Some(at));
            info!("scénario pickup_persist : {id} visé (modèle posé : {shown})");
            let held_before = held(world, PICKUP_ITEM);
            let mut check = world.resource_mut::<PickupCheck>();
            check.target = Some((entity, id));
            check.held_before = held_before;
            (check.step, check.since) = (2, 0.0);
        }
        // La visée désigne l'objet : la touche d'interaction.
        2 => {
            let Some((entity, _)) = world.resource::<PickupCheck>().target.clone() else {
                return;
            };
            let aimed = world
                .resource::<crate::interaction::Focus>()
                .0
                .as_ref()
                .is_some_and(|t| t.entity == entity);
            if aimed {
                world.write_message(crate::interaction::InteractRequest);
                next(world);
            } else if since > 3.0 {
                fail(world, "la visée ne désigne pas l'objet".into());
            }
        }
        // Ramassé : retiré du monde, dans l'inventaire, noté pris ; sauvegarder et recharger.
        3 if since > 0.5 => {
            let Some((entity, id)) = world.resource::<PickupCheck>().target.clone() else {
                return;
            };
            let before = world.resource::<PickupCheck>().held_before;
            let gone = world.get::<Interactable>(entity).is_none();
            let count = held(world, PICKUP_ITEM);
            let noted = world
                .get_resource::<crate::items::PickedUp>()
                .is_some_and(|p| p.contains(&id));
            if !(gone && count == before + 1 && noted) {
                fail(
                    world,
                    format!(
                        "ramassage : retiré {gone}, inventaire {before} → {count}, noté pris {noted}"
                    ),
                );
                return;
            }
            let Some(path) = crate::save::write_save(world, crate::save::SaveKind::Manual) else {
                fail(world, "sauvegarde refusée".into());
                return;
            };
            info!(
                "scénario pickup_persist : ramassé, sauvegarde {}",
                path.display()
            );
            crate::save::request_load(world, &path);
            next(world);
        }
        // Le chargement quitte la partie : on attend d'y être revenu.
        4 if !in_game => next(world),
        // Rechargé, la salle de nouveau chargée : l'objet n'y est plus.
        6 if cell_local_ready(world, PICKUP_CELL) && since > 2.0 => {
            let Some((_, id)) = world.resource::<PickupCheck>().target.clone() else {
                return;
            };
            let mut nodes = world.query::<(&Name, Option<&PickupId>)>();
            let back = nodes
                .iter(world)
                .any(|(name, pid)| name.as_str() == PICKUP_NODE || pid.is_some_and(|p| p.0 == id));
            let count = held(world, PICKUP_ITEM);
            let noted = world
                .get_resource::<crate::items::PickedUp>()
                .is_some_and(|p| p.contains(&id));
            if back || count == 0 || !noted {
                fail(
                    world,
                    format!(
                        "après chargement : objet revenu {back}, inventaire {count}, noté pris {noted}"
                    ),
                );
                return;
            }
            info!(
                "scénario pickup_persist terminé : {id} ramassé, absent après chargement, {count} dans l'inventaire"
            );
            world.write_message(AppExit::Success);
            world.resource_mut::<PickupCheck>().step = u8::MAX;
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
    (consoles, focus, mut interact, terminal): (
        Query<(&crate::sector::gltf::Interactable, &GlobalTransform)>,
        Res<crate::interaction::Focus>,
        MessageWriter<crate::interaction::InteractRequest>,
        Option<Res<crate::ui::terminal::Terminal>>,
    ),
    mut exit: MessageWriter<AppExit>,
) {
    use crate::sector::doors::DoorState;
    use crate::sector::gltf::Interactable;

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
        // Une approche en vol prend plus longtemps qu'une transition.
        let limit = match step {
            Step::FlyToBeacon => 120.0,
            // D'une planète à l'autre, en croisière.
            Step::Autopilot(_) => AUTOPILOT_SECONDS,
            _ => TRANSITION_SECONDS,
        };
        if scenario.step_time > limit {
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
        // Attend que le joueur soit posé (cellule prête, physique active) ; pas indéfiniment :
        // un joueur coincé (levé dans un meuble) ou qui tombe doit faire échouer le scénario.
        scenario.since_progress += time.delta_secs();
        if scenario.since_progress > 2.0 * STUCK_SECONDS {
            error!(
                "scénario {} : le joueur ne se pose pas (position {:?}, cellule {:?})",
                scenario.name,
                transform.translation(),
                cells.current
            );
            exit.write(AppExit::error());
        }
        return;
    }
    if !scenario.started {
        scenario.started = true;
        scenario.since_progress = 0.0;
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
    // Repère du vaisseau ; amarré, ses cellules sont placées dans celui du quartier.
    let placement = docked
        .is_some()
        .then(|| {
            sector
                .as_ref()
                .and_then(|s| s.sector.cells.iter().find_map(|c| c.placement))
        })
        .flatten();
    // La cible de la marche, la porte à ouvrir d'abord, la distance d'arrivée.
    let (target, door, arrive) = match step {
        Step::Walk(x, y, door) => (
            placement.map_or(Vec3::new(x, 0.0, -y), |p| {
                crate::sector::v3(p.apply((x, 0.0, -y)))
            }),
            door,
            0.35,
        ),
        Step::WalkDistrict(x, z, door) => (Vec3::new(x, 0.0, z), door, 0.35),
        Step::UseConsole(label) => {
            if terminal.as_ref().is_some_and(|t| t.page.is_some()) {
                info!("scénario {} : console « {label} » utilisée", scenario.name);
                advance(&mut scenario);
                return;
            }
            let Some(console) = consoles
                .iter()
                .find(|(i, _)| matches!(i, Interactable::Console { label: l } if l == label))
                .map(|(_, t)| t.translation())
            else {
                error!("scénario {} : aucune console « {label} »", scenario.name);
                exit.write(AppExit::error());
                return;
            };
            let pos = transform.translation();
            let flat = Vec2::new(console.x - pos.x, console.z - pos.z).length();
            if flat > CONSOLE_DISTANCE {
                (Vec3::new(console.x, 0.0, console.z), None, CONSOLE_DISTANCE)
            } else {
                // Devant la console : la viser (une main humaine, 2,5 rad/s au plus), puis la
                // touche d'interaction dès que la visée la désigne (une fois par seconde).
                let eye = pos + Vec3::Y * controller.eye_height();
                let d = (console - eye).normalize_or(Vec3::NEG_Z);
                let wrap = |a: f32| {
                    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI
                };
                let dy = wrap(controller.yaw - (-d.x).atan2(-d.z));
                let dp = controller.pitch - d.y.clamp(-1.0, 1.0).asin();
                let max = 2.5 * dt;
                intent.look = Vec2::new(dy.clamp(-max, max), dp.clamp(-max, max));
                let aimed = focus.0.as_ref().is_some_and(
                    |t| matches!(&t.kind, Interactable::Console { label: l } if l == label),
                );
                let retry = scenario.step_time - scenario.last_issue > 1.0;
                if aimed && (scenario.issued.is_none() || retry) {
                    interact.write(crate::interaction::InteractRequest);
                    scenario.issued = Some(None);
                    scenario.last_issue = scenario.step_time;
                }
                scenario.step_time += dt;
                if scenario.step_time > TRANSITION_SECONDS {
                    error!(
                        "scénario {} : la console « {label} » ne s'ouvre pas (visée {})",
                        scenario.name,
                        if aimed { "posée" } else { "manquée" }
                    );
                    exit.write(AppExit::error());
                }
                return;
            }
        }
        Step::Thrust(seconds) | Step::Turn(seconds) | Step::Cruise(seconds) => {
            match step {
                Step::Turn(_) => pilot.turn = Vec2::new(0.6, 0.0),
                Step::Cruise(_) => {
                    pilot.thrust = Vec3::NEG_Z;
                    pilot.cruise = true;
                }
                _ => pilot.thrust = Vec3::NEG_Z,
            }
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
        | Step::Talk(_)
        | Step::Choose(_)
        | Step::Hail
        | Step::Assert(..)
        | Step::Shot(_)
        | Step::Wait(_)
        | Step::JumpNear(_)
        | Step::JumpAboveBeacon(_)
        | Step::FlyToBeacon
        | Step::Autopilot(_)
        | Step::Tutorial(_)
        | Step::QuickSave
        | Step::Script(..)
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

    let pos = transform.translation();
    let to = Vec2::new(target.x - pos.x, target.z - pos.z);
    let distance = to.length();
    if distance < arrive {
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
/// ou `accept:any`, `betray`, `repair`, `back` (reprendre la conversation), `close`.
fn choice_matches(choice: &crate::ui::terminal::Choice, pattern: &str) -> bool {
    use crate::ui::terminal::{Action, Screen};
    match (&choice.action, pattern.split_once(':')) {
        (Action::Say { key, .. }, Some(("say", k))) => key == k,
        (Action::Accept(c), Some(("accept", to))) => to == "any" || c.to.as_str() == to,
        (Action::Betray(_), None) => pattern == "betray",
        (Action::Repair, None) => pattern == "repair",
        (Action::Goto(Screen::Dialogue { .. }), None) => pattern == "back",
        (Action::Refuel(_), None) => pattern == "refuel",
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
                        Terminal::Codex => Screen::Codex,
                        Terminal::Repair => Screen::Repair,
                    };
                    world.write_message(OpenTerminal(screen));
                    world.resource_mut::<Scenario>().issued = Some(None);
                }
                _ => {}
            }
        }
        Step::Talk(role) => {
            let open = world.get_resource::<Term>().map(|t| t.page.is_some());
            match (issued, open) {
                (true, Some(true)) => next(world),
                (false, None) => {
                    world.write_message(crate::items::TalkTo {
                        role: role.to_owned(),
                        variant: "a".to_owned(),
                    });
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
        Step::JumpAboveBeacon(distance) => {
            // La balise apparaît avec la vue : on l'attend.
            let Some((beacon, _)) = beacon_of(world) else {
                return;
            };
            // L'astre de la balise : la surface la plus proche.
            let center = world
                .query::<(&crate::space::SpacePosition, &crate::ship::SurfaceBody)>()
                .iter(world)
                .map(|(p, b)| (p.0, p.0.distance(beacon) - b.radius))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(p, _)| p);
            let Some(center) = center else {
                return;
            };
            let up = beacon - center;
            let up = up * (1.0 / up.length().max(1e-6));
            let Some(root) = world
                .get_resource::<crate::sector::ActiveSector>()
                .map(|s| s.root)
            else {
                return;
            };
            if let Some(mut ship) = world.get_mut::<crate::space::SpacePosition>(root) {
                ship.0 = beacon + up * distance;
            }
            if let Some(mut motion) = world.get_mut::<crate::ship::ShipMotion>(root) {
                motion.velocity = Default::default();
                motion.angular = Vec3::ZERO;
            }
            next(world);
        }
        Step::FlyToBeacon => fly_to_beacon(world),
        Step::Autopilot(target) => match autopilot_step(world, target, issued) {
            Ok(true) => next(world),
            Ok(false) => {}
            Err(message) => fail(world, format!("autopilote vers {target} : {message}")),
        },
        Step::Tutorial(n) => {
            let enabled = world
                .get_resource::<crate::options::UserSettings>()
                .is_some_and(|s| s.tutorial);
            if !enabled {
                fail(world, "tutoriel coupé : lancer avec SH_TUTORIAL=1".into());
                return;
            }
            let Some(tutorial) = world.get_resource::<crate::tutorial::Tutorial>().cloned() else {
                return;
            };
            if tutorial.number() >= n {
                info!(
                    "scénario tutorial : étape {n} atteinte ({:?})",
                    tutorial.progress
                );
                next(world);
            } else if world.resource::<Scenario>().step_time > TRANSITION_SECONDS - 0.5 {
                fail(
                    world,
                    format!(
                        "le tutoriel n'atteint pas l'étape {n} : {:?}, cases {:?}",
                        tutorial.progress, tutorial.ticked
                    ),
                );
            }
        }
        Step::QuickSave => {
            crate::save::save_now(world, crate::save::SaveKind::Quick);
            next(world);
        }
        Step::Script(what, run) => match run(world) {
            Ok(true) => {
                let name = world.resource::<Scenario>().name.clone();
                info!("scénario {name} : {what} — fait");
                next(world);
            }
            Ok(false) => {}
            Err(message) => fail(world, format!("{what} : {message}")),
        },
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

/// La balise d'approche du système (position, portée), en vue Globale.
fn beacon_of(world: &mut World) -> Option<(sh_core::SpacePos, f64)> {
    world
        .query::<(&crate::transition::DockBeacon, &crate::space::SpacePosition)>()
        .iter(world)
        .next()
        .map(|(b, p)| (p.0, b.range))
}

/// [`Step::FlyToBeacon`] : un frame de vol vers la balise, par la logique de l'autopilote du
/// jeu ([`crate::ship::approach`], arrêt visé à mi-portée) ; l'étape finit dès qu'on est à
/// portée.
fn fly_to_beacon(world: &mut World) {
    let Some((beacon, range)) = beacon_of(world) else {
        return;
    };
    let Some(sector) = world.get_resource::<crate::sector::ActiveSector>() else {
        return;
    };
    let (root, ship) = (sector.root, sector.sector.id.to_string());
    let pose = world
        .get::<crate::space::SpacePosition>(root)
        .zip(world.get::<crate::space::SpaceRotation>(root))
        .map(|(p, r)| (p.0, r.0));
    let velocity = world
        .get::<crate::ship::ShipMotion>(root)
        .map_or(bevy::math::DVec3::ZERO, |m| m.velocity);
    let Some((pos, rot)) = pose else {
        return;
    };
    let distance = beacon.distance(pos);
    if distance < range * 0.85 {
        info!("scénario : balise à {distance:.0} m, à portée");
        *world.resource_mut::<crate::ship::PilotIntent>() = crate::ship::PilotIntent::default();
        let mut scenario = world.resource_mut::<Scenario>();
        scenario.index += 1;
        scenario.issued = None;
        scenario.step_time = 0.0;
        return;
    }
    let data = world.resource::<Data>().clone();
    let Some(handling) = data.flight.ships.get(ship.as_str()) else {
        return;
    };
    let step = crate::ship::approach(pos, rot, velocity, beacon, range * 0.5, handling, true);
    *world.resource_mut::<crate::ship::PilotIntent>() = step.intent;
}

/// [`Step::Autopilot`] : pose la demande comme la carte, puis attend que l'autopilote la lève ;
/// fini (`Ok(true)`) si le vaisseau est alors arrêté à la distance d'arrêt de la cible.
fn autopilot_step(world: &mut World, target: &str, issued: bool) -> Result<bool, String> {
    use crate::ship::{AutopilotRequest, NavTarget, ShipMotion};
    use crate::space::SpacePosition;
    if !issued {
        world.insert_resource(AutopilotRequest(Some(target.to_owned())));
        world.resource_mut::<Scenario>().issued = Some(None);
        info!("scénario : autopilote engagé vers {target}");
        return Ok(false);
    }
    if world.resource::<AutopilotRequest>().0.is_some() {
        return Ok(false);
    }
    let goal = world
        .query::<(&NavTarget, &SpacePosition)>()
        .iter(world)
        .find(|(t, _)| t.id == target)
        .map(|(t, p)| (p.0, t.stop));
    let Some((goal, stop)) = goal else {
        return Err("cible absente du système".into());
    };
    let root = world
        .get_resource::<crate::sector::ActiveSector>()
        .map(|s| s.root)
        .ok_or("pas de vaisseau")?;
    let ship = world
        .get::<SpacePosition>(root)
        .map(|p| p.0)
        .ok_or("vaisseau sans position")?;
    let speed = world
        .get::<ShipMotion>(root)
        .map_or(0.0, |m| m.velocity.length());
    let distance = ship.distance(goal);
    info!(
        "scénario : autopilote arrêté à {distance:.0} m de {target} (arrêt visé {stop:.0} m, {speed:.1} m/s)"
    );
    if distance <= stop + 1_000.0 && speed < 5.0 {
        Ok(true)
    } else {
        Err(format!(
            "désengagé loin de la cible : {distance:.0} m (arrêt visé {stop:.0} m), {speed:.1} m/s"
        ))
    }
}

/// `deliver_vorrh` : un contrat de transport Hexis → Vorrh, celui du bureau s'il en propose
/// un, sinon le premier transport offert, détourné vers Vorrh (le scénario ne dépend pas du
/// classement des offres de la semaine).
fn give_vorrh_contract(world: &mut World) -> Result<bool, String> {
    give_contract_to(world, "sys_vorrh")
}

/// Un contrat de transport d'Hexis vers ce système, accepté comme au bureau des contrats : une
/// offre du jour vers lui, à défaut le premier transport proposé, redirigé (scénarios, et
/// `SH_CONTRACT` pour les captures du repère de destination). `Ok(false)` : la partie n'est pas
/// encore prête.
fn give_contract_to(world: &mut World, to: &str) -> Result<bool, String> {
    let Some(today) = world
        .get_resource::<crate::clock::GameClock>()
        .map(|c| c.date)
    else {
        return Ok(false);
    };
    if !world.contains_resource::<crate::career::Career>() {
        return Ok(false);
    }
    let data = world.resource::<Data>().clone();
    let id = |s: &str| sh_core::Id::<sh_data::Place>::new(s).map_err(|e| e.to_string());
    let (hexis, dest) = (id("sys_hexis")?, id(to)?);
    if data.galaxy.system(&dest).is_none() {
        return Err(format!("système inconnu : {to}"));
    }
    let offers = world
        .get_resource::<crate::living::LivingWorld>()
        .map(|l| sh_sim::contract_offers(&data, &l.sim, &hexis, today))
        .ok_or("monde vivant absent")?;
    let contract = offers
        .iter()
        .find(|c| c.to == dest)
        .cloned()
        .or_else(|| {
            offers
                .iter()
                .find(|c| c.kind != sh_sim::ContractKind::Towing)
                .cloned()
                .map(|c| sh_sim::Contract {
                    id: format!("CT-sys_hexis-{to}-scenario-{}", today.day_index()),
                    to: dest.clone(),
                    deadline: today.add_days(60),
                    ..c
                })
        })
        .ok_or("aucun transport proposé à Hexis")?;
    info!("autopilote : contrat {} vers {to} accepté", contract.id);
    let out = world
        .resource_mut::<crate::career::Career>()
        .captain
        .accept(&data, contract)
        .map_err(|e| format!("{e:?}"))?;
    world.write_message(crate::career::CareerOutcomes(out));
    Ok(true)
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
