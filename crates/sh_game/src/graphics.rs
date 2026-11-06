//! Préréglages graphiques (Bas / Moyen / Haut) et échelle de rendu (AGENTS.md §10).
//!
//! Les valeurs vivent dans `assets/data/gameplay/graphics.ron`. Ce module les applique :
//! - à chaque caméra du monde ([`WorldCamera`]) : HDR, bloom, anticrénelage, pas de MSAA ;
//! - à la lumière directionnelle : ombres, cascades, taille de la carte d'ombre ;
//! - à l'échelle de rendu : sous 1, la scène 3D est rendue dans une image plus petite, affichée
//!   en plein écran sous l'interface, qui reste à pleine résolution ;
//! - au décor par niveau (M9, docs/m9-retours.md §3.2) : un nœud glTF de décor optionnel
//!   ([`DecorDetail`], extra `detail`) est caché avec ses enfants si son niveau dépasse
//!   `decor_detail` ; un changement de préréglage s'applique en direct, sans recharger la cellule
//!   (le réglage à l'arrivée d'un modèle est fait par `sector::gltf`) ;
//! - au budget d'ombres des lumières ponctuelles (M11 : seulement les lumières visibles, et sans
//!   va-et-vient entre deux lumières presque à égale distance, `shadow_budget`).

use bevy::anti_alias::fxaa::Fxaa;
use bevy::anti_alias::smaa::Smaa;
use bevy::camera::visibility::VisibilitySystems;
use bevy::camera::{CameraOutputMode, Hdr, RenderTarget};
use bevy::light::cluster::GlobalClusterSettings;
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::{BlendState, TextureFormat};
use bevy::render::view::Msaa;
use bevy::window::PrimaryWindow;
use sh_data::{AntiAliasing, GraphicsPreset, PresetLevel, ShadowFilter};

use crate::data::Data;
use crate::sector::gltf::{DecorDetail, decor_visibility};
use crate::ui::UiCamera;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, cpu_light_clustering)
            .add_systems(
                PostUpdate,
                (
                    init_settings.run_if(resource_added::<Data>),
                    (
                        configure_cameras,
                        configure_directional_lights,
                        update_render_scale,
                        sync_ui_clear,
                        budget_point_shadows,
                    )
                        .chain()
                        .run_if(resource_exists::<GraphicsSettings>),
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

fn update_render_scale(
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<&mut RenderTarget, With<WorldCamera>>,
    target: Option<ResMut<ScaledTarget>>,
    nodes: Query<Entity, With<ScaledViewImage>>,
) {
    let scale = settings.preset.render_scale.clamp(0.25, 1.0);
    let full = window.physical_size();
    let wanted = (full.as_vec2() * scale).round().as_uvec2().max(UVec2::ONE);

    if scale >= 0.999 || cameras.is_empty() {
        // Rendu direct dans la fenêtre.
        for mut render_target in &mut cameras {
            if !matches!(*render_target, RenderTarget::Window(_)) {
                *render_target = RenderTarget::default();
            }
        }
        if target.is_some() {
            commands.remove_resource::<ScaledTarget>();
        }
        for entity in &nodes {
            commands.entity(entity).despawn();
        }
        return;
    }

    let image = match target {
        Some(t) if t.size == wanted => t.image.clone(),
        _ => {
            let image = images.add(Image::new_target_texture(
                wanted.x,
                wanted.y,
                TextureFormat::Rgba8Unorm,
                Some(TextureFormat::Rgba8UnormSrgb),
            ));
            commands.insert_resource(ScaledTarget {
                image: image.clone(),
                size: wanted,
            });
            for entity in &nodes {
                commands.entity(entity).despawn();
            }
            commands.spawn((
                Name::new("Image de la vue (échelle de rendu)"),
                ScaledViewImage,
                ImageNode::new(image.clone()),
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
    for mut render_target in &mut cameras {
        let up_to_date = matches!(&*render_target, RenderTarget::Image(t) if t.handle == image);
        if !up_to_date {
            *render_target = RenderTarget::Image(image.clone().into());
        }
    }
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
