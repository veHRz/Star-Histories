//! Journal d'histoire de la simulation : `cargo run -p sh_sim --example histoire -- [jours] [graine]`.
//! Imprime les événements (publics et secrets) en français, puis un résumé par genre.

use std::collections::BTreeMap;
use std::path::Path;

use sh_data::GameData;
use sh_sim::SimWorld;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let days: u32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(730);
    let seed: Option<u64> = args.get(2).and_then(|a| a.parse().ok());
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
    let data = match GameData::load(&dir) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let mut world = SimWorld::from_data(&data, seed);
    world.advance_days(days);
    let names = |id: &str| -> String {
        data.galaxy
            .fleets
            .iter()
            .find(|f| f.id.as_str() == id)
            .map(|f| f.name.clone())
            .or_else(|| {
                data.factions
                    .factions
                    .iter()
                    .find(|f| f.id.as_str() == id)
                    .map(|f| f.name.clone())
            })
            .or_else(|| {
                let p = sh_core::Id::new(id).ok()?;
                data.galaxy
                    .system(&p)
                    .map(|s| s.name.clone())
                    .or_else(|| data.galaxy.body(&p).map(|b| b.name.clone()))
                    .or_else(|| data.galaxy.station(&p).map(|s| s.name.clone()))
            })
            .unwrap_or_else(|| id.to_owned())
    };
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for e in world.history() {
        println!("{}", e.narrate(&names));
        let kind = format!("{:?}", e.kind);
        let kind = kind
            .split([' ', '{', '('])
            .next()
            .unwrap_or_default()
            .to_owned();
        *kinds.entry(kind).or_default() += 1;
    }
    println!(
        "\n{} événements en {days} jours : {kinds:?}",
        world.history().len()
    );
}
