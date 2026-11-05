//! Apparence peinte des astres de la vue Globale (`assets/data/gameplay/planets.ron`, M9).
//!
//! Un astre qui a une entrée est rendu par le matériau de planète du jeu
//! (`sh_game::views::planet`, shader `assets/shaders/planet.wesl`) avec des textures peintes hors
//! du jeu (`tools/planets/`) : couleur de jour, données de surface (lumières de nuit, ville,
//! industrie, eau) et détail carrelable plaqué à deux échelles. Un astre sans entrée garde sa
//! texture procédurale. Ce sont des réglages visuels, pas des faits d'univers : Hexis, monde-usine
//! très urbanisé, suit la décision de l'auteur (Q17).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Place;
use crate::galaxy::Galaxy;
use crate::settings::PresetLevel;

/// Le fichier des apparences : identifiant d'astre → apparence.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlanetLooks {
    #[serde(default)]
    pub bodies: BTreeMap<String, PlanetLook>,
}

/// Une valeur par préréglage graphique (la résolution d'une texture, par exemple).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ByPreset<T> {
    pub low: T,
    pub medium: T,
    pub high: T,
}

impl<T> ByPreset<T> {
    pub fn get(&self, level: PresetLevel) -> &T {
        match level {
            PresetLevel::Low => &self.low,
            PresetLevel::Medium => &self.medium,
            PresetLevel::High => &self.high,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [&self.low, &self.medium, &self.high].into_iter()
    }
}

/// L'apparence d'un astre. Les chemins sont relatifs au dossier `assets/`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanetLook {
    /// Couleur de jour, équirectangulaire, sRGB, par préréglage (2K en Bas, 4K en Haut).
    pub color: ByPreset<String>,
    /// Données de surface, équirectangulaire, linéaires : R lumières de nuit, G densité urbaine,
    /// B industrie, A eau.
    pub surface: String,
    /// Détail carrelable, linéaire : R trame urbaine, G éclairage des rues, B toits industriels,
    /// A relief (cratères, gravats). Chaque canal est divisé par sa moyenne : il module la couleur
    /// sans la décaler, et s'efface au loin avec les mipmaps.
    pub detail: String,
    /// Les deux couches de détail.
    pub detail_layers: DetailLayers,
    pub detail_strength: DetailStrength,
    pub night: NightLights,
    pub haze: Haze,
    /// Rugosité perçue des terres (0 à 1).
    pub roughness: f32,
    /// Rugosité perçue de l'eau : plus lisse, elle renvoie un reflet du soleil.
    pub water_roughness: f32,
    /// Fini des zones industrielles (toits et cuves métalliques) : rugosité et part de métal,
    /// pour qu'elles accrochent des reflets du soleil.
    pub industry_roughness: f32,
    pub industry_metallic: f32,
}

/// La grande couche (quartiers, vue de loin) et la fine (îlots, orbite basse).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DetailLayers {
    pub coarse: DetailLayer,
    pub fine: DetailLayer,
}

/// Une couche de détail plaquée en projection triplanaire.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DetailLayer {
    /// Côté d'un carreau, en kilomètres de jeu (échelle P2).
    pub tile_km: f32,
    /// Poids de la couche dans la modulation.
    pub weight: f32,
    /// Flou volontaire, en niveaux de mipmap (0 : net).
    #[serde(default)]
    pub blur: f32,
}

/// Force de la modulation par canal du détail.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DetailStrength {
    pub urban: f32,
    pub industry: f32,
    pub rough: f32,
    pub lights: f32,
}

/// Lumières des villes, côté nuit seulement.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NightLights {
    /// Teinte des lumières (linéaire), et des centres les plus éclairés.
    pub color: [f32; 3],
    pub core_color: [f32; 3],
    /// Émission d'un texel pleinement éclairé, avant le tonemapping. L'émissif n'est pas
    /// atténué par l'exposition de la caméra (AGENTS.md §4) : ≈ 1 à 2 pour une ville dense.
    pub intensity: f32,
    /// Allumage selon −N·L (N : normale, L : vers l'étoile) : éteint en deçà du premier seuil,
    /// pleinement allumé au-delà du second. Une valeur négative allume avant le terminateur.
    pub on: [f32; 2],
}

/// Voile de brume : liseré au limbe (fresnel), éclairé par l'étoile.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Haze {
    /// Teinte (linéaire), multipliée par l'éclairement de l'étoile.
    pub color: [f32; 3],
    /// Opacité au limbe (0 à 1).
    pub strength: f32,
    /// Puissance du fresnel : plus elle est grande, plus le liseré est fin.
    pub power: f32,
    /// Voile minimal sur tout le disque (0 à 1).
    pub floor: f32,
    /// Éclairage du voile selon N·L : nul en deçà du premier seuil, plein au-delà du second
    /// (le crépuscule déborde un peu sur le côté nuit).
    pub lit: [f32; 2],
}

impl PlanetLooks {
    pub fn get(&self, body: &str) -> Option<&PlanetLook> {
        self.bodies.get(body)
    }

    /// Incohérences (messages sans nom de fichier : la validation le donne en contexte) : astre
    /// inconnu ou secret, texture absente (si `assets_dir` est donné), réglage hors de sa plage.
    /// Vide si tout va bien.
    pub fn check(&self, galaxy: &Galaxy, assets_dir: Option<&Path>) -> Vec<String> {
        let mut issues = Vec::new();
        for (id, look) in &self.bodies {
            let body = sh_core::Id::<Place>::new(id)
                .ok()
                .and_then(|i| galaxy.body(&i));
            match body {
                None => issues.push(format!("astre inconnu « {id} »")),
                Some(b) if b.secret => issues.push(format!("« {id} » est un astre secret")),
                Some(_) => {}
            }
            if let Some(dir) = assets_dir {
                let paths = look
                    .color
                    .iter()
                    .chain([&look.surface, &look.detail])
                    .filter(|p| !dir.join(p).is_file());
                for path in paths {
                    issues.push(format!("{id} : texture absente « {path} »"));
                }
            }
            let layers = [look.detail_layers.coarse, look.detail_layers.fine];
            if layers
                .iter()
                .any(|l| !(l.tile_km > 0.0 && l.weight >= 0.0 && l.blur >= 0.0))
            {
                issues.push(format!("{id} : couche de détail invalide"));
            }
            let unit = |v: f32| (0.0..=1.0).contains(&v);
            let finish = [
                look.roughness,
                look.water_roughness,
                look.industry_roughness,
                look.industry_metallic,
            ];
            if !finish.into_iter().all(unit) {
                issues.push(format!("{id} : rugosité ou métal hors de [0, 1]"));
            }
            if !unit(look.haze.strength) || !unit(look.haze.floor) || look.haze.power <= 0.0 {
                issues.push(format!("{id} : voile hors de sa plage"));
            }
            if look.night.intensity < 0.0
                || look.night.on[0] >= look.night.on[1]
                || look.haze.lit[0] >= look.haze.lit[1]
            {
                issues.push(format!("{id} : seuils jour / nuit invalides"));
            }
        }
        issues
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn assets() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }

    #[test]
    fn le_fichier_du_jeu_est_coherent() {
        let data = assets().join("data");
        let looks: PlanetLooks = crate::read_ron(&data.join(crate::paths::PLANETS))
            .unwrap_or_else(|err| panic!("{err}"));
        let galaxy: Galaxy =
            crate::read_ron(&data.join(crate::paths::GALAXY)).unwrap_or_else(|err| panic!("{err}"));
        assert!(looks.get("hexis").is_some(), "Hexis a son apparence (Q17)");
        let issues = looks.check(&galaxy, Some(&assets()));
        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn valeur_par_preset() {
        let v = ByPreset {
            low: 1,
            medium: 2,
            high: 3,
        };
        assert_eq!(*v.get(PresetLevel::Low), 1);
        assert_eq!(*v.get(PresetLevel::High), 3);
        assert_eq!(v.iter().copied().sum::<i32>(), 6);
    }
}
