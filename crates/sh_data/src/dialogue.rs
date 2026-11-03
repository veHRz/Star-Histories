//! Dialogues en données (M5, GDD §3.4) : des nœuds, des choix, des conditions et des effets.
//!
//! Un dialogue appartient à un interlocuteur (`npc`) qui se souvient du joueur : les faits
//! mémorisés (`Remember`) et lus (`Knows`) vivent dans la fiche du capitaine. Le premier
//! point d'entrée dont les conditions tiennent ouvre la conversation, si bien que le même
//! guichetier accueille autrement un nouveau venu, un habitué ou un traître. Les répliques sont
//! des clés de langue.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sh_core::Id;

use crate::common::Status;
use crate::factions::Faction;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dialogue {
    pub id: String,
    #[serde(default)]
    pub status: Status,
    /// La console qui ouvre ce dialogue (étiquette `INTERACT_console` du secteur).
    pub console: String,
    /// Le secteur où se trouve la console.
    pub sector: String,
    /// L'interlocuteur, clé de sa mémoire.
    pub npc: String,
    /// Clé de langue de son nom ou de sa fonction.
    pub name_key: String,
    /// L'organisation qu'il représente (pour les conditions de réputation par défaut).
    #[serde(default)]
    pub faction: Option<Id<Faction>>,
    /// Points d'entrée, dans l'ordre : le premier dont les conditions tiennent.
    pub entries: Vec<Entry>,
    pub nodes: BTreeMap<String, Node>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    #[serde(default)]
    pub when: Vec<Condition>,
    pub node: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    /// Réplique de l'interlocuteur (clé de langue).
    pub text: String,
    /// Effets appliqués en arrivant sur le nœud.
    #[serde(default)]
    pub effects: Vec<Effect>,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    /// Réplique du joueur (clé de langue).
    pub text: String,
    /// Le choix n'apparaît que si ces conditions tiennent.
    #[serde(default)]
    pub when: Vec<Condition>,
    #[serde(default)]
    pub effects: Vec<Effect>,
    /// Nœud suivant ; aucun : la conversation se termine (ou l'effet ouvre un écran).
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    /// L'interlocuteur se souvient de ce fait.
    Knows(String),
    NotKnows(String),
    /// Le joueur porte ce trait.
    Trait(String),
    NoTrait(String),
    ReputationAtLeast(Id<Faction>, i32),
    ReputationBelow(Id<Faction>, i32),
    /// L'organisation (ou sa maison mère) a exclu le joueur.
    Expelled(Id<Faction>),
    NotExpelled(Id<Faction>),
    HasDebt,
    /// Une échéance est due ou en retard.
    InstallmentDue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// L'interlocuteur retient ce fait.
    Remember(String),
    /// Ouvre un écran du terminal.
    Open(Screen),
    /// Paie l'échéance de la dette.
    PayInstallment,
    /// Lit les dernières nouvelles (le tavernier, les rumeurs).
    News,
}

/// Les écrans du terminal qu'un dialogue peut ouvrir.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Contracts,
    Market,
    Sheet,
    Journal,
}

impl Dialogue {
    /// Toutes les clés de langue citées.
    pub fn lang_keys(&self) -> Vec<&str> {
        let mut keys = vec![self.name_key.as_str()];
        for node in self.nodes.values() {
            keys.push(&node.text);
            keys.extend(node.choices.iter().map(|c| c.text.as_str()));
        }
        keys
    }

    /// Les nœuds cités (entrées et choix) qui n'existent pas.
    pub fn missing_nodes(&self) -> Vec<&str> {
        let cited = self.entries.iter().map(|e| e.node.as_str()).chain(
            self.nodes
                .values()
                .flat_map(|n| n.choices.iter().filter_map(|c| c.next.as_deref())),
        );
        cited.filter(|n| !self.nodes.contains_key(*n)).collect()
    }

    /// Les organisations citées par les conditions.
    pub fn factions(&self) -> Vec<&Id<Faction>> {
        let conditions = self.entries.iter().flat_map(|e| e.when.iter()).chain(
            self.nodes
                .values()
                .flat_map(|n| n.choices.iter().flat_map(|c| c.when.iter())),
        );
        conditions
            .filter_map(|c| match c {
                Condition::ReputationAtLeast(f, _)
                | Condition::ReputationBelow(f, _)
                | Condition::Expelled(f)
                | Condition::NotExpelled(f) => Some(f),
                _ => None,
            })
            .chain(self.faction.iter())
            .collect()
    }
}
