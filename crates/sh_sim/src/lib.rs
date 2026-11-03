//! Simulation macro du monde vivant (AGENTS.md §7).
//!
//! Headless, déterministe, testée, sans Bevy. Le jalon M0 n'en pose que le socle : l'horloge
//! du monde et le générateur de hasard. Les factions, flottes, marchés et projets arrivent en M4.

pub mod travel;

use sh_core::{SimRng, StellarDate, Year};

/// L'état global de la simulation.
#[derive(Clone, Debug)]
pub struct SimWorld {
    date: StellarDate,
    rng: SimRng,
}

impl SimWorld {
    /// Un monde au premier jour de `start`, tiré de la graine `seed`.
    pub fn new(seed: u64, start: StellarDate) -> Self {
        Self {
            date: start,
            rng: SimRng::new(seed),
        }
    }

    /// Le monde au début de la partie : premier jour de 425at.
    pub fn at_present(seed: u64) -> Self {
        Self::new(seed, StellarDate::start_of(Year::PRESENT))
    }

    pub fn date(&self) -> StellarDate {
        self.date
    }

    pub fn seed(&self) -> u64 {
        self.rng.seed()
    }

    /// Avance d'un jour (le pas de la simulation macro).
    pub fn step_day(&mut self) {
        self.date = self.date.add_days(1);
    }

    /// Avance de `days` jours.
    pub fn advance_days(&mut self, days: u32) {
        for _ in 0..days {
            self.step_day();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_temps_passe() {
        let mut world = SimWorld::at_present(1);
        assert_eq!(world.date().year(), Year(425));
        world.advance_days(365);
        assert_eq!(world.date(), StellarDate::start_of(Year(426)));
    }
}
