//! Les événements de la simulation (`SimEvent`) : ce qui arrive dans la galaxie, jour après jour.
//!
//! Chaque événement est **public** (il alimente les nouvelles, les rumeurs et la connaissance
//! du joueur) ou **secret** (il pilote la simulation, le joueur n'en voit que les effets, règle
//! d'or n°3). Le récit ([`SimEvent::narrate`]) est écrit en français pour le journal d'histoire
//! de l'auteur ; le jeu traduit les événements publics par ses fichiers de langue.
//!
//! Il n'existe aucun type d'événement pour l'Essaim, un vaisseau capitale ou une désassimilation :
//! les interdits durs de la v1 (AGENTS.md §7.2) sont impossibles par construction.

use sh_core::{Id, StellarDate};
use sh_data::{Faction, Fleet, Place};

#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    /// Une flotte arrive dans un lieu.
    FleetArrived { fleet: Id<Fleet>, at: Id<Place> },
    /// Une flotte en réparation est de nouveau opérationnelle.
    FleetRepaired { fleet: Id<Fleet> },
    /// Un convoi est pillé ; `raiders` : le nom public des pillards.
    ConvoyRaided {
        convoy: String,
        route: (Id<Place>, Id<Place>),
        owner: Id<Faction>,
        raiders: String,
        lost: f64,
    },
    /// Une attaque repoussée par l'escorte.
    ConvoyDefended {
        convoy: String,
        route: (Id<Place>, Id<Place>),
        owner: Id<Faction>,
        raiders: String,
        escort: Id<Fleet>,
    },
    /// Accrochage entre deux factions en guerre ; `winner` l'emporte.
    Skirmish {
        winner: Id<Faction>,
        loser: Id<Faction>,
        at: Id<Place>,
    },
    /// Une marchandise vient à manquer.
    Shortage { at: Id<Place>, commodity: String },
    /// La pénurie prend fin.
    Resupplied { at: Id<Place>, commodity: String },
    /// Une révolte éclate, puis elle est réprimée.
    Revolt { at: Id<Place> },
    /// Un projet franchit une étape (en pour cent) ou s'achève (100).
    ProjectMilestone {
        project: String,
        name: String,
        percent: u32,
    },
    /// L'Arène de Tharkast se tient.
    Arena { at: Id<Place> },
    /// La campagne présidentielle de l'UGL s'ouvre.
    UglCampaign { at: Id<Place> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimEvent {
    pub date: StellarDate,
    pub kind: EventKind,
    pub public: bool,
}

impl SimEvent {
    /// Le récit de l'événement, en français, pour le journal d'histoire. `name` donne le nom
    /// affichable d'un identifiant (lieu, flotte, faction).
    pub fn narrate(&self, name: &dyn Fn(&str) -> String) -> String {
        let when = format!("jour {}, {}", self.date.day_of_year(), self.date.year());
        let what = match &self.kind {
            EventKind::FleetArrived { fleet, at } => {
                format!("{} arrive à {}.", name(fleet.as_str()), name(at.as_str()))
            }
            EventKind::FleetRepaired { fleet } => {
                format!(
                    "{} sort de réparation : de nouveau opérationnelle.",
                    name(fleet.as_str())
                )
            }
            EventKind::ConvoyRaided {
                route,
                owner,
                raiders,
                lost,
                ..
            } => format!(
                "Le convoi {} → {} ({}) est pillé par {raiders} : {lost:.0} unités perdues.",
                name(route.0.as_str()),
                name(route.1.as_str()),
                name(owner.as_str())
            ),
            EventKind::ConvoyDefended {
                route,
                raiders,
                escort,
                ..
            } => format!(
                "{} repousse {raiders}, qui attaquaient le convoi {} → {}.",
                name(escort.as_str()),
                name(route.0.as_str()),
                name(route.1.as_str())
            ),
            EventKind::Skirmish { winner, loser, at } => format!(
                "Accrochage près de {} : {} l'emporte sur {}.",
                name(at.as_str()),
                name(winner.as_str()),
                name(loser.as_str())
            ),
            EventKind::Shortage { at, commodity } => {
                format!(
                    "Pénurie de {} à {}.",
                    commodity_fr(commodity),
                    name(at.as_str())
                )
            }
            EventKind::Resupplied { at, commodity } => {
                format!(
                    "Fin de la pénurie de {} à {}.",
                    commodity_fr(commodity),
                    name(at.as_str())
                )
            }
            EventKind::Revolt { at } => format!(
                "Révolte à {} ; elle est réprimée dans les jours qui suivent.",
                name(at.as_str())
            ),
            EventKind::ProjectMilestone {
                name: n, percent, ..
            } if *percent >= 100 => {
                format!("{n} est achevé.")
            }
            EventKind::ProjectMilestone {
                name: n, percent, ..
            } => {
                format!("{n} : chantier avancé à {percent} %.")
            }
            EventKind::Arena { at } => format!("L'Arène se tient à {}.", name(at.as_str())),
            EventKind::UglCampaign { at } => format!(
                "À {}, la campagne présidentielle de l'UGL s'ouvre : le second mandat de Nyra Maren touche à sa fin.",
                name(at.as_str())
            ),
        };
        let veil = if self.public { "" } else { " [🔒 secret]" };
        format!("{when} — {what}{veil}")
    }
}

/// Nom français d'une marchandise, pour le journal d'histoire.
pub fn commodity_fr(commodity: &str) -> &str {
    match commodity {
        "food" => "nourriture",
        "heavy_fuel" => "carburant lourd",
        "ore" => "minerai",
        "rare_earths" => "terres rares",
        "parts" => "pièces détachées",
        "weapons" => "armes",
        other => other,
    }
}
