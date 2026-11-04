//! Écran des options, commun au menu principal et au menu de pause : qualité graphique, plein
//! écran, volumes, souris, puis la page des touches (remappage par touche, voir `options`).
//! Échap revient en arrière (annule d'abord une touche en attente) ; l'écran se ferme au
//! relâchement, comme le terminal, pour que l'écran dessous ne voie pas la touche.

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};
use sh_data::PresetLevel;

use crate::data::Texts;
use crate::options::{REMAPPABLE, UserSettings, choosable, key_name};
use crate::ui::theme::{self, MenuButton, UiFonts, menu_button};

pub struct OptionsUiPlugin;

impl Plugin for OptionsUiPlugin {
    fn build(&self, app: &mut App) {
        // Une condition par système, évaluée juste avant lui (posée sur le groupe chaîné, elle
        // ne le serait qu'une fois, et `escape` ferme l'écran entre deux systèmes). Le rendu
        // tourne aussi écran fermé : il efface ce qui en reste.
        app.add_observer(on_activate).add_systems(
            Update,
            (
                capture_key.run_if(resource_exists::<OptionsScreen>),
                escape.run_if(resource_exists::<OptionsScreen>),
                render.run_if(resource_exists::<Texts>),
            )
                .chain(),
        );
    }
}

/// L'écran des options est ouvert.
#[derive(Resource, Default)]
pub struct OptionsScreen {
    page: Page,
    /// Touche d'origine dont on attend la nouvelle touche.
    waiting: Option<KeyCode>,
    closing: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Main,
    Keys,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Channel {
    Master,
    Music,
    Effects,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
enum OptionsAction {
    Preset(PresetLevel),
    Fullscreen,
    Volume(Channel, f32),
    Sensitivity(f32),
    InvertY,
    Tips,
    Keys,
    Rebind(KeyCode),
    ResetKeys,
    Back,
}

#[derive(Component)]
struct OptionsUi;

/// Ouvre l'écran des options par-dessus l'écran courant.
pub fn open(commands: &mut Commands) {
    commands.insert_resource(OptionsScreen::default());
}

/// Ouvre directement la page des touches (captures de l'autopilote).
pub fn open_keys(commands: &mut Commands) {
    commands.insert_resource(OptionsScreen {
        page: Page::Keys,
        ..default()
    });
}

fn on_activate(
    event: On<Activate>,
    actions: Query<&OptionsAction>,
    screen: Option<ResMut<OptionsScreen>>,
    mut settings: ResMut<UserSettings>,
) {
    let (Ok(action), Some(mut screen)) = (actions.get(event.entity), screen) else {
        return;
    };
    if screen.waiting.is_some() {
        return;
    }
    match *action {
        OptionsAction::Preset(level) => settings.preset = level,
        OptionsAction::Fullscreen => settings.fullscreen = !settings.fullscreen,
        OptionsAction::Volume(channel, delta) => {
            let volume = match channel {
                Channel::Master => &mut settings.master_volume,
                Channel::Music => &mut settings.music_volume,
                Channel::Effects => &mut settings.effects_volume,
            };
            *volume = (*volume + delta).clamp(0.0, 1.0);
        }
        OptionsAction::Sensitivity(delta) => {
            settings.mouse_sensitivity = (settings.mouse_sensitivity + delta).clamp(0.2, 3.0);
        }
        OptionsAction::InvertY => settings.invert_y = !settings.invert_y,
        OptionsAction::Tips => settings.tips = !settings.tips,
        OptionsAction::Keys => screen.page = Page::Keys,
        OptionsAction::Rebind(default) => screen.waiting = Some(default),
        OptionsAction::ResetKeys => settings.keys.clear(),
        OptionsAction::Back => match screen.page {
            Page::Keys => screen.page = Page::Main,
            Page::Main => screen.closing = true,
        },
    }
}

/// La touche pressée devient celle de l'action en attente (Échap annule, dans `escape`).
fn capture_key(
    mut reader: MessageReader<KeyboardInput>,
    screen: Option<ResMut<OptionsScreen>>,
    mut settings: ResMut<UserSettings>,
) {
    let Some(mut screen) = screen else {
        return;
    };
    let Some(default) = screen.waiting else {
        reader.clear();
        return;
    };
    for input in reader.read() {
        if input.state != ButtonState::Pressed || input.repeat {
            continue;
        }
        if choosable(input.key_code) {
            settings.assign(default, input.key_code);
            screen.waiting = None;
            return;
        }
    }
}

fn escape(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    screen: Option<ResMut<OptionsScreen>>,
) {
    let Some(mut screen) = screen else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        if screen.waiting.is_some() {
            screen.waiting = None;
        } else {
            match screen.page {
                Page::Keys => screen.page = Page::Main,
                Page::Main => screen.closing = true,
            }
        }
    }
    if screen.closing && !keys.pressed(KeyCode::Escape) {
        commands.remove_resource::<OptionsScreen>();
    }
}

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    settings: Res<UserSettings>,
    screen: Option<Res<OptionsScreen>>,
    roots: Query<Entity, With<OptionsUi>>,
    mut drawn: Local<bool>,
) {
    // Écran fermé (Échap, Retour, autre chemin) : ce qui reste affiché part ici.
    let Some(screen) = screen else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *drawn = false;
        return;
    };
    if *drawn && !screen.is_changed() && !settings.is_changed() && !texts.is_changed() {
        return;
    }
    *drawn = true;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if screen.closing {
        *drawn = false;
        return;
    }
    let root = commands
        .spawn((
            Name::new("Options"),
            OptionsUi,
            GlobalZIndex(30),
            theme::fullscreen_backdrop(),
        ))
        .id();
    commands.entity(root).with_children(|ui| {
        let title = match screen.page {
            Page::Main => "menu.options",
            Page::Keys => "options.keys.title",
        };
        ui.spawn((
            Text::new(texts.get(title)),
            fonts.title(40.0),
            TextColor(theme::TEXT),
            Node {
                margin: UiRect::bottom(px(20)),
                ..default()
            },
        ));
        match screen.page {
            Page::Main => main_page(ui, &texts, &fonts, &settings),
            Page::Keys => keys_page(ui, &texts, &fonts, &settings, screen.waiting),
        }
        ui.spawn(menu_button(
            &fonts,
            texts.get("menu.back"),
            OptionsAction::Back,
            true,
        ));
    });
}

fn main_page(ui: &mut ChildSpawnerCommands, texts: &Texts, fonts: &UiFonts, s: &UserSettings) {
    let percent = |v: f32| format!("{} %", texts.number(f64::from(v) * 100.0, 0));
    row(ui, fonts, texts.get("options.graphics"), |r| {
        for (level, key) in [
            (PresetLevel::Low, "options.preset.low"),
            (PresetLevel::Medium, "options.preset.medium"),
            (PresetLevel::High, "options.preset.high"),
        ] {
            r.spawn(small_button(
                fonts,
                texts.get(key),
                OptionsAction::Preset(level),
                s.preset == level,
            ));
        }
    });
    row(ui, fonts, texts.get("options.fullscreen"), |r| {
        r.spawn(toggle(
            fonts,
            texts,
            OptionsAction::Fullscreen,
            s.fullscreen,
        ));
    });
    for (key, channel, value) in [
        ("options.master_volume", Channel::Master, s.master_volume),
        ("options.music_volume", Channel::Music, s.music_volume),
        ("options.effects_volume", Channel::Effects, s.effects_volume),
    ] {
        row(ui, fonts, texts.get(key), |r| {
            r.spawn(small_button(
                fonts,
                "−".into(),
                OptionsAction::Volume(channel, -0.1),
                false,
            ));
            r.spawn(value_text(fonts, percent(value)));
            r.spawn(small_button(
                fonts,
                "+".into(),
                OptionsAction::Volume(channel, 0.1),
                false,
            ));
        });
    }
    row(ui, fonts, texts.get("options.sensitivity"), |r| {
        r.spawn(small_button(
            fonts,
            "−".into(),
            OptionsAction::Sensitivity(-0.1),
            false,
        ));
        r.spawn(value_text(
            fonts,
            texts.number(f64::from(s.mouse_sensitivity), 1),
        ));
        r.spawn(small_button(
            fonts,
            "+".into(),
            OptionsAction::Sensitivity(0.1),
            false,
        ));
    });
    row(ui, fonts, texts.get("options.invert_y"), |r| {
        r.spawn(toggle(fonts, texts, OptionsAction::InvertY, s.invert_y));
    });
    row(ui, fonts, texts.get("options.tips"), |r| {
        r.spawn(toggle(fonts, texts, OptionsAction::Tips, s.tips));
    });
    ui.spawn(menu_button(
        fonts,
        texts.get("options.keys"),
        OptionsAction::Keys,
        true,
    ));
    ui.spawn((
        Text::new(texts.get("options.hint")),
        fonts.body(15.0),
        TextColor(theme::TEXT_DIM),
        Node {
            margin: UiRect::vertical(px(10)),
            ..default()
        },
    ));
}

fn keys_page(
    ui: &mut ChildSpawnerCommands,
    texts: &Texts,
    fonts: &UiFonts,
    s: &UserSettings,
    waiting: Option<KeyCode>,
) {
    ui.spawn((
        Text::new(texts.get("options.keys.hint")),
        fonts.body(15.0),
        TextColor(theme::TEXT_DIM),
        Node {
            margin: UiRect::bottom(px(12)),
            ..default()
        },
    ));
    // Deux colonnes : la liste tient sans défilement.
    ui.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(28),
        ..default()
    })
    .with_children(|columns| {
        let half = REMAPPABLE.len().div_ceil(2);
        for chunk in REMAPPABLE.chunks(half) {
            columns
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|column| {
                    for original in chunk {
                        let label = if waiting == Some(*original) {
                            texts.get("options.keys.waiting")
                        } else {
                            texts.key_label(&key_name(s.key_for(*original)))
                        };
                        column
                            .spawn(Node {
                                width: px(470),
                                height: px(38),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::SpaceBetween,
                                ..default()
                            })
                            .with_children(|line| {
                                line.spawn((
                                    Text::new(
                                        texts.get(&format!("options.key.{}", key_name(*original))),
                                    ),
                                    fonts.body(15.0),
                                    TextColor(theme::TEXT),
                                ));
                                line.spawn(small_button(
                                    fonts,
                                    label,
                                    OptionsAction::Rebind(*original),
                                    waiting == Some(*original),
                                ));
                            });
                    }
                });
        }
    });
    ui.spawn(menu_button(
        fonts,
        texts.get("options.keys.reset"),
        OptionsAction::ResetKeys,
        true,
    ));
}

/// Une ligne : l'intitulé à gauche, les commandes à droite.
fn row(
    ui: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    label: String,
    controls: impl FnOnce(&mut ChildSpawnerCommands),
) {
    ui.spawn(Node {
        width: px(640),
        height: px(48),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    })
    .with_children(|line| {
        line.spawn((Text::new(label), fonts.body(19.0), TextColor(theme::TEXT)));
        line.spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            ..default()
        })
        .with_children(controls);
    });
}

fn value_text(fonts: &UiFonts, text: String) -> impl Bundle {
    (
        Text::new(text),
        fonts.body(18.0),
        TextColor(theme::TEXT),
        Node {
            width: px(70),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextLayout::justify(Justify::Center),
    )
}

fn toggle(fonts: &UiFonts, texts: &Texts, action: OptionsAction, on: bool) -> impl Bundle {
    let key = if on { "options.on" } else { "options.off" };
    small_button(fonts, texts.get(key), action, on)
}

/// Petit bouton ; `selected` : bordure d'accent (choix courant, touche en attente).
fn small_button(
    fonts: &UiFonts,
    label: String,
    action: OptionsAction,
    selected: bool,
) -> impl Bundle {
    (
        MenuButton,
        Button,
        ActivateOnPress,
        action,
        Node {
            min_width: px(44),
            height: px(34),
            padding: UiRect::horizontal(px(12)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(if selected { 2.0 } else { 1.0 })),
            ..default()
        },
        BorderColor::all(if selected {
            theme::ACCENT
        } else {
            theme::BUTTON_BORDER
        }),
        BackgroundColor(theme::BUTTON),
        children![(Text::new(label), fonts.body(17.0), TextColor(theme::TEXT))],
    )
}

/// Le bouton « Retour » de l'écran des options (scénarios de l'autopilote).
pub fn find_back_button(world: &mut World) -> Option<Entity> {
    let mut buttons = world.query::<(Entity, &OptionsAction)>();
    buttons
        .iter(world)
        .find(|(_, action)| **action == OptionsAction::Back)
        .map(|(entity, _)| entity)
}

/// L'écran des options est-il affiché ?
pub fn is_shown(world: &mut World) -> bool {
    let mut screens = world.query_filtered::<(), With<OptionsUi>>();
    screens.iter(world).next().is_some()
}
