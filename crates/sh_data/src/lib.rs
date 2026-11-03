//! Données de Star Histories : schémas, chargement RON et validation.
//!
//! Sans Bevy. Le jeu (`sh_game`), la simulation (`sh_sim`) et les outils (`sh_import`) partagent
//! ces schémas. Arborescence lue sous `assets/data/` :
//!
//! - `canon/` : données transcrites du canon, chaque entrée cite sa section (`source: "§3.3"`) ;
//! - `propositions/` : données inventées (`status: Proposition("P12")`) ;
//! - `gameplay/` : réglages et contenu générique ;
//! - `lang/` : textes visibles du joueur.

use std::fs;
use std::path::{Path, PathBuf};

use ron::extensions::Extensions;
use serde::de::DeserializeOwned;
use thiserror::Error;

pub mod common;
pub mod currencies;
pub mod factions;
pub mod galaxy;
pub mod lang;
pub mod layout;
pub mod scene;
pub mod sector;
pub mod settings;
pub mod ships;
pub mod start;
pub mod validate;

pub use common::{Location, Place, Population, Status};
pub use currencies::{CurrenciesFile, Currency};
pub use factions::{Faction, FactionKind, FactionsFile};
pub use galaxy::{
    Body, BodyKind, Fleet, Galaxy, MapFrame, OrbitOrder, Phenomenon, PortalHub, StarSystem,
    Station, Zone,
};
pub use lang::Lang;
pub use layout::{Placed, PlacedKind, system_layout};
pub use scene::{Arrival, Structure, StructureKind, SystemScene, Traffic};
pub use sector::{Cell, Portal, PortalKind, Sector, SectorKind, Spawn};
pub use settings::{
    AntiAliasing, ClockSettings, FlightSettings, GraphicsPreset, GraphicsPresets, Handling,
    PresetLevel, SpaceSettings, SystemScale,
};
pub use ships::{Formation, FormationsFile, ShipClass, ShipClassesFile};
pub use start::Start;
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
    pub const LANG_EN: &str = "lang/en.ron";
    /// Dossiers scannés pour les descripteurs de secteurs (`*.ron`).
    pub const SECTOR_DIRS: [&str; 2] = ["canon/sectors", "propositions/sectors"];
    /// Dossiers scannés pour les mises en scène de systèmes (`*.ron`).
    pub const SCENE_DIRS: [&str; 2] = ["canon/systems", "propositions/systems"];
    /// Dossiers scannés pour les départs de partie (`*.ron`).
    pub const START_DIRS: [&str; 1] = ["propositions/starts"];
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
    pub lang: Lang,
    pub sectors: Vec<Sector>,
    pub scenes: Vec<SystemScene>,
    pub starts: Vec<Start>,
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
}

/// Options RON communes : `Some(x)` peut s'écrire `x` dans les fichiers écrits à la main.
pub fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME)
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
            lang: read_ron(&file(paths::LANG_EN))?,
            sectors: read_dir_ron(data_dir, &paths::SECTOR_DIRS)?,
            scenes: read_dir_ron(data_dir, &paths::SCENE_DIRS)?,
            starts: read_dir_ron(data_dir, &paths::START_DIRS)?,
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
