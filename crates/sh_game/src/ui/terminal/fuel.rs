//! Le ravitaillement en carburant lourd au chantier (M9) : ce qu'un plein ou une tranche fait
//! entrer dans le réservoir et ce qu'il coûte. Logique pure, testée.
//!
//! Sans lui, chaque saut hyperspatial entamait pour de bon la réserve de départ. La réserve vit
//! dans `voyage::ShipFuel` (unités de carburant) ; la carte et les données de voyage la comptent
//! en jours (`travel.ron`, `fuel_per_day` : unités consommées par jour de route). Le prix d'un
//! jour et la tranche viennent de `combat.ron` (`refuel`, à côté du prix des réparations).

use sh_core::Credits;

use crate::data::Data;

/// Un ravitaillement chiffré : unités ajoutées et prix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Refuel {
    pub units: f64,
    pub cost: Credits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefuelError {
    /// Le réservoir est déjà plein.
    TankFull,
    /// Le compte ne couvre pas le prix : le chantier refuse (pas de crédit sur le carburant).
    NotEnoughCredits,
}

/// Le prix d'une unité : celui d'un jour de carburant, divisé par la consommation d'un jour.
pub fn unit_price(day_price: f64, fuel_per_day: f64) -> f64 {
    if fuel_per_day > 0.0 {
        day_price / fuel_per_day
    } else {
        day_price
    }
}

/// Le devis : `wanted` unités (aucune : le plein), plafonnées à la place libre du réservoir.
pub fn quote(have: f64, capacity: f64, wanted: Option<f64>, unit_price: f64) -> Option<Refuel> {
    let room = (capacity - have).max(0.0);
    let units = wanted.map_or(room, |w| w.clamp(0.0, room));
    // En dessous d'un centième d'unité, le réservoir est plein (arrondis des voyages).
    (units >= 0.01).then(|| Refuel {
        units,
        cost: Credits::from_cents((units * unit_price.max(0.0) * 100.0).round() as i64),
    })
}

/// Le ravitaillement, s'il est possible : de la place dans le réservoir, et de quoi payer.
pub fn refuel(
    have: f64,
    capacity: f64,
    wanted: Option<f64>,
    unit_price: f64,
    credits: Credits,
) -> Result<Refuel, RefuelError> {
    let refuel = quote(have, capacity, wanted, unit_price).ok_or(RefuelError::TankFull)?;
    if credits.cents < refuel.cost.cents {
        return Err(RefuelError::NotEnoughCredits);
    }
    Ok(refuel)
}

/// La consommation d'un jour du vaisseau du joueur (1 si les données ne la donnent pas).
pub fn fuel_per_day(data: &Data) -> f64 {
    data.start()
        .and_then(|s| data.travel.ships.get(&s.ship))
        .map_or(1.0, |s| s.fuel_per_day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_plein_comble_la_place_libre() {
        let plein = quote(20.0, 60.0, None, 30.0);
        assert_eq!(
            plein,
            Some(Refuel {
                units: 40.0,
                cost: Credits::units(1200)
            })
        );
        assert_eq!(quote(60.0, 60.0, None, 30.0), None);
    }

    #[test]
    fn une_tranche_ne_deborde_pas_du_reservoir() {
        let tranche = quote(20.0, 60.0, Some(10.0), 30.0);
        assert_eq!(tranche.map(|r| r.units), Some(10.0));
        assert_eq!(tranche.map(|r| r.cost), Some(Credits::units(300)));
        let reste = quote(55.0, 60.0, Some(10.0), 30.0);
        assert_eq!(reste.map(|r| r.units), Some(5.0));
        assert_eq!(reste.map(|r| r.cost), Some(Credits::units(150)));
    }

    #[test]
    fn refus_si_plein_ou_sans_argent() {
        assert_eq!(
            refuel(60.0, 60.0, None, 30.0, Credits::units(5000)),
            Err(RefuelError::TankFull)
        );
        assert_eq!(
            refuel(0.0, 60.0, None, 30.0, Credits::units(1799)),
            Err(RefuelError::NotEnoughCredits)
        );
        assert_eq!(
            refuel(0.0, 60.0, None, 30.0, Credits::units(1800)).map(|r| r.units),
            Ok(60.0)
        );
    }

    #[test]
    fn prix_d_une_unite() {
        assert!((unit_price(30.0, 2.0) - 15.0).abs() < 1e-9);
        assert!((unit_price(30.0, 0.0) - 30.0).abs() < 1e-9);
    }
}
