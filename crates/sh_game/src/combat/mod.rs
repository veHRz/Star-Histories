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
use crate::ui::theme::{self, UiFonts};
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

/// Le verrou de croisière, avec hystérésis (M11) : posé dès qu'un hostile est à moins de
/// `distance` (m), il ne se lève qu'une fois le plus proche au-delà de `distance + margin`.
/// La croisière monte en douceur : sans marge, un pillard qui poursuit la rebloquerait avant
/// qu'elle ne le distance. `nearest` : distance du plus proche hostile, s'il y en a. Logique pure.
pub fn cruise_locked(was_locked: bool, nearest: Option<f64>, distance: f64, margin: f64) -> bool {
    let reach = if was_locked {
        distance + margin.max(0.0)
    } else {
        distance
    };
    nearest.is_some_and(|d| d < reach)
}

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
    let nearest = here.ok().and_then(|(side, at)| {
        ships
            .iter()
            .filter(|(c, _)| side.hostile(c.side) && !c.hull.destroyed())
            .map(|(_, p)| p.0.distance(at))
            .min_by(f64::total_cmp)
    });
    let rules = &data.combat.raid;
    let locked = cruise_locked(
        lock.0,
        nearest,
        rules.cruise_lock_distance,
        rules.cruise_unlock_margin,
    );
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

/// La jauge qui se remplit jusqu'au retour au menu.
#[derive(Component)]
struct DeathGauge;

/// Durée de l'écran de mort avant le retour au menu principal (s).
const DEATH_SECONDS: f32 = 5.0;
/// Largeur de la jauge de l'écran de mort (px).
const DEATH_GAUGE_WIDTH: f32 = 360.0;
const GAUGE_MIN_SCALE: f32 = 0.001;

/// La mort : un écran, puis le menu principal ; la partie est perdue (pas encore de
/// sauvegarde).
///
/// Habillage (M8) : l'écran d'appareil des menus, passé au rouge d'alarme (équerres, titre en
/// capitales, filet), et une jauge qui se remplit jusqu'au retour au menu : le joueur voit que
/// le jeu n'est pas figé.
#[allow(clippy::too_many_arguments)]
fn tick_death(
    mut commands: Commands,
    time: Res<Time>,
    texts: Res<Texts>,
    fonts: Option<Res<UiFonts>>,
    death: Option<ResMut<Death>>,
    screens: Query<Entity, With<DeathScreen>>,
    mut gauges: Query<&mut UiTransform, With<DeathGauge>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(mut death) = death else {
        return;
    };
    if screens.is_empty()
        && let Some(fonts) = fonts
    {
        commands
            .spawn((
                Name::new("Écran de mort"),
                DeathScreen,
                DespawnOnExit(AppState::InGame),
                GlobalZIndex(50),
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                // Voile presque noir, à peine rougi : la vue s'éteint.
                BackgroundColor(Color::srgba(0.035, 0.008, 0.008, 0.88)),
            ))
            .with_children(|screen| {
                theme::spawn_corner_marks(screen, 28.0, 44.0, theme::DANGER.with_alpha(0.55));
                screen.spawn(theme::heading(
                    &fonts,
                    &texts.get("combat.death.title"),
                    40.0,
                    theme::DANGER,
                ));
                // Le filet des titres, au rouge d'alarme.
                screen.spawn((
                    Node {
                        width: px(DEATH_GAUGE_WIDTH + 80.0),
                        height: px(1),
                        margin: UiRect::new(px(0), px(0), px(14), px(22)),
                        ..default()
                    },
                    BackgroundColor(theme::DANGER.with_alpha(0.45)),
                ));
                screen.spawn((
                    Text::new(texts.get("combat.death.hint")),
                    fonts.body(18.0),
                    TextColor(theme::TEXT_DIM),
                ));
                screen.spawn((
                    Node {
                        width: px(DEATH_GAUGE_WIDTH),
                        height: px(4),
                        margin: UiRect::top(px(16)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(theme::RULE),
                    children![(
                        DeathGauge,
                        Node {
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                        BackgroundColor(theme::DANGER),
                        // Vide au départ ; `UiTransform` : pas de remise en page à chaque image.
                        UiTransform {
                            scale: Vec2::new(GAUGE_MIN_SCALE, 1.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    )],
                ));
            });
    }
    death.timer += time.delta_secs();
    // La jauge se remplit depuis la gauche : mise à l'échelle autour du centre, recalée.
    // Jamais une échelle nulle : la transformation du nœud doit rester inversible.
    let k = (death.timer / DEATH_SECONDS).clamp(GAUGE_MIN_SCALE, 1.0);
    for mut gauge in &mut gauges {
        gauge.scale = Vec2::new(k, 1.0);
        gauge.translation = Val2::px(-(1.0 - k) * DEATH_GAUGE_WIDTH / 2.0, 0.0);
    }
    if death.timer > DEATH_SECONDS {
        // La partie est perdue : la quitter l'efface (`state::forget_game`), une nouvelle partie
        // repart de zéro ou d'une sauvegarde (en Homme de fer, il n'en reste aucune).
        commands.queue(crate::save::erase_ironman);
        next.set(AppState::MainMenu);
        info!("mort du joueur : retour au menu principal");
    }
}

/// La cible verrouillée et l'état du vaisseau, pour l'interface.
pub fn player_section_state(hull: &Hull, section: sh_data::SectionId) -> f64 {
    hull.section(section)
        .map_or(0.0, |s| s.structure / s.max_structure.max(1e-9))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_verrou_de_croisiere_ne_bat_pas_autour_du_seuil() {
        // Un pillard qui oscille autour de 8 km : posé à 7,9 km, le verrou tient jusqu'au-delà
        // de 9 km, puis ne revient qu'en deçà de 8 km. Trois bascules, pas une par frame.
        let distances = [
            7_900.0, 8_050.0, 7_990.0, 8_500.0, 8_990.0, 9_010.0, 8_999.0, 8_010.0, 7_999.0,
        ];
        let expected = [true, true, true, true, true, false, false, false, true];
        let mut locked = false;
        for (d, want) in distances.iter().zip(expected) {
            locked = cruise_locked(locked, Some(*d), 8_000.0, 1_000.0);
            assert_eq!(locked, want, "à {d} m");
        }
        // Plus d'hostile : levé ; sans marge, le seuil d'avant M11.
        assert!(!cruise_locked(true, None, 8_000.0, 1_000.0));
        assert!(!cruise_locked(true, Some(8_050.0), 8_000.0, 0.0));
    }

    /// Le joueur file en ligne droite devant un pillard qui le poursuit à `chaser` m/s (pire cas :
    /// en vrai, le verrou ne se lève que si les pillards ne le serrent plus), en
    /// partant juste au-delà de la distance où le verrou se lève ; bridé à 250 m/s sous le
    /// verrou, en croisière (montée de `space.ron`) sinon. Rend le nombre de fois où le verrou
    /// revient et l'écart final après 20 s.
    fn flee(margin: f64, chaser: f64) -> (u32, f64) {
        let scale = sh_data::SystemScale {
            au: 1.496e9,
            first_orbit_au: 0.4,
            orbit_ratio: 1.7,
            star_radius: 7.0e7,
            planet_radius: 6.4e5,
            gas_giant_radius: 7.0e6,
            moon_radius: 1.7e5,
            moon_orbit_radii: 12.0,
            parking_orbit_radii: 1.5,
            cruise_rate: 0.15,
            cruise_min_speed: 2000.0,
            cruise_max_speed: 3.0e7,
            cruise_spool_accel: 30.0,
            cruise_spool_time: 1.5,
            manoeuvre_max_speed: 1000.0,
        };
        let (lock, dt) = (8_000.0, 1.0 / 60.0);
        let (mut gap, mut speed, mut spool) = (lock + margin + 1.0, 250.0f64, None);
        // Le verrou était posé : il se lève au premier frame.
        let (mut locked, mut relocks) = (true, 0);
        for _ in 0..(20.0 / dt) as usize {
            let now = cruise_locked(locked, Some(gap), lock, margin);
            relocks += u32::from(now && !locked);
            locked = now;
            if locked {
                (speed, spool) = (speed.min(250.0), None);
            } else {
                let level = spool.unwrap_or_else(|| scale.cruise_spool_level(speed));
                let (next, level) = scale.cruise_climb(speed, level, scale.cruise_max_speed, dt);
                (speed, spool) = (next, Some(level));
            }
            gap += (speed - chaser) * dt;
        }
        (relocks, gap)
    }

    #[test]
    fn la_fuite_reussit_malgre_la_montee_douce_de_la_croisiere() {
        // Le pire cas : un pillard (`pirate_corvette`, `combat.ron`) lancé à sa pleine vitesse,
        // 290 m/s (il attaque à 0,85 × cette vitesse, `space::ai_pilots`).
        let chaser = 290.0;
        // Avec la marge de 1 km : le verrou levé ne revient pas, la croisière distance le
        // pillard bien au-delà de la distance de fuite (15 km).
        let (relocks, gap) = flee(1_000.0, chaser);
        assert_eq!(relocks, 0);
        assert!(gap > 15_000.0, "{gap}");
        // Sans marge, il revient aussitôt : la croisière n'a pas le temps de monter.
        let (relocks, _) = flee(0.0, chaser);
        assert!(relocks >= 1);
    }
}
