//! Règles du combat (M6) : `gameplay/combat.ron` (P26).
//!
//! Le canon fixe le cadre (§2.4) : canons à projectiles magnétiques, missiles et torpilles,
//! **aucun bouclier** (seul le blindage protège), classes de coques et tailles. Les chiffres
//! (blindages, dégâts, cadences, composition d'un raid) sont proposés.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sh_core::Id;

use crate::common::Status;
use crate::factions::Faction;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombatRules {
    #[serde(default)]
    pub status: Status,
    /// Armes de bord.
    pub ship_weapons: BTreeMap<String, ShipWeapon>,
    /// Coques en combat, par clé (`player_ship`, `pirate_corvette`…).
    pub ships: BTreeMap<String, ShipCombat>,
    /// Armes portatives.
    pub firearms: BTreeMap<String, Firearm>,
    /// Protections corporelles.
    pub armors: BTreeMap<String, BodyArmor>,
    /// Combattants à pied, par rôle (`player`, `pirate`).
    pub fighters: BTreeMap<String, FighterDef>,
    pub hull: HullRules,
    pub raid: RaidRules,
    pub repair: RepairRules,
}

/// Comment un impact se répartit (blindage, structure, cœur).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HullRules {
    /// Part des dégâts d'un projectile qui perce reportée sur le cœur.
    pub core_share: f64,
    /// Usure du blindage par un projectile qui perce, en part de ses dégâts.
    pub armor_wear_penetrating: f64,
    /// Usure du blindage et éclats d'un projectile arrêté, en part de ses dégâts (multipliés
    /// par le rapport pénétration / blindage).
    pub armor_wear_stopped: f64,
    pub spall_stopped: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponKind {
    /// Canon à projectiles magnétiques : obus balistiques.
    Cannon,
    /// Missile guidé, leurré par les contre-mesures.
    Missile,
    /// Torpille : lente, lourde, guidée.
    Torpedo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipWeapon {
    pub kind: WeaponKind,
    /// Vitesse du projectile relative au tireur (m/s).
    pub speed: f64,
    pub damage: f64,
    /// Capacité à traverser le blindage (comparée à son épaisseur).
    pub penetration: f64,
    /// Secondes entre deux tirs.
    pub cooldown: f64,
    /// Dispersion (degrés).
    #[serde(default)]
    pub spread_deg: f64,
    /// Durée de vol avant autodestruction (s).
    pub lifetime: f64,
    /// Guidage : virage maximal (degrés par seconde) ; 0 pour un obus.
    #[serde(default)]
    pub turn_rate_deg: f64,
    /// Probabilité d'être leurré par une contre-mesure lancée à temps.
    #[serde(default)]
    pub decoy_chance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SectionId {
    /// Proue : capteurs et armes de chasse.
    Bow,
    /// Poupe : moteurs.
    Stern,
    /// Bâbord.
    Port,
    /// Tribord.
    Starboard,
}

impl SectionId {
    pub const ALL: [SectionId; 4] = [
        SectionId::Bow,
        SectionId::Stern,
        SectionId::Port,
        SectionId::Starboard,
    ];

    pub fn key(self) -> &'static str {
        match self {
            SectionId::Bow => "bow",
            SectionId::Stern => "stern",
            SectionId::Port => "port",
            SectionId::Starboard => "starboard",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionDef {
    pub id: SectionId,
    /// Épaisseur de blindage (points, comparés à la pénétration).
    pub armor: f64,
    /// Structure derrière le blindage.
    pub structure: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipCombat {
    /// Clé de langue du type de coque.
    pub name_key: String,
    pub model: String,
    /// Rayon de la coque pour les impacts (m).
    pub radius: f64,
    /// Longueur (m) : sert à placer les sections.
    pub length: f64,
    /// Vitesse de combat maximale (m/s), accélération (m/s²), virage (degrés/s).
    pub max_speed: f64,
    pub accel: f64,
    pub turn_rate_deg: f64,
    pub sections: Vec<SectionDef>,
    /// Le cœur (réacteur, quille) : à zéro, la coque est perdue.
    pub core: f64,
    /// Armes montées (clés de `ship_weapons`), avec leurs munitions (0 : illimitées).
    pub weapons: Vec<Mount>,
    pub countermeasures: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mount {
    pub weapon: String,
    #[serde(default)]
    pub ammo: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Firearm {
    pub name_key: String,
    pub damage: f64,
    pub penetration: f64,
    /// Coups par seconde.
    pub rate: f64,
    pub magazine: u32,
    pub reserve: u32,
    pub reload_seconds: f64,
    /// Vitesse du projectile (m/s).
    pub speed: f64,
    pub spread_deg: f64,
    /// Relèvement de la visée par coup (degrés).
    pub recoil_deg: f64,
    pub range: f64,
    /// Modèle vu à la première personne quand l'arme est en main (chemin sous `assets/`).
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BodyArmor {
    pub name_key: String,
    /// Une pénétration inférieure est arrêtée en partie.
    pub protection: f64,
    /// Part des dégâts absorbée quand le projectile est arrêté.
    pub absorb: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FighterDef {
    pub health: f64,
    pub weapon: String,
    #[serde(default)]
    pub armor: Option<String>,
    /// Précision des PNJ : dispersion ajoutée (degrés).
    #[serde(default)]
    pub aim_error_deg: f64,
    /// Temps de réaction avant le premier tir (s).
    #[serde(default)]
    pub reaction_seconds: f64,
    /// Portée de vue et d'ouïe (m).
    #[serde(default)]
    pub sight: f64,
    #[serde(default)]
    pub hearing: f64,
}

/// Un raid de pillards, à l'arrivée dans un système de leurs terrains de chasse.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RaidRules {
    /// Probabilité d'un raid à l'arrivée, multipliée par l'état de la flotte de pillards.
    pub chance_on_arrival: f64,
    /// Les coques des pillards (au complet ; l'état de leur flotte en retire).
    pub attackers: Vec<String>,
    /// Les défenseurs du système, quand une flotte de la faction qui le tient y est.
    pub defenders: Vec<String>,
    /// Distance d'apparition des pillards (m) et délai d'arrivée des défenseurs (s).
    pub spawn_distance: f64,
    pub defenders_delay: f64,
    /// Au-delà de cette distance de tous les pillards, le joueur a fui (m).
    pub escape_distance: f64,
    /// Pas de croisière à moins de cette distance d'une coque hostile (m) : fuir, c'est
    /// d'abord distancer les pillards.
    pub cruise_lock_distance: f64,
    /// Perte d'état de la flotte de pillards par coque détruite.
    pub condition_per_kill: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepairRules {
    /// Le chantier qui répare (division Chantiers d'Hexa-Corp : CRV, §4.4).
    pub issuer: Id<Faction>,
    pub credits_per_point: f64,
    /// Les systèmes dont les chantiers réparent le vaisseau du joueur.
    pub systems: Vec<Id<crate::common::Place>>,
}

impl CombatRules {
    pub fn ship(&self, key: &str) -> Option<&ShipCombat> {
        self.ships.get(key)
    }
}
