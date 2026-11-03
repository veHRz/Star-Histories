//! Calendrier stellaire.
//!
//! Canon (§2.1 de l'export) : on compte en Années Stellaires, l'an 0 est le Triomphe ;
//! `Xbt` = X ans avant, `Xat` = X ans après. Le découpage de l'année est une décision de
//! l'auteur (`docs/QUESTIONS.md` Q3, proposition P10 validée) : 365 jours numérotés, sans mois.
//!
//! La notation `425at` / `50bt` ne dépend pas de la langue : elle est formatée ici. Le reste
//! de l'affichage (« Day 12, 425at ») appartient à l'interface et à ses fichiers de langue.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Nombre de jours d'une année stellaire (P10).
pub const DAYS_PER_YEAR: u16 = 365;

/// Une année stellaire. `Year(0)` est l'an du Triomphe ; les années `bt` sont négatives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Year(pub i32);

impl Year {
    /// L'année où commence le jeu (§0 de l'export : « le présent de l'univers est 425at »).
    pub const PRESENT: Year = Year(425);
}

impl fmt::Display for Year {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 0 {
            write!(f, "{}bt", -i64::from(self.0))
        } else {
            write!(f, "{}at", self.0)
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("année stellaire invalide : « {0} » (attendu par exemple « 425at » ou « 50bt »)")]
pub struct YearParseError(pub String);

impl FromStr for Year {
    type Err = YearParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let err = || YearParseError(s.to_owned());
        let (digits, sign) = if let Some(d) = trimmed.strip_suffix("at") {
            (d, 1)
        } else if let Some(d) = trimmed.strip_suffix("bt") {
            (d, -1)
        } else {
            return Err(err());
        };
        let value: i32 = digits.trim().parse().map_err(|_| err())?;
        if value < 0 || (sign < 0 && value == 0) {
            return Err(err());
        }
        Ok(Year(sign * value))
    }
}

impl TryFrom<String> for Year {
    type Error = YearParseError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Year> for String {
    fn from(year: Year) -> Self {
        year.to_string()
    }
}

/// Un jour précis du calendrier stellaire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StellarDate {
    year: Year,
    /// Jour de l'année, de 1 à [`DAYS_PER_YEAR`].
    day: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("jour {day} hors de l'année (1 à {DAYS_PER_YEAR})")]
pub struct DayOutOfRange {
    pub day: u16,
}

impl StellarDate {
    /// Construit une date ; `day` va de 1 à [`DAYS_PER_YEAR`].
    pub fn new(year: Year, day: u16) -> Result<Self, DayOutOfRange> {
        if (1..=DAYS_PER_YEAR).contains(&day) {
            Ok(Self { year, day })
        } else {
            Err(DayOutOfRange { day })
        }
    }

    /// Premier jour d'une année.
    pub fn start_of(year: Year) -> Self {
        Self { year, day: 1 }
    }

    pub fn year(self) -> Year {
        self.year
    }

    pub fn day_of_year(self) -> u16 {
        self.day
    }

    /// Nombre de jours écoulés depuis le premier jour de l'an 0 (négatif avant le Triomphe).
    /// Sert de base à l'arithmétique et aux durées de la simulation.
    pub fn day_index(self) -> i64 {
        i64::from(self.year.0) * i64::from(DAYS_PER_YEAR) + i64::from(self.day) - 1
    }

    /// Inverse de [`StellarDate::day_index`].
    pub fn from_day_index(index: i64) -> Self {
        let per_year = i64::from(DAYS_PER_YEAR);
        let year = index.div_euclid(per_year);
        let day = index.rem_euclid(per_year) + 1;
        // Les bornes sont garanties par div_euclid / rem_euclid ; une année hors d'un i32
        // est très loin de tout usage du jeu, on la sature.
        let year = i32::try_from(year).unwrap_or(if year < 0 { i32::MIN } else { i32::MAX });
        Self {
            year: Year(year),
            day: u16::try_from(day).unwrap_or(1),
        }
    }

    /// La date `days` jours plus tard (ou plus tôt si négatif).
    pub fn add_days(self, days: i64) -> Self {
        Self::from_day_index(self.day_index() + days)
    }

    /// Nombre de jours de `self` à `later` (négatif si `later` est avant).
    pub fn days_until(self, later: StellarDate) -> i64 {
        later.day_index() - self.day_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affichage_des_annees() {
        assert_eq!(Year(425).to_string(), "425at");
        assert_eq!(Year(0).to_string(), "0at");
        assert_eq!(Year(-50).to_string(), "50bt");
        assert_eq!(Year(-600).to_string(), "600bt");
    }

    #[test]
    fn lecture_des_annees() {
        assert_eq!("425at".parse(), Ok(Year(425)));
        assert_eq!(" 53bt ".parse(), Ok(Year(-53)));
        assert_eq!("0at".parse(), Ok(Year(0)));
        assert!("0bt".parse::<Year>().is_err(), "l'an 0 s'écrit 0at");
        assert!("425".parse::<Year>().is_err());
        assert!("-3at".parse::<Year>().is_err());
        assert!("abcat".parse::<Year>().is_err());
    }

    #[test]
    fn aller_retour_texte() {
        for y in [-600, -1, 0, 1, 357, 425] {
            let year = Year(y);
            assert_eq!(year.to_string().parse(), Ok(year));
        }
    }

    #[test]
    fn jours_hors_annee_refuses() {
        assert!(StellarDate::new(Year::PRESENT, 0).is_err());
        assert!(StellarDate::new(Year::PRESENT, 366).is_err());
        assert!(StellarDate::new(Year::PRESENT, 365).is_ok());
    }

    #[test]
    fn arithmetique_des_jours() {
        let d = StellarDate::new(Year(425), 364).unwrap();
        let next = d.add_days(2);
        assert_eq!(next.year(), Year(426));
        assert_eq!(next.day_of_year(), 1);
        assert_eq!(d.days_until(next), 2);
        assert_eq!(next.add_days(-2), d);
    }

    #[test]
    fn dates_avant_le_triomphe() {
        let triomphe = StellarDate::start_of(Year(0));
        assert_eq!(triomphe.day_index(), 0);
        let veille = triomphe.add_days(-1);
        assert_eq!(veille.year(), Year(-1));
        assert_eq!(veille.day_of_year(), DAYS_PER_YEAR);
        assert_eq!(StellarDate::from_day_index(veille.day_index()), veille);
    }

    #[test]
    fn ordre_chronologique() {
        let a = StellarDate::new(Year(-1), 365).unwrap();
        let b = StellarDate::new(Year(0), 1).unwrap();
        let c = StellarDate::new(Year(0), 2).unwrap();
        assert!(a < b && b < c);
    }

    #[test]
    fn serde_ron() {
        let year: Year = ron::from_str("\"425at\"").unwrap();
        assert_eq!(year, Year(425));
        assert_eq!(ron::to_string(&Year(-53)).unwrap(), "\"53bt\"");
    }
}
