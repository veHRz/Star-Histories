//! Écran des options (M7, à onglets depuis M9), commun au menu principal et au menu de pause.
//! Cinq onglets : Général (disposition du clavier, encart des touches), Affichage (qualité,
//! plein écran, limite d'images, synchronisation verticale, compteur d'images : M11), Gameplay (tutoriel, conseils), Son (volumes) et Contrôles, qui a ses pages :
//! Souris (sensibilités à pied et au pilotage, inversion), puis une page de touches par
//! catégorie (Interface, À pied, Pilotage, Carte : `input::KEY_ACTIONS`, règles dans `options`).
//!
//! Navigation à la souris, ou au clavier : Tab et Maj+Tab parcourent toutes les pages (onglets
//! et pages de Contrôles), les flèches haut et bas choisissent une ligne, gauche et droite la
//! règlent (ou choisissent la case d'une touche), Entrée valide, Retour arrière vide une case.
//! Échap annule d'abord une touche en attente, sinon ferme l'écran ; il se ferme au relâchement,
//! comme le terminal, pour que l'écran dessous ne voie pas la touche. Ces touches s'affichent
//! dans l'encart des touches (`keys_panel`, M10) ; sous le panneau ne reste que la note de la
//! page (ce que font ses réglages).
//!
//! Menu des touches : deux cases par action. La touche choisie est assignée telle quelle, sans
//! échange. Si une action active en même temps l'a déjà, un message le dit, ses cases passent en
//! rouge sur toutes les pages, et l'onglet de chaque page concernée porte un voyant rouge.
//!
//! Habillage (M8) : l'écran d'appareil du menu de pause (équerres, titre en capitales sur un
//! filet), les réglages dans un panneau à pans coupés, une ligne par réglage séparée d'un filet,
//! petits boutons à cadre (le choix courant allumé en ambre), valeurs en relevé à chasse fixe.
//! Le titre et les onglets restent en place d'une page à l'autre ; le panneau suit son contenu.

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};
use sh_data::PresetLevel;

use crate::data::{Data, Texts};
use crate::frame_pace::step_fps_limit;
use crate::input::{KEY_ACTIONS, KeyCategory};
use crate::options::{Slot, UserSettings, choosable, current_layout, physical_label};
use crate::ui::keys_panel::{KeyHelp, KeyHint, KeySource};
use crate::ui::menu::panel_node;
use crate::ui::theme::{self, Frame, MenuButton, UiFonts, menu_button};

pub struct OptionsUiPlugin;

impl Plugin for OptionsUiPlugin {
    fn build(&self, app: &mut App) {
        // `SH_OPTIONS=<page>` : `dev` ouvre l'écran au menu principal (`keys` : la première page
        // des touches) ; les autres pages (`display`, `keys:piloting`…) se choisissent ici.
        if let Ok(value) = std::env::var("SH_OPTIONS") {
            match Page::from_env(value.trim()) {
                Some(page) => {
                    app.insert_resource(StartPage(page));
                }
                None => warn!(
                    "SH_OPTIONS inconnu : « {value} » (general, display, gameplay, sound, controls, keys, keys:interface|foot|piloting|map)"
                ),
            }
        }
        // Une condition par système, évaluée juste avant lui (posée sur le groupe chaîné, elle
        // ne le serait qu'une fois, et `escape` ferme l'écran entre deux systèmes). Le rendu
        // tourne aussi écran fermé : il efface ce qui en reste. `navigate` passe avant
        // `capture_key` : la touche qui vient d'être assignée ne navigue pas en plus.
        app.add_observer(on_activate).add_systems(
            Update,
            (
                start_page
                    .run_if(resource_exists::<OptionsScreen>)
                    .run_if(resource_exists::<StartPage>),
                navigate.run_if(resource_exists::<OptionsScreen>),
                capture_key.run_if(resource_exists::<OptionsScreen>),
                escape.run_if(resource_exists::<OptionsScreen>),
                render.run_if(resource_exists::<Texts>),
            )
                .chain()
                .in_set(OptionsInput),
        );
    }
}

/// Les systèmes de l'écran des options : ce qui lit aussi le clavier dans un menu (le repli de
/// l'encart des touches) passe avant, pour voir si une case attend sa touche.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OptionsInput;

/// L'écran des options est ouvert.
#[derive(Resource, Default)]
pub struct OptionsScreen {
    page: Page,
    /// Ligne choisie au clavier, et case choisie sur une page de touches.
    cursor: usize,
    slot: Slot,
    /// Le clavier a servi : la ligne choisie est mise en évidence (la souris l'efface).
    keyboard: bool,
    /// Case dont on attend la nouvelle touche (action de `KEY_ACTIONS`, case).
    waiting: Option<(usize, Slot)>,
    /// Dernière touche assignée (action, touche) : son conflit éventuel s'affiche.
    notice: Option<(usize, KeyCode)>,
    closing: bool,
}

impl OptionsScreen {
    /// Une case attend sa nouvelle touche : toute touche pressée lui revient.
    pub fn is_waiting(&self) -> bool {
        self.waiting.is_some()
    }

    fn set_page(&mut self, page: Page) {
        self.page = page;
        self.cursor = 0;
        self.slot = Slot::Primary;
        self.notice = None;
    }
}

/// Une page de l'écran : un onglet, ou une page de l'onglet Contrôles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Page {
    #[default]
    General,
    Display,
    Gameplay,
    Sound,
    Mouse,
    Keys(KeyCategory),
}

impl Page {
    /// L'ordre de Tab : les onglets, les pages de Contrôles à leur place.
    const ORDER: [Page; 9] = [
        Page::General,
        Page::Display,
        Page::Gameplay,
        Page::Sound,
        Page::Mouse,
        Page::Keys(KeyCategory::Interface),
        Page::Keys(KeyCategory::OnFoot),
        Page::Keys(KeyCategory::Piloting),
        Page::Keys(KeyCategory::Map),
    ];

    fn tab(self) -> Tab {
        match self {
            Page::General => Tab::General,
            Page::Display => Tab::Display,
            Page::Gameplay => Tab::Gameplay,
            Page::Sound => Tab::Sound,
            Page::Mouse | Page::Keys(_) => Tab::Controls,
        }
    }

    /// La page voisine dans l'ordre de Tab (`step` : 1 ou −1), en boucle.
    fn cycle(self, step: isize) -> Page {
        let len = Self::ORDER.len() as isize;
        let index = Self::ORDER.iter().position(|p| *p == self).unwrap_or(0) as isize;
        Self::ORDER[(index + step).rem_euclid(len) as usize]
    }

    /// La page demandée par `SH_OPTIONS`.
    fn from_env(value: &str) -> Option<Page> {
        match value {
            "" | "1" | "general" => Some(Page::General),
            "display" => Some(Page::Display),
            "gameplay" => Some(Page::Gameplay),
            "sound" => Some(Page::Sound),
            "controls" | "mouse" => Some(Page::Mouse),
            "keys" => Some(Page::Keys(KeyCategory::Interface)),
            other => other.strip_prefix("keys:").and_then(|name| {
                KeyCategory::ALL
                    .into_iter()
                    .find(|c| c.name() == name)
                    .map(Page::Keys)
            }),
        }
    }

    /// Le libellé d'une page de Contrôles.
    fn label(self, texts: &Texts) -> String {
        match self {
            Page::Keys(category) => texts.get(&format!("options.keys.page.{}", category.name())),
            _ => texts.get("options.page.mouse"),
        }
    }

    /// La note de la page, sous le panneau : ce que font ses réglages (les touches de l'écran
    /// sont dans l'encart des touches). Les pages de touches n'en ont pas.
    fn note(self) -> Option<&'static str> {
        match self {
            Page::General => Some("options.note.general"),
            Page::Display => Some("options.note.display"),
            Page::Gameplay => Some("options.note.gameplay"),
            Page::Sound => Some("options.note.sound"),
            Page::Mouse => Some("options.note.mouse"),
            Page::Keys(_) => None,
        }
    }
}

/// Les touches de l'écran, pour l'encart (M10) : ce que la consigne écrite sous le panneau
/// disait. Une case qui attend sa touche n'a que deux gestes : la vider ou annuler.
fn key_help(texts: &Texts, screen: &OptionsScreen) -> KeyHelp {
    if screen.is_waiting() {
        return KeyHelp::new([
            KeyHint::key(KeyCode::Backspace, texts.get("options.keys.help.clear")),
            KeyHint::key(KeyCode::Escape, texts.get("options.keys.help.cancel")),
        ]);
    }
    let keys_page = matches!(screen.page, Page::Keys(_));
    let mut hints = vec![
        KeyHint::keys(
            [
                KeySource::Key(KeyCode::Tab),
                KeySource::Chord(KeyCode::ShiftLeft, KeyCode::Tab),
            ],
            texts.get("options.help.pages"),
        ),
        KeyHint::keys(
            [
                KeySource::Key(KeyCode::ArrowUp),
                KeySource::Key(KeyCode::ArrowDown),
            ],
            texts.get("options.help.rows"),
        ),
        KeyHint::keys(
            [
                KeySource::Key(KeyCode::ArrowLeft),
                KeySource::Key(KeyCode::ArrowRight),
            ],
            texts.get(if keys_page {
                "options.keys.help.slot"
            } else {
                "options.help.adjust"
            }),
        ),
        KeyHint::key(
            KeyCode::Enter,
            texts.get(if keys_page {
                "options.keys.help.change"
            } else {
                "options.help.confirm"
            }),
        )
        .and_mouse(MouseButton::Left),
    ];
    if keys_page {
        hints.push(KeyHint::key(
            KeyCode::Backspace,
            texts.get("options.keys.help.clear"),
        ));
    }
    hints.push(KeyHint::key(
        KeyCode::Escape,
        texts.get("options.help.close"),
    ));
    KeyHelp::new(hints)
}

/// Les pages de l'onglet Contrôles, dans l'ordre de leurs boutons.
const CONTROL_PAGES: [Page; 5] = [
    Page::Mouse,
    Page::Keys(KeyCategory::Interface),
    Page::Keys(KeyCategory::OnFoot),
    Page::Keys(KeyCategory::Piloting),
    Page::Keys(KeyCategory::Map),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    General,
    Display,
    Gameplay,
    Sound,
    Controls,
}

impl Tab {
    const ALL: [Tab; 5] = [
        Tab::General,
        Tab::Display,
        Tab::Gameplay,
        Tab::Sound,
        Tab::Controls,
    ];

    fn label(self) -> &'static str {
        match self {
            Tab::General => "options.tab.general",
            Tab::Display => "options.tab.display",
            Tab::Gameplay => "options.tab.gameplay",
            Tab::Sound => "options.tab.sound",
            Tab::Controls => "options.tab.controls",
        }
    }

    fn first_page(self) -> Page {
        match self {
            Tab::General => Page::General,
            Tab::Display => Page::Display,
            Tab::Gameplay => Page::Gameplay,
            Tab::Sound => Page::Sound,
            Tab::Controls => Page::Mouse,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Channel {
    Master,
    Music,
    Effects,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sense {
    Foot,
    Piloting,
}

/// Une ligne d'une page : un réglage, ou une action du menu des touches (index de
/// `KEY_ACTIONS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Layout,
    KeyHelp,
    Preset,
    Fullscreen,
    /// Limite d'images par seconde (M11, rame_pace::FPS_LIMITS).
    FpsLimit,
    Vsync,
    ShowFps,
    Tutorial,
    Tips,
    Volume(Channel),
    Sensitivity(Sense),
    InvertY,
    Key(usize),
}

impl Row {
    fn label(self) -> &'static str {
        match self {
            Row::Layout => "options.layout",
            Row::KeyHelp => "options.key_help",
            Row::Preset => "options.graphics",
            Row::Fullscreen => "options.fullscreen",
            Row::FpsLimit => "options.fps_limit",
            Row::Vsync => "options.vsync",
            Row::ShowFps => "options.show_fps",
            Row::Tutorial => "options.tutorial",
            Row::Tips => "options.tips",
            Row::Volume(Channel::Master) => "options.master_volume",
            Row::Volume(Channel::Music) => "options.music_volume",
            Row::Volume(Channel::Effects) => "options.effects_volume",
            Row::Sensitivity(Sense::Foot) => "options.sensitivity",
            Row::Sensitivity(Sense::Piloting) => "options.sensitivity_piloting",
            Row::InvertY => "options.invert_y",
            Row::Key(_) => "options.keys.header.action",
        }
    }
}

/// Les lignes d'une page, dans l'ordre affiché.
fn rows(page: Page) -> Vec<Row> {
    match page {
        Page::General => vec![Row::Layout, Row::KeyHelp],
        Page::Display => vec![
            Row::Preset,
            Row::Fullscreen,
            Row::FpsLimit,
            Row::Vsync,
            Row::ShowFps,
        ],
        Page::Gameplay => vec![Row::Tutorial, Row::Tips],
        Page::Sound => vec![
            Row::Volume(Channel::Master),
            Row::Volume(Channel::Music),
            Row::Volume(Channel::Effects),
        ],
        Page::Mouse => vec![
            Row::Sensitivity(Sense::Foot),
            Row::Sensitivity(Sense::Piloting),
            Row::InvertY,
        ],
        Page::Keys(category) => KEY_ACTIONS
            .iter()
            .enumerate()
            .filter(|(_, a)| a.category == category)
            .map(|(index, _)| Row::Key(index))
            .collect(),
    }
}

const PRESETS: [(PresetLevel, &str); 3] = [
    (PresetLevel::Low, "options.preset.low"),
    (PresetLevel::Medium, "options.preset.medium"),
    (PresetLevel::High, "options.preset.high"),
];

/// Règle une ligne d'un cran (`step` : −1 ou 1). Une bascule s'inverse quel que soit le sens,
/// un choix tourne en boucle, une valeur avance d'un dixième dans ses bornes.
fn adjust(row: Row, step: i8, s: &mut UserSettings, keyboard: Option<&sh_data::Keyboard>) {
    let stepped = |value: f32, low: f32, high: f32| {
        // Arrondi au dixième : dix pas de 0,1 retombent juste.
        (((value + 0.1 * f32::from(step)) * 10.0).round() / 10.0).clamp(low, high)
    };
    let cycle = |index: usize, len: usize| {
        (index as isize + isize::from(step)).rem_euclid(len.max(1) as isize) as usize
    };
    match row {
        Row::Layout => {
            let Some(keyboard) = keyboard else {
                return;
            };
            let ids: Vec<&String> = keyboard.layouts.keys().collect();
            let index = {
                let current = current_layout(keyboard, s);
                ids.iter()
                    .position(|id| id.as_str() == current)
                    .unwrap_or(0)
            };
            if let Some(id) = ids.get(cycle(index, ids.len())) {
                s.keyboard_layout = (*id).clone();
            }
        }
        Row::Preset => {
            let index = PRESETS
                .iter()
                .position(|(level, _)| *level == s.preset)
                .unwrap_or(1);
            s.preset = PRESETS[cycle(index, PRESETS.len())].0;
        }
        Row::KeyHelp => s.key_help = !s.key_help,
        Row::Fullscreen => s.fullscreen = !s.fullscreen,
        Row::FpsLimit => s.fps_limit = step_fps_limit(s.fps_limit, step),
        Row::Vsync => s.vsync = !s.vsync,
        Row::ShowFps => s.show_fps = !s.show_fps,
        Row::Tutorial => s.tutorial = !s.tutorial,
        Row::Tips => s.tips = !s.tips,
        Row::InvertY => s.invert_y = !s.invert_y,
        Row::Volume(channel) => {
            let volume = match channel {
                Channel::Master => &mut s.master_volume,
                Channel::Music => &mut s.music_volume,
                Channel::Effects => &mut s.effects_volume,
            };
            *volume = stepped(*volume, 0.0, 1.0);
        }
        Row::Sensitivity(Sense::Foot) => {
            s.mouse_sensitivity = stepped(s.mouse_sensitivity, 0.2, 3.0);
        }
        Row::Sensitivity(Sense::Piloting) => {
            s.piloting_sensitivity = stepped(s.piloting_sensitivity, 0.2, 3.0);
        }
        Row::Key(_) => {}
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
enum OptionsAction {
    Page(Page),
    /// Régler une ligne d'un cran (− et +, bascule).
    Adjust(Row, i8),
    Preset(PresetLevel),
    /// Disposition du clavier, par son rang dans `keyboard.ron`.
    Layout(usize),
    /// Une case du menu des touches : attendre sa nouvelle touche.
    Slot(usize, Slot),
    ResetKeys,
    Back,
}

#[derive(Component)]
struct OptionsUi;

/// La page à montrer à l'ouverture (`SH_OPTIONS`, captures de l'autopilote).
#[derive(Resource, Clone, Copy)]
struct StartPage(Page);

/// Ouvre l'écran des options par-dessus l'écran courant.
pub fn open(commands: &mut Commands) {
    commands.insert_resource(OptionsScreen::default());
}

/// Ouvre directement la première page des touches (captures de l'autopilote).
pub fn open_keys(commands: &mut Commands) {
    commands.insert_resource(OptionsScreen {
        page: Page::Keys(KeyCategory::Interface),
        ..default()
    });
}

fn start_page(mut commands: Commands, start: Res<StartPage>, mut screen: ResMut<OptionsScreen>) {
    screen.set_page(start.0);
    commands.remove_resource::<StartPage>();
}

fn on_activate(
    event: On<Activate>,
    actions: Query<&OptionsAction>,
    screen: Option<ResMut<OptionsScreen>>,
    mut settings: ResMut<UserSettings>,
    data: Option<Res<Data>>,
) {
    let (Ok(&action), Some(mut screen)) = (actions.get(event.entity), screen) else {
        return;
    };
    if let Some(waiting) = screen.waiting {
        // Un clic pendant l'attente : une autre case attend à sa place, tout autre bouton
        // annule l'attente.
        screen.waiting = match action {
            OptionsAction::Slot(index, slot) if (index, slot) != waiting => Some((index, slot)),
            _ => None,
        };
        return;
    }
    screen.keyboard = false;
    match action {
        OptionsAction::Page(page) => screen.set_page(page),
        OptionsAction::Adjust(row, step) => adjust(
            row,
            step,
            &mut settings,
            data.as_deref().map(|d| &d.keyboard),
        ),
        OptionsAction::Preset(level) => settings.preset = level,
        OptionsAction::Layout(index) => {
            if let Some(id) = data
                .as_deref()
                .and_then(|d| d.keyboard.layouts.keys().nth(index))
            {
                settings.keyboard_layout = id.clone();
            }
        }
        OptionsAction::Slot(index, slot) => {
            screen.cursor = rows(screen.page)
                .iter()
                .position(|row| *row == Row::Key(index))
                .unwrap_or(0);
            screen.slot = slot;
            screen.waiting = Some((index, slot));
            screen.notice = None;
        }
        OptionsAction::ResetKeys => {
            settings.reset_keys();
            screen.notice = None;
        }
        OptionsAction::Back => screen.closing = true,
    }
}

/// Le clavier hors attente d'une touche : pages, ligne choisie, réglage, case vidée.
fn navigate(
    keys: Res<ButtonInput<KeyCode>>,
    screen: Option<ResMut<OptionsScreen>>,
    mut settings: ResMut<UserSettings>,
    data: Option<Res<Data>>,
) {
    let Some(mut screen) = screen else {
        return;
    };
    if screen.waiting.is_some() || screen.closing {
        return;
    }
    if keys.just_pressed(KeyCode::Tab) {
        let back = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let page = screen.page.cycle(if back { -1 } else { 1 });
        screen.set_page(page);
        return;
    }
    let up = keys.just_pressed(KeyCode::ArrowUp);
    let down = keys.just_pressed(KeyCode::ArrowDown);
    let left = keys.just_pressed(KeyCode::ArrowLeft);
    let right = keys.just_pressed(KeyCode::ArrowRight);
    let enter = keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]);
    let clear = keys.just_pressed(KeyCode::Backspace);
    let rows = rows(screen.page);
    if !(up || down || left || right || enter || clear) || rows.is_empty() {
        return;
    }
    if !screen.keyboard {
        // Premier appui après la souris : montrer la ligne choisie, sans rien régler.
        screen.keyboard = true;
        return;
    }
    let len = rows.len();
    if up || down {
        screen.cursor = (screen.cursor + if down { 1 } else { len - 1 }) % len;
        return;
    }
    let row = rows[screen.cursor.min(len - 1)];
    let keyboard = data.as_deref().map(|d| &d.keyboard);
    match row {
        Row::Key(index) => {
            if left || right {
                screen.slot = if right {
                    Slot::Secondary
                } else {
                    Slot::Primary
                };
            } else if enter {
                screen.waiting = Some((index, screen.slot));
                screen.notice = None;
            } else if clear && let Some(action) = KEY_ACTIONS.get(index) {
                settings.set_slot(action, screen.slot, None);
                screen.notice = None;
            }
        }
        _ if left || right => adjust(row, if right { 1 } else { -1 }, &mut settings, keyboard),
        _ if enter => adjust(row, 1, &mut settings, keyboard),
        _ => {}
    }
}

/// La touche pressée va dans la case en attente, telle quelle ; Retour arrière la vide (Échap
/// annule, dans `escape`).
fn capture_key(
    mut reader: MessageReader<KeyboardInput>,
    screen: Option<ResMut<OptionsScreen>>,
    mut settings: ResMut<UserSettings>,
) {
    let Some(mut screen) = screen else {
        return;
    };
    let Some((index, slot)) = screen.waiting else {
        reader.clear();
        return;
    };
    let Some(action) = KEY_ACTIONS.get(index) else {
        screen.waiting = None;
        return;
    };
    for input in reader.read() {
        if input.state != ButtonState::Pressed || input.repeat {
            continue;
        }
        if input.key_code == KeyCode::Backspace {
            settings.set_slot(action, slot, None);
            screen.waiting = None;
            screen.notice = None;
            return;
        }
        if choosable(input.key_code) {
            settings.set_slot(action, slot, Some(input.key_code));
            screen.waiting = None;
            screen.notice = Some((index, input.key_code));
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
            screen.closing = true;
        }
    }
    if screen.closing && !keys.pressed(KeyCode::Escape) {
        commands.remove_resource::<OptionsScreen>();
    }
}

/// Largeur des boutons du bas (« Touches par défaut », « Retour »).
const FOOT_BUTTON_WIDTH: f32 = 300.0;
/// Largeur d'une ligne (intitulé à gauche, commandes à droite), sur toutes les pages.
const ROW_WIDTH: f32 = 640.0;
/// Hauteur d'une ligne de réglage.
const ROW_HEIGHT: f32 = 46.0;
/// Hauteur d'une ligne du menu des touches (13 lignes au plus, Pilotage, en 1600 × 900).
const KEY_ROW_HEIGHT: f32 = 30.0;
/// Largeur d'une case de touche.
const SLOT_WIDTH: f32 = 128.0;
/// Largeur de la valeur d'un réglage à − et + (« 100 % », « 1,5 »).
const VALUE_WIDTH: f32 = 76.0;
/// Largeur de la limite d'images : « Illimitée » (« Unlimited ») tient sur une ligne.
const FPS_VALUE_WIDTH: f32 = 112.0;
/// Largeur d'un onglet.
const TAB_WIDTH: f32 = 124.0;
/// Largeur d'un bouton de page de Contrôles.
const PAGE_BUTTON_WIDTH: f32 = 118.0;
/// Filet entre deux lignes d'options.
const ROW_RULE: Color = Color::srgba(0.42, 0.45, 0.49, 0.22);
/// Fond de la ligne choisie au clavier.
const SELECTED_ROW: Color = Color::srgba(0.95, 0.66, 0.24, 0.10);

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    settings: Res<UserSettings>,
    data: Option<Res<Data>>,
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
    let keyboard = data.as_deref().map(|d| &d.keyboard);
    let root = commands
        .spawn((
            Name::new("Options"),
            OptionsUi,
            GlobalZIndex(30),
            theme::fullscreen_backdrop(),
            key_help(&texts, &screen),
        ))
        // En haut plutôt qu'au centre : le titre et les onglets ne bougent pas quand la
        // hauteur du panneau change d'une page à l'autre.
        .insert(Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            padding: UiRect::top(Val::Vh(5.0)),
            ..default()
        })
        .id();
    commands.entity(root).with_children(|ui| {
        theme::spawn_corner_marks(ui, 28.0, 44.0, theme::SCREEN_DIM);
        ui.spawn(theme::heading(
            &fonts,
            &texts.get("menu.options"),
            40.0,
            theme::TEXT,
        ));
        ui.spawn(theme::rule(
            px(420),
            UiRect::new(px(0), px(0), px(10), px(16)),
        ));
        tab_bar(ui, &texts, &fonts, screen.page);
        ui.spawn((panel_node(UiRect::ZERO), fonts.frame(Frame::Panel)))
            .with_children(|panel| {
                if screen.page.tab() == Tab::Controls {
                    control_pages(panel, &texts, &fonts, &settings, screen.page);
                }
                match screen.page {
                    Page::Keys(category) => {
                        keys_page(panel, &texts, &fonts, &settings, &screen, category);
                    }
                    page => {
                        settings_page(panel, &texts, &fonts, &settings, keyboard, &screen, page)
                    }
                }
                foot_buttons(panel, &texts, &fonts, screen.page);
            });
        // La note de la page, sous le panneau : longue, elle ne doit pas l'élargir. Les touches
        // sont dans l'encart (`key_help`).
        if let Some(note) = screen.page.note() {
            ui.spawn((
                Text::new(texts.get(note)),
                fonts.body(15.0),
                TextColor(theme::TEXT_DIM),
                TextLayout::justify(Justify::Center),
                Node {
                    max_width: px(900),
                    margin: UiRect::top(px(12)),
                    ..default()
                },
            ));
        }
    });
}

/// Les cinq onglets ; celui de la page courante est allumé.
fn tab_bar(ui: &mut ChildSpawnerCommands, texts: &Texts, fonts: &UiFonts, page: Page) {
    ui.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(8),
        margin: UiRect::bottom(px(12)),
        ..default()
    })
    .with_children(|bar| {
        for tab in Tab::ALL {
            // L'onglet courant garde sa page (une page de touches reste ouverte).
            let target = if tab == page.tab() {
                page
            } else {
                tab.first_page()
            };
            spawn_button(
                bar,
                fonts,
                texts.get(tab.label()),
                OptionsAction::Page(target),
                tab == page.tab(),
                theme::TEXT,
                18.0,
            )
            .insert(Node {
                width: px(TAB_WIDTH),
                height: px(40),
                padding: UiRect::new(px(15), px(12), px(0), px(0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            });
        }
    });
}

/// Les pages de Contrôles : Souris, puis une par catégorie de touches. Un voyant rouge signale
/// une page où deux actions actives ensemble partagent une touche.
fn control_pages(
    ui: &mut ChildSpawnerCommands,
    texts: &Texts,
    fonts: &UiFonts,
    s: &UserSettings,
    page: Page,
) {
    ui.spawn(Node {
        width: px(ROW_WIDTH),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        column_gap: px(6),
        margin: UiRect::bottom(px(10)),
        ..default()
    })
    .with_children(|bar| {
        for target in CONTROL_PAGES {
            let warned = match target {
                Page::Keys(category) => page_in_conflict(s, category),
                _ => false,
            };
            let mut button = spawn_button(
                bar,
                fonts,
                target.label(texts),
                OptionsAction::Page(target),
                target == page,
                theme::TEXT,
                15.0,
            );
            button.insert(Node {
                width: px(PAGE_BUTTON_WIDTH),
                height: px(32),
                padding: UiRect::new(px(15), px(10), px(0), px(0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            });
            if warned {
                button.with_children(|b| {
                    b.spawn((
                        Node {
                            width: px(7),
                            height: px(7),
                            ..default()
                        },
                        BackgroundColor(theme::DANGER),
                    ));
                });
            }
        }
    });
}

/// Une page de la catégorie a-t-elle une touche en conflit ?
fn page_in_conflict(s: &UserSettings, category: KeyCategory) -> bool {
    KEY_ACTIONS
        .iter()
        .filter(|a| a.category == category)
        .any(|a| Slot::BOTH.iter().any(|slot| s.in_conflict(a, *slot)))
}

#[allow(clippy::too_many_arguments)]
fn settings_page(
    ui: &mut ChildSpawnerCommands,
    texts: &Texts,
    fonts: &UiFonts,
    s: &UserSettings,
    keyboard: Option<&sh_data::Keyboard>,
    screen: &OptionsScreen,
    page: Page,
) {
    let percent = |v: f32| format!("{} %", texts.number(f64::from(v) * 100.0, 0));
    let rows = rows(page);
    for (i, row) in rows.iter().copied().enumerate() {
        let selected = screen.keyboard && screen.cursor == i;
        setting_row(
            ui,
            fonts,
            texts.get(row.label()),
            i + 1 < rows.len(),
            selected,
            |r| match row {
                Row::Layout => {
                    let Some(keyboard) = keyboard else {
                        return;
                    };
                    let current = current_layout(keyboard, s);
                    for (index, id) in keyboard.layouts.keys().enumerate() {
                        spawn_small(
                            r,
                            fonts,
                            texts.get(&format!("options.layout.{id}")),
                            OptionsAction::Layout(index),
                            id == current,
                        );
                    }
                }
                Row::Preset => {
                    for (level, key) in PRESETS {
                        spawn_small(
                            r,
                            fonts,
                            texts.get(key),
                            OptionsAction::Preset(level),
                            s.preset == level,
                        );
                    }
                }
                Row::KeyHelp => toggle(r, fonts, texts, row, s.key_help),
                Row::Fullscreen => toggle(r, fonts, texts, row, s.fullscreen),
                Row::FpsLimit => {
                    let value = if s.fps_limit == 0 {
                        texts.get("options.fps_limit.unlimited")
                    } else {
                        texts.number(f64::from(s.fps_limit), 0)
                    };
                    stepper(r, fonts, row, value, FPS_VALUE_WIDTH);
                }
                Row::Vsync => toggle(r, fonts, texts, row, s.vsync),
                Row::ShowFps => toggle(r, fonts, texts, row, s.show_fps),
                Row::Tutorial => toggle(r, fonts, texts, row, s.tutorial),
                Row::Tips => toggle(r, fonts, texts, row, s.tips),
                Row::InvertY => toggle(r, fonts, texts, row, s.invert_y),
                Row::Volume(channel) => {
                    let value = match channel {
                        Channel::Master => s.master_volume,
                        Channel::Music => s.music_volume,
                        Channel::Effects => s.effects_volume,
                    };
                    stepper(r, fonts, row, percent(value), VALUE_WIDTH);
                }
                Row::Sensitivity(sense) => {
                    let value = match sense {
                        Sense::Foot => s.mouse_sensitivity,
                        Sense::Piloting => s.piloting_sensitivity,
                    };
                    stepper(
                        r,
                        fonts,
                        row,
                        texts.number(f64::from(value), 1),
                        VALUE_WIDTH,
                    );
                }
                Row::Key(_) => {}
            },
        );
    }
}

/// Une page de touches : en-tête, une ligne par action (intitulé, principale, secondaire), puis
/// la ligne de message (attente, conflit).
fn keys_page(
    ui: &mut ChildSpawnerCommands,
    texts: &Texts,
    fonts: &UiFonts,
    s: &UserSettings,
    screen: &OptionsScreen,
    category: KeyCategory,
) {
    ui.spawn((
        Node {
            width: px(ROW_WIDTH),
            height: px(22),
            align_items: AlignItems::Center,
            column_gap: px(10),
            border: UiRect::bottom(px(1)),
            ..default()
        },
        BorderColor::all(theme::RULE),
    ))
    .with_children(|header| {
        header.spawn((
            Text::new(theme::caps(&texts.get("options.keys.header.action"))),
            fonts.body(12.0),
            TextColor(theme::TEXT_DIM),
            Node {
                flex_grow: 1.0,
                ..default()
            },
        ));
        for key in [
            "options.keys.header.primary",
            "options.keys.header.secondary",
        ] {
            header.spawn((
                Text::new(theme::caps(&texts.get(key))),
                fonts.body(12.0),
                TextColor(theme::TEXT_DIM),
                TextLayout::justify(Justify::Center),
                Node {
                    width: px(SLOT_WIDTH),
                    ..default()
                },
            ));
        }
    });
    let rows = rows(Page::Keys(category));
    for (i, row) in rows.iter().enumerate() {
        let Row::Key(index) = *row else {
            continue;
        };
        let Some(action) = KEY_ACTIONS.get(index) else {
            continue;
        };
        let slots = s.slots(action);
        let selected = screen.keyboard && screen.cursor == i;
        let conflict = Slot::BOTH.iter().any(|slot| s.in_conflict(action, *slot));
        ui.spawn((
            Node {
                width: px(ROW_WIDTH),
                height: px(KEY_ROW_HEIGHT),
                align_items: AlignItems::Center,
                column_gap: px(10),
                // Le filet sous chaque ligne, sauf la dernière.
                border: UiRect::bottom(px(if i + 1 < rows.len() { 1.0 } else { 0.0 })),
                ..default()
            },
            BorderColor::all(ROW_RULE),
            BackgroundColor(if selected { SELECTED_ROW } else { Color::NONE }),
        ))
        .with_children(|line| {
            line.spawn((
                Text::new(texts.get(&format!("options.action.{}", action.id))),
                fonts.body(15.0),
                TextColor(if conflict { theme::DANGER } else { theme::TEXT }),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ));
            for slot in Slot::BOTH {
                let waiting = screen.waiting == Some((index, slot));
                let key = slots.get(slot);
                let label = match (waiting, key) {
                    (true, _) => texts.get("options.keys.waiting"),
                    (false, Some(key)) => physical_label(texts, key),
                    (false, None) => texts.get("key.None"),
                };
                let color = if s.in_conflict(action, slot) {
                    theme::DANGER
                } else if key.is_none() {
                    theme::TEXT_DIM
                } else {
                    theme::TEXT
                };
                let lit = waiting || (selected && screen.slot == slot);
                spawn_button(
                    line,
                    fonts,
                    label,
                    OptionsAction::Slot(index, slot),
                    lit,
                    color,
                    15.0,
                )
                .insert(Node {
                    width: px(SLOT_WIDTH),
                    height: px(26),
                    padding: UiRect::new(px(14), px(10), px(0), px(0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    ..default()
                });
            }
        });
    }
    let (message, color) = keys_message(texts, s, screen, category);
    ui.spawn((
        Text::new(message),
        fonts.body(15.0),
        TextColor(color),
        TextLayout::justify(Justify::Center),
        Node {
            width: px(ROW_WIDTH),
            min_height: px(38),
            margin: UiRect::top(px(8)),
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
}

/// La ligne de message d'une page de touches : la touche attendue, le conflit de la touche
/// qu'on vient de choisir, ou les conflits de la page. Vide sinon (la ligne garde sa place).
fn keys_message(
    texts: &Texts,
    s: &UserSettings,
    screen: &OptionsScreen,
    category: KeyCategory,
) -> (String, Color) {
    if screen.waiting.is_some() {
        return (texts.get("options.keys.press"), theme::ACCENT);
    }
    if let Some((index, key)) = screen.notice
        && let Some(action) = KEY_ACTIONS.get(index)
    {
        let others = s.conflicts(action, key);
        if !others.is_empty() {
            let names: Vec<String> = others
                .iter()
                .map(|other| {
                    texts.format(
                        "options.keys.conflict_item",
                        &[
                            (
                                "action",
                                &texts.get(&format!("options.action.{}", other.id)),
                            ),
                            (
                                "page",
                                &texts.get(&format!("options.keys.page.{}", other.category.name())),
                            ),
                        ],
                    )
                })
                .collect();
            let message = texts.format(
                "options.keys.conflict",
                &[
                    ("key", &physical_label(texts, key)),
                    (
                        "actions",
                        &names.join(&texts.get("options.keys.conflict_separator")),
                    ),
                ],
            );
            return (message, theme::DANGER);
        }
    }
    if page_in_conflict(s, category) {
        return (texts.get("options.keys.conflicts"), theme::DANGER);
    }
    (String::new(), theme::TEXT_DIM)
}

/// Les boutons du bas, côte à côte : « Touches par défaut » (pages de touches) et « Retour ».
fn foot_buttons(ui: &mut ChildSpawnerCommands, texts: &Texts, fonts: &UiFonts, page: Page) {
    ui.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(16),
        margin: UiRect::top(px(12)),
        ..default()
    })
    .with_children(|buttons| {
        let mut list = Vec::new();
        if matches!(page, Page::Keys(_)) {
            list.push((texts.get("options.keys.reset"), OptionsAction::ResetKeys));
        }
        list.push((texts.get("menu.back"), OptionsAction::Back));
        for (label, action) in list {
            buttons
                .spawn(menu_button(fonts, label, action, true))
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
}

/// Une ligne de réglage : l'intitulé à gauche, les commandes à droite ; `rule` : un filet
/// dessous ; `selected` : la ligne choisie au clavier.
fn setting_row(
    ui: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    label: String,
    rule: bool,
    selected: bool,
    controls: impl FnOnce(&mut ChildSpawnerCommands),
) {
    ui.spawn((
        Node {
            width: px(ROW_WIDTH),
            height: px(ROW_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border: UiRect::bottom(px(if rule { 1.0 } else { 0.0 })),
            ..default()
        },
        BorderColor::all(ROW_RULE),
        BackgroundColor(if selected { SELECTED_ROW } else { Color::NONE }),
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

/// − valeur + : une valeur réglée (volume, sensibilité, limite d'images), en relevé à chasse
/// fixe ; `width` : la place de la valeur, assez large pour la plus longue (les boutons ne
/// bougent pas quand elle change).
fn stepper(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    row: Row,
    value: String,
    width: f32,
) {
    spawn_small(
        parent,
        fonts,
        "−".into(),
        OptionsAction::Adjust(row, -1),
        false,
    );
    parent.spawn((
        Text::new(value),
        fonts.mono(17.0),
        TextColor(theme::SCREEN),
        Node {
            width: px(width),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextLayout::justify(Justify::Center).with_no_wrap(),
    ));
    spawn_small(
        parent,
        fonts,
        "+".into(),
        OptionsAction::Adjust(row, 1),
        false,
    );
}

fn toggle(parent: &mut ChildSpawnerCommands, fonts: &UiFonts, texts: &Texts, row: Row, on: bool) {
    let key = if on { "options.on" } else { "options.off" };
    spawn_small(
        parent,
        fonts,
        texts.get(key),
        OptionsAction::Adjust(row, 1),
        on,
    );
}

/// Petit bouton à cadre (le même que les boutons des menus, en plus petit).
fn spawn_small<'a>(
    parent: &'a mut ChildSpawnerCommands,
    fonts: &UiFonts,
    label: String,
    action: OptionsAction,
    selected: bool,
) -> EntityCommands<'a> {
    spawn_button(parent, fonts, label, action, selected, theme::TEXT, 17.0)
}

/// Bouton à cadre. `selected` (choix courant, onglet ouvert, touche en attente) : cadre allumé
/// en ambre. Il n'est alors pas marqué [`MenuButton`] : le survol, qui rend le cadre éteint en
/// sortant, ne l'éteint pas.
fn spawn_button<'a>(
    parent: &'a mut ChildSpawnerCommands,
    fonts: &UiFonts,
    label: String,
    action: OptionsAction,
    selected: bool,
    color: Color,
    size: f32,
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
        children![(Text::new(label), fonts.body(size), TextColor(color))],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_de_sh_options() {
        assert_eq!(Page::from_env("1"), Some(Page::General));
        assert_eq!(Page::from_env("sound"), Some(Page::Sound));
        assert_eq!(Page::from_env("controls"), Some(Page::Mouse));
        assert_eq!(
            Page::from_env("keys"),
            Some(Page::Keys(KeyCategory::Interface))
        );
        assert_eq!(
            Page::from_env("keys:piloting"),
            Some(Page::Keys(KeyCategory::Piloting))
        );
        assert_eq!(Page::from_env("keys:nulle_part"), None);
    }

    /// Tab parcourt toutes les pages et revient au départ ; chaque page a des lignes.
    #[test]
    fn tab_fait_le_tour() {
        let mut page = Page::General;
        for _ in 0..Page::ORDER.len() {
            assert!(!rows(page).is_empty(), "{page:?}");
            page = page.cycle(1);
        }
        assert_eq!(page, Page::General);
        assert_eq!(Page::General.cycle(-1), Page::Keys(KeyCategory::Map));
        // Toutes les actions remappables ont leur ligne, une seule fois.
        let keys: usize = KeyCategory::ALL
            .iter()
            .map(|c| rows(Page::Keys(*c)).len())
            .sum();
        assert_eq!(keys, KEY_ACTIONS.len());
    }

    /// Les lignes de l'onglet Affichage et le compteur d'images ont leurs textes, dans les deux
    /// langues.
    #[test]
    fn libelles_de_l_affichage() {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
        for code in [sh_data::paths::LANG_CODE, sh_data::paths::LANG_EN_CODE] {
            let lang = sh_data::load_lang(&data, code).unwrap_or_else(|e| panic!("{e}"));
            let mut keys: Vec<&str> = rows(Page::Display).iter().map(|r| r.label()).collect();
            keys.extend([
                "options.fps_limit.unlimited",
                "options.fps.counter",
                "options.note.display",
            ]);
            for key in keys {
                assert!(lang.has(key), "{code} : clé absente {key}");
            }
        }
    }

    #[test]
    fn reglages_au_cran() {
        let mut s = UserSettings::default();
        for _ in 0..10 {
            adjust(Row::Volume(Channel::Music), 1, &mut s, None);
        }
        assert_eq!(s.music_volume, 1.0);
        adjust(Row::Sensitivity(Sense::Piloting), 1, &mut s, None);
        assert!((s.piloting_sensitivity - 1.1).abs() < 1e-6);
        adjust(Row::Preset, 1, &mut s, None);
        assert_eq!(s.preset, PresetLevel::High);
        adjust(Row::Preset, 1, &mut s, None);
        assert_eq!(s.preset, PresetLevel::Low);
        adjust(Row::Tips, -1, &mut s, None);
        assert!(!s.tips);
        // Limite d'images (M11) : illimitée par défaut, « − » descend vers 240, puis 165…
        assert_eq!(s.fps_limit, 0);
        adjust(Row::FpsLimit, -1, &mut s, None);
        assert_eq!(s.fps_limit, 240);
        for _ in 0..5 {
            adjust(Row::FpsLimit, -1, &mut s, None);
        }
        assert_eq!(s.fps_limit, 75);
        adjust(Row::FpsLimit, 1, &mut s, None);
        assert_eq!(s.fps_limit, 90);
        adjust(Row::Vsync, 1, &mut s, None);
        adjust(Row::ShowFps, -1, &mut s, None);
        assert!(s.vsync && s.show_fps);
        assert_eq!(
            rows(Page::Display),
            [
                Row::Preset,
                Row::Fullscreen,
                Row::FpsLimit,
                Row::Vsync,
                Row::ShowFps
            ]
        );
        let keyboard = sh_data::Keyboard {
            default_layout: "azerty".into(),
            layouts: [
                ("azerty".into(), Default::default()),
                ("qwerty".into(), Default::default()),
            ]
            .into_iter()
            .collect(),
        };
        adjust(Row::Layout, 1, &mut s, Some(&keyboard));
        assert_eq!(s.keyboard_layout, "qwerty");
        adjust(Row::Layout, 1, &mut s, Some(&keyboard));
        assert_eq!(s.keyboard_layout, "azerty");
    }
}
