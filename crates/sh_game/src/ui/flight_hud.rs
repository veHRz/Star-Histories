//! Bandeau de pilotage (GDD §7), visible aux commandes : vitesse, mode de vol, astre le plus
//! proche. Il remplacera plus tard un affichage diégétique sur les consoles du cockpit.

use bevy::prelude::*;

use crate::data::Texts;
use crate::sector::ActiveSector;
use crate::ship::{FlightMode, PilotIntent, ShipMotion, SurfaceBody, nearest_surface};
use crate::space::SpacePosition;
use crate::state::{ControlMode, ViewLevel};
use crate::transition::DockBeacon;
use crate::ui::theme::{self, UiFonts};

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
    let line = |kind: FlightLine, size: f32, color: Color| {
        (kind, Text::new(""), fonts.body(size), TextColor(color))
    };
    commands.spawn((
        Name::new("Bandeau de pilotage"),
        FlightHud,
        DespawnOnExit(ViewLevel::Global),
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            bottom: px(28),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(18), px(8)),
            ..default()
        },
        UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
        BackgroundColor(theme::PANEL),
        children![
            line(FlightLine::Speed, 26.0, theme::TEXT),
            line(FlightLine::Mode, 15.0, theme::ACCENT),
            line(FlightLine::Nearest, 14.0, theme::TEXT_DIM),
            line(FlightLine::Dock, 15.0, theme::TEXT),
        ],
    ));
}

/// Nombre lisible : groupes de milliers séparés par une espace fine.
fn grouped(value: f64) -> String {
    let digits = format!("{:.0}", value.max(0.0));
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    out
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
        texts.format("hud.flight.speed_ms", &[("value", &grouped(speed))])
    } else {
        texts.format(
            "hud.flight.speed_kms",
            &[("value", &grouped(speed / 1000.0))],
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
            &[("name", name.as_str()), ("distance", &grouped(d / 1000.0))],
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
                &[("name", name.as_str()), ("distance", &grouped(d / 1000.0))],
            )
        });
    for (line, mut text) in &mut lines {
        let wanted = match line {
            FlightLine::Speed => &speed_text,
            FlightLine::Mode => &texts.get(mode_key),
            FlightLine::Nearest => &nearest_text,
            FlightLine::Dock => &dock_text,
        };
        if text.0 != *wanted {
            text.0.clone_from(wanted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::grouped;

    #[test]
    fn milliers_groupes() {
        assert_eq!(grouped(0.4), "0");
        assert_eq!(grouped(245.0), "245");
        assert_eq!(grouped(1523.4), "1\u{202f}523");
        assert_eq!(grouped(1_000_000.0), "1\u{202f}000\u{202f}000");
    }
}
