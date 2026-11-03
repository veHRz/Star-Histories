//! Amarrage et départ : les transitions entre la vue Globale (le vaisseau en vol) et la vue
//! Sectorielle (le vaisseau amarré à un quartier), AGENTS.md §6.3.
//!
//! - **Amarrage** : aux commandes, à portée d'une balise d'approche, `G` demande l'amarrage. Le
//!   pilote automatique conduit le vaisseau vers la balise pendant que le quartier se charge en
//!   arrière-plan ; un fondu au noir bref couvre le changement de vue ; le joueur reprend sa
//!   place aux commandes, le vaisseau amarré à la passerelle.
//! - **Départ** : à quai, assis aux commandes, `G` : fondu, et le vaisseau repart de la balise.
//!
//! Le fondu est le seul « écran de chargement », et il est court : les modèles du quartier sont
//! préchargés pendant l'approche, et l'intérieur du vaisseau n'est jamais déchargé du cache.

use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::world_serialization::WorldAsset;
use bevy_enhanced_input::prelude::*;
use sh_core::SpacePos;

use crate::data::{Data, Texts};
use crate::input::Dock;
use crate::player::{Player, PlayerController, PlayerMode};
use crate::sector::gltf::Interactable;
use crate::sector::streaming::{CellReady, CellStates};
use crate::sector::{Aboard, ActiveSector, Docked, PendingAboard};
use crate::ship::ShipMotion;
use crate::space::{SpacePosition, SpaceRotation};
use crate::state::{CurrentPlace, ViewLevel};
use crate::ui::hud::Notice;

pub struct TransitionPlugin;

impl Plugin for TransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fade>()
            .add_observer(on_dock_key)
            .add_systems(Startup, spawn_fade)
            .add_systems(
                Update,
                (
                    run_sequence.run_if(resource_exists::<Sequence>),
                    animate_fade,
                )
                    .chain(),
            );
    }
}

/// Durée de l'approche automatique avant le fondu (s).
const APPROACH_SECONDS: f32 = 5.0;
/// Vitesse d'approche maximale (m/s).
const APPROACH_SPEED: f64 = 400.0;
/// Durée d'un fondu (s).
const FADE_SECONDS: f32 = 0.5;
/// Au départ, le vaisseau réapparaît à cette distance de la balise (m).
const DEPARTURE_DISTANCE: f64 = 2500.0;

/// Balise d'approche d'un port, en vue Globale.
#[derive(Component, Clone, Debug)]
pub struct DockBeacon {
    pub id: String,
    pub sector: String,
    pub range: f64,
}

/// Où la vue Globale fait arriver le vaisseau (sinon l'arrivée de la mise en scène).
#[derive(Resource, Clone, Debug)]
pub struct ArrivalRequest {
    pub near: String,
    pub distance: f64,
}

/// La transition en cours.
#[derive(Resource, Clone, Debug)]
pub enum Sequence {
    /// Approche automatique de la balise, puis fondu et vue Sectorielle.
    Docking {
        t: f32,
        beacon: SpacePos,
        sector: String,
        /// Modèles du quartier et du vaisseau, gardés en mémoire jusqu'à l'arrivée.
        keep: Vec<UntypedHandle>,
    },
    /// Fondu, puis vue Globale près de la balise du port.
    Departing {
        near: String,
        system: String,
        keep: Vec<UntypedHandle>,
    },
    /// Le fondu se lève quand la cellule du joueur est prête.
    Arriving { keep: Vec<UntypedHandle> },
}

/// Le voile noir des transitions : opacité courante et visée.
#[derive(Resource, Default)]
pub struct Fade {
    pub alpha: f32,
    pub target: f32,
}

#[derive(Component)]
struct FadeVeil;

fn spawn_fade(mut commands: Commands) {
    commands.spawn((
        Name::new("Voile de transition"),
        FadeVeil,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(0.0)),
        GlobalZIndex(i32::MAX - 1),
        Pickable::IGNORE,
    ));
}

fn animate_fade(
    time: Res<Time>,
    mut fade: ResMut<Fade>,
    mut veil: Query<&mut BackgroundColor, With<FadeVeil>>,
) {
    let step = time.delta_secs() / FADE_SECONDS;
    let alpha = if fade.alpha < fade.target {
        (fade.alpha + step).min(fade.target)
    } else {
        (fade.alpha - step).max(fade.target)
    };
    if alpha != fade.alpha {
        fade.alpha = alpha;
        for mut color in &mut veil {
            color.0 = Color::BLACK.with_alpha(alpha);
        }
    }
}

fn on_dock_key(_: On<Start<Dock>>, mut commands: Commands) {
    commands.queue(request_dock);
}

/// Le joueur est-il assis au poste de pilotage ?
fn at_helm(world: &mut World) -> bool {
    let mut players = world.query_filtered::<&PlayerController, With<Player>>();
    let Ok(controller) = players.single(world) else {
        return false;
    };
    let PlayerMode::Seated { seat, .. } = controller.mode else {
        return false;
    };
    world
        .get::<Interactable>(seat)
        .is_some_and(|i| matches!(i, Interactable::Seat { role } if role == "pilot"))
}

/// Demande d'amarrage (en vol) ou de départ (à quai). Appelée par la touche ou l'autopilote.
pub fn request_dock(world: &mut World) {
    if world.contains_resource::<Sequence>() || !at_helm(world) {
        return;
    }
    let view = world.get_resource::<State<ViewLevel>>().map(|s| *s.get());
    let notice = |world: &mut World, key: &str| {
        let text = world.resource::<Texts>().get(key);
        world.write_message(Notice(text));
    };
    match view {
        Some(ViewLevel::Global) => {
            let Some(sector) = world.get_resource::<ActiveSector>().map(|s| s.root) else {
                return;
            };
            let Some(ship) = world.get::<SpacePosition>(sector).map(|p| p.0) else {
                return;
            };
            let mut beacons = world.query::<(&DockBeacon, &SpacePosition)>();
            let nearest = beacons
                .iter(world)
                .map(|(b, p)| (b.clone(), p.0, p.0.distance(ship)))
                .filter(|(b, _, d)| *d <= b.range)
                .min_by(|a, b| a.2.total_cmp(&b.2));
            let Some((beacon, pos, _)) = nearest else {
                notice(world, "notice.dock.out_of_range");
                return;
            };
            // Préchargement du quartier pendant l'approche (et maintien du vaisseau en cache).
            let ship = world.resource::<ActiveSector>().sector.id.to_string();
            let keep = preload(world, &[beacon.sector.as_str(), ship.as_str()]);
            world.insert_resource(Sequence::Docking {
                t: 0.0,
                beacon: pos,
                sector: beacon.sector,
                keep,
            });
            notice(world, "notice.dock.cleared");
        }
        Some(ViewLevel::Sector) => {
            let Some(docked) = world.get_resource::<Docked>().cloned() else {
                return;
            };
            // Le port de ce quartier, dans la mise en scène de son système.
            let found = world.resource::<Data>().scenes.iter().find_map(|scene| {
                scene
                    .docks
                    .iter()
                    .find(|d| d.sector == docked.district)
                    .map(|d| (d.id.clone(), scene.system.to_string()))
            });
            let Some((near, system)) = found else {
                warn!("départ impossible : aucun port pour {}", docked.district);
                return;
            };
            let keep = preload(world, &[docked.ship.as_str()]);
            world.insert_resource(Sequence::Departing { near, system, keep });
            world.resource_mut::<Fade>().target = 1.0;
            notice(world, "notice.undock.cleared");
        }
        _ => {}
    }
}

/// Charge (ou retient en cache) les modèles et lightmaps des cellules de ces secteurs.
fn preload(world: &World, sectors: &[&str]) -> Vec<UntypedHandle> {
    let data = world.resource::<Data>();
    let server = world.resource::<AssetServer>();
    let mut handles = Vec::new();
    for cell in sectors
        .iter()
        .filter_map(|id| data.sector(id))
        .flat_map(|s| s.cells.iter())
    {
        // Les mêmes chemins et étiquettes que le streaming : le cache sert directement.
        for model in std::iter::once(&cell.model).chain(cell.lod.as_ref()) {
            let scene: Handle<WorldAsset> =
                server.load(GltfAssetLabel::Scene(0).from_asset(model.clone()));
            handles.push(scene.untyped());
        }
        if let Some(lightmap) = &cell.lightmap {
            handles.push(server.load::<Image>(lightmap.image.clone()).untyped());
        }
    }
    handles
}

/// La place à bord qu'on reprend après une transition : aux commandes.
fn helm_aboard(data: &Data) -> Option<Aboard> {
    let start = data.start()?;
    Some(Aboard {
        spawn: start.aboard_spawn.clone(),
        seat_role: Some("pilot".into()),
    })
}

#[allow(clippy::too_many_arguments)]
fn run_sequence(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<Data>,
    sequence: Option<ResMut<Sequence>>,
    mut fade: ResMut<Fade>,
    sector: Option<Res<ActiveSector>>,
    cells: Res<CellStates>,
    ready: Query<(), With<CellReady>>,
    player: Query<&PlayerController, With<Player>>,
    mut ships: Query<(&mut SpacePosition, &mut SpaceRotation, &mut ShipMotion)>,
    mut place: ResMut<CurrentPlace>,
    mut view: ResMut<NextState<ViewLevel>>,
) {
    let Some(mut sequence) = sequence else {
        return;
    };
    let dt = time.delta_secs();
    match &mut *sequence {
        Sequence::Docking {
            t,
            beacon,
            sector: target,
            keep,
        } => {
            *t += dt;
            // Pilote automatique : cap sur la balise, vitesse d'approche décroissante.
            if let Some(root) = sector.as_ref().map(|s| s.root)
                && let Ok((mut pos, mut rot, mut motion)) = ships.get_mut(root)
            {
                let to = *beacon - pos.0;
                let distance = to.length().max(1e-3);
                let dir = DVec3::new(to.x, to.y, to.z) / distance;
                let speed = (distance * 0.5).min(APPROACH_SPEED);
                motion.velocity = dir * speed;
                motion.angular = Vec3::ZERO;
                let goal = Transform::default()
                    .looking_to(dir.as_vec3(), Vec3::Y)
                    .rotation;
                rot.0 = rot.0.slerp(goal, (2.0 * dt).min(1.0));
                let d = motion.velocity * f64::from(dt);
                pos.0 += SpacePos::new(d.x, d.y, d.z);
            }
            if *t >= APPROACH_SECONDS {
                fade.target = 1.0;
            }
            if *t >= APPROACH_SECONDS && fade.alpha >= 0.999 {
                if let Some(aboard) = helm_aboard(&data) {
                    commands.insert_resource(PendingAboard(aboard));
                }
                place.0 = Some(target.clone());
                view.set(ViewLevel::Sector);
                let keep = std::mem::take(keep);
                *sequence = Sequence::Arriving { keep };
            }
        }
        Sequence::Departing { near, system, keep } => {
            if fade.alpha >= 0.999 {
                if let Some(aboard) = helm_aboard(&data) {
                    commands.insert_resource(PendingAboard(aboard));
                }
                commands.insert_resource(ArrivalRequest {
                    near: near.clone(),
                    distance: DEPARTURE_DISTANCE,
                });
                place.0 = Some(system.clone());
                view.set(ViewLevel::Global);
                let keep = std::mem::take(keep);
                *sequence = Sequence::Arriving { keep };
            }
        }
        Sequence::Arriving { .. } => {
            // Le voile se lève quand le joueur est à sa place, cellule prête.
            let seated = player
                .single()
                .is_ok_and(|p| matches!(p.mode, PlayerMode::Seated { .. }));
            let cell_ready = cells
                .current
                .as_deref()
                .is_some_and(|c| cells.is_local_ready(c, &ready));
            if sector.is_some() && seated && cell_ready {
                fade.target = 0.0;
                if fade.alpha <= 0.001 {
                    commands.remove_resource::<Sequence>();
                }
            }
        }
    }
}
