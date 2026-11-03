//! Règles du cœur RPG (M5) : `gameplay/career.ron` (P23) et échelles de
//! grades (`propositions/ranks.ron`, P24 ; Q4 : grades inférieurs non canon, provisoires).
//!
//! Le canon fixe le cadre (§4.4) : contrats Hexa-Corp par sigles (CT, CR, CRS : division
//! Logistique), la Banque de la division Finance et ses saisies, la Flotte Beta qui traque les
//! mauvais payeurs, « deux contrats Hexa-Corp ne s'affrontent jamais ». Les chiffres sont proposés.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sh_core::Id;

use crate::common::Status;
use crate::factions::Faction;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CareerRules {
    #[serde(default)]
    pub status: Status,
    /// Les compétences (GDD §3.2), progression par l'usage.
    pub skills: Vec<String>,
    /// Expérience pour le niveau n : `xp_per_level × n²`.
    pub xp_per_level: f64,
    /// Paliers de réputation, du plus bas au plus haut : seuil et clé de langue.
    pub reputation_tiers: Vec<ReputationTier>,
    /// Part d'un changement de réputation auprès d'une division qui rejaillit sur sa maison mère.
    pub parent_share: f64,
    /// Taux d'Hexis : crédits pour un Wuro (vide du canon, §11.1 : proposé).
    pub credits_per_wuro: f64,
    /// Capacité de la soute du vaisseau du joueur (unités de marchandise).
    pub cargo_capacity: f64,
    /// Écart d'achat et de vente autour du prix du marché (0,05 : ±5 %).
    pub market_spread: f64,
    pub contracts: ContractRules,
    pub betrayal: BetrayalRules,
    pub bank: BankRules,
    /// Contrats honorés pour gagner le trait « Parole tenue » (P9).
    pub word_kept_after: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReputationTier {
    pub min: i32,
    pub key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractRules {
    /// L'émetteur des contrats CT, CR et CRS (la division Logistique, §4.4).
    pub issuer: Id<Faction>,
    /// Récompense d'un transport : par unité (en part du prix de référence) et par jour de route.
    pub value_share: f64,
    pub per_day: i64,
    /// Un ravitaillement vers un système en pénurie paie davantage.
    pub resupply_factor: f64,
    /// Récompense fixe d'un remorquage.
    pub towing_reward: i64,
    /// Délai : durée du trajet le plus rapide × facteur, plus une marge en jours.
    pub deadline_factor: f64,
    pub deadline_margin_days: f64,
    pub rep_success: i32,
    pub rep_failure: i32,
    /// Nombre maximal d'offres affichées par bureau.
    pub offers: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BetrayalRules {
    /// Réputation perdue auprès de l'émetteur (la maison mère en perd sa part).
    pub reputation: i32,
    /// Indemnité exigée, en multiple de la valeur de la cargaison détournée.
    pub indemnity_factor: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BankRules {
    /// La Banque (division Finance, §4.4).
    pub lender: Id<Faction>,
    pub installment: i64,
    pub every_days: u32,
    pub late_penalty: i64,
    /// Échéances manquées avant la saisie automatique (canon : hypothèque, « saisie
    /// automatique »).
    pub seize_after_missed: u32,
}

/// Échelles de grades par organisation (P24).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ranks {
    #[serde(default)]
    pub status: Status,
    pub ladders: BTreeMap<Id<Faction>, Vec<RankDef>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankDef {
    /// Clé de langue du titre.
    pub key: String,
    pub min_reputation: i32,
    pub min_contracts: u32,
}
