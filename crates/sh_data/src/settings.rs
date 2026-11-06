//! Réglages de jeu en données (`assets/data/gameplay/`) : préréglages graphiques, horloge.
//!
//! Ces schémas restent indépendants de Bevy : `sh_game` traduit chaque champ en composants et
//! ressources de rendu.

use serde::{Deserialize, Serialize};

/// Les trois préréglages graphiques (AGENTS.md §10). « Bas » est la cible de la machine de
/// référence (UHD 770, 1080p, 60 i/s ; « Moyen » jusqu'à M7, Q15).
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

/// Filtrage des ombres : `Hardware2x2` lit un échantillon par pixel, `Gaussian` neuf (bords
/// plus doux, nettement plus cher sur GPU intégré).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowFilter {
    Hardware2x2,
    #[default]
    Gaussian,
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
    #[serde(default)]
    pub shadow_filter: ShadowFilter,
    /// Lumières ponctuelles ombrées en même temps, au plus (cellules sans lightmap seulement).
    pub max_shadowed_point_lights: u8,
    /// Éclairage précalculé des intérieurs (lightmaps) : sans lui, tout l'éclairage est
    /// dynamique (comparaisons de performance, `SH_BAKED_LIGHTING=0`).
    pub baked_lighting: bool,
    /// Lumières dynamiques dans les cellules à lightmap : leur diffus est déjà cuit, elles
    /// n'ajoutent que reflets et éclairage des objets mobiles (portes, personnages). Sur GPU
    /// intégré, chaque pixel les évalue toutes : c'est le poste le plus cher (mesures de M7).
    #[serde(default)]
    pub baked_cell_lights: bool,
    /// Vitrages éclairés (reflets du soleil) ; sinon une simple teinte, bien moins chère.
    #[serde(default)]
    pub lit_glass: bool,
    pub bloom: bool,
    pub anti_aliasing: AntiAliasing,
    /// Filtrage anisotrope des textures de la bibliothèque de matériaux (1 = désactivé, au plus
    /// 16) : net sur les sols vus en biais, mais chaque échantillon coûte plus de lectures.
    #[serde(default = "default_anisotropy")]
    pub anisotropy: u16,
    /// Distance d'affichage maximale des détails (mètres), pour les `VisibilityRange`.
    pub detail_distance: f32,
    /// Niveau de décor affiché (docs/m9-retours.md §3.2) : un nœud glTF dont l'extra `detail`
    /// dépasse ce niveau est caché avec ses enfants. 0 en Bas (le décor de base seul), 1 en
    /// Moyen, 2 en Haut (tout le décor) ; au plus [`GraphicsPreset::MAX_DECOR_DETAIL`].
    #[serde(default)]
    pub decor_detail: u8,
}

fn default_anisotropy() -> u16 {
    1
}

impl GraphicsPreset {
    /// Le niveau de décor le plus riche qu'un objet puisse demander.
    pub const MAX_DECOR_DETAIL: u8 = 2;

    /// Un décor de ce niveau (extra glTF `detail`, absent = 0) est-il affiché ?
    pub fn shows_detail(&self, detail: u8) -> bool {
        detail <= self.decor_detail
    }
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
    /// bornée par `cruise_min_speed` et `cruise_max_speed` (m/s). `cruise_min_speed` reste
    /// au-dessus de toute poussée ([`Self::manoeuvre_max_speed`]) : la croisière finit toujours
    /// plus vite que les moteurs seuls (le tutoriel la reconnaît à sa vitesse).
    pub cruise_rate: f64,
    pub cruise_min_speed: f64,
    pub cruise_max_speed: f64,
    /// Montée de la croisière (M11, réponse de l'auteur à Q19 : pas d'accélération brutale d'un
    /// coup) : l'accélération part de `cruise_spool_accel` (m/s²) et se multiplie par e toutes
    /// les `cruise_spool_time` secondes ([`Self::cruise_climb`]) : douce au départ, de plus en
    /// plus vive, jusqu'à la vitesse limite. La descente, elle, suit la limite sans délai.
    pub cruise_spool_accel: f64,
    pub cruise_spool_time: f64,
    /// Vitesse maximale en poussée de tout vaisseau, hors croisière (m/s) : le plafond des
    /// `max_speed` de `flight.ron` (P33). La montée de la croisière compte à partir d'elle.
    pub manoeuvre_max_speed: f64,
}

/// Au plus e^60 : la montée d'une longue croisière ne déborde pas (elle suit alors sa limite).
const MAX_SPOOL_EXPONENT: f64 = 60.0;

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

    /// Le niveau de montée (s) qui correspond à `speed` : le temps qu'il faut à la montée, partie
    /// de la poussée maximale, pour l'atteindre (0 jusqu'à elle). Une croisière engagée ou suivie
    /// à cette vitesse reprend sa montée de là : ni saut, ni reprise trop molle.
    pub fn cruise_spool_level(&self, speed: f64) -> f64 {
        let over = (speed - self.manoeuvre_max_speed).max(0.0);
        let tau = self.cruise_spool_time.max(1e-6);
        tau * (over / (self.cruise_spool_accel.max(1e-6) * tau)).ln_1p()
    }

    /// Un pas de montée de la croisière : de `speed` vers `target` (plus haute), au niveau de
    /// montée `spool` (s). L'accélération vaut `cruise_spool_accel` × e^(spool / τ) : depuis la
    /// poussée maximale, la vitesse gagnée est a₀·τ·(e^(t/τ) − 1). Rend la vitesse et le niveau
    /// suivants ; rattrapée par sa consigne, la montée se recale sur la vitesse atteinte
    /// ([`Self::cruise_spool_level`]). Logique pure (vol et durées de trajet).
    pub fn cruise_climb(&self, speed: f64, spool: f64, target: f64, dt: f64) -> (f64, f64) {
        let tau = self.cruise_spool_time.max(1e-6);
        let accel = self.cruise_spool_accel * (spool / tau).min(MAX_SPOOL_EXPONENT).exp();
        let next = (speed + accel * dt).min(target);
        let spool = if next >= target {
            (spool + dt).min(self.cruise_spool_level(next))
        } else {
            spool + dt
        };
        (next, spool)
    }

    /// Durée (s) d'un trajet en croisière en ligne droite, de `start` mètres de la surface du
    /// départ à `end` mètres de celle de l'arrivée, sur `length` mètres : départ arrêté, montée
    /// progressive ([`Self::cruise_climb`]) sous la vitesse limite, qu'on suit ensuite
    /// (intégration numérique ; sert à l'équilibrage et aux tests).
    pub fn cruise_time(&self, length: f64, start: f64, end: f64) -> f64 {
        let span = length - start - end;
        if span <= 0.0 {
            return 0.0;
        }
        let (mut travelled, mut time, mut v, mut spool) = (0.0, 0.0, 0.0f64, 0.0);
        while travelled < span {
            let nearest = (start + travelled).min(span - travelled + end);
            let target = self.cruise_speed_limit(nearest);
            // Pas adaptatif : au plus 1 % de la distance au plus proche ou 0,1 s.
            let dt = (0.01 * nearest.max(1.0) / v.max(1.0)).clamp(1e-3, 0.1);
            if target > v {
                (v, spool) = self.cruise_climb(v, spool, target, dt);
            } else {
                v = target;
                spool = (spool + dt).min(self.cruise_spool_level(v));
            }
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
    /// Vitesse maximale en poussée (m/s), hors croisière (P33 : 1 km/s pour la corvette).
    pub max_speed: f64,
    /// Vitesse maximale en poussée tant que la croisière est interdite par des hostiles proches
    /// (m/s, hypothèse de M11, Q19) : sans elle, un raid se fuirait à la poussée et les combats
    /// (pillards à 260–320 m/s, tirs) seraient déséquilibrés. Au-dessus, le vaisseau ralentit
    /// comme au sortir de la croisière ([`Self::cruise_spool_down`]).
    pub combat_max_speed: f64,
    /// Accélérations par axe (m/s²), jusqu'à la vitesse de combat.
    pub accel_forward: f64,
    pub accel_backward: f64,
    pub accel_lateral: f64,
    pub accel_vertical: f64,
    /// Au-delà de la vitesse de combat, les accélérations sont multipliées par ce facteur (≥ 1) :
    /// la finesse à basse vitesse (amarrage, combat) ne change pas, et l'on atteint la poussée
    /// maximale, ou l'on s'arrête, sans y passer une demi-minute.
    pub fast_accel_factor: f64,
    /// Vitesses de rotation maximales (degrés/s).
    pub pitch_rate_deg: f64,
    pub yaw_rate_deg: f64,
    pub roll_rate_deg: f64,
    /// Réactivité de la rotation (1/s) : à quelle vitesse la rotation suit la commande.
    pub turn_response: f64,
    /// Durée caractéristique (s) du retour de la croisière à la vitesse de manœuvre (et de la
    /// poussée à la vitesse de combat).
    pub cruise_spool_down: f64,
}

impl Handling {
    /// Vitesse maximale en poussée : celle de combat quand des hostiles interdisent la croisière.
    pub fn thrust_limit(&self, combat: bool) -> f64 {
        if combat {
            self.combat_max_speed.min(self.max_speed)
        } else {
            self.max_speed
        }
    }

    /// Multiplicateur des accélérations à `speed` m/s ([`Self::fast_accel_factor`] au-delà de la
    /// vitesse de combat).
    pub fn accel_factor(&self, speed: f64) -> f64 {
        if speed > self.combat_max_speed {
            self.fast_accel_factor.max(1.0)
        } else {
            1.0
        }
    }

    /// Distance (m) pour s'arrêter en vol assisté depuis `speed` m/s (au plus la poussée
    /// maximale), en marche arrière : accélération renforcée au-delà de la vitesse de combat.
    pub fn brake_distance(&self, speed: f64) -> f64 {
        let brake = self.accel_backward.max(1e-3);
        let low = self.combat_max_speed.max(0.0);
        let speed = speed.max(0.0);
        if speed <= low {
            speed * speed / (2.0 * brake)
        } else {
            let fast = brake * self.accel_factor(speed);
            (speed * speed - low * low) / (2.0 * fast) + low * low / (2.0 * brake)
        }
    }
}

/// Horloge du monde.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClockSettings {
    /// Secondes de jeu par seconde réelle hors voyages (×30 : P4, validée par l'auteur).
    pub time_scale: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corvette() -> Handling {
        Handling {
            max_speed: 1000.0,
            combat_max_speed: 250.0,
            accel_forward: 45.0,
            accel_backward: 25.0,
            accel_lateral: 20.0,
            accel_vertical: 20.0,
            fast_accel_factor: 3.0,
            pitch_rate_deg: 40.0,
            yaw_rate_deg: 30.0,
            roll_rate_deg: 80.0,
            turn_response: 3.0,
            cruise_spool_down: 1.5,
        }
    }

    #[test]
    fn la_vitesse_de_combat_borne_la_poussee() {
        let h = corvette();
        assert_eq!(h.thrust_limit(false), 1000.0);
        assert_eq!(h.thrust_limit(true), 250.0);
        // Une vitesse de combat au-dessus de la poussée maximale ne la dépasse pas.
        let odd = Handling {
            combat_max_speed: 2000.0,
            ..corvette()
        };
        assert_eq!(odd.thrust_limit(true), 1000.0);
        assert_eq!(h.accel_factor(200.0), 1.0);
        assert_eq!(h.accel_factor(250.0), 1.0);
        assert_eq!(h.accel_factor(251.0), 3.0);
    }

    #[test]
    fn la_distance_de_freinage_suit_l_integration() {
        let h = corvette();
        // Freinage pas à pas, comme le vol assisté (accélération lue au début de chaque pas).
        for start in [100.0, 250.0, 600.0, 1000.0] {
            let (mut v, mut d, dt) = (start, 0.0, 1e-3);
            while v > 0.0 {
                let a = h.accel_backward * h.accel_factor(v);
                let next = (v - a * dt).max(0.0);
                d += 0.5 * (v + next) * dt;
                v = next;
            }
            let closed = h.brake_distance(start);
            assert!(
                (d - closed).abs() < 0.01 * closed + 1.0,
                "{start} m/s : {d} / {closed}"
            );
        }
        // Repères de P33 : de 1 000 m/s, ≈ 7,5 km ; de 250 m/s, 1,25 km comme avant M11.
        assert!((h.brake_distance(1000.0) - 7500.0).abs() < 1.0);
        assert!((h.brake_distance(250.0) - 1250.0).abs() < 1e-6);
    }

    /// L'échelle de `space.ron` (croisière de P2, montée de M11).
    fn scale() -> SystemScale {
        SystemScale {
            au: 1.496e9,
            first_orbit_au: 0.4,
            orbit_ratio: 1.7,
            star_radius: 7.0e7,
            planet_radius: 6.4e5,
            gas_giant_radius: 7.0e6,
            moon_radius: 1.7e5,
            moon_orbit_radii: 12.0,
            parking_orbit_radii: 1.5,
            cruise_rate: 0.15,
            cruise_min_speed: 2000.0,
            cruise_max_speed: 3.0e7,
            cruise_spool_accel: 30.0,
            cruise_spool_time: 1.5,
            manoeuvre_max_speed: 1000.0,
        }
    }

    #[test]
    fn la_montee_suit_sa_loi_et_se_recale_sur_la_vitesse() {
        let s = scale();
        // Depuis la poussée maximale, au pas fin : 1 000 + a₀·τ·(e^(t/τ) − 1).
        let (mut v, mut spool, dt) = (1000.0, 0.0, 1e-4);
        for _ in 0..30_000 {
            (v, spool) = s.cruise_climb(v, spool, f64::MAX, dt);
        }
        let expected = 1000.0 + 30.0 * 1.5 * (2.0f64).exp_m1();
        assert!((v - expected).abs() < 0.01 * expected, "{v} / {expected}");
        // Le niveau correspond à la vitesse : son inverse la redonne.
        assert!((spool - s.cruise_spool_level(v)).abs() < 0.01, "{spool}");
        assert_eq!(s.cruise_spool_level(800.0), 0.0);
        // Rattrapée par sa consigne, la montée se recale sur la vitesse atteinte.
        let (v, spool) = s.cruise_climb(5.0e4, 40.0, 5.0e4 + 1.0, 0.1);
        assert_eq!(v, 5.0e4 + 1.0);
        assert!((spool - s.cruise_spool_level(v)).abs() < 1e-9);
        // Une très longue montée ne déborde pas.
        let (v, _) = s.cruise_climb(1.0e6, 1.0e6, 3.0e7, 1.0 / 60.0);
        assert_eq!(v, 3.0e7);
    }

    #[test]
    fn la_montee_ajoute_quelques_secondes_aux_trajets() {
        let s = scale();
        let instant = SystemScale {
            cruise_spool_accel: 1.0e12,
            ..scale()
        };
        let parking = s.parking_orbit_radii * s.planet_radius;
        let length = s.orbit_radius(3) - s.orbit_radius(2);
        let (slow, fast) = (
            s.cruise_time(length, parking, parking),
            instant.cruise_time(length, parking, parking),
        );
        assert!((5.0..20.0).contains(&(slow - fast)), "{slow} / {fast}");
    }
}
