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
    /// Lumières ponctuelles ombrées en même temps, au plus.
    pub max_shadowed_point_lights: u8,
    pub bloom: bool,
    pub anti_aliasing: AntiAliasing,
    /// Distance d'affichage maximale des détails (mètres), pour les `VisibilityRange`.
    pub detail_distance: f32,
}

/// Horloge du monde.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClockSettings {
    /// Secondes de jeu par seconde réelle hors voyages (×30 : P4, validée par l'auteur).
    pub time_scale: f64,
}
