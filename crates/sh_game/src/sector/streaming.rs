//! Streaming des cellules : qualité Locale autour du joueur, Sectorielle ailleurs.
//!
//! Règles (AGENTS.md §6.2) :
//! - cellule du joueur + voisines (profondeur [`LOCAL_DEPTH`]) : qualité Locale ;
//! - les autres : qualité Sectorielle (coque simplifiée) ;
//! - chargement **anticipé** des modèles jusqu'à [`PRELOAD_DEPTH`] portails ;
//! - **hystérésis** : on ne change de cellule courante qu'une fois franchement entré dans une
//!   autre, et une cellule qui n'est plus voulue en Locale le reste [`UNLOAD_GRACE`] secondes ;
//! - **budget** : au plus [`SPAWNS_PER_FRAME`] instanciation de cellule complète par frame ;
//! - **cellules imbriquées** (le vaisseau amarré dans le hangar) : la cellule courante est la plus
//!   petite qui contient le joueur (`Sector::cell_at`) ; entrer dans une cellule emboîtée dans la
//!   courante se fait sans attendre la marge d'hystérésis ;
//! - **arrivée** : l'écran de chargement (voile du HUD, fondu d'une transition) reste tant que la
//!   cellule du joueur et ses voisines Locales ne sont pas toutes prêtes
//!   ([`CellStates::arrival_ready`]), au plus [`ARRIVAL_MAX_WAIT`] secondes pour les voisines :
//!   on n'arrive plus dans des pièces vides où seules les portes et les lampes sont là.

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
/// Attente maximale des cellules voisines à l'arrivée dans un secteur, en secondes : au-delà,
/// l'écran de chargement se lève dès que la cellule du joueur est prête (un modèle manquant ne
/// bloque pas la partie).
pub const ARRIVAL_MAX_WAIT: f32 = 6.0;

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
    /// Les cellules voulues en qualité Locale autour de la cellule courante (elle comprise).
    pub local: Vec<String>,
    /// Secondes écoulées depuis l'entrée dans le secteur.
    pub waited: f32,
}

impl CellStates {
    /// La cellule est-elle prête en qualité Locale (collisions comprises) ?
    pub fn is_local_ready(&self, cell: &str, ready: &Query<(), With<CellReady>>) -> bool {
        self.slots
            .get(cell)
            .and_then(|s| s.full)
            .is_some_and(|e| ready.contains(e))
    }

    /// L'arrivée est-elle prête à être montrée ? La cellule du joueur est prête en qualité
    /// Locale, et ses voisines Locales aussi (ou [`ARRIVAL_MAX_WAIT`] est écoulé). Lu par le voile
    /// de chargement du HUD et par la fin des transitions (`transition::Sequence::Arriving`).
    pub fn arrival_ready(&self, ready: &Query<(), With<CellReady>>) -> bool {
        let Some(current) = self.current.as_deref() else {
            return false;
        };
        self.is_local_ready(current, ready)
            && (self.waited >= ARRIVAL_MAX_WAIT
                || self.local.iter().all(|c| self.is_local_ready(c, ready)))
    }
}

/// Faut-il changer de cellule courante ? `best` : la plus petite cellule qui contient le point
/// (`Sector::cell_at`). On garde la courante tant qu'on est dans sa marge (hystérésis), sauf pour
/// entrer dans une cellule emboîtée en elle (le sas d'un vaisseau posé dans le hangar).
fn keeps_current(
    current: Option<&sh_data::sector::Cell>,
    best: Option<&sh_data::sector::Cell>,
    p: (f32, f32, f32),
) -> bool {
    let Some(current) = current else {
        return false;
    };
    match best {
        Some(best) if best.id == current.id => true,
        Some(best) if nested(best, current) => false,
        _ => current.contains_with_margin(p, EXIT_MARGIN),
    }
}

/// La boîte de `inner` est-elle dans celle de `outer` (et plus petite) ?
fn nested(inner: &sh_data::sector::Cell, outer: &sh_data::sector::Cell) -> bool {
    let inside = |lo: (f32, f32, f32), hi: (f32, f32, f32)| {
        lo.0 >= outer.min.0
            && lo.1 >= outer.min.1
            && lo.2 >= outer.min.2
            && hi.0 <= outer.max.0
            && hi.1 <= outer.max.1
            && hi.2 <= outer.max.2
    };
    inside(inner.min, inner.max) && inner.volume() < outer.volume()
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
    let s = &sector.sector;
    let best = s.cell_at(p);
    let current = states.current.as_deref().and_then(|id| s.cell(id));
    if keeps_current(current, best, p) {
        return;
    }
    if let Some(cell) = best
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
    let dt = time.delta_secs();
    states.waited += dt;
    let Some(current) = states.current.clone() else {
        return;
    };
    let s = &sector.sector;
    let local = s.within(&current, LOCAL_DEPTH);
    let preload = s.within(&current, PRELOAD_DEPTH);
    if states.local != local {
        states.local.clone_from(&local);
    }
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

/// Cellule qui contient un point (repère du secteur), sans hystérésis : la plus petite, quand
/// des cellules s'emboîtent (`Sector::cell_at`).
pub fn cell_at(sector: &sh_data::Sector, p: Vec3) -> Option<&str> {
    sector.cell_at((p.x, p.y, p.z)).map(|c| c.id.as_str())
}

/// Centre d'une cellule.
pub fn cell_center(sector: &sh_data::Sector, id: &str) -> Option<Vec3> {
    sector.cell(id).map(|c| v3(c.center()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(id: &str, min: (f32, f32, f32), max: (f32, f32, f32)) -> sh_data::sector::Cell {
        sh_data::sector::Cell {
            id: id.into(),
            name_key: None,
            model: String::new(),
            lod: None,
            min,
            max,
            lightmap: None,
            placement: None,
        }
    }

    #[test]
    fn hysteresis_et_cellules_emboitees() {
        let hangar = cell("hangar", (-10.0, 0.0, -10.0), (10.0, 8.0, 10.0));
        let sas = cell("sas", (0.0, 0.0, -1.0), (2.0, 2.5, 1.0));
        let couloir = cell("couloir", (2.0, 0.0, -1.0), (6.0, 2.5, 1.0));
        // Du hangar, entrer dans le sas qui y est posé : tout de suite.
        assert!(!keeps_current(Some(&hangar), Some(&sas), (0.1, 1.0, 0.0)));
        // Sortir du sas vers le hangar : seulement au-delà de la marge.
        assert!(keeps_current(Some(&sas), Some(&hangar), (-0.2, 1.0, 0.0)));
        assert!(!keeps_current(Some(&sas), Some(&hangar), (-0.5, 1.0, 0.0)));
        // Deux cellules voisines (non emboîtées) : la marge, dans les deux sens.
        assert!(keeps_current(Some(&sas), Some(&couloir), (2.2, 1.0, 0.0)));
        assert!(keeps_current(Some(&couloir), Some(&sas), (1.8, 1.0, 0.0)));
        assert!(!keeps_current(Some(&couloir), Some(&sas), (1.5, 1.0, 0.0)));
        // Rien de courant : on prend la meilleure.
        assert!(!keeps_current(None, Some(&sas), (1.0, 1.0, 0.0)));
        assert!(nested(&sas, &hangar) && !nested(&hangar, &sas) && !nested(&couloir, &sas));
    }
}
