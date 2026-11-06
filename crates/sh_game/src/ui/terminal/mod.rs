//! Le terminal des bornes (M5, refait en M9) : parler (dialogues en données), prendre des
//! contrats, commercer, payer la Banque, faire réparer et ravitailler en carburant lourd (`fuel`),
//! lire son journal, sa fiche et le codex.
//!
//! Il s'ouvre par une console (bureau des contrats, comptoir de la taverne, terminal du marché),
//! par un PNJ de comptoir (`items::TalkTo` : la conversation de son rôle, `Dialogue::for_role`,
//! dont les réponses ouvrent les services), par les touches de la fiche, du journal et du codex,
//! ou par l'autopilote. La fiche du capitaine a un bouton vers l'écran du personnage
//! (`items::OpenCharacterScreen`). Ouvrir une conversation fait connaître au codex
//! l'organisation que représente l'interlocuteur (`codex::SpokeWith`).
//!
//! Le jeu passe en mode Menu : curseur libre. Souris et clavier : clic sur une ligne pour la
//! choisir, sur une action pour la faire ; ↑ ↓ (ou la molette) changent de ligne, ← → d'onglet,
//! Entrée fait l'action principale, 1 à 9 les actions numérotées, Échap revient à la
//! conversation d'origine ou ferme. Le contenu d'un écran est une [`Page`] calculée sans
//! `World` (`page`) ; l'affichage (`view`) ne la redessine que si elle change ou que la
//! sélection bouge, jamais à chaque frame. L'autopilote choisit par l'action, pas par le texte
//! ([`TerminalChoice`]).

mod fuel;
mod page;
mod view;

pub use page::{
    Action, Body, Cell, Choice, Column, Ctx, Detail, Field, Gauge, Glyph, Header, Note, Page, Row,
    Slot, Tab, Tone, installment_due, page,
};

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_enhanced_input::prelude::*;
use sh_core::{Credits, StellarDate};
use sh_data::dialogue::{Dialogue, Effect, Screen as DialogueScreen};
use sh_sim::{Captain, CareerError, SimCommand};

use crate::career::{Career, CareerOutcomes, money};
use crate::clock::GameClock;
use crate::codex::SpokeWith;
use crate::data::{Data, Texts};
use crate::input::{Back, OpenCodex, OpenJournal, OpenSheet};
use crate::items::{CharacterTab, OpenCharacterScreen, TalkTo};
use crate::living::{LivingWorld, PlayerKnowledge, latest_news};
use crate::sector::{ActiveSector, Docked};
use crate::state::{AppState, ControlMode, ViewLevel};
use crate::ui::character::naming::NamingScreen;
use crate::ui::hud::Notice;
use crate::voyage::{Location, ShipFuel};

pub struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        // Une condition par système (AGENTS.md §4) : celle d'un groupe chaîné n'est évaluée
        // qu'une fois, avant le premier.
        let career = resource_exists::<Career>;
        app.add_message::<OpenTerminal>()
            .add_message::<TerminalChoice>()
            .add_observer(on_back)
            .add_observer(on_open_sheet)
            .add_observer(on_open_journal)
            .add_observer(on_open_codex)
            .add_observer(view::on_choice_button)
            .add_observer(view::on_row_button)
            .add_observer(view::on_tab_button)
            .add_observer(view::on_hover)
            .add_observer(view::on_unhover)
            .add_systems(
                Update,
                (
                    talk_to.run_if(career),
                    open_terminal.run_if(career),
                    navigate.run_if(career),
                    apply_choices.run_if(career),
                    finish_close.run_if(career),
                    view::render.run_if(career),
                )
                    .chain()
                    .after(crate::career::CareerSystems),
            )
            .add_systems(
                Update,
                open_character_after.run_if(resource_exists::<CharacterAfter>),
            )
            .add_systems(OnExit(AppState::InGame), |mut commands: Commands| {
                commands.remove_resource::<Terminal>();
                commands.remove_resource::<CharacterAfter>();
            });
    }
}

/// Un écran du terminal.
#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    /// Une conversation ; `node` vide : le point d'entrée choisi par l'interlocuteur.
    Dialogue {
        id: String,
        node: String,
    },
    Contracts,
    Market,
    /// Le marché, cette marchandise choisie.
    Commodity(String),
    /// Le compte et l'hypothèque.
    Bank,
    /// Le chantier : devis et réparation du vaisseau.
    Repair,
    Sheet,
    Journal,
    /// Le codex : la liste des fiches découvertes et le détail de la fiche choisie.
    Codex,
    /// Le codex, cette fiche choisie.
    CodexEntry(crate::codex::Entry),
}

impl From<DialogueScreen> for Screen {
    fn from(screen: DialogueScreen) -> Self {
        match screen {
            DialogueScreen::Contracts => Screen::Contracts,
            DialogueScreen::Market => Screen::Market,
            DialogueScreen::Sheet => Screen::Sheet,
            DialogueScreen::Journal => Screen::Journal,
            DialogueScreen::Bank => Screen::Bank,
            DialogueScreen::Repair => Screen::Repair,
        }
    }
}

/// Le terminal ouvert.
#[derive(Resource, Debug)]
pub struct Terminal {
    pub screen: Screen,
    /// Ce qu'a donné la dernière action (erreur, confirmation).
    pub note: Option<Note>,
    /// Texte ajouté par un effet de dialogue (les nouvelles du tavernier).
    pub extra: Option<String>,
    /// La page affichée (recalculée quand elle est effacée).
    pub page: Option<Page>,
    /// La conversation d'où vient l'écran de service affiché (« Retour », Échap).
    pub origin: Option<String>,
    /// La ligne choisie (liste) ou l'action en évidence (autres écrans).
    pub cursor: usize,
    /// Première ligne visible de la liste.
    scroll: usize,
    /// L'écran pour lequel `cursor` vaut : un autre écran repart de sa présélection.
    cursor_screen: Option<Screen>,
    /// Le jour de la page calculée (prix, délais) : elle se recalcule le lendemain.
    page_day: Option<StellarDate>,
    /// Le curseur et le défilement dessinés.
    drawn: Option<(usize, usize)>,
    /// Échap a été pressé : revenir à la conversation, ou fermer.
    back: bool,
    resume: ControlMode,
    closing: bool,
    /// Une fois fermé : ouvrir l'écran du personnage sur cet onglet.
    character_after: Option<CharacterTab>,
}

impl Terminal {
    fn new(screen: Screen, resume: ControlMode) -> Self {
        Self {
            screen,
            note: None,
            extra: None,
            page: None,
            origin: None,
            cursor: 0,
            scroll: 0,
            cursor_screen: None,
            page_day: None,
            drawn: None,
            back: false,
            resume,
            closing: false,
            character_after: None,
        }
    }

    /// Change d'écran : la page se recalcule, le curseur repart de la présélection.
    fn goto(&mut self, screen: Screen) {
        self.screen = screen;
        self.extra = None;
        self.page = None;
    }
}

/// Ouvre le terminal sur un écran (console, touche, PNJ, autopilote).
#[derive(Message, Clone, Debug)]
pub struct OpenTerminal(pub Screen);

/// Fait l'action n (0 = la première) de la page affichée, qu'elle soit visible ou non (les
/// actions d'une ligne non choisie restent accessibles à l'autopilote).
#[derive(Message, Clone, Copy, Debug)]
pub struct TerminalChoice(pub usize);

/// Après la fermeture du terminal : l'écran du personnage, une fois les commandes rendues.
#[derive(Resource)]
struct CharacterAfter(CharacterTab);

// --- Ouverture, choix, fermeture ----------------------------------------------------------

fn on_open_codex(_: On<Start<OpenCodex>>, mut open: MessageWriter<OpenTerminal>) {
    open.write(OpenTerminal(Screen::Codex));
}

/// La touche de la fiche ouvre la fiche du capitaine (M10, demande de l'auteur ; en M9 elle
/// ouvrait l'inventaire) : réputations, emprunts, et son bouton vers l'écran du personnage.
fn on_open_sheet(_: On<Start<OpenSheet>>, mut open: MessageWriter<OpenTerminal>) {
    open.write(OpenTerminal(Screen::Sheet));
}

fn on_open_journal(_: On<Start<OpenJournal>>, mut open: MessageWriter<OpenTerminal>) {
    open.write(OpenTerminal(Screen::Journal));
}

fn on_back(_: On<Start<Back>>, terminal: Option<ResMut<Terminal>>) {
    if let Some(mut terminal) = terminal {
        terminal.back = true;
    }
}

/// Parler à un PNJ de comptoir : la conversation de son rôle, ou à défaut le service de la
/// borne qu'il tient.
fn talk_to(
    mut reader: MessageReader<TalkTo>,
    data: Res<Data>,
    docked: Option<Res<Docked>>,
    active: Option<Res<ActiveSector>>,
    location: Option<Res<Location>>,
    mut open: MessageWriter<OpenTerminal>,
) {
    let Some(talk) = reader.read().last().cloned() else {
        return;
    };
    // À quai, le secteur chargé assemble le vaisseau et le quartier : le quartier compte.
    let sector = docked
        .as_ref()
        .map(|d| d.district.as_str())
        .or(active.as_ref().map(|a| a.sector.id.as_str()));
    if let Some(dialogue) = Dialogue::for_role(&data.dialogues, &talk.role, sector) {
        open.write(OpenTerminal(Screen::Dialogue {
            id: dialogue.id.clone(),
            node: String::new(),
        }));
        return;
    }
    let screen = match talk.role.as_str() {
        "contracts" => Some(Screen::Contracts),
        "bank" => Some(Screen::Bank),
        "market" if location.is_some() => Some(Screen::Market),
        "shipyard" => Some(Screen::Repair),
        _ => None,
    };
    match screen {
        Some(screen) => {
            open.write(OpenTerminal(screen));
        }
        None => warn!(
            "PNJ de comptoir « {} » ({}) : aucune conversation ni service pour ce rôle",
            talk.role, talk.variant
        ),
    }
}

/// Applique les effets d'arrivée sur un nœud de dialogue.
fn enter_node(
    data: &Data,
    cap: &mut Captain,
    term: &mut Terminal,
    id: &str,
    node: &str,
) -> Vec<Effect> {
    term.goto(Screen::Dialogue {
        id: id.to_owned(),
        node: node.to_owned(),
    });
    let dialogue = data.dialogues.iter().find(|d| d.id == id);
    let effects = dialogue
        .and_then(|d| d.nodes.get(node))
        .map(|n| n.effects.clone())
        .unwrap_or_default();
    if let Some(d) = dialogue {
        for e in &effects {
            if let Effect::Remember(fact) = e {
                cap.remember(&d.npc, fact);
            }
        }
    }
    effects
}

/// Entre dans une conversation par le point d'entrée que choisit l'interlocuteur ; faux s'il
/// n'en a aucun.
fn talk_again(
    data: &Data,
    cap: &mut Captain,
    today: StellarDate,
    term: &mut Terminal,
    id: &str,
) -> bool {
    let entry = data
        .dialogues
        .iter()
        .find(|d| d.id == id)
        .and_then(|d| cap.dialogue_entry(data, d, today))
        .map(str::to_owned);
    let Some(entry) = entry else {
        warn!("dialogue {id} : aucun point d'entrée");
        return false;
    };
    enter_node(data, cap, term, id, &entry);
    term.origin = None;
    true
}

#[allow(clippy::too_many_arguments)]
fn open_terminal(
    mut commands: Commands,
    mut reader: MessageReader<OpenTerminal>,
    data: Res<Data>,
    clock: Res<GameClock>,
    mode: Res<State<ControlMode>>,
    mut next_mode: ResMut<NextState<ControlMode>>,
    mut career: ResMut<Career>,
    terminal: Option<ResMut<Terminal>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut spoke: MessageWriter<SpokeWith>,
    naming: Option<Res<NamingScreen>>,
) {
    let Some(OpenTerminal(screen)) = reader.read().last().cloned() else {
        return;
    };
    // Pendant la saisie du nom d'un casier, rien ne s'ouvre par-dessus : son mode Menu n'est
    // effectif qu'au frame suivant, et un message envoyé entre-temps (autopilote, interaction
    // du même frame) ouvrirait le terminal sur le champ.
    if naming.is_some() {
        return;
    }
    let resume = match terminal.as_ref() {
        Some(t) => t.resume,
        None if matches!(mode.get(), ControlMode::OnFoot | ControlMode::Piloting) => *mode.get(),
        // Ni à pied ni aux commandes (carte, menu) : pas de terminal.
        None => return,
    };
    let mut term = Terminal::new(screen.clone(), resume);
    if let Screen::Dialogue { id, node } = &screen
        && node.is_empty()
        && !talk_again(&data, &mut career.captain, clock.date, &mut term, id)
    {
        return;
    }
    // Parler à quelqu'un, c'est connaître la maison qu'il représente (codex).
    if let Screen::Dialogue { id, .. } = &screen {
        spoke.write(SpokeWith(id.clone()));
    }
    commands.insert_resource(term);
    next_mode.set(ControlMode::Menu);
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

const DIGITS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// Pixels de défilement fin (pavé tactile) qui valent une ligne de molette.
const WHEEL_PIXELS_PER_STEP: f32 = 40.0;

/// Le clavier et la molette (lus directement : le contexte Menu n'a pas d'action par option,
/// et ces touches ne se remappent pas). 1 à 9 : les actions numérotées ; Entrée : l'action
/// principale (ligne choisie) ou celle en évidence ; ↑ ↓ et la molette : la ligne ou l'action
/// en évidence ; ← → : l'onglet.
fn navigate(
    keys: Res<ButtonInput<KeyCode>>,
    wheel: Res<AccumulatedMouseScroll>,
    terminal: Option<ResMut<Terminal>>,
    mut choices: MessageWriter<TerminalChoice>,
    mut fine: Local<f32>,
) {
    let Some(mut term) = terminal else {
        return;
    };
    let Some(page) = term.page.as_ref().filter(|_| !term.closing) else {
        return;
    };
    let has_rows = !page.rows().is_empty();
    let numbered = page.numbered(term.cursor);
    for (i, key) in DIGITS.iter().enumerate() {
        if keys.just_pressed(*key)
            && let Some(&index) = numbered.get(i)
        {
            choices.write(TerminalChoice(index));
            return;
        }
    }
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
        let target = if has_rows {
            page.primary(term.cursor)
        } else {
            numbered.get(term.cursor).copied()
        };
        if let Some(index) = target {
            choices.write(TerminalChoice(index));
        }
        return;
    }
    let mut step = 0i32;
    if keys.just_pressed(KeyCode::ArrowDown) {
        step += 1;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        step -= 1;
    }
    match wheel.unit {
        MouseScrollUnit::Line => step -= wheel.delta.y.round() as i32,
        MouseScrollUnit::Pixel => {
            *fine += wheel.delta.y;
            let lines = (*fine / WHEEL_PIXELS_PER_STEP).trunc();
            *fine -= lines * WHEEL_PIXELS_PER_STEP;
            step -= lines as i32;
        }
    }
    let count = page.cursor_len();
    if step != 0 && count > 0 {
        let cursor = (term.cursor as i32 + step).clamp(0, count as i32 - 1) as usize;
        if cursor != term.cursor {
            term.cursor = cursor;
        }
        return;
    }
    let tab_step = i32::from(keys.just_pressed(KeyCode::ArrowRight))
        - i32::from(keys.just_pressed(KeyCode::ArrowLeft));
    if tab_step != 0 && !page.tabs.is_empty() {
        let tabs = page.tabs.len() as i32;
        let current = page.tabs.iter().position(|t| t.active).unwrap_or(0) as i32;
        let next = (current + tab_step).rem_euclid(tabs) as usize;
        if let Some(screen) = page.tabs.get(next).map(|t| t.screen.clone()) {
            term.note = None;
            term.goto(screen);
        }
    }
}

fn career_error(texts: &Texts, err: &CareerError) -> String {
    let key = match err {
        CareerError::NotEnoughCredits => "career.error.credits",
        CareerError::CargoFull => "career.error.cargo",
        CareerError::NotEnoughGoods => "career.error.goods",
        CareerError::NoSuchContract => "career.error.contract",
        CareerError::WrongPlace => "career.error.place",
        CareerError::Refused => "career.error.refused",
        CareerError::AlreadyTaken => "career.error.taken",
        CareerError::NoDebt => "career.error.no_debt",
        CareerError::NothingToRepair => "career.error.no_repair",
    };
    texts.get(key)
}

#[allow(clippy::too_many_arguments)]
fn apply_choices(
    mut reader: MessageReader<TerminalChoice>,
    texts: Res<Texts>,
    data: Res<Data>,
    clock: Res<GameClock>,
    location: Option<Res<Location>>,
    knowledge: Option<Res<PlayerKnowledge>>,
    mut living: Option<ResMut<LivingWorld>>,
    mut career: ResMut<Career>,
    terminal: Option<ResMut<Terminal>>,
    mut ship_fuel: Option<ResMut<ShipFuel>>,
    mut outcomes: MessageWriter<CareerOutcomes>,
    mut notices: MessageWriter<Notice>,
) {
    let Some(mut term) = terminal else {
        reader.clear();
        return;
    };
    let today = clock.date;
    // Échap : un service ouvert depuis une conversation y revient ; sinon le terminal se ferme
    // (au relâchement de la touche, `finish_close`).
    if term.back {
        term.back = false;
        let origin = term
            .origin
            .clone()
            .filter(|_| !matches!(term.screen, Screen::Dialogue { .. }));
        let returned =
            origin.is_some_and(|id| talk_again(&data, &mut career.captain, today, &mut term, &id));
        if !returned {
            term.closing = true;
        }
        reader.clear();
        return;
    }
    for &TerminalChoice(index) in reader.read() {
        let Some(chosen) = term
            .page
            .as_ref()
            .and_then(|p| p.choices.get(index))
            .filter(|c| c.enabled)
            .cloned()
        else {
            continue;
        };
        let here = location.as_ref().map(|l| l.system.clone());
        let cap = &mut career.captain;
        let mut out = Vec::new();
        let mut sim_commands = Vec::new();
        let price = |commodity: &str, living: &Option<ResMut<LivingWorld>>| {
            let p = living
                .as_ref()
                .and_then(|w| w.sim.price(here.as_ref()?, commodity))?;
            let spread = data.career.market_spread;
            Some((p * (1.0 + spread), p * (1.0 - spread)))
        };
        term.note = None;
        match chosen.action {
            Action::Close => term.closing = true,
            Action::Goto(Screen::Dialogue { id, node }) if node.is_empty() => {
                if !talk_again(&data, cap, today, &mut term, &id) {
                    term.closing = true;
                }
            }
            Action::Goto(screen) => term.goto(screen),
            Action::Character => {
                term.character_after = Some(CharacterTab::Inventory);
                term.closing = true;
            }
            Action::Say { effects, next, .. } => {
                let Screen::Dialogue { id, .. } = term.screen.clone() else {
                    continue;
                };
                let npc = data
                    .dialogues
                    .iter()
                    .find(|d| d.id == id)
                    .map(|d| d.npc.clone())
                    .unwrap_or_default();
                let mut opened = None;
                term.extra = None;
                for effect in effects {
                    match effect {
                        Effect::Remember(fact) => cap.remember(&npc, &fact),
                        Effect::Open(screen) => opened = Some(Screen::from(screen)),
                        Effect::PayInstallment => match cap.pay_installment(&data) {
                            Ok(o) => out.extend(o),
                            Err(e) => term.note = Some(Note::error(career_error(&texts, &e))),
                        },
                        Effect::News => {
                            term.extra =
                                knowledge.as_ref().map(|k| latest_news(&texts, &data, k, 3));
                        }
                    }
                }
                match (opened, next) {
                    (Some(screen), _) => {
                        term.goto(screen);
                        term.origin = Some(id);
                    }
                    (None, Some(node)) => {
                        let extra = term.extra.take();
                        enter_node(&data, cap, &mut term, &id, &node);
                        term.extra = extra;
                    }
                    (None, None) => term.closing = true,
                }
            }
            Action::Accept(contract) => {
                let (commodity, amount, from) = (
                    contract.commodity.clone(),
                    contract.amount,
                    contract.from.clone(),
                );
                match cap.accept(&data, contract) {
                    Ok(o) => {
                        out.extend(o);
                        // La cargaison quitte les entrepôts du système de départ.
                        if let Some(commodity) = commodity {
                            sim_commands.push(SimCommand::Take {
                                at: from,
                                commodity,
                                amount,
                            });
                        }
                    }
                    Err(e) => term.note = Some(Note::error(career_error(&texts, &e))),
                }
            }
            Action::Buy(commodity, amount) => {
                let Some(((unit, _), at)) = price(&commodity, &living).zip(here.clone()) else {
                    continue;
                };
                // Pas plus que la soute et la bourse ne le permettent.
                let room = (cap.cargo_capacity - cap.cargo_load()).max(0.0);
                let affordable = (cap.credits.as_f64() / unit).floor().max(0.0);
                let amount = amount.min(room).min(affordable).floor();
                let result = if amount < 1.0 {
                    Err(if room < 1.0 {
                        CareerError::CargoFull
                    } else {
                        CareerError::NotEnoughCredits
                    })
                } else {
                    cap.buy(&commodity, amount, unit)
                };
                match result {
                    Ok(cost) => {
                        term.note = Some(Note::good(texts.format(
                            "market.bought",
                            &[
                                ("amount", &texts.number(amount, 0)),
                                ("commodity", &texts.get(&format!("commodity.{commodity}"))),
                                ("cost", &money(&texts, cost)),
                            ],
                        )));
                        cap.practice(
                            &data.career,
                            "commerce",
                            amount * data.career.practice.trade_per_unit,
                            &mut out,
                        );
                        sim_commands.push(SimCommand::Take {
                            at,
                            commodity,
                            amount,
                        });
                    }
                    Err(e) => term.note = Some(Note::error(career_error(&texts, &e))),
                }
            }
            Action::Sell(commodity, amount) => {
                let Some(((_, unit), at)) = price(&commodity, &living).zip(here.clone()) else {
                    continue;
                };
                match cap.sell(&commodity, amount, unit) {
                    Ok(gain) => {
                        term.note = Some(Note::good(texts.format(
                            "market.sold",
                            &[
                                ("amount", &texts.number(amount, 0)),
                                ("commodity", &texts.get(&format!("commodity.{commodity}"))),
                                ("gain", &money(&texts, gain)),
                            ],
                        )));
                        cap.practice(
                            &data.career,
                            "commerce",
                            amount * data.career.practice.trade_per_unit,
                            &mut out,
                        );
                        sim_commands.push(SimCommand::Deliver {
                            at,
                            commodity,
                            amount,
                        });
                    }
                    Err(e) => term.note = Some(Note::error(career_error(&texts, &e))),
                }
            }
            Action::Betray(id) => {
                let found = cap
                    .contracts
                    .iter()
                    .find(|c| c.id == id)
                    .and_then(|c| Some((c.commodity.clone()?, c.amount)));
                let Some((commodity, amount)) = found else {
                    continue;
                };
                let Some(((_, unit), at)) = price(&commodity, &living).zip(here.clone()) else {
                    continue;
                };
                let sale = Credits::from_cents((amount * unit * 100.0).round() as i64);
                match cap.betray(&data, &id, sale, today) {
                    Ok(o) => {
                        out.extend(o);
                        sim_commands.push(SimCommand::Deliver {
                            at,
                            commodity,
                            amount,
                        });
                    }
                    Err(e) => term.note = Some(Note::error(career_error(&texts, &e))),
                }
            }
            Action::PayInstallment => {
                let due = cap.loan.as_ref().map(installment_due);
                match cap.pay_installment(&data) {
                    Ok(o) => {
                        out.extend(o);
                        term.note = due.map(|due| {
                            Note::good(
                                texts.format("bank.paid", &[("amount", &money(&texts, due))]),
                            )
                        });
                    }
                    Err(e) => term.note = Some(Note::error(career_error(&texts, &e))),
                }
            }
            Action::Repair => match cap.repair_ship(&data) {
                Ok(cost) => {
                    term.note = Some(Note::good(
                        texts.format("repair.done", &[("cost", &money(&texts, cost))]),
                    ));
                }
                Err(e) => term.note = Some(Note::error(career_error(&texts, &e))),
            },
            Action::Refuel(wanted) => {
                let Some(tank) = ship_fuel.as_mut() else {
                    continue;
                };
                let rules = &data.combat.refuel;
                let per_day = fuel::fuel_per_day(&data);
                let unit = fuel::unit_price(rules.day_price, per_day);
                match fuel::refuel(tank.units, tank.capacity, wanted, unit, cap.credits) {
                    Ok(done) => {
                        cap.credits -= done.cost;
                        tank.units = (tank.units + done.units).min(tank.capacity);
                        let days = if per_day > 0.0 {
                            done.units / per_day
                        } else {
                            done.units
                        };
                        term.note = Some(Note::good(texts.format(
                            "refuel.done",
                            &[
                                ("days", &texts.number(days, 0)),
                                ("cost", &money(&texts, done.cost)),
                            ],
                        )));
                        // Le carburant sort des stocks du système : le marché le voit.
                        if let Some(at) = here.clone() {
                            sim_commands.push(SimCommand::Take {
                                at,
                                commodity: rules.commodity.clone(),
                                amount: done.units,
                            });
                        }
                    }
                    Err(fuel::RefuelError::TankFull) => {
                        term.note = Some(Note::error(texts.get("refuel.full")));
                    }
                    Err(fuel::RefuelError::NotEnoughCredits) => {
                        let cost = fuel::quote(tank.units, tank.capacity, wanted, unit)
                            .map_or(Credits::ZERO, |q| q.cost);
                        term.note = Some(Note::error(texts.format(
                            "refuel.no_credits",
                            &[
                                ("cost", &money(&texts, cost)),
                                ("credits", &money(&texts, cap.credits)),
                            ],
                        )));
                    }
                }
            }
            Action::Exchange(amount) => match cap.exchange_to_wuro(&data.career, amount) {
                Ok(w) => {
                    term.note = Some(Note::good(texts.format(
                        "market.exchanged",
                        &[
                            ("credits", &money(&texts, amount)),
                            ("wuro", &texts.number(w.as_f64(), 2)),
                        ],
                    )));
                }
                Err(e) => term.note = Some(Note::error(career_error(&texts, &e))),
            },
        }
        if let Some(world) = living.as_mut() {
            for command in &sim_commands {
                world.sim.apply(command);
            }
        }
        if !out.is_empty() {
            outcomes.write(CareerOutcomes(out));
        }
        if let Some(note) = &term.note {
            notices.write(Notice(note.text.clone()));
        }
        term.page = None;
    }
}

/// Ferme le terminal une fois Échap relâché : sinon le contexte rendu (à pied, pilotage)
/// verrait la touche encore enfoncée et ouvrirait le menu de pause.
fn finish_close(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    terminal: Option<Res<Terminal>>,
    view: Option<Res<State<ViewLevel>>>,
    mut next_mode: ResMut<NextState<ControlMode>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Some(terminal) = terminal else {
        return;
    };
    if !terminal.closing || keys.pressed(KeyCode::Escape) {
        return;
    }
    next_mode.set(terminal.resume);
    commands.remove_resource::<Terminal>();
    if let Some(tab) = terminal.character_after {
        commands.insert_resource(CharacterAfter(tab));
    }
    if view.is_some_and(|v| matches!(v.get(), ViewLevel::Sector | ViewLevel::Global)) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// L'écran du personnage demandé depuis la fiche : il s'ouvre une fois le terminal parti et
/// les commandes rendues (il refuserait de s'ouvrir par-dessus un menu).
fn open_character_after(
    mut commands: Commands,
    after: Res<CharacterAfter>,
    mode: Res<State<ControlMode>>,
    terminal: Option<Res<Terminal>>,
    mut open: MessageWriter<OpenCharacterScreen>,
) {
    if terminal.is_some() || !matches!(mode.get(), ControlMode::OnFoot | ControlMode::Piloting) {
        return;
    }
    open.write(OpenCharacterScreen(after.0));
    commands.remove_resource::<CharacterAfter>();
}
