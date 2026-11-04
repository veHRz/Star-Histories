//! Menu principal : Continuer (la dernière sauvegarde), Nouvelle partie (libre ou Homme de
//! fer), Charger, Options, Quitter. La liste des sauvegardes sert aussi au menu de pause.
//!
//! Habillage (M8) : écran d'appareil (équerres aux coins, vignette), titre en capitales
//! espacées sur un filet, boutons dans un panneau à pans coupés (`theme`).

use std::path::PathBuf;

use bevy::picking::events::{PointerOut, PointerOver};
use bevy::prelude::*;
use bevy::text::LetterSpacing;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;

use crate::data::{Data, Texts};
use crate::save::{GameMode, SaveEntry, list_saves, request_load};
use crate::state::{AppState, CurrentPlace, EnterGame, ViewLevel};
use crate::ui::theme::{self, Frame, MenuButton, UiFonts, menu_button};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<MenuScreen>()
            .add_systems(OnEnter(MenuScreen::Main), spawn_main_menu)
            .add_systems(OnEnter(MenuScreen::Load), spawn_load_screen)
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

/// Équerres des écrans pleins : retrait depuis le bord, longueur des branches.
const CORNER_INSET: f32 = 28.0;
const CORNER_LENGTH: f32 = 44.0;

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
        theme::spawn_corner_marks(menu, CORNER_INSET, CORNER_LENGTH, theme::SCREEN_DIM);
        menu.spawn(theme::heading(
            &fonts,
            &texts.get("app.title"),
            64.0,
            theme::TEXT,
        ));
        menu.spawn(theme::rule(
            px(520),
            UiRect::new(px(0), px(0), px(14), px(12)),
        ));
        // L'accroche garde sa casse (« 425at » est une notation canon) : seulement espacée.
        menu.spawn((
            Text::new(texts.get("menu.tagline")),
            fonts.body(18.0),
            LetterSpacing::Px(3.0),
            TextColor(theme::ACCENT),
        ));
        menu.spawn((
            Name::new("Commandes"),
            panel_node(UiRect::top(px(44))),
            fonts.frame(Frame::Panel),
        ))
        .with_children(|panel| {
            let mut continue_button = panel.spawn(menu_button(
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
                panel.spawn(menu_button(&fonts, texts.get(key), action, true));
            }
            let mut load_button = panel.spawn(menu_button(
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
                panel.spawn(menu_button(&fonts, texts.get(key), action, true));
            }
        });
    });
}

/// Le nœud d'un panneau de menu (cadre `Frame::Panel` à poser avec) : colonne centrée, marges
/// intérieures assez larges pour laisser voir la coupe ambre et les rivets.
pub fn panel_node(margin: UiRect) -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        padding: UiRect::axes(px(30), px(24)),
        margin,
        ..default()
    }
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
            Node {
                margin: UiRect::vertical(px(12)),
                ..default()
            },
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
                padding: UiRect::horizontal(px(18)),
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
        theme::spawn_corner_marks(menu, CORNER_INSET, CORNER_LENGTH, theme::SCREEN_DIM);
        menu.spawn(theme::heading(
            &fonts,
            &texts.get("menu.load"),
            40.0,
            theme::TEXT,
        ));
        menu.spawn(theme::rule(
            px(420),
            UiRect::new(px(0), px(0), px(12), px(22)),
        ));
        menu.spawn((panel_node(UiRect::ZERO), fonts.frame(Frame::Panel)))
            .with_children(|panel| {
                spawn_save_list(panel, &texts, &data, &fonts, &saves);
                panel
                    .spawn(menu_button(
                        &fonts,
                        texts.get("menu.back"),
                        MenuAction::Back,
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
    });
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
        MenuAction::Options => crate::ui::options::open(&mut commands),
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

/// Survol de tout bouton de menu (menus, pause, options) : cadre allumé ou fond d'accent.
fn on_hover(
    event: On<PointerOver>,
    fonts: Res<UiFonts>,
    mut buttons: Query<
        (Option<&mut ImageNode>, Option<&mut BackgroundColor>),
        (With<MenuButton>, Without<InteractionDisabled>),
    >,
) {
    if let Ok((image, background)) = buttons.get_mut(event.entity) {
        theme::set_hovered(&fonts, true, image, background);
    }
}

fn on_unhover(
    event: On<PointerOut>,
    fonts: Res<UiFonts>,
    mut buttons: Query<(Option<&mut ImageNode>, Option<&mut BackgroundColor>), With<MenuButton>>,
) {
    if let Ok((image, background)) = buttons.get_mut(event.entity) {
        theme::set_hovered(&fonts, false, image, background);
    }
}
