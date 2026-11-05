//! Départs de partie (GDD §3.1) : un départ est un fichier de données. La v1 n'en a qu'un
//! (P1, validé : capitaine indépendant à Hexis) ; les débuts multiples de `_context/futur.md`
//! s'ajouteront à côté, sans changer le code.
//!
//! Le vaisseau du joueur y est décrit par trois choses distinctes (M9) : la clé de ses
//! statistiques (`ship` : carburant dans `travel.ron`, coque de combat dans `combat.ron`),
//! son **intérieur type** (`interior` : un secteur de genre `Ship`, partagé par tous les
//! vaisseaux de la classe, P12, dont le maniement est dans `flight.ron`) et sa coque extérieure
//! (`hull`), posée à quai dans le berceau du hangar.

use serde::{Deserialize, Serialize};
use sh_core::Id;

use crate::common::{Place, Status};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Start {
    pub id: String,
    #[serde(default)]
    pub status: Status,
    /// Système où commence la partie (vue Globale).
    pub system: Id<Place>,
    /// Clé des statistiques du vaisseau du joueur (`travel.ron`, `combat.ron`).
    pub ship: String,
    /// Secteur intérieur du vaisseau du joueur (intérieur type de sa classe, `flight.ron`).
    pub interior: String,
    /// Coque extérieure (sous `assets/`), même repère que l'intérieur : posée à quai seulement.
    /// Absente (ou fichier pas encore produit) : le vaisseau amarré n'a que son intérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hull: Option<String>,
    /// Point d'apparition aux commandes : on y reprend sa place après la carte, un voyage, un
    /// amarrage ou un départ, assis au poste de pilotage.
    pub aboard_spawn: String,
    /// Point d'apparition d'une nouvelle partie ; absent : [`Start::aboard_spawn`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_spawn: Option<String>,
    /// Siège où le joueur commence, par son rôle (`pilot`), ou debout si absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat_role: Option<String>,
    /// Argent de départ, en crédits.
    #[serde(default)]
    pub credits: i64,
    /// Dette envers la Banque (capital restant dû), en crédits.
    #[serde(default)]
    pub debt: i64,
    /// Réputations de départ.
    #[serde(default)]
    pub reputations: std::collections::BTreeMap<Id<crate::factions::Faction>, i32>,
}

impl Start {
    /// Où apparaît le joueur au début d'une nouvelle partie.
    pub fn first_spawn(&self) -> &str {
        self.first_spawn.as_deref().unwrap_or(&self.aboard_spawn)
    }
}
