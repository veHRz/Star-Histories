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
//!
//! Depuis M10, les conteneurs sont rangés **par vaisseau propriétaire** ([`Containers`],
//! [`PLAYER_SHIP`]) avec les noms que le joueur leur donne (plaque d'un casier,
//! [`NameContainer`]) ; un rangement spécialisé n'accepte qu'une sous-catégorie d'objets
//! (`sh_data::items::StorageDef`). La plaque d'un casier du joueur montre son nom, ou celui de
//! son genre (« Casier », « Frigo ») tant qu'il n'en a pas reçu ([`label_containers`]).

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use serde::{Deserialize, Serialize};
use sh_data::{EquipSlot, ItemsFile};

use crate::data::{Data, Texts};
use crate::input::{OpenInventory, OpenSkills};
use crate::sector::gltf::ContainerLabel;
use crate::state::AppState;
use crate::views::FootLabel;

/// Les objets que le joueur a sur lui (objet → quantité). Défini dans `interaction` (ramassage,
/// sauvegarde, `state::forget_game`) ; réexporté ici pour que tout ce qui touche aux objets se
/// trouve au même endroit.
pub use crate::interaction::Inventory;

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenContainer>()
            .add_message::<NameContainer>()
            .add_message::<TalkTo>()
            .add_message::<OpenCharacterScreen>()
            .add_observer(on_open_inventory)
            .add_observer(on_open_skills)
            .add_systems(
                PostUpdate,
                sync_catalog.run_if(resource_exists_and_changed::<Data>),
            )
            .add_systems(Update, label_containers)
            .add_systems(OnEnter(AppState::InGame), start_items)
            .add_systems(OnExit(AppState::InGame), forget_items);
    }
}

// --- Ressources --------------------------------------------------------------------------------

/// Le catalogue des objets (`gameplay/items.ron`), copié des données dès qu'elles sont chargées.
#[derive(Resource, Clone, Debug, Default, Deref)]
pub struct ItemCatalog(pub ItemsFile);

/// Le vaisseau propriétaire des conteneurs du vaisseau du joueur ([`Containers`]). Le jeu n'a
/// encore qu'un vaisseau du joueur : quand il en aura plusieurs, chacun aura sa clé.
pub const PLAYER_SHIP: &str = "player";

/// Nom d'un conteneur donné par le joueur : au plus tant de caractères (la plaque est petite).
pub const MAX_NAME_CHARS: usize = 24;

/// Le propriétaire des conteneurs d'une cellule : le vaisseau du joueur si la cellule en est une
/// salle (`ActiveSector::ship_cells`), sinon personne, et ses casiers restent verrouillés (un
/// autre vaisseau, une station : M10 ne donne accès qu'aux siens).
pub fn owner_of_cell(ship_cells: &[String], cell: &str) -> Option<String> {
    ship_cells
        .iter()
        .any(|c| c == cell)
        .then(|| PLAYER_SHIP.to_owned())
}

/// Le nom d'un conteneur tel que le joueur l'a tapé, nettoyé : espaces de bord retirés,
/// caractères de contrôle et accolades ou crochets (syntaxe des textes de langue) écartés,
/// [`MAX_NAME_CHARS`] caractères au plus. Vide : aucun nom (le nom par défaut revient).
pub fn clean_name(raw: &str) -> Option<String> {
    let kept: String = raw
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '{' | '}' | '[' | ']'))
        .collect();
    let name: String = kept.trim().chars().take(MAX_NAME_CHARS).collect();
    let name = name.trim_end().to_owned();
    (!name.is_empty()).then_some(name)
}

/// Les rangements d'un vaisseau : contenu et noms, par identifiant de conteneur (paramètre
/// `container` d'un `INTERACT_*` de genre `locker`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShipStorage {
    /// Conteneur → (objet → quantité). Un conteneur présent mais vide a été vidé : son contenu
    /// de départ ne revient pas.
    #[serde(default)]
    pub contents: BTreeMap<String, BTreeMap<String, u32>>,
    /// Conteneur → nom donné par le joueur (plaque d'un casier). Absent : nom par défaut.
    #[serde(default)]
    pub names: BTreeMap<String, String>,
}

/// Les conteneurs (casiers, frigo…) d'une partie, **par vaisseau propriétaire** (M10 : deux
/// vaisseaux du même modèle ont les mêmes identifiants de casiers, pas le même contenu).
/// Aujourd'hui seul le vaisseau du joueur ([`PLAYER_SHIP`]) en a. Initialisé depuis le
/// catalogue au début d'une partie ; sauvegardé (`save`).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Containers {
    #[serde(default)]
    pub ships: BTreeMap<String, ShipStorage>,
}

impl Containers {
    /// Le contenu de départ de tous les conteneurs du catalogue, dans le vaisseau du joueur.
    pub fn from_catalog(items: &ItemsFile) -> Self {
        let mut containers = Containers::default();
        containers.fill_missing(items);
        containers
    }

    /// Les conteneurs du vaisseau du joueur tels qu'une partie d'avant M10 les rangeait (clé
    /// globale) : ils deviennent ceux du vaisseau du joueur.
    pub fn from_player_contents(contents: BTreeMap<String, BTreeMap<String, u32>>) -> Self {
        let mut containers = Containers::default();
        containers.ships.insert(
            PLAYER_SHIP.to_owned(),
            ShipStorage {
                contents,
                names: BTreeMap::new(),
            },
        );
        containers
    }

    /// Donne leur contenu de départ aux conteneurs du catalogue que le vaisseau du joueur ne
    /// connaît pas encore (un rangement ajouté aux données après la sauvegarde : le frigo de
    /// M10) ; un conteneur connu, même vidé, n'y gagne rien.
    pub fn fill_missing(&mut self, items: &ItemsFile) {
        let ship = self.ships.entry(PLAYER_SHIP.to_owned()).or_default();
        for (container, start) in &items.containers {
            ship.contents
                .entry(container.clone())
                .or_insert_with(|| start.clone());
        }
    }

    pub fn ship(&self, ship: &str) -> Option<&ShipStorage> {
        self.ships.get(ship)
    }

    /// Ce que contient un conteneur de ce vaisseau (aucun : vide).
    pub fn contents_in(&self, ship: &str, container: &str) -> Option<&BTreeMap<String, u32>> {
        self.ship(ship)?.contents.get(container)
    }

    /// Ce que contient un conteneur du vaisseau du joueur.
    pub fn contents(&self, container: &str) -> Option<&BTreeMap<String, u32>> {
        self.contents_in(PLAYER_SHIP, container)
    }

    /// Le nom donné à un conteneur de ce vaisseau, s'il en a un.
    pub fn name(&self, ship: &str, container: &str) -> Option<&str> {
        self.ship(ship)?.names.get(container).map(String::as_str)
    }

    /// Nomme un conteneur (le nom est nettoyé, [`clean_name`]) ; vide : il reprend son nom par
    /// défaut.
    pub fn set_name(&mut self, ship: &str, container: &str, raw: &str) {
        let names = &mut self.ships.entry(ship.to_owned()).or_default().names;
        match clean_name(raw) {
            Some(name) => {
                names.insert(container.to_owned(), name);
            }
            None => {
                names.remove(container);
            }
        }
    }

    /// Range `count` exemplaires d'un objet dans un conteneur de ce vaisseau (sans contrôle : ce
    /// que le conteneur accepte se vérifie avant, `ItemsFile::accepts`).
    pub fn add(&mut self, ship: &str, container: &str, item: &str, count: u32) {
        if count == 0 {
            return;
        }
        let held = self
            .ships
            .entry(ship.to_owned())
            .or_default()
            .contents
            .entry(container.to_owned())
            .or_default()
            .entry(item.to_owned())
            .or_default();
        *held = held.saturating_add(count);
    }

    /// Retire au plus `count` exemplaires d'un objet ; renvoie le nombre retiré. Un objet épuisé
    /// disparaît de la liste (le conteneur reste, vide : il a été vidé, pas oublié).
    pub fn remove(&mut self, ship: &str, container: &str, item: &str, count: u32) -> u32 {
        let Some(contents) = self
            .ships
            .get_mut(ship)
            .and_then(|s| s.contents.get_mut(container))
        else {
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

    /// Combien d'objets acceptés par ce conteneur il contient (tous, pour un casier ordinaire) :
    /// le compteur de son écran. Le stock de rations du vaisseau, c'est ce que compte le frigo.
    pub fn stock(&self, ship: &str, container: &str, items: &ItemsFile) -> u32 {
        self.contents_in(ship, container)
            .into_iter()
            .flatten()
            .filter(|(item, _)| items.accepts(container, item))
            .fold(0u32, |sum, (_, count)| sum.saturating_add(*count))
    }
}

/// Le nom d'un conteneur à montrer (invite, écran de rangement, plaque) : celui que le joueur
/// lui a donné, s'il en a un. Pour la plaque d'un casier (`sector::gltf::ContainerLabel`).
pub fn container_name<'a>(
    containers: Option<&'a Containers>,
    ship: &str,
    container: &str,
) -> Option<&'a str> {
    containers?.name(ship, container)
}

/// Distance de lecture du nom d'un casier sur sa plaque (m) : plus courte que celle des noms de
/// salle, une chambre a plusieurs casiers côte à côte et leurs plaques s'empileraient de loin.
const PLATE_DISTANCE: f32 = 3.0;

/// Le texte de la plaque d'un casier : le nom que le joueur lui a donné, sinon celui de son
/// genre (`StorageDef::kind_key` : « Casier », « Frigo ») ; aucun pour un casier qui n'est pas
/// au joueur (sa plaque reste muette, comme il reste verrouillé).
pub fn plate_text(
    lang: &sh_data::Lang,
    items: Option<&ItemsFile>,
    containers: Option<&Containers>,
    owner: Option<&str>,
    container: &str,
) -> Option<String> {
    let owner = owner.filter(|o| crate::interaction::usable_locker(container, Some(o)))?;
    Some(match container_name(containers, owner, container) {
        Some(name) => name.to_owned(),
        None => lang.get(items.map_or(sh_data::items::LOCKER_KIND_KEY, |i| i.kind_key(container))),
    })
}

/// Écrit le nom des casiers sur leur plaque (`sector::gltf::ContainerLabel`, ancre
/// `SOCKET_<n>_label`) : une [`FootLabel`] lisible de face (l'avant du nœud, −Z, sort du casier),
/// depuis la cellule du casier. Recalculé pour une plaque nouvelle, et pour toutes quand les noms
/// (`Containers`), le catalogue ou la langue changent ; le reste du temps, rien.
fn label_containers(
    mut commands: Commands,
    texts: Option<Res<Texts>>,
    catalog: Option<Res<ItemCatalog>>,
    containers: Option<Res<Containers>>,
    mut labels: Query<(Entity, Ref<ContainerLabel>, Option<&mut FootLabel>)>,
) {
    let Some(texts) = texts else {
        return;
    };
    let refresh = texts.is_changed()
        || catalog.as_ref().is_some_and(|c| c.is_changed())
        || containers.as_ref().is_some_and(|c| c.is_changed());
    for (entity, label, foot) in &mut labels {
        if !refresh && !label.is_changed() && foot.is_some() {
            continue;
        }
        let text = plate_text(
            &texts.0,
            catalog.as_deref().map(|c| &c.0),
            containers.as_deref(),
            label.owner.as_deref(),
            &label.container,
        );
        match (text, foot) {
            (Some(text), Some(mut foot)) => {
                if foot.text != text {
                    foot.text = text;
                }
            }
            (Some(text), None) => {
                let mut plate = FootLabel::new(text)
                    .facing(Vec3::NEG_Z)
                    .within(PLATE_DISTANCE);
                if let Some(cell) = &label.cell {
                    plate = plate.in_cell(cell.clone());
                }
                commands.entity(entity).try_insert(plate);
            }
            (None, Some(_)) => {
                commands.entity(entity).try_remove::<FootLabel>();
            }
            (None, None) => {}
        }
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
///
/// Le conteneur est toujours un de ceux du vaisseau du joueur ([`PLAYER_SHIP`]) : on n'ouvre
/// pas ceux des autres (`interaction` les montre verrouillés).
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct OpenContainer(pub String);

/// Nommer un conteneur du vaisseau du joueur (sa plaque, `interact: "locker_label"`) : ouvre la
/// saisie du nom (`ui::character`). Envoyé par `interaction` ; l'autopilote peut l'envoyer aussi.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct NameContainer(pub String);

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
    containers: Option<ResMut<Containers>>,
    picked: Option<Res<PickedUp>>,
    equipment: Option<Res<Equipment>>,
) {
    let Some(data) = data else {
        return;
    };
    match containers {
        // Une sauvegarde peut précéder un rangement ajouté depuis : il reçoit son contenu de
        // départ, les autres restent tels qu'ils ont été laissés.
        Some(mut containers) => containers.fill_missing(&data.items),
        None => commands.insert_resource(Containers::from_catalog(&data.items)),
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
        containers.add(PLAYER_SHIP, "casier", "ration", 3);
        containers.add(PLAYER_SHIP, "casier", "ration", 0);
        assert_eq!(containers.remove(PLAYER_SHIP, "casier", "ration", 2), 2);
        assert_eq!(containers.remove(PLAYER_SHIP, "casier", "ration", 5), 1);
        assert_eq!(containers.remove(PLAYER_SHIP, "casier", "ration", 1), 0);
        assert_eq!(containers.remove(PLAYER_SHIP, "ailleurs", "ration", 1), 0);
        // Vidé, le casier reste connu : son contenu de départ ne revient pas.
        assert_eq!(containers.contents("casier"), Some(&BTreeMap::new()));
    }

    /// Deux vaisseaux du même modèle ont les mêmes identifiants de casiers, pas le même
    /// contenu ni les mêmes noms.
    #[test]
    fn chaque_vaisseau_ses_casiers() {
        let mut containers = Containers::default();
        containers.add(PLAYER_SHIP, "corvette_chambre_casier_1", "ration", 2);
        containers.add("autre", "corvette_chambre_casier_1", "medkit", 1);
        containers.set_name(PLAYER_SHIP, "corvette_chambre_casier_1", "  Mes affaires ");
        let mine = containers.contents("corvette_chambre_casier_1");
        assert_eq!(mine.and_then(|c| c.get("ration")), Some(&2));
        assert_eq!(mine.and_then(|c| c.get("medkit")), None);
        assert_eq!(
            containers
                .contents_in("autre", "corvette_chambre_casier_1")
                .and_then(|c| c.get("medkit")),
            Some(&1)
        );
        assert_eq!(
            containers.name(PLAYER_SHIP, "corvette_chambre_casier_1"),
            Some("Mes affaires")
        );
        assert_eq!(containers.name("autre", "corvette_chambre_casier_1"), None);
        assert_eq!(
            container_name(Some(&containers), PLAYER_SHIP, "corvette_chambre_casier_1"),
            Some("Mes affaires")
        );
        // Un nom vide rend le nom par défaut.
        containers.set_name(PLAYER_SHIP, "corvette_chambre_casier_1", "   ");
        assert_eq!(
            containers.name(PLAYER_SHIP, "corvette_chambre_casier_1"),
            None
        );
        // Seules les salles du vaisseau du joueur lui donnent ses casiers.
        let cells = vec!["chambre".to_owned(), "soute".to_owned()];
        assert_eq!(
            owner_of_cell(&cells, "chambre").as_deref(),
            Some(PLAYER_SHIP)
        );
        assert_eq!(owner_of_cell(&cells, "hall"), None);
        assert_eq!(owner_of_cell(&[], "chambre"), None);
    }

    #[test]
    fn noms_nettoyes() {
        assert_eq!(clean_name("  Frigo  ").as_deref(), Some("Frigo"));
        assert_eq!(clean_name("").as_deref(), None);
        assert_eq!(clean_name(" \t\n ").as_deref(), None);
        assert_eq!(clean_name("Ta{b}le[KeyE]").as_deref(), Some("TableKeyE"));
        assert_eq!(clean_name("a\u{7}b").as_deref(), Some("ab"));
        let long = "é".repeat(40);
        assert_eq!(
            clean_name(&long).map(|n| n.chars().count()),
            Some(MAX_NAME_CHARS)
        );
        // Coupé à 24 caractères, sans espace qui traîne au bout.
        let cut = clean_name("Rangement du second ma rgaux").unwrap_or_default();
        assert_eq!(cut, "Rangement du second ma r");
        assert_eq!(
            clean_name("Rangement du second mat  xx").as_deref(),
            Some("Rangement du second mat")
        );
    }

    /// Un rangement ajouté aux données après une sauvegarde reçoit son contenu de départ ; un
    /// casier vidé, non.
    #[test]
    fn contenu_de_depart_des_rangements_nouveaux() {
        let items: ItemsFile = sh_data::ron_options()
            .from_str(
                r#"(items: [(id: "ration", name_key: "k", description_key: "d",
                    category: Consumable, subcategory: Food, max_stack: 10),
                   (id: "medkit", name_key: "k", description_key: "d", category: Consumable)],
                   containers: { "casier": { "medkit": 1 }, "frigo": { "ration": 12 } },
                   storage: { "frigo": (accepts: Food) })"#,
            )
            .unwrap_or_else(|e| panic!("{e}"));
        let mut saved = Containers::from_player_contents(BTreeMap::from([(
            "casier".to_owned(),
            BTreeMap::new(),
        )]));
        saved.fill_missing(&items);
        assert_eq!(saved.contents("casier"), Some(&BTreeMap::new()));
        assert_eq!(
            saved.contents("frigo").and_then(|c| c.get("ration")),
            Some(&12)
        );
        // Le compteur du frigo : ses rations ; un casier compte tout.
        saved.add(PLAYER_SHIP, "casier", "medkit", 2);
        assert_eq!(saved.stock(PLAYER_SHIP, "frigo", &items), 12);
        assert_eq!(saved.stock(PLAYER_SHIP, "casier", &items), 2);
        assert_eq!(saved.stock("autre", "frigo", &items), 0);
        assert_eq!(
            Containers::from_catalog(&items).stock(PLAYER_SHIP, "frigo", &items),
            12
        );
    }

    /// La plaque : le nom donné, sinon le genre du rangement ; muette hors du vaisseau du joueur.
    #[test]
    fn texte_des_plaques() {
        let mut lang = sh_data::Lang::default();
        lang.strings.insert(
            sh_data::items::LOCKER_KIND_KEY.to_owned(),
            "Casier".to_owned(),
        );
        lang.strings
            .insert("container.fridge".to_owned(), "Frigo".to_owned());
        let items: ItemsFile = sh_data::ron_options()
            .from_str(r#"(items: [], storage: { "frigo": (kind_key: "container.fridge") })"#)
            .unwrap_or_else(|e| panic!("{e}"));
        let mut containers = Containers::default();
        let text = |c: &Containers, owner: Option<&str>, id: &str| {
            plate_text(&lang, Some(&items), Some(c), owner, id)
        };
        assert_eq!(
            text(&containers, Some(PLAYER_SHIP), "casier").as_deref(),
            Some("Casier")
        );
        assert_eq!(
            text(&containers, Some(PLAYER_SHIP), "frigo").as_deref(),
            Some("Frigo")
        );
        containers.set_name(PLAYER_SHIP, "casier", "Mes affaires");
        assert_eq!(
            text(&containers, Some(PLAYER_SHIP), "casier").as_deref(),
            Some("Mes affaires")
        );
        assert_eq!(text(&containers, None, "casier"), None);
        assert_eq!(text(&containers, Some("autre"), "casier"), None);
        assert_eq!(text(&containers, Some(PLAYER_SHIP), ""), None);
        // Sans catalogue : le genre du casier.
        assert_eq!(
            plate_text(&lang, None, None, Some(PLAYER_SHIP), "frigo").as_deref(),
            Some("Casier")
        );
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
