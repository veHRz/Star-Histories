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
//! genre `npc`). La conversation d'un PNJ est celle de son rôle **rattachée au lieu** où l'on
//! est (`place` : le port d'amarrage, puis son système), sinon celle du secteur où il se trouve,
//! sinon la première de ce rôle ([`Dialogue::for_role_at`]) : un spatioport par défaut, partagé
//! par plusieurs lieux (P7), a ainsi les interlocuteurs propres à chaque lieu, et à défaut ceux
//! d'un autre, sans dupliquer les dialogues.

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
    /// Le lieu auquel la conversation est rattachée : un port d'amarrage (`DockPort::id`) ou un
    /// système (`sys_hexis`). Les guichetiers d'Hexis ne parlent qu'au spatioport d'Hexis, même
    /// si d'autres lieux partagent le même secteur ; absent : tout lieu de ce secteur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
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
    /// une, sinon la première de ce rôle (spatioport par défaut ou propre à un lieu). Sans lieu :
    /// voir [`Dialogue::for_role_at`].
    pub fn for_role<'a>(
        dialogues: &'a [Dialogue],
        role: &str,
        sector: Option<&str>,
    ) -> Option<&'a Dialogue> {
        Self::for_role_at(dialogues, role, sector, &[])
    }

    /// La conversation d'un PNJ de comptoir de ce rôle, là où l'on est : d'abord celle rattachée
    /// au premier lieu de `places` qui en a une (du plus précis au plus large : le port, puis son
    /// système), puis celle du secteur sans lieu, puis celle du secteur, enfin la première de ce
    /// rôle.
    pub fn for_role_at<'a>(
        dialogues: &'a [Dialogue],
        role: &str,
        sector: Option<&str>,
        places: &[&str],
    ) -> Option<&'a Dialogue> {
        let of_role = || {
            dialogues
                .iter()
                .filter(move |d| d.role.as_deref() == Some(role))
        };
        let in_sector = |d: &&Dialogue| Some(d.sector.as_str()) == sector;
        places
            .iter()
            .find_map(|place| of_role().find(|d| d.place.as_deref() == Some(*place)))
            .or_else(|| of_role().find(|d| d.place.is_none() && in_sector(d)))
            .or_else(|| of_role().find(in_sector))
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
        lire_au(id, role, sector, None)
    }

    /// Le même, rattaché à un lieu.
    fn lire_au(id: &str, role: Option<&str>, sector: &str, place: Option<&str>) -> Dialogue {
        let role = role.map_or(String::new(), |r| format!("role: \"{r}\","));
        let place = place.map_or(String::new(), |p| format!("place: \"{p}\","));
        let text = format!(
            "(id: \"{id}\", {role} {place} sector: \"{sector}\", npc: \"pnj\", name_key: \"cle\",
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

    #[test]
    fn un_pnj_du_spatioport_par_defaut_parle_selon_le_lieu() {
        let dialogues = vec![
            lire_au("banque_hexis", Some("bank"), "spaceport", Some("sys_hexis")),
            lire_au(
                "banque_quai",
                Some("bank"),
                "spaceport",
                Some("quai_cygnara"),
            ),
            lire("banque_generique", Some("bank"), "spaceport"),
            lire_au("bar_hexis", Some("bar"), "spaceport", Some("sys_hexis")),
        ];
        let id = |places: &[&str]| {
            Dialogue::for_role_at(&dialogues, "bank", Some("spaceport"), places)
                .map(|d| d.id.as_str())
        };
        // Le port d'abord, puis son système.
        assert_eq!(id(&["quai_cygnara", "sys_cygnara"]), Some("banque_quai"));
        assert_eq!(id(&["port_hexis", "sys_hexis"]), Some("banque_hexis"));
        // Un lieu sans conversation propre : celle du secteur qui n'est rattachée à aucun lieu.
        assert_eq!(id(&["port_x", "sys_x"]), Some("banque_generique"));
        assert_eq!(id(&[]), Some("banque_generique"));
        // Repli par rôle : un lieu sans tavernier propre a celui d'Hexis.
        let bar = Dialogue::for_role_at(&dialogues, "bar", Some("spaceport"), &["sys_x"]);
        assert_eq!(bar.map(|d| d.id.as_str()), Some("bar_hexis"));
        assert_eq!(
            Dialogue::for_role(&dialogues, "bar", None).map(|d| d.id.as_str()),
            Some("bar_hexis")
        );
    }
}
