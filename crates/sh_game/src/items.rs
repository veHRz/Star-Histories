//! Objets du joueur (M9) : catalogue, contenu des conteneurs, objets ramassés, équipement porté,
//! et les messages qui ouvrent un casier, une conversation ou l'écran du personnage.
//!
//! Types partagés entre les lots de M9 (docs/m9-retours.md §3.7) : les casiers et la
//! persistance (L3), les conversations avec les PNJ (L4) et l'écran du personnage (L5) s'en
//! servent ; ce module ne dessine rien. Les ressources d'une partie ([`Containers`],
//! [`PickedUp`], [`Equipment`]) naissent à l'entrée en jeu si elles manquent (une sauvegarde
//! chargée les apporte avant, comme la carrière : `save::install`) et disparaissent en quittant
//! la partie ([`forget_items`]) : on les lit par `Option<Res<…>>` ou derrière
//! `resource_exists`. Le catalogue ([`ItemCatalog`]) vient des données et reste.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use serde::{Deserialize, Serialize};
use sh_data::{EquipSlot, ItemsFile};

use crate::data::Data;
use crate::input::{OpenInventory, OpenSkills};
use crate::state::AppState;

/// Les objets que le joueur a sur lui (objet → quantité). Défini dans `interaction` (ramassage,
/// sauvegarde, `state::forget_game`) ; réexporté ici pour que tout ce qui touche aux objets se
/// trouve au même endroit.
pub use crate::interaction::Inventory;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenContainer>()
            .add_message::<TalkTo>()
            .add_message::<OpenCharacterScreen>()
            .add_observer(on_open_inventory)
            .add_observer(on_open_skills)
            .add_systems(
                PostUpdate,
                sync_catalog.run_if(resource_exists_and_changed::<Data>),
            )
            .add_systems(OnEnter(AppState::InGame), start_items)
            .add_systems(OnExit(AppState::InGame), forget_items);
    }
}

// --- Ressources --------------------------------------------------------------------------------

/// Le catalogue des objets (`gameplay/items.ron`), copié des données dès qu'elles sont chargées.
#[derive(Resource, Clone, Debug, Default, Deref)]
pub struct ItemCatalog(pub ItemsFile);

/// Le contenu des conteneurs (casiers…) d'une partie, par identifiant de conteneur (paramètre
/// `container` d'un `INTERACT_*` de genre `locker`) : objet → quantité. Initialisé depuis le
/// catalogue au début d'une partie.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Containers(pub BTreeMap<String, BTreeMap<String, u32>>);

impl Containers {
    /// Le contenu de départ de tous les conteneurs du catalogue.
    pub fn from_catalog(items: &ItemsFile) -> Self {
        Containers(items.containers.clone())
    }

    /// Ce que contient un conteneur (aucun : vide).
    pub fn contents(&self, container: &str) -> Option<&BTreeMap<String, u32>> {
        self.0.get(container)
    }

    /// Range `count` exemplaires d'un objet dans un conteneur.
    pub fn add(&mut self, container: &str, item: &str, count: u32) {
        if count == 0 {
            return;
        }
        *self
            .0
            .entry(container.to_owned())
            .or_default()
            .entry(item.to_owned())
            .or_default() += count;
    }

    /// Retire au plus `count` exemplaires d'un objet ; renvoie le nombre retiré. Un objet épuisé
    /// disparaît de la liste (le conteneur reste, vide : il a été vidé, pas oublié).
    pub fn remove(&mut self, container: &str, item: &str, count: u32) -> u32 {
        let Some(contents) = self.0.get_mut(container) else {
            return 0;
        };
        let Some(held) = contents.get_mut(item) else {
            return 0;
        };
        let taken = count.min(*held);
        *held -= taken;
        if *held == 0 {
            contents.remove(item);
        }
        taken
    }
}

/// Les objets posés dans le monde qu'on a déjà ramassés, par identifiant stable du point de
/// ramassage (`sector::gltf::pickup_id` : `<secteur>/<cellule>/<nœud sans INTERACT_>`, par
/// exemple `corvette/chambre/chambre_pickup_datapad`) : ils ne reviennent plus, la
/// cellule qui se charge les retire (`sector::gltf`). Sauvegardé (`save`).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickedUp(pub BTreeSet<String>);

impl PickedUp {
    pub fn contains(&self, id: &str) -> bool {
        self.0.contains(id)
    }

    /// Note un point de ramassage vidé ; faux s'il l'était déjà.
    pub fn insert(&mut self, id: impl Into<String>) -> bool {
        self.0.insert(id.into())
    }
}

/// L'équipement porté : emplacement → identifiant d'objet du catalogue.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment(pub BTreeMap<EquipSlot, String>);

impl Equipment {
    /// L'équipement de départ du catalogue.
    pub fn from_catalog(items: &ItemsFile) -> Self {
        Equipment(items.starting_equipment())
    }

    pub fn get(&self, slot: EquipSlot) -> Option<&str> {
        self.0.get(&slot).map(String::as_str)
    }
}

// --- Messages ----------------------------------------------------------------------------------

/// Ouvrir un conteneur (casier), par son identifiant : l'écran de rangement (L5) l'affiche à côté
/// de l'inventaire. Envoyé par `interaction` quand le joueur utilise un `INTERACT_*` de genre
/// `locker` ; le contenu se lit et se modifie dans [`Containers`] (`contents`, `add`, `remove`),
/// et l'inventaire dans [`Inventory`] : les deux sont sauvegardés.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct OpenContainer(pub String);

/// Parler à un PNJ de comptoir (`interact: "npc"`, docs/m9-retours.md §3.4) : la conversation
/// (L4) ouvre les services de son rôle (`contracts`, `bank`, `market`, `shipyard`, `bar`…) ;
/// `variant` (`a`, `b`, `c`) est celle de son modèle. Envoyé par `interaction` quand le joueur
/// vise le PNJ (ou son comptoir) et appuie sur la touche d'interaction.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct TalkTo {
    pub role: String,
    pub variant: String,
}

/// Ouvrir l'écran du personnage sur un onglet (touches de l'inventaire et des compétences,
/// fiche du terminal, autopilote `SH_SCREEN`).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenCharacterScreen(pub CharacterTab);

/// Les onglets de l'écran du personnage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CharacterTab {
    #[default]
    Inventory,
    Skills,
}

impl CharacterTab {
    pub const ALL: [CharacterTab; 2] = [CharacterTab::Inventory, CharacterTab::Skills];

    /// Suffixe de la clé de langue de l'onglet (`character.tab.<key>`), et valeur de `SH_SCREEN`.
    pub fn key(self) -> &'static str {
        match self {
            CharacterTab::Inventory => "inventory",
            CharacterTab::Skills => "skills",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.key() == key)
    }
}

// --- Systèmes ----------------------------------------------------------------------------------

fn on_open_inventory(_: On<Start<OpenInventory>>, mut open: MessageWriter<OpenCharacterScreen>) {
    open.write(OpenCharacterScreen(CharacterTab::Inventory));
}

fn on_open_skills(_: On<Start<OpenSkills>>, mut open: MessageWriter<OpenCharacterScreen>) {
    open.write(OpenCharacterScreen(CharacterTab::Skills));
}

fn sync_catalog(mut commands: Commands, data: Res<Data>) {
    commands.insert_resource(ItemCatalog(data.items.clone()));
}

/// Début de partie : ce qu'une sauvegarde n'a pas apporté part du catalogue.
fn start_items(
    mut commands: Commands,
    data: Option<Res<Data>>,
    containers: Option<Res<Containers>>,
    picked: Option<Res<PickedUp>>,
    equipment: Option<Res<Equipment>>,
) {
    let Some(data) = data else {
        return;
    };
    if containers.is_none() {
        commands.insert_resource(Containers::from_catalog(&data.items));
    }
    if picked.is_none() {
        commands.insert_resource(PickedUp::default());
    }
    if equipment.is_none() {
        commands.insert_resource(Equipment::from_catalog(&data.items));
    }
}

/// Quitter la partie efface ses objets (comme `state::forget_game` le reste). Public : une
/// écriture de sauvegarde en quittant doit passer avant (`.before(items::forget_items)`).
pub fn forget_items(mut commands: Commands) {
    commands.remove_resource::<Containers>();
    commands.remove_resource::<PickedUp>();
    commands.remove_resource::<Equipment>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranger_et_retirer() {
        let mut containers = Containers::default();
        containers.add("casier", "ration", 3);
        containers.add("casier", "ration", 0);
        assert_eq!(containers.remove("casier", "ration", 2), 2);
        assert_eq!(containers.remove("casier", "ration", 5), 1);
        assert_eq!(containers.remove("casier", "ration", 1), 0);
        assert_eq!(containers.remove("ailleurs", "ration", 1), 0);
        // Vidé, le casier reste connu : son contenu de départ ne revient pas.
        assert_eq!(containers.contents("casier"), Some(&BTreeMap::new()));
    }

    #[test]
    fn un_objet_pris_le_reste() {
        let mut picked = PickedUp::default();
        assert!(picked.insert("corvette/chambre/tablette"));
        assert!(!picked.insert("corvette/chambre/tablette"));
        assert!(picked.contains("corvette/chambre/tablette"));
        assert!(!picked.contains("corvette/soute/tablette"));
    }

    #[test]
    fn onglets_par_cle() {
        for tab in CharacterTab::ALL {
            assert_eq!(CharacterTab::from_key(tab.key()), Some(tab));
        }
        assert_eq!(CharacterTab::from_key("journal"), None);
    }
}
