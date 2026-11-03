//! Le cœur RPG sur les vraies données (critères de M5) : prendre un contrat de transport à
//! Hexis, le livrer à Cygnara, être payé, voir sa réputation changer, rembourser une échéance ;
//! la trahison d'un contrat a des conséquences.

use std::path::Path;

use sh_core::{Credits, Id, StellarDate, Year};
use sh_data::GameData;
use sh_sim::{
    Captain, CareerError, ContractKind, ContractState, Outcome, SimWorld, contract_offers,
};

fn donnees() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
    GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"))
}

fn debut() -> StellarDate {
    StellarDate::start_of(Year(425))
}

fn lieu(id: &str) -> Id<sh_data::Place> {
    Id::new(id).expect("identifiant")
}

fn fac(id: &str) -> Id<sh_data::Faction> {
    Id::new(id).expect("identifiant")
}

fn hexis() -> Id<sh_data::Place> {
    lieu("sys_hexis")
}

fn cygnara() -> Id<sh_data::Place> {
    lieu("sys_cygnara")
}

#[test]
fn le_capitaine_commence_endette() {
    let data = donnees();
    let cap = Captain::from_start(&data, debut());
    assert_eq!(cap.credits, Credits::units(3000));
    let loan = cap.loan.as_ref().expect("dette de départ");
    assert_eq!(loan.balance, Credits::units(45_000));
    assert_eq!(loan.next_due, debut().add_days(30));
    assert_eq!(cap.reputation(&fac("hc_finance")), -5);
    assert_eq!(cap.tier(&data.career, &fac("hexa_corp")), "rep.neutral");
}

#[test]
fn hexis_propose_un_transport_vers_cygnara() {
    let data = donnees();
    let sim = SimWorld::from_data(&data, Some(1));
    let offers = contract_offers(&data, &sim, &hexis(), sim.date());
    assert!(!offers.is_empty());
    assert!(offers.len() <= data.career.contracts.offers);
    let ct = offers
        .iter()
        .find(|c| c.to == cygnara())
        .expect("une offre vers Cygnara");
    assert!(matches!(
        ct.kind,
        ContractKind::Transport | ContractKind::Resupply
    ));
    assert!(ct.reward.cents > 0 && ct.amount > 0.0);
    assert!(ct.deadline > sim.date());
    // Déterministe : même jour, mêmes offres.
    assert_eq!(offers, contract_offers(&data, &sim, &hexis(), sim.date()));
    // Aucune offre vers un lieu secret.
    for o in &offers {
        assert!(data.galaxy.system(&o.to).is_none_or(|s| !s.secret));
    }
}

#[test]
fn livrer_a_cygnara_paie_et_fait_monter_la_reputation() {
    let data = donnees();
    let sim = SimWorld::from_data(&data, Some(1));
    let today = sim.date();
    let mut cap = Captain::from_start(&data, today);
    let ct = contract_offers(&data, &sim, &hexis(), today)
        .into_iter()
        .find(|c| c.to == cygnara())
        .expect("offre");
    let reward = ct.reward;
    let id = ct.id.clone();
    cap.accept(&data, ct.clone()).expect("accepté");
    assert_eq!(cap.accept(&data, ct), Err(CareerError::AlreadyTaken));
    assert_eq!(cap.ranks.get(&fac("hexa_corp")), Some(&0), "contractant");
    // Pas à destination : refusé.
    assert_eq!(
        cap.deliver(&data, &id, &hexis(), today.add_days(1)),
        Err(CareerError::WrongPlace)
    );
    let before = cap.credits;
    let out = cap
        .deliver(&data, &id, &cygnara(), today.add_days(1))
        .expect("livré");
    assert_eq!(cap.credits, before + reward);
    assert!(out.contains(&Outcome::ContractPaid {
        id: id.clone(),
        reward
    }));
    assert_eq!(cap.reputation(&fac("hc_logistique")), 6);
    assert_eq!(
        cap.reputation(&fac("hexa_corp")),
        3,
        "la maison mère en reçoit la moitié"
    );
    assert!(cap.skills["commerce"] > 0.0);
    // L'échéance de la Banque.
    let balance = cap.loan.as_ref().map(|l| l.balance).expect("dette");
    cap.pay_installment(&data).expect("payée");
    assert_eq!(
        cap.loan.as_ref().map(|l| l.balance),
        Some(balance - Credits::units(2500))
    );
    assert_eq!(cap.reputation(&fac("hc_finance")), -3);
}

#[test]
fn un_contrat_en_retard_echoue() {
    let data = donnees();
    let sim = SimWorld::from_data(&data, Some(1));
    let today = sim.date();
    let mut cap = Captain::from_start(&data, today);
    let ct = contract_offers(&data, &sim, &hexis(), today)
        .into_iter()
        .find(|c| c.to == cygnara())
        .expect("offre");
    let (id, deadline) = (ct.id.clone(), ct.deadline);
    cap.accept(&data, ct).expect("accepté");
    let out = cap.tick(&data, deadline.add_days(1));
    assert!(out.contains(&Outcome::ContractFailed { id: id.clone() }));
    assert_eq!(cap.contracts[0].state, ContractState::Failed);
    assert_eq!(cap.reputation(&fac("hc_logistique")), -10);
    assert_eq!(
        cap.deliver(&data, &id, &cygnara(), deadline),
        Err(CareerError::NoSuchContract)
    );
}

#[test]
fn trahir_un_contrat_ferme_hexa_corp() {
    let data = donnees();
    let sim = SimWorld::from_data(&data, Some(1));
    let today = sim.date();
    let mut cap = Captain::from_start(&data, today);
    let offers = contract_offers(&data, &sim, &hexis(), today);
    let ct = offers
        .iter()
        .find(|c| c.to == cygnara())
        .cloned()
        .expect("offre");
    let id = ct.id.clone();
    cap.accept(&data, ct).expect("accepté");
    let debt = cap.loan.as_ref().map(|l| l.balance).expect("dette");
    let sale = Credits::units(4000);
    let out = cap.betray(&data, &id, sale, today).expect("trahi");
    assert!(out.contains(&Outcome::ContractBetrayed {
        id: id.clone(),
        indemnity: Credits::units(8000)
    }));
    assert_eq!(
        cap.loan.as_ref().map(|l| l.balance),
        Some(debt + Credits::units(8000))
    );
    assert!(cap.traits.contains(sh_sim::career::NOTORIOUS_TRAITOR));
    assert!(cap.expelled.contains(&fac("hexa_corp")));
    assert!(!cap.ranks.contains_key(&fac("hexa_corp")));
    assert!(cap.knows("hexa_corp", "betrayed"));
    // Plus aucun contrat de la division, ni d'aucune autre division d'Hexa-Corp.
    let other = offers
        .into_iter()
        .find(|c| c.id != id)
        .expect("autre offre");
    assert_eq!(cap.accept(&data, other), Err(CareerError::Refused));
    assert_eq!(cap.tier(&data.career, &fac("hc_logistique")), "rep.hostile");
}

#[test]
fn trois_echeances_manquees_et_la_banque_saisit() {
    let data = donnees();
    let today = debut();
    let mut cap = Captain::from_start(&data, today);
    let mut seized = None;
    for month in 1..=3 {
        for o in cap.tick(&data, today.add_days(30 * month)) {
            if let Outcome::Seized { amount } = o {
                seized = Some(amount);
            }
        }
    }
    assert_eq!(seized, Some(Credits::units(3000)), "tout le compte");
    assert_eq!(cap.credits, Credits::ZERO);
    assert!(cap.traits.contains(sh_sim::career::BAD_PAYER));
    assert!(cap.reputation(&fac("hc_finance")) < -5);
}

#[test]
fn commerce_et_change() {
    let data = donnees();
    let mut cap = Captain::from_start(&data, debut());
    assert_eq!(cap.buy("parts", 10.0, 50.0), Ok(Credits::units(500)));
    assert_eq!(cap.credits, Credits::units(2500));
    assert_eq!(cap.buy("parts", 1000.0, 1.0), Err(CareerError::CargoFull));
    assert_eq!(
        cap.buy("ore", 100.0, 100.0),
        Err(CareerError::NotEnoughCredits)
    );
    assert_eq!(
        cap.sell("parts", 11.0, 60.0),
        Err(CareerError::NotEnoughGoods)
    );
    assert_eq!(cap.sell("parts", 10.0, 60.0), Ok(Credits::units(600)));
    assert!(cap.cargo.is_empty());
    let w = cap
        .exchange_to_wuro(&data.career, Credits::units(250))
        .expect("change");
    assert_eq!(w, sh_core::Wuro::units(100));
    assert_eq!(cap.credits, Credits::units(2850));
}

#[test]
fn les_competences_montent_par_l_usage() {
    let data = donnees();
    let mut cap = Captain::from_start(&data, debut());
    let mut out = Vec::new();
    cap.practice(&data.career, "piloting", 99.0, &mut out);
    assert!(out.is_empty());
    cap.practice(&data.career, "piloting", 1.0, &mut out);
    assert_eq!(
        out,
        vec![Outcome::SkillUp {
            skill: "piloting".into(),
            level: 1
        }]
    );
    // Niveau n à 100 × n² : le niveau 2 demande 400.
    cap.practice(&data.career, "piloting", 299.0, &mut out);
    assert_eq!(cap.skill_level(&data.career, "piloting"), 1);
}

#[test]
fn trois_contrats_tenus_donnent_parole_tenue_et_une_promotion() {
    let data = donnees();
    let mut sim = SimWorld::from_data(&data, Some(3));
    let mut cap = Captain::from_start(&data, sim.date());
    let mut promoted = false;
    for _ in 0..6 {
        let today = sim.date();
        let ct = contract_offers(&data, &sim, &hexis(), today)
            .into_iter()
            .find(|c| c.kind != ContractKind::Towing)
            .expect("offre");
        let (id, to) = (ct.id.clone(), ct.to.clone());
        cap.accept(&data, ct).expect("accepté");
        for o in cap
            .deliver(&data, &id, &to, today.add_days(1))
            .expect("livré")
        {
            promoted |= matches!(o, Outcome::Promoted { .. });
        }
        sim.advance_days(7);
    }
    assert!(cap.traits.contains(sh_sim::career::WORD_KEPT));
    assert!(promoted, "réputation 18 et 6 contrats : grade 1");
    assert_eq!(cap.ranks.get(&fac("hexa_corp")), Some(&1));
}

#[test]
fn livrer_remplit_les_stocks_et_fait_baisser_le_prix() {
    let data = donnees();
    let mut sim = SimWorld::from_data(&data, Some(1));
    let before = sim.price(&cygnara(), "food").expect("prix");
    sim.apply(&sh_sim::SimCommand::Deliver {
        at: cygnara(),
        commodity: "food".into(),
        amount: 3000.0,
    });
    let after = sim.price(&cygnara(), "food").expect("prix");
    assert!(after < before, "{after} < {before}");
    sim.apply(&sh_sim::SimCommand::Take {
        at: cygnara(),
        commodity: "food".into(),
        amount: 1e9,
    });
    assert!(sim.price(&cygnara(), "food").expect("prix") > before);
}
