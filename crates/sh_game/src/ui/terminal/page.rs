//! Le contenu des écrans du terminal, calculé sans `World`.
//!
//! Une [`Page`] décrit tout ce que montre la borne : l'en-tête (nom de l'écran, organisation qui
//! tient la borne, pictogramme), le bandeau de relevés, les onglets, le corps (une conversation,
//! une liste et le détail de la ligne choisie, ou des panneaux côte à côte) et **toutes** ses
//! actions ([`Choice`]), y compris celles des lignes non choisies : l'autopilote choisit par
//! l'action (`TerminalChoice`), pas par ce qui est affiché. L'affichage (`view`) dessine la page ;
//! ce module ne dépend de rien de Bevy hormis les ressources qu'il lit par [`Ctx`].

use std::collections::BTreeMap;

use sh_core::{Credits, Id, StellarDate};
use sh_data::Place;
use sh_data::combat::SectionId;
use sh_data::dialogue::Effect;
use sh_sim::{Captain, Contract, ContractKind, ContractState, SimWorld, contract_offers};

use super::{Screen, Terminal};
use crate::career::{MissionLog, contract_title, money, rank_name};
use crate::clock::format_date;
use crate::codex::{Entry, organization_name};
use crate::data::{Data, Texts};
use crate::living::display_name;

/// Lignes visibles d'une liste : au-delà, la liste défile avec la sélection.
pub const VISIBLE_ROWS: usize = 8;

/// La couleur d'une valeur, dans le code de l'appareil : ambre pour ce qui demande l'attention,
/// cyan pour les relevés sains, rouge d'alerte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Normal,
    Dim,
    Accent,
    Screen,
    Danger,
    Disabled,
}

/// Le pictogramme de l'en-tête (dessiné en nœuds d'interface, sans texte).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Glyph {
    /// Une bulle de conversation : interlocuteur sans rôle connu.
    #[default]
    Talk,
    Contracts,
    Market,
    Bank,
    Shipyard,
    Bar,
    Log,
    Sheet,
    Codex,
}

/// Une ligne « libellé … valeur ».
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Field {
    pub label: String,
    pub value: String,
    pub tone: Tone,
    /// Valeur chiffrée : police à chasse fixe.
    pub mono: bool,
    /// La valeur peut passer à la ligne (un nom long, en entier) ; jamais un chiffre, un
    /// montant ou une date.
    pub wrap: bool,
}

impl Field {
    fn text(label: String, value: String) -> Self {
        Self {
            label,
            value,
            wrap: true,
            ..Self::default()
        }
    }

    fn number(label: String, value: String) -> Self {
        Self {
            label,
            value,
            mono: true,
            ..Self::default()
        }
    }

    fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Une valeur qui ne passe jamais à la ligne (une date).
    fn nowrap(mut self) -> Self {
        self.wrap = false;
        self
    }
}

/// Une colonne de liste : largeur fixe en pixels (0 : prend la place restante), alignement.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column {
    pub label: String,
    pub width: f32,
    pub right: bool,
}

impl Column {
    fn flex(label: String) -> Self {
        Self {
            label,
            ..Self::default()
        }
    }

    fn fixed(label: String, width: f32) -> Self {
        Self {
            label,
            width,
            right: false,
        }
    }

    fn right(mut self) -> Self {
        self.right = true;
        self
    }
}

/// Une case de liste.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    pub text: String,
    pub tone: Tone,
    pub mono: bool,
}

impl Cell {
    fn text(text: String) -> Self {
        Self {
            text,
            ..Self::default()
        }
    }

    fn number(text: String) -> Self {
        Self {
            text,
            mono: true,
            ..Self::default()
        }
    }

    fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// Une jauge (état d'une section de coque) : une barre par colonne, de 0 à 1.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gauge {
    pub label: String,
    pub parts: Vec<f64>,
    pub value: String,
    pub tone: Tone,
}

/// Un panneau de détail : la ligne choisie d'une liste, ou un panneau fixe.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Detail {
    pub title: String,
    pub subtitle: Option<(String, Tone)>,
    pub fields: Vec<Field>,
    /// Champs sur deux colonnes (listes longues de valeurs courtes : compétences).
    pub compact: bool,
    /// En-têtes des colonnes de jauges.
    pub gauge_columns: Vec<String>,
    pub gauges: Vec<Gauge>,
    /// Paragraphes (consignes, avertissements).
    pub text: Vec<(String, Tone)>,
}

/// Une ligne de liste et son détail.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
    /// Grisée (le détail explique pourquoi), mais toujours sélectionnable.
    pub enabled: bool,
    /// Repère coloré à gauche de la ligne (`Normal` : aucun).
    pub marker: Tone,
    pub detail: Detail,
}

/// Un onglet (journal, fiche, codex).
#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    pub label: String,
    pub screen: Screen,
    pub active: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Header {
    /// Le nom de l'écran ou de la borne.
    pub title: String,
    /// L'organisation qui tient la borne (ou le lieu, ou la tablette du joueur).
    pub owner: Option<String>,
    pub glyph: Glyph,
    pub date: String,
}

/// Le corps d'une page.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    /// Une conversation : qui parle, sa réplique, ce qu'il raconte (nouvelles) ; les réponses
    /// sont les actions de la page.
    Dialogue {
        speaker: Option<String>,
        line: String,
        extra: Option<String>,
    },
    /// Une liste à gauche, le détail de la ligne choisie à droite ; `below` : un encadré sous
    /// les deux (l'historique du journal).
    List {
        columns: Vec<Column>,
        rows: Vec<Row>,
        empty: String,
        below: Option<Detail>,
        /// La ligne choisie en arrivant sur l'écran.
        preselect: Option<usize>,
    },
    /// Des panneaux côte à côte (banque, chantier, fiche).
    Panels(Vec<Detail>),
}

impl Default for Body {
    fn default() -> Self {
        Body::Panels(Vec::new())
    }
}

/// Où s'affiche une action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Dans le détail de cette ligne, quand elle est choisie.
    Row(usize),
    /// Au pied de ce panneau (les réponses d'une conversation : panneau 0).
    Panel(usize),
    /// Dans la barre du bas de l'écran.
    Bar,
}

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
    PayInstallment,
    Repair,
    /// Ravitailler en carburant lourd : tant d'unités (aucune : le plein).
    Refuel(Option<f64>),
    /// Fermer le terminal et ouvrir l'écran du personnage.
    Character,
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
    pub slot: Slot,
}

impl Choice {
    fn new(label: String, action: Action, slot: Slot) -> Self {
        Self {
            label,
            action,
            enabled: true,
            slot,
        }
    }

    fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Ce qu'a donné la dernière action (confirmation, refus).
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub text: String,
    pub tone: Tone,
}

impl Note {
    pub fn good(text: String) -> Self {
        Self {
            text,
            tone: Tone::Screen,
        }
    }

    pub fn error(text: String) -> Self {
        Self {
            text,
            tone: Tone::Danger,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Page {
    pub header: Header,
    pub readouts: Vec<Field>,
    pub tabs: Vec<Tab>,
    pub body: Body,
    pub choices: Vec<Choice>,
    pub note: Option<Note>,
}

impl Page {
    /// Les lignes de la liste (aucune hors d'une liste).
    pub fn rows(&self) -> &[Row] {
        match &self.body {
            Body::List { rows, .. } => rows,
            _ => &[],
        }
    }

    pub fn preselect(&self) -> Option<usize> {
        match &self.body {
            Body::List { preselect, .. } => *preselect,
            _ => None,
        }
    }

    /// Les actions numérotées (touches 1 à 9), dans l'ordre d'affichage : celles de la ligne
    /// choisie (`cursor`), des panneaux ou de la conversation, puis celles de la barre du bas.
    /// « Partir » n'a pas de numéro : c'est Échap.
    pub fn numbered(&self, cursor: usize) -> Vec<usize> {
        let shown = |c: &Choice| match c.slot {
            Slot::Row(row) => row == cursor,
            _ => true,
        };
        let pick = |bar: bool| {
            self.choices
                .iter()
                .enumerate()
                .filter(move |(_, c)| {
                    !matches!(c.action, Action::Close) && (c.slot == Slot::Bar) == bar && shown(c)
                })
                .map(|(i, _)| i)
        };
        pick(false).chain(pick(true)).collect()
    }

    /// L'action d'Entrée sur une liste : la première action possible de la ligne choisie.
    pub fn primary(&self, cursor: usize) -> Option<usize> {
        self.choices
            .iter()
            .position(|c| c.enabled && c.slot == Slot::Row(cursor))
    }

    /// L'action « Partir ».
    pub fn close_index(&self) -> Option<usize> {
        self.choices
            .iter()
            .position(|c| matches!(c.action, Action::Close))
    }

    /// Le nombre de positions du curseur : les lignes d'une liste, sinon les actions numérotées.
    pub fn cursor_len(&self) -> usize {
        match self.rows().len() {
            0 => self.numbered(0).len(),
            rows => rows,
        }
    }
}

/// La première ligne visible pour que la ligne choisie le reste (défilement minimal).
pub fn scroll_for(cursor: usize, scroll: usize, rows: usize) -> usize {
    if rows <= VISIBLE_ROWS {
        return 0;
    }
    let scroll = if cursor < scroll {
        cursor
    } else if cursor >= scroll + VISIBLE_ROWS {
        cursor + 1 - VISIBLE_ROWS
    } else {
        scroll
    };
    scroll.min(rows - VISIBLE_ROWS)
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
    pub items: &'a BTreeMap<String, u32>,
    /// Les fiches découvertes.
    pub codex: &'a crate::codex::Codex,
    /// Le carburant lourd du vaisseau (unités, capacité), s'il est connu (`voyage::ShipFuel`).
    pub fuel: Option<(f64, f64)>,
    /// Unités consommées par jour de route (`travel.ron`) : la réserve s'affiche en jours.
    pub fuel_per_day: f64,
}

impl Ctx<'_> {
    /// Des unités de carburant, en jours de route.
    fn fuel_days(&self, units: f64) -> f64 {
        if self.fuel_per_day > 0.0 {
            units / self.fuel_per_day
        } else {
            units
        }
    }

    fn fuel_readout(&self) -> Field {
        let t = self.texts;
        match self.fuel {
            Some((have, capacity)) => {
                let fraction = if capacity > 0.0 { have / capacity } else { 1.0 };
                Field::number(
                    t.get("term.readout.fuel"),
                    t.format(
                        "refuel.days_value",
                        &[
                            ("have", &t.number(self.fuel_days(have), 0)),
                            ("capacity", &t.number(self.fuel_days(capacity), 0)),
                        ],
                    ),
                )
                .tone(fraction_tone(fraction))
            }
            None => Field::number(t.get("term.readout.fuel"), "—".to_owned()).tone(Tone::Dim),
        }
    }

    /// Le panneau du carburant lourd (réserve, prix, plein) ; ses actions (le plein, une
    /// tranche) vont au pied du panneau `panel`. Un compte trop maigre n'empêche pas d'essayer :
    /// le chantier refuse alors en clair (`refuel.no_credits`).
    fn fuel_panel(&self, choices: &mut Vec<Choice>, panel: usize) -> Detail {
        let t = self.texts;
        let rules = &self.data.combat.refuel;
        let mut detail = Detail {
            title: t.get("refuel.panel"),
            ..Detail::default()
        };
        let Some((have, capacity)) = self.fuel else {
            detail.text.push((t.get("refuel.no_tank"), Tone::Dim));
            return detail;
        };
        let unit = super::fuel::unit_price(rules.day_price, self.fuel_per_day);
        let fraction = if capacity > 0.0 { have / capacity } else { 1.0 };
        detail.gauges.push(Gauge {
            label: t.get("refuel.tank"),
            parts: vec![fraction],
            value: percent(t, fraction),
            tone: fraction_tone(fraction),
        });
        detail.fields = vec![
            self.fuel_readout(),
            Field::number(
                t.get("refuel.field.price"),
                t.format(
                    "refuel.price_value",
                    &[("price", &t.number(rules.day_price, 0))],
                ),
            ),
        ];
        let full = super::fuel::quote(have, capacity, None, unit);
        match full {
            Some(full) => {
                detail.fields.push(
                    Field::number(t.get("refuel.field.full"), money(t, full.cost))
                        .tone(Tone::Accent),
                );
                if self.cap.credits.cents < full.cost.cents {
                    detail.text.push((t.get("refuel.short"), Tone::Danger));
                }
            }
            None => detail.text.push((t.get("refuel.full"), Tone::Screen)),
        }
        detail.text.push((t.get("refuel.note"), Tone::Dim));
        let step = rules.step_days * self.fuel_per_day.max(0.0);
        let partial = super::fuel::quote(have, capacity, Some(step), unit);
        for (wanted, quote, key) in [
            (None, full, "refuel.full_do"),
            (Some(step), partial, "refuel.step_do"),
        ] {
            let (units, cost) = quote.map_or((0.0, Credits::ZERO), |q| (q.units, q.cost));
            choices.push(
                Choice::new(
                    t.format(
                        key,
                        &[
                            ("days", &t.number(self.fuel_days(units), 0)),
                            ("cost", &money(t, cost)),
                        ],
                    ),
                    Action::Refuel(wanted),
                    Slot::Panel(panel),
                )
                .enabled(quote.is_some()),
            );
        }
        detail
    }

    /// Prix d'achat et de vente d'une marchandise ici (écart du marché autour du prix).
    pub fn prices(&self, commodity: &str) -> Option<(f64, f64)> {
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

    fn header(&self, title: String, owner: Option<String>, glyph: Glyph) -> Header {
        Header {
            title,
            owner,
            glyph,
            date: format_date(self.texts, self.today),
        }
    }

    fn place_name(&self) -> Option<String> {
        self.here.map(|h| display_name(self.data, h.as_str()))
    }

    // --- Relevés du bandeau -------------------------------------------------------------------

    fn credits(&self) -> Field {
        Field::number(
            self.texts.get("term.readout.credits"),
            money(self.texts, self.cap.credits),
        )
        .tone(if self.cap.credits.is_negative() {
            Tone::Danger
        } else {
            Tone::Screen
        })
    }

    fn wuro(&self) -> Field {
        Field::number(
            self.texts.get("term.readout.wuro"),
            self.texts.number(self.cap.wuro.as_f64(), 2),
        )
    }

    fn hold(&self) -> Field {
        let full = self.cap.cargo_load() + 1e-9 >= self.cap.cargo_capacity;
        Field::number(self.texts.get("term.readout.hold"), self.hold_value()).tone(if full {
            Tone::Accent
        } else {
            Tone::Normal
        })
    }

    fn hold_value(&self) -> String {
        self.texts.format(
            "term.hold_value",
            &[
                ("cargo", &self.texts.number(self.cap.cargo_load(), 0)),
                ("capacity", &self.texts.number(self.cap.cargo_capacity, 0)),
            ],
        )
    }

    fn active_contracts(&self) -> Field {
        let count = self
            .cap
            .contracts
            .iter()
            .filter(|c| c.state == ContractState::Active)
            .count();
        Field::number(self.texts.get("term.readout.contracts"), count.to_string())
    }

    fn rate(&self) -> Field {
        Field::number(
            self.texts.get("term.readout.rate"),
            self.texts.format(
                "term.rate_value",
                &[(
                    "rate",
                    &self.texts.number(self.data.career.credits_per_wuro, 1),
                )],
            ),
        )
    }

    fn exchange(&self, slot: Slot) -> Choice {
        let amount = Credits::units(1000);
        Choice::new(
            self.texts
                .format("market.exchange", &[("amount", &money(self.texts, amount))]),
            Action::Exchange(amount),
            slot,
        )
        .enabled(self.cap.credits.cents >= amount.cents)
    }

    // --- Contrats -----------------------------------------------------------------------------

    /// Jours restants avant une date, et leur couleur.
    fn days_left(&self, date: StellarDate) -> (String, Tone) {
        let days = self.today.days_until(date);
        let text = if days < 0 {
            self.texts.get("term.overdue")
        } else {
            self.texts
                .format("term.days_left", &[("days", &days.to_string())])
        };
        let tone = match days {
            ..=0 => Tone::Danger,
            1..=2 => Tone::Accent,
            _ => Tone::Normal,
        };
        (text, tone)
    }

    fn deadline(&self, date: StellarDate) -> (String, Tone) {
        let (days, tone) = self.days_left(date);
        let text = self.texts.format(
            "term.deadline_value",
            &[("date", &format_date(self.texts, date)), ("days", &days)],
        );
        (text, tone)
    }

    fn cargo_text(&self, c: &Contract) -> String {
        let t = self.texts;
        match &c.commodity {
            Some(commodity) if c.kind != ContractKind::Towing => t.format(
                "term.cargo_value",
                &[
                    ("amount", &t.number(c.amount, 0)),
                    ("commodity", &t.get(&format!("commodity.{commodity}"))),
                ],
            ),
            _ => t.get("term.wreck"),
        }
    }

    fn contract_columns(&self) -> Vec<Column> {
        let t = self.texts;
        vec![
            Column::fixed(t.get("term.col.type"), 48.0),
            Column::flex(t.get("term.col.cargo")),
            Column::fixed(t.get("term.col.destination"), 190.0),
            Column::fixed(t.get("term.col.reward"), 100.0).right(),
            Column::fixed(t.get("term.col.deadline"), 56.0).right(),
        ]
    }

    fn contract_cells(&self, c: &Contract) -> Vec<Cell> {
        let (days, tone) = self.days_left(c.deadline);
        vec![
            Cell::number(c.kind.code().to_owned()).tone(Tone::Accent),
            Cell::text(self.cargo_text(c)),
            Cell::text(display_name(self.data, c.to.as_str())),
            Cell::number(money(self.texts, c.reward)).tone(Tone::Screen),
            Cell::number(days).tone(tone),
        ]
    }

    /// Le détail d'un contrat : une offre (`offer`, place en soute et consignes) ou un contrat
    /// en cours (son état).
    fn contract_detail(&self, c: &Contract, offer: bool) -> Detail {
        let t = self.texts;
        let kind = match c.kind {
            ContractKind::Transport => "term.kind.transport",
            ContractKind::Resupply => "term.kind.resupply",
            ContractKind::Towing => "term.kind.towing",
        };
        let (deadline, deadline_tone) = self.deadline(c.deadline);
        let mut detail = Detail {
            title: contract_title(t, self.data, c),
            subtitle: Some((t.get(kind), Tone::Screen)),
            fields: vec![
                Field::text(
                    t.get("term.field.issuer"),
                    organization_name(t, self.data, &c.issuer),
                ),
                Field::text(
                    t.get("term.field.from"),
                    display_name(self.data, c.from.as_str()),
                ),
                Field::text(
                    t.get("term.field.to"),
                    display_name(self.data, c.to.as_str()),
                ),
                Field::text(t.get("term.field.cargo"), self.cargo_text(c)),
                Field::number(t.get("term.field.reward"), money(t, c.reward)).tone(Tone::Screen),
                Field::text(t.get("term.field.deadline"), deadline)
                    .tone(deadline_tone)
                    .nowrap(),
            ],
            ..Detail::default()
        };
        if offer {
            let after = self.cap.cargo_load() + c.amount;
            let fits = after <= self.cap.cargo_capacity + 1e-9;
            if c.commodity.is_some() {
                detail.fields.push(
                    Field::number(
                        t.get("term.field.hold_after"),
                        t.format(
                            "term.hold_value",
                            &[
                                ("cargo", &t.number(after, 0)),
                                ("capacity", &t.number(self.cap.cargo_capacity, 0)),
                            ],
                        ),
                    )
                    .tone(if fits { Tone::Normal } else { Tone::Danger }),
                );
            }
            let note = if c.kind == ContractKind::Towing {
                t.get("contracts.towing")
            } else {
                t.format(
                    "contracts.delivery",
                    &[("issuer", &organization_name(t, self.data, &c.issuer))],
                )
            };
            detail.text.push((note, Tone::Dim));
            if !fits {
                detail.text.push((t.get("contracts.no_room"), Tone::Danger));
            }
        } else {
            let state = match (c.kind, c.towed) {
                (ContractKind::Towing, true) => ("journal.state.towed", Tone::Screen),
                (ContractKind::Towing, false) => ("journal.state.find_wreck", Tone::Accent),
                _ => ("journal.state.active", Tone::Screen),
            };
            detail
                .fields
                .push(Field::text(t.get("term.field.state"), t.get(state.0)).tone(state.1));
        }
        detail
    }

    fn dossier_tabs(&self, active: &Screen) -> Vec<Tab> {
        let t = self.texts;
        [
            ("journal.title", Screen::Journal),
            ("sheet.title", Screen::Sheet),
            ("codex.title", Screen::Codex),
        ]
        .into_iter()
        .map(|(key, screen)| Tab {
            label: t.get(key),
            active: &screen == active
                || matches!((active, &screen), (Screen::CodexEntry(_), Screen::Codex)),
            screen,
        })
        .collect()
    }
}

/// Un texte commençant par une capitale (les noms de marchandises et de rôles sont en
/// minuscules dans les fichiers de langue : ils se glissent dans une phrase).
pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// La couleur d'un état de 0 à 1 (coque, blindage).
pub fn fraction_tone(f: f64) -> Tone {
    if f >= 0.75 {
        Tone::Screen
    } else if f >= 0.4 {
        Tone::Accent
    } else {
        Tone::Danger
    }
}

fn percent(t: &Texts, f: f64) -> String {
    t.format("term.percent", &[("value", &t.number(f * 100.0, 0))])
}

/// Le contenu d'un écran.
pub fn page(ctx: &Ctx, term: &Terminal) -> Page {
    let t = ctx.texts;
    let mut page = match &term.screen {
        Screen::Dialogue { id, node } => dialogue_page(ctx, id, node, term.extra.as_deref()),
        Screen::Contracts => contracts_page(ctx),
        Screen::Market => market_page(ctx, None),
        Screen::Commodity(c) => market_page(ctx, Some(c)),
        Screen::Bank => bank_page(ctx),
        Screen::Repair => repair_page(ctx),
        Screen::Sheet => sheet_page(ctx),
        Screen::Journal => journal_page(ctx),
        Screen::Codex => codex_page(ctx, None),
        Screen::CodexEntry(entry) => codex_page(ctx, Some(entry)),
    };
    // Un service ouvert depuis une conversation y ramène.
    if let Some(id) = &term.origin
        && !matches!(term.screen, Screen::Dialogue { .. })
    {
        page.choices.push(Choice::new(
            t.get("terminal.back_to_talk"),
            Action::Goto(Screen::Dialogue {
                id: id.clone(),
                node: String::new(),
            }),
            Slot::Bar,
        ));
    }
    page.note = term.note.clone();
    if !page.choices.iter().any(|c| c.action.closes()) {
        page.choices.push(Choice::new(
            t.get("terminal.close"),
            Action::Close,
            Slot::Bar,
        ));
    }
    page
}

fn role_glyph(role: Option<&str>) -> Glyph {
    match role {
        Some("contracts") => Glyph::Contracts,
        Some("bank") => Glyph::Bank,
        Some("market") => Glyph::Market,
        Some("shipyard") => Glyph::Shipyard,
        Some("bar") => Glyph::Bar,
        _ => Glyph::Talk,
    }
}

fn dialogue_page(ctx: &Ctx, id: &str, node: &str, extra: Option<&str>) -> Page {
    let t = ctx.texts;
    let Some(dialogue) = ctx.data.dialogues.iter().find(|d| d.id == id) else {
        return Page::default();
    };
    let Some(n) = dialogue.nodes.get(node) else {
        return Page::default();
    };
    let owner = dialogue
        .faction
        .as_ref()
        .map(|f| organization_name(t, ctx.data, f))
        .or_else(|| ctx.place_name());
    // Qui parle : le rôle du PNJ de comptoir, s'il a un libellé.
    let speaker = dialogue
        .role
        .as_ref()
        .map(|role| format!("npc.role.{role}"))
        .filter(|key| t.has(key))
        .map(|key| capitalize(&t.get(&key)));
    let choices = n
        .choices
        .iter()
        .filter(|c| {
            ctx.cap
                .satisfies_all(ctx.data, &dialogue.npc, &c.when, ctx.today)
        })
        .map(|c| {
            Choice::new(
                t.get(&c.text),
                Action::Say {
                    key: c.text.clone(),
                    effects: c.effects.clone(),
                    next: c.next.clone(),
                },
                Slot::Panel(0),
            )
        })
        .collect();
    Page {
        header: ctx.header(
            t.get(&dialogue.name_key),
            owner,
            role_glyph(dialogue.role.as_deref()),
        ),
        body: Body::Dialogue {
            speaker,
            line: t.get(&n.text),
            extra: extra.map(str::to_owned),
        },
        choices,
        ..Page::default()
    }
}

fn contracts_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let issuer = &ctx.data.career.contracts.issuer;
    let mut page = Page {
        header: ctx.header(
            t.get("contracts.title"),
            Some(organization_name(t, ctx.data, issuer)),
            Glyph::Contracts,
        ),
        readouts: vec![ctx.credits(), ctx.hold(), ctx.active_contracts()],
        ..Page::default()
    };
    if ctx.cap.refused_by(ctx.data, issuer) {
        page.body = Body::Panels(vec![Detail {
            title: t.get("contracts.refused_title"),
            text: vec![(t.get("contracts.refused"), Tone::Danger)],
            ..Detail::default()
        }]);
        return page;
    }
    let taken = |c: &Contract| ctx.cap.contracts.iter().any(|k| k.id == c.id);
    let mut rows = Vec::new();
    for offer in ctx.offers().into_iter().filter(|c| !taken(c)) {
        let fits = ctx.cap.cargo_load() + offer.amount <= ctx.cap.cargo_capacity + 1e-9;
        let row = rows.len();
        rows.push(Row {
            cells: ctx.contract_cells(&offer),
            enabled: fits,
            marker: Tone::Normal,
            detail: ctx.contract_detail(&offer, true),
        });
        page.choices.push(
            Choice::new(
                t.get("contracts.accept"),
                Action::Accept(offer),
                Slot::Row(row),
            )
            .enabled(fits),
        );
    }
    page.body = Body::List {
        columns: ctx.contract_columns(),
        rows,
        empty: t.get("contracts.none"),
        below: None,
        preselect: None,
    };
    page
}

fn market_page(ctx: &Ctx, preselect: Option<&str>) -> Page {
    let t = ctx.texts;
    let mut page = Page {
        header: ctx.header(t.get("market.title"), ctx.place_name(), Glyph::Market),
        readouts: vec![ctx.credits(), ctx.wuro(), ctx.hold(), ctx.rate()],
        ..Page::default()
    };
    if ctx.here.is_none() {
        page.body = Body::Panels(vec![Detail {
            text: vec![(t.get("console.market.message"), Tone::Dim)],
            ..Detail::default()
        }]);
        return page;
    }
    let room = (ctx.cap.cargo_capacity - ctx.cap.cargo_load()).max(0.0);
    let mut rows = Vec::new();
    let mut selected = None;
    for commodity in ctx.data.sim.base_prices.keys() {
        let Some((buy, sell)) = ctx.prices(commodity) else {
            continue;
        };
        let held = ctx.cap.cargo.get(commodity).copied().unwrap_or(0.0);
        let name = capitalize(&t.get(&format!("commodity.{commodity}")));
        let row = rows.len();
        if preselect == Some(commodity.as_str()) {
            selected = Some(row);
        }
        let price = |p: f64| t.format("market.unit_price", &[("price", &t.number(p, 0))]);
        rows.push(Row {
            cells: vec![
                Cell::text(name.clone()),
                Cell::number(t.number(buy, 0)),
                Cell::number(t.number(sell, 0)),
                Cell::number(t.number(held, 0)).tone(if held > 0.0 {
                    Tone::Screen
                } else {
                    Tone::Dim
                }),
            ],
            enabled: true,
            marker: Tone::Normal,
            detail: Detail {
                title: name,
                subtitle: Some((t.get("market.goods"), Tone::Screen)),
                fields: vec![
                    Field::number(t.get("market.field.buy"), price(buy)),
                    Field::number(t.get("market.field.sell"), price(sell)),
                    Field::number(t.get("market.field.held"), t.number(held, 0))
                        .tone(if held > 0.0 { Tone::Screen } else { Tone::Dim }),
                    Field::number(t.get("market.field.room"), t.number(room, 0)),
                    ctx.credits(),
                ],
                text: vec![(t.get("market.hint"), Tone::Dim)],
                ..Detail::default()
            },
        });
        let slot = Slot::Row(row);
        for amount in [10.0, 50.0] {
            page.choices.push(Choice::new(
                t.format("market.buy", &[("amount", &t.number(amount, 0))]),
                Action::Buy(commodity.clone(), amount),
                slot,
            ));
        }
        page.choices.push(
            Choice::new(
                t.format("market.sell", &[("amount", &t.number(10.0, 0))]),
                Action::Sell(commodity.clone(), 10.0),
                slot,
            )
            .enabled(held + 1e-9 >= 10.0),
        );
        page.choices.push(
            Choice::new(
                t.format("market.sell_all", &[("amount", &t.number(held, 0))]),
                Action::Sell(commodity.clone(), held),
                slot,
            )
            .enabled(held > 0.0),
        );
    }
    // Détourner une cargaison sous contrat : elle se vend ici comme une autre.
    for c in ctx
        .cap
        .contracts
        .iter()
        .filter(|c| c.state == ContractState::Active && c.commodity.is_some())
    {
        let commodity = c.commodity.as_deref().unwrap_or_default();
        let sell = ctx.prices(commodity).map_or(0.0, |(_, sell)| sell);
        let value = Credits::from_cents((c.amount * sell * 100.0).round() as i64);
        let row = rows.len();
        rows.push(Row {
            cells: vec![
                Cell::text(t.format(
                    "market.contract_row",
                    &[("cargo", &ctx.cargo_text(c)), ("code", c.kind.code())],
                ))
                .tone(Tone::Danger),
                Cell::number("—".to_owned()).tone(Tone::Dim),
                Cell::number(t.number(c.amount * sell, 0)).tone(Tone::Danger),
                Cell::number(t.number(c.amount, 0)),
            ],
            enabled: true,
            marker: Tone::Danger,
            detail: Detail {
                title: contract_title(t, ctx.data, c),
                subtitle: Some((t.get("market.under_contract"), Tone::Danger)),
                fields: vec![
                    Field::text(
                        t.get("term.field.issuer"),
                        organization_name(t, ctx.data, &c.issuer),
                    ),
                    Field::text(
                        t.get("term.field.to"),
                        display_name(ctx.data, c.to.as_str()),
                    ),
                    Field::number(t.get("market.field.value"), money(t, value)).tone(Tone::Danger),
                ],
                text: vec![(
                    t.format(
                        "market.betray_warning",
                        &[("issuer", &organization_name(t, ctx.data, &c.issuer))],
                    ),
                    Tone::Danger,
                )],
                ..Detail::default()
            },
        });
        page.choices.push(Choice::new(
            t.get("market.betray"),
            Action::Betray(c.id.clone()),
            Slot::Row(row),
        ));
    }
    page.choices.push(ctx.exchange(Slot::Bar));
    page.body = Body::List {
        columns: vec![
            Column::flex(t.get("term.col.commodity")),
            Column::fixed(t.get("term.col.buy"), 92.0).right(),
            Column::fixed(t.get("term.col.sell"), 92.0).right(),
            Column::fixed(t.get("term.col.held"), 92.0).right(),
        ],
        rows,
        empty: t.get("market.none"),
        below: None,
        preselect: selected,
    };
    page
}

fn bank_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let rules = &ctx.data.career.bank;
    let loan = ctx.cap.loan.as_ref();
    let lender = loan.map_or(&rules.lender, |l| &l.lender);
    let mut readouts = vec![ctx.credits(), ctx.wuro()];
    let mut debt = Detail {
        title: t.get("bank.panel.loan"),
        ..Detail::default()
    };
    match loan {
        Some(loan) => {
            let (due, due_tone) = ctx.deadline(loan.next_due);
            readouts.push(
                Field::number(t.get("term.readout.debt"), money(t, loan.balance))
                    .tone(Tone::Accent),
            );
            debt.fields = vec![
                Field::text(
                    t.get("bank.field.lender"),
                    organization_name(t, ctx.data, &loan.lender),
                ),
                Field::number(t.get("bank.field.balance"), money(t, loan.balance))
                    .tone(Tone::Accent),
                Field::number(t.get("bank.field.installment"), money(t, loan.installment)),
                Field::text(t.get("bank.field.next_due"), due)
                    .tone(due_tone)
                    .nowrap(),
                Field::number(
                    t.get("bank.field.missed"),
                    t.format(
                        "bank.missed_value",
                        &[
                            ("missed", &loan.missed.to_string()),
                            ("limit", &rules.seize_after_missed.to_string()),
                        ],
                    ),
                )
                .tone(if loan.missed > 0 {
                    Tone::Danger
                } else {
                    Tone::Normal
                }),
            ];
            debt.text.push((
                t.format(
                    "bank.rules",
                    &[
                        ("days", &rules.every_days.to_string()),
                        ("limit", &rules.seize_after_missed.to_string()),
                    ],
                ),
                Tone::Dim,
            ));
        }
        None => debt.text.push((t.get("bank.no_debt"), Tone::Screen)),
    }
    let account = Detail {
        title: t.get("bank.panel.account"),
        fields: vec![ctx.credits(), ctx.wuro(), ctx.rate()],
        text: vec![(t.get("bank.exchange_note"), Tone::Dim)],
        ..Detail::default()
    };
    let installment = loan.map(|l| {
        if l.balance.cents < l.installment.cents {
            l.balance
        } else {
            l.installment
        }
    });
    let pay = Choice::new(
        t.format(
            "bank.pay",
            &[(
                "amount",
                &money(t, installment.unwrap_or(Credits::from_cents(0))),
            )],
        ),
        Action::PayInstallment,
        Slot::Panel(0),
    )
    .enabled(installment.is_some());
    Page {
        header: ctx.header(
            t.get("bank.title"),
            Some(organization_name(t, ctx.data, lender)),
            Glyph::Bank,
        ),
        readouts,
        body: Body::Panels(vec![debt, account]),
        choices: vec![pay, ctx.exchange(Slot::Panel(1))],
        ..Page::default()
    }
}

fn repair_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let rules = &ctx.data.combat.repair;
    let mut page = Page {
        header: ctx.header(
            t.get("repair.title"),
            Some(organization_name(t, ctx.data, &rules.issuer)),
            Glyph::Shipyard,
        ),
        ..Page::default()
    };
    let Some(hull) = ctx.cap.hull.as_ref() else {
        page.readouts = vec![ctx.fuel_readout(), ctx.credits()];
        let fuel = ctx.fuel_panel(&mut page.choices, 1);
        page.body = Body::Panels(vec![
            Detail {
                title: t.get("repair.panel.hull"),
                text: vec![(t.get("repair.no_hull"), Tone::Dim)],
                ..Detail::default()
            },
            fuel,
        ]);
        return page;
    };
    let cost = hull.repair_cost(rules);
    let integrity = hull.integrity();
    let damaged = cost.cents > 0;
    // Les chantiers de ce système prennent-ils le vaisseau du joueur ? (`combat.ron`)
    let here_ok = ctx.here.is_some_and(|h| rules.systems.contains(h));
    page.readouts = vec![
        Field::number(t.get("term.readout.integrity"), percent(t, integrity))
            .tone(fraction_tone(integrity)),
        Field::number(t.get("term.readout.quote"), money(t, cost)).tone(if damaged {
            Tone::Accent
        } else {
            Tone::Normal
        }),
        ctx.fuel_readout(),
        ctx.credits(),
    ];
    let ratio = |now: f64, max: f64| if max <= 0.0 { 1.0 } else { now / max };
    let mut gauges: Vec<Gauge> = SectionId::ALL
        .iter()
        .filter_map(|id| Some((id, hull.sections.get(id)?)))
        .map(|(id, s)| {
            let whole = ratio(s.armor + s.structure, s.max_armor + s.max_structure);
            Gauge {
                label: t.get(&format!("repair.section.{}", id.key())),
                parts: vec![
                    ratio(s.armor, s.max_armor),
                    ratio(s.structure, s.max_structure),
                ],
                value: percent(t, whole),
                tone: fraction_tone(whole),
            }
        })
        .collect();
    let core = ratio(hull.core, hull.max_core);
    gauges.push(Gauge {
        label: t.get("repair.core"),
        parts: vec![core],
        value: percent(t, core),
        tone: fraction_tone(core),
    });
    let state = Detail {
        title: t.get("repair.panel.hull"),
        gauge_columns: vec![t.get("repair.col.armor"), t.get("repair.col.structure")],
        gauges,
        ..Detail::default()
    };
    let paid = if ctx.cap.credits.cents >= cost.cents {
        cost
    } else {
        Credits::from_cents(ctx.cap.credits.cents.max(0))
    };
    let mut quote = Detail {
        title: t.get("repair.panel.quote"),
        fields: vec![
            Field::number(t.get("repair.field.integrity"), percent(t, integrity))
                .tone(fraction_tone(integrity)),
            Field::number(t.get("repair.field.cost"), money(t, cost)).tone(if damaged {
                Tone::Accent
            } else {
                Tone::Normal
            }),
            ctx.credits(),
            Field::number(
                t.get("repair.field.after"),
                money(t, Credits::from_cents(ctx.cap.credits.cents - paid.cents)),
            ),
        ],
        text: vec![(t.get("repair.note"), Tone::Dim)],
        ..Detail::default()
    };
    if !damaged {
        quote.text.push((t.get("repair.intact"), Tone::Screen));
    } else if !here_ok {
        quote.text.push((t.get("repair.not_here"), Tone::Danger));
    }
    // Le compte n'y suffit pas : le chantier répare au prorata de ce qui est payé.
    let label = if ctx.cap.credits.cents >= cost.cents {
        t.format("repair.do", &[("cost", &money(t, cost))])
    } else {
        t.format(
            "repair.partial",
            &[("paid", &money(t, paid)), ("cost", &money(t, cost))],
        )
    };
    page.choices.push(
        Choice::new(label, Action::Repair, Slot::Panel(1))
            .enabled(damaged && here_ok && ctx.cap.credits.cents > 0),
    );
    let fuel = ctx.fuel_panel(&mut page.choices, 2);
    page.body = Body::Panels(vec![state, quote, fuel]);
    page
}

fn journal_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let active: Vec<&Contract> = ctx
        .cap
        .contracts
        .iter()
        .filter(|c| c.state == ContractState::Active)
        .collect();
    let rows = active
        .iter()
        .map(|c| Row {
            cells: ctx.contract_cells(c),
            enabled: true,
            marker: Tone::Normal,
            detail: ctx.contract_detail(c, false),
        })
        .collect();
    let mut history: Vec<(String, Tone)> = ctx
        .log
        .lines
        .iter()
        .rev()
        .take(5)
        .map(|line| (line.clone(), Tone::Normal))
        .collect();
    if history.is_empty() {
        history.push((t.get("journal.empty"), Tone::Dim));
    }
    Page {
        header: ctx.header(
            t.get("journal.title"),
            Some(t.get("term.datapad")),
            Glyph::Log,
        ),
        readouts: vec![ctx.active_contracts(), ctx.credits(), ctx.hold()],
        tabs: ctx.dossier_tabs(&Screen::Journal),
        body: Body::List {
            columns: ctx.contract_columns(),
            rows,
            empty: t.get("journal.none"),
            below: Some(Detail {
                subtitle: Some((t.get("journal.history"), Tone::Screen)),
                text: history,
                ..Detail::default()
            }),
            preselect: None,
        },
        ..Page::default()
    }
}

fn sheet_page(ctx: &Ctx) -> Page {
    let t = ctx.texts;
    let cap = ctx.cap;
    let rules = &ctx.data.career;
    let mut account = Detail {
        title: t.get("sheet.panel.account"),
        fields: vec![ctx.credits(), ctx.wuro()],
        ..Detail::default()
    };
    match &cap.loan {
        Some(loan) => {
            let (due, tone) = ctx.deadline(loan.next_due);
            account.fields.push(
                Field::number(
                    t.format(
                        "sheet.field.debt",
                        &[("lender", &organization_name(t, ctx.data, &loan.lender))],
                    ),
                    money(t, loan.balance),
                )
                .tone(Tone::Accent),
            );
            account.fields.push(Field::number(
                t.get("bank.field.installment"),
                money(t, loan.installment),
            ));
            account.fields.push(
                Field::text(t.get("sheet.field.next_due"), due)
                    .tone(tone)
                    .nowrap(),
            );
        }
        None => account.fields.push(
            Field::text(t.get("sheet.field.no_debt"), t.get("sheet.no_debt")).tone(Tone::Screen),
        ),
    }
    account.fields.push(ctx.hold());
    for (commodity, amount) in &cap.cargo {
        account.fields.push(Field::number(
            capitalize(&t.get(&format!("commodity.{commodity}"))),
            t.number(*amount, 0),
        ));
    }
    for (id, count) in ctx.items {
        account.fields.push(Field::number(
            capitalize(&t.get(&format!("item.{id}"))),
            t.format("sheet.count", &[("count", &count.to_string())]),
        ));
    }
    let mut standing = Detail {
        title: t.get("sheet.panel.reputation"),
        ..Detail::default()
    };
    for (faction, value) in &cap.reputation {
        // Une organisation secrète n'apparaît jamais (règle d'or n° 3).
        if ctx.data.faction(faction).is_none_or(|f| f.secret) {
            continue;
        }
        let expelled = cap.expelled.contains(faction);
        let mut shown = t.format(
            "sheet.rep_value",
            &[
                ("tier", &t.get(cap.tier(rules, faction))),
                ("value", &format!("{value:+}")),
            ],
        );
        if let Some(rank) = cap.ranks.get(faction) {
            shown = t.format(
                "sheet.with_rank",
                &[
                    ("standing", &shown),
                    ("rank", &rank_name(t, ctx.data, faction, *rank)),
                ],
            );
        }
        let tone = if expelled || *value <= -20 {
            Tone::Danger
        } else if *value >= 20 {
            Tone::Screen
        } else {
            Tone::Normal
        };
        if expelled {
            shown = t.format("sheet.closed", &[("standing", &shown)]);
        }
        standing
            .fields
            .push(Field::text(organization_name(t, ctx.data, faction), shown).tone(tone));
    }
    if standing.fields.is_empty() {
        standing
            .text
            .push((t.get("sheet.no_reputation"), Tone::Dim));
    }
    let mut skills =
        Detail {
            title: t.get("sheet.panel.skills"),
            compact: true,
            fields: rules
                .skills
                .iter()
                .map(|s| {
                    let level = cap.skill_level(rules, s);
                    Field::number(t.get(&format!("skill.{s}")), level.to_string())
                        .tone(if level > 0 { Tone::Screen } else { Tone::Dim })
                })
                .collect(),
            ..Detail::default()
        };
    if !cap.traits.is_empty() {
        let traits: Vec<String> = cap
            .traits
            .iter()
            .map(|id| t.get(&format!("trait.{id}")))
            .collect();
        skills.text.push((
            t.format("sheet.traits", &[("traits", &traits.join(", "))]),
            Tone::Accent,
        ));
    }
    Page {
        header: ctx.header(
            t.get("sheet.title"),
            Some(t.get("term.datapad")),
            Glyph::Sheet,
        ),
        readouts: vec![ctx.credits(), ctx.wuro(), ctx.hold()],
        tabs: ctx.dossier_tabs(&Screen::Sheet),
        body: Body::Panels(vec![account, standing, skills]),
        choices: vec![Choice::new(
            t.get("sheet.character"),
            Action::Character,
            Slot::Bar,
        )],
        ..Page::default()
    }
}

fn codex_page(ctx: &Ctx, preselect: Option<&Entry>) -> Page {
    let t = ctx.texts;
    let entries: Vec<(Entry, &str)> = ctx
        .codex
        .places
        .iter()
        .map(|p| (Entry::Place(p.clone()), "codex.type.place"))
        .chain(
            ctx.codex
                .factions
                .iter()
                .map(|f| (Entry::Faction(f.clone()), "codex.type.faction")),
        )
        .collect();
    let selected = preselect.and_then(|wanted| entries.iter().position(|(e, _)| e == wanted));
    let rows = entries
        .iter()
        .map(|(entry, kind)| {
            let (title, lines) = crate::codex::describe(t, ctx.data, entry)
                .unwrap_or_else(|| (crate::codex::entry_name(t, ctx.data, entry), Vec::new()));
            Row {
                cells: vec![
                    Cell::text(crate::codex::entry_name(t, ctx.data, entry)),
                    Cell::text(t.get(kind)).tone(Tone::Dim),
                ],
                enabled: true,
                marker: Tone::Normal,
                detail: Detail {
                    title,
                    subtitle: Some((t.get(kind), Tone::Screen)),
                    text: lines.into_iter().map(|l| (l, Tone::Normal)).collect(),
                    ..Detail::default()
                },
            }
        })
        .collect();
    Page {
        header: ctx.header(
            t.get("codex.title"),
            Some(t.get("term.datapad")),
            Glyph::Codex,
        ),
        readouts: vec![Field::number(
            t.get("term.readout.entries"),
            ctx.codex.len().to_string(),
        )],
        tabs: ctx.dossier_tabs(&Screen::Codex),
        body: Body::List {
            columns: vec![
                Column::flex(t.get("term.col.entry")),
                Column::fixed(t.get("term.col.kind"), 150.0),
            ],
            rows,
            empty: t.get("codex.empty"),
            below: None,
            preselect: selected,
        },
        ..Page::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(slot: Slot, action: Action) -> Choice {
        Choice::new(String::new(), action, slot)
    }

    #[test]
    fn numeros_de_la_ligne_choisie_puis_de_la_barre() {
        let page = Page {
            choices: vec![
                choice(Slot::Row(0), Action::Repair),
                choice(Slot::Bar, Action::Character),
                choice(Slot::Row(1), Action::PayInstallment),
                choice(Slot::Row(1), Action::Repair).enabled(false),
                choice(Slot::Bar, Action::Close),
            ],
            ..Page::default()
        };
        assert_eq!(page.numbered(0), vec![0, 1]);
        assert_eq!(page.numbered(1), vec![2, 3, 1]);
        assert_eq!(page.primary(1), Some(2));
        assert_eq!(page.primary(2), None);
        assert_eq!(page.close_index(), Some(4));
    }

    #[test]
    fn la_liste_defile_avec_la_selection() {
        assert_eq!(scroll_for(3, 0, 5), 0);
        assert_eq!(scroll_for(VISIBLE_ROWS, 0, 20), 1);
        assert_eq!(scroll_for(2, 5, 20), 2);
        assert_eq!(scroll_for(19, 0, 20), 20 - VISIBLE_ROWS);
        assert_eq!(scroll_for(4, 3, 20), 3);
    }

    #[test]
    fn capitale_accentuee() {
        assert_eq!(capitalize("épave"), "Épave");
        assert_eq!(capitalize(""), "");
    }
}
