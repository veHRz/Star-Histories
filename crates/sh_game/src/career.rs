//! Le capitaine du joueur en jeu (M5) : la fiche de `sh_sim::career`, tenue dans une ressource.
//!
//! - Elle suit l'horloge : échéances de la Banque et délais des contrats tombent jour après
//!   jour (y compris pendant un voyage).
//! - Chaque résultat ([`Outcome`]) devient un message à l'écran et une ligne du journal de
//!   missions.
//! - En vue Globale, la touche d'amarrage livre les contrats du système (remorqueurs
//!   d'Hexa-Corp, à portée d'un hub ou de chantiers) et accroche l'épave d'un remorquage.

use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use sh_core::{Credits, Id, SpacePos, StellarDate};
use sh_data::{Faction, Place, StructureKind};
use sh_sim::{Captain, ContractKind, ContractState, Outcome, SimCommand};

use crate::clock::{GameClock, format_date};
use crate::data::{Data, Texts};
use crate::living::{LivingWorld, display_name};
use crate::space::{SpacePosition, SpaceRotation};
use crate::state::{AppState, ViewLevel};
use crate::ui::hud::Notice;
use crate::ui::theme::{self, UiFonts};
use crate::views::WorldLabel;
use crate::voyage::Location;

pub struct CareerPlugin;

impl Plugin for CareerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CareerOutcomes>()
            .init_resource::<MissionLog>()
            .add_systems(
                Update,
                start_career
                    .run_if(in_state(AppState::InGame))
                    .run_if(not(resource_exists::<Career>))
                    .run_if(resource_exists::<GameClock>),
            )
            .add_systems(
                Update,
                (tick_career, report_outcomes)
                    .chain()
                    .in_set(CareerSystems)
                    .run_if(resource_exists::<Career>)
                    .run_if(resource_exists::<GameClock>),
            )
            .add_systems(
                Update,
                practice_flight
                    .in_set(CareerSystems)
                    .before(report_outcomes)
                    .run_if(resource_exists::<Career>),
            )
            .add_systems(
                Update,
                spawn_wrecks
                    .run_if(in_state(ViewLevel::Global))
                    .run_if(resource_exists::<Career>),
            );
    }
}

/// Échéances et annonces de la carrière (le terminal passe après).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CareerSystems;

/// La fiche du capitaine du joueur.
#[derive(Resource, Deref, DerefMut)]
pub struct Career {
    #[deref]
    pub captain: Captain,
    /// Dernier jour traité (échéances, délais).
    last_tick: StellarDate,
}

impl Career {
    /// La fiche d'une sauvegarde.
    pub fn restored(captain: Captain, last_tick: StellarDate) -> Self {
        Self { captain, last_tick }
    }

    pub fn last_tick(&self) -> StellarDate {
        self.last_tick
    }
}

/// Ce qu'une action du joueur a déclenché, à annoncer.
#[derive(Message, Clone, Debug)]
pub struct CareerOutcomes(pub Vec<Outcome>);

/// Le journal de missions : ce qui est arrivé, daté, du plus ancien au plus récent.
#[derive(Resource, Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MissionLog {
    pub lines: Vec<String>,
}

/// Un point de livraison en orbite (hub de Portails, chantiers, dépôt de fret ; M10 : tout
/// système à économie en a un, P32) : les remorqueurs d'Hexa-Corp y prennent la cargaison des
/// contrats. Le repère de destination (`ui::flight_hud`) montre le plus proche.
#[derive(Component, Clone, Copy, Debug)]
pub struct CargoPoint(pub StructureKind);

/// L'épave d'un contrat de remorquage, en attente d'être accrochée.
#[derive(Component, Clone, Debug)]
pub struct Wreck {
    pub contract: String,
}

/// Lignes gardées dans le journal.
const LOG_KEPT: usize = 40;

fn start_career(mut commands: Commands, data: Res<Data>, clock: Res<GameClock>) {
    commands.insert_resource(Career {
        captain: Captain::from_start(&data, clock.date),
        last_tick: clock.date,
    });
}

fn tick_career(
    data: Res<Data>,
    clock: Res<GameClock>,
    mut career: ResMut<Career>,
    mut outcomes: MessageWriter<CareerOutcomes>,
) {
    if clock.date <= career.last_tick {
        return;
    }
    // Jour par jour : un voyage de trois mois manque trois échéances, pas une.
    let mut out = Vec::new();
    let mut day = career.last_tick;
    while day < clock.date {
        day = day.add_days(1);
        out.extend(career.captain.tick(&data, day));
    }
    career.last_tick = clock.date;
    if !out.is_empty() {
        outcomes.write(CareerOutcomes(out));
    }
}

fn report_outcomes(
    texts: Res<Texts>,
    data: Res<Data>,
    clock: Res<GameClock>,
    mut reader: MessageReader<CareerOutcomes>,
    mut log: ResMut<MissionLog>,
    mut notices: MessageWriter<Notice>,
) {
    for CareerOutcomes(outcomes) in reader.read() {
        let lines: Vec<String> = outcomes
            .iter()
            .map(|o| outcome_text(&texts, &data, o))
            .collect();
        if lines.is_empty() {
            continue;
        }
        let date = format_date(&texts, clock.date);
        for line in &lines {
            info!("carrière : {line}");
            log.lines.push(format!("{date} — {line}"));
        }
        let excess = log.lines.len().saturating_sub(LOG_KEPT);
        log.lines.drain(..excess);
        notices.write(Notice(lines.join("\n")));
    }
}

/// Un montant en crédits pour l'interface (« 12 500 cr »).
pub fn money(texts: &Texts, amount: Credits) -> String {
    let units = amount.cents.div_euclid(100) as f64;
    texts.format("money.credits", &[("amount", &texts.number(units, 0))])
}

/// Le nom d'une organisation ; une division porte celui de sa maison mère (« Hexa-Corp
/// Logistique »).
pub fn faction_name(texts: &Texts, data: &Data, id: &Id<Faction>) -> String {
    let name = display_name(data, id.as_str());
    match data.faction(id).and_then(|f| f.parent.as_ref()) {
        Some(parent) => texts.format(
            "faction.division",
            &[
                ("parent", &display_name(data, parent.as_str())),
                ("name", &name),
            ],
        ),
        None => name,
    }
}

/// Le titre d'un contrat (« CT — weapons to Cygnara »).
pub fn contract_title(texts: &Texts, data: &Data, c: &sh_sim::Contract) -> String {
    let to = display_name(data, c.to.as_str());
    match (&c.kind, &c.commodity) {
        (ContractKind::Towing, _) | (_, None) => {
            texts.format("contract.title.towing", &[("code", c.kind.code())])
        }
        (_, Some(commodity)) => texts.format(
            "contract.title.cargo",
            &[
                ("code", c.kind.code()),
                ("amount", &format!("{:.0}", c.amount)),
                ("commodity", &texts.get(&format!("commodity.{commodity}"))),
                ("place", &to),
            ],
        ),
    }
}

pub fn outcome_text(texts: &Texts, data: &Data, outcome: &Outcome) -> String {
    let title = |id: &str| id.split('-').next().unwrap_or(id).to_owned();
    match outcome {
        Outcome::ContractAccepted { id } => {
            texts.format("outcome.contract_accepted", &[("code", &title(id))])
        }
        Outcome::ContractPaid { id, reward } => texts.format(
            "outcome.contract_paid",
            &[("code", &title(id)), ("reward", &money(texts, *reward))],
        ),
        Outcome::ContractFailed { id } => {
            texts.format("outcome.contract_failed", &[("code", &title(id))])
        }
        Outcome::ContractBetrayed { id, indemnity } => texts.format(
            "outcome.contract_betrayed",
            &[
                ("code", &title(id)),
                ("indemnity", &money(texts, *indemnity)),
            ],
        ),
        Outcome::ReputationChanged { faction, delta } => texts.format(
            "outcome.reputation",
            &[
                ("faction", &faction_name(texts, data, faction)),
                ("delta", &format!("{delta:+}")),
            ],
        ),
        Outcome::Promoted { faction, rank } => texts.format(
            "outcome.promoted",
            &[
                ("faction", &faction_name(texts, data, faction)),
                ("rank", &rank_name(texts, data, faction, *rank)),
            ],
        ),
        Outcome::Demoted { faction, rank } => texts.format(
            "outcome.demoted",
            &[
                ("faction", &faction_name(texts, data, faction)),
                ("rank", &rank_name(texts, data, faction, *rank)),
            ],
        ),
        Outcome::Dismissed { faction } => texts.format(
            "outcome.dismissed",
            &[("faction", &faction_name(texts, data, faction))],
        ),
        Outcome::Expelled { faction } => texts.format(
            "outcome.expelled",
            &[("faction", &faction_name(texts, data, faction))],
        ),
        Outcome::TraitEarned { id } => texts.format(
            "outcome.trait",
            &[("trait", &texts.get(&format!("trait.{id}")))],
        ),
        Outcome::SkillUp { skill, level } => texts.format(
            "outcome.skill_up",
            &[
                ("skill", &texts.get(&format!("skill.{skill}"))),
                ("level", &level.to_string()),
            ],
        ),
        Outcome::InstallmentPaid { balance } => texts.format(
            "outcome.installment_paid",
            &[("balance", &money(texts, *balance))],
        ),
        Outcome::InstallmentMissed { penalty } => texts.format(
            "outcome.installment_missed",
            &[("penalty", &money(texts, *penalty))],
        ),
        Outcome::LoanRepaid => texts.get("outcome.loan_repaid"),
        Outcome::Seized { amount } => {
            texts.format("outcome.seized", &[("amount", &money(texts, *amount))])
        }
    }
}

pub fn rank_name(texts: &Texts, data: &Data, faction: &Id<Faction>, rank: usize) -> String {
    data.ranks
        .ladders
        .get(faction)
        .and_then(|l| l.get(rank))
        .map_or_else(String::new, |r| texts.get(&r.key))
}

/// Un contrat actif peut-il finir dans ce système (même règle que [`deliver_here`]) ? Le
/// repère de destination ne se montre que dans ce cas.
pub fn deliverable_here(captain: &Captain, system: &Id<Place>) -> bool {
    captain.contracts.iter().any(|c| {
        c.state == ContractState::Active
            && &c.to == system
            && (c.kind != ContractKind::Towing || c.towed)
    })
}

/// Livre ce qui peut l'être dans ce système (vue Globale, touche d'amarrage) : contrats arrivés
/// à destination, épave remorquée jusqu'aux chantiers. Renvoie `true` si quelque chose a été
/// livré.
pub fn deliver_here(world: &mut World, system: &Id<Place>) -> bool {
    let today = world.resource::<GameClock>().date;
    let data = world.resource::<Data>().clone();
    let Some(mut career) = world.get_resource_mut::<Career>() else {
        return false;
    };
    let due: Vec<(String, Option<String>, f64)> = career
        .contracts
        .iter()
        .filter(|c| c.state == ContractState::Active && &c.to == system)
        // Même règle que `deliverable_here` : une épave se livre une fois accrochée.
        .filter(|c| c.kind != ContractKind::Towing || c.towed)
        .map(|c| (c.id.clone(), c.commodity.clone(), c.amount))
        .collect();
    let mut outcomes = Vec::new();
    let mut commands = Vec::new();
    for (id, commodity, amount) in due {
        match career.captain.deliver(&data, &id, system, today) {
            Ok(out) => {
                if let Some(commodity) = commodity {
                    commands.push(SimCommand::Deliver {
                        at: system.clone(),
                        commodity,
                        amount,
                    });
                }
                outcomes.extend(out);
            }
            Err(err) => warn!("livraison de {id} impossible : {err}"),
        }
    }
    if outcomes.is_empty() {
        return false;
    }
    if let Some(mut living) = world.get_resource_mut::<LivingWorld>() {
        for command in &commands {
            living.sim.apply(command);
        }
    }
    world.write_message(CareerOutcomes(outcomes));
    true
}

/// Fait apparaître, près des chantiers du système, l'épave des remorquages en cours (une
/// épave de cargo : mise en scène, sans fiche canon).
fn spawn_wrecks(
    mut commands: Commands,
    data: Res<Data>,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    asset_server: Res<AssetServer>,
    career: Res<Career>,
    location: Option<Res<Location>>,
    points: Query<(&CargoPoint, &SpacePosition)>,
    wrecks: Query<&Wreck>,
) {
    let Some(location) = location else {
        return;
    };
    let Some(yard) = points
        .iter()
        .find(|(p, _)| p.0 == StructureKind::Shipyard)
        .map(|(_, pos)| pos.0)
    else {
        return;
    };
    for contract in career.contracts.iter().filter(|c| {
        c.kind == ContractKind::Towing
            && c.state == ContractState::Active
            && !c.towed
            && c.from == location.system
    }) {
        if wrecks.iter().any(|w| w.contract == contract.id) {
            continue;
        }
        // Un peu au-dessus du plan, loin des cales : il faut aller la chercher.
        let d = data.career.contracts.wreck_distance;
        let pos = yard + SpacePos::new(d * 0.8, d * 0.15, -d * 0.58);
        let name = texts.get("wreck.name");
        let entity = commands
            .spawn((
                Name::new(name.clone()),
                Wreck {
                    contract: contract.id.clone(),
                },
                DespawnOnExit(ViewLevel::Global),
                SpacePosition(pos),
                SpaceRotation(Quat::from_euler(EulerRot::YXZ, 0.7, 0.4, 1.1)),
                WorldAssetRoot(
                    asset_server
                        .load(GltfAssetLabel::Scene(0).from_asset("models/space/freighter.gltf")),
                ),
            ))
            .id();
        commands.spawn((
            DespawnOnExit(ViewLevel::Global),
            WorldLabel::Follow(entity),
            Text::new(name),
            fonts.body(14.0),
            TextColor(theme::ACCENT),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
        ));
    }
}

/// La touche d'amarrage en vue Globale, avant l'amarrage : accrocher une épave à portée, ou
/// livrer les contrats du système près d'un hub ou de chantiers, à défaut ouvrir l'écran du
/// chantier (réparation, carburant). Renvoie `true` si elle a servi.
pub fn deliver_or_tow(world: &mut World, ship: SpacePos) -> bool {
    if !world.contains_resource::<Career>() {
        return false;
    }
    let Some(system) = world.get_resource::<Location>().map(|l| l.system.clone()) else {
        return false;
    };
    let rules = world.resource::<Data>().career.contracts.clone();
    let mut wrecks = world.query::<(Entity, &Wreck, &SpacePosition)>();
    let hooked = wrecks
        .iter(world)
        .find(|(_, _, p)| p.0.distance(ship) <= rules.tow_range)
        .map(|(e, w, _)| (e, w.contract.clone()));
    if let Some((entity, contract)) = hooked {
        let attached = world
            .resource_mut::<Career>()
            .captain
            .attach_tow(&contract)
            .is_ok();
        if attached {
            // L'épave suit le vaisseau au bout du câble : elle quitte la scène.
            world.entity_mut(entity).despawn();
            let text = world.resource::<Texts>().get("notice.tow.attached");
            world.write_message(Notice(text));
            return true;
        }
    }
    let mut points = world.query::<(&CargoPoint, &SpacePosition)>();
    let near: Vec<StructureKind> = points
        .iter(world)
        .filter(|(_, p)| p.0.distance(ship) <= rules.delivery_range)
        .map(|(c, _)| c.0)
        .collect();
    if near.is_empty() {
        return false;
    }
    if deliver_here(world, &system) {
        return true;
    }
    // Rien à livrer, mais un vaisseau abîmé ou un réservoir entamé près des chantiers qui le
    // réparent : le devis (réparation et carburant sont sur le même écran, M9). Sans le
    // réservoir dans la condition, un vaisseau intact ne pouvait faire le plein qu'à quai.
    let data = world.resource::<Data>().clone();
    let damaged = world
        .resource::<Career>()
        .captain
        .hull
        .as_ref()
        .is_some_and(|h| h.damage_points() > 0.0);
    // Même seuil que le devis du chantier (`ui::terminal::fuel::quote`) : moins d'un centième
    // d'unité de place, le réservoir est plein (arrondis des voyages).
    let thirsty = world
        .get_resource::<crate::voyage::ShipFuel>()
        .is_some_and(|tank| tank.capacity - tank.units >= 0.01);
    if (damaged || thirsty)
        && near.contains(&StructureKind::Shipyard)
        && data.combat.repair.systems.contains(&system)
    {
        world.write_message(crate::ui::terminal::OpenTerminal(
            crate::ui::terminal::Screen::Repair,
        ));
        return true;
    }
    false
}

/// Les compétences de vol montent à l'usage : piloter fait progresser le pilotage, arriver dans
/// un nouveau système la navigation.
fn practice_flight(
    time: Res<Time>,
    data: Res<Data>,
    mode: Res<State<crate::state::ControlMode>>,
    ships: Query<&crate::ship::ShipMotion>,
    location: Option<Res<Location>>,
    mut last_system: Local<Option<Id<Place>>>,
    mut career: ResMut<Career>,
    mut outcomes: MessageWriter<CareerOutcomes>,
) {
    let mut out = Vec::new();
    let rules = &data.career;
    if *mode.get() == crate::state::ControlMode::Piloting
        && ships.iter().any(|m| m.velocity.length() > 10.0)
    {
        let xp = rules.practice.piloting_per_second * f64::from(time.delta_secs());
        career.captain.practice(rules, "piloting", xp, &mut out);
    }
    if let Some(location) = location
        && last_system.as_ref() != Some(&location.system)
    {
        if last_system.is_some() {
            career.captain.practice(
                rules,
                "navigation",
                rules.practice.navigation_per_trip,
                &mut out,
            );
        }
        *last_system = Some(location.system.clone());
    }
    if !out.is_empty() {
        outcomes.write(CareerOutcomes(out));
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sh_core::Year;
    use sh_sim::{Contract, SimWorld, contract_offers};

    use super::*;

    #[test]
    fn le_repere_ne_vise_que_ce_qui_se_livre_ici() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data");
        let data = sh_data::GameData::load(&dir).unwrap_or_else(|e| panic!("{e}"));
        let today = StellarDate::start_of(Year(425));
        let sim = SimWorld::from_data(&data, Some(1));
        let place = |s: &str| Id::<Place>::new(s).unwrap_or_else(|e| panic!("{e}"));
        let hexis = place("sys_hexis");
        let mut captain = Captain::from_start(&data, today);
        let offers = contract_offers(&data, &sim, &hexis, today);
        let cargo: Contract = offers
            .iter()
            .find(|c| c.kind != ContractKind::Towing)
            .cloned()
            .unwrap_or_else(|| panic!("aucun transport : {offers:?}"));
        let to = cargo.to.clone();
        assert!(!deliverable_here(&captain, &to));
        captain
            .accept(&data, cargo)
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert!(deliverable_here(&captain, &to));
        assert!(!deliverable_here(&captain, &hexis));
        // Un remorquage : seulement une fois l'épave accrochée.
        if let Some(tow) = offers.iter().find(|c| c.kind == ContractKind::Towing) {
            let id = tow.id.clone();
            captain
                .accept(&data, tow.clone())
                .unwrap_or_else(|e| panic!("{e:?}"));
            assert!(!deliverable_here(&captain, &hexis));
            captain.attach_tow(&id).unwrap_or_else(|e| panic!("{e:?}"));
            assert!(deliverable_here(&captain, &hexis));
        }
    }
}
