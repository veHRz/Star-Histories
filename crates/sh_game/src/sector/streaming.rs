//! Streaming des cellules : qualité Locale autour du joueur, Sectorielle ailleurs.
//!
//! Règles (AGENTS.md §6.2) :
//! - cellule du joueur + voisines (profondeur [`LOCAL_DEPTH`]) : qualité Locale ;
//! - les autres : qualité Sectorielle (coque simplifiée) ;
//! - chargement **anticipé** des modèles jusqu'à [`PRELOAD_DEPTH`] portails ;
//! - **hystérésis** : on ne change de cellule courante qu'une fois franchement entré dans une
//!   autre, et une cellule qui n'est plus voulue en Locale le reste [`UNLOAD_GRACE`] secondes ;
//! - **budget** : au plus [`SPAWNS_PER_FRAME`] instanciation de cellule complète par frame.

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

use crate::graphics::GraphicsSettings;
use crate::player::Player;
use crate::sector::{ActiveSector, placement_transform, v3};
use crate::state::ViewLevel;

pub struct StreamingPlugin;

impl Plugin for StreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CellStates>()
            .add_systems(OnExit(ViewLevel::Sector), reset_states)
            .add_systems(OnExit(ViewLevel::Global), reset_states)
            .add_systems(
                Update,
                (track_current_cell, stream_cells)
                    .chain()
                    .run_if(resource_exists::<ActiveSector>),
            );
    }
}

fn reset_states(mut states: ResMut<CellStates>) {
    *states = CellStates::default();
}

pub const LOCAL_DEPTH: usize = 1;
pub const PRELOAD_DEPTH: usize = 2;
pub const UNLOAD_GRACE: f32 = 2.0;
pub const SPAWNS_PER_FRAME: usize = 1;
/// Marge de sortie de la cellule courante (hystérésis), en mètres.
const EXIT_MARGIN: f32 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Local,
    Sector,
}

/// Une instance de cellule dans le monde.
#[derive(Component, Clone, Debug)]
pub struct CellInstance {
    pub cell: String,
    pub quality: Quality,
}

/// Posé par le post-traitement glTF quand l'instance est complète et prête.
#[derive(Component)]
pub struct CellReady;

#[derive(Default)]
pub struct CellSlot {
    pub full_handle: Option<Handle<WorldAsset>>,
    pub lod_handle: Option<Handle<WorldAsset>>,
    /// Lightmap de la cellule, chargée avec ses modèles (aucune cellule n'apparaît sans).
    pub lightmap_handle: Option<Handle<Image>>,
    pub full: Option<Entity>,
    pub lod: Option<Entity>,
    /// Secondes écoulées depuis que la cellule n'est plus voulue en Locale.
    pub unwanted_for: f32,
}

/// État du streaming : cellule courante et emplacements par cellule.
#[derive(Resource, Default)]
pub struct CellStates {
    pub current: Option<String>,
    pub slots: BTreeMap<String, CellSlot>,
    /// Transitions de cellule courante depuis l'entrée dans le secteur (mesures).
    pub transitions: u32,
}

impl CellStates {
    /// La cellule est-elle prête en qualité Locale (collisions comprises) ?
    pub fn is_local_ready(&self, cell: &str, ready: &Query<(), With<CellReady>>) -> bool {
        self.slots
            .get(cell)
            .and_then(|s| s.full)
            .is_some_and(|e| ready.contains(e))
    }
}

fn track_current_cell(
    sector: Res<ActiveSector>,
    player: Query<&GlobalTransform, With<Player>>,
    mut states: ResMut<CellStates>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    let p = player.translation();
    let p = (p.x, p.y + 0.5, p.z);
    let keep = states
        .current
        .as_deref()
        .and_then(|id| sector.sector.cell(id))
        .is_some_and(|c| c.contains_with_margin(p, EXIT_MARGIN));
    if keep {
        return;
    }
    if let Some(cell) = sector.sector.cells.iter().find(|c| c.contains(p))
        && states.current.as_deref() != Some(&cell.id)
    {
        debug!("cellule courante : {}", cell.id);
        states.current = Some(cell.id.clone());
        states.transitions += 1;
    }
}

fn stream_cells(
    mut commands: Commands,
    time: Res<Time>,
    sector: Res<ActiveSector>,
    asset_server: Res<AssetServer>,
    mut states: ResMut<CellStates>,
    ready: Query<(), With<CellReady>>,
    settings: Option<Res<GraphicsSettings>>,
) {
    let baked = settings.is_some_and(|s| s.preset.baked_lighting);
    let Some(current) = states.current.clone() else {
        return;
    };
    let s = &sector.sector;
    let local = s.within(&current, LOCAL_DEPTH);
    let preload = s.within(&current, PRELOAD_DEPTH);
    let dt = time.delta_secs();
    let mut budget = SPAWNS_PER_FRAME;

    for cell in &s.cells {
        let slot = states.slots.entry(cell.id.clone()).or_default();
        let wanted_local = local.contains(&cell.id);

        // Anticipation : les modèles des cellules proches se chargent avant d'être nécessaires.
        if preload.contains(&cell.id) && slot.full_handle.is_none() {
            slot.full_handle =
                Some(asset_server.load(GltfAssetLabel::Scene(0).from_asset(cell.model.clone())));
        }
        if slot.lod_handle.is_none()
            && let Some(lod) = &cell.lod
        {
            slot.lod_handle =
                Some(asset_server.load(GltfAssetLabel::Scene(0).from_asset(lod.clone())));
        }
        if baked
            && slot.lightmap_handle.is_none()
            && let Some(lightmap) = &cell.lightmap
        {
            slot.lightmap_handle = Some(asset_server.load(lightmap.image.clone()));
        }
        let lightmap_loaded = slot
            .lightmap_handle
            .as_ref()
            .is_none_or(|h| asset_server.is_loaded(h));

        if wanted_local {
            slot.unwanted_for = 0.0;
            if slot.full.is_none()
                && budget > 0
                && lightmap_loaded
                && let Some(handle) = &slot.full_handle
                && asset_server.is_loaded_with_dependencies(handle)
            {
                budget -= 1;
                let entity = commands
                    .spawn((
                        Name::new(format!("Cellule {} (Locale)", cell.id)),
                        CellInstance {
                            cell: cell.id.clone(),
                            quality: Quality::Local,
                        },
                        WorldAssetRoot(handle.clone()),
                        placement_transform(cell.placement),
                        ChildOf(sector.root),
                    ))
                    .id();
                slot.full = Some(entity);
            }
        } else if slot.full.is_some() {
            slot.unwanted_for += dt;
            if slot.unwanted_for >= UNLOAD_GRACE
                && let Some(entity) = slot.full.take()
            {
                commands.entity(entity).despawn();
            }
        }

        // La coque simplifiée couvre la cellule tant que la version complète n'est pas prête.
        let full_ready = slot.full.is_some_and(|e| ready.contains(e));
        if full_ready {
            if let Some(entity) = slot.lod.take() {
                commands.entity(entity).despawn();
            }
        } else if slot.lod.is_none()
            && lightmap_loaded
            && let Some(handle) = &slot.lod_handle
            && asset_server.is_loaded_with_dependencies(handle)
        {
            let entity = commands
                .spawn((
                    Name::new(format!("Cellule {} (Sectorielle)", cell.id)),
                    CellInstance {
                        cell: cell.id.clone(),
                        quality: Quality::Sector,
                    },
                    WorldAssetRoot(handle.clone()),
                    placement_transform(cell.placement),
                    ChildOf(sector.root),
                ))
                .id();
            slot.lod = Some(entity);
        }
    }
}

/// Cellule qui contient un point (repère du secteur), sans hystérésis.
pub fn cell_at(sector: &sh_data::Sector, p: Vec3) -> Option<&str> {
    sector
        .cells
        .iter()
        .find(|c| c.contains((p.x, p.y, p.z)))
        .map(|c| c.id.as_str())
}

/// Centre d'une cellule.
pub fn cell_center(sector: &sh_data::Sector, id: &str) -> Option<Vec3> {
    sector.cell(id).map(|c| v3(c.center()))
}
