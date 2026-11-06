//! Écran du personnage (M9, lot L5) : deux onglets, ouverts par `items::OpenCharacterScreen`
//! (touches physiques `KeyI` et `KeyO` par défaut, fiche du terminal, autopilote `SH_SCREEN`),
//! et le rangement d'un casier, ouvert par `items::OpenContainer`.
//!
//! - **Inventaire** : objets portés en grille d'icônes (`ItemDef::icon_path`, quantité en coin,
//!   une case par pile : `max_stack`), détail de l'objet survolé ou choisi (M11 : il suit la
//!   dernière case pleine survolée tant que la souris reste sur les cases, `Focus`), équipement
//!   par emplacement (`items::Equipment` ; équiper et retirer déplacent l'objet entre l'inventaire
//!   et l'emplacement), compte (crédits, Wuro), récapitulatif des réputations (comme la fiche
//!   du terminal : jamais une organisation secrète) et des emprunts.
//! - **Compétences** : niveau de chaque compétence et barre d'expérience vers le suivant
//!   (niveau n à `xp_per_level × n²` points, P23), traits gagnés.
//! - **Casier** : deux grilles côte à côte (casier, inventaire) ; un clic déplace une case
//!   entière (une pile, au plus `max_stack`), un clic droit un seul exemplaire ; au clavier,
//!   flèches et Entrée. Le contenu vit dans `items::Containers`, rangé par vaisseau (ici
//!   toujours celui du joueur, `items::PLAYER_SHIP`) et sauvegardé. Depuis M10, un rangement
//!   spécialisé (le frigo) refuse ce qui n'est pas de sa sous-catégorie, l'écran dit pourquoi, et
//!   montre son compteur (« Rations : 12 ») ; un casier nommé porte son nom en titre.
//! - **Nom d'un casier** ([`naming`], M10) : la saisie ouverte par sa plaque.
//!
//! Les touches de chaque vue sont dans l'encart des touches (`ui::keys_panel::KeyHelp`, posé sur
//! la racine de l'écran), plus en consigne écrite sous l'écran.
//!
//! Comme le terminal : le jeu passe en mode Menu (curseur libre), le temps du monde continue,
//! et l'écran se ferme une fois les touches relâchées (Échap, la touche de l'onglet ouvert,
//! l'interaction pour un casier), si bien que le contexte rendu ne les voit pas encore
//! enfoncées (pas de pause qui s'ouvre derrière). L'écran n'est redessiné que quand ce qu'il
//! montre change ; le survol ne fait que recolorer les cases et refaire le panneau de détail.
//!
//! Autopilote (captures) : `SH_CONTAINER=<id>` ouvre ce casier 3 s après l'entrée en partie,
//! `SH_GIVE=<objet>:<n>,…` ajoute des objets à l'inventaire au début de la partie,
//! `SH_XP=<compétence>:<points>,…` pose l'expérience de compétences (barres de l'onglet),
//! `SH_LOCKER_NAME=<conteneur>:<nom>,…` nomme des casiers au début de la partie, et
//! `SH_NAMING=<conteneur>` ouvre la saisie du nom de ce casier 3 s après l'entrée en partie.
//!
//! Textes dans `lang/<code>/character.ron` ; noms, descriptions, catégories et emplacements des
//! objets dans `lang/<code>/world.ron`.

use std::collections::BTreeMap;

use bevy::ecs::system::SystemParam;
use bevy::image::ImageSampler;
use bevy::picking::events::{PointerClick, PointerOut, PointerOver};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_enhanced_input::prelude::{Action, Binding, Bindings, Start};
use sh_data::{EquipSlot, ItemsFile};

use crate::career::Career;
use crate::clock::GameClock;
use crate::combat::Death;
use crate::data::{Data, Texts};
use crate::input::{Back, Confirm, Interact, Navigate, OpenInventory, OpenSkills};
use crate::items::{
    CharacterTab, Containers, Equipment, Inventory, ItemCatalog, OpenCharacterScreen,
    OpenContainer, PLAYER_SHIP,
};
use crate::state::{AppState, ControlMode, ViewLevel};
use crate::transition::Sequence;
use crate::ui::pause::Paused;
use crate::ui::terminal::Terminal;
use crate::ui::theme::{self, UiFonts};

mod draw;
pub mod logic;
pub mod naming;

use self::draw::{Body, Content};
use self::naming::NamingScreen;

pub struct CharacterScreenPlugin;

impl Plugin for CharacterScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_back)
            .add_observer(on_confirm)
            .add_observer(on_navigate)
            .add_observer(on_activate)
            .add_observer(on_cell_click)
            .add_observer(on_cell_over)
            .add_observer(on_cell_out)
            .add_observer(on_zone_over)
            .add_observer(on_zone_out)
            .add_observer(on_button_over)
            .add_observer(on_button_out)
            .add_systems(
                Update,
                (
                    load_icons.run_if(resource_exists_and_changed::<ItemCatalog>),
                    prepare_icons.run_if(icons_pending),
                ),
            )
            // Une condition par système, évaluée juste avant lui : la fermeture retire l'écran
            // entre deux systèmes du groupe (AGENTS.md §4, conditions d'un groupe chaîné).
            .add_systems(
                Update,
                (
                    open_screen,
                    screen_keys.run_if(resource_exists::<CharacterScreen>),
                    finish_close.run_if(resource_exists::<CharacterScreen>),
                    render_content.run_if(resource_exists::<CharacterScreen>),
                    render_focus.run_if(resource_exists::<CharacterScreen>),
                )
                    .chain()
                    .after(crate::career::CareerSystems),
            )
            .add_systems(OnExit(AppState::InGame), |mut commands: Commands| {
                commands.remove_resource::<CharacterScreen>();
            });
        naming::plugin(app);
        // Autopilote : un casier ouvert, des objets en poche, pour les captures.
        if let Ok(id) = std::env::var("SH_CONTAINER") {
            app.insert_resource(ContainerShot(id.trim().to_owned()))
                .add_systems(Update, container_shot.run_if(resource_exists::<Career>));
        }
        if let Ok(spec) = std::env::var("SH_GIVE") {
            app.insert_resource(GiveShot(logic::parse_counts(&spec, 999)))
                .add_systems(Update, give_items.run_if(resource_exists::<Career>));
        }
        if let Ok(spec) = std::env::var("SH_XP") {
            app.insert_resource(XpShot(logic::parse_counts(&spec, 1_000_000)))
                .add_systems(Update, give_xp.run_if(resource_exists::<Career>));
        }
        // M10 : des casiers déjà nommés (plaque, invite, titre du rangement), la saisie ouverte.
        if let Ok(spec) = std::env::var("SH_LOCKER_NAME") {
            app.insert_resource(NameShot(logic::parse_names(&spec)))
                .add_systems(Update, give_names.run_if(resource_exists::<Career>));
        }
        if let Ok(id) = std::env::var("SH_NAMING") {
            app.insert_resource(NamingShot(id.trim().to_owned()))
                .add_systems(Update, naming_shot.run_if(resource_exists::<Career>));
        }
    }
}

// --- État de l'écran ---------------------------------------------------------------------------

/// Ce que montre l'écran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenView {
    Tab(CharacterTab),
    /// Un casier, par son identifiant (`items::Containers`), à côté de l'inventaire.
    Container(String),
}

/// Une case de l'écran : une case d'une grille (par son rang) ou un emplacement d'équipement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellRef {
    /// L'inventaire.
    Pack(usize),
    /// Le casier ouvert.
    Stash(usize),
    Slot(EquipSlot),
}

/// La case survolée (souris), la dernière case pleine survolée, et la case choisie (clic,
/// clavier). Le détail montre la première des trois qui contient un objet.
///
/// M11 (retour de l'auteur) : en glissant d'une case à l'autre, le détail ne revient plus à la
/// case choisie. La zone de survol d'une case couvre l'espace qui la sépare de ses voisines
/// (`draw::spawn_cell`), et une case vide survolée garde le détail de la dernière case pleine
/// (`pointed`). Il ne revient à la case choisie que quand la souris quitte vraiment les cases
/// (ni sur une case, ni dans une zone qui les relie, [`CellZone`]) : hors des cases, le détail
/// décrit la case marquée en ambre, celle sur laquelle agissent Entrée et le bouton du détail.
/// Le garder sur la dernière case survolée tromperait : pour atteindre le bouton du détail, la
/// souris traverse souvent d'autres cases, et le bouton serait celui de la dernière traversée.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Focus {
    hovered: Option<CellRef>,
    /// La dernière case pleine survolée, tant que la souris reste sur les cases.
    pointed: Option<CellRef>,
    selected: Option<CellRef>,
}

impl Focus {
    fn shown(&self, content: &Content) -> Option<CellRef> {
        let filled = |at: &CellRef| content.item_at(*at).is_some();
        self.hovered
            .filter(filled)
            .or(self.pointed.filter(filled))
            .or(self.selected.filter(filled))
    }

    /// Une fois passés tous les événements du pointeur d'un frame (la sortie d'une case précède
    /// l'entrée dans la voisine) : si la souris n'est plus sur une case ni dans une zone qui les
    /// relie (`in_zone`), elle a vraiment quitté les cases, et le détail oublie la dernière case
    /// survolée pour revenir à la case choisie.
    fn settle(&mut self, in_zone: bool) {
        if self.hovered.is_none() && !in_zone {
            self.pointed = None;
        }
    }
}

/// Une zone de cases (une grille, les deux grilles d'un casier) : tant que la souris y reste,
/// même entre deux cases ou entre les deux grilles, le détail garde la dernière case pleine
/// survolée.
#[derive(Component)]
struct CellZone;

/// L'écran du personnage ouvert.
#[derive(Resource, Debug)]
pub struct CharacterScreen {
    pub view: ScreenView,
    /// Le jeu de commandes à rendre en fermant.
    resume: ControlMode,
    /// Les touches de l'écran ont été relâchées depuis l'ouverture : un nouvel appui compte.
    armed: bool,
    /// Fermeture demandée : effective quand les touches sont relâchées.
    closing: bool,
    focus: Focus,
    /// La zone de cases sous la souris ([`CellZone`]), tenue par les entrées et sorties du
    /// pointeur.
    pointer_zone: Option<Entity>,
    /// Au prochain dessin, choisir la première case pleine (ouverture, changement d'onglet).
    auto_select: bool,
    /// Au prochain dessin, choisir la case de cet objet dans l'inventaire (objet retiré).
    select_item: Option<String>,
    /// Le dernier objet que le rangement a refusé (rangement spécialisé) : l'écran le dit.
    refused: Option<String>,
    /// Le contenu dessiné ; `None` : à redessiner.
    drawn: Option<Content>,
    drawn_date: Option<sh_core::StellarDate>,
    drawn_focus: Option<Focus>,
    /// La case dont le panneau de détail est dessiné : il n'est refait que si elle change (une
    /// case vide survolée ne le redessine pas).
    drawn_shown: Option<Option<CellRef>>,
}

impl CharacterScreen {
    fn new(view: ScreenView, resume: ControlMode) -> Self {
        Self {
            view,
            resume,
            armed: false,
            closing: false,
            focus: Focus::default(),
            pointer_zone: None,
            auto_select: true,
            select_item: None,
            refused: None,
            drawn: None,
            drawn_date: None,
            drawn_focus: None,
            drawn_shown: None,
        }
    }

    /// Change de vue (onglet, casier) : tout est redessiné.
    fn show(&mut self, view: ScreenView) {
        if self.view != view {
            self.view = view;
            self.focus = Focus::default();
            self.auto_select = true;
            self.refused = None;
            self.drawn = None;
        }
    }

    /// L'objet que le rangement vient de refuser (aucun : le message s'efface). Le message fait
    /// partie du contenu : un changement redessine l'écran.
    fn refuse(&mut self, item: Option<String>) {
        if self.refused != item {
            self.refused = item;
            self.drawn = None;
        }
    }
}

/// L'écran est-il affiché ? (Scénarios de l'autopilote.)
pub fn is_shown(world: &mut World) -> bool {
    let mut roots = world.query_filtered::<(), With<CharacterUi>>();
    roots.iter(world).next().is_some()
}

/// La racine de l'écran.
#[derive(Component)]
struct CharacterUi;

/// Une case dessinée.
#[derive(Component, Clone, Copy, Debug)]
struct GridCell {
    at: CellRef,
    filled: bool,
}

/// Le panneau de détail, refait à chaque changement de case survolée ou choisie.
#[derive(Component)]
struct DetailPane;

/// Les boutons de l'écran.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
enum ScreenButton {
    Tab(CharacterTab),
    Equip(String),
    Unequip(EquipSlot),
    Close,
}

/// L'onglet affiché : son bouton reste allumé, le survol n'y touche pas.
#[derive(Component)]
struct ActiveTab;

/// Les icônes des objets, chargées une fois pour toutes (poignées gardées vivantes).
#[derive(Resource, Default)]
struct ItemIcons {
    handles: BTreeMap<String, Handle<Image>>,
    /// Icônes dont les mipmaps restent à calculer (lecture en cours).
    pending: Vec<Handle<Image>>,
}

// --- Ouverture et fermeture --------------------------------------------------------------------

/// Délai pendant lequel une demande d'ouverture attend que le jeu la permette : la fiche du
/// terminal peut fermer le terminal et demander l'écran au même moment.
const PENDING_SECS: f32 = 1.0;

#[derive(Default)]
struct Pending {
    view: Option<ScreenView>,
    age: f32,
}

/// Ce qui interdit d'ouvrir l'écran (ou la saisie d'un nom) : un autre écran, la mort, une
/// manœuvre.
#[derive(SystemParam)]
struct Blockers<'w> {
    terminal: Option<Res<'w, Terminal>>,
    paused: Option<Res<'w, Paused>>,
    death: Option<Res<'w, Death>>,
    sequence: Option<Res<'w, Sequence>>,
    naming: Option<Res<'w, NamingScreen>>,
}

impl Blockers<'_> {
    fn any(&self) -> bool {
        self.terminal.is_some()
            || self.paused.is_some()
            || self.death.is_some()
            || self.sequence.is_some()
            || self.naming.is_some()
    }
}

fn open_screen(
    mut commands: Commands,
    mut tabs: MessageReader<OpenCharacterScreen>,
    mut lockers: MessageReader<OpenContainer>,
    mut pending: Local<Pending>,
    time: Res<Time<Real>>,
    mode: Res<State<ControlMode>>,
    mut next_mode: ResMut<NextState<ControlMode>>,
    screen: Option<ResMut<CharacterScreen>>,
    blockers: Blockers,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut wanted = None;
    for OpenCharacterScreen(tab) in tabs.read() {
        wanted = Some(ScreenView::Tab(*tab));
    }
    for OpenContainer(id) in lockers.read() {
        wanted = Some(ScreenView::Container(id.clone()));
    }
    if let Some(mut screen) = screen {
        // Déjà ouvert : la demande change seulement de vue.
        if let Some(view) = wanted
            && !screen.closing
        {
            screen.show(view);
        }
        pending.view = None;
        return;
    }
    if let Some(view) = wanted {
        *pending = Pending {
            view: Some(view),
            age: 0.0,
        };
    }
    let Some(view) = pending.view.clone() else {
        return;
    };
    let resume = *mode.get();
    // À pied ou aux commandes seulement (ni carte, ni menu), et rien d'autre à l'écran.
    if matches!(resume, ControlMode::OnFoot | ControlMode::Piloting) && !blockers.any() {
        pending.view = None;
        commands.insert_resource(CharacterScreen::new(view, resume));
        next_mode.set(ControlMode::Menu);
        for mut cursor in &mut cursors {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
        return;
    }
    pending.age += time.delta_secs();
    if pending.age > PENDING_SECS {
        pending.view = None;
    }
}

/// Les touches qui ouvrent et ferment l'écran, lues dans les liaisons des actions : elles
/// suivent le remappage (le contexte Menu, actif pendant l'écran, ne les déclare pas).
#[derive(SystemParam)]
struct ScreenKeys<'w, 's> {
    input: Res<'w, ButtonInput<KeyCode>>,
    inventory: Query<'w, 's, &'static Bindings, With<Action<OpenInventory>>>,
    skills: Query<'w, 's, &'static Bindings, With<Action<OpenSkills>>>,
    interact: Query<'w, 's, &'static Bindings, With<Action<Interact>>>,
    bindings: Query<'w, 's, &'static Binding>,
}

impl ScreenKeys<'_, '_> {
    fn keyboard<'a>(&self, actions: impl Iterator<Item = &'a Bindings>) -> Vec<KeyCode> {
        actions
            .flat_map(|bindings| bindings.into_iter())
            .filter_map(|entity| match self.bindings.get(entity) {
                Ok(Binding::Keyboard { key, .. }) => Some(*key),
                _ => None,
            })
            .collect()
    }

    fn inventory(&self) -> Vec<KeyCode> {
        self.keyboard(self.inventory.iter())
    }

    fn skills(&self) -> Vec<KeyCode> {
        self.keyboard(self.skills.iter())
    }

    fn interact(&self) -> Vec<KeyCode> {
        self.keyboard(self.interact.iter())
    }

    fn pressed(&self, keys: &[KeyCode]) -> bool {
        keys.iter().any(|key| self.input.pressed(*key))
    }

    fn just_pressed(&self, keys: &[KeyCode]) -> bool {
        keys.iter().any(|key| self.input.just_pressed(*key))
    }

    /// Une touche de l'écran (ou Échap) est encore enfoncée.
    fn any_held(&self) -> bool {
        self.input.pressed(KeyCode::Escape)
            || self.pressed(&self.inventory())
            || self.pressed(&self.skills())
            || self.pressed(&self.interact())
    }
}

/// La touche de l'onglet ouvert le ferme, celle de l'autre onglet y passe ; un casier se ferme
/// aussi par la touche d'interaction qui l'a ouvert.
fn screen_keys(keys: ScreenKeys, death: Option<Res<Death>>, mut screen: ResMut<CharacterScreen>) {
    if screen.closing {
        return;
    }
    if death.is_some() {
        screen.closing = true;
        return;
    }
    let (inventory, skills, interact) = (keys.inventory(), keys.skills(), keys.interact());
    if !screen.armed {
        // La touche qui a ouvert l'écran doit d'abord être relâchée.
        if !keys.pressed(&inventory) && !keys.pressed(&skills) && !keys.pressed(&interact) {
            screen.armed = true;
        }
        return;
    }
    let (close, switch) = match &screen.view {
        ScreenView::Tab(CharacterTab::Inventory) => (
            keys.just_pressed(&inventory),
            keys.just_pressed(&skills).then_some(CharacterTab::Skills),
        ),
        ScreenView::Tab(CharacterTab::Skills) => (
            keys.just_pressed(&skills),
            keys.just_pressed(&inventory)
                .then_some(CharacterTab::Inventory),
        ),
        ScreenView::Container(_) => (
            keys.just_pressed(&inventory) || keys.just_pressed(&interact),
            keys.just_pressed(&skills).then_some(CharacterTab::Skills),
        ),
    };
    if close {
        screen.closing = true;
    } else if let Some(tab) = switch {
        screen.show(ScreenView::Tab(tab));
    }
}

/// Échap (action Retour du contexte Menu) ferme l'écran.
fn on_back(_: On<Start<Back>>, screen: Option<ResMut<CharacterScreen>>) {
    if let Some(mut screen) = screen {
        screen.closing = true;
    }
}

/// Ferme l'écran une fois ses touches relâchées : rend les commandes et le curseur.
fn finish_close(
    mut commands: Commands,
    keys: ScreenKeys,
    screen: Res<CharacterScreen>,
    view: Option<Res<State<ViewLevel>>>,
    mut next_mode: ResMut<NextState<ControlMode>>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    roots: Query<Entity, With<CharacterUi>>,
) {
    if !screen.closing || keys.any_held() {
        return;
    }
    next_mode.set(screen.resume);
    commands.remove_resource::<CharacterScreen>();
    // L'écran part avec sa ressource : sans elle, plus rien ne l'effacerait.
    for root in &roots {
        commands.entity(root).despawn();
    }
    if view.is_some_and(|v| matches!(v.get(), ViewLevel::Sector | ViewLevel::Global)) {
        for mut cursor in &mut cursors {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

// --- Actions sur les objets --------------------------------------------------------------------

/// Les objets du joueur, modifiés par l'écran (ressources d'une partie : absentes hors partie).
#[derive(SystemParam)]
struct Stores<'w> {
    catalog: Option<Res<'w, ItemCatalog>>,
    inventory: Option<ResMut<'w, Inventory>>,
    equipment: Option<ResMut<'w, Equipment>>,
    containers: Option<ResMut<'w, Containers>>,
}

/// Une action du joueur sur ses objets.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Deed {
    Equip(String),
    Unequip(EquipSlot),
    /// Du casier vers l'inventaire.
    Take {
        item: String,
        count: u32,
    },
    /// De l'inventaire vers le casier.
    Store {
        item: String,
        count: u32,
    },
}

/// L'action d'une case : équiper ou retirer (inventaire), déplacer (casier). `one` : un seul
/// exemplaire au lieu de la case entière.
fn deed_for(content: &Content, at: CellRef, catalog: &ItemsFile, one: bool) -> Option<Deed> {
    let (item, count) = content.item_at(at)?;
    let count = if one { 1 } else { count };
    match (&content.body, at) {
        (Body::Inventory(_), CellRef::Pack(_)) => catalog
            .get(item)?
            .slot
            .map(|_| Deed::Equip(item.to_owned())),
        (Body::Inventory(_), CellRef::Slot(slot)) => Some(Deed::Unequip(slot)),
        (Body::Container(_), CellRef::Stash(_)) => Some(Deed::Take {
            item: item.to_owned(),
            count,
        }),
        (Body::Container(_), CellRef::Pack(_)) => Some(Deed::Store {
            item: item.to_owned(),
            count,
        }),
        _ => None,
    }
}

fn perform(deed: Deed, screen: &mut CharacterScreen, stores: &mut Stores) {
    let (Some(catalog), Some(inventory)) = (stores.catalog.as_deref(), stores.inventory.as_mut())
    else {
        return;
    };
    match deed {
        Deed::Equip(item) => {
            let Some(equipment) = stores.equipment.as_mut() else {
                return;
            };
            if let Some(slot) = logic::equip(&mut inventory.items, &mut equipment.0, catalog, &item)
            {
                screen.focus.selected = Some(CellRef::Slot(slot));
            }
        }
        Deed::Unequip(slot) => {
            let Some(equipment) = stores.equipment.as_mut() else {
                return;
            };
            if let Some(item) = logic::unequip(&mut inventory.items, &mut equipment.0, slot) {
                screen.select_item = Some(item);
            }
        }
        Deed::Take { item, count } => {
            let (ScreenView::Container(id), Some(containers)) =
                (&screen.view, stores.containers.as_mut())
            else {
                return;
            };
            let at = logic::Place {
                ship: PLAYER_SHIP,
                container: id,
            };
            logic::retrieve(&mut inventory.items, containers, at, &item, count);
            screen.refuse(None);
        }
        Deed::Store { item, count } => {
            let (ScreenView::Container(id), Some(containers)) =
                (&screen.view, stores.containers.as_mut())
            else {
                return;
            };
            let at = logic::Place {
                ship: PLAYER_SHIP,
                container: id,
            };
            let stored = logic::store(&mut inventory.items, containers, catalog, at, &item, count);
            // Refusé (le frigo et ce qui ne se mange pas) : l'écran le dit, rien ne bouge.
            screen.refuse(stored.is_none().then_some(item));
        }
    }
}

/// Clic sur une case. Inventaire : clic pour examiner, clic droit pour équiper ou retirer.
/// Casier : clic pour déplacer la case entière, clic droit pour un seul exemplaire.
fn on_cell_click(
    event: On<PointerClick>,
    cells: Query<&GridCell>,
    screen: Option<ResMut<CharacterScreen>>,
    mut stores: Stores,
) {
    let (Ok(cell), Some(mut screen)) = (cells.get(event.entity), screen) else {
        return;
    };
    if screen.closing {
        return;
    }
    let secondary = match event.button {
        PointerButton::Primary => false,
        PointerButton::Secondary => true,
        PointerButton::Middle => return,
    };
    let deed = {
        let (Some(content), Some(catalog)) = (screen.drawn.as_ref(), stores.catalog.as_deref())
        else {
            return;
        };
        let inventory_tab = matches!(content.body, Body::Inventory(_));
        if inventory_tab && !secondary {
            None
        } else {
            deed_for(content, cell.at, catalog, secondary && !inventory_tab)
        }
    };
    if !matches!(screen.view, ScreenView::Container(_)) {
        screen.focus.selected = Some(cell.at);
    }
    if let Some(deed) = deed {
        perform(deed, &mut screen, &mut stores);
    }
}

/// Entrée : l'action de la case choisie (à défaut, survolée).
fn on_confirm(_: On<Start<Confirm>>, screen: Option<ResMut<CharacterScreen>>, mut stores: Stores) {
    let Some(mut screen) = screen else {
        return;
    };
    if screen.closing {
        return;
    }
    let deed = {
        let (Some(content), Some(catalog)) = (screen.drawn.as_ref(), stores.catalog.as_deref())
        else {
            return;
        };
        let Some(at) = screen.focus.selected.or(screen.focus.hovered) else {
            return;
        };
        deed_for(content, at, catalog, false)
    };
    if let Some(deed) = deed {
        perform(deed, &mut screen, &mut stores);
    }
}

/// Flèches : la case choisie se déplace (d'une grille à l'autre, ou vers l'équipement).
fn on_navigate(event: On<Start<Navigate>>, screen: Option<ResMut<CharacterScreen>>) {
    let Some(mut screen) = screen else {
        return;
    };
    let axis = |v: f32| {
        if v > 0.5 {
            1
        } else if v < -0.5 {
            -1
        } else {
            0
        }
    };
    // Vers le haut : la rangée précédente.
    let (dx, dy) = (axis(event.value.x), -axis(event.value.y));
    let next = screen
        .drawn
        .as_ref()
        .and_then(|content| content.step(screen.focus.selected, dx, dy));
    if next.is_some() && next != screen.focus.selected {
        screen.focus.selected = next;
        // Le clavier prend la main sur la souris jusqu'à son prochain survol : le détail suit
        // la case choisie, plus la dernière survolée.
        screen.focus.hovered = None;
        screen.focus.pointed = None;
    }
}

fn on_activate(
    event: On<Activate>,
    buttons: Query<&ScreenButton>,
    screen: Option<ResMut<CharacterScreen>>,
    mut stores: Stores,
) {
    let (Ok(button), Some(mut screen)) = (buttons.get(event.entity), screen) else {
        return;
    };
    if screen.closing {
        return;
    }
    match button.clone() {
        ScreenButton::Tab(tab) => screen.show(ScreenView::Tab(tab)),
        ScreenButton::Equip(item) => perform(Deed::Equip(item), &mut screen, &mut stores),
        ScreenButton::Unequip(slot) => perform(Deed::Unequip(slot), &mut screen, &mut stores),
        ScreenButton::Close => screen.closing = true,
    }
}

/// Une case survolée (directement, ou par sa zone de survol qui déborde : l'événement remonte
/// de la zone à la case). Pleine, elle devient celle que montre le détail.
fn on_cell_over(
    event: On<PointerOver>,
    cells: Query<&GridCell>,
    screen: Option<ResMut<CharacterScreen>>,
) {
    let (Ok(cell), Some(mut screen)) = (cells.get(event.entity), screen) else {
        return;
    };
    if screen.focus.hovered != Some(cell.at) {
        screen.focus.hovered = Some(cell.at);
    }
    if cell.filled && screen.focus.pointed != Some(cell.at) {
        screen.focus.pointed = Some(cell.at);
    }
}

/// La souris quitte une case : elle n'est plus survolée, mais le détail garde son objet tant
/// que la souris reste sur les cases (`render_focus` en juge, une fois tous les événements du
/// frame passés).
fn on_cell_out(
    event: On<PointerOut>,
    cells: Query<&GridCell>,
    screen: Option<ResMut<CharacterScreen>>,
) {
    let (Ok(cell), Some(mut screen)) = (cells.get(event.entity), screen) else {
        return;
    };
    // Seulement si elle est encore la case survolée : l'entrée dans la suivante a pu passer
    // avant la sortie de celle-ci.
    if screen.focus.hovered == Some(cell.at) {
        screen.focus.hovered = None;
    }
}

/// La souris entre dans une zone de cases (ou passe d'un de ses éléments à un autre : la
/// sortie de l'un et l'entrée dans l'autre remontent toutes deux jusqu'à la zone, dans cet
/// ordre, au même frame).
fn on_zone_over(
    event: On<PointerOver>,
    zones: Query<(), With<CellZone>>,
    screen: Option<ResMut<CharacterScreen>>,
) {
    let Some(mut screen) = screen.filter(|_| zones.contains(event.entity)) else {
        return;
    };
    if screen.pointer_zone != Some(event.entity) {
        screen.pointer_zone = Some(event.entity);
    }
}

fn on_zone_out(
    event: On<PointerOut>,
    zones: Query<(), With<CellZone>>,
    screen: Option<ResMut<CharacterScreen>>,
) {
    let Some(mut screen) = screen.filter(|_| zones.contains(event.entity)) else {
        return;
    };
    if screen.pointer_zone == Some(event.entity) {
        screen.pointer_zone = None;
    }
}

fn on_button_over(
    event: On<PointerOver>,
    fonts: Res<UiFonts>,
    mut buttons: Query<
        (Option<&mut ImageNode>, Option<&mut BackgroundColor>),
        (With<ScreenButton>, Without<ActiveTab>),
    >,
) {
    if let Ok((image, background)) = buttons.get_mut(event.entity) {
        theme::set_hovered(&fonts, true, image, background);
    }
}

fn on_button_out(
    event: On<PointerOut>,
    fonts: Res<UiFonts>,
    mut buttons: Query<
        (Option<&mut ImageNode>, Option<&mut BackgroundColor>),
        (With<ScreenButton>, Without<ActiveTab>),
    >,
) {
    if let Ok((image, background)) = buttons.get_mut(event.entity) {
        theme::set_hovered(&fonts, false, image, background);
    }
}

// --- Affichage ---------------------------------------------------------------------------------

/// Ce que l'écran lit pour se dessiner.
#[derive(SystemParam)]
struct Sources<'w> {
    texts: Res<'w, Texts>,
    data: Res<'w, Data>,
    inventory: Option<Res<'w, Inventory>>,
    equipment: Option<Res<'w, Equipment>>,
    containers: Option<Res<'w, Containers>>,
    career: Option<Res<'w, Career>>,
    clock: Option<Res<'w, GameClock>>,
}

impl Sources<'_> {
    /// Quelque chose de ce que montre l'écran a pu changer (l'horloge compte à part : elle
    /// avance à chaque frame, seule sa date importe).
    fn changed(&self) -> bool {
        self.texts.is_changed()
            || self.data.is_changed()
            || self.inventory.as_ref().is_some_and(|r| r.is_changed())
            || self.equipment.as_ref().is_some_and(|r| r.is_changed())
            || self.containers.as_ref().is_some_and(|r| r.is_changed())
            || self.career.as_ref().is_some_and(|r| r.is_changed())
    }
}

/// Les icônes prêtes : celle d'un objet, sauf si son fichier est illisible (une abréviation la
/// remplace alors).
struct Icons<'a> {
    icons: Option<&'a ItemIcons>,
    server: &'a AssetServer,
}

impl Icons<'_> {
    fn get(&self, item: &str) -> Option<Handle<Image>> {
        let handle = self.icons?.handles.get(item)?;
        (!self.server.load_state(handle).is_failed()).then(|| handle.clone())
    }
}

/// Redessine l'écran quand son contenu change (ou le retire dès que la fermeture est demandée).
fn render_content(
    mut commands: Commands,
    sources: Sources,
    fonts: Res<UiFonts>,
    icons: Option<Res<ItemIcons>>,
    server: Res<AssetServer>,
    mut screen: ResMut<CharacterScreen>,
    roots: Query<Entity, With<CharacterUi>>,
) {
    if screen.closing {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    }
    let date = sources.clock.as_ref().map(|c| c.date);
    let fresh = screen.drawn.is_none() || roots.is_empty();
    if !fresh && !sources.changed() && screen.drawn_date == date {
        return;
    }
    let content = draw::content(&sources, &screen.view, screen.refused.as_deref());
    if !fresh && screen.drawn.as_ref() == Some(&content) {
        screen.drawn_date = date;
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    let icons = Icons {
        icons: icons.as_deref(),
        server: &server,
    };
    let kit = draw::Kit {
        fonts: &fonts,
        icons: &icons,
        t: &sources.texts,
        data: &sources.data,
    };
    draw::spawn_screen(&mut commands, &kit, &content);
    // La case choisie : la première pleine à l'ouverture, celle de l'objet qu'on vient de
    // retirer, et jamais une case qui n'existe plus.
    if let Some(item) = screen.select_item.take() {
        screen.focus.selected = content.pack_cell_of(&item).or(screen.focus.selected);
    }
    if screen.auto_select
        || screen
            .focus
            .selected
            .is_some_and(|at| !content.has_cell(at))
    {
        screen.focus.selected = content.first_filled();
        screen.auto_select = false;
    }
    screen.drawn = Some(content);
    screen.drawn_date = date;
    screen.drawn_focus = None;
    // Un panneau de détail neuf, vide : à remplir.
    screen.drawn_shown = None;
}

/// Survol et choix : couleurs des cases et panneau de détail, sans redessiner le reste.
fn render_focus(
    mut commands: Commands,
    texts: Res<Texts>,
    data: Res<Data>,
    fonts: Res<UiFonts>,
    icons: Option<Res<ItemIcons>>,
    server: Res<AssetServer>,
    mut screen: ResMut<CharacterScreen>,
    mut cells: Query<(&GridCell, &mut BorderColor, &mut BackgroundColor)>,
    panes: Query<Entity, With<DetailPane>>,
    zones: Query<(), With<CellZone>>,
) {
    if screen.closing {
        return;
    }
    // Ici, après les observateurs du pointeur (`PreUpdate`) : la souris a-t-elle vraiment quitté
    // les cases ? (Une zone effacée par un redessin ne compte plus : sa sortie n'arrive jamais.)
    if screen.focus.pointed.is_some() {
        let in_zone = screen.pointer_zone.is_some_and(|zone| zones.contains(zone));
        screen.focus.settle(in_zone);
    }
    let Some(content) = screen.drawn.as_ref() else {
        return;
    };
    let shown = screen.focus.shown(content);
    let recolor = screen.drawn_focus.as_ref() != Some(&screen.focus);
    let redetail = screen.drawn_shown != Some(shown);
    if !recolor && !redetail {
        return;
    }
    if content.has_cells() {
        // Le panneau vient d'être créé par le dessin : il existe dès la frame suivante.
        let Ok(pane) = panes.single() else {
            return;
        };
        if recolor {
            for (cell, mut border, mut background) in &mut cells {
                let (fill, edge) = draw::cell_colors(
                    cell.filled,
                    screen.focus.hovered == Some(cell.at),
                    screen.focus.selected == Some(cell.at),
                );
                background.0 = fill;
                *border = BorderColor::all(edge);
            }
        }
        // Seulement si la case montrée change : glisser sur une case vide ou entre deux cases
        // ne refait pas le panneau.
        if redetail {
            let icons = Icons {
                icons: icons.as_deref(),
                server: &server,
            };
            let kit = draw::Kit {
                fonts: &fonts,
                icons: &icons,
                t: &texts,
                data: &data,
            };
            commands.entity(pane).despawn_children();
            draw::spawn_detail(&mut commands, pane, &kit, content, shown);
        }
    }
    let focus = screen.focus.clone();
    screen.drawn_focus = Some(focus);
    screen.drawn_shown = Some(shown);
}

// --- Icônes ------------------------------------------------------------------------------------

fn load_icons(mut commands: Commands, catalog: Res<ItemCatalog>, server: Res<AssetServer>) {
    let handles: BTreeMap<String, Handle<Image>> = catalog
        .items
        .iter()
        .map(|item| (item.id.clone(), server.load(item.icon_path())))
        .collect();
    let pending = handles.values().cloned().collect();
    commands.insert_resource(ItemIcons { handles, pending });
}

fn icons_pending(icons: Option<Res<ItemIcons>>) -> bool {
    icons.is_some_and(|icons| !icons.pending.is_empty())
}

/// Mipmaps des icônes : dessinées en 52 px depuis 256 px, elles crénelleraient sans elles.
fn prepare_icons(
    mut icons: ResMut<ItemIcons>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
) {
    let mut waiting = Vec::new();
    for handle in std::mem::take(&mut icons.pending) {
        let Some(image) = images.get(&handle) else {
            if server.load_state(&handle).is_failed() {
                warn!("icône d'objet illisible : {:?}", handle.path());
            } else {
                waiting.push(handle);
            }
            continue;
        };
        let size = image.texture_descriptor.size;
        let rgba8 = matches!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm
        );
        // Une image déjà traitée (catalogue rechargé) a déjà sa chaîne : rien à refaire.
        let mips = if rgba8 && image.texture_descriptor.mip_level_count == 1 {
            image.data.as_deref().and_then(|data| {
                crate::materials::mip_chain(
                    data,
                    size.width,
                    size.height,
                    crate::materials::TextureKind::Color,
                )
            })
        } else {
            None
        };
        if let Some(mut image) = images.get_mut(&handle) {
            if let Some((data, levels)) = mips {
                image.data = Some(data);
                image.texture_descriptor.mip_level_count = levels;
            }
            image.sampler = ImageSampler::linear();
        }
    }
    icons.pending = waiting;
}

// --- Autopilote --------------------------------------------------------------------------------

/// `SH_CONTAINER=<id>` : ouvre ce casier (captures du rangement).
#[derive(Resource)]
struct ContainerShot(String);

fn container_shot(
    time: Res<Time<Real>>,
    shot: Res<ContainerShot>,
    mut open: MessageWriter<OpenContainer>,
    mut since: Local<f32>,
) {
    let before = *since;
    *since += time.delta_secs();
    if before <= 3.0 && *since > 3.0 {
        open.write(OpenContainer(shot.0.clone()));
    }
}

/// `SH_GIVE=<objet>:<n>,…` : objets ajoutés à l'inventaire au début de la partie (captures).
#[derive(Resource)]
struct GiveShot(Vec<(String, u32)>);

fn give_items(
    shot: Res<GiveShot>,
    catalog: Option<Res<ItemCatalog>>,
    inventory: Option<ResMut<Inventory>>,
    mut done: Local<bool>,
) {
    let (false, Some(catalog), Some(mut inventory)) = (*done, catalog, inventory) else {
        return;
    };
    *done = true;
    for (item, count) in &shot.0 {
        if catalog.get(item).is_none() {
            warn!("SH_GIVE : objet inconnu « {item} »");
            continue;
        }
        logic::put(&mut inventory.items, item, *count);
    }
}

/// `SH_XP=<compétence>:<points>,…` : expérience posée au début de la partie (captures des
/// barres de l'onglet Compétences).
#[derive(Resource)]
struct XpShot(Vec<(String, u32)>);

fn give_xp(shot: Res<XpShot>, data: Res<Data>, mut career: ResMut<Career>, mut done: Local<bool>) {
    if *done {
        return;
    }
    *done = true;
    for (skill, xp) in &shot.0 {
        if !data.career.skills.contains(skill) {
            warn!("SH_XP : compétence inconnue « {skill} »");
            continue;
        }
        career.captain.skills.insert(skill.clone(), f64::from(*xp));
    }
}

/// `SH_LOCKER_NAME=<conteneur>:<nom>,…` : casiers du vaisseau du joueur nommés au début de la
/// partie (captures de la plaque, de l'invite et du titre du rangement).
#[derive(Resource)]
struct NameShot(Vec<(String, String)>);

fn give_names(shot: Res<NameShot>, containers: Option<ResMut<Containers>>, mut done: Local<bool>) {
    let (false, Some(mut containers)) = (*done, containers) else {
        return;
    };
    *done = true;
    for (container, name) in &shot.0 {
        containers.set_name(PLAYER_SHIP, container, name);
    }
}

/// `SH_NAMING=<conteneur>` : la saisie du nom de ce casier, ouverte 3 s après l'entrée en
/// partie (capture de la fenêtre).
#[derive(Resource)]
struct NamingShot(String);

fn naming_shot(
    time: Res<Time<Real>>,
    shot: Res<NamingShot>,
    mut open: MessageWriter<crate::items::NameContainer>,
    mut since: Local<f32>,
) {
    let before = *since;
    *since += time.delta_secs();
    if before <= 3.0 && *since > 3.0 {
        open.write(crate::items::NameContainer(shot.0.clone()));
    }
}
