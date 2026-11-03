//! Pont entre les données (`sh_data`) et Bevy : ressources, chemins, textes.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::asset::io::file::FileAssetReader;
use bevy::prelude::*;
use sh_data::{GameData, Lang};

/// Dossier `assets/data`, résolu comme Bevy résout son dossier d'assets (variable
/// `BEVY_ASSET_ROOT`, sinon dossier du manifeste en développement, sinon dossier de l'exécutable).
pub fn data_dir() -> PathBuf {
    assets_dir().join("data")
}

pub fn assets_dir() -> PathBuf {
    FileAssetReader::get_base_path().join("assets")
}

/// Toutes les données du jeu, chargées et validées. Partagées en lecture seule.
#[derive(Resource, Clone, Deref)]
pub struct Data(pub Arc<GameData>);

/// Les textes de l'interface, disponibles dès le démarrage (avant les autres données, pour
/// l'écran de chargement lui-même).
#[derive(Resource, Clone, Default, Deref)]
pub struct Texts(pub Lang);

impl Texts {
    /// Charge la langue d'interface. Un échec n'empêche pas de démarrer : les clés s'affichent
    /// entre crochets, et la validation des données signale le problème au chargement.
    pub fn load() -> Self {
        let path = data_dir().join(sh_data::paths::LANG_EN);
        match sh_data::read_ron::<Lang>(&path) {
            Ok(lang) => Texts(lang),
            Err(err) => {
                error!("textes d'interface illisibles : {err}");
                Texts::default()
            }
        }
    }
}
