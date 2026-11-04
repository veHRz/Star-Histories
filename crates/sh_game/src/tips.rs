//! Conseils (tutoriel léger, M7) : la première fois qu'une situation se présente (aux commandes,
//! à quai, à pied, sur la carte, un contrat, un raid, un abordage), un encart dit ce qu'on peut
//! y faire et avec quelles touches. Chaque conseil ne vient qu'une fois par partie (sauvegardé) ;
//! les options peuvent les couper. L'autopilote les coupe (captures propres), sauf `SH_TIPS=1`.
//! Pendant le tutoriel guidé (M8), ceux qu'il remplace se taisent (`tutorial::REPLACED_TIPS`).

use std::collections::{BTreeSet, VecDeque};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::career::Career;
use crate::combat::Raid;
use crate::combat::foot::Fighter;
use crate::data::Texts;
use crate::options::UserSettings;
use crate::sector::Docked;
use crate::state::{AppState, ControlMode, ViewLevel};
use crate::ui::theme::{self, UiFonts};

pub struct TipsPlugin;

impl Plugin for TipsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TipsSeen>()
            .init_resource::<TipQueue>()
            .add_systems(
                Update,
                (detect, show)
                    .chain()
                    .run_if(in_state(AppState::InGame))
                    .run_if(resource_exists::<Career>),
            )
            .add_systems(OnExit(AppState::InGame), |mut queue: ResMut<TipQueue>| {
                *queue = TipQueue::default();
            });
    }
}

/// Les conseils déjà montrés dans cette partie.
#[derive(Resource, Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TipsSeen(pub BTreeSet<String>);

#[derive(Resource, Default)]
struct TipQueue {
    pending: VecDeque<&'static str>,
    /// L'encart affiché et le temps qu'il lui reste.
    shown: Option<(Entity, f32)>,
}

/// Secondes d'affichage d'un conseil.
const TIP_SECONDS: f32 = 12.0;

#[derive(Component)]
struct TipUi;

#[allow(clippy::too_many_arguments)]
fn detect(
    settings: Res<UserSettings>,
    mode: Res<State<ControlMode>>,
    view: Option<Res<State<ViewLevel>>>,
    docked: Option<Res<Docked>>,
    raid: Option<Res<Raid>>,
    fighters: Query<&Fighter>,
    career: Res<Career>,
    tutorial: Option<Res<crate::tutorial::Tutorial>>,
    time: Res<Time<Real>>,
    mut seen: ResMut<TipsSeen>,
    mut queue: ResMut<TipQueue>,
    mut held: Local<std::collections::HashMap<&'static str, f32>>,
) {
    if !settings.tips {
        return;
    }
    let view = view.map(|v| *v.get());
    let mode = *mode.get();
    let mut situations: Vec<&'static str> = Vec::new();
    if mode == ControlMode::Piloting && view == Some(ViewLevel::Global) {
        situations.extend(["helm", "dock"]);
    }
    if mode == ControlMode::OnFoot {
        situations.push(if docked.is_some() { "port" } else { "foot" });
    }
    if view == Some(ViewLevel::Galaxy) {
        situations.push("map");
    }
    if career
        .contracts
        .iter()
        .any(|c| c.state == sh_sim::ContractState::Active)
    {
        situations.push("contract");
    }
    if raid.is_some_and(|r| r.outcome.is_none()) {
        situations.push("raid");
    }
    if fighters.iter().any(Fighter::alive) {
        situations.push("boarding");
    }
    // Le tutoriel guide déjà ces situations : ses conseils attendent qu'il soit fini ou passé.
    if crate::tutorial::is_active(&settings, tutorial.as_deref()) {
        situations.retain(|tip| !crate::tutorial::REPLACED_TIPS.contains(tip));
    }
    // Une situation doit durer une seconde : au départ d'une partie, on est à pied l'instant
    // d'avant d'être assis aux commandes.
    held.retain(|tip, _| situations.contains(tip));
    for tip in situations {
        let since = held.entry(tip).or_default();
        *since += time.delta_secs();
        // `contains` d'abord : ne pas marquer la ressource modifiée à chaque frame.
        if *since >= SETTLE_SECONDS && !seen.0.contains(tip) {
            seen.0.insert(tip.to_owned());
            queue.pending.push_back(tip);
        }
    }
}

/// Durée minimale d'une situation avant son conseil.
const SETTLE_SECONDS: f32 = 1.0;

fn show(
    mut commands: Commands,
    time: Res<Time<Real>>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    mut queue: ResMut<TipQueue>,
) {
    if let Some((entity, left)) = queue.shown.as_mut() {
        *left -= time.delta_secs();
        if *left > 0.0 {
            return;
        }
        commands.entity(*entity).try_despawn();
        queue.shown = None;
    }
    let Some(tip) = queue.pending.pop_front() else {
        return;
    };
    let entity = commands
        .spawn((
            Name::new("Conseil"),
            TipUi,
            DespawnOnExit(AppState::InGame),
            GlobalZIndex(5),
            Node {
                position_type: PositionType::Absolute,
                top: px(96),
                left: percent(50),
                width: px(620),
                margin: UiRect::left(px(-310)),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(18), px(12)),
                border: UiRect::left(px(3)),
                row_gap: px(4),
                ..default()
            },
            BorderColor::all(theme::ACCENT),
            BackgroundColor(theme::PANEL),
            children![
                (
                    Text::new(texts.get("tips.title")),
                    fonts.title(16.0),
                    TextColor(theme::ACCENT),
                ),
                (
                    Text::new(texts.get(&format!("tips.{tip}"))),
                    fonts.body(16.0),
                    TextColor(theme::TEXT),
                ),
            ],
        ))
        .id();
    queue.shown = Some((entity, TIP_SECONDS));
}
