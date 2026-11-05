//! Les vues du jeu (AGENTS.md §6). En M0, chacune est une vue **de test** : elle prouve que
//! l'état se charge, se décharge et lit les données. Les vraies vues arrivent en M1 (Sectorielle),
//! M2 (Globale) et M3 (Galaxie).
//!
//! Deux sortes d'étiquettes d'interface accrochées au monde vivent ici : [`WorldLabel`] (repères
//! de navigation, aux commandes) et [`FootLabel`] (M10 : petites plaques lues à pied, noms des
//! salles aux portes, noms des casiers), que n'importe quel domaine pose sur une entité.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::clock::HUD_CORNER_WIDTH;
use crate::data::Texts;
use crate::graphics::WorldCamera;
use crate::sector::streaming::CellStates;
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
        .add_systems(
            Update,
            (
                place_labels,
                (spawn_foot_plates, sync_foot_texts, place_foot_labels).chain(),
            ),
        )
        .add_observer(drop_foot_plate);
    }
}

/// Bas du bandeau du lieu (deux lignes : le lieu, puis le détail), en pixels d'interface : ce qui
/// se place dessous (encart des touches, conseils) part de là.
pub const BANNER_BOTTOM: f32 = 76.0;

/// Le bandeau du lieu (pour le retrouver).
#[derive(Component)]
pub struct PlaceBanner;

/// La seconde ligne du bandeau : où l'on est dans le lieu (« Votre vaisseau · Cockpit »),
/// remplie par `ui::hud` d'après la cellule du joueur ; cachée tant qu'elle est vide.
#[derive(Component)]
pub struct BannerDetail;

/// Bandeau du lieu, en haut à gauche : une plaquette à barre ambre, le nom du lieu en capitales
/// espacées et, dessous, le détail ([`BannerDetail`] : à bord de quoi, dans quelle salle).
///
/// M9 : les commandes n'y sont plus (encart des touches, `ui::keys_panel`) ; `_hint_key` reste
/// dans la signature pour les appelants des vues, il ne sert plus. Le bandeau s'arrête avant la
/// colonne du coin haut droit (date, nouvelles, tutoriel : [`crate::clock::HudCorner`]) : le
/// nœud extérieur fixe cette largeur permise, la plaquette prend celle de son texte.
pub fn view_banner(
    _texts: &Texts,
    fonts: &UiFonts,
    title: String,
    _hint_key: &str,
    scope: impl Component,
) -> impl Bundle {
    (
        scope,
        PlaceBanner,
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
                row_gap: px(2),
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
                    BannerDetail,
                    Text::new(""),
                    fonts.body(14.0),
                    TextColor(theme::TEXT_DIM),
                    Pickable::IGNORE,
                    Node {
                        display: Display::None,
                        ..default()
                    },
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

// --- Étiquettes à pied (M10) ---------------------------------------------------------------------

/// Une étiquette lue **à pied** : une petite plaque d'interface (cadre `Plate`, capitales)
/// accrochée à l'entité qui porte ce composant, au point de son `GlobalTransform` ; le bas de
/// la plaque est centré sur ce point (elle se lit au-dessus).
///
/// Visible seulement en mode à pied, devant la caméra, à moins de `max_distance`, du côté où
/// pointe `normal` et, si `cell` est donnée, quand le joueur est dans cette cellule (les cloisons
/// ne cachent pas l'interface : la cellule fait office d'occultation). Elle apparaît et
/// s'efface en fondu. Le texte est déjà traduit : changer `text` met la plaque à jour (le
/// changement est détecté, aucune recherche par frame). Retirer le composant, ou l'entité,
/// retire la plaque.
///
/// Usage (nom d'un casier sur sa plaque, `SOCKET_<n>_label` dont l'avant pointe hors du casier) :
/// `commands.entity(socket).insert(FootLabel::new(nom).facing(Vec3::NEG_Z).in_cell(cellule))`.
#[derive(Component, Clone, Debug, PartialEq)]
#[require(Transform)]
pub struct FootLabel {
    /// Texte affiché (traduit), mis en capitales à l'affichage.
    pub text: String,
    /// Côté lisible, dans le repère de l'entité (tourné avec elle) ; `Vec3::ZERO` : de partout.
    pub normal: Vec3,
    /// Distance de lecture (m) : au-delà, la plaque s'efface.
    pub max_distance: f32,
    /// Cellule du secteur d'où l'étiquette se lit (`CellStates::current`) ; `None` : partout.
    pub cell: Option<String>,
}

impl FootLabel {
    /// Distance de lecture par défaut (m).
    pub const DEFAULT_DISTANCE: f32 = 6.0;

    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            normal: Vec3::ZERO,
            max_distance: Self::DEFAULT_DISTANCE,
            cell: None,
        }
    }

    /// Lisible seulement du côté où pointe `normal` (repère de l'entité).
    pub fn facing(mut self, normal: Vec3) -> Self {
        self.normal = normal;
        self
    }

    /// Lisible seulement depuis cette cellule.
    pub fn in_cell(mut self, cell: impl Into<String>) -> Self {
        self.cell = Some(cell.into());
        self
    }

    pub fn within(mut self, distance: f32) -> Self {
        self.max_distance = distance;
        self
    }

    /// Opacité voulue pour une caméra en `eye` (regard `forward`), selon la distance et le côté :
    /// 1 de près et de face, puis fondu sur le dernier mètre et en biais ; 0 derrière la caméra.
    /// `normal` : la normale déjà dans le repère du monde.
    pub fn target_alpha(&self, anchor: Vec3, normal: Vec3, eye: Vec3, forward: Vec3) -> f32 {
        let to_anchor = anchor - eye;
        if to_anchor.dot(forward) < 0.1 {
            return 0.0;
        }
        let distance = to_anchor.length();
        let near = ((self.max_distance - distance) / FOOT_FADE_BAND).clamp(0.0, 1.0);
        let facing = match normal.try_normalize() {
            // Cosinus entre la normale et la direction de la caméra : de dos ou très en biais,
            // la plaque disparaît.
            Some(n) => ((-to_anchor.normalize_or_zero().dot(n) - 0.15) / 0.25).clamp(0.0, 1.0),
            None => 1.0,
        };
        near * facing
    }
}

/// Fondu des étiquettes à pied : largeur de la bande de distance (m), et durée (s).
const FOOT_FADE_BAND: f32 = 1.0;
const FOOT_FADE_TIME: f32 = 0.2;

/// La plaque d'une [`FootLabel`] (sur l'entité de l'étiquette) et son opacité courante.
#[derive(Component)]
struct FootPlateOf {
    plate: Entity,
    text: Entity,
    alpha: f32,
}

/// Marques des nœuds d'une plaque (requêtes disjointes).
#[derive(Component)]
struct FootPlate;
#[derive(Component)]
struct FootPlateText;

/// Crée la plaque d'interface d'une étiquette nouvelle, cachée jusqu'au placement.
fn spawn_foot_plates(
    mut commands: Commands,
    fonts: Option<Res<UiFonts>>,
    labels: Query<(Entity, &FootLabel), Without<FootPlateOf>>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    for (entity, label) in &labels {
        let text = commands
            .spawn((
                FootPlateText,
                Text::new(theme::caps(&label.text)),
                fonts.heading(12.0),
                TextColor(theme::TEXT.with_alpha(0.0)),
                theme::label_shadow(),
                TextLayout::no_wrap(),
                Pickable::IGNORE,
            ))
            .id();
        let mut frame = fonts.frame(Frame::Plate);
        frame.color = frame.color.with_alpha(0.0);
        let plate = commands
            .spawn((
                Name::new("Étiquette à pied"),
                FootPlate,
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    // Large à gauche : la barre ambre de la plaquette.
                    padding: UiRect::new(px(12), px(9), px(4), px(5)),
                    ..default()
                },
                // Le bas de la plaque, centré sur le point d'accroche.
                UiTransform::from_translation(Val2::percent(-50, -100)),
                frame,
                // Sous le reste de l'interface (HUD, invites, écrans).
                GlobalZIndex(-1),
                Visibility::Hidden,
            ))
            .add_child(text)
            .id();
        commands.entity(entity).insert(FootPlateOf {
            plate,
            text,
            alpha: 0.0,
        });
    }
}

/// Le texte d'une étiquette a changé (un casier renommé) : la plaque suit.
fn sync_foot_texts(
    labels: Query<(&FootLabel, &FootPlateOf), Changed<FootLabel>>,
    mut texts: Query<&mut Text, With<FootPlateText>>,
) {
    for (label, of) in &labels {
        if let Ok(mut text) = texts.get_mut(of.text) {
            let caps = theme::caps(&label.text);
            if text.0 != caps {
                text.0 = caps;
            }
        }
    }
}

/// L'étiquette disparaît (son entité ou le composant) : sa plaque aussi.
fn drop_foot_plate(remove: On<Remove<FootLabel>>, mut commands: Commands, of: Query<&FootPlateOf>) {
    if let Ok(of) = of.get(remove.entity) {
        commands.entity(of.plate).try_despawn();
        commands.entity(remove.entity).try_remove::<FootPlateOf>();
    }
}

/// Place les plaques à l'écran et règle leur fondu.
#[allow(clippy::too_many_arguments)]
fn place_foot_labels(
    time: Res<Time<Real>>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mode: Res<State<ControlMode>>,
    states: Option<Res<CellStates>>,
    mut labels: Query<(&FootLabel, &GlobalTransform, &mut FootPlateOf)>,
    mut plates: Query<(&mut Node, &mut Visibility, &mut ImageNode), With<FootPlate>>,
    mut texts: Query<(&mut TextColor, &mut TextShadow), With<FootPlateText>>,
) {
    let view = camera.single().ok().zip(window.single().ok());
    let on_foot = *mode.get() == ControlMode::OnFoot;
    let current = states.as_ref().and_then(|s| s.current.as_deref());
    let step = time.delta_secs() / FOOT_FADE_TIME;
    for (label, at, mut of) in &mut labels {
        let anchor = at.translation();
        let mut target = 0.0;
        let mut point = None;
        if let Some(((camera, camera_transform), window)) = view
            && on_foot
            && label.cell.as_deref().is_none_or(|c| Some(c) == current)
        {
            let eye = camera_transform.translation();
            let normal = at.rotation() * label.normal;
            target = label.target_alpha(anchor, normal, eye, *camera_transform.forward());
            if target > 0.0 {
                point = camera
                    .world_to_viewport(camera_transform, anchor)
                    .ok()
                    .map(|p| {
                        // Avec l'échelle de rendu, la caméra dessine dans une image plus petite que
                        // la fenêtre.
                        let to_window = camera
                            .logical_viewport_size()
                            .map_or(Vec2::ONE, |size| window.size() / size.max(Vec2::ONE));
                        p * to_window
                    });
            }
        }
        // Hors de l'écran : effacée tout de suite (rien à fondre là où on ne la voit pas).
        let alpha = if point.is_some() {
            of.alpha + (target - of.alpha).clamp(-step, step)
        } else {
            0.0
        };
        if of.alpha <= 0.0 && alpha <= 0.0 {
            continue;
        }
        of.alpha = alpha;
        let Ok((mut node, mut visibility, mut image)) = plates.get_mut(of.plate) else {
            continue;
        };
        let shown = alpha > 0.01;
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let (true, Some(point)) = (shown, point) {
            node.left = px(point.x);
            node.top = px(point.y);
        }
        image.color = image.color.with_alpha(alpha);
        if let Ok((mut color, mut shadow)) = texts.get_mut(of.text) {
            color.0 = color.0.with_alpha(alpha);
            shadow.color = shadow.color.with_alpha(0.85 * alpha);
        }
    }
}
