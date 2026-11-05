//! Catalogue des objets (`assets/data/gameplay/items.ron`) : ce que le joueur ramasse, range dans
//! un casier ou porte sur lui (M9).
//!
//! Contenu générique de gameplay, pas un fait d'univers (docs/m9-retours.md §2). Chaque objet a
//! son nom et sa description dans la langue (clés `item.<id>` et `item.<id>.description`, fichier
//! `lang/<code>/world.ron`) ; son modèle et son icône se déduisent de son identifiant, sans champ
//! ([`ItemDef::model_path`], [`ItemDef::icon_path`]). Le fichier donne aussi le contenu de départ
//! des conteneurs (paramètre `container` d'un `INTERACT_*` de genre `locker`) et l'équipement
//! porté au début d'une partie.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Le fichier du catalogue.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemsFile {
    pub items: Vec<ItemDef>,
    /// Contenu de départ des conteneurs : identifiant de conteneur → (objet → quantité).
    #[serde(default)]
    pub containers: BTreeMap<String, BTreeMap<String, u32>>,
    /// Objets portés au début d'une partie : chacun va dans son emplacement ([`ItemDef::slot`]).
    #[serde(default)]
    pub equipped: Vec<String>,
}

/// Un objet du catalogue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemDef {
    /// Identifiant stable en snake_case (`datapad`) : aussi celui du paramètre `item` d'un
    /// ramassage, du modèle et de l'icône.
    pub id: String,
    /// Nom, en minuscules : il se glisse dans une phrase (« Ramassé : tablette »).
    pub name_key: String,
    pub description_key: String,
    pub category: ItemCategory,
    /// Emplacement d'équipement, si l'objet se porte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<EquipSlot>,
    /// Quantité au plus dans une même case d'inventaire.
    #[serde(default = "one")]
    pub max_stack: u32,
    /// L'arme portative de `combat.ron` (`firearms`) que l'objet donne une fois équipé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firearm: Option<String>,
}

fn one() -> u32 {
    1
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ItemCategory {
    /// Ce qui se porte : armes, tenues.
    Equipment,
    Tool,
    /// Ce qui s'utilise et disparaît : soins, rations.
    Consumable,
    /// Ce qui se lit : tablettes, puces, manifestes.
    Document,
    /// Pièces et matières premières.
    Material,
}

impl ItemCategory {
    pub const ALL: [ItemCategory; 5] = [
        ItemCategory::Equipment,
        ItemCategory::Tool,
        ItemCategory::Consumable,
        ItemCategory::Document,
        ItemCategory::Material,
    ];

    /// Suffixe de la clé de langue du libellé : `item.category.<key>`.
    pub fn key(self) -> &'static str {
        match self {
            ItemCategory::Equipment => "equipment",
            ItemCategory::Tool => "tool",
            ItemCategory::Consumable => "consumable",
            ItemCategory::Document => "document",
            ItemCategory::Material => "material",
        }
    }
}

/// Les emplacements d'équipement du joueur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    Weapon,
    Outfit,
    Tool,
}

impl EquipSlot {
    pub const ALL: [EquipSlot; 3] = [EquipSlot::Weapon, EquipSlot::Outfit, EquipSlot::Tool];

    /// Suffixe de la clé de langue du libellé : `item.slot.<key>`.
    pub fn key(self) -> &'static str {
        match self {
            EquipSlot::Weapon => "weapon",
            EquipSlot::Outfit => "outfit",
            EquipSlot::Tool => "tool",
        }
    }
}

impl ItemDef {
    /// Le modèle posé dans le monde (origine au centre de la base), relatif à `assets/`.
    pub fn model_path(&self) -> String {
        format!("models/items/{}.gltf", self.id)
    }

    /// L'icône d'inventaire (256 × 256, fond transparent), relative à `assets/`.
    pub fn icon_path(&self) -> String {
        format!("ui/icons/items/{}.png", self.id)
    }
}

impl ItemsFile {
    pub fn get(&self, id: &str) -> Option<&ItemDef> {
        self.items.iter().find(|item| item.id == id)
    }

    /// L'équipement de départ, par emplacement (un objet sans emplacement est ignoré : la
    /// validation le signale).
    pub fn starting_equipment(&self) -> BTreeMap<EquipSlot, String> {
        self.equipped
            .iter()
            .filter_map(|id| Some((self.get(id)?.slot?, id.clone())))
            .collect()
    }

    /// Les problèmes de cohérence internes au fichier (la langue et les armes sont vérifiées
    /// par `validate`).
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for item in &self.items {
            if !sh_core::id::is_valid_id(&item.id) {
                out.push(format!("identifiant d'objet invalide : {}", item.id));
            }
            if !seen.insert(item.id.as_str()) {
                out.push(format!("objet en double : {}", item.id));
            }
            if item.max_stack == 0 {
                out.push(format!("{} : max_stack doit valoir au moins 1", item.id));
            }
            if item.firearm.is_some() && item.slot != Some(EquipSlot::Weapon) {
                out.push(format!(
                    "{} : une arme portative s'équipe dans l'emplacement Weapon",
                    item.id
                ));
            }
        }
        for (container, contents) in &self.containers {
            if !sh_core::id::is_valid_id(container) {
                out.push(format!("identifiant de conteneur invalide : {container}"));
            }
            for (item, count) in contents {
                if self.get(item).is_none() {
                    out.push(format!("conteneur {container} : objet inconnu {item}"));
                }
                if *count == 0 {
                    out.push(format!("conteneur {container} : quantité nulle de {item}"));
                }
            }
        }
        let mut slots = BTreeSet::new();
        for id in &self.equipped {
            match self.get(id).map(|item| item.slot) {
                None => out.push(format!("équipement de départ inconnu : {id}")),
                Some(None) => out.push(format!("équipement de départ sans emplacement : {id}")),
                Some(Some(slot)) => {
                    if !slots.insert(slot) {
                        out.push(format!(
                            "équipement de départ : deux objets dans l'emplacement {slot:?}"
                        ));
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, category: ItemCategory, slot: Option<EquipSlot>) -> ItemDef {
        ItemDef {
            id: id.into(),
            name_key: format!("item.{id}"),
            description_key: format!("item.{id}.description"),
            category,
            slot,
            max_stack: 1,
            firearm: None,
        }
    }

    #[test]
    fn lecture_et_chemins_derives() {
        let text = r#"(
            items: [
                (id: "mag_pistol", name_key: "item.mag_pistol",
                 description_key: "item.mag_pistol.description", category: Equipment,
                 slot: Weapon, firearm: "pistol"),
                (id: "ration", name_key: "item.ration", description_key: "item.ration.description",
                 category: Consumable, max_stack: 10),
            ],
            containers: { "casier_1": { "ration": 3 } },
            equipped: ["mag_pistol"],
        )"#;
        let file: ItemsFile = crate::ron_options()
            .from_str(text)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(file.problems().is_empty(), "{:?}", file.problems());
        let pistol = file.get("mag_pistol").unwrap_or_else(|| panic!("absent"));
        assert_eq!(pistol.slot, Some(EquipSlot::Weapon));
        assert_eq!(pistol.max_stack, 1);
        assert_eq!(pistol.model_path(), "models/items/mag_pistol.gltf");
        assert_eq!(pistol.icon_path(), "ui/icons/items/mag_pistol.png");
        assert_eq!(
            file.starting_equipment(),
            BTreeMap::from([(EquipSlot::Weapon, "mag_pistol".to_owned())])
        );
    }

    #[test]
    fn problemes_detectes() {
        let file = ItemsFile {
            items: vec![
                item("toolkit", ItemCategory::Tool, Some(EquipSlot::Tool)),
                item("toolkit", ItemCategory::Tool, None),
                item("ration", ItemCategory::Consumable, None),
            ],
            containers: BTreeMap::from([(
                "casier".to_owned(),
                BTreeMap::from([("fantome".to_owned(), 1), ("ration".to_owned(), 0)]),
            )]),
            equipped: vec!["ration".into()],
        };
        let problems = file.problems().join("\n");
        assert!(problems.contains("objet en double : toolkit"), "{problems}");
        assert!(problems.contains("objet inconnu fantome"), "{problems}");
        assert!(problems.contains("quantité nulle de ration"), "{problems}");
        assert!(problems.contains("sans emplacement : ration"), "{problems}");
    }

    /// L'équipement se sauvegarde en dictionnaire à clés d'énumération : l'aller-retour RON tient.
    #[test]
    fn equipement_aller_retour_ron() {
        let equipment = BTreeMap::from([
            (EquipSlot::Weapon, "mag_pistol".to_owned()),
            (EquipSlot::Outfit, "flight_suit".to_owned()),
        ]);
        let text = ron::to_string(&equipment).unwrap_or_else(|e| panic!("{e}"));
        let back: BTreeMap<EquipSlot, String> = crate::ron_options()
            .from_str(&text)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, equipment);
    }
}
