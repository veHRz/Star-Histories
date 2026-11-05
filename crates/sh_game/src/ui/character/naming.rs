//! Saisie du nom d'un casier (M10) : ouverte par `items::NameContainer` (la plaque d'un casier
//! du vaisseau du joueur, `interact: "locker_label"`), une petite fenêtre avec un champ de texte
//! (`bevy::ui_widgets::TextInput`, [`MAX_NAME_CHARS`] caractères au plus). Entrée valide, Échap
//! annule ; un champ vide rend au casier son nom par défaut. Le nom (nettoyé,
//! `items::clean_name`) va dans `items::Containers`, sauvegardé avec la partie : il paraît dans
//! l'invite du casier, en titre de son écran de rangement, et sur sa plaque (L6).
//!
//! Comme l'écran du personnage : le jeu passe en mode Menu (le contexte Menu ne déclare que
//! Retour, Valider et les flèches, si bien qu'aucune touche de jeu ne part pendant qu'on écrit),
//! le curseur se libère, et la fenêtre se ferme une fois Entrée ou Échap relâchée. Le champ reste
//! en lecture seule tant que la touche qui l'a ouvert est enfoncée : sa répétition taperait des
//! « e » dans le nom.

use bevy::input_focus::{AutoFocus, FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, EditableTextFilter, TextCursorStyle, TextEdit, TextReadWriteMode};
use bevy::ui_widgets::TextInput;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_enhanced_input::prelude::Start;

use super::{Blockers, CharacterScreen, ScreenKeys};
use crate::data::{Data, Texts};
use crate::input::{Back, Confirm};
use crate::items::{Containers, MAX_NAME_CHARS, NameContainer, PLAYER_SHIP};
use crate::state::{AppState, ControlMode, ViewLevel};
use crate::ui::keys_panel::{KeyHelp, KeyHint};
use crate::ui::theme::{self, Frame, UiFonts};

pub(super) fn plugin(app: &mut App) {
    app.add_observer(on_confirm)
        .add_observer(on_back)
        // Une condition par système : la fermeture retire la ressource entre deux systèmes
        // (AGENTS.md §4, conditions d'un groupe chaîné).
        .add_systems(
            Update,
            (
                // Bevy 0.20 panique sur un paramètre de ressource absent : la saisie ne s'ouvre
                // qu'en partie, données, textes et polices chargés.
                open_naming
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Data>)
                    .run_if(resource_exists::<Texts>)
                    .run_if(resource_exists::<UiFonts>),
                arm_naming.run_if(resource_exists::<NamingScreen>),
                keep_focus.run_if(resource_exists::<NamingScreen>),
                finish_naming.run_if(resource_exists::<NamingScreen>),
            )
                .chain(),
        )
        .add_systems(OnExit(AppState::InGame), |mut commands: Commands| {
            commands.remove_resource::<NamingScreen>();
        });
}

/// La saisie du nom ouverte.
#[derive(Resource, Debug)]
pub struct NamingScreen {
    /// Le conteneur nommé (du vaisseau du joueur).
    pub container: String,
    /// Le jeu de commandes à rendre en fermant.
    resume: ControlMode,
    /// La touche qui a ouvert la saisie a été relâchée : le champ accepte la frappe.
    armed: bool,
    /// Fermeture demandée (valider ou annuler) : effective quand Entrée et Échap sont relâchées.
    closing: Option<Outcome>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Submit,
    Cancel,
}

/// La racine de la fenêtre.
#[derive(Component)]
struct NamingUi;

/// Le champ de texte.
#[derive(Component)]
struct NameField;

/// La saisie est-elle affichée ? (Scénarios de l'autopilote.)
pub fn is_naming(world: &mut World) -> bool {
    let mut roots = world.query_filtered::<(), With<NamingUi>>();
    roots.iter(world).next().is_some()
}

/// Autopilote : remplace le texte du champ (comme si le joueur l'avait tapé) ; faux si la
/// saisie n'est pas ouverte.
pub fn type_name(world: &mut World, text: &str) -> bool {
    let mut fields = world.query_filtered::<&mut EditableText, With<NameField>>();
    let Ok(mut field) = fields.single_mut(world) else {
        return false;
    };
    let text: String = text.chars().take(MAX_NAME_CHARS).collect();
    field.editor_mut().set_text(&text);
    field.queue_edit(TextEdit::TextEnd(false));
    true
}

/// Autopilote : valide la saisie (comme Entrée).
pub fn submit_name(world: &mut World) {
    if let Some(mut screen) = world.get_resource_mut::<NamingScreen>() {
        screen.closing = Some(Outcome::Submit);
    }
}

// --- Ouverture et fermeture --------------------------------------------------------------------

fn open_naming(
    mut commands: Commands,
    mut requests: MessageReader<NameContainer>,
    texts: Res<Texts>,
    data: Res<Data>,
    fonts: Res<UiFonts>,
    containers: Option<Res<Containers>>,
    mode: Res<State<ControlMode>>,
    mut next_mode: ResMut<NextState<ControlMode>>,
    (character, blockers): (Option<Res<CharacterScreen>>, Blockers),
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Some(NameContainer(container)) = requests.read().last().cloned() else {
        return;
    };
    let resume = *mode.get();
    // À pied ou aux commandes seulement, et rien d'autre à l'écran (une saisie déjà ouverte
    // compte parmi les `Blockers`).
    if character.is_some()
        || blockers.any()
        || !matches!(resume, ControlMode::OnFoot | ControlMode::Piloting)
    {
        return;
    }
    let current = containers
        .as_deref()
        .and_then(|c| c.name(PLAYER_SHIP, &container))
        .unwrap_or_default()
        .to_owned();
    let kind = data.items.kind_key(&container);
    spawn_naming(&mut commands, &texts, &fonts, &texts.get(kind), &current);
    commands.insert_resource(NamingScreen {
        container,
        resume,
        armed: false,
        closing: None,
    });
    next_mode.set(ControlMode::Menu);
    for mut cursor in &mut cursors {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// La frappe compte une fois relâchées la touche d'interaction et Entrée.
fn arm_naming(
    keys: ScreenKeys,
    mut screen: ResMut<NamingScreen>,
    mut fields: Query<&mut TextReadWriteMode, With<NameField>>,
) {
    if screen.armed || keys.pressed(&keys.interact()) || keys.input.pressed(KeyCode::Enter) {
        return;
    }
    screen.armed = true;
    for mut mode in &mut fields {
        *mode = TextReadWriteMode::Editable;
    }
}

/// Le champ garde le focus clavier : un clic à côté, ou Échap (que le champ relâche), le lui
/// ferait perdre et la frappe n'irait nulle part.
fn keep_focus(
    screen: Res<NamingScreen>,
    focus: Option<ResMut<InputFocus>>,
    fields: Query<Entity, With<NameField>>,
) {
    let Some(mut focus) = focus else {
        return;
    };
    if screen.closing.is_some() {
        return;
    }
    if let Ok(field) = fields.single()
        && focus.get() != Some(field)
    {
        focus.set(field, FocusCause::Auto);
    }
}

fn on_confirm(_: On<Start<Confirm>>, screen: Option<ResMut<NamingScreen>>) {
    if let Some(mut screen) = screen
        && screen.armed
        && screen.closing.is_none()
    {
        screen.closing = Some(Outcome::Submit);
    }
}

fn on_back(_: On<Start<Back>>, screen: Option<ResMut<NamingScreen>>) {
    if let Some(mut screen) = screen
        && screen.closing.is_none()
    {
        screen.closing = Some(Outcome::Cancel);
    }
}

/// Ferme la saisie une fois Entrée et Échap relâchées : enregistre le nom (validé), rend les
/// commandes et le curseur.
fn finish_naming(
    mut commands: Commands,
    keys: ScreenKeys,
    screen: Res<NamingScreen>,
    containers: Option<ResMut<Containers>>,
    view: Option<Res<State<ViewLevel>>>,
    mut next_mode: ResMut<NextState<ControlMode>>,
    focus: Option<ResMut<InputFocus>>,
    (fields, roots): (
        Query<&EditableText, With<NameField>>,
        Query<Entity, With<NamingUi>>,
    ),
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Some(outcome) = screen.closing else {
        return;
    };
    if keys.input.pressed(KeyCode::Enter) || keys.input.pressed(KeyCode::Escape) {
        return;
    }
    if outcome == Outcome::Submit
        && let (Ok(field), Some(mut containers)) = (fields.single(), containers)
    {
        let typed = field.value().to_string();
        containers.set_name(PLAYER_SHIP, &screen.container, &typed);
    }
    next_mode.set(screen.resume);
    commands.remove_resource::<NamingScreen>();
    if let Some(mut focus) = focus {
        focus.clear();
    }
    // La fenêtre part avec sa ressource : sans elle, plus rien ne l'effacerait.
    for root in &roots {
        commands.entity(root).despawn();
    }
    if view.is_some_and(|v| matches!(v.get(), ViewLevel::Sector | ViewLevel::Global)) {
        for mut cursor in &mut cursors {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

// --- Affichage ---------------------------------------------------------------------------------

/// La fenêtre : titre, champ (le nom actuel, curseur à la fin), consigne ; l'encart des touches
/// montre Entrée et Échap.
fn spawn_naming(
    commands: &mut Commands,
    texts: &Texts,
    fonts: &UiFonts,
    kind_title: &str,
    current: &str,
) {
    let root = commands
        .spawn((
            Name::new("Saisie du nom d'un casier"),
            NamingUi,
            DespawnOnExit(AppState::InGame),
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(theme::OVERLAY.with_alpha(0.45)),
            KeyHelp::new([
                KeyHint::key(KeyCode::Enter, texts.get("character.keys.confirm_name")),
                KeyHint::key(KeyCode::Escape, texts.get("character.keys.cancel")),
            ]),
        ))
        .id();
    let panel = commands
        .spawn((
            ChildOf(root),
            Node {
                width: px(520),
                max_width: percent(94),
                flex_direction: FlexDirection::Column,
                padding: UiRect::new(
                    px(Frame::Panel.inner() + 8.0),
                    px(Frame::Panel.inner() + 4.0),
                    px(Frame::Panel.inner()),
                    px(Frame::Panel.inner()),
                ),
                row_gap: px(10),
                ..default()
            },
            fonts.frame(Frame::Panel),
        ))
        .id();
    commands.spawn((
        ChildOf(panel),
        Text::new(texts.get("character.naming.title")),
        fonts.title(22.0),
        TextColor(theme::ACCENT),
    ));
    commands.spawn((
        ChildOf(panel),
        theme::rule(percent(100), UiRect::new(px(0), px(0), px(2), px(4))),
    ));
    commands.spawn((
        ChildOf(panel),
        NameField,
        TextInput,
        EditableText {
            max_characters: Some(MAX_NAME_CHARS),
            allow_newlines: false,
            visible_lines: Some(1.0),
            ..EditableText::new(current)
        },
        // Accolades et crochets : la syntaxe des textes de langue, refusée dès la frappe.
        EditableTextFilter::new(|c| !c.is_control() && !matches!(c, '{' | '}' | '[' | ']')),
        TextReadWriteMode::ReadOnly,
        TextLayout::no_wrap(),
        fonts.body(20.0),
        TextColor(theme::TEXT),
        TextCursorStyle {
            color: theme::ACCENT,
            ..default()
        },
        Node {
            width: percent(100),
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(theme::SCREEN_DIM),
        BackgroundColor(Color::srgba(0.015, 0.018, 0.022, 0.85)),
        AutoFocus,
    ));
    commands.spawn((
        ChildOf(panel),
        Text::new(texts.format(
            "character.naming.note",
            &[
                ("max", &MAX_NAME_CHARS.to_string()),
                ("default", kind_title),
            ],
        )),
        fonts.body(14.0),
        TextColor(theme::TEXT_DIM),
    ));
}
