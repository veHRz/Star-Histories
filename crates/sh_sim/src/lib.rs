//! Simulation macro du monde vivant (AGENTS.md §7).
//!
//! Headless, déterministe, testée, sans Bevy. [`SimWorld`] tient l'état de la galaxie (flottes,
//! économies, stabilité, projets) et avance d'un jour à la fois ; il produit des [`SimEvent`],
//! publics (nouvelles, connaissance du joueur) ou secrets (ils pilotent sans être montrés).
//! Les rails du canon (`canon/rails.ron`) tiennent la trajectoire connue ; les interdits durs
//! n'ont pas d'événement. [`travel`] chiffre les voyages ; [`career`] tient le capitaine du joueur.

pub mod career;
pub mod events;
pub mod travel;
pub mod world;

pub use career::{
    Captain, CareerError, Contract, ContractKind, ContractState, Loan, Outcome, contract_offers,
};
pub use events::{EventKind, SimEvent};
pub use world::{FleetState, Mission, SimCommand, SimWorld};

#[cfg(test)]
mod tests {
    use sh_core::{StellarDate, Year};

    use super::*;

    #[test]
    fn le_temps_passe() {
        let mut world = SimWorld::at_present(1);
        assert_eq!(world.date().year(), Year(425));
        world.advance_days(365);
        assert_eq!(world.date(), StellarDate::start_of(Year(426)));
    }
}
