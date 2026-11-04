//! Bandeau de pilotage (GDD §7), visible aux commandes : vitesse, mode de vol, astre le plus
//! proche, invite d'amarrage. Habillage (M8) : relevé d'instrument à équerres cyan, chiffres à
//! chasse fixe. Il remplacera plus tard un affichage diégétique sur les consoles du cockpit.

use bevy::prelude::*;

use crate::data::Texts;
use crate::sector::ActiveSector;
use crate::ship::{FlightMode, PilotIntent, ShipMotion, SurfaceBody, nearest_surface};
use crate::space::SpacePosition;
use crate::state::{ControlMode, ViewLevel};
use crate::transition::DockBeacon;
use crate::ui::theme::{self, Frame, UiFonts};

pub struct FlightHudPlugin;

impl Plugin for FlightHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Global), spawn_flight_hud)
            .add_systems(
                Update,
                update_flight_hud
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
}

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
}

#[allow(clippy::too_many_arguments)]
fn update_flight_hud(
    texts: Res<Texts>,
    mode: Res<State<ControlMode>>,
    sector: Res<ActiveSector>,
    intent: Res<PilotIntent>,
    ships: Query<(&SpacePosition, &ShipMotion)>,
    bodies: Query<(&SpacePosition, &SurfaceBody, &Name)>,
    beacons: Query<(&SpacePosition, &DockBeacon, &Name)>,
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
    let mode_text = theme::caps(&texts.get(mode_key));
    for (line, mut text) in &mut lines {
        let wanted = match line {
            FlightLine::Speed => &speed_text,
            FlightLine::Mode => &mode_text,
            FlightLine::Nearest => &nearest_text,
            FlightLine::Dock => &dock_text,
        };
        if text.0 != *wanted {
            text.0.clone_from(wanted);
        }
    }
}
