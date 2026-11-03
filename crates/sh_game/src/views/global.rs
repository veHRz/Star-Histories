//! Vue Globale : un système stellaire à l'échelle de jeu (propositions P2, P15 à P18).
//!
//! Les astres et les stations sont disposés par la règle générique de `sh_data::layout` ;
//! ce que le canon ne place pas (hub de Portails, chantiers, trafic, point d'arrivée) vient de la
//! mise en scène du système (`assets/data/propositions/systems/`). Positions absolues en `f64`
//! ([`SpacePosition`]) ; la caméra est le point suivi par l'origine flottante ([`crate::space`]).
//! Le ciel est généré depuis la position du système dans la galaxie ([`super::sky`]).
//! Les corps secrets ne sont jamais dessinés.
//!
//! Le vol est encore celui d'une caméra libre : le pilotage depuis le cockpit arrive avec la
//! tâche suivante de M2. `Boost` passe en croisière (loi de vitesse de l'échelle P2).

use bevy::light::Skybox;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use bevy_enhanced_input::prelude::*;
use sh_core::{SimRng, SpacePos};
use sh_data::{Galaxy, Place, PlacedKind, StructureKind, SystemScale, system_layout};

use crate::data::{Data, Texts};
use crate::graphics::WorldCamera;
use crate::input::{Boost, Roll, Steer, Thrust};
use crate::space::{FloatingOrigin, OriginFocus, PositionDriver, SpacePosition};
use crate::state::{ControlMode, CurrentPlace, ViewLevel};
use crate::ui::theme::{self, UiFonts};
use crate::views::planet::{Look, planet_texture};
use crate::views::sky::{SkyNebula, SkySpec, cube_image, render_sky};
use crate::views::{WorldLabel, view_banner};

pub struct GlobalViewPlugin;

impl Plugin for GlobalViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Global), spawn_global)
            .add_systems(
                Update,
                (fly, move_traffic, aim_sunlight)
                    .chain()
                    .run_if(in_state(ViewLevel::Global)),
            );
    }
}

const SCOPE: DespawnOnExit<ViewLevel> = DespawnOnExit(ViewLevel::Global);
/// Luminance du ciel (cd/m²) pour une valeur 1 de la cubemap.
const SKY_BRIGHTNESS: f32 = 900.0;
const SKY_SIZE: usize = 1024;
/// Éclairement de l'étoile, avec l'exposition par défaut de la caméra (réglage visuel).
const SUNLIGHT_LUX: f32 = 2500.0;
/// Hauteur des textures de planète (la largeur est double).
const PLANET_TEXTURE: usize = 512;

/// Un astre : sa surface compte pour la vitesse de croisière.
#[derive(Component)]
struct Body {
    radius: f64,
}

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

/// Un point à `radius` mètres de `center`, dans la direction `angle_deg` du plan du système.
fn around(center: SpacePos, radius: f64, angle_deg: f64) -> SpacePos {
    let a = angle_deg.to_radians();
    center + SpacePos::new(radius * a.cos(), 0.0, radius * a.sin())
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
    mut mode: ResMut<NextState<ControlMode>>,
) {
    mode.set(ControlMode::Piloting);
    let g = &data.galaxy;
    let scale = &data.space.scale;
    let place_id = place.0.clone().unwrap_or_else(|| "test".into());
    let system = system_of(g, &place_id).filter(|s| !s.secret);
    let scene = system.and_then(|s| data.scene(&s.id));
    let sky = images.add(sky_for(g, system.map(|s| (s.id.as_str(), s.center))));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.6, 0.7, 0.9),
        brightness: 15.0,
        ..default()
    });

    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(128, 64));
    let mut anchors: Vec<(String, SpacePos)> = Vec::new();
    // Stations et structures : les extrémités des trajets du trafic.
    let mut ports: Vec<SpacePos> = Vec::new();
    let mut title_place = place_id.clone();

    let placed = system
        .map(|s| system_layout(g, s, scale))
        .unwrap_or_default();
    if system.is_none() && place_id != "test" {
        warn!("vue Globale : aucun système public pour « {place_id} »");
    }
    if let Some(system) = system {
        title_place = system.name.clone();
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
                    Body { radius: p.radius },
                    Mesh3d(sphere.clone()),
                    // Émissif intense (en nits) : le disque sature et le bloom fait le halo.
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::BLACK,
                        emissive: LinearRgba::rgb(4.0e4, 3.4e4, 2.6e4),
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
                let mut station = commands.spawn(Transform::default().looking_to(tangent, Vec3::Y));
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
                let look = match (kind, id.as_deref()) {
                    (_, Some("hexis")) => Look::Industrial,
                    (PlacedKind::GasGiant, _) => Look::GasGiant,
                    _ => Look::Rocky,
                };
                let seed = id
                    .as_deref()
                    .map_or(0, |i| sh_core::rng::fnv1a(i.as_bytes()));
                let texture = images.add(planet_texture(look, seed, PLANET_TEXTURE));
                commands
                    .spawn((
                        Body { radius: p.radius },
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
        let facing = match st.kind {
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
                SpacePosition(pos),
                Transform::default().looking_to(facing.normalize_or(Vec3::NEG_Z), Vec3::Y),
                WorldAssetRoot(
                    asset_server.load(GltfAssetLabel::Scene(0).from_asset(st.model.clone())),
                ),
            ))
            .id();
        commands.spawn(label(&fonts, &name, entity));
        anchors.push((st.id.clone(), pos));
        ports.push(pos);
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
                Transform::default().looking_to(dir(to - from), Vec3::Y),
                WorldAssetRoot(model.clone()),
            ));
        }
    }

    // La lumière de l'étoile (orientée par `aim_sunlight`).
    commands.spawn((
        SCOPE,
        Sunlight,
        DirectionalLight {
            illuminance: SUNLIGHT_LUX,
            ..default()
        },
        Transform::default(),
    ));

    // Arrivée : du côté de l'étoile, au-dessus du plan, en regardant l'objet visé.
    let arrival = scene.map(|s| &s.arrival);
    // `SH_ARRIVAL=<objet>,<distance>` (autopilote) : arrivée ailleurs, pour les captures.
    let forced = arrival_from_env();
    let near = forced
        .as_ref()
        .map(|f| f.0.as_str())
        .or(arrival.map(|a| a.near.as_str()));
    let target = near
        .and_then(|n| anchors.iter().find(|(id, _)| id == n))
        .map_or(
            SpacePos::new(0.0, 0.0, 4.0 * scale.star_radius),
            |(_, p)| *p,
        );
    let distance = forced
        .map(|f| f.1)
        .or(arrival.map(|a| a.distance))
        .unwrap_or(scale.star_radius);
    let elevation = arrival.map_or(15.0, |a| a.elevation_deg).to_radians();
    let azimuth = arrival.map_or(0.0, |a| a.azimuth_deg).to_radians() as f32;
    let to_star = dir(SpacePos::ZERO - target);
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

    commands.spawn((
        Name::new("Caméra de la vue Globale"),
        SCOPE,
        WorldCamera,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 70f32.to_radians(),
            ..default()
        }),
        Skybox {
            image: Some(sky),
            brightness: SKY_BRIGHTNESS,
            ..default()
        },
        // La caméra est pilotée par son Transform, et l'origine flottante la suit.
        SpacePosition(eye),
        PositionDriver::Render,
        OriginFocus,
        Transform::default().looking_to(look, Vec3::Y),
    ));

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

/// Le ciel vu depuis un système (ou depuis le centre de la carte, faute de système).
fn sky_for(galaxy: &Galaxy, system: Option<(&str, sh_core::MapPos)>) -> Image {
    let frame = &galaxy.frame;
    let observer = system.map_or([frame.center.0, frame.center.1 + 100.0], |(_, c)| {
        [c.x, c.y]
    });
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
        seed: system.map_or(0, |(id, _)| sh_core::rng::fnv1a(id.as_bytes())),
    };
    let start = std::time::Instant::now();
    let image = cube_image(&render_sky(&spec, SKY_SIZE));
    info!(
        "ciel généré en {:.0} ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
    image
}

fn label(fonts: &UiFonts, name: &str, target: Entity) -> impl Bundle {
    (
        SCOPE,
        WorldLabel::Follow(target),
        Text::new(name),
        fonts.body(14.0),
        TextColor(theme::TEXT),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
    )
}

/// Vitesse permise à la caméra : manœuvre, ou croisière (`Boost`) selon la distance à la
/// surface la plus proche (loi de l'échelle P2).
fn flight_speed(
    scale: &SystemScale,
    boost: bool,
    eye: SpacePos,
    bodies: &[(SpacePos, f64)],
) -> f64 {
    if !boost {
        return scale.manoeuvre_max_speed;
    }
    let nearest = bodies
        .iter()
        .map(|(pos, radius)| eye.distance(*pos) - radius)
        .fold(f64::MAX, f64::min);
    scale.cruise_speed_limit(nearest)
}

/// Vol libre de test : poussée 3 axes, souris pour orienter, Q/E pour le roulis.
#[allow(clippy::too_many_arguments)]
fn fly(
    time: Res<Time>,
    data: Res<Data>,
    origin: Res<FloatingOrigin>,
    thrust: Single<&Action<Thrust>>,
    steer: Single<&Action<Steer>>,
    roll: Single<&Action<Roll>>,
    boost: Single<&Action<Boost>>,
    bodies: Query<(&SpacePosition, &Body)>,
    mut camera: Single<&mut Transform, With<WorldCamera>>,
) {
    let dt = time.delta_secs();
    let steer: Vec2 = ***steer;
    let roll: f32 = ***roll;
    let rotation = Quat::from_euler(
        EulerRot::YXZ,
        -steer.x * 0.002,
        -steer.y * 0.002,
        -roll * 1.2 * dt,
    );
    camera.rotation *= rotation;
    let bodies: Vec<(SpacePos, f64)> = bodies.iter().map(|(p, b)| (p.0, b.radius)).collect();
    let eye = origin.to_space(camera.translation);
    let speed = flight_speed(&data.space.scale, ***boost, eye, &bodies) as f32;
    let thrust: Vec3 = ***thrust;
    let movement = camera.rotation * thrust * speed * dt;
    camera.translation += movement;
}

/// Les figurants avancent sur leur trajet, et repartent en sens inverse au bout.
fn move_traffic(
    time: Res<Time>,
    mut shuttles: Query<(&mut Shuttle, &mut SpacePosition, &mut Transform)>,
) {
    let dt = f64::from(time.delta_secs());
    for (mut shuttle, mut pos, mut transform) in &mut shuttles {
        let length = shuttle.from.distance(shuttle.to).max(1.0);
        shuttle.progress += shuttle.speed * dt / length;
        if shuttle.progress >= 1.0 {
            let (from, to) = (shuttle.from, shuttle.to);
            shuttle.from = to;
            shuttle.to = from;
            shuttle.progress -= 1.0;
            transform.look_to(dir(from - to), Vec3::Y);
        }
        pos.0 = shuttle.from + (shuttle.to - shuttle.from) * shuttle.progress;
    }
}

/// La lumière vient de l'étoile, vers la caméra.
fn aim_sunlight(
    star: Query<&SpacePosition, With<Star>>,
    camera: Query<&SpacePosition, With<OriginFocus>>,
    mut light: Query<&mut Transform, With<Sunlight>>,
) {
    let (Ok(star), Ok(eye), Ok(mut light)) = (star.single(), camera.single(), light.single_mut())
    else {
        return;
    };
    let direction = dir(eye.0 - star.0);
    if light.forward().dot(direction) < 0.999_999 {
        light.look_to(direction, Vec3::Y);
    }
}
