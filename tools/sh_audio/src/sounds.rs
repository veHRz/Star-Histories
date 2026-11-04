//! Les recettes des sons. Direction proposée (P27) : space opera sobre et industriel, synthèse
//! lisible plutôt qu'imitation réaliste ; musique modale lente en exploration, pulsée en combat.

use crate::dsp::{Brown, Osc, Rng, Svf, decay, echo, envelope, midi, wrap_loop};

/// Fréquence des effets.
pub const SFX_RATE: f32 = 44_100.0;
/// Fréquence des musiques et ambiances (contenu grave et doux : moitié moins lourd).
pub const LONG_RATE: f32 = 22_050.0;

fn render(rate: f32, seconds: f32, mut f: impl FnMut(f32) -> f32) -> Vec<f32> {
    let n = (rate * seconds) as usize;
    (0..n).map(|i| f(i as f32 / rate)).collect()
}

// --- Interface ---------------------------------------------------------------------------------

pub fn click() -> Vec<f32> {
    let mut osc = Osc::new();
    render(SFX_RATE, 0.05, |t| {
        osc.sine(1800.0, SFX_RATE) * decay(t, 90.0) * 0.6
            + osc.sine(3600.0, SFX_RATE) * decay(t, 200.0) * 0.2
    })
}

pub fn notification() -> Vec<f32> {
    let (mut a, mut b) = (Osc::new(), Osc::new());
    render(SFX_RATE, 0.7, |t| {
        a.sine(880.0, SFX_RATE) * decay(t, 6.0) * 0.5
            + b.sine(1320.0, SFX_RATE) * decay(t - 0.12, 5.0) * 0.45 * f32::from(t > 0.12)
    })
}

// --- À bord ------------------------------------------------------------------------------------

pub fn door() -> Vec<f32> {
    let mut rng = Rng::new(11);
    let mut filter = Svf::new(0.7);
    let mut thump = Osc::new();
    render(SFX_RATE, 0.8, |t| {
        let sweep = 700.0 - 450.0 * (t / 0.6).min(1.0);
        let hiss = filter.process(rng.noise(), sweep, SFX_RATE).band * envelope(t, 0.62, 0.08, 0.2);
        hiss * 0.9 + thump.sine(70.0, SFX_RATE) * decay(t - 0.6, 14.0) * f32::from(t > 0.6) * 0.8
    })
}

pub fn docking() -> Vec<f32> {
    let mut rng = Rng::new(12);
    let (mut low, mut high) = (Svf::new(1.2), Svf::new(1.0));
    let mut clunk = Osc::new();
    render(SFX_RATE, 1.4, |t| {
        let hit = clunk.sine(82.0, SFX_RATE) * decay(t, 9.0)
            + low.process(rng.noise(), 320.0, SFX_RATE).low * decay(t, 12.0) * 2.0;
        let release = high.process(rng.noise(), 5000.0, SFX_RATE).high
            * envelope(t - 0.35, 1.0, 0.05, 0.8)
            * 0.35;
        hit + release
    })
}

pub fn alarm() -> Vec<f32> {
    let mut osc = Osc::new();
    let mut filter = Svf::new(1.4);
    // Une seconde, deux tons : la boucle ne s'entend pas.
    render(SFX_RATE, 1.0, |t| {
        let freq = if t < 0.5 { 660.0 } else { 880.0 };
        let edge = envelope(t % 0.5, 0.5, 0.01, 0.01);
        filter
            .process(osc.square(freq, SFX_RATE), 2400.0, SFX_RATE)
            .low
            * edge
            * 0.6
    })
}

pub fn engine() -> Vec<f32> {
    let mut rng = Rng::new(13);
    let mut brown = Brown::new();
    let (mut rumble, mut buzz) = (Svf::new(1.4), Svf::new(1.0));
    let (mut sub, mut saw, mut lfo) = (Osc::new(), Osc::new(), Osc::new());
    let length = 4.0;
    let samples = render(SFX_RATE, length + 0.5, |_| {
        let swell = 0.85 + 0.15 * lfo.sine(0.5, SFX_RATE);
        let noise = rumble.process(brown.next(&mut rng), 140.0, SFX_RATE).low;
        let body = buzz.process(saw.saw(90.0, SFX_RATE), 280.0, SFX_RATE).low;
        (noise * 1.2 + body * 0.5 + sub.sine(45.0, SFX_RATE) * 0.6) * swell
    });
    wrap_loop(samples, (length * SFX_RATE) as usize)
}

// --- Armes et impacts --------------------------------------------------------------------------

pub fn pistol() -> Vec<f32> {
    let mut rng = Rng::new(21);
    let mut snap = Svf::new(1.0);
    let (mut zap, mut thump) = (Osc::new(), Osc::new());
    render(SFX_RATE, 0.4, |t| {
        let crack = snap.process(rng.noise(), 4000.0, SFX_RATE).high * decay(t, 140.0) * 1.2;
        let glide = 2400.0 * (-t * 14.0).exp() + 300.0;
        crack
            + zap.sine(glide, SFX_RATE) * decay(t, 18.0) * 0.6
            + thump.sine(95.0, SFX_RATE) * decay(t, 25.0) * 0.7
    })
}

pub fn impact() -> Vec<f32> {
    let mut rng = Rng::new(22);
    let mut filter = Svf::new(0.6);
    let (mut a, mut b) = (Osc::new(), Osc::new());
    render(SFX_RATE, 0.3, |t| {
        filter.process(rng.noise(), 1500.0, SFX_RATE).band * decay(t, 45.0) * 1.4
            + (a.sine(1170.0, SFX_RATE) + b.sine(1730.0, SFX_RATE) * 0.7) * decay(t, 22.0) * 0.25
    })
}

pub fn cannon() -> Vec<f32> {
    let mut rng = Rng::new(23);
    let mut filter = Svf::new(1.2);
    let (mut boom, mut zap) = (Osc::new(), Osc::new());
    render(SFX_RATE, 0.7, |t| {
        let pitch = 35.0 + 25.0 * (-t * 10.0).exp();
        boom.sine(pitch, SFX_RATE) * decay(t, 7.0)
            + filter.process(rng.noise(), 420.0, SFX_RATE).low * decay(t, 10.0) * 1.6
            + zap.sine(150.0 + 750.0 * (-t * 20.0).exp(), SFX_RATE) * decay(t, 16.0) * 0.35
    })
}

pub fn missile() -> Vec<f32> {
    let mut rng = Rng::new(24);
    let mut filter = Svf::new(0.9);
    let mut whistle = Osc::new();
    render(SFX_RATE, 1.6, |t| {
        let cutoff = 300.0 + 2700.0 * (t / 0.4).min(1.0);
        let roar = filter.process(rng.noise(), cutoff, SFX_RATE).low * envelope(t, 1.6, 0.05, 1.2);
        let glide = 600.0 + 800.0 * (t / 1.2).min(1.0);
        roar * 1.3 + whistle.sine(glide, SFX_RATE) * envelope(t, 1.4, 0.2, 0.9) * 0.12
    })
}

pub fn explosion() -> Vec<f32> {
    let mut rng = Rng::new(25);
    let mut brown = Brown::new();
    let mut filter = Svf::new(1.1);
    let mut sub = Osc::new();
    render(SFX_RATE, 2.8, |t| {
        let cutoff = 150.0 + 1850.0 * (-t * 2.2).exp();
        let body = filter
            .process(brown.next(&mut rng) + rng.noise() * 0.3, cutoff, SFX_RATE)
            .low;
        let crackle = if rng.unit() < 0.004 * (-t * 1.5).exp() {
            rng.noise() * 2.0
        } else {
            0.0
        };
        (body * 2.0 + crackle) * envelope(t, 2.8, 0.005, 2.4)
            + sub.sine(40.0, SFX_RATE) * decay(t, 2.5) * 0.8
    })
}

pub fn hull_hit() -> Vec<f32> {
    let mut rng = Rng::new(26);
    let mut filter = Svf::new(1.0);
    let mut boom = Osc::new();
    let mut partials: Vec<Osc> = (0..4).map(|_| Osc::new()).collect();
    let freqs = [113.0, 287.0, 431.0, 619.0];
    render(SFX_RATE, 1.0, |t| {
        let metal: f32 = partials
            .iter_mut()
            .zip(freqs)
            .enumerate()
            .map(|(i, (o, f))| o.sine(f, SFX_RATE) * decay(t, 3.0 + i as f32 * 2.0))
            .sum();
        boom.sine(60.0 - 20.0 * (t / 0.5).min(1.0), SFX_RATE) * decay(t, 5.0)
            + metal * 0.25
            + filter.process(rng.noise(), 900.0, SFX_RATE).low * decay(t, 20.0)
    })
}

pub fn jump() -> Vec<f32> {
    let mut rng = Rng::new(27);
    let mut filter = Svf::new(0.5);
    let mut tone = Osc::new();
    render(SFX_RATE, 2.6, |t| {
        let k = (t / 2.2).min(1.0);
        let sweep = 200.0 * 20f32.powf(k);
        filter.process(rng.noise(), sweep, SFX_RATE).band * envelope(t, 2.6, 0.6, 0.6) * 1.5
            + tone.sine(100.0 + 700.0 * k * k, SFX_RATE) * envelope(t, 2.4, 0.8, 0.5) * 0.25
    })
}

// --- Ambiances ---------------------------------------------------------------------------------

/// L'intérieur du vaisseau : le ronronnement électrique, la ventilation.
pub fn ship_ambience() -> Vec<f32> {
    let length = 16.0;
    let mut rng = Rng::new(31);
    let mut brown = Brown::new();
    let mut vent = Svf::new(1.3);
    let (mut hum, mut hum2, mut lfo) = (Osc::new(), Osc::new(), Osc::new());
    let samples = render(LONG_RATE, length + 2.0, |_| {
        let breath = 0.75 + 0.25 * lfo.sine(1.0 / 8.0, LONG_RATE);
        vent.process(brown.next(&mut rng), 900.0 * breath, LONG_RATE)
            .low
            * 0.8
            + hum.sine(60.0, LONG_RATE) * 0.08
            + hum2.sine(120.25, LONG_RATE) * 0.04
    });
    wrap_loop(samples, (length * LONG_RATE) as usize)
}

/// Le hall du spatioport : l'air, une foule lointaine, des chocs de manutention.
pub fn port_ambience() -> Vec<f32> {
    let length = 20.0;
    let mut rng = Rng::new(32);
    let mut brown = Brown::new();
    let (mut air, mut crowd, mut am) = (Svf::new(1.4), Svf::new(0.8), Svf::new(1.4));
    let mut clank: Vec<(f32, f32)> = Vec::new();
    let mut seeds = Rng::new(33);
    let mut t0 = 1.0;
    while t0 < length {
        clank.push((t0, 180.0 + seeds.unit() * 400.0));
        t0 += 2.5 + seeds.unit() * 4.0;
    }
    let mut metal = Osc::new();
    let samples = render(LONG_RATE, length + 2.0, |t| {
        let wobble = am.process(rng.noise(), 3.0, LONG_RATE).low.abs() * 6.0;
        let voices = crowd.process(rng.noise(), 520.0, LONG_RATE).band * (0.3 + wobble);
        let hits: f32 = clank
            .iter()
            .map(|(at, f)| {
                let dt = t - at;
                if (0.0..1.5).contains(&dt) {
                    (dt * f * std::f32::consts::TAU).sin() * decay(dt, 6.0) * 0.12
                } else {
                    0.0
                }
            })
            .sum();
        air.process(brown.next(&mut rng), 600.0, LONG_RATE).low * 0.7
            + voices * 0.5
            + hits
            + metal.sine(55.0, LONG_RATE) * 0.02
    });
    wrap_loop(samples, (length * LONG_RATE) as usize)
}

// --- Musiques ----------------------------------------------------------------------------------

/// Exploration : quatre accords tenus (ré mineur modal), basse, notes pincées en écho.
pub fn music_space() -> Vec<f32> {
    let chord_len = 16.0;
    let chords: [(i32, [i32; 5]); 4] = [
        (38, [50, 53, 57, 60, 64]), // Rém9
        (34, [46, 50, 53, 57, 62]), // Sib maj7
        (31, [43, 46, 50, 53, 57]), // Solm9
        (33, [45, 50, 52, 55, 61]), // La sus4 → la
    ];
    let length = chord_len * chords.len() as f32;
    let total = (length + 8.0) * LONG_RATE;
    let mut out = vec![0f32; total as usize];
    let rate = LONG_RATE;
    for (index, (bass, notes)) in chords.iter().enumerate() {
        let start = index as f32 * chord_len;
        let span = chord_len + 4.0;
        // Nappe : deux scies désaccordées par note, filtrées, coupure qui respire.
        for (voice, note) in notes.iter().enumerate() {
            let (mut a, mut b, mut lfo) = (Osc::new(), Osc::new(), Osc::new());
            let mut filter = Svf::new(1.2);
            let f = midi(*note);
            let first = (start * rate) as usize;
            for i in 0..(span * rate) as usize {
                let t = i as f32 / rate;
                let cutoff = 700.0 + 500.0 * lfo.sine(0.07 + voice as f32 * 0.013, rate);
                let raw = a.saw(f * 1.003, rate) + b.saw(f * 0.997, rate);
                let v = filter.process(raw, cutoff, rate).low * envelope(t, span, 3.5, 5.0);
                out[first + i] += v * 0.09;
            }
        }
        // Basse ronde.
        let mut sub = Osc::new();
        let first = (start * rate) as usize;
        for i in 0..(span * rate) as usize {
            let t = i as f32 / rate;
            out[first + i] += sub.sine(midi(*bass), rate) * envelope(t, span, 2.0, 5.0) * 0.28;
        }
        // Notes pincées, une par seconde et demie, motif fixe sur les notes de l'accord.
        let pattern = [2usize, 4, 3, 1, 4, 2, 0, 3, 4, 1];
        for (k, step) in pattern.iter().enumerate() {
            let at = start + 1.0 + k as f32 * 1.5;
            let f = midi(notes[*step] + 12);
            let mut osc = Osc::new();
            let first = (at * rate) as usize;
            for i in 0..(3.0 * rate) as usize {
                let t = i as f32 / rate;
                let v = (osc.sine(f, rate) + 0.3 * (t * f * 2.0 * std::f32::consts::TAU).sin())
                    * decay(t, 2.2);
                if let Some(s) = out.get_mut(first + i) {
                    *s += v * 0.07;
                }
            }
        }
    }
    echo(&mut out, rate, 0.375, 0.35);
    wrap_loop(out, (length * rate) as usize)
}

/// Combat : 120 battements, basse pulsée, batterie sèche, accords tendus.
pub fn music_combat() -> Vec<f32> {
    let rate = LONG_RATE;
    let beat = 0.5;
    let bars = 16;
    let length = bars as f32 * 4.0 * beat;
    let total = ((length + 2.0) * rate) as usize;
    let mut out = vec![0f32; total];
    let mut rng = Rng::new(41);
    let roots = [38, 34, 36, 33]; // ré, sib, do, la
    let stab_chords: [[i32; 3]; 4] = [[62, 65, 69], [58, 62, 65], [60, 64, 67], [57, 61, 64]];
    let add = |out: &mut Vec<f32>, at: f32, seconds: f32, f: &mut dyn FnMut(f32) -> f32| {
        let first = (at * rate) as usize;
        for i in 0..(seconds * rate) as usize {
            if let Some(s) = out.get_mut(first + i) {
                *s += f(i as f32 / rate);
            }
        }
    };
    for bar in 0..bars {
        let section = (bar / 4) % 4;
        let bar_start = bar as f32 * 4.0 * beat;
        // Basse en doubles croches, filtrée.
        for sixteenth in 0..16 {
            let at = bar_start + sixteenth as f32 * beat / 4.0;
            let up = if sixteenth % 8 == 6 { 12 } else { 0 };
            let f = midi(roots[section] + up);
            let mut osc = Osc::new();
            let mut filter = Svf::new(0.8);
            add(&mut out, at, beat / 4.0, &mut |t| {
                filter
                    .process(osc.saw(f, rate), 200.0 + 1200.0 * decay(t, 30.0), rate)
                    .low
                    * envelope(t, beat / 4.0, 0.003, 0.03)
                    * 0.35
            });
        }
        for b in 0..4 {
            let at = bar_start + b as f32 * beat;
            // Grosse caisse.
            let mut kick = Osc::new();
            add(&mut out, at, 0.3, &mut |t| {
                kick.sine(45.0 + 75.0 * (-t * 30.0).exp(), rate) * decay(t, 12.0) * 0.8
            });
            // Caisse claire sur 2 et 4.
            if b % 2 == 1 {
                let mut filter = Svf::new(0.9);
                let mut body = Osc::new();
                let mut noise: Vec<f32> =
                    (0..(0.25 * rate) as usize).map(|_| rng.noise()).collect();
                let mut n = noise.drain(..);
                add(&mut out, at, 0.25, &mut |t| {
                    let x = n.next().unwrap_or(0.0);
                    filter.process(x, 1800.0, rate).band * decay(t, 18.0) * 0.9
                        + body.sine(185.0, rate) * decay(t, 25.0) * 0.3
                });
            }
            // Charleston en croches.
            for half in 0..2 {
                let at = at + half as f32 * beat / 2.0;
                let mut filter = Svf::new(1.0);
                let mut hiss: Vec<f32> = (0..(0.06 * rate) as usize).map(|_| rng.noise()).collect();
                let mut h = hiss.drain(..);
                add(&mut out, at, 0.06, &mut |t| {
                    filter.process(h.next().unwrap_or(0.0), 5000.0, rate).high
                        * decay(t, 70.0)
                        * 0.25
                });
            }
        }
        // Accord sec à contretemps, une mesure sur deux.
        if bar % 2 == 1 {
            for note in stab_chords[section] {
                let f = midi(note);
                let (mut a, mut b) = (Osc::new(), Osc::new());
                let mut filter = Svf::new(1.0);
                add(&mut out, bar_start + 1.5 * beat, 0.6, &mut |t| {
                    filter
                        .process(
                            a.saw(f, rate) + b.saw(f * 1.006, rate),
                            600.0 + 2400.0 * decay(t, 8.0),
                            rate,
                        )
                        .low
                        * envelope(t, 0.6, 0.005, 0.3)
                        * 0.12
                });
            }
        }
    }
    wrap_loop(out, (length * rate) as usize)
}
