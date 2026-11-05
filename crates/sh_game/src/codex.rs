//! Codex (M7, GDD §5.7) : les fiches des systèmes et des organisations que le joueur a
//! découverts. Elles ne disent que les champs **publics** du canon, ceux de la carte (contrôle,
//! population, Portails, astres, stations ; type, capitale, fondation, préfixe) : jamais un
//! secret 🔒, jamais de prose inventée (les résumés canon sont internes, `*_note`).
//!
//! Découverte : un système en y arrivant (et l'organisation qui le contrôle) ; une organisation
//! quand le joueur traite avec elle (réputation), la croise (raid, rescousse) ou parle à l'un de
//! ses représentants (M10 : la conversation d'un PNJ dont le dialogue porte une `faction`,
//! message [`SpokeWith`]).
//!
//! Noms (M10, demande de l'auteur) : une **division** s'affiche précédée de sa maison mère
//! (« Hexa-Corp Finance »), par la clé de format `faction.division` ; le nom canon reste court
//! (« Finance », `canon/factions.ron`). Tout écran qui nomme une organisation passe par
//! [`organization_name`].

use std::collections::BTreeSet;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use sh_core::Id;
use sh_data::{Dialogue, Faction, FactionKind, GameData, Lang, Place};

use crate::career::Career;
use crate::combat::Raid;
use crate::data::{Data, Texts};
use crate::state::AppState;
use crate::ui::hud::Notice;
use crate::voyage::Location;

pub struct CodexPlugin;

impl Plugin for CodexPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Codex>()
            .add_message::<SpokeWith>()
            .add_systems(
                Update,
                discover
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Data>),
            );
    }
}

/// Le joueur a ouvert cette conversation (identifiant du dialogue) : il connaît désormais
/// l'organisation que représente son interlocuteur.
#[derive(Message, Clone, Debug)]
pub struct SpokeWith(pub String);

/// Ce que le joueur a découvert.
#[derive(Resource, Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Codex {
    pub places: BTreeSet<Id<Place>>,
    pub factions: BTreeSet<Id<Faction>>,
}

/// Une fiche du codex.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Place(Id<Place>),
    Faction(Id<Faction>),
}

impl Codex {
    /// Découvre un système public et l'organisation qui le contrôle ; ses nouvelles fiches.
    pub fn discover_place(&mut self, data: &GameData, id: &Id<Place>) -> Vec<Entry> {
        let Some(system) = data.galaxy.system(id).filter(|s| !s.secret) else {
            return Vec::new();
        };
        let mut new = Vec::new();
        if self.places.insert(id.clone()) {
            new.push(Entry::Place(id.clone()));
        }
        if let Some(controller) = &system.controller {
            new.extend(self.discover_faction(data, controller));
        }
        new
    }

    /// Découvre une organisation publique et sa maison mère.
    pub fn discover_faction(&mut self, data: &GameData, id: &Id<Faction>) -> Vec<Entry> {
        let Some(faction) = data.faction(id).filter(|f| !f.secret) else {
            return Vec::new();
        };
        let mut new = Vec::new();
        if self.factions.insert(id.clone()) {
            new.push(Entry::Faction(id.clone()));
        }
        if let Some(parent) = &faction.parent {
            new.extend(self.discover_faction(data, parent));
        }
        new
    }

    /// Une conversation ouverte : l'organisation de l'interlocuteur et sa maison mère (le
    /// guichetier des contrats fait connaître la Logistique et Hexa-Corp).
    pub fn discover_speaker(&mut self, data: &GameData, dialogue: &Dialogue) -> Vec<Entry> {
        match &dialogue.faction {
            Some(faction) => self.discover_faction(data, faction),
            None => Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.places.len() + self.factions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Le nom affiché d'une organisation : une division (avec maison mère publique) porte celui de
/// sa maison mère devant le sien (« Hexa-Corp Finance », clé `faction.division`) ; les autres,
/// leur nom canon. Une branche militaire (« Flottes Officielles de l'Union ») se nomme déjà en
/// entier : elle garde son nom. Identifiant inconnu : l'identifiant lui-même.
pub fn organization_name(lang: &Lang, data: &GameData, id: &Id<Faction>) -> String {
    let Some(faction) = data.faction(id) else {
        return id.as_str().to_owned();
    };
    let parent = faction
        .parent
        .as_ref()
        .filter(|_| matches!(faction.kind, FactionKind::Division))
        .and_then(|p| data.faction(p))
        .filter(|p| !p.secret);
    match parent {
        Some(parent) => lang.format(
            "faction.division",
            &[("parent", &parent.name), ("name", &faction.name)],
        ),
        None => faction.name.clone(),
    }
}

/// Le nom d'un lieu s'il est public.
fn public_place(data: &GameData, id: &Id<Place>) -> Option<String> {
    let g = &data.galaxy;
    let public = g.system(id).is_some_and(|s| !s.secret)
        || g.body(id).is_some_and(|b| !b.secret)
        || g.station(id).is_some_and(|s| !s.secret);
    public
        .then(|| g.place_name(id).map(str::to_owned))
        .flatten()
}

fn kind_key(kind: FactionKind) -> &'static str {
    match kind {
        FactionKind::Power => "codex.kind.power",
        FactionKind::Corporation => "codex.kind.corporation",
        FactionKind::Division => "codex.kind.division",
        FactionKind::MilitaryBranch => "codex.kind.military",
        FactionKind::Syndicate => "codex.kind.syndicate",
        FactionKind::Kingdom => "codex.kind.kingdom",
        FactionKind::Pirates => "codex.kind.pirates",
        FactionKind::Order => "codex.kind.order",
        FactionKind::Guild => "codex.kind.guild",
        FactionKind::Mercenaries => "codex.kind.mercenaries",
        FactionKind::StationNation => "codex.kind.station_nation",
        FactionKind::Historical => "codex.kind.historical",
    }
}

/// Le titre et les lignes d'une fiche (champs publics seulement).
pub fn describe(texts: &Texts, data: &GameData, entry: &Entry) -> Option<(String, Vec<String>)> {
    let g = &data.galaxy;
    match entry {
        Entry::Place(id) => {
            let system = g.system(id).filter(|s| !s.secret)?;
            let mut lines = vec![texts.format("codex.cell", &[("cell", &system.cell.to_string())])];
            let public = sh_data::public_map(g);
            let map = public.iter().find(|e| &e.id == id);
            if let Some(name) = system
                .controller
                .as_ref()
                .and_then(|c| data.faction(c))
                .filter(|f| !f.secret)
            {
                lines.push(texts.format(
                    "codex.controller",
                    &[("faction", &organization_name(texts, data, &name.id))],
                ));
            }
            if let Some(population) = map.map(|e| e.population).filter(|p| *p > 0) {
                lines.push(texts.format(
                    "codex.population",
                    &[("population", &texts.number(population as f64, 0))],
                ));
            }
            if map.is_some_and(|e| e.has_portal_hub) {
                lines.push(texts.get("codex.portal_hub"));
            }
            let bodies: Vec<&str> = g
                .bodies
                .iter()
                .filter(|b| b.system.as_ref() == Some(id) && !b.secret)
                .map(|b| b.name.as_str())
                .collect();
            if !bodies.is_empty() {
                lines.push(texts.format("codex.bodies", &[("names", &bodies.join(", "))]));
            }
            let stations: Vec<&str> = g
                .stations
                .iter()
                .filter(|s| s.system.as_ref() == Some(id) && !s.secret)
                .map(|s| s.name.as_str())
                .collect();
            if !stations.is_empty() {
                lines.push(texts.format("codex.stations", &[("names", &stations.join(", "))]));
            }
            Some((system.name.clone(), lines))
        }
        Entry::Faction(id) => {
            let f = data.faction(id).filter(|f| !f.secret)?;
            let mut lines = vec![texts.get(kind_key(f.kind))];
            if let Some(parent) = f
                .parent
                .as_ref()
                .and_then(|p| data.faction(p))
                .filter(|p| !p.secret)
            {
                lines.push(texts.format(
                    "codex.parent",
                    &[("faction", &organization_name(texts, data, &parent.id))],
                ));
            }
            if let Some(capital) = f.capital.as_ref().and_then(|c| public_place(data, c)) {
                lines.push(texts.format("codex.capital", &[("place", &capital)]));
            }
            if let Some(year) = f.founded {
                lines.push(texts.format("codex.founded", &[("year", &year.to_string())]));
            }
            if let Some(year) = f.ended {
                lines.push(texts.format("codex.ended", &[("year", &year.to_string())]));
            }
            if let Some(prefix) = &f.ship_prefix {
                lines.push(texts.format("codex.prefix", &[("prefix", prefix)]));
            }
            Some((organization_name(texts, data, id), lines))
        }
    }
}

/// Le nom d'une fiche, pour les listes et les annonces (une division : « Hexa-Corp Finance »).
pub fn entry_name(lang: &Lang, data: &GameData, entry: &Entry) -> String {
    match entry {
        Entry::Place(id) => data.galaxy.place_name(id).unwrap_or(id.as_str()).to_owned(),
        Entry::Faction(id) => organization_name(lang, data, id),
    }
}

/// Ce que le joueur découvre : le système où il est, ceux avec qui il traite, ceux qu'il croise.
fn discover(
    data: Res<Data>,
    texts: Res<Texts>,
    mut codex: ResMut<Codex>,
    location: Option<Res<Location>>,
    career: Option<Res<Career>>,
    raid: Option<Res<Raid>>,
    mut spoke: MessageReader<SpokeWith>,
    mut notices: MessageWriter<Notice>,
) {
    // Ce qu'on sait en commençant (nouvelle partie) ne s'annonce pas.
    let quiet = codex.is_empty();
    let mut new = Vec::new();
    for SpokeWith(id) in spoke.read() {
        if let Some(dialogue) = data.dialogues.iter().find(|d| &d.id == id) {
            new.extend(codex.discover_speaker(&data, dialogue));
        }
    }
    if let Some(location) = location.filter(|l| l.is_changed()) {
        new.extend(codex.discover_place(&data, &location.system));
    }
    if let Some(career) = career.filter(|c| c.is_changed()) {
        for faction in career.reputation.keys() {
            new.extend(codex.discover_faction(&data, faction));
        }
    }
    if let Some(raid) = raid.filter(|r| r.is_added()) {
        let fleets = std::iter::once(&raid.encounter.raiders)
            .chain(raid.encounter.defenders.as_ref().map(|(f, _)| f));
        for fleet in fleets {
            let faction = data
                .galaxy
                .fleets
                .iter()
                .find(|f| &f.id == fleet && !f.secret)
                .and_then(|f| f.faction.clone());
            if let Some(faction) = faction {
                new.extend(codex.discover_faction(&data, &faction));
            }
        }
    }
    if quiet {
        return;
    }
    for entry in &new {
        notices.write(Notice(
            texts.format("codex.new", &[("name", &entry_name(&texts, &data, entry))]),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn donnees() -> GameData {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
        GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"))
    }

    fn id(text: &str) -> Id<Faction> {
        Id::new(text).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Une division porte le nom de sa maison mère ; le reste garde son nom canon.
    #[test]
    fn nom_affiche_d_une_division() {
        let data = donnees();
        let lang = &data.lang;
        assert_eq!(
            organization_name(lang, &data, &id("hc_finance")),
            "Hexa-Corp Finance"
        );
        assert_eq!(
            organization_name(lang, &data, &id("hc_logistique")),
            "Hexa-Corp Logistique"
        );
        assert_eq!(
            organization_name(lang, &data, &id("hexa_corp")),
            "Hexa-Corp"
        );
        // Une branche militaire a une maison mère mais se nomme déjà en entier.
        let fou = id("fou");
        if let Some(f) = data.faction(&fou) {
            assert_eq!(organization_name(lang, &data, &fou), f.name);
        }
        // Le nom canon reste court : seul l'affichage change.
        assert_eq!(
            data.faction(&id("hc_finance")).map(|f| f.name.as_str()),
            Some("Finance")
        );
        assert_eq!(
            entry_name(lang, &data, &Entry::Faction(id("hc_chantiers"))),
            "Hexa-Corp Chantiers"
        );
    }

    /// Parler aux guichetiers d'Hexis fait connaître leur division et Hexa-Corp ; un
    /// interlocuteur sans organisation (le tavernier) ne découvre rien.
    #[test]
    fn decouverte_a_la_conversation() {
        let data = donnees();
        let dialogue = |name: &str| {
            data.dialogues
                .iter()
                .find(|d| d.id == name)
                .unwrap_or_else(|| panic!("dialogue {name} absent"))
        };
        let mut codex = Codex::default();
        let new = codex.discover_speaker(&data, dialogue("hexis_contract_office"));
        assert_eq!(
            new,
            vec![
                Entry::Faction(id("hc_logistique")),
                Entry::Faction(id("hexa_corp"))
            ]
        );
        // La maison mère déjà connue ne s'annonce pas deux fois.
        assert_eq!(
            codex.discover_speaker(&data, dialogue("hexis_bank")),
            vec![Entry::Faction(id("hc_finance"))]
        );
        assert_eq!(
            codex.discover_speaker(&data, dialogue("hexis_shipyard")),
            vec![Entry::Faction(id("hc_chantiers"))]
        );
        assert!(
            codex
                .discover_speaker(&data, dialogue("hexis_contract_office"))
                .is_empty()
        );
        assert!(
            codex
                .discover_speaker(&data, dialogue("hexis_tavern"))
                .is_empty()
        );
    }

    /// Aucune fiche ne nomme un lieu secret ni ne cite une note interne ou secrète.
    #[test]
    fn le_codex_ne_montre_que_du_public() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
        let data = GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"));
        // La vraie langue : avec des clés manquantes, les fiches seraient vides et le test, vain.
        let lang =
            sh_data::read_ron(&dir.join(sh_data::paths::LANG)).unwrap_or_else(|e| panic!("{e}"));
        let texts = Texts(lang);
        let g = &data.galaxy;
        let secret_names: Vec<&str> = g
            .systems
            .iter()
            .filter(|s| s.secret)
            .map(|s| s.name.as_str())
            .chain(
                g.bodies
                    .iter()
                    .filter(|b| b.secret)
                    .map(|b| b.name.as_str()),
            )
            .chain(
                g.stations
                    .iter()
                    .filter(|s| s.secret)
                    .map(|s| s.name.as_str()),
            )
            .collect();
        let mut shown = 0;
        let entries = g.systems.iter().map(|s| Entry::Place(s.id.clone())).chain(
            data.factions
                .factions
                .iter()
                .map(|f| Entry::Faction(f.id.clone())),
        );
        for entry in entries {
            let Some((title, lines)) = describe(&texts, &data, &entry) else {
                continue;
            };
            shown += 1;
            let text = format!("{title}\n{}", lines.join("\n"));
            assert!(!text.contains('🔒'), "{text}");
            for name in &secret_names {
                assert!(
                    !text.contains(name),
                    "lieu secret « {name} » dans :\n{text}"
                );
            }
            if let Entry::Faction(id) = &entry
                && let Some(f) = data.faction(id)
            {
                assert!(!f.summary_note.is_empty() && !text.contains(&f.summary_note));
            }
        }
        assert!(shown > 20, "trop peu de fiches : {shown}");
        // Les systèmes secrets n'ont pas de fiche, même découverts par erreur.
        let mut codex = Codex::default();
        for s in g.systems.iter().filter(|s| s.secret) {
            assert!(codex.discover_place(&data, &s.id).is_empty());
        }
    }
}
