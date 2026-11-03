//! Les vues du jeu (AGENTS.md §6). En M0, chacune est une vue **de test** : elle prouve que
//! l'état se charge, se décharge et lit les données. Les vraies vues arrivent en M1 (Sectorielle),
//! M2 (Globale) et M3 (Galaxie).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_enhanced_input::prelude::*;

use crate::data::Texts;
use crate::graphics::WorldCamera;
use crate::input::Pause;
use crate::state::{AppState, ControlMode};
use crate::ui::theme::{self, UiFonts};

pub mod galaxy;
pub mod global;
pub mod planet;
pub mod procedural;
pub mod sector;
pub mod sky;

pub struct ViewsPlugin;

impl Plugin for ViewsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            galaxy::GalaxyViewPlugin,
            global::GlobalViewPlugin,
            sector::SectorViewPlugin,
            procedural::ProceduralPlugin,
        ))
        .add_observer(on_pause)
        // GlobalTransform de la caméra du frame précédent : un frame de retard, invisible.
        .add_systems(Update, place_labels);
    }
}

/// Échap en partie : retour au menu principal (le vrai menu de pause viendra avec M7).
fn on_pause(
    _: On<Start<Pause>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if *state.get() == AppState::InGame {
        next.set(AppState::MainMenu);
    }
}

/// Bandeau de titre d'une vue de test, en haut à gauche.
pub fn view_banner(
    texts: &Texts,
    fonts: &UiFonts,
    title: String,
    hint_key: &str,
    scope: impl Component,
) -> impl Bundle {
    (
        scope,
        Node {
            position_type: PositionType::Absolute,
            left: px(20),
            top: px(16),
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(px(14), px(10)),
            ..default()
        },
        BackgroundColor(theme::PANEL),
        children![
            (Text::new(title), fonts.title(22.0), TextColor(theme::TEXT)),
            (
                Text::new(texts.get(hint_key)),
                fonts.body(14.0),
                TextColor(theme::TEXT_DIM)
            ),
        ],
    )
}

/// Étiquette d'interface accrochée à un point du monde (noms sur les cartes de test).
#[derive(Component)]
pub enum WorldLabel {
    /// Un point fixe du rendu (carte galactique).
    At(Vec3),
    /// Une entité : l'étiquette la suit, y compris quand l'origine flottante se déplace.
    Follow(Entity),
}

pub(crate) fn place_labels(
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut labels: Query<(&WorldLabel, &mut Node, &mut Visibility)>,
    targets: Query<&GlobalTransform>,
    mode: Res<State<ControlMode>>,
) {
    // Les repères des astres sont une aide de navigation : affichés aux commandes seulement.
    let follow_visible = *mode.get() == ControlMode::Piloting;
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    // Avec l'échelle de rendu, la caméra dessine dans une image plus petite que la fenêtre.
    let to_window = camera
        .logical_viewport_size()
        .map_or(Vec2::ONE, |size| window.size() / size.max(Vec2::ONE));
    for (label, mut node, mut visibility) in &mut labels {
        let anchor = match label {
            WorldLabel::At(point) => *point,
            WorldLabel::Follow(_) if !follow_visible => {
                *visibility = Visibility::Hidden;
                continue;
            }
            WorldLabel::Follow(entity) => match targets.get(*entity) {
                Ok(t) => t.translation(),
                Err(_) => {
                    *visibility = Visibility::Hidden;
                    continue;
                }
            },
        };
        match camera.world_to_viewport(camera_transform, anchor) {
            Ok(point) => {
                let point = point * to_window;
                node.left = px(point.x + 7.0);
                node.top = px(point.y - 9.0);
                *visibility = Visibility::Inherited;
            }
            Err(_) => *visibility = Visibility::Hidden,
        }
    }
}
