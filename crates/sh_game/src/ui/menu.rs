//! Menu principal : Nouvelle partie, Continuer (grisé tant qu'il n'y a pas de sauvegarde),
//! Options (vide en M0), Quitter.

use bevy::picking::events::{PointerOut, PointerOver};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};

use crate::data::Texts;
use crate::state::{AppState, CurrentPlace, EnterGame, ViewLevel};
use crate::ui::theme::{self, UiFonts};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<MenuScreen>()
            .add_systems(OnEnter(MenuScreen::Main), spawn_main_menu)
            .add_systems(OnEnter(MenuScreen::Options), spawn_options)
            .add_observer(on_activate)
            .add_observer(on_hover)
            .add_observer(on_unhover);
    }
}

#[derive(SubStates, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::MainMenu)]
pub enum MenuScreen {
    #[default]
    Main,
    Options,
}

/// Ce que fait un bouton de menu.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    NewGame,
    Continue,
    Options,
    Back,
    Quit,
}

/// Où commence une nouvelle partie : à bord du vaisseau du joueur (P1, P12). L'amarrage à Hexis
/// arrive avec M2.
fn new_game() -> EnterGame {
    EnterGame {
        view: ViewLevel::Sector,
        place: Some("player_ship".into()),
    }
}

fn menu_button(fonts: &UiFonts, label: String, action: MenuAction, enabled: bool) -> impl Bundle {
    let text_color = if enabled {
        theme::TEXT
    } else {
        theme::TEXT_DISABLED
    };
    (
        Button,
        ActivateOnPress,
        action,
        Node {
            width: px(320),
            height: px(52),
            margin: UiRect::vertical(px(6)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(theme::BUTTON_BORDER),
        BackgroundColor(theme::BUTTON),
        children![(Text::new(label), fonts.body(22.0), TextColor(text_color))],
    )
}

fn spawn_main_menu(mut commands: Commands, texts: Res<Texts>, fonts: Res<UiFonts>) {
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
        menu.spawn(menu_button(
            &fonts,
            texts.get("menu.new_game"),
            MenuAction::NewGame,
            true,
        ));
        // Pas de sauvegarde avant M7 : « Continuer » reste grisé.
        menu.spawn((
            menu_button(
                &fonts,
                texts.get("menu.continue"),
                MenuAction::Continue,
                false,
            ),
            InteractionDisabled,
        ));
        menu.spawn(menu_button(
            &fonts,
            texts.get("menu.options"),
            MenuAction::Options,
            true,
        ));
        menu.spawn(menu_button(
            &fonts,
            texts.get("menu.quit"),
            MenuAction::Quit,
            true,
        ));
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
    actions: Query<&MenuAction, Without<InteractionDisabled>>,
    mut screen: ResMut<NextState<MenuScreen>>,
    mut app_state: ResMut<NextState<AppState>>,
    mut view: ResMut<NextState<ViewLevel>>,
    mut place: ResMut<CurrentPlace>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    match action {
        MenuAction::NewGame => new_game().apply(&mut app_state, &mut view, &mut place),
        MenuAction::Continue => {}
        MenuAction::Options => screen.set(MenuScreen::Options),
        MenuAction::Back => screen.set(MenuScreen::Main),
        MenuAction::Quit => {
            exit.write(AppExit::Success);
        }
    }
}

fn on_hover(
    event: On<PointerOver>,
    mut buttons: Query<&mut BackgroundColor, (With<MenuAction>, Without<InteractionDisabled>)>,
) {
    if let Ok(mut color) = buttons.get_mut(event.entity) {
        *color = theme::BUTTON_HOVER.into();
    }
}

fn on_unhover(event: On<PointerOut>, mut buttons: Query<&mut BackgroundColor, With<MenuAction>>) {
    if let Ok(mut color) = buttons.get_mut(event.entity) {
        *color = theme::BUTTON.into();
    }
}
