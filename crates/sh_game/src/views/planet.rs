//! Apparence des astres de la vue Globale. Deux voies :
//!
//! - **apparence peinte** (M9, `assets/data/gameplay/planets.ron`) : textures produites hors du
//!   jeu (`tools/planets/`), chargées pendant l'écran de chargement, et matériau de planète
//!   ([`PlanetMaterial`], shader `assets/shaders/planet.wesl`) : couleur de jour modulée par un
//!   détail carrelable à deux échelles (net en orbite basse), lumières des villes côté nuit, voile
//!   de brume au limbe. Un seul matériau, une seule passe. Hexis, monde-usine très urbanisé (Q17) ;
//! - **texture procédurale** (proposition P18) pour tous les autres : équirectangulaire, calculée
//!   une fois, sans couture (le bruit est échantillonné en 3D sur la sphère), une palette par genre
//!   d'astre, la graine tirée de l'identifiant.
//!
//! La vue Globale (`views::global`) crée chaque astre avec un `StandardMaterial` dont la couleur
//! vient du cache [`Procedural`]. Pour un astre peint, le cache tient d'avance sa carte de couleur
//! peinte : c'est elle qui désigne l'astre à habiller, et [`dress_planets`] remplace alors son
//! matériau par le matériau de planète, avant la première image.

use bevy::asset::RenderAssetUsages;
use bevy::image::{
    Image, ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
    ImageSamplerDescriptor,
};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use sh_data::{PlanetLook, PlanetLooks, PresetLevel};

use crate::data::data_dir;
use crate::materials::{TextureKind, mip_chain};
use crate::ship::SurfaceBody;
use crate::state::{AppState, ViewLevel};
use crate::views::procedural::{Procedural, planet_key};
use crate::views::sky::fbm;

/// Le shader du matériau de planète, relatif à `assets/`.
pub const PLANET_SHADER: &str = "shaders/planet.wesl";

/// Netteté du mélange triplanaire du détail (exposant des poids) : assez haute pour qu'un pixel
/// ne lise le plus souvent qu'une projection, assez basse pour que les raccords restent doux.
const TRIPLANAR_SHARPNESS: f32 = 6.0;

pub struct PlanetLookPlugin;

impl Plugin for PlanetLookPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<PlanetMaterial>::default())
            .init_resource::<PaintedPlanets>()
            .add_systems(OnEnter(AppState::Loading), load_painted)
            .add_systems(
                Update,
                (
                    finish_painted,
                    dress_planets.run_if(in_state(ViewLevel::Global)),
                )
                    .chain(),
            );
    }
}

// --------------------------------------------------------------------------------------------
// Matériau de planète
// --------------------------------------------------------------------------------------------

/// Le matériau des astres peints : le `StandardMaterial` (couleur de jour, rugosité) étendu par
/// le shader de planète.
pub type PlanetMaterial = ExtendedMaterial<StandardMaterial, PlanetExtension>;

/// Ce que le shader de planète ajoute au `StandardMaterial` (liaisons à partir de 100, celles du
/// matériau de base restant en deçà).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PlanetExtension {
    #[uniform(100)]
    pub params: PlanetParams,
    /// Données de surface (R lumières, G ville, B industrie, A eau).
    #[texture(101)]
    #[sampler(102)]
    pub surface: Handle<Image>,
    /// Détail carrelable (R trame urbaine, G éclairage des rues, B industrie, A relief).
    #[texture(103)]
    #[sampler(104)]
    pub detail: Handle<Image>,
}

impl MaterialExtension for PlanetExtension {
    fn fragment_shader() -> ShaderRef {
        PLANET_SHADER.into()
    }
}

/// Réglages du shader, empaquetés en `vec4` (la disposition est commentée dans le shader).
#[derive(ShaderType, Debug, Clone, Default, PartialEq)]
pub struct PlanetParams {
    pub detail_scale: Vec4,
    pub detail_misc: Vec4,
    pub detail_strength: Vec4,
    pub night_color: Vec4,
    pub night_core: Vec4,
    pub terminator: Vec4,
    pub haze: Vec4,
    pub haze_shape: Vec4,
    pub surface_finish: Vec4,
}

impl PlanetParams {
    /// Réglages d'une apparence pour un astre de rayon `radius_m` (mètres de jeu) : la taille
    /// d'un carreau de détail devient un nombre de carreaux par rayon.
    pub fn new(look: &PlanetLook, radius_m: f64) -> Self {
        let radius_km = (radius_m / 1000.0) as f32;
        let sh_data::planets::DetailLayers { coarse, fine } = look.detail_layers;
        let s = look.detail_strength;
        let scaled = |c: [f32; 3], k: f32| Vec4::new(c[0] * k, c[1] * k, c[2] * k, 0.0);
        Self {
            detail_scale: Vec4::new(
                radius_km / coarse.tile_km.max(1e-3),
                radius_km / fine.tile_km.max(1e-3),
                coarse.weight,
                fine.weight,
            ),
            detail_misc: Vec4::new(coarse.blur, fine.blur, TRIPLANAR_SHARPNESS, 0.0),
            detail_strength: Vec4::new(s.urban, s.industry, s.rough, s.lights),
            night_color: scaled(look.night.color, look.night.intensity),
            night_core: scaled(look.night.core_color, look.night.intensity),
            terminator: Vec4::new(
                look.night.on[0],
                look.night.on[1],
                look.haze.lit[0],
                look.haze.lit[1],
            ),
            haze: Vec4::new(
                look.haze.color[0],
                look.haze.color[1],
                look.haze.color[2],
                look.haze.strength,
            ),
            haze_shape: Vec4::new(look.haze.power, look.haze.floor, 0.0, 0.0),
            surface_finish: Vec4::new(
                look.roughness,
                look.water_roughness,
                look.industry_roughness,
                look.industry_metallic,
            ),
        }
    }
}

// --------------------------------------------------------------------------------------------
// Textures peintes
// --------------------------------------------------------------------------------------------

/// Les astres peints : leur apparence, leurs textures et leur matériau (créé à la première vue
/// Globale, puis réutilisé).
#[derive(Resource, Default)]
pub struct PaintedPlanets {
    looks: Vec<Painted>,
    /// Filtrage anisotrope du préréglage de démarrage.
    anisotropy: u16,
    started: bool,
}

impl PaintedPlanets {
    /// Toutes les textures peintes sont prêtes (mipmaps comprises), ou en échec.
    pub fn ready(&self) -> bool {
        self.looks.iter().all(|l| {
            l.textures()
                .iter()
                .all(|t| matches!(t.stage, Stage::Done | Stage::Failed))
        })
    }
}

struct Painted {
    body: String,
    look: PlanetLook,
    color: PaintedTexture,
    surface: PaintedTexture,
    detail: PaintedTexture,
    material: Option<Handle<PlanetMaterial>>,
}

impl Painted {
    fn textures(&self) -> [&PaintedTexture; 3] {
        [&self.color, &self.surface, &self.detail]
    }

    fn failed(&self) -> bool {
        self.textures()
            .iter()
            .any(|t| matches!(t.stage, Stage::Failed))
    }

    fn material(&self, radius_m: f64) -> PlanetMaterial {
        ExtendedMaterial {
            base: StandardMaterial {
                base_color_texture: Some(self.color.handle.clone()),
                perceptual_roughness: self.look.roughness,
                ..default()
            },
            extension: PlanetExtension {
                params: PlanetParams::new(&self.look, radius_m),
                surface: self.surface.handle.clone(),
                detail: self.detail.handle.clone(),
            },
        }
    }
}

/// Répétition d'une texture peinte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wrap {
    /// Équirectangulaire : raccord en longitude, bord franc aux pôles.
    Equirect,
    /// Carrelable dans les deux sens.
    Tile,
}

enum Stage {
    Loading,
    /// Mipmaps en cours de calcul sur un fil de calcul.
    Building(Task<Option<(Vec<u8>, u32)>>),
    Done,
    Failed,
}

struct PaintedTexture {
    handle: Handle<Image>,
    kind: TextureKind,
    wrap: Wrap,
    stage: Stage,
}

impl PaintedTexture {
    /// Lecture sans envoi au GPU : les mipmaps sont calculées d'abord, puis tout part en un envoi
    /// (comme la bibliothèque de matériaux, `crate::materials`).
    fn load(asset_server: &AssetServer, path: &str, kind: TextureKind, wrap: Wrap) -> Self {
        let handle = asset_server
            .load_builder()
            .with_settings(move |s: &mut ImageLoaderSettings| {
                s.is_srgb = kind.is_srgb();
                s.asset_usage = RenderAssetUsages::MAIN_WORLD;
            })
            .load(path.to_owned());
        Self {
            handle,
            kind,
            wrap,
            stage: Stage::Loading,
        }
    }

    fn advance(&mut self, images: &mut Assets<Image>, asset_server: &AssetServer, anisotropy: u16) {
        match &mut self.stage {
            Stage::Loading => {
                if let Some(image) = images.get(&self.handle) {
                    let size = image.texture_descriptor.size;
                    let rgba8 = matches!(
                        image.texture_descriptor.format,
                        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
                    );
                    match (image.data.clone(), rgba8) {
                        (Some(data), true) => {
                            let kind = self.kind;
                            let task = AsyncComputeTaskPool::get().spawn(async move {
                                mip_chain(&data, size.width, size.height, kind)
                            });
                            self.stage = Stage::Building(task);
                        }
                        _ => {
                            warn!("texture de planète {:?} sans mipmaps", self.handle.path());
                            finish_image(images, &self.handle, self.wrap, None, anisotropy);
                            self.stage = Stage::Done;
                        }
                    }
                } else if asset_server.load_state(&self.handle).is_failed() {
                    error!("texture de planète illisible : {:?}", self.handle.path());
                    self.stage = Stage::Failed;
                }
            }
            Stage::Building(task) => {
                if let Some(mips) = check_ready(task) {
                    finish_image(images, &self.handle, self.wrap, mips, anisotropy);
                    self.stage = Stage::Done;
                }
            }
            Stage::Done | Stage::Failed => {}
        }
    }
}

/// Échantillonneur d'une texture peinte : filtrage trilinéaire et anisotrope (wgpu exige des
/// filtres linéaires dès que l'anisotropie dépasse 1). L'anisotropie compte ici : en orbite
/// basse, le sol se voit en lumière rasante jusqu'à l'horizon.
fn planet_sampler(wrap: Wrap, anisotropy: u16) -> ImageSamplerDescriptor {
    let v = match wrap {
        Wrap::Equirect => ImageAddressMode::ClampToEdge,
        Wrap::Tile => ImageAddressMode::Repeat,
    };
    ImageSamplerDescriptor {
        label: Some("texture de planète".into()),
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: v,
        address_mode_w: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: anisotropy.max(1),
        ..default()
    }
}

/// Pose les mipmaps et l'échantillonneur, puis confie l'image au GPU seul (un envoi).
fn finish_image(
    images: &mut Assets<Image>,
    handle: &Handle<Image>,
    wrap: Wrap,
    mips: Option<(Vec<u8>, u32)>,
    anisotropy: u16,
) {
    let Some(mut image) = images.get_mut(handle) else {
        return;
    };
    let levels = match mips {
        Some((data, levels)) => {
            image.data = Some(data);
            image.texture_descriptor.mip_level_count = levels;
            levels
        }
        None => 1,
    };
    image.sampler = ImageSampler::Descriptor(planet_sampler(
        wrap,
        if levels > 1 { anisotropy } else { 1 },
    ));
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
}

/// Filtrage anisotrope d'un préréglage, lu dans les données (les réglages graphiques n'existent
/// pas encore quand l'écran de chargement commence).
fn preset_anisotropy(level: PresetLevel) -> u16 {
    let path = data_dir().join(sh_data::paths::GRAPHICS);
    match sh_data::read_ron::<sh_data::GraphicsPresets>(&path) {
        Ok(presets) => presets.get(level).anisotropy.clamp(1, 16),
        Err(err) => {
            warn!("préréglages graphiques illisibles ({err}) : planètes sans filtrage anisotrope");
            1
        }
    }
}

/// Début de l'écran de chargement : lit les apparences et lance la lecture des textures, à la
/// résolution du préréglage de démarrage (un changement de préréglage vaut au lancement suivant,
/// comme pour la bibliothèque de matériaux). Le fichier est lu directement, sans attendre les
/// données validées sur leur fil : les textures se chargent en même temps, et sont prêtes avant
/// la première vue Globale, même lancée tout de suite (`SH_START=global:hexis`).
fn load_painted(
    mut painted: ResMut<PaintedPlanets>,
    mut cache: ResMut<Procedural>,
    asset_server: Res<AssetServer>,
    user: Option<Res<crate::options::UserSettings>>,
) {
    if painted.started {
        return;
    }
    painted.started = true;
    let path = data_dir().join(sh_data::paths::PLANETS);
    let looks = match sh_data::read_ron::<PlanetLooks>(&path) {
        Ok(looks) => looks,
        Err(err) => {
            // Les astres gardent leur texture procédurale ; la validation des données le signale.
            error!("apparences des astres illisibles : {err}");
            return;
        }
    };
    let level = crate::graphics::startup_level(user.as_deref());
    painted.anisotropy = preset_anisotropy(level);
    for (body, look) in looks.bodies {
        let color = PaintedTexture::load(
            &asset_server,
            look.color.get(level),
            TextureKind::Color,
            Wrap::Equirect,
        );
        // Données et détail : moyenne linéaire canal par canal (comme les cartes ORM).
        let surface = PaintedTexture::load(
            &asset_server,
            &look.surface,
            TextureKind::Orm,
            Wrap::Equirect,
        );
        let detail =
            PaintedTexture::load(&asset_server, &look.detail, TextureKind::Orm, Wrap::Tile);
        // La vue Globale prendra cette couleur au lieu de calculer la texture procédurale.
        cache.provide(planet_key(&body), color.handle.clone());
        info!(
            "astre peint : {body} ({}, préréglage {level:?})",
            look.color.get(level)
        );
        painted.looks.push(Painted {
            body,
            look,
            color,
            surface,
            detail,
            material: None,
        });
    }
}

/// Mipmaps des textures peintes, au fil de leur lecture.
fn finish_painted(
    mut painted: ResMut<PaintedPlanets>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<Procedural>,
) {
    if painted.ready() {
        return;
    }
    let anisotropy = painted.anisotropy;
    for look in &mut painted.looks {
        let was_failed = look.failed();
        for texture in [&mut look.color, &mut look.surface, &mut look.detail] {
            texture.advance(&mut images, &asset_server, anisotropy);
        }
        if !was_failed && look.failed() {
            // Sans ses textures, l'astre reprend sa texture procédurale à la vue Globale suivante.
            error!("{} : apparence peinte abandonnée", look.body);
            cache.forget(&planet_key(&look.body));
        }
    }
}

/// Remplace le matériau d'un astre peint, que la vue Globale vient de créer, par le matériau de
/// planète. L'astre est reconnu à sa carte de couleur (celle que le cache a fournie).
fn dress_planets(
    mut commands: Commands,
    mut painted: ResMut<PaintedPlanets>,
    standard: Res<Assets<StandardMaterial>>,
    mut planet_materials: ResMut<Assets<PlanetMaterial>>,
    bodies: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>, &SurfaceBody),
        Added<MeshMaterial3d<StandardMaterial>>,
    >,
) {
    for (entity, material, body) in &bodies {
        let Some(texture) = standard
            .get(&material.0)
            .and_then(|m| m.base_color_texture.as_ref())
        else {
            continue;
        };
        let Some(look) = painted
            .looks
            .iter_mut()
            .find(|l| !l.failed() && l.color.handle.id() == texture.id())
        else {
            continue;
        };
        let handle = match &look.material {
            Some(handle) => handle.clone(),
            None => {
                let handle = planet_materials.add(look.material(body.radius));
                look.material = Some(handle.clone());
                handle
            }
        };
        debug!("{} : matériau de planète", look.body);
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
    }
}

// --------------------------------------------------------------------------------------------
// Textures procédurales (P18)
// --------------------------------------------------------------------------------------------

/// Apparence générale d'un astre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// Monde industriel ravagé : plaines grises et rouillées, mers sombres, cicatrices (Hexis,
    /// faute de son apparence peinte).
    Industrial,
    /// Monde rocheux générique.
    Rocky,
    /// Géante gazeuse : bandes.
    GasGiant,
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t.clamp(0.0, 1.0))
}

/// Couleur linéaire d'un point de la surface (direction unitaire `d`).
fn surface(look: Look, d: [f64; 3], seed: f64) -> [f32; 3] {
    let n = |s: f64| fbm(d[0] * s + seed, d[1] * s - seed, d[2] * s + 0.5 * seed) as f32;
    match look {
        Look::Industrial => {
            let land = n(3.5);
            let detail = n(16.0);
            let fine = n(48.0);
            let sea = [0.018, 0.022, 0.026];
            let plain = mix([0.10, 0.095, 0.09], [0.22, 0.14, 0.09], detail * 1.6 - 0.4);
            let plain = mix(plain, [0.06, 0.06, 0.06], (fine - 0.55) * 3.0);
            let scar = [0.36, 0.34, 0.31];
            let base = if land < 0.44 {
                mix(sea, plain, (land - 0.40) / 0.04)
            } else {
                plain
            };
            // Cicatrices industrielles : filaments clairs là où le bruit fin culmine.
            mix(base, scar, ((detail - 0.64) * 5.0) * f32::from(land > 0.44))
        }
        Look::Rocky => {
            let h = n(3.0);
            mix([0.20, 0.19, 0.18], [0.45, 0.40, 0.34], h * 1.6 - 0.3)
        }
        Look::GasGiant => {
            let band = ((d[1] * 9.0 + 1.5 * f64::from(n(1.5))).sin() * 0.5 + 0.5) as f32;
            mix([0.55, 0.42, 0.30], [0.85, 0.75, 0.60], band)
        }
    }
}

fn to_srgb8(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5) as u8
}

/// Texture équirectangulaire (largeur = 2 × hauteur), alignée sur les UV de `Sphere::mesh().uv`.
pub fn planet_texture(look: Look, seed: u64, height: usize) -> Image {
    let width = 2 * height;
    let seed = (seed % 10_000) as f64 * 0.137;
    let mut data = Vec::with_capacity(width * height * 4);
    for j in 0..height {
        // Latitude du pôle nord (+Y) au pôle sud.
        let lat =
            std::f64::consts::FRAC_PI_2 - std::f64::consts::PI * (j as f64 + 0.5) / height as f64;
        for i in 0..width {
            let lon = std::f64::consts::TAU * (i as f64 + 0.5) / width as f64;
            let d = [lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()];
            data.extend(surface(look, d, seed).map(to_srgb8));
            data.push(255);
        }
    }
    Image::new(
        Extent3d {
            width: u32::try_from(width).unwrap_or(2),
            height: u32::try_from(height).unwrap_or(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_deterministe_et_bien_dimensionnee() {
        let a = planet_texture(Look::Industrial, 42, 32);
        let b = planet_texture(Look::Industrial, 42, 32);
        assert_eq!(a.width(), 64);
        assert_eq!(a.height(), 32);
        assert_eq!(a.data, b.data);
    }

    #[test]
    fn reglages_du_shader_d_hexis() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/data")
            .join(sh_data::paths::PLANETS);
        let looks: PlanetLooks = sh_data::read_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        let look = looks
            .get("hexis")
            .unwrap_or_else(|| panic!("Hexis sans apparence"));
        let params = PlanetParams::new(look, 6.4e5);
        // 640 km de rayon : 4 carreaux de 160 km par rayon, 25,6 de 25 km.
        let layers = look.detail_layers;
        assert!((params.detail_scale.x - 640.0 / layers.coarse.tile_km).abs() < 1e-3);
        assert!((params.detail_scale.y - 640.0 / layers.fine.tile_km).abs() < 1e-3);
        assert!(
            params.detail_scale.y > params.detail_scale.x,
            "la fine après la grande"
        );
        // Les lumières sont prémultipliées par leur intensité.
        assert!((params.night_color.x - look.night.color[0] * look.night.intensity).abs() < 1e-6);
        assert!(params.terminator.x < params.terminator.y);
    }
}
