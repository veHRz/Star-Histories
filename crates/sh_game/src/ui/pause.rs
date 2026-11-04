//! Menu de pause (Échap en partie) : reprendre, sauvegarder, charger, quitter vers le menu
//! principal. Le temps du jeu est suspendu (`Time<Virtual>`) : horloge du monde, physique,
//! combats. Il se ferme au relâchement d'Échap, comme le terminal : le contexte rendu ne voit
//! pas la touche encore enfoncée.

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_enhanced_input::prelude::Start;

use crate::combat::Death;
use crate::data::{Data, Texts};
use crate::input::Pause;
use crate::save::{GameMode, SaveKind, list_saves, save_now};
use crate::state::{AppState, ControlMode};
use crate::transition::Sequence;
use crate::ui::menu::spawn_save_list;
use crate::ui::terminal::Terminal;
use crate::ui::theme::{self, UiFonts, menu_button};

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_pause)
            .add_observer(on_activate)
            .add_systems(
                Update,
                (close_on_escape, render)
                    .chain()
                    .run_if(resource_exists::<Paused>),
            )
            .add_systems(OnExit(AppState::InGame), leave);
    }
}

/// La partie est en pause ; `resume` : le jeu de commandes à rendre en reprenant.
#[derive(Resource)]
pub struct Paused {
    resume: ControlMode,
    screen: PauseScreen,
    /// Échap a été relâché depuis l'ouverture : un nouvel appui ferme le menu.
    armed: bool,
    closing: bool,
    drawn: Option<PauseScreen>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PauseScreen {
    Main,
    Load,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum PauseAction {
    Resume,
    Save,
    Load,
    Back,
    Quit,
}

#[derive(Component)]
struct PauseUi;

fn on_pause(_: On<Start<Pause>>, mut commands: Commands) {
    commands.queue(|world: &mut World| open(world, false));
}

/// Met la partie en pause (rien en dehors d'une partie, d'un terminal, d'une manœuvre ou de la
/// mort). `load` : directement sur la liste des sauvegardes (captures de l'autopilote).
pub fn open(world: &mut World, load: bool) {
    let in_game = world
        .get_resource::<State<AppState>>()
        .is_some_and(|s| *s.get() == AppState::InGame);
    if !in_game
        || world.contains_resource::<Paused>()
        || world.contains_resource::<Terminal>()
        || world.contains_resource::<Death>()
        || world.contains_resource::<Sequence>()
    {
        return;
    }
    let resume = *world.resource::<State<ControlMode>>().get();
    if !matches!(
        resume,
        ControlMode::OnFoot | ControlMode::Piloting | ControlMode::Map
    ) {
        return;
    }
    world.insert_resource(Paused {
        resume,
        screen: if load {
            PauseScreen::Load
        } else {
            PauseScreen::Main
        },
        armed: false,
        closing: false,
        drawn: None,
    });
    world
        .resource_mut::<NextState<ControlMode>>()
        .set(ControlMode::Menu);
    world.resource_mut::<Time<Virtual>>().pause();
    let mut cursors = world.query_filtered::<&mut CursorOptions, With<PrimaryWindow>>();
    if let Ok(mut cursor) = cursors.single_mut(world) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Reprend la partie : temps, commandes, curseur.
fn resume(world: &mut World) {
    let Some(paused) = world.remove_resource::<Paused>() else {
        return;
    };
    world.resource_mut::<Time<Virtual>>().unpause();
    world
        .resource_mut::<NextState<ControlMode>>()
        .set(paused.resume);
    if matches!(paused.resume, ControlMode::OnFoot | ControlMode::Piloting) {
        let mut cursors = world.query_filtered::<&mut CursorOptions, With<PrimaryWindow>>();
        if let Ok(mut cursor) = cursors.single_mut(world) {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

fn close_on_escape(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut paused: ResMut<Paused>,
) {
    if !keys.pressed(KeyCode::Escape) && !paused.armed {
        paused.armed = true;
        return;
    }
    if paused.armed && keys.just_pressed(KeyCode::Escape) {
        match paused.screen {
            PauseScreen::Load => paused.screen = PauseScreen::Main,
            PauseScreen::Main => paused.closing = true,
        }
    }
    if paused.closing && !keys.pressed(KeyCode::Escape) {
        commands.queue(resume);
    }
}

fn on_activate(
    event: On<Activate>,
    mut commands: Commands,
    actions: Query<&PauseAction, Without<InteractionDisabled>>,
    paused: Option<ResMut<Paused>>,
    mode: Res<GameMode>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let (Ok(action), Some(mut paused)) = (actions.get(event.entity), paused) else {
        return;
    };
    match action {
        PauseAction::Resume => commands.queue(resume),
        PauseAction::Save => {
            commands.queue(|world: &mut World| {
                // La capture lit l'état de jeu : les commandes de pause n'y changent rien.
                save_now(world, SaveKind::Manual);
            });
        }
        PauseAction::Load => paused.screen = PauseScreen::Load,
        PauseAction::Back => paused.screen = PauseScreen::Main,
        PauseAction::Quit => {
            // Homme de fer : la partie se sauvegarde en partant.
            if mode.ironman {
                commands.queue(|world: &mut World| save_now(world, SaveKind::Auto));
            }
            next_state.set(AppState::MainMenu);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    texts: Res<Texts>,
    data: Res<Data>,
    fonts: Res<UiFonts>,
    mode: Res<GameMode>,
    mut paused: ResMut<Paused>,
    roots: Query<Entity, With<PauseUi>>,
) {
    if paused.drawn == Some(paused.screen) {
        return;
    }
    paused.drawn = Some(paused.screen);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            Name::new("Pause"),
            PauseUi,
            DespawnOnExit(AppState::InGame),
            GlobalZIndex(20),
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.025, 0.035, 0.82)),
        ))
        .id();
    let screen = paused.screen;
    commands.entity(root).with_children(|menu| {
        let title = match screen {
            PauseScreen::Main => "pause.title",
            PauseScreen::Load => "menu.load",
        };
        menu.spawn((
            Text::new(texts.get(title)),
            fonts.title(40.0),
            TextColor(theme::TEXT),
            Node {
                margin: UiRect::bottom(px(24)),
                ..default()
            },
        ));
        match screen {
            PauseScreen::Main => {
                let free_saves = !mode.ironman;
                for (key, action, enabled) in [
                    ("pause.resume", PauseAction::Resume, true),
                    ("pause.save", PauseAction::Save, free_saves),
                    ("pause.load", PauseAction::Load, free_saves),
                    ("pause.quit", PauseAction::Quit, true),
                ] {
                    let mut button =
                        menu.spawn(menu_button(&fonts, texts.get(key), action, enabled));
                    if !enabled {
                        button.insert(InteractionDisabled);
                    }
                }
                let hint = if mode.ironman {
                    "pause.hint_ironman"
                } else {
                    "pause.hint"
                };
                menu.spawn((
                    Text::new(texts.get(hint)),
                    fonts.body(16.0),
                    TextColor(theme::TEXT_DIM),
                    Node {
                        margin: UiRect::top(px(20)),
                        ..default()
                    },
                ));
            }
            PauseScreen::Load => {
                spawn_save_list(menu, &texts, &data, &fonts, &list_saves());
                menu.spawn(menu_button(
                    &fonts,
                    texts.get("menu.back"),
                    PauseAction::Back,
                    true,
                ));
            }
        }
    });
}

/// En quittant la partie (menu principal, chargement, mort) : plus de pause.
fn leave(mut commands: Commands, mut time: ResMut<Time<Virtual>>) {
    commands.remove_resource::<Paused>();
    time.unpause();
}
