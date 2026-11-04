//! Bibliothèque de textures partagées des matériaux « futur usé » (M8, docs/refonte-visuelle.md §3).
//!
//! Les modèles du pipeline Blender sont des glTF séparés dont les images pointent, par chemin
//! relatif, vers `assets/textures/materials/`. Bevy ne charge une image qu'une fois par chemin :
//! dix cellules qui partagent un matériau partagent ses textures en mémoire vidéo.
//!
//! Ce module précharge toute la bibliothèque pendant l'écran de chargement, avant le premier
//! modèle, pour en fixer les réglages (le premier chargement d'un chemin impose les siens) :
//! - espace colorimétrique selon le suffixe du fichier (contrat avec les scripts Blender) :
//!   `_color` et `_emissive` en sRGB, `_normal` et `_orm` en linéaire ;
//! - mipmaps générées sur un fil de calcul (sans elles, les textures scintillent au loin et
//!   coûtent en bande passante, le point faible du GPU intégré de référence) ;
//! - échantillonnage en répétition (textures carrelables, trim sheets) et filtrage anisotrope du
//!   préréglage graphique de démarrage (un changement de préréglage s'applique au lancement
//!   suivant : changer l'échantillonneur réenverrait toutes les textures au GPU).
//!
//! Les images sont lues sans être envoyées au GPU, puis envoyées une seule fois, mipmaps
//! comprises ; leur copie en mémoire centrale est alors libérée.

use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};

use crate::data::{assets_dir, data_dir};
use crate::state::AppState;

/// Dossier de la bibliothèque, relatif à `assets/`.
pub const LIBRARY_DIR: &str = "textures/materials";

pub struct MaterialLibraryPlugin;

impl Plugin for MaterialLibraryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextureLibrary>()
            .add_systems(OnEnter(AppState::Loading), start_library)
            .add_systems(Update, build_mipmaps);
    }
}

/// Rôle d'une texture, déduit du suffixe de son nom de fichier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureKind {
    /// Couleur de base (sRGB).
    Color,
    /// Émission (sRGB).
    Emissive,
    /// Normale en espace tangent (linéaire, vecteurs renormalisés à chaque niveau).
    Normal,
    /// Occlusion, rugosité, métal empaquetés (linéaire).
    Orm,
}

impl TextureKind {
    /// Rôle d'après le nom (`panel_painted_color.png`…) ; `None` si le suffixe est inconnu.
    pub fn from_file_name(name: &str) -> Option<Self> {
        let stem = name.strip_suffix(".png")?;
        [
            ("_color", Self::Color),
            ("_emissive", Self::Emissive),
            ("_normal", Self::Normal),
            ("_orm", Self::Orm),
        ]
        .into_iter()
        .find(|(suffix, _)| stem.ends_with(suffix))
        .map(|(_, kind)| kind)
    }

    pub fn is_srgb(self) -> bool {
        matches!(self, Self::Color | Self::Emissive)
    }
}

enum Stage {
    /// Lecture du fichier en cours.
    Loading,
    /// Mipmaps en cours de calcul.
    Building(Task<Option<(Vec<u8>, u32)>>),
    Done,
}

struct LibraryTexture {
    handle: Handle<Image>,
    kind: TextureKind,
    stage: Stage,
}

/// Les textures de la bibliothèque (poignées gardées vivantes : les modèles retrouvent les
/// mêmes images par leur chemin).
#[derive(Resource, Default)]
pub struct TextureLibrary {
    textures: Vec<LibraryTexture>,
    anisotropy: u16,
}

impl TextureLibrary {
    /// Toutes les textures sont prêtes (mipmaps comprises) : le chargement peut finir.
    pub fn ready(&self) -> bool {
        self.textures.iter().all(|t| matches!(t.stage, Stage::Done))
    }
}

/// Filtrage anisotrope du préréglage de démarrage, lu directement dans les données : la
/// bibliothèque se charge en même temps qu'elles, avant que les réglages graphiques existent.
fn startup_anisotropy(user: Option<&crate::options::UserSettings>) -> u16 {
    let path = data_dir().join(sh_data::paths::GRAPHICS);
    match sh_data::read_ron::<sh_data::GraphicsPresets>(&path) {
        Ok(presets) => presets
            .get(crate::graphics::startup_level(user))
            .anisotropy
            .clamp(1, 16),
        Err(err) => {
            warn!("préréglages graphiques illisibles ({err}) : pas de filtrage anisotrope");
            1
        }
    }
}

fn start_library(
    mut library: ResMut<TextureLibrary>,
    asset_server: Res<AssetServer>,
    user: Option<Res<crate::options::UserSettings>>,
) {
    library.anisotropy = startup_anisotropy(user.as_deref());
    library.textures.clear();
    let dir = assets_dir().join(LIBRARY_DIR);
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".png"))
        .collect();
    // Ordre stable (journaux reproductibles).
    names.sort_unstable();
    for name in names {
        let kind = TextureKind::from_file_name(&name).unwrap_or_else(|| {
            warn!("texture de matériau sans suffixe connu, lue comme une couleur : {name}");
            TextureKind::Color
        });
        let path = Path::new(LIBRARY_DIR).join(&name);
        let handle = asset_server
            .load_builder()
            .with_settings(move |s: &mut ImageLoaderSettings| {
                s.is_srgb = kind.is_srgb();
                // Pas d'envoi au GPU avant les mipmaps : un seul envoi, complet.
                s.asset_usage = RenderAssetUsages::MAIN_WORLD;
            })
            .load(path);
        library.textures.push(LibraryTexture {
            handle,
            kind,
            stage: Stage::Loading,
        });
    }
    info!(
        "bibliothèque de matériaux : {} textures, anisotropie {}",
        library.textures.len(),
        library.anisotropy
    );
}

/// Échantillonneur des textures de la bibliothèque : répétition, filtrage trilinéaire et
/// anisotrope (wgpu exige des filtres linéaires dès que l'anisotropie dépasse 1).
pub fn library_sampler(anisotropy: u16) -> ImageSamplerDescriptor {
    ImageSamplerDescriptor {
        label: Some("bibliothèque de matériaux".into()),
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: anisotropy.max(1),
        ..default()
    }
}

fn build_mipmaps(
    mut library: ResMut<TextureLibrary>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let anisotropy = library.anisotropy;
    for texture in &mut library.textures {
        match &mut texture.stage {
            Stage::Loading => {
                if let Some(image) = images.get(&texture.handle) {
                    let size = image.texture_descriptor.size;
                    let format = image.texture_descriptor.format;
                    let (Some(data), true) = (image.data.clone(), is_rgba8(format)) else {
                        warn!(
                            "texture de matériau {:?} : format {format:?} sans mipmaps",
                            texture.handle.path()
                        );
                        finish(&mut images, &texture.handle, None, anisotropy);
                        texture.stage = Stage::Done;
                        continue;
                    };
                    let kind = texture.kind;
                    let task = AsyncComputeTaskPool::get()
                        .spawn(async move { mip_chain(&data, size.width, size.height, kind) });
                    texture.stage = Stage::Building(task);
                } else if asset_server.load_state(&texture.handle).is_failed() {
                    error!(
                        "texture de matériau illisible : {:?}",
                        texture.handle.path()
                    );
                    texture.stage = Stage::Done;
                }
            }
            Stage::Building(task) => {
                if let Some(result) = check_ready(task) {
                    finish(&mut images, &texture.handle, result, anisotropy);
                    texture.stage = Stage::Done;
                }
            }
            Stage::Done => {}
        }
    }
}

fn is_rgba8(format: TextureFormat) -> bool {
    matches!(
        format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
    )
}

/// Pose les mipmaps et l'échantillonneur, puis confie l'image au GPU seul (un envoi).
fn finish(
    images: &mut Assets<Image>,
    handle: &Handle<Image>,
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
    // L'anisotropie n'a de sens (et n'est permise) qu'avec des mipmaps filtrées.
    image.sampler =
        ImageSampler::Descriptor(library_sampler(if levels > 1 { anisotropy } else { 1 }));
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
}

/// Nombre de niveaux d'une chaîne complète de mipmaps jusqu'à 1 × 1.
pub fn mip_levels(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

/// Chaîne complète de mipmaps d'une image RGBA 8 bits (niveau 0 compris, niveaux à la suite,
/// rangées serrées : la disposition qu'attend wgpu). Filtre boîte 2 × 2, en lumière linéaire
/// pour les textures sRGB, vecteurs renormalisés pour les normales. `None` si les données ne
/// correspondent pas à la taille.
pub fn mip_chain(
    data: &[u8],
    width: u32,
    height: u32,
    kind: TextureKind,
) -> Option<(Vec<u8>, u32)> {
    let (w0, h0) = (width as usize, height as usize);
    if w0 == 0 || h0 == 0 || data.len() != w0 * h0 * 4 {
        return None;
    }
    let levels = mip_levels(width, height);
    let to_linear: Vec<f32> = (0..=255u8).map(srgb_to_linear).collect();
    let mut out = Vec::with_capacity(data.len() * 4 / 3 + 64);
    out.extend_from_slice(data);
    let (mut start, mut w, mut h) = (0usize, w0, h0);
    for _ in 1..levels {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = Vec::with_capacity(nw * nh * 4);
        {
            let src = &out[start..start + w * h * 4];
            let texel = |x: usize, y: usize| {
                let i = (y.min(h - 1) * w + x.min(w - 1)) * 4;
                [src[i], src[i + 1], src[i + 2], src[i + 3]]
            };
            for y in 0..nh {
                for x in 0..nw {
                    let quad = [
                        texel(2 * x, 2 * y),
                        texel(2 * x + 1, 2 * y),
                        texel(2 * x, 2 * y + 1),
                        texel(2 * x + 1, 2 * y + 1),
                    ];
                    next.extend_from_slice(&average(&quad, kind, &to_linear));
                }
            }
        }
        start = out.len();
        out.extend_from_slice(&next);
        (w, h) = (nw, nh);
    }
    Some((out, levels))
}

fn average(quad: &[[u8; 4]; 4], kind: TextureKind, to_linear: &[f32]) -> [u8; 4] {
    let alpha = quad
        .iter()
        .map(|t| u32::from(t[3]))
        .sum::<u32>()
        .div_ceil(4) as u8;
    match kind {
        TextureKind::Color | TextureKind::Emissive => {
            let mut rgb = [0.0f32; 3];
            for t in quad {
                for c in 0..3 {
                    rgb[c] += to_linear[usize::from(t[c])];
                }
            }
            let enc = |v: f32| linear_to_srgb(v / 4.0);
            [enc(rgb[0]), enc(rgb[1]), enc(rgb[2]), alpha]
        }
        TextureKind::Normal => {
            let mut n = [0.0f32; 3];
            for t in quad {
                for c in 0..3 {
                    n[c] += f32::from(t[c]) / 127.5 - 1.0;
                }
            }
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            // Normales opposées qui s'annulent : on retombe sur la normale de la surface.
            let n = if len > 1e-4 {
                [n[0] / len, n[1] / len, n[2] / len]
            } else {
                [0.0, 0.0, 1.0]
            };
            let enc = |v: f32| ((v + 1.0) * 127.5).round().clamp(0.0, 255.0) as u8;
            [enc(n[0]), enc(n[1]), enc(n[2]), alpha]
        }
        TextureKind::Orm => {
            let avg = |c: usize| {
                quad.iter()
                    .map(|t| u32::from(t[c]))
                    .sum::<u32>()
                    .div_ceil(4) as u8
            };
            [avg(0), avg(1), avg(2), alpha]
        }
    }
}

fn srgb_to_linear(v: u8) -> f32 {
    let c = f32::from(v) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let c = if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_suffixe_donne_le_role() {
        use TextureKind::*;
        assert_eq!(
            TextureKind::from_file_name("panel_painted_color.png"),
            Some(Color)
        );
        assert_eq!(
            TextureKind::from_file_name("screen_emissive.png"),
            Some(Emissive)
        );
        assert_eq!(
            TextureKind::from_file_name("grating_normal.png"),
            Some(Normal)
        );
        assert_eq!(TextureKind::from_file_name("trim_ship_orm.png"), Some(Orm));
        assert_eq!(TextureKind::from_file_name("notes.png"), None);
        assert!(Color.is_srgb() && Emissive.is_srgb());
        assert!(!Normal.is_srgb() && !Orm.is_srgb());
    }

    #[test]
    fn chaine_complete_jusqu_a_un_texel() {
        assert_eq!(mip_levels(1024, 1024), 11);
        assert_eq!(mip_levels(1024, 512), 11);
        assert_eq!(mip_levels(1, 1), 1);
        let data = vec![200u8; 8 * 4 * 4];
        let (out, levels) = mip_chain(&data, 8, 4, TextureKind::Orm).expect("chaîne");
        assert_eq!(levels, 4);
        // 8×4 + 4×2 + 2×1 + 1×1 texels.
        assert_eq!(out.len(), (32 + 8 + 2 + 1) * 4);
        assert!(out.iter().all(|&v| v == 200));
    }

    #[test]
    fn la_moyenne_srgb_se_fait_en_lumiere_lineaire() {
        // Noir et blanc en damier : la moyenne linéaire (0,5) vaut ≈ 188 en sRGB, pas 128.
        let data: Vec<u8> = [0u8, 255, 255, 0]
            .iter()
            .flat_map(|&v| [v, v, v, 255])
            .collect();
        let (out, _) = mip_chain(&data, 2, 2, TextureKind::Color).expect("chaîne");
        assert_eq!(&out[16..20], &[188, 188, 188, 255]);
        let (lin, _) = mip_chain(&data, 2, 2, TextureKind::Orm).expect("chaîne");
        assert_eq!(&lin[16..20], &[128, 128, 128, 255]);
    }

    #[test]
    fn les_normales_restent_unitaires() {
        // Deux normales inclinées de part et d'autre : la moyenne pointe droit, unitaire.
        let tilt = |x: f32| {
            let z = (1.0 - x * x).sqrt();
            let e = |v: f32| ((v + 1.0) * 127.5).round() as u8;
            [e(x), e(0.0), e(z), 255]
        };
        let data: Vec<u8> = [tilt(0.6), tilt(-0.6), tilt(0.6), tilt(-0.6)]
            .into_iter()
            .flatten()
            .collect();
        let (out, _) = mip_chain(&data, 2, 2, TextureKind::Normal).expect("chaîne");
        let n: Vec<f32> = out[16..19]
            .iter()
            .map(|&v| f32::from(v) / 127.5 - 1.0)
            .collect();
        assert!(n[0].abs() < 0.02 && (n[2] - 1.0).abs() < 0.02, "{n:?}");
    }

    #[test]
    fn des_donnees_tronquees_sont_refusees() {
        assert!(mip_chain(&[0; 10], 2, 2, TextureKind::Color).is_none());
    }
}
