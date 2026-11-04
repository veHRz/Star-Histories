//! Tutoriel guidé des premières minutes (M8, `docs/refonte-visuelle.md` §6).
//!
//! Huit étapes mènent une nouvelle partie de l'orbite d'Hexis jusqu'à la première livraison et
//! la première sauvegarde : prendre les commandes, s'amarrer, descendre à quai, prendre un
//! contrat, ouvrir le journal, la fiche et le codex, partir, livrer, sauvegarder.
//!
//! - **Un fait observé fait avancer, jamais une minuterie** : chaque étape a ses vérifications
//!   ([`Check`]), cochées dès que le jeu les constate ([`Facts`]) et gardées une fois cochées.
//!   Le joueur peut donc les faire dans l'ordre qu'il veut, ou les avoir déjà faites.
//! - Un **panneau d'objectif** (en haut à droite) donne la consigne ; les touches y sont des
//!   jetons (`[KeyG]`) qui suivent la disposition du clavier et le remappage. Un **repère** à
//!   l'écran montre où aller quand c'est utile (balise, sas, bureau, poste de pilotage, hub),
//!   ramené au bord de l'écran, avec une flèche, quand la cible est hors champ.
//! - Il se coupe dans les options (« Tutoriel ») et se passe à tout moment ([`SkipTutorial`],
//!   deux appuis : un appui égaré ne le fait pas perdre). Son avancement est sauvegardé ; une
//!   sauvegarde d'avant M8 le charge comme terminé (on n'impose pas le début à une partie déjà
//!   avancée).
//! - Pendant qu'il tourne, les conseils de première fois qu'il remplace se taisent
//!   ([`REPLACED_TIPS`]) ; terminé, il les marque comme vus. Passé, il les laisse venir.
//!
//! La logique (étapes, vérifications, avancement) est pure et testée ; les systèmes ne font que
//! relever les faits et afficher. L'autopilote le coupe, sauf `SH_TUTORIAL=1` ;
//! `SH_TUTORIAL_STEP=<n>` démarre à l'étape n (captures du panneau).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_enhanced_input::prelude::Start;
use serde::{Deserialize, Serialize};
use sh_data::StructureKind;
use sh_data::dialogue::{Dialogue, Effect, Screen as DialogueScreen};
use sh_sim::{Captain, Contract, ContractKind, ContractState};

use crate::career::{Career, CargoPoint};
use crate::clock::HudCorner;
use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::input::SkipTutorial;
use crate::living::display_name;
use crate::options::UserSettings;
use crate::player::Player;
use crate::save::{GameMode, GameSaved, SaveKind};
use crate::sector::gltf::Interactable;
use crate::sector::streaming::CellStates;
use crate::sector::{ActiveSector, Docked, v3};
use crate::ship::ShipMotion;
use crate::space::SpaceRotation;
use crate::state::{AppState, ControlMode, ViewLevel};
use crate::tips::TipsSeen;
use crate::transition::{DockBeacon, Sequence};
use crate::ui::hud::Notice;
use crate::ui::terminal::{Screen, Terminal};
use crate::ui::theme::{self, Frame, UiFonts};
use crate::voyage::{Location, RoutePlan};

pub struct TutorialPlugin;

impl Plugin for TutorialPlugin {
    fn build(&self, app: &mut App) {
        // Une condition par système (AGENTS.md §4 : posée sur le groupe chaîné, elle ne serait
        // évaluée qu'une fois).
        app.init_resource::<SkipArm>()
            .add_observer(on_skip_key)
            .add_systems(
                Update,
                (
                    start_tutorial
                        .run_if(in_state(AppState::InGame))
                        .run_if(not(resource_exists::<Tutorial>)),
                    track
                        .run_if(in_state(AppState::InGame))
                        .run_if(resource_exists::<Tutorial>)
                        .run_if(resource_exists::<Career>),
                    render_panel.run_if(in_state(AppState::InGame)),
                    place_marker.run_if(in_state(AppState::InGame)),
                )
                    .chain(),
            )
            .add_systems(OnExit(AppState::InGame), |mut arm: ResMut<SkipArm>| {
                *arm = SkipArm::default();
            });
    }
}

// --- Logique pure ------------------------------------------------------------------------------

/// Les conseils de première fois (`tips`) que le tutoriel remplace. `raid` et `boarding`
/// restent : ces situations viennent plus tard.
pub const REPLACED_TIPS: [&str; 6] = ["helm", "dock", "port", "foot", "map", "contract"];

/// Une étape du tutoriel, dans l'ordre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Stage {
    /// Aux commandes en orbite : regarder, pousser, la croisière.
    Flight,
    /// Rejoindre la balise du spatioport et s'amarrer.
    Dock,
    /// Se lever, sortir par le sas, trouver le bureau des contrats.
    Office,
    /// Parler au guichetier, prendre un contrat de transport.
    Contract,
    /// Ouvrir le journal, la fiche, le codex.
    Screens,
    /// Revenir aux commandes, partir, ouvrir la carte, choisir la destination.
    Departure,
    /// Voyager, livrer en orbite, être payé.
    Delivery,
    /// Sauvegarder.
    Save,
}

impl Stage {
    pub const ALL: [Stage; 8] = [
        Stage::Flight,
        Stage::Dock,
        Stage::Office,
        Stage::Contract,
        Stage::Screens,
        Stage::Departure,
        Stage::Delivery,
        Stage::Save,
    ];

    /// Numéro affiché (1 à 8).
    pub fn number(self) -> usize {
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .map_or(0, |i| i + 1)
    }

    /// L'étape de ce numéro (1 à 8).
    pub fn from_number(n: usize) -> Option<Self> {
        n.checked_sub(1).and_then(|i| Self::ALL.get(i).copied())
    }

    pub fn next(self) -> Option<Self> {
        Self::from_number(self.number() + 1)
    }

    /// Clé de langue de l'étape (`tutorial.<clé>.title`, `.body`).
    pub fn key(self) -> &'static str {
        match self {
            Stage::Flight => "flight",
            Stage::Dock => "dock",
            Stage::Office => "office",
            Stage::Contract => "contract",
            Stage::Screens => "screens",
            Stage::Departure => "departure",
            Stage::Delivery => "delivery",
            Stage::Save => "save",
        }
    }

    /// Ce qu'il faut avoir fait pour franchir l'étape.
    pub fn checks(self) -> &'static [Check] {
        match self {
            Stage::Flight => &[Check::Look, Check::Thrust, Check::Cruise],
            Stage::Dock => &[Check::Dock],
            Stage::Office => &[Check::Stand, Check::Airlock, Check::Office],
            Stage::Contract => &[Check::Talk, Check::Contract],
            Stage::Screens => &[Check::Journal, Check::Sheet, Check::Codex],
            Stage::Departure => &[Check::Helm, Check::Undock, Check::Map, Check::Route],
            Stage::Delivery => &[Check::Arrive, Check::Deliver],
            Stage::Save => &[Check::Save],
        }
    }
}

/// Une vérification d'étape : un fait que le jeu constate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Check {
    Look,
    Thrust,
    Cruise,
    Dock,
    Stand,
    Airlock,
    Office,
    Talk,
    Contract,
    Journal,
    Sheet,
    Codex,
    Helm,
    Undock,
    Map,
    Route,
    Arrive,
    Deliver,
    Save,
}

impl Check {
    /// Clé de langue de la ligne de la liste (`tutorial.check.<clé>`).
    pub fn key(self) -> &'static str {
        match self {
            Check::Look => "look",
            Check::Thrust => "thrust",
            Check::Cruise => "cruise",
            Check::Dock => "dock",
            Check::Stand => "stand",
            Check::Airlock => "airlock",
            Check::Office => "office",
            Check::Talk => "talk",
            Check::Contract => "contract",
            Check::Journal => "journal",
            Check::Sheet => "sheet",
            Check::Codex => "codex",
            Check::Helm => "helm",
            Check::Undock => "undock",
            Check::Map => "map",
            Check::Route => "route",
            Check::Arrive => "arrive",
            Check::Deliver => "deliver",
            Check::Save => "save",
        }
    }

    /// Le fait est-il constaté ce frame ?
    pub fn holds(self, f: &Facts) -> bool {
        match self {
            Check::Look => f.looked,
            Check::Thrust => f.thrusting,
            Check::Cruise => f.cruising,
            Check::Dock => f.docked,
            Check::Stand => f.standing_docked,
            Check::Airlock => f.left_ship,
            Check::Office => f.at_office,
            Check::Talk => f.talking,
            Check::Contract => f.cargo_contract.is_some(),
            Check::Journal => f.journal,
            Check::Sheet => f.sheet,
            Check::Codex => f.codex,
            Check::Helm => f.at_helm,
            Check::Undock => f.undocked,
            Check::Map => f.map,
            Check::Route => f.route_to_target,
            Check::Arrive => f.arrived,
            Check::Deliver => f.settled,
            Check::Save => f.saved,
        }
    }
}

/// Ce que le jeu constate ce frame (relevé par [`Observed::facts`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    /// Le vaisseau a tourné d'assez d'un total, aux commandes.
    pub looked: bool,
    /// Le vaisseau file à bonne vitesse de manœuvre.
    pub thrusting: bool,
    /// Le vaisseau va plus vite que sa vitesse de manœuvre : la croisière.
    pub cruising: bool,
    pub docked: bool,
    /// À quai et debout.
    pub standing_docked: bool,
    /// À quai, dans une cellule du quartier (sorti du vaisseau).
    pub left_ship: bool,
    /// Près de la console du bureau des contrats.
    pub at_office: bool,
    /// Le terminal montre le guichetier ou ses contrats.
    pub talking: bool,
    /// Un contrat de livraison actif (transport ou ravitaillement), le plus récent.
    pub cargo_contract: Option<String>,
    pub journal: bool,
    pub sheet: bool,
    pub codex: bool,
    pub at_helm: bool,
    /// En vol, aux commandes, plus à quai.
    pub undocked: bool,
    pub map: bool,
    /// L'itinéraire choisi mène à la destination du contrat suivi.
    pub route_to_target: bool,
    /// En vol dans le système de destination, voyage fini.
    pub arrived: bool,
    /// Le contrat suivi n'est plus en cours (payé, d'ordinaire).
    pub settled: bool,
    /// Le joueur a sauvegardé (rapide, manuelle, ou Homme de fer).
    pub saved: bool,
}

/// Où en est le tutoriel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Progress {
    Running(Stage),
    Finished,
    Skipped,
}

/// L'avancement du tutoriel de la partie (sauvegardé).
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tutorial {
    pub progress: Progress,
    /// Les vérifications de l'étape en cours déjà constatées.
    #[serde(default)]
    pub ticked: Vec<Check>,
    /// Le contrat pris à l'étape du bureau : sa destination guide la suite.
    #[serde(default)]
    pub contract: Option<String>,
}

/// Ce qu'un relevé a changé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    Stage(Stage),
    Finished,
}

impl Tutorial {
    pub fn new(stage: Stage) -> Self {
        Self {
            progress: Progress::Running(stage),
            ticked: Vec::new(),
            contract: None,
        }
    }

    /// Un tutoriel déjà terminé : celui des sauvegardes d'avant M8.
    pub fn finished() -> Self {
        Self {
            progress: Progress::Finished,
            ticked: Vec::new(),
            contract: None,
        }
    }

    pub fn stage(&self) -> Option<Stage> {
        match self.progress {
            Progress::Running(stage) => Some(stage),
            Progress::Finished | Progress::Skipped => None,
        }
    }

    /// Numéro de l'étape en cours ; 9 une fois fini ou passé (vérifications de l'autopilote).
    pub fn number(&self) -> usize {
        self.stage().map_or(Stage::ALL.len() + 1, Stage::number)
    }

    pub fn is_ticked(&self, check: Check) -> bool {
        self.ticked.contains(&check)
    }

    /// Coche les vérifications constatées de l'étape en cours ; une étape dont tout est coché
    /// laisse place à la suivante (une seule par relevé : chaque étape s'affiche au moins un
    /// frame).
    pub fn observe(&mut self, facts: &Facts) -> Option<Advance> {
        let stage = self.stage()?;
        for check in stage.checks() {
            if !self.is_ticked(*check) && check.holds(facts) {
                self.ticked.push(*check);
                if *check == Check::Contract {
                    self.contract = facts.cargo_contract.clone();
                }
            }
        }
        if !stage.checks().iter().all(|c| self.is_ticked(*c)) {
            return None;
        }
        self.ticked.clear();
        match stage.next() {
            Some(next) => {
                self.progress = Progress::Running(next);
                Some(Advance::Stage(next))
            }
            None => {
                self.progress = Progress::Finished;
                Some(Advance::Finished)
            }
        }
    }

    pub fn skip(&mut self) {
        self.progress = Progress::Skipped;
        self.ticked.clear();
    }
}

/// Le tutoriel guide-t-il le joueur en ce moment ?
pub fn is_active(settings: &UserSettings, tutorial: Option<&Tutorial>) -> bool {
    settings.tutorial && tutorial.is_some_and(|t| t.stage().is_some())
}

/// Le contrat que suit le tutoriel : celui pris à l'étape du bureau, sinon le contrat de
/// livraison actif le plus récent (départ à une étape avancée, `SH_TUTORIAL_STEP`).
pub fn target_contract<'a>(tutorial: &Tutorial, captain: &'a Captain) -> Option<&'a Contract> {
    match &tutorial.contract {
        Some(id) => captain.contracts.iter().find(|c| &c.id == id),
        None => latest_cargo_contract(captain),
    }
}

/// Le contrat de livraison (transport ou ravitaillement, pas un remorquage) actif le plus
/// récent.
pub fn latest_cargo_contract(captain: &Captain) -> Option<&Contract> {
    captain
        .contracts
        .iter()
        .rev()
        .find(|c| c.state == ContractState::Active && c.kind != ContractKind::Towing)
}

/// Le dialogue mène-t-il aux contrats (le guichetier d'un bureau des contrats) ?
pub fn offers_contracts(dialogue: &Dialogue) -> bool {
    let opens = |effects: &[Effect]| {
        effects
            .iter()
            .any(|e| matches!(e, Effect::Open(DialogueScreen::Contracts)))
    };
    dialogue
        .nodes
        .values()
        .any(|n| opens(&n.effects) || n.choices.iter().any(|c| opens(&c.effects)))
}

/// Où poser un repère, en pixels de l'écran (`size`) : le point projeté s'il est à l'écran,
/// sinon le bord de l'écran dans la direction de la cible, avec la flèche qui y mène.
/// `local` : la cible dans le repère de la caméra (−Z devant, +Y en haut).
pub fn marker_spot(
    projected: Option<Vec2>,
    local: Vec3,
    size: Vec2,
    margin: f32,
) -> (Vec2, Option<char>) {
    let center = size / 2.0;
    let half = (center - Vec2::splat(margin)).max(Vec2::ONE);
    let in_front = local.z < 0.0;
    if in_front && let Some(p) = projected {
        let inside = (p - center).abs();
        if inside.x <= half.x && inside.y <= half.y {
            return (p, None);
        }
    }
    // Hors champ : la direction à l'écran (y vers le bas). Juste derrière : vers le bas.
    let direction = match (in_front, projected) {
        (true, Some(p)) => p - center,
        _ => Vec2::new(local.x, -local.y),
    };
    let direction = direction.try_normalize().unwrap_or(Vec2::Y);
    let scale = (half.x / direction.x.abs().max(1e-6)).min(half.y / direction.y.abs().max(1e-6));
    (center + direction * scale, Some(arrow(direction)))
}

/// La flèche la plus proche d'une direction de l'écran (y vers le bas).
fn arrow(direction: Vec2) -> char {
    const ARROWS: [char; 8] = ['→', '↘', '↓', '↙', '←', '↖', '↑', '↗'];
    let angle = direction.y.atan2(direction.x);
    let sector = (angle / std::f32::consts::FRAC_PI_4).round() as i32;
    ARROWS[sector.rem_euclid(8) as usize]
}

// --- Relevé des faits --------------------------------------------------------------------------

/// Rotation cumulée (degrés) au-delà de laquelle le joueur a « regardé » aux commandes.
const LOOK_DEGREES: f32 = 30.0;
/// Vitesse (m/s) au-delà de laquelle le joueur a « poussé ».
const THRUST_SPEED: f64 = 100.0;
/// La croisière : au-delà de cette part de la vitesse de manœuvre.
const CRUISE_FACTOR: f64 = 1.5;
/// Distance horizontale (m) à la console du bureau à laquelle on l'a « trouvé ».
const OFFICE_DISTANCE: f32 = 4.0;
/// Délai (s) pour confirmer le passage du tutoriel par un second appui.
const SKIP_CONFIRM_SECONDS: f32 = 5.0;

/// Ce qu'il faut lire du monde pour relever les faits et poser le repère.
#[derive(SystemParam)]
struct Observed<'w, 's> {
    data: Res<'w, Data>,
    mode: Res<'w, State<ControlMode>>,
    view: Option<Res<'w, State<ViewLevel>>>,
    docked: Option<Res<'w, Docked>>,
    sector: Option<Res<'w, ActiveSector>>,
    cells: Option<Res<'w, CellStates>>,
    terminal: Option<Res<'w, Terminal>>,
    route: Option<Res<'w, RoutePlan>>,
    location: Option<Res<'w, Location>>,
    sequence: Option<Res<'w, Sequence>>,
    ships: Query<'w, 's, (&'static SpaceRotation, &'static ShipMotion)>,
    player: Query<'w, 's, &'static GlobalTransform, With<Player>>,
    interactables: Query<'w, 's, (&'static Interactable, &'static GlobalTransform)>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
}

impl Observed<'_, '_> {
    fn view(&self) -> Option<ViewLevel> {
        self.view.as_ref().map(|v| *v.get())
    }

    fn piloting(&self) -> bool {
        *self.mode.get() == ControlMode::Piloting
    }

    /// En vol aux commandes, hors manœuvre automatique ou voyage.
    fn flying(&self) -> bool {
        self.view() == Some(ViewLevel::Global) && self.piloting() && self.sequence.is_none()
    }

    /// Le vaisseau du joueur en vol : orientation et mouvement.
    fn ship(&self) -> Option<(Quat, &ShipMotion)> {
        let root = self.sector.as_ref()?.root;
        self.ships.get(root).ok().map(|(r, m)| (r.0, m))
    }

    /// Vitesse de manœuvre du vaisseau du joueur (m/s).
    fn max_speed(&self) -> f64 {
        self.sector
            .as_ref()
            .and_then(|s| self.data.flight.ships.get(s.sector.id.as_str()))
            .map_or(250.0, |h| h.max_speed)
    }

    /// Une cellule du vaisseau amarré (placée dans le repère du quartier) ?
    fn is_ship_cell(&self, cell: &str) -> Option<bool> {
        let sector = self.sector.as_ref()?;
        sector
            .sector
            .cells
            .iter()
            .find(|c| c.id == cell)
            .map(|c| c.placement.is_some())
    }

    /// À quai, le joueur est-il dans le quartier (sorti par le sas) ?
    fn in_district(&self) -> bool {
        self.docked.is_some()
            && self
                .cells
                .as_ref()
                .and_then(|c| c.current.as_deref())
                .and_then(|cell| self.is_ship_cell(cell))
                == Some(false)
    }

    /// Le guichetier du bureau des contrats du quartier où l'on est amarré.
    fn office(&self) -> Option<&Dialogue> {
        let district = &self.docked.as_ref()?.district;
        self.data
            .dialogues
            .iter()
            .find(|d| &d.sector == district && offers_contracts(d))
    }

    /// La console de ce guichetier.
    fn office_console(&self) -> Option<Vec3> {
        let label = &self.office()?.console;
        self.interactables
            .iter()
            .find(|(i, _)| matches!(i, Interactable::Console { label: l } if l == label))
            .map(|(_, t)| t.translation())
    }

    /// Le siège du pilote.
    fn helm(&self) -> Option<Vec3> {
        self.interactables
            .iter()
            .find(|(i, _)| matches!(i, Interactable::Seat { role } if role == "pilot"))
            .map(|(_, t)| t.translation())
    }

    /// Le sas : le portail entre une cellule du vaisseau et une cellule du quartier.
    fn airlock(&self) -> Option<Vec3> {
        let sector = self.sector.as_ref()?;
        let portal = sector.sector.portals.iter().find(|p| {
            matches!(
                (self.is_ship_cell(&p.a), self.is_ship_cell(&p.b)),
                (Some(a), Some(b)) if a != b
            )
        })?;
        let root = self.transforms.get(sector.root).ok()?;
        Some(root.transform_point(v3(portal.center)))
    }

    fn player_position(&self) -> Option<Vec3> {
        self.player.single().ok().map(GlobalTransform::translation)
    }

    /// Relève les faits du frame. `turned` : rotation cumulée aux commandes (degrés).
    fn facts(&self, tutorial: &Tutorial, captain: &Captain, turned: f32, saved: bool) -> Facts {
        let view = self.view();
        let docked = self.docked.is_some();
        let speed = self
            .ship()
            .filter(|_| self.flying())
            .map_or(0.0, |(_, m)| m.velocity.length());
        let screen = self.terminal.as_ref().map(|t| &t.screen);
        let office = self.office().map(|d| d.id.as_str());
        let at_office = match (self.office_console(), self.player_position()) {
            (Some(console), Some(player)) => {
                docked
                    && Vec2::new(console.x - player.x, console.z - player.z).length()
                        < OFFICE_DISTANCE
            }
            _ => false,
        };
        let target = target_contract(tutorial, captain);
        let here = self.location.as_ref().map(|l| &l.system);
        Facts {
            looked: turned >= LOOK_DEGREES,
            thrusting: speed >= THRUST_SPEED,
            cruising: speed >= self.max_speed() * CRUISE_FACTOR,
            docked: docked && view == Some(ViewLevel::Sector),
            standing_docked: docked && *self.mode.get() == ControlMode::OnFoot,
            left_ship: self.in_district(),
            at_office,
            talking: match screen {
                Some(Screen::Dialogue { id, .. }) => Some(id.as_str()) == office,
                Some(Screen::Contracts) => true,
                _ => false,
            },
            cargo_contract: latest_cargo_contract(captain).map(|c| c.id.clone()),
            journal: matches!(screen, Some(Screen::Journal)),
            sheet: matches!(screen, Some(Screen::Sheet)),
            codex: matches!(screen, Some(Screen::Codex | Screen::CodexEntry(_))),
            at_helm: self.piloting(),
            undocked: !docked && self.flying(),
            map: view == Some(ViewLevel::Galaxy),
            route_to_target: target
                .is_some_and(|c| self.route.as_ref().is_some_and(|r| r.dest == c.to)),
            arrived: target.is_some_and(|c| here == Some(&c.to)) && self.flying(),
            settled: target.is_some_and(|c| c.state != ContractState::Active),
            saved,
        }
    }
}

/// Une nouvelle partie commence au début du tutoriel (une sauvegarde chargée apporte le sien).
fn start_tutorial(mut commands: Commands) {
    let stage = crate::dev::autopilot_active()
        .then(|| std::env::var("SH_TUTORIAL_STEP").ok())
        .flatten()
        .and_then(|v| v.trim().parse().ok())
        .and_then(Stage::from_number)
        .unwrap_or(Stage::Flight);
    commands.insert_resource(Tutorial::new(stage));
}

/// Rotation cumulée du vaisseau aux commandes, pour « regarder ».
#[derive(Default)]
struct Turning {
    last: Option<Quat>,
    degrees: f32,
}

#[allow(clippy::too_many_arguments)]
fn track(
    settings: Res<UserSettings>,
    texts: Res<Texts>,
    career: Res<Career>,
    observed: Observed,
    mut tutorial: ResMut<Tutorial>,
    mut saves: MessageReader<GameSaved>,
    mut notices: MessageWriter<Notice>,
    mut seen: ResMut<TipsSeen>,
    mut turning: Local<Turning>,
) {
    // Les sauvegardes automatiques ne comptent pas : on apprend à sauvegarder soi-même (sauf
    // en Homme de fer, où il n'y en a pas d'autres).
    // `count` lit tout : un message laissé de côté reviendrait au frame suivant.
    let saved = saves.read().filter(|s| s.0 != SaveKind::Auto).count() > 0;
    if !is_active(&settings, Some(&tutorial)) {
        *turning = Turning::default();
        return;
    }
    if tutorial.stage() == Some(Stage::Flight)
        && observed.flying()
        && let Some((rotation, _)) = observed.ship()
    {
        if let Some(last) = turning.last {
            turning.degrees += last.angle_between(rotation).to_degrees();
        }
        turning.last = Some(rotation);
    } else {
        turning.last = None;
    }
    let facts = observed.facts(&tutorial, &career.captain, turning.degrees, saved);
    // Sur une copie : la ressource n'est marquée modifiée que si quelque chose a changé.
    let mut next = tutorial.clone();
    let advance = next.observe(&facts);
    if next != *tutorial {
        *tutorial = next;
    }
    match advance {
        Some(Advance::Stage(stage)) => {
            info!("tutoriel : étape {} ({})", stage.number(), stage.key());
        }
        Some(Advance::Finished) => {
            info!("tutoriel : terminé");
            notices.write(Notice(texts.get("tutorial.finished")));
            // Appris : les conseils qu'il remplaçait n'ont plus à venir.
            for tip in REPLACED_TIPS {
                if !seen.0.contains(tip) {
                    seen.0.insert(tip.to_owned());
                }
            }
        }
        None => {}
    }
}

// --- Passer le tutoriel ------------------------------------------------------------------------

/// Premier appui sur la touche « passer » : un second, dans le délai, confirme.
#[derive(Resource, Default)]
struct SkipArm {
    armed_at: Option<f32>,
}

impl SkipArm {
    fn armed(&self, now: f32) -> bool {
        self.armed_at
            .is_some_and(|t| (0.0..SKIP_CONFIRM_SECONDS).contains(&(now - t)))
    }
}

fn on_skip_key(
    _: On<Start<SkipTutorial>>,
    time: Res<Time<Real>>,
    settings: Res<UserSettings>,
    texts: Option<Res<Texts>>,
    tutorial: Option<ResMut<Tutorial>>,
    mut arm: ResMut<SkipArm>,
    mut notices: MessageWriter<Notice>,
) {
    let Some(mut tutorial) = tutorial.filter(|t| is_active(&settings, Some(t))) else {
        return;
    };
    let now = time.elapsed_secs();
    if !arm.armed(now) {
        arm.armed_at = Some(now);
        return;
    }
    arm.armed_at = None;
    tutorial.skip();
    info!("tutoriel : passé par le joueur");
    if let Some(texts) = texts {
        notices.write(Notice(texts.get("tutorial.skipped")));
    }
}

// --- Panneau d'objectif ------------------------------------------------------------------------

#[derive(Component)]
struct TutorialPanel;

/// Largeur du panneau d'objectif : celle de la colonne du coin haut droit.
const PANEL_WIDTH: f32 = crate::clock::HUD_CORNER_WIDTH;

/// Ce que montre le panneau (comparé d'un frame à l'autre : il n'est refait qu'au changement).
#[derive(Clone, Debug, PartialEq)]
struct PanelContent {
    header: String,
    title: String,
    body: String,
    checks: Vec<(bool, String)>,
    footer: String,
}

/// Le nom d'un lieu des données (système, astre).
fn place_name(data: &Data, id: &sh_core::Id<sh_data::Place>) -> String {
    data.galaxy
        .place_name(id)
        .map_or_else(|| display_name(data, id.as_str()), str::to_owned)
}

/// La balise d'approche du port du système (celle de la mise en scène).
fn beacon_name(texts: &Texts, data: &Data, location: Option<&Location>) -> String {
    location
        .and_then(|l| data.scene(&l.system))
        .and_then(|s| s.docks.first())
        .map_or_else(String::new, |d| texts.get(&d.name_key))
}

#[allow(clippy::too_many_arguments)]
fn panel_content(
    texts: &Texts,
    data: &Data,
    tutorial: &Tutorial,
    stage: Stage,
    captain: Option<&Captain>,
    location: Option<&Location>,
    ironman: bool,
    armed: bool,
) -> PanelContent {
    let place = match stage {
        Stage::Departure | Stage::Delivery => captain
            .and_then(|c| target_contract(tutorial, c))
            .map(|c| place_name(data, &c.to)),
        _ => location.map(|l| place_name(data, &l.system)),
    }
    .unwrap_or_default();
    let beacon = beacon_name(texts, data, location);
    let args = [("place", place.as_str()), ("beacon", beacon.as_str())];
    let body_key = if stage == Stage::Save && ironman {
        "tutorial.save.body_ironman".to_owned()
    } else {
        format!("tutorial.{}.body", stage.key())
    };
    PanelContent {
        header: texts.format(
            "tutorial.header",
            &[
                ("n", &stage.number().to_string()),
                ("total", &Stage::ALL.len().to_string()),
            ],
        ),
        title: texts.get(&format!("tutorial.{}.title", stage.key())),
        body: texts.format(&body_key, &args),
        checks: stage
            .checks()
            .iter()
            .map(|c| {
                (
                    tutorial.is_ticked(*c),
                    texts.format(&format!("tutorial.check.{}", c.key()), &args),
                )
            })
            .collect(),
        footer: texts.get(if armed {
            "tutorial.skip_confirm"
        } else {
            "tutorial.skip_hint"
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_panel(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    data: Res<Data>,
    settings: Res<UserSettings>,
    time: Res<Time<Real>>,
    arm: Res<SkipArm>,
    (tutorial, career, location, mode): (
        Option<Res<Tutorial>>,
        Option<Res<Career>>,
        Option<Res<Location>>,
        Option<Res<GameMode>>,
    ),
    panels: Query<Entity, With<TutorialPanel>>,
    corners: Query<Entity, With<HudCorner>>,
    mut shown: Local<Option<PanelContent>>,
) {
    let stage = tutorial
        .as_deref()
        .filter(|t| is_active(&settings, Some(t)))
        .and_then(Tutorial::stage);
    let (Some(tutorial), Some(stage)) = (tutorial.as_deref(), stage) else {
        for panel in &panels {
            commands.entity(panel).despawn();
        }
        *shown = None;
        return;
    };
    let content = panel_content(
        &texts,
        &data,
        tutorial,
        stage,
        career.as_deref().map(|c| &c.captain),
        location.as_deref(),
        mode.is_some_and(|m| m.ironman),
        arm.armed(time.elapsed_secs()),
    );
    if shown.as_ref() == Some(&content) && !panels.is_empty() {
        return;
    }
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    // Panneau d'appareil (coupe ambre, rivets), rangé dans la colonne du coin haut droit, sous
    // la date et le fil des nouvelles (`clock::HudCorner`) : il descend quand les nouvelles
    // s'allongent, remonte sur la carte où elles se cachent, et ne les couvre jamais.
    let mut node = Node {
        width: px(PANEL_WIDTH),
        flex_direction: FlexDirection::Column,
        // Retrait : la coupe ambre en haut à gauche et les rivets des coins restent dégagés.
        padding: UiRect::new(px(22), px(20), px(16), px(16)),
        row_gap: px(5),
        ..default()
    };
    let corner = corners.iter().next();
    if corner.is_none() {
        node.position_type = PositionType::Absolute;
        node.top = px(76);
        node.right = px(20);
    }
    let panel = commands
        .spawn((
            Name::new("Tutoriel"),
            TutorialPanel,
            DespawnOnExit(AppState::InGame),
            GlobalZIndex(4),
            Pickable::IGNORE,
            node,
            fonts.frame(Frame::Panel),
        ))
        .id();
    if let Some(corner) = corner {
        commands.entity(panel).insert(ChildOf(corner));
    }
    // Le panneau ne retient jamais un clic (la carte galactique est cliquable dessous).
    let line = |text: String, font: TextFont, color: Color| {
        (
            ChildOf(panel),
            Text::new(text),
            font,
            TextColor(color),
            Pickable::IGNORE,
        )
    };
    // En-tête : l'étape en capitales ambre, les voyants de l'appareil.
    commands.spawn((
        ChildOf(panel),
        Pickable::IGNORE,
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(12),
            ..default()
        },
        children![
            (
                theme::heading(&fonts, &content.header, 13.0, theme::ACCENT),
                Pickable::IGNORE,
            ),
            (theme::status_lights(), Pickable::IGNORE),
        ],
    ));
    commands.spawn(line(content.title.clone(), fonts.title(19.0), theme::TEXT));
    commands.spawn((
        ChildOf(panel),
        Pickable::IGNORE,
        theme::rule(percent(100), UiRect::new(px(0), px(0), px(1), px(5))),
    ));
    commands.spawn(line(content.body.clone(), fonts.body(15.0), theme::TEXT));
    for (done, text) in &content.checks {
        // Une case cochée s'efface un peu : l'œil va à ce qui reste à faire.
        let (mark, color, mark_color) = if *done {
            ('✓', theme::TEXT_DIM, theme::SCREEN)
        } else {
            ('○', theme::TEXT, theme::ACCENT)
        };
        commands.spawn((
            ChildOf(panel),
            Pickable::IGNORE,
            Text::new(format!("{mark}  ")),
            fonts.body(15.0),
            TextColor(mark_color),
            children![(
                TextSpan::new(text.clone()),
                fonts.body(15.0),
                TextColor(color)
            )],
        ));
    }
    commands.spawn((
        ChildOf(panel),
        Pickable::IGNORE,
        Node {
            width: percent(100),
            height: px(1),
            margin: UiRect::top(px(4)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(theme::RULE),
    ));
    commands.spawn(line(
        content.footer.clone(),
        fonts.body(13.0),
        theme::TEXT_DIM,
    ));
    *shown = Some(content);
}

// --- Repère ------------------------------------------------------------------------------------

#[derive(Component)]
struct TutorialMarker;

/// Largeur de l'étiquette du repère, centrée sur la cible (px).
const MARKER_WIDTH: f32 = 280.0;
/// Marge du bord de l'écran pour un repère hors champ (px).
const MARKER_MARGIN: f32 = 48.0;

/// La cible du repère de l'étape, et son nom.
fn marker_target(
    texts: &Texts,
    observed: &Observed,
    stage: Stage,
    target: Option<&Contract>,
    beacons: &Query<(&DockBeacon, &Name, &GlobalTransform)>,
    points: &Query<(&CargoPoint, &Name, &GlobalTransform)>,
) -> Option<(Vec3, String)> {
    let view = observed.view();
    let docked = observed.docked.is_some();
    match stage {
        Stage::Dock if view == Some(ViewLevel::Global) => beacons
            .iter()
            .next()
            .map(|(_, name, t)| (t.translation(), name.as_str().to_owned())),
        Stage::Office if docked && !observed.in_district() => observed
            .airlock()
            .map(|p| (p, texts.get("tutorial.marker.airlock"))),
        Stage::Office | Stage::Contract if docked => observed
            .office_console()
            .map(|p| (p, texts.get("tutorial.marker.office"))),
        Stage::Departure if docked && !observed.piloting() => observed
            .helm()
            .map(|p| (p, texts.get("tutorial.marker.helm"))),
        Stage::Delivery
            if view == Some(ViewLevel::Global)
                && target.is_some_and(|c| {
                    observed.location.as_ref().map(|l| &l.system) == Some(&c.to)
                }) =>
        {
            // Le hub de Portails d'abord : on en sort à l'arrivée par Portail.
            points
                .iter()
                .min_by_key(|(p, _, _)| p.0 != StructureKind::PortalHub)
                .map(|(_, name, t)| (t.translation(), name.as_str().to_owned()))
        }
        _ => None,
    }
}

/// Une distance pour l'interface : en mètres de près, en kilomètres au-delà.
fn distance_text(texts: &Texts, meters: f32) -> String {
    if meters < 1000.0 {
        texts.format(
            "tutorial.marker.meters",
            &[("distance", &texts.number(f64::from(meters), 0))],
        )
    } else {
        let km = f64::from(meters) / 1000.0;
        let decimals = if km < 100.0 { 1 } else { 0 };
        texts.format(
            "tutorial.marker.kilometers",
            &[("distance", &texts.number(km, decimals))],
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn place_marker(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    settings: Res<UserSettings>,
    (tutorial, career): (Option<Res<Tutorial>>, Option<Res<Career>>),
    observed: Observed,
    beacons: Query<(&DockBeacon, &Name, &GlobalTransform)>,
    points: Query<(&CargoPoint, &Name, &GlobalTransform)>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut markers: Query<(Entity, &mut Node, &mut Text, &mut Visibility), With<TutorialMarker>>,
) {
    let stage = tutorial
        .as_deref()
        .filter(|t| is_active(&settings, Some(t)))
        .and_then(Tutorial::stage);
    let target = match (stage, tutorial.as_deref()) {
        (Some(stage), Some(tutorial)) => {
            let contract = career
                .as_deref()
                .and_then(|c| target_contract(tutorial, &c.captain));
            marker_target(&texts, &observed, stage, contract, &beacons, &points)
        }
        _ => None,
    };
    // Sous un écran (terminal, carte), le repère n'a rien à montrer.
    let hidden = observed.terminal.is_some() || observed.view() == Some(ViewLevel::Galaxy);
    let shot = match (target.filter(|_| !hidden), camera.single()) {
        (Some((point, name)), Ok((camera, eye))) => {
            // Avec l'échelle de rendu, la caméra dessine dans une image plus petite que la
            // fenêtre (même conversion que `views::place_labels`).
            let to_window = camera
                .logical_viewport_size()
                .map_or(Vec2::ONE, |size| window.size() / size.max(Vec2::ONE));
            let projected = camera
                .world_to_viewport(eye, point)
                .ok()
                .map(|p| p * to_window);
            let local = eye.affine().inverse().transform_point3(point);
            let (spot, arrow) = marker_spot(projected, local, window.size(), MARKER_MARGIN);
            let glyph = arrow.unwrap_or('◆');
            let distance = distance_text(&texts, eye.translation().distance(point));
            let label = texts.format(
                "tutorial.marker.label",
                &[("name", &name), ("distance", &distance)],
            );
            Some((spot, format!("{glyph}\n{label}")))
        }
        _ => None,
    };
    let Some((spot, text)) = shot else {
        for (_, _, _, mut visibility) in &mut markers {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };
    let left =
        (spot.x - MARKER_WIDTH / 2.0).clamp(8.0, (window.width() - MARKER_WIDTH - 8.0).max(8.0));
    let top = (spot.y - 10.0).clamp(8.0, (window.height() - 48.0).max(8.0));
    match markers.single_mut() {
        Ok((_, mut node, mut label, mut visibility)) => {
            node.left = px(left);
            node.top = px(top);
            if label.0 != text {
                label.0 = text;
            }
            if *visibility != Visibility::Inherited {
                *visibility = Visibility::Inherited;
            }
        }
        Err(_) => {
            for (entity, ..) in &markers {
                commands.entity(entity).despawn();
            }
            commands.spawn((
                Name::new("Repère du tutoriel"),
                TutorialMarker,
                DespawnOnExit(AppState::InGame),
                GlobalZIndex(3),
                Pickable::IGNORE,
                Text::new(text),
                fonts.title(15.0),
                TextColor(theme::ACCENT),
                // Ombre courte : l'ambre reste lisible sur une coque claire ou un écran allumé.
                theme::label_shadow(),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    top: px(top),
                    width: px(MARKER_WIDTH),
                    ..default()
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les faits qui franchissent une étape entière.
    fn all_of(stage: Stage) -> Facts {
        let mut f = Facts::default();
        for check in stage.checks() {
            match check {
                Check::Look => f.looked = true,
                Check::Thrust => f.thrusting = true,
                Check::Cruise => f.cruising = true,
                Check::Dock => f.docked = true,
                Check::Stand => f.standing_docked = true,
                Check::Airlock => f.left_ship = true,
                Check::Office => f.at_office = true,
                Check::Talk => f.talking = true,
                Check::Contract => f.cargo_contract = Some("CT-1".into()),
                Check::Journal => f.journal = true,
                Check::Sheet => f.sheet = true,
                Check::Codex => f.codex = true,
                Check::Helm => f.at_helm = true,
                Check::Undock => f.undocked = true,
                Check::Map => f.map = true,
                Check::Route => f.route_to_target = true,
                Check::Arrive => f.arrived = true,
                Check::Deliver => f.settled = true,
                Check::Save => f.saved = true,
            }
        }
        f
    }

    #[test]
    fn huit_etapes_numerotees() {
        assert_eq!(Stage::ALL.len(), 8);
        for (i, stage) in Stage::ALL.iter().enumerate() {
            assert_eq!(stage.number(), i + 1);
            assert_eq!(Stage::from_number(i + 1), Some(*stage));
            assert!(!stage.checks().is_empty());
        }
        assert_eq!(Stage::from_number(0), None);
        assert_eq!(Stage::from_number(9), None);
        assert_eq!(Stage::Save.next(), None);
    }

    #[test]
    fn rien_n_avance_sans_fait_observe() {
        let mut t = Tutorial::new(Stage::Flight);
        for _ in 0..100 {
            assert_eq!(t.observe(&Facts::default()), None);
        }
        assert_eq!(t.stage(), Some(Stage::Flight));
    }

    #[test]
    fn les_cases_cochees_restent_cochees() {
        let mut t = Tutorial::new(Stage::Flight);
        let looked = Facts {
            looked: true,
            ..default()
        };
        assert_eq!(t.observe(&looked), None);
        assert!(t.is_ticked(Check::Look));
        // Le joueur ne tourne plus, mais pousse puis passe en croisière : l'étape est franchie.
        assert_eq!(
            t.observe(&Facts {
                thrusting: true,
                ..default()
            }),
            None
        );
        assert_eq!(
            t.observe(&Facts {
                cruising: true,
                ..default()
            }),
            Some(Advance::Stage(Stage::Dock))
        );
        assert!(t.ticked.is_empty());
    }

    #[test]
    fn un_fait_d_une_autre_etape_ne_compte_pas() {
        let mut t = Tutorial::new(Stage::Flight);
        // Amarré pendant l'étape du vol : rien n'est retenu pour l'étape d'amarrage.
        t.observe(&Facts {
            docked: true,
            ..default()
        });
        assert!(!t.is_ticked(Check::Dock));
        t.observe(&all_of(Stage::Flight));
        assert_eq!(t.stage(), Some(Stage::Dock));
        assert!(!t.is_ticked(Check::Dock));
    }

    #[test]
    fn tout_le_parcours_jusqu_a_la_fin() {
        let mut t = Tutorial::new(Stage::Flight);
        let mut seen = Vec::new();
        for stage in Stage::ALL {
            assert_eq!(t.stage(), Some(stage));
            seen.push(t.observe(&all_of(stage)));
        }
        assert_eq!(seen.last(), Some(&Some(Advance::Finished)));
        assert_eq!(t.progress, Progress::Finished);
        assert_eq!(t.number(), 9);
        // Une étape par relevé : la 2e n'est pas franchie au même frame que la 1re.
        assert_eq!(seen[0], Some(Advance::Stage(Stage::Dock)));
        // Le contrat pris est retenu pour la suite.
        assert_eq!(t.contract.as_deref(), Some("CT-1"));
        assert_eq!(t.observe(&all_of(Stage::Save)), None);
    }

    #[test]
    fn passer_le_tutoriel() {
        let mut t = Tutorial::new(Stage::Office);
        t.observe(&Facts {
            standing_docked: true,
            ..default()
        });
        t.skip();
        assert_eq!(t.progress, Progress::Skipped);
        assert_eq!(t.stage(), None);
        assert_eq!(t.observe(&all_of(Stage::Office)), None);
    }

    #[test]
    fn actif_seulement_si_l_option_le_veut() {
        let on = UserSettings::default();
        assert!(on.tutorial, "activé par défaut");
        let off = UserSettings {
            tutorial: false,
            ..default()
        };
        let running = Tutorial::new(Stage::Dock);
        assert!(is_active(&on, Some(&running)));
        assert!(!is_active(&off, Some(&running)));
        assert!(!is_active(&on, None));
        assert!(!is_active(&on, Some(&Tutorial::finished())));
    }

    #[test]
    fn avancement_en_ron() {
        let mut t = Tutorial::new(Stage::Contract);
        t.observe(&Facts {
            talking: true,
            ..default()
        });
        let text = sh_data::ron_options().to_string(&t).expect("sérialisable");
        let back: Tutorial = sh_data::ron_options().from_str(&text).expect("relisible");
        assert_eq!(back, t);
        // Champs facultatifs absents : relisible quand même.
        let minimal: Tutorial = sh_data::ron_options()
            .from_str("(progress: Running(Delivery))")
            .expect("relisible");
        assert_eq!(minimal.stage(), Some(Stage::Delivery));
    }

    #[test]
    fn repere_a_l_ecran_ou_au_bord() {
        let size = Vec2::new(1600.0, 900.0);
        // Devant et à l'écran : sur la cible, sans flèche.
        let (p, arrow) = marker_spot(Some(Vec2::new(400.0, 300.0)), -Vec3::Z, size, 48.0);
        assert_eq!((p, arrow), (Vec2::new(400.0, 300.0), None));
        // Devant mais hors champ à droite : au bord droit, flèche vers la droite.
        let (p, arrow) = marker_spot(Some(Vec2::new(5000.0, 450.0)), -Vec3::Z, size, 48.0);
        assert_eq!(arrow, Some('→'));
        assert!((p.x - (1600.0 - 48.0)).abs() < 0.01 && (p.y - 450.0).abs() < 0.01);
        // Derrière, à gauche : au bord gauche.
        let (p, arrow) = marker_spot(None, Vec3::new(-5.0, 0.0, 10.0), size, 48.0);
        assert_eq!(arrow, Some('←'));
        assert!((p.x - 48.0).abs() < 0.01);
        // Juste derrière : en bas.
        let (_, arrow) = marker_spot(None, Vec3::new(0.0, 0.0, 10.0), size, 48.0);
        assert_eq!(arrow, Some('↓'));
        // Au-dessus, hors champ : flèche vers le haut.
        let (_, arrow) = marker_spot(None, Vec3::new(0.0, 5.0, 1.0), size, 48.0);
        assert_eq!(arrow, Some('↑'));
    }
}
