//! Rythme des images (M11) : limite d'images par seconde, synchronisation verticale, compteur
//! d'images pour le joueur, et mesures du rendu lues par la trace de l'autopilote (`dev`).
//!
//! **Où attendre.** Le limiteur attend dans le monde de rendu, au bout du programme `Render`
//! (`RenderSystems::PostCleanup`, après la présentation de l'image). Avec le rendu en pipeline
//! (`PipelinedRenderingPlugin`), le fil principal prépare l'image N+1 pendant que le fil du rendu
//! dessine l'image N, et l'extraction de N+1 attend que le rendu de N soit fini : une attente au
//! bout du rendu retient donc les deux fils. Elle passe aussi juste avant `send_time`
//! (`bevy_render`), qui envoie au monde principal l'instant de fin de l'image : le `Time<Real>`
//! du jeu (animations, physique, horloge) avance alors au pas régulier des échéances, et non au
//! gré du travail de chaque image. Une attente au début de la frame du fil principal laisserait
//! entre elle et la présentation toute la mise à jour du jeu, et son irrégularité.
//!
//! L'attente est un système **exclusif** : l'exécuteur le lance sur le fil qui exécute le
//! programme (le fil du rendu, `spawn_on_scope`), jamais sur un fil de la réserve de calcul que
//! le monde principal utilise au même moment.
//!
//! **Échéances** ([`Pacer`]) : un rythme fixe (échéance suivante = échéance + période), pas la
//! fin de l'image précédente, pour que les petits retards de réveil ne s'additionnent pas. Une
//! image en retard ne fait pas courir la suivante : le rythme repart d'elle.
//!
//! **Attente** : `std::thread::sleep` jusqu'à environ 1 ms de l'échéance, puis une courte attente
//! active. Sous Windows, la bibliothèque standard de Rust 1.99 dort sur un minuteur haute
//! résolution (`CreateWaitableTimerExW`, `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`, Windows 10
//! 1803 et plus ; vérifié dans les imports de sa `std`), sinon sur `Sleep`. Le dépassement observé
//! des réveils élargit la marge, jamais au-delà de [`MAX_MARGIN`] : sur le GPU intégré de la
//! machine de référence, un cœur qui tourne à vide prend de la puissance au GPU.
//!
//! **Synchronisation verticale** : `PresentMode::AutoVsync` ou `AutoNoVsync`, posé à chaud sur la
//! fenêtre (Bevy reconfigure la surface quand le mode change).
//!
//! Réglages du joueur : `options::UserSettings` (`fps_limit`, `vsync`, `show_fps`). L'autopilote
//! ne lit pas ces réglages : `SH_FPS_LIMIT=<n>` (0 : illimitée), `SH_VSYNC=1` et
//! `SH_FPS_COUNTER=1` (le compteur, pour une capture) décident pour lui, sans rien écrire dans le
//! fichier du joueur.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::render::render_asset::{
    RenderAssetBytesPerFrameLimiter, reset_render_asset_bytes_per_frame,
};
use bevy::render::render_resource::PipelineCache;
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::window::{PresentMode, PrimaryWindow};

use crate::data::Texts;
use crate::options::UserSettings;
use crate::ui::theme::{self, UiFonts};

pub struct FramePacePlugin;

impl Plugin for FramePacePlugin {
    fn build(&self, app: &mut App) {
        let shared = PaceShared::default();
        app.insert_resource(shared.clone())
            .insert_resource(PaceOverride::from_env())
            .init_resource::<FrameLimit>()
            .init_resource::<FpsCounter>()
            .add_systems(PostUpdate, apply_frame_limit)
            .add_systems(Update, fps_counter);
        // Sans rendu (tests, application sans fenêtre), rien à rythmer.
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(shared)
                .init_resource::<RenderPace>()
                .add_systems(First, mark_render_start)
                .add_systems(
                    Render,
                    read_render_stats
                        .in_set(RenderSystems::Cleanup)
                        .before(reset_render_asset_bytes_per_frame),
                )
                .add_systems(Render, pace_frame.in_set(RenderSystems::PostCleanup));
        }
    }
}

// --- Réglage ---------------------------------------------------------------------------------

/// Les choix de l'option « Limite d'images par seconde », en images par seconde ; 0 : illimitée.
pub const FPS_LIMITS: [u32; 11] = [0, 30, 40, 50, 60, 75, 90, 120, 144, 165, 240];
/// Bornes d'une limite écrite à la main (`SH_FPS_LIMIT`, fichier de réglages modifié).
pub const FPS_MIN: u32 = 10;
pub const FPS_MAX: u32 = 1000;

/// Rang d'une limite du plus lent au plus rapide : « illimitée » vient après 240.
fn speed_rank(fps: u32) -> u32 {
    if fps == 0 { u32::MAX } else { fps }
}

/// La limite suivante (`step` > 0 : plus d'images) ou précédente de l'option, bornée aux deux
/// bouts (30 et Illimitée). Une valeur hors de la liste (écrite à la main) passe à la plus proche
/// dans le sens demandé.
pub fn step_fps_limit(current: u32, step: i8) -> u32 {
    let mut ordered = FPS_LIMITS;
    ordered.sort_by_key(|fps| speed_rank(*fps));
    let rank = speed_rank(current);
    let found = match step.signum() {
        1 => ordered.iter().find(|fps| speed_rank(**fps) > rank),
        -1 => ordered.iter().rev().find(|fps| speed_rank(**fps) < rank),
        _ => None,
    };
    match (found, step.signum()) {
        (Some(fps), _) => *fps,
        // Déjà au bout : on y reste (une valeur écrite à la main va au bout le plus proche).
        (None, 1) => *ordered.last().unwrap_or(&0),
        (None, -1) if FPS_LIMITS.contains(&current) => current,
        (None, -1) => ordered[0],
        _ => current,
    }
}

/// La limite appliquée : celle du joueur, ou celle de l'autopilote (`SH_FPS_LIMIT`, `SH_VSYNC`).
/// Lue par la trace par frame (`dev`).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameLimit {
    /// Images par seconde au plus ; `None` : illimitée.
    pub fps: Option<u32>,
    pub vsync: bool,
}

impl FrameLimit {
    /// Une limite donnée en images par seconde (0 : illimitée), bornée à [`FPS_MIN`, `FPS_MAX`].
    pub fn new(fps: u32, vsync: bool) -> Self {
        Self {
            fps: (fps > 0).then(|| fps.clamp(FPS_MIN, FPS_MAX)),
            vsync,
        }
    }

    /// L'intervalle entre deux images, s'il y a une limite.
    pub fn period(self) -> Option<Duration> {
        self.fps
            .map(|fps| Duration::from_secs_f64(1.0 / f64::from(fps.max(1))))
    }

    pub fn present_mode(self) -> PresentMode {
        if self.vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        }
    }
}

/// `SH_FPS_LIMIT`, `SH_VSYNC` et `SH_FPS_COUNTER`, lus au lancement : ils l'emportent sur les
/// réglages du joueur (comme `SH_PRESET`), sans y être écrits.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PaceOverride {
    /// Images par seconde (0 : illimitée).
    fps: Option<u32>,
    vsync: Option<bool>,
    /// Le compteur d'images (captures de l'autopilote, qui ne lit pas les réglages du joueur).
    counter: Option<bool>,
}

impl PaceOverride {
    fn from_env() -> Self {
        let fps =
            std::env::var("SH_FPS_LIMIT")
                .ok()
                .and_then(|value| match parse_fps_limit(&value) {
                    Ok(fps) => Some(fps),
                    Err(message) => {
                        warn!("{message}");
                        None
                    }
                });
        let flag = |name: &str| std::env::var(name).ok().map(|v| v.trim() == "1");
        Self {
            fps,
            vsync: flag("SH_VSYNC"),
            counter: flag("SH_FPS_COUNTER"),
        }
    }

    /// La limite voulue : l'autopilote d'abord, puis le joueur, sinon illimitée sans vsync.
    fn resolve(self, settings: Option<&UserSettings>) -> FrameLimit {
        FrameLimit::new(
            self.fps
                .or(settings.map(|s| s.fps_limit))
                .unwrap_or_default(),
            self.vsync.or(settings.map(|s| s.vsync)).unwrap_or_default(),
        )
    }
}

/// `SH_FPS_LIMIT` : 0 (illimitée) ou un nombre d'images par seconde dans les bornes.
fn parse_fps_limit(value: &str) -> Result<u32, String> {
    match value.trim().parse::<u32>() {
        Ok(0) => Ok(0),
        Ok(fps) if (FPS_MIN..=FPS_MAX).contains(&fps) => Ok(fps),
        _ => Err(format!(
            "SH_FPS_LIMIT : « {value} » ignoré (0 pour illimitée, ou de {FPS_MIN} à {FPS_MAX})"
        )),
    }
}

/// Pose la limite et le mode de présentation quand le réglage change (et au premier frame).
fn apply_frame_limit(
    settings: Option<Res<UserSettings>>,
    env: Res<PaceOverride>,
    shared: Res<PaceShared>,
    mut limit: ResMut<FrameLimit>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut applied: Local<bool>,
) {
    let wanted = env.resolve(settings.as_deref());
    if *applied && wanted == *limit {
        return;
    }
    *applied = true;
    let vsync = if wanted.vsync { "avec" } else { "sans" };
    match wanted.fps {
        Some(fps) => info!("limite d'images : {fps} i/s, {vsync} synchronisation verticale"),
        None => info!("limite d'images : aucune, {vsync} synchronisation verticale"),
    }
    *limit = wanted;
    shared.set_period(wanted.period());
    let mode = wanted.present_mode();
    for mut window in &mut windows {
        if window.present_mode != mode {
            window.present_mode = mode;
        }
    }
}

// --- Mesures partagées avec le monde de rendu ------------------------------------------------

/// Ce que les deux mondes partagent : la période voulue (monde principal → rendu) et les mesures
/// de la dernière image rendue (rendu → trace de l'autopilote). Des atomiques : aucun verrou
/// entre le fil principal et le fil du rendu.
#[derive(Resource, Clone, Default)]
pub struct PaceShared(Arc<PaceStats>);

#[derive(Default)]
struct PaceStats {
    /// Période en nanosecondes ; 0 : pas de limite.
    period_ns: AtomicU64,
    /// Durée du programme de rendu (préparation, envoi au GPU, présentation), attente exclue.
    render_ns: AtomicU64,
    /// Attente du limiteur.
    wait_ns: AtomicU64,
    late: AtomicBool,
    pipelines: AtomicU32,
    pipelines_waiting: AtomicU32,
    /// Octets envoyés au GPU (maillages, images) ; compté seulement quand
    /// `RenderAssetBytesPerFrame` a une limite (la trace en pose une, très haute).
    upload_bytes: AtomicU64,
}

/// Les mesures de la dernière image rendue (en pipeline : une image derrière le monde principal).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderSample {
    pub render_ms: f64,
    pub wait_ms: f64,
    pub late: bool,
    pub pipelines: u32,
    pub pipelines_waiting: u32,
    pub upload_bytes: u64,
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

impl PaceShared {
    fn set_period(&self, period: Option<Duration>) {
        self.0
            .period_ns
            .store(period.map_or(0, nanos), Ordering::Relaxed);
    }

    fn period(&self) -> Option<Duration> {
        let ns = self.0.period_ns.load(Ordering::Relaxed);
        (ns > 0).then(|| Duration::from_nanos(ns))
    }

    pub fn sample(&self) -> RenderSample {
        let ms = |a: &AtomicU64| a.load(Ordering::Relaxed) as f64 / 1e6;
        RenderSample {
            render_ms: ms(&self.0.render_ns),
            wait_ms: ms(&self.0.wait_ns),
            late: self.0.late.load(Ordering::Relaxed),
            pipelines: self.0.pipelines.load(Ordering::Relaxed),
            pipelines_waiting: self.0.pipelines_waiting.load(Ordering::Relaxed),
            upload_bytes: self.0.upload_bytes.load(Ordering::Relaxed),
        }
    }
}

// --- Limiteur (monde de rendu) ---------------------------------------------------------------

/// Ce que le limiteur fait de l'image arrivée au bout du rendu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaceStep {
    /// Pas de limite.
    Free,
    /// Première image d'un rythme (limite posée ou changée) : elle part tout de suite.
    Start,
    /// En avance : attendre l'échéance.
    Wait(Instant),
    /// En retard : partir tout de suite ; le rythme repart de cette image.
    Late,
}

/// Les échéances du limiteur : un rythme fixe, recalé seulement sur une image en retard.
#[derive(Clone, Debug, Default)]
pub struct Pacer {
    period: Option<Duration>,
    deadline: Option<Instant>,
}

impl Pacer {
    /// L'image arrive à `now` au point d'attente ; `period` : la limite voulue.
    pub fn step(&mut self, now: Instant, period: Option<Duration>) -> PaceStep {
        if period != self.period {
            self.period = period;
            self.deadline = None;
        }
        let Some(period) = period else {
            return PaceStep::Free;
        };
        let (step, base) = match self.deadline {
            None => (PaceStep::Start, now),
            Some(deadline) if deadline > now => (PaceStep::Wait(deadline), deadline),
            // Rattraper le retard ferait courir les images suivantes (des à-coups dans l'autre
            // sens) : le rythme repart d'ici.
            Some(_) => (PaceStep::Late, now),
        };
        self.deadline = Some(base + period);
        step
    }
}

/// Marge d'attente active avant l'échéance : le sommeil du système s'arrête avant.
const SPIN_MARGIN: Duration = Duration::from_millis(1);
/// Marge maximale, même si les réveils sont très en retard (minuteur sans haute résolution) :
/// mieux vaut une image un peu en retard qu'un cœur qui tourne à vide.
pub const MAX_MARGIN: Duration = Duration::from_millis(3);

/// La marge d'attente active pour un dépassement de réveil estimé.
fn spin_margin(oversleep: Duration) -> Duration {
    (SPIN_MARGIN + oversleep).min(MAX_MARGIN)
}

/// Le dépassement de réveil estimé : il monte vite (un réveil tardif se répète souvent), il
/// redescend lentement.
fn next_oversleep(estimate: Duration, observed: Duration) -> Duration {
    let rate = if observed > estimate { 0.5 } else { 0.1 };
    (estimate.mul_f64(1.0 - rate) + observed.mul_f64(rate)).min(MAX_MARGIN)
}

/// Le sommeil du système avant `until`, en laissant `margin` à l'attente active.
fn sleep_duration(now: Instant, until: Instant, margin: Duration) -> Duration {
    until.saturating_duration_since(now).saturating_sub(margin)
}

/// Attend `until` : sommeil du système, puis attente active. Renvoie le dépassement du réveil.
fn wait_until(until: Instant, margin: Duration) -> Duration {
    let start = Instant::now();
    let sleep = sleep_duration(start, until, margin);
    let mut overshoot = Duration::ZERO;
    if !sleep.is_zero() {
        std::thread::sleep(sleep);
        overshoot = Instant::now().saturating_duration_since(start + sleep);
    }
    while Instant::now() < until {
        std::hint::spin_loop();
    }
    overshoot
}

/// État du limiteur, dans le monde de rendu.
#[derive(Resource, Default)]
struct RenderPace {
    pacer: Pacer,
    oversleep: Duration,
    /// Début du programme de rendu de l'image en cours.
    frame_start: Option<Instant>,
}

fn mark_render_start(mut pace: ResMut<RenderPace>) {
    pace.frame_start = Some(Instant::now());
}

/// Pipelines et octets envoyés au GPU : avant la remise à zéro du compteur d'octets.
fn read_render_stats(
    shared: Res<PaceShared>,
    limiter: Res<RenderAssetBytesPerFrameLimiter>,
    cache: Option<Res<PipelineCache>>,
) {
    let stats = &shared.0;
    stats.upload_bytes.store(
        limiter.bytes_written.load(Ordering::Relaxed) as u64,
        Ordering::Relaxed,
    );
    if let Some(cache) = cache {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        stats
            .pipelines
            .store(count(cache.pipelines().count()), Ordering::Relaxed);
        stats
            .pipelines_waiting
            .store(count(cache.waiting_pipelines().count()), Ordering::Relaxed);
    }
}

/// Le limiteur : exclusif, pour dormir sur le fil du rendu (voir l'en-tête du module).
fn pace_frame(world: &mut World) {
    let Some(shared) = world.get_resource::<PaceShared>().cloned() else {
        return;
    };
    let Some(mut pace) = world.get_resource_mut::<RenderPace>() else {
        return;
    };
    let now = Instant::now();
    if let Some(start) = pace.frame_start.take() {
        shared.0.render_ns.store(
            nanos(now.saturating_duration_since(start)),
            Ordering::Relaxed,
        );
    }
    let step = pace.pacer.step(now, shared.period());
    let waited = match step {
        PaceStep::Wait(until) => {
            let overshoot = wait_until(until, spin_margin(pace.oversleep));
            pace.oversleep = next_oversleep(pace.oversleep, overshoot);
            Instant::now().saturating_duration_since(now)
        }
        PaceStep::Free | PaceStep::Start | PaceStep::Late => Duration::ZERO,
    };
    shared.0.wait_ns.store(nanos(waited), Ordering::Relaxed);
    shared
        .0
        .late
        .store(step == PaceStep::Late, Ordering::Relaxed);
}

// --- Compteur d'images pour le joueur --------------------------------------------------------

/// Intervalle de rafraîchissement du compteur : réécrire un texte coûte cher sur ce GPU.
const COUNTER_REFRESH_SECONDS: f64 = 0.5;

/// Les images d'une fenêtre de mesure du compteur.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FpsWindow {
    frames: u32,
    elapsed: f64,
    worst: f64,
}

/// Une lecture du compteur : images par seconde, temps moyen et pire temps d'une image (ms).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FpsReading {
    pub fps: f64,
    pub mean_ms: f64,
    pub worst_ms: f64,
}

impl FpsWindow {
    /// Ajoute une image de `dt` secondes ; une lecture quand la fenêtre dépasse `refresh`
    /// secondes (la fenêtre repart alors de zéro).
    pub fn push(&mut self, dt: f64, refresh: f64) -> Option<FpsReading> {
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        self.frames += 1;
        self.elapsed += dt;
        self.worst = self.worst.max(dt);
        if self.elapsed < refresh {
            return None;
        }
        let frames = f64::from(self.frames);
        let reading = FpsReading {
            fps: frames / self.elapsed,
            mean_ms: self.elapsed * 1000.0 / frames,
            worst_ms: self.worst * 1000.0,
        };
        *self = Self::default();
        Some(reading)
    }
}

#[derive(Resource, Default)]
struct FpsCounter {
    window: FpsWindow,
}

/// Le texte du compteur, dans le coin bas droit (rien d'autre n'y est posé, voir le HUD).
#[derive(Component)]
struct FpsCounterText;

/// Le compteur suit l'option « Afficher les i/s » : il apparaît, se met à jour deux fois par
/// seconde, disparaît. Il n'appartient à aucun écran : menus et partie le gardent.
fn fps_counter(
    mut commands: Commands,
    time: Res<Time<Real>>,
    settings: Option<Res<UserSettings>>,
    env: Res<PaceOverride>,
    texts: Option<Res<Texts>>,
    fonts: Option<Res<UiFonts>>,
    mut counter: ResMut<FpsCounter>,
    mut shown: Query<(Entity, &mut Text), With<FpsCounterText>>,
) {
    let wanted = env
        .counter
        .or(settings.map(|s| s.show_fps))
        .unwrap_or_default();
    let (true, Some(texts), Some(fonts)) = (wanted, texts, fonts) else {
        for (entity, _) in &shown {
            commands.entity(entity).despawn();
        }
        counter.window = FpsWindow::default();
        return;
    };
    if shown.is_empty() {
        commands.spawn((
            Name::new("Compteur d'images"),
            FpsCounterText,
            Text::new(""),
            fonts.mono(13.0),
            TextColor(theme::TEXT_DIM),
            theme::label_shadow(),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
            GlobalZIndex(COUNTER_Z),
            Node {
                position_type: PositionType::Absolute,
                right: px(8),
                bottom: px(4),
                ..default()
            },
        ));
        counter.window = FpsWindow::default();
        return;
    }
    let Some(reading) = counter
        .window
        .push(time.delta_secs_f64(), COUNTER_REFRESH_SECONDS)
    else {
        return;
    };
    let line = texts.format(
        "options.fps.counter",
        &[
            ("fps", &texts.number(reading.fps, 0)),
            ("ms", &texts.number(reading.mean_ms, 1)),
            ("worst", &texts.number(reading.worst_ms, 1)),
        ],
    );
    for (_, mut text) in &mut shown {
        if text.0 != line {
            text.0.clone_from(&line);
        }
    }
}

/// Au-dessus de tous les écrans (options, pause, terminal) : un compteur qu'on a demandé reste
/// lisible partout.
const COUNTER_Z: i32 = 1_000;

#[cfg(test)]
mod tests {
    use super::*;

    const P: Duration = Duration::from_micros(16_667);

    #[test]
    fn choix_de_la_limite() {
        // Du plus lent au plus rapide, « illimitée » au bout ; bornes tenues.
        assert_eq!(step_fps_limit(0, -1), 240);
        assert_eq!(step_fps_limit(240, 1), 0);
        assert_eq!(step_fps_limit(0, 1), 0);
        assert_eq!(step_fps_limit(30, -1), 30);
        assert_eq!(step_fps_limit(60, 1), 75);
        assert_eq!(step_fps_limit(60, -1), 50);
        // Une valeur écrite à la main rejoint la liste dans le sens demandé.
        assert_eq!(step_fps_limit(45, 1), 50);
        assert_eq!(step_fps_limit(45, -1), 40);
        assert_eq!(step_fps_limit(20, -1), 30);
        assert_eq!(step_fps_limit(500, 1), 0);
        assert_eq!(step_fps_limit(500, -1), 240);
        // Toutes les valeurs de l'option se parcourent, dans les deux sens.
        let mut fps = 30;
        let mut seen = vec![fps];
        while fps != 0 {
            fps = step_fps_limit(fps, 1);
            seen.push(fps);
        }
        assert_eq!(seen.len(), FPS_LIMITS.len());
        assert!(FPS_LIMITS.iter().all(|f| seen.contains(f)));
    }

    #[test]
    fn lecture_de_sh_fps_limit() {
        assert_eq!(parse_fps_limit(" 60 "), Ok(60));
        assert_eq!(parse_fps_limit("0"), Ok(0));
        assert!(parse_fps_limit("5").is_err());
        assert!(parse_fps_limit("soixante").is_err());
        let env = PaceOverride {
            fps: Some(60),
            vsync: Some(true),
            counter: None,
        };
        let player = UserSettings {
            fps_limit: 144,
            ..UserSettings::default()
        };
        // L'autopilote l'emporte, sinon le joueur, sinon rien.
        assert_eq!(env.resolve(Some(&player)), FrameLimit::new(60, true));
        assert_eq!(
            PaceOverride::default().resolve(Some(&player)),
            FrameLimit::new(144, false)
        );
        assert_eq!(PaceOverride::default().resolve(None), FrameLimit::default());
        assert_eq!(FrameLimit::new(0, false).period(), None);
        assert_eq!(FrameLimit::new(5000, false).fps, Some(FPS_MAX));
        assert_eq!(
            FrameLimit::new(0, true).present_mode(),
            PresentMode::AutoVsync
        );
    }

    #[test]
    fn echeances_a_rythme_fixe() {
        let t0 = Instant::now();
        let mut pacer = Pacer::default();
        assert_eq!(pacer.step(t0, None), PaceStep::Free);
        // Première image : elle part, l'échéance suivante est une période plus loin.
        assert_eq!(pacer.step(t0, Some(P)), PaceStep::Start);
        // En avance : attendre l'échéance, quelle que soit l'heure d'arrivée.
        assert_eq!(
            pacer.step(t0 + Duration::from_millis(5), Some(P)),
            PaceStep::Wait(t0 + P)
        );
        assert_eq!(
            pacer.step(t0 + P + Duration::from_millis(12), Some(P)),
            PaceStep::Wait(t0 + 2 * P)
        );
        // En retard : on part, et la suivante a une période entière (pas de rattrapage).
        let late = t0 + 3 * P + Duration::from_millis(4);
        assert_eq!(pacer.step(late, Some(P)), PaceStep::Late);
        assert_eq!(
            pacer.step(late + Duration::from_millis(2), Some(P)),
            PaceStep::Wait(late + P)
        );
        // Nouvelle limite : nouveau rythme.
        let half = P * 2;
        assert_eq!(pacer.step(late + P, Some(half)), PaceStep::Start);
        assert_eq!(pacer.step(late + P, None), PaceStep::Free);
    }

    /// Mille images à l'heure, réveillées avec un peu de retard : les échéances ne dérivent pas.
    #[test]
    fn pas_de_derive() {
        let t0 = Instant::now();
        let mut pacer = Pacer::default();
        let mut now = t0;
        assert_eq!(pacer.step(now, Some(P)), PaceStep::Start);
        for k in 1..=1000u32 {
            // Réveil 0,2 ms après l'échéance, puis 10 ms de travail.
            now += Duration::from_millis(10);
            match pacer.step(now, Some(P)) {
                PaceStep::Wait(until) => {
                    assert_eq!(until, t0 + P * k);
                    now = until + Duration::from_micros(200);
                }
                other => panic!("image {k} : {other:?}"),
            }
        }
    }

    #[test]
    fn sommeil_et_marge() {
        let t0 = Instant::now();
        let until = t0 + Duration::from_millis(10);
        assert_eq!(
            sleep_duration(t0, until, SPIN_MARGIN),
            Duration::from_millis(9)
        );
        // Moins d'une marge avant l'échéance : attente active seule.
        assert_eq!(
            sleep_duration(t0 + Duration::from_micros(9_500), until, SPIN_MARGIN),
            Duration::ZERO
        );
        assert_eq!(sleep_duration(until, t0, SPIN_MARGIN), Duration::ZERO);
        // La marge suit les réveils tardifs, sans dépasser la borne.
        assert_eq!(spin_margin(Duration::ZERO), SPIN_MARGIN);
        assert_eq!(spin_margin(Duration::from_millis(10)), MAX_MARGIN);
        let up = next_oversleep(Duration::ZERO, Duration::from_millis(1));
        assert!(up.abs_diff(Duration::from_micros(500)) < Duration::from_micros(1));
        let down = next_oversleep(up, Duration::ZERO);
        assert!(down < up && down > Duration::ZERO);
        assert_eq!(
            next_oversleep(Duration::ZERO, Duration::from_secs(1)),
            MAX_MARGIN
        );
    }

    #[test]
    fn compteur_lisse() {
        let mut window = FpsWindow::default();
        // 20 images à 50 i/s (0,40 s), puis une lente de 50 ms (0,45 s) : la fenêtre de 0,5 s
        // n'est pas pleine. Aucun cumul ne tombe près de 0,5 s : pas de bord flottant.
        for _ in 0..20 {
            assert_eq!(window.push(0.02, 0.5), None);
        }
        assert_eq!(window.push(0.05, 0.5), None);
        // 0,47 s, 0,49 s : toujours rien ; 0,51 s : la lecture sort.
        assert_eq!(window.push(0.02, 0.5), None);
        assert_eq!(window.push(0.02, 0.5), None);
        let reading = window
            .push(0.02, 0.5)
            .unwrap_or_else(|| panic!("pas de lecture"));
        // Une image lente au milieu : la moyenne bouge peu, le pire la montre.
        assert!((reading.worst_ms - 50.0).abs() < 1e-9);
        assert!((reading.fps - 24.0 / 0.51).abs() < 1e-6, "{reading:?}");
        assert!((reading.mean_ms - 1000.0 / reading.fps).abs() < 1e-9);
        // La fenêtre repart de zéro ; une durée nulle ou absurde est ignorée.
        assert_eq!(window, FpsWindow::default());
        assert_eq!(window.push(0.0, 0.5), None);
        assert_eq!(window.push(f64::NAN, 0.5), None);
    }
}
