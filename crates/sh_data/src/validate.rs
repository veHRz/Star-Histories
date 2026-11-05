//! Validation de cohérence des données.
//!
//! Tout ce que le chargement ne peut pas garantir à lui seul : identifiants uniques, références
//! résolues, case canon cohérente avec les coordonnées, source citée pour chaque entrée canon,
//! aucun passage 🔒 hors des champs secrets. La validation collecte **tous** les problèmes avant
//! d'échouer, pour qu'une régénération du canon se corrige en une passe.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Serialize;
use sh_core::{GridCell, Id, MapPos};

use crate::GameData;
use crate::common::{Location, Place, Status};
use crate::factions::Faction;
use crate::galaxy::BodyKind;

/// Le marqueur des secrets d'univers dans le canon.
pub const SECRET_MARK: char = '🔒';

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    /// Où (fichier et entrée).
    pub context: String,
    pub message: String,
}

/// L'ensemble des problèmes trouvés.
#[derive(Clone, Debug, Default)]
pub struct ValidationReport {
    pub issues: Vec<Issue>,
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} problème(s) dans les données :", self.issues.len())?;
        for issue in &self.issues {
            writeln!(f, "  - {} : {}", issue.context, issue.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationReport {}

struct Checker {
    report: ValidationReport,
}

impl Checker {
    fn fail(&mut self, context: impl Into<String>, message: impl Into<String>) {
        self.report.issues.push(Issue {
            context: context.into(),
            message: message.into(),
        });
    }

    fn check(&mut self, ok: bool, context: impl Into<String>, message: impl Into<String>) {
        if !ok {
            self.fail(context, message);
        }
    }

    /// Une entrée canon cite sa section ; une proposition porte son numéro de registre.
    fn status(&mut self, context: &str, status: &Status, source: Option<&str>) {
        match status {
            Status::Canon | Status::AuthorDraft => {
                let ok = source.is_some_and(|s| s.trim_start().starts_with('§'));
                self.check(
                    ok,
                    context,
                    "entrée canon sans `source` (attendu par exemple « §3.3 »)",
                );
            }
            Status::Proposition(number) => {
                let ok = number
                    .strip_prefix('P')
                    .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
                self.check(
                    ok,
                    context,
                    format!("numéro de proposition invalide « {number} » (attendu « P12 »)"),
                );
            }
        }
    }
}

/// Collecte les doublons d'une liste d'identifiants.
fn duplicates<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen = BTreeSet::new();
    let mut dup = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            dup.insert(id);
        }
    }
    dup.into_iter().collect()
}

pub fn validate(data: &GameData) -> Result<(), ValidationReport> {
    let mut c = Checker {
        report: ValidationReport::default(),
    };
    let g = &data.galaxy;

    // --- Unicité -------------------------------------------------------------------------
    let place_ids = g
        .systems
        .iter()
        .map(|s| s.id.as_str())
        .chain(g.bodies.iter().map(|b| b.id.as_str()))
        .chain(g.stations.iter().map(|s| s.id.as_str()))
        .chain(g.phenomena.iter().map(|p| p.id.as_str()));
    for id in duplicates(place_ids) {
        c.fail(
            "galaxy.ron",
            format!("identifiant de lieu en double : {id}"),
        );
    }
    let checks: [(&str, Vec<&str>); 8] = [
        (
            "galaxy.ron (zones)",
            g.zones.iter().map(|z| z.id.as_str()).collect(),
        ),
        (
            "galaxy.ron (flottes)",
            g.fleets.iter().map(|f| f.id.as_str()).collect(),
        ),
        (
            "factions.ron",
            data.factions
                .factions
                .iter()
                .map(|f| f.id.as_str())
                .collect(),
        ),
        (
            "ship_classes.ron",
            data.ship_classes
                .classes
                .iter()
                .map(|s| s.id.as_str())
                .collect(),
        ),
        (
            "ship_classes.ron (catégories)",
            data.ship_classes
                .categories
                .iter()
                .map(|s| s.id.as_str())
                .collect(),
        ),
        (
            "ship_classes.ron (préfixes)",
            data.ship_classes
                .prefixes
                .iter()
                .map(|p| p.prefix.as_str())
                .collect(),
        ),
        (
            "formations.ron",
            data.formations
                .formations
                .iter()
                .map(|f| f.id.as_str())
                .collect(),
        ),
        (
            "currencies.ron",
            data.currencies
                .currencies
                .iter()
                .map(|m| m.id.as_str())
                .collect(),
        ),
    ];
    for (file, ids) in checks {
        for id in duplicates(ids) {
            c.fail(file, format!("identifiant en double : {id}"));
        }
    }

    // --- Index -----------------------------------------------------------------------------
    let systems: BTreeMap<&Id<Place>, _> = g.systems.iter().map(|s| (&s.id, s)).collect();
    let bodies: BTreeMap<&Id<Place>, _> = g.bodies.iter().map(|b| (&b.id, b)).collect();
    let stations: BTreeMap<&Id<Place>, _> = g.stations.iter().map(|s| (&s.id, s)).collect();
    let places: BTreeSet<&Id<Place>> = systems
        .keys()
        .chain(bodies.keys())
        .chain(stations.keys())
        .copied()
        .chain(g.phenomena.iter().map(|p| &p.id))
        .collect();
    let factions: BTreeSet<&Id<Faction>> = data.factions.factions.iter().map(|f| &f.id).collect();
    let secret_places: BTreeSet<&Id<Place>> = g
        .systems
        .iter()
        .filter(|s| s.secret)
        .map(|s| &s.id)
        .chain(g.bodies.iter().filter(|b| b.secret).map(|b| &b.id))
        .chain(g.stations.iter().filter(|s| s.secret).map(|s| &s.id))
        .collect();

    let faction_ref = |c: &mut Checker, ctx: &str, id: Option<&Id<Faction>>| {
        if let Some(id) = id {
            c.check(
                factions.contains(id),
                ctx,
                format!("faction inconnue : {id}"),
            );
        }
    };

    // --- Cases ------------------------------------------------------------------------------
    let cell_check = |c: &mut Checker,
                      ctx: &str,
                      cell: GridCell,
                      drawn_in: Option<GridCell>,
                      pos: MapPos| {
        let expected = drawn_in.unwrap_or(cell);
        match pos.cell() {
            Some(actual) if actual == expected => {}
            Some(actual) => c.fail(
                ctx,
                format!(
                    "case incohérente : les coordonnées ({}, {}) tombent en {actual}, attendu {expected}{}",
                    pos.x,
                    pos.y,
                    if drawn_in.is_some() { " (case de dessin)" } else { "" }
                ),
            ),
            None => c.fail(ctx, format!("coordonnées hors de la carte : ({}, {})", pos.x, pos.y)),
        }
    };

    // --- Zones -----------------------------------------------------------------------------
    for z in &g.zones {
        let ctx = format!("zone {}", z.id);
        c.status(&ctx, &z.status, z.source.as_deref());
        faction_ref(&mut c, &ctx, z.controller.as_ref());
        c.check(
            z.outline.is_empty() || z.outline.len() >= 3,
            &ctx,
            "contour de moins de 3 points",
        );
    }

    // --- Systèmes --------------------------------------------------------------------------
    for s in &g.systems {
        let ctx = format!("système {}", s.id);
        c.status(&ctx, &s.status, s.source.as_deref());
        faction_ref(&mut c, &ctx, s.controller.as_ref());
        cell_check(&mut c, &ctx, s.cell, None, s.center);
        for star in &s.stars {
            match bodies.get(star) {
                Some(b) => c.check(
                    b.kind == BodyKind::Star,
                    &ctx,
                    format!("{star} est listé comme étoile mais n'en est pas une"),
                ),
                None => c.fail(&ctx, format!("étoile inconnue : {star}")),
            }
        }
        for id in &s.bodies {
            match bodies.get(id) {
                Some(b) => c.check(
                    b.system.as_ref() == Some(&s.id),
                    &ctx,
                    format!("{id} est listé dans le système mais rattaché ailleurs"),
                ),
                None => c.fail(&ctx, format!("corps inconnu : {id}")),
            }
        }
        for id in &s.stations {
            match stations.get(id) {
                Some(st) => c.check(
                    st.system.as_ref() == Some(&s.id),
                    &ctx,
                    format!("station {id} listée dans le système mais rattachée ailleurs"),
                ),
                None => c.fail(&ctx, format!("station inconnue : {id}")),
            }
        }
    }

    // --- Astres ----------------------------------------------------------------------------
    for b in &g.bodies {
        let ctx = format!("astre {}", b.id);
        c.status(&ctx, &b.status, b.source.as_deref());
        faction_ref(&mut c, &ctx, b.controller.as_ref());
        match (b.cell, b.pos) {
            (Some(cell), Some(pos)) => cell_check(&mut c, &ctx, cell, b.drawn_in, pos),
            (None, Some(_)) => c.fail(&ctx, "coordonnées sans case canon"),
            (_, None) => c.check(
                b.drawn_in.is_none(),
                &ctx,
                "case de dessin sans coordonnées",
            ),
        }
        if let Some(sys_id) = &b.system {
            match systems.get(sys_id) {
                Some(sys) => {
                    c.check(
                        sys.bodies.contains(&b.id) || sys.stars.contains(&b.id),
                        &ctx,
                        format!("absent de la liste des corps de {sys_id}"),
                    );
                    c.check(
                        b.cell == Some(sys.cell),
                        &ctx,
                        format!(
                            "case canon différente de celle de son système ({})",
                            sys.cell
                        ),
                    );
                    c.check(
                        !sys.secret || b.secret,
                        &ctx,
                        format!("astre public dans le système secret {sys_id}"),
                    );
                }
                None => c.fail(&ctx, format!("système inconnu : {sys_id}")),
            }
        }
        if let Some(parent_id) = &b.parent {
            match bodies.get(parent_id) {
                Some(parent) => c.check(
                    parent.system == b.system,
                    &ctx,
                    format!("parent {parent_id} dans un autre système"),
                ),
                None => c.fail(&ctx, format!("parent inconnu : {parent_id}")),
            }
        }
    }

    // --- Stations --------------------------------------------------------------------------
    for s in &g.stations {
        let ctx = format!("station {}", s.id);
        c.status(&ctx, &s.status, s.source.as_deref());
        faction_ref(&mut c, &ctx, s.controller.as_ref());
        cell_check(&mut c, &ctx, s.cell, s.drawn_in, s.pos);
        if let Some(sys_id) = &s.system {
            match systems.get(sys_id) {
                Some(sys) => c.check(
                    !sys.secret || s.secret,
                    &ctx,
                    format!("station publique dans le système secret {sys_id}"),
                ),
                None => c.fail(&ctx, format!("système inconnu : {sys_id}")),
            }
        }
    }

    // --- Phénomènes --------------------------------------------------------------------------
    for p in &g.phenomena {
        let ctx = format!("phénomène {}", p.id);
        c.status(&ctx, &p.status, p.source.as_deref());
        if let (Some(cell), Some(pos)) = (p.cell, p.pos) {
            cell_check(&mut c, &ctx, cell, None, pos);
        }
        if let Some(sys_id) = &p.system {
            c.check(
                systems.contains_key(sys_id),
                &ctx,
                format!("système inconnu : {sys_id}"),
            );
        }
    }

    // --- Hubs de Portails ------------------------------------------------------------------
    for h in &g.portal_hubs {
        let ctx = format!("hub {}", h.body);
        c.status(&ctx, &h.status, h.source.as_deref());
        faction_ref(&mut c, &ctx, h.controller.as_ref());
        c.check(bodies.contains_key(&h.body), &ctx, "astre inconnu");
    }

    // --- Flottes ----------------------------------------------------------------------------
    for f in &g.fleets {
        let ctx = format!("flotte {}", f.id);
        c.status(&ctx, &f.status, f.source.as_deref());
        faction_ref(&mut c, &ctx, f.faction.as_ref());
        for loc in [&f.base, &f.position] {
            if let Location::Place(id) = loc {
                c.check(places.contains(id), &ctx, format!("lieu inconnu : {id}"));
                c.check(
                    !secret_places.contains(id) || f.secret,
                    &ctx,
                    format!("flotte publique dans le lieu secret {id}"),
                );
            }
        }
    }

    // --- Factions ----------------------------------------------------------------------------
    for f in &data.factions.factions {
        let ctx = format!("faction {}", f.id);
        c.status(&ctx, &f.status, f.source.as_deref());
        faction_ref(&mut c, &ctx, f.parent.as_ref());
        c.check(
            f.parent.as_ref() != Some(&f.id),
            &ctx,
            "est son propre parent",
        );
        if let Some(capital) = &f.capital {
            c.check(
                places.contains(capital),
                &ctx,
                format!("capitale inconnue : {capital}"),
            );
        }
        if let (Some(start), Some(end)) = (f.founded, f.ended) {
            c.check(start <= end, &ctx, "fin avant la fondation");
        }
    }
    // Pas de cycle dans l'arbre des organisations.
    for f in &data.factions.factions {
        let mut seen = BTreeSet::new();
        let mut current = f.parent.as_ref();
        while let Some(id) = current {
            if !seen.insert(id) {
                c.fail(format!("faction {}", f.id), "cycle dans les parents");
                break;
            }
            current = data.faction(id).and_then(|p| p.parent.as_ref());
        }
    }

    // --- Vaisseaux et formations -------------------------------------------------------------
    for s in &data.ship_classes.classes {
        let ctx = format!("classe {}", s.id);
        c.status(&ctx, &s.status, s.source.as_deref());
        for (what, r) in [("longueur", s.length_m), ("équipage", s.crew)] {
            c.check(
                r.max.is_none_or(|m| m >= r.min),
                &ctx,
                format!("{what} : min > max"),
            );
        }
    }
    for s in &data.ship_classes.categories {
        c.status(
            &format!("catégorie {}", s.id),
            &s.status,
            s.source.as_deref(),
        );
    }
    for p in &data.ship_classes.prefixes {
        let ctx = format!("préfixe {}", p.prefix);
        c.status(&ctx, &p.status, p.source.as_deref());
        faction_ref(&mut c, &ctx, p.faction.as_ref());
    }
    for f in &data.formations.formations {
        let ctx = format!("formation {}", f.id);
        c.status(&ctx, &f.status, f.source.as_deref());
        c.check(
            f.ships.max.is_none_or(|m| m >= f.ships.min),
            &ctx,
            "min > max",
        );
    }
    for m in &data.currencies.currencies {
        let ctx = format!("monnaie {}", m.id);
        c.status(&ctx, &m.status, m.source.as_deref());
        faction_ref(&mut c, &ctx, m.issuer.as_ref());
        for id in m.imposed_in.iter().chain(&m.banned_in) {
            faction_ref(&mut c, &ctx, Some(id));
        }
    }

    // --- Réglages -----------------------------------------------------------------------------
    for level in crate::PresetLevel::ALL {
        let p = data.graphics.get(level);
        let ctx = format!("graphics.ron ({level:?})");
        c.check(
            (0.25..=1.0).contains(&p.render_scale),
            &ctx,
            "render_scale hors de [0,25 ; 1]",
        );
        c.check(
            (1..=4).contains(&p.shadow_cascades),
            &ctx,
            "shadow_cascades hors de [1 ; 4]",
        );
        c.check(
            p.shadow_map_size.is_power_of_two(),
            &ctx,
            "shadow_map_size doit être une puissance de 2",
        );
        c.check(
            p.detail_distance > 0.0,
            &ctx,
            "detail_distance doit être positive",
        );
        c.check(
            (1..=16).contains(&p.anisotropy),
            &ctx,
            "anisotropy hors de [1 ; 16]",
        );
        c.check(
            p.decor_detail <= crate::GraphicsPreset::MAX_DECOR_DETAIL,
            &ctx,
            format!(
                "decor_detail hors de [0 ; {}]",
                crate::GraphicsPreset::MAX_DECOR_DETAIL
            ),
        );
    }
    c.check(
        data.clock.time_scale > 0.0,
        "clock.ron",
        "time_scale doit être positif",
    );
    c.check(
        data.space.recenter_distance.is_finite() && data.space.recenter_distance >= 0.0,
        "space.ron",
        "recenter_distance doit être positive ou nulle",
    );
    let sc = &data.space.scale;
    let lengths = [
        sc.au,
        sc.first_orbit_au,
        sc.star_radius,
        sc.planet_radius,
        sc.gas_giant_radius,
        sc.moon_radius,
        sc.moon_orbit_radii,
        sc.parking_orbit_radii,
        sc.cruise_rate,
        sc.cruise_min_speed,
        sc.manoeuvre_max_speed,
    ];
    c.check(
        lengths.iter().all(|v| v.is_finite() && *v > 0.0),
        "space.ron",
        "échelle des systèmes : toutes les valeurs doivent être positives",
    );
    c.check(
        sc.orbit_ratio > 1.0 && sc.cruise_max_speed > sc.cruise_min_speed,
        "space.ron",
        "échelle des systèmes : orbit_ratio > 1 et cruise_max_speed > cruise_min_speed",
    );
    c.check(
        sc.moon_radius < sc.planet_radius
            && sc.planet_radius < sc.gas_giant_radius
            && sc.orbit_radius(1) > sc.star_radius * 2.0,
        "space.ron",
        "échelle des systèmes : lune < planète < géante, premier rang hors de l'étoile",
    );

    // --- Secteurs ----------------------------------------------------------------------------
    for s in &data.sectors {
        let ctx = format!("secteur {}", s.id);
        c.status(&ctx, &s.status, s.source.as_deref());
        c.check(
            data.lang.has(&s.name_key),
            &ctx,
            format!("clé de nom absente de la langue : {}", s.name_key),
        );
        for problem in s.problems() {
            c.fail(&ctx, problem);
        }
        for cell in &s.cells {
            if let Some(key) = &cell.name_key {
                c.check(
                    data.lang.has(key),
                    &ctx,
                    format!(
                        "cellule {} : nom de salle absent de la langue : {key}",
                        cell.id
                    ),
                );
            }
        }
    }
    for id in duplicates(data.sectors.iter().map(|s| s.id.as_str())) {
        c.fail("secteurs", format!("identifiant en double : {id}"));
    }

    // --- Mises en scène de systèmes -------------------------------------------------------
    for scene in &data.scenes {
        let ctx = format!("scène {}", scene.system);
        c.status(&ctx, &scene.status, None);
        let Some(system) = data.galaxy.system(&scene.system) else {
            c.fail(&ctx, "système inconnu");
            continue;
        };
        c.check(
            !system.secret,
            &ctx,
            "un système secret n'a pas de mise en scène publique",
        );
        let in_system = |id: &str| {
            system
                .bodies
                .iter()
                .chain(&system.stations)
                .any(|b| b.as_str() == id)
        };
        for st in &scene.structures {
            c.check(
                in_system(st.around.as_str()),
                &ctx,
                format!("{} : astre {} hors du système", st.id, st.around),
            );
            c.check(
                data.lang.has(&st.name_key),
                &ctx,
                format!("clé de nom absente de la langue : {}", st.name_key),
            );
            c.check(
                st.altitude_radii > 0.0 && st.angle_deg.is_finite(),
                &ctx,
                format!("{} : altitude ou angle invalide", st.id),
            );
        }
        for id in duplicates(scene.structures.iter().map(|s| s.id.as_str())) {
            c.fail(&ctx, format!("structure en double : {id}"));
        }
        let near = &scene.arrival.near;
        c.check(
            in_system(near) || scene.structures.iter().any(|s| &s.id == near),
            &ctx,
            format!("arrivée près d'un objet inconnu : {near}"),
        );
        c.check(
            scene.arrival.distance > 0.0,
            &ctx,
            "distance d'arrivée invalide",
        );
        for station in scene.station_models.keys() {
            c.check(
                system.stations.iter().any(|s| s.as_str() == station),
                &ctx,
                format!("modèle pour une station hors du système : {station}"),
            );
        }
        if let Some(traffic) = &scene.traffic {
            c.check(traffic.speed > 0.0, &ctx, "vitesse du trafic invalide");
        }
        for dock in &scene.docks {
            c.check(
                in_system(dock.around.as_str()),
                &ctx,
                format!("port {} : astre {} hors du système", dock.id, dock.around),
            );
            c.check(
                data.sectors
                    .iter()
                    .any(|s| s.id.as_str() == dock.sector && s.external_port("@ship").is_some()),
                &ctx,
                format!(
                    "port {} : secteur sans point d'attache @ship : {}",
                    dock.id, dock.sector
                ),
            );
            // Le spatioport par défaut est partagé : à quai, le bandeau dit le nom du lieu, jamais
            // « Spatioport » tout court.
            c.check(
                dock.sector != crate::scene::DEFAULT_SPACEPORT || dock.place_key.is_some(),
                &ctx,
                format!(
                    "port {} : spatioport par défaut sans nom de lieu (place_key)",
                    dock.id
                ),
            );
            for key in std::iter::once(&dock.name_key).chain(dock.place_key.as_ref()) {
                c.check(
                    data.lang.has(key),
                    &ctx,
                    format!("port {} : clé de nom absente de la langue : {key}", dock.id),
                );
            }
            c.check(
                dock.range > 0.0 && dock.altitude_radii > 0.0,
                &ctx,
                format!("port {} : portée ou altitude invalide", dock.id),
            );
        }
    }
    for id in duplicates(data.scenes.iter().map(|s| s.system.as_str())) {
        c.fail("scènes", format!("système mis en scène deux fois : {id}"));
    }

    // --- Maniement des vaisseaux ------------------------------------------------------------
    for (ship, h) in &data.flight.ships {
        let ctx = format!("flight.ron : {ship}");
        c.check(
            data.sectors.iter().any(|s| s.id.as_str() == ship),
            &ctx,
            "secteur de vaisseau inconnu",
        );
        let values = [
            h.max_speed,
            h.accel_forward,
            h.accel_backward,
            h.accel_lateral,
            h.accel_vertical,
            h.pitch_rate_deg,
            h.yaw_rate_deg,
            h.roll_rate_deg,
            h.turn_response,
            h.cruise_spool_down,
        ];
        c.check(
            values.iter().all(|v| v.is_finite() && *v > 0.0),
            &ctx,
            "toutes les valeurs doivent être positives",
        );
        c.check(
            h.max_speed <= data.space.scale.manoeuvre_max_speed,
            &ctx,
            "max_speed au-delà de la vitesse de manœuvre de l'échelle (space.ron)",
        );
    }
    // Le maniement est celui de l'intérieur type (la classe du vaisseau), pas du départ.
    for start in &data.starts {
        c.check(
            data.flight.ships.contains_key(&start.interior),
            format!("départ {}", start.id),
            format!(
                "intérieur sans maniement dans flight.ron : {}",
                start.interior
            ),
        );
    }

    // --- Voyages ------------------------------------------------------------------------------
    let t = &data.travel;
    c.check(
        [
            t.hyperspace_days_per_unit,
            t.improvised_factor,
            t.nebula_margin,
            t.portal_transit_hours,
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.0)
            && t.portal_toll_credits >= 0,
        "travel.ron",
        "valeurs de voyage invalides",
    );
    for start in &data.starts {
        c.check(
            t.ships.contains_key(&start.ship),
            "travel.ron",
            format!("vaisseau sans logistique : {}", start.ship),
        );
    }
    c.status("portal_links.ron", &data.portal_links.status, None);
    let is_hub = |id: &Id<Place>| data.galaxy.portal_hubs.iter().any(|h| &h.body == id);
    for (a, b) in &data.portal_links.links {
        for end in [a, b] {
            c.check(
                is_hub(end),
                "portal_links.ron",
                format!("{end} ne porte pas de hub de Portails"),
            );
        }
        c.check(
            a != b,
            "portal_links.ron",
            format!("liaison de {a} vers lui-même"),
        );
    }

    // --- Simulation macro et rails ------------------------------------------------------------
    let place_known = |id: &Id<Place>| {
        data.galaxy.system(id).is_some()
            || data.galaxy.body(id).is_some()
            || data.galaxy.station(id).is_some()
    };
    let fleet_known = |id: &str| data.galaxy.fleets.iter().any(|f| f.id.as_str() == id);
    let faction_known = |id: &Id<Faction>| data.faction(id).is_some();
    let sim = &data.sim;
    c.status("sim.ron", &sim.status, None);
    for place in sim.economies.keys().chain(sim.stability.keys()) {
        c.check(
            place_known(place),
            "sim.ron",
            format!("lieu inconnu : {place}"),
        );
    }
    for convoy in &sim.convoys {
        c.check(
            place_known(&convoy.from) && place_known(&convoy.to) && faction_known(&convoy.faction),
            "sim.ron",
            format!("convoi {} : lieu ou faction inconnus", convoy.id),
        );
        c.check(
            sim.base_prices.contains_key(&convoy.commodity) && convoy.every_days > 0,
            "sim.ron",
            format!("convoi {} : marchandise ou fréquence invalide", convoy.id),
        );
        if let Some(escort) = &convoy.escort {
            c.check(
                fleet_known(escort.as_str()),
                "sim.ron",
                format!("escorte inconnue : {escort}"),
            );
        }
    }
    for r in &sim.raiders {
        c.check(
            fleet_known(r.fleet.as_str())
                && faction_known(&r.faction)
                && r.grounds.iter().all(place_known)
                && r.targets.iter().all(faction_known),
            "sim.ron",
            format!("pillards {} : référence inconnue", r.fleet),
        );
        let secret = data
            .galaxy
            .fleets
            .iter()
            .any(|f| f.id == r.fleet && f.secret);
        c.check(
            !secret || r.public_alias.is_some(),
            "sim.ron",
            format!(
                "pillards secrets {} sans nom public : leurs raids trahiraient le secret",
                r.fleet
            ),
        );
    }
    for (fleet, route) in &sim.patrols {
        c.check(
            fleet_known(fleet.as_str()) && route.iter().all(place_known),
            "sim.ron",
            format!("patrouille {fleet} : référence inconnue"),
        );
    }
    for fleet in sim.wanderers.iter().chain(sim.repairs.keys()) {
        c.check(
            fleet_known(fleet.as_str()),
            "sim.ron",
            format!("flotte inconnue : {fleet}"),
        );
    }
    for project in &sim.projects {
        c.check(
            faction_known(&project.owner) && place_known(&project.place),
            "sim.ron",
            format!("projet {} : référence inconnue", project.id),
        );
    }
    let rails = &data.rails;
    for stay in &rails.fleets_stay {
        c.check(
            fleet_known(stay.fleet.as_str()) && place_known(&stay.at),
            "rails.ron",
            format!("rail de flotte {} : référence inconnue", stay.fleet),
        );
    }
    for rail in &rails.projects_not_before {
        c.check(
            sim.projects.iter().any(|p| p.id == rail.project),
            "rails.ron",
            format!("rail de projet inconnu : {}", rail.project),
        );
    }

    // --- Cœur RPG ------------------------------------------------------------------------------
    let career = &data.career;
    c.status("career.ron", &career.status, None);
    c.check(
        faction_known(&career.contracts.issuer) && faction_known(&career.bank.lender),
        "career.ron",
        "émetteur des contrats ou prêteur inconnu",
    );
    c.check(
        career
            .reputation_tiers
            .windows(2)
            .all(|w| w[0].min < w[1].min)
            && career
                .reputation_tiers
                .iter()
                .all(|t| data.lang.has(&t.key)),
        "career.ron",
        "paliers de réputation non croissants ou sans texte",
    );
    for skill in &career.skills {
        c.check(
            data.lang.has(&format!("skill.{skill}")),
            "career.ron",
            format!("compétence sans texte : skill.{skill}"),
        );
    }
    c.status("ranks.ron", &data.ranks.status, None);
    for (faction, ladder) in &data.ranks.ladders {
        c.check(
            faction_known(faction),
            "ranks.ron",
            format!("faction inconnue : {faction}"),
        );
        for rank in ladder {
            c.check(
                data.lang.has(&rank.key),
                "ranks.ron",
                format!("grade sans texte : {}", rank.key),
            );
        }
    }

    // --- Combat -----------------------------------------------------------------------------
    let combat = &data.combat;
    c.status("combat.ron", &combat.status, None);
    for (key, ship) in &combat.ships {
        let ctx = format!("combat.ron : coque {key}");
        c.check(
            data.lang.has(&ship.name_key),
            &ctx,
            format!("texte absent : {}", ship.name_key),
        );
        for section in crate::SectionId::ALL {
            c.check(
                ship.sections.iter().filter(|s| s.id == section).count() == 1,
                &ctx,
                format!("section {} absente ou en double", section.key()),
            );
        }
        for mount in &ship.weapons {
            c.check(
                combat.ship_weapons.contains_key(&mount.weapon),
                &ctx,
                format!("arme inconnue : {}", mount.weapon),
            );
        }
    }
    for key in combat.raid.attackers.iter().chain(&combat.raid.defenders) {
        c.check(
            combat.ships.contains_key(key),
            "combat.ron : raid",
            format!("coque inconnue : {key}"),
        );
    }
    for (role, fighter) in &combat.fighters {
        let ctx = format!("combat.ron : combattant {role}");
        c.check(
            combat.firearms.contains_key(&fighter.weapon),
            &ctx,
            format!("arme inconnue : {}", fighter.weapon),
        );
        if let Some(armor) = &fighter.armor {
            c.check(
                combat.armors.contains_key(armor),
                &ctx,
                format!("protection inconnue : {armor}"),
            );
        }
    }
    for (key, firearm) in &combat.firearms {
        c.check(
            data.lang.has(&firearm.name_key),
            "combat.ron",
            format!("texte absent pour l'arme {key} : {}", firearm.name_key),
        );
    }
    c.check(
        faction_known(&combat.repair.issuer),
        "combat.ron",
        "chantier de réparation inconnu",
    );
    for system in &combat.repair.systems {
        c.check(
            data.galaxy.system(system).is_some_and(|s| !s.secret),
            "combat.ron",
            format!("système de réparation inconnu ou secret : {system}"),
        );
    }
    // Le carburant vendu au chantier sort des stocks du système (`SimCommand::Take`) : une
    // marchandise inconnue de la simulation serait prélevée dans le vide. Un prix ou une tranche
    // nuls rendraient le plein gratuit ou le bouton de tranche inopérant.
    let refuel = &combat.refuel;
    c.check(
        sim.base_prices.contains_key(&refuel.commodity),
        "combat.ron : refuel",
        format!(
            "marchandise inconnue de sim.ron (base_prices) : {}",
            refuel.commodity
        ),
    );
    c.check(
        refuel.day_price.is_finite() && refuel.day_price > 0.0,
        "combat.ron : refuel",
        format!(
            "prix d'un jour de carburant invalide : {}",
            refuel.day_price
        ),
    );
    c.check(
        refuel.step_days.is_finite() && refuel.step_days > 0.0,
        "combat.ron : refuel",
        format!("tranche de ravitaillement invalide : {}", refuel.step_days),
    );

    // --- Objets -----------------------------------------------------------------------------
    let items_ctx = crate::paths::ITEMS;
    for problem in data.items.problems() {
        c.fail(items_ctx, problem);
    }
    for item in &data.items.items {
        for key in [&item.name_key, &item.description_key] {
            c.check(
                data.lang.has(key),
                items_ctx,
                format!("{} : texte absent : {key}", item.id),
            );
        }
        let category = format!("item.category.{}", item.category.key());
        c.check(
            data.lang.has(&category),
            items_ctx,
            format!("{} : libellé de catégorie absent : {category}", item.id),
        );
        if let Some(slot) = item.slot {
            let slot = format!("item.slot.{}", slot.key());
            c.check(
                data.lang.has(&slot),
                items_ctx,
                format!("{} : libellé d'emplacement absent : {slot}", item.id),
            );
        }
        if let Some(firearm) = &item.firearm {
            c.check(
                data.combat.firearms.contains_key(firearm),
                items_ctx,
                format!("{} : arme inconnue de combat.ron : {firearm}", item.id),
            );
        }
    }

    // --- Apparence des astres ----------------------------------------------------------------
    // Les données seules (pas de dossier d'assets ici) : la présence des textures est vérifiée
    // par le test de `planets.rs`.
    for problem in data.planets.check(&data.galaxy, None) {
        c.fail(crate::paths::PLANETS, problem);
    }

    // --- Dialogues ---------------------------------------------------------------------------
    for id in duplicates(data.dialogues.iter().map(|d| d.id.as_str())) {
        c.fail("dialogues", format!("identifiant en double : {id}"));
    }
    for dialogue in &data.dialogues {
        let ctx = format!("dialogue {}", dialogue.id);
        c.status(&ctx, &dialogue.status, None);
        c.check(
            data.sectors
                .iter()
                .any(|s| s.id.as_str() == dialogue.sector),
            &ctx,
            format!("secteur inconnu : {}", dialogue.sector),
        );
        // Rattaché à un lieu : un port d'amarrage ou un système mis en scène.
        if let Some(place) = &dialogue.place {
            let known = data
                .scenes
                .iter()
                .any(|s| s.system.as_str() == place || s.docks.iter().any(|d| &d.id == place));
            c.check(
                known,
                &ctx,
                format!("lieu inconnu (ni port, ni système mis en scène) : {place}"),
            );
        }
        c.check(!dialogue.entries.is_empty(), &ctx, "aucun point d'entrée");
        for node in dialogue.missing_nodes() {
            c.fail(&ctx, format!("nœud inconnu : {node}"));
        }
        for key in dialogue.lang_keys() {
            c.check(data.lang.has(key), &ctx, format!("texte absent : {key}"));
        }
        for faction in dialogue.factions() {
            c.check(
                faction_known(faction),
                &ctx,
                format!("faction inconnue : {faction}"),
            );
        }
    }

    // --- Départs de partie ------------------------------------------------------------------
    c.check(!data.starts.is_empty(), "départs", "aucun départ de partie");
    for start in &data.starts {
        let ctx = format!("départ {}", start.id);
        c.status(&ctx, &start.status, None);
        c.check(
            data.galaxy.system(&start.system).is_some_and(|s| !s.secret),
            &ctx,
            format!("système inconnu ou secret : {}", start.system),
        );
        // Deux identifiants distincts (M9) : les statistiques du vaisseau et son intérieur type.
        c.check(
            data.combat.ship(&start.ship).is_some(),
            &ctx,
            format!(
                "vaisseau sans coque de combat dans combat.ron : {}",
                start.ship
            ),
        );
        match data
            .sectors
            .iter()
            .find(|s| s.id.as_str() == start.interior)
        {
            Some(ship) => {
                c.check(
                    ship.kind == crate::sector::SectorKind::Ship,
                    &ctx,
                    format!("intérieur qui n'est pas un vaisseau : {}", start.interior),
                );
                c.check(
                    ship.external_port("@dock").is_some(),
                    &ctx,
                    format!("intérieur sans sas d'amarrage @dock : {}", start.interior),
                );
                for spawn in [start.aboard_spawn.as_str(), start.first_spawn()] {
                    c.check(
                        ship.spawns.iter().any(|sp| sp.id == spawn),
                        &ctx,
                        format!("apparition inconnue à bord : {spawn}"),
                    );
                }
            }
            None => c.fail(&ctx, format!("intérieur inconnu : {}", start.interior)),
        }
        if let Some(hull) = &start.hull {
            c.check(
                hull.ends_with(".gltf") || hull.ends_with(".glb"),
                &ctx,
                format!("coque qui n'est pas un modèle glTF : {hull}"),
            );
        }
    }

    // --- Clavier ----------------------------------------------------------------------------
    c.check(
        data.keyboard
            .layouts
            .contains_key(&data.keyboard.default_layout),
        crate::paths::KEYBOARD,
        format!(
            "disposition par défaut inconnue : {}",
            data.keyboard.default_layout
        ),
    );

    // --- Langue -----------------------------------------------------------------------------
    for (key, text) in &data.lang.strings {
        // Les séparateurs de nombres sont des espaces par nature (« 12 500 »).
        c.check(
            !text.trim().is_empty() || key.starts_with("number."),
            data.lang
                .origins
                .get(key)
                .map_or(crate::paths::LANG, String::as_str),
            format!("texte vide pour {key}"),
        );
    }
    for dup in &data.lang.duplicates {
        if dup.first == dup.second {
            c.fail(&dup.first, format!("clé de texte en double : {}", dup.key));
        } else {
            c.fail(
                format!("{} et {}", dup.first, dup.second),
                format!("clé de texte en double : {}", dup.key),
            );
        }
    }

    // --- Secrets ------------------------------------------------------------------------------
    leaks(&mut c, "galaxy.ron", g);
    leaks(&mut c, "factions.ron", &data.factions);
    leaks(&mut c, "ship_classes.ron", &data.ship_classes);
    leaks(&mut c, "formations.ron", &data.formations);
    leaks(&mut c, "currencies.ron", &data.currencies);
    leaks(&mut c, crate::paths::ITEMS, &data.items);
    // La langue fusionne plusieurs fichiers (`lang/<code>.ron` et `lang/<code>/*.ron`).
    leaks(
        &mut c,
        &format!("{}/{} (fusionnée)", crate::paths::LANG_DIR, data.lang.code),
        &data.lang,
    );

    if c.report.issues.is_empty() {
        Ok(())
    } else {
        Err(c.report)
    }
}

/// Signale tout texte contenant 🔒 hors d'un champ `secret_notes`.
fn leaks<T: Serialize>(c: &mut Checker, file: &str, value: &T) {
    let parsed = ron::to_string(value)
        .ok()
        .and_then(|text| ron::from_str::<ron::Value>(&text).ok());
    match parsed {
        Some(v) => walk(c, file, &v, &mut Vec::new()),
        None => c.fail(file, "impossible d'inspecter le fichier pour les secrets"),
    }
}

fn walk(c: &mut Checker, file: &str, value: &ron::Value, path: &mut Vec<String>) {
    match value {
        ron::Value::String(s) if s.contains(SECRET_MARK) => c.fail(
            format!("{file} [{}]", path.join(".")),
            format!("passage secret 🔒 hors de `secret_notes` : « {s} »"),
        ),
        ron::Value::Map(map) => {
            for (k, v) in map.iter() {
                let key = match k {
                    ron::Value::String(s) => s.clone(),
                    other => format!("{other:?}"),
                };
                if key == "secret_notes" {
                    continue;
                }
                walk(c, file, k, path);
                path.push(key);
                walk(c, file, v, path);
                path.pop();
            }
        }
        ron::Value::Seq(items) => {
            for (i, v) in items.iter().enumerate() {
                path.push(i.to_string());
                walk(c, file, v, path);
                path.pop();
            }
        }
        ron::Value::Option(Some(v)) => walk(c, file, v, path),
        _ => {}
    }
}
