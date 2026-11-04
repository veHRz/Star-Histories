//! Menu principal : Continuer (la dernière sauvegarde), Nouvelle partie (libre ou Homme de
//! fer), Charger, Options, Quitter. La liste des sauvegardes sert aussi au menu de pause.

use std::path::PathBuf;

use bevy::picking::events::{PointerOut, PointerOver};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;

use crate::data::{Data, Texts};
use crate::save::{GameMode, SaveEntry, list_saves, request_load};
use crate::state::{AppState, CurrentPlace, EnterGame, ViewLevel};
use crate::ui::theme::{self, MenuButton, UiFonts, menu_button};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<MenuScreen>()
            .add_systems(OnEnter(MenuScreen::Main), spawn_main_menu)
            .add_systems(OnEnter(MenuScreen::Load), spawn_load_screen)
            .add_systems(OnEnter(MenuScreen::Options), spawn_options)
            .add_observer(on_activate)
            .add_observer(on_load_button)
            .add_observer(on_hover)
            .add_observer(on_unhover);
    }
}

#[derive(SubStates, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::MainMenu)]
pub enum MenuScreen {
    #[default]
    Main,
    Load,
    Options,
}

/// Ce que fait un bouton de menu.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    NewGame,
    NewGameIronman,
    Continue,
    Load,
    Options,
    Back,
    Quit,
}

/// Un bouton de la liste des sauvegardes : charge ce fichier.
#[derive(Component, Clone, Debug)]
pub struct LoadButton(pub PathBuf);

/// Sauvegardes affichées au plus dans une liste.
const SAVES_SHOWN: usize = 8;

fn spawn_main_menu(mut commands: Commands, texts: Res<Texts>, fonts: Res<UiFonts>) {
    let has_save = !list_saves().is_empty();
    let root = commands
        .spawn((
            Name::new("Menu principal"),
            DespawnOnExit(MenuScreen::Main),
            theme::fullscreen_backdrop(),
        ))
        .id();
    commands.entity(root).with_children(|menu| {
        menu.spawn((
            Text::new(texts.get("app.title")),
            fonts.title(64.0),
            TextColor(theme::TEXT),
        ));
        menu.spawn((
            Text::new(texts.get("menu.tagline")),
            fonts.body(18.0),
            TextColor(theme::ACCENT),
            Node {
                margin: UiRect::bottom(px(48)),
                ..default()
            },
        ));
        let mut continue_button = menu.spawn(menu_button(
            &fonts,
            texts.get("menu.continue"),
            MenuAction::Continue,
            has_save,
        ));
        if !has_save {
            continue_button.insert(InteractionDisabled);
        }
        for (key, action) in [
            ("menu.new_game", MenuAction::NewGame),
            ("menu.new_game_ironman", MenuAction::NewGameIronman),
        ] {
            menu.spawn(menu_button(&fonts, texts.get(key), action, true));
        }
        let mut load_button = menu.spawn(menu_button(
            &fonts,
            texts.get("menu.load"),
            MenuAction::Load,
            has_save,
        ));
        if !has_save {
            load_button.insert(InteractionDisabled);
        }
        for (key, action) in [
            ("menu.options", MenuAction::Options),
            ("menu.quit", MenuAction::Quit),
        ] {
            menu.spawn(menu_button(&fonts, texts.get(key), action, true));
        }
    });
}

/// La liste des sauvegardes, la plus récente en premier.
pub fn spawn_save_list(
    parent: &mut ChildSpawnerCommands,
    texts: &Texts,
    data: &Data,
    fonts: &UiFonts,
    saves: &[SaveEntry],
) {
    if saves.is_empty() {
        parent.spawn((
            Text::new(texts.get("menu.load.empty")),
            fonts.body(18.0),
            TextColor(theme::TEXT_DIM),
        ));
    }
    for entry in saves.iter().take(SAVES_SHOWN) {
        parent
            .spawn(menu_button(
                fonts,
                crate::save::describe(texts, data, &entry.meta),
                LoadButton(entry.path.clone()),
                true,
            ))
            .insert(Node {
                width: px(620),
                height: px(44),
                margin: UiRect::vertical(px(4)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                ..default()
            });
    }
}

fn spawn_load_screen(
    mut commands: Commands,
    texts: Res<Texts>,
    data: Res<Data>,
    fonts: Res<UiFonts>,
) {
    let saves = list_saves();
    let root = commands
        .spawn((
            Name::new("Charger une partie"),
            DespawnOnExit(MenuScreen::Load),
            theme::fullscreen_backdrop(),
        ))
        .id();
    commands.entity(root).with_children(|menu| {
        menu.spawn((
            Text::new(texts.get("menu.load")),
            fonts.title(40.0),
            TextColor(theme::TEXT),
            Node {
                margin: UiRect::bottom(px(24)),
                ..default()
            },
        ));
        spawn_save_list(menu, &texts, &data, &fonts, &saves);
        menu.spawn(menu_button(
            &fonts,
            texts.get("menu.back"),
            MenuAction::Back,
            true,
        ))
        .insert(Node {
            width: px(400),
            height: px(52),
            margin: UiRect::top(px(24)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(1)),
            ..default()
        });
    });
}

fn spawn_options(mut commands: Commands, texts: Res<Texts>, fonts: Res<UiFonts>) {
    commands.spawn((
        Name::new("Options"),
        DespawnOnExit(MenuScreen::Options),
        theme::fullscreen_backdrop(),
        children![
            (
                Text::new(texts.get("menu.options")),
                fonts.title(40.0),
                TextColor(theme::TEXT),
            ),
            (
                Text::new(texts.get("menu.options.empty")),
                fonts.body(18.0),
                TextColor(theme::TEXT_DIM),
                Node {
                    margin: UiRect::vertical(px(32)),
                    ..default()
                },
            ),
            menu_button(&fonts, texts.get("menu.back"), MenuAction::Back, true),
        ],
    ));
}

fn on_activate(
    event: On<Activate>,
    mut commands: Commands,
    actions: Query<&MenuAction, Without<InteractionDisabled>>,
    mut screen: ResMut<NextState<MenuScreen>>,
    mut app_state: ResMut<NextState<AppState>>,
    mut view: ResMut<NextState<ViewLevel>>,
    mut place: ResMut<CurrentPlace>,
    data: Res<Data>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    match action {
        MenuAction::NewGame | MenuAction::NewGameIronman => {
            commands.insert_resource(GameMode {
                ironman: *action == MenuAction::NewGameIronman,
            });
            EnterGame::new_game(&data).apply(&mut app_state, &mut view, &mut place);
        }
        MenuAction::Continue => {
            if let Some(latest) = list_saves().into_iter().next() {
                commands.queue(move |world: &mut World| request_load(world, &latest.path));
            }
        }
        MenuAction::Load => screen.set(MenuScreen::Load),
        MenuAction::Options => screen.set(MenuScreen::Options),
        MenuAction::Back => screen.set(MenuScreen::Main),
        MenuAction::Quit => {
            exit.write(AppExit::Success);
        }
    }
}

fn on_load_button(event: On<Activate>, mut commands: Commands, buttons: Query<&LoadButton>) {
    if let Ok(LoadButton(path)) = buttons.get(event.entity) {
        let path = path.clone();
        commands.queue(move |world: &mut World| request_load(world, &path));
    }
}

fn on_hover(
    event: On<PointerOver>,
    mut buttons: Query<&mut BackgroundColor, (With<MenuButton>, Without<InteractionDisabled>)>,
) {
    if let Ok(mut color) = buttons.get_mut(event.entity) {
        *color = theme::BUTTON_HOVER.into();
    }
}

fn on_unhover(event: On<PointerOut>, mut buttons: Query<&mut BackgroundColor, With<MenuButton>>) {
    if let Ok(mut color) = buttons.get_mut(event.entity) {
        *color = theme::BUTTON.into();
    }
}
