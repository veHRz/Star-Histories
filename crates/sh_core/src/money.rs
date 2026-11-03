//! Monnaies : le crédit et le Wuro.
//!
//! Canon (§2.8 de l'export) : le crédit est la monnaie universelle de fait ; le Wuro est la
//! monnaie d'État impériale, interdite dans l'UGL ; le « taux d'Hexis » sert de change
//! officieux. Le canon ne fixe aucune valeur (vide connu §11.1) : les taux sont des données
//! d'équilibrage, jamais du code.
//!
//! Les montants sont des entiers en centièmes, pour que les calculs restent exacts et
//! déterministes. Les deux monnaies sont des types distincts : additionner des crédits et des
//! Wuros ne compile pas.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

macro_rules! currency {
    ($(#[$doc:meta])* $name:ident, $symbol:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name {
            /// Montant en centièmes d'unité.
            pub cents: i64,
        }

        impl $name {
            pub const ZERO: Self = Self { cents: 0 };

            /// Montant en unités entières.
            pub const fn units(units: i64) -> Self {
                Self { cents: units * 100 }
            }

            pub const fn from_cents(cents: i64) -> Self {
                Self { cents }
            }

            /// Montant en unités, pour l'affichage ou les ratios (jamais pour la comptabilité).
            pub fn as_f64(self) -> f64 {
                self.cents as f64 / 100.0
            }

            pub fn is_negative(self) -> bool {
                self.cents < 0
            }

            pub fn checked_sub(self, other: Self) -> Option<Self> {
                self.cents.checked_sub(other.cents).map(Self::from_cents)
            }
        }

        impl Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self { cents: self.cents + rhs.cents }
            }
        }

        impl AddAssign for $name {
            fn add_assign(&mut self, rhs: Self) {
                self.cents += rhs.cents;
            }
        }

        impl Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self { cents: self.cents - rhs.cents }
            }
        }

        impl SubAssign for $name {
            fn sub_assign(&mut self, rhs: Self) {
                self.cents -= rhs.cents;
            }
        }

        impl Neg for $name {
            type Output = Self;
            fn neg(self) -> Self {
                Self { cents: -self.cents }
            }
        }

        impl Sum for $name {
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold(Self::ZERO, Add::add)
            }
        }

        /// Format technique (journaux, débogage). L'interface formate selon la langue.
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let sign = if self.cents < 0 { "-" } else { "" };
                let abs = self.cents.unsigned_abs();
                write!(f, "{sign}{}.{:02} {}", abs / 100, abs % 100, $symbol)
            }
        }
    };
}

currency!(
    /// Le crédit (ex-crédit impérial), monnaie universelle de fait.
    Credits,
    "cr"
);
currency!(
    /// Le Wuro, monnaie d'État de l'Empire depuis la Renaissance.
    Wuro,
    "W"
);

/// Taux de change crédit/Wuro : combien de crédits vaut un Wuro.
///
/// Stocké en millièmes de crédit par Wuro pour rester entier. Le « taux d'Hexis » du jeu est
/// une valeur de données qui varie avec la simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExchangeRate {
    /// Crédits pour 1 Wuro, en millièmes.
    pub credits_per_wuro_milli: i64,
}

impl ExchangeRate {
    /// Taux à partir d'un nombre de crédits pour un Wuro (ex. 1.25).
    pub fn from_credits_per_wuro(rate: f64) -> Self {
        Self {
            credits_per_wuro_milli: (rate * 1000.0).round() as i64,
        }
    }

    pub fn credits_per_wuro(self) -> f64 {
        self.credits_per_wuro_milli as f64 / 1000.0
    }

    /// Convertit des Wuros en crédits, arrondi au centième le plus proche (demi vers zéro exclu).
    pub fn to_credits(self, wuro: Wuro) -> Credits {
        Credits::from_cents(div_round(
            i128::from(wuro.cents) * i128::from(self.credits_per_wuro_milli),
            1000,
        ))
    }

    /// Convertit des crédits en Wuros, arrondi au centième le plus proche.
    /// Un taux nul ou négatif (donnée invalide) donne zéro.
    pub fn to_wuro(self, credits: Credits) -> Wuro {
        if self.credits_per_wuro_milli <= 0 {
            return Wuro::ZERO;
        }
        Wuro::from_cents(div_round(
            i128::from(credits.cents) * 1000,
            i128::from(self.credits_per_wuro_milli),
        ))
    }
}

/// Division entière arrondie au plus proche (les demis s'éloignent de zéro), saturée en i64.
fn div_round(num: i128, den: i128) -> i64 {
    let q = num / den;
    let r = num % den;
    let q = if 2 * r.abs() >= den.abs() {
        q + num.signum() * den.signum()
    } else {
        q
    };
    i64::try_from(q).unwrap_or(if q < 0 { i64::MIN } else { i64::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetique() {
        let a = Credits::units(10);
        let b = Credits::from_cents(250);
        assert_eq!((a - b).cents, 750);
        assert_eq!([a, b, b].into_iter().sum::<Credits>().cents, 1500);
        assert!((b - a).is_negative());
        assert_eq!(
            Credits::from_cents(i64::MIN).checked_sub(Credits::units(1)),
            None
        );
    }

    #[test]
    fn affichage_technique() {
        assert_eq!(Credits::from_cents(123_456).to_string(), "1234.56 cr");
        assert_eq!(Wuro::from_cents(-5).to_string(), "-0.05 W");
    }

    #[test]
    fn change() {
        let taux = ExchangeRate::from_credits_per_wuro(1.25);
        assert_eq!(taux.to_credits(Wuro::units(4)), Credits::units(5));
        assert_eq!(taux.to_wuro(Credits::units(5)), Wuro::units(4));
        // Arrondi au centième : 1 cr / 1,25 = 0,80 W ; 0,01 W × 1,25 = 0,0125 cr → 0,01 cr.
        assert_eq!(taux.to_wuro(Credits::units(1)), Wuro::from_cents(80));
        assert_eq!(taux.to_credits(Wuro::from_cents(1)), Credits::from_cents(1));
        assert_eq!(
            ExchangeRate::from_credits_per_wuro(0.0).to_wuro(Credits::units(3)),
            Wuro::ZERO
        );
    }

    #[test]
    fn arrondi() {
        assert_eq!(div_round(5, 10), 1);
        assert_eq!(div_round(4, 10), 0);
        assert_eq!(div_round(-5, 10), -1);
        assert_eq!(div_round(-4, 10), 0);
    }
}
