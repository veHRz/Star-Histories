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
    }
    for id in duplicates(data.sectors.iter().map(|s| s.id.as_str())) {
        c.fail("secteurs", format!("identifiant en double : {id}"));
    }

    // --- Langue -----------------------------------------------------------------------------
    for (key, text) in &data.lang.strings {
        c.check(
            !text.trim().is_empty(),
            "lang/en.ron",
            format!("texte vide pour {key}"),
        );
    }

    // --- Secrets ------------------------------------------------------------------------------
    leaks(&mut c, "galaxy.ron", g);
    leaks(&mut c, "factions.ron", &data.factions);
    leaks(&mut c, "ship_classes.ron", &data.ship_classes);
    leaks(&mut c, "formations.ron", &data.formations);
    leaks(&mut c, "currencies.ron", &data.currencies);
    leaks(&mut c, "lang/en.ron", &data.lang);

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
