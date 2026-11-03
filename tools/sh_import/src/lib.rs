//! Conversion de la carte canon (`_context/star-histories-carte.json`) en données du jeu
//! (`assets/data/canon/galaxy.ron`).
//!
//! La conversion est pure et déterministe : même JSON ⇒ même RON, octet pour octet. Le test
//! `galaxy_ron_est_a_jour` le vérifie contre le fichier versionné. Les choix d'interprétation
//! (rattachement d'un texte d'affiliation à une faction, isolement des passages 🔒) sont des
//! règles explicites de ce module, pas des retouches à la main.

use std::path::{Path, PathBuf};

use ron::ser::PrettyConfig;
use serde::Deserialize;
use sh_core::{GridCell, Id, MapPos};
use sh_data::galaxy::{
    Body, BodyKind, Fleet, Galaxy, MapFrame, OrbitOrder, Phenomenon, PortalHub, StarSystem,
    Station, Zone,
};
use sh_data::{Faction, Location, Place, Population, Status};
use thiserror::Error;

pub const DEFAULT_INPUT: &str = "_context/star-histories-carte.json";
pub const DEFAULT_OUTPUT: &str = "assets/data/canon/galaxy.ron";

const HEADER: &str = "\
// Généré par tools/sh_import depuis _context/star-histories-carte.json.
// NE PAS ÉDITER À LA MAIN : relancer l'import (voir tools/sh_import/README.md).
// Les champs *_note sont les textes canon d'origine : internes, jamais affichés au joueur.
";

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("JSON invalide ou de forme inattendue : {0}")]
    Json(#[from] serde_json::Error),
    #[error("identifiant « {0} » impossible à convertir")]
    Id(String),
    #[error("case « {value} » invalide pour {context}")]
    Cell { context: String, value: String },
    #[error("{context} : {message}")]
    Missing { context: String, message: String },
    #[error("écriture RON impossible : {0}")]
    Ron(#[from] ron::Error),
}

/// Racine du dépôt (les chemins par défaut sont relatifs à elle).
pub fn workspace_root() -> PathBuf {
    // tools/sh_import → racine : deux niveaux au-dessus du manifeste.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .unwrap_or(manifest)
        .to_path_buf()
}

// --- Forme du JSON source ---------------------------------------------------------------------
// `deny_unknown_fields` : si l'auteur ajoute un champ à l'export, l'import échoue et oblige à
// décider quoi en faire, au lieu de l'ignorer en silence.

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Carte {
    meta: Meta,
    repere: Repere,
    zones: Vec<JZone>,
    systemes: Vec<JSysteme>,
    astres: Vec<JAstre>,
    stations: Vec<JStation>,
    phenomenes: Vec<JPhenomene>,
    hubs_de_portails: Vec<JHub>,
    flottes_425at: Vec<JFlotte>,
}

#[derive(Deserialize)]
struct Meta {
    date_export: String,
}

#[derive(Deserialize)]
struct Repere {
    carte_svg: CarteSvg,
    galaxie: JGalaxie,
}

#[derive(Deserialize)]
struct CarteSvg {
    largeur: f64,
    hauteur: f64,
}

#[derive(Deserialize)]
struct JGalaxie {
    centre: (f64, f64),
    rayon_x: f64,
    rayon_y: f64,
}

#[derive(Deserialize)]
struct Xy {
    x: f64,
    y: f64,
}

impl From<&Xy> for MapPos {
    fn from(xy: &Xy) -> Self {
        MapPos::new(xy.x, xy.y)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JZone {
    id: String,
    nom: String,
    controle: String,
    cases: String,
    part_galaxie: Option<String>,
    description: String,
    contour_svg: Option<Vec<(f64, f64)>>,
    contour_note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JSysteme {
    id: String,
    nom: String,
    case: String,
    etoiles: Vec<String>,
    etoile_note: Option<String>,
    affiliation: String,
    note: String,
    centre_carte: Xy,
    corps: Vec<String>,
    stations: Vec<String>,
    population_totale_approx: u64,
    #[serde(default)]
    secret: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JPopulation {
    civile: String,
    militaire_forces_de_l_ordre: String,
    ouvriers_de_chantiers: String,
    total_approx: u64,
    note: Option<String>,
}

/// Rang orbital : un nombre, ou « dernier » (la plus extérieure).
#[derive(Deserialize)]
#[serde(untagged)]
enum JOrdre {
    Rank(u8),
    Word(String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JAstre {
    id: String,
    nom: String,
    categorie: String,
    #[serde(rename = "type")]
    kind: String,
    systeme: Option<String>,
    parent: Option<String>,
    ordre: Option<JOrdre>,
    ordre_note: Option<String>,
    case: Option<String>,
    case_dessin: Option<String>,
    affiliation: String,
    etat: String,
    role: String,
    xy_carte: Option<Xy>,
    xy_source: Option<String>,
    pos_note: Option<String>,
    population: Option<JPopulation>,
    note: Option<String>,
    #[serde(default)]
    secret: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JStation {
    id: String,
    nom: String,
    #[serde(rename = "type")]
    kind: String,
    taille: Option<String>,
    case: String,
    case_dessin: Option<String>,
    position_note: Option<String>,
    systeme: Option<String>,
    affiliation: String,
    etat: String,
    population: JPopulation,
    role: String,
    xy_carte: Xy,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JPhenomene {
    id: String,
    nom: String,
    #[serde(rename = "type")]
    kind: String,
    case: Option<String>,
    role: String,
    xy_carte: Option<Xy>,
    rayon_approx: Option<f64>,
    #[serde(rename = "ref")]
    reference: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JHub {
    astre: String,
    camp: String,
    role: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JFlotte {
    id: String,
    nom: String,
    surnom: Option<String>,
    entite: String,
    base: Option<String>,
    position_425at: Option<String>,
    etat: String,
    navire_amiral: Option<String>,
    commandant: Option<String>,
    composition: String,
    role: String,
    devise: Option<String>,
    #[serde(default)]
    secret: bool,
}

// --- Règles de conversion -----------------------------------------------------------------------

/// `sys-concordia` → `sys_concordia`.
fn id<T>(raw: &str) -> Result<Id<T>, ImportError> {
    Id::new(&raw.trim().replace('-', "_")).map_err(|_| ImportError::Id(raw.to_owned()))
}

fn cell(context: &str, raw: &str) -> Result<GridCell, ImportError> {
    raw.parse().map_err(|_| ImportError::Cell {
        context: context.to_owned(),
        value: raw.to_owned(),
    })
}

fn opt_cell(context: &str, raw: Option<&str>) -> Result<Option<GridCell>, ImportError> {
    raw.map(|r| cell(context, r)).transpose()
}

/// Rattache un texte d'affiliation canon à une faction des données.
///
/// Règle : le texte **commence par** le nom de l'organisation ; la première règle qui
/// correspond gagne (les plus précises d'abord). « aucune », « personne », « indépendante »
/// et les mentions collectives (« royaumes et indépendants ») ne rattachent à rien.
pub fn faction_of(affiliation: &str) -> Option<&'static str> {
    const RULES: &[(&str, &str)] = &[
        ("UGL (FOU)", "fou"),
        ("UGL (FDU)", "fdu"),
        ("UGL", "ugl"),
        ("espace UGL", "ugl"),
        ("Empire", "empire"),
        ("Hexa-Corp", "hexa_corp"),
        ("La Fédération", "federation"),
        ("Royaume Grothul", "royaume_grothul"),
        ("Ligue des Pirates", "ligue_des_pirates"),
        ("Syndicat Tosear", "syndicat_tosear"),
        ("Sœurs de Myrth", "soeurs_de_myrth"),
        ("pirates indépendants", "pirates_independants"),
    ];
    let text = affiliation.trim();
    RULES
        .iter()
        .find(|(prefix, _)| text.starts_with(prefix))
        .map(|(_, faction)| *faction)
}

fn faction_id(affiliation: &str) -> Result<Option<Id<Faction>>, ImportError> {
    faction_of(affiliation).map(id).transpose()
}

/// Sépare la partie publique d'un texte canon de ses passages 🔒.
///
/// Règles, appliquées à chaque marqueur :
/// - si le 🔒 est à l'intérieur d'une parenthèse, toute la parenthèse est secrète
///   (« légende d'une cache (vérité 🔒 non tranchée) ») ;
/// - sinon, **tout ce qui suit le marqueur jusqu'à la fin du champ** est secret. C'est la
///   lecture prudente : dans le canon, ce qui suit un 🔒 dans une même phrase appartient au
///   secret (« 🔒 cadence jamais mesurée ; berceau de la Flotte d'Attaque »).
///
/// Renvoie le texte public nettoyé et les passages secrets retirés.
pub fn split_secret(text: &str) -> (String, Vec<String>) {
    const MARK: char = '🔒';
    let mut public = text.to_owned();
    let mut secrets = Vec::new();
    while let Some(mark) = public.find(MARK) {
        let before = &public[..mark];
        // Parenthèse ouverte et non refermée avant le marqueur ?
        let open = before.rfind('(').filter(|&o| !before[o..].contains(')'));
        let (start, end) = match open {
            Some(o) => {
                let close = public[mark..]
                    .find(')')
                    .map_or(public.len(), |c| mark + c + ')'.len_utf8());
                (o, close)
            }
            None => (mark, public.len()),
        };
        secrets.push(public[start..end].trim().to_owned());
        public.replace_range(start..end, "");
    }
    if secrets.is_empty() {
        return (public, secrets);
    }
    // Nettoyage : espaces doublés, séparateur ou espace laissé en fin de texte.
    let mut cleaned = public.split_whitespace().collect::<Vec<_>>().join(" ");
    cleaned = cleaned.replace(" .", ".").replace(" ,", ",");
    while let Some(stripped) = cleaned
        .strip_suffix(';')
        .or_else(|| cleaned.strip_suffix(','))
    {
        cleaned = stripped.trim_end().to_owned();
    }
    if text.trim_end().ends_with('.') && !cleaned.is_empty() && !cleaned.ends_with('.') {
        cleaned.push('.');
    }
    (cleaned, secrets)
}

/// Applique [`split_secret`] à un champ et range les passages secrets, préfixés du nom du champ.
fn public(field: &str, text: &str, secrets: &mut Vec<String>) -> String {
    let (public, found) = split_secret(text);
    secrets.extend(found.into_iter().map(|s| format!("{field} : {s}")));
    public
}

fn population(p: &JPopulation, secrets: &mut Vec<String>) -> Population {
    Population {
        civil: public("population.civile", &p.civile, secrets),
        military: public(
            "population.militaire",
            &p.militaire_forces_de_l_ordre,
            secrets,
        ),
        shipyard: public("population.chantiers", &p.ouvriers_de_chantiers, secrets),
        total: p.total_approx,
        note: p
            .note
            .as_deref()
            .map(|n| public("population.note", n, secrets))
            .filter(|n| !n.is_empty()),
    }
}

fn location(raw: Option<&str>, known: &[Id<Place>]) -> Result<Location, ImportError> {
    let Some(raw) = raw.map(str::trim) else {
        return Ok(Location::Unknown);
    };
    match id::<Place>(raw) {
        Ok(candidate) if known.contains(&candidate) => Ok(Location::Place(candidate)),
        _ => Ok(Location::Unplaced(raw.to_owned())),
    }
}

fn source(section: &str) -> Option<String> {
    Some(section.to_owned())
}

/// Convertit le JSON de la carte en données de galaxie.
pub fn convert(json: &str) -> Result<Galaxy, ImportError> {
    let carte: Carte = serde_json::from_str(json)?;

    let zones = carte
        .zones
        .iter()
        .map(|z| {
            let mut secret_notes = Vec::new();
            let control_note = public("controle", &z.controle, &mut secret_notes);
            let description_note = public("description", &z.description, &mut secret_notes);
            Ok(Zone {
                id: id(&z.id)?,
                name: z.nom.clone(),
                controller: faction_id(&control_note)?,
                control_note,
                cells_note: z.cases.clone(),
                share_note: z.part_galaxie.clone(),
                description_note,
                outline: z
                    .contour_svg
                    .iter()
                    .flatten()
                    .map(|&(x, y)| MapPos::new(x, y))
                    .collect(),
                outline_note: z.contour_note.clone(),
                secret: false,
                secret_notes,
                status: Status::Canon,
                source: source("§3.2"),
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;

    let systems = carte
        .systemes
        .iter()
        .map(|s| {
            let ctx = format!("système {}", s.id);
            let mut secret_notes = Vec::new();
            let note = public("note", &s.note, &mut secret_notes);
            let affiliation_note = public("affiliation", &s.affiliation, &mut secret_notes);
            Ok(StarSystem {
                id: id(&s.id)?,
                name: s.nom.clone(),
                cell: cell(&ctx, &s.case)?,
                center: (&s.centre_carte).into(),
                stars: s.etoiles.iter().map(|e| id(e)).collect::<Result<_, _>>()?,
                star_note: s.etoile_note.clone(),
                controller: faction_id(&affiliation_note)?,
                affiliation_note,
                note,
                bodies: s.corps.iter().map(|e| id(e)).collect::<Result<_, _>>()?,
                stations: s.stations.iter().map(|e| id(e)).collect::<Result<_, _>>()?,
                population_total: s.population_totale_approx,
                secret: s.secret,
                secret_notes,
                status: Status::Canon,
                source: source("§3.3"),
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;

    let bodies = carte
        .astres
        .iter()
        .map(|a| {
            let ctx = format!("astre {}", a.id);
            let mut secret_notes = Vec::new();
            let kind = match (a.categorie.as_str(), a.systeme.is_some()) {
                ("étoile", _) => BodyKind::Star,
                ("lune", _) => BodyKind::Moon,
                ("ceinture", _) => BodyKind::Belt,
                ("planète errante", _) => BodyKind::RoguePlanet,
                ("planète", _) => BodyKind::Planet,
                (other, _) => {
                    return Err(ImportError::Missing {
                        context: ctx,
                        message: format!("catégorie inconnue « {other} »"),
                    });
                }
            };
            let affiliation_note = public("affiliation", &a.affiliation, &mut secret_notes);
            let state_note = public("etat", &a.etat, &mut secret_notes);
            let notes = [a.ordre_note.as_deref(), a.note.as_deref()]
                .into_iter()
                .flatten()
                .map(|n| public("note", n, &mut secret_notes))
                .collect();
            let pos_note = match (a.pos_note.as_deref(), a.xy_source.as_deref()) {
                (Some(p), _) | (None, Some(p)) => Some(p.to_owned()),
                (None, None) => None,
            };
            Ok(Body {
                id: id(&a.id)?,
                name: a.nom.clone(),
                kind,
                system: a.systeme.as_deref().map(id).transpose()?,
                parent: a.parent.as_deref().map(id).transpose()?,
                orbit_order: match &a.ordre {
                    None => None,
                    Some(JOrdre::Rank(n)) => Some(OrbitOrder::Rank(*n)),
                    Some(JOrdre::Word(w)) if w == "dernier" => Some(OrbitOrder::Outermost),
                    Some(JOrdre::Word(w)) => {
                        return Err(ImportError::Missing {
                            context: ctx,
                            message: format!("rang orbital inconnu « {w} »"),
                        });
                    }
                },
                cell: opt_cell(&ctx, a.case.as_deref())?,
                drawn_in: opt_cell(&ctx, a.case_dessin.as_deref())?,
                pos: a.xy_carte.as_ref().map(MapPos::from),
                pos_note,
                destroyed: a.etat.trim_start().starts_with("détruite"),
                population: a
                    .population
                    .as_ref()
                    .map(|p| population(p, &mut secret_notes)),
                controller: faction_id(&affiliation_note)?,
                kind_note: public("type", &a.kind, &mut secret_notes),
                affiliation_note,
                state_note,
                role_note: public("role", &a.role, &mut secret_notes),
                notes,
                secret: a.secret,
                secret_notes,
                status: Status::Canon,
                source: source(if a.systeme.is_some() {
                    "§3.3"
                } else {
                    "§3.4"
                }),
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;

    let stations = carte
        .stations
        .iter()
        .map(|s| {
            let ctx = format!("station {}", s.id);
            let mut secret_notes = Vec::new();
            let affiliation_note = public("affiliation", &s.affiliation, &mut secret_notes);
            Ok(Station {
                id: id(&s.id)?,
                name: s.nom.clone(),
                kind_note: public("type", &s.kind, &mut secret_notes),
                system: s.systeme.as_deref().map(id).transpose()?,
                cell: cell(&ctx, &s.case)?,
                drawn_in: opt_cell(&ctx, s.case_dessin.as_deref())?,
                pos: (&s.xy_carte).into(),
                position_note: s.position_note.clone(),
                size_note: s.taille.clone(),
                under_construction: s.etat.trim_start().starts_with("en construction"),
                controller: faction_id(&affiliation_note)?,
                affiliation_note,
                state_note: public("etat", &s.etat, &mut secret_notes),
                population: population(&s.population, &mut secret_notes),
                role_note: public("role", &s.role, &mut secret_notes),
                secret: false,
                secret_notes,
                status: Status::Canon,
                source: source("§3.5"),
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;

    let phenomena = carte
        .phenomenes
        .iter()
        .map(|p| {
            let mut secret_notes = Vec::new();
            let raw_cell = p.case.as_deref().map(str::trim);
            // « M6 (coin est) » → M6 ; « J7/K7 (jonction) » ne désigne pas une case unique.
            let parsed = raw_cell
                .and_then(|c| c.split_whitespace().next())
                .and_then(|c| c.parse::<GridCell>().ok());
            let cell_note = raw_cell
                .filter(|c| parsed.is_none_or(|cell| cell.to_string() != *c))
                .map(str::to_owned);
            Ok(Phenomenon {
                id: id(&p.id)?,
                name: p.nom.clone(),
                kind_note: public("type", &p.kind, &mut secret_notes),
                cell: parsed,
                cell_note,
                pos: p.xy_carte.as_ref().map(MapPos::from),
                radius: p.rayon_approx,
                system: p.reference.as_deref().map(id).transpose()?,
                blocks_hyperspace: p.kind.contains("bloque l'hyperespace"),
                role_note: public("role", &p.role, &mut secret_notes),
                secret: false,
                secret_notes,
                status: Status::Canon,
                source: source("§3.6"),
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;

    let portal_hubs = carte
        .hubs_de_portails
        .iter()
        .map(|h| {
            Ok(PortalHub {
                body: id(&h.astre)?,
                controller: faction_id(&h.camp)?,
                camp_note: h.camp.clone(),
                role_note: h.role.clone(),
                status: Status::Canon,
                source: source("§3.7"),
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;

    let known: Vec<Id<Place>> = systems
        .iter()
        .map(|s| s.id.clone())
        .chain(bodies.iter().map(|b| b.id.clone()))
        .chain(stations.iter().map(|s| s.id.clone()))
        .chain(phenomena.iter().map(|p| p.id.clone()))
        .collect();

    let fleets = carte
        .flottes_425at
        .iter()
        .map(|f| {
            let mut secret_notes = Vec::new();
            let mut opt = |field: &str, text: &Option<String>| {
                text.as_deref()
                    .map(|t| public(field, t, &mut secret_notes))
                    .filter(|t| !t.is_empty())
            };
            let nickname = opt("surnom", &f.surnom);
            let flagship_note = opt("navire_amiral", &f.navire_amiral);
            let commander_note = opt("commandant", &f.commandant);
            let motto = opt("devise", &f.devise);
            Ok(Fleet {
                id: id(&f.id)?,
                name: f.nom.clone(),
                nickname,
                faction: faction_id(&f.entite)?,
                entity_note: public("entite", &f.entite, &mut secret_notes),
                base: location(f.base.as_deref(), &known)?,
                position: location(f.position_425at.as_deref(), &known)?,
                state_note: public("etat", &f.etat, &mut secret_notes),
                flagship_note,
                commander_note,
                composition_note: public("composition", &f.composition, &mut secret_notes),
                role_note: public("role", &f.role, &mut secret_notes),
                motto,
                secret: f.secret,
                secret_notes,
                status: Status::Canon,
                source: source("§6.1"),
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;

    Ok(Galaxy {
        origin: format!("{DEFAULT_INPUT} (export du {})", carte.meta.date_export),
        frame: MapFrame {
            width: carte.repere.carte_svg.largeur,
            height: carte.repere.carte_svg.hauteur,
            center: carte.repere.galaxie.centre,
            radius_x: carte.repere.galaxie.rayon_x,
            radius_y: carte.repere.galaxie.rayon_y,
        },
        zones,
        systems,
        bodies,
        stations,
        phenomena,
        portal_hubs,
        fleets,
    })
}

/// Sérialise la galaxie en RON lisible, avec l'en-tête « fichier généré ».
pub fn render(galaxy: &Galaxy) -> Result<String, ImportError> {
    let config = PrettyConfig::new()
        .indentor("    ")
        // Fins de ligne LF sur toutes les plates-formes : le fichier versionné (normalisé par Git)
        // doit rester identique à la sortie de l'outil.
        .new_line("\n")
        .depth_limit(4)
        .struct_names(false)
        .separate_tuple_members(false)
        .escape_strings(false);
    let body = ron::ser::to_string_pretty(galaxy, config)?;
    Ok(format!("{HEADER}{body}\n"))
}

/// JSON → RON complet.
pub fn import(json: &str) -> Result<String, ImportError> {
    render(&convert(json)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_entre_parentheses() {
        let (p, s) = split_secret("personne (🔒 l'Essaim s'y reconstruit)");
        assert_eq!(p, "personne");
        assert_eq!(s, vec!["(🔒 l'Essaim s'y reconstruit)"]);
        let (p, s) = split_secret("légende d'une cache (vérité 🔒 non tranchée).");
        assert_eq!(p, "légende d'une cache.");
        assert_eq!(s, vec!["(vérité 🔒 non tranchée)"]);
    }

    #[test]
    fn secret_jusqu_a_la_fin_du_champ() {
        let (p, s) = split_secret("Chantier ; 🔒 cadence inconnue ; berceau de la Flotte.");
        assert_eq!(p, "Chantier.");
        assert_eq!(s, vec!["🔒 cadence inconnue ; berceau de la Flotte."]);
        let (p, s) = split_secret("🔒 tout est secret");
        assert_eq!(p, "");
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn texte_sans_secret_intact() {
        let texte = "Capitale ; hub (le plus fréquenté) ; rien à cacher.";
        assert_eq!(split_secret(texte), (texte.to_owned(), vec![]));
    }

    #[test]
    fn rattachement_aux_factions() {
        assert_eq!(faction_of("UGL — capitale"), Some("ugl"));
        assert_eq!(faction_of("UGL (FOU)"), Some("fou"));
        assert_eq!(faction_of("Empire (annexion)"), Some("empire"));
        assert_eq!(
            faction_of("Hexa-Corp — contrat CFM de Franchise"),
            Some("hexa_corp")
        );
        assert_eq!(faction_of("aucune (ruines en espace UGL)"), None);
        assert_eq!(faction_of("royaumes et indépendants"), None);
        assert_eq!(faction_of("indépendante, isolationniste"), None);
    }
}
