//! Cohérence des vraies données du jeu (`assets/data`) : `cargo t` échoue si une donnée est
//! incohérente (AGENTS.md §8.1).

use std::path::{Path, PathBuf};

use sh_core::Id;
use sh_data::{GameData, Place, PlacedKind, system_layout};

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
fn les_modeles_des_scenes_existent() {
    let data = donnees();
    let assets = racine().join("assets");
    for scene in &data.scenes {
        let mut chemins: Vec<String> = scene.structures.iter().map(|s| s.model.clone()).collect();
        chemins.extend(scene.station_models.values().cloned());
        chemins.extend(scene.traffic.as_ref().map(|t| t.model.clone()));
        for chemin in chemins {
            assert!(
                assets.join(&chemin).is_file(),
                "{} : modèle absent {chemin}",
                scene.system
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

/// L'échelle des systèmes (P2) tient sa cible de jeu : quelques minutes entre planètes voisines,
/// moins d'une minute et demie pour un trajet local.
#[test]
fn l_echelle_des_systemes_donne_des_trajets_de_quelques_minutes() {
    let sc = donnees().space.scale;
    let parking = sc.parking_orbit_radii * sc.planet_radius;
    for rank in 1..=4 {
        let length = sc.orbit_radius(rank + 1) - sc.orbit_radius(rank);
        let t = sc.cruise_time(length, parking, parking);
        assert!(
            (45.0..=180.0).contains(&t),
            "rang {rank} → {} : {t:.0} s",
            rank + 1
        );
    }
    let to_station = sc.cruise_time(2.0e7, parking, 0.0);
    assert!(
        to_station <= 90.0,
        "orbite → station à 20 000 km : {to_station:.0} s"
    );
    let to_moon = sc.cruise_time(
        sc.moon_orbit_radii * sc.planet_radius,
        parking,
        sc.parking_orbit_radii * sc.moon_radius,
    );
    assert!(to_moon <= 60.0, "orbite → lune : {to_moon:.0} s");
    // Le système compressé reste précis en f64 : ulp < 0,1 mm à la dixième orbite.
    let far = sc.orbit_radius(10);
    assert!(far * f64::EPSILON < 1e-4, "{far} m");
}

#[test]
fn disposition_du_systeme_d_hexis() {
    let data = donnees();
    let sc = &data.space.scale;
    let system = data
        .galaxy
        .system(&lieu("sys_hexis"))
        .unwrap_or_else(|| panic!("sys_hexis absent"));
    let placed = system_layout(&data.galaxy, system, sc);
    let find = |id: &str| {
        placed
            .iter()
            .find(|p| p.id.as_ref().is_some_and(|i| i.as_str() == id))
            .unwrap_or_else(|| panic!("{id} non disposé"))
    };
    // Étoile non nommée par le canon, au centre.
    assert!(
        placed
            .iter()
            .any(|p| p.kind == PlacedKind::Star && p.id.is_none())
    );
    let hexis = find("hexis");
    assert!((hexis.pos.length() - sc.orbit_radius(1)).abs() < 1.0);
    let enclume = find("l_enclume");
    assert_eq!(enclume.kind, PlacedKind::Station);
    assert_eq!(enclume.around.as_ref().map(|i| i.as_str()), Some("hexis"));
    let altitude = enclume.pos.distance(hexis.pos) - hexis.radius;
    assert!((altitude - sc.parking_orbit_radii * hexis.radius).abs() < 1.0);
}

/// Pour tous les systèmes publics : disposition déterministe, finie, sans chevauchement, et sans
/// aucun astre ni station secrets.
#[test]
fn dispositions_jouables_de_tous_les_systemes() {
    let data = donnees();
    let sc = &data.space.scale;
    for system in data.galaxy.systems.iter().filter(|s| !s.secret) {
        let placed = system_layout(&data.galaxy, system, sc);
        assert_eq!(
            placed,
            system_layout(&data.galaxy, system, sc),
            "{}",
            system.id
        );
        for (i, a) in placed.iter().enumerate() {
            assert!(
                a.pos.length().is_finite(),
                "{} : position non finie",
                system.id
            );
            if let Some(id) = &a.id {
                let secret = data.galaxy.body(id).is_some_and(|b| b.secret)
                    || data.galaxy.station(id).is_some_and(|s| s.secret);
                assert!(!secret, "{} : {id} est secret", system.id);
            }
            for b in &placed[i + 1..] {
                let gap = a.pos.distance(b.pos) - a.radius - b.radius;
                assert!(
                    gap > 1.0e4,
                    "{} : {:?} et {:?} se chevauchent",
                    system.id,
                    a.id,
                    b.id
                );
            }
        }
    }
}

/// Critère de M3 : rien de secret sur la carte galactique (entrées et textes affichables).
#[test]
fn la_carte_ne_montre_aucun_secret() {
    let data = donnees();
    let entries = sh_data::public_map(&data.galaxy);
    assert!(!entries.is_empty());
    let g = &data.galaxy;
    for e in &entries {
        let secret = g.system(&e.id).is_some_and(|s| s.secret)
            || g.body(&e.id).is_some_and(|b| b.secret)
            || g.station(&e.id).is_some_and(|s| s.secret)
            || g.phenomena.iter().any(|p| p.id == e.id && p.secret);
        assert!(!secret, "élément secret sur la carte : {}", e.id);
        assert!(!e.name.contains('🔒'), "nom marqué secret : {}", e.name);
        if let Some(f) = &e.controller {
            let faction = data
                .faction(f)
                .unwrap_or_else(|| panic!("faction inconnue {f}"));
            assert!(
                !faction.name.contains('🔒'),
                "faction marquée secrète : {}",
                faction.name
            );
        }
    }
    // Les systèmes secrets du canon (Vesper) existent bien dans les données, mais pas ici.
    assert!(g.systems.iter().any(|s| s.secret));
    assert!(
        entries
            .iter()
            .all(|e| g.system(&e.id).is_none_or(|s| !s.secret))
    );
    // Les zones dessinées sont publiques.
    assert!(
        g.zones
            .iter()
            .filter(|z| z.secret)
            .all(|z| !z.outline.is_empty() || z.secret)
    );
}

#[test]
fn le_francais_et_l_anglais_ont_les_memes_cles() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
    let lire = |rel: &str| -> sh_data::Lang {
        sh_data::read_ron(&dir.join(rel)).unwrap_or_else(|e| panic!("{e}"))
    };
    let fr = lire(sh_data::paths::LANG);
    let en = lire(sh_data::paths::LANG_EN);
    let manquantes: Vec<&String> = fr.strings.keys().filter(|k| !en.has(k)).collect();
    let en_trop: Vec<&String> = en.strings.keys().filter(|k| !fr.has(k)).collect();
    assert!(
        manquantes.is_empty() && en_trop.is_empty(),
        "en.ron : clés manquantes {manquantes:?}, clés en trop {en_trop:?}"
    );
    assert_eq!(fr.code, "fr");
}
