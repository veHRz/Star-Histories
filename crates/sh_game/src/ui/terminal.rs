//! Le terminal (M5) : un panneau pour parler (dialogues en données), prendre des contrats,
//! commercer, lire sa fiche et son journal de missions.
//!
//! Il s'ouvre par une console (bureau des contrats, comptoir de la taverne, terminal du marché)
//! ou par les touches de la fiche et du journal. Le jeu passe en mode Menu : curseur libre,
//! choix à la souris ou aux touches 1 à 9, Échap pour fermer. Le contenu d'un écran est une
//! [`Page`] calculée sans `World` ([`page`]) ; l'autopilote choisit par l'action, pas par le
//! texte ([`TerminalChoice`]).

use bevy::picking::events::{PointerOut, PointerOver};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_enhanced_input::prelude::*;
use sh_core::{Credits, Id, StellarDate};
use sh_data::Place;
use sh_data::dialogue::{Effect, Screen as DialogueScreen};
use sh_sim::{
    Captain, CareerError, Contract, ContractState, SimCommand, SimWorld, contract_offers,
};

use crate::career::{
    Career, CareerOutcomes, MissionLog, contract_title, faction_name, money, rank_name,
};
use crate::clock::{GameClock, format_date};
use crate::data::{Data, Texts};
use crate::input::{Back, OpenCodex, OpenJournal, OpenSheet};
use crate::living::{LivingWorld, PlayerKnowledge, display_name, latest_news};
use crate::state::{AppState, ControlMode, ViewLevel};
use crate::ui::hud::Notice;
use crate::ui::theme::{self, UiFonts};
use crate::voyage::Location;

pub struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenTerminal>()
            .add_message::<TerminalChoice>()
            .add_observer(on_back)
            .add_observer(on_open_sheet)
            .add_observer(on_open_journal)
            .add_observer(on_open_codex)
            .add_observer(on_choice_button)
            .add_observer(on_hover)
            .add_observer(on_unhover)
            .add_systems(
                Update,
                (
                    open_terminal,
                    digit_keys,
                    apply_choices,
                    finish_close,
                    render,
                )
                    .chain()
                    .after(crate::career::CareerSystems)
                    .run_if(resource_exists::<Career>),
            )
            .add_systems(OnExit(AppState::InGame), |mut commands: Commands| {
                commands.remove_resource::<Terminal>();
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
    Commodity(String),
    Sheet,
    Journal,
    /// Le codex : la liste des fiches découvertes, puis une fiche.
    Codex,
    CodexEntry(crate::codex::Entry),
    /// Le chantier : devis et réparation du vaisseau.
    Repair,
}

impl From<DialogueScreen> for Screen {
    fn from(screen: DialogueScreen) -> Self {
        match screen {
            DialogueScreen::Contracts => Screen::Contracts,
            DialogueScreen::Market => Screen::Market,
            DialogueScreen::Sheet => Screen::Sheet,
            DialogueScreen::Journal => Screen::Journal,
        }
    }
}

/// Le terminal ouvert.
#[derive(Resource, Debug)]
pub struct Terminal {
    pub screen: Screen,
    /// Ce qu'a donné la dernière action (erreur, confirmation).
    pub note: Option<String>,
    /// Texte ajouté par un effet de dialogue (les nouvelles du tavernier).
    pub extra: Option<String>,
    /// La page affichée (recalculée quand elle est effacée).
    pub page: Option<Page>,
    resume: ControlMode,
    closing: bool,
}

/// Ouvre le terminal sur un écran (console, touche, autopilote).
#[derive(Message, Clone, Debug)]
pub struct OpenTerminal(pub Screen);

/// Choisit l'option n (0 = la première) de la page affichée.
#[derive(Message, Clone, Copy, Debug)]
pub struct TerminalChoice(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Close,
    Goto(Screen),
    /// Réplique de dialogue : `key`, sa clé de langue (pour l'autopilote).
    Say {
        key: String,
        effects: Vec<Effect>,
        next: Option<String>,
    },
    Accept(Contract),
    Buy(String, f64),
    Sell(String, f64),
    /// Vendre la cargaison d'un contrat : trahison.
    Betray(String),
    Exchange(Credits),
    Repair,
}

impl Action {
    /// L'action ferme-t-elle le terminal ? (Une réplique sans suite ni écran clôt la
    /// conversation : inutile d'ajouter « Partir ».)
    pub fn closes(&self) -> bool {
        match self {
            Action::Close => true,
            Action::Say { effects, next, .. } => {
                next.is_none() && !effects.iter().any(|e| matches!(e, Effect::Open(_)))
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub label: String,
    pub action: Action,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Page {
    pub title: String,
    pub body: String,
    pub choices: Vec<Choice>,
}

/// Ce que lit une page.
pub struct Ctx<'a> {
    pub texts: &'a Texts,
    pub data: &'a Data,
    pub cap: &'a Captain,
    pub sim: Option<&'a SimWorld>,
    pub here: Option<&'a Id<Place>>,
    pub today: StellarDate,
    pub log: &'a MissionLog,
    /// Les objets portés (ramassés à pied).
    pub items: &'a std::collections::BTreeMap<String, u32>,
    /// Les fiches découvertes.
    pub codex: &'a crate::codex::Codex,
}

impl Ctx<'_> {
    /// Prix d'achat et de vente d'une marchandise ici (écart du marché autour du prix).
    fn prices(&self, commodity: &str) -> Option<(f64, f64)> {
        let price = self.sim?.price(self.here?, commodity)?;
        let spread = self.data.career.market_spread;
        Some((price * (1.0 + spread), price * (1.0 - spread)))
    }

    fn offers(&self) -> Vec<Contract> {
        match (self.sim, self.here) {
            (Some(sim), Some(here)) => contract_offers(self.data, sim, here, self.today),
            _ => Vec::new(),
        }
    }
}

fn choice(label: String, action: Action) -> Choice {
    Choice {
        label,
        action,
        enabled: true,
    }
}

/// Le contenu d'un écran.
pub fn page(ctx: &Ctx, term: &Terminal) -> Page {
    let t = ctx.texts;
    let mut page = match &term.screen {
        Screen::Dialogue { id, node } => dialogue_page(ctx, id, node, term.extra.as_deref()),
        Screen::Contracts => contracts_page(ctx),
        Screen::Market => market_page(ctx),
        Screen::Commodity(c) => commodity_page(ctx, c),
        Screen::Sheet => sheet_page(ctx),
        Screen::Journal => journal_page(ctx),
        Screen::Repair => repair_page(ctx),
        Screen::Codex => codex_page(ctx),
        Screen::CodexEntry(entry) => codex_entry_page(ctx, entry),
    };
    if let Some(note) = &term.note {
        page.body = format!("{}\n\n{note}", page.body);
    }
    if !page.choices.iter().any(|c| c.action.closes()) {
        page.choices
            .push(choice(t.get("terminal.close"), Action::Close));
    }
    page
}

fn dialogue_page(ctx: &Ctx, id: &str, node: &str, extra: Option<&str>) -> Page {
    let t = ctx.texts;
    let Some(dialogue) = ctx.data.dialogues.iter().find(|d| d.id == id) else {
        return Page::default();
    };
    let Some(n) = dialogue.nodes.get(node) else {
        return Page::default();
    };
    let mut body = t.get(&n.text);
    if let Some(extra) = extra {
        body = format!("{body}\n\n{extra}");
    }
    let choices = n
        .choices
        .iter()
        .filter(|c| {
            ctx.cap
                .satisfies_all(ctx.data, &dialogue.npc, &c.when, ctx.today)
        })
        .map(|c| {
            choice(
                t.get(&c.text),
                Action::Say {
                    key: c.text.clone(),
                    effects: c.effects.clone(),
                    next: c.next.clone(),
                },
            )
        })
        .collect();
    Page {
        title: t.get(&dialogue.name_key),
        body,
        choices,
    }
}

fn contract_line(ctx: &Ctx, c: &Contract) -> String {
    ctx.texts.format(
        "contract.line",
        &[
            ("title", &contract_title(ctx.texts, ctx.data, c)),
            ("reward", &money(ctx.texts, c.reward)),
            ("deadline", &format_date(ctx.texts, c.deadline)),
        ],
    )
}

fn contracts_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let issuer = &ctx.data.career.contracts.issuer;
    let mut body = Vec::new();
    let mut choices = Vec::new();
    if ctx.cap.refused_by(ctx.data, issuer) {
        body.push(t.get("contracts.refused"));
    } else {
        body.push(t.format(
            "contracts.header",
            &[
                ("cargo", &format!("{:.0}", ctx.cap.cargo_load())),
                ("capacity", &format!("{:.0}", ctx.cap.cargo_capacity)),
            ],
        ));
        let taken = |c: &Contract| ctx.cap.contracts.iter().any(|k| k.id == c.id);
        for offer in ctx.offers().into_iter().filter(|c| !taken(c)) {
            let fits = ctx.cap.cargo_load() + offer.amount <= ctx.cap.cargo_capacity + 1e-9;
            let mut label = contract_line(ctx, &offer);
            if !fits {
                label = t.format("contracts.no_room", &[("line", &label)]);
            }
            choices.push(Choice {
                label,
                action: Action::Accept(offer),
                enabled: fits,
            });
        }
        if choices.is_empty() {
            body.push(t.get("contracts.none"));
        }
    }
    Page {
        title: t.get("contracts.title"),
        body: body.join("\n"),
        choices,
    }
}

fn market_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let Some(here) = ctx.here else {
        return Page {
            title: t.get("market.title"),
            body: t.get("console.market.message"),
            choices: Vec::new(),
        };
    };
    let lines = [t.format(
        "market.header",
        &[
            ("place", &display_name(ctx.data, here.as_str())),
            ("credits", &money(t, ctx.cap.credits)),
            ("cargo", &format!("{:.0}", ctx.cap.cargo_load())),
            ("capacity", &format!("{:.0}", ctx.cap.cargo_capacity)),
        ],
    )];
    let mut choices = Vec::new();
    for commodity in ctx.data.sim.base_prices.keys() {
        let Some((buy, sell)) = ctx.prices(commodity) else {
            continue;
        };
        let held = ctx.cap.cargo.get(commodity).copied().unwrap_or(0.0);
        let name = t.get(&format!("commodity.{commodity}"));
        // Une ligne par marchandise, qui ouvre son écran d'échange.
        choices.push(choice(
            t.format(
                "market.row",
                &[
                    ("commodity", &name),
                    ("buy", &format!("{buy:.0}")),
                    ("sell", &format!("{sell:.0}")),
                    ("held", &format!("{held:.0}")),
                ],
            ),
            Action::Goto(Screen::Commodity(commodity.clone())),
        ));
    }
    // Détourner une cargaison sous contrat : elle se vend ici comme une autre.
    for c in ctx
        .cap
        .contracts
        .iter()
        .filter(|c| c.state == ContractState::Active && c.commodity.is_some())
    {
        choices.push(choice(
            t.format(
                "market.betray",
                &[("title", &contract_title(t, ctx.data, c))],
            ),
            Action::Betray(c.id.clone()),
        ));
    }
    let rate = ctx.data.career.credits_per_wuro;
    choices.push(choice(
        t.format(
            "market.exchange",
            &[
                ("amount", &money(t, Credits::units(1000))),
                ("rate", &t.number(rate, 1)),
            ],
        ),
        Action::Exchange(Credits::units(1000)),
    ));
    Page {
        title: t.get("market.title"),
        body: lines.join("\n"),
        choices,
    }
}

fn commodity_page(ctx: &Ctx, commodity: &str) -> Page {
    let t = ctx.texts;
    let name = t.get(&format!("commodity.{commodity}"));
    let held = ctx.cap.cargo.get(commodity).copied().unwrap_or(0.0);
    let (buy, sell) = ctx.prices(commodity).unwrap_or((0.0, 0.0));
    let body = t.format(
        "market.commodity",
        &[
            ("commodity", &name),
            ("buy", &format!("{buy:.0}")),
            ("sell", &format!("{sell:.0}")),
            ("held", &format!("{held:.0}")),
            ("credits", &money(t, ctx.cap.credits)),
        ],
    );
    let mut choices = Vec::new();
    for amount in [10.0, 50.0] {
        choices.push(choice(
            t.format("market.buy", &[("amount", &format!("{amount:.0}"))]),
            Action::Buy(commodity.to_owned(), amount),
        ));
    }
    choices.push(choice(
        t.format("market.sell", &[("amount", "10")]),
        Action::Sell(commodity.to_owned(), 10.0),
    ));
    choices.push(Choice {
        label: t.get("market.sell_all"),
        action: Action::Sell(commodity.to_owned(), held),
        enabled: held > 0.0,
    });
    choices.push(choice(t.get("terminal.back"), Action::Goto(Screen::Market)));
    Page {
        title: name,
        body,
        choices,
    }
}

fn sheet_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let cap = ctx.cap;
    let rules = &ctx.data.career;
    let mut lines = vec![t.format(
        "sheet.money",
        &[
            ("credits", &money(t, cap.credits)),
            ("wuro", &t.number(cap.wuro.as_f64(), 2)),
        ],
    )];
    lines.push(match &cap.loan {
        Some(loan) => t.format(
            "sheet.debt",
            &[
                ("lender", &faction_name(t, ctx.data, &loan.lender)),
                ("balance", &money(t, loan.balance)),
                ("installment", &money(t, loan.installment)),
                ("due", &format_date(t, loan.next_due)),
            ],
        ),
        None => t.get("sheet.no_debt"),
    });
    let cargo: Vec<String> = cap
        .cargo
        .iter()
        .map(|(c, a)| format!("{a:.0} {}", t.get(&format!("commodity.{c}"))))
        .collect();
    lines.push(t.format(
        "sheet.cargo",
        &[
            ("cargo", &format!("{:.0}", cap.cargo_load())),
            ("capacity", &format!("{:.0}", cap.cargo_capacity)),
            (
                "goods",
                &if cargo.is_empty() {
                    t.get("sheet.empty")
                } else {
                    cargo.join(", ")
                },
            ),
        ],
    ));
    let items: Vec<String> = ctx
        .items
        .iter()
        .map(|(id, n)| format!("{} ×{n}", t.get(&format!("item.{id}"))))
        .collect();
    lines.push(t.format(
        "sheet.items",
        &[(
            "items",
            &if items.is_empty() {
                t.get("sheet.empty")
            } else {
                items.join(", ")
            },
        )],
    ));
    if !cap.traits.is_empty() {
        let traits: Vec<String> = cap
            .traits
            .iter()
            .map(|id| t.get(&format!("trait.{id}")))
            .collect();
        lines.push(t.format("sheet.traits", &[("traits", &traits.join(", "))]));
    }
    let skills: Vec<String> = rules
        .skills
        .iter()
        .map(|s| {
            format!(
                "{} {}",
                t.get(&format!("skill.{s}")),
                cap.skill_level(rules, s)
            )
        })
        .collect();
    lines.push(t.format("sheet.skills", &[("skills", &skills.join(" · "))]));
    lines.push(t.get("sheet.reputation"));
    for (faction, value) in &cap.reputation {
        // Une organisation secrète n'apparaît jamais (règle d'or n° 3).
        if ctx.data.faction(faction).is_none_or(|f| f.secret) {
            continue;
        }
        let mut line = t.format(
            "sheet.rep_line",
            &[
                ("faction", &faction_name(t, ctx.data, faction)),
                ("tier", &t.get(cap.tier(rules, faction))),
                ("value", &format!("{value:+}")),
            ],
        );
        if let Some(rank) = cap.ranks.get(faction) {
            line.push_str(&t.format(
                "sheet.rank",
                &[("rank", &rank_name(t, ctx.data, faction, *rank))],
            ));
        }
        if cap.expelled.contains(faction) {
            line.push_str(&t.get("sheet.expelled"));
        }
        lines.push(line);
    }
    Page {
        title: t.get("sheet.title"),
        body: lines.join("\n"),
        choices: vec![
            choice(t.get("journal.title"), Action::Goto(Screen::Journal)),
            choice(t.get("codex.title"), Action::Goto(Screen::Codex)),
        ],
    }
}

fn repair_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let rules = &ctx.data.combat.repair;
    let Some(hull) = ctx.cap.hull.as_ref() else {
        return Page::default();
    };
    let cost = hull.repair_cost(rules);
    let body = t.format(
        "repair.body",
        &[
            ("yard", &faction_name(t, ctx.data, &rules.issuer)),
            ("integrity", &t.number(hull.integrity() * 100.0, 0)),
            ("cost", &money(t, cost)),
            ("credits", &money(t, ctx.cap.credits)),
        ],
    );
    let damaged = cost.cents > 0;
    // Le compte n'y suffit pas : le chantier répare au prorata de ce qui est payé.
    let label = if ctx.cap.credits.cents >= cost.cents {
        t.format("repair.do", &[("cost", &money(t, cost))])
    } else {
        t.format(
            "repair.partial",
            &[
                ("paid", &money(t, ctx.cap.credits)),
                ("cost", &money(t, cost)),
            ],
        )
    };
    Page {
        title: t.get("repair.title"),
        body,
        choices: vec![Choice {
            label,
            action: Action::Repair,
            enabled: damaged && ctx.cap.credits.cents > 0,
        }],
    }
}

fn journal_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let mut lines = vec![t.get("journal.active")];
    let active: Vec<&Contract> = ctx
        .cap
        .contracts
        .iter()
        .filter(|c| c.state == ContractState::Active)
        .collect();
    if active.is_empty() {
        lines.push(t.get("journal.no_active"));
    }
    for c in active {
        let mut line = contract_line(ctx, c);
        if c.kind == sh_sim::ContractKind::Towing {
            line.push_str(&t.get(if c.towed {
                "journal.towed"
            } else {
                "journal.find_wreck"
            }));
        }
        lines.push(line);
    }
    lines.push(String::new());
    lines.push(t.get("journal.history"));
    let history: Vec<&String> = ctx.log.lines.iter().rev().take(10).collect();
    if history.is_empty() {
        lines.push(t.get("journal.empty"));
    }
    lines.extend(history.into_iter().cloned());
    Page {
        title: t.get("journal.title"),
        body: lines.join("\n"),
        choices: vec![
            choice(t.get("sheet.title"), Action::Goto(Screen::Sheet)),
            choice(t.get("codex.title"), Action::Goto(Screen::Codex)),
        ],
    }
}

fn codex_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let entries = ctx
        .codex
        .places
        .iter()
        .map(|p| crate::codex::Entry::Place(p.clone()))
        .chain(
            ctx.codex
                .factions
                .iter()
                .map(|f| crate::codex::Entry::Faction(f.clone())),
        );
    let choices = entries
        .map(|e| {
            choice(
                crate::codex::entry_name(ctx.data, &e),
                Action::Goto(Screen::CodexEntry(e)),
            )
        })
        .collect();
    Page {
        title: t.get("codex.title"),
        body: if ctx.codex.is_empty() {
            t.get("codex.empty")
        } else {
            t.get("codex.intro")
        },
        choices,
    }
}

fn codex_entry_page(ctx: &Ctx, entry: &crate::codex::Entry) -> Page {
    let t = ctx.texts;
    let (title, lines) = crate::codex::describe(t, ctx.data, entry)
        .unwrap_or_else(|| (t.get("codex.title"), vec![t.get("codex.empty")]));
    Page {
        title,
        body: lines.join(
            "
",
        ),
        choices: vec![choice(t.get("codex.back"), Action::Goto(Screen::Codex))],
    }
}

// --- Ouverture, choix, fermeture ----------------------------------------------------------

fn on_open_codex(_: On<Start<OpenCodex>>, mut open: MessageWriter<OpenTerminal>) {
    open.write(OpenTerminal(Screen::Codex));
}

fn on_open_sheet(_: On<Start<OpenSheet>>, mut open: MessageWriter<OpenTerminal>) {
    open.write(OpenTerminal(Screen::Sheet));
}

fn on_open_journal(_: On<Start<OpenJournal>>, mut open: MessageWriter<OpenTerminal>) {
    open.write(OpenTerminal(Screen::Journal));
}

fn on_back(_: On<Start<Back>>, terminal: Option<ResMut<Terminal>>) {
    if let Some(mut terminal) = terminal {
        match terminal.screen {
            Screen::Commodity(_) => {
                terminal.screen = Screen::Market;
                terminal.page = None;
            }
            _ => terminal.closing = true,
        }
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
    term.screen = Screen::Dialogue {
        id: id.to_owned(),
        node: node.to_owned(),
    };
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
) {
    let Some(OpenTerminal(screen)) = reader.read().last().cloned() else {
        return;
    };
    let resume = match terminal.as_ref() {
        Some(t) => t.resume,
        None if matches!(mode.get(), ControlMode::OnFoot | ControlMode::Piloting) => *mode.get(),
        // Ni à pied ni aux commandes (carte, menu) : pas de terminal.
        None => return,
    };
    let mut term = Terminal {
        screen: screen.clone(),
        note: None,
        extra: None,
        page: None,
        resume,
        closing: false,
    };
    if let Screen::Dialogue { id, node } = &screen
        && node.is_empty()
    {
        let entry = data
            .dialogues
            .iter()
            .find(|d| &d.id == id)
            .and_then(|d| career.captain.dialogue_entry(&data, d, clock.date))
            .map(str::to_owned);
        let Some(entry) = entry else {
            warn!("dialogue {id} : aucun point d'entrée");
            return;
        };
        enter_node(&data, &mut career.captain, &mut term, id, &entry);
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

/// Les touches 1 à 9 choisissent (lues directement : le contexte Menu n'a pas d'action par
/// option, et ces touches ne se remappent pas).
fn digit_keys(
    keys: Res<ButtonInput<KeyCode>>,
    terminal: Option<Res<Terminal>>,
    mut choices: MessageWriter<TerminalChoice>,
) {
    if terminal.is_none() {
        return;
    }
    for (i, key) in DIGITS.iter().enumerate() {
        if keys.just_pressed(*key) {
            choices.write(TerminalChoice(i));
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
    mut outcomes: MessageWriter<CareerOutcomes>,
    mut notices: MessageWriter<Notice>,
) {
    let Some(mut term) = terminal else {
        reader.clear();
        return;
    };
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
        let today = clock.date;
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
            Action::Goto(screen) => {
                term.screen = screen;
                term.extra = None;
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
                            Err(e) => term.note = Some(career_error(&texts, &e)),
                        },
                        Effect::News => {
                            term.extra =
                                knowledge.as_ref().map(|k| latest_news(&texts, &data, k, 3));
                        }
                    }
                }
                match (opened, next) {
                    (Some(screen), _) => term.screen = screen,
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
                    Err(e) => term.note = Some(career_error(&texts, &e)),
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
                        term.note = Some(texts.format(
                            "market.bought",
                            &[
                                ("amount", &format!("{amount:.0}")),
                                ("commodity", &texts.get(&format!("commodity.{commodity}"))),
                                ("cost", &money(&texts, cost)),
                            ],
                        ));
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
                    Err(e) => term.note = Some(career_error(&texts, &e)),
                }
            }
            Action::Sell(commodity, amount) => {
                let Some(((_, unit), at)) = price(&commodity, &living).zip(here.clone()) else {
                    continue;
                };
                match cap.sell(&commodity, amount, unit) {
                    Ok(gain) => {
                        term.note = Some(texts.format(
                            "market.sold",
                            &[
                                ("amount", &format!("{amount:.0}")),
                                ("commodity", &texts.get(&format!("commodity.{commodity}"))),
                                ("gain", &money(&texts, gain)),
                            ],
                        ));
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
                    Err(e) => term.note = Some(career_error(&texts, &e)),
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
                    Err(e) => term.note = Some(career_error(&texts, &e)),
                }
            }
            Action::Repair => match cap.repair_ship(&data) {
                Ok(cost) => {
                    term.note =
                        Some(texts.format("repair.done", &[("cost", &money(&texts, cost))]));
                }
                Err(e) => term.note = Some(career_error(&texts, &e)),
            },
            Action::Exchange(amount) => match cap.exchange_to_wuro(&data.career, amount) {
                Ok(w) => {
                    term.note = Some(texts.format(
                        "market.exchanged",
                        &[
                            ("credits", &money(&texts, amount)),
                            ("wuro", &texts.number(w.as_f64(), 2)),
                        ],
                    ));
                }
                Err(e) => term.note = Some(career_error(&texts, &e)),
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
            notices.write(Notice(note.clone()));
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
    if view.is_some_and(|v| matches!(v.get(), ViewLevel::Sector | ViewLevel::Global)) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

// --- Affichage ----------------------------------------------------------------------------

#[derive(Component)]
struct TerminalUi;

#[derive(Component, Clone, Copy)]
struct ChoiceButton(usize);

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    texts: Res<Texts>,
    data: Res<Data>,
    fonts: Res<UiFonts>,
    clock: Res<GameClock>,
    location: Option<Res<Location>>,
    living: Option<Res<LivingWorld>>,
    log: Res<MissionLog>,
    inventory: Res<crate::interaction::Inventory>,
    codex: Res<crate::codex::Codex>,
    career: Res<Career>,
    terminal: Option<ResMut<Terminal>>,
    roots: Query<Entity, With<TerminalUi>>,
) {
    let Some(mut term) = terminal.filter(|t| !t.closing) else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    };
    if term.page.is_some() && !career.is_changed() && !roots.is_empty() {
        return;
    }
    let ctx = Ctx {
        texts: &texts,
        data: &data,
        cap: &career.captain,
        sim: living.as_ref().map(|w| &w.sim),
        here: location.as_ref().map(|l| &l.system),
        today: clock.date,
        log: &log,
        items: &inventory.items,
        codex: &codex,
    };
    let fresh = page(&ctx, &term);
    if term.page.as_ref() == Some(&fresh) && !roots.is_empty() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            Name::new("Terminal"),
            TerminalUi,
            DespawnOnExit(AppState::InGame),
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                // En haut : le bas de l'écran reste aux messages (résultats des actions).
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                padding: UiRect::top(percent(7)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
        ))
        .id();
    let panel = commands
        .spawn((
            ChildOf(root),
            Node {
                width: px(820),
                max_width: percent(94),
                max_height: percent(58),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(20)),
                row_gap: px(6),
                overflow: Overflow::clip_y(),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(theme::BUTTON_BORDER),
            BackgroundColor(theme::PANEL),
        ))
        .id();
    commands.spawn((
        ChildOf(panel),
        Text::new(fresh.title.clone()),
        fonts.title(24.0),
        TextColor(theme::ACCENT),
    ));
    commands.spawn((
        ChildOf(panel),
        Text::new(fresh.body.clone()),
        fonts.body(16.0),
        TextColor(theme::TEXT),
        Node {
            margin: UiRect::bottom(px(8)),
            ..default()
        },
    ));
    for (i, c) in fresh.choices.iter().enumerate() {
        let color = if c.enabled {
            theme::TEXT
        } else {
            theme::TEXT_DISABLED
        };
        let number = if i < DIGITS.len() {
            format!("{}. ", i + 1)
        } else {
            String::new()
        };
        commands.spawn((
            ChildOf(panel),
            Button,
            ActivateOnPress,
            ChoiceButton(i),
            Node {
                padding: UiRect::axes(px(12), px(5)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(theme::BUTTON_BORDER),
            BackgroundColor(theme::BUTTON),
            children![(
                Text::new(format!("{number}{}", c.label)),
                fonts.body(16.0),
                TextColor(color)
            )],
        ));
    }
    commands.spawn((
        ChildOf(panel),
        Text::new(texts.get("terminal.hint")),
        fonts.body(13.0),
        TextColor(theme::TEXT_DIM),
    ));
    term.page = Some(fresh);
}

fn on_choice_button(
    event: On<Activate>,
    buttons: Query<&ChoiceButton>,
    mut choices: MessageWriter<TerminalChoice>,
) {
    if let Ok(ChoiceButton(i)) = buttons.get(event.entity) {
        choices.write(TerminalChoice(*i));
    }
}

fn on_hover(event: On<PointerOver>, mut buttons: Query<&mut BackgroundColor, With<ChoiceButton>>) {
    if let Ok(mut color) = buttons.get_mut(event.entity) {
        *color = theme::BUTTON_HOVER.into();
    }
}

fn on_unhover(event: On<PointerOut>, mut buttons: Query<&mut BackgroundColor, With<ChoiceButton>>) {
    if let Ok(mut color) = buttons.get_mut(event.entity) {
        *color = theme::BUTTON.into();
    }
}
