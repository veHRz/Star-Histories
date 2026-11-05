//! Vue Globale : un système stellaire à l'échelle de jeu (propositions P2, P15 à P18).
//!
//! Les astres et les stations sont disposés par la règle générique de `sh_data::layout` ;
//! ce que le canon ne place pas (hub de Portails, chantiers, trafic, point d'arrivée) vient de la
//! mise en scène du système (`assets/data/propositions/systems/`). Positions absolues en `f64`
//! ([`SpacePosition`]) ; la caméra est le point suivi par l'origine flottante ([`crate::space`]).
//! Le ciel est généré depuis la position du système dans la galaxie ([`super::sky`]).
//! Les corps secrets ne sont jamais dessinés.
//!
//! Le joueur est **à bord de son vaisseau** (départ de partie) : le secteur du vaisseau est
//! chargé ([`crate::sector`]), sa racine reçoit la pose d'arrivée et devient le référentiel de
//! l'origine flottante ; le pilotage est dans [`crate::ship`].
//!
//! **Monde isolé** (M9, Q18) : on arrive en orbite d'un astre hors système. La scène est
//! générique (`sh_data::layout::isolated_layout`) : l'astre, ses lunes, aucune structure ni
//! spatioport. Aucune étoile n'est nommée par le canon : un monde « solitaire » reçoit la
//! lumière d'une étoile lointaine (un point brillant du ciel, direction tirée de son nom) ; une
//! planète errante, celle du bulbe galactique, plus faible.

use bevy::light::Skybox;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use sh_core::{MapPos, SimRng, SpacePos};
use sh_data::travel::{Endpoint, endpoint};
use sh_data::{Body, BodyKind, Galaxy, Place, PlacedKind, StructureKind, system_layout};

use crate::career::CargoPoint;
use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::sector::{ActiveSector, EnterSector};
use crate::ship::{ShipMotion, SurfaceBody};
use crate::space::{FloatingOrigin, PositionDriver, SpacePosition, SpaceRotation};
use crate::state::{CurrentPlace, ViewLevel};
use crate::transition::{ArrivalRequest, DockBeacon};
use crate::ui::theme::{self, UiFonts};
use crate::views::procedural::{Procedural, SKY_SIZE, make_planet, make_sky, planet_key, sky_key};
use crate::views::sky::{SkyNebula, SkySpec, cube_image, render_sky};
use crate::views::{WorldLabel, view_banner};
use crate::voyage::{Location, ResumeFlight};

pub struct GlobalViewPlugin;

impl Plugin for GlobalViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Global), spawn_global.after(EnterSector))
            .add_systems(
                Update,
                (move_traffic, aim_sunlight, attach_sky)
                    .chain()
                    .run_if(in_state(ViewLevel::Global)),
            );
    }
}

const SCOPE: DespawnOnExit<ViewLevel> = DespawnOnExit(ViewLevel::Global);
/// La caméra du joueur a l'exposition d'intérieur (`Exposure::INDOOR`, EV100 7) : l'extérieur,
/// vu par le hublot, est réglé pour elle (2,7 IL plus sensible que l'exposition par défaut).
/// Luminance du ciel (cd/m²) pour une valeur 1 de la cubemap.
const SKY_BRIGHTNESS: f32 = 140.0;
/// Éclairement de l'étoile, avec l'exposition par défaut de la caméra (réglage visuel).
const SUNLIGHT_LUX: f32 = 400.0;
/// Monde isolé : part de l'éclairement d'une étoile proche que reçoit un monde solitaire (une
/// étoile lointaine), et une planète errante (la lueur du bulbe). Réglages visuels : assez pour
/// voir l'astre, assez peu pour sentir l'isolement.
const LONELY_LIGHT: f32 = 0.8;
const ROGUE_LIGHT: f32 = 0.45;
/// L'étoile lointaine d'un monde solitaire : distance (m) et rayon apparent (≈ 0,3°).
const DISTANT_STAR_DISTANCE: f64 = 4.0e8;
const DISTANT_STAR_RADIUS: f32 = 2.0e6;

/// Le ciel du système, posé sur la caméra du joueur dès qu'elle existe.
#[derive(Resource)]
struct GlobalSky(Handle<Image>);

/// L'étoile qui éclaire le système.
#[derive(Component)]
struct Star;

/// La lumière de l'étoile, orientée chaque frame de l'étoile vers la caméra.
#[derive(Component)]
struct Sunlight;

/// Figurant du trafic ambiant : navette entre deux points, à vitesse constante.
#[derive(Component)]
struct Shuttle {
    from: SpacePos,
    to: SpacePos,
    speed: f64,
    /// Avancée sur le trajet, de 0 à 1.
    progress: f64,
}

/// Le système à afficher pour un lieu : le lieu lui-même s'il est un système, sinon le système
/// de l'astre ou de la station.
fn system_of<'a>(galaxy: &'a Galaxy, place: &str) -> Option<&'a sh_data::StarSystem> {
    let id = sh_core::Id::<Place>::new(place).ok()?;
    galaxy.system(&id).or_else(|| {
        let system = galaxy
            .body(&id)
            .and_then(|b| b.system.clone())
            .or_else(|| galaxy.station(&id).and_then(|s| s.system.clone()))?;
        galaxy.system(&system)
    })
}

/// Le monde isolé public à afficher pour un lieu (M9) : le lieu lui-même s'il en est un.
fn isolated_of<'a>(galaxy: &'a Galaxy, place: &str) -> Option<&'a Body> {
    let id = sh_core::Id::<Place>::new(place).ok()?;
    match endpoint(galaxy, &id)? {
        Endpoint::Isolated(body) => Some(body),
        Endpoint::System(_) => None,
    }
}

/// Direction absolue (unitaire) d'où vient la lumière d'un monde isolé. Plan du système = plan
/// galactique (X = est de la carte, Z = sud). Planète errante : le bulbe galactique, un peu
/// au-dessus du plan. Monde solitaire : une étoile lointaine, direction tirée de son nom
/// (stable d'une visite à l'autre, sans rien affirmer de plus).
fn isolated_light(galaxy: &Galaxy, root: &Body) -> SpacePos {
    let (x, z, y) = if root.kind == BodyKind::RoguePlanet {
        let pos = root
            .pos
            .unwrap_or(MapPos::new(galaxy.frame.center.0, galaxy.frame.center.1));
        let (dx, dz) = (galaxy.frame.center.0 - pos.x, galaxy.frame.center.1 - pos.y);
        let length = dx.hypot(dz).max(1e-6);
        (dx / length, dz / length, 0.12)
    } else {
        let hash = sh_core::rng::fnv1a(root.id.as_str().as_bytes());
        let angle = ((hash % 3600) as f64 / 10.0).to_radians();
        let elevation = 8f64.to_radians();
        (
            elevation.cos() * angle.cos(),
            elevation.cos() * angle.sin(),
            elevation.sin(),
        )
    };
    let v = SpacePos::new(x, y, z);
    v * (1.0 / v.length().max(1e-9))
}

/// Le ciel vu d'un monde isolé (même génération que celui d'un système, autre point de vue).
fn make_isolated_sky(galaxy: &Galaxy, root: &Body) -> Image {
    let frame = &galaxy.frame;
    let observer = root
        .pos
        .map_or([frame.center.0, frame.center.1], |p| [p.x, p.y]);
    let nebulae = galaxy
        .phenomena
        .iter()
        .filter(|p| !p.secret && p.kind_note.to_lowercase().contains("nébuleuse"))
        .filter_map(|p| {
            Some(SkyNebula {
                center: [p.pos?.x, p.pos?.y],
                radius: p.radius?,
            })
        })
        .collect();
    let spec = SkySpec {
        observer,
        galaxy_center: [frame.center.0, frame.center.1],
        galaxy_radii: [frame.radius_x, frame.radius_y],
        nebulae,
        seed: sh_core::rng::fnv1a(root.id.as_str().as_bytes()),
    };
    cube_image(&render_sky(&spec, SKY_SIZE))
}

/// Un point à `radius` mètres de `center`, dans la direction `angle_deg` du plan du système.
fn around(center: SpacePos, radius: f64, angle_deg: f64) -> SpacePos {
    let a = angle_deg.to_radians();
    center + SpacePos::new(radius * a.cos(), 0.0, radius * a.sin())
}

/// Orientation absolue qui pointe l'avant (−Z) d'un objet vers `forward`.
fn facing(forward: Vec3) -> Quat {
    Transform::default().looking_to(forward, Vec3::Y).rotation
}

/// Direction d'un déplacement absolu, en `f32` (les écarts sont petits devant la précision).
fn dir(v: SpacePos) -> Vec3 {
    Vec3::new(v.x as f32, v.y as f32, v.z as f32).normalize_or(Vec3::NEG_Z)
}

#[allow(clippy::too_many_arguments)]
fn spawn_global(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    place: Res<CurrentPlace>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    sector: Option<Res<ActiveSector>>,
    request: Option<Res<ArrivalRequest>>,
    resume: Option<Res<ResumeFlight>>,
    mut cache: ResMut<Procedural>,
) {
    let g = &data.galaxy;
    let scale = &data.space.scale;
    let place_id = place.0.clone().unwrap_or_else(|| "test".into());
    let system = system_of(g, &place_id).filter(|s| !s.secret);
    // Pas de système : peut-être un monde isolé (M9), et le monde au centre de sa scène.
    let isolated = system
        .is_none()
        .then(|| isolated_of(g, &place_id))
        .flatten();
    let root = isolated.map(|b| sh_data::travel::isolated_root(g, b));
    let scene = system.and_then(|s| data.scene(&s.id));
    let location = system
        .map(|s| s.id.clone())
        .or_else(|| isolated.map(|b| b.id.clone()));
    if let Some(location) = &location {
        commands.insert_resource(Location {
            system: location.clone(),
        });
    }
    // Retour de la carte : le vaisseau reprend sa pose au même endroit.
    let resume = resume
        .filter(|r| location.as_ref() == Some(&r.system))
        .map(|r| (r.pos, r.rot));
    commands.remove_resource::<ResumeFlight>();
    let sky = match root {
        Some(root) => cache.get(&format!("sky:{}", root.id), &mut images, || {
            make_isolated_sky(g, root)
        }),
        None => cache.get(&sky_key(system), &mut images, || make_sky(g, system)),
    };
    // D'où vient la lumière d'un monde isolé (pour un système : de son étoile).
    let light_dir = root.map(|r| isolated_light(g, r));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.6, 0.7, 0.9),
        brightness: 2.5,
        // L'intérieur du vaisseau a ses lightmaps (rebonds compris).
        affects_lightmapped_meshes: false,
    });

    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(128, 64));
    let mut anchors: Vec<(String, SpacePos)> = Vec::new();
    // Stations et structures : les extrémités des trajets du trafic.
    let mut ports: Vec<SpacePos> = Vec::new();
    let mut title_place = place_id.clone();

    let placed = match (system, isolated) {
        (Some(s), _) => system_layout(g, s, scale),
        (None, Some(body)) => sh_data::layout::isolated_layout(g, body, scale),
        (None, None) => Vec::new(),
    };
    if system.is_none() && isolated.is_none() && place_id != "test" {
        warn!("vue Globale : aucun système ni monde isolé public pour « {place_id} »");
    }
    if let Some(name) = system
        .map(|s| s.name.clone())
        .or_else(|| isolated.map(|b| b.name.clone()))
    {
        title_place = name;
    }
    for p in &placed {
        let id = p.id.as_ref().map(|i| i.as_str().to_owned());
        let name = match (&p.id, p.kind) {
            (Some(id), PlacedKind::Station) => g.station(id).map(|s| s.name.clone()),
            (Some(id), _) => g.body(id).map(|b| b.name.clone()),
            (None, _) => None,
        }
        .or_else(|| {
            scene
                .and_then(|s| s.star_name.clone())
                .filter(|_| p.kind == PlacedKind::Star)
        })
        .unwrap_or_else(|| texts.get("view.global.unnamed_star"));
        if let Some(id) = &id {
            debug!("{id} ({:?}) en {:?}", p.kind, p.pos);
            anchors.push((id.clone(), p.pos));
        }

        let entity = match p.kind {
            PlacedKind::Star => commands
                .spawn((
                    Star,
                    SurfaceBody { radius: p.radius },
                    Mesh3d(sphere.clone()),
                    // Émissif intense (en nits) : le disque sature et le bloom fait le halo.
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::BLACK,
                        emissive: LinearRgba::rgb(6.0e3, 5.2e3, 4.0e3),
                        ..default()
                    })),
                    Transform::from_scale(Vec3::splat(p.radius as f32)),
                ))
                .id(),
            PlacedKind::Station => {
                let model = id
                    .as_ref()
                    .and_then(|id| scene.and_then(|s| s.station_models.get(id)));
                // Le grand axe d'une station suit son orbite autour de son astre.
                let parent = p
                    .around
                    .as_ref()
                    .and_then(|a| placed.iter().find(|q| q.id.as_ref() == Some(a)))
                    .map_or(SpacePos::ZERO, |q| q.pos);
                let radial = dir(p.pos - parent);
                let tangent = Vec3::Y.cross(radial).normalize_or(Vec3::NEG_Z);
                ports.push(p.pos);
                let mut station = commands.spawn(SpaceRotation(facing(tangent)));
                match model {
                    Some(model) => station.insert(WorldAssetRoot(
                        asset_server.load(GltfAssetLabel::Scene(0).from_asset(model.clone())),
                    )),
                    None => station.insert((
                        Mesh3d(meshes.add(Cuboid::new(400.0, 150.0, 400.0))),
                        MeshMaterial3d(materials.add(Color::srgb(0.6, 0.6, 0.62))),
                    )),
                };
                station.id()
            }
            kind => {
                let body = id.clone().unwrap_or_default();
                let texture =
                    cache.get(&planet_key(&body), &mut images, || make_planet(kind, &body));
                commands
                    .spawn((
                        SurfaceBody { radius: p.radius },
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color_texture: Some(texture),
                            perceptual_roughness: 0.95,
                            ..default()
                        })),
                        Transform::from_scale(Vec3::splat(p.radius as f32)),
                    ))
                    .id()
            }
        };
        commands
            .entity(entity)
            .insert((Name::new(name.clone()), SCOPE, SpacePosition(p.pos)));
        commands.spawn(label(&fonts, &name, entity));
    }

    // Structures sans fiche canon (hub de Portails, chantiers) : mise en scène proposée.
    for st in scene.map(|s| s.structures.as_slice()).unwrap_or_default() {
        let Some(body) = placed.iter().find(|q| q.id.as_ref() == Some(&st.around)) else {
            continue;
        };
        let pos = around(
            body.pos,
            body.radius * (1.0 + st.altitude_radii),
            st.angle_deg,
        );
        let radial = dir(pos - body.pos);
        let facing_dir = match st.kind {
            // Les anneaux du hub, dans le plan de l'orbite ; les cales, le long de l'orbite.
            StructureKind::PortalHub => radial,
            StructureKind::Shipyard => Vec3::Y.cross(radial),
        };
        let name = texts.get(&st.name_key);
        debug!("structure {} en {pos:?}", st.id);
        let entity = commands
            .spawn((
                Name::new(name.clone()),
                SCOPE,
                CargoPoint(st.kind),
                SpacePosition(pos),
                SpaceRotation(facing(facing_dir)),
                WorldAssetRoot(
                    asset_server.load(GltfAssetLabel::Scene(0).from_asset(st.model.clone())),
                ),
            ))
            .id();
        commands.spawn(label(&fonts, &name, entity));
        anchors.push((st.id.clone(), pos));
        ports.push(pos);
    }

    // Balises d'approche des ports (le spatioport d'Hexis) : un fanal et un repère.
    let beacon_mesh = meshes.add(Sphere::new(30.0).mesh().uv(16, 8));
    let beacon_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(400.0, 160.0, 30.0),
        ..default()
    });
    for dock in scene.map(|s| s.docks.as_slice()).unwrap_or_default() {
        let Some(body) = placed.iter().find(|q| q.id.as_ref() == Some(&dock.around)) else {
            continue;
        };
        let pos = around(
            body.pos,
            body.radius * (1.0 + dock.altitude_radii),
            dock.angle_deg,
        );
        let name = texts.get(&dock.name_key);
        let entity = commands
            .spawn((
                Name::new(name.clone()),
                SCOPE,
                DockBeacon {
                    id: dock.id.clone(),
                    sector: dock.sector.clone(),
                    range: dock.range,
                },
                SpacePosition(pos),
                Mesh3d(beacon_mesh.clone()),
                MeshMaterial3d(beacon_material.clone()),
            ))
            .id();
        commands.spawn(label(&fonts, &name, entity));
        anchors.push((dock.id.clone(), pos));
    }

    // Trafic ambiant, réparti de façon déterministe : la moitié des figurants fait la navette
    // entre deux installations, l'autre moitié des approches courtes autour d'une seule (le
    // trafic se voit là où l'on est, pas seulement sur les longs trajets).
    if let (Some(traffic), Some(system)) = (scene.and_then(|s| s.traffic.as_ref()), system) {
        let hubs = &ports;
        let model = asset_server.load(GltfAssetLabel::Scene(0).from_asset(traffic.model.clone()));
        let mut rng =
            SimRng::new(sh_core::rng::fnv1a(system.id.as_str().as_bytes())).fork_named("trafic");
        for i in 0..traffic.ships {
            if hubs.len() < 2 {
                break;
            }
            let a = i as usize % hubs.len();
            let b = (a + 1 + rng.range_u32(0, hubs.len() as u32 - 1) as usize) % hubs.len();
            let lane = |r: &mut SimRng, spread: f64| {
                SpacePos::new(
                    r.unit_f64() - 0.5,
                    0.2 * (r.unit_f64() - 0.5),
                    r.unit_f64() - 0.5,
                ) * spread
            };
            let (from, to) = if i % 2 == 0 {
                (
                    hubs[a] + lane(&mut rng, 3000.0),
                    hubs[b] + lane(&mut rng, 3000.0),
                )
            } else {
                (
                    hubs[a] + lane(&mut rng, 16_000.0),
                    hubs[a] + lane(&mut rng, 2500.0),
                )
            };
            let progress = rng.unit_f64();
            // Repère pour l'autopilote (`SH_ARRIVAL=traffic_0,…`) : sa position de départ.
            anchors.push((format!("traffic_{i}"), from + (to - from) * progress));
            commands.spawn((
                Name::new(format!("Trafic {i}")),
                SCOPE,
                Shuttle {
                    from,
                    to,
                    speed: traffic.speed * (0.7 + 0.6 * rng.unit_f64()),
                    progress,
                },
                SpacePosition(from + (to - from) * progress),
                SpaceRotation(facing(dir(to - from))),
                WorldAssetRoot(model.clone()),
            ));
        }
    }

    // Monde isolé : l'étoile lointaine (un point brillant du ciel) ou, pour une planète errante,
    // un repère invisible vers le bulbe. `aim_sunlight` oriente la lumière depuis elle.
    if let (Some(root), Some(light)) = (root, light_dir) {
        let mut star = commands.spawn((
            Name::new("Étoile lointaine"),
            SCOPE,
            Star,
            SpacePosition(light * DISTANT_STAR_DISTANCE),
        ));
        if root.kind != BodyKind::RoguePlanet {
            star.insert((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::BLACK,
                    emissive: LinearRgba::rgb(2.4e3, 2.2e3, 1.9e3),
                    ..default()
                })),
                Transform::from_scale(Vec3::splat(DISTANT_STAR_RADIUS)),
            ));
        }
    }

    // La lumière de l'étoile (orientée par `aim_sunlight`).
    let illuminance = match root {
        Some(r) if r.kind == BodyKind::RoguePlanet => SUNLIGHT_LUX * ROGUE_LIGHT,
        Some(_) => SUNLIGHT_LUX * LONELY_LIGHT,
        None => SUNLIGHT_LUX,
    };
    commands.spawn((
        SCOPE,
        Sunlight,
        DirectionalLight {
            illuminance,
            ..default()
        },
        Transform::default(),
    ));

    // Arrivée : du côté de l'étoile, au-dessus du plan, en regardant l'objet visé.
    let arrival = scene.map(|s| &s.arrival);
    // Au départ d'un quai, on arrive près de sa balise ; `SH_ARRIVAL=<objet>,<distance>`
    // (autopilote) : arrivée ailleurs, pour les captures.
    let forced = request
        .map(|r| (r.near.clone(), r.distance))
        .or_else(arrival_from_env);
    commands.remove_resource::<ArrivalRequest>();
    // Monde isolé : en orbite de l'astre visé (la lune elle-même si c'est une lune).
    let orbit = isolated.and_then(|body| {
        placed
            .iter()
            .find(|p| p.id.as_ref() == Some(&body.id))
            .map(|p| {
                (
                    body.id.as_str(),
                    p.radius * data.travel.isolated_arrival_radii,
                )
            })
    });
    let near = forced
        .as_ref()
        .map(|f| f.0.as_str())
        .or(arrival.map(|a| a.near.as_str()))
        .or(orbit.map(|o| o.0));
    let target = near
        .and_then(|n| anchors.iter().find(|(id, _)| id == n))
        .map_or(
            SpacePos::new(0.0, 0.0, 4.0 * scale.star_radius),
            |(_, p)| *p,
        );
    let distance = forced
        .map(|f| f.1)
        .or(arrival.map(|a| a.distance))
        .or(orbit.map(|o| o.1))
        .unwrap_or(scale.star_radius);
    // Sans mise en scène, un monde isolé s'aborde de trois quarts, comme Hexis.
    let (elevation, azimuth) = match (arrival, isolated) {
        (Some(a), _) => (a.elevation_deg, a.azimuth_deg),
        (None, Some(_)) => (12.0, -50.0),
        (None, None) => (15.0, 0.0),
    };
    let elevation = elevation.to_radians();
    let azimuth = azimuth.to_radians() as f32;
    let to_star = match light_dir {
        Some(light) => dir(light),
        None => dir(SpacePos::ZERO - target),
    };
    let horizontal =
        Quat::from_rotation_y(azimuth) * Vec3::new(to_star.x, 0.0, to_star.z).normalize_or(Vec3::Z);
    let offset = horizontal * elevation.cos() as f32 + Vec3::Y * elevation.sin() as f32;
    let eye = target
        + SpacePos::new(
            f64::from(offset.x),
            f64::from(offset.y),
            f64::from(offset.z),
        ) * distance;
    let look = look_from_env().unwrap_or(-offset);

    // Le vaisseau du joueur arrive là, l'avant vers l'objet visé : sa racine devient le
    // référentiel de l'origine flottante.
    match sector {
        Some(sector) => {
            let (eye, rotation) = resume.unwrap_or((eye, facing(look)));
            commands.entity(sector.root).insert((
                SpacePosition(eye),
                SpaceRotation(rotation),
                PositionDriver::Frame,
                ShipMotion::default(),
            ));
        }
        None => warn!("vue Globale sans vaisseau du joueur (aucun départ de partie ?)"),
    }
    commands.insert_resource(GlobalSky(sky));

    commands.spawn(view_banner(
        &texts,
        &fonts,
        texts.format("view.global.title", &[("place", &title_place)]),
        "view.global.hint",
        SCOPE,
    ));
}

fn arrival_from_env() -> Option<(String, f64)> {
    let value = std::env::var("SH_ARRIVAL").ok()?;
    let (near, distance) = value.split_once(',')?;
    Some((near.trim().to_owned(), distance.trim().parse().ok()?))
}

/// `SH_LOOK=x,y,z` (autopilote) : direction initiale de la caméra, pour les captures.
fn look_from_env() -> Option<Vec3> {
    let values: Vec<f32> = std::env::var("SH_LOOK")
        .ok()?
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    let dir = Vec3::new(*values.first()?, *values.get(1)?, *values.get(2)?);
    (dir.length_squared() > 0.0).then_some(dir)
}

fn label(fonts: &UiFonts, name: &str, target: Entity) -> impl Bundle {
    (
        SCOPE,
        WorldLabel::Follow(target),
        Text::new(name),
        fonts.body(14.0),
        TextColor(theme::TEXT),
        // Ombre courte : le nom reste lisible devant un astre éclairé ou la bande galactique.
        theme::label_shadow(),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
    )
}

/// Les figurants avancent sur leur trajet, et repartent en sens inverse au bout.
fn move_traffic(
    time: Res<Time>,
    mut shuttles: Query<(&mut Shuttle, &mut SpacePosition, &mut SpaceRotation)>,
) {
    let dt = f64::from(time.delta_secs());
    for (mut shuttle, mut pos, mut rotation) in &mut shuttles {
        let length = shuttle.from.distance(shuttle.to).max(1.0);
        shuttle.progress += shuttle.speed * dt / length;
        if shuttle.progress >= 1.0 {
            let (from, to) = (shuttle.from, shuttle.to);
            shuttle.from = to;
            shuttle.to = from;
            shuttle.progress -= 1.0;
            rotation.0 = facing(dir(from - to));
        }
        pos.0 = shuttle.from + (shuttle.to - shuttle.from) * shuttle.progress;
    }
}

/// La lumière vient de l'étoile vers l'origine du rendu (la caméra ou le vaisseau), dans les
/// axes du rendu ; le ciel tourne avec eux.
fn aim_sunlight(
    origin: Res<FloatingOrigin>,
    star: Query<&SpacePosition, With<Star>>,
    mut light: Query<&mut Transform, With<Sunlight>>,
    mut skies: Query<&mut Skybox>,
) {
    let (Ok(star), Ok(mut light)) = (star.single(), light.single_mut()) else {
        return;
    };
    let d = origin.origin - star.0;
    let direction = origin
        .render_direction(bevy::math::DVec3::new(d.x, d.y, d.z))
        .normalize_or(Vec3::NEG_Z);
    if light.forward().dot(direction) < 0.999_999 {
        light.look_to(direction, Vec3::Y);
    }
    // Le shader lit le ciel avec `rotation⁻¹ × rayon` : rayon du rendu → direction absolue.
    let sky_rotation = origin.orientation.inverse().as_quat();
    for mut sky in &mut skies {
        if sky.rotation != sky_rotation {
            sky.rotation = sky_rotation;
        }
    }
}

/// Le ciel sur la caméra du monde (celle du joueur, créée après la vue).
fn attach_sky(
    mut commands: Commands,
    sky: Option<Res<GlobalSky>>,
    cameras: Query<Entity, (With<WorldCamera>, Without<Skybox>)>,
) {
    let Some(sky) = sky else {
        return;
    };
    for camera in &cameras {
        commands.entity(camera).insert(Skybox {
            image: Some(sky.0.clone()),
            brightness: SKY_BRIGHTNESS,
            ..default()
        });
    }
}
