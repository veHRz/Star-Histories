//! Briques de synthèse : hasard déterministe, oscillateurs, filtres, enveloppes, écriture WAV.

use std::f32::consts::TAU;
use std::io::Write;
use std::path::Path;

/// Hasard reproductible (xorshift64*) : mêmes sons à chaque génération.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Réel uniforme dans [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Bruit blanc dans [-1, 1).
    pub fn noise(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

/// Bruit brun (intégré) : grave, souffle de ventilation, grondements.
pub struct Brown {
    value: f32,
}

impl Brown {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }

    pub fn next(&mut self, rng: &mut Rng) -> f32 {
        self.value = (self.value + rng.noise() * 0.02) * 0.998;
        self.value * 3.5
    }
}

/// Filtre à variables d'état (passe-bas, passe-bande, passe-haut), coupure modulable.
pub struct Svf {
    low: f32,
    band: f32,
    /// Amortissement : 2 ≈ sans résonance, 0,2 très résonant.
    damping: f32,
}

pub struct SvfOut {
    pub low: f32,
    pub band: f32,
    pub high: f32,
}

impl Svf {
    pub fn new(damping: f32) -> Self {
        Self {
            low: 0.0,
            band: 0.0,
            damping,
        }
    }

    pub fn process(&mut self, input: f32, cutoff: f32, rate: f32) -> SvfOut {
        let f =
            (2.0 * (std::f32::consts::PI * cutoff.clamp(10.0, rate * 0.24) / rate).sin()).min(1.0);
        self.low += f * self.band;
        let high = input - self.low - self.damping * self.band;
        self.band += f * high;
        SvfOut {
            low: self.low,
            band: self.band,
            high,
        }
    }
}

/// Oscillateur à phase continue (les glissandos ne craquent pas).
pub struct Osc {
    phase: f32,
}

impl Osc {
    pub fn new() -> Self {
        Self { phase: 0.0 }
    }

    fn advance(&mut self, freq: f32, rate: f32) -> f32 {
        let p = self.phase;
        self.phase = (self.phase + freq / rate).fract();
        p
    }

    pub fn sine(&mut self, freq: f32, rate: f32) -> f32 {
        (self.advance(freq, rate) * TAU).sin()
    }

    pub fn saw(&mut self, freq: f32, rate: f32) -> f32 {
        self.advance(freq, rate) * 2.0 - 1.0
    }

    pub fn square(&mut self, freq: f32, rate: f32) -> f32 {
        if self.advance(freq, rate) < 0.5 {
            1.0
        } else {
            -1.0
        }
    }
}

/// Décroissance exponentielle : 1 au départ, divisée par e toutes les `1 / speed` secondes.
pub fn decay(t: f32, speed: f32) -> f32 {
    if t < 0.0 { 0.0 } else { (-t * speed).exp() }
}

/// Montée linéaire en `attack` s, palier, descente linéaire sur les `release` dernières s.
pub fn envelope(t: f32, length: f32, attack: f32, release: f32) -> f32 {
    if t < 0.0 || t > length {
        return 0.0;
    }
    let up = if attack > 0.0 {
        (t / attack).min(1.0)
    } else {
        1.0
    };
    let down = if release > 0.0 {
        ((length - t) / release).min(1.0)
    } else {
        1.0
    };
    up * down
}

/// Fréquence d'une note MIDI (69 = la 440).
pub fn midi(note: i32) -> f32 {
    440.0 * 2f32.powf((note - 69) as f32 / 12.0)
}

/// Écho simple (délai en secondes, réinjection).
pub fn echo(buffer: &mut [f32], rate: f32, delay: f32, feedback: f32) {
    let d = (delay * rate) as usize;
    for i in d..buffer.len() {
        buffer[i] += buffer[i - d] * feedback;
    }
}

/// Rabat sur le début ce qui dépasse `length` échantillons : la boucle se raccorde sans coupure
/// (les queues de la fin sonnent au début, comme à la deuxième lecture).
pub fn wrap_loop(mut buffer: Vec<f32>, length: usize) -> Vec<f32> {
    if buffer.len() > length {
        let tail: Vec<f32> = buffer.drain(length..).collect();
        for (i, v) in tail.into_iter().enumerate() {
            buffer[i % length] += v;
        }
    }
    buffer.resize(length, 0.0);
    buffer
}

/// Écrit un WAV mono 16 bits, normalisé à `peak` (0 à 1).
pub fn write_wav(path: &Path, rate: u32, samples: &[f32], peak: f32) -> std::io::Result<()> {
    let max = samples.iter().fold(0f32, |m, s| m.max(s.abs())).max(1e-6);
    let gain = peak / max;
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s * gain).clamp(-1.0, 1.0);
        out.extend_from_slice(&((v * 32767.0).round() as i16).to_le_bytes());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::File::create(path)?.write_all(&out)
}
