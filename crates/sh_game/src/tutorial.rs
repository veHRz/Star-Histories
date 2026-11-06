//! Tutoriel guidé des premières minutes (M8, `docs/refonte-visuelle.md` §6 ; M9,
//! `docs/m9-retours.md` §8.2 ; M11, `docs/m11.md` §2).
//!
//! Onze étapes mènent une nouvelle partie de la chambre du vaisseau jusqu'à la première livraison
//! et la première sauvegarde : la chambre (un casier, la tablette, l'inventaire), le tour du
//! vaisseau salle par salle avec le rôle de chacune, puis prendre les commandes, s'amarrer,
//! descendre à quai (hangar, hall, bureau), prendre un contrat auprès du guichetier, passer au
//! guichet de la Banque (M11 : la dette du départ, ses échéances, ce que coûte un retard ; ouvrir
//! son compte suffit, payer n'est pas exigé), ouvrir le journal, l'écran du personnage et le
//! codex, partir, livrer, sauvegarder.
//!
//! - **Un fait observé fait avancer, jamais une minuterie** : chaque étape a ses vérifications
//!   ([`Check`]), cochées dès que le jeu les constate ([`Facts`]) et gardées une fois cochées.
//!   Le joueur peut donc les faire dans l'ordre qu'il veut, ou les avoir déjà faites.
//! - **Les salles se reconnaissent à leur usage**, pas à un identifiant de secteur : la clé de
//!   nom d'une cellule (`room.cockpit`, `room.contracts_office`, `room.bank`…,
//!   `sh_data::Cell::name_key`) dit ce qu'elle est, et `sector::aboard_cell` si le joueur est
//!   dans son propre vaisseau. Un autre intérieur type ou un autre spatioport aux salles de même
//!   usage suit le même tutoriel.
//! - Un **panneau d'objectif** (en haut à droite) donne la consigne ; les touches y sont des
//!   jetons (`[KeyG]`) qui suivent la disposition du clavier et le remappage. Un **repère** à
//!   l'écran montre où aller quand c'est utile (casier, tablette, balise, poste de pilotage,
//!   guichetier, employé de la Banque, hub), ramené au bord de l'écran, avec une flèche, quand la
//!   cible est hors champ. Vers une salle, il montre la prochaine porte du chemin le plus court
//!   ([`path`]).
//! - Il se coupe dans les options (« Tutoriel ») et se passe à tout moment ([`SkipTutorial`],
//!   deux appuis : un appui égaré ne le fait pas perdre). Son avancement est sauvegardé par le
//!   **nom** de l'étape et des vérifications, jamais par leur numéro : une sauvegarde de M8 à M10
//!   prise en plein tutoriel reprend à la même étape malgré les étapes ajoutées (une partie
//!   sauvegardée au contrat passe ensuite par la Banque ; une partie déjà plus loin ne revient
//!   pas en arrière) ; une sauvegarde d'avant M8 le charge comme terminé (on n'impose pas le début
//!   à une partie déjà avancée).
//! - Pendant qu'il tourne, les conseils de première fois qu'il remplace se taisent
//!   ([`REPLACED_TIPS`]) ; terminé, il les marque comme vus. Passé, il les laisse venir.
//!
//! La logique (étapes, vérifications, avancement, chemin du repère) est pure et testée ; les
//! systèmes ne font que relever les faits et afficher. L'autopilote le coupe, sauf
//! `SH_TUTORIAL=1` ; `SH_TUTORIAL_STEP=<n>` démarre à l'étape n, de 1 (la chambre) à 11 (la
//! sauvegarde), ou terminé avec 12 (captures du panneau, [`Tutorial::at_number`]).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_enhanced_input::prelude::Start;
use serde::{Deserialize, Serialize};
use sh_core::Credits;
use sh_data::StructureKind;
use sh_data::dialogue::{Dialogue, Effect, Screen as DialogueScreen};
use sh_sim::{Captain, Contract, ContractKind, ContractState};

use crate::career::{Career, CargoPoint, faction_name, money};
use crate::clock::HudCorner;
use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::input::SkipTutorial;
use crate::items::{CharacterTab, PickedUp};
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
use crate::ui::character::{CharacterScreen, ScreenView};
use crate::ui::hud::Notice;
use crate::ui::terminal::{Screen, Terminal};
use crate::ui::theme::{self, Frame, UiFonts};
use crate::views::set_px_if_changed;
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

/// Une étape du tutoriel, dans l'ordre. Sauvegardée par son nom : on peut en ajouter, en
/// insérer (M9 : la chambre et le tour du vaisseau, devant ; M11 : la Banque, après le contrat)
/// sans changer celle d'une sauvegarde.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Stage {
    /// Dans la chambre : ouvrir un casier, ramasser la tablette, ouvrir l'inventaire.
    Cabin,
    /// Le tour du vaisseau : chaque salle, son nom et son rôle, jusqu'au cockpit.
    Tour,
    /// S'asseoir aux commandes, en orbite : regarder, pousser, la croisière.
    Flight,
    /// Rejoindre la balise du spatioport et s'amarrer.
    Dock,
    /// Se lever, sortir par le sas dans le hangar, traverser le hall jusqu'au bureau des
    /// contrats.
    Office,
    /// Parler au guichetier, prendre un contrat de transport.
    Contract,
    /// Au guichet de la Banque : la dette du départ (l'hypothèque du vaisseau), ses échéances et
    /// ce que coûte un retard ; parler à l'employé, ouvrir son compte. Payer n'est pas exigé : le
    /// joueur n'en a peut-être pas encore les moyens (M11).
    Bank,
    /// Ouvrir le journal, l'écran du personnage, le codex.
    Screens,
    /// Revenir aux commandes, partir, ouvrir la carte, choisir la destination.
    Departure,
    /// Voyager, livrer en orbite, être payé.
    Delivery,
    /// Sauvegarder.
    Save,
}

impl Stage {
    pub const ALL: [Stage; 11] = [
        Stage::Cabin,
        Stage::Tour,
        Stage::Flight,
        Stage::Dock,
        Stage::Office,
        Stage::Contract,
        Stage::Bank,
        Stage::Screens,
        Stage::Departure,
        Stage::Delivery,
        Stage::Save,
    ];

    /// Numéro affiché (1 à 11). Jamais sauvegardé : il change quand on ajoute une étape.
    pub fn number(self) -> usize {
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .map_or(0, |i| i + 1)
    }

    /// L'étape de ce numéro (1 à 11).
    pub fn from_number(n: usize) -> Option<Self> {
        n.checked_sub(1).and_then(|i| Self::ALL.get(i).copied())
    }

    pub fn next(self) -> Option<Self> {
        Self::from_number(self.number() + 1)
    }

    /// Clé de langue de l'étape (`tutorial.<clé>.title`, `.body`).
    pub fn key(self) -> &'static str {
        match self {
            Stage::Cabin => "cabin",
            Stage::Tour => "tour",
            Stage::Flight => "flight",
            Stage::Dock => "dock",
            Stage::Office => "office",
            Stage::Contract => "contract",
            Stage::Bank => "bank",
            Stage::Screens => "screens",
            Stage::Departure => "departure",
            Stage::Delivery => "delivery",
            Stage::Save => "save",
        }
    }

    /// Ce qu'il faut avoir fait pour franchir l'étape.
    pub fn checks(self) -> &'static [Check] {
        match self {
            Stage::Cabin => &[Check::Locker, Check::Pickup, Check::Inventory],
            // Dans l'ordre du repère ([`TOUR`]).
            Stage::Tour => &[
                Check::Visit(Room::Quarters),
                Check::Visit(Room::Mess),
                Check::Visit(Room::Airlock),
                Check::Visit(Room::Hold),
                Check::Visit(Room::Engine),
                Check::Visit(Room::Cockpit),
            ],
            Stage::Flight => &[Check::Helm, Check::Look, Check::Thrust, Check::Cruise],
            Stage::Dock => &[Check::Dock],
            Stage::Office => &[Check::Stand, Check::Hangar, Check::Hall, Check::Office],
            Stage::Contract => &[Check::Talk, Check::Contract],
            Stage::Bank => &[Check::Bank, Check::Teller, Check::Account],
            Stage::Screens => &[Check::Journal, Check::Sheet, Check::Codex],
            Stage::Departure => &[Check::Helm, Check::Undock, Check::Map, Check::Route],
            Stage::Delivery => &[Check::Arrive, Check::Deliver],
            Stage::Save => &[Check::Save],
        }
    }
}

/// Une salle du tour du vaisseau, reconnue à l'usage de sa cellule (clé de nom `room.*`, la même
/// pour tout intérieur type : `lang/<code>/rooms.ron`). La chambre n'en est pas : on y commence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Room {
    Cockpit,
    Airlock,
    Hold,
    Engine,
    Mess,
    Quarters,
}

/// L'ordre du tour, celui que suit le repère : de la chambre, à l'avant, les quartiers et le
/// carré qui donnent sur le même couloir, puis vers l'arrière le sas, la soute et les machines,
/// enfin le cockpit, où l'étape suivante fait prendre les commandes.
pub const TOUR: [Room; 6] = [
    Room::Quarters,
    Room::Mess,
    Room::Airlock,
    Room::Hold,
    Room::Engine,
    Room::Cockpit,
];

impl Room {
    /// La clé de nom des cellules de cet usage.
    pub fn name_key(self) -> &'static str {
        match self {
            Room::Cockpit => "room.cockpit",
            Room::Airlock => "room.airlock",
            Room::Hold => "room.cargo_hold",
            Room::Engine => "room.engine_room",
            Room::Mess => "room.mess",
            Room::Quarters => "room.crew_quarters",
        }
    }

    /// La salle de cet usage, d'après la clé de nom d'une cellule.
    pub fn from_name_key(key: &str) -> Option<Self> {
        TOUR.into_iter().find(|r| r.name_key() == key)
    }
}

/// Usage de la cellule du bureau des contrats, du hall par lequel on y arrive, et de la Banque
/// (spatioport).
const OFFICE_ROOM: &str = "room.contracts_office";
const HALL_ROOM: &str = "room.arrivals_hall";
const BANK_ROOM: &str = "room.bank";
/// Le rôle du PNJ de comptoir qui donne les contrats (`items::TalkTo`), et de celui de la Banque.
const CONTRACTS_ROLE: &str = "contracts";
const BANK_ROLE: &str = "bank";

/// La prochaine salle du tour pas encore visitée, dans l'ordre du repère.
pub fn next_room(ticked: &[Check]) -> Option<Room> {
    TOUR.into_iter()
        .find(|r| !ticked.contains(&Check::Visit(*r)))
}

/// Le plus court chemin de la cellule `from` à la cellule `to`, à travers les portails `edges`
/// (paires de cellules) : le rang dans `edges` de chaque portail franchi, dans l'ordre. `None`
/// si l'on y est déjà ou si rien n'y mène. Parcours en largeur dans l'ordre des portails : le
/// même chemin à chaque frame, le repère ne saute pas d'une porte à l'autre.
pub fn path<S: AsRef<str>>(edges: &[(S, S)], from: &str, to: &str) -> Option<Vec<usize>> {
    if from == to {
        return None;
    }
    // Chaque cellule atteinte garde son chemin depuis `from` (quelques salles : c'est court).
    let mut queue: std::collections::VecDeque<(&str, Vec<usize>)> =
        std::collections::VecDeque::from([(from, Vec::new())]);
    let mut seen: Vec<&str> = vec![from];
    while let Some((cell, route)) = queue.pop_front() {
        for (i, (a, b)) in edges.iter().enumerate() {
            let (a, b) = (a.as_ref(), b.as_ref());
            let other = if a == cell {
                b
            } else if b == cell {
                a
            } else {
                continue;
            };
            if seen.contains(&other) {
                continue;
            }
            let mut next = route.clone();
            next.push(i);
            if other == to {
                return Some(next);
            }
            seen.push(other);
            queue.push_back((other, next));
        }
    }
    None
}

/// Une vérification d'étape : un fait que le jeu constate. Sauvegardée par son nom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Check {
    /// Un casier ouvert (écran de rangement).
    Locker,
    /// Un objet ramassé à bord (la tablette de la chambre).
    Pickup,
    /// L'inventaire ouvert.
    Inventory,
    /// Une salle du tour visitée.
    Visit(Room),
    Look,
    Thrust,
    Cruise,
    Dock,
    Stand,
    /// Sorti du vaisseau, dans le hangar. Ancien « sortir par le sas » (M8) : même fait, les
    /// sauvegardes de M8 le relisent.
    #[serde(alias = "Airlock")]
    Hangar,
    /// Dans le hall des arrivées, en chemin vers le bureau.
    Hall,
    Office,
    Talk,
    Contract,
    /// Dans la salle de la Banque, ou près de son employé.
    Bank,
    /// La conversation de l'employé de la Banque (ou son service) à l'écran.
    Teller,
    /// Le compte et l'hypothèque à l'écran.
    Account,
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
            Check::Locker => "locker",
            Check::Pickup => "pickup",
            Check::Inventory => "inventory",
            Check::Visit(room) => match room {
                Room::Cockpit => "visit_cockpit",
                Room::Airlock => "visit_airlock",
                Room::Hold => "visit_hold",
                Room::Engine => "visit_engine",
                Room::Mess => "visit_mess",
                Room::Quarters => "visit_quarters",
            },
            Check::Look => "look",
            Check::Thrust => "thrust",
            Check::Cruise => "cruise",
            Check::Dock => "dock",
            Check::Stand => "stand",
            Check::Hangar => "hangar",
            Check::Hall => "hall",
            Check::Office => "office",
            Check::Talk => "talk",
            Check::Contract => "contract",
            Check::Bank => "bank",
            Check::Teller => "teller",
            Check::Account => "account",
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
            Check::Locker => f.locker_open,
            Check::Pickup => f.picked_up,
            Check::Inventory => f.inventory,
            Check::Visit(room) => f.room == Some(room),
            Check::Look => f.looked,
            Check::Thrust => f.thrusting,
            Check::Cruise => f.cruising,
            Check::Dock => f.docked,
            Check::Stand => f.standing_docked,
            Check::Hangar => f.left_ship,
            Check::Hall => f.in_hall,
            Check::Office => f.at_office,
            Check::Talk => f.talking,
            Check::Contract => f.cargo_contract.is_some(),
            Check::Bank => f.at_bank,
            Check::Teller => f.talking_bank,
            Check::Account => f.account,
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
    /// L'écran de rangement d'un casier est ouvert.
    pub locker_open: bool,
    /// Un objet posé dans le vaisseau du joueur a été ramassé (`items::PickedUp`).
    pub picked_up: bool,
    /// L'écran du personnage est ouvert sur l'inventaire.
    pub inventory: bool,
    /// La salle du tour où se trouve le joueur, à bord de son vaisseau.
    pub room: Option<Room>,
    /// Le vaisseau a tourné d'assez d'un total, aux commandes.
    pub looked: bool,
    /// Le vaisseau file à bonne vitesse de manœuvre.
    pub thrusting: bool,
    /// Le vaisseau va plus vite que sa vitesse de manœuvre : la croisière.
    pub cruising: bool,
    pub docked: bool,
    /// À quai et debout.
    pub standing_docked: bool,
    /// À quai, dans une cellule du quartier (sorti du vaisseau : le hangar d'abord).
    pub left_ship: bool,
    /// À quai, dans le hall des arrivées (ou déjà au bureau, s'il n'y a pas de hall).
    pub in_hall: bool,
    /// Dans le bureau des contrats, ou près de son guichetier.
    pub at_office: bool,
    /// Le terminal montre le guichetier ou ses contrats.
    pub talking: bool,
    /// Un contrat de livraison actif (transport ou ravitaillement), le plus récent.
    pub cargo_contract: Option<String>,
    /// Dans la salle de la Banque, ou près de son employé (ou en conversation avec lui).
    pub at_bank: bool,
    /// Le terminal montre l'employé de la Banque ou son service.
    pub talking_bank: bool,
    /// Le terminal montre le compte et l'hypothèque.
    pub account: bool,
    pub journal: bool,
    /// L'écran du personnage (inventaire, compétences) ou la fiche de la tablette.
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

    /// Le tutoriel d'une partie qui commence à l'étape de ce numéro (`SH_TUTORIAL_STEP`) : de 1
    /// à 11, ou terminé juste après la dernière (12, captures sans panneau) ; `None` au-delà.
    pub fn at_number(n: usize) -> Option<Self> {
        match Stage::from_number(n) {
            Some(stage) => Some(Self::new(stage)),
            None if n == Stage::ALL.len() + 1 => Some(Self::finished()),
            None => None,
        }
    }

    pub fn stage(&self) -> Option<Stage> {
        match self.progress {
            Progress::Running(stage) => Some(stage),
            Progress::Finished | Progress::Skipped => None,
        }
    }

    /// Numéro de l'étape en cours ; 12 (le nombre d'étapes plus un) une fois fini ou passé
    /// (vérifications de l'autopilote).
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

/// Le dialogue ouvre-t-il ce service (les contrats d'un guichetier, le compte de la Banque…), à
/// l'arrivée sur un nœud ou par une réponse ? Un interlocuteur se reconnaît à ce qu'il fait, pas
/// à son nom.
pub fn opens_screen(dialogue: &Dialogue, screen: DialogueScreen) -> bool {
    let opens = |effects: &[Effect]| effects.contains(&Effect::Open(screen));
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
/// Distance horizontale (m) à un PNJ de comptoir (guichetier des contrats, employé de la
/// Banque) à laquelle on a « trouvé » son bureau.
const COUNTER_DISTANCE: f32 = 4.0;
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
    /// L'écran du personnage ouvert, et les objets déjà ramassés.
    screens: (Option<Res<'w, CharacterScreen>>, Option<Res<'w, PickedUp>>),
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

    fn active(&self) -> Option<&ActiveSector> {
        self.sector.as_deref()
    }

    /// La cellule où se trouve le joueur, à bord ou non.
    fn current_cell(&self) -> Option<&str> {
        self.cells.as_ref()?.current.as_deref()
    }

    /// Le joueur est-il dans son propre vaisseau, et dans quelle cellule ? En vol comme à quai,
    /// quel que soit l'intérieur type (`sector::aboard_cell`).
    fn aboard(&self) -> Option<&str> {
        crate::sector::aboard_cell(self.active(), self.cells.as_deref()?)
    }

    /// À quai, le joueur est-il sorti de son vaisseau (une cellule du quartier) ?
    fn in_district(&self) -> bool {
        self.docked.is_some() && self.current_cell().is_some() && self.aboard().is_none()
    }

    /// L'usage d'une cellule du secteur chargé : sa clé de nom (`room.*`).
    fn room_key(&self, cell: &str) -> Option<&str> {
        self.active()?.sector.cell(cell)?.name_key.as_deref()
    }

    /// La première cellule de cet usage, dans le vaisseau du joueur (`aboard`) ou hors de lui.
    fn cell_of(&self, key: &str, aboard: bool) -> Option<&str> {
        let active = self.active()?;
        active
            .sector
            .cells
            .iter()
            .find(|c| c.name_key.as_deref() == Some(key) && active.is_ship_cell(&c.id) == aboard)
            .map(|c| c.id.as_str())
    }

    /// La prochaine porte du chemin vers cette cellule, dans le monde ; `None` si l'on y est
    /// déjà (ou si rien n'y mène). Une ouverture (deux couloirs bout à bout) ne se montre pas :
    /// on y passe sans rien faire ; le repère va à la première vraie porte du chemin.
    fn door_toward(&self, target: &str) -> Option<Vec3> {
        let active = self.active()?;
        let from = self.current_cell()?;
        let portals = &active.sector.portals;
        let edges: Vec<(&str, &str)> = portals
            .iter()
            .map(|p| (p.a.as_str(), p.b.as_str()))
            .collect();
        let route: Vec<&sh_data::sector::Portal> = path(&edges, from, target)?
            .into_iter()
            .filter_map(|i| portals.get(i))
            .collect();
        let portal = route
            .iter()
            .find(|p| !matches!(p.kind, sh_data::sector::PortalKind::Opening))
            .or(route.first())?;
        let root = self.transforms.get(active.root).ok()?;
        Some(root.transform_point(v3(portal.center)))
    }

    fn player_position(&self) -> Option<Vec3> {
        self.player.single().ok().map(GlobalTransform::translation)
    }

    /// L'interactif voulu le plus proche du joueur, s'il est chargé.
    fn nearest(&self, wanted: impl Fn(&Interactable) -> bool) -> Option<(&Interactable, Vec3)> {
        let player = self.player_position()?;
        self.interactables
            .iter()
            .filter(|(i, _)| wanted(i))
            .map(|(i, t)| (i, t.translation()))
            .min_by(|a, b| {
                a.1.distance_squared(player)
                    .total_cmp(&b.1.distance_squared(player))
            })
    }

    /// Le PNJ de comptoir de ce rôle le plus proche (guichetier des contrats, employé de la
    /// Banque), s'il est chargé.
    fn npc(&self, wanted: &str) -> Option<Vec3> {
        self.nearest(|i| matches!(i, Interactable::Npc { role, .. } if role == wanted))
            .map(|(_, p)| p)
    }

    /// Le joueur est-il à portée du PNJ de comptoir de ce rôle ?
    fn near_npc(&self, role: &str) -> bool {
        match (self.npc(role), self.player_position()) {
            (Some(npc), Some(player)) => {
                Vec2::new(npc.x - player.x, npc.z - player.z).length() < COUNTER_DISTANCE
            }
            _ => false,
        }
    }

    /// Le joueur est-il dans la cellule de cet usage, hors de son vaisseau (bureau, Banque) ?
    fn in_room(&self, key: &str) -> bool {
        let current = self.current_cell();
        current.is_some() && current == self.cell_of(key, false)
    }

    /// Le siège du pilote.
    fn helm(&self) -> Option<Vec3> {
        self.nearest(|i| matches!(i, Interactable::Seat { role } if role == "pilot"))
            .map(|(_, p)| p)
    }

    /// Ce que montre l'écran du personnage, s'il est ouvert.
    fn character_view(&self) -> Option<&ScreenView> {
        self.screens.0.as_ref().map(|s| &s.view)
    }

    /// Un objet posé dans le vaisseau du joueur a-t-il été ramassé ? Un point de ramassage pris
    /// a pour identifiant `<secteur>/<cellule>/<nœud>` (`sector::gltf::pickup_id`).
    fn picked_aboard(&self) -> bool {
        let (Some(active), Some(picked)) = (self.active(), self.screens.1.as_ref()) else {
            return false;
        };
        picked.0.iter().any(|id| {
            id.split('/')
                .nth(1)
                .is_some_and(|cell| active.is_ship_cell(cell))
        })
    }

    /// Le terminal montre-t-il ce guichet : la conversation d'un PNJ de ce rôle (ou d'un
    /// interlocuteur qui ouvre ce service), ou le service lui-même (les contrats, le compte) ?
    fn talking_to(&self, role: &str, service: DialogueScreen) -> bool {
        match self.terminal.as_ref().map(|t| &t.screen) {
            Some(Screen::Dialogue { id, .. }) => self.data.dialogues.iter().any(|d| {
                &d.id == id && (d.role.as_deref() == Some(role) || opens_screen(d, service))
            }),
            Some(screen) => *screen == Screen::from(service),
            None => false,
        }
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
        let talking = self.talking_to(CONTRACTS_ROLE, DialogueScreen::Contracts);
        let talking_bank = self.talking_to(BANK_ROLE, DialogueScreen::Bank);
        let current = self.current_cell();
        let at_office =
            docked && (self.in_room(OFFICE_ROOM) || self.near_npc(CONTRACTS_ROLE) || talking);
        let at_bank =
            docked && (self.in_room(BANK_ROOM) || self.near_npc(BANK_ROLE) || talking_bank);
        let in_district = self.in_district();
        let in_hall =
            in_district && (current.and_then(|c| self.room_key(c)) == Some(HALL_ROOM) || at_office);
        let character = self.character_view();
        let target = target_contract(tutorial, captain);
        let here = self.location.as_ref().map(|l| &l.system);
        Facts {
            locker_open: matches!(character, Some(ScreenView::Container(_))),
            picked_up: self.picked_aboard(),
            inventory: character == Some(&ScreenView::Tab(CharacterTab::Inventory)),
            room: self
                .aboard()
                .and_then(|cell| self.room_key(cell))
                .and_then(Room::from_name_key),
            looked: turned >= LOOK_DEGREES,
            thrusting: speed >= THRUST_SPEED,
            cruising: speed >= self.max_speed() * CRUISE_FACTOR,
            docked: docked && view == Some(ViewLevel::Sector),
            standing_docked: docked && *self.mode.get() == ControlMode::OnFoot,
            left_ship: in_district,
            in_hall,
            at_office,
            talking,
            cargo_contract: latest_cargo_contract(captain).map(|c| c.id.clone()),
            at_bank,
            talking_bank,
            account: matches!(screen, Some(Screen::Bank)),
            journal: matches!(screen, Some(Screen::Journal)),
            // M9 : [KeyC], [KeyI], [KeyO] ouvrent l'écran du personnage ; la fiche de la
            // tablette compte aussi.
            sheet: matches!(character, Some(ScreenView::Tab(_)))
                || matches!(screen, Some(Screen::Sheet)),
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
    let tutorial = crate::dev::autopilot_active()
        .then(|| std::env::var("SH_TUTORIAL_STEP").ok())
        .flatten()
        .and_then(|v| v.trim().parse().ok())
        .and_then(Tutorial::at_number)
        .unwrap_or_else(|| Tutorial::new(Stage::ALL[0]));
    commands.insert_resource(tutorial);
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
        // Le journal dit ce qui a été constaté (mise au point des scénarios de l'autopilote).
        for check in next.ticked.iter().filter(|c| !tutorial.is_ticked(**c)) {
            info!("tutoriel : coché {check:?}");
        }
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

/// Les chiffres de la dette que cite l'étape de la Banque : la dette du départ de partie, le
/// prêteur et l'échéance du prêt en cours (à défaut, ceux des règles), la cadence des échéances,
/// la pénalité de retard et le nombre d'échéances manquées avant la saisie (`career.ron`). Tirés
/// des données, jamais écrits dans le texte : le panneau dit exactement ce que le jeu applique
/// (`sh_sim::Captain::tick`, `pay_installment`), même si l'équilibrage change.
fn debt_args(texts: &Texts, data: &Data, captain: Option<&Captain>) -> Vec<(&'static str, String)> {
    let rules = &data.career.bank;
    let loan = captain.and_then(|c| c.loan.as_ref());
    let lender = loan.map_or(&rules.lender, |l| &l.lender);
    let installment = loan.map_or(Credits::units(rules.installment), |l| l.installment);
    let debt = data.start().map_or(0, |s| s.debt);
    vec![
        ("debt", money(texts, Credits::units(debt))),
        ("lender", faction_name(texts, data, lender)),
        ("installment", money(texts, installment)),
        ("days", texts.number(f64::from(rules.every_days), 0)),
        ("penalty", money(texts, Credits::units(rules.late_penalty))),
        (
            "limit",
            texts.number(f64::from(rules.seize_after_missed), 0),
        ),
    ]
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
    let mut owned: Vec<(&str, String)> = vec![("place", place), ("beacon", beacon)];
    if stage == Stage::Bank {
        owned.extend(debt_args(texts, data, captain));
    }
    let args: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
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
                // Une salle du tour : son nom, celui du bandeau (`lang/<code>/rooms.ron`).
                let room = match c {
                    Check::Visit(room) => texts.get(room.name_key()),
                    _ => String::new(),
                };
                let mut args = args.clone();
                args.push(("room", room.as_str()));
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

/// La cible du repère de l'étape, et son nom. Vers une salle, la prochaine porte du chemin
/// ([`Observed::door_toward`]) ; dans la salle, ce qu'on y cherche (siège, guichetier, employé
/// de la Banque).
fn marker_target(
    texts: &Texts,
    observed: &Observed,
    tutorial: &Tutorial,
    stage: Stage,
    target: Option<&Contract>,
    beacons: &Query<(&DockBeacon, &Name, &GlobalTransform)>,
    points: &Query<(&CargoPoint, &Name, &GlobalTransform)>,
) -> Option<(Vec3, String)> {
    let view = observed.view();
    let docked = observed.docked.is_some();
    // La cible si l'on est dans sa salle, sinon la prochaine porte vers elle.
    let toward = |cell: Option<&str>, here: Option<Vec3>| match cell {
        Some(cell) if observed.current_cell() != Some(cell) => observed.door_toward(cell).or(here),
        _ => here,
    };
    match stage {
        Stage::Cabin if !tutorial.is_ticked(Check::Locker) => observed
            .nearest(|i| matches!(i, Interactable::Locker { .. }))
            .map(|(_, p)| (p, texts.get("tutorial.marker.locker"))),
        Stage::Cabin if !tutorial.is_ticked(Check::Pickup) => observed
            .nearest(|i| matches!(i, Interactable::Pickup { .. }))
            .map(|(i, p)| {
                let item = match i {
                    Interactable::Pickup { item } => texts.get(&format!("item.{item}")),
                    _ => String::new(),
                };
                (
                    p,
                    texts.format("tutorial.marker.pickup", &[("item", &item)]),
                )
            }),
        Stage::Tour => {
            let room = next_room(&tutorial.ticked)?;
            let cell = observed.cell_of(room.name_key(), true)?;
            observed
                .door_toward(cell)
                .map(|p| (p, texts.get(room.name_key())))
        }
        Stage::Flight | Stage::Departure
            if !observed.piloting() && view != Some(ViewLevel::Galaxy) =>
        {
            let cockpit = observed.cell_of(Room::Cockpit.name_key(), true);
            toward(cockpit, observed.helm()).map(|p| (p, texts.get("tutorial.marker.helm")))
        }
        Stage::Office | Stage::Contract if docked => {
            let office = observed.cell_of(OFFICE_ROOM, false);
            toward(office, observed.npc(CONTRACTS_ROLE))
                .map(|p| (p, texts.get("tutorial.marker.office")))
        }
        // Du bureau des contrats, par le hall, jusqu'à l'employé derrière son guichet.
        Stage::Bank if docked => {
            let bank = observed.cell_of(BANK_ROOM, false);
            toward(bank, observed.npc(BANK_ROLE)).map(|p| (p, texts.get("tutorial.marker.bank")))
        }
        Stage::Dock if view == Some(ViewLevel::Global) => beacons
            .iter()
            .next()
            .map(|(_, name, t)| (t.translation(), name.as_str().to_owned())),
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
            marker_target(
                &texts, &observed, tutorial, stage, contract, &beacons, &points,
            )
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
            // Immobile, le repère ne relance pas la mise en page de l'interface (M11).
            set_px_if_changed(&mut node, left, top);
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
                Check::Locker => f.locker_open = true,
                Check::Pickup => f.picked_up = true,
                Check::Inventory => f.inventory = true,
                // Une salle par relevé : voir `tout_le_parcours_jusqu_a_la_fin`.
                Check::Visit(room) => f.room = Some(*room),
                Check::Look => f.looked = true,
                Check::Thrust => f.thrusting = true,
                Check::Cruise => f.cruising = true,
                Check::Dock => f.docked = true,
                Check::Stand => f.standing_docked = true,
                Check::Hangar => f.left_ship = true,
                Check::Hall => f.in_hall = true,
                Check::Office => f.at_office = true,
                Check::Talk => f.talking = true,
                Check::Contract => f.cargo_contract = Some("CT-1".into()),
                Check::Bank => f.at_bank = true,
                Check::Teller => f.talking_bank = true,
                Check::Account => f.account = true,
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
    fn onze_etapes_numerotees() {
        assert_eq!(Stage::ALL.len(), 11);
        for (i, stage) in Stage::ALL.iter().enumerate() {
            assert_eq!(stage.number(), i + 1);
            assert_eq!(Stage::from_number(i + 1), Some(*stage));
            assert!(!stage.checks().is_empty());
        }
        assert_eq!(Stage::from_number(0), None);
        assert_eq!(Stage::from_number(12), None);
        assert_eq!(Stage::Save.next(), None);
        // Une nouvelle partie commence dans la chambre ; la Banque vient après le contrat.
        assert_eq!(Stage::ALL[0], Stage::Cabin);
        assert_eq!(Stage::Contract.next(), Some(Stage::Bank));
        assert_eq!(Stage::Bank.next(), Some(Stage::Screens));
        // L'ordre de l'énumération est celui du parcours.
        assert!(Stage::ALL.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn depart_a_une_etape_choisie() {
        // `SH_TUTORIAL_STEP` : de 1 à 11, puis 12 = terminé (captures sans panneau).
        assert_eq!(
            Tutorial::at_number(1).and_then(|t| t.stage()),
            Some(Stage::Cabin)
        );
        assert_eq!(
            Tutorial::at_number(7).and_then(|t| t.stage()),
            Some(Stage::Bank)
        );
        assert_eq!(
            Tutorial::at_number(11).and_then(|t| t.stage()),
            Some(Stage::Save)
        );
        let done = Tutorial::at_number(12).expect("terminé");
        assert_eq!(done.progress, Progress::Finished);
        assert_eq!(done.number(), 12);
        assert_eq!(Tutorial::at_number(0), None);
        assert_eq!(Tutorial::at_number(13), None);
    }

    #[test]
    fn la_banque_se_franchit_sans_payer() {
        let mut t = Tutorial::new(Stage::Bank);
        // Entré dans la salle de la Banque : une case, pas l'étape.
        assert_eq!(
            t.observe(&Facts {
                at_bank: true,
                ..default()
            }),
            None
        );
        // La conversation de l'employé, puis le compte : l'étape est franchie, sans paiement
        // (aucun fait de paiement n'est demandé).
        assert_eq!(
            t.observe(&Facts {
                talking_bank: true,
                ..default()
            }),
            None
        );
        assert_eq!(
            t.observe(&Facts {
                account: true,
                ..default()
            }),
            Some(Advance::Stage(Stage::Screens))
        );
        // Parler au guichetier des contrats ne compte pas pour la Banque.
        let mut t = Tutorial::new(Stage::Bank);
        t.observe(&Facts {
            at_office: true,
            talking: true,
            ..default()
        });
        assert!(t.ticked.is_empty());
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
        // Assis aux commandes, et il regarde autour de lui.
        let looked = Facts {
            at_helm: true,
            looked: true,
            ..default()
        };
        assert_eq!(t.observe(&looked), None);
        assert!(t.is_ticked(Check::Look) && t.is_ticked(Check::Helm));
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
        let mut t = Tutorial::new(Stage::Cabin);
        let mut seen = Vec::new();
        for stage in Stage::ALL {
            assert_eq!(t.stage(), Some(stage));
            if stage == Stage::Tour {
                // Une salle à la fois, dans le désordre : l'étape tient jusqu'à la dernière.
                for room in [
                    Room::Cockpit,
                    Room::Hold,
                    Room::Quarters,
                    Room::Engine,
                    Room::Mess,
                ] {
                    let f = Facts {
                        room: Some(room),
                        ..default()
                    };
                    assert_eq!(t.observe(&f), None, "{room:?}");
                }
                assert_eq!(next_room(&t.ticked), Some(Room::Airlock));
                seen.push(t.observe(&Facts {
                    room: Some(Room::Airlock),
                    ..default()
                }));
                continue;
            }
            seen.push(t.observe(&all_of(stage)));
        }
        assert_eq!(seen.last(), Some(&Some(Advance::Finished)));
        assert_eq!(t.progress, Progress::Finished);
        assert_eq!(t.number(), 12);
        // Une étape par relevé : la 2e n'est pas franchie au même frame que la 1re.
        assert_eq!(seen[0], Some(Advance::Stage(Stage::Tour)));
        assert_eq!(seen[1], Some(Advance::Stage(Stage::Flight)));
        // Le contrat pris est retenu pour la suite.
        assert_eq!(t.contract.as_deref(), Some("CT-1"));
        assert_eq!(t.observe(&all_of(Stage::Save)), None);
    }

    #[test]
    fn le_tour_suit_l_ordre_du_repere() {
        assert_eq!(next_room(&[]), Some(Room::Quarters));
        let ticked = [Check::Visit(Room::Quarters), Check::Visit(Room::Airlock)];
        assert_eq!(next_room(&ticked), Some(Room::Mess));
        let all: Vec<Check> = TOUR.iter().map(|r| Check::Visit(*r)).collect();
        assert_eq!(next_room(&all), None);
        // Les vérifications du tour sont les salles du repère, dans le même ordre.
        assert_eq!(Stage::Tour.checks(), all.as_slice());
        for room in TOUR {
            assert_eq!(Room::from_name_key(room.name_key()), Some(room));
        }
        assert_eq!(Room::from_name_key("room.cabin"), None);
        assert_eq!(Room::from_name_key("room.corridor"), None);
    }

    #[test]
    fn la_prochaine_porte_du_chemin() {
        // Le vaisseau type, amarré : chambre → couloir avant → couloir arrière → sas → hangar.
        let edges = [
            ("couloir_arriere", "machines"),
            ("couloir_arriere", "sas"),
            ("couloir_arriere", "couloir_avant"),
            ("couloir_avant", "chambre"),
            ("couloir_avant", "cockpit"),
            ("sas", "@dock"),
            ("sas", "hangar"),
            ("couloir", "hangar"),
            ("couloir", "hall"),
            ("hall", "bureau"),
        ];
        assert_eq!(path(&edges, "chambre", "cockpit"), Some(vec![3, 4]));
        assert_eq!(path(&edges, "couloir_avant", "cockpit"), Some(vec![4]));
        assert_eq!(
            path(&edges, "chambre", "bureau"),
            Some(vec![3, 2, 1, 6, 7, 8, 9])
        );
        assert_eq!(path(&edges, "sas", "bureau"), Some(vec![6, 7, 8, 9]));
        assert_eq!(path(&edges, "hall", "bureau"), Some(vec![9]));
        assert_eq!(
            path(&edges, "bureau", "cockpit"),
            Some(vec![9, 8, 7, 6, 1, 2, 4])
        );
        // Déjà là, ou rien n'y mène (le point d'attache externe `@dock` ne mène nulle part).
        assert_eq!(path(&edges, "bureau", "bureau"), None);
        assert_eq!(path(&edges, "chambre", "taverne"), None);
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
        let mut t = Tutorial::new(Stage::Tour);
        t.observe(&Facts {
            room: Some(Room::Hold),
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
    fn sauvegardes_de_m8_en_plein_tutoriel() {
        // L'étape et les cases sont sauvegardées par leur nom : une partie de M8 sortie du
        // vaisseau (« Airlock », devenu « Hangar ») reprend à la même étape, cases gardées,
        // malgré les deux étapes ajoutées devant.
        let old: Tutorial = sh_data::ron_options()
            .from_str("(progress: Running(Office), ticked: [Stand, Airlock], contract: None)")
            .expect("relisible");
        assert_eq!(old.stage(), Some(Stage::Office));
        assert!(old.is_ticked(Check::Stand) && old.is_ticked(Check::Hangar));
        for (text, stage) in [
            ("Flight", Stage::Flight),
            ("Contract", Stage::Contract),
            ("Save", Stage::Save),
        ] {
            let t: Tutorial = sh_data::ron_options()
                .from_str(&format!("(progress: Running({text}))"))
                .expect("relisible");
            assert_eq!(t.stage(), Some(stage));
        }
        let done: Tutorial = sh_data::ron_options()
            .from_str("(progress: Finished)")
            .expect("relisible");
        assert_eq!(done.stage(), None);
    }

    #[test]
    fn sauvegardes_d_avant_l_etape_de_la_banque() {
        // Une partie de M10 sauvegardée au contrat (guichetier déjà vu) : le contrat pris, elle
        // passe par la Banque, bien que l'étape n'existât pas quand elle a été sauvegardée.
        let mut t: Tutorial = sh_data::ron_options()
            .from_str("(progress: Running(Contract), ticked: [Talk], contract: None)")
            .expect("relisible");
        assert_eq!(t.stage(), Some(Stage::Contract));
        assert_eq!(
            t.observe(&Facts {
                cargo_contract: Some("CT-7".into()),
                ..default()
            }),
            Some(Advance::Stage(Stage::Bank))
        );
        assert_eq!(t.contract.as_deref(), Some("CT-7"));
        // Sauvegardée à l'étape de la Banque (par son nom), elle y reprend, cases gardées.
        t.observe(&Facts {
            at_bank: true,
            ..default()
        });
        let text = sh_data::ron_options().to_string(&t).expect("sérialisable");
        assert!(text.contains("Bank"), "{text}");
        let back: Tutorial = sh_data::ron_options().from_str(&text).expect("relisible");
        assert_eq!(back, t);
        assert_eq!(back.stage(), Some(Stage::Bank));
        assert!(back.is_ticked(Check::Bank));
        // Une partie déjà aux écrans (ou plus loin) ne revient pas en arrière : la suite est le
        // départ, jamais la Banque.
        for (text, stage, next) in [
            ("Screens", Stage::Screens, Some(Stage::Departure)),
            ("Departure", Stage::Departure, Some(Stage::Delivery)),
        ] {
            let mut t: Tutorial = sh_data::ron_options()
                .from_str(&format!("(progress: Running({text}))"))
                .expect("relisible");
            assert_eq!(t.stage(), Some(stage));
            assert_eq!(t.observe(&all_of(stage)), next.map(Advance::Stage));
            assert_ne!(t.stage(), Some(Stage::Bank));
        }
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
