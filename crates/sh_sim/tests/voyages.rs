//! Voyages sur les vraies données : durées cohérentes, réseau conforme au canon, secrets exclus,
//! mondes isolés joignables par l'hyperespace (M9, Q18).

use std::path::Path;

use sh_core::Id;
use sh_data::{GameData, MapKind, Place};
use sh_sim::travel::{
    blocking_nebulae, hyperspace_blocked, hyperspace_path, hyperspace_plan, inside_obstacle,
    portal_plan,
};

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
    assert!(
        (15.0..=35.0).contains(&hyper.days()),
        "hyperespace : {} j",
        hyper.days()
    );
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
        assert!(
            portal_plan(
                &d.galaxy,
                &d.portal_links,
                &d.travel,
                ship,
                &hexis,
                &secret.id
            )
            .is_none()
        );
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
    assert!(
        detours > 0,
        "aucun trajet ne traverse une nébuleuse bloquante"
    );
}

#[test]
fn vers_un_monde_isole_par_l_hyperespace() {
    let d = donnees();
    let ship = &d.travel.ships["player_ship"];
    let (hexis, calderis) = (lieu("sys_hexis"), lieu("calderis"));
    let aller = hyperspace_plan(&d.galaxy, &d.travel, ship, &hexis, &calderis)
        .unwrap_or_else(|| panic!("pas d'hyperespace Hexis → Calderis"));
    // ≈ 65 unités de carte entre deux lieux tenus (Hexa-Corp, UGL) : près de trois semaines.
    assert!(
        (15.0..=25.0).contains(&aller.days()),
        "Hexis → Calderis : {} j",
        aller.days()
    );
    assert!(aller.feasible, "le carburant doit suffire : {}", aller.fuel);
    assert!(aller.fuel > 0.0 && aller.toll.cents == 0);
    // Q18 : aucun Portail vers un monde isolé (réglage par défaut).
    assert!(
        portal_plan(
            &d.galaxy,
            &d.portal_links,
            &d.travel,
            ship,
            &hexis,
            &calderis
        )
        .is_none()
    );
    // Et le retour vers un système.
    let retour = hyperspace_plan(&d.galaxy, &d.travel, ship, &calderis, &hexis)
        .unwrap_or_else(|| panic!("pas de retour Calderis → Hexis"));
    assert!((retour.hours - aller.hours).abs() < 1e-9);
    // Un monde sans maître (Cendres) : hors des routes cartographiées, plus long au même écart.
    let cendres = hyperspace_plan(&d.galaxy, &d.travel, ship, &hexis, &lieu("cendres"))
        .unwrap_or_else(|| panic!("pas d'hyperespace Hexis → Cendres"));
    let longueur = cendres.map_length * d.travel.hyperspace_days_per_unit * 24.0;
    assert!((cendres.hours - longueur * d.travel.improvised_factor).abs() < 1e-6);
}

#[test]
fn tous_les_mondes_isoles_de_la_carte_sont_des_destinations() {
    let d = donnees();
    let ship = &d.travel.ships["player_ship"];
    let hexis = lieu("sys_hexis");
    let obstacles = blocking_nebulae(&d.galaxy, &d.travel);
    let mondes: Vec<_> = sh_data::public_map(&d.galaxy)
        .into_iter()
        .filter(|e| e.kind == MapKind::Body)
        .collect();
    assert!(mondes.len() >= 14, "{} mondes isolés", mondes.len());
    for monde in &mondes {
        assert!(monde.destination, "{} n'est pas une destination", monde.id);
        let plan = hyperspace_plan(&d.galaxy, &d.travel, ship, &hexis, &monde.id);
        if inside_obstacle(monde.pos, &obstacles) {
            // Rade-Noire, au cœur du Croissant : pas d'hyperespace (§3.6).
            assert!(plan.is_none(), "{} est dans une nébuleuse", monde.id);
            assert!(hyperspace_blocked(&d.galaxy, &d.travel, &monde.id));
        } else {
            let plan = plan.unwrap_or_else(|| panic!("pas d'hyperespace vers {}", monde.id));
            assert!(plan.hours > 0.0, "{}", monde.id);
            assert!(!hyperspace_blocked(&d.galaxy, &d.travel, &monde.id));
        }
    }
    assert!(
        hyperspace_blocked(&d.galaxy, &d.travel, &lieu("rade_noire")),
        "Rade-Noire doit être au cœur du Croissant"
    );
    // Les stations et les phénomènes ne sont pas des destinations.
    assert!(hyperspace_plan(&d.galaxy, &d.travel, ship, &hexis, &lieu("terminus")).is_none());
    assert!(
        hyperspace_plan(
            &d.galaxy,
            &d.travel,
            ship,
            &hexis,
            &lieu("nebuleuse_du_croissant")
        )
        .is_none()
    );
}

#[test]
fn aucun_plan_vers_un_astre_secret() {
    let mut d = donnees();
    let ship = d.travel.ships["player_ship"].clone();
    let hexis = lieu("sys_hexis");
    // Les astres secrets des données (ceux d'un système secret) : jamais des destinations.
    let secrets: Vec<_> = d
        .galaxy
        .bodies
        .iter()
        .filter(|b| b.secret)
        .map(|b| b.id.clone())
        .collect();
    assert!(
        !secrets.is_empty(),
        "le canon a des astres secrets (Vesper)"
    );
    for id in &secrets {
        assert!(sh_data::travel::endpoint(&d.galaxy, id).is_none(), "{id}");
        assert!(hyperspace_plan(&d.galaxy, &d.travel, &ship, &hexis, id).is_none());
    }
    // Un monde isolé qui deviendrait secret disparaît des destinations, et ses lunes avec lui.
    for body in d
        .galaxy
        .bodies
        .iter_mut()
        .filter(|b| b.id.as_str() == "alcyone")
    {
        body.secret = true;
    }
    for id in ["alcyone", "maia", "merope"] {
        assert!(
            hyperspace_plan(&d.galaxy, &d.travel, &ship, &hexis, &lieu(id)).is_none(),
            "{id} reste une destination"
        );
        assert!(
            sh_data::public_map(&d.galaxy)
                .iter()
                .all(|e| e.id.as_str() != id),
            "{id} reste sur la carte"
        );
    }
}

#[test]
fn le_trace_d_hyperespace_suit_la_duree() {
    let d = donnees();
    let ship = &d.travel.ships["player_ship"];
    let obstacles = blocking_nebulae(&d.galaxy, &d.travel);
    // Hexis → Karza frôle le Croissant : le tracé le contourne, et sa longueur est celle du plan.
    let (a, b) = (lieu("sys_hexis"), lieu("karza"));
    let plan = hyperspace_plan(&d.galaxy, &d.travel, ship, &a, &b)
        .unwrap_or_else(|| panic!("pas d'hyperespace Hexis → Karza"));
    let from = d
        .galaxy
        .place_pos(&a)
        .unwrap_or_else(|| panic!("Hexis sans position"));
    let to = d
        .galaxy
        .place_pos(&b)
        .unwrap_or_else(|| panic!("Karza sans position"));
    let path = hyperspace_path(from, to, &obstacles);
    let drawn: f64 = path
        .windows(2)
        .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
        .sum();
    assert!(
        (drawn - plan.map_length).abs() < 0.5,
        "{drawn} contre {}",
        plan.map_length
    );
    assert!(plan.detour, "Hexis → Karza devrait frôler le Croissant");
    assert!(path.len() > 2);
}

#[test]
fn karza_monde_isole_a_hub_se_rejoint_par_portail() {
    let mut d = donnees();
    let ship = d.travel.ships["player_ship"].clone();
    let (hexis, karza) = (lieu("sys_hexis"), lieu("karza"));
    // Karza, « grand hub de Portails » (§3.4, §3.7), relié à celui d'Hexis (P19) : un passage,
    // dans les deux sens ; l'hyperespace reste possible.
    let plan = portal_plan(&d.galaxy, &d.portal_links, &d.travel, &ship, &hexis, &karza)
        .unwrap_or_else(|| panic!("pas de Portail Hexis → Karza"));
    assert_eq!(plan.hubs, vec![lieu("hexis"), karza.clone()]);
    assert!(portal_plan(&d.galaxy, &d.portal_links, &d.travel, &ship, &karza, &hexis).is_some());
    assert!(hyperspace_plan(&d.galaxy, &d.travel, &ship, &hexis, &karza).is_some());
    // Vantra porte un hub canon, mais aucune liaison ne le relie au réseau : pas de Portail.
    assert!(
        portal_plan(
            &d.galaxy,
            &d.portal_links,
            &d.travel,
            &ship,
            &hexis,
            &lieu("vantra")
        )
        .is_none()
    );
    // Un hub qui ne serait qu'une proposition n'ouvre pas un monde isolé aux Portails.
    for hub in d
        .galaxy
        .portal_hubs
        .iter_mut()
        .filter(|h| h.body.as_str() == "karza")
    {
        hub.status = sh_data::Status::Proposition("P99".into());
    }
    assert!(portal_plan(&d.galaxy, &d.portal_links, &d.travel, &ship, &hexis, &karza).is_none());
}
