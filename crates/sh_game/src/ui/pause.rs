//! Menu de pause (Échap en partie) : reprendre, sauvegarder, charger, quitter vers le menu
//! principal. Le temps du jeu est suspendu (`Time<Virtual>`) : horloge du monde, physique,
//! combats. Il se ferme au relâchement d'Échap, comme le terminal : le contexte rendu ne voit
//! pas la touche encore enfoncée.
//!
//! Habillage (M8) : voile sombre sur la vue figée, équerres aux coins, titre en capitales sur
//! un filet, boutons dans le même panneau à pans coupés que le menu principal. Ses touches sont
//! dans l'encart des touches (M10), pas écrites sous le panneau.

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
use crate::ui::keys_panel::{KeyHelp, KeyHint};
use crate::ui::menu::{panel_node, spawn_save_list};
use crate::ui::options::OptionsScreen;
use crate::ui::terminal::Terminal;
use crate::ui::theme::{self, Frame, UiFonts, menu_button};

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_pause)
            .add_observer(on_activate)
            // Une condition par système, évaluée juste avant lui : posée sur le groupe chaîné,
            // elle ne le serait qu'une fois, et la fermeture (Échap) retire la pause entre
            // les deux systèmes.
            .add_systems(
                Update,
                (
                    close_on_escape
                        .run_if(resource_exists::<Paused>)
                        .run_if(not(resource_exists::<OptionsScreen>)),
                    render.run_if(resource_exists::<Paused>),
                )
                    .chain(),
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
    Options,
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
    // L'écran de pause part avec elle (il n'est redessiné que pendant la pause).
    let mut screens = world.query_filtered::<Entity, With<PauseUi>>();
    let screens: Vec<Entity> = screens.iter(world).collect();
    for entity in screens {
        world.entity_mut(entity).despawn();
    }
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
    paused: Option<ResMut<Paused>>,
) {
    let Some(mut paused) = paused else {
        return;
    };
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
        PauseAction::Options => crate::ui::options::open(&mut commands),
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
    paused: Option<ResMut<Paused>>,
    roots: Query<Entity, With<PauseUi>>,
) {
    let Some(mut paused) = paused else {
        return;
    };
    if paused.drawn == Some(paused.screen) {
        return;
    }
    paused.drawn = Some(paused.screen);
    for root in &roots {
        commands.entity(root).despawn();
    }
    // Les gestes de l'écran, dans l'encart des touches (M10) : Échap reprend la partie, ou
    // revient de la liste des sauvegardes.
    let escape = match paused.screen {
        PauseScreen::Main => "pause.help.resume",
        PauseScreen::Load => "pause.help.back",
    };
    let help = KeyHelp::new([
        KeyHint::mouse(MouseButton::Left, texts.get("pause.help.choose")),
        KeyHint::key(KeyCode::Escape, texts.get(escape)),
    ]);
    let root = commands
        .spawn((
            Name::new("Pause"),
            PauseUi,
            help,
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
            // Voile sombre : la vue reste devinée derrière, le jeu est figé.
            BackgroundColor(Color::srgba(0.015, 0.018, 0.024, 0.80)),
        ))
        .id();
    let screen = paused.screen;
    commands.entity(root).with_children(|menu| {
        theme::spawn_corner_marks(menu, 28.0, 44.0, theme::SCREEN_DIM);
        let title = match screen {
            PauseScreen::Main => "pause.title",
            PauseScreen::Load => "menu.load",
        };
        menu.spawn(theme::heading(&fonts, &texts.get(title), 40.0, theme::TEXT));
        menu.spawn(theme::rule(
            px(420),
            UiRect::new(px(0), px(0), px(12), px(22)),
        ));
        let mut panel = menu.spawn((panel_node(UiRect::ZERO), fonts.frame(Frame::Panel)));
        match screen {
            PauseScreen::Main => {
                let free_saves = !mode.ironman;
                panel.with_children(|panel| {
                    for (key, action, enabled) in [
                        ("pause.resume", PauseAction::Resume, true),
                        ("pause.save", PauseAction::Save, free_saves),
                        ("pause.load", PauseAction::Load, free_saves),
                        ("menu.options", PauseAction::Options, true),
                        ("pause.quit", PauseAction::Quit, true),
                    ] {
                        let mut button =
                            panel.spawn(menu_button(&fonts, texts.get(key), action, enabled));
                        if !enabled {
                            button.insert(InteractionDisabled);
                        }
                    }
                });
                let note = if mode.ironman {
                    "pause.note_ironman"
                } else {
                    "pause.note"
                };
                // Sous le panneau, hors du cadre : la note ne doit pas l'élargir. Les touches
                // sont dans l'encart.
                menu.spawn((
                    Text::new(texts.get(note)),
                    fonts.body(16.0),
                    TextColor(theme::TEXT_DIM),
                    Node {
                        margin: UiRect::top(px(20)),
                        ..default()
                    },
                ));
            }
            PauseScreen::Load => {
                panel.with_children(|panel| {
                    spawn_save_list(panel, &texts, &data, &fonts, &list_saves());
                    panel
                        .spawn(menu_button(
                            &fonts,
                            texts.get("menu.back"),
                            PauseAction::Back,
                            true,
                        ))
                        .insert(Node {
                            width: px(400),
                            height: px(52),
                            margin: UiRect::top(px(20)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(px(18)),
                            ..default()
                        });
                });
            }
        }
    });
}

/// En quittant la partie (menu principal, chargement, mort) : plus de pause.
fn leave(mut commands: Commands, mut time: ResMut<Time<Virtual>>) {
    commands.remove_resource::<Paused>();
    time.unpause();
}

/// Le bouton « Reprendre » ou « Options » du menu de pause (scénarios de l'autopilote).
pub fn find_button(world: &mut World, options: bool) -> Option<Entity> {
    let wanted = if options {
        PauseAction::Options
    } else {
        PauseAction::Resume
    };
    let mut buttons = world.query::<(Entity, &PauseAction)>();
    buttons
        .iter(world)
        .find(|(_, action)| **action == wanted)
        .map(|(entity, _)| entity)
}

/// L'écran de pause est-il affiché ?
pub fn is_shown(world: &mut World) -> bool {
    let mut screens = world.query_filtered::<(), With<PauseUi>>();
    screens.iter(world).next().is_some()
}
