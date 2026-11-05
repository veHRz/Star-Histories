//! Sauvegarde et chargement (M7 ; Q7 : sauvegarde libre, option Homme de fer).
//!
//! Une sauvegarde est un fichier RON versionné ([`FORMAT_VERSION`]) du dossier du joueur
//! ([`user_dir`]) : le monde simulé sans ses constantes (reconstruites depuis les données), la
//! fiche du capitaine, l'horloge, le lieu, les objets (inventaire, casiers, objets pris dans le
//! monde, équipement : [`SavedItems`]). On sauvegarde à quai ou en vol, jamais en combat ni
//! pendant une manœuvre. Charger passe par le menu principal : quitter la partie l'efface
//! (`state::forget_game`, `items::forget_items`), puis les ressources de la sauvegarde remplacent
//! celles qu'une nouvelle partie aurait créées (monde, horloge, carrière et objets ne se recréent
//! que s'ils manquent).
//!
//! Le format ne change de version que si une ancienne sauvegarde ne peut plus se relire : un
//! champ ajouté a une valeur par défaut (`#[serde(default)]`) et la version reste la même.
//!
//! Le **contenu** a sa propre version ([`CONTENT_VERSION`]) : les identifiants de secteurs et de
//! points de ramassage qu'une sauvegarde cite. Une sauvegarde d'un contenu plus ancien se relit
//! telle quelle puis se migre au chargement ([`migrate_place`], [`migrate_items`]) : depuis M9, le
//! vaisseau du joueur est l'intérieur type `corvette` (avant : `player_ship`), Hexis le
//! spatioport par défaut `spaceport` (avant : `hexis_spaceport`), et la tablette des quartiers
//! est dans la chambre. La sauvegarde ne note pas la position à pied : à quai, on reprend au quai
//! (première apparition du quartier), en vol aux commandes ; un quartier disparu des données
//! retombe sur le spatioport par défaut.
//!
//! Homme de fer : une seule sauvegarde, écrite automatiquement (à quai, à l'arrivée, en quittant)
//! et effacée à la mort.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy_enhanced_input::prelude::Start;
use serde::{Deserialize, Serialize};
use sh_core::{Id, SpacePos, StellarDate};
use sh_data::Place;
use sh_sim::{Captain, SimWorld, WorldSave};

use crate::career::{Career, MissionLog};
use crate::clock::GameClock;
use crate::codex::Codex;
use crate::combat::{Death, Raid};
use crate::data::{Data, Texts};
use crate::input::{QuickLoad, QuickSave};
use crate::interaction::Inventory;
use crate::items::{Containers, Equipment, PickedUp};
use crate::living::{LivingWorld, PlayerKnowledge};
use crate::sector::{ActiveSector, Docked, PendingAboard};
use crate::space::{SpacePosition, SpaceRotation};
use crate::state::{AppState, CurrentPlace, EnterGame, ViewLevel};
use crate::tips::TipsSeen;
use crate::transition::Sequence;
use crate::tutorial::Tutorial;
use crate::ui::hud::Notice;
use crate::voyage::{Location, ResumeFlight, ShipFuel};

/// Version du format : une sauvegarde d'une autre version est refusée, jamais devinée.
pub const FORMAT_VERSION: u32 = 1;

/// Version du contenu (identifiants cités) : 0 avant M9, 1 depuis les intérieurs types.
pub const CONTENT_VERSION: u32 = 1;

/// Secteurs renommés au contenu 1 (M9) : le vaisseau du joueur devient l'intérieur type des
/// corvettes, le spatioport d'Hexis le spatioport par défaut.
const RENAMED_SECTORS_1: &[(&str, &str)] = &[
    ("player_ship", "corvette"),
    ("hexis_spaceport", "spaceport"),
];

/// Points de ramassage déplacés au contenu 1 (M9) : la tablette des quartiers est dans la chambre.
const MOVED_PICKUPS_1: &[(&str, &str)] = &[(
    "player_ship/quartiers/quartiers_pickup_datapad",
    "corvette/chambre/chambre_pickup_datapad",
)];

/// Le nom actuel d'un secteur cité par une sauvegarde de ce contenu.
fn renamed_sector(id: &str, content: u32) -> &str {
    if content >= 1 {
        return id;
    }
    RENAMED_SECTORS_1
        .iter()
        .find(|(old, _)| *old == id)
        .map_or(id, |(_, new)| new)
}

/// Le lieu d'une sauvegarde de ce contenu, au contenu courant.
pub fn migrate_place(place: SavedPlace, content: u32) -> SavedPlace {
    match place {
        SavedPlace::Docked { district } => SavedPlace::Docked {
            district: renamed_sector(&district, content).to_owned(),
        },
        space => space,
    }
}

/// Les objets d'une sauvegarde de ce contenu, au contenu courant : identifiants des points de
/// ramassage (`<secteur>/<cellule>/<nœud>`) déplacés ou dont le secteur a changé de nom.
pub fn migrate_items(mut items: SavedItems, content: u32) -> SavedItems {
    if content >= 1 {
        return items;
    }
    let moved = |id: &str| -> String {
        if let Some((_, new)) = MOVED_PICKUPS_1.iter().find(|(old, _)| *old == id) {
            return (*new).to_owned();
        }
        match id.split_once('/') {
            Some((sector, rest)) => format!("{}/{rest}", renamed_sector(sector, content)),
            None => id.to_owned(),
        }
    };
    items.picked_up = PickedUp(items.picked_up.0.iter().map(|id| moved(id)).collect());
    items
}

/// Le quartier où reprendre une partie sauvegardée à quai : le sien s'il accueille encore un
/// vaisseau, sinon le spatioport par défaut (un secteur retiré des données ne bloque pas la
/// sauvegarde).
pub fn docking_district(district: &str, accepts_ship: impl Fn(&str) -> bool) -> String {
    if accepts_ship(district) {
        district.to_owned()
    } else {
        sh_data::scene::DEFAULT_SPACEPORT.to_owned()
    }
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMode>()
            .add_message::<GameSaved>()
            .add_observer(on_quick_save)
            .add_observer(on_quick_load)
            .add_systems(
                Update,
                apply_pending_load.run_if(resource_exists::<PendingLoad>),
            )
            .add_systems(
                Update,
                autosave
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Career>),
            );
    }
}

/// La partie en cours : Homme de fer ou non.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameMode {
    pub ironman: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveKind {
    Manual,
    Quick,
    Auto,
    Ironman,
}

impl SaveKind {
    fn text_key(self) -> &'static str {
        match self {
            SaveKind::Manual => "save.kind.manual",
            SaveKind::Quick => "save.kind.quick",
            SaveKind::Auto => "save.kind.auto",
            SaveKind::Ironman => "save.kind.ironman",
        }
    }
}

/// Une sauvegarde vient d'être écrite (le tutoriel attend la première du joueur).
#[derive(Message, Clone, Copy, Debug)]
pub struct GameSaved(pub SaveKind);

/// Ce que montre la liste des sauvegardes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveMeta {
    pub kind: SaveKind,
    pub date: StellarDate,
    pub system: Id<Place>,
    pub docked: bool,
    /// Secondes depuis l'époque Unix : le classement de la liste.
    pub saved_at: u64,
}

/// Où reprend le joueur.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SavedPlace {
    /// À quai : le quartier qui accueille le vaisseau (secteur composé).
    Docked { district: String },
    /// En vol, aux commandes : la pose du vaisseau dans le système.
    Space { pos: SpacePos, rot: [f32; 4] },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    /// Version du contenu ([`CONTENT_VERSION`]) ; absente : avant M9 (0).
    #[serde(default)]
    pub content: u32,
    pub meta: SaveMeta,
    pub mode: GameMode,
    pub world: WorldSave,
    /// Événements du monde déjà relus par le jeu (nouvelles).
    pub read: usize,
    pub knowledge: PlayerKnowledge,
    pub captain: Captain,
    pub last_tick: StellarDate,
    pub missions: MissionLog,
    pub clock: GameClock,
    pub fuel: Option<(f64, f64)>,
    pub inventory: Inventory,
    pub place: SavedPlace,
    /// Fiches découvertes ; absent des premières sauvegardes de M7 (format inchangé).
    #[serde(default)]
    pub codex: Codex,
    /// Conseils déjà montrés.
    #[serde(default)]
    pub tips: TipsSeen,
    /// Avancement du tutoriel (M8). Absent d'une sauvegarde plus ancienne : terminé, on
    /// n'impose pas le début du jeu à une partie déjà avancée.
    #[serde(default = "Tutorial::finished")]
    pub tutorial: Tutorial,
    /// Casiers, objets pris dans le monde, équipement porté (M9). Absent d'une sauvegarde plus
    /// ancienne : rien n'a été pris, casiers et équipement repartent du catalogue.
    #[serde(default)]
    pub items: SavedItems,
}

/// Les objets d'une partie hors inventaire (`items`). Casiers et équipement sont facultatifs :
/// absents, l'entrée en jeu les tire du catalogue (`items::start_items`) au lieu de les laisser
/// vides.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedItems {
    #[serde(default)]
    pub containers: Option<Containers>,
    #[serde(default)]
    pub picked_up: PickedUp,
    #[serde(default)]
    pub equipment: Option<Equipment>,
}

impl SavedItems {
    fn capture(world: &World) -> Self {
        SavedItems {
            containers: world.get_resource::<Containers>().cloned(),
            picked_up: world
                .get_resource::<PickedUp>()
                .cloned()
                .unwrap_or_default(),
            equipment: world.get_resource::<Equipment>().cloned(),
        }
    }

    /// Installe les objets avant l'entrée en jeu : `items::start_items` ne crée que ce qui
    /// manque.
    fn install(self, world: &mut World) {
        if let Some(containers) = self.containers {
            world.insert_resource(containers);
        }
        world.insert_resource(self.picked_up);
        if let Some(equipment) = self.equipment {
            world.insert_resource(equipment);
        }
    }
}

/// Une sauvegarde à charger : appliquée au menu principal, une fois la partie quittée.
#[derive(Resource)]
pub struct PendingLoad(pub Box<SaveFile>);

/// Une sauvegarde de la liste.
#[derive(Clone, Debug)]
pub struct SaveEntry {
    pub path: PathBuf,
    pub meta: SaveMeta,
}

// --- Fichiers ----------------------------------------------------------------------------------

/// Dossier des fichiers du joueur (sauvegardes, réglages) : `SH_USER_DIR` s'il est posé, celui
/// de l'autopilote quand il pilote (ses parties ne touchent jamais celles du joueur), sinon le
/// dossier de données de l'utilisateur (`%APPDATA%\StarHistories`,
/// `~/.local/share/star_histories`).
pub fn user_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("SH_USER_DIR") {
        return PathBuf::from(dir);
    }
    if crate::dev::autopilot_active() {
        return PathBuf::from("target/autopilot-user");
    }
    if let Some(dir) = std::env::var_os("APPDATA") {
        return PathBuf::from(dir).join("StarHistories");
    }
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(dir).join("star_histories");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/star_histories");
    }
    PathBuf::from("user")
}

pub fn saves_dir() -> PathBuf {
    user_dir().join("saves")
}

fn file_name(kind: SaveKind, saved_at: u64) -> String {
    match kind {
        SaveKind::Manual => format!("partie-{saved_at}.ron"),
        SaveKind::Quick => "rapide.ron".into(),
        SaveKind::Auto => "auto.ron".into(),
        SaveKind::Ironman => "homme-de-fer.ron".into(),
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Lit une sauvegarde ; une autre version du format est refusée.
pub fn read_save(path: &Path) -> Result<SaveFile, String> {
    #[derive(Deserialize)]
    struct Version {
        version: u32,
    }
    // La version d'abord : un format différent ne se lirait pas jusqu'au bout.
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let version: Version = sh_data::ron_options()
        .from_str(&text)
        .map_err(|e| e.to_string())?;
    if version.version != FORMAT_VERSION {
        return Err(format!("version {}", version.version));
    }
    sh_data::ron_options()
        .from_str(&text)
        .map_err(|e| e.to_string())
}

/// Les sauvegardes lisibles, de la plus récente à la plus ancienne.
pub fn list_saves() -> Vec<SaveEntry> {
    let Ok(dir) = std::fs::read_dir(saves_dir()) else {
        return Vec::new();
    };
    let mut entries: Vec<SaveEntry> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ron"))
        .filter_map(|path| match read_save(&path) {
            Ok(save) => Some(SaveEntry {
                path,
                meta: save.meta,
            }),
            Err(err) => {
                warn!("sauvegarde ignorée {} : {err}", path.display());
                None
            }
        })
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.meta.saved_at));
    entries
}

/// La ligne d'une sauvegarde dans une liste : genre, date, lieu.
pub fn describe(texts: &Texts, data: &Data, meta: &SaveMeta) -> String {
    let system = data
        .galaxy
        .place_name(&meta.system)
        .map_or_else(|| meta.system.to_string(), str::to_owned);
    let place = if meta.docked {
        texts.format("save.place.docked", &[("place", &system)])
    } else {
        system
    };
    texts.format(
        "save.entry",
        &[
            ("kind", &texts.get(meta.kind.text_key())),
            ("date", &crate::clock::format_date(texts, meta.date)),
            ("place", &place),
        ],
    )
}

// --- Sauvegarder -------------------------------------------------------------------------------

/// Ce qui empêche de sauvegarder maintenant (clé de texte), ou la sauvegarde.
fn capture(world: &World, kind: SaveKind) -> Result<SaveFile, &'static str> {
    if world
        .get_resource::<Raid>()
        .is_some_and(|r| r.outcome.is_none())
        || world.contains_resource::<Death>()
    {
        return Err("save.refused.combat");
    }
    if world.contains_resource::<Sequence>() {
        return Err("save.refused.transition");
    }
    let refused = "save.refused.place";
    let location = world.get_resource::<Location>().ok_or(refused)?;
    let view = world.get_resource::<State<ViewLevel>>().map(|s| *s.get());
    let place = match view {
        Some(ViewLevel::Global) => {
            let root = world.get_resource::<ActiveSector>().ok_or(refused)?.root;
            let pos = world.get::<SpacePosition>(root).ok_or(refused)?.0;
            let rot = world.get::<SpaceRotation>(root).ok_or(refused)?.0;
            SavedPlace::Space {
                pos,
                rot: rot.to_array(),
            }
        }
        Some(ViewLevel::Sector) => SavedPlace::Docked {
            district: world
                .get_resource::<Docked>()
                .ok_or(refused)?
                .district
                .clone(),
        },
        _ => return Err(refused),
    };
    let living = world.get_resource::<LivingWorld>().ok_or(refused)?;
    let career = world.get_resource::<Career>().ok_or(refused)?;
    let clock = world.get_resource::<GameClock>().ok_or(refused)?;
    Ok(SaveFile {
        version: FORMAT_VERSION,
        content: CONTENT_VERSION,
        meta: SaveMeta {
            kind,
            date: clock.date,
            system: location.system.clone(),
            docked: matches!(place, SavedPlace::Docked { .. }),
            saved_at: now(),
        },
        mode: world
            .get_resource::<GameMode>()
            .copied()
            .unwrap_or_default(),
        world: living.sim.save(),
        read: living.read(),
        knowledge: world
            .get_resource::<PlayerKnowledge>()
            .cloned()
            .unwrap_or_default(),
        captain: career.captain.clone(),
        last_tick: career.last_tick(),
        missions: MissionLog {
            lines: world
                .get_resource::<MissionLog>()
                .map(|l| l.lines.clone())
                .unwrap_or_default(),
        },
        clock: clock.clone(),
        fuel: world
            .get_resource::<ShipFuel>()
            .map(|f| (f.units, f.capacity)),
        inventory: Inventory {
            items: world
                .get_resource::<Inventory>()
                .map(|i| i.items.clone())
                .unwrap_or_default(),
        },
        place,
        codex: world.get_resource::<Codex>().cloned().unwrap_or_default(),
        tips: world
            .get_resource::<TipsSeen>()
            .cloned()
            .unwrap_or_default(),
        tutorial: world
            .get_resource::<Tutorial>()
            .cloned()
            .unwrap_or_else(Tutorial::finished),
        items: SavedItems::capture(world),
    })
}

/// Sauvegarde maintenant et l'annonce. En Homme de fer, seules les sauvegardes automatiques
/// s'écrivent, toujours dans le même fichier.
pub fn save_now(world: &mut World, kind: SaveKind) {
    write_save(world, kind);
}

/// Comme [`save_now`], et rend le fichier écrit (l'autopilote recharge exactement celui-là :
/// une sauvegarde automatique de la même seconde ne s'y substitue pas).
pub fn write_save(world: &mut World, kind: SaveKind) -> Option<PathBuf> {
    let ironman = world.get_resource::<GameMode>().is_some_and(|m| m.ironman);
    let kind = match (ironman, kind) {
        (true, SaveKind::Auto) => SaveKind::Ironman,
        (true, _) => {
            notify(world, "save.refused.ironman", &[]);
            return None;
        }
        (false, kind) => kind,
    };
    let save = match capture(world, kind) {
        Ok(save) => save,
        Err(key) => {
            // Une sauvegarde automatique refusée (combat) se tait : elle reviendra.
            if kind != SaveKind::Auto && kind != SaveKind::Ironman {
                notify(world, key, &[]);
            }
            return None;
        }
    };
    let path = saves_dir().join(file_name(kind, save.meta.saved_at));
    match sh_data::write_ron(&path, &save) {
        Ok(()) => {
            info!("sauvegarde écrite : {}", path.display());
            world.write_message(GameSaved(kind));
            let key = match kind {
                SaveKind::Auto | SaveKind::Ironman => "save.done_auto",
                _ => "save.done",
            };
            notify(world, key, &[]);
            Some(path)
        }
        Err(err) => {
            error!("sauvegarde impossible : {err}");
            notify(world, "save.failed", &[("error", &err.to_string())]);
            None
        }
    }
}

/// La mort efface la sauvegarde Homme de fer.
pub fn erase_ironman(world: &mut World) {
    if world.get_resource::<GameMode>().is_some_and(|m| m.ironman) {
        let path = saves_dir().join(file_name(SaveKind::Ironman, 0));
        if let Err(err) = std::fs::remove_file(&path)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            error!("sauvegarde Homme de fer non effacée : {err}");
        }
    }
}

fn notify(world: &mut World, key: &str, args: &[(&str, &str)]) {
    let text = world.resource::<Texts>().format(key, args);
    world.write_message(Notice(text));
}

/// Sauvegarde automatique peu après l'arrivée dans une vue (à quai, en vol après un voyage
/// ou un départ) : le temps que la scène soit en place.
fn autosave(
    mut commands: Commands,
    time: Res<Time<Real>>,
    view: Option<Res<State<ViewLevel>>>,
    docked: Option<Res<Docked>>,
    mut armed: Local<(Option<(ViewLevel, bool)>, f32)>,
) {
    let current = view.map(|v| (*v.get(), docked.is_some()));
    let (seen, timer) = &mut *armed;
    if current != *seen {
        *seen = current;
        *timer = 1.5;
        return;
    }
    if *timer <= 0.0 {
        return;
    }
    *timer -= time.delta_secs();
    let saveable = matches!(
        current,
        Some((ViewLevel::Global, _)) | Some((ViewLevel::Sector, true))
    );
    if *timer <= 0.0 && saveable {
        commands.queue(|world: &mut World| save_now(world, SaveKind::Auto));
    }
}

fn on_quick_save(_: On<Start<QuickSave>>, mut commands: Commands) {
    commands.queue(|world: &mut World| save_now(world, SaveKind::Quick));
}

fn on_quick_load(_: On<Start<QuickLoad>>, mut commands: Commands) {
    commands.queue(|world: &mut World| {
        if world.get_resource::<GameMode>().is_some_and(|m| m.ironman) {
            notify(world, "save.refused.ironman", &[]);
            return;
        }
        let path = saves_dir().join(file_name(SaveKind::Quick, 0));
        if path.exists() {
            request_load(world, &path);
        } else {
            notify(world, "load.no_quicksave", &[]);
        }
    });
}

// --- Charger -----------------------------------------------------------------------------------

/// Demande le chargement d'un fichier : la partie en cours est quittée d'abord.
pub fn request_load(world: &mut World, path: &Path) {
    match read_save(path) {
        Ok(save) => {
            info!("chargement de {}", path.display());
            world.insert_resource(PendingLoad(Box::new(save)));
        }
        Err(err) => {
            error!("sauvegarde illisible {} : {err}", path.display());
            notify(world, "load.failed", &[("error", &err)]);
        }
    }
}

/// En jeu, quitte la partie ; au menu, installe la sauvegarde et entre en jeu.
fn apply_pending_load(world: &mut World) {
    let state = *world.resource::<State<AppState>>().get();
    match state {
        AppState::InGame => {
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::MainMenu);
        }
        AppState::MainMenu => {
            let Some(PendingLoad(save)) = world.remove_resource::<PendingLoad>() else {
                return;
            };
            install(world, *save);
        }
        _ => {}
    }
}

fn install(world: &mut World, save: SaveFile) {
    let data = world.resource::<Data>().clone();
    let system = save.meta.system.clone();
    world.insert_resource(save.mode);
    world.insert_resource(LivingWorld::restored(
        SimWorld::restore(&data, save.world),
        save.read,
    ));
    world.insert_resource(save.knowledge);
    world.insert_resource(Career::restored(save.captain, save.last_tick));
    world.insert_resource(save.missions);
    world.insert_resource(save.clock);
    world.insert_resource(save.inventory);
    world.insert_resource(save.codex);
    world.insert_resource(save.tips);
    world.insert_resource(save.tutorial);
    if save.content < CONTENT_VERSION {
        info!(
            "sauvegarde du contenu {} : migrée vers le contenu {CONTENT_VERSION}",
            save.content
        );
    }
    migrate_items(save.items, save.content).install(world);
    world.insert_resource(Location {
        system: system.clone(),
    });
    if let Some((units, capacity)) = save.fuel {
        world.insert_resource(ShipFuel { units, capacity });
    }
    let request = match migrate_place(save.place, save.content) {
        SavedPlace::Space { pos, rot } => {
            world.insert_resource(ResumeFlight {
                system: system.clone(),
                pos,
                rot: Quat::from_array(rot),
            });
            // En vol, on reprend aux commandes (une nouvelle partie, elle, commence debout).
            if let Some(aboard) = crate::voyage::helm_aboard(&data) {
                world.insert_resource(PendingAboard(aboard));
            }
            EnterGame {
                view: ViewLevel::Global,
                place: Some(system.as_str().to_owned()),
            }
        }
        SavedPlace::Docked { district } => EnterGame {
            view: ViewLevel::Sector,
            place: Some(docking_district(&district, |id| {
                data.sector(id)
                    .is_some_and(|s| s.external_port(crate::sector::DISTRICT_PORT).is_some())
            })),
        },
    };
    world.resource_scope(|world, mut app_state: Mut<NextState<AppState>>| {
        world.resource_scope(|world, mut view: Mut<NextState<ViewLevel>>| {
            let mut place = world.resource_mut::<CurrentPlace>();
            request.apply(&mut app_state, &mut view, &mut place);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_objets_se_sauvegardent_et_les_anciennes_sauvegardes_se_relisent() {
        let mut containers = Containers::default();
        containers.add("corvette_chambre_casier_1", "ration", 2);
        let mut picked_up = PickedUp::default();
        picked_up.insert("corvette/chambre/chambre_pickup_datapad");
        let mut equipment = Equipment::default();
        equipment
            .0
            .insert(sh_data::EquipSlot::Weapon, "mag_pistol".to_owned());
        let items = SavedItems {
            containers: Some(containers),
            picked_up,
            equipment: Some(equipment),
        };
        // Écrit comme une sauvegarde, relu comme une sauvegarde.
        let path = std::env::temp_dir().join(format!("sh-objets-{}.ron", std::process::id()));
        sh_data::write_ron(&path, &items).expect("écriture");
        let text = std::fs::read_to_string(&path).expect("lecture");
        let _ = std::fs::remove_file(&path);
        let back: SavedItems = sh_data::ron_options().from_str(&text).expect("relecture");
        assert_eq!(back, items);
        // Une sauvegarde d'avant M9 n'a pas ces champs : rien de pris, le reste du catalogue.
        let old: SavedItems = sh_data::ron_options()
            .from_str("()")
            .expect("ancien format");
        assert_eq!(old, SavedItems::default());
        assert!(old.containers.is_none() && old.equipment.is_none());
    }

    /// Une sauvegarde d'avant M9 (contenu 0) : le vaisseau `player_ship`, le spatioport
    /// `hexis_spaceport`, la tablette prise dans les quartiers.
    #[test]
    fn une_sauvegarde_d_avant_les_interieurs_types_se_migre() {
        let old: SavedPlace = sh_data::ron_options()
            .from_str(r#"Docked(district: "hexis_spaceport")"#)
            .expect("lieu d'une ancienne sauvegarde");
        assert!(matches!(
            migrate_place(old, 0),
            SavedPlace::Docked { district } if district == "spaceport"
        ));
        // Au contenu courant, rien ne bouge, même un nom qui ressemble à un ancien.
        let current = SavedPlace::Docked {
            district: "hexis_spaceport".into(),
        };
        assert!(matches!(
            migrate_place(current, CONTENT_VERSION),
            SavedPlace::Docked { district } if district == "hexis_spaceport"
        ));
        let mut picked_up = PickedUp::default();
        picked_up.insert("player_ship/quartiers/quartiers_pickup_datapad");
        picked_up.insert("player_ship/soute/soute_pickup_parts");
        picked_up.insert("hexis_spaceport/hall/hall_pickup_chip");
        picked_up.insert("salle_temoin/salle/objet");
        let items = migrate_items(
            SavedItems {
                picked_up,
                ..SavedItems::default()
            },
            0,
        );
        let ids: Vec<&str> = items.picked_up.0.iter().map(String::as_str).collect();
        assert_eq!(
            ids,
            vec![
                "corvette/chambre/chambre_pickup_datapad",
                "corvette/soute/soute_pickup_parts",
                "salle_temoin/salle/objet",
                "spaceport/hall/hall_pickup_chip",
            ]
        );
        // Une sauvegarde sans version de contenu est d'avant M9.
        #[derive(Deserialize)]
        struct Content {
            #[serde(default)]
            content: u32,
        }
        let content: Content = sh_data::ron_options()
            .from_str("(version: 1)")
            .expect("en-tête");
        assert_eq!(content.content, 0);
    }

    #[test]
    fn un_quartier_disparu_retombe_sur_le_spatioport_par_defaut() {
        let known = |id: &str| id == "spaceport" || id == "quai_propre";
        assert_eq!(docking_district("quai_propre", known), "quai_propre");
        assert_eq!(docking_district("hexis_spaceport", known), "spaceport");
    }

    #[test]
    fn noms_des_fichiers() {
        assert_eq!(file_name(SaveKind::Quick, 5), "rapide.ron");
        assert_eq!(file_name(SaveKind::Manual, 42), "partie-42.ron");
        assert_eq!(file_name(SaveKind::Ironman, 42), "homme-de-fer.ron");
    }
}
