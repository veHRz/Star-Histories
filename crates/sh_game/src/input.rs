//! Entrées par contextes (`bevy_enhanced_input`) : À pied, Pilotage, Carte, Menu.
//!
//! Chaque contexte est une entité qui porte ses actions et leurs liaisons (déclarées en BSN, la
//! syntaxe recommandée par la crate). Le contexte actif suit l'état [`ControlMode`] : passer du
//! siège de pilotage à la marche ne fait que changer d'état. Le remappage (M7) modifiera les
//! entités de liaison, sans toucher aux systèmes de jeu qui lisent les actions.

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

// --- Fiche et journal (à pied et aux commandes) -------------------------------------------

#[derive(InputAction)]
#[action_output(bool)]
pub struct OpenSheet;

#[derive(InputAction)]
#[action_output(bool)]
pub struct OpenJournal;

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
            Bindings [ Binding(KeyCode::Space) -- Binding(GamepadButton::South) ]
            --
            Action::<Sprint>::new()
            Bindings [ Binding(KeyCode::ShiftLeft) -- Binding(GamepadButton::LeftThumb) ]
            --
            Action::<Crouch>::new()
            Bindings [ Binding(KeyCode::ControlLeft) -- Binding(GamepadButton::East) ]
            --
            Action::<Interact>::new()
            Bindings [ Binding(KeyCode::KeyE) -- Binding(GamepadButton::West) ]
            --
            Action::<ToggleMap>::new()
            Bindings [ Binding(KeyCode::KeyM) -- Binding(GamepadButton::Select) ]
            --
            Action::<OpenSheet>::new()
            Bindings [ Binding(KeyCode::KeyC) ]
            --
            Action::<OpenJournal>::new()
            Bindings [ Binding(KeyCode::KeyJ) ]
            --
            Action::<Pause>::new()
            Bindings [ Binding(KeyCode::Escape) -- Binding(GamepadButton::Start) ]
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
            Bindings [ Binding(KeyCode::KeyZ) ]
            --
            Action::<Dock>::new()
            Bindings [ Binding(KeyCode::KeyG) ]
            --
            Action::<LeaveSeat>::new()
            Bindings [ Binding(KeyCode::KeyF) ]
            --
            Action::<FireGuns>::new()
            Bindings [ Binding(MouseButton::Left) ]
            --
            Action::<FireMissile>::new()
            Bindings [ Binding(MouseButton::Right) ]
            --
            Action::<Countermeasure>::new()
            Bindings [ Binding(KeyCode::KeyX) ]
            --
            Action::<ToggleMap>::new()
            Bindings [ Binding(KeyCode::KeyM) ]
            --
            Action::<OpenSheet>::new()
            Bindings [ Binding(KeyCode::KeyC) ]
            --
            Action::<OpenJournal>::new()
            Bindings [ Binding(KeyCode::KeyJ) ]
            --
            Action::<Pause>::new()
            Bindings [ Binding(KeyCode::Escape) -- Binding(GamepadButton::Start) ]
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
            Bindings [ Binding(MouseButton::Left) -- Binding(GamepadButton::South) ]
            --
            Action::<TravelPortal>::new()
            Bindings [ Binding(KeyCode::Digit1) ]
            --
            Action::<TravelHyperspace>::new()
            Bindings [ Binding(KeyCode::Digit2) ]
            --
            Action::<ToggleMap>::new()
            Bindings [ Binding(KeyCode::KeyM) -- Binding(GamepadButton::Select) ]
            --
            Action::<Pause>::new()
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
            Bindings [ Binding(KeyCode::Escape) -- Binding(GamepadButton::East) ]
            --
            Action::<Confirm>::new()
            Bindings [ Binding(KeyCode::Enter) -- Binding(GamepadButton::South) ]
            --
            Action::<Navigate>::new()
            Bindings [ {preset::cardinal::arrows()} -- {preset::cardinal::dpad()} ]
        ]
    });
}
