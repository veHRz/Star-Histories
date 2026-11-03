//! Ciel étoilé d'un système (AGENTS.md §6.4) : une cubemap générée depuis la position réelle du
//! système dans la galaxie (proposition P17 pour le dessin des bras et des nébuleuses).
//!
//! Repère : plan galactique = plan XZ du jeu (X = est de la carte, Z = sud), Y = pôle nord
//! galactique ; les longueurs sont en unités de la carte canon (1000 × 660).
//! - **Lueur diffuse** (bande galactique, bulbe, bras) : densité de lumière intégrée le long de
//!   chaque ligne de visée, calculée à basse résolution puis interpolée (elle est lisse) ;
//! - **étoiles** : tirées selon la même densité, éclat en 1/d² ;
//! - **nébuleuses** canon publiques qui ont un rayon (le Croissant, vu d'Hexis) : arcs diffus.
//!
//! Le canon dit seulement « galaxie spirale à deux bras, vue de dessus, dessinée comme une
//! ellipse » et « noyau : trou noir ordinaire » : la position exacte des bras est inventée.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};
use sh_core::SimRng;

/// Ce qu'il faut savoir de la galaxie pour dessiner le ciel d'un point.
#[derive(Clone, Debug)]
pub struct SkySpec {
    /// Position de l'observateur sur la carte.
    pub observer: [f64; 2],
    pub galaxy_center: [f64; 2],
    pub galaxy_radii: [f64; 2],
    pub nebulae: Vec<SkyNebula>,
    pub seed: u64,
}

#[derive(Clone, Debug)]
pub struct SkyNebula {
    pub center: [f64; 2],
    pub radius: f64,
}

/// Six faces carrées de radiance linéaire, dans l'ordre des cubemaps (+X, −X, +Y, −Y, +Z, −Z).
pub struct CubeFaces {
    pub size: usize,
    pub faces: [Vec<[f32; 3]>; 6],
}

const STARS: usize = 40_000;
const GLOW_SIZE: usize = 96;
/// Pas de 2 unités jusqu'à 900 : toute la galaxie depuis n'importe où.
const MARCH_STEP: f64 = 2.0;
const MARCH_MAX: f64 = 900.0;
/// Inclinaison des bras spiraux et largeur angulaire d'un bras (radians).
const ARM_PITCH_DEG: f64 = 15.0;
const ARM_WIDTH: f64 = 0.45;
const ARM_PHASE: f64 = 0.6;
/// Demi-épaisseur du disque, en unités de carte.
const DISK_THICKNESS: f64 = 3.0;

/// Direction (normalisée) du texel `(i, j)` de la face `face` (convention des cubemaps).
fn texel_dir(face: usize, i: usize, j: usize, size: usize) -> [f64; 3] {
    let u = 2.0 * (i as f64 + 0.5) / size as f64 - 1.0;
    let v = 2.0 * (j as f64 + 0.5) / size as f64 - 1.0;
    let d = match face {
        0 => [1.0, -v, -u],
        1 => [-1.0, -v, u],
        2 => [u, 1.0, v],
        3 => [u, -1.0, -v],
        4 => [u, -v, 1.0],
        _ => [-u, -v, -1.0],
    };
    normalize(d)
}

/// Face et coordonnées `[0, 1)` d'une direction (inverse de [`texel_dir`]).
fn dir_to_face(d: [f64; 3]) -> (usize, f64, f64) {
    let [x, y, z] = d;
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    let (face, u, v) = if ax >= ay && ax >= az {
        if x > 0.0 {
            (0, -z / ax, -y / ax)
        } else {
            (1, z / ax, -y / ax)
        }
    } else if ay >= az {
        if y > 0.0 {
            (2, x / ay, z / ay)
        } else {
            (3, x / ay, -z / ay)
        }
    } else if z > 0.0 {
        (4, x / az, -y / az)
    } else {
        (5, -x / az, -y / az)
    };
    (face, (u + 1.0) / 2.0, (v + 1.0) / 2.0)
}

/// Le shader de skybox de Bevy lit la cubemap avec `direction × (1, 1, −1)` (convention main
/// gauche des cubemaps) : passage d'une direction du monde à une direction de la cubemap, et
/// retour (l'opération est sa propre inverse).
fn world_cube(d: [f64; 3]) -> [f64; 3] {
    [d[0], d[1], -d[2]]
}

fn normalize(d: [f64; 3]) -> [f64; 3] {
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-12);
    [d[0] / l, d[1] / l, d[2] / l]
}

/// Lumière émise par unité de volume en un point de la galaxie (x, z sur la carte, h en hauteur),
/// séparée en composantes (bulbe chaud, bras bleutés, disque neutre).
fn emission(spec: &SkySpec, x: f64, z: f64, h: f64) -> [f64; 3] {
    let dx = (x - spec.galaxy_center[0]) / spec.galaxy_radii[0];
    let dz = (z - spec.galaxy_center[1]) / spec.galaxy_radii[1];
    let rho = (dx * dx + dz * dz).sqrt();
    if rho > 1.25 {
        return [0.0; 3];
    }
    let edge = ((1.25 - rho) / 0.3).clamp(0.0, 1.0);
    // Bulbe : ellipsoïde aplati (hauteur ≈ 0,6 fois son rayon dans le plan).
    let hn = h / spec.galaxy_radii[0] / 0.6;
    let bulge = 6.0 * (-(rho * rho + hn * hn) / (0.11 * 0.11)).exp();
    let disk = (-rho / 0.38).exp() * edge;
    // Deux bras logarithmiques : phase ψ = φ − ln ρ / tan(inclinaison).
    let phi = dz.atan2(dx);
    let psi = phi - (rho + 0.04).ln() / ARM_PITCH_DEG.to_radians().tan() - ARM_PHASE;
    let wrapped = (psi.rem_euclid(std::f64::consts::PI) - std::f64::consts::FRAC_PI_2).abs();
    let arm_shape =
        (-(std::f64::consts::FRAC_PI_2 - wrapped).powi(2) / (2.0 * ARM_WIDTH * ARM_WIDTH)).exp();
    let arms = 1.6 * arm_shape * disk * ((rho - 0.12) / 0.1).clamp(0.0, 1.0);
    // Disque mince à profil gaussien : pas de voile lumineux vers les pôles.
    let vertical = (-(h / DISK_THICKNESS).powi(2)).exp();
    [bulge, arms * vertical, 0.5 * disk * vertical]
}

/// Bruit de valeur 3D déterministe (hachage entier, interpolation lissée), dans `[0, 1)`.
fn value_noise(x: f64, y: f64, z: f64) -> f64 {
    fn hash(i: i64, j: i64, k: i64) -> f64 {
        let mut h = (i.wrapping_mul(73_856_093)
            ^ j.wrapping_mul(19_349_663)
            ^ k.wrapping_mul(83_492_791)) as u64;
        h ^= h >> 13;
        h = h.wrapping_mul(0x5bd1_e995);
        h ^= h >> 15;
        (h & 0xffff) as f64 / 65_536.0
    }
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let smooth = |t: f64| t * t * (3.0 - 2.0 * t);
    let (fx, fy, fz) = (smooth(x - xi), smooth(y - yi), smooth(z - zi));
    let (i, j, k) = (xi as i64, yi as i64, zi as i64);
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let mut layer = [0.0; 2];
    for (dk, out) in layer.iter_mut().enumerate() {
        let k = k + dk as i64;
        let a = lerp(hash(i, j, k), hash(i + 1, j, k), fx);
        let b = lerp(hash(i, j + 1, k), hash(i + 1, j + 1, k), fx);
        *out = lerp(a, b, fy);
    }
    lerp(layer[0], layer[1], fz)
}

/// Trois octaves de bruit : la texture filamenteuse des nébuleuses.
pub(crate) fn fbm(x: f64, y: f64, z: f64) -> f64 {
    (0..3)
        .map(|o| {
            let f = f64::from(1 << o);
            value_noise(x * f, y * f, z * f) / f
        })
        .sum::<f64>()
        / 1.75
}

/// Lueur d'une nébuleuse en arc : un anneau partiel, ouvert vers l'est, mince en hauteur.
fn nebula_emission(n: &SkyNebula, x: f64, z: f64, h: f64) -> f64 {
    let (dx, dz) = (x - n.center[0], z - n.center[1]);
    let r = (dx * dx + dz * dz).sqrt() / n.radius;
    if r > 1.4 {
        return 0.0;
    }
    let ring = (-((r - 0.75) / 0.22).powi(2)).exp();
    // Ouverture de l'arc : l'émission s'éteint côté est (angle 0).
    let angle = dz.atan2(dx).abs();
    let arc = ((angle - 0.9) / 0.8).clamp(0.0, 1.0);
    let vertical = (-(h / (0.22 * n.radius)).powi(2)).exp();
    // Filaments : le bruit creuse et renforce la nappe (échelle ≈ un quart du rayon).
    let k = 4.0 / n.radius;
    let texture = (fbm(x * k, h * k, z * k) - 0.35).max(0.0) * 2.2;
    ring * arc * vertical * texture * texture
}

/// Radiance diffuse dans une direction, intégrée le long de la ligne de visée.
fn glow(spec: &SkySpec, d: [f64; 3]) -> [f32; 3] {
    let [ox, oz] = spec.observer;
    let mut sum = [0.0_f64; 3];
    let mut nebula = 0.0;
    let mut t = 0.5 * MARCH_STEP;
    while t < MARCH_MAX {
        let (x, h, z) = (ox + d[0] * t, d[1] * t, oz + d[2] * t);
        // Au-delà de 60 unités de hauteur, plus rien à intégrer.
        if h.abs() > 60.0 {
            break;
        }
        let e = emission(spec, x, z, h);
        for (acc, v) in sum.iter_mut().zip(e) {
            *acc += v * MARCH_STEP;
        }
        for n in &spec.nebulae {
            nebula += nebula_emission(n, x, z, h) * MARCH_STEP;
        }
        t += MARCH_STEP;
    }
    // Couleurs : bulbe chaud, bras bleutés, disque neutre ; nébuleuse rouge et violette (P17).
    let [b, a, s] = sum.map(|v| v * 0.004);
    let n = nebula * 0.05;
    [
        (1.00 * b + 0.70 * a + 0.85 * s + 1.00 * n) as f32,
        (0.82 * b + 0.78 * a + 0.82 * s + 0.25 * n) as f32,
        (0.60 * b + 1.00 * a + 0.80 * s + 0.60 * n) as f32,
    ]
}

fn sample_bilinear(face: &[[f32; 3]], size: usize, u: f64, v: f64) -> [f32; 3] {
    let x = (u * size as f64 - 0.5).clamp(0.0, size as f64 - 1.0);
    let y = (v * size as f64 - 0.5).clamp(0.0, size as f64 - 1.0);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(size - 1), (y0 + 1).min(size - 1));
    let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
    let at = |i: usize, j: usize| face[j * size + i];
    std::array::from_fn(|k| {
        let top = at(x0, y0)[k] * (1.0 - fx) + at(x1, y0)[k] * fx;
        let bottom = at(x0, y1)[k] * (1.0 - fx) + at(x1, y1)[k] * fx;
        top * (1.0 - fy) + bottom * fy
    })
}

/// Couleur d'une étoile selon un tirage de « température » (de l'orangé au bleu-blanc).
fn star_color(t: f64) -> [f32; 3] {
    if t < 0.35 {
        [1.0, 0.78, 0.55]
    } else if t < 0.75 {
        [1.0, 0.95, 0.86]
    } else {
        [0.78, 0.86, 1.0]
    }
}

/// Génère le ciel. `size` : côté d'une face en texels.
pub fn render_sky(spec: &SkySpec, size: usize) -> CubeFaces {
    // Lueur diffuse à basse résolution.
    let low: Vec<Vec<[f32; 3]>> = (0..6)
        .map(|f| {
            let mut face = Vec::with_capacity(GLOW_SIZE * GLOW_SIZE);
            for j in 0..GLOW_SIZE {
                for i in 0..GLOW_SIZE {
                    face.push(glow(spec, world_cube(texel_dir(f, i, j, GLOW_SIZE))));
                }
            }
            face
        })
        .collect();
    let mut faces: [Vec<[f32; 3]>; 6] = std::array::from_fn(|f| {
        let mut face = Vec::with_capacity(size * size);
        for j in 0..size {
            for i in 0..size {
                let u = (i as f64 + 0.5) / size as f64;
                let v = (j as f64 + 0.5) / size as f64;
                face.push(sample_bilinear(&low[f], GLOW_SIZE, u, v));
            }
        }
        face
    });

    // Étoiles : tirage par rejet selon la densité du disque et du bulbe.
    let mut rng = SimRng::new(spec.seed).fork_named("ciel");
    let [cx, cz] = spec.galaxy_center;
    let [rx, rz] = spec.galaxy_radii;
    let mut placed = 0;
    let mut attempts = 0;
    while placed < STARS && attempts < STARS * 60 {
        attempts += 1;
        let x = cx + (rng.unit_f64() * 2.0 - 1.0) * rx * 1.2;
        let z = cz + (rng.unit_f64() * 2.0 - 1.0) * rz * 1.2;
        // Hauteur : loi de Laplace d'échelle 4 unités.
        let s = rng.unit_f64();
        let h = if s < 0.5 {
            4.0 * (2.0 * s).max(1e-9).ln()
        } else {
            -4.0 * (2.0 * (1.0 - s)).max(1e-9).ln()
        };
        let e = emission(spec, x, z, 0.0);
        let density = (e[0] * 0.25 + e[1] + e[2]) / 2.0;
        if rng.unit_f64() > density.min(1.0) {
            continue;
        }
        placed += 1;
        let d = [x - spec.observer[0], h, z - spec.observer[1]];
        let dist2 = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).max(4.0);
        // Luminosités en loi de puissance : beaucoup d'étoiles faibles, quelques-unes très vives.
        let luminosity = (1.0 - rng.unit_f64()).max(1e-4).powf(-1.4);
        let flux = (luminosity * 140.0 / dist2).min(6.0) as f32;
        let color = star_color(rng.unit_f64());
        splat(&mut faces, size, normalize(d), color.map(|c| c * flux));
    }
    // Quelques étoiles proches, réparties sur tout le ciel (voisinage immédiat).
    for _ in 0..600 {
        let d = normalize([
            rng.unit_f64() - 0.5,
            rng.unit_f64() - 0.5,
            rng.unit_f64() - 0.5,
        ]);
        let flux = (0.15 + 1.6 * rng.unit_f64().powi(6)) as f32;
        let color = star_color(rng.unit_f64());
        splat(&mut faces, size, d, color.map(|c| c * flux));
    }
    CubeFaces { size, faces }
}

/// Dépose une étoile ponctuelle sur ses 2 × 2 texels voisins (sans aliasing de position).
fn splat(faces: &mut [Vec<[f32; 3]>; 6], size: usize, d: [f64; 3], rgb: [f32; 3]) {
    let (f, u, v) = dir_to_face(world_cube(d));
    let x = u * size as f64 - 0.5;
    let y = v * size as f64 - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
    for (dx, dy, w) in [
        (0, 0, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let (i, j) = (x0 as i64 + dx, y0 as i64 + dy);
        if i < 0 || j < 0 || i >= size as i64 || j >= size as i64 {
            continue;
        }
        let texel = &mut faces[f][j as usize * size + i as usize];
        for (t, c) in texel.iter_mut().zip(rgb) {
            *t += c * w;
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

/// La cubemap en image Bevy (sRGB 8 bits, six couches, vue `Cube`).
pub fn cube_image(sky: &CubeFaces) -> Image {
    let mut data = Vec::with_capacity(6 * sky.size * sky.size * 4);
    for face in &sky.faces {
        for texel in face {
            data.extend(texel.map(to_srgb8));
            data.push(255);
        }
    }
    let size = u32::try_from(sky.size).unwrap_or(1);
    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..Default::default()
    });
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hexis() -> SkySpec {
        SkySpec {
            observer: [639.5, 330.0],
            galaxy_center: [500.0, 330.0],
            galaxy_radii: [310.0, 248.0],
            nebulae: vec![SkyNebula {
                center: [645.0, 272.0],
                radius: 24.0,
            }],
            seed: 7,
        }
    }

    #[test]
    fn faces_et_directions_se_correspondent() {
        for face in 0..6 {
            for (i, j) in [(0, 0), (5, 9), (15, 15), (8, 3)] {
                let d = texel_dir(face, i, j, 16);
                let (f, u, v) = dir_to_face(d);
                assert_eq!(f, face);
                assert!((u * 16.0 - (i as f64 + 0.5)).abs() < 1e-9);
                assert!((v * 16.0 - (j as f64 + 0.5)).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn le_noyau_brille_plus_que_le_pole() {
        let spec = hexis();
        let luminance = |d| glow(&spec, normalize(d)).iter().sum::<f32>();
        let core = luminance([-1.0, 0.0, 0.0]); // vers l'ouest : le noyau
        let outward = luminance([1.0, 0.0, 0.0]); // vers l'est : le bord de la galaxie
        let pole = luminance([0.0, 1.0, 0.0]);
        assert!(core > 3.0 * outward, "noyau {core}, bord {outward}");
        assert!(outward > 2.0 * pole, "bord {outward}, pôle {pole}");
    }

    #[test]
    fn le_croissant_se_voit_depuis_hexis() {
        let spec = hexis();
        let mut without = spec.clone();
        without.nebulae.clear();
        // Vers le nord (−Z), un peu à l'ouest de l'arc.
        let d = normalize([-0.3, 0.0, -1.0]);
        let with_nebula = glow(&spec, d)[0];
        let bare = glow(&without, d)[0];
        assert!(with_nebula > 1.5 * bare, "avec {with_nebula}, sans {bare}");
    }

    #[test]
    fn le_ciel_est_deterministe() {
        let spec = hexis();
        let a = render_sky(&spec, 32);
        let b = render_sky(&spec, 32);
        assert!(a.faces == b.faces);
    }
}
