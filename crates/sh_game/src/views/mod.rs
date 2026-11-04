//! Les vues du jeu (AGENTS.md §6). En M0, chacune est une vue **de test** : elle prouve que
//! l'état se charge, se décharge et lit les données. Les vraies vues arrivent en M1 (Sectorielle),
//! M2 (Globale) et M3 (Galaxie).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::clock::HUD_CORNER_WIDTH;
use crate::data::Texts;
use crate::graphics::WorldCamera;
use crate::state::ControlMode;
use crate::ui::theme::{self, Frame, UiFonts};

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
        // GlobalTransform de la caméra du frame précédent : un frame de retard, invisible.
        .add_systems(Update, place_labels);
    }
}

/// Bandeau du lieu et des commandes, en haut à gauche : une plaquette à barre ambre, le nom du
/// lieu en capitales espacées, la ligne des commandes dessous.
///
/// Il s'arrête avant la colonne du coin haut droit (date, nouvelles, tutoriel :
/// [`crate::clock::HudCorner`]) : la ligne des commandes, longue, passe à la ligne plutôt que
/// de passer sous la date. Le nœud extérieur fixe cette largeur permise ; la plaquette prend
/// celle de son texte.
pub fn view_banner(
    texts: &Texts,
    fonts: &UiFonts,
    title: String,
    hint_key: &str,
    scope: impl Component,
) -> impl Bundle {
    (
        scope,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(20),
            top: px(16),
            right: px(HUD_CORNER_WIDTH + 40.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            ..default()
        },
        children![(
            Pickable::IGNORE,
            Node {
                max_width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                // Large à gauche : la barre ambre de la plaquette.
                padding: UiRect::new(px(16), px(14), px(7), px(9)),
                ..default()
            },
            fonts.frame(Frame::Plate),
            children![
                (
                    Text::new(theme::caps(&title)),
                    fonts.heading(17.0),
                    TextColor(theme::TEXT),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(texts.get(hint_key)),
                    fonts.body(14.0),
                    TextColor(theme::TEXT_DIM),
                    Pickable::IGNORE,
                ),
            ],
        )],
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
