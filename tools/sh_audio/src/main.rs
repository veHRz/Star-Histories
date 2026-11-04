//! Génère les sons du jeu dans `assets/audio` (WAV mono 16 bits) : synthèse déterministe, sans
//! échantillon extérieur ni dépendance, donc sans question de licence (`assets/CREDITS.md`).
//!
//!     cargo run --release -p sh_audio [-- <dossier>]
//!
//! Les boucles (musiques, ambiances, moteur, alarme) se raccordent sans coupure.

mod dsp;
mod sounds;

use std::path::PathBuf;
use std::process::ExitCode;

type Recipe = fn() -> Vec<f32>;

/// Chaque son : chemin relatif, fréquence, crête de normalisation, recette.
const SOUNDS: &[(&str, f32, f32, Recipe)] = &[
    (
        "music/espace.wav",
        sounds::LONG_RATE,
        0.8,
        sounds::music_space,
    ),
    (
        "music/combat.wav",
        sounds::LONG_RATE,
        0.8,
        sounds::music_combat,
    ),
    (
        "ambience/vaisseau.wav",
        sounds::LONG_RATE,
        0.7,
        sounds::ship_ambience,
    ),
    (
        "ambience/spatioport.wav",
        sounds::LONG_RATE,
        0.7,
        sounds::port_ambience,
    ),
    ("sfx/clic.wav", sounds::SFX_RATE, 0.6, sounds::click),
    (
        "sfx/notification.wav",
        sounds::SFX_RATE,
        0.6,
        sounds::notification,
    ),
    ("sfx/porte.wav", sounds::SFX_RATE, 0.8, sounds::door),
    ("sfx/amarrage.wav", sounds::SFX_RATE, 0.9, sounds::docking),
    ("sfx/alarme.wav", sounds::SFX_RATE, 0.35, sounds::alarm),
    ("sfx/moteur.wav", sounds::SFX_RATE, 0.8, sounds::engine),
    ("sfx/pistolet.wav", sounds::SFX_RATE, 0.9, sounds::pistol),
    ("sfx/impact.wav", sounds::SFX_RATE, 0.8, sounds::impact),
    ("sfx/canon.wav", sounds::SFX_RATE, 0.9, sounds::cannon),
    ("sfx/missile.wav", sounds::SFX_RATE, 0.85, sounds::missile),
    (
        "sfx/explosion.wav",
        sounds::SFX_RATE,
        0.95,
        sounds::explosion,
    ),
    ("sfx/coque.wav", sounds::SFX_RATE, 0.95, sounds::hull_hit),
    ("sfx/saut.wav", sounds::SFX_RATE, 0.8, sounds::jump),
];

fn main() -> ExitCode {
    let dir = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("assets/audio"), PathBuf::from);
    let mut failed = false;
    for (path, rate, peak, recipe) in SOUNDS {
        let samples = recipe();
        let file = dir.join(path);
        match dsp::write_wav(&file, *rate as u32, &samples, *peak) {
            Ok(()) => {
                // Niveau moyen après normalisation : un son muet ou écrasé se voit ici.
                let max = samples.iter().fold(0f32, |m, s| m.max(s.abs())).max(1e-9);
                let rms = (samples
                    .iter()
                    .map(|s| (s / max * peak).powi(2))
                    .sum::<f32>()
                    / samples.len().max(1) as f32)
                    .sqrt();
                println!(
                    "{} : {:.2} s, {} Hz, niveau moyen {:.1} dB",
                    file.display(),
                    samples.len() as f32 / rate,
                    rate,
                    20.0 * rms.max(1e-9).log10()
                );
            }
            Err(err) => {
                eprintln!("{} : {err}", file.display());
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_synthese_est_reproductible() {
        assert_eq!(sounds::click(), sounds::click());
        assert_eq!(sounds::door(), sounds::door());
    }

    #[test]
    fn les_boucles_ont_leur_longueur_exacte() {
        let engine = sounds::engine();
        assert_eq!(engine.len(), (4.0 * sounds::SFX_RATE) as usize);
        assert!(engine.iter().all(|s| s.is_finite()));
    }
}
