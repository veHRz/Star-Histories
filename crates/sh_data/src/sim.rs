//! Données de la simulation macro (AGENTS.md §7) : réglages et état de départ proposés
//! (`gameplay/sim.ron`, P22) et rails du canon (`canon/rails.ron`).
//!
//! Les positions et états de départ des flottes viennent du canon (`galaxy.ron`, §6.1) ; ce
//! fichier n'ajoute que ce que le canon ne chiffre pas : économie, convois, routes de
//! patrouille, probabilités, vitesses d'avancement des projets.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sh_core::{Id, StellarDate};

use crate::common::{Place, Status};
use crate::factions::Faction;
use crate::galaxy::Fleet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimSettings {
    #[serde(default)]
    pub status: Status,
    /// Graine de la partie (même graine, mêmes commandes ⇒ même histoire).
    pub seed: u64,
    /// Prix de référence des marchandises (crédits par unité).
    pub base_prices: BTreeMap<String, f64>,
    /// Production et consommation journalières par système.
    pub economies: BTreeMap<Id<Place>, Economy>,
    pub convoys: Vec<ConvoyRoute>,
    pub raiders: Vec<Raiders>,
    pub skirmishes: Vec<Skirmish>,
    pub stability: BTreeMap<Id<Place>, StabilityRule>,
    /// Routes de patrouille (lieux visités tour à tour), par flotte.
    pub patrols: BTreeMap<Id<Fleet>, Vec<Id<Place>>>,
    /// Flottes qui parcourent toute la galaxie (Flotte Beta).
    pub wanderers: Vec<Id<Fleet>>,
    /// Flottes en réparation : état de départ (0 à 1) et gain par jour.
    pub repairs: BTreeMap<Id<Fleet>, Repair>,
    pub projects: Vec<ProjectDef>,
    pub calendar: Vec<CalendarDef>,
    /// Jours passés dans un système de patrouille avant de repartir.
    pub patrol_dwell_days: u32,
    /// Flottes dont la position canon n'est pas un lieu (« mobile », « chaque système ») : non
    /// simulées en M4.
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Economy {
    #[serde(default)]
    pub produces: BTreeMap<String, f64>,
    #[serde(default)]
    pub consumes: BTreeMap<String, f64>,
}

/// Un convoi régulier.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConvoyRoute {
    pub id: String,
    pub faction: Id<Faction>,
    pub from: Id<Place>,
    pub to: Id<Place>,
    pub commodity: String,
    pub amount: f64,
    pub every_days: u32,
    /// Flotte d'escorte, si elle est à l'une des extrémités au départ du convoi.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escort: Option<Id<Fleet>>,
}

/// Des pillards qui chassent les convois passant près de leurs terrains.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Raiders {
    pub fleet: Id<Fleet>,
    pub faction: Id<Faction>,
    pub grounds: Vec<Id<Place>>,
    /// Portée (unités de carte) entre un terrain et la route d'un convoi.
    pub range: f64,
    /// Probabilité d'attaque d'un convoi à portée.
    pub chance: f64,
    /// Probabilité de succès contre un convoi escorté.
    pub escorted_success: f64,
    /// Les factions dont ils attaquent les convois.
    pub targets: Vec<Id<Faction>>,
    /// Nom public de ces pillards quand la flotte elle-même est secrète (la « Grande
    /// Faucheuse » : la galaxie ignore qu'elle appartient au Syndicat Tosear).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_alias: Option<String>,
}

/// Escarmouches entre deux factions en guerre ouverte (Tosear contre la Ligue).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skirmish {
    pub a: Id<Faction>,
    pub b: Id<Faction>,
    pub places: Vec<Id<Place>>,
    pub chance_per_day: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StabilityRule {
    /// 0 : chaos, 1 : calme.
    pub initial: f64,
    /// Sous ce seuil, une révolte éclate.
    pub revolt_below: f64,
    /// Retour naturel vers le calme, par jour.
    pub recovery_per_day: f64,
    /// Pression quotidienne (occupation, faim, isolement).
    pub pressure_per_day: f64,
    /// Après une révolte réprimée, la stabilité revient à cette valeur.
    pub after_repression: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Repair {
    pub condition: f64,
    pub per_day: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectDef {
    pub id: String,
    /// Le nom tel qu'il s'affiche (nom propre du canon).
    pub name: String,
    pub owner: Id<Faction>,
    pub place: Id<Place>,
    /// Avancement de départ, de 0 à 1.
    pub progress: f64,
    /// Avancement par jour.
    pub per_day: f64,
    /// Accélération : l'avancement journalier croît de cette fraction par an (L'Enclume).
    #[serde(default)]
    pub acceleration_per_year: f64,
    /// Projet secret : ses étapes ne sont jamais publiques.
    #[serde(default)]
    pub secret: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarDef {
    pub date: StellarDate,
    pub kind: CalendarKind,
    pub place: Id<Place>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalendarKind {
    /// L'Arène de Tharkast (Fédération).
    Arena,
    /// Ouverture de la campagne présidentielle de l'UGL (fin du mandat de Nyra Maren).
    UglCampaign,
}

/// Les rails du canon : ce que la simulation laissée à elle-même ne doit pas faire.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rails {
    pub fleets_stay: Vec<FleetStay>,
    pub spares: Vec<Spare>,
    pub projects_not_before: Vec<ProjectRail>,
    /// Pas de guerre ouverte entre ces factions (la Fausse Paix armée).
    pub no_open_war: Vec<NoWar>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FleetStay {
    pub fleet: Id<Fleet>,
    pub at: Id<Place>,
    /// Jusqu'à cette date (toujours, si absente).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<StellarDate>,
    pub source: String,
}

/// `attacker` n'attaque jamais `spared` (la Ligue épargne l'Empire : 🔒, l'effet se voit).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spare {
    pub attacker: Id<Faction>,
    pub spared: Id<Faction>,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectRail {
    pub project: String,
    pub not_before: StellarDate,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoWar {
    pub a: Id<Faction>,
    pub b: Id<Faction>,
    pub source: String,
}
