//! Textures procédurales des astres de la vue Globale (proposition P18) : équirectangulaires,
//! calculées une fois au chargement, sans couture (le bruit est échantillonné en 3D sur la
//! sphère). Une palette par genre d'astre ; la graine vient de l'identifiant de l'astre.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::views::sky::fbm;

/// Apparence générale d'un astre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// Monde industriel ravagé (Hexis) : plaines grises et rouillées, mers sombres, cicatrices.
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
}
