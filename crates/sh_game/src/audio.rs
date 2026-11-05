//! Le son (M7) : musiques, ambiances, effets, tous générés par `tools/sh_audio` (WAV, sans
//! licence extérieure). Ce module n'ajoute rien aux systèmes de jeu : il **observe** ce qui
//! apparaît (tirs, balles, éclairs, portes, séquences, coups reçus) et joue le son qui va avec,
//! atténué par la distance. Les volumes viennent des options (général × musique ou effets).
//!
//! Boucles (musique, ambiance, moteur, alarme) : une entité par boucle, dont le volume glisse
//! vers sa cible ; une musique qui change s'efface pendant que l'autre monte.

use std::collections::HashMap;

use bevy::audio::{AudioSinkPlayback, PlaybackMode, Volume};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::career::Career;
use crate::combat::Raid;
use crate::combat::foot::{Bullet, Fighter};
use crate::combat::space::{Flash, PlayerHit, Shot};
use crate::graphics::WorldCamera;
use crate::options::UserSettings;
use crate::sector::doors::{Door, DoorState};
use crate::sector::{ActiveSector, Docked};
use crate::ship::PilotIntent;
use crate::space::SpacePosition;
use crate::state::{AppState, ViewLevel};
use crate::transition::Sequence;
use crate::ui::hud::Notice;
use crate::ui::pause::Paused;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_sounds)
            .add_observer(on_click)
            .add_systems(
                Update,
                (
                    choose_loops,
                    fade_loops,
                    (
                        space_effects,
                        foot_effects,
                        door_effects,
                        sequence_effects,
                        hit_effects,
                        notice_effects,
                    ),
                )
                    .chain()
                    .run_if(resource_exists::<Sounds>),
            );
    }
}

#[derive(Resource)]
struct Sounds {
    music_space: Handle<AudioSource>,
    music_combat: Handle<AudioSource>,
    ambience_ship: Handle<AudioSource>,
    ambience_port: Handle<AudioSource>,
    engine: Handle<AudioSource>,
    alarm: Handle<AudioSource>,
    click: Handle<AudioSource>,
    notification: Handle<AudioSource>,
    door: Handle<AudioSource>,
    docking: Handle<AudioSource>,
    jump: Handle<AudioSource>,
    pistol: Handle<AudioSource>,
    impact: Handle<AudioSource>,
    cannon: Handle<AudioSource>,
    missile: Handle<AudioSource>,
    explosion: Handle<AudioSource>,
    hull: Handle<AudioSource>,
}

fn load_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    let load = |path: &str| assets.load(format!("audio/{path}.wav"));
    commands.insert_resource(Sounds {
        music_space: load("music/espace"),
        music_combat: load("music/combat"),
        ambience_ship: load("ambience/vaisseau"),
        ambience_port: load("ambience/spatioport"),
        engine: load("sfx/moteur"),
        alarm: load("sfx/alarme"),
        click: load("sfx/clic"),
        notification: load("sfx/notification"),
        door: load("sfx/porte"),
        docking: load("sfx/amarrage"),
        jump: load("sfx/saut"),
        pistol: load("sfx/pistolet"),
        impact: load("sfx/impact"),
        cannon: load("sfx/canon"),
        missile: load("sfx/missile"),
        explosion: load("sfx/explosion"),
        hull: load("sfx/coque"),
    });
}

// --- Boucles -----------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LoopKind {
    MusicSpace,
    MusicCombat,
    AmbienceShip,
    AmbiencePort,
    Engine,
    Alarm,
}

impl LoopKind {
    fn is_music(self) -> bool {
        matches!(self, LoopKind::MusicSpace | LoopKind::MusicCombat)
    }
}

/// Une boucle qui joue : son volume glisse vers `target` (0 : elle s'efface puis disparaît).
#[derive(Component)]
struct Loop {
    kind: LoopKind,
    current: f32,
    target: f32,
}

/// Les boucles voulues maintenant, et leur volume relatif.
fn choose_loops(
    mut commands: Commands,
    sounds: Res<Sounds>,
    state: Res<State<AppState>>,
    view: Option<Res<State<ViewLevel>>>,
    docked: Option<Res<Docked>>,
    raid: Option<Res<Raid>>,
    fighters: Query<&Fighter>,
    pilot: Option<Res<PilotIntent>>,
    career: Option<Res<Career>>,
    paused: Option<Res<Paused>>,
    mut loops: Query<&mut Loop>,
) {
    let in_game = *state.get() == AppState::InGame;
    let view = view.map(|v| *v.get());
    let raiding = raid.as_ref().is_some_and(|r| r.outcome.is_none());
    let boarded = fighters.iter().any(|f| f.alive());
    let mut wanted: HashMap<LoopKind, f32> = HashMap::new();
    // Musique : partout, celle du combat quand on se bat.
    let music = if in_game && (raiding || boarded) {
        LoopKind::MusicCombat
    } else {
        LoopKind::MusicSpace
    };
    wanted.insert(music, 1.0);
    if in_game {
        let ambience = if view == Some(ViewLevel::Sector) && docked.is_some() {
            LoopKind::AmbiencePort
        } else {
            LoopKind::AmbienceShip
        };
        wanted.insert(ambience, 0.55);
        // Le moteur, en vol : plus fort qu'on pousse ; rien en pause.
        if view == Some(ViewLevel::Global) && paused.is_none() {
            let push = pilot.as_ref().map_or(0.0, |p| {
                p.thrust.length().min(1.0) + if p.cruise { 0.6 } else { 0.0 }
            });
            wanted.insert(LoopKind::Engine, 0.25 + 0.55 * push.min(1.0));
        }
        // L'alarme, pendant un raid, dès qu'une section est ouverte.
        let breached = career
            .as_ref()
            .and_then(|c| c.hull.as_ref())
            .is_some_and(|h| h.sections.values().any(|s| s.breached()));
        if raiding && breached && paused.is_none() {
            wanted.insert(LoopKind::Alarm, 0.5);
        }
    }
    for mut lp in &mut loops {
        lp.target = wanted.remove(&lp.kind).unwrap_or(0.0);
    }
    for (kind, target) in wanted {
        let handle = match kind {
            LoopKind::MusicSpace => sounds.music_space.clone(),
            LoopKind::MusicCombat => sounds.music_combat.clone(),
            LoopKind::AmbienceShip => sounds.ambience_ship.clone(),
            LoopKind::AmbiencePort => sounds.ambience_port.clone(),
            LoopKind::Engine => sounds.engine.clone(),
            LoopKind::Alarm => sounds.alarm.clone(),
        };
        commands.spawn((
            Name::new(format!("Boucle {kind:?}")),
            AudioPlayer::new(handle),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::SILENT,
                ..default()
            },
            Loop {
                kind,
                current: 0.0,
                target,
            },
        ));
    }
}

/// Fondus : une musique change en deux secondes, le moteur suit la poussée en un quart.
fn fade_loops(
    mut commands: Commands,
    time: Res<Time<Real>>,
    settings: Res<UserSettings>,
    mut loops: Query<(Entity, &mut Loop, Option<&mut AudioSink>)>,
) {
    let dt = time.delta_secs();
    for (entity, mut lp, sink) in &mut loops {
        let speed = match lp.kind {
            LoopKind::Engine => 4.0,
            LoopKind::Alarm => 8.0,
            _ => 0.5,
        };
        let step = speed * dt;
        lp.current += (lp.target - lp.current).clamp(-step, step);
        if lp.target <= 0.0 && lp.current <= 0.001 {
            commands.entity(entity).despawn();
            continue;
        }
        let channel = if lp.kind.is_music() {
            settings.music_volume
        } else {
            settings.effects_volume
        };
        if let Some(mut sink) = sink {
            if sink.is_added() {
                info!("son : boucle {:?} en lecture", lp.kind);
            }
            sink.set_volume(Volume::Linear(
                lp.current * channel * settings.master_volume,
            ));
        }
    }
}

// --- Effets ------------------------------------------------------------------------------------

/// Joue un effet une fois, au volume relatif `gain` (atténuation, intensité).
fn play(commands: &mut Commands, settings: &UserSettings, handle: &Handle<AudioSource>, gain: f32) {
    let volume = gain * settings.effects_volume * settings.master_volume;
    if volume < 0.01 {
        return;
    }
    commands.spawn((
        AudioPlayer::new(handle.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
    ));
}

/// Atténuation par la distance : 1 à moins de `near` mètres, puis en 1/d².
fn falloff(distance: f64, near: f64) -> f32 {
    (1.0 / (1.0 + (distance / near).powi(2))) as f32
}

fn on_click(
    event: On<Activate>,
    mut commands: Commands,
    sounds: Option<Res<Sounds>>,
    settings: Res<UserSettings>,
    buttons: Query<(), With<bevy::ui_widgets::Button>>,
) {
    if let Some(sounds) = sounds
        && buttons.contains(event.entity)
    {
        play(&mut commands, &settings, &sounds.click, 0.6);
    }
}

/// Combat spatial : départs de tirs, impacts et explosions (éclairs), selon la distance au
/// vaisseau du joueur. Au plus un son de chaque genre par frame : dix coques tirent ensemble.
fn space_effects(
    mut commands: Commands,
    sounds: Res<Sounds>,
    settings: Res<UserSettings>,
    sector: Option<Res<ActiveSector>>,
    positions: Query<&SpacePosition>,
    shots: Query<(&Shot, &SpacePosition), Added<Shot>>,
    flashes: Query<(&Flash, &SpacePosition), Added<Flash>>,
) {
    let Some(me) = sector.and_then(|s| positions.get(s.root).ok()).map(|p| p.0) else {
        return;
    };
    let mut cannon = 0f32;
    let mut missile = 0f32;
    for (shot, pos) in &shots {
        let gain = falloff(pos.0.distance(me), 400.0);
        match shot.def.kind {
            sh_data::WeaponKind::Cannon => cannon = cannon.max(gain),
            _ => missile = missile.max(gain),
        }
    }
    let mut impact = 0f32;
    let mut explosion = 0f32;
    for (flash, pos) in &flashes {
        let near = if flash.size > 30.0 { 2500.0 } else { 600.0 };
        let gain = falloff(pos.0.distance(me), near);
        if flash.size > 30.0 {
            explosion = explosion.max(gain);
        } else {
            impact = impact.max(gain * 0.7);
        }
    }
    for (handle, gain) in [
        (&sounds.cannon, cannon * 0.7),
        (&sounds.missile, missile),
        (&sounds.impact, impact),
        (&sounds.explosion, explosion),
    ] {
        play(&mut commands, &settings, handle, gain);
    }
}

/// Combat à pied : coups de feu au départ des balles, impacts où elles s'arrêtent.
#[allow(clippy::too_many_arguments)]
fn foot_effects(
    mut commands: Commands,
    sounds: Res<Sounds>,
    settings: Res<UserSettings>,
    camera: Query<&GlobalTransform, With<WorldCamera>>,
    new_bullets: Query<&Transform, Added<Bullet>>,
    bullets: Query<(Entity, &Transform), With<Bullet>>,
    mut gone: RemovedComponents<Bullet>,
    mut last: Local<HashMap<Entity, Vec3>>,
) {
    let Ok(ear) = camera.single().map(GlobalTransform::translation) else {
        last.clear();
        return;
    };
    let mut shot = 0f32;
    for t in &new_bullets {
        shot = shot.max(falloff(f64::from(t.translation.distance(ear)), 6.0));
    }
    let mut impact = 0f32;
    for entity in gone.read() {
        if let Some(at) = last.remove(&entity) {
            impact = impact.max(falloff(f64::from(at.distance(ear)), 4.0) * 0.6);
        }
    }
    for (entity, t) in &bullets {
        last.insert(entity, t.translation);
    }
    play(&mut commands, &settings, &sounds.pistol, shot);
    play(&mut commands, &settings, &sounds.impact, impact);
}

/// Une porte se met en mouvement. Seulement si elle bouge vraiment : une fermeture demandée
/// sur une porte encore close (elle attendait sa cellule), ou une ouverture d'une porte déjà
/// ouverte, ne fait aucun bruit (M10 : la porte du sas « fermée » en vol jouait le son).
fn door_effects(
    mut commands: Commands,
    sounds: Res<Sounds>,
    settings: Res<UserSettings>,
    camera: Query<&GlobalTransform, With<WorldCamera>>,
    doors: Query<(Entity, &Door, &GlobalTransform), Changed<Door>>,
    mut last: Local<HashMap<Entity, DoorState>>,
) {
    let ear = camera.single().map(GlobalTransform::translation).ok();
    let mut gain = 0f32;
    for (entity, door, at) in &doors {
        let previous = last.insert(entity, door.state);
        let moving = match door.state {
            DoorState::Opening => door.openness < 1.0,
            DoorState::Closing => door.openness > 0.0,
            _ => false,
        };
        if moving && previous != Some(door.state) {
            let d = ear.map_or(0.0, |e| e.distance(at.translation()));
            gain = gain.max(falloff(f64::from(d), 5.0));
        }
    }
    play(&mut commands, &settings, &sounds.door, gain);
}

/// Amarrage, départ et voyage : le son de la manœuvre quand elle commence.
fn sequence_effects(
    mut commands: Commands,
    sounds: Res<Sounds>,
    settings: Res<UserSettings>,
    sequence: Option<Res<Sequence>>,
) {
    let Some(sequence) = sequence.filter(|s| s.is_added()) else {
        return;
    };
    let handle = match *sequence {
        Sequence::Docking { .. } | Sequence::Departing { .. } => &sounds.docking,
        Sequence::Travel { .. } => &sounds.jump,
        _ => return,
    };
    play(&mut commands, &settings, handle, 0.9);
}

/// Le vaisseau du joueur encaisse.
fn hit_effects(
    mut commands: Commands,
    sounds: Res<Sounds>,
    settings: Res<UserSettings>,
    mut hits: MessageReader<PlayerHit>,
) {
    let mut gain = 0f32;
    for hit in hits.read() {
        gain = gain.max(if hit.penetrated { 1.0 } else { 0.6 });
    }
    play(&mut commands, &settings, &sounds.hull, gain);
}

/// Un message à l'écran : un carillon discret.
fn notice_effects(
    mut commands: Commands,
    sounds: Res<Sounds>,
    settings: Res<UserSettings>,
    mut notices: MessageReader<Notice>,
) {
    if notices.read().count() > 0 {
        play(&mut commands, &settings, &sounds.notification, 0.35);
    }
}
