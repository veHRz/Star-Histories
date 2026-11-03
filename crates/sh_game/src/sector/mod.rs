//! Secteurs jouables : vues Sectorielle et Locale (AGENTS.md §6.2).
//!
//! Un secteur est un graphe de cellules reliées par des portails. Autour du joueur, les cellules
//! sont en qualité **Locale** (modèle complet, collisions, lumières, interactions) ; ailleurs en
//! qualité **Sectorielle** (coque simplifiée, rien d'autre). Le passage de l'une à l'autre est un
//! streaming invisible ([`streaming`]), les portes servent de portes de chargement ([`doors`]).

use std::sync::Arc;

use avian3d::prelude::*;
use bevy::prelude::*;
use sh_data::Sector;

use crate::data::Data;
use crate::state::{CurrentPlace, ViewLevel};

pub mod doors;
pub mod gltf;
pub mod streaming;

pub struct SectorPlugin;

impl Plugin for SectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            streaming::StreamingPlugin,
            doors::DoorsPlugin,
            gltf::GltfSetupPlugin,
        ))
        .add_systems(OnEnter(ViewLevel::Sector), enter_sector)
        .add_systems(OnExit(ViewLevel::Sector), |mut commands: Commands| {
            commands.remove_resource::<ActiveSector>();
        });
    }
}

/// Couches de collision du jeu.
#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum Layer {
    /// Géométrie fixe des cellules.
    #[default]
    Static,
    /// Panneaux de porte (cinématiques).
    Door,
    /// Zones d'interaction (capteurs, ne bloquent rien).
    Interact,
}

/// Le secteur chargé et son entité racine (toutes les cellules en sont les enfants : quand le
/// vaisseau deviendra mobile en M2, il suffira de déplacer cette racine).
#[derive(Resource, Clone)]
pub struct ActiveSector {
    pub sector: Arc<Sector>,
    pub root: Entity,
}

/// Racine d'un secteur chargé.
#[derive(Component)]
pub struct SectorRoot;

/// Le lieu demandé est-il un secteur des données ?
pub fn is_sector(data: &Data, place: &CurrentPlace) -> bool {
    place
        .0
        .as_deref()
        .is_some_and(|id| data.sector(id).is_some())
}

fn enter_sector(mut commands: Commands, data: Res<Data>, place: Res<CurrentPlace>) {
    let Some(sector) = place.0.as_deref().and_then(|id| data.sector(id)) else {
        return;
    };
    let root = commands
        .spawn((
            Name::new(format!("Secteur {}", sector.id)),
            SectorRoot,
            DespawnOnExit(ViewLevel::Sector),
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    info!(
        "secteur {} : {} cellules, {} portails",
        sector.id,
        sector.cells.len(),
        sector.portals.len()
    );
    commands.insert_resource(ActiveSector {
        sector: Arc::new(sector.clone()),
        root,
    });
}

/// Conversion d'un triplet des données en vecteur.
pub fn v3(t: (f32, f32, f32)) -> Vec3 {
    Vec3::new(t.0, t.1, t.2)
}
