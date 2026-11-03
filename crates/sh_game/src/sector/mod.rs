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

/// À bord : où apparaît le joueur, et sur quel siège il commence (départ de partie, ou
/// continuité d'une transition de vue, voir [`PendingAboard`]).
#[derive(Resource, Clone, Debug)]
pub struct Aboard {
    pub spawn: String,
    pub seat_role: Option<String>,
}

/// Posé par une transition (amarrage, départ) pour la vue suivante : le joueur y reprend sa
/// place à bord.
#[derive(Resource, Clone, Debug)]
pub struct PendingAboard(pub Aboard);

/// Le vaisseau du joueur est amarré dans le secteur actif (secteur composé, `sh_data::dock`).
#[derive(Resource, Clone, Debug)]
pub struct Docked {
    /// Le vaisseau (secteur) et le quartier qui l'accueille.
    pub ship: String,
    pub district: String,
}

/// Point d'attache d'un vaisseau (son sas) et d'un quartier (sa passerelle).
pub const SHIP_PORT: &str = "@dock";
pub const DISTRICT_PORT: &str = "@ship";

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
    pending: Option<Res<PendingAboard>>,
    ambient: Option<ResMut<GlobalAmbientLight>>,
) {
    let view = *view.get();
    let pending = pending.map(|p| p.0.clone());
    commands.remove_resource::<PendingAboard>();
    let start = data.start();
    let sector = match view {
        ViewLevel::Global => {
            let Some(start) = start else {
                return;
            };
            commands.insert_resource(pending.unwrap_or_else(|| Aboard {
                spawn: start.aboard_spawn.clone(),
                seat_role: start.seat_role.clone(),
            }));
            data.sector(&start.ship).cloned()
        }
        _ => {
            let district = place.0.as_deref().and_then(|id| data.sector(id));
            // Un quartier qui accueille un vaisseau (point d'attache « @ship ») : le vaisseau du
            // joueur y est amarré, les deux secteurs n'en font plus qu'un.
            let ship = start.and_then(|s| data.sector(&s.ship));
            match (district, ship) {
                (Some(district), Some(ship)) if district.external_port(DISTRICT_PORT).is_some() => {
                    match sh_data::dock(ship, SHIP_PORT, district, DISTRICT_PORT) {
                        Ok(docked) => {
                            commands.insert_resource(Docked {
                                ship: ship.id.to_string(),
                                district: district.id.to_string(),
                            });
                            if let Some(aboard) = pending {
                                commands.insert_resource(aboard);
                            }
                            Some(docked)
                        }
                        Err(err) => {
                            error!("amarrage impossible : {err}");
                            Some(district.clone())
                        }
                    }
                }
                (district, _) => district.cloned(),
            }
        }
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
        sector: Arc::new(sector),
        root,
        view,
    });
}

fn leave_sector(mut commands: Commands) {
    commands.remove_resource::<ActiveSector>();
    commands.remove_resource::<Aboard>();
    commands.remove_resource::<Docked>();
}

/// Transformation d'une cellule placée (vaisseau amarré).
pub fn placement_transform(placement: Option<sh_data::Placement>) -> Transform {
    placement.map_or(Transform::IDENTITY, |p| {
        Transform::from_translation(v3(p.translation)).with_rotation(Quat::from_rotation_y(
            std::f32::consts::FRAC_PI_2 * f32::from(p.quarter_turns % 4),
        ))
    })
}

/// Conversion d'un triplet des données en vecteur.
pub fn v3(t: (f32, f32, f32)) -> Vec3 {
    Vec3::new(t.0, t.1, t.2)
}
