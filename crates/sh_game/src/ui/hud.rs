//! Interface en jeu, minimale à pied (GDD §7) : réticule, invite d'interaction, messages
//! brefs, voile de chargement tant que la cellule du joueur n'est pas prête.
//!
//! Habillage (M8) : un réticule fin à quatre branches qui laisse le centre libre, et des
//! plaquettes à barre ambre (`theme::Frame::Plate`) sous les textes, cachées quand ils sont
//! vides : la vue à la première personne reste dégagée.
//!
//! Bandeau du lieu (M9) : en vue Sectorielle, le bandeau ([`crate::views::view_banner`]) porte
//! le nom du secteur ; dans toutes les vues à secteur, sa seconde ligne dit où est le joueur :
//! à bord de quoi (ou dans quel système) et dans quelle salle, quand la cellule a un nom
//! (`name_key` du descripteur, `lang/<code>/rooms.ron`).

use bevy::prelude::*;

use crate::data::{Data, Texts};
use crate::sector::streaming::{CellReady, CellStates};
use crate::sector::{ActiveSector, Docked};
use crate::state::ViewLevel;
use crate::ui::pause::Paused;
use crate::ui::theme::{self, Frame, UiFonts};
use crate::views::{BannerDetail, view_banner};
use crate::voyage::Location;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Prompt>()
            .add_message::<Notice>()
            .add_systems(Update, spawn_hud.run_if(resource_added::<ActiveSector>))
            .add_systems(
                Update,
                // Une condition par système (AGENTS.md §4, groupes chaînés).
                (
                    show_prompt.run_if(resource_exists::<ActiveSector>),
                    show_notices.run_if(resource_exists::<ActiveSector>),
                    show_plates.run_if(resource_exists::<ActiveSector>),
                )
                    .chain(),
            )
            .add_systems(Update, show_veil.run_if(resource_exists::<ActiveSector>))
            .add_systems(
                Update,
                show_banner_detail.run_if(resource_exists::<ActiveSector>),
            )
            .add_systems(OnExit(ViewLevel::Sector), clear_prompt)
            .add_systems(OnExit(ViewLevel::Global), clear_prompt);
    }
}

/// Invite affichée sous le réticule (« E: Open door »).
#[derive(Resource, Default, Debug)]
pub struct Prompt(pub Option<String>);

/// Message bref en bas de l'écran.
#[derive(Message, Clone, Debug)]
pub struct Notice(pub String);

const NOTICE_SECONDS: f32 = 3.5;

/// La plaquette de l'invite (cachée sans invite).
#[derive(Component)]
struct PromptPlate;

#[derive(Component)]
struct PromptText;

/// La plaquette des messages (cachée sans message).
#[derive(Component)]
struct NoticePlate;

#[derive(Component)]
struct NoticeText {
    remaining: f32,
}

#[derive(Component)]
struct LoadingVeil;

/// Réticule : demi-taille du carré qui le contient, longueur et épaisseur des branches.
const RETICLE_HALF: f32 = 11.0;
const RETICLE_ARM: f32 = 6.0;
const RETICLE_THICK: f32 = 2.0;

fn clear_prompt(mut prompt: ResMut<Prompt>) {
    prompt.0 = None;
}

/// Une plaquette centrée horizontalement (invite, message), cachée tant qu'elle est vide.
fn plate(fonts: &UiFonts, vertical: (Val, Val)) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            top: vertical.0,
            bottom: vertical.1,
            // Large à gauche : la barre ambre de la plaquette.
            padding: UiRect::new(px(16), px(14), px(5), px(6)),
            ..default()
        },
        UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
        fonts.frame(Frame::Plate),
        Visibility::Hidden,
    )
}

fn spawn_hud(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    sector: Res<ActiveSector>,
) {
    let scope = sector.scope();
    // Le bandeau du lieu : la vue Globale pose le sien (le système) ; en vue Sectorielle, c'est
    // le secteur (un quartier où le vaisseau est amarré porte le nom du quartier).
    if sector.view == ViewLevel::Sector {
        commands.spawn(view_banner(
            &texts,
            &fonts,
            texts.get(&sector.sector.name_key),
            "",
            scope.clone(),
        ));
    }
    // Réticule : quatre branches autour d'un point, le centre reste lisible.
    let reticle = commands
        .spawn((
            scope.clone(),
            Node {
                position_type: PositionType::Absolute,
                left: percent(50),
                top: percent(50),
                width: px(2.0 * RETICLE_HALF),
                height: px(2.0 * RETICLE_HALF),
                margin: UiRect::all(px(-RETICLE_HALF)),
                ..default()
            },
        ))
        .id();
    let color = BackgroundColor(theme::TEXT.with_alpha(0.75));
    let mid = RETICLE_HALF - RETICLE_THICK / 2.0;
    let far = 2.0 * RETICLE_HALF - RETICLE_ARM;
    for (left, top, width, height) in [
        (mid, mid, RETICLE_THICK, RETICLE_THICK),
        (0.0, mid, RETICLE_ARM, RETICLE_THICK),
        (far, mid, RETICLE_ARM, RETICLE_THICK),
        (mid, 0.0, RETICLE_THICK, RETICLE_ARM),
        (mid, far, RETICLE_THICK, RETICLE_ARM),
    ] {
        commands.spawn((
            ChildOf(reticle),
            Node {
                position_type: PositionType::Absolute,
                left: px(left),
                top: px(top),
                width: px(width),
                height: px(height),
                ..default()
            },
            color,
        ));
    }
    commands.spawn((
        scope.clone(),
        PromptPlate,
        plate(&fonts, (percent(54), Val::Auto)),
        children![(
            PromptText,
            Text::new(""),
            fonts.body(18.0),
            TextColor(theme::TEXT),
        )],
    ));
    commands.spawn((
        scope.clone(),
        NoticePlate,
        // Au-dessus du bandeau de pilotage (bas de l'écran, vue Globale), et au-dessus du voile
        // de la pause et du terminal (20) : un résultat d'action reste lisible ; sous les
        // options (30).
        plate(&fonts, (Val::Auto, px(NOTICE_BOTTOM))),
        GlobalZIndex(25),
        children![(
            NoticeText { remaining: 0.0 },
            Text::new(""),
            fonts.body(18.0),
            TextColor(theme::TEXT),
        )],
    ));
    commands
        .spawn((
            scope,
            LoadingVeil,
            theme::fullscreen_backdrop(),
            GlobalZIndex(100),
        ))
        .with_children(|veil| {
            theme::spawn_corner_marks(veil, 28.0, 44.0, theme::SCREEN_DIM);
            veil.spawn(theme::heading(
                &fonts,
                &texts.get("loading.title"),
                20.0,
                theme::TEXT_DIM,
            ));
            veil.spawn(theme::rule(px(260), UiRect::top(px(12))));
        });
}

fn show_prompt(prompt: Res<Prompt>, mut text: Query<&mut Text, With<PromptText>>) {
    if !prompt.is_changed() {
        return;
    }
    let wanted = prompt.0.clone().unwrap_or_default();
    for mut t in &mut text {
        if t.0 != wanted {
            t.0.clone_from(&wanted);
        }
    }
}

fn show_notices(
    // Temps réel : la pause fige le temps virtuel, un message s'y effacerait jamais.
    time: Res<Time<Real>>,
    mut notices: MessageReader<Notice>,
    mut text: Query<(&mut Text, &mut NoticeText)>,
) {
    let latest = notices.read().last().cloned();
    for (mut t, mut notice) in &mut text {
        if let Some(Notice(message)) = &latest {
            t.0.clone_from(message);
            notice.remaining = NOTICE_SECONDS;
        } else if notice.remaining > 0.0 {
            notice.remaining -= time.delta_secs();
            if notice.remaining <= 0.0 {
                t.0.clear();
            }
        }
    }
}

/// Bas de la plaquette des messages : au-dessus du bandeau de pilotage en jeu ; sous la pause,
/// tout en bas, sous le panneau et sa consigne (centrés à l'écran), pour ne jamais les couvrir.
const NOTICE_BOTTOM: f32 = 150.0;
const NOTICE_BOTTOM_PAUSED: f32 = 36.0;

/// Les plaquettes : visibles seulement avec un texte. Sous la pause, l'invite (sans objet) se
/// cache et les messages (« Partie sauvegardée. ») descendent au bas de l'écran, au-dessus du
/// voile de la pause.
#[allow(clippy::type_complexity)]
fn show_plates(
    paused: Option<Res<Paused>>,
    prompt: Query<&Text, With<PromptText>>,
    notice: Query<&Text, With<NoticeText>>,
    mut plates: Query<
        (&mut Visibility, &mut Node, Has<PromptPlate>),
        Or<(With<PromptPlate>, With<NoticePlate>)>,
    >,
) {
    let paused = paused.is_some();
    let prompt_shown = !paused && prompt.iter().any(|t| !t.0.is_empty());
    let notice_shown = notice.iter().any(|t| !t.0.is_empty());
    for (mut visibility, mut node, is_prompt) in &mut plates {
        let shown = if is_prompt {
            prompt_shown
        } else {
            notice_shown
        };
        let wanted = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
        if is_prompt {
            continue;
        }
        let bottom = px(if paused {
            NOTICE_BOTTOM_PAUSED
        } else {
            NOTICE_BOTTOM
        });
        // Lecture d'abord : ne remettre en page que si la position change.
        if node.bottom != bottom {
            node.bottom = bottom;
        }
    }
}

/// La seconde ligne du bandeau du lieu. Recalculée seulement quand la cellule du joueur, la
/// langue ou le lieu changent ; réécrite seulement si elle diffère (un texte réécrit est remis
/// en page).
#[allow(clippy::too_many_arguments)]
fn show_banner_detail(
    texts: Res<Texts>,
    data: Res<Data>,
    sector: Res<ActiveSector>,
    states: Res<CellStates>,
    location: Option<Res<Location>>,
    docked: Option<Res<Docked>>,
    added: Query<(), Added<BannerDetail>>,
    mut details: Query<(&mut Text, &mut Node), With<BannerDetail>>,
) {
    let changed = texts.is_changed()
        || sector.is_changed()
        || states.is_changed()
        || location.as_ref().is_some_and(|l| l.is_changed())
        || docked.as_ref().is_some_and(|d| d.is_changed())
        || !added.is_empty();
    if !changed || details.is_empty() {
        return;
    }
    let wanted = banner_detail(&texts, &data, &sector, &states, location.as_deref());
    let display = if wanted.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    for (mut text, mut node) in &mut details {
        if text.0 != wanted {
            text.0.clone_from(&wanted);
        }
        if node.display != display {
            node.display = display;
        }
    }
}

/// « Votre vaisseau · Cockpit », « Système d'Hexis · Hall des arrivées » ; sans nom de salle
/// (cellule sans `name_key`), le premier terme seul.
fn banner_detail(
    texts: &Texts,
    data: &Data,
    sector: &ActiveSector,
    states: &CellStates,
    location: Option<&Location>,
) -> String {
    let current = states.current.as_deref();
    let room = current
        .and_then(|id| sector.sector.cell(id))
        .and_then(|cell| cell.name_key.as_deref())
        .map(|key| texts.get(key));
    let place = match sector.view {
        // En vol, à bord du vaisseau du joueur : le système est déjà le titre du bandeau.
        ViewLevel::Global => Some(texts.get(&sector.sector.name_key)),
        _ => {
            // Dans les salles du vaisseau du joueur, « Votre vaisseau » ; ailleurs, le système.
            match sector.aboard_cell(states) {
                Some(_) => Some(texts.get(crate::sector::PLAYER_SHIP_NAME_KEY)),
                None => location
                    .and_then(|l| data.galaxy.system(&l.system))
                    .map(|s| s.name.clone()),
            }
        }
    };
    match (place, room) {
        (Some(place), Some(room)) => texts.format(
            "hud.banner.detail",
            &[("place", place.as_str()), ("room", room.as_str())],
        ),
        (Some(text), None) | (None, Some(text)) => text,
        (None, None) => String::new(),
    }
}

/// Le voile couvre l'arrivée dans le secteur jusqu'à ce que la cellule du joueur et ses voisines
/// Locales soient prêtes.
fn show_veil(
    mut commands: Commands,
    states: Res<CellStates>,
    ready: Query<(), With<CellReady>>,
    veil: Query<Entity, With<LoadingVeil>>,
) {
    // La cellule du joueur et ses voisines Locales (M9 : plus de pièces vides à l'arrivée).
    if states.arrival_ready(&ready) {
        for entity in &veil {
            commands.entity(entity).despawn();
        }
    }
}
