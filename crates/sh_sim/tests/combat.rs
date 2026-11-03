//! Le combat sur les vraies données (M6) : coques du joueur et des pillards, raids issus de la
//! simulation dans le système Cygnara.

use std::path::Path;

use sh_core::Id;
use sh_data::{GameData, Place};
use sh_sim::combat::{Hull, raid_on_arrival};
use sh_sim::{SimCommand, SimWorld};

fn donnees() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
    GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"))
}

fn lieu(id: &str) -> Id<Place> {
    Id::new(id).expect("identifiant")
}

#[test]
fn la_ligue_tend_des_embuscades_a_cygnara_pas_a_hexis() {
    let data = donnees();
    let sim = SimWorld::from_data(&data, Some(1));
    let day = sim.date().day_index();
    let raid = raid_on_arrival(&data, &sim, &lieu("sys_cygnara"), day, true).expect("raid");
    assert_eq!(raid.raiders.as_str(), "meutes_ligue");
    assert!(!raid.attackers.is_empty());
    let (fleet, defenders) = raid.defenders.expect("rescousse");
    assert_eq!(fleet.as_str(), "flotte_defense_cygnara");
    // Avec le joueur, dix coques au plus : le budget de performance.
    assert!(raid.attackers.len() + defenders.len() < 10);
    assert!(raid_on_arrival(&data, &sim, &lieu("sys_hexis"), day, true).is_none());
}

#[test]
fn les_raids_sont_deterministes_et_ni_rares_ni_systematiques() {
    let data = donnees();
    let sim = SimWorld::from_data(&data, Some(4));
    let cygnara = lieu("sys_cygnara");
    let day = sim.date().day_index();
    let tirages: Vec<bool> = (0..200)
        .map(|d| raid_on_arrival(&data, &sim, &cygnara, day + d, false).is_some())
        .collect();
    let again: Vec<bool> = (0..200)
        .map(|d| raid_on_arrival(&data, &sim, &cygnara, day + d, false).is_some())
        .collect();
    assert_eq!(tirages, again);
    let n = tirages.iter().filter(|&&r| r).count();
    assert!((40..160).contains(&n), "{n} raids sur 200 arrivées");
}

#[test]
fn une_flotte_epuisee_ne_chasse_plus() {
    let data = donnees();
    let mut sim = SimWorld::from_data(&data, Some(1));
    let fleet = Id::new("meutes_ligue").expect("identifiant");
    sim.apply(&SimCommand::FleetLosses {
        fleet,
        condition: 1.0,
    });
    let day = sim.date().day_index();
    assert!(raid_on_arrival(&data, &sim, &lieu("sys_cygnara"), day, true).is_none());
}

#[test]
fn les_coques_du_raid_sont_jouables() {
    let data = donnees();
    for key in ["player_ship", "pirate_corvette", "ugl_corvette"] {
        let def = data.combat.ship(key).expect("coque");
        let hull = Hull::new(def, data.combat.hull);
        assert_eq!(hull.integrity(), 1.0);
        assert_eq!(hull.damage_points(), 0.0);
    }
}

#[test]
fn les_degats_restent_jusqu_a_la_reparation_payee() {
    let data = donnees();
    let mut cap =
        sh_sim::Captain::from_start(&data, sh_core::StellarDate::start_of(sh_core::Year(425)));
    assert_eq!(
        cap.repair_ship(&data),
        Err(sh_sim::CareerError::NothingToRepair)
    );
    let hull = cap.hull.as_mut().expect("coque");
    hull.hit(sh_data::SectionId::Stern, 80.0, 60.0);
    let cost = hull.repair_cost(&data.combat.repair);
    assert!(cost.cents > 0);
    let before = cap.credits;
    assert_eq!(cap.repair_ship(&data), Ok(cost));
    assert_eq!(cap.credits, before - cost);
    assert_eq!(cap.hull.as_ref().map(|h| h.integrity()), Some(1.0));
}
