//! États du jeu (AGENTS.md §6.2).
//!
//! - [`AppState`] : le cycle de vie de l'application (démarrage, chargement, menu, partie).
//! - [`ViewLevel`] : la vue active pendant une partie (Galaxie, Globale, Sectorielle). La vue
//!   Locale n'est pas un état : c'est un niveau de détail streamé dans la vue Sectorielle (M1).
//! - [`ControlMode`] : le jeu de commandes actif, indépendant de la vue (on peut marcher dans
//!   son vaisseau en vue Globale).
//!
//! Chaque entité d'une vue porte `DespawnOnExit(ViewLevel::…)` : quitter la vue la décharge.

use bevy::prelude::*;

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_sub_state::<ViewLevel>()
            .init_state::<ControlMode>()
            .init_resource::<CurrentPlace>()
            .add_systems(
                OnEnter(AppState::MainMenu),
                |mut mode: ResMut<NextState<ControlMode>>| {
                    mode.set(ControlMode::Menu);
                },
            )
            // Sans sauvegarde, quitter la partie (Échap, mort) l'abandonne. L'effacer pendant la
            // transition d'état, et non au milieu d'`Update`, évite qu'un groupe de systèmes déjà
            // autorisé à tourner trouve ses ressources disparues en cours de route.
            .add_systems(OnExit(AppState::InGame), forget_game)
            .add_systems(Update, log_transitions);
    }
}

#[derive(States, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    /// Premier frame : réglages qui ne dépendent pas des données (polices, caméra d'interface).
    #[default]
    Boot,
    /// Lecture et validation des données (`assets/data`), écran de chargement.
    Loading,
    MainMenu,
    InGame,
}

#[derive(SubStates, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::InGame)]
pub enum ViewLevel {
    /// La carte d'Astréa.
    Galaxy,
    /// Un système stellaire, une planète vue d'orbite, un gros vaisseau.
    Global,
    /// Un secteur jouable : quartier, section de vaisseau, petit vaisseau entier.
    #[default]
    Sector,
}

impl ViewLevel {
    /// Nom technique, celui de `SH_START` (`galaxy`, `global`, `sector`).
    pub fn key(self) -> &'static str {
        match self {
            ViewLevel::Galaxy => "galaxy",
            ViewLevel::Global => "global",
            ViewLevel::Sector => "sector",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "galaxy" => Some(ViewLevel::Galaxy),
            "global" => Some(ViewLevel::Global),
            "sector" => Some(ViewLevel::Sector),
            _ => None,
        }
    }
}

/// Le jeu de commandes actif (contextes d'entrée de `crate::input`).
#[derive(States, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ControlMode {
    #[default]
    Menu,
    OnFoot,
    Piloting,
    Map,
}

/// Le lieu chargé par la vue courante : un identifiant de lieu des données, ou un lieu de test
/// (`test`) tant que les vrais lieux n'existent pas.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct CurrentPlace(pub Option<String>);

/// Demande d'entrer en partie dans une vue donnée (menu « Nouvelle partie » ou `SH_START`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnterGame {
    pub view: ViewLevel,
    pub place: Option<String>,
}

impl EnterGame {
    /// Une nouvelle partie (menu « Nouvelle partie », `SH_START=new`) : aux commandes, dans le
    /// système du départ (P1) ; la vue Globale co-charge le vaisseau du joueur.
    pub fn new_game(data: &sh_data::GameData) -> Self {
        EnterGame {
            view: ViewLevel::Global,
            place: data.start().map(|s| s.system.as_str().to_owned()),
        }
    }

    /// Applique la demande : les deux états changent au même frame, la sous-vue démarre
    /// directement sur la bonne valeur (pas de passage par la vue par défaut).
    pub fn apply(
        &self,
        app_state: &mut NextState<AppState>,
        view: &mut NextState<ViewLevel>,
        place: &mut CurrentPlace,
    ) {
        place.0 = self.place.clone();
        app_state.set(AppState::InGame);
        view.set(self.view);
    }
}

/// Efface la partie quittée : tout ce qu'elle tenait disparaît, et l'entrée en jeu suivante
/// recrée monde, horloge et carrière.
fn forget_game(world: &mut World) {
    world.remove_resource::<crate::combat::Death>();
    world.remove_resource::<crate::combat::Raid>();
    world.remove_resource::<crate::career::Career>();
    world.remove_resource::<crate::living::LivingWorld>();
    world.remove_resource::<crate::living::PlayerKnowledge>();
    world.remove_resource::<crate::clock::GameClock>();
    world.remove_resource::<crate::voyage::Location>();
    world.remove_resource::<crate::voyage::ShipFuel>();
    world.remove_resource::<crate::voyage::ResumeFlight>();
    world.remove_resource::<crate::combat::foot::PlayerArms>();
    world.insert_resource(crate::career::MissionLog::default());
    world.insert_resource(crate::interaction::Inventory::default());
    world.insert_resource(crate::codex::Codex::default());
    world.insert_resource(crate::tips::TipsSeen::default());
}

fn log_transitions(
    mut app: MessageReader<StateTransitionEvent<AppState>>,
    mut view: MessageReader<StateTransitionEvent<ViewLevel>>,
) {
    for t in app.read() {
        info!("état : {:?} → {:?}", t.exited, t.entered);
    }
    for t in view.read() {
        info!("vue : {:?} → {:?}", t.exited, t.entered);
    }
}
