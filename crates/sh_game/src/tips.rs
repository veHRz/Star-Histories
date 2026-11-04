//! Conseils (tutoriel léger, M7) : la première fois qu'une situation se présente (aux commandes,
//! à quai, à pied, sur la carte, un contrat, un raid, un abordage), un encart dit ce qu'on peut
//! y faire et avec quelles touches. Chaque conseil ne vient qu'une fois par partie (sauvegardé) ;
//! les options peuvent les couper. L'autopilote les coupe (captures propres), sauf `SH_TIPS=1`.
//! Pendant le tutoriel guidé (M8), ceux qu'il remplace se taisent (`tutorial::REPLACED_TIPS`).
//! Habillage (M8) : une plaquette à barre ambre, titre en capitales.

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
use crate::ui::theme::{self, Frame, UiFonts};

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

/// Place de l'encart : centré en haut, sous le bandeau du lieu (px).
const TIP_TOP: f32 = 112.0;
const TIP_WIDTH: f32 = 620.0;

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
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                // Sous le bandeau du lieu, même quand sa ligne des commandes passe à la ligne.
                top: px(TIP_TOP),
                left: percent(50),
                width: px(TIP_WIDTH),
                margin: UiRect::left(px(-TIP_WIDTH / 2.0)),
                flex_direction: FlexDirection::Column,
                // Large à gauche : la barre ambre de la plaquette.
                padding: UiRect::new(px(20), px(18), px(10), px(13)),
                row_gap: px(6),
                ..default()
            },
            fonts.frame(Frame::Plate),
            children![
                (
                    theme::heading(
                        fonts.as_ref(),
                        &texts.get("tips.title"),
                        14.0,
                        theme::ACCENT
                    ),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(texts.get(&format!("tips.{tip}"))),
                    fonts.body(16.0),
                    TextColor(theme::TEXT),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id();
    queue.shown = Some((entity, TIP_SECONDS));
}
