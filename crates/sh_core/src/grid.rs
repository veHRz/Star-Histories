//! Repère de la carte canonique et cases de grille.
//!
//! Canon (§3.1 de l'export) : repère de 1000 × 660 unités, nord en haut (y petit), est à droite ;
//! grille de 20 colonnes A → T et 13 lignes 1 → 13, cases de 50 × 50 unités ;
//! colonne = lettre(⌊x / 50⌋), ligne = ⌊y / 50⌋ + 1. Toute position canon est une case.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Largeur du repère de la carte, en unités de carte.
pub const MAP_WIDTH: f64 = 1000.0;
/// Hauteur du repère de la carte, en unités de carte.
pub const MAP_HEIGHT: f64 = 660.0;
/// Côté d'une case de grille, en unités de carte.
pub const CELL_SIZE: f64 = 50.0;
/// Nombre de colonnes (A → T).
pub const COLUMNS: u8 = 20;
/// Nombre de lignes (1 → 13).
pub const ROWS: u8 = 13;

/// Une position sur la carte canonique, en unités de carte.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapPos {
    pub x: f64,
    pub y: f64,
}

impl MapPos {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Distance euclidienne en unités de carte.
    pub fn distance(self, other: MapPos) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }

    /// La case qui contient cette position, si elle est dans le repère.
    pub fn cell(self) -> Option<GridCell> {
        GridCell::containing(self)
    }
}

/// Une case de la grille de repérage, affichée « K7 ».
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GridCell {
    /// Colonne, 0 = A … 19 = T.
    col: u8,
    /// Ligne, 1 … 13.
    row: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("case de grille invalide : « {0} » (attendu une lettre A → T suivie d'un nombre 1 → 13)")]
pub struct CellParseError(pub String);

impl GridCell {
    /// Construit une case à partir de son indice de colonne (0 = A) et de sa ligne (1 → 13).
    pub fn new(col: u8, row: u8) -> Option<Self> {
        (col < COLUMNS && (1..=ROWS).contains(&row)).then_some(Self { col, row })
    }

    /// La case qui contient une position (formule canon §3.1). `None` hors du repère.
    pub fn containing(pos: MapPos) -> Option<Self> {
        if !(0.0..MAP_WIDTH).contains(&pos.x) || !(0.0..MAP_HEIGHT).contains(&pos.y) {
            return None;
        }
        // Les bornes viennent d'être vérifiées : les conversions ne débordent pas.
        let col = (pos.x / CELL_SIZE).floor() as u8;
        let row = (pos.y / CELL_SIZE).floor() as u8 + 1;
        Self::new(col, row)
    }

    pub fn column_letter(self) -> char {
        char::from(b'A' + self.col)
    }

    pub fn column_index(self) -> u8 {
        self.col
    }

    pub fn row(self) -> u8 {
        self.row
    }

    /// Centre de la case, en unités de carte.
    pub fn center(self) -> MapPos {
        MapPos::new(
            (f64::from(self.col) + 0.5) * CELL_SIZE,
            (f64::from(self.row) - 0.5) * CELL_SIZE,
        )
    }
}

impl fmt::Display for GridCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.column_letter(), self.row)
    }
}

impl FromStr for GridCell {
    type Err = CellParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || CellParseError(s.to_owned());
        let trimmed = s.trim();
        let mut chars = trimmed.chars();
        let letter = chars.next().ok_or_else(err)?.to_ascii_uppercase();
        if !letter.is_ascii_uppercase() {
            return Err(err());
        }
        let col = letter as u8 - b'A';
        let row: u8 = chars.as_str().parse().map_err(|_| err())?;
        Self::new(col, row).ok_or_else(err)
    }
}

impl TryFrom<String> for GridCell {
    type Error = CellParseError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<GridCell> for String {
    fn from(cell: GridCell) -> Self {
        cell.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases_canon_depuis_les_coordonnees() {
        // Valeurs du tableau §3.9 de l'export.
        let cas = [
            (515.5, 317.6, "K7"), // Concordia
            (766.6, 322.6, "P7"), // Vorrh
            (639.5, 330.0, "M7"), // Hexis
            (593.0, 394.5, "L8"), // Cygnara
            (425.0, 475.1, "I10"),
            (755.1, 491.9, "P10"), // Terminus
            (500.0, 330.0, "K7"),  // noyau : à la jonction J7/K7, la formule donne K7
        ];
        for (x, y, attendu) in cas {
            let cell = MapPos::new(x, y).cell().unwrap();
            assert_eq!(cell.to_string(), attendu, "({x}, {y})");
        }
    }

    #[test]
    fn hors_du_repere() {
        assert_eq!(MapPos::new(-0.1, 10.0).cell(), None);
        assert_eq!(MapPos::new(1000.0, 10.0).cell(), None);
        assert_eq!(MapPos::new(10.0, 660.0).cell(), None);
        assert_eq!(MapPos::new(0.0, 0.0).cell().unwrap().to_string(), "A1");
        assert_eq!(MapPos::new(999.9, 649.9).cell().unwrap().to_string(), "T13");
    }

    #[test]
    fn lecture_des_cases() {
        assert_eq!(
            "K7".parse::<GridCell>().unwrap(),
            GridCell::new(10, 7).unwrap()
        );
        assert_eq!("t13".parse::<GridCell>().unwrap().to_string(), "T13");
        for faux in ["", "K", "U1", "A0", "A14", "7K", "K7b", "É3"] {
            assert!(faux.parse::<GridCell>().is_err(), "{faux}");
        }
    }

    #[test]
    fn centre_de_case() {
        let k7: GridCell = "K7".parse().unwrap();
        assert_eq!(k7.center(), MapPos::new(525.0, 325.0));
        assert_eq!(k7.center().cell(), Some(k7));
    }

    #[test]
    fn serde_ron() {
        let cell: GridCell = ron::from_str("\"M7\"").unwrap();
        assert_eq!(cell.to_string(), "M7");
        assert!(ron::from_str::<GridCell>("\"Z9\"").is_err());
    }
}
