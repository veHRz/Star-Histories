//! Dialogues en données (M5, GDD §3.4) : des nœuds, des choix, des conditions et des effets.
//!
//! Un dialogue appartient à un interlocuteur (`npc`) qui se souvient du joueur : les faits
//! mémorisés (`Remember`) et lus (`Knows`) vivent dans la fiche du capitaine. Le premier
//! point d'entrée dont les conditions tiennent ouvre la conversation, si bien que le même
//! guichetier accueille autrement un nouveau venu, un habitué ou un traître. Les répliques sont
//! des clés de langue.
//!
//! Deux portes d'entrée (M9) : une console du secteur (`console`, l'étiquette d'un
//! `INTERACT_console`) ou un PNJ de comptoir (`role`, le paramètre `role` d'un `INTERACT_*` de
//! genre `npc`). La conversation d'un PNJ est celle de son rôle dans le secteur où il se trouve,
//! sinon la première de ce rôle : un spatioport par défaut, partagé par plusieurs lieux, a ainsi
//! ses interlocuteurs sans dupliquer les dialogues.

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
    /// La console qui ouvre ce dialogue (étiquette `INTERACT_console` du secteur) ; vide : aucune
    /// (un dialogue tenu seulement par un PNJ de comptoir).
    #[serde(default)]
    pub console: String,
    /// Le rôle du PNJ de comptoir qui tient cette conversation (`contracts`, `bank`, `market`,
    /// `shipyard`, `bar`…) ; aucun : console seulement.
    #[serde(default)]
    pub role: Option<String>,
    /// Le secteur où se trouvent la console et le PNJ.
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
    /// Le compte et l'hypothèque (guichet de la Banque).
    Bank,
    /// Le devis et la réparation du vaisseau (chantier).
    Repair,
}

impl Dialogue {
    /// La conversation d'un PNJ de comptoir de ce rôle : celle du secteur `sector` s'il y en a
    /// une, sinon la première de ce rôle (spatioport par défaut ou propre à un lieu).
    pub fn for_role<'a>(
        dialogues: &'a [Dialogue],
        role: &str,
        sector: Option<&str>,
    ) -> Option<&'a Dialogue> {
        let of_role = || {
            dialogues
                .iter()
                .filter(move |d| d.role.as_deref() == Some(role))
        };
        of_role()
            .find(|d| Some(d.sector.as_str()) == sector)
            .or_else(|| of_role().next())
    }

    /// Le dialogue ouvre-t-il cet écran (par un effet d'arrivée ou de réponse) ?
    pub fn opens(&self, screen: Screen) -> bool {
        let opens = |effects: &[Effect]| effects.contains(&Effect::Open(screen));
        self.nodes
            .values()
            .any(|n| opens(&n.effects) || n.choices.iter().any(|c| opens(&c.effects)))
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dialogue minimal écrit comme les fichiers du jeu (console facultative, rôle en clair).
    fn lire(id: &str, role: Option<&str>, sector: &str) -> Dialogue {
        let role = role.map_or(String::new(), |r| format!("role: \"{r}\","));
        let text = format!(
            "(id: \"{id}\", {role} sector: \"{sector}\", npc: \"pnj\", name_key: \"cle\",
              entries: [(node: \"a\")],
              nodes: {{ \"a\": (text: \"cle\", choices: [(text: \"cle\", effects: [Open(Bank)])]) }})"
        );
        crate::ron_options()
            .from_str(&text)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    #[test]
    fn un_pnj_trouve_la_conversation_de_son_role() {
        let dialogues = vec![
            lire("console", None, "port_a"),
            lire("banque_a", Some("bank"), "port_a"),
            lire("banque_b", Some("bank"), "port_b"),
        ];
        assert!(dialogues[0].console.is_empty());
        let id = |sector| Dialogue::for_role(&dialogues, "bank", sector).map(|d| d.id.as_str());
        assert_eq!(id(Some("port_b")), Some("banque_b"));
        // Un secteur sans conversation propre prend la première du rôle.
        assert_eq!(id(Some("ailleurs")), Some("banque_a"));
        assert_eq!(id(None), Some("banque_a"));
        assert!(Dialogue::for_role(&dialogues, "bar", None).is_none());
        assert!(dialogues[1].opens(Screen::Bank));
        assert!(!dialogues[1].opens(Screen::Market));
    }
}
