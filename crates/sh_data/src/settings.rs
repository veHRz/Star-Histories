//! Réglages de jeu en données (`assets/data/gameplay/`) : préréglages graphiques, horloge.
//!
//! Ces schémas restent indépendants de Bevy : `sh_game` traduit chaque champ en composants et
//! ressources de rendu.

use serde::{Deserialize, Serialize};

/// Les trois préréglages graphiques (AGENTS.md §10). « Moyen » est la cible de la machine de
/// référence (UHD 770, 1080p, 60 i/s).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphicsPresets {
    pub low: GraphicsPreset,
    pub medium: GraphicsPreset,
    pub high: GraphicsPreset,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetLevel {
    Low,
    #[default]
    Medium,
    High,
}

impl PresetLevel {
    pub const ALL: [PresetLevel; 3] = [PresetLevel::Low, PresetLevel::Medium, PresetLevel::High];
}

impl GraphicsPresets {
    pub fn get(&self, level: PresetLevel) -> &GraphicsPreset {
        match level {
            PresetLevel::Low => &self.low,
            PresetLevel::Medium => &self.medium,
            PresetLevel::High => &self.high,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiAliasing {
    Off,
    Fxaa,
    Smaa,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphicsPreset {
    /// Résolution de rendu de la scène 3D relative à la fenêtre (0,5 à 1). L'interface reste nette.
    pub render_scale: f32,
    /// Ombres de la lumière directionnelle principale.
    pub directional_shadows: bool,
    /// Taille (en pixels) de la carte d'ombre directionnelle.
    pub shadow_map_size: u32,
    /// Nombre de cascades d'ombre.
    pub shadow_cascades: u8,
    /// Distance maximale des ombres, en mètres.
    pub shadow_max_distance: f32,
    /// Lumières ponctuelles ombrées en même temps, au plus (cellules sans lightmap seulement).
    pub max_shadowed_point_lights: u8,
    /// Éclairage précalculé des intérieurs (lightmaps) : sans lui, tout l'éclairage est
    /// dynamique (comparaisons de performance, `SH_BAKED_LIGHTING=0`).
    pub baked_lighting: bool,
    pub bloom: bool,
    pub anti_aliasing: AntiAliasing,
    /// Distance d'affichage maximale des détails (mètres), pour les `VisibilityRange`.
    pub detail_distance: f32,
}

/// Espace de la vue Globale (AGENTS.md §6.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpaceSettings {
    /// Distance (m) du point suivi à l'origine flottante au-delà de laquelle on recentre. Sous
    /// 2 km, un `f32` reste précis au dixième de millimètre.
    pub recenter_distance: f64,
    /// Échelle de jeu des systèmes stellaires (proposition P2).
    pub scale: SystemScale,
}

/// Échelle de jeu des systèmes stellaires : **proposition P2** (gameplay, pas un fait d'univers).
///
/// Le canon ne donne ni tailles ni distances orbitales (la disposition de la carte est
/// schématique, §0 de l'export). Le jeu compresse : tailles ÷ 10, distances ÷ 100 par rapport à
/// des ordres de grandeur réels, pour que les astres soient lisibles de loin et que les trajets
/// intra-système durent quelques minutes. Toutes les longueurs sont en mètres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SystemScale {
    /// Une « unité astronomique » de jeu (1 UA réelle ÷ 100).
    pub au: f64,
    /// Rayon orbital du premier rang autour de l'étoile, en UA de jeu.
    pub first_orbit_au: f64,
    /// Rapport entre deux rangs orbitaux successifs.
    pub orbit_ratio: f64,
    /// Rayons typiques par genre d'astre (tailles réelles ÷ 10).
    pub star_radius: f64,
    pub planet_radius: f64,
    pub gas_giant_radius: f64,
    pub moon_radius: f64,
    /// Orbite d'une lune, en rayons de sa planète.
    pub moon_orbit_radii: f64,
    /// Altitude des stations et des orbites de stationnement, en rayons de la planète.
    pub parking_orbit_radii: f64,
    /// Croisière : vitesse limite = `cruise_rate` × distance à la surface la plus proche,
    /// bornée par `cruise_min_speed` et `cruise_max_speed` (m/s).
    pub cruise_rate: f64,
    pub cruise_min_speed: f64,
    pub cruise_max_speed: f64,
    /// Vitesse maximale en vol de manœuvre et de combat, hors croisière (m/s).
    pub manoeuvre_max_speed: f64,
}

impl SystemScale {
    /// Rayon orbital (m) du rang `rank` (1 = le plus proche de l'étoile).
    pub fn orbit_radius(&self, rank: u8) -> f64 {
        let exponent = i32::from(rank.max(1)) - 1;
        self.au * self.first_orbit_au * self.orbit_ratio.powi(exponent)
    }

    /// Vitesse de croisière permise à `distance` mètres de la surface la plus proche.
    pub fn cruise_speed_limit(&self, distance: f64) -> f64 {
        (self.cruise_rate * distance.max(0.0)).clamp(self.cruise_min_speed, self.cruise_max_speed)
    }

    /// Durée (s) d'un trajet en croisière en ligne droite, de `start` mètres de la surface du
    /// départ à `end` mètres de celle de l'arrivée, sur `length` mètres, à la vitesse limite
    /// (intégration numérique ; sert à l'équilibrage et aux tests).
    pub fn cruise_time(&self, length: f64, start: f64, end: f64) -> f64 {
        let span = length - start - end;
        if span <= 0.0 {
            return 0.0;
        }
        let (mut travelled, mut time) = (0.0, 0.0);
        while travelled < span {
            let nearest = (start + travelled).min(span - travelled + end);
            let v = self.cruise_speed_limit(nearest);
            // Pas adaptatif : au plus 1 % de la distance au plus proche ou 0,1 s.
            let dt = (0.01 * nearest.max(1.0) / v).clamp(1e-3, 0.1);
            travelled += v * dt;
            time += dt;
        }
        time
    }
}

/// Maniement des vaisseaux pilotables, par identifiant de secteur (`gameplay/flight.ron`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlightSettings {
    pub ships: std::collections::BTreeMap<String, Handling>,
}

/// Maniement d'un vaisseau (réglage de jeu, pas un fait d'univers).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Handling {
    /// Vitesse maximale en vol assisté (m/s), hors croisière.
    pub max_speed: f64,
    /// Accélérations par axe (m/s²).
    pub accel_forward: f64,
    pub accel_backward: f64,
    pub accel_lateral: f64,
    pub accel_vertical: f64,
    /// Vitesses de rotation maximales (degrés/s).
    pub pitch_rate_deg: f64,
    pub yaw_rate_deg: f64,
    pub roll_rate_deg: f64,
    /// Réactivité de la rotation (1/s) : à quelle vitesse la rotation suit la commande.
    pub turn_response: f64,
    /// Durée caractéristique (s) du retour de la croisière à la vitesse de manœuvre.
    pub cruise_spool_down: f64,
}

/// Horloge du monde.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClockSettings {
    /// Secondes de jeu par seconde réelle hors voyages (×30 : P4, validée par l'auteur).
    pub time_scale: f64,
}
