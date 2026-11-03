//! Le capitaine du joueur (M5, GDD §3 et §5) : logique pure, testée sans `World`.
//!
//! Fiche (compétences par l'usage, traits), portefeuille crédit/Wuro au taux d'Hexis, soute,
//! réputations (une division rejaillit sur sa maison mère), grades, contrats Hexa-Corp
//! (CT, CR, CRS), dette envers la Banque d'Hexis, mémoire des PNJ. Chaque action renvoie ce
//! qu'elle a déclenché ([`Outcome`]) : le jeu en fait des messages et des nouvelles.
//!
//! La trahison d'un contrat (détourner sa cargaison) est toujours détectée : la cargaison est
//! enregistrée par Hexa-Corp, qui « traque les mauvais payeurs » (§6.5).

use std::collections::{BTreeMap, BTreeSet};

use sh_core::{Credits, Id, StellarDate, Wuro};
use sh_data::dialogue::{Condition, Dialogue};
use sh_data::{CareerRules, Faction, GameData, Place, Ranks};

use crate::SimWorld;
use crate::travel::{hyperspace_plan, portal_plan};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractKind {
    /// CT : transport.
    Transport,
    /// CR : ravitaillement (vers un système en pénurie, mieux payé).
    Resupply,
    /// CRS : remorquage d'une épave jusqu'aux chantiers.
    Towing,
}

impl ContractKind {
    /// Le sigle canon (§13).
    pub fn code(self) -> &'static str {
        match self {
            ContractKind::Transport => "CT",
            ContractKind::Resupply => "CR",
            ContractKind::Towing => "CRS",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractState {
    Active,
    Done,
    Failed,
    Betrayed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Contract {
    pub id: String,
    pub kind: ContractKind,
    pub issuer: Id<Faction>,
    pub from: Id<Place>,
    pub to: Id<Place>,
    pub commodity: Option<String>,
    pub amount: f64,
    pub reward: Credits,
    pub deadline: StellarDate,
    pub state: ContractState,
    /// Remorquage : l'épave est-elle accrochée ?
    pub towed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Loan {
    pub lender: Id<Faction>,
    pub balance: Credits,
    pub installment: Credits,
    pub next_due: StellarDate,
    pub missed: u32,
}

/// Ce qu'une action a déclenché.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    ContractAccepted {
        id: String,
    },
    ContractPaid {
        id: String,
        reward: Credits,
    },
    ContractFailed {
        id: String,
    },
    ContractBetrayed {
        id: String,
        indemnity: Credits,
    },
    ReputationChanged {
        faction: Id<Faction>,
        delta: i32,
    },
    Promoted {
        faction: Id<Faction>,
        rank: usize,
    },
    Demoted {
        faction: Id<Faction>,
        rank: usize,
    },
    /// Réputation trop basse pour le grade d'entrée : le joueur n'est plus membre (il peut
    /// revenir en regagnant sa confiance).
    Dismissed {
        faction: Id<Faction>,
    },
    /// Trahison : l'organisation ne traitera plus jamais avec le joueur.
    Expelled {
        faction: Id<Faction>,
    },
    TraitEarned {
        id: String,
    },
    SkillUp {
        skill: String,
        level: u32,
    },
    InstallmentPaid {
        balance: Credits,
    },
    InstallmentMissed {
        penalty: Credits,
    },
    LoanRepaid,
    Seized {
        amount: Credits,
    },
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum CareerError {
    #[error("pas assez de crédits")]
    NotEnoughCredits,
    #[error("pas assez de place en soute")]
    CargoFull,
    #[error("pas assez de marchandise")]
    NotEnoughGoods,
    #[error("contrat inconnu ou terminé")]
    NoSuchContract,
    #[error("pas à destination")]
    WrongPlace,
    #[error("l'organisation refuse de traiter avec vous")]
    Refused,
    #[error("contrat déjà pris")]
    AlreadyTaken,
    #[error("aucune dette")]
    NoDebt,
}

/// Traits gagnés par les actes (P9).
pub const WORD_KEPT: &str = "word_kept";
pub const NOTORIOUS_TRAITOR: &str = "notorious_traitor";
pub const BAD_PAYER: &str = "bad_payer";

#[derive(Clone, Debug, PartialEq)]
pub struct Captain {
    pub credits: Credits,
    pub wuro: Wuro,
    pub skills: BTreeMap<String, f64>,
    pub reputation: BTreeMap<Id<Faction>, i32>,
    /// Organisations rejointes et grade (indice dans l'échelle).
    pub ranks: BTreeMap<Id<Faction>, usize>,
    pub expelled: BTreeSet<Id<Faction>>,
    pub traits: BTreeSet<String>,
    /// Marchandises libres en soute (les cargaisons sous contrat sont dans les contrats).
    pub cargo: BTreeMap<String, f64>,
    pub cargo_capacity: f64,
    pub contracts: Vec<Contract>,
    pub completed: BTreeMap<Id<Faction>, u32>,
    pub loan: Option<Loan>,
    /// Mémoire des PNJ : ce que chaque interlocuteur sait du joueur.
    pub memory: BTreeMap<String, BTreeSet<String>>,
}

/// La maison mère d'une faction (une division d'Hexa-Corp → Hexa-Corp).
fn parent_of(data: &GameData, faction: &Id<Faction>) -> Option<Id<Faction>> {
    data.faction(faction).and_then(|f| f.parent.clone())
}

/// L'organisation dont l'échelle de grades vaut pour cette faction (elle-même ou sa mère).
fn ladder_owner(ranks: &Ranks, data: &GameData, faction: &Id<Faction>) -> Option<Id<Faction>> {
    if ranks.ladders.contains_key(faction) {
        return Some(faction.clone());
    }
    parent_of(data, faction).filter(|p| ranks.ladders.contains_key(p))
}

impl Captain {
    /// Le capitaine du départ de partie : argent, dette envers la Banque, réputations.
    pub fn from_start(data: &GameData, today: StellarDate) -> Self {
        let rules = &data.career;
        let start = data.start();
        let debt = start.map_or(0, |s| s.debt);
        Self {
            credits: Credits::units(start.map_or(0, |s| s.credits)),
            wuro: Wuro::ZERO,
            skills: rules.skills.iter().map(|s| (s.clone(), 0.0)).collect(),
            reputation: start.map(|s| s.reputations.clone()).unwrap_or_default(),
            ranks: BTreeMap::new(),
            expelled: BTreeSet::new(),
            traits: BTreeSet::new(),
            cargo: BTreeMap::new(),
            cargo_capacity: rules.cargo_capacity,
            contracts: Vec::new(),
            completed: BTreeMap::new(),
            loan: (debt > 0).then(|| Loan {
                lender: rules.bank.lender.clone(),
                balance: Credits::units(debt),
                installment: Credits::units(rules.bank.installment),
                next_due: today.add_days(i64::from(rules.bank.every_days)),
                missed: 0,
            }),
            memory: BTreeMap::new(),
        }
    }

    pub fn reputation(&self, faction: &Id<Faction>) -> i32 {
        self.reputation.get(faction).copied().unwrap_or(0)
    }

    /// Le palier de réputation (clé de langue).
    pub fn tier<'a>(&self, rules: &'a CareerRules, faction: &Id<Faction>) -> &'a str {
        let rep = self.reputation(faction);
        rules
            .reputation_tiers
            .iter()
            .rev()
            .find(|t| rep >= t.min)
            .map_or("", |t| t.key.as_str())
    }

    pub fn skill_level(&self, rules: &CareerRules, skill: &str) -> u32 {
        let xp = self.skills.get(skill).copied().unwrap_or(0.0);
        (xp / rules.xp_per_level).sqrt().floor() as u32
    }

    /// Progression par l'usage.
    pub fn practice(&mut self, rules: &CareerRules, skill: &str, xp: f64, out: &mut Vec<Outcome>) {
        let before = self.skill_level(rules, skill);
        *self.skills.entry(skill.to_owned()).or_default() += xp.max(0.0);
        let after = self.skill_level(rules, skill);
        if after > before {
            out.push(Outcome::SkillUp {
                skill: skill.to_owned(),
                level: after,
            });
        }
    }

    /// Change la réputation (bornée à ±100) ; la maison mère en reçoit sa part ; les grades
    /// suivent.
    pub fn change_reputation(
        &mut self,
        data: &GameData,
        faction: &Id<Faction>,
        delta: i32,
        out: &mut Vec<Outcome>,
    ) {
        let mut apply = |me: &mut Self, f: &Id<Faction>, d: i32| {
            if d == 0 {
                return;
            }
            let rep = me.reputation.entry(f.clone()).or_default();
            *rep = (*rep + d).clamp(-100, 100);
            out.push(Outcome::ReputationChanged {
                faction: f.clone(),
                delta: d,
            });
        };
        apply(self, faction, delta);
        if let Some(parent) = parent_of(data, faction) {
            let share = (f64::from(delta) * data.career.parent_share).round() as i32;
            apply(self, &parent, share);
        }
        self.update_ranks(data, out);
    }

    /// Grades : on monte quand réputation et contrats le permettent, on descend sinon. Une
    /// organisation trahie ne reprend jamais le joueur.
    fn update_ranks(&mut self, data: &GameData, out: &mut Vec<Outcome>) {
        let members: Vec<Id<Faction>> = self.ranks.keys().cloned().collect();
        for faction in members {
            let Some(ladder) = data.ranks.ladders.get(&faction) else {
                continue;
            };
            let rep = self.reputation(&faction);
            let done = self.completed.get(&faction).copied().unwrap_or(0);
            let wanted = ladder
                .iter()
                .rposition(|r| rep >= r.min_reputation && done >= r.min_contracts);
            let current = self.ranks.get(&faction).copied().unwrap_or(0);
            match wanted {
                Some(rank) if rank > current => {
                    self.ranks.insert(faction.clone(), rank);
                    out.push(Outcome::Promoted { faction, rank });
                }
                Some(rank) if rank < current => {
                    self.ranks.insert(faction.clone(), rank);
                    out.push(Outcome::Demoted { faction, rank });
                }
                None => {
                    self.ranks.remove(&faction);
                    out.push(Outcome::Dismissed { faction });
                }
                _ => {}
            }
        }
    }

    /// Cargaison totale en soute (libre et sous contrat).
    pub fn cargo_load(&self) -> f64 {
        // `+ 0.0` : la somme d'un itérateur vide de f64 vaut −0, affiché « -0 ».
        0.0 + self.cargo.values().sum::<f64>()
            + self
                .contracts
                .iter()
                .filter(|c| c.state == ContractState::Active)
                .map(|c| c.amount)
                .sum::<f64>()
    }

    /// L'organisation (ou sa mère) refuse-t-elle de traiter avec le joueur ?
    pub fn refused_by(&self, data: &GameData, faction: &Id<Faction>) -> bool {
        self.expelled.contains(faction)
            || parent_of(data, faction).is_some_and(|p| self.expelled.contains(&p))
    }

    /// Accepte un contrat : sa cargaison est chargée (fournie par l'émetteur).
    pub fn accept(
        &mut self,
        data: &GameData,
        contract: Contract,
    ) -> Result<Vec<Outcome>, CareerError> {
        if self.refused_by(data, &contract.issuer) {
            return Err(CareerError::Refused);
        }
        if self.contracts.iter().any(|c| c.id == contract.id) {
            return Err(CareerError::AlreadyTaken);
        }
        if self.cargo_load() + contract.amount > self.cargo_capacity + 1e-9 {
            return Err(CareerError::CargoFull);
        }
        let mut out = vec![Outcome::ContractAccepted {
            id: contract.id.clone(),
        }];
        // Le premier contrat fait du joueur un contractant (échelle de l'organisation).
        if let Some(owner) = ladder_owner(&data.ranks, data, &contract.issuer)
            && !self.ranks.contains_key(&owner)
            && !self.expelled.contains(&owner)
        {
            self.ranks.insert(owner, 0);
        }
        self.contracts.push(Contract {
            state: ContractState::Active,
            ..contract
        });
        self.update_ranks(data, &mut out);
        Ok(out)
    }

    fn active_mut(&mut self, id: &str) -> Result<&mut Contract, CareerError> {
        self.contracts
            .iter_mut()
            .find(|c| c.id == id && c.state == ContractState::Active)
            .ok_or(CareerError::NoSuchContract)
    }

    /// Livre un contrat à destination : paiement, réputation, compétences, traits, grades.
    pub fn deliver(
        &mut self,
        data: &GameData,
        id: &str,
        at: &Id<Place>,
        today: StellarDate,
    ) -> Result<Vec<Outcome>, CareerError> {
        let contract = self.active_mut(id)?;
        if &contract.to != at {
            return Err(CareerError::WrongPlace);
        }
        if contract.kind == ContractKind::Towing && !contract.towed {
            return Err(CareerError::WrongPlace);
        }
        let mut out = Vec::new();
        if today > contract.deadline {
            contract.state = ContractState::Failed;
            let (id, issuer) = (contract.id.clone(), contract.issuer.clone());
            out.push(Outcome::ContractFailed { id });
            self.change_reputation(data, &issuer, data.career.contracts.rep_failure, &mut out);
            return Ok(out);
        }
        contract.state = ContractState::Done;
        let (reward, issuer, cid) = (
            contract.reward,
            contract.issuer.clone(),
            contract.id.clone(),
        );
        self.credits += reward;
        out.push(Outcome::ContractPaid { id: cid, reward });
        // Les grades se gagnent auprès de l'organisation qui tient l'échelle (la maison mère).
        let owner = ladder_owner(&data.ranks, data, &issuer).unwrap_or_else(|| issuer.clone());
        *self.completed.entry(owner).or_default() += 1;
        let rules = &data.career;
        self.practice(rules, "commerce", 60.0, &mut out);
        self.practice(rules, "negotiation", 30.0, &mut out);
        let total: u32 = self
            .contracts
            .iter()
            .filter(|c| c.state == ContractState::Done)
            .count() as u32;
        if total >= rules.word_kept_after
            && !self.traits.contains(NOTORIOUS_TRAITOR)
            && self.traits.insert(WORD_KEPT.to_owned())
        {
            out.push(Outcome::TraitEarned {
                id: WORD_KEPT.to_owned(),
            });
        }
        self.change_reputation(data, &issuer, rules.contracts.rep_success, &mut out);
        Ok(out)
    }

    /// Détourne la cargaison d'un contrat (la vendre au marché) : trahison. Hexa-Corp le sait
    /// toujours : réputation, exclusion, indemnité ajoutée à la dette, « Traître notoire ».
    pub fn betray(
        &mut self,
        data: &GameData,
        id: &str,
        sale: Credits,
        today: StellarDate,
    ) -> Result<Vec<Outcome>, CareerError> {
        let contract = self.active_mut(id)?;
        contract.state = ContractState::Betrayed;
        let (cid, issuer) = (contract.id.clone(), contract.issuer.clone());
        let rules = &data.career;
        self.credits += sale;
        let indemnity =
            Credits::from_cents((sale.cents as f64 * rules.betrayal.indemnity_factor) as i64);
        let mut out = vec![Outcome::ContractBetrayed { id: cid, indemnity }];
        match self.loan.as_mut() {
            Some(loan) => loan.balance += indemnity,
            None => {
                self.loan = Some(Loan {
                    lender: rules.bank.lender.clone(),
                    balance: indemnity,
                    installment: Credits::units(rules.bank.installment),
                    next_due: today.add_days(i64::from(rules.bank.every_days)),
                    missed: 0,
                });
            }
        }
        self.traits.remove(WORD_KEPT);
        if self.traits.insert(NOTORIOUS_TRAITOR.to_owned()) {
            out.push(Outcome::TraitEarned {
                id: NOTORIOUS_TRAITOR.to_owned(),
            });
        }
        // La trahison ferme l'organisation, quelle que soit la réputation qui reste (avant la
        // perte de réputation, pour ne pas annoncer aussi un simple renvoi).
        if let Some(owner) = ladder_owner(&data.ranks, data, &issuer) {
            self.ranks.remove(&owner);
            if self.expelled.insert(owner.clone()) {
                out.push(Outcome::Expelled { faction: owner });
            }
        }
        self.change_reputation(data, &issuer, rules.betrayal.reputation, &mut out);
        self.remember("hexa_corp", "betrayed");
        Ok(out)
    }

    /// Accroche l'épave d'un contrat de remorquage.
    pub fn attach_tow(&mut self, id: &str) -> Result<(), CareerError> {
        let contract = self.active_mut(id)?;
        if contract.kind != ContractKind::Towing {
            return Err(CareerError::NoSuchContract);
        }
        contract.towed = true;
        Ok(())
    }

    pub fn buy(&mut self, commodity: &str, amount: f64, unit: f64) -> Result<Credits, CareerError> {
        if self.cargo_load() + amount > self.cargo_capacity + 1e-9 {
            return Err(CareerError::CargoFull);
        }
        let cost = Credits::from_cents((amount * unit * 100.0).round() as i64);
        let left = self
            .credits
            .checked_sub(cost)
            .ok_or(CareerError::NotEnoughCredits)?;
        if left.is_negative() {
            return Err(CareerError::NotEnoughCredits);
        }
        self.credits = left;
        *self.cargo.entry(commodity.to_owned()).or_default() += amount;
        Ok(cost)
    }

    pub fn sell(
        &mut self,
        commodity: &str,
        amount: f64,
        unit: f64,
    ) -> Result<Credits, CareerError> {
        let have = self.cargo.get(commodity).copied().unwrap_or(0.0);
        if have + 1e-9 < amount {
            return Err(CareerError::NotEnoughGoods);
        }
        let gain = Credits::from_cents((amount * unit * 100.0).round() as i64);
        self.credits += gain;
        let left = have - amount;
        if left <= 1e-9 {
            self.cargo.remove(commodity);
        } else {
            self.cargo.insert(commodity.to_owned(), left);
        }
        Ok(gain)
    }

    /// Change des crédits en Wuro au taux d'Hexis.
    pub fn exchange_to_wuro(
        &mut self,
        rules: &CareerRules,
        credits: Credits,
    ) -> Result<Wuro, CareerError> {
        let left = self
            .credits
            .checked_sub(credits)
            .ok_or(CareerError::NotEnoughCredits)?;
        if left.is_negative() {
            return Err(CareerError::NotEnoughCredits);
        }
        let rate = sh_core::ExchangeRate::from_credits_per_wuro(rules.credits_per_wuro);
        let wuro = rate.to_wuro(credits);
        self.credits = left;
        self.wuro += wuro;
        Ok(wuro)
    }

    /// Paie l'échéance due (ou la suivante, d'avance).
    pub fn pay_installment(&mut self, data: &GameData) -> Result<Vec<Outcome>, CareerError> {
        let loan = self.loan.as_mut().ok_or(CareerError::NoDebt)?;
        let due = if loan.balance.cents < loan.installment.cents {
            loan.balance
        } else {
            loan.installment
        };
        let left = self
            .credits
            .checked_sub(due)
            .ok_or(CareerError::NotEnoughCredits)?;
        if left.is_negative() {
            return Err(CareerError::NotEnoughCredits);
        }
        self.credits = left;
        loan.balance -= due;
        loan.next_due = loan
            .next_due
            .add_days(i64::from(data.career.bank.every_days));
        loan.missed = loan.missed.saturating_sub(1);
        let mut out = vec![Outcome::InstallmentPaid {
            balance: loan.balance,
        }];
        let lender = loan.lender.clone();
        if loan.balance.cents <= 0 {
            self.loan = None;
            out.push(Outcome::LoanRepaid);
        }
        self.change_reputation(data, &lender, 2, &mut out);
        Ok(out)
    }

    /// Le temps passe : échéances manquées (pénalité, puis saisie automatique) et délais des
    /// contrats dépassés.
    pub fn tick(&mut self, data: &GameData, today: StellarDate) -> Vec<Outcome> {
        let mut out = Vec::new();
        let rules = &data.career;
        if let Some(loan) = self.loan.as_mut()
            && today >= loan.next_due
        {
            let penalty = Credits::units(rules.bank.late_penalty);
            loan.balance += penalty;
            loan.missed += 1;
            loan.next_due = loan.next_due.add_days(i64::from(rules.bank.every_days));
            out.push(Outcome::InstallmentMissed { penalty });
            let lender = loan.lender.clone();
            if loan.missed >= rules.bank.seize_after_missed {
                // Saisie automatique : la Banque prend ce qu'il y a sur le compte.
                let amount = if self.credits.cents < loan.balance.cents {
                    self.credits
                } else {
                    loan.balance
                };
                loan.balance -= amount;
                loan.missed = 0;
                self.credits -= amount;
                out.push(Outcome::Seized { amount });
                if self.traits.insert(BAD_PAYER.to_owned()) {
                    out.push(Outcome::TraitEarned {
                        id: BAD_PAYER.to_owned(),
                    });
                }
            }
            self.change_reputation(data, &lender, -4, &mut out);
        }
        let overdue: Vec<(String, Id<Faction>)> = self
            .contracts
            .iter_mut()
            .filter(|c| c.state == ContractState::Active && today > c.deadline)
            .map(|c| {
                c.state = ContractState::Failed;
                (c.id.clone(), c.issuer.clone())
            })
            .collect();
        for (id, issuer) in overdue {
            out.push(Outcome::ContractFailed { id });
            self.change_reputation(data, &issuer, rules.contracts.rep_failure, &mut out);
        }
        out
    }

    pub fn remember(&mut self, npc: &str, fact: &str) {
        self.memory
            .entry(npc.to_owned())
            .or_default()
            .insert(fact.to_owned());
    }

    pub fn knows(&self, npc: &str, fact: &str) -> bool {
        self.memory.get(npc).is_some_and(|m| m.contains(fact))
    }

    /// Une condition de dialogue tient-elle ? (`npc` : l'interlocuteur, pour sa mémoire.)
    pub fn satisfies(
        &self,
        data: &GameData,
        npc: &str,
        condition: &Condition,
        today: StellarDate,
    ) -> bool {
        match condition {
            Condition::Knows(fact) => self.knows(npc, fact),
            Condition::NotKnows(fact) => !self.knows(npc, fact),
            Condition::Trait(t) => self.traits.contains(t),
            Condition::NoTrait(t) => !self.traits.contains(t),
            Condition::ReputationAtLeast(f, min) => self.reputation(f) >= *min,
            Condition::ReputationBelow(f, max) => self.reputation(f) < *max,
            Condition::Expelled(f) => self.refused_by(data, f),
            Condition::NotExpelled(f) => !self.refused_by(data, f),
            Condition::HasDebt => self.loan.is_some(),
            Condition::InstallmentDue => self.loan.as_ref().is_some_and(|l| today >= l.next_due),
        }
    }

    pub fn satisfies_all(
        &self,
        data: &GameData,
        npc: &str,
        conditions: &[Condition],
        today: StellarDate,
    ) -> bool {
        conditions
            .iter()
            .all(|c| self.satisfies(data, npc, c, today))
    }

    /// Le nœud par lequel l'interlocuteur ouvre la conversation.
    pub fn dialogue_entry<'a>(
        &self,
        data: &GameData,
        dialogue: &'a Dialogue,
        today: StellarDate,
    ) -> Option<&'a str> {
        dialogue
            .entries
            .iter()
            .find(|e| self.satisfies_all(data, &dialogue.npc, &e.when, today))
            .map(|e| e.node.as_str())
    }
}

/// Les offres d'un bureau de contrats Hexa-Corp dans un système : transports (CT) vers les
/// systèmes du monde vivant, ravitaillements (CR) vers ceux en pénurie, un remorquage (CRS)
/// sur place quand le système a des chantiers. Déterministe (même jour, mêmes offres).
pub fn contract_offers(
    data: &GameData,
    sim: &SimWorld,
    here: &Id<Place>,
    today: StellarDate,
) -> Vec<Contract> {
    let rules = &data.career.contracts;
    let ship = data.start().and_then(|s| data.travel.ships.get(&s.ship));
    let Some(ship) = ship else {
        return Vec::new();
    };
    let week = today.day_index().div_euclid(7);
    let produced: Vec<String> = data
        .sim
        .economies
        .get(here)
        .map(|e| e.produces.keys().cloned().collect())
        .unwrap_or_default();
    let fastest = |to: &Id<Place>| {
        let p = portal_plan(
            &data.galaxy,
            &data.portal_links,
            &data.travel,
            ship,
            here,
            to,
        )
        .map(|p| p.days());
        let h = hyperspace_plan(&data.galaxy, &data.travel, ship, here, to).map(|h| h.days());
        match (p, h) {
            (Some(p), Some(h)) => Some(p.min(h)),
            (p, h) => p.or(h),
        }
    };
    let mut routes = Vec::new();
    let capacity = data.career.cargo_capacity;
    for (i, (to, _)) in data.sim.economies.iter().enumerate() {
        if to == here || data.galaxy.system(to).is_none_or(|s| s.secret) {
            continue;
        }
        let Some(days) = fastest(to) else {
            continue;
        };
        // Pénurie à destination : un ravitaillement, mieux payé, de la marchandise qui manque.
        let shortage = sim
            .systems
            .get(to)
            .and_then(|s| s.shortages.iter().next().cloned());
        let (kind, commodity) = match shortage {
            Some(c) => (ContractKind::Resupply, c),
            None if !produced.is_empty() => (
                ContractKind::Transport,
                produced[(i as i64 + week).rem_euclid(produced.len() as i64) as usize].clone(),
            ),
            None => continue,
        };
        let amount = (capacity * (0.4 + 0.1 * (i as i64 + week).rem_euclid(5) as f64)).round();
        let base = data
            .sim
            .base_prices
            .get(&commodity)
            .copied()
            .unwrap_or(10.0);
        let mut reward = amount * base * rules.value_share + (rules.per_day as f64) * (days + 1.0);
        if kind == ContractKind::Resupply {
            reward *= rules.resupply_factor;
        }
        routes.push((
            days,
            Contract {
                id: format!("{}-{here}-{to}-{week}", kind.code()),
                kind,
                issuer: rules.issuer.clone(),
                from: here.clone(),
                to: to.clone(),
                commodity: Some(commodity),
                amount,
                reward: Credits::units(reward.round() as i64),
                deadline: today.add_days(
                    (days * rules.deadline_factor + rules.deadline_margin_days).ceil() as i64,
                ),
                state: ContractState::Active,
                towed: false,
            },
        ));
    }
    // Un bureau traite d'abord les routes de son voisinage : les plus proches destinations.
    routes.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.id.cmp(&b.1.id)));
    let mut offers: Vec<Contract> = routes.into_iter().map(|(_, c)| c).collect();
    // Remorquage : une épave à ramener aux chantiers du système (mise en scène), sur place.
    let has_yards = data.scene(here).is_some_and(|s| {
        s.structures
            .iter()
            .any(|st| st.kind == sh_data::StructureKind::Shipyard)
    });
    offers.truncate(rules.offers.saturating_sub(usize::from(has_yards)));
    if has_yards {
        offers.push(Contract {
            id: format!("CRS-{here}-{week}"),
            kind: ContractKind::Towing,
            issuer: rules.issuer.clone(),
            from: here.clone(),
            to: here.clone(),
            commodity: None,
            amount: 0.0,
            reward: Credits::units(rules.towing_reward),
            deadline: today.add_days(5),
            state: ContractState::Active,
            towed: false,
        });
    }
    offers.sort_by(|a, b| b.reward.cents.cmp(&a.reward.cents).then(a.id.cmp(&b.id)));
    offers
}
