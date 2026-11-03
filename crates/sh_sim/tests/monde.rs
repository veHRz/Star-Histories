//! Le monde vivant sur les vraies données (critères de M4) : déterminisme, budget, rails,
//! secrets, et un journal d'histoire de deux ans pour l'auteur (`target/histoire_2ans.txt`).

use std::path::Path;
use std::time::Instant;

use sh_core::{StellarDate, Year};
use sh_data::GameData;
use sh_sim::{EventKind, SimWorld};

fn donnees() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
    GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"))
}

const DEUX_ANS: u32 = 730;

fn simule(data: &GameData, seed: u64) -> SimWorld {
    let mut world = SimWorld::from_data(data, Some(seed));
    world.advance_days(DEUX_ANS);
    world
}

#[test]
fn meme_graine_meme_histoire() {
    let data = donnees();
    let a = simule(&data, 7);
    let b = simule(&data, 7);
    assert_eq!(a.history(), b.history());
    assert!(!a.history().is_empty());
    let c = simule(&data, 8);
    assert_ne!(a.history(), c.history(), "deux graines, deux histoires");
}

#[test]
fn deux_ans_en_moins_de_deux_secondes() {
    let data = donnees();
    let start = Instant::now();
    let world = simule(&data, 1);
    let elapsed = start.elapsed().as_secs_f64();
    assert_eq!(world.date(), StellarDate::start_of(Year(427)));
    assert!(elapsed < 2.0, "deux ans simulés en {elapsed:.2} s");
}

#[test]
fn aucun_rail_viole() {
    let data = donnees();
    for seed in [1, 2, 3, 4, 5] {
        let mut world = SimWorld::from_data(&data, Some(seed));
        for _ in 0..DEUX_ANS {
            world.step_day();
            let date = world.date();
            for rail in &data.rails.fleets_stay {
                if rail.until.is_none_or(|u| date < u)
                    && let Some(f) = world.fleets.get(&rail.fleet)
                {
                    assert!(
                        f.location == rail.at && f.transit.is_none(),
                        "graine {seed}, {date:?} : {} a quitté {}",
                        rail.fleet,
                        rail.at
                    );
                }
            }
            for rail in &data.rails.projects_not_before {
                if date < rail.not_before
                    && let Some(p) = world.projects.get(&rail.project)
                {
                    assert!(
                        p.progress < 1.0,
                        "graine {seed} : {} achevé trop tôt",
                        rail.project
                    );
                }
            }
        }
        for e in world.history() {
            if let EventKind::ConvoyRaided { owner, .. } | EventKind::ConvoyDefended { owner, .. } =
                &e.kind
            {
                for spare in &data.rails.spares {
                    assert_ne!(
                        owner, &spare.spared,
                        "graine {seed} : convoi épargné attaqué"
                    );
                }
            }
        }
    }
}

#[test]
fn rien_de_secret_dans_les_evenements_publics() {
    let data = donnees();
    let world = simule(&data, 3);
    let secret_fleets: Vec<_> = data
        .galaxy
        .fleets
        .iter()
        .filter(|f| f.secret)
        .map(|f| &f.id)
        .collect();
    let secret_projects: Vec<_> = data
        .sim
        .projects
        .iter()
        .filter(|p| p.secret)
        .map(|p| p.id.as_str())
        .collect();
    for e in world.public_events() {
        match &e.kind {
            EventKind::FleetArrived { fleet, .. } | EventKind::FleetRepaired { fleet } => {
                assert!(
                    !secret_fleets.contains(&fleet),
                    "flotte secrète publiée : {fleet}"
                );
            }
            EventKind::ProjectMilestone { project, .. } => {
                assert!(
                    !secret_projects.contains(&project.as_str()),
                    "projet secret publié : {project}"
                );
            }
            EventKind::ConvoyRaided { raiders, .. } | EventKind::ConvoyDefended { raiders, .. } => {
                // La Flotte Nordique n'apparaît que sous son nom public.
                assert!(
                    !raiders.contains("Nordique"),
                    "pillards secrets nommés : {raiders}"
                );
            }
            _ => {}
        }
    }
}

#[test]
fn le_monde_bouge_de_facon_plausible() {
    let data = donnees();
    let world = simule(&data, 425);
    let count = |pred: &dyn Fn(&EventKind) -> bool| {
        world.history().iter().filter(|e| pred(&e.kind)).count()
    };
    let raids = count(&|k| matches!(k, EventKind::ConvoyRaided { .. }));
    let skirmishes = count(&|k| matches!(k, EventKind::Skirmish { .. }));
    let patrols = count(&|k| matches!(k, EventKind::FleetArrived { .. }));
    let revolts = count(&|k| matches!(k, EventKind::Revolt { .. }));
    // Les pirates remontent jusqu'à Cygnara (§9.1) ; Tosear et la Ligue s'accrochent sans cesse.
    assert!((5..=200).contains(&raids), "raids : {raids}");
    assert!(
        (10..=80).contains(&skirmishes),
        "escarmouches : {skirmishes}"
    );
    assert!(patrols >= 20, "mouvements de flottes : {patrols}");
    assert!((1..=6).contains(&revolts), "révoltes : {revolts}");
    // Les convois nourrissent Cygnara : pas de famine permanente.
    let cygnara = sh_core::Id::new("sys_cygnara").unwrap_or_else(|e| panic!("{e}"));
    let food = world.price(&cygnara, "food").unwrap_or(f64::NAN);
    assert!(food < 40.0, "nourriture à Cygnara : {food:.1} crédits");

    // Le journal d'histoire, pour l'auteur.
    let names = |id: &str| -> String {
        let place = sh_core::Id::new(id).ok();
        place
            .as_ref()
            .and_then(|p| data.galaxy.system(p).map(|s| s.name.clone()))
            .or_else(|| {
                place
                    .as_ref()
                    .and_then(|p| data.galaxy.body(p).map(|b| b.name.clone()))
            })
            .or_else(|| {
                place
                    .as_ref()
                    .and_then(|p| data.galaxy.station(p).map(|s| s.name.clone()))
            })
            .or_else(|| {
                data.galaxy
                    .fleets
                    .iter()
                    .find(|f| f.id.as_str() == id)
                    .map(|f| f.name.clone())
            })
            .or_else(|| {
                data.factions
                    .factions
                    .iter()
                    .find(|f| f.id.as_str() == id)
                    .map(|f| f.name.clone())
            })
            .unwrap_or_else(|| id.to_owned())
    };
    let mut text = String::from(
        "Journal d'histoire : deux ans de simulation (graine 425), généré par cargo t.\n\n",
    );
    for e in world.history() {
        text.push_str(&e.narrate(&names));
        text.push('\n');
    }
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/histoire_2ans.txt");
    std::fs::write(&out, text).unwrap_or_else(|e| panic!("{e}"));
}
