//! Interface en jeu, minimale à pied (GDD §7) : réticule, invite d'interaction, messages
//! brefs, voile de chargement tant que la cellule du joueur n'est pas prête.

use bevy::prelude::*;

use crate::data::Texts;
use crate::sector::ActiveSector;
use crate::sector::streaming::{CellReady, CellStates};
use crate::state::ViewLevel;
use crate::ui::theme::{self, UiFonts};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Prompt>()
            .add_message::<Notice>()
            .add_systems(Update, spawn_hud.run_if(resource_added::<ActiveSector>))
            .add_systems(
                Update,
                (show_prompt, show_notices, show_veil).run_if(resource_exists::<ActiveSector>),
            )
            .add_systems(OnExit(ViewLevel::Sector), clear_prompt)
            .add_systems(OnExit(ViewLevel::Global), clear_prompt);
    }
}

/// Invite affichée sous le réticule (« E: Open door »).
#[derive(Resource, Default, Debug)]
pub struct Prompt(pub Option<String>);

/// Message bref en bas de l'écran.
#[derive(Message, Clone, Debug)]
pub struct Notice(pub String);

const NOTICE_SECONDS: f32 = 3.5;

#[derive(Component)]
struct PromptText;

#[derive(Component)]
struct NoticeText {
    remaining: f32,
}

#[derive(Component)]
struct LoadingVeil;

fn clear_prompt(mut prompt: ResMut<Prompt>) {
    prompt.0 = None;
}

fn spawn_hud(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    sector: Res<ActiveSector>,
) {
    let scope = sector.scope();
    // Réticule.
    commands.spawn((
        scope.clone(),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            top: percent(50),
            width: px(4),
            height: px(4),
            margin: UiRect::all(px(-2)),
            ..default()
        },
        BackgroundColor(theme::TEXT.with_alpha(0.8)),
    ));
    commands.spawn((
        scope.clone(),
        PromptText,
        Text::new(""),
        fonts.body(18.0),
        TextColor(theme::TEXT),
        TextShadow::default(),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            top: percent(54),
            ..default()
        },
        UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
    ));
    commands.spawn((
        scope.clone(),
        NoticeText { remaining: 0.0 },
        Text::new(""),
        fonts.body(18.0),
        TextColor(theme::ACCENT),
        TextShadow::default(),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            bottom: px(80),
            ..default()
        },
        UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
    ));
    commands.spawn((
        scope,
        LoadingVeil,
        theme::fullscreen_backdrop(),
        GlobalZIndex(100),
        children![(
            Text::new(texts.get("loading.title")),
            fonts.body(20.0),
            TextColor(theme::TEXT_DIM)
        )],
    ));
}

fn show_prompt(prompt: Res<Prompt>, mut text: Query<&mut Text, With<PromptText>>) {
    if !prompt.is_changed() {
        return;
    }
    for mut t in &mut text {
        t.0 = prompt.0.clone().unwrap_or_default();
    }
}

fn show_notices(
    time: Res<Time>,
    mut notices: MessageReader<Notice>,
    mut text: Query<(&mut Text, &mut NoticeText)>,
) {
    let latest = notices.read().last().cloned();
    for (mut t, mut notice) in &mut text {
        if let Some(Notice(message)) = &latest {
            t.0.clone_from(message);
            notice.remaining = NOTICE_SECONDS;
        } else if notice.remaining > 0.0 {
            notice.remaining -= time.delta_secs();
            if notice.remaining <= 0.0 {
                t.0.clear();
            }
        }
    }
}

/// Le voile couvre l'arrivée dans le secteur jusqu'à ce que la cellule du joueur soit prête.
fn show_veil(
    mut commands: Commands,
    states: Res<CellStates>,
    ready: Query<(), With<CellReady>>,
    veil: Query<Entity, With<LoadingVeil>>,
) {
    let current_ready = states
        .current
        .as_deref()
        .is_some_and(|c| states.is_local_ready(c, &ready));
    if current_ready {
        for entity in &veil {
            commands.entity(entity).despawn();
        }
    }
}
