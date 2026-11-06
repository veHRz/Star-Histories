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
        // Modèle de porte propre à un portail (porte blindée du sas, M10).
        chemins.extend(sector.portals.iter().filter_map(|p| p.door_model.clone()));
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
        chemins.extend(data.combat.ships.values().map(|c| c.model.clone()));
        chemins.extend(
            data.combat
                .firearms
                .values()
                .filter_map(|f| f.model.clone()),
        );
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
    let depart = data.start().unwrap_or_else(|| panic!("aucun départ"));
    let vaisseau = data
        .sector(&depart.interior)
        .unwrap_or_else(|| panic!("{} absent", depart.interior));
    assert_eq!(vaisseau.within("sas", 8).len(), vaisseau.cells.len());
    assert!(vaisseau.spawns.iter().any(|s| s.id == "sas"));
    // M9 : la partie commence dans la chambre, debout ; aux commandes, au cockpit.
    assert_eq!(depart.first_spawn(), "chambre");
    assert_eq!(depart.aboard_spawn, "cockpit");
    assert!(depart.seat_role.is_none());
}

/// Le vaisseau du joueur amarré au spatioport par défaut (M9, docs/m9-retours.md §3.8) : pont
/// au niveau du sol du hangar, une seule porte au raccord (la porte extérieure du sas posée sur
/// le portail du hangar), et chaque position dans la cellule la plus précise.
#[test]
fn le_vaisseau_s_amarre_dans_le_hangar() {
    let data = donnees();
    let depart = data.start().unwrap_or_else(|| panic!("aucun départ"));
    let ship = data
        .sector(&depart.interior)
        .unwrap_or_else(|| panic!("intérieur absent"));
    let port = data
        .sector(sh_data::scene::DEFAULT_SPACEPORT)
        .unwrap_or_else(|| panic!("spatioport par défaut absent"));
    let docked =
        sh_data::sector::dock(ship, "@dock", port, "@ship").unwrap_or_else(|e| panic!("{e}"));
    assert!(docked.problems().is_empty(), "{:?}", docked.problems());
    let placement = docked
        .cells
        .iter()
        .find_map(|c| c.placement)
        .unwrap_or_else(|| panic!("vaisseau non placé"));
    // Pont du vaisseau au niveau du sol du hangar.
    assert!(placement.translation.1.abs() < 1e-4, "{placement:?}");
    // Une seule porte entre le vaisseau et le hangar : la porte extérieure du sas, à
    // l'emplacement du portail `hangar__ship`, tournée vers le quai.
    let hatch = ship
        .external_port("@dock")
        .unwrap_or_else(|| panic!("sas sans porte extérieure"));
    let berth = port
        .external_port("@ship")
        .unwrap_or_else(|| panic!("hangar sans portail du vaisseau"));
    let joints: Vec<_> = docked
        .portals
        .iter()
        .filter(|p| {
            let ship_side = |c: &str| ship.cell(c).is_some();
            ship_side(&p.a) != ship_side(&p.b)
        })
        .collect();
    assert_eq!(joints.len(), 1, "{joints:?}");
    let joint = joints[0];
    assert_eq!((joint.a.as_str(), joint.b.as_str()), ("sas", "hangar"));
    let moved = placement.apply(hatch.center);
    for (a, b) in [
        (moved.0, berth.center.0),
        (moved.1, berth.center.1),
        (moved.2, berth.center.2),
    ] {
        assert!(
            (a - b).abs() < 1e-3,
            "porte du sas {moved:?}, portail du hangar {:?}",
            berth.center
        );
    }
    assert!(joint.normal.0 < -0.99, "{joint:?}");
    // Les portes du vaisseau gardent leur modèle à quai, sas compris (M10 : elles prenaient
    // celui du quartier, jaune) ; celles du quartier, le leur.
    assert_eq!(docked.door_model_of(joint), ship.door_model_of(&hatch));
    for p in &ship.portals {
        if let Some(moved) = docked.portals.iter().find(|d| d.id == p.id) {
            assert_eq!(
                docked.door_model_of(moved),
                ship.door_model_of(p),
                "{}",
                p.id
            );
        }
    }
    for p in &port.portals {
        if let Some(kept) = docked.portals.iter().find(|d| d.id == p.id) {
            assert_eq!(
                docked.door_model_of(kept),
                port.door_model_of(p),
                "{}",
                p.id
            );
        }
    }
    // Le quai est du côté du hangar, devant la porte ; l'apparition du sas, à bord.
    let at = |id: &str| {
        let spawn = docked
            .spawns
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("apparition {id} absente"));
        docked.cell_at(spawn.pos).map(|c| c.id.clone())
    };
    assert_eq!(at("quai").as_deref(), Some("hangar"));
    assert_eq!(at("sas").as_deref(), Some("sas"));
    assert_eq!(at("chambre").as_deref(), Some("chambre"));
    // Le hangar est la première cellule : un chargement à quai sans siège démarre au quai.
    assert_eq!(docked.spawns.first().map(|s| s.id.as_str()), Some("quai"));
}

/// Le port d'Hexis utilise le spatioport par défaut, sous son propre nom, et ses guichetiers.
#[test]
fn hexis_a_le_spatioport_par_defaut() {
    let data = donnees();
    let (scene, port) = sh_data::scene::find_dock(
        &data.scenes,
        sh_data::scene::DEFAULT_SPACEPORT,
        Some("sys_hexis"),
    )
    .unwrap_or_else(|| panic!("aucun port pour le spatioport par défaut"));
    assert_eq!(scene.system.as_str(), "sys_hexis");
    assert_eq!(port.place_key.as_deref(), Some("port.hexis"));
    let banque = sh_data::Dialogue::for_role_at(
        &data.dialogues,
        "bank",
        Some(&port.sector),
        &[&port.id, scene.system.as_str()],
    );
    assert_eq!(banque.map(|d| d.id.as_str()), Some("hexis_bank"));
}

/// L'échelle des systèmes (P2, croisière plus lente de M11) tient sa cible de jeu : de une à
/// quatre minutes entre planètes voisines, moins de deux minutes pour un trajet local.
#[test]
fn l_echelle_des_systemes_donne_des_trajets_de_quelques_minutes() {
    let sc = donnees().space.scale;
    let parking = sc.parking_orbit_radii * sc.planet_radius;
    for rank in 1..=4 {
        let length = sc.orbit_radius(rank + 1) - sc.orbit_radius(rank);
        let t = sc.cruise_time(length, parking, parking);
        assert!(
            (60.0..=240.0).contains(&t),
            "rang {rank} → {} : {t:.0} s",
            rank + 1
        );
    }
    let to_station = sc.cruise_time(2.0e7, parking, 0.0);
    assert!(
        to_station <= 120.0,
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

/// Vitesses de vol de M11 (P33, Q19) : la poussée du joueur reste sous la croisière, et sa
/// vitesse de combat sous celle des pillards de la Ligue (un raid ne se fuit pas à la poussée :
/// il faut les distancer hors de portée, puis la croisière, comme en M6).
#[test]
fn la_vitesse_de_combat_garde_les_raids_de_m6() {
    let data = donnees();
    let sc = &data.space.scale;
    let corvette = data
        .flight
        .ships
        .get("corvette")
        .unwrap_or_else(|| panic!("maniement de la corvette absent"));
    assert!(corvette.max_speed >= 1000.0, "{}", corvette.max_speed);
    // Le tutoriel reconnaît la croisière au-delà d'une fois et demie la poussée maximale
    // (`tutorial.rs`, `CRUISE_FACTOR`) : même au ras d'une surface, elle y est.
    assert!(1.5 * corvette.max_speed < sc.cruise_min_speed);
    let raiders = &data.combat.raid.attackers;
    assert!(!raiders.is_empty());
    for id in raiders {
        let ship = data
            .combat
            .ships
            .get(id)
            .unwrap_or_else(|| panic!("coque inconnue : {id}"));
        assert!(
            corvette.combat_max_speed < ship.max_speed,
            "{id} : {} m/s, combat du joueur à {} m/s",
            ship.max_speed,
            corvette.combat_max_speed
        );
    }
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

/// Sur l'union des fichiers de chaque langue (fichier commun et fichiers de domaine).
#[test]
fn le_francais_et_l_anglais_ont_les_memes_cles() {
    let dir = racine().join("assets/data");
    let lire = |code: &str| -> sh_data::Lang {
        sh_data::load_lang(&dir, code).unwrap_or_else(|e| panic!("{e}"))
    };
    let fr = lire(sh_data::paths::LANG_CODE);
    let en = lire(sh_data::paths::LANG_EN_CODE);
    let manquantes: Vec<&String> = fr.strings.keys().filter(|k| !en.has(k)).collect();
    let en_trop: Vec<&String> = en.strings.keys().filter(|k| !fr.has(k)).collect();
    assert!(
        manquantes.is_empty() && en_trop.is_empty(),
        "en : clés manquantes {manquantes:?}, clés en trop {en_trop:?}"
    );
    assert_eq!(fr.code, "fr");
    assert_eq!(en.code, "en");
    for lang in [&fr, &en] {
        assert!(
            lang.duplicates.is_empty(),
            "{} : clés en double {:?}",
            lang.code,
            lang.duplicates
        );
    }
    // Des fichiers de domaine sont bien lus (la langue n'est pas réduite au fichier commun).
    assert!(
        fr.origins.values().any(|f| f.starts_with("lang/fr/")),
        "aucun fichier de domaine lu"
    );
}

#[test]
fn detecte_une_cle_de_texte_en_double() {
    let mut data = donnees();
    data.lang.duplicates.push(sh_data::DuplicateKey {
        key: "menu.quit".into(),
        first: "lang/fr.ron".into(),
        second: "lang/fr/hud.ron".into(),
    });
    let m = messages(&data);
    assert!(
        m.contains("lang/fr.ron et lang/fr/hud.ron") && m.contains("en double : menu.quit"),
        "{m}"
    );
}

#[test]
fn detecte_un_ravitaillement_invalide() {
    let mut data = donnees();
    data.combat.refuel.day_price = 0.0;
    assert!(messages(&data).contains("prix d'un jour de carburant invalide"));
    let mut data = donnees();
    data.combat.refuel.day_price = -5.0;
    assert!(messages(&data).contains("prix d'un jour de carburant invalide"));
    let mut data = donnees();
    data.combat.refuel.step_days = f64::NAN;
    assert!(messages(&data).contains("tranche de ravitaillement invalide"));
    let mut data = donnees();
    data.combat.refuel.commodity = "carburant_fantome".into();
    assert!(
        messages(&data)
            .contains("marchandise inconnue de sim.ron (base_prices) : carburant_fantome")
    );
}

#[test]
fn detecte_un_depart_incoherent() {
    let mut data = donnees();
    data.starts[0].interior = "vaisseau_fantome".into();
    assert!(messages(&data).contains("intérieur inconnu : vaisseau_fantome"));
    let mut data = donnees();
    data.starts[0].first_spawn = Some("nulle_part".into());
    assert!(messages(&data).contains("apparition inconnue à bord : nulle_part"));
    let mut data = donnees();
    data.scenes
        .iter_mut()
        .flat_map(|s| s.docks.iter_mut())
        .for_each(|d| d.place_key = None);
    assert!(messages(&data).contains("spatioport par défaut sans nom de lieu"));
    let mut data = donnees();
    data.dialogues[0].place = Some("lieu_fantome".into());
    assert!(
        messages(&data).contains("lieu inconnu (ni port, ni système mis en scène) : lieu_fantome")
    );
}

/// Le troisième casier de la chambre n'a pas d'entrée dans `items.ron` : il est vide (le jeu
/// l'ouvre sans contenu et sans avertissement), les deux autres ont leur contenu de départ.
#[test]
fn les_casiers_de_la_chambre() {
    let data = donnees();
    let contenu = |id: &str| data.items.containers.get(id);
    assert!(contenu("corvette_chambre_casier_1").is_some_and(|c| !c.is_empty()));
    assert!(contenu("corvette_chambre_casier_2").is_some_and(|c| !c.is_empty()));
    assert!(contenu("corvette_chambre_casier_3").is_none_or(|c| c.is_empty()));
}

#[test]
fn detecte_un_nom_de_salle_absent() {
    let mut data = donnees();
    data.sectors[0].cells[0].name_key = Some("room.salle_fantome".into());
    assert!(messages(&data).contains("nom de salle absent de la langue : room.salle_fantome"));
}

#[test]
fn detecte_un_objet_incoherent() {
    let mut data = donnees();
    data.items.items[0].name_key = "item.objet_fantome".into();
    assert!(messages(&data).contains("texte absent : item.objet_fantome"));
    let mut data = donnees();
    data.items
        .containers
        .entry("casier_test".into())
        .or_default()
        .insert("objet_fantome".into(), 1);
    assert!(messages(&data).contains("objet inconnu objet_fantome"));
    let mut data = donnees();
    data.graphics.low.decor_detail = 3;
    assert!(messages(&data).contains("decor_detail hors de"));
}

/// Les casiers de départ et l'équipement porté désignent des objets du catalogue, et l'arme
/// portée est celle du combattant joueur.
#[test]
fn le_catalogue_d_objets_de_depart() {
    let data = donnees();
    for id in [
        "datapad",
        "spare_parts",
        "mag_pistol",
        "flight_suit",
        "toolkit",
        "medkit",
        "ration",
        "data_chip",
        "cargo_manifest",
    ] {
        assert!(data.item(id).is_some(), "objet absent : {id}");
    }
    let equipment = data.items.starting_equipment();
    assert_eq!(
        equipment
            .get(&sh_data::EquipSlot::Weapon)
            .map(String::as_str),
        Some("mag_pistol")
    );
    let player_weapon = data
        .combat
        .fighters
        .get("player")
        .map(|f| f.weapon.as_str());
    assert_eq!(
        data.item("mag_pistol").and_then(|i| i.firearm.as_deref()),
        player_weapon
    );
}
