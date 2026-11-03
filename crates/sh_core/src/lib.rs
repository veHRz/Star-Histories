//! Types purs partagés de Star Histories.
//!
//! Aucune dépendance à Bevy : ces types servent aux données (`sh_data`), à la simulation
//! (`sh_sim`) et au jeu (`sh_game`). Tout ce qui est ici doit rester déterministe et testable
//! sans fenêtre.

pub mod date;
pub mod grid;
pub mod id;
pub mod money;
pub mod rng;

pub use date::{DAYS_PER_YEAR, StellarDate, Year};
pub use grid::{GridCell, MapPos};
pub use id::{Id, IdError};
pub use money::{Credits, ExchangeRate, Wuro};
pub use rng::SimRng;
