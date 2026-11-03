//! Hasard déterministe.
//!
//! La simulation doit rejouer la même histoire à partir de la même graine (AGENTS.md §7.2).
//! [`SimRng`] enveloppe ChaCha8 (rapide, de bonne qualité, reproductible sur toutes les
//! plates-formes) et fournit des sous-flux indépendants : chaque système de la simulation tire
//! dans son propre flux, si bien qu'ajouter un tirage dans l'un ne décale pas les autres.

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Générateur de nombres pseudo-aléatoires déterministe.
#[derive(Clone, Debug)]
pub struct SimRng {
    seed: u64,
    inner: ChaCha8Rng,
}

impl SimRng {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            inner: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Un sous-flux indépendant, identifié par un numéro stable (ex. un hachage du nom du
    /// système qui l'utilise). Le résultat ne dépend que de la graine et de `stream`, jamais
    /// des tirages déjà faits.
    pub fn fork(&self, stream: u64) -> Self {
        let mut inner = ChaCha8Rng::seed_from_u64(self.seed);
        inner.set_stream(stream);
        Self {
            seed: self.seed,
            inner,
        }
    }

    /// Sous-flux nommé : le nom est haché de façon stable (FNV-1a), indépendamment de la
    /// plate-forme et de la version de Rust (contrairement au `Hasher` par défaut).
    pub fn fork_named(&self, name: &str) -> Self {
        self.fork(fnv1a(name.as_bytes()))
    }

    /// Entier uniforme dans `[low, high)`. Renvoie `low` si l'intervalle est vide.
    pub fn range_u32(&mut self, low: u32, high: u32) -> u32 {
        if high <= low {
            low
        } else {
            self.inner.random_range(low..high)
        }
    }

    /// Réel uniforme dans `[0, 1)`.
    pub fn unit_f64(&mut self) -> f64 {
        self.inner.random::<f64>()
    }

    /// Vrai avec la probabilité `p` (bornée à `[0, 1]`).
    pub fn chance(&mut self, p: f64) -> bool {
        self.inner.random_bool(p.clamp(0.0, 1.0))
    }

    /// Un élément au hasard d'une tranche, ou `None` si elle est vide.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            return None;
        }
        let len = u32::try_from(items.len()).unwrap_or(u32::MAX);
        let index = self.range_u32(0, len) as usize;
        items.get(index)
    }

    /// Accès au générateur sous-jacent, pour les distributions de `rand`.
    pub fn inner_mut(&mut self) -> &mut ChaCha8Rng {
        &mut self.inner
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes
        .iter()
        .fold(OFFSET, |hash, &b| (hash ^ u64::from(b)).wrapping_mul(PRIME))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tirages(rng: &mut SimRng) -> Vec<u32> {
        (0..16).map(|_| rng.range_u32(0, 1_000_000)).collect()
    }

    #[test]
    fn meme_graine_meme_suite() {
        let mut a = SimRng::new(425);
        let mut b = SimRng::new(425);
        assert_eq!(tirages(&mut a), tirages(&mut b));
        let mut c = SimRng::new(426);
        assert_ne!(tirages(&mut SimRng::new(425)), tirages(&mut c));
    }

    #[test]
    fn sous_flux_independants_des_tirages_precedents() {
        let racine = SimRng::new(7);
        let mut usee = racine.clone();
        let _ = tirages(&mut usee);
        assert_eq!(
            tirages(&mut racine.fork_named("flottes")),
            tirages(&mut usee.fork_named("flottes"))
        );
        assert_ne!(
            tirages(&mut racine.fork_named("flottes")),
            tirages(&mut racine.fork_named("marches"))
        );
    }

    #[test]
    fn valeurs_stables_entre_versions() {
        // Garde-fou : si ce test casse, une mise à jour de rand ou de rand_chacha a changé les
        // suites, et toutes les sauvegardes et histoires rejouées divergeraient.
        let mut rng = SimRng::new(425);
        let premiers: Vec<u32> = (0..4).map(|_| rng.range_u32(0, 1000)).collect();
        assert_eq!(premiers, [185, 911, 856, 874]);
        let mut flottes = SimRng::new(425).fork_named("flottes");
        let suite: Vec<u32> = (0..2).map(|_| flottes.range_u32(0, 1000)).collect();
        assert_eq!(suite, [456, 400]);
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    fn bornes() {
        let mut rng = SimRng::new(1);
        assert_eq!(rng.range_u32(5, 5), 5);
        assert!(!rng.chance(0.0));
        assert!(rng.chance(1.0));
        assert!(rng.chance(7.0));
        assert_eq!(rng.pick::<u8>(&[]), None);
        for _ in 0..100 {
            let x = rng.unit_f64();
            assert!((0.0..1.0).contains(&x));
        }
    }
}
