//! Pont entre la simulation macro (`sh_sim`) et le jeu (AGENTS.md §7.1) : le monde vivant.
//!
//! - La simulation suit l'horloge de jeu : chaque jour qui passe (×30 à bord, d'un coup pendant
//!   un voyage) est un pas de [`SimWorld`].
//! - Ses événements **publics** deviennent des nouvelles (fil à l'écran, console de
//!   communications) et nourrissent la connaissance du joueur ; les secrets n'en sortent pas.
//! - Les marchés du jeu lisent ses prix (terminal du marché du spatioport).
//! - [`PlayerKnowledge`] : ce que le joueur sait des flottes, et depuis quand (brouillard
//!   d'information de la carte). Les flottes secrètes n'y entrent jamais.

use std::collections::BTreeMap;

use bevy::prelude::*;
use sh_core::{Id, StellarDate};
use sh_data::travel::Endpoint;
use sh_data::{Fleet, Place};
use sh_sim::{EventKind, SimEvent, SimWorld};

use crate::clock::{GameClock, HUD_CORNER_WIDTH, HudCorner, format_date};
use crate::data::{Data, Texts};
use crate::state::{AppState, ViewLevel};
use crate::ui::theme::{self, Frame, UiFonts};
use crate::voyage::Location;

pub struct LivingWorldPlugin;

impl Plugin for LivingWorldPlugin {
    fn build(&self, app: &mut App) {
        // Le fil des nouvelles se range sous la date : la colonne qui les porte doit exister.
        app.add_systems(
            OnEnter(AppState::InGame),
            (start_world, spawn_news.after(crate::clock::spawn_date)).chain(),
        )
        .add_systems(
            Update,
            (follow_clock, observe_location, show_news)
                .chain()
                // Hors partie, il n'y a pas de vue à suivre.
                .run_if(in_state(AppState::InGame))
                .run_if(resource_exists::<LivingWorld>)
                .run_if(resource_exists::<GameClock>),
        );
    }
}

/// Le monde simulé, et le nombre d'événements déjà relus par le jeu.
#[derive(Resource)]
pub struct LivingWorld {
    pub sim: SimWorld,
    read: usize,
}

impl LivingWorld {
    /// Le monde d'une sauvegarde : `read` événements avaient déjà été relus par le jeu.
    pub fn restored(sim: SimWorld, read: usize) -> Self {
        Self { sim, read }
    }

    /// Événements déjà relus (nouvelles, connaissance du joueur).
    pub fn read(&self) -> usize {
        self.read
    }
}

/// Une flotte connue du joueur : où, et à quelle date l'information remonte.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KnownFleet {
    pub at: Id<Place>,
    pub as_of: StellarDate,
}

/// Ce que le joueur sait des flottes (jamais de flotte secrète).
#[derive(Resource, Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerKnowledge {
    pub fleets: BTreeMap<Id<Fleet>, KnownFleet>,
    /// Les dernières nouvelles publiques, de la plus ancienne à la plus récente.
    pub news: Vec<SimEvent>,
}

/// Nouvelles gardées en mémoire.
const NEWS_KEPT: usize = 30;
/// Nouvelles affichées dans le fil.
const NEWS_SHOWN: usize = 3;

fn start_world(mut commands: Commands, data: Res<Data>, world: Option<Res<LivingWorld>>) {
    if world.is_some() {
        return;
    }
    let sim = SimWorld::from_data(&data, None);
    let knowledge = initial_knowledge(&sim);
    commands.insert_resource(knowledge);
    commands.insert_resource(LivingWorld { sim, read: 0 });
}

/// La simulation rattrape l'horloge ; les nouveaux événements publics sont relus.
fn follow_clock(
    clock: Res<GameClock>,
    mut world: ResMut<LivingWorld>,
    mut knowledge: ResMut<PlayerKnowledge>,
) {
    if world.sim.date() >= clock.date {
        return;
    }
    let start = std::time::Instant::now();
    let days = world.sim.date().days_until(clock.date);
    world.sim.advance_to(clock.date);
    if days > 1 {
        debug!(
            "monde vivant : {days} jours simulés en {:.1} ms",
            start.elapsed().as_secs_f64() * 1e3
        );
    }
    let fresh: Vec<SimEvent> = world.sim.history()[world.read..].to_vec();
    world.read = world.sim.history().len();
    absorb(&mut knowledge, &world.sim, &fresh);
}

/// Au départ, le joueur connaît les positions publiques du canon (§6.1), datées du jour 1.
pub fn initial_knowledge(sim: &SimWorld) -> PlayerKnowledge {
    let mut knowledge = PlayerKnowledge::default();
    for (id, fleet) in sim.fleets.iter().filter(|(_, f)| !f.secret) {
        knowledge.fleets.insert(
            id.clone(),
            KnownFleet {
                at: fleet.location.clone(),
                as_of: sim.date(),
            },
        );
    }
    knowledge
}

/// Ce que les nouvelles apprennent au joueur : seulement les événements publics, et jamais
/// rien d'une flotte secrète.
pub fn absorb(knowledge: &mut PlayerKnowledge, sim: &SimWorld, events: &[SimEvent]) {
    for event in events.iter().filter(|e| e.public) {
        if let EventKind::FleetArrived { fleet, at } = &event.kind
            && sim.fleets.get(fleet).is_some_and(|f| !f.secret)
        {
            knowledge.fleets.insert(
                fleet.clone(),
                KnownFleet {
                    at: at.clone(),
                    as_of: event.date,
                },
            );
        }
        knowledge.news.push(event.clone());
    }
    let excess = knowledge.news.len().saturating_sub(NEWS_KEPT);
    knowledge.news.drain(..excess);
}

/// Sur place, on voit les flottes présentes (vue Globale d'un système, ou en orbite d'un monde
/// isolé depuis M9 : la 3ème Flotte à Karza).
fn observe_location(
    view: Res<State<ViewLevel>>,
    location: Option<Res<Location>>,
    data: Res<Data>,
    world: Res<LivingWorld>,
    mut knowledge: ResMut<PlayerKnowledge>,
) {
    let (ViewLevel::Global, Some(location)) = (view.get(), location) else {
        return;
    };
    // Les lieux visibles d'ici : le système, ses astres et ses stations ; ou le monde isolé, sa
    // planète (pour une lune) et les lunes de celle-ci, tous dans la même scène.
    let visible: Vec<&Id<Place>> = match location.endpoint(&data) {
        Some(Endpoint::System(system)) => std::iter::once(&system.id)
            .chain(&system.bodies)
            .chain(&system.stations)
            .collect(),
        Some(Endpoint::Isolated(body)) => {
            let root = sh_data::travel::isolated_root(&data.galaxy, body);
            std::iter::once(&root.id)
                .chain(
                    data.galaxy
                        .bodies
                        .iter()
                        .filter(|b| !b.secret && b.parent.as_ref() == Some(&root.id))
                        .map(|b| &b.id),
                )
                .collect()
        }
        None => return,
    };
    let date = world.sim.date();
    for (id, fleet) in &world.sim.fleets {
        let here = visible.contains(&&fleet.location);
        if fleet.secret || fleet.transit.is_some() || !here {
            continue;
        }
        let known = KnownFleet {
            at: fleet.location.clone(),
            as_of: date,
        };
        if knowledge.fleets.get(id) != Some(&known) {
            knowledge.fleets.insert(id.clone(), known);
        }
    }
}

/// Nom affichable d'un identifiant (lieu, flotte, faction) : les noms propres du canon.
pub fn display_name(data: &Data, id: &str) -> String {
    let place = Id::<Place>::new(id).ok();
    data.galaxy
        .fleets
        .iter()
        .find(|f| f.id.as_str() == id)
        .map(|f| f.name.clone())
        .or_else(|| {
            data.factions
                .factions
                .iter()
                .find(|f| f.id.as_str() == id)
                .map(|f| f.name.clone())
        })
        .or_else(|| {
            place
                .as_ref()
                .and_then(|p| data.galaxy.system(p).map(|s| s.name.clone()))
        })
        .or_else(|| {
            place
                .as_ref()
                .and_then(|p| data.galaxy.body(p).map(|b| b.name.clone()))
        })
        .or_else(|| {
            place
                .as_ref()
                .and_then(|p| data.galaxy.station(p).map(|s| s.name.clone()))
        })
        .unwrap_or_else(|| id.to_owned())
}

/// Une nouvelle, en anglais (clés `news.*`), noms propres en français.
pub fn news_line(texts: &Texts, data: &Data, event: &SimEvent) -> String {
    let n = |id: &str| display_name(data, id);
    let commodity = |c: &str| texts.get(&format!("commodity.{c}"));
    let line = match &event.kind {
        EventKind::FleetArrived { fleet, at } => texts.format(
            "news.fleet_arrived",
            &[("fleet", &n(fleet.as_str())), ("place", &n(at.as_str()))],
        ),
        EventKind::FleetRepaired { fleet } => {
            texts.format("news.fleet_repaired", &[("fleet", &n(fleet.as_str()))])
        }
        EventKind::ConvoyRaided {
            route,
            owner,
            raiders,
            ..
        } => texts.format(
            "news.convoy_raided",
            &[
                ("raiders", raiders),
                ("owner", &n(owner.as_str())),
                ("from", &n(route.0.as_str())),
                ("to", &n(route.1.as_str())),
            ],
        ),
        EventKind::ConvoyDefended {
            route,
            raiders,
            escort,
            ..
        } => texts.format(
            "news.convoy_defended",
            &[
                ("escort", &n(escort.as_str())),
                ("raiders", raiders),
                ("to", &n(route.1.as_str())),
            ],
        ),
        EventKind::Skirmish { winner, loser, at } => texts.format(
            "news.skirmish",
            &[
                ("place", &n(at.as_str())),
                ("winner", &n(winner.as_str())),
                ("loser", &n(loser.as_str())),
            ],
        ),
        EventKind::Shortage { at, commodity: c } => texts.format(
            "news.shortage",
            &[("place", &n(at.as_str())), ("commodity", &commodity(c))],
        ),
        EventKind::Resupplied { at, commodity: c } => texts.format(
            "news.resupplied",
            &[("place", &n(at.as_str())), ("commodity", &commodity(c))],
        ),
        EventKind::Revolt { at } => texts.format("news.revolt", &[("place", &n(at.as_str()))]),
        EventKind::ProjectMilestone { name, percent, .. } if *percent >= 100 => {
            texts.format("news.project_done", &[("name", name)])
        }
        EventKind::ProjectMilestone { name, percent, .. } => texts.format(
            "news.project_progress",
            &[("name", name), ("percent", &percent.to_string())],
        ),
        EventKind::Arena { at } => texts.format("news.arena", &[("place", &n(at.as_str()))]),
        EventKind::UglCampaign { at } => {
            texts.format("news.ugl_campaign", &[("place", &n(at.as_str()))])
        }
    };
    format!("{} — {line}", format_date(texts, event.date))
}

/// Les dernières nouvelles, pour la console de communications.
pub fn latest_news(
    texts: &Texts,
    data: &Data,
    knowledge: &PlayerKnowledge,
    count: usize,
) -> String {
    let lines: Vec<String> = knowledge
        .news
        .iter()
        .rev()
        .take(count)
        .map(|e| news_line(texts, data, e))
        .collect();
    if lines.is_empty() {
        texts.get("news.none")
    } else {
        lines.join("\n")
    }
}

/// Les prix d'un marché, pour le terminal du marché.
pub fn market_board(texts: &Texts, data: &Data, world: &LivingWorld, system: &Id<Place>) -> String {
    let mut parts = Vec::new();
    for commodity in data.sim.base_prices.keys() {
        if let Some(price) = world.sim.price(system, commodity) {
            parts.push(texts.format(
                "market.price",
                &[
                    ("commodity", &texts.get(&format!("commodity.{commodity}"))),
                    ("price", &format!("{price:.0}")),
                ],
            ));
        }
    }
    if parts.is_empty() {
        texts.get("console.market.message")
    } else {
        texts.format(
            "market.board",
            &[
                ("place", &display_name(data, system.as_str())),
                ("prices", &parts.join("  ·  ")),
            ],
        )
    }
}

/// La plaquette du fil des nouvelles (cachée sur la carte, qui a son propre panneau).
#[derive(Component)]
struct NewsTicker;

#[derive(Component)]
struct NewsText;

/// Le fil se range dans la colonne du coin haut droit, sous la date (`clock::HudCorner`, créée
/// juste avant : voir l'ordre dans le plugin).
fn spawn_news(mut commands: Commands, fonts: Res<UiFonts>, corner: Query<Entity, With<HudCorner>>) {
    let mut node = Node {
        max_width: px(HUD_CORNER_WIDTH),
        // Large à gauche : la barre ambre de la plaquette.
        padding: UiRect::new(px(14), px(12), px(6), px(7)),
        ..default()
    };
    let corner = corner.iter().next();
    if corner.is_none() {
        // Sans la colonne (ne devrait pas arriver), le fil se place seul, sous la date.
        node.position_type = PositionType::Absolute;
        node.right = px(20);
        node.top = px(60);
    }
    let plate = commands
        .spawn((
            Name::new("Fil des nouvelles"),
            NewsTicker,
            DespawnOnExit(AppState::InGame),
            Pickable::IGNORE,
            node,
            fonts.frame(Frame::Plate),
            children![(
                NewsText,
                Text::new(""),
                fonts.body(14.0),
                TextColor(theme::TEXT_DIM),
                Pickable::IGNORE,
            )],
        ))
        .id();
    if let Some(corner) = corner {
        commands.entity(plate).insert(ChildOf(corner));
    }
}

fn show_news(
    texts: Res<Texts>,
    data: Res<Data>,
    view: Res<State<ViewLevel>>,
    knowledge: Res<PlayerKnowledge>,
    mut plates: Query<&mut Node, With<NewsTicker>>,
    mut ticker: Query<&mut Text, With<NewsText>>,
) {
    // Le fil des nouvelles, à bord et à quai (la carte a son propre panneau). Caché, il rend sa
    // place dans la colonne : le panneau du tutoriel remonte sous la date.
    let shown = matches!(view.get(), ViewLevel::Global | ViewLevel::Sector);
    let display = if shown { Display::Flex } else { Display::None };
    for mut node in &mut plates {
        if node.display != display {
            node.display = display;
        }
    }
    for mut text in &mut ticker {
        if knowledge.is_changed() || text.0.is_empty() {
            let lines = latest_news(&texts, &data, &knowledge, NEWS_SHOWN);
            if text.0 != lines {
                text.0 = lines;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn la_connaissance_du_joueur_n_a_aucun_secret() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
        let data = sh_data::GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"));
        let mut sim = SimWorld::from_data(&data, Some(11));
        let mut knowledge = initial_knowledge(&sim);
        sim.advance_days(730);
        absorb(&mut knowledge, &sim, sim.history());
        assert!(!knowledge.fleets.is_empty());
        for id in knowledge.fleets.keys() {
            let fleet = data.galaxy.fleets.iter().find(|f| &f.id == id);
            assert!(
                fleet.is_some_and(|f| !f.secret),
                "flotte secrète connue : {id}"
            );
        }
        assert!(knowledge.news.iter().all(|e| e.public));
        // Les nouvelles datent l'information : la Flotte de l'Aube patrouille, sa position bouge.
        let aube = Id::new("flotte_aube").unwrap_or_else(|e| panic!("{e}"));
        assert!(
            knowledge
                .fleets
                .get(&aube)
                .is_some_and(|k| k.as_of > StellarDate::start_of(sh_core::Year(425)))
        );
    }
}
