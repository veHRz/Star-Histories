//! Amarrage et départ : les transitions entre la vue Globale (le vaisseau en vol) et la vue
//! Sectorielle (le vaisseau amarré à un quartier), AGENTS.md §6.3.
//!
//! - **Amarrage** : aux commandes, à portée d'une balise d'approche, `G` demande l'amarrage. Le
//!   pilote automatique conduit le vaisseau vers la balise pendant que le quartier se charge en
//!   arrière-plan ; un fondu au noir bref couvre le changement de vue ; le joueur reprend sa
//!   place aux commandes, le vaisseau posé dans le berceau du hangar (M9), sa coque autour de
//!   lui ; on en sort par le sas puis la passerelle.
//! - **Départ** : à quai, assis aux commandes, `G` : fondu, et le vaisseau repart de la balise
//!   du port où il s'était amarré (`Docked::port` : un spatioport par défaut sert plusieurs
//!   lieux).
//!
//! Le fondu est le seul « écran de chargement », et il est court : les modèles du quartier (et la
//! coque) sont préchargés pendant l'approche, et l'intérieur du vaisseau n'est jamais déchargé du
//! cache. Il ne se lève que lorsque la cellule du joueur et ses voisines Locales sont prêtes
//! (`CellStates::arrival_ready`) : on n'arrive jamais dans des pièces vides.
//! Un voyage interstellaire, plus long, pose sur le fondu une carte de chargement au style de
//! l'écran de démarrage (`loading`).

use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::world_serialization::WorldAsset;
use bevy_enhanced_input::prelude::*;
use sh_core::SpacePos;

use crate::clock::{GameClock, format_date};
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
use crate::ui::theme::{self, UiFonts};
use crate::voyage::{Location, ResumeFlight, RoutePlan, ShipFuel};
use sh_core::Id;
use sh_data::{Place, StructureKind};
use sh_sim::travel::{TravelMode, TravelPlan};

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
            )
            .add_systems(
                Update,
                animate_travel_card.run_if(resource_exists::<Sequence>),
            );
    }
}

/// Durée de l'approche automatique avant le fondu (s).
const APPROACH_SECONDS: f32 = 5.0;
/// Vitesse d'approche maximale (m/s).
const APPROACH_SPEED: f64 = 400.0;
/// Durée d'un fondu (s).
const FADE_SECONDS: f32 = 0.5;
/// Durée minimale d'affichage de la carte de chargement d'un voyage (s).
const CARD_SECONDS: f32 = 1.6;
/// À l'arrivée par Portail, le vaisseau sort à cette distance du hub (m).
const PORTAL_EXIT_DISTANCE: f64 = 6000.0;
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
    /// Voyage interstellaire depuis la carte : fondu, carte de chargement, la date avance, et
    /// l'on arrive en vue Globale à destination.
    Travel {
        dest: Id<Place>,
        plan: TravelPlan,
        t: f32,
        card: Entity,
    },
    /// Le fondu se lève quand le joueur est à sa place (assis, si l'arrivée l'attend) et que sa
    /// cellule et ses voisines Locales sont prêtes.
    Arriving {
        keep: Vec<UntypedHandle>,
        card: Option<Entity>,
    },
}

/// Le voile noir des transitions : opacité courante et visée.
#[derive(Resource, Default)]
pub struct Fade {
    pub alpha: f32,
    pub target: f32,
}

#[derive(Component)]
struct FadeVeil;

/// La carte de chargement d'un voyage (sa vignette suit le fondu).
#[derive(Component)]
struct TravelCard;

/// Le curseur de la jauge de la carte de chargement.
#[derive(Component)]
struct TravelSweep;

/// Jauge de la carte de chargement : largeur de la piste et du curseur (mêmes valeurs que
/// l'écran de chargement du démarrage, `loading`).
const CARD_TRACK_WIDTH: f32 = 360.0;
const CARD_SWEEP_WIDTH: f32 = 72.0;

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
    mut cards: Query<&mut BackgroundGradient, With<TravelCard>>,
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
        for mut gradient in &mut cards {
            *gradient = theme::backdrop_gradient(alpha);
        }
    }
}

/// Le curseur de la carte de chargement va et vient sur sa piste (comme celui de `loading`).
fn animate_travel_card(
    time: Res<Time<Real>>,
    mut sweep: Query<&mut UiTransform, With<TravelSweep>>,
) {
    let phase = 0.5 - 0.5 * (time.elapsed_secs() * 2.2).cos();
    let x = phase * (CARD_TRACK_WIDTH - CARD_SWEEP_WIDTH);
    for mut transform in &mut sweep {
        transform.translation = Val2::px(x, 0.0);
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
            // Livrer un contrat ou accrocher une épave passe avant l'amarrage.
            if crate::career::deliver_or_tow(world, ship) {
                return;
            }
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
            // Préchargement du quartier et de la coque pendant l'approche (et maintien du
            // vaisseau en cache).
            let ship = world.resource::<ActiveSector>().sector.id.to_string();
            let mut keep = preload(world, &[beacon.sector.as_str(), ship.as_str()]);
            let hull = world
                .resource::<Data>()
                .start()
                .and_then(|s| s.hull.clone())
                .filter(|path| crate::data::assets_dir().join(path).is_file());
            if let Some(path) = hull {
                let scene: Handle<WorldAsset> = world
                    .resource::<AssetServer>()
                    .load(GltfAssetLabel::Scene(0).from_asset(path));
                keep.push(scene.untyped());
            }
            world.insert_resource(Sequence::Docking {
                t: 0.0,
                beacon: pos,
                sector: beacon.sector,
                keep,
            });
            // L'approche automatique remplace l'autopilote du joueur.
            world.insert_resource(crate::ship::AutopilotRequest(None));
            notice(world, "notice.dock.cleared");
        }
        Some(ViewLevel::Sector) => {
            let Some(docked) = world.get_resource::<Docked>().cloned() else {
                return;
            };
            // Le port où l'on s'est amarré, sinon celui de ce quartier dans le système où l'on
            // est (un spatioport par défaut sert plusieurs lieux).
            let found = departure_port(world.resource::<Data>(), &docked);
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

/// D'où repart le vaisseau amarré : la balise de son port et son système.
fn departure_port(data: &Data, docked: &Docked) -> Option<(String, String)> {
    let by_id = docked.port.as_deref().and_then(|port| {
        data.scenes.iter().find_map(|scene| {
            scene
                .docks
                .iter()
                .find(|d| d.id == port)
                .map(|d| (d.id.clone(), scene.system.to_string()))
        })
    });
    by_id.or_else(|| {
        sh_data::scene::find_dock(&data.scenes, &docked.district, docked.system.as_deref())
            .map(|(scene, d)| (d.id.clone(), scene.system.to_string()))
    })
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
    mut travel: (ResMut<GameClock>, Option<ResMut<ShipFuel>>),
    aboard: Option<Res<Aboard>>,
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
                if let Some(aboard) = crate::voyage::helm_aboard(&data) {
                    commands.insert_resource(PendingAboard(aboard));
                }
                place.0 = Some(target.clone());
                view.set(ViewLevel::Sector);
                let keep = std::mem::take(keep);
                *sequence = Sequence::Arriving { keep, card: None };
            }
        }
        Sequence::Departing { near, system, keep } => {
            if fade.alpha >= 0.999 {
                if let Some(aboard) = crate::voyage::helm_aboard(&data) {
                    commands.insert_resource(PendingAboard(aboard));
                }
                commands.insert_resource(ArrivalRequest {
                    near: near.clone(),
                    distance: DEPARTURE_DISTANCE,
                });
                place.0 = Some(system.clone());
                view.set(ViewLevel::Global);
                let keep = std::mem::take(keep);
                *sequence = Sequence::Arriving { keep, card: None };
            }
        }
        Sequence::Travel {
            dest,
            plan,
            t,
            card,
        } => {
            *t += time.delta_secs();
            fade.target = 1.0;
            if fade.alpha >= 0.999 && *t >= CARD_SECONDS {
                travel.0.advance_hours(plan.hours);
                if let Some(fuel) = travel.1.as_mut() {
                    fuel.units = (fuel.units - plan.fuel).max(0.0);
                }
                commands.insert_resource(Location {
                    system: dest.clone(),
                });
                commands.remove_resource::<ResumeFlight>();
                commands.remove_resource::<RoutePlan>();
                // Par Portail, on sort près du hub d'arrivée (mis en scène, ou le hub générique
                // d'un système sans scène, P32) ; par l'hyperespace, à l'arrivée habituelle du
                // système.
                let hub = sh_data::scene::system_structures(&data.galaxy, data.scene(dest), dest)
                    .into_iter()
                    .find(|st| st.kind == StructureKind::PortalHub);
                if plan.mode == TravelMode::Portal
                    && let Some(hub) = hub
                {
                    commands.insert_resource(ArrivalRequest {
                        near: hub.id.clone(),
                        distance: PORTAL_EXIT_DISTANCE,
                    });
                }
                if let Some(aboard) = crate::voyage::helm_aboard(&data) {
                    commands.insert_resource(PendingAboard(aboard));
                }
                place.0 = Some(dest.to_string());
                view.set(ViewLevel::Global);
                let card = Some(*card);
                *sequence = Sequence::Arriving {
                    keep: Vec::new(),
                    card,
                };
            }
        }
        Sequence::Arriving { card, .. } => {
            // Le voile se lève quand le joueur est à sa place (assis si l'on reprend un poste),
            // les cellules autour de lui prêtes.
            let wants_seat = aboard.as_ref().is_some_and(|a| a.seat_role.is_some());
            let seated = player
                .single()
                .is_ok_and(|p| matches!(p.mode, PlayerMode::Seated { .. }));
            if sector.is_some() && (seated || !wants_seat) && cells.arrival_ready(&ready) {
                fade.target = 0.0;
                if let Some(card) = card.take() {
                    commands.entity(card).despawn();
                }
                if fade.alpha <= 0.001 {
                    commands.remove_resource::<Sequence>();
                }
            }
        }
    }
}

/// Lance un voyage : la carte de chargement s'affiche sur le voile, puis la vue Globale de la
/// destination (voir [`Sequence::Travel`]).
pub fn start_travel(world: &mut World, dest: Id<Place>, plan: TravelPlan) {
    let data = world.resource::<Data>().clone();
    let texts = world.resource::<Texts>().clone();
    let fonts = world.resource::<UiFonts>().clone();
    // Un système ou, depuis M9, un monde isolé public (Q18) ; jamais un secret.
    let Some(place) = sh_data::travel::endpoint(&data.galaxy, &dest) else {
        return;
    };
    let mut arrival = world.resource::<GameClock>().clone();
    arrival.advance_hours(plan.hours);
    let mode = match plan.mode {
        TravelMode::Portal => texts.format(
            "travel.card.portal",
            &[
                ("hours", &format!("{:.0}", plan.hours)),
                ("jumps", &(plan.hubs.len().saturating_sub(1)).to_string()),
                ("toll", &(plan.toll.cents / 100).to_string()),
            ],
        ),
        TravelMode::Hyperspace => texts.format(
            "travel.card.hyperspace",
            &[("days", &format!("{:.0}", plan.days().ceil()))],
        ),
    };
    let controller = place
        .controller()
        .and_then(|f| data.faction(f))
        .map_or_else(|| texts.get("map.tip.no_controller"), |f| f.name.clone());
    let cell = place.cell().map_or_else(String::new, |c| c.to_string());
    let info = texts.format(
        "travel.card.info",
        &[("cell", &cell), ("controller", &controller)],
    );
    let tip = texts.get(&format!(
        "travel.tip.{}",
        arrival.date.day_index().rem_euclid(3)
    ));
    let arrival_text = texts.format(
        "travel.card.arrival",
        &[("date", &format_date(&texts, arrival.date))],
    );
    // L'écran de chargement du démarrage (`loading`) : équerres, nom en capitales sur un filet,
    // jauge à curseur ambre (la durée n'est pas mesurable), astuce en bas. La vignette suit le
    // fondu au noir (`animate_fade`) : la carte apparaît avec lui au lieu de couper la vue.
    let alpha = world.resource::<Fade>().alpha;
    let card = {
        let mut commands = world.commands();
        let mut card = commands.spawn((
            Name::new("Carte de chargement du voyage"),
            TravelCard,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            theme::backdrop_gradient(alpha),
            GlobalZIndex(i32::MAX),
            Pickable::IGNORE,
        ));
        card.with_children(|screen| {
            theme::spawn_corner_marks(screen, 28.0, 44.0, theme::SCREEN_DIM);
            screen.spawn(theme::heading(&fonts, place.name(), 44.0, theme::TEXT));
            screen.spawn(theme::rule(
                px(CARD_TRACK_WIDTH + 80.0),
                UiRect::new(px(0), px(0), px(14), px(22)),
            ));
            screen.spawn((Text::new(mode), fonts.body(20.0), TextColor(theme::ACCENT)));
            screen.spawn((
                Text::new(arrival_text),
                fonts.mono(17.0),
                TextColor(theme::TEXT),
                Node {
                    margin: UiRect::top(px(10)),
                    ..default()
                },
            ));
            screen.spawn((
                Text::new(info),
                fonts.body(16.0),
                TextColor(theme::TEXT_DIM),
                Node {
                    margin: UiRect::top(px(6)),
                    ..default()
                },
            ));
            // La piste et son curseur, qui glisse par `UiTransform` (pas de remise en page).
            screen.spawn((
                Node {
                    width: px(CARD_TRACK_WIDTH),
                    height: px(4),
                    margin: UiRect::vertical(px(22)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(theme::RULE),
                children![(
                    TravelSweep,
                    Node {
                        width: px(CARD_SWEEP_WIDTH),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(theme::ACCENT),
                    UiTransform::default(),
                )],
            ));
            screen.spawn((
                Text::new(tip),
                fonts.body(15.0),
                TextColor(theme::TEXT_DIM),
                TextLayout::justify(Justify::Center),
                Node {
                    max_width: px(720),
                    ..default()
                },
            ));
        });
        card.id()
    };
    world.flush();
    // L'autopilote vise une cible du système qu'on quitte.
    world.insert_resource(crate::ship::AutopilotRequest(None));
    world.insert_resource(Sequence::Travel {
        dest,
        plan,
        t: 0.0,
        card,
    });
    world.resource_mut::<Fade>().target = 1.0;
}
