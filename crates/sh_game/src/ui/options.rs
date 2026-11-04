//! Écran des options, commun au menu principal et au menu de pause : qualité graphique, plein
//! écran, volumes, souris, puis la page des touches (remappage par touche, voir `options`).
//! Échap revient en arrière (annule d'abord une touche en attente) ; l'écran se ferme au
//! relâchement, comme le terminal, pour que l'écran dessous ne voie pas la touche.
//!
//! Habillage (M8) : l'écran d'appareil du menu de pause (équerres, titre en capitales sur un
//! filet), les réglages dans un panneau à pans coupés, une ligne par réglage séparée d'un filet,
//! petits boutons à cadre (le choix courant allumé en ambre), valeurs en relevé à chasse fixe.

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};
use sh_data::PresetLevel;

use crate::data::Texts;
use crate::options::{REMAPPABLE, UserSettings, choosable, key_name};
use crate::ui::menu::panel_node;
use crate::ui::theme::{self, Frame, MenuButton, UiFonts, menu_button};

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
    Tutorial,
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
        OptionsAction::Tutorial => settings.tutorial = !settings.tutorial,
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
        theme::spawn_corner_marks(ui, 28.0, 44.0, theme::SCREEN_DIM);
        let title = match screen.page {
            Page::Main => "menu.options",
            Page::Keys => "options.keys.title",
        };
        ui.spawn(theme::heading(&fonts, &texts.get(title), 40.0, theme::TEXT));
        ui.spawn(theme::rule(
            px(420),
            UiRect::new(px(0), px(0), px(12), px(20)),
        ));
        ui.spawn((panel_node(UiRect::ZERO), fonts.frame(Frame::Panel)))
            .with_children(|panel| {
                let secondary = match screen.page {
                    Page::Main => {
                        main_page(panel, &texts, &fonts, &settings);
                        (texts.get("options.keys"), OptionsAction::Keys)
                    }
                    Page::Keys => {
                        keys_page(panel, &texts, &fonts, &settings, screen.waiting);
                        (texts.get("options.keys.reset"), OptionsAction::ResetKeys)
                    }
                };
                // Les deux boutons du bas, côte à côte : la page des touches tient en hauteur.
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(16),
                        margin: UiRect::top(px(14)),
                        ..default()
                    })
                    .with_children(|buttons| {
                        for (label, action) in
                            [secondary, (texts.get("menu.back"), OptionsAction::Back)]
                        {
                            buttons
                                .spawn(menu_button(&fonts, label, action, true))
                                .insert(Node {
                                    width: px(FOOT_BUTTON_WIDTH),
                                    height: px(48),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::horizontal(px(18)),
                                    ..default()
                                });
                        }
                    });
            });
        // La consigne de la page, sous le panneau : longue, elle ne doit pas l'élargir.
        let hint = match screen.page {
            Page::Main => "options.hint",
            Page::Keys => "options.keys.hint",
        };
        ui.spawn((
            Text::new(texts.get(hint)),
            fonts.body(15.0),
            TextColor(theme::TEXT_DIM),
            TextLayout::justify(Justify::Center),
            Node {
                max_width: px(900),
                margin: UiRect::top(px(16)),
                ..default()
            },
        ));
    });
}

/// Largeur des deux boutons du bas (« Touches… » ou « Touches par défaut », « Retour »).
const FOOT_BUTTON_WIDTH: f32 = 300.0;
/// Largeur d'une ligne de la page principale (intitulé à gauche, commandes à droite).
const ROW_WIDTH: f32 = 640.0;
/// Largeur d'une ligne de la page des touches (deux colonnes).
const KEY_ROW_WIDTH: f32 = 470.0;

fn main_page(ui: &mut ChildSpawnerCommands, texts: &Texts, fonts: &UiFonts, s: &UserSettings) {
    let percent = |v: f32| format!("{} %", texts.number(f64::from(v) * 100.0, 0));
    row(ui, fonts, texts.get("options.graphics"), true, |r| {
        for (level, key) in [
            (PresetLevel::Low, "options.preset.low"),
            (PresetLevel::Medium, "options.preset.medium"),
            (PresetLevel::High, "options.preset.high"),
        ] {
            spawn_small(
                r,
                fonts,
                texts.get(key),
                OptionsAction::Preset(level),
                s.preset == level,
            );
        }
    });
    row(ui, fonts, texts.get("options.fullscreen"), true, |r| {
        toggle(r, fonts, texts, OptionsAction::Fullscreen, s.fullscreen);
    });
    for (key, channel, value) in [
        ("options.master_volume", Channel::Master, s.master_volume),
        ("options.music_volume", Channel::Music, s.music_volume),
        ("options.effects_volume", Channel::Effects, s.effects_volume),
    ] {
        row(ui, fonts, texts.get(key), true, |r| {
            spawn_small(
                r,
                fonts,
                "−".into(),
                OptionsAction::Volume(channel, -0.1),
                false,
            );
            r.spawn(value_text(fonts, percent(value)));
            spawn_small(
                r,
                fonts,
                "+".into(),
                OptionsAction::Volume(channel, 0.1),
                false,
            );
        });
    }
    row(ui, fonts, texts.get("options.sensitivity"), true, |r| {
        spawn_small(
            r,
            fonts,
            "−".into(),
            OptionsAction::Sensitivity(-0.1),
            false,
        );
        r.spawn(value_text(
            fonts,
            texts.number(f64::from(s.mouse_sensitivity), 1),
        ));
        spawn_small(r, fonts, "+".into(), OptionsAction::Sensitivity(0.1), false);
    });
    row(ui, fonts, texts.get("options.invert_y"), true, |r| {
        toggle(r, fonts, texts, OptionsAction::InvertY, s.invert_y);
    });
    row(ui, fonts, texts.get("options.tips"), true, |r| {
        toggle(r, fonts, texts, OptionsAction::Tips, s.tips);
    });
    row(ui, fonts, texts.get("options.tutorial"), false, |r| {
        toggle(r, fonts, texts, OptionsAction::Tutorial, s.tutorial);
    });
}

fn keys_page(
    ui: &mut ChildSpawnerCommands,
    texts: &Texts,
    fonts: &UiFonts,
    s: &UserSettings,
    waiting: Option<KeyCode>,
) {
    // Deux colonnes séparées d'un filet vertical : la liste tient sans défilement.
    ui.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(18),
        ..default()
    })
    .with_children(|columns| {
        let half = REMAPPABLE.len().div_ceil(2);
        for (index, chunk) in REMAPPABLE.chunks(half).enumerate() {
            if index > 0 {
                columns.spawn((
                    Node {
                        width: px(1),
                        align_self: AlignSelf::Stretch,
                        ..default()
                    },
                    BackgroundColor(theme::RULE),
                ));
            }
            columns
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|column| {
                    for (i, original) in chunk.iter().enumerate() {
                        let label = if waiting == Some(*original) {
                            texts.get("options.keys.waiting")
                        } else {
                            texts.key_label(&key_name(s.key_for(*original)))
                        };
                        column
                            .spawn(Node {
                                width: px(KEY_ROW_WIDTH),
                                height: px(31),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::SpaceBetween,
                                column_gap: px(12),
                                // Le filet sous chaque ligne, sauf la dernière.
                                border: UiRect::bottom(px(if i + 1 < chunk.len() {
                                    1.0
                                } else {
                                    0.0
                                })),
                                ..default()
                            })
                            .insert(BorderColor::all(ROW_RULE))
                            .with_children(|line| {
                                line.spawn((
                                    Text::new(
                                        texts.get(&format!("options.key.{}", key_name(*original))),
                                    ),
                                    fonts.body(15.0),
                                    TextColor(theme::TEXT),
                                ));
                                spawn_small(
                                    line,
                                    fonts,
                                    label,
                                    OptionsAction::Rebind(*original),
                                    waiting == Some(*original),
                                )
                                .insert(Node {
                                    min_width: px(52),
                                    height: px(27),
                                    padding: UiRect::new(px(14), px(12), px(0), px(0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    flex_shrink: 0.0,
                                    ..default()
                                });
                            });
                    }
                });
        }
    });
}

/// Filet entre deux lignes d'options.
const ROW_RULE: Color = Color::srgba(0.42, 0.45, 0.49, 0.22);

/// Une ligne : l'intitulé à gauche, les commandes à droite ; `rule` : un filet dessous.
fn row(
    ui: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    label: String,
    rule: bool,
    controls: impl FnOnce(&mut ChildSpawnerCommands),
) {
    ui.spawn((
        Node {
            width: px(ROW_WIDTH),
            height: px(46),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border: UiRect::bottom(px(if rule { 1.0 } else { 0.0 })),
            ..default()
        },
        BorderColor::all(ROW_RULE),
    ))
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

/// Une valeur réglée (volume, sensibilité) : un relevé à chasse fixe, centré entre − et +.
fn value_text(fonts: &UiFonts, text: String) -> impl Bundle {
    (
        Text::new(text),
        fonts.mono(17.0),
        TextColor(theme::SCREEN),
        Node {
            width: px(76),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextLayout::justify(Justify::Center),
    )
}

fn toggle(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    texts: &Texts,
    action: OptionsAction,
    on: bool,
) {
    let key = if on { "options.on" } else { "options.off" };
    spawn_small(parent, fonts, texts.get(key), action, on);
}

/// Petit bouton à cadre (le même que les boutons des menus, en plus petit). `selected` (choix
/// courant, touche en attente) : cadre allumé en ambre. Il n'est alors pas marqué
/// [`MenuButton`] : le survol, qui rend le cadre éteint en sortant, ne l'éteint pas.
fn spawn_small<'a>(
    parent: &'a mut ChildSpawnerCommands,
    fonts: &UiFonts,
    label: String,
    action: OptionsAction,
    selected: bool,
) -> EntityCommands<'a> {
    let frame = fonts.frame(if selected {
        Frame::ButtonHover
    } else {
        Frame::Button
    });
    let mut button = parent.spawn((
        Button,
        ActivateOnPress,
        action,
        Node {
            min_width: px(48),
            height: px(34),
            // À gauche : la languette du cadre.
            padding: UiRect::new(px(15), px(12), px(0), px(0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        frame,
        children![(Text::new(label), fonts.body(17.0), TextColor(theme::TEXT))],
    ));
    if !selected {
        button.insert(MenuButton);
    }
    button
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
