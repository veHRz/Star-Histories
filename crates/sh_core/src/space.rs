//! Positions spatiales en double précision (AGENTS.md §6.4).
//!
//! Un `f32` n'a que 24 bits de mantisse : à 150 000 km du centre d'un système, deux positions
//! voisines sont séparées de 16 m. Les positions de simulation sont donc en `f64` (précision
//! submillimétrique jusqu'à ≈ 10⁹ km), et le rendu travaille en `f32` **relativement à une origine
//! flottante** proche de la caméra ([`SpacePos::relative_to`]).

use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// Position (ou déplacement) dans le repère d'un système stellaire, en mètres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpacePos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl SpacePos {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn distance(self, other: Self) -> f64 {
        (self - other).length()
    }

    /// Position relative à `origin`, en `f32` pour le rendu. Exacte au ½ ulp `f32` près de la
    /// distance à l'origine (0,1 mm à 2 km) : la soustraction se fait en `f64`, avant l'arrondi.
    pub fn relative_to(self, origin: Self) -> [f32; 3] {
        let d = self - origin;
        [d.x as f32, d.y as f32, d.z as f32]
    }

    /// Position absolue d'un point exprimé en `f32` relativement à `origin`.
    pub fn from_relative(origin: Self, local: [f32; 3]) -> Self {
        origin
            + Self::new(
                f64::from(local[0]),
                f64::from(local[1]),
                f64::from(local[2]),
            )
    }
}

/// Faut-il recentrer l'origine ? Oui dès que le point suivi s'en éloigne de plus de `threshold`
/// mètres (un seuil nul recentre à chaque frame : le vaisseau du joueur comme référentiel).
pub fn needs_recenter(local: [f32; 3], threshold: f64) -> bool {
    let [x, y, z] = local.map(f64::from);
    (x * x + y * y + z * z).sqrt() > threshold
}

impl Add for SpacePos {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for SpacePos {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f64> for SpacePos {
    type Output = Self;
    fn mul(self, k: f64) -> Self {
        Self::new(self.x * k, self.y * k, self.z * k)
    }
}

impl AddAssign for SpacePos {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for SpacePos {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 10⁹ km : bien au-delà du plus grand système compressé (P2).
    const FAR: f64 = 1.0e12;

    #[test]
    fn relatif_exact_loin_du_centre() {
        let origin = SpacePos::new(FAR, -FAR / 3.0, 0.5 * FAR);
        let p = origin + SpacePos::new(1.25, -0.5, 0.001);
        let local = p.relative_to(origin);
        // f64 à 10¹² m : ulp de 0,12 mm ; le résultat relatif garde cette précision.
        assert!((local[0] - 1.25).abs() < 2e-4);
        assert!((local[1] + 0.5).abs() < 2e-4);
        assert!((local[2] - 0.001).abs() < 2e-4);
    }

    #[test]
    fn un_f32_seul_ne_suffirait_pas() {
        // Ce que l'origine flottante évite : à 1,5 × 10⁸ m, un f32 ne distingue pas 1 m.
        let a = 1.5e8_f32;
        assert_eq!(a + 1.0, a);
        let origin = SpacePos::new(1.5e8, 0.0, 0.0);
        let p = origin + SpacePos::new(1.0, 0.0, 0.0);
        assert_eq!(p.relative_to(origin)[0], 1.0);
    }

    #[test]
    fn aller_retour_relatif() {
        let origin = SpacePos::new(3.0e9, 7.0e8, -2.0e9);
        let local = [1234.5_f32, -0.25, 999.75];
        let p = SpacePos::from_relative(origin, local);
        assert_eq!(p.relative_to(origin), local);
    }

    #[test]
    fn seuil_de_recentrage() {
        assert!(!needs_recenter([1000.0, 0.0, 0.0], 2000.0));
        assert!(needs_recenter([1500.0, 1500.0, 0.0], 2000.0));
        assert!(needs_recenter([0.001, 0.0, 0.0], 0.0));
    }

    #[test]
    fn serde_ron() {
        let p = SpacePos::new(1.5e11, -2.0, 0.125);
        let text = ron::to_string(&p).unwrap_or_default();
        let back: SpacePos = ron::from_str(&text).unwrap_or_default();
        assert_eq!(back, p);
    }
}
