//! Données de Star Histories : schémas, chargement RON et validation.
//!
//! Sans Bevy. Le jeu (`sh_game`), la simulation (`sh_sim`) et les outils (`sh_import`) partagent
//! ces schémas. Arborescence lue sous `assets/data/` :
//!
//! - `canon/` : données transcrites du canon, chaque entrée cite sa section (`source: "§3.3"`) ;
//! - `propositions/` : données inventées (`status: Proposition("P12")`) ;
//! - `gameplay/` : réglages et contenu générique ;
//! - `lang/` : textes visibles du joueur, par langue (fichier commun et fichiers de domaine,
//!   [`load_lang`]).

use std::fs;
use std::path::{Path, PathBuf};

use ron::extensions::Extensions;
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

pub mod career;
pub mod combat;
pub mod common;
pub mod currencies;
pub mod dialogue;
pub mod factions;
pub mod galaxy;
pub mod items;
pub mod lang;
pub mod layout;
pub mod map;
pub mod planets;
pub mod scene;
pub mod sector;
pub mod settings;
pub mod ships;
pub mod sim;
pub mod start;
pub mod travel;
pub mod validate;

pub use career::{CareerRules, Ranks};
pub use combat::{CombatRules, SectionId, WeaponKind};
pub use common::{Location, Place, Population, Status};
pub use currencies::{CurrenciesFile, Currency};
pub use dialogue::Dialogue;
pub use factions::{Faction, FactionKind, FactionsFile};
pub use galaxy::{
    Body, BodyKind, Fleet, Galaxy, MapFrame, OrbitOrder, Phenomenon, PortalHub, StarSystem,
    Station, Zone,
};
pub use items::{EquipSlot, ItemCategory, ItemDef, ItemsFile};
pub use lang::{DuplicateKey, Keyboard, Lang, load_lang};
pub use layout::{Placed, PlacedKind, system_layout};
pub use map::{MapEntry, MapKind, entry_at, public_map};
pub use planets::{PlanetLook, PlanetLooks};
pub use scene::{Arrival, DockPort, Structure, StructureKind, SystemScene, Traffic};
pub use sector::{
    Cell, DockError, Placement, Portal, PortalKind, Sector, SectorKind, Spawn, dock, is_external,
};
pub use settings::{
    AntiAliasing, ClockSettings, FlightSettings, GraphicsPreset, GraphicsPresets, Handling,
    PresetLevel, ShadowFilter, SpaceSettings, SystemScale,
};
pub use ships::{Formation, FormationsFile, ShipClass, ShipClassesFile};
pub use sim::{CalendarKind, Rails, SimSettings};
pub use start::Start;
pub use travel::{PortalLinks, ShipLogistics, TravelSettings};
pub use validate::{Issue, ValidationReport};

/// Chemins relatifs des fichiers, sous le dossier de données.
pub mod paths {
    pub const GALAXY: &str = "canon/galaxy.ron";
    pub const FACTIONS: &str = "canon/factions.ron";
    pub const SHIP_CLASSES: &str = "canon/ship_classes.ron";
    pub const FORMATIONS: &str = "canon/formations.ron";
    pub const CURRENCIES: &str = "canon/currencies.ron";
    pub const GRAPHICS: &str = "gameplay/graphics.ron";
    pub const CLOCK: &str = "gameplay/clock.ron";
    pub const SPACE: &str = "gameplay/space.ron";
    pub const FLIGHT: &str = "gameplay/flight.ron";
    pub const TRAVEL: &str = "gameplay/travel.ron";
    pub const SIM: &str = "gameplay/sim.ron";
    pub const CAREER: &str = "gameplay/career.ron";
    pub const COMBAT: &str = "gameplay/combat.ron";
    pub const RANKS: &str = "propositions/ranks.ron";
    pub const RAILS: &str = "canon/rails.ron";
    pub const PORTAL_LINKS: &str = "propositions/portal_links.ron";
    /// Catalogue des objets, contenu de départ des casiers, équipement de départ (M9).
    pub const ITEMS: &str = "gameplay/items.ron";
    /// Apparence peinte des astres en vue Globale (M9).
    pub const PLANETS: &str = "gameplay/planets.ron";
    /// Dossier des langues : `<code>.ron` (textes communs) et `<code>/*.ron` (un fichier par
    /// domaine), fusionnés par [`crate::load_lang`].
    pub const LANG_DIR: &str = "lang";
    /// La langue du jeu (Q11 : le français).
    pub const LANG_CODE: &str = "fr";
    /// Traduction anglaise, tenue avec les mêmes clés.
    pub const LANG_EN_CODE: &str = "en";
    /// Fichier commun de la langue du jeu (seulement une partie de ses textes : les autres sont
    /// dans les fichiers de domaine, lire la langue par [`crate::load_lang`]).
    pub const LANG: &str = "lang/fr.ron";
    /// Fichier commun de la traduction anglaise.
    pub const LANG_EN: &str = "lang/en.ron";
    pub const KEYBOARD: &str = "gameplay/keyboard.ron";
    /// Dossiers scannés pour les descripteurs de secteurs (`*.ron`).
    pub const SECTOR_DIRS: [&str; 2] = ["canon/sectors", "propositions/sectors"];
    /// Dossiers scannés pour les mises en scène de systèmes (`*.ron`).
    pub const SCENE_DIRS: [&str; 2] = ["canon/systems", "propositions/systems"];
    /// Dossiers scannés pour les départs de partie (`*.ron`).
    pub const START_DIRS: [&str; 1] = ["propositions/starts"];
    /// Dossiers scannés pour les dialogues (`*.ron`).
    pub const DIALOGUE_DIRS: [&str; 1] = ["propositions/dialogues"];
}

/// Toutes les données chargées au démarrage du jeu.
#[derive(Clone, Debug)]
pub struct GameData {
    pub galaxy: Galaxy,
    pub factions: FactionsFile,
    pub ship_classes: ShipClassesFile,
    pub formations: FormationsFile,
    pub currencies: CurrenciesFile,
    pub graphics: GraphicsPresets,
    pub clock: ClockSettings,
    pub space: SpaceSettings,
    pub flight: FlightSettings,
    pub travel: TravelSettings,
    pub sim: SimSettings,
    pub career: CareerRules,
    pub combat: CombatRules,
    pub ranks: Ranks,
    pub rails: Rails,
    pub portal_links: PortalLinks,
    pub items: ItemsFile,
    pub planets: PlanetLooks,
    pub lang: Lang,
    pub keyboard: Keyboard,
    pub sectors: Vec<Sector>,
    pub scenes: Vec<SystemScene>,
    pub starts: Vec<Start>,
    pub dialogues: Vec<Dialogue>,
}

#[derive(Debug, Error)]
pub enum DataError {
    #[error("lecture de {path} impossible : {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} : {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: Box<ron::error::SpannedError>,
    },
    #[error("écriture de {path} impossible : {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} : {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: ron::Error,
    },
}

/// Options RON communes : `Some(x)` peut s'écrire `x` dans les fichiers écrits à la main.
pub fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME)
}

/// Écrit une valeur en RON lisible (sauvegardes, réglages du joueur). L'écriture passe par un
/// fichier voisin renommé ensuite : une coupure en cours d'écriture ne laisse jamais un fichier
/// tronqué à la place de l'ancien.
pub fn write_ron<T: Serialize>(path: &Path, value: &T) -> Result<(), DataError> {
    let text =
        ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::new()).map_err(|source| {
            DataError::Serialize {
                path: path.to_owned(),
                source,
            }
        })?;
    let write_err = |source| DataError::Write {
        path: path.to_owned(),
        source,
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(write_err)?;
    }
    let partial = path.with_extension("ron.part");
    fs::write(&partial, text).map_err(write_err)?;
    fs::rename(&partial, path).map_err(write_err)
}

/// Lit et désérialise un fichier RON.
pub fn read_ron<T: DeserializeOwned>(path: &Path) -> Result<T, DataError> {
    let text = fs::read_to_string(path).map_err(|source| DataError::Io {
        path: path.to_owned(),
        source,
    })?;
    ron_options()
        .from_str(&text)
        .map_err(|source| DataError::Parse {
            path: path.to_owned(),
            source: Box::new(source),
        })
}

impl GameData {
    /// Charge toutes les données depuis `data_dir` (le dossier `assets/data`).
    /// La validation est une étape séparée : [`GameData::validate`].
    pub fn load(data_dir: &Path) -> Result<Self, DataError> {
        let file = |rel: &str| data_dir.join(rel);
        Ok(Self {
            galaxy: read_ron(&file(paths::GALAXY))?,
            factions: read_ron(&file(paths::FACTIONS))?,
            ship_classes: read_ron(&file(paths::SHIP_CLASSES))?,
            formations: read_ron(&file(paths::FORMATIONS))?,
            currencies: read_ron(&file(paths::CURRENCIES))?,
            graphics: read_ron(&file(paths::GRAPHICS))?,
            clock: read_ron(&file(paths::CLOCK))?,
            space: read_ron(&file(paths::SPACE))?,
            flight: read_ron(&file(paths::FLIGHT))?,
            travel: read_ron(&file(paths::TRAVEL))?,
            sim: read_ron(&file(paths::SIM))?,
            career: read_ron(&file(paths::CAREER))?,
            combat: read_ron(&file(paths::COMBAT))?,
            ranks: read_ron(&file(paths::RANKS))?,
            rails: read_ron(&file(paths::RAILS))?,
            portal_links: read_ron(&file(paths::PORTAL_LINKS))?,
            items: read_ron(&file(paths::ITEMS))?,
            planets: read_ron(&file(paths::PLANETS))?,
            lang: load_lang(data_dir, paths::LANG_CODE)?,
            keyboard: read_ron(&file(paths::KEYBOARD))?,
            sectors: read_dir_ron(data_dir, &paths::SECTOR_DIRS)?,
            scenes: read_dir_ron(data_dir, &paths::SCENE_DIRS)?,
            starts: read_dir_ron(data_dir, &paths::START_DIRS)?,
            dialogues: read_dir_ron(data_dir, &paths::DIALOGUE_DIRS)?,
        })
    }

    /// Vérifie la cohérence de l'ensemble (identifiants, références, cases, sources, secrets).
    pub fn validate(&self) -> Result<(), ValidationReport> {
        validate::validate(self)
    }

    /// Le départ de partie (la v1 n'en a qu'un : le premier, par ordre de fichier).
    pub fn start(&self) -> Option<&Start> {
        self.starts.first()
    }

    /// La mise en scène d'un système, s'il en a une.
    pub fn scene(&self, system: &sh_core::Id<Place>) -> Option<&SystemScene> {
        self.scenes.iter().find(|s| &s.system == system)
    }

    pub fn faction(&self, id: &sh_core::Id<Faction>) -> Option<&Faction> {
        self.factions.factions.iter().find(|f| &f.id == id)
    }

    /// Un objet du catalogue.
    pub fn item(&self, id: &str) -> Option<&ItemDef> {
        self.items.get(id)
    }
}

/// Lit tous les `*.ron` des dossiers donnés (absents : ignorés), triés par chemin.
fn read_dir_ron<T: DeserializeOwned>(data_dir: &Path, dirs: &[&str]) -> Result<Vec<T>, DataError> {
    let mut files = Vec::new();
    for dir in dirs {
        let dir = data_dir.join(dir);
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "ron") {
                files.push(path);
            }
        }
    }
    files.sort();
    files.iter().map(|path| read_ron(path)).collect()
}

impl GameData {
    pub fn sector(&self, id: &str) -> Option<&Sector> {
        self.sectors.iter().find(|s| s.id.as_str() == id)
    }
}
