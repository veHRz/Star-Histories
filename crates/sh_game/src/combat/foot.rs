//! Le combat à pied (M6, GDD §5.5) : armes portatives, armure, PNJ de combat.
//!
//! - **Projectiles** : de vraies balles (vitesse, portée) ; murs et portes les arrêtent (rayons
//!   avian sur les couches fixes), les corps sont des capsules. La protection du joueur absorbe
//!   ce qu'elle arrête (`sh_sim::combat::body_damage`). Peu de points de vie : chaque balle compte.
//! - **PNJ** ([`FighterAi`]) : calmes, puis en alerte (vue, bruit d'un tir, alerte d'un voisin).
//!   En combat, ils cherchent une **couverture** (un point où un obstacle les cache accroupis,
//!   d'où ils voient le joueur debout), s'y rendent par les portails du secteur (en ouvrant les
//!   portes), puis tirent par rafales en se relevant ; pris à revers, ils changent d'abri.
//! - Le joueur tire au clic gauche, recharge sur [`Reload`] ; le recul relève la visée.
//!
//! `SH_BOARDING=N` (autopilote) : N pirates abordent le vaisseau du joueur, par le couloir
//! arrière.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use bevy_enhanced_input::prelude::*;
use sh_core::SimRng;
use sh_data::combat::{BodyArmor, Firearm};
use sh_sim::combat::body_damage;

use crate::data::{Data, Texts};
use crate::input::{FireWeapon, Reload};
use crate::player::{Player, PlayerCamera, PlayerController, PlayerSystems};
use crate::sector::doors::Door;
use crate::sector::streaming::{CellReady, CellStates};
use crate::sector::{ActiveSector, Layer};
use crate::state::ControlMode;
use crate::ui::hud::Notice;
use crate::ui::theme::{self, UiFonts};

pub struct FootCombatPlugin;

impl Plugin for FootCombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Noise>()
            .init_resource::<FootFireIntent>()
            .add_systems(Startup, setup_bullet_assets)
            .add_systems(
                Update,
                (
                    arm_player,
                    spawn_boarders,
                    player_firearm,
                    held_weapon,
                    perceive,
                    decide,
                    walk,
                    crouch_models,
                    npc_fire,
                    move_bullets,
                    fallen,
                    recover,
                    foot_hud,
                )
                    .chain()
                    .after(PlayerSystems)
                    .run_if(resource_exists::<ActiveSector>),
            );
    }
}

/// Hauteurs d'un combattant (m) : debout, accroupi ; rayon du corps.
const STAND: f32 = 1.8;
const CROUCH: f32 = 1.1;
const RADIUS: f32 = 0.35;
/// Vitesse de course des PNJ (m/s).
const NPC_SPEED: f32 = 3.6;
/// Rayon d'alerte entre PNJ (m).
const SHOUT: f32 = 15.0;

/// Les armes et la santé du joueur à pied.
#[derive(Resource, Clone, Debug)]
pub struct PlayerArms {
    pub weapon: Firearm,
    pub armor: Option<BodyArmor>,
    pub health: f32,
    pub max_health: f32,
    pub mag: u32,
    pub reserve: u32,
    pub cooldown: f32,
    pub reload: f32,
    /// Secondes depuis le dernier impact subi.
    pub since_hit: f32,
}

/// Ce que l'autopilote demande à l'arme du joueur ce frame.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct FootFireIntent {
    pub fire: bool,
    pub reload: bool,
}

/// Un PNJ de combat.
#[derive(Component, Clone, Debug)]
pub struct Fighter {
    pub weapon: Firearm,
    pub armor: Option<BodyArmor>,
    pub health: f32,
    pub aim_error_deg: f32,
    pub reaction: f32,
    pub sight: f32,
    pub hearing: f32,
    pub mag: u32,
    pub cooldown: f32,
    pub reload: f32,
}

impl Fighter {
    pub fn alive(&self) -> bool {
        self.health > 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FootState {
    #[default]
    Calm,
    /// Le joueur est repéré (ou entendu) : on le cherche.
    Alert,
    /// Aux prises : couverture, tirs.
    Combat,
}

#[derive(Component, Clone, Debug, Default)]
pub struct FighterAi {
    pub state: FootState,
    /// Délai avant la première réaction (s).
    pub react: f32,
    /// Dernière position connue du joueur (pieds).
    pub last_seen: Option<Vec3>,
    pub cover: Option<Vec3>,
    /// Points de passage (portails) jusqu'à la destination.
    pub path: Vec<Vec3>,
    pub crouched: bool,
    /// Rafale en cours (s restantes) ; pause sinon.
    pub burst: f32,
    pub pause: f32,
    pub rethink: f32,
    pub sees: bool,
    pub velocity: Vec3,
}

/// Un PNJ tombé : il s'affaisse, puis reste au sol.
#[derive(Component)]
pub struct Fallen(pub f32);

/// Le modèle d'un PNJ (enfant), pour l'accroupi et la chute.
#[derive(Component)]
pub struct FighterModel;

/// Un bruit (tir) : les PNJ qui l'entendent passent en alerte.
#[derive(Message, Clone, Copy, Debug)]
pub struct Noise {
    pub at: Vec3,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Player,
    Npc(Entity),
}

#[derive(Component, Clone, Debug)]
pub struct Bullet {
    pub owner: Owner,
    pub velocity: Vec3,
    pub range: f32,
    pub damage: f32,
    pub penetration: f32,
}

#[derive(Resource, Clone)]
pub struct BulletAssets {
    mesh: Handle<Mesh>,
    player: Handle<StandardMaterial>,
    hostile: Handle<StandardMaterial>,
}

fn setup_bullet_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let glow = |r: f32, g: f32, b: f32| StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(r * 30.0, g * 30.0, b * 30.0),
        unlit: true,
        ..default()
    };
    commands.insert_resource(BulletAssets {
        mesh: meshes.add(Cuboid::new(0.025, 0.025, 0.9)),
        player: materials.add(glow(0.6, 0.85, 1.0)),
        hostile: materials.add(glow(1.0, 0.45, 0.15)),
    });
}

/// Le joueur à pied reçoit son arme et sa protection (une fois par partie).
fn arm_player(mut commands: Commands, data: Res<Data>, arms: Option<Res<PlayerArms>>) {
    if arms.is_some() {
        return;
    }
    let combat = &data.combat;
    let Some(def) = combat.fighters.get("player") else {
        return;
    };
    let Some(weapon) = combat.firearms.get(&def.weapon).cloned() else {
        return;
    };
    commands.insert_resource(PlayerArms {
        mag: weapon.magazine,
        reserve: weapon.reserve,
        armor: def
            .armor
            .as_ref()
            .and_then(|a| combat.armors.get(a).cloned()),
        health: def.health as f32,
        max_health: def.health as f32,
        cooldown: 0.0,
        reload: 0.0,
        since_hit: f32::MAX,
        weapon,
    });
}

/// `SH_BOARDING=N` : un groupe d'abordage dans le couloir arrière, dès qu'il a ses collisions.
#[allow(clippy::too_many_arguments)]
fn spawn_boarders(
    mut commands: Commands,
    data: Res<Data>,
    asset_server: Res<AssetServer>,
    sector: Res<ActiveSector>,
    states: Res<CellStates>,
    ready: Query<(), With<CellReady>>,
    mut done: Local<bool>,
    mut notices: MessageWriter<Notice>,
    texts: Res<Texts>,
) {
    if *done {
        return;
    }
    let Some(count) = std::env::var("SH_BOARDING")
        .ok()
        .and_then(|n| n.trim().parse::<usize>().ok())
    else {
        *done = true;
        return;
    };
    let Some(cell) = sector.sector.cell("couloir_arriere") else {
        *done = true;
        return;
    };
    if !states.is_local_ready(&cell.id, &ready) {
        return;
    }
    *done = true;
    let (min, max) = (Vec3::from(cell.min), Vec3::from(cell.max));
    for i in 0..count {
        let t = (i as f32 + 0.5) / count as f32;
        let at = Vec3::new(
            (min.x + max.x) / 2.0,
            min.y + 0.2,
            min.z + 1.0 + t * (max.z - min.z - 2.0),
        );
        spawn_fighter(
            &mut commands,
            &data,
            &asset_server,
            &sector,
            "pirate",
            at,
            FootState::Alert,
        );
    }
    notices.write(Notice(texts.get("combat.boarding")));
}

/// Fait apparaître un combattant (rôle de `combat.ron`) aux pieds `at`.
pub fn spawn_fighter(
    commands: &mut Commands,
    data: &Data,
    asset_server: &AssetServer,
    sector: &ActiveSector,
    role: &str,
    at: Vec3,
    state: FootState,
) -> Option<Entity> {
    let combat = &data.combat;
    let def = combat.fighters.get(role)?;
    let weapon = combat.firearms.get(&def.weapon)?.clone();
    let entity = commands
        .spawn((
            Name::new(format!("Combattant ({role})")),
            sector.scope(),
            Fighter {
                mag: weapon.magazine,
                armor: def
                    .armor
                    .as_ref()
                    .and_then(|a| combat.armors.get(a).cloned()),
                health: def.health as f32,
                aim_error_deg: def.aim_error_deg as f32,
                reaction: def.reaction_seconds as f32,
                sight: def.sight as f32,
                hearing: def.hearing as f32,
                cooldown: 0.0,
                reload: 0.0,
                weapon,
            },
            FighterAi {
                state,
                react: def.reaction_seconds as f32,
                // Le temps de se mettre en place avant la première rafale.
                pause: 1.0 + (at.x.abs() + at.z.abs()).fract(),
                ..default()
            },
            Transform::from_translation(at),
            Visibility::Inherited,
        ))
        .id();
    commands.spawn((
        ChildOf(entity),
        FighterModel,
        WorldAssetRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/characters/pirate.gltf")),
        ),
        Transform::default(),
    ));
    Some(entity)
}

/// Le filtre des rayons : géométrie fixe et portes.
fn walls() -> SpatialQueryFilter {
    SpatialQueryFilter::from_mask([Layer::Static, Layer::Door])
}

/// `a` voit-il `b` (aucun mur ni porte entre les deux) ?
fn clear_line(spatial: &SpatialQuery, a: Vec3, b: Vec3) -> bool {
    let d = b - a;
    let Ok(dir) = Dir3::new(d) else {
        return true;
    };
    spatial
        .cast_ray(a, dir, d.length(), true, &walls())
        .is_none()
}

/// Distance d'un segment à une capsule verticale (pieds `feet`, hauteur `height`).
fn segment_to_body(a: Vec3, b: Vec3, feet: Vec3, height: f32) -> (f32, f32) {
    let (c0, c1) = (feet + Vec3::Y * RADIUS, feet + Vec3::Y * (height - RADIUS));
    // Plus courte distance entre deux segments, comme en vue Globale (en f32 ici).
    let (d1, d2, r) = (b - a, c1 - c0, a - c0);
    let (aa, ee, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let c = d1.dot(r);
    let bb = d1.dot(d2);
    let denom = aa * ee - bb * bb;
    let mut s = if denom > 1e-9 {
        ((bb * f - c * ee) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut t = if ee > 1e-9 { (bb * s + f) / ee } else { 0.0 };
    if t < 0.0 {
        t = 0.0;
        s = if aa > 1e-9 {
            (-c / aa).clamp(0.0, 1.0)
        } else {
            0.0
        };
    } else if t > 1.0 {
        t = 1.0;
        s = if aa > 1e-9 {
            ((bb - c) / aa).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }
    ((a + d1 * s).distance(c0 + d2 * t), s)
}

fn spread_dir(rng: &mut SimRng, dir: Vec3, deg: f32) -> Vec3 {
    let dir = dir.normalize_or(Vec3::NEG_Z);
    let a = deg.to_radians() * rng.unit_f64() as f32;
    let phi = std::f32::consts::TAU * rng.unit_f64() as f32;
    let ortho = Quat::from_axis_angle(dir, phi) * dir.any_orthonormal_vector();
    (Quat::from_axis_angle(ortho, a) * dir).normalize()
}

fn spawn_bullet(
    commands: &mut Commands,
    assets: &BulletAssets,
    sector: &ActiveSector,
    owner: Owner,
    weapon: &Firearm,
    from: Vec3,
    dir: Vec3,
) {
    let material = if owner == Owner::Player {
        assets.player.clone()
    } else {
        assets.hostile.clone()
    };
    commands.spawn((
        Name::new("Balle"),
        sector.scope(),
        Bullet {
            owner,
            velocity: dir * weapon.speed as f32,
            range: weapon.range as f32,
            damage: weapon.damage as f32,
            penetration: weapon.penetration as f32,
        },
        Mesh3d(assets.mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(from).looking_to(dir, Vec3::Y),
    ));
}

/// Des hostiles debout dans le secteur : l'arme du joueur est en main.
fn hostiles_present(fighters: &Query<&Fighter>) -> bool {
    fighters.iter().any(Fighter::alive)
}

/// L'arme du joueur : tir, recul, rechargement.
#[allow(clippy::too_many_arguments)]
fn player_firearm(
    mut commands: Commands,
    time: Res<Time>,
    mode: Res<State<ControlMode>>,
    scripted: Res<crate::player::ScriptedControl>,
    mut intent: ResMut<FootFireIntent>,
    fire: Single<&Action<FireWeapon>>,
    reload: Single<&Action<Reload>>,
    arms: Option<ResMut<PlayerArms>>,
    assets: Res<BulletAssets>,
    sector: Res<ActiveSector>,
    fighters: Query<&Fighter>,
    camera: Single<&GlobalTransform, With<PlayerCamera>>,
    mut player: Single<&mut PlayerController, With<Player>>,
    mut rng: Local<Option<SimRng>>,
    mut noises: MessageWriter<Noise>,
) {
    let Some(mut arms) = arms else {
        return;
    };
    let dt = time.delta_secs();
    arms.cooldown = (arms.cooldown - dt).max(0.0);
    arms.since_hit += dt;
    let (want_fire, want_reload) = if scripted.0 {
        let i = std::mem::take(&mut *intent);
        (i.fire, i.reload)
    } else {
        let on_foot = *mode.get() == ControlMode::OnFoot;
        (on_foot && ***fire, on_foot && ***reload)
    };
    if arms.health <= 0.0 || !hostiles_present(&fighters) {
        return;
    }
    if arms.reload > 0.0 {
        arms.reload -= dt;
        if arms.reload <= 0.0 {
            let take = (arms.weapon.magazine - arms.mag).min(arms.reserve);
            arms.mag += take;
            arms.reserve -= take;
        }
        return;
    }
    if (want_reload || (want_fire && arms.mag == 0))
        && arms.mag < arms.weapon.magazine
        && arms.reserve > 0
    {
        arms.reload = arms.weapon.reload_seconds as f32;
        return;
    }
    if !want_fire || arms.cooldown > 0.0 || arms.mag == 0 {
        return;
    }
    arms.mag -= 1;
    arms.cooldown = 1.0 / arms.weapon.rate as f32;
    let rng = rng.get_or_insert_with(|| SimRng::new(0x00f0_07ed));
    let eye = camera.translation();
    let dir = spread_dir(
        rng,
        camera.forward().as_vec3(),
        arms.weapon.spread_deg as f32,
    );
    spawn_bullet(
        &mut commands,
        &assets,
        &sector,
        Owner::Player,
        &arms.weapon,
        eye + dir * 0.5,
        dir,
    );
    // Le recul relève la visée.
    player.pitch = (player.pitch + (arms.weapon.recoil_deg as f32).to_radians()).min(1.5);
    noises.write(Noise {
        at: eye,
        radius: 30.0,
    });
}

/// L'arme en main, vue à la première personne (enfant de la caméra).
#[derive(Component)]
struct HeldWeapon;

/// Position de repos de l'arme dans le repère de la caméra : en bas à droite, assez près pour
/// rester dans la capsule du joueur (rayon 0,3 m) et ne pas traverser les cloisons.
const HELD_REST: Vec3 = Vec3::new(0.15, -0.19, -0.28);

/// Montre l'arme du joueur quand elle est en main (des hostiles debout), avec un recul à
/// chaque coup et l'arme baissée pendant le rechargement.
fn held_weapon(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    arms: Option<Res<PlayerArms>>,
    fighters: Query<&Fighter>,
    camera: Single<Entity, With<PlayerCamera>>,
    mut held: Query<(Entity, &mut Transform), With<HeldWeapon>>,
) {
    let model = arms
        .as_ref()
        .filter(|a| a.health > 0.0 && hostiles_present(&fighters))
        .and_then(|a| a.weapon.model.clone());
    let Some(model) = model else {
        for (entity, _) in &held {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(arms) = arms else {
        return;
    };
    let Ok((_, mut transform)) = held.single_mut() else {
        commands.spawn((
            Name::new("Arme en main"),
            HeldWeapon,
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(model))),
            Transform::from_translation(HELD_REST),
            ChildOf(*camera),
        ));
        return;
    };
    // Recul : l'arme part en arrière et se relève juste après le coup, puis revient.
    let period = (1.0 / arms.weapon.rate as f32).max(0.05);
    let kick = (arms.cooldown / period).clamp(0.0, 1.0);
    let lowered = if arms.reload > 0.0 { 1.0 } else { 0.0 };
    transform.translation = HELD_REST + Vec3::new(0.0, -0.12 * lowered, 0.05 * kick);
    transform.rotation = Quat::from_rotation_x(0.18 * kick - 0.5 * lowered);
}

/// Perception : vue (ligne dégagée, portée) et ouïe (bruits) ; l'alerte se propage.
#[allow(clippy::too_many_arguments)]
fn perceive(
    time: Res<Time>,
    spatial: SpatialQuery,
    mut noises: MessageReader<Noise>,
    player: Single<(&GlobalTransform, &PlayerController), With<Player>>,
    mut npcs: Query<(Entity, &Fighter, &mut FighterAi, &Transform)>,
) {
    let dt = time.delta_secs();
    let (pt, controller) = *player;
    let feet = pt.translation();
    let eye = feet + Vec3::Y * controller.eye_height();
    let heard: Vec<Noise> = noises.read().copied().collect();
    let mut shouts = Vec::new();
    for (e, f, mut ai, t) in &mut npcs {
        if !f.alive() {
            continue;
        }
        let head = t.translation
            + Vec3::Y
                * if ai.crouched {
                    CROUCH - 0.15
                } else {
                    STAND - 0.15
                };
        ai.sees = head.distance(eye) < f.sight && clear_line(&spatial, head, eye);
        if ai.sees {
            ai.last_seen = Some(feet);
        }
        let hears = heard
            .iter()
            .any(|n| n.at.distance(head) < n.radius.min(f.hearing.max(1.0) * 1.5));
        if ai.state == FootState::Calm && (ai.sees || hears) {
            ai.state = FootState::Alert;
            ai.last_seen.get_or_insert(feet);
        }
        if ai.state != FootState::Calm {
            // En alerte sans contact (un abordage) : on sait à peu près où chercher.
            ai.last_seen.get_or_insert(feet);
            shouts.push((e, t.translation));
        }
        if ai.sees && ai.state == FootState::Alert {
            ai.react -= dt;
            if ai.react <= 0.0 {
                ai.state = FootState::Combat;
            }
        }
    }
    // Un PNJ en alerte prévient ses voisins.
    for (e, _, mut ai, t) in &mut npcs {
        if ai.state == FootState::Calm
            && shouts
                .iter()
                .any(|(o, p)| *o != e && p.distance(t.translation) < SHOUT)
        {
            ai.state = FootState::Alert;
            ai.last_seen.get_or_insert(feet);
        }
    }
}

/// La cellule du secteur qui contient un point.
fn cell_at(sector: &sh_data::Sector, p: Vec3) -> Option<&sh_data::sector::Cell> {
    sector.cells.iter().find(|c| {
        let (min, max) = (Vec3::from(c.min), Vec3::from(c.max));
        p.x >= min.x
            && p.x <= max.x
            && p.z >= min.z
            && p.z <= max.z
            && p.y >= min.y - 0.5
            && p.y <= max.y
    })
}

/// Le chemin de `from` à `to` : les centres des portails à franchir (recherche en largeur sur
/// le graphe des cellules), puis la destination.
fn route(sector: &sh_data::Sector, from: Vec3, to: Vec3) -> Vec<Vec3> {
    let (Some(a), Some(b)) = (cell_at(sector, from), cell_at(sector, to)) else {
        return vec![to];
    };
    if a.id == b.id {
        return vec![to];
    }
    let mut prev: std::collections::BTreeMap<String, (String, usize)> = Default::default();
    let mut frontier = std::collections::VecDeque::from([a.id.clone()]);
    let mut seen = std::collections::BTreeSet::from([a.id.clone()]);
    while let Some(cell) = frontier.pop_front() {
        if cell == b.id {
            break;
        }
        for (i, p) in sector.portals.iter().enumerate() {
            let Some(next) = p.other(&cell) else {
                continue;
            };
            if next.starts_with('@') || !seen.insert(next.to_owned()) {
                continue;
            }
            prev.insert(next.to_owned(), (cell.clone(), i));
            frontier.push_back(next.to_owned());
        }
    }
    let mut path = vec![to];
    let mut at = b.id.clone();
    while let Some((back, portal)) = prev.get(&at).cloned() {
        let p = &sector.portals[portal];
        path.push(Vec3::new(p.center.0, from.y, p.center.2));
        at = back;
    }
    path.reverse();
    path
}

/// Le point de couverture le plus intéressant autour d'un PNJ, contre un joueur dont l'œil est
/// en `eye`.
fn find_cover(
    spatial: &SpatialQuery,
    sector: &sh_data::Sector,
    me: Vec3,
    eye: Vec3,
    taken: &[Vec3],
) -> Option<Vec3> {
    let here = cell_at(sector, me)?;
    let mut cells = vec![here];
    cells.extend(sector.neighbors(&here.id).filter_map(|id| sector.cell(id)));
    let free = |p: Vec3| {
        [Vec3::ZERO, Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
            .iter()
            .all(|o| {
                [0.4, 1.2].iter().all(|h| {
                    spatial
                        .point_intersections(p + *o * RADIUS + Vec3::Y * *h, &walls())
                        .is_empty()
                })
            })
    };
    let mut best: Option<(f32, Vec3)> = None;
    for cell in cells {
        let (min, max) = (Vec3::from(cell.min), Vec3::from(cell.max));
        let mut x = min.x + 0.6;
        while x < max.x - 0.5 {
            let mut z = min.z + 0.6;
            while z < max.z - 0.5 {
                let p = Vec3::new(x, me.y, z);
                z += 0.7;
                if taken.iter().any(|t| t.distance(p) < 1.5) {
                    continue;
                }
                let low = p + Vec3::Y * (CROUCH - 0.2);
                let high = p + Vec3::Y * (STAND - 0.15);
                let to_eye = eye - low;
                // Caché accroupi : un obstacle tout proche entre lui et le joueur.
                let hidden = Dir3::new(to_eye)
                    .ok()
                    .and_then(|d| spatial.cast_ray(low, d, to_eye.length(), true, &walls()));
                let Some(hit) = hidden else {
                    continue;
                };
                if hit.distance > 2.0 || !clear_line(spatial, high, eye) || !free(p) {
                    continue;
                }
                let score = p.distance(me) + (p.distance(eye) - 7.0).abs() * 0.5;
                if best.is_none_or(|(s, _)| score < s) {
                    best = Some((score, p));
                }
            }
            x += 0.7;
        }
    }
    best.map(|(_, p)| p)
}

/// Décisions : où aller (couverture, dernier contact), quand repenser.
fn decide(
    time: Res<Time>,
    spatial: SpatialQuery,
    sector: Res<ActiveSector>,
    player: Single<(&GlobalTransform, &PlayerController), With<Player>>,
    mut npcs: Query<(Entity, &Fighter, &mut FighterAi, &Transform)>,
) {
    let dt = time.delta_secs();
    let (pt, controller) = *player;
    let eye = pt.translation() + Vec3::Y * controller.eye_height();
    let taken: Vec<(Entity, Vec3)> = npcs
        .iter()
        .filter_map(|(e, _, ai, _)| ai.cover.map(|c| (e, c)))
        .collect();
    for (e, f, mut ai, t) in &mut npcs {
        if !f.alive() || ai.state == FootState::Calm {
            continue;
        }
        ai.rethink -= dt;
        let me = t.translation;
        // À couvert mais vu accroupi (pris à revers) : l'abri ne vaut plus rien.
        let flanked = ai.cover.is_some_and(|c| {
            c.distance(me) < 0.6 && clear_line(&spatial, c + Vec3::Y * (CROUCH - 0.2), eye)
        });
        if ai.rethink > 0.0 && !flanked {
            continue;
        }
        ai.rethink = 3.0;
        let others: Vec<Vec3> = taken
            .iter()
            .filter(|(o, _)| *o != e)
            .map(|(_, c)| *c)
            .collect();
        if ai.state == FootState::Combat || ai.sees {
            ai.cover = find_cover(&spatial, &sector.sector, me, eye, &others);
            ai.path = ai
                .cover
                .map(|c| route(&sector.sector, me, c))
                .unwrap_or_default();
        } else if let Some(seen) = ai.last_seen {
            // En alerte sans le voir : aller au dernier contact.
            ai.path = route(&sector.sector, me, seen);
        }
    }
}

/// Déplacements : « move and slide » le long des points de passage ; les portes s'ouvrent.
fn walk(
    time: Res<Time>,
    move_and_slide: MoveAndSlide,
    player: Single<&GlobalTransform, With<Player>>,
    mut doors: Query<(&GlobalTransform, &mut Door)>,
    mut npcs: Query<(&Fighter, &mut FighterAi, &mut Transform), Without<FighterModel>>,
) {
    let dt = time.delta_secs().min(0.05);
    let target = player.translation();
    for (f, mut ai, mut t) in &mut npcs {
        if !f.alive() {
            continue;
        }
        let me = t.translation;
        // Ouvrir la porte d'à côté.
        for (dt_, mut door) in &mut doors {
            // `request_open` ne fait rien sur une porte déjà ouverte ou qui s'ouvre.
            if dt_.translation().distance(me + Vec3::Y) < 1.6 && !ai.path.is_empty() {
                door.request_open();
            }
        }
        while ai
            .path
            .first()
            .is_some_and(|p| Vec2::new(p.x - me.x, p.z - me.z).length() < 0.25)
        {
            ai.path.remove(0);
        }
        let wish = match ai.path.first() {
            Some(p) => {
                let d = Vec3::new(p.x - me.x, 0.0, p.z - me.z);
                d.normalize_or_zero() * NPC_SPEED
            }
            None => Vec3::ZERO,
        };
        ai.velocity = ai.velocity.move_towards(wish, 20.0 * dt);
        let height = if ai.crouched { CROUCH } else { STAND };
        let shape = Collider::capsule(RADIUS, height - 2.0 * RADIUS);
        let center = me + Vec3::Y * height / 2.0;
        let out = move_and_slide.move_and_slide(
            &shape,
            center,
            Quat::IDENTITY,
            ai.velocity,
            time.delta().min(std::time::Duration::from_millis(50)),
            &MoveAndSlideConfig::default(),
            &walls(),
            |_| MoveAndSlideHitResponse::Accept,
        );
        t.translation = out.position - Vec3::Y * height / 2.0;
        // Face au joueur en combat, sinon dans le sens de la marche.
        let look = if ai.state == FootState::Combat || ai.sees {
            target - t.translation
        } else {
            ai.velocity
        };
        let flat = Vec3::new(look.x, 0.0, look.z);
        if flat.length_squared() > 1e-4 {
            // L'avant du modèle est +Y dans Blender, −Z dans le jeu.
            t.rotation = Transform::default().looking_to(flat, Vec3::Y).rotation;
        }
    }
}

/// L'accroupi se lit sur le modèle (pas de squelette en M6) : il se tasse.
fn crouch_models(
    npcs: Query<(&FighterAi, &Fighter)>,
    mut models: Query<(&ChildOf, &mut Transform), With<FighterModel>>,
) {
    for (parent, mut t) in &mut models {
        let Ok((ai, f)) = npcs.get(parent.parent()) else {
            continue;
        };
        if !f.alive() {
            continue;
        }
        let wanted = if ai.crouched { CROUCH / STAND } else { 1.0 };
        if (t.scale.y - wanted).abs() > 1e-3 {
            t.scale.y = wanted;
        }
    }
}

/// Les tirs des PNJ : à couvert, ils se relèvent pour une rafale, puis se rabaissent.
#[allow(clippy::too_many_arguments)]
fn npc_fire(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<BulletAssets>,
    sector: Res<ActiveSector>,
    player: Single<(&GlobalTransform, &PlayerController), With<Player>>,
    mut npcs: Query<(Entity, &mut Fighter, &mut FighterAi, &Transform)>,
    mut rng: Local<Option<SimRng>>,
    mut noises: MessageWriter<Noise>,
) {
    let dt = time.delta_secs();
    let rng = rng.get_or_insert_with(|| SimRng::new(0x0bad_f00d));
    let (pt, controller) = *player;
    // On vise le torse.
    let aim_at = pt.translation() + Vec3::Y * controller.eye_height() * 0.8;
    for (e, mut f, mut ai, t) in &mut npcs {
        if !f.alive() || ai.state != FootState::Combat {
            continue;
        }
        f.cooldown = (f.cooldown - dt).max(0.0);
        if f.reload > 0.0 {
            f.reload -= dt;
            ai.crouched = ai.cover.is_some();
            if f.reload <= 0.0 {
                f.mag = f.weapon.magazine;
            }
            continue;
        }
        let in_cover = ai.cover.is_some_and(|c| c.distance(t.translation) < 0.6);
        // Rafale, puis pause (à couvert : accroupi pendant la pause).
        if ai.burst > 0.0 {
            ai.burst -= dt;
            ai.crouched = false;
        } else {
            ai.pause -= dt;
            ai.crouched = in_cover;
            if ai.pause <= 0.0 {
                ai.burst = 0.5 + rng.unit_f64() as f32 * 0.4;
                ai.pause = 1.5 + rng.unit_f64() as f32 * 1.0;
            }
        }
        // En marche vers son abri, on tire quand même sur ce qu'on voit, moins juste.
        let moving = !ai.path.is_empty() && !in_cover;
        if ai.burst <= 0.0 || !ai.sees || f.cooldown > 0.0 {
            continue;
        }
        if f.mag == 0 {
            f.reload = f.weapon.reload_seconds as f32;
            continue;
        }
        f.mag -= 1;
        f.cooldown = 1.0 / f.weapon.rate as f32 * if moving { 2.0 } else { 1.0 };
        let muzzle = t.translation + Vec3::Y * (STAND - 0.45) + (t.rotation * Vec3::NEG_Z) * 0.6;
        let error = f.aim_error_deg * if moving { 3.0 } else { 1.0 } + f.weapon.spread_deg as f32;
        let dir = spread_dir(rng, aim_at - muzzle, error);
        spawn_bullet(
            &mut commands,
            &assets,
            &sector,
            Owner::Npc(e),
            &f.weapon,
            muzzle,
            dir,
        );
        noises.write(Noise {
            at: muzzle,
            radius: 30.0,
        });
    }
}

/// Les balles avancent ; un mur les arrête, un corps les reçoit.
#[allow(clippy::too_many_arguments)]
fn move_bullets(
    mut commands: Commands,
    time: Res<Time>,
    spatial: SpatialQuery,
    texts: Res<Texts>,
    mut bullets: Query<(Entity, &mut Bullet, &mut Transform), Without<Fighter>>,
    player: Single<(&GlobalTransform, &PlayerController), With<Player>>,
    mut arms: Option<ResMut<PlayerArms>>,
    mut npcs: Query<(Entity, &mut Fighter, &mut FighterAi, &Transform), Without<Bullet>>,
    mut notices: MessageWriter<Notice>,
    mut death: Option<Res<crate::combat::Death>>,
) {
    let dt = time.delta_secs().min(0.05);
    let (pt, controller) = *player;
    let player_feet = pt.translation();
    let player_height = controller.height;
    for (e, mut b, mut t) in &mut bullets {
        let a = t.translation;
        let step = b.velocity * dt;
        let next = a + step;
        b.range -= step.length();
        // Le mur le plus proche sur ce trajet.
        let wall = Dir3::new(step)
            .ok()
            .and_then(|d| spatial.cast_ray(a, d, step.length(), true, &walls()))
            .map_or(1.0, |h| h.distance / step.length().max(1e-6));
        // Le corps le plus proche avant le mur.
        let mut hit: Option<(f32, Option<Entity>)> = None;
        if b.owner != Owner::Player {
            let (d, s) = segment_to_body(a, next, player_feet, player_height);
            if d < RADIUS && s <= wall {
                hit = Some((s, None));
            }
        }
        for (ne, f, ai, nt) in &npcs {
            // Les balles des PNJ épargnent leurs alliés (ils ne tirent pas à travers eux).
            if !f.alive() || b.owner != Owner::Player {
                continue;
            }
            let h = if ai.crouched { CROUCH } else { STAND };
            let (d, s) = segment_to_body(a, next, nt.translation, h);
            if d < RADIUS && s <= wall && hit.is_none_or(|(hs, _)| s < hs) {
                hit = Some((s, Some(ne)));
            }
        }
        match hit {
            Some((_, None)) => {
                if let Some(arms) = arms.as_mut() {
                    let armor = arms.armor.clone();
                    let damage = body_damage(
                        armor.as_ref(),
                        f64::from(b.damage),
                        f64::from(b.penetration),
                    ) as f32;
                    arms.health = (arms.health - damage).max(0.0);
                    arms.since_hit = 0.0;
                    debug!("joueur touché : {damage:.0}, santé {:.0}", arms.health);
                    if arms.health <= 0.0 && death.is_none() {
                        commands.insert_resource(crate::combat::Death { timer: 0.0 });
                        death = None;
                        notices.write(Notice(texts.get("combat.foot.dead")));
                    }
                }
                commands.entity(e).despawn();
            }
            Some((_, Some(ne))) => {
                if let Ok((_, mut f, mut ai, _)) = npcs.get_mut(ne) {
                    let armor = f.armor.clone();
                    let damage = body_damage(
                        armor.as_ref(),
                        f64::from(b.damage),
                        f64::from(b.penetration),
                    ) as f32;
                    f.health -= damage;
                    debug!("pirate touché : {damage:.0}, santé {:.0}", f.health);
                    // Touché : il sait d'où ça vient.
                    ai.state = FootState::Combat;
                    ai.last_seen = Some(player_feet);
                    if !f.alive() {
                        commands.entity(ne).insert(Fallen(0.0));
                    }
                }
                commands.entity(e).despawn();
            }
            None if wall < 1.0 || b.range <= 0.0 => {
                commands.entity(e).despawn();
            }
            None => {
                t.translation = next;
            }
        }
    }
}

/// Un PNJ tombé s'affaisse.
fn fallen(
    time: Res<Time>,
    mut npcs: Query<(&mut Fallen, &Children)>,
    mut models: Query<&mut Transform, With<FighterModel>>,
) {
    for (mut fallen, children) in &mut npcs {
        if fallen.0 >= 1.0 {
            continue;
        }
        fallen.0 = (fallen.0 + time.delta_secs() * 2.5).min(1.0);
        for child in children.iter() {
            if let Ok(mut t) = models.get_mut(child) {
                t.scale = Vec3::ONE;
                t.rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2 * fallen.0);
                t.translation = Vec3::Y * 0.2 * fallen.0;
            }
        }
    }
}

/// Le calme revenu (plus d'hostile debout), le joueur récupère ses forces.
fn recover(time: Res<Time>, fighters: Query<&Fighter>, arms: Option<ResMut<PlayerArms>>) {
    let Some(mut arms) = arms else {
        return;
    };
    if arms.health <= 0.0 || hostiles_present(&fighters) || arms.health >= arms.max_health {
        return;
    }
    arms.health = (arms.health + 5.0 * time.delta_secs()).min(arms.max_health);
}

#[derive(Component)]
struct FootHud;

/// Santé et munitions, quand ça compte (hostiles présents ou blessure).
fn foot_hud(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    sector: Res<ActiveSector>,
    arms: Option<Res<PlayerArms>>,
    fighters: Query<&Fighter>,
    mut hud: Query<(&mut Text, &mut Visibility), With<FootHud>>,
) {
    let Some(arms) = arms else {
        return;
    };
    if hud.is_empty() {
        commands.spawn((
            Name::new("Santé et munitions"),
            FootHud,
            sector.scope(),
            Text::new(""),
            fonts.body(16.0),
            TextColor(theme::TEXT),
            TextShadow::default(),
            Node {
                position_type: PositionType::Absolute,
                right: px(24),
                bottom: px(28),
                ..default()
            },
            Visibility::Hidden,
        ));
        return;
    }
    let shown = hostiles_present(&fighters) || arms.health < arms.max_health;
    let line = texts.format(
        if arms.reload > 0.0 {
            "combat.foot.hud_reload"
        } else {
            "combat.foot.hud"
        },
        &[
            ("health", &texts.number(f64::from(arms.health.max(0.0)), 0)),
            ("weapon", &texts.get(&arms.weapon.name_key)),
            ("mag", &arms.mag.to_string()),
            ("reserve", &arms.reserve.to_string()),
        ],
    );
    for (mut text, mut v) in &mut hud {
        let wanted = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != wanted {
            *v = wanted;
        }
        if text.0 != line {
            text.0.clone_from(&line);
        }
    }
}
