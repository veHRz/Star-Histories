//! Les vues du jeu (AGENTS.md §6). En M0, chacune est une vue **de test** : elle prouve que
//! l'état se charge, se décharge et lit les données. Les vraies vues arrivent en M1 (Sectorielle),
//! M2 (Globale) et M3 (Galaxie).
//!
//! Deux sortes d'étiquettes d'interface accrochées au monde vivent ici : [`WorldLabel`] (repères
//! de navigation, aux commandes) et [`FootLabel`] (M10 : petites plaques lues à pied, noms des
//! salles aux portes, noms des casiers), que n'importe quel domaine pose sur une entité.
//!
//! **Pas d'écriture inutile** (M11, retour 7 : lisser les vrais i/s) : dans Bevy, écrire un
//! composant par `Mut` le marque modifié même si la valeur ne change pas. Un `Node` modifié
//! relance la mise en page de l'interface, une `Visibility` sa propagation, une couleur de texte
//! sa préparation. Les étiquettes ne réécrivent donc leur place que si elle bouge d'au moins
//! [`LABEL_MOVE_EPSILON`] ([`set_px_if_changed`], partagé avec les repères du tutoriel et du
//! pilotage), leur opacité que si elle change d'au moins [`LABEL_ALPHA_EPSILON`], leur visibilité
//! que si elle change : joueur immobile, rien n'est recalculé.
//!
//! Les étiquettes à pied laissent libres le coin haut droit, le bandeau du lieu et tout élément
//! marqué [`LabelFreeZone`] : elles s'y effacent au lieu de s'y lire au travers.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::clock::{HUD_CORNER_WIDTH, HudCorner};
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
/// M9 : les commandes n'y sont plus (encart des touches, `ui::keys_panel`) ; M11 : la clé de
/// leur ligne d'aide a quitté la signature avec les textes `view.*.hint`. Le bandeau s'arrête
/// avant la colonne du coin haut droit (date, nouvelles, tutoriel : [`crate::clock::HudCorner`]) :
/// le nœud extérieur fixe cette largeur permise, la plaquette prend celle de son texte.
pub fn view_banner(
    _texts: &Texts,
    fonts: &UiFonts,
    title: String,
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
            // La plaquette, pas le nœud extérieur : celui-ci s'étend jusqu'au coin haut droit, et
            // les étiquettes à pied s'effaceraient sur toute la largeur du haut de l'écran.
            LabelFreeZone,
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
            WorldLabel::At(point) => Some(*point),
            WorldLabel::Follow(_) if !follow_visible => None,
            WorldLabel::Follow(entity) => targets.get(*entity).ok().map(|t| t.translation()),
        };
        // Rien n'est écrit qui n'ait changé (en-tête du module).
        match anchor.and_then(|a| camera.world_to_viewport(camera_transform, a).ok()) {
            Some(point) => {
                let point = point * to_window;
                set_px_if_changed(&mut node, point.x + 7.0, point.y - 9.0);
                visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

// --- Écritures économes (M11) --------------------------------------------------------------------

/// Déplacement (px d'interface) en dessous duquel une étiquette n'est pas replacée : l'interface
/// arrondit de toute façon au pixel, et chaque écriture du `Node` relance la mise en page.
const LABEL_MOVE_EPSILON: f32 = 0.5;

/// Variation d'opacité en dessous de laquelle une étiquette garde la sienne (invisible à l'œil ;
/// chaque écriture d'une couleur de texte relance sa préparation).
const LABEL_ALPHA_EPSILON: f32 = 0.005;

/// Faut-il réécrire cette coordonnée ? Oui si elle n'est pas en pixels ou si elle bouge d'au
/// moins [`LABEL_MOVE_EPSILON`]. Logique pure.
fn px_moved(current: Val, wanted: f32) -> bool {
    match current {
        Val::Px(at) => (at - wanted).abs() >= LABEL_MOVE_EPSILON,
        _ => true,
    }
}

/// Place un nœud en absolu (`left`, `top`, px d'interface) sans le marquer modifié s'il n'a pas
/// bougé d'au moins [`LABEL_MOVE_EPSILON`] : la lecture d'un `Mut` ne compte pas comme une
/// écriture, seule l'affectation le fait. Étiquettes, repères du tutoriel et du pilotage.
pub(crate) fn set_px_if_changed(node: &mut Mut<Node>, left: f32, top: f32) {
    if px_moved(node.left, left) {
        node.left = px(left);
    }
    if px_moved(node.top, top) {
        node.top = px(top);
    }
}

/// Faut-il réécrire cette opacité ? Oui si elle change d'au moins [`LABEL_ALPHA_EPSILON`], et
/// toujours pour atteindre exactement 0 ou 1 (une plaque finit pleinement nette ou effacée,
/// comme avant). Logique pure.
fn alpha_changed(current: f32, wanted: f32) -> bool {
    current != wanted
        && ((current - wanted).abs() >= LABEL_ALPHA_EPSILON || wanted <= 0.0 || wanted >= 1.0)
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

/// Un élément du HUD qu'une étiquette à pied ne recouvre jamais (M11, plaque vue sur la date) :
/// la plaque s'efface en l'approchant, et a disparu quand elle le touche. Le bandeau du lieu le
/// porte ; le coin haut droit (date, nouvelles, tutoriel : [`crate::clock::HudCorner`]) en est un
/// d'office ; tout autre élément du HUD peut le recevoir (l'encart des touches).
///
/// Pourquoi un effacement et pas l'ordre de dessin : les plaques sont **déjà** dessinées sous
/// tout le HUD (`GlobalZIndex(-1)`, sous les racines à 0 et l'encart à 4). Mais ces éléments ne
/// cachent pas ce qui passe dessous : le relevé de la date est un verre sombre translucide, les
/// plaquettes peintes ne sont opaques qu'aux deux tiers, le fil des nouvelles n'a pas de fond.
/// Une plaque dessous s'y lisait au travers, mêlée à la date.
#[derive(Component)]
pub struct LabelFreeZone;

/// Distance (px d'interface) sur laquelle une étiquette à pied s'efface en approchant d'une
/// [`LabelFreeZone`] : un fondu dans l'espace, sans retard (elle ne passe jamais dessus, même
/// quand la vue tourne vite), et pas un saut au bord.
const FREE_ZONE_FADE: f32 = 24.0;

/// La place d'une plaque de taille `size` dont le bas est centré sur `point` (px d'interface,
/// comme `UiTransform::from_translation(Val2::percent(-50, -100))`).
fn plate_rect(point: Vec2, size: Vec2) -> Rect {
    Rect::new(
        point.x - size.x / 2.0,
        point.y - size.y,
        point.x + size.x / 2.0,
        point.y,
    )
}

/// Écart entre deux rectangles (px) : 0 s'ils se touchent ou se chevauchent.
fn rect_gap(a: &Rect, b: &Rect) -> f32 {
    let dx = (a.min.x - b.max.x).max(b.min.x - a.max.x).max(0.0);
    let dy = (a.min.y - b.max.y).max(b.min.y - a.max.y).max(0.0);
    Vec2::new(dx, dy).length()
}

/// Opacité permise à une plaque par les zones libres : 0 sur l'une d'elles, 1 à
/// [`FREE_ZONE_FADE`] px ou plus de toutes, un fondu entre les deux. Logique pure.
fn free_zone_factor(plate: &Rect, zones: &[Rect]) -> f32 {
    zones
        .iter()
        .map(|zone| (rect_gap(plate, zone) / FREE_ZONE_FADE).clamp(0.0, 1.0))
        .fold(1.0, f32::min)
}

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

/// Les zones du HUD que les étiquettes à pied laissent libres (voir [`LabelFreeZone`]).
type FreeZones<'w, 's> = Query<
    'w,
    's,
    (
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static InheritedVisibility,
    ),
    Or<(With<HudCorner>, With<LabelFreeZone>)>,
>;

/// Place les plaques à l'écran et règle leur fondu ; rien n'est écrit qui n'ait changé (en-tête
/// du module).
#[allow(clippy::too_many_arguments)]
fn place_foot_labels(
    time: Res<Time<Real>>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mode: Res<State<ControlMode>>,
    states: Option<Res<CellStates>>,
    zones: FreeZones,
    mut labels: Query<(&FootLabel, &GlobalTransform, &mut FootPlateOf)>,
    mut plates: Query<(&mut Node, &mut Visibility, &mut ImageNode, &ComputedNode), With<FootPlate>>,
    mut texts: Query<(&mut TextColor, &mut TextShadow), With<FootPlateText>>,
) {
    let view = camera.single().ok().zip(window.single().ok());
    let on_foot = *mode.get() == ControlMode::OnFoot;
    let current = states.as_ref().and_then(|s| s.current.as_deref());
    let step = time.delta_secs() / FOOT_FADE_TIME;
    // Leur place du frame précédent, en px logiques (comme les repères du pilotage) ; un élément
    // caché ou replié (`Display::None` : taille nulle) ne retient rien.
    let free_zones: Vec<Rect> = zones
        .iter()
        .filter(|(node, _, visible)| visible.get() && !node.is_empty())
        .map(|(node, transform, _)| {
            let scale = node.inverse_scale_factor();
            Rect::from_center_size(transform.translation * scale, node.size() * scale)
        })
        .collect();
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
        if of.alpha != alpha {
            of.alpha = alpha;
        }
        let Ok((mut node, mut visibility, mut image, computed)) = plates.get_mut(of.plate) else {
            continue;
        };
        // Près d'une zone libre du HUD, la plaque s'efface sur place (sans retard : `of.alpha`
        // garde le fondu de distance et de côté, que la zone ne fait que masquer).
        let drawn = point.map_or(0.0, |point| {
            let size = computed.size() * computed.inverse_scale_factor();
            alpha * free_zone_factor(&plate_rect(point, size), &free_zones)
        });
        let shown = drawn > 0.01;
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let (true, Some(point)) = (shown, point) {
            set_px_if_changed(&mut node, point.x, point.y);
        }
        // Le cadre, le texte et son ombre changent ensemble : l'opacité du cadre fait foi.
        if alpha_changed(image.color.alpha(), drawn) {
            image.color = image.color.with_alpha(drawn);
            if let Ok((mut color, mut shadow)) = texts.get_mut(of.text) {
                color.0 = color.0.with_alpha(drawn);
                shadow.color = shadow.color.with_alpha(0.85 * drawn);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_position_ne_se_reecrit_qu_au_dela_du_demi_pixel() {
        assert!(px_moved(Val::Auto, 10.0));
        assert!(px_moved(percent(50), 10.0));
        assert!(!px_moved(px(100.0), 100.0));
        assert!(!px_moved(px(100.0), 100.3));
        assert!(!px_moved(px(100.0), 99.7));
        assert!(px_moved(px(100.0), 100.6));
        assert!(px_moved(px(100.0), 99.4));
    }

    #[test]
    fn une_opacite_ne_se_reecrit_que_si_elle_se_voit() {
        assert!(!alpha_changed(1.0, 1.0));
        assert!(!alpha_changed(0.0, 0.0));
        assert!(!alpha_changed(0.5, 0.503));
        assert!(alpha_changed(0.5, 0.51));
        assert!(alpha_changed(0.5, 0.49));
        // Les fins de fondu se rejoignent toujours exactement : nette, ou effacée.
        assert!(alpha_changed(0.998, 1.0));
        assert!(alpha_changed(0.002, 0.0));
    }

    #[test]
    fn une_plaque_s_efface_pres_des_zones_libres_du_hud() {
        // Le bas de la plaque est centré sur le point d'accroche.
        let plate = plate_rect(Vec2::new(500.0, 300.0), Vec2::new(120.0, 24.0));
        assert_eq!(plate, Rect::new(440.0, 276.0, 560.0, 300.0));
        // Sans zone, ou loin de toutes : pleinement visible.
        assert_eq!(free_zone_factor(&plate, &[]), 1.0);
        let far = Rect::new(1200.0, 16.0, 1580.0, 120.0);
        assert_eq!(free_zone_factor(&plate, &[far]), 1.0);
        // Sur une zone (la date, en haut à droite) : effacée.
        let corner = Rect::new(1380.0, 16.0, 1580.0, 56.0);
        let on_corner = plate_rect(Vec2::new(1400.0, 40.0), Vec2::new(120.0, 24.0));
        assert_eq!(free_zone_factor(&on_corner, &[far, corner]), 0.0);
        // Contre son bord : effacée aussi (rien ne la touche).
        let touching = plate_rect(Vec2::new(1320.0, 40.0), Vec2::new(120.0, 24.0));
        assert_eq!(rect_gap(&touching, &corner), 0.0);
        assert_eq!(free_zone_factor(&touching, &[corner]), 0.0);
        // À mi-chemin de la bande de fondu : à moitié.
        let near = plate_rect(
            Vec2::new(1380.0 - 60.0 - FREE_ZONE_FADE / 2.0, 40.0),
            Vec2::new(120.0, 24.0),
        );
        assert!((free_zone_factor(&near, &[corner]) - 0.5).abs() < 1e-5);
        // La zone la plus proche décide : 6 px sous la date, une autre zone bien plus loin.
        let below = plate_rect(Vec2::new(1450.0, 56.0 + 24.0 + 6.0), Vec2::new(120.0, 24.0));
        assert!((rect_gap(&below, &corner) - 6.0).abs() < 1e-5);
        let keys = Rect::new(20.0, 88.0, 280.0, 400.0);
        let factor = free_zone_factor(&below, &[keys, corner]);
        assert!((factor - 6.0 / FREE_ZONE_FADE).abs() < 1e-5);
    }

    #[derive(Resource)]
    struct Wanted(Vec2);

    #[derive(Resource, Default)]
    struct Writes(usize);

    fn move_nodes(wanted: Res<Wanted>, mut nodes: Query<&mut Node>) {
        for mut node in &mut nodes {
            set_px_if_changed(&mut node, wanted.0.x, wanted.0.y);
        }
    }

    fn count_writes(mut writes: ResMut<Writes>, changed: Query<(), Changed<Node>>) {
        writes.0 += changed.iter().count();
    }

    /// Le vrai critère : un nœud immobile n'est pas marqué modifié (pas de mise en page).
    #[test]
    fn un_noeud_immobile_n_est_pas_marque_modifie() {
        let mut app = App::new();
        app.init_resource::<Writes>()
            .insert_resource(Wanted(Vec2::new(100.0, 50.0)))
            .add_systems(Update, (move_nodes, count_writes).chain());
        let node = app
            .world_mut()
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(100.0),
                top: px(50.0),
                ..default()
            })
            .id();
        let writes = |app: &App| app.world().resource::<Writes>().0;
        let place = |app: &App| app.world().get::<Node>(node).map(|n| (n.left, n.top));
        // Le premier passage voit l'ajout du nœud.
        app.update();
        let added = writes(&app);
        assert_eq!(added, 1);
        // Immobile : aucune écriture.
        app.update();
        app.update();
        assert_eq!(writes(&app), added);
        // Sous le demi-pixel : toujours rien.
        app.world_mut().resource_mut::<Wanted>().0 = Vec2::new(100.3, 49.8);
        app.update();
        assert_eq!(writes(&app), added);
        assert_eq!(place(&app), Some((px(100.0), px(50.0))));
        // Un vrai déplacement : une écriture, de la seule coordonnée qui a bougé.
        app.world_mut().resource_mut::<Wanted>().0 = Vec2::new(103.0, 49.8);
        app.update();
        assert_eq!(writes(&app), added + 1);
        assert_eq!(place(&app), Some((px(103.0), px(50.0))));
    }
}
