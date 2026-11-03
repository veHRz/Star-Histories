//! Le combat spatial en vue Globale (M6) : coques en combat, tirs, missiles, contre-mesures,
//! IA des pilotes, impacts et explosions.
//!
//! Tout vit en positions absolues `f64` ([`SpacePosition`]) ; le rendu suit l'origine
//! flottante. Les dégâts passent par le modèle pur (`sh_sim::combat::Hull`) : section frappée
//! selon le point d'impact, blindage, structure, cœur. Le vaisseau du joueur est un combattant
//! comme les autres, dont la coque est celle de sa fiche (les dégâts persistent).

use bevy::math::{DQuat, DVec3};
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use bevy_enhanced_input::prelude::*;
use sh_core::{SimRng, SpacePos};
use sh_data::combat::{ShipCombat, ShipWeapon};
use sh_data::{SectionId, WeaponKind};
use sh_sim::combat::{Hull, section_at};

use crate::career::Career;
use crate::data::{Data, Texts};
use crate::input::{Countermeasure, FireGuns, FireMissile};
use crate::sector::ActiveSector;
use crate::ship::ShipMotion;
use crate::space::{SpacePosition, SpaceRotation};
use crate::state::{ControlMode, ViewLevel};
use crate::ui::theme::UiFonts;
use crate::views::WorldLabel;

const SCOPE: DespawnOnExit<ViewLevel> = DespawnOnExit(ViewLevel::Global);

/// Le camp d'un combattant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Player,
    Raiders,
    Defenders,
}

impl Side {
    pub fn hostile(self, other: Side) -> bool {
        matches!(
            (self, other),
            (Side::Raiders, Side::Player | Side::Defenders)
                | (Side::Player | Side::Defenders, Side::Raiders)
        )
    }
}

/// Une arme montée et son état.
#[derive(Clone, Debug)]
pub struct WeaponState {
    pub key: String,
    pub def: ShipWeapon,
    pub cooldown: f64,
    /// Munitions restantes ; `None` : illimitées.
    pub ammo: Option<u32>,
}

/// Une coque en combat.
#[derive(Component, Clone, Debug)]
pub struct Combatant {
    pub side: Side,
    pub ship: String,
    pub def: ShipCombat,
    pub hull: Hull,
    pub weapons: Vec<WeaponState>,
    pub countermeasures: u32,
    pub cm_cooldown: f64,
    /// Secondes depuis le dernier impact subi (alarmes, IA).
    pub since_hit: f64,
}

impl Combatant {
    pub fn new(side: Side, ship: &str, def: &ShipCombat, hull: Hull, data: &Data) -> Self {
        let weapons = def
            .weapons
            .iter()
            .filter_map(|m| {
                data.combat
                    .ship_weapons
                    .get(&m.weapon)
                    .map(|w| WeaponState {
                        key: m.weapon.clone(),
                        def: w.clone(),
                        cooldown: 0.0,
                        ammo: (m.ammo > 0).then_some(m.ammo),
                    })
            })
            .collect();
        Self {
            side,
            ship: ship.to_owned(),
            def: def.clone(),
            hull,
            weapons,
            countermeasures: def.countermeasures,
            cm_cooldown: 0.0,
            since_hit: f64::MAX,
        }
    }

    fn weapon_mut(&mut self, kind: WeaponKind) -> Option<&mut WeaponState> {
        self.weapons
            .iter_mut()
            .find(|w| w.def.kind == kind && w.ammo != Some(0))
    }

    pub fn ammo(&self, kind: WeaponKind) -> u32 {
        self.weapons
            .iter()
            .filter(|w| w.def.kind == kind)
            .map(|w| w.ammo.unwrap_or(0))
            .sum()
    }
}

/// Le pilote d'une coque non jouée.
#[derive(Component, Clone, Debug, Default)]
pub struct AiPilot {
    pub velocity: DVec3,
    pub target: Option<Entity>,
    pub mode: AiMode,
    /// Secondes avant de repenser la cible et la manœuvre.
    pub think: f64,
    /// Côté du dégagement (±1), pour ne pas foncer dans la cible.
    pub break_side: f64,
    /// Secondes avant d'entrer en jeu (rescousse).
    pub delay: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AiMode {
    #[default]
    Attack,
    /// Dégagement après une passe trop proche.
    Break,
    /// Fuite : coque trop abîmée (pillards).
    Flee,
}

/// Un projectile : obus, missile ou torpille.
#[derive(Component, Clone, Debug)]
pub struct Shot {
    pub owner: Entity,
    pub side: Side,
    pub def: ShipWeapon,
    pub velocity: DVec3,
    pub life: f64,
    /// Guidage (missiles, torpilles) ; perdu s'il est leurré.
    pub target: Option<Entity>,
}

/// Éclair d'une explosion ou d'un impact.
#[derive(Component)]
pub struct Flash {
    pub life: f32,
    pub max: f32,
    pub size: f32,
}

/// Une coque vient d'être perdue.
#[derive(Message, Clone, Copy, Debug)]
pub struct ShipLost {
    pub entity: Entity,
    pub side: Side,
}

/// Impacts subis par le vaisseau du joueur ce frame (effets à bord).
#[derive(Message, Clone, Copy, Debug)]
pub struct PlayerHit {
    pub section: SectionId,
    pub penetrated: bool,
    pub breached: bool,
    pub damage: f64,
}

/// La cible verrouillée par le joueur (la plus proche du réticule, à portée).
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct PlayerTarget(pub Option<Entity>);

/// Les maillages et matériaux des tirs.
#[derive(Resource, Clone)]
pub struct CombatAssets {
    tracer: Handle<Mesh>,
    missile: Handle<Mesh>,
    flash: Handle<Mesh>,
    tracer_hostile: Handle<StandardMaterial>,
    tracer_friendly: Handle<StandardMaterial>,
    missile_mat: Handle<StandardMaterial>,
    flash_mat: Handle<StandardMaterial>,
}

pub fn setup_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let glow = |r: f32, g: f32, b: f32, k: f32| StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(r * k, g * k, b * k),
        unlit: true,
        ..default()
    };
    commands.insert_resource(CombatAssets {
        // Long et fin, orienté vers −Z : un obus est un trait de lumière à ces vitesses.
        tracer: meshes.add(Cuboid::new(0.8, 0.8, 40.0)),
        missile: meshes.add(Cuboid::new(1.6, 1.6, 6.0)),
        flash: meshes.add(Sphere::new(1.0).mesh().uv(12, 6)),
        tracer_hostile: materials.add(glow(1.0, 0.35, 0.15, 60.0)),
        tracer_friendly: materials.add(glow(0.5, 0.8, 1.0, 60.0)),
        missile_mat: materials.add(glow(1.0, 0.85, 0.5, 80.0)),
        flash_mat: materials.add(glow(1.0, 0.6, 0.25, 40.0)),
    });
}

/// Fait apparaître une coque de combat non jouée.
#[allow(clippy::too_many_arguments)]
pub fn spawn_ship(
    commands: &mut Commands,
    asset_server: &AssetServer,
    data: &Data,
    texts: &Texts,
    fonts: &UiFonts,
    side: Side,
    key: &str,
    pos: SpacePos,
    facing: DVec3,
    delay: f64,
    break_side: f64,
) -> Option<Entity> {
    let def = data.combat.ship(key)?;
    let rotation = look_rotation(facing);
    // Des équipages qui ne tirent pas tous à la même seconde : délais initiaux tirés de la
    // position d'apparition (déterministe).
    let mut rng = SimRng::new(pos.x.to_bits() ^ pos.z.to_bits().rotate_left(17));
    let mut combatant = Combatant::new(side, key, def, Hull::new(def, data.combat.hull), data);
    for w in &mut combatant.weapons {
        let max = match w.def.kind {
            WeaponKind::Cannon => 2.0,
            _ => 12.0,
        };
        w.cooldown = rng.unit_f64() * max + delay;
    }
    let name = texts.get(&def.name_key);
    let entity = commands
        .spawn((
            Name::new(name.clone()),
            SCOPE,
            combatant,
            AiPilot {
                velocity: facing.normalize_or_zero() * def.max_speed * 0.6,
                break_side,
                delay,
                ..default()
            },
            SpacePosition(pos),
            SpaceRotation(rotation.as_quat()),
            WorldAssetRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(def.model.clone())),
            ),
            if delay > 0.0 {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            },
        ))
        .id();
    let color = match side {
        Side::Raiders => Color::srgb(1.0, 0.35, 0.25),
        _ => Color::srgb(0.55, 0.8, 1.0),
    };
    commands.spawn((
        SCOPE,
        ShipMarker(entity),
        WorldLabel::Follow(entity),
        Text::new(name),
        fonts.body(13.0),
        TextColor(color),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        if delay > 0.0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        },
    ));
    Some(entity)
}

/// L'étiquette d'une coque (nom, distance, état).
#[derive(Component)]
pub struct ShipMarker(pub Entity);

/// L'orientation qui regarde dans `dir` (avant −Z, haut +Y).
pub fn look_rotation(dir: DVec3) -> DQuat {
    let f = dir.normalize_or(DVec3::NEG_Z);
    let up = if f.y.abs() > 0.99 { DVec3::Z } else { DVec3::Y };
    let right = f.cross(up).normalize();
    let up = right.cross(f);
    DQuat::from_mat3(&bevy::math::DMat3::from_cols(right, up, -f))
}

fn to_d(p: SpacePos) -> DVec3 {
    DVec3::new(p.x, p.y, p.z)
}

fn to_s(v: DVec3) -> SpacePos {
    SpacePos::new(v.x, v.y, v.z)
}

/// Une manœuvre : l'avant pivote vers `wanted` d'au plus `turn_rate` par seconde, la vitesse
/// suit l'avant à `speed` dans la limite de l'accélération.
pub fn steer(
    velocity: DVec3,
    rotation: DQuat,
    wanted: DVec3,
    speed: f64,
    def: &ShipCombat,
    thrust_factor: f64,
    dt: f64,
) -> (DVec3, DQuat) {
    let forward = rotation * DVec3::NEG_Z;
    let wanted = wanted.normalize_or(forward);
    let angle = forward.angle_between(wanted);
    let max = def.turn_rate_deg.to_radians() * dt;
    let rotation = if angle > 1e-6 {
        let arc = DQuat::from_rotation_arc(forward, wanted);
        let t = (max / angle).min(1.0);
        (DQuat::IDENTITY.slerp(arc, t) * rotation).normalize()
    } else {
        rotation
    };
    let forward = rotation * DVec3::NEG_Z;
    let target = forward * speed.min(def.max_speed * thrust_factor);
    let delta = target - velocity;
    let limit = def.accel * thrust_factor * dt;
    let velocity = velocity + delta.clamp_length_max(limit);
    (velocity, rotation)
}

/// Point visé pour toucher une cible mobile avec un projectile de vitesse `speed`.
pub fn lead(
    shooter: DVec3,
    shooter_vel: DVec3,
    target: DVec3,
    target_vel: DVec3,
    speed: f64,
) -> DVec3 {
    let rel = target_vel - shooter_vel;
    let t = shooter.distance(target) / speed.max(1.0);
    target + rel * t
}

/// Distance entre le segment `a → b` et la capsule d'axe `c0 → c1`, et le point du segment le
/// plus proche.
pub fn segment_capsule(a: DVec3, b: DVec3, c0: DVec3, c1: DVec3) -> (f64, DVec3) {
    // Plus courte distance entre deux segments (méthode classique, bornée à [0, 1]).
    let d1 = b - a;
    let d2 = c1 - c0;
    let r = a - c0;
    let (aa, ee, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let (s, t) = if aa <= 1e-12 && ee <= 1e-12 {
        (0.0, 0.0)
    } else if aa <= 1e-12 {
        (0.0, (f / ee).clamp(0.0, 1.0))
    } else {
        let c = d1.dot(r);
        if ee <= 1e-12 {
            ((-c / aa).clamp(0.0, 1.0), 0.0)
        } else {
            let bb = d1.dot(d2);
            let denom = aa * ee - bb * bb;
            let mut s = if denom > 1e-12 {
                ((bb * f - c * ee) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (bb * s + f) / ee;
            if t < 0.0 {
                t = 0.0;
                s = (-c / aa).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((bb - c) / aa).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    let p = a + d1 * s;
    let q = c0 + d2 * t;
    (p.distance(q), p)
}

/// L'axe de la capsule d'une coque (de la proue à la poupe, rayon déduit).
fn hull_axis(pos: DVec3, rot: DQuat, def: &ShipCombat) -> (DVec3, DVec3) {
    let half = (def.length / 2.0 - def.radius).max(0.0);
    let fwd = rot * DVec3::NEG_Z;
    (pos + fwd * half, pos - fwd * half)
}

/// Pilotes : choix de la cible, manœuvre, tirs. Les coques non jouées seulement.
#[allow(clippy::too_many_arguments)]
pub fn ai_pilots(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<CombatAssets>,
    mut rng: Local<Option<SimRng>>,
    mut ships: Query<(
        Entity,
        &mut Combatant,
        &mut AiPilot,
        &mut SpacePosition,
        &mut SpaceRotation,
        &mut Visibility,
    )>,
    player: Query<(Entity, &Combatant, &SpacePosition, &ShipMotion), Without<AiPilot>>,
    mut markers: Query<(&ShipMarker, &mut Visibility), Without<AiPilot>>,
) {
    let dt = f64::from(time.delta_secs().min(0.1));
    let rng = rng.get_or_insert_with(|| SimRng::new(0x5ca1_ab1e));
    // Positions et camps de tous, pour viser.
    let mut all: Vec<(Entity, Side, DVec3, DVec3, bool)> = ships
        .iter()
        .map(|(e, c, ai, p, _, _)| {
            (
                e,
                c.side,
                to_d(p.0),
                ai.velocity,
                ai.delay <= 0.0 && !c.hull.destroyed(),
            )
        })
        .collect();
    all.extend(
        player
            .iter()
            .map(|(e, c, p, m)| (e, c.side, to_d(p.0), m.velocity, !c.hull.destroyed())),
    );
    for (entity, mut c, mut ai, mut pos, mut rot, mut visibility) in &mut ships {
        if c.hull.destroyed() {
            continue;
        }
        if ai.delay > 0.0 {
            ai.delay -= dt;
            if ai.delay <= 0.0 {
                *visibility = Visibility::Inherited;
                for (marker, mut v) in &mut markers {
                    if marker.0 == entity {
                        *v = Visibility::Inherited;
                    }
                }
            }
            continue;
        }
        let me = to_d(pos.0);
        let rotation = rot.0.as_dquat();
        ai.think -= dt;
        c.since_hit += dt;
        for w in &mut c.weapons {
            w.cooldown = (w.cooldown - dt).max(0.0);
        }
        c.cm_cooldown = (c.cm_cooldown - dt).max(0.0);
        // Repenser : la cible hostile la plus proche ; un pillard trop abîmé fuit.
        if ai.think <= 0.0 {
            ai.think = 1.5;
            ai.target = all
                .iter()
                .filter(|(e, side, _, _, alive)| *alive && *e != entity && c.side.hostile(*side))
                .min_by(|a, b| a.2.distance(me).total_cmp(&b.2.distance(me)))
                .map(|(e, ..)| *e);
            if c.side == Side::Raiders && c.hull.core < c.hull.max_core * 0.3 {
                ai.mode = AiMode::Flee;
            }
        }
        let target = ai
            .target
            .and_then(|t| all.iter().find(|(e, _, _, _, alive)| *e == t && *alive))
            .copied();
        let thrust = c.hull.thrust_factor();
        let (wanted, speed) = match (ai.mode, target) {
            (AiMode::Flee, Some((_, _, tp, _, _))) => (me - tp, c.def.max_speed),
            (_, None) => (rotation * DVec3::NEG_Z, c.def.max_speed * 0.5),
            (AiMode::Break, Some((_, _, tp, _, _))) => {
                let away = (me - tp).normalize_or(DVec3::X);
                let side = away.cross(DVec3::Y).normalize_or(DVec3::X) * ai.break_side;
                if me.distance(tp) > 2200.0 {
                    ai.mode = AiMode::Attack;
                }
                (away + side, c.def.max_speed)
            }
            (AiMode::Attack, Some((_, _, tp, tv, _))) => {
                let gun_speed = c
                    .weapons
                    .iter()
                    .find(|w| w.def.kind == WeaponKind::Cannon)
                    .map_or(2000.0, |w| w.def.speed);
                let aim = lead(me, ai.velocity, tp, tv, gun_speed);
                if me.distance(tp) < 450.0 {
                    ai.mode = AiMode::Break;
                }
                (aim - me, c.def.max_speed * 0.85)
            }
        };
        let (velocity, rotation) = steer(ai.velocity, rotation, wanted, speed, &c.def, thrust, dt);
        ai.velocity = velocity;
        rot.0 = rotation.as_quat();
        pos.0 += to_s(velocity * dt);
        // Tirs : canon dans un cône de 3°, missile entre 1,2 et 5 km dans un cône de 30°.
        let Some((target_entity, _, tp, tv, _)) = target else {
            continue;
        };
        if ai.mode == AiMode::Flee {
            continue;
        }
        let forward = rotation * DVec3::NEG_Z;
        let dist = me.distance(tp);
        let fire_factor = c.hull.fire_rate_factor();
        let side = c.side;
        if let Some(gun) = c.weapon_mut(WeaponKind::Cannon) {
            let aim = lead(me, velocity, tp, tv, gun.def.speed);
            let angle = forward.angle_between(aim - me).to_degrees();
            if gun.cooldown <= 0.0 && angle < 3.0 && dist < AI_GUN_RANGE {
                gun.cooldown = gun.def.cooldown / fire_factor;
                let def = gun.def.clone();
                let dir = spread(rng, aim - me, def.spread_deg);
                fire(
                    &mut commands,
                    &assets,
                    entity,
                    side,
                    &def,
                    me + forward * c.def.length * 0.5,
                    velocity,
                    dir,
                    None,
                );
            }
        }
        for kind in [WeaponKind::Missile, WeaponKind::Torpedo] {
            let length = c.def.length;
            if let Some(m) = c.weapon_mut(kind) {
                let angle = forward.angle_between(tp - me).to_degrees();
                if m.cooldown <= 0.0 && angle < 30.0 && (1200.0..5000.0).contains(&dist) {
                    m.cooldown = m.def.cooldown;
                    m.ammo = m.ammo.map(|a| a - 1);
                    let def = m.def.clone();
                    fire(
                        &mut commands,
                        &assets,
                        entity,
                        side,
                        &def,
                        me + forward * length * 0.5,
                        velocity,
                        forward,
                        Some(target_entity),
                    );
                    break;
                }
            }
        }
    }
}

/// Portée de tir des canons de l'IA (m) : au-delà, la dispersion gâche les obus.
const AI_GUN_RANGE: f64 = 2500.0;

/// Dispersion aléatoire autour d'une direction.
fn spread(rng: &mut SimRng, dir: DVec3, deg: f64) -> DVec3 {
    let dir = dir.normalize_or(DVec3::NEG_Z);
    let a = deg.to_radians() * rng.unit_f64();
    let phi = std::f64::consts::TAU * rng.unit_f64();
    let ortho = dir.any_orthonormal_vector();
    let ortho = DQuat::from_axis_angle(dir, phi) * ortho;
    (DQuat::from_axis_angle(ortho, a) * dir).normalize()
}

/// Tire un projectile.
#[allow(clippy::too_many_arguments)]
pub fn fire(
    commands: &mut Commands,
    assets: &CombatAssets,
    owner: Entity,
    side: Side,
    def: &ShipWeapon,
    from: DVec3,
    shooter_velocity: DVec3,
    dir: DVec3,
    target: Option<Entity>,
) {
    let dir = dir.normalize_or(DVec3::NEG_Z);
    let (mesh, material) = match def.kind {
        WeaponKind::Cannon => (
            assets.tracer.clone(),
            if side == Side::Raiders {
                assets.tracer_hostile.clone()
            } else {
                assets.tracer_friendly.clone()
            },
        ),
        _ => (assets.missile.clone(), assets.missile_mat.clone()),
    };
    commands.spawn((
        Name::new("Tir"),
        SCOPE,
        Shot {
            owner,
            side,
            def: def.clone(),
            velocity: shooter_velocity + dir * def.speed,
            life: def.lifetime,
            target,
        },
        SpacePosition(to_s(from)),
        SpaceRotation(look_rotation(dir).as_quat()),
        Mesh3d(mesh),
        MeshMaterial3d(material),
    ));
}

/// Les armes du joueur : canons au clic gauche (maintenu), missile au clic droit sur la cible
/// verrouillée, contre-mesures.
#[allow(clippy::too_many_arguments)]
pub fn player_weapons(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<CombatAssets>,
    mode: Res<State<ControlMode>>,
    scripted: Res<crate::player::ScriptedControl>,
    mut fire_intent: ResMut<FireIntent>,
    guns: Single<&Action<FireGuns>>,
    missile: Single<&Action<FireMissile>>,
    decoy: Single<&Action<Countermeasure>>,
    target: Res<PlayerTarget>,
    mut rng: Local<Option<SimRng>>,
    mut player: Query<
        (
            Entity,
            &mut Combatant,
            &SpacePosition,
            &SpaceRotation,
            &ShipMotion,
        ),
        Without<AiPilot>,
    >,
) {
    let rng = rng.get_or_insert_with(|| SimRng::new(0x009a_11e7));
    let Ok((entity, mut c, pos, rot, motion)) = player.single_mut() else {
        return;
    };
    let dt = f64::from(time.delta_secs().min(0.1));
    c.since_hit += dt;
    for w in &mut c.weapons {
        w.cooldown = (w.cooldown - dt).max(0.0);
    }
    c.cm_cooldown = (c.cm_cooldown - dt).max(0.0);
    if c.hull.destroyed() {
        return;
    }
    // Les entrées : le joueur aux commandes, ou l'autopilote.
    let piloting = *mode.get() == ControlMode::Piloting;
    let (want_guns, want_missile, want_decoy) = if scripted.0 {
        let i = std::mem::take(&mut *fire_intent);
        (i.guns, i.missile, i.decoy)
    } else {
        (
            piloting && ***guns,
            piloting && ***missile,
            piloting && ***decoy,
        )
    };
    let me = to_d(pos.0);
    let forward = rot.0.as_dquat() * DVec3::NEG_Z;
    let length = c.def.length;
    let fire_factor = c.hull.fire_rate_factor();
    if want_guns
        && let Some(gun) = c.weapon_mut(WeaponKind::Cannon)
        && gun.cooldown <= 0.0
    {
        gun.cooldown = gun.def.cooldown / fire_factor;
        let def = gun.def.clone();
        let dir = spread(rng, forward, def.spread_deg);
        fire(
            &mut commands,
            &assets,
            entity,
            Side::Player,
            &def,
            me + forward * length * 0.5,
            motion.velocity,
            dir,
            None,
        );
    }
    if want_missile
        && let Some(t) = target.0
        && let Some(m) = c.weapon_mut(WeaponKind::Missile)
        && m.cooldown <= 0.0
    {
        m.cooldown = m.def.cooldown;
        m.ammo = m.ammo.map(|a| a - 1);
        let def = m.def.clone();
        fire(
            &mut commands,
            &assets,
            entity,
            Side::Player,
            &def,
            me + forward * length * 0.5,
            motion.velocity,
            forward,
            Some(t),
        );
    }
    if want_decoy && c.countermeasures > 0 && c.cm_cooldown <= 0.0 {
        c.countermeasures -= 1;
        c.cm_cooldown = 2.0;
        commands.write_message(Decoy { owner: entity });
    }
}

/// Ce que l'autopilote demande aux armes du joueur ce frame.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct FireIntent {
    pub guns: bool,
    pub missile: bool,
    pub decoy: bool,
}

/// Une contre-mesure lancée : les missiles qui visent son lanceur peuvent être leurrés.
#[derive(Message, Clone, Copy, Debug)]
pub struct Decoy {
    pub owner: Entity,
}

/// Les pillards lancent leurs leurres quand un missile approche.
pub fn ai_countermeasures(
    shots: Query<(&Shot, &SpacePosition)>,
    mut ships: Query<(Entity, &mut Combatant, &SpacePosition), With<AiPilot>>,
    mut decoys: MessageWriter<Decoy>,
) {
    for (entity, mut c, pos) in &mut ships {
        if c.countermeasures == 0 || c.cm_cooldown > 0.0 || c.hull.destroyed() {
            continue;
        }
        let threatened = shots
            .iter()
            .any(|(s, p)| s.target == Some(entity) && p.0.distance(pos.0) < 1200.0);
        if threatened {
            c.countermeasures -= 1;
            c.cm_cooldown = 3.0;
            decoys.write(Decoy { owner: entity });
        }
    }
}

/// Les leurres : chaque missile qui vise le lanceur tire sa chance d'être trompé.
pub fn apply_decoys(
    mut reader: MessageReader<Decoy>,
    mut rng: Local<Option<SimRng>>,
    mut shots: Query<&mut Shot>,
) {
    let rng = rng.get_or_insert_with(|| SimRng::new(0x00de_c075));
    for decoy in reader.read() {
        for mut shot in &mut shots {
            if shot.target == Some(decoy.owner)
                && sh_sim::combat::decoyed(rng, shot.def.decoy_chance)
            {
                shot.target = None;
            }
        }
    }
}

/// Les projectiles avancent (les missiles se guident), touchent, ou s'éteignent.
#[allow(clippy::too_many_arguments)]
pub fn move_shots(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<CombatAssets>,
    mut shots: Query<
        (Entity, &mut Shot, &mut SpacePosition, &mut SpaceRotation),
        Without<Combatant>,
    >,
    mut ships: Query<(
        Entity,
        &mut Combatant,
        &SpacePosition,
        &SpaceRotation,
        Option<&AiPilot>,
        Option<&ShipMotion>,
    )>,
    mut lost: MessageWriter<ShipLost>,
    mut player_hits: MessageWriter<PlayerHit>,
) {
    let dt = f64::from(time.delta_secs().min(0.1));
    let targets: Vec<(Entity, DVec3, DVec3)> = ships
        .iter()
        .map(|(e, _, p, _, ai, m)| {
            let v = ai
                .map(|a| a.velocity)
                .or(m.map(|m| m.velocity))
                .unwrap_or_default();
            (e, to_d(p.0), v)
        })
        .collect();
    for (shot_entity, mut shot, mut pos, mut rot) in &mut shots {
        shot.life -= dt;
        if shot.life <= 0.0 {
            commands.entity(shot_entity).despawn();
            continue;
        }
        let here = to_d(pos.0);
        // Guidage : la vitesse pivote vers le point de rencontre.
        if shot.def.turn_rate_deg > 0.0
            && let Some(t) = shot.target
            && let Some(&(_, tp, tv)) = targets.iter().find(|(e, ..)| *e == t)
        {
            let speed = shot.velocity.length();
            let aim = lead(here, DVec3::ZERO, tp, tv, speed) - here;
            let v = shot.velocity.normalize_or(aim);
            let angle = v.angle_between(aim.normalize_or(v));
            let max = shot.def.turn_rate_deg.to_radians() * dt;
            let turned = if angle > 1e-6 {
                DQuat::IDENTITY.slerp(
                    DQuat::from_rotation_arc(v, aim.normalize_or(v)),
                    (max / angle).min(1.0),
                ) * v
            } else {
                v
            };
            shot.velocity = turned * speed;
        }
        let next = here + shot.velocity * dt;
        let mut hit_at = None;
        for (e, mut c, sp, sr, _, _) in &mut ships {
            if e == shot.owner || c.hull.destroyed() || !shot.side.hostile(c.side) {
                continue;
            }
            let sp = to_d(sp.0);
            let srot = sr.0.as_dquat();
            let (c0, c1) = hull_axis(sp, srot, &c.def);
            let (d, p) = segment_capsule(here, next, c0, c1);
            if d > c.def.radius {
                continue;
            }
            let local = srot.inverse() * (p - sp);
            let section = section_at(local.to_array(), c.def.length, c.def.radius);
            let hit = c.hull.hit(section, shot.def.damage, shot.def.penetration);
            c.since_hit = 0.0;
            if c.side == Side::Player {
                player_hits.write(PlayerHit {
                    section,
                    penetrated: hit.penetrated,
                    breached: hit.breached,
                    damage: hit.structure_damage,
                });
            }
            if hit.destroyed {
                lost.write(ShipLost {
                    entity: e,
                    side: c.side,
                });
            }
            hit_at = Some((p, hit.destroyed, c.def.radius));
            break;
        }
        if let Some((p, destroyed, radius)) = hit_at {
            commands.entity(shot_entity).despawn();
            let size = if destroyed {
                radius as f32 * 3.0
            } else if shot.def.kind == WeaponKind::Cannon {
                4.0
            } else {
                14.0
            };
            spawn_flash(
                &mut commands,
                &assets,
                p,
                size,
                if destroyed { 2.5 } else { 0.4 },
            );
            continue;
        }
        pos.0 = to_s(next);
        rot.0 = look_rotation(shot.velocity).as_quat();
    }
}

pub fn spawn_flash(
    commands: &mut Commands,
    assets: &CombatAssets,
    at: DVec3,
    size: f32,
    life: f32,
) {
    commands.spawn((
        Name::new("Éclair"),
        SCOPE,
        Flash {
            life,
            max: life,
            size,
        },
        SpacePosition(to_s(at)),
        Mesh3d(assets.flash.clone()),
        MeshMaterial3d(assets.flash_mat.clone()),
        Transform::from_scale(Vec3::splat(size * 0.3)),
    ));
}

/// Les éclairs grossissent et s'éteignent.
pub fn fade_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut Flash, &mut Transform)>,
) {
    for (e, mut f, mut t) in &mut flashes {
        f.life -= time.delta_secs();
        if f.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let k = 1.0 - f.life / f.max;
        t.scale = Vec3::splat(f.size * (0.3 + 0.7 * k.sqrt()) * (1.0 - k * 0.5));
    }
}

/// Une coque perdue explose et disparaît (avec son étiquette).
pub fn remove_lost(
    mut commands: Commands,
    mut reader: MessageReader<ShipLost>,
    markers: Query<(Entity, &ShipMarker)>,
) {
    for lost in reader.read() {
        if lost.side == Side::Player {
            continue;
        }
        commands.entity(lost.entity).despawn();
        for (e, m) in &markers {
            if m.0 == lost.entity {
                commands.entity(e).despawn();
            }
        }
    }
}

/// La cible du joueur : l'hostile le plus proche de l'axe de visée (cône de 25°, 8 km).
pub fn pick_target(
    mut target: ResMut<PlayerTarget>,
    player: Query<(&Combatant, &SpacePosition, &SpaceRotation), Without<AiPilot>>,
    ships: Query<(Entity, &Combatant, &AiPilot, &SpacePosition)>,
) {
    let Ok((me, pos, rot)) = player.single() else {
        target.0 = None;
        return;
    };
    let here = to_d(pos.0);
    let forward = rot.0.as_dquat() * DVec3::NEG_Z;
    let best = ships
        .iter()
        .filter(|(_, c, ai, _)| me.side.hostile(c.side) && ai.delay <= 0.0 && !c.hull.destroyed())
        .filter_map(|(e, _, _, p)| {
            let d = to_d(p.0) - here;
            let angle = forward.angle_between(d).to_degrees();
            (angle < 25.0 && d.length() < 8000.0).then_some((e, angle))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(e, _)| e);
    if target.0 != best {
        target.0 = best;
    }
}

/// Le vaisseau du joueur garde ses dégâts : sa coque de combat est celle de sa fiche.
pub fn sync_player_hull(
    player: Query<&Combatant, (Without<AiPilot>, Changed<Combatant>)>,
    career: Option<ResMut<Career>>,
) {
    let (Ok(c), Some(mut career)) = (player.single(), career) else {
        return;
    };
    if career.captain.hull.as_ref() != Some(&c.hull) {
        career.captain.hull = Some(c.hull.clone());
    }
}

/// Les étiquettes des coques : nom, distance, état.
pub fn update_markers(
    texts: Res<Texts>,
    sector: Option<Res<ActiveSector>>,
    origin: Query<&SpacePosition, Without<ShipMarker>>,
    ships: Query<(&Combatant, &SpacePosition)>,
    target: Res<PlayerTarget>,
    mut markers: Query<(&ShipMarker, &mut Text, &mut TextColor)>,
) {
    let Some(here) = sector.and_then(|s| origin.get(s.root).ok()).map(|p| p.0) else {
        return;
    };
    for (marker, mut text, mut color) in &mut markers {
        let Ok((c, p)) = ships.get(marker.0) else {
            continue;
        };
        let km = p.0.distance(here) / 1000.0;
        let locked = target.0 == Some(marker.0);
        let label = texts.format(
            if locked {
                "combat.marker.locked"
            } else {
                "combat.marker"
            },
            &[
                ("name", &texts.get(&c.def.name_key)),
                ("distance", &texts.number(km, 1)),
                ("hull", &texts.number(c.hull.integrity() * 100.0, 0)),
            ],
        );
        if text.0 != label {
            text.0 = label;
        }
        let wanted = if locked {
            Color::srgb(1.0, 0.85, 0.3)
        } else if c.side == Side::Raiders {
            Color::srgb(1.0, 0.35, 0.25)
        } else {
            Color::srgb(0.55, 0.8, 1.0)
        };
        if color.0 != wanted {
            color.0 = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_capsule_arrete_un_obus_qui_la_traverse() {
        let (d, p) = segment_capsule(
            DVec3::new(-100.0, 0.0, 0.0),
            DVec3::new(100.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, -20.0),
            DVec3::new(0.0, 0.0, 20.0),
        );
        assert!(d < 1e-9);
        assert!(p.length() < 1e-9);
        let (d, _) = segment_capsule(
            DVec3::new(-100.0, 30.0, 0.0),
            DVec3::new(100.0, 30.0, 0.0),
            DVec3::new(0.0, 0.0, -20.0),
            DVec3::new(0.0, 0.0, 20.0),
        );
        assert!((d - 30.0).abs() < 1e-9);
    }

    #[test]
    fn viser_en_avant_d_une_cible_mobile() {
        let aim = lead(
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::new(0.0, 0.0, -2000.0),
            DVec3::new(100.0, 0.0, 0.0),
            2000.0,
        );
        assert!((aim.x - 100.0).abs() < 1e-9);
    }

    #[test]
    fn une_coque_tourne_a_sa_vitesse_limite() {
        let def = ShipCombat {
            name_key: String::new(),
            model: String::new(),
            radius: 10.0,
            length: 50.0,
            max_speed: 200.0,
            accel: 50.0,
            turn_rate_deg: 30.0,
            sections: Vec::new(),
            core: 1.0,
            weapons: Vec::new(),
            countermeasures: 0,
        };
        let (_, rot) = steer(
            DVec3::ZERO,
            DQuat::IDENTITY,
            DVec3::X,
            100.0,
            &def,
            1.0,
            1.0,
        );
        let turned = (rot * DVec3::NEG_Z)
            .angle_between(DVec3::NEG_Z)
            .to_degrees();
        assert!((turned - 30.0).abs() < 1e-6, "{turned}");
        let (vel, _) = steer(
            DVec3::ZERO,
            DQuat::IDENTITY,
            DVec3::NEG_Z,
            100.0,
            &def,
            0.3,
            1.0,
        );
        assert!(
            (vel.length() - 15.0).abs() < 1e-9,
            "poussée réduite : {}",
            vel.length()
        );
    }

    #[test]
    fn regarder_dans_une_direction() {
        for dir in [DVec3::X, DVec3::NEG_Z, DVec3::new(1.0, 2.0, 3.0), DVec3::Y] {
            let r = look_rotation(dir);
            assert!((r * DVec3::NEG_Z).distance(dir.normalize()) < 1e-9);
        }
    }
}
