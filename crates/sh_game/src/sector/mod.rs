//! Secteurs jouables : vues Sectorielle et Locale (AGENTS.md §6.2).
//!
//! Un secteur est un graphe de cellules reliées par des portails. Autour du joueur, les cellules
//! sont en qualité **Locale** (modèle complet, collisions, lumières, interactions) ; ailleurs en
//! qualité **Sectorielle** (coque simplifiée, rien d'autre). Le passage de l'une à l'autre est un
//! streaming invisible ([`streaming`]), les portes servent de portes de chargement ([`doors`]).
//!
//! **Le vaisseau du joueur** (M9) est l'intérieur type de sa classe (`Start::interior`,
//! `corvette`) : en vue Globale, c'est tout le secteur chargé ; à quai, ses cellules sont placées
//! dans le quartier qui l'accueille (`sh_data::dock`), et sa coque extérieure (`Start::hull`) est
//! posée avec lui dans le berceau du hangar ([`ShipHull`]). Où que l'on soit,
//! [`ActiveSector::aboard_cell`] dit si le joueur est dans son propre vaisseau et dans quelle
//! cellule, sans qu'aucun appelant ne cite d'identifiant de secteur ; le bandeau l'appelle
//! « Votre vaisseau » ([`PLAYER_SHIP_NAME_KEY`]).
//!
//! **Spatioport par défaut** (P7) : un port qui ne déclare pas son secteur accueille le
//! vaisseau dans `spaceport` ; le port d'amarrage (et son système) est retrouvé d'après le
//! système où l'on est ([`Docked::port`]), et le lieu porte le nom de ce port (« Spatioport
//! d'Hexis »).

use std::sync::Arc;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use sh_data::Sector;

use crate::data::{Data, assets_dir};
use crate::sector::streaming::CellStates;
use crate::state::{CurrentPlace, ViewLevel};
use crate::voyage::Location;

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
    /// Les cellules du vaisseau du joueur dans ce secteur : toutes quand le secteur est son
    /// intérieur (en vol, ou chargé seul), celles du vaisseau amarré à quai, aucune ailleurs.
    pub ship_cells: Vec<String>,
}

impl ActiveSector {
    pub fn scope(&self) -> DespawnOnExit<ViewLevel> {
        DespawnOnExit(self.view)
    }

    /// Cette cellule est-elle une salle du vaisseau du joueur ?
    pub fn is_ship_cell(&self, cell: &str) -> bool {
        self.ship_cells.iter().any(|c| c == cell)
    }

    /// Le joueur est-il dans son propre vaisseau, et dans quelle cellule ? `Some(cellule)` à bord
    /// (en vol comme à quai), `None` ailleurs (sorti par le sas, dans un autre secteur) ou tant
    /// que sa cellule n'est pas connue. Les identifiants de cellule sont ceux de l'intérieur type
    /// (`chambre`, `cockpit`, `sas`…), les mêmes à quai et en vol.
    pub fn aboard_cell<'a>(&self, states: &'a CellStates) -> Option<&'a str> {
        states
            .current
            .as_deref()
            .filter(|cell| self.is_ship_cell(cell))
    }
}

/// Le joueur est-il dans son propre vaisseau, et dans quelle cellule ? Comme
/// [`ActiveSector::aboard_cell`], sans secteur chargé : `None` (carte, menu).
pub fn aboard_cell<'a>(active: Option<&ActiveSector>, states: &'a CellStates) -> Option<&'a str> {
    active.and_then(|a| a.aboard_cell(states))
}

/// Clé de langue du vaisseau du joueur dans le bandeau du lieu, quel que soit son intérieur type.
pub const PLAYER_SHIP_NAME_KEY: &str = "sector.player_ship";

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
    /// Le vaisseau (son intérieur type) et le quartier qui l'accueille (secteurs).
    pub ship: String,
    pub district: String,
    /// Le port d'amarrage (`DockPort::id`) et son système : un spatioport par défaut est partagé
    /// par plusieurs lieux, c'est le port qui dit lequel. Aucun : départ direct dans un secteur
    /// qu'aucun port ne désigne.
    pub port: Option<String>,
    pub system: Option<String>,
}

impl Docked {
    /// Les lieux où l'on est, du plus précis au plus large (le port, puis son système) : ceux
    /// auxquels une conversation peut être rattachée (`Dialogue::for_role_at`).
    pub fn places(&self) -> Vec<&str> {
        self.port
            .iter()
            .chain(&self.system)
            .map(String::as_str)
            .collect()
    }
}

/// La coque extérieure du vaisseau amarré, posée avec lui dans le berceau du hangar.
#[derive(Component)]
pub struct ShipHull;

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

/// Charge le secteur de la vue : le lieu lui-même en vue Sectorielle (un quartier qui accueille
/// un vaisseau : le vaisseau du joueur y est amarré, sa coque posée), l'intérieur du vaisseau du
/// joueur en vue Globale.
#[allow(clippy::too_many_arguments)]
fn enter_sector(
    mut commands: Commands,
    data: Res<Data>,
    place: Res<CurrentPlace>,
    view: Res<State<ViewLevel>>,
    pending: Option<Res<PendingAboard>>,
    location: Option<Res<Location>>,
    asset_server: Res<AssetServer>,
    ambient: Option<ResMut<GlobalAmbientLight>>,
) {
    let view = *view.get();
    let pending = pending.map(|p| p.0.clone());
    commands.remove_resource::<PendingAboard>();
    let start = data.start();
    let interior = start.map(|s| s.interior.as_str());
    // Le secteur à charger, les cellules du vaisseau du joueur, et son placement s'il est amarré.
    let mut hull_at = None;
    let (sector, ship_cells) = match view {
        ViewLevel::Global => {
            let Some(start) = start else {
                return;
            };
            // Nouvelle partie : là où elle commence ; sinon la place que la transition a gardée.
            commands.insert_resource(pending.unwrap_or_else(|| Aboard {
                spawn: start.first_spawn().to_owned(),
                seat_role: start.seat_role.clone(),
            }));
            let Some(ship) = data.sector(&start.interior) else {
                return;
            };
            let mut ship = ship.clone();
            ship.name_key = PLAYER_SHIP_NAME_KEY.to_owned();
            let cells = ship.cells.iter().map(|c| c.id.clone()).collect();
            (ship, cells)
        }
        _ => {
            let Some(district) = place.0.as_deref().and_then(|id| data.sector(id)) else {
                return;
            };
            let ship = interior.and_then(|id| data.sector(id));
            match ship {
                // L'intérieur du vaisseau chargé seul (autopilote) : c'est le vaisseau du joueur.
                Some(ship) if ship.id == district.id => {
                    if let Some(aboard) = pending {
                        commands.insert_resource(aboard);
                    }
                    let mut ship = ship.clone();
                    ship.name_key = PLAYER_SHIP_NAME_KEY.to_owned();
                    let cells = ship.cells.iter().map(|c| c.id.clone()).collect();
                    (ship, cells)
                }
                // Un quartier qui accueille un vaisseau (point d'attache « @ship ») : le vaisseau
                // du joueur y est amarré, les deux secteurs n'en font plus qu'un.
                Some(ship) if district.external_port(DISTRICT_PORT).is_some() => {
                    match sh_data::dock(ship, SHIP_PORT, district, DISTRICT_PORT) {
                        Ok(mut docked) => {
                            let system = location.as_ref().map(|l| l.system.as_str());
                            let found = sh_data::scene::find_dock(
                                &data.scenes,
                                district.id.as_str(),
                                system,
                            );
                            // Le lieu porte le nom de son port (« Spatioport d'Hexis »).
                            if let Some(key) = found.and_then(|(_, port)| port.place_key.clone()) {
                                docked.name_key = key;
                            }
                            commands.insert_resource(Docked {
                                ship: ship.id.to_string(),
                                district: district.id.to_string(),
                                port: found.map(|(_, port)| port.id.clone()),
                                system: found.map(|(scene, _)| scene.system.to_string()),
                            });
                            if let Some(aboard) = pending {
                                commands.insert_resource(aboard);
                            }
                            hull_at = docked.cells.iter().find_map(|c| c.placement);
                            let cells = ship.cells.iter().map(|c| c.id.clone()).collect();
                            (docked, cells)
                        }
                        Err(err) => {
                            error!("amarrage impossible : {err}");
                            (district.clone(), Vec::new())
                        }
                    }
                }
                _ => (district.clone(), Vec::new()),
            }
        }
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
    if let Some(placement) = hull_at {
        spawn_hull(
            &mut commands,
            &asset_server,
            start.and_then(|s| s.hull.as_deref()),
            placement,
            root,
        );
    }
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
        ship_cells,
    });
}

/// Pose la coque extérieure du vaisseau amarré, au placement de ses cellules (même repère que
/// l'intérieur, pont à y = 0). Ses nœuds `*_col` deviennent des collisions fixes et ses décors
/// `detail` suivent le préréglage (`gltf::on_instance_ready`, comme une cellule). Fichier pas
/// encore produit : le vaisseau reste sans coque, sans erreur.
fn spawn_hull(
    commands: &mut Commands,
    asset_server: &AssetServer,
    hull: Option<&str>,
    placement: sh_data::Placement,
    root: Entity,
) {
    let Some(path) = hull else {
        return;
    };
    if !assets_dir().join(path).is_file() {
        debug!("coque du vaisseau absente ({path}) : amarré sans coque");
        return;
    }
    commands.spawn((
        Name::new("Coque du vaisseau amarré"),
        ShipHull,
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path.to_owned()))),
        placement_transform(Some(placement)),
        Visibility::default(),
        RigidBody::Static,
        ChildOf(root),
    ));
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
