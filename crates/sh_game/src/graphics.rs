//! Préréglages graphiques (Bas / Moyen / Haut) et échelle de rendu (AGENTS.md §10).
//!
//! Les valeurs vivent dans `assets/data/gameplay/graphics.ron`. Ce module les applique :
//! - à chaque caméra du monde ([`WorldCamera`]) : HDR, bloom, anticrénelage, pas de MSAA ;
//! - à la lumière directionnelle : ombres, cascades, taille de la carte d'ombre ;
//! - à l'échelle de rendu : sous 1, la scène 3D est rendue dans une image plus petite, affichée
//!   en plein écran sous l'interface, qui reste à pleine résolution ;
//! - à la résolution dynamique (M11, option activée par défaut pour le joueur, `SH_DYNRES=1` pour
//!   l'autopilote) : l'échelle descend par paliers de 0,05 sous celle du préréglage, jusqu'à 0,5,
//!   quand le rendu dépasse son budget, et remonte quand le GPU a de la marge
//!   ([`ScaleController`], mesures du limiteur de `frame_pace`) ; sans réallocation d'image ;
//! - au décor par niveau (M9, docs/m9-retours.md §3.2) : un nœud glTF de décor optionnel
//!   ([`DecorDetail`], extra `detail`) est caché avec ses enfants si son niveau dépasse
//!   `decor_detail` ; un changement de préréglage s'applique en direct, sans recharger la cellule
//!   (le réglage à l'arrivée d'un modèle est fait par `sector::gltf`) ;
//! - au budget d'ombres des lumières ponctuelles (M11 : seulement les lumières visibles, et sans
//!   va-et-vient entre deux lumières presque à égale distance, `shadow_budget`).

use bevy::anti_alias::fxaa::Fxaa;
use bevy::anti_alias::smaa::Smaa;
use bevy::camera::visibility::VisibilitySystems;
use bevy::camera::{CameraOutputMode, Hdr, RenderTarget, Viewport};
use bevy::light::cluster::GlobalClusterSettings;
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::{BlendState, TextureFormat};
use bevy::render::view::Msaa;
use bevy::window::PrimaryWindow;
use sh_data::{AntiAliasing, GraphicsPreset, PresetLevel, ShadowFilter};

use crate::data::Data;
use crate::frame_pace::{FrameLimit, PaceShared, RenderSample};
use crate::options::UserSettings;
use crate::sector::gltf::{DecorDetail, decor_visibility};
use crate::ui::UiCamera;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DynamicResolution>()
            .add_systems(Startup, cpu_light_clustering)
            .add_systems(
                PostUpdate,
                (
                    init_settings.run_if(resource_added::<Data>),
                    // Une condition par système (AGENTS.md §4, groupes chaînés).
                    (
                        configure_cameras.run_if(resource_exists::<GraphicsSettings>),
                        configure_directional_lights.run_if(resource_exists::<GraphicsSettings>),
                        control_render_scale.run_if(resource_exists::<GraphicsSettings>),
                        update_render_scale.run_if(resource_exists::<GraphicsSettings>),
                        sync_ui_clear.run_if(resource_exists::<GraphicsSettings>),
                        budget_point_shadows.run_if(resource_exists::<GraphicsSettings>),
                    )
                        .chain(),
                )
                    .chain()
                    .before(TransformSystems::Propagate),
            )
            // Ne tourne que le frame où le réglage change : rien à payer sinon.
            .add_systems(
                PostUpdate,
                apply_decor_detail
                    .run_if(resource_exists_and_changed::<GraphicsSettings>)
                    .before(VisibilitySystems::VisibilityPropagate),
            );
    }
}

/// Le réglage graphique actif.
#[derive(Resource, Clone, Debug)]
pub struct GraphicsSettings {
    pub level: PresetLevel,
    pub preset: GraphicsPreset,
}

/// Marque la caméra qui dessine le monde 3D de la vue courante.
#[derive(Component, Default)]
pub struct WorldCamera;

/// Image intermédiaire de l'échelle de rendu, affichée en plein écran.
#[derive(Component)]
struct ScaledViewImage;

#[derive(Resource)]
struct ScaledTarget {
    image: Handle<Image>,
    size: UVec2,
}

/// Préréglage demandé par variable d'environnement (`SH_PRESET=low|medium|high`), pour mesurer.
fn level_from_env() -> Option<PresetLevel> {
    match std::env::var("SH_PRESET").ok()?.trim() {
        "low" => Some(PresetLevel::Low),
        "medium" => Some(PresetLevel::Medium),
        "high" => Some(PresetLevel::High),
        other => {
            warn!("SH_PRESET inconnu : « {other} » (low, medium, high)");
            None
        }
    }
}

/// Préréglage du lancement : `SH_PRESET` (autopilote) l'emporte sur le choix du joueur.
pub fn startup_level(user: Option<&crate::options::UserSettings>) -> PresetLevel {
    level_from_env()
        .or(user.map(|u| u.preset))
        .unwrap_or_default()
}

fn init_settings(
    mut commands: Commands,
    data: Res<Data>,
    user: Option<Res<crate::options::UserSettings>>,
) {
    let level = startup_level(user.as_deref());
    let mut preset = data.graphics.get(level).clone();
    // `SH_BAKED_LIGHTING=0` : tout l'éclairage en dynamique, pour comparer les deux voies.
    if let Ok(v) = std::env::var("SH_BAKED_LIGHTING") {
        preset.baked_lighting = v.trim() != "0";
    }
    info!("préréglage graphique : {level:?} ({preset:?})");
    commands.insert_resource(DirectionalLightShadowMap {
        size: preset.shadow_map_size as usize,
    });
    commands.insert_resource(GraphicsSettings { level, preset });
}

fn configure_cameras(
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    cameras: Query<Entity, With<WorldCamera>>,
    added: Query<(), Added<WorldCamera>>,
) {
    if !settings.is_changed() && added.is_empty() {
        return;
    }
    let p = &settings.preset;
    for entity in &cameras {
        let mut camera = commands.entity(entity);
        // Le MSAA coûte trop cher sur GPU intégré : anticrénelage en post-traitement.
        camera.insert((Msaa::Off, Hdr));
        camera.insert(match p.shadow_filter {
            ShadowFilter::Hardware2x2 => ShadowFilteringMethod::Hardware2x2,
            ShadowFilter::Gaussian => ShadowFilteringMethod::Gaussian,
        });
        camera.remove::<(Fxaa, Smaa, Bloom)>();
        match p.anti_aliasing {
            AntiAliasing::Off => {}
            AntiAliasing::Fxaa => {
                camera.insert(Fxaa::default());
            }
            AntiAliasing::Smaa => {
                camera.insert(Smaa::default());
            }
        }
        if p.bloom {
            camera.insert(Bloom::NATURAL);
        }
    }
}

fn configure_directional_lights(
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    mut lights: Query<(Entity, &mut DirectionalLight)>,
) {
    let p = &settings.preset;
    if settings.is_changed() && shadow_map.size != p.shadow_map_size as usize {
        shadow_map.size = p.shadow_map_size as usize;
    }
    for (entity, mut light) in &mut lights {
        if !settings.is_changed() && !light.is_added() {
            continue;
        }
        light.shadow_maps_enabled = p.directional_shadows;
        // Sans ombre, rien n'arrête le soleil : il traverserait les parois du vaisseau. Les
        // surfaces cuites gardent alors leur seul éclairage précalculé (les objets mobiles,
        // eux, le reçoivent encore : c'est pourquoi tous les préréglages gardent les ombres).
        light.affects_lightmapped_mesh_diffuse = p.directional_shadows;
        commands.entity(entity).insert(
            CascadeShadowConfigBuilder {
                num_cascades: usize::from(p.shadow_cascades),
                maximum_distance: p.shadow_max_distance,
                ..default()
            }
            .build(),
        );
    }
}

/// L'échelle de rendu du préréglage, bornée.
fn preset_scale(settings: &GraphicsSettings) -> f32 {
    settings.preset.render_scale.clamp(0.25, 1.0)
}

/// Une taille de fenêtre à une échelle donnée, en pixels (au moins 1 × 1).
fn scaled_size(full: UVec2, scale: f32) -> UVec2 {
    (full.as_vec2() * scale).round().as_uvec2().max(UVec2::ONE)
}

/// Rend la 3D à l'échelle voulue. Sous 1 (ou avec la résolution dynamique), la caméra dessine
/// dans une image à l'échelle du **préréglage**, affichée en plein écran sous l'interface. La
/// résolution dynamique ne réalloue rien : la caméra ne dessine qu'une partie de cette image
/// (`Camera::viewport`), et c'est cette partie qui s'affiche, étirée (`ImageNode::rect`). Les
/// textures intermédiaires de Bevy (HDR, profondeur) suivent la taille de l'image, pas celle du
/// viewport : elles ne bougent pas non plus. Seule la chaîne du bloom (Moyen, Haut), dimensionnée
/// au viewport, se refait à chaque palier, dans le cache de textures de Bevy.
fn update_render_scale(
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    dynamic: Res<DynamicResolution>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<(&mut Camera, &mut RenderTarget), With<WorldCamera>>,
    target: Option<ResMut<ScaledTarget>>,
    mut nodes: Query<(Entity, &mut ImageNode), With<ScaledViewImage>>,
) {
    let ceiling = preset_scale(&settings);
    let full = window.physical_size();

    if (ceiling >= 0.999 && !dynamic.enabled) || cameras.is_empty() {
        // Rendu direct dans la fenêtre.
        for (mut camera, mut render_target) in &mut cameras {
            if !matches!(*render_target, RenderTarget::Window(_)) {
                *render_target = RenderTarget::default();
            }
            if camera.viewport.is_some() {
                camera.viewport = None;
            }
        }
        if target.is_some() {
            commands.remove_resource::<ScaledTarget>();
        }
        for (entity, _) in &nodes {
            commands.entity(entity).despawn();
        }
        return;
    }

    let allocated = scaled_size(full, ceiling);
    let shown = scaled_size(full, dynamic.scale.min(ceiling)).min(allocated);
    // La partie de l'image rendue et affichée ; toute l'image au plafond.
    let part = (shown != allocated).then_some(shown);
    let rect = part.map(|size| Rect::new(0.0, 0.0, size.x as f32, size.y as f32));

    let image = match target {
        Some(t) if t.size == allocated => t.image.clone(),
        _ => {
            // Seulement au démarrage, au changement de préréglage ou de taille de fenêtre.
            let image = images.add(Image::new_target_texture(
                allocated.x,
                allocated.y,
                TextureFormat::Rgba8Unorm,
                Some(TextureFormat::Rgba8UnormSrgb),
            ));
            commands.insert_resource(ScaledTarget {
                image: image.clone(),
                size: allocated,
            });
            for (entity, _) in &nodes {
                commands.entity(entity).despawn();
            }
            commands.spawn((
                Name::new("Image de la vue (échelle de rendu)"),
                ScaledViewImage,
                ImageNode {
                    rect,
                    ..ImageNode::new(image.clone())
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                // Sous toute l'interface.
                GlobalZIndex(i32::MIN + 1),
            ));
            image
        }
    };
    // Écrire seulement ce qui change : un `Camera` ou un `ImageNode` touché est recalculé.
    for (_, mut node) in &mut nodes {
        if node.rect != rect {
            node.rect = rect;
        }
    }
    for (mut camera, mut render_target) in &mut cameras {
        let up_to_date = matches!(&*render_target, RenderTarget::Image(t) if t.handle == image);
        if !up_to_date {
            *render_target = RenderTarget::Image(image.clone().into());
        }
        if camera.viewport.as_ref().map(|v| v.physical_size) != part {
            camera.viewport = part.map(|size| Viewport {
                physical_position: UVec2::ZERO,
                physical_size: size,
                ..default()
            });
        }
    }
}

// --- Résolution dynamique (M11) --------------------------------------------------------------

/// Plancher de l'échelle de rendu dynamique : en dessous, l'image devient trop floue.
pub const DYNAMIC_FLOOR: f32 = 0.5;
/// Un palier de l'échelle.
pub const DYNAMIC_STEP: f32 = 0.05;
/// Budget d'une image sans limite d'images : la cible de 60 i/s (AGENTS.md §10).
const DEFAULT_BUDGET_MS: f32 = 1000.0 / 60.0;
/// Descente rapide : autant d'images en retard dans cette fenêtre (secondes)…
const FAST_LATE_FRAMES: usize = 3;
const FAST_WINDOW: f32 = 0.25;
/// … ou des retards isolés mais fréquents : autant dans cette fenêtre (≈ 3 % à 60 i/s).
const SLOW_LATE_FRAMES: usize = 4;
const SLOW_WINDOW: f32 = 2.0;
/// Remontée : marge du GPU au-dessus de ce seuil, pendant [`UP_SECONDS`] (allongé après un échec).
const UP_HEADROOM: f32 = 0.25;
const UP_SECONDS: f32 = 2.0;
/// Sans mesure de marge (synchronisation verticale sans limite) : un essai de remontée après ce
/// délai sans retard.
const PROBE_SECONDS: f32 = 4.0;
/// Délai de remontée le plus long, après des échecs répétés.
const UP_SECONDS_MAX: f32 = 32.0;
/// Une remontée suivie d'une descente dans ce délai a échoué : le délai suivant double. Une
/// remontée qui tient plus longtemps le ramène à sa base.
const FAILED_UP_WINDOW: f32 = 2.0;
const KEPT_UP_WINDOW: f32 = 8.0;
/// Après un palier, l'effet n'est pas encore visible (rendu en pipeline, images en vol) : les
/// mesures de ce délai sont ignorées.
const SETTLE_SECONDS: f32 = 0.3;

/// Ce que voit le contrôleur pour une image rendue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleInput {
    /// Temps écoulé depuis la mesure précédente (secondes).
    pub dt: f32,
    /// L'image a dépassé son budget à cause du rendu (pas de la mise à jour du jeu).
    pub late: bool,
    /// Marge du GPU sur le budget (0,3 : 30 % libres) ; `None` : pas de mesure possible.
    pub headroom: Option<f32>,
}

/// Le contrôleur de l'échelle de rendu dynamique, par paliers sous le plafond (l'échelle du
/// préréglage) : il descend vite quand les images sont en retard, remonte lentement quand le GPU
/// a de la marge, et allonge son délai de remontée après chaque remontée ratée (pas de
/// va-et-vient entre deux paliers).
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleController {
    /// Paliers sous le plafond (0 : au plafond).
    level: u32,
    clock: f32,
    since_change: f32,
    /// Instants des images en retard récentes (les [`SLOW_WINDOW`] dernières secondes).
    late: Vec<f32>,
    comfortable_for: f32,
    up_delay: f32,
    last_up: Option<f32>,
}

impl Default for ScaleController {
    fn default() -> Self {
        Self {
            level: 0,
            clock: 0.0,
            since_change: SETTLE_SECONDS,
            late: Vec::new(),
            comfortable_for: 0.0,
            up_delay: UP_SECONDS,
            last_up: None,
        }
    }
}

impl ScaleController {
    /// Paliers possibles sous un plafond donné.
    fn max_level(ceiling: f32) -> u32 {
        (((ceiling - DYNAMIC_FLOOR) / DYNAMIC_STEP) + 1e-3)
            .floor()
            .max(0.0) as u32
    }

    /// L'échelle courante sous ce plafond.
    pub fn scale(&self, ceiling: f32) -> f32 {
        let level = self.level.min(Self::max_level(ceiling));
        (ceiling - level as f32 * DYNAMIC_STEP).max(DYNAMIC_FLOOR.min(ceiling))
    }

    /// Une image de plus ; `true` si l'échelle change.
    pub fn update(&mut self, ceiling: f32, input: ScaleInput) -> bool {
        let dt = input.dt.max(0.0);
        self.clock += dt;
        self.since_change += dt;
        let max = Self::max_level(ceiling);
        self.level = self.level.min(max);
        if let Some(up) = self.last_up
            && self.clock - up > KEPT_UP_WINDOW
        {
            // La dernière remontée a tenu : le délai revient à sa base.
            self.up_delay = UP_SECONDS;
            self.last_up = None;
        }
        if self.since_change < SETTLE_SECONDS {
            return false;
        }
        let clock = self.clock;
        self.late.retain(|t| clock - *t <= SLOW_WINDOW);
        if input.late {
            self.late.push(clock);
            self.comfortable_for = 0.0;
            let recent = self
                .late
                .iter()
                .filter(|t| clock - **t <= FAST_WINDOW)
                .count();
            if recent >= FAST_LATE_FRAMES || self.late.len() >= SLOW_LATE_FRAMES {
                return self.step_down(max);
            }
            return false;
        }
        let comfortable = input.headroom.is_none_or(|h| h > UP_HEADROOM);
        if !comfortable {
            self.comfortable_for = 0.0;
            return false;
        }
        self.comfortable_for += dt;
        // Sans mesure de marge, la remontée est un essai : plus prudent.
        let needed = match input.headroom {
            Some(_) => self.up_delay,
            None => self.up_delay.max(PROBE_SECONDS),
        };
        if self.level > 0 && self.comfortable_for >= needed {
            self.level -= 1;
            self.changed();
            self.last_up = Some(self.clock);
            return true;
        }
        false
    }

    fn step_down(&mut self, max: u32) -> bool {
        if let Some(up) = self.last_up
            && self.clock - up <= FAILED_UP_WINDOW
        {
            // La remontée précédente n'a pas tenu : attendre plus longtemps la prochaine.
            self.up_delay = (self.up_delay * 2.0).min(UP_SECONDS_MAX);
            self.last_up = None;
        }
        if self.level >= max {
            self.late.clear();
            return false;
        }
        self.level += 1;
        self.changed();
        true
    }

    fn changed(&mut self) {
        self.since_change = 0.0;
        self.comfortable_for = 0.0;
        self.late.clear();
    }
}

/// Ce que voit le contrôleur d'une image rendue (mesures du limiteur, `frame_pace`).
/// - Retard : le **rendu** a dépassé le budget (une mise à jour du jeu trop longue ne compte pas :
///   baisser la résolution n'y changerait rien). Avec la synchronisation verticale, le rendu
///   attend l'écran et ne dit plus rien : on regarde alors l'image elle-même (> 1,5 × budget).
/// - Marge : avec une limite, le travail du GPU mesuré par le limiteur ; sans limite ni vsync,
///   l'intervalle entre deux images ; avec vsync sans limite, aucune (la remontée se fait par
///   essais).
pub fn scale_input(sample: &RenderSample, limit: FrameLimit, frame_ms: f32, dt: f32) -> ScaleInput {
    let budget = limit
        .period()
        .map_or(DEFAULT_BUDGET_MS, |p| p.as_secs_f32() * 1000.0);
    let late = if limit.vsync {
        frame_ms > 1.5 * budget
    } else {
        sample.render_ms as f32 > budget
    };
    let headroom = match (limit.fps, limit.vsync) {
        (Some(_), _) => (sample.gpu_ms > 0.0).then(|| 1.0 - sample.gpu_ms as f32 / budget),
        (None, false) => Some(1.0 - frame_ms / budget),
        (None, true) => None,
    };
    ScaleInput { dt, late, headroom }
}

/// La résolution dynamique : l'échelle de rendu effective de la 3D, sous celle du préréglage
/// quand le GPU n'a plus de marge. Sans elle, c'est l'échelle du préréglage.
#[derive(Resource, Debug)]
pub struct DynamicResolution {
    enabled: bool,
    scale: f32,
    ceiling: f32,
    controller: ScaleController,
    /// Dernière image rendue vue (une mesure ne se compte qu'une fois).
    last_frame: u64,
    /// Temps écoulé depuis la dernière mesure comptée.
    pending: f32,
}

impl Default for DynamicResolution {
    fn default() -> Self {
        Self {
            enabled: false,
            scale: 1.0,
            ceiling: 1.0,
            controller: ScaleController::default(),
            last_frame: 0,
            pending: 0.0,
        }
    }
}

impl DynamicResolution {
    /// L'échelle de rendu effective de la 3D (compteur d'images, trace par frame).
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

/// Fait suivre l'échelle de rendu aux mesures du rendu, si l'option est active (le joueur, ou
/// `SH_DYNRES=1` pour l'autopilote).
fn control_render_scale(
    time: Res<Time<Real>>,
    settings: Res<GraphicsSettings>,
    user: Option<Res<UserSettings>>,
    limit: Option<Res<FrameLimit>>,
    pace: Option<Res<PaceShared>>,
    mut dynamic: ResMut<DynamicResolution>,
) {
    let ceiling = preset_scale(&settings);
    let enabled = user.is_some_and(|u| u.dynamic_resolution) && ceiling > DYNAMIC_FLOOR;
    let d = &mut *dynamic;
    if !enabled || d.ceiling != ceiling || !d.enabled {
        // Coupée, ou nouveau plafond : on repart du plafond.
        d.controller = ScaleController::default();
        d.ceiling = ceiling;
        d.pending = 0.0;
    }
    d.enabled = enabled;
    if !enabled {
        d.scale = ceiling;
        return;
    }
    d.pending += time.delta_secs();
    let Some(sample) = pace.map(|p| p.sample()) else {
        d.scale = d.controller.scale(ceiling);
        return;
    };
    if sample.frame == d.last_frame {
        return;
    }
    d.last_frame = sample.frame;
    let limit = limit.as_deref().copied().unwrap_or_default();
    let input = scale_input(
        &sample,
        limit,
        time.delta_secs() * 1000.0,
        std::mem::take(&mut d.pending),
    );
    if d.controller.update(ceiling, input) {
        debug!(
            "résolution dynamique : {:.0} %",
            d.controller.scale(ceiling) * 100.0
        );
    }
    d.scale = d.controller.scale(ceiling);
}

/// Composition de l'interface sur la vue 3D.
///
/// La caméra du monde rend en HDR, la caméra d'interface non : elles n'ont donc pas la même
/// texture intermédiaire, et la seconde écraserait la première. Quand une caméra du monde
/// dessine dans la fenêtre, l'interface est rendue sur fond transparent puis **mélangée** par-dessus
/// (alpha prémultiplié) ; sinon elle efface et écrit la fenêtre elle-même.
fn sync_ui_clear(
    world: Query<&RenderTarget, With<WorldCamera>>,
    mut ui: Query<&mut Camera, With<UiCamera>>,
) {
    let world_draws_window = world.iter().any(|t| matches!(t, RenderTarget::Window(_)));
    for mut camera in &mut ui {
        let blending = matches!(
            camera.output_mode,
            CameraOutputMode::Write {
                blend_state: Some(_),
                ..
            }
        );
        if world_draws_window && !blending {
            camera.clear_color = ClearColorConfig::Custom(Color::NONE);
            camera.output_mode = CameraOutputMode::Write {
                blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            };
        } else if !world_draws_window && blending {
            camera.clear_color = ClearColorConfig::Default;
            camera.output_mode = CameraOutputMode::default();
        }
    }
}

/// Le clustering des lumières sur GPU (nouveau dans Bevy 0.20) n'éclaire rien sur la machine de
/// référence (Intel UHD 770, Vulkan) : les lumières ponctuelles restent sans effet. Le clustering
/// CPU fonctionne. `SH_GPU_CLUSTERING=1` réactive le GPU pour revérifier après une mise à jour.
fn cpu_light_clustering(mut settings: ResMut<GlobalClusterSettings>) {
    if std::env::var("SH_GPU_CLUSTERING").is_ok_and(|v| v == "1") {
        info!("clustering des lumières : GPU (SH_GPU_CLUSTERING=1)");
    } else {
        settings.gpu_clustering = None;
    }
}

/// Budget d'ombres des lumières ponctuelles (AGENTS.md §10) : seules les lumières candidates
/// visibles les plus proches de la caméra projettent des ombres, dans la limite du préréglage
/// ([`shadow_budget`]).
pub(crate) fn budget_point_shadows(
    settings: Res<GraphicsSettings>,
    camera: Query<&GlobalTransform, With<WorldCamera>>,
    mut lights: Query<
        (&GlobalTransform, &InheritedVisibility, &mut PointLight),
        With<crate::sector::gltf::ShadowCandidate>,
    >,
) {
    let Ok(eye) = camera.single() else {
        return;
    };
    let eye = eye.translation();
    let mut found: Vec<(ShadowLight, Mut<PointLight>)> = lights
        .iter_mut()
        .map(|(t, visibility, light)| {
            let candidate = ShadowLight {
                distance: t.translation().distance(eye),
                visible: visibility.get(),
                shadowed: light.shadow_maps_enabled,
            };
            (candidate, light)
        })
        .collect();
    let candidates: Vec<ShadowLight> = found.iter().map(|(c, _)| *c).collect();
    let wanted = shadow_budget(
        &candidates,
        usize::from(settings.preset.max_shadowed_point_lights),
    );
    for ((_, light), wanted) in found.iter_mut().zip(wanted) {
        if light.shadow_maps_enabled != wanted {
            light.shadow_maps_enabled = wanted;
        }
    }
}

/// Une lumière candidate à l'ombre, vue de la caméra.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShadowLight {
    /// Distance à la caméra, en mètres.
    pub distance: f32,
    /// Visible dans la hiérarchie (`InheritedVisibility`) : une cellule cachée derrière une
    /// porte fermée (`sector::streaming`) cache ses lumières, que le rendu n'extrait plus.
    pub visible: bool,
    /// Porte déjà une ombre.
    pub shadowed: bool,
}

/// Avantage d'une lumière qui porte déjà une ombre : sa distance compte pour 80 %. Sans lui,
/// deux lumières presque à égale distance s'échangent l'ombre au moindre pas, et chaque échange
/// recrée les six vues de la carte d'ombre d'une lumière ponctuelle (spécialisation et file de
/// rendu de tout ce qu'elle voit) : un à-coup à chaque fois.
const SHADOW_KEEP: f32 = 0.8;

/// Les lumières qui portent une ombre : les plus proches parmi les **visibles**, au plus
/// `allowed`. Une lumière cachée n'en porte pas et ne prend pas de place dans le budget (il va
/// aux lumières qu'on voit) ; une lumière déjà ombrée garde l'avantage ([`SHADOW_KEEP`]).
pub(crate) fn shadow_budget(lights: &[ShadowLight], allowed: usize) -> Vec<bool> {
    let mut ranked: Vec<(usize, f32)> = lights
        .iter()
        .enumerate()
        .filter(|(_, l)| l.visible)
        .map(|(i, l)| {
            let keep = if l.shadowed { SHADOW_KEEP } else { 1.0 };
            (i, l.distance * keep)
        })
        .collect();
    // À égalité, l'ordre des lumières décide : le même d'un frame à l'autre.
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut wanted = vec![false; lights.len()];
    for (i, _) in ranked.into_iter().take(allowed) {
        wanted[i] = true;
    }
    wanted
}

/// Le préréglage a changé (options) : chaque décor optionnel déjà instancié est montré ou caché
/// selon le nouveau niveau. Seuls les nœuds dont la visibilité change sont touchés.
pub(crate) fn apply_decor_detail(
    settings: Res<GraphicsSettings>,
    mut decor: Query<(&DecorDetail, &mut Visibility)>,
) {
    for (detail, mut visibility) in &mut decor {
        visibility.set_if_neq(decor_visibility(settings.preset.shows_detail(detail.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sh_data::GraphicsPresets;

    fn presets() -> GraphicsPresets {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/data")
            .join(sh_data::paths::GRAPHICS);
        sh_data::read_ron(&path).unwrap_or_else(|err| panic!("{err}"))
    }

    fn settings(presets: &GraphicsPresets, level: PresetLevel) -> GraphicsSettings {
        GraphicsSettings {
            level,
            preset: presets.get(level).clone(),
        }
    }

    #[test]
    fn niveaux_de_decor_par_prereglage() {
        let presets = presets();
        let shown = |level| {
            (0..=GraphicsPreset::MAX_DECOR_DETAIL)
                .filter(|d| presets.get(level).shows_detail(*d))
                .collect::<Vec<_>>()
        };
        assert_eq!(shown(PresetLevel::Low), [0]);
        assert_eq!(shown(PresetLevel::Medium), [0, 1]);
        assert_eq!(shown(PresetLevel::High), [0, 1, 2]);
    }

    /// Une image à 60 i/s : en retard ou non, marge du GPU.
    fn frame(late: bool, headroom: Option<f32>) -> ScaleInput {
        ScaleInput {
            dt: 1.0 / 60.0,
            late,
            headroom,
        }
    }

    /// Fait passer `seconds` secondes d'images identiques ; renvoie les changements d'échelle.
    fn run(c: &mut ScaleController, ceiling: f32, seconds: f32, input: ScaleInput) -> u32 {
        let frames = (seconds / input.dt).round() as u32;
        (0..frames).filter(|_| c.update(ceiling, input)).count() as u32
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn paliers_et_bornes() {
        let c = ScaleController::default();
        assert!(close(c.scale(0.8), 0.8));
        assert_eq!(ScaleController::max_level(0.8), 6);
        assert_eq!(ScaleController::max_level(0.75), 5);
        assert_eq!(ScaleController::max_level(1.0), 10);
        assert_eq!(ScaleController::max_level(0.5), 0);
        assert_eq!(ScaleController::max_level(0.4), 0);
        // Toujours en retard : on descend jusqu'au plancher, jamais plus bas.
        let mut c = ScaleController::default();
        run(&mut c, 0.8, 10.0, frame(true, Some(-0.2)));
        assert!(close(c.scale(0.8), DYNAMIC_FLOOR), "{}", c.scale(0.8));
        // Toujours à l'aise : on remonte jusqu'au plafond, jamais plus haut.
        run(&mut c, 0.8, 60.0, frame(false, Some(0.5)));
        assert!(close(c.scale(0.8), 0.8), "{}", c.scale(0.8));
        // Un plafond sous le plancher : pas de résolution dynamique.
        let mut c = ScaleController::default();
        run(&mut c, 0.45, 2.0, frame(true, Some(-0.2)));
        assert!(close(c.scale(0.45), 0.45));
    }

    /// Descente rapide : trois images en retard en 0,25 s font descendre d'un palier ; puis
    /// l'effet se laisse voir avant le palier suivant.
    #[test]
    fn descente_rapide() {
        let mut c = ScaleController::default();
        assert!(!c.update(0.8, frame(true, None)));
        assert!(!c.update(0.8, frame(true, None)));
        assert!(c.update(0.8, frame(true, None)));
        assert!(close(c.scale(0.8), 0.75));
        // Pendant l'attente de l'effet, les retards ne comptent pas.
        assert_eq!(run(&mut c, 0.8, 0.25, frame(true, None)), 0);
        // Retards continus : un palier à peu près toutes les 0,35 s.
        let steps = run(&mut c, 0.8, 1.0, frame(true, None));
        assert!((2..=3).contains(&steps), "{steps}");
    }

    /// Des retards isolés mais fréquents (4 en 2 s) font aussi descendre ; plus rares, non.
    #[test]
    fn retards_isoles() {
        let mut c = ScaleController::default();
        let mut changes = 0;
        // Un retard toutes les 30 images (0,5 s) : 4 en 2 s.
        for i in 0..150 {
            if c.update(0.8, frame(i % 30 == 0, Some(0.1))) {
                changes += 1;
            }
        }
        assert_eq!(changes, 1);
        // Un retard toutes les 3 s : rien ne bouge.
        let mut c = ScaleController::default();
        for i in 0..1800 {
            assert!(!c.update(0.8, frame(i % 180 == 0, Some(0.1))));
        }
        assert!(close(c.scale(0.8), 0.8));
    }

    /// Remontée lente : 2 s de marge au-dessus de 25 % ; une marge trop faible ne remonte pas.
    #[test]
    fn remontee_lente() {
        let mut c = ScaleController::default();
        run(&mut c, 0.8, 0.1, frame(true, None));
        assert!(close(c.scale(0.8), 0.75));
        assert_eq!(run(&mut c, 0.8, 30.0, frame(false, Some(0.2))), 0);
        assert_eq!(run(&mut c, 0.8, 1.9, frame(false, Some(0.3))), 0);
        assert_eq!(run(&mut c, 0.8, 0.2, frame(false, Some(0.3))), 1);
        assert!(close(c.scale(0.8), 0.8));
    }

    /// Une remontée aussitôt ratée double le délai de la suivante : pas de va-et-vient.
    #[test]
    fn pas_de_va_et_vient() {
        let mut c = ScaleController::default();
        run(&mut c, 0.8, 0.1, frame(true, None));
        // Attente de l'effet du palier (0,25 s restantes), puis 2 s de marge : on remonte.
        assert_eq!(run(&mut c, 0.8, 2.4, frame(false, Some(0.3))), 1);
        // Au palier du dessus, les retards reviennent : on redescend.
        assert_eq!(run(&mut c, 0.8, 0.3, frame(true, None)), 1);
        assert!(close(c.scale(0.8), 0.75));
        // La remontée suivante attend 4 s au lieu de 2 (plus la fin de l'attente de l'effet).
        assert_eq!(run(&mut c, 0.8, 3.8, frame(false, Some(0.3))), 0);
        assert_eq!(run(&mut c, 0.8, 0.6, frame(false, Some(0.3))), 1);
        assert!(close(c.scale(0.8), 0.8));
    }

    /// Sans mesure de marge (vsync sans limite), la remontée est un essai après 4 s sans retard.
    #[test]
    fn remontee_par_essai() {
        let mut c = ScaleController::default();
        run(&mut c, 0.8, 0.1, frame(true, None));
        assert_eq!(run(&mut c, 0.8, 3.8, frame(false, None)), 0);
        assert_eq!(run(&mut c, 0.8, 0.6, frame(false, None)), 1);
    }

    #[test]
    fn signal_du_controleur() {
        let sample = RenderSample {
            render_ms: 20.0,
            gpu_ms: 12.0,
            ..RenderSample::default()
        };
        // Limite à 60 sans vsync : rendu au-delà du budget = retard ; marge du GPU 28 %.
        let limited = scale_input(&sample, FrameLimit::new(60, false), 16.7, 0.016);
        assert!(limited.late);
        assert!(limited.headroom.is_some_and(|h| (h - 0.28).abs() < 0.01));
        // Mise à jour du jeu trop longue, rendu dans le budget : pas un retard du rendu.
        let cpu = RenderSample {
            render_ms: 6.0,
            ..sample
        };
        assert!(!scale_input(&cpu, FrameLimit::new(60, false), 40.0, 0.04).late);
        // Sans limite ni vsync : la marge vient de l'intervalle entre images.
        let free = scale_input(&cpu, FrameLimit::new(0, false), 10.0, 0.01);
        assert!(free.headroom.is_some_and(|h| (h - 0.4).abs() < 0.01));
        // Vsync sans limite : pas de marge ; retard seulement au-delà de 1,5 × le budget.
        let vsync = scale_input(&sample, FrameLimit::new(0, true), 16.7, 0.016);
        assert!(!vsync.late && vsync.headroom.is_none());
        assert!(scale_input(&sample, FrameLimit::new(0, true), 33.4, 0.033).late);
        // Pas encore de mesure du GPU : pas de marge.
        let unmeasured = RenderSample {
            gpu_ms: 0.0,
            ..sample
        };
        assert!(
            scale_input(&unmeasured, FrameLimit::new(60, false), 16.7, 0.016)
                .headroom
                .is_none()
        );
    }

    fn light(distance: f32, visible: bool, shadowed: bool) -> ShadowLight {
        ShadowLight {
            distance,
            visible,
            shadowed,
        }
    }

    /// Les plus proches des lumières visibles ; une cachée n'a pas d'ombre et ne compte pas.
    #[test]
    fn budget_des_ombres_aux_lumieres_visibles() {
        let lights = [
            light(2.0, false, true),
            light(4.0, true, false),
            light(9.0, true, false),
            light(6.0, true, false),
        ];
        assert_eq!(shadow_budget(&lights, 2), [false, true, false, true]);
        assert_eq!(shadow_budget(&lights, 0), [false; 4]);
        assert_eq!(shadow_budget(&lights, 10), [false, true, true, true]);
        assert!(shadow_budget(&[], 2).is_empty());
    }

    /// Une lumière ombrée garde son ombre face à une voisine à peine plus proche ; elle la
    /// cède à une nettement plus proche.
    #[test]
    fn budget_des_ombres_sans_va_et_vient() {
        let close_call = [light(5.0, true, true), light(4.6, true, false)];
        assert_eq!(shadow_budget(&close_call, 1), [true, false]);
        let clear_win = [light(5.0, true, true), light(3.0, true, false)];
        assert_eq!(shadow_budget(&clear_win, 1), [false, true]);
        // Égalité parfaite : toujours la même.
        let tie = [light(5.0, true, false), light(5.0, true, false)];
        assert_eq!(shadow_budget(&tie, 1), [true, false]);
    }

    #[test]
    fn le_decor_suit_le_prereglage_en_direct() {
        let presets = presets();
        let mut app = App::new();
        app.add_systems(
            Update,
            apply_decor_detail.run_if(resource_exists_and_changed::<GraphicsSettings>),
        );
        let medium = app
            .world_mut()
            .spawn((DecorDetail(1), Visibility::Inherited))
            .id();
        let high = app
            .world_mut()
            .spawn((DecorDetail(2), Visibility::Inherited))
            .id();
        let visibility = |app: &App, e: Entity| app.world().get::<Visibility>(e).copied();

        app.insert_resource(settings(&presets, PresetLevel::Low));
        app.update();
        assert_eq!(visibility(&app, medium), Some(Visibility::Hidden));
        assert_eq!(visibility(&app, high), Some(Visibility::Hidden));

        app.insert_resource(settings(&presets, PresetLevel::Medium));
        app.update();
        assert_eq!(visibility(&app, medium), Some(Visibility::Inherited));
        assert_eq!(visibility(&app, high), Some(Visibility::Hidden));

        app.insert_resource(settings(&presets, PresetLevel::High));
        app.update();
        assert_eq!(visibility(&app, high), Some(Visibility::Inherited));

        // Sans changement de réglage, le système ne repasse pas sur les nœuds.
        app.world_mut().entity_mut(high).insert(Visibility::Hidden);
        app.update();
        assert_eq!(visibility(&app, high), Some(Visibility::Hidden));
    }
}
