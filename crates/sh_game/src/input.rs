//! Entrées par contextes (`bevy_enhanced_input`) : À pied, Pilotage, Carte, Menu.
//!
//! Chaque contexte est une entité qui porte ses actions et leurs liaisons (déclarées en BSN, la
//! syntaxe recommandée par la crate). Le contexte actif suit l'état [`ControlMode`] : passer du
//! siège de pilotage à la marche ne fait que changer d'état, et un seul contexte de jeu est
//! actif à la fois.
//!
//! Remappage (M7, refait en M9) : [`KEY_ACTIONS`] liste les actions dont le joueur choisit les
//! touches, par page du menu des touches ([`KeyCategory`]), avec leur touche d'origine et les
//! contextes où elles sont liées. Il doit suivre les liaisons par défaut ci-dessous (test
//! `table_des_actions` d'`options`). `options` retrouve chaque liaison clavier par son contexte
//! et sa touche d'origine, lui ajoute une liaison jumelle pour la seconde touche, et réinsère
//! les `Binding` choisis : les systèmes de jeu qui lisent les actions n'en savent rien. Les
//! sensibilités de la souris sont des modificateurs `Scale` posés sur [`Look`] et [`Steer`].

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::state::ControlMode;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin)
            .add_input_context::<OnFoot>()
            .add_input_context::<Piloting>()
            .add_input_context::<MapControls>()
            .add_input_context::<MenuControls>()
            .sync_context_to_state::<OnFoot, ControlMode>()
            .sync_context_to_state::<Piloting, ControlMode>()
            .sync_context_to_state::<MapControls, ControlMode>()
            .sync_context_to_state::<MenuControls, ControlMode>()
            .add_systems(Startup, spawn_contexts);
    }
}

// --- Contextes ---------------------------------------------------------------------------------

/// À pied : marcher, regarder, interagir.
#[derive(Component, Default, Clone)]
pub struct OnFoot;

/// Aux commandes d'un vaisseau.
#[derive(Component, Default, Clone)]
pub struct Piloting;

/// Carte galactique ou système.
#[derive(Component, Default, Clone)]
pub struct MapControls;

/// Menus et écrans d'interface.
#[derive(Component, Default, Clone)]
pub struct MenuControls;

// --- Actions communes ------------------------------------------------------------------------

/// Ouvrir le menu de pause (ou revenir en arrière).
#[derive(InputAction)]
#[action_output(bool)]
pub struct Pause;

/// Ouvrir ou fermer la carte.
#[derive(InputAction)]
#[action_output(bool)]
pub struct ToggleMap;

/// Sauvegarde rapide (à pied et aux commandes).
#[derive(InputAction)]
#[action_output(bool)]
pub struct QuickSave;

/// Chargement de la sauvegarde rapide.
#[derive(InputAction)]
#[action_output(bool)]
pub struct QuickLoad;

/// Passer le tutoriel guidé (deux appuis : voir `tutorial`).
#[derive(InputAction)]
#[action_output(bool)]
pub struct SkipTutorial;

// --- À pied --------------------------------------------------------------------------------

/// Déplacement : x = droite, y = avant.
#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Move;

/// Visée : déplacement de la souris, en pixels.
#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Look;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Jump;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Sprint;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Crouch;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Interact;

// --- Pilotage ------------------------------------------------------------------------------

/// Poussée sur 3 axes, dans le repère du vaisseau (−z = avant).
#[derive(InputAction)]
#[action_output(Vec3)]
pub struct Thrust;

/// Tangage et lacet à la souris, en pixels.
#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Steer;

/// Roulis : + = vers la droite.
#[derive(InputAction)]
#[action_output(f32)]
pub struct Roll;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Boost;

/// Demande d'amarrage (en vol, près d'une balise) ou de départ (à quai, aux commandes).
#[derive(InputAction)]
#[action_output(bool)]
pub struct Dock;

/// Carte : partir vers la destination choisie par les Portails.
#[derive(InputAction)]
#[action_output(bool)]
pub struct TravelPortal;

/// Carte : partir vers la destination choisie par l'hyperespace.
#[derive(InputAction)]
#[action_output(bool)]
pub struct TravelHyperspace;

/// Bascule vol assisté / découplé.
#[derive(InputAction)]
#[action_output(bool)]
pub struct ToggleFlightAssist;

#[derive(InputAction)]
#[action_output(bool)]
pub struct LeaveSeat;

// --- Combat (aux commandes) ----------------------------------------------------------------

/// Canons de chasse (maintenu).
#[derive(InputAction)]
#[action_output(bool)]
pub struct FireGuns;

/// Missile sur la cible verrouillée.
#[derive(InputAction)]
#[action_output(bool)]
pub struct FireMissile;

/// Contre-mesures (leurres).
#[derive(InputAction)]
#[action_output(bool)]
pub struct Countermeasure;

/// Arme portative (à pied).
#[derive(InputAction)]
#[action_output(bool)]
pub struct FireWeapon;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Reload;

// --- Fiche et journal (à pied et aux commandes) -------------------------------------------

#[derive(InputAction)]
#[action_output(bool)]
pub struct OpenSheet;

#[derive(InputAction)]
#[action_output(bool)]
pub struct OpenJournal;

#[derive(InputAction)]
#[action_output(bool)]
pub struct OpenCodex;

// --- Écran du personnage et aide aux touches (M9, à pied et aux commandes) ---------------------

/// L'écran du personnage, onglet Inventaire (`items::OpenCharacterScreen`).
#[derive(InputAction)]
#[action_output(bool)]
pub struct OpenInventory;

/// L'écran du personnage, onglet Compétences.
#[derive(InputAction)]
#[action_output(bool)]
pub struct OpenSkills;

/// Afficher ou replier l'encart des touches (`ui::keys_panel`) ; aussi sur la carte.
#[derive(InputAction)]
#[action_output(bool)]
pub struct ToggleKeyHelp;

// --- Carte ---------------------------------------------------------------------------------

/// Panoramique : x = est, y = nord.
#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Pan;

/// Zoom : + = rapprocher.
#[derive(InputAction)]
#[action_output(f32)]
pub struct Zoom;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Select;

// --- Menu ----------------------------------------------------------------------------------

#[derive(InputAction)]
#[action_output(bool)]
pub struct Back;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Confirm;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Navigate;

// --- Actions remappables (M9) ---------------------------------------------------------------

/// Contexte À pied, en bit : une action liée dans plusieurs contextes porte plusieurs bits.
pub const CTX_FOOT: u8 = 1;
/// Contexte Pilotage.
pub const CTX_PILOTING: u8 = 2;
/// Contexte Carte.
pub const CTX_MAP: u8 = 4;
const CTX_ALL: u8 = CTX_FOOT | CTX_PILOTING | CTX_MAP;
/// Les actions communes à pied et aux commandes (fiche, journal, sauvegarde…).
const CTX_SHIP_AND_FOOT: u8 = CTX_FOOT | CTX_PILOTING;

/// Le contexte (bit) qu'active un mode de commande ; `None` dans les menus, où aucune action
/// remappable n'est active.
pub fn context_of(mode: ControlMode) -> Option<u8> {
    match mode {
        ControlMode::OnFoot => Some(CTX_FOOT),
        ControlMode::Piloting => Some(CTX_PILOTING),
        ControlMode::Map => Some(CTX_MAP),
        ControlMode::Menu => None,
    }
}

/// Une page du menu des touches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyCategory {
    /// Écrans et commandes communes (carte, fiche, journal, sauvegarde rapide…).
    Interface,
    OnFoot,
    Piloting,
    Map,
}

impl KeyCategory {
    pub const ALL: [KeyCategory; 4] = [
        KeyCategory::Interface,
        KeyCategory::OnFoot,
        KeyCategory::Piloting,
        KeyCategory::Map,
    ];

    /// Nom stable : clés de langue (`options.keys.page.<nom>`) et `SH_OPTIONS=keys:<nom>`.
    pub fn name(self) -> &'static str {
        match self {
            KeyCategory::Interface => "interface",
            KeyCategory::OnFoot => "foot",
            KeyCategory::Piloting => "piloting",
            KeyCategory::Map => "map",
        }
    }
}

/// Une action dont le joueur choisit les touches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyAction {
    /// Identifiant stable : fichier de réglages, `SH_KEYS`, libellé `options.action.<id>`.
    pub id: &'static str,
    pub category: KeyCategory,
    /// La touche de la liaison par défaut (BSN ci-dessous).
    pub default: KeyCode,
    /// Les contextes où l'action est liée ([`CTX_FOOT`]…) : deux actions qui en partagent un
    /// sont actives en même temps et ne doivent pas avoir la même touche.
    pub contexts: u8,
}

const fn action(
    id: &'static str,
    category: KeyCategory,
    default: KeyCode,
    contexts: u8,
) -> KeyAction {
    KeyAction {
        id,
        category,
        default,
        contexts,
    }
}

/// Les actions remappables, dans l'ordre des pages et des lignes du menu des touches. Échap,
/// Entrée, les flèches et la souris restent fixes.
pub static KEY_ACTIONS: [KeyAction; 40] = {
    use KeyCategory::{Interface, Map, OnFoot, Piloting};
    use KeyCode::{
        ControlLeft, Digit1, Digit2, F1, F5, F9, KeyA, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI,
        KeyJ, KeyK, KeyM, KeyO, KeyQ, KeyR, KeyS, KeyW, KeyX, KeyZ, ShiftLeft, Space,
    };
    const BOTH: u8 = CTX_SHIP_AND_FOOT;
    [
        action("interface_map", Interface, KeyM, CTX_ALL),
        action("interface_sheet", Interface, KeyC, BOTH),
        action("interface_journal", Interface, KeyJ, BOTH),
        action("interface_codex", Interface, KeyK, BOTH),
        action("interface_inventory", Interface, KeyI, BOTH),
        action("interface_skills", Interface, KeyO, BOTH),
        action("interface_key_help", Interface, F1, CTX_ALL),
        action("interface_quick_save", Interface, F5, BOTH),
        action("interface_quick_load", Interface, F9, BOTH),
        action("interface_skip_tutorial", Interface, KeyH, CTX_ALL),
        action("foot_forward", OnFoot, KeyW, CTX_FOOT),
        action("foot_back", OnFoot, KeyS, CTX_FOOT),
        action("foot_left", OnFoot, KeyA, CTX_FOOT),
        action("foot_right", OnFoot, KeyD, CTX_FOOT),
        action("foot_jump", OnFoot, Space, CTX_FOOT),
        action("foot_sprint", OnFoot, ShiftLeft, CTX_FOOT),
        action("foot_crouch", OnFoot, ControlLeft, CTX_FOOT),
        action("foot_interact", OnFoot, KeyE, CTX_FOOT),
        action("foot_reload", OnFoot, KeyR, CTX_FOOT),
        action("piloting_forward", Piloting, KeyW, CTX_PILOTING),
        action("piloting_back", Piloting, KeyS, CTX_PILOTING),
        action("piloting_left", Piloting, KeyA, CTX_PILOTING),
        action("piloting_right", Piloting, KeyD, CTX_PILOTING),
        action("piloting_up", Piloting, Space, CTX_PILOTING),
        action("piloting_down", Piloting, ControlLeft, CTX_PILOTING),
        action("piloting_roll_left", Piloting, KeyQ, CTX_PILOTING),
        action("piloting_roll_right", Piloting, KeyE, CTX_PILOTING),
        action("piloting_cruise", Piloting, ShiftLeft, CTX_PILOTING),
        action("piloting_assist", Piloting, KeyZ, CTX_PILOTING),
        action("piloting_dock", Piloting, KeyG, CTX_PILOTING),
        action("piloting_leave_seat", Piloting, KeyF, CTX_PILOTING),
        action("piloting_decoys", Piloting, KeyX, CTX_PILOTING),
        action("map_north", Map, KeyW, CTX_MAP),
        action("map_south", Map, KeyS, CTX_MAP),
        action("map_west", Map, KeyA, CTX_MAP),
        action("map_east", Map, KeyD, CTX_MAP),
        action("map_zoom_in", Map, KeyR, CTX_MAP),
        action("map_zoom_out", Map, KeyF, CTX_MAP),
        action("map_portal", Map, Digit1, CTX_MAP),
        action("map_hyperspace", Map, Digit2, CTX_MAP),
    ]
};

/// L'action remappable d'une liaison clavier par défaut : celle du contexte `context` dont la
/// touche d'origine est `key` (une seule par contexte, test `table_des_actions`).
pub fn key_action_index(context: u8, key: KeyCode) -> Option<usize> {
    KEY_ACTIONS
        .iter()
        .position(|a| a.contexts & context != 0 && a.default == key)
}

// --- Liaisons par défaut -------------------------------------------------------------------

fn spawn_contexts(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        #ContexteAPied
        OnFoot
        ~ContextActivity::<OnFoot>::INACTIVE
        ~ActiveInStates::<OnFoot, ControlMode>::single(ControlMode::OnFoot)
        Actions<OnFoot> [
            Action::<Move>::new()
            DeadZone
            Bindings [
                {preset::cardinal::wasd_keys()}
                --
                {preset::axial::left_stick()}
            ]
            --
            Action::<Look>::new()
            Bindings [ Binding::mouse_motion() ]
            --
            Action::<Jump>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::Space) -- Binding(GamepadButton::South) ]
            --
            Action::<Sprint>::new()
            Bindings [ Binding(KeyCode::ShiftLeft) -- Binding(GamepadButton::LeftThumb) ]
            --
            Action::<Crouch>::new()
            Bindings [ Binding(KeyCode::ControlLeft) -- Binding(GamepadButton::East) ]
            --
            Action::<Interact>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyE) -- Binding(GamepadButton::West) ]
            --
            Action::<FireWeapon>::new()
            Bindings [ Binding(MouseButton::Left) -- Binding(GamepadButton::RightTrigger2) ]
            --
            Action::<Reload>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyR) ]
            --
            Action::<ToggleMap>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyM) -- Binding(GamepadButton::Select) ]
            --
            Action::<OpenSheet>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyC) ]
            --
            Action::<OpenJournal>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyJ) ]
            --
            Action::<OpenCodex>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyK) ]
            --
            Action::<OpenInventory>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyI) ]
            --
            Action::<OpenSkills>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyO) ]
            --
            Action::<ToggleKeyHelp>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::F1) ]
            --
            Action::<Pause>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::Escape) -- Binding(GamepadButton::Start) ]
            --
            Action::<QuickSave>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::F5) ]
            --
            Action::<QuickLoad>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::F9) ]
            --
            Action::<SkipTutorial>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyH) ]
        ]
    });

    commands.spawn_scene(bsn! {
        #ContextePilotage
        Piloting
        ~ContextActivity::<Piloting>::INACTIVE
        ~ActiveInStates::<Piloting, ControlMode>::single(ControlMode::Piloting)
        Actions<Piloting> [
            Action::<Thrust>::new()
            Bindings [
                {preset::spatial::new(
                    KeyCode::KeyW,
                    KeyCode::KeyS,
                    KeyCode::KeyA,
                    KeyCode::KeyD,
                    KeyCode::Space,
                    KeyCode::ControlLeft,
                )}
            ]
            --
            Action::<Steer>::new()
            Bindings [ Binding::mouse_motion() ]
            --
            Action::<Roll>::new()
            Bindings [ {preset::bidirectional::new(KeyCode::KeyE, KeyCode::KeyQ)} ]
            --
            Action::<Boost>::new()
            Bindings [ Binding(KeyCode::ShiftLeft) ]
            --
            Action::<ToggleFlightAssist>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyZ) ]
            --
            Action::<Dock>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyG) ]
            --
            Action::<LeaveSeat>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyF) ]
            --
            Action::<FireGuns>::new()
            Bindings [ Binding(MouseButton::Left) ]
            --
            Action::<FireMissile>::new()
            Bindings [ Binding(MouseButton::Right) ]
            --
            Action::<Countermeasure>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyX) ]
            --
            Action::<ToggleMap>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyM) ]
            --
            Action::<OpenSheet>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyC) ]
            --
            Action::<OpenJournal>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyJ) ]
            --
            Action::<OpenCodex>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyK) ]
            --
            Action::<OpenInventory>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyI) ]
            --
            Action::<OpenSkills>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyO) ]
            --
            Action::<ToggleKeyHelp>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::F1) ]
            --
            Action::<Pause>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::Escape) -- Binding(GamepadButton::Start) ]
            --
            Action::<QuickSave>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::F5) ]
            --
            Action::<QuickLoad>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::F9) ]
            --
            Action::<SkipTutorial>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyH) ]
        ]
    });

    commands.spawn_scene(bsn! {
        #ContexteCarte
        MapControls
        ~ContextActivity::<MapControls>::INACTIVE
        ~ActiveInStates::<MapControls, ControlMode>::single(ControlMode::Map)
        Actions<MapControls> [
            Action::<Pan>::new()
            DeadZone
            Bindings [
                {preset::cardinal::wasd_keys()}
                --
                {preset::cardinal::arrows()}
                --
                {preset::axial::left_stick()}
            ]
            --
            Action::<Zoom>::new()
            Bindings [
                Binding::mouse_wheel()
                ~SwizzleAxis::YXZ
                --
                {preset::bidirectional::new(KeyCode::KeyR, KeyCode::KeyF)}
            ]
            --
            Action::<Select>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(MouseButton::Left) -- Binding(GamepadButton::South) ]
            --
            Action::<TravelPortal>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::Digit1) ]
            --
            Action::<TravelHyperspace>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::Digit2) ]
            --
            Action::<ToggleMap>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyM) -- Binding(GamepadButton::Select) ]
            --
            Action::<SkipTutorial>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::KeyH) ]
            --
            Action::<ToggleKeyHelp>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::F1) ]
            --
            Action::<Pause>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::Escape) -- Binding(GamepadButton::Start) ]
        ]
    });

    commands.spawn_scene(bsn! {
        #ContexteMenu
        MenuControls
        ~ContextActivity::<MenuControls>::INACTIVE
        ~ActiveInStates::<MenuControls, ControlMode>::single(ControlMode::Menu)
        Actions<MenuControls> [
            Action::<Back>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::Escape) -- Binding(GamepadButton::East) ]
            --
            Action::<Confirm>::new()
            ActionSettings { require_reset: true }
            Bindings [ Binding(KeyCode::Enter) -- Binding(GamepadButton::South) ]
            --
            Action::<Navigate>::new()
            Bindings [ {preset::cardinal::arrows()} -- {preset::cardinal::dpad()} ]
        ]
    });
}
