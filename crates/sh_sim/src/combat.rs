//! Le combat (M6, GDD §5.5) : logique pure, testée sans `World`.
//!
//! - **Coques** ([`Hull`]) : quatre sections (proue, poupe, bâbord, tribord), chacune un
//!   blindage puis une structure, et un cœur. Pas de bouclier (§2.4) : un projectile qui ne
//!   perce pas use le blindage ; qui perce frappe la structure, et une part atteint le cœur.
//!   Une section ouverte laisse tout passer jusqu'au cœur. Cœur à zéro : la coque est perdue.
//!   Les dégâts restent jusqu'à réparation (chantier).
//! - **À pied** ([`body_damage`]) : la protection arrête une partie de ce qu'elle ne laisse
//!   pas passer.
//! - **Raids** ([`raid_on_arrival`]) : les pillards de la simulation qui chassent dans un
//!   système y tendent une embuscade, plus souvent quand leur flotte est en forme ;
//!   la flotte de la faction qui tient le système vient à la rescousse si elle y est.

use std::collections::BTreeMap;

use sh_core::rng::fnv1a;
use sh_core::{Credits, Id, SimRng};
use sh_data::combat::{BodyArmor, HullRules, RepairRules, ShipCombat};
use sh_data::{Fleet, GameData, Place, SectionId};

use crate::SimWorld;

#[derive(Clone, Debug, PartialEq)]
pub struct SectionState {
    pub armor: f64,
    pub structure: f64,
    pub max_armor: f64,
    pub max_structure: f64,
}

impl SectionState {
    pub fn breached(&self) -> bool {
        self.structure <= 0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hull {
    pub sections: BTreeMap<SectionId, SectionState>,
    pub core: f64,
    pub max_core: f64,
    pub rules: HullRules,
}

/// Ce qu'un impact a fait.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hit {
    pub penetrated: bool,
    /// Dégâts infligés à la structure et au cœur.
    pub structure_damage: f64,
    /// La section vient de s'ouvrir.
    pub breached: bool,
    /// La coque vient d'être perdue.
    pub destroyed: bool,
}

impl Hull {
    pub fn new(def: &ShipCombat, rules: HullRules) -> Self {
        Self {
            sections: def
                .sections
                .iter()
                .map(|s| {
                    (
                        s.id,
                        SectionState {
                            armor: s.armor,
                            structure: s.structure,
                            max_armor: s.armor,
                            max_structure: s.structure,
                        },
                    )
                })
                .collect(),
            core: def.core,
            max_core: def.core,
            rules,
        }
    }

    pub fn destroyed(&self) -> bool {
        self.core <= 0.0
    }

    /// Un projectile frappe une section.
    pub fn hit(&mut self, section: SectionId, damage: f64, penetration: f64) -> Hit {
        let mut hit = Hit::default();
        if self.destroyed() {
            return hit;
        }
        let Some(s) = self.sections.get_mut(&section) else {
            return hit;
        };
        let mut to_core = 0.0;
        if s.breached() {
            // Section ouverte : rien n'arrête plus le projectile.
            hit.penetrated = true;
            to_core = damage;
        } else {
            let ratio = if s.armor <= 0.0 {
                f64::INFINITY
            } else {
                penetration / s.armor
            };
            let structure_damage = if ratio >= 1.0 {
                hit.penetrated = true;
                s.armor = (s.armor - damage * self.rules.armor_wear_penetrating).max(0.0);
                to_core = damage * self.rules.core_share;
                damage
            } else {
                s.armor = (s.armor - damage * self.rules.armor_wear_stopped * ratio).max(0.0);
                damage * self.rules.spall_stopped * ratio
            };
            let before = s.structure;
            s.structure = (s.structure - structure_damage).max(0.0);
            // Ce que la structure n'a pas pu encaisser passe au cœur.
            to_core += (structure_damage - before).max(0.0);
            hit.structure_damage += before - s.structure;
            hit.breached = before > 0.0 && s.breached();
        }
        let before = self.core;
        self.core = (self.core - to_core).max(0.0);
        hit.structure_damage += before - self.core;
        hit.destroyed = before > 0.0 && self.destroyed();
        hit
    }

    pub fn section(&self, id: SectionId) -> Option<&SectionState> {
        self.sections.get(&id)
    }

    /// Poussée disponible : moteurs (poupe) ouverts, il reste les verniers.
    pub fn thrust_factor(&self) -> f64 {
        if self
            .section(SectionId::Stern)
            .is_some_and(SectionState::breached)
        {
            0.3
        } else {
            1.0
        }
    }

    /// Cadence des armes de chasse : proue ouverte, elles tirent trois fois moins vite.
    pub fn fire_rate_factor(&self) -> f64 {
        if self
            .section(SectionId::Bow)
            .is_some_and(SectionState::breached)
        {
            1.0 / 3.0
        } else {
            1.0
        }
    }

    /// État global, de 0 à 1 (structure et cœur).
    pub fn integrity(&self) -> f64 {
        let (now, max) = self
            .sections
            .values()
            .fold((self.core, self.max_core), |(n, m), s| {
                (n + s.structure, m + s.max_structure)
            });
        if max <= 0.0 { 0.0 } else { now / max }
    }

    /// Points à réparer (blindage, structure, cœur).
    pub fn damage_points(&self) -> f64 {
        self.sections
            .values()
            .map(|s| (s.max_armor - s.armor) + (s.max_structure - s.structure))
            .sum::<f64>()
            + (self.max_core - self.core)
    }

    pub fn repair_cost(&self, rules: &RepairRules) -> Credits {
        Credits::units((self.damage_points() * rules.credits_per_point).ceil() as i64)
    }

    pub fn repair(&mut self) {
        self.repair_fraction(1.0);
    }

    /// Répare une part `f` (0 à 1) de chaque dégât : ce qu'un budget limité permet.
    pub fn repair_fraction(&mut self, f: f64) {
        let f = f.clamp(0.0, 1.0);
        for s in self.sections.values_mut() {
            s.armor += (s.max_armor - s.armor) * f;
            s.structure += (s.max_structure - s.structure) * f;
        }
        self.core += (self.max_core - self.core) * f;
    }
}

/// La section frappée, d'après le point d'impact dans le repère du vaisseau (avant : −Z,
/// droite : +X), rapporté à sa longueur et à son rayon.
pub fn section_at(local: [f64; 3], length: f64, radius: f64) -> SectionId {
    let z = local[2] / (length / 2.0).max(1e-6);
    let x = local[0] / radius.max(1e-6);
    if z.abs() >= x.abs() {
        if z < 0.0 {
            SectionId::Bow
        } else {
            SectionId::Stern
        }
    } else if x > 0.0 {
        SectionId::Starboard
    } else {
        SectionId::Port
    }
}

/// Dégâts subis à pied : la protection arrête en partie un projectile qui ne la perce pas.
pub fn body_damage(armor: Option<&BodyArmor>, damage: f64, penetration: f64) -> f64 {
    match armor {
        Some(a) if penetration < a.protection => damage * (1.0 - a.absorb),
        _ => damage,
    }
}

/// Un missile est-il leurré par une contre-mesure ?
pub fn decoyed(rng: &mut SimRng, chance: f64) -> bool {
    rng.chance(chance)
}

/// Une embuscade de pillards à l'arrivée dans un système.
#[derive(Clone, Debug, PartialEq)]
pub struct Encounter {
    pub raiders: Id<Fleet>,
    /// Le nom public des pillards (flotte, ou alias).
    pub raiders_name: String,
    /// Les coques en jeu (l'état de la flotte en retire).
    pub attackers: Vec<String>,
    /// La flotte qui vient à la rescousse, et ses coques.
    pub defenders: Option<(Id<Fleet>, Vec<String>)>,
}

/// Y a-t-il un raid à l'arrivée du joueur dans `system`, le jour `day` ? Déterministe (graine
/// du monde, jour, système). `force` : toujours, si des pillards chassent ici (autopilote).
pub fn raid_on_arrival(
    data: &GameData,
    sim: &SimWorld,
    system: &Id<Place>,
    day: i64,
    force: bool,
) -> Option<Encounter> {
    let rules = &data.combat.raid;
    let mut rng = SimRng::new(sim.seed()).fork(fnv1a(system.as_str().as_bytes()) ^ day as u64);
    for raider in &data.sim.raiders {
        if !raider.grounds.contains(system) {
            continue;
        }
        let fleet = sim.fleets.get(&raider.fleet);
        // Une flotte secrète sans nom public ne se montre pas ; en route ailleurs, elle ne
        // chasse pas.
        let secret = fleet.is_some_and(|f| f.secret) && raider.public_alias.is_none();
        let away = fleet.is_some_and(|f| f.transit.is_some());
        let condition = fleet.map_or(1.0, |f| f.condition);
        if secret || away || condition <= 0.2 {
            continue;
        }
        if !force && !rng.chance(rules.chance_on_arrival * condition) {
            continue;
        }
        let count = ((rules.attackers.len() as f64) * condition)
            .round()
            .max(1.0) as usize;
        let raiders_name = raider
            .public_alias
            .clone()
            .or_else(|| fleet.map(|f| f.name.clone()))
            .unwrap_or_else(|| raider.fleet.to_string());
        // La rescousse : une flotte publique de la faction qui tient le système, sur place.
        let controller = data
            .galaxy
            .system(system)
            .and_then(|s| s.controller.clone());
        let defenders = sim
            .fleets
            .iter()
            .find(|(_, f)| {
                !f.secret
                    && f.transit.is_none()
                    && &f.location == system
                    && f.faction.as_ref().is_some_and(|fac| {
                        // Une flotte d'une division (FDU) défend ce que tient sa maison (UGL).
                        let parent = data.faction(fac).and_then(|x| x.parent.as_ref());
                        controller
                            .as_ref()
                            .is_some_and(|c| c == fac || Some(c) == parent)
                    })
                    && f.condition > 0.2
            })
            .map(|(id, _)| (id.clone(), rules.defenders.clone()));
        return Some(Encounter {
            raiders: raider.fleet.clone(),
            raiders_name,
            attackers: rules.attackers.iter().take(count).cloned().collect(),
            defenders,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use sh_data::combat::{SectionDef, ShipCombat};

    use super::*;

    const REGLES: HullRules = HullRules {
        core_share: 0.25,
        armor_wear_penetrating: 0.1,
        armor_wear_stopped: 0.25,
        spall_stopped: 0.1,
    };

    fn coque() -> Hull {
        Hull::new(
            &ShipCombat {
                name_key: String::new(),
                model: String::new(),
                radius: 10.0,
                length: 40.0,
                max_speed: 100.0,
                accel: 10.0,
                turn_rate_deg: 30.0,
                sections: SectionId::ALL
                    .iter()
                    .map(|&id| SectionDef {
                        id,
                        armor: 40.0,
                        structure: 100.0,
                    })
                    .collect(),
                core: 200.0,
                weapons: Vec::new(),
                countermeasures: 0,
            },
            REGLES,
        )
    }

    #[test]
    fn un_obus_qui_ne_perce_pas_use_le_blindage() {
        let mut h = coque();
        let hit = h.hit(SectionId::Bow, 30.0, 20.0);
        assert!(!hit.penetrated);
        let bow = h
            .section(SectionId::Bow)
            .cloned()
            .unwrap_or_else(|| panic!());
        assert!(bow.armor < 40.0);
        assert!(
            (bow.structure - 98.5).abs() < 1e-9,
            "éclats : {}",
            bow.structure
        );
        assert_eq!(h.core, 200.0);
    }

    #[test]
    fn un_obus_qui_perce_touche_la_structure_et_le_coeur() {
        let mut h = coque();
        let hit = h.hit(SectionId::Port, 60.0, 70.0);
        assert!(hit.penetrated);
        assert_eq!(h.section(SectionId::Port).map(|s| s.structure), Some(40.0));
        assert_eq!(h.core, 185.0);
        assert!((hit.structure_damage - 75.0).abs() < 1e-9);
    }

    #[test]
    fn une_section_ouverte_laisse_tout_passer_jusqu_a_la_perte() {
        let mut h = coque();
        let first = h.hit(SectionId::Stern, 150.0, 100.0);
        assert!(first.breached);
        assert_eq!(h.thrust_factor(), 0.3);
        // 150 sur 100 de structure : 50 de trop, plus le quart de 150, vont au cœur.
        assert!((h.core - (200.0 - 50.0 - 37.5)).abs() < 1e-9);
        let mut destroyed = false;
        for _ in 0..10 {
            destroyed |= h.hit(SectionId::Stern, 20.0, 1.0).destroyed;
        }
        assert!(destroyed && h.destroyed());
        assert_eq!(h.hit(SectionId::Bow, 99.0, 99.0), Hit::default());
    }

    #[test]
    fn reparer_coute_et_remet_a_neuf() {
        let mut h = coque();
        h.hit(SectionId::Bow, 60.0, 70.0);
        let rules = RepairRules {
            issuer: Id::new("hc_chantiers").unwrap_or_else(|e| panic!("{e}")),
            credits_per_point: 2.0,
            systems: Vec::new(),
        };
        // 60 de structure, 6 de blindage, 15 de cœur : 81 points.
        assert_eq!(h.repair_cost(&rules), Credits::units(162));
        let mut half = h.clone();
        half.repair_fraction(0.5);
        assert!((half.damage_points() - 40.5).abs() < 1e-9);
        h.repair();
        assert_eq!(h, coque());
        assert_eq!(h.integrity(), 1.0);
    }

    #[test]
    fn la_section_depend_du_point_d_impact() {
        assert_eq!(section_at([0.0, 0.0, -18.0], 40.0, 10.0), SectionId::Bow);
        assert_eq!(section_at([0.0, 2.0, 15.0], 40.0, 10.0), SectionId::Stern);
        assert_eq!(
            section_at([9.0, 0.0, 3.0], 40.0, 10.0),
            SectionId::Starboard
        );
        assert_eq!(section_at([-9.0, 0.0, -3.0], 40.0, 10.0), SectionId::Port);
    }

    #[test]
    fn la_protection_arrete_ce_qui_ne_la_perce_pas() {
        let vest = BodyArmor {
            name_key: String::new(),
            protection: 30.0,
            absorb: 0.6,
        };
        assert!((body_damage(Some(&vest), 30.0, 20.0) - 12.0).abs() < 1e-9);
        assert_eq!(body_damage(Some(&vest), 30.0, 40.0), 30.0);
        assert_eq!(body_damage(None, 30.0, 20.0), 30.0);
    }
}
