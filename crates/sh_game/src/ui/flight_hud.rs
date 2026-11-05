//! Bandeau de pilotage (GDD §7), visible aux commandes : vitesse, mode de vol, astre le plus
//! proche, invite d'amarrage, autopilote engagé. Habillage (M8) : relevé d'instrument à
//! équerres cyan, chiffres à chasse fixe. Il remplacera plus tard un affichage diégétique sur
//! les consoles du cockpit.
//!
//! **Repère de destination** (M10) : dans le système de destination d'un contrat actif, un
//! repère permanent montre le point de livraison le plus proche et sa distance, comme celui de
//! l'étape Livraison du tutoriel (point projeté, ou flèche au bord de l'écran,
//! [`crate::tutorial::marker_spot`]) ; il s'efface quand le tutoriel montre le sien.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::career::{Career, CargoPoint, deliverable_here};
use crate::data::Texts;
use crate::graphics::WorldCamera;
use crate::options::UserSettings;
use crate::sector::ActiveSector;
use crate::ship::{
    AutopilotRequest, FlightMode, NavTarget, PilotIntent, ShipMotion, SurfaceBody, nearest_surface,
};
use crate::space::SpacePosition;
use crate::state::{ControlMode, ViewLevel};
use crate::transition::DockBeacon;
use crate::tutorial::{Stage, Tutorial, is_active, marker_spot};
use crate::ui::theme::{self, Frame, UiFonts};
use crate::voyage::Location;

pub struct FlightHudPlugin;

impl Plugin for FlightHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Global), spawn_flight_hud)
            .add_systems(
                Update,
                (update_flight_hud, place_destination_marker)
                    .run_if(in_state(ViewLevel::Global))
                    .run_if(resource_exists::<ActiveSector>),
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

/// Largeur de l'étiquette du repère, centrée sur la cible (px), et marge du bord de l'écran
/// pour un repère hors champ (mêmes valeurs que le repère du tutoriel).
const MARKER_WIDTH: f32 = 280.0;
const MARKER_MARGIN: f32 = 48.0;

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
}

#[allow(clippy::too_many_arguments)]
fn update_flight_hud(
    texts: Res<Texts>,
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
    let mode_key = if intent.cruise || speed > 1_000.0 {
        "hud.flight.mode.cruise"
    } else {
        match motion.mode {
            FlightMode::Assisted => "hud.flight.mode.assisted",
            FlightMode::Decoupled => "hud.flight.mode.decoupled",
        }
    };
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
    let mode_text = theme::caps(&texts.get(mode_key));
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
    sector: Res<ActiveSector>,
    ships: Query<&SpacePosition, With<ShipMotion>>,
    points: Query<(&Name, &SpacePosition, &GlobalTransform), With<CargoPoint>>,
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
            .map(|(name, pos, t)| (pos.0.distance(ship), name, t.translation()))
            .min_by(|a, b| a.0.total_cmp(&b.0))
    });
    let shot = match (nearest, camera.single()) {
        (Some((distance, name, point)), Ok((camera, eye))) => {
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
