//! L'horloge du monde (AGENTS.md §7.2) : la date de jeu, affichée « Day N, 425at » (Q3).
//!
//! Dans les vues Locale, Sectorielle et Globale, le temps passe au rythme de `clock.ron` (×30,
//! P4) ; un voyage interstellaire le fait avancer d'un bloc ([`GameClock::advance_hours`]). La
//! carte galactique, les menus et les écrans de chargement l'arrêtent.

use bevy::prelude::*;
use sh_core::{StellarDate, Year};

use crate::data::{Data, Texts};
use crate::state::{AppState, ViewLevel};
use crate::ui::theme::{self, Frame, UiFonts};

pub struct ClockPlugin;

impl Plugin for ClockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), (start_clock, spawn_date).chain())
            .add_systems(
                Update,
                (tick.run_if(time_flows), show_date)
                    .chain()
                    .run_if(resource_exists::<GameClock>),
            );
    }
}

const SECONDS_PER_DAY: f64 = 86_400.0;

/// Le temps du monde passe dans les vues Globale et Sectorielle (et Locale, qui en fait partie).
fn time_flows(view: Option<Res<State<ViewLevel>>>) -> bool {
    view.is_some_and(|v| matches!(v.get(), ViewLevel::Global | ViewLevel::Sector))
}

/// La date du monde, et l'heure du jour en secondes.
#[derive(Resource, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameClock {
    pub date: StellarDate,
    pub seconds: f64,
}

impl GameClock {
    pub fn at(date: StellarDate) -> Self {
        Self { date, seconds: 0.0 }
    }

    /// Fait passer `seconds` secondes de jeu.
    pub fn advance_seconds(&mut self, seconds: f64) {
        let total = self.seconds + seconds.max(0.0);
        let days = (total / SECONDS_PER_DAY).floor();
        self.date = self.date.add_days(days as i64);
        self.seconds = total - days * SECONDS_PER_DAY;
    }

    pub fn advance_hours(&mut self, hours: f64) {
        self.advance_seconds(hours * 3600.0);
    }

    /// Heures écoulées depuis une date et une heure antérieures.
    pub fn hours_since(&self, earlier: &GameClock) -> f64 {
        let days = earlier.date.days_until(self.date) as f64;
        (days * SECONDS_PER_DAY + self.seconds - earlier.seconds) / 3600.0
    }
}

/// La partie commence au jour 1 de 425at (le présent du canon, §0) ; une partie en cours garde
/// sa date.
fn start_clock(mut commands: Commands, clock: Option<Res<GameClock>>) {
    if clock.is_none() {
        let mut clock = GameClock::at(StellarDate::start_of(Year::PRESENT));
        // `SH_DAYS=N` (autopilote) : la partie commence N jours plus tard, le monde vivant
        // rattrape son retard (captures d'un monde qui a vécu).
        if let Some(days) = std::env::var("SH_DAYS")
            .ok()
            .and_then(|d| d.trim().parse::<u32>().ok())
        {
            clock.advance_hours(f64::from(days) * 24.0);
        }
        commands.insert_resource(clock);
    }
}

fn tick(time: Res<Time>, data: Res<Data>, mut clock: ResMut<GameClock>) {
    clock.advance_seconds(f64::from(time.delta_secs()) * data.clock.time_scale);
}

/// La date affichée.
#[derive(Component)]
struct DateText;

/// La colonne du coin haut droit : la date (relevé à chasse fixe), puis le fil des nouvelles
/// (`living`), puis le panneau du tutoriel (`tutorial`). Empilés dans un même nœud, ils ne se
/// chevauchent jamais, quel que soit le nombre de lignes des nouvelles ; un élément caché
/// (`Display::None`) rend sa place.
#[derive(Component)]
pub struct HudCorner;

/// Largeur maximale de cette colonne, en pixels d'interface (le panneau du tutoriel la remplit).
/// Le bandeau du lieu (`views::view_banner`) s'arrête avant.
pub const HUD_CORNER_WIDTH: f32 = 400.0;

pub(crate) fn spawn_date(mut commands: Commands, fonts: Res<UiFonts>) {
    commands.spawn((
        Name::new("Coin haut droit"),
        HudCorner,
        DespawnOnExit(AppState::InGame),
        // Rien n'y retient un clic : la carte galactique reste cliquable dessous.
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            right: px(20),
            top: px(16),
            max_width: px(HUD_CORNER_WIDTH),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            row_gap: px(6),
            ..default()
        },
        children![(
            Name::new("Date du monde"),
            Pickable::IGNORE,
            Node {
                padding: UiRect::axes(px(16), px(9)),
                ..default()
            },
            fonts.frame(Frame::Readout),
            children![(
                DateText,
                Text::new(""),
                fonts.mono(16.0),
                TextColor(theme::TEXT),
                Pickable::IGNORE,
            )],
        )],
    ));
}

/// « Day N, 425at » (clé `date.display`).
pub fn format_date(texts: &Texts, date: StellarDate) -> String {
    texts.format(
        "date.display",
        &[
            ("day", &date.day_of_year().to_string()),
            ("year", &date.year().to_string()),
        ],
    )
}

fn show_date(texts: Res<Texts>, clock: Res<GameClock>, mut text: Query<&mut Text, With<DateText>>) {
    let wanted = format_date(&texts, clock.date);
    for mut t in &mut text {
        if t.0 != wanted {
            t.0.clone_from(&wanted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_jours_et_les_annees_passent() {
        let mut clock = GameClock::at(StellarDate::start_of(Year(425)));
        clock.advance_hours(30.0);
        assert_eq!(clock.date.day_of_year(), 2);
        assert!((clock.seconds - 6.0 * 3600.0).abs() < 1e-6);
        let before = clock.clone();
        clock.advance_hours(24.0 * 365.0);
        assert_eq!(clock.date.year(), Year(426));
        assert!((clock.hours_since(&before) - 24.0 * 365.0).abs() < 1e-6);
    }
}
