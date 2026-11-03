//! Départs de partie (GDD §3.1) : un départ est un fichier de données. La v1 n'en a qu'un
//! (P1, validé : capitaine indépendant à Hexis) ; les débuts multiples de `_context/futur.md`
//! s'ajouteront à côté, sans changer le code.

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
    /// Secteur du vaisseau du joueur.
    pub ship: String,
    /// Point d'apparition à bord.
    pub aboard_spawn: String,
    /// Siège où le joueur commence, par son rôle (`pilot`), ou debout si absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat_role: Option<String>,
}
