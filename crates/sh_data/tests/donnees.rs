//! Cohérence des vraies données du jeu (`assets/data`) : `cargo t` échoue si une donnée est
//! incohérente (AGENTS.md §8.1).

use std::path::{Path, PathBuf};

use sh_core::Id;
use sh_data::{GameData, Place};

fn racine() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .unwrap_or(manifest)
        .to_path_buf()
}

fn donnees() -> GameData {
    GameData::load(&racine().join("assets/data")).unwrap_or_else(|err| panic!("{err}"))
}

fn lieu(id: &str) -> Id<Place> {
    Id::new(id).unwrap_or_else(|err| panic!("{err}"))
}

#[test]
fn les_donnees_sont_valides() {
    if let Err(report) = donnees().validate() {
        panic!("{report}");
    }
}

#[test]
fn les_hubs_canon_sont_tous_presents() {
    // §3.7 : sept grands hubs de Portails.
    let data = donnees();
    let mut hubs: Vec<&str> = data
        .galaxy
        .portal_hubs
        .iter()
        .map(|h| h.body.as_str())
        .collect();
    hubs.sort_unstable();
    assert_eq!(
        hubs,
        [
            "concordia",
            "cygnara",
            "hexis",
            "karza",
            "primaterre",
            "vantra",
            "vorrhal"
        ]
    );
}

#[test]
fn les_secrets_canon_sont_marques() {
    // §10 : Vesper, Verdine, l'EESS, la FSS, la Flotte d'Attaque fédérale, la Flotte Nordique.
    let data = donnees();
    let g = &data.galaxy;
    assert!(g.system(&lieu("sys_vesper")).is_some_and(|s| s.secret));
    for astre in ["vesper", "verdine"] {
        assert!(g.body(&lieu(astre)).is_some_and(|b| b.secret), "{astre}");
    }
    for flotte in [
        "eess",
        "fss",
        "flotte_attaque_federation",
        "flotte_nordique",
    ] {
        let f = g.fleets.iter().find(|f| f.id.as_str() == flotte);
        assert!(f.is_some_and(|f| f.secret), "{flotte}");
    }
    // Le préfixe OXL trahit la Flotte Nordique : secret lui aussi.
    let oxl = data
        .ship_classes
        .prefixes
        .iter()
        .find(|p| p.prefix == "OXL");
    assert!(oxl.is_some_and(|p| p.secret));
}

#[test]
fn le_vaisseau_capitale_n_a_jamais_existe() {
    // §11.2 : « Aucun vaisseau capitale n'a jamais existé. »
    let data = donnees();
    let capitale = data
        .ship_classes
        .classes
        .iter()
        .find(|c| c.id.as_str() == "vaisseau_capitale");
    assert!(capitale.is_some_and(|c| !c.ever_built));
    let construites = data
        .ship_classes
        .classes
        .iter()
        .filter(|c| c.ever_built)
        .count();
    assert_eq!(construites, data.ship_classes.classes.len() - 1);
}

#[test]
fn les_propositions_sont_au_registre() {
    let registre = std::fs::read_to_string(racine().join("docs/propositions.md"))
        .unwrap_or_else(|err| panic!("docs/propositions.md : {err}"));
    let mut manquantes = Vec::new();
    for fichier in fichiers_ron(&racine().join("assets/data")) {
        let texte = std::fs::read_to_string(&fichier).unwrap_or_default();
        for (i, _) in texte.match_indices("Proposition(\"") {
            let reste = &texte[i + "Proposition(\"".len()..];
            let numero: String = reste.chars().take_while(|c| *c != '"').collect();
            if !registre.contains(&format!("| {numero} |")) {
                manquantes.push(format!("{} : {numero}", fichier.display()));
            }
        }
    }
    assert!(
        manquantes.is_empty(),
        "propositions absentes du registre : {manquantes:?}"
    );
}

fn fichiers_ron(dossier: &Path) -> Vec<PathBuf> {
    let mut fichiers = Vec::new();
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return fichiers;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            fichiers.extend(fichiers_ron(&chemin));
        } else if chemin.extension().is_some_and(|e| e == "ron") {
            fichiers.push(chemin);
        }
    }
    fichiers.sort();
    fichiers
}

// --- La validation attrape bien les erreurs ------------------------------------------------------

fn messages(data: &GameData) -> String {
    match data.validate() {
        Ok(()) => String::new(),
        Err(report) => report.to_string(),
    }
}

#[test]
fn detecte_un_identifiant_en_double() {
    let mut data = donnees();
    let copie = data.galaxy.bodies[1].clone();
    data.galaxy.bodies.push(copie);
    assert!(messages(&data).contains("en double"));
}

#[test]
fn detecte_une_case_incoherente() {
    let mut data = donnees();
    let systeme = &mut data.galaxy.systems[0];
    systeme.center.x += 100.0;
    assert!(messages(&data).contains("case incohérente"));
}

#[test]
fn detecte_une_reference_inconnue() {
    let mut data = donnees();
    data.galaxy.fleets[0].faction =
        Some(Id::new("faction_fantome").unwrap_or_else(|e| panic!("{e}")));
    assert!(messages(&data).contains("faction inconnue : faction_fantome"));
}

#[test]
fn detecte_une_source_manquante() {
    let mut data = donnees();
    data.factions.factions[0].source = None;
    assert!(messages(&data).contains("sans `source`"));
}

#[test]
fn detecte_une_fuite_de_secret() {
    let mut data = donnees();
    data.galaxy.bodies[0].role_note.push_str(" 🔒 révélation");
    assert!(messages(&data).contains("passage secret"));
    // Le même texte dans `secret_notes` est permis.
    let mut data = donnees();
    data.galaxy.bodies[0]
        .secret_notes
        .push("🔒 révélation".into());
    assert_eq!(messages(&data), "");
}

#[test]
fn detecte_un_astre_public_dans_un_systeme_secret() {
    let mut data = donnees();
    let verdine = data
        .galaxy
        .bodies
        .iter_mut()
        .find(|b| b.id.as_str() == "verdine")
        .unwrap_or_else(|| panic!("Verdine absente"));
    verdine.secret = false;
    assert!(messages(&data).contains("système secret"));
}

#[test]
fn les_modeles_des_secteurs_existent() {
    let data = donnees();
    let assets = racine().join("assets");
    assert!(!data.sectors.is_empty(), "aucun secteur chargé");
    for sector in &data.sectors {
        let mut chemins = vec![sector.door_model.clone()];
        for cell in &sector.cells {
            chemins.push(cell.model.clone());
            chemins.extend(cell.lod.clone());
            chemins.extend(cell.lightmap.as_ref().map(|l| l.image.clone()));
        }
        for chemin in chemins {
            assert!(
                assets.join(&chemin).is_file(),
                "{} : fichier absent {chemin}",
                sector.id
            );
        }
    }
}

#[test]
fn le_vaisseau_du_joueur_est_connexe() {
    let data = donnees();
    let vaisseau = data
        .sector("player_ship")
        .unwrap_or_else(|| panic!("player_ship absent"));
    assert_eq!(vaisseau.within("sas", 8).len(), vaisseau.cells.len());
    assert!(vaisseau.spawns.iter().any(|s| s.id == "sas"));
}
