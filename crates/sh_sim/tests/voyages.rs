//! Voyages sur les vraies données : durées cohérentes, réseau conforme au canon, secrets exclus.

use std::path::Path;

use sh_core::Id;
use sh_data::{GameData, Place};
use sh_sim::travel::{hyperspace_plan, portal_plan};

fn donnees() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
    GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"))
}

fn lieu(id: &str) -> Id<Place> {
    Id::new(id).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn hexis_cygnara_par_portail_et_par_hyperespace() {
    let d = donnees();
    let ship = &d.travel.ships["player_ship"];
    let (a, b) = (lieu("sys_hexis"), lieu("sys_cygnara"));
    let portal = portal_plan(&d.galaxy, &d.portal_links, &d.travel, ship, &a, &b)
        .unwrap_or_else(|| panic!("pas de Portail Hexis → Cygnara"));
    let hyper = hyperspace_plan(&d.galaxy, &d.travel, ship, &a, &b)
        .unwrap_or_else(|| panic!("pas d'hyperespace Hexis → Cygnara"));
    assert_eq!(portal.hubs.len(), 2, "un seul passage");
    assert!(portal.hours <= 12.0, "Portail : {} h", portal.hours);
    // ≈ 80 unités de carte : environ trois semaines (≈ 3 jours entre voisins, §2.2).
    assert!((15.0..=35.0).contains(&hyper.days()), "hyperespace : {} j", hyper.days());
    assert!(hyper.feasible && portal.feasible);
    assert!(hyper.hours > 20.0 * portal.hours);
}

#[test]
fn seul_hexis_relie_les_deux_blocs() {
    let d = donnees();
    let ship = &d.travel.ships["player_ship"];
    let plan = portal_plan(
        &d.galaxy,
        &d.portal_links,
        &d.travel,
        ship,
        &lieu("sys_cygnara"),
        &lieu("sys_vorrh"),
    )
    .unwrap_or_else(|| panic!("pas de Portail Cygnara → Vorrh"));
    assert!(plan.hubs.contains(&lieu("hexis")), "{:?}", plan.hubs);
}

#[test]
fn aucun_plan_vers_un_systeme_secret() {
    let d = donnees();
    let ship = &d.travel.ships["player_ship"];
    let hexis = lieu("sys_hexis");
    for secret in d.galaxy.systems.iter().filter(|s| s.secret) {
        assert!(hyperspace_plan(&d.galaxy, &d.travel, ship, &hexis, &secret.id).is_none());
        assert!(portal_plan(&d.galaxy, &d.portal_links, &d.travel, ship, &hexis, &secret.id).is_none());
    }
}

#[test]
fn le_croissant_se_contourne() {
    let d = donnees();
    let ship = &d.travel.ships["player_ship"];
    let public: Vec<_> = d.galaxy.systems.iter().filter(|s| !s.secret).collect();
    let mut detours = 0;
    for a in &public {
        for b in &public {
            if let Some(plan) = hyperspace_plan(&d.galaxy, &d.travel, ship, &a.id, &b.id)
                && plan.detour
            {
                let straight = (b.center.x - a.center.x).hypot(b.center.y - a.center.y);
                assert!(plan.map_length > straight, "{} → {}", a.id, b.id);
                detours += 1;
            }
        }
    }
    assert!(detours > 0, "aucun trajet ne traverse une nébuleuse bloquante");
}
