//! Bandeau de pilotage (GDD §7), visible aux commandes : vitesse, mode de vol, astre le plus
//! proche, invite d'amarrage, autopilote engagé. Habillage (M8) : relevé d'instrument à
//! équerres cyan, chiffres à chasse fixe. Il remplacera plus tard un affichage diégétique sur
//! les consoles du cockpit.
//!
//! **Repère de destination** (M10) : dans le système de destination d'un contrat actif, un
//! repère permanent montre le point de livraison le plus proche et sa distance, comme celui de
//! l'étape Livraison du tutoriel (point projeté, ou flèche au bord de l'écran,
//! [`crate::tutorial::marker_spot`]) ; il s'efface quand le tutoriel montre le sien.
//!
//! **Vitesse de combat** (M11) : tant que des hostiles interdisent la croisière (`CruiseLock`),
//! le mode de vol le dit (« Vol assisté · combat ») : la poussée y est bornée (`ship.rs`).
//!
//! **Repères du système** (M11, retour de l'auteur) : aux commandes, chaque cible de navigation
//! du système (astres, stations, structures, hub de Portails, ports : les [`NavTarget`] de la
//! vue Globale, la même source que la carte et l'autopilote) porte un repère discret quand elle
//! est dans le champ : un losange, son nom, sa distance. Pas de flèche au bord de l'écran (le
//! repère de destination garde la sienne), pas de chevauchement (les plus proches d'abord, nom
//! masqué au-delà de [`NAV_NAMED`] ou s'il en couvrirait un autre, [`arrange_markers`]), rien
//! quand la cible remplit l'écran. Le texte coûte cher au GPU intégré de référence : un ensemble
//! fixe de nœuds réutilisés, aucun spawn par frame, un `Text` réécrit seulement quand sa chaîne
//! change (distance arrondie à trois chiffres, [`marker_distance`]).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::career::{Career, CargoPoint, deliverable_here};
use crate::combat::CruiseLock;
use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::options::UserSettings;
use crate::sector::ActiveSector;
use crate::ship::{
    AutopilotRequest, FlightMode, NavTarget, PilotIntent, ShipMotion, SurfaceBody, nearest_surface,
};
use crate::space::SpacePosition;
use crate::state::{ControlMode, ViewLevel};
use crate::transition::{DockBeacon, Sequence};
use crate::tutorial::{Stage, Tutorial, is_active, marker_spot};
use crate::ui::theme::{self, Frame, UiFonts};
use crate::voyage::Location;

pub struct FlightHudPlugin;

impl Plugin for FlightHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DestinationShown>()
            .add_systems(OnEnter(ViewLevel::Global), spawn_flight_hud)
            .add_systems(
                Update,
                (
                    update_flight_hud
                        .run_if(in_state(ViewLevel::Global))
                        .run_if(resource_exists::<ActiveSector>),
                    // Les repères du système évitent celui de la destination, placé juste avant
                    // (conditions sur chaque système d'un groupe chaîné, AGENTS.md §4).
                    (
                        place_destination_marker
                            .run_if(in_state(ViewLevel::Global))
                            .run_if(resource_exists::<ActiveSector>),
                        place_nav_markers
                            .run_if(in_state(ViewLevel::Global))
                            .run_if(resource_exists::<ActiveSector>),
                    )
                        .chain(),
                ),
            );
    }
}

#[derive(Component)]
struct FlightHud;

#[derive(Component, Clone, Copy)]
enum FlightLine {
    Speed,
    Mode,
    Nearest,
    Dock,
    Autopilot,
}

/// Le repère du point de livraison d'un contrat.
#[derive(Component)]
struct DestinationMarker;

/// Ce que montre le repère de destination ce frame : sa cible (pas de second repère sur elle)
/// et la place de son étiquette (les repères du système l'évitent), en px de la fenêtre.
#[derive(Resource, Default)]
struct DestinationShown {
    entity: Option<Entity>,
    rect: Option<Rect>,
}

/// Largeur de l'étiquette du repère, centrée sur la cible (px), et marge du bord de l'écran
/// pour un repère hors champ (mêmes valeurs que le repère du tutoriel).
const MARKER_WIDTH: f32 = 280.0;
const MARKER_MARGIN: f32 = 48.0;
/// Hauteur de l'étiquette du repère de destination (deux lignes), pour les repères qui
/// l'évitent (px).
const MARKER_HEIGHT: f32 = 44.0;

/// Un emplacement de repère du système, réutilisé d'un frame à l'autre : ses trois textes
/// (losange, nom, distance) et la cible qu'il montre (un emplacement garde sa cible tant
/// qu'elle reste affichée : son nom n'est pas réécrit).
#[derive(Component)]
struct NavMarker {
    glyph: Entity,
    name: Entity,
    distance: Entity,
    target: Option<Entity>,
}

/// Un texte d'un [`NavMarker`].
#[derive(Component)]
struct NavMarkerPart;

/// Les textes des repères du système.
type MarkerParts<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text,
        &'static mut TextColor,
        &'static mut Visibility,
    ),
    (With<NavMarkerPart>, Without<NavMarker>),
>;

/// Emplacements de repères du système (les plus proches d'abord ; le plus grand système public,
/// Kholvar, a une quinzaine de cibles).
const NAV_SLOTS: usize = 20;
/// Repères qui portent leur nom, au plus ; les suivants ne montrent que leur losange.
const NAV_NAMED: usize = 6;
/// Marge du bord de l'écran (px) : une cible plus près du bord n'a pas de repère.
const NAV_MARGIN: f32 = 24.0;
/// Le losange : largeur de sa case et hauteur d'une ligne de repère (px).
const GLYPH_WIDTH: f32 = 14.0;
const ROW_HEIGHT: f32 = 18.0;
/// Écart entre le losange, le nom et la distance (px).
const LABEL_GAP: f32 = 5.0;
/// Chasse moyenne estimée d'un caractère du nom (corps 13) et de la distance (chasse fixe,
/// corps 12), en px : un peu large, pour ne jamais sous-estimer un chevauchement.
const NAME_CHAR_WIDTH: f32 = 7.5;
const DIGIT_CHAR_WIDTH: f32 = 7.5;
/// Un repère s'efface quand le rayon apparent de sa cible dépasse cette part de la hauteur de
/// l'écran (elle le remplit presque).
const FILL_FRACTION: f32 = 0.45;
/// Rayon (m) prêté à une cible sans surface (station, structure, port) pour juger si elle
/// remplit l'écran : l'ordre de grandeur de leurs modèles (≈ 0,6 à 3 km de long).
const OBJECT_RADIUS: f64 = 500.0;
/// Le losange d'un repère ; plein, en ambre, pour la cible de l'autopilote. Des signes, pas des
/// mots (comme les flèches du repère de destination).
const GLYPH: &str = "◇";
const GLYPH_ENGAGED: &str = "◆";

fn spawn_flight_hud(mut commands: Commands, fonts: Res<UiFonts>) {
    // Un relevé d'instrument : vitesse en chiffres à chasse fixe (ils ne dansent pas quand la
    // valeur change), mode en capitales ambre, distances en cyan d'écran.
    commands.spawn((
        Name::new("Bandeau de pilotage"),
        FlightHud,
        DespawnOnExit(ViewLevel::Global),
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            bottom: px(28),
            min_width: px(280),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::new(px(22), px(22), px(10), px(12)),
            ..default()
        },
        UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
        fonts.frame(Frame::Readout),
        children![
            (
                FlightLine::Speed,
                Text::new(""),
                fonts.mono(28.0),
                TextColor(theme::TEXT)
            ),
            (
                FlightLine::Mode,
                Text::new(""),
                fonts.heading(15.0),
                TextColor(theme::ACCENT)
            ),
            (
                FlightLine::Autopilot,
                Text::new(""),
                fonts.heading(14.0),
                TextColor(theme::SCREEN)
            ),
            (
                Node {
                    width: percent(100),
                    height: px(1),
                    margin: UiRect::vertical(px(6)),
                    ..default()
                },
                BackgroundColor(theme::SCREEN_DIM),
            ),
            (
                FlightLine::Nearest,
                Text::new(""),
                fonts.mono(15.0),
                TextColor(theme::SCREEN)
            ),
            (
                FlightLine::Dock,
                Text::new(""),
                fonts.body(16.0),
                TextColor(theme::TEXT)
            ),
        ],
    ));
    commands.spawn((
        Name::new("Repère de destination"),
        DestinationMarker,
        DespawnOnExit(ViewLevel::Global),
        Visibility::Hidden,
        GlobalZIndex(3),
        Pickable::IGNORE,
        Text::new(""),
        fonts.title(15.0),
        TextColor(theme::SCREEN),
        // Ombre courte : le cyan reste lisible sur un astre éclairé ou la bande galactique.
        theme::label_shadow(),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            width: px(MARKER_WIDTH),
            ..default()
        },
    ));
    // Les repères du système : un ensemble fixe, cachés jusqu'à ce qu'une cible en prenne un.
    // Sous le repère de destination (3) ; losange cyan, nom clair, distance en chasse fixe.
    for _ in 0..NAV_SLOTS {
        let part = |text: &str, color: Color| {
            (
                NavMarkerPart,
                Text::new(text),
                TextColor(color),
                theme::label_shadow(),
                Pickable::IGNORE,
            )
        };
        let glyph = commands
            .spawn((
                part(GLYPH, theme::SCREEN),
                fonts.title(13.0),
                TextLayout::justify(Justify::Center).with_no_wrap(),
                Node {
                    width: px(GLYPH_WIDTH),
                    ..default()
                },
            ))
            .id();
        let name = commands
            .spawn((
                part("", theme::TEXT),
                fonts.body(13.0),
                TextLayout::no_wrap(),
            ))
            .id();
        let distance = commands
            .spawn((
                part("", theme::SCREEN),
                fonts.mono(12.0),
                TextLayout::no_wrap(),
            ))
            .id();
        commands
            .spawn((
                Name::new("Repère du système"),
                NavMarker {
                    glyph,
                    name,
                    distance,
                    target: None,
                },
                DespawnOnExit(ViewLevel::Global),
                Visibility::Hidden,
                GlobalZIndex(2),
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(LABEL_GAP),
                    height: px(ROW_HEIGHT),
                    ..default()
                },
            ))
            .add_children(&[glyph, name, distance]);
    }
}

/// Le mode de vol affiché : la croisière (demandée, ou la vitesse qui en redescend), sinon le
/// vol assisté ou découplé ; sous le verrou des hostiles, ce dernier suivi de « combat » (la
/// croisière y est interdite, la poussée bornée).
fn flight_mode_text(texts: &Texts, flight: FlightMode, cruising: bool, combat: bool) -> String {
    let flight_key = match flight {
        FlightMode::Assisted => "hud.flight.mode.assisted",
        FlightMode::Decoupled => "hud.flight.mode.decoupled",
    };
    let text = if combat {
        texts.format(
            "hud.flight.mode.combat",
            &[("mode", &texts.get(flight_key))],
        )
    } else if cruising {
        texts.get("hud.flight.mode.cruise")
    } else {
        texts.get(flight_key)
    };
    theme::caps(&text)
}

#[allow(clippy::too_many_arguments)]
fn update_flight_hud(
    texts: Res<Texts>,
    (data, lock): (Res<Data>, Option<Res<CruiseLock>>),
    mode: Res<State<ControlMode>>,
    sector: Res<ActiveSector>,
    intent: Res<PilotIntent>,
    autopilot: Res<AutopilotRequest>,
    ships: Query<(&SpacePosition, &ShipMotion)>,
    bodies: Query<(&SpacePosition, &SurfaceBody, &Name)>,
    beacons: Query<(&SpacePosition, &DockBeacon, &Name)>,
    targets: Query<(&NavTarget, &Name)>,
    mut hud: Query<&mut Visibility, With<FlightHud>>,
    mut lines: Query<(&FlightLine, &mut Text)>,
) {
    let piloting = *mode.get() == ControlMode::Piloting;
    for mut visibility in &mut hud {
        let wanted = if piloting {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    let Ok((pos, motion)) = ships.get(sector.root) else {
        return;
    };
    if !piloting {
        return;
    }
    let speed = motion.velocity.length();
    let speed_text = if speed < 10_000.0 {
        texts.format(
            "hud.flight.speed_ms",
            &[("value", &texts.number(speed.max(0.0), 0))],
        )
    } else {
        texts.format(
            "hud.flight.speed_kms",
            &[("value", &texts.number((speed / 1000.0).max(0.0), 0))],
        )
    };
    // Au-dessus de la poussée maximale, on redescend de la croisière (`ship::step`).
    let max_speed = data
        .flight
        .ships
        .get(sector.sector.id.as_str())
        .map_or(f64::MAX, |h| h.max_speed);
    let mode_text = flight_mode_text(
        &texts,
        motion.mode,
        intent.cruise || speed > max_speed * 1.01,
        lock.is_some_and(|l| l.0),
    );
    let nearest = bodies
        .iter()
        .map(|(p, b, n)| (nearest_surface(pos.0, std::iter::once((p.0, b.radius))), n))
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let nearest_text = nearest.map_or_else(String::new, |(d, name)| {
        texts.format(
            "hud.flight.nearest",
            &[
                ("name", name.as_str()),
                ("distance", &texts.number((d / 1000.0).max(0.0), 0)),
            ],
        )
    });
    // À portée d'une balise : l'invite d'amarrage.
    let dock_text = beacons
        .iter()
        .map(|(p, b, n)| (p.0.distance(pos.0), b, n))
        .filter(|(d, b, _)| *d <= b.range)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map_or_else(String::new, |(d, _, name)| {
            texts.format(
                "hud.flight.dock",
                &[
                    ("name", name.as_str()),
                    ("distance", &texts.number((d / 1000.0).max(0.0), 0)),
                ],
            )
        });
    // L'autopilote engagé, et vers quoi.
    let autopilot_text = autopilot
        .0
        .as_ref()
        .and_then(|id| targets.iter().find(|(t, _)| &t.id == id))
        .map_or_else(String::new, |(_, name)| {
            theme::caps(&texts.format("nav.autopilot.engaged", &[("target", name.as_str())]))
        });
    for (line, mut text) in &mut lines {
        let wanted = match line {
            FlightLine::Speed => &speed_text,
            FlightLine::Mode => &mode_text,
            FlightLine::Nearest => &nearest_text,
            FlightLine::Dock => &dock_text,
            FlightLine::Autopilot => &autopilot_text,
        };
        if text.0 != *wanted {
            text.0.clone_from(wanted);
        }
    }
}

/// Une distance pour le repère : en mètres de près, en kilomètres au-delà.
fn distance_text(texts: &Texts, meters: f64) -> String {
    if meters < 1000.0 {
        texts.format(
            "nav.distance.meters",
            &[("distance", &texts.number(meters, 0))],
        )
    } else {
        let km = meters / 1000.0;
        let decimals = if km < 100.0 { 1 } else { 0 };
        texts.format(
            "nav.distance.kilometers",
            &[("distance", &texts.number(km, decimals))],
        )
    }
}

/// Le repère du point de livraison le plus proche, quand un contrat actif finit dans ce
/// système. Masqué sous un écran (terminal, pause, carte) et quand le tutoriel, à l'étape de la
/// livraison, montre déjà le sien.
#[allow(clippy::too_many_arguments)]
fn place_destination_marker(
    texts: Res<Texts>,
    mode: Res<State<ControlMode>>,
    (settings, tutorial, career, location): (
        Res<UserSettings>,
        Option<Res<Tutorial>>,
        Option<Res<Career>>,
        Option<Res<Location>>,
    ),
    (sector, mut shown): (Res<ActiveSector>, ResMut<DestinationShown>),
    ships: Query<&SpacePosition, With<ShipMotion>>,
    points: Query<(Entity, &Name, &SpacePosition, &GlobalTransform), With<CargoPoint>>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut markers: Query<(&mut Node, &mut Text, &mut Visibility), With<DestinationMarker>>,
) {
    let tutorial_shows = is_active(&settings, tutorial.as_deref())
        && tutorial.as_deref().and_then(Tutorial::stage) == Some(Stage::Delivery);
    let wanted = location.as_deref().is_some_and(|l| {
        career
            .as_deref()
            .is_some_and(|c| deliverable_here(&c.captain, &l.system))
    }) && !tutorial_shows
        && matches!(mode.get(), ControlMode::Piloting | ControlMode::OnFoot);
    let ship = ships.get(sector.root).ok().map(|p| p.0);
    let nearest = ship.filter(|_| wanted).and_then(|ship| {
        points
            .iter()
            .map(|(entity, name, pos, t)| (pos.0.distance(ship), entity, name, t.translation()))
            .min_by(|a, b| a.0.total_cmp(&b.0))
    });
    shown.entity = None;
    shown.rect = None;
    let shot = match (nearest, camera.single()) {
        (Some((distance, entity, name, point)), Ok((camera, eye))) => {
            shown.entity = Some(entity);
            // Avec l'échelle de rendu, la caméra dessine dans une image plus petite que la
            // fenêtre (même conversion que `views::place_labels`).
            let to_window = camera
                .logical_viewport_size()
                .map_or(Vec2::ONE, |size| window.size() / size.max(Vec2::ONE));
            let projected = camera
                .world_to_viewport(eye, point)
                .ok()
                .map(|p| p * to_window);
            let local = eye.affine().inverse().transform_point3(point);
            let (spot, arrow) = marker_spot(projected, local, window.size(), MARKER_MARGIN);
            let label = texts.format(
                "nav.delivery.label",
                &[
                    ("name", name.as_str()),
                    ("distance", &distance_text(&texts, distance)),
                ],
            );
            Some((spot, format!("{}\n{label}", arrow.unwrap_or('◇'))))
        }
        _ => None,
    };
    for (mut node, mut text, mut visibility) in &mut markers {
        let Some((spot, label)) = &shot else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        let left = (spot.x - MARKER_WIDTH / 2.0)
            .clamp(8.0, (window.width() - MARKER_WIDTH - 8.0).max(8.0));
        let top = (spot.y - 10.0).clamp(8.0, (window.height() - 48.0).max(8.0));
        shown.rect = Some(Rect::new(
            left,
            top,
            left + MARKER_WIDTH,
            top + MARKER_HEIGHT,
        ));
        node.left = px(left);
        node.top = px(top);
        if text.0 != *label {
            text.0.clone_from(label);
        }
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }
}

// --- Repères du système (M11) --------------------------------------------------------------------

/// Ce que montre un repère du système.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MarkerLook {
    /// Rien : plus d'emplacement, ou son losange en couvrirait un autre.
    Hidden,
    /// Le losange seul.
    Glyph,
    /// Le losange, le nom et la distance.
    Full,
}

/// Un repère candidat, en px de la fenêtre.
#[derive(Clone, Copy, Debug, PartialEq)]
struct MarkerCandidate {
    /// Le point de la cible à l'écran (centre du losange).
    spot: Vec2,
    /// Sa distance (m) : les plus proches passent d'abord.
    distance: f64,
    /// Largeur de l'étiquette complète (losange, nom, distance).
    width: f32,
    /// Servi avant tous les autres : la cible de l'autopilote.
    first: bool,
}

/// La case du losange, centrée sur la cible.
fn glyph_rect(spot: Vec2) -> Rect {
    Rect::from_center_size(spot, Vec2::new(GLYPH_WIDTH, ROW_HEIGHT))
}

/// L'étiquette complète : le losange, puis le nom et la distance à sa droite.
fn label_rect(spot: Vec2, width: f32) -> Rect {
    let min = spot - Vec2::new(GLYPH_WIDTH, ROW_HEIGHT) / 2.0;
    Rect::new(min.x, min.y, min.x + width, min.y + ROW_HEIGHT)
}

/// Deux zones se touchent-elles (à 2 px près : deux étiquettes ne se collent pas) ?
fn overlaps(a: &Rect, b: &Rect) -> bool {
    const PAD: f32 = 2.0;
    a.min.x < b.max.x + PAD
        && b.min.x < a.max.x + PAD
        && a.min.y < b.max.y + PAD
        && b.min.y < a.max.y + PAD
}

/// Largeur estimée de l'étiquette complète d'un repère (px).
fn label_width(name: &str, distance: &str) -> f32 {
    GLYPH_WIDTH
        + 2.0 * LABEL_GAP
        + name.chars().count() as f32 * NAME_CHAR_WIDTH
        + distance.chars().count() as f32 * DIGIT_CHAR_WIDTH
}

/// Distance d'un repère arrondie à trois chiffres significatifs (à 10 m près sous le
/// kilomètre) : en croisière, le texte ne change pas à chaque frame (chaque changement le
/// recompose, ce que le GPU intégré paie).
fn marker_distance(meters: f64) -> f64 {
    let meters = meters.max(0.0);
    if meters < 1000.0 {
        return (meters / 10.0).round() * 10.0;
    }
    let step = 10f64.powi(meters.log10().floor() as i32 - 2);
    (meters / step).round() * step
}

/// La cible remplit-elle l'écran ? `radius` : son rayon apparent (px).
fn fills_view(radius: f32, screen: Vec2) -> bool {
    radius > FILL_FRACTION * screen.y.min(screen.x)
}

/// Ce que montre chaque repère, dans l'ordre de `candidates` : la cible de l'autopilote, puis
/// les plus proches d'abord ; au plus `slots` repères et `named` noms. Un losange qui couvrirait
/// un repère déjà placé ou une zone de `blocked` (repère de destination, bandeau de pilotage)
/// disparaît ; une étiquette qui en couvrirait un, ou sortirait de l'écran (`screen`), se réduit
/// au losange. Logique pure.
fn arrange_markers(
    candidates: &[MarkerCandidate],
    blocked: &[Rect],
    screen: Vec2,
    slots: usize,
    named: usize,
) -> Vec<MarkerLook> {
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by(|&a, &b| {
        let (a, b) = (&candidates[a], &candidates[b]);
        b.first
            .cmp(&a.first)
            .then(a.distance.total_cmp(&b.distance))
    });
    let mut taken: Vec<Rect> = blocked.to_vec();
    let mut looks = vec![MarkerLook::Hidden; candidates.len()];
    let (mut shown, mut with_name) = (0, 0);
    for i in order {
        if shown >= slots {
            break;
        }
        let candidate = &candidates[i];
        let glyph = glyph_rect(candidate.spot);
        if taken.iter().any(|r| overlaps(r, &glyph)) {
            continue;
        }
        let label = label_rect(candidate.spot, candidate.width);
        let fits = with_name < named
            && label.max.x <= screen.x
            && !taken.iter().any(|r| overlaps(r, &label));
        if fits {
            with_name += 1;
            looks[i] = MarkerLook::Full;
            taken.push(label);
        } else {
            looks[i] = MarkerLook::Glyph;
            taken.push(glyph);
        }
        shown += 1;
    }
    looks
}

/// Une cible retenue ce frame, et ce que montre son repère.
struct ShownTarget<'a> {
    entity: Entity,
    spot: Vec2,
    look: MarkerLook,
    name: &'a str,
    distance: String,
    engaged: bool,
}

/// Les repères des cibles du système (voir l'en-tête du module) : aux commandes en vue Globale,
/// hors séquence (amarrage, départ).
#[allow(clippy::too_many_arguments)]
fn place_nav_markers(
    texts: Res<Texts>,
    mode: Res<State<ControlMode>>,
    (sector, autopilot, shown, sequence): (
        Res<ActiveSector>,
        Res<AutopilotRequest>,
        Res<DestinationShown>,
        Option<Res<Sequence>>,
    ),
    ships: Query<&SpacePosition, With<ShipMotion>>,
    targets: Query<(
        Entity,
        &NavTarget,
        &Name,
        &SpacePosition,
        &GlobalTransform,
        Option<&SurfaceBody>,
    )>,
    (camera, window, hud): (
        Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
        Single<&Window, With<PrimaryWindow>>,
        Query<(&ComputedNode, &UiGlobalTransform), With<FlightHud>>,
    ),
    mut slots: Query<(Entity, &mut NavMarker, &mut Node, &mut Visibility)>,
    mut parts: MarkerParts,
) {
    let piloting = *mode.get() == ControlMode::Piloting && sequence.is_none();
    let ship = ships
        .get(sector.root)
        .ok()
        .map(|p| p.0)
        .filter(|_| piloting);
    let (Some(ship), Ok((camera, eye))) = (ship, camera.single()) else {
        for (_, mut marker, _, mut visibility) in &mut slots {
            if marker.target.is_some() {
                marker.target = None;
            }
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };
    let size = window.size();
    // Avec l'échelle de rendu, la caméra dessine dans une image plus petite que la fenêtre.
    let to_window = camera
        .logical_viewport_size()
        .map_or(Vec2::ONE, |s| size / s.max(Vec2::ONE));
    let to_eye = eye.affine().inverse();
    let right = eye.right().as_vec3();
    let engaged = autopilot.0.as_deref();
    let mut found: Vec<ShownTarget> = Vec::new();
    let mut candidates: Vec<MarkerCandidate> = Vec::new();
    for (entity, target, name, pos, transform, body) in &targets {
        // La destination a déjà son repère.
        if shown.entity == Some(entity) {
            continue;
        }
        let point = transform.translation();
        let Ok(projected) = camera.world_to_viewport(eye, point) else {
            continue;
        };
        let projected = projected * to_window;
        let (spot, arrow) = marker_spot(
            Some(projected),
            to_eye.transform_point3(point),
            size,
            NAV_MARGIN,
        );
        // Hors du champ : pas de flèche au bord (c'est celle du repère de destination).
        if arrow.is_some() {
            continue;
        }
        // Rayon apparent : le bord de la cible, de côté dans le plan de l'écran.
        let radius = body.map_or(OBJECT_RADIUS, |b| b.radius);
        let edge = camera
            .world_to_viewport(eye, point + right * radius as f32)
            .ok()
            .map(|p| p * to_window);
        if edge.is_none_or(|e| fills_view(e.distance(spot), size)) {
            continue;
        }
        // Un astre : la distance à sa surface, comme l'astre le plus proche du bandeau.
        let meters = (pos.0.distance(ship) - body.map_or(0.0, |b| b.radius)).max(0.0);
        let distance = distance_text(&texts, marker_distance(meters));
        let first = engaged == Some(target.id.as_str());
        candidates.push(MarkerCandidate {
            spot,
            distance: meters,
            width: label_width(name.as_str(), &distance),
            first,
        });
        found.push(ShownTarget {
            entity,
            spot,
            look: MarkerLook::Hidden,
            name: name.as_str(),
            distance,
            engaged: first,
        });
    }
    // Les zones à laisser libres : l'étiquette de la destination, le bandeau de pilotage (sa
    // place du frame précédent, en px logiques).
    let mut blocked: Vec<Rect> = shown.rect.into_iter().collect();
    if let Ok((node, transform)) = hud.single()
        && !node.is_empty()
    {
        let scale = node.inverse_scale_factor();
        blocked.push(Rect::from_center_size(
            transform.translation * scale,
            node.size() * scale,
        ));
    }
    let looks = arrange_markers(&candidates, &blocked, size, NAV_SLOTS, NAV_NAMED);
    for (target, look) in found.iter_mut().zip(looks) {
        target.look = look;
    }
    found.retain(|t| t.look != MarkerLook::Hidden);

    // Un emplacement garde sa cible tant qu'elle reste affichée (son nom n'est pas réécrit) ;
    // les cibles nouvelles prennent les emplacements libres.
    let mut placed = vec![false; found.len()];
    let mut free: Vec<Entity> = Vec::new();
    for (slot, marker, mut node, mut visibility) in &mut slots {
        let kept = marker
            .target
            .and_then(|t| found.iter().position(|f| f.entity == t))
            .filter(|&i| !placed[i]);
        match kept {
            Some(i) => {
                placed[i] = true;
                show_marker(&found[i], &marker, &mut node, &mut visibility, &mut parts);
            }
            None => free.push(slot),
        }
    }
    let mut free = free.into_iter();
    for (i, target) in found.iter().enumerate() {
        if placed[i] {
            continue;
        }
        let Some(slot) = free.next() else {
            break;
        };
        if let Ok((_, mut marker, mut node, mut visibility)) = slots.get_mut(slot) {
            marker.target = Some(target.entity);
            show_marker(target, &marker, &mut node, &mut visibility, &mut parts);
        }
    }
    for slot in free {
        if let Ok((_, mut marker, _, mut visibility)) = slots.get_mut(slot) {
            if marker.target.is_some() {
                marker.target = None;
            }
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// Place un emplacement de repère sur sa cible et remplit ses textes ; rien n'est écrit qui
/// n'ait changé (ni la mise en page, ni les textes ne sont refaits pour rien).
fn show_marker(
    target: &ShownTarget,
    marker: &NavMarker,
    node: &mut Mut<Node>,
    visibility: &mut Mut<Visibility>,
    parts: &mut MarkerParts,
) {
    let left = px(target.spot.x - GLYPH_WIDTH / 2.0);
    let top = px(target.spot.y - ROW_HEIGHT / 2.0);
    if node.left != left {
        node.left = left;
    }
    if node.top != top {
        node.top = top;
    }
    if **visibility != Visibility::Inherited {
        **visibility = Visibility::Inherited;
    }
    let (glyph, color) = if target.engaged {
        (GLYPH_ENGAGED, theme::ACCENT)
    } else {
        (GLYPH, theme::SCREEN)
    };
    let full = target.look == MarkerLook::Full;
    set_part(parts, marker.glyph, Some((glyph, color)), true);
    set_part(
        parts,
        marker.name,
        full.then_some((target.name, theme::TEXT)),
        full,
    );
    set_part(
        parts,
        marker.distance,
        full.then_some((target.distance.as_str(), theme::SCREEN)),
        full,
    );
}

/// Un texte de repère : son contenu et sa couleur s'ils changent, et s'il est visible.
fn set_part(parts: &mut MarkerParts, entity: Entity, content: Option<(&str, Color)>, shown: bool) {
    let Ok((mut text, mut color, mut visibility)) = parts.get_mut(entity) else {
        return;
    };
    if let Some((content, wanted)) = content {
        if text.0 != content {
            content.clone_into(&mut text.0);
        }
        if color.0 != wanted {
            color.0 = wanted;
        }
    }
    let wanted = if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *visibility != wanted {
        *visibility = wanted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Vec2 = Vec2::new(1600.0, 900.0);

    fn candidate(x: f32, y: f32, distance: f64) -> MarkerCandidate {
        MarkerCandidate {
            spot: Vec2::new(x, y),
            distance,
            width: 160.0,
            first: false,
        }
    }

    #[test]
    fn les_reperes_se_rangent_sans_se_chevaucher() {
        use MarkerLook::{Full, Glyph, Hidden};
        let looks = arrange_markers(
            &[
                // Au même point qu'un plus proche : rien.
                candidate(400.0, 300.0, 5.0e6),
                candidate(400.0, 300.0, 1.0e6),
                // Juste dessous : la place pour tout.
                candidate(400.0, 330.0, 2.0e6),
                // Sur l'étiquette du plus proche (à droite de son losange) : rien.
                candidate(480.0, 300.0, 3.0e6),
                // Ailleurs : tout.
                candidate(900.0, 600.0, 9.0e6),
            ],
            &[],
            SCREEN,
            NAV_SLOTS,
            NAV_NAMED,
        );
        assert_eq!(looks, vec![Hidden, Full, Full, Hidden, Full]);
        // Le losange à gauche d'un repère plus proche : son étiquette couvrirait celle du plus
        // proche, il se réduit au losange.
        let looks = arrange_markers(
            &[
                candidate(400.0, 300.0, 1.0e6),
                candidate(250.0, 300.0, 2.0e6),
            ],
            &[],
            SCREEN,
            NAV_SLOTS,
            NAV_NAMED,
        );
        assert_eq!(looks, vec![Full, Glyph]);
    }

    #[test]
    fn les_plus_proches_d_abord_et_la_cible_de_l_autopilote_avant_tout() {
        // Huit repères bien séparés : les plus proches portent leur nom, au plus `named`.
        let spread: Vec<MarkerCandidate> = (0..8)
            .map(|i| candidate(100.0, 60.0 + 90.0 * i as f32, 1.0e6 * f64::from(8 - i)))
            .collect();
        let looks = arrange_markers(&spread, &[], SCREEN, NAV_SLOTS, 3);
        let named: Vec<usize> = (0..8).filter(|&i| looks[i] == MarkerLook::Full).collect();
        assert_eq!(named, vec![5, 6, 7]);
        assert!(looks.iter().all(|l| *l != MarkerLook::Hidden));
        // La cible de l'autopilote passe devant, même lointaine ; et au plus `slots` repères.
        let mut engaged = spread.clone();
        engaged[0].first = true;
        let looks = arrange_markers(&engaged, &[], SCREEN, 2, 3);
        assert_eq!(looks[0], MarkerLook::Full);
        assert_eq!(looks[7], MarkerLook::Full);
        assert_eq!(
            looks.iter().filter(|l| **l == MarkerLook::Hidden).count(),
            6
        );
    }

    #[test]
    fn les_zones_reservees_et_le_bord_de_l_ecran_sont_respectes() {
        // Sous l'étiquette du repère de destination : rien ; l'étiquette qui y entrerait : le
        // losange seul.
        let destination = Rect::new(300.0, 280.0, 580.0, 324.0);
        let looks = arrange_markers(
            &[
                candidate(400.0, 300.0, 1.0e6),
                candidate(200.0, 300.0, 2.0e6),
            ],
            &[destination],
            SCREEN,
            NAV_SLOTS,
            NAV_NAMED,
        );
        assert_eq!(looks, vec![MarkerLook::Hidden, MarkerLook::Glyph]);
        // Contre le bord droit : l'étiquette sortirait de l'écran.
        let looks = arrange_markers(
            &[candidate(1540.0, 300.0, 1.0e6)],
            &[],
            SCREEN,
            NAV_SLOTS,
            NAV_NAMED,
        );
        assert_eq!(looks, vec![MarkerLook::Glyph]);
    }

    #[test]
    fn la_distance_d_un_repere_est_arrondie() {
        assert_eq!(marker_distance(-5.0), 0.0);
        assert_eq!(marker_distance(844.0), 840.0);
        assert_eq!(marker_distance(12_345.0), 12_300.0);
        assert_eq!(marker_distance(1_234_567.0), 1_230_000.0);
        assert!((marker_distance(3.734_544e9) - 3.73e9).abs() < 1.0);
        // En croisière, 1 % de chemin ne change pas le texte d'un repère lointain.
        assert_eq!(marker_distance(5.004e8), marker_distance(5.0e8));
    }

    #[test]
    fn une_cible_qui_remplit_l_ecran_n_a_pas_de_repere() {
        assert!(!fills_view(100.0, SCREEN));
        assert!(fills_view(420.0, SCREEN));
        // Une étiquette plus longue est plus large.
        assert!(
            label_width("Vorrh-II", "12,3 km")
                < label_width("Chantiers orbitaux de Vorrh-II", "12,3 km")
        );
    }
}
