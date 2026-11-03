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
        .add_systems(OnEnter(ViewLevel::Sector), enter_sector.in_set(EnterSector))
        .add_systems(OnEnter(ViewLevel::Global), enter_sector.in_set(EnterSector))
        .add_systems(OnExit(ViewLevel::Sector), leave_sector)
        .add_systems(OnExit(ViewLevel::Global), leave_sector);
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

/// Le secteur chargé et son entité racine (toutes les cellules en sont les enfants). En vue
/// Globale, c'est le vaisseau du joueur, à bord duquel il se trouve : sa racine est le
/// référentiel de l'origine flottante ([`crate::space`]).
#[derive(Resource, Clone)]
pub struct ActiveSector {
    pub sector: Arc<Sector>,
    pub root: Entity,
    /// La vue qui l'a chargé : tout ce qui en dépend disparaît avec elle.
    pub view: ViewLevel,
}

impl ActiveSector {
    pub fn scope(&self) -> DespawnOnExit<ViewLevel> {
        DespawnOnExit(self.view)
    }
}

/// Chargement du secteur à l'entrée d'une vue (les vues qui en dépendent s'ordonnent après).
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EnterSector;

/// À bord en vue Globale : où apparaît le joueur, et sur quel siège il commence.
#[derive(Resource, Clone, Debug)]
pub struct Aboard {
    pub spawn: String,
    pub seat_role: Option<String>,
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

/// Charge le secteur de la vue : le lieu lui-même en vue Sectorielle, le vaisseau du joueur
/// (départ de partie) en vue Globale.
fn enter_sector(
    mut commands: Commands,
    data: Res<Data>,
    place: Res<CurrentPlace>,
    view: Res<State<ViewLevel>>,
    ambient: Option<ResMut<GlobalAmbientLight>>,
) {
    let view = *view.get();
    let sector = match view {
        ViewLevel::Global => {
            let Some(start) = data.start() else {
                return;
            };
            commands.insert_resource(Aboard {
                spawn: start.aboard_spawn.clone(),
                seat_role: start.seat_role.clone(),
            });
            data.sector(&start.ship)
        }
        _ => place.0.as_deref().and_then(|id| data.sector(id)),
    };
    let Some(sector) = sector else {
        return;
    };
    // Les rebonds de lumière sont dans les lightmaps : l'ambiante ne les compte pas deux fois.
    if let Some(mut ambient) = ambient {
        ambient.affects_lightmapped_meshes = false;
    }
    let root = commands
        .spawn((
            Name::new(format!("Secteur {}", sector.id)),
            SectorRoot,
            DespawnOnExit(view),
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    info!(
        "secteur {} : {} cellules, {} portails ({view:?})",
        sector.id,
        sector.cells.len(),
        sector.portals.len()
    );
    commands.insert_resource(ActiveSector {
        sector: Arc::new(sector.clone()),
        root,
        view,
    });
}

fn leave_sector(mut commands: Commands) {
    commands.remove_resource::<ActiveSector>();
    commands.remove_resource::<Aboard>();
}

/// Conversion d'un triplet des données en vecteur.
pub fn v3(t: (f32, f32, f32)) -> Vec3 {
    Vec3::new(t.0, t.1, t.2)
}
