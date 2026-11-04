//! Le combat (M6) : raids issus de la simulation, combat spatial ([`space`]), combat à pied
//! ([`foot`]), effets à bord, mort.
//!
//! **Raid** : en arrivant dans un système où la Ligue chasse, `sh_sim::combat::raid_on_arrival`
//! décide d'une embuscade (déterministe). Les pillards surgissent à quelques kilomètres ; la
//! flotte de défense du système arrive après un délai. Le raid finit quand les pillards sont
//! détruits ou en fuite (gagné), quand le joueur s'est éloigné ou est parti (fui), ou quand sa
//! coque est perdue (perdu). Les pertes des pillards retournent à la simulation.
//!
//! `SH_RAID=1` force un raid à l'arrivée (autopilote), `SH_RAID=alone` aussi mais sans
//! rescousse, `SH_RAID=0` les interdit.

pub mod foot;
pub mod space;

use bevy::math::DVec3;
use bevy::prelude::*;
use sh_core::{Id, SpacePos};
use sh_data::Place;
use sh_sim::SimCommand;
use sh_sim::combat::{Encounter, Hull, raid_on_arrival};

use crate::career::{Career, MissionLog};
use crate::clock::{GameClock, format_date};
use crate::data::{Data, Texts};
use crate::living::LivingWorld;
use crate::sector::ActiveSector;
use crate::space::{SpacePosition, SpaceRotation};
use crate::state::{AppState, ViewLevel};
use crate::ui::hud::Notice;
use crate::ui::theme::UiFonts;
use crate::voyage::Location;
use space::{AiMode, AiPilot, Combatant, PlayerHit, ShipLost, Side};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ShipLost>()
            .add_message::<PlayerHit>()
            .add_message::<space::Decoy>()
            .init_resource::<space::PlayerTarget>()
            .init_resource::<space::FireIntent>()
            .init_resource::<RaidChecked>()
            .init_resource::<Shake>()
            .init_resource::<CruiseLock>()
            .add_plugins(foot::FootCombatPlugin)
            .add_systems(Startup, space::setup_assets)
            .add_systems(
                Update,
                (
                    start_raid,
                    space::pick_target,
                    space::player_weapons,
                    space::ai_pilots,
                    space::ai_countermeasures,
                    space::apply_decoys,
                    space::move_shots,
                    space::remove_lost,
                    space::sync_player_hull,
                    space::fade_flashes,
                    space::update_markers,
                    on_player_hit,
                    cruise_lock,
                    tick_raid,
                )
                    .chain()
                    .in_set(CombatSystems)
                    .after(crate::ship::ShipSystems)
                    .run_if(in_state(ViewLevel::Global))
                    .run_if(resource_exists::<ActiveSector>),
            )
            .add_systems(
                Update,
                (shake_camera, tick_death).after(crate::player::PlayerSystems),
            )
            .add_systems(OnExit(ViewLevel::Global), leave_raid);
    }
}

/// Les systèmes du combat (après le vol).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CombatSystems;

/// Le raid en cours.
#[derive(Resource, Debug)]
pub struct Raid {
    pub encounter: Encounter,
    pub system: Id<Place>,
    pub kills: u32,
    pub outcome: Option<RaidOutcome>,
    /// Secondes depuis la fin (les survivants s'en vont, puis tout se range).
    pub after: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaidOutcome {
    Won,
    Fled,
    Lost,
}

/// Le dernier couple (système, jour) examiné : un seul tirage par arrivée.
#[derive(Resource, Default, Debug)]
struct RaidChecked(Option<(Id<Place>, i64)>);

/// Une coque hostile est trop proche pour passer en croisière (lu par le vol au frame suivant).
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CruiseLock(pub bool);

fn cruise_lock(
    data: Res<Data>,
    texts: Res<Texts>,
    sector: Res<ActiveSector>,
    intent: Res<crate::ship::PilotIntent>,
    ships: Query<(&Combatant, &SpacePosition)>,
    mut lock: ResMut<CruiseLock>,
    mut notices: MessageWriter<Notice>,
) {
    let here = ships.get(sector.root).map(|(c, p)| (c.side, p.0));
    let locked = here.is_ok_and(|(side, at)| {
        ships.iter().any(|(c, p)| {
            side.hostile(c.side)
                && !c.hull.destroyed()
                && p.0.distance(at) < data.combat.raid.cruise_lock_distance
        })
    });
    if locked && intent.cruise && !lock.0 {
        notices.write(Notice(texts.get("combat.cruise_locked")));
    }
    if lock.0 != locked {
        lock.0 = locked;
    }
}

/// Secousse de la caméra à bord (impacts), qui s'amortit.
#[derive(Resource, Default, Debug)]
pub struct Shake(pub f32);

/// La mort du joueur : écran, puis retour au menu.
#[derive(Resource, Debug)]
pub struct Death {
    pub timer: f32,
}

#[allow(clippy::too_many_arguments)]
fn start_raid(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    asset_server: Res<AssetServer>,
    clock: Res<GameClock>,
    sector: Res<ActiveSector>,
    location: Option<Res<Location>>,
    living: Option<Res<LivingWorld>>,
    career: Option<Res<Career>>,
    raid: Option<Res<Raid>>,
    mut checked: ResMut<RaidChecked>,
    ships: Query<(&SpacePosition, &SpaceRotation)>,
    mut notices: MessageWriter<Notice>,
) {
    let (Some(location), Some(living)) = (location, living) else {
        return;
    };
    if raid.is_some() {
        return;
    }
    let day = clock.date.day_index();
    let key = (location.system.clone(), day);
    if checked.0.as_ref() == Some(&key) {
        return;
    }
    // On attend que le vaisseau soit placé dans le système (arrivée faite).
    let Ok((pos, rot)) = ships.get(sector.root) else {
        return;
    };
    checked.0 = Some(key);
    let force = std::env::var("SH_RAID").ok();
    if force.as_deref() == Some("0") {
        return;
    }
    let forced = matches!(force.as_deref(), Some("1" | "alone"));
    let Some(mut encounter) = raid_on_arrival(&data, &living.sim, &location.system, day, forced)
    else {
        return;
    };
    if force.as_deref() == Some("alone") {
        encounter.defenders = None;
    }
    let Some(player_def) = data.start().and_then(|s| data.combat.ship(&s.ship)) else {
        return;
    };
    let me = DVec3::new(pos.0.x, pos.0.y, pos.0.z);
    let forward = rot.0.as_dquat() * DVec3::NEG_Z;
    let rules = &data.combat.raid;
    // Les pillards en arc devant le joueur, face à lui ; la rescousse dans son dos, en retard.
    let n = encounter.attackers.len().max(1) as f64;
    for (i, key) in encounter.attackers.iter().enumerate() {
        let a = (i as f64 - (n - 1.0) / 2.0) * 0.35;
        let dir = bevy::math::DQuat::from_rotation_y(a) * forward;
        let p = me + dir * rules.spawn_distance + DVec3::Y * (i as f64 * 120.0 - 200.0);
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        space::spawn_ship(
            &mut commands,
            &asset_server,
            &data,
            &texts,
            &fonts,
            Side::Raiders,
            key,
            sp(p),
            me - p,
            0.0,
            side,
        );
    }
    if let Some((_, defenders)) = &encounter.defenders {
        let m = defenders.len().max(1) as f64;
        for (i, key) in defenders.iter().enumerate() {
            let a = (i as f64 - (m - 1.0) / 2.0) * 0.25;
            let dir = bevy::math::DQuat::from_rotation_y(a) * -forward;
            let p = me + dir * rules.spawn_distance * 1.4;
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            space::spawn_ship(
                &mut commands,
                &asset_server,
                &data,
                &texts,
                &fonts,
                Side::Defenders,
                key,
                sp(p),
                me - p,
                rules.defenders_delay,
                side,
            );
        }
    }
    // Le vaisseau du joueur entre en combat avec sa coque telle qu'elle est.
    let hull = career
        .and_then(|c| c.captain.hull.clone())
        .unwrap_or_else(|| Hull::new(player_def, data.combat.hull));
    let ship_key = data.start().map(|s| s.ship.clone()).unwrap_or_default();
    commands.entity(sector.root).insert(Combatant::new(
        Side::Player,
        &ship_key,
        player_def,
        hull,
        &data,
    ));
    info!(
        "raid : {} ({} coques) à {}, rescousse {:?}",
        encounter.raiders_name,
        encounter.attackers.len(),
        location.system,
        encounter.defenders.as_ref().map(|d| d.1.len())
    );
    notices.write(Notice(texts.format(
        "combat.raid.start",
        &[
            ("raiders", &encounter.raiders_name),
            ("count", &encounter.attackers.len().to_string()),
        ],
    )));
    commands.insert_resource(Raid {
        encounter,
        system: location.system.clone(),
        kills: 0,
        outcome: None,
        after: 0.0,
    });
}

fn sp(v: DVec3) -> SpacePos {
    SpacePos::new(v.x, v.y, v.z)
}

/// Le déroulé du raid : pertes, fuites, issue.
#[allow(clippy::too_many_arguments)]
fn tick_raid(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<Data>,
    texts: Res<Texts>,
    clock: Res<GameClock>,
    sector: Res<ActiveSector>,
    raid: Option<ResMut<Raid>>,
    mut lost: MessageReader<ShipLost>,
    ships: Query<(Entity, &Combatant, &SpacePosition, Option<&AiPilot>)>,
    living: Option<ResMut<LivingWorld>>,
    mut log: ResMut<MissionLog>,
    mut notices: MessageWriter<Notice>,
) {
    let Some(mut raid) = raid else {
        lost.clear();
        return;
    };
    for l in lost.read() {
        match l.side {
            Side::Raiders => raid.kills += 1,
            Side::Player if raid.outcome.is_none() => {
                raid.outcome = Some(RaidOutcome::Lost);
                commands.insert_resource(Death { timer: 0.0 });
            }
            _ => {}
        }
    }
    let Ok((_, _, player_pos, _)) = ships.get(sector.root) else {
        return;
    };
    let here = player_pos.0;
    let rules = &data.combat.raid;
    if raid.outcome.is_none() {
        // Les pillards qui ont fui assez loin quittent la scène.
        let mut alive = 0;
        let mut nearest = f64::MAX;
        for (e, c, p, ai) in &ships {
            if c.side != Side::Raiders || c.hull.destroyed() {
                continue;
            }
            let d = p.0.distance(here);
            if ai.is_some_and(|a| a.mode == AiMode::Flee) && d > rules.escape_distance {
                commands.entity(e).despawn();
                continue;
            }
            alive += 1;
            nearest = nearest.min(d);
        }
        let outcome = if alive == 0 {
            Some(RaidOutcome::Won)
        } else if nearest > rules.escape_distance {
            Some(RaidOutcome::Fled)
        } else {
            None
        };
        if let Some(outcome) = outcome {
            raid.outcome = Some(outcome);
            conclude(
                &mut raid,
                &data,
                &texts,
                &clock,
                living,
                &mut log,
                &mut notices,
            );
        }
        return;
    }
    if raid.outcome == Some(RaidOutcome::Lost) {
        return;
    }
    // Après l'issue : quelques secondes, puis les survivants se retirent.
    raid.after += time.delta_secs();
    if raid.after > 12.0 {
        for (e, c, _, ai) in &ships {
            if ai.is_some() && c.side != Side::Player {
                commands.entity(e).despawn();
            }
        }
        commands.entity(sector.root).remove::<Combatant>();
        commands.remove_resource::<Raid>();
    }
}

/// L'issue d'un raid : message, journal, pertes des pillards rendues à la simulation.
fn conclude(
    raid: &mut Raid,
    data: &Data,
    texts: &Texts,
    clock: &GameClock,
    living: Option<ResMut<LivingWorld>>,
    log: &mut MissionLog,
    notices: &mut MessageWriter<Notice>,
) {
    let key = match raid.outcome {
        Some(RaidOutcome::Won) => "combat.raid.won",
        Some(RaidOutcome::Fled) => "combat.raid.fled",
        _ => "combat.raid.lost",
    };
    let line = texts.format(
        key,
        &[
            ("raiders", &raid.encounter.raiders_name),
            ("kills", &raid.kills.to_string()),
        ],
    );
    info!("raid : {line}");
    log.lines
        .push(format!("{} — {line}", format_date(texts, clock.date)));
    notices.write(Notice(line));
    if raid.kills > 0
        && let Some(mut living) = living
    {
        living.sim.apply(&SimCommand::FleetLosses {
            fleet: raid.encounter.raiders.clone(),
            condition: f64::from(raid.kills) * data.combat.raid.condition_per_kill,
        });
    }
}

/// Quitter la vue Globale pendant un raid (voyage, carte) : c'est une fuite.
#[allow(clippy::too_many_arguments)]
fn leave_raid(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    clock: Option<Res<GameClock>>,
    raid: Option<ResMut<Raid>>,
    living: Option<ResMut<LivingWorld>>,
    mut log: ResMut<MissionLog>,
    mut notices: MessageWriter<Notice>,
    sector: Option<Res<ActiveSector>>,
) {
    let (Some(mut raid), Some(clock)) = (raid, clock) else {
        return;
    };
    if raid.outcome.is_none() {
        raid.outcome = Some(RaidOutcome::Fled);
        conclude(
            &mut raid,
            &data,
            &texts,
            &clock,
            living,
            &mut log,
            &mut notices,
        );
    }
    if raid.outcome != Some(RaidOutcome::Lost) {
        commands.remove_resource::<Raid>();
        if let Some(sector) = sector {
            commands.entity(sector.root).remove::<Combatant>();
        }
    }
}

/// Un impact sur le vaisseau du joueur se ressent à bord : secousse, alarme, message.
fn on_player_hit(
    texts: Res<Texts>,
    mut hits: MessageReader<PlayerHit>,
    mut shake: ResMut<Shake>,
    mut notices: MessageWriter<Notice>,
) {
    for hit in hits.read() {
        debug!(
            "impact sur le vaisseau du joueur : {:?}, {:.0} dégâts, percé : {}",
            hit.section, hit.damage, hit.penetrated
        );
        let k = if hit.penetrated { 0.6 } else { 0.25 };
        shake.0 = (shake.0 + k * (hit.damage as f32 / 60.0).clamp(0.3, 2.0)).min(1.5);
        if hit.breached {
            notices.write(Notice(
                texts.get(&format!("combat.breach.{}", hit.section.key())),
            ));
        }
    }
}

/// La caméra du joueur tremble après un impact.
fn shake_camera(
    time: Res<Time>,
    mut shake: ResMut<Shake>,
    mut camera: Query<&mut Transform, With<crate::player::PlayerCamera>>,
) {
    if shake.0 <= 0.0 {
        return;
    }
    let t = time.elapsed_secs();
    let amp = shake.0 * 0.04;
    for mut transform in &mut camera {
        transform.translation += Vec3::new((t * 53.0).sin(), (t * 71.0).cos(), 0.0) * amp;
        transform.rotation *= Quat::from_rotation_z((t * 37.0).sin() * amp * 0.5);
    }
    shake.0 = (shake.0 - time.delta_secs() * 1.2).max(0.0);
}

#[derive(Component)]
struct DeathScreen;

/// La mort : un écran, puis le menu principal ; la partie est perdue (pas encore de
/// sauvegarde).
fn tick_death(
    mut commands: Commands,
    time: Res<Time>,
    texts: Res<Texts>,
    fonts: Option<Res<UiFonts>>,
    death: Option<ResMut<Death>>,
    screens: Query<Entity, With<DeathScreen>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(mut death) = death else {
        return;
    };
    if screens.is_empty()
        && let Some(fonts) = fonts
    {
        commands.spawn((
            DeathScreen,
            DespawnOnExit(AppState::InGame),
            GlobalZIndex(50),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            children![
                (
                    Text::new(texts.get("combat.death.title")),
                    fonts.title(40.0),
                    TextColor(Color::srgb(0.9, 0.25, 0.2)),
                ),
                (
                    Text::new(texts.get("combat.death.hint")),
                    fonts.body(18.0),
                    TextColor(crate::ui::theme::TEXT_DIM),
                ),
            ],
        ));
    }
    death.timer += time.delta_secs();
    if death.timer > 5.0 {
        // La partie est perdue : la quitter l'efface (`state::forget_game`), une nouvelle partie
        // repart de zéro.
        next.set(AppState::MainMenu);
        info!("mort du joueur : retour au menu principal");
    }
}

/// La cible verrouillée et l'état du vaisseau, pour l'interface.
pub fn player_section_state(hull: &Hull, section: sh_data::SectionId) -> f64 {
    hull.section(section)
        .map_or(0.0, |s| s.structure / s.max_structure.max(1e-9))
}
