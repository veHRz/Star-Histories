//! Dessin de l'écran du personnage : le contenu affiché (calculé des ressources du jeu, comparé
//! d'une fois sur l'autre pour ne redessiner qu'en cas de changement) et les entités
//! d'interface qui le montrent.
//!
//! Disposition (panneau d'appareil à pans coupés, comme le terminal) :
//! - Inventaire : en haut, trois colonnes (équipement et compte | grille des objets | détail) ;
//!   en bas, deux colonnes (réputations | emprunts).
//! - Compétences : une carte par compétence sur deux colonnes (nom, niveau, barre, points), puis
//!   les traits.
//! - Casier : deux grilles côte à côte (casier | inventaire), puis le détail en bandeau.

use std::collections::BTreeMap;

use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui_widgets::{ActivateOnPress, Button};
use sh_data::EquipSlot;
use sh_sim::Captain;

use super::ActiveTab;
use super::logic::{self, Stack, Step};
use super::{CellRef, CharacterUi, DetailPane, GridCell, Icons, ScreenButton, ScreenView, Sources};
use crate::career::{faction_name, money, rank_name};
use crate::clock::format_date;
use crate::data::{Data, Texts};
use crate::items::CharacterTab;
use crate::state::AppState;
use crate::ui::theme::{self, Frame, UiFonts};

/// Côté d'une case, en pixels d'interface.
const CELL: f32 = 64.0;
/// Côté de l'icône dans sa case.
const ICON: f32 = 52.0;
const GAP: f32 = 6.0;
/// Grille de l'onglet Inventaire : au moins 6 × 4 cases.
const PACK_COLUMNS: usize = 6;
const PACK_ROWS: usize = 4;
/// Grilles du casier (casier et inventaire côte à côte) : au moins 5 × 4 cases chacune.
const BOX_COLUMNS: usize = 5;
const BOX_ROWS: usize = 4;
/// Réputations montrées ; les autres (les plus faibles) sont comptées.
const MAX_REPUTATIONS: usize = 6;
/// Largeur du panneau : onglets (trois colonnes), casier (deux grilles).
const PANEL_WIDE: f32 = 1080.0;
const PANEL_NARROW: f32 = 820.0;
const LEFT_WIDTH: f32 = 240.0;
const DETAIL_WIDTH: f32 = 300.0;
const GAUGE_WIDTH: f32 = 110.0;

/// Styles propres à l'écran : cases en verre sombre, survol cyan (relevé), choix ambre (voyant).
const CELL_FILL: Color = Color::srgba(0.015, 0.018, 0.022, 0.80);
const CELL_FILL_HOVER: Color = Color::srgba(0.45, 0.82, 0.90, 0.10);
const CELL_FILL_SELECTED: Color = Color::srgba(0.95, 0.66, 0.24, 0.10);
const CELL_EDGE: Color = Color::srgba(0.42, 0.45, 0.49, 0.55);
const CELL_EDGE_EMPTY: Color = Color::srgba(0.42, 0.45, 0.49, 0.22);
const CARD_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.025);
const BAR_BACK: Color = Color::srgba(1.0, 1.0, 1.0, 0.07);
const BAR_TICK: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);

// --- Contenu -----------------------------------------------------------------------------------

/// Ce que l'écran affiche : deux contenus égaux donnent le même écran.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Content {
    pub title: String,
    pub hint: String,
    /// L'onglet ouvert ; `None` : un casier (pas d'onglets).
    pub tabs: Option<CharacterTab>,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Body {
    Inventory(InventoryBody),
    Skills(SkillsBody),
    Container(ContainerBody),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct InventoryBody {
    slots: Vec<SlotEntry>,
    pack: Vec<Stack>,
    pack_cells: usize,
    credits: String,
    wuro: String,
    reputations: Vec<RepEntry>,
    more_reputations: usize,
    loans: Vec<LoanEntry>,
}

#[derive(Clone, Debug, PartialEq)]
struct SlotEntry {
    slot: EquipSlot,
    item: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct RepEntry {
    name: String,
    tier: String,
    value: i32,
    /// Grade, organisation fermée.
    note: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct LoanEntry {
    lender: String,
    balance: String,
    installment: String,
    /// La date de la prochaine échéance, et le délai qui reste (« dans 30 jours »).
    due: String,
    due_note: Option<String>,
    missed: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SkillsBody {
    skills: Vec<SkillEntry>,
    traits: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct SkillEntry {
    name: String,
    level: u32,
    xp: u64,
    next: u64,
    left: u64,
    /// La barre, en millièmes.
    permille: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ContainerBody {
    stash: Vec<Stack>,
    stash_cells: usize,
    pack: Vec<Stack>,
    pack_cells: usize,
}

pub(super) fn content(src: &Sources, view: &ScreenView) -> Content {
    let t: &Texts = &src.texts;
    match view {
        ScreenView::Tab(tab) => Content {
            title: t.get("character.title"),
            hint: t.get(match tab {
                CharacterTab::Inventory => "character.hint.inventory",
                CharacterTab::Skills => "character.hint.skills",
            }),
            tabs: Some(*tab),
            body: match tab {
                CharacterTab::Inventory => Body::Inventory(inventory_body(src)),
                CharacterTab::Skills => Body::Skills(skills_body(src)),
            },
        },
        ScreenView::Container(id) => Content {
            title: t.get("character.container.title"),
            hint: t.get("character.hint.container"),
            tabs: None,
            body: Body::Container(container_body(src, id)),
        },
    }
}

fn inventory_body(src: &Sources) -> InventoryBody {
    let t: &Texts = &src.texts;
    let data: &Data = &src.data;
    let empty = BTreeMap::new();
    let items = src
        .inventory
        .as_ref()
        .map_or(&empty, |inventory| &inventory.items);
    let pack = logic::stacks(items, &data.items);
    let slots = EquipSlot::ALL
        .iter()
        .map(|slot| SlotEntry {
            slot: *slot,
            item: src
                .equipment
                .as_ref()
                .and_then(|equipment| equipment.get(*slot))
                .map(str::to_owned),
        })
        .collect();
    let today = src.clock.as_ref().map(|clock| clock.date);
    let captain = src.career.as_ref().map(|career| &career.captain);
    let mut reputations = captain.map_or_else(Vec::new, |cap| reputations(t, data, cap));
    let more_reputations = reputations.len().saturating_sub(MAX_REPUTATIONS);
    reputations.truncate(MAX_REPUTATIONS);
    InventoryBody {
        slots,
        pack_cells: logic::grid_cells(pack.len(), PACK_COLUMNS, PACK_ROWS),
        pack,
        credits: captain.map_or_else(|| t.get("character.none"), |cap| money(t, cap.credits)),
        wuro: captain.map_or_else(
            || t.get("character.none"),
            |cap| {
                t.format(
                    "character.wuro",
                    &[("amount", &t.number(cap.wuro.as_f64(), 2))],
                )
            },
        ),
        reputations,
        more_reputations,
        loans: captain.map_or_else(Vec::new, |cap| loans(t, data, cap, today)),
    }
}

/// Les organisations connues, les plus marquées d'abord. Comme la fiche du terminal, une
/// organisation secrète n'apparaît jamais (règle d'or n° 3).
fn reputations(t: &Texts, data: &Data, cap: &Captain) -> Vec<RepEntry> {
    let rules = &data.career;
    let mut rows: Vec<RepEntry> = cap
        .reputation
        .iter()
        .filter(|(faction, _)| data.faction(faction).is_some_and(|f| !f.secret))
        .map(|(faction, value)| {
            let mut notes = Vec::new();
            if let Some(rank) = cap.ranks.get(faction) {
                let rank = rank_name(t, data, faction, *rank);
                if !rank.is_empty() {
                    notes.push(rank);
                }
            }
            if cap.expelled.contains(faction) {
                notes.push(t.get("character.rep.expelled"));
            }
            RepEntry {
                name: faction_name(t, data, faction),
                tier: t.get(cap.tier(rules, faction)),
                value: *value,
                note: (!notes.is_empty()).then(|| notes.join(" · ")),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.value
            .unsigned_abs()
            .cmp(&a.value.unsigned_abs())
            .then_with(|| a.name.cmp(&b.name))
    });
    rows
}

/// Les emprunts en cours (l'hypothèque de la Banque, P23) : créancier, reste dû, prochaine
/// échéance et sa date.
fn loans(
    t: &Texts,
    data: &Data,
    cap: &Captain,
    today: Option<sh_core::StellarDate>,
) -> Vec<LoanEntry> {
    cap.loan
        .iter()
        .map(|loan| {
            // La dernière échéance ne dépasse pas le reste dû (`Captain::pay_installment`).
            let installment = if loan.balance.cents < loan.installment.cents {
                loan.balance
            } else {
                loan.installment
            };
            let due_note = match today.map(|d| d.days_until(loan.next_due)) {
                Some(0) => Some(t.get("character.loan.today")),
                Some(1) => Some(t.get("character.loan.tomorrow")),
                Some(days) if days > 1 => Some(t.format(
                    "character.loan.in_days",
                    &[("days", &t.number(days as f64, 0))],
                )),
                _ => None,
            };
            LoanEntry {
                lender: faction_name(t, data, &loan.lender),
                balance: money(t, loan.balance),
                installment: money(t, installment),
                due: format_date(t, loan.next_due),
                due_note,
                missed: loan.missed,
            }
        })
        .collect()
}

fn skills_body(src: &Sources) -> SkillsBody {
    let t: &Texts = &src.texts;
    let rules = &src.data.career;
    let captain = src.career.as_ref().map(|career| &career.captain);
    let skills = rules
        .skills
        .iter()
        .map(|skill| {
            let xp = captain
                .and_then(|cap| cap.skills.get(skill))
                .copied()
                .unwrap_or(0.0);
            let progress = logic::progress(xp, rules.xp_per_level);
            SkillEntry {
                name: t.get(&format!("skill.{skill}")),
                level: progress.level,
                xp: progress.xp.floor() as u64,
                next: progress.next.round() as u64,
                left: progress.remaining().ceil() as u64,
                permille: (progress.fraction() * 1000.0).round() as u16,
            }
        })
        .collect();
    let traits = captain.map_or_else(Vec::new, |cap| {
        cap.traits
            .iter()
            .map(|id| t.get(&format!("trait.{id}")))
            .collect()
    });
    SkillsBody { skills, traits }
}

fn container_body(src: &Sources, id: &str) -> ContainerBody {
    let data: &Data = &src.data;
    let empty = BTreeMap::new();
    let stash_items = src
        .containers
        .as_ref()
        .and_then(|containers| containers.contents(id))
        .unwrap_or(&empty);
    let items = src
        .inventory
        .as_ref()
        .map_or(&empty, |inventory| &inventory.items);
    let stash = logic::stacks(stash_items, &data.items);
    let pack = logic::stacks(items, &data.items);
    ContainerBody {
        stash_cells: logic::grid_cells(stash.len(), BOX_COLUMNS, BOX_ROWS),
        pack_cells: logic::grid_cells(pack.len(), BOX_COLUMNS, BOX_ROWS),
        stash,
        pack,
    }
}

impl Content {
    fn pack(&self) -> &[Stack] {
        match &self.body {
            Body::Inventory(b) => &b.pack,
            Body::Container(b) => &b.pack,
            Body::Skills(_) => &[],
        }
    }

    fn stash(&self) -> &[Stack] {
        match &self.body {
            Body::Container(b) => &b.stash,
            _ => &[],
        }
    }

    /// L'objet d'une case et sa quantité (`None` : case vide ou absente).
    pub(super) fn item_at(&self, at: CellRef) -> Option<(&str, u32)> {
        match at {
            CellRef::Pack(i) => self.pack().get(i).map(|s| (s.item.as_str(), s.count)),
            CellRef::Stash(i) => self.stash().get(i).map(|s| (s.item.as_str(), s.count)),
            CellRef::Slot(slot) => match &self.body {
                Body::Inventory(b) => b
                    .slots
                    .iter()
                    .find(|entry| entry.slot == slot)
                    .and_then(|entry| entry.item.as_deref())
                    .map(|item| (item, 1)),
                _ => None,
            },
        }
    }

    /// La case existe-t-elle (pleine ou vide) ?
    pub(super) fn has_cell(&self, at: CellRef) -> bool {
        match (&self.body, at) {
            (Body::Inventory(b), CellRef::Pack(i)) => i < b.pack_cells,
            (Body::Inventory(_), CellRef::Slot(_)) => true,
            (Body::Container(b), CellRef::Pack(i)) => i < b.pack_cells,
            (Body::Container(b), CellRef::Stash(i)) => i < b.stash_cells,
            _ => false,
        }
    }

    /// L'écran a-t-il des cases (et un panneau de détail) ?
    pub(super) fn has_cells(&self) -> bool {
        !matches!(self.body, Body::Skills(_))
    }

    /// La première case de l'inventaire qui contient cet objet.
    pub(super) fn pack_cell_of(&self, item: &str) -> Option<CellRef> {
        self.pack()
            .iter()
            .position(|s| s.item == item)
            .map(CellRef::Pack)
    }

    /// La première case pleine : objets (du casier d'abord), sinon équipement porté.
    pub(super) fn first_filled(&self) -> Option<CellRef> {
        match &self.body {
            Body::Inventory(b) => (!b.pack.is_empty())
                .then_some(CellRef::Pack(0))
                .or_else(|| {
                    b.slots
                        .iter()
                        .find(|entry| entry.item.is_some())
                        .map(|entry| CellRef::Slot(entry.slot))
                }),
            Body::Container(b) => (!b.stash.is_empty())
                .then_some(CellRef::Stash(0))
                .or_else(|| (!b.pack.is_empty()).then_some(CellRef::Pack(0))),
            Body::Skills(_) => None,
        }
    }

    /// Un pas au clavier depuis la case choisie (`dy` positif vers le bas). On passe de la
    /// grille de l'inventaire à l'équipement par la gauche, du casier à l'inventaire par la
    /// droite.
    pub(super) fn step(&self, from: Option<CellRef>, dx: i32, dy: i32) -> Option<CellRef> {
        let first = match &self.body {
            Body::Inventory(_) => CellRef::Pack(0),
            Body::Container(_) => CellRef::Stash(0),
            Body::Skills(_) => return None,
        };
        let Some(from) = from else {
            return Some(first);
        };
        let slot_at = |index: usize| {
            EquipSlot::ALL
                .get(index)
                .copied()
                .unwrap_or(EquipSlot::Tool)
        };
        match (&self.body, from) {
            (Body::Inventory(b), CellRef::Pack(i)) => {
                Some(match logic::step(i, b.pack_cells, PACK_COLUMNS, dx, dy) {
                    Step::To(j) => CellRef::Pack(j),
                    Step::OutLeft(row) => CellRef::Slot(slot_at(row)),
                    Step::OutRight(_) => from,
                })
            }
            (Body::Inventory(b), CellRef::Slot(slot)) => {
                let index = EquipSlot::ALL.iter().position(|s| *s == slot).unwrap_or(0);
                Some(if dx > 0 {
                    CellRef::Pack((index * PACK_COLUMNS).min(b.pack_cells.saturating_sub(1)))
                } else if dy != 0 {
                    CellRef::Slot(slot_at(index.saturating_add_signed(dy.signum() as isize)))
                } else {
                    from
                })
            }
            (Body::Container(b), CellRef::Stash(i)) => {
                Some(match logic::step(i, b.stash_cells, BOX_COLUMNS, dx, dy) {
                    Step::To(j) => CellRef::Stash(j),
                    Step::OutRight(row) => {
                        CellRef::Pack((row * BOX_COLUMNS).min(b.pack_cells.saturating_sub(1)))
                    }
                    Step::OutLeft(_) => from,
                })
            }
            (Body::Container(b), CellRef::Pack(i)) => {
                Some(match logic::step(i, b.pack_cells, BOX_COLUMNS, dx, dy) {
                    Step::To(j) => CellRef::Pack(j),
                    Step::OutLeft(row) => CellRef::Stash(
                        (row * BOX_COLUMNS + BOX_COLUMNS - 1).min(b.stash_cells.saturating_sub(1)),
                    ),
                    Step::OutRight(_) => from,
                })
            }
            _ => Some(first),
        }
    }
}

// --- Entités -----------------------------------------------------------------------------------

/// Ce qu'il faut pour dessiner.
pub(super) struct Kit<'a> {
    pub fonts: &'a UiFonts,
    pub icons: &'a Icons<'a>,
    pub t: &'a Texts,
    pub data: &'a Data,
}

/// Fond et bord d'une case selon son état.
pub(super) fn cell_colors(filled: bool, hovered: bool, selected: bool) -> (Color, Color) {
    let fill = if hovered {
        CELL_FILL_HOVER
    } else if selected {
        CELL_FILL_SELECTED
    } else {
        CELL_FILL
    };
    let edge = if selected {
        theme::ACCENT
    } else if hovered {
        theme::SCREEN
    } else if filled {
        CELL_EDGE
    } else {
        CELL_EDGE_EMPTY
    };
    (fill, edge)
}

pub(super) fn spawn_screen(commands: &mut Commands, kit: &Kit, content: &Content) {
    let root = commands
        .spawn((
            Name::new("Écran du personnage"),
            CharacterUi,
            DespawnOnExit(AppState::InGame),
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            // Plus sombre que le voile du terminal : l'écran est grand et dense.
            BackgroundColor(theme::OVERLAY.with_alpha(0.55)),
        ))
        .id();
    let width = if content.tabs.is_some() {
        PANEL_WIDE
    } else {
        PANEL_NARROW
    };
    let panel = commands
        .spawn((
            ChildOf(root),
            Node {
                width: px(width),
                max_width: percent(96),
                max_height: percent(94),
                flex_direction: FlexDirection::Column,
                // Retrait sur les quatre côtés : filets, coupe ambre et coupe du bas dégagés.
                padding: UiRect::new(
                    px(Frame::Panel.inner() + 8.0),
                    px(Frame::Panel.inner() + 4.0),
                    px(Frame::Panel.inner()),
                    px(Frame::Panel.inner()),
                ),
                row_gap: px(8),
                overflow: Overflow::clip(),
                ..default()
            },
            kit.fonts.frame(Frame::Panel),
        ))
        .id();
    spawn_header(commands, panel, kit, content);
    // Le corps est la seule partie qui rétrécit (et se coupe) si l'écran manque de hauteur.
    let body = commands
        .spawn((
            ChildOf(panel),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                flex_shrink: 1.0,
                min_height: px(0),
                overflow: Overflow::clip_y(),
                ..default()
            },
        ))
        .id();
    match &content.body {
        Body::Inventory(b) => spawn_inventory(commands, body, kit, b),
        Body::Skills(b) => spawn_skills(commands, body, kit, b),
        Body::Container(b) => spawn_container(commands, body, kit, b),
    }
    spawn_footer(commands, panel, kit, content);
}

fn spawn_header(commands: &mut Commands, panel: Entity, kit: &Kit, content: &Content) {
    let header = commands
        .spawn((
            ChildOf(panel),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    commands.spawn((
        ChildOf(header),
        Text::new(content.title.clone()),
        kit.fonts.title(24.0),
        TextColor(theme::ACCENT),
        Node {
            margin: UiRect::right(px(14)),
            ..default()
        },
    ));
    if let Some(active) = content.tabs {
        for tab in CharacterTab::ALL {
            spawn_tab(commands, header, kit, tab, tab == active);
        }
    }
    commands.spawn((
        ChildOf(header),
        Node {
            flex_grow: 1.0,
            ..default()
        },
    ));
    commands.spawn((ChildOf(header), theme::status_lights()));
    commands.spawn((
        ChildOf(panel),
        theme::rule(percent(100), UiRect::new(px(0), px(0), px(2), px(6))),
    ));
}

/// Un onglet : l'onglet ouvert garde le cadre allumé.
fn spawn_tab(commands: &mut Commands, parent: Entity, kit: &Kit, tab: CharacterTab, active: bool) {
    let frame = kit.fonts.frame(if active {
        Frame::ButtonHover
    } else {
        Frame::Button
    });
    let color = if active { theme::TEXT } else { theme::TEXT_DIM };
    let mut button = commands.spawn((
        ChildOf(parent),
        Button,
        ActivateOnPress,
        ScreenButton::Tab(tab),
        Node {
            padding: UiRect::new(px(18), px(14), px(6), px(6)),
            flex_shrink: 0.0,
            ..default()
        },
        frame,
        children![(
            Text::new(kit.t.get(&format!("character.tab.{}", tab.key()))),
            kit.fonts.body(16.0),
            TextColor(color),
            Pickable::IGNORE,
        )],
    ));
    if active {
        button.insert(ActiveTab);
    }
}

fn spawn_footer(commands: &mut Commands, panel: Entity, kit: &Kit, content: &Content) {
    commands.spawn((
        ChildOf(panel),
        Node {
            width: percent(100),
            height: px(1),
            margin: UiRect::top(px(4)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(theme::RULE),
    ));
    let row = commands
        .spawn((
            ChildOf(panel),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(16),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    commands.spawn((
        ChildOf(row),
        Text::new(content.hint.clone()),
        kit.fonts.body(14.0),
        TextColor(theme::TEXT_DIM),
        // Base nulle : la consigne prend la place qui reste et passe à la ligne au besoin.
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            ..default()
        },
    ));
    spawn_button(
        commands,
        row,
        kit,
        kit.t.get("character.action.close"),
        ScreenButton::Close,
    );
}

/// Un bouton à languette.
fn spawn_button(
    commands: &mut Commands,
    parent: Entity,
    kit: &Kit,
    label: String,
    action: ScreenButton,
) {
    commands.spawn((
        ChildOf(parent),
        Button,
        ActivateOnPress,
        action,
        Node {
            padding: UiRect::new(px(16), px(12), px(5), px(5)),
            align_self: AlignSelf::FlexStart,
            flex_shrink: 0.0,
            ..default()
        },
        kit.fonts.frame(Frame::Button),
        children![(
            Text::new(label),
            kit.fonts.body(15.0),
            TextColor(theme::TEXT),
            Pickable::IGNORE,
        )],
    ));
}

/// Un titre de section : capitales espacées sur un filet.
fn spawn_section(commands: &mut Commands, parent: Entity, kit: &Kit, title: &str) {
    commands.spawn((
        ChildOf(parent),
        theme::heading(kit.fonts, title, 13.0, theme::TEXT_DIM),
    ));
    commands.spawn((
        ChildOf(parent),
        theme::rule(percent(100), UiRect::new(px(0), px(0), px(1), px(4))),
    ));
}

fn spawn_column(commands: &mut Commands, parent: Entity, width: Val) -> Entity {
    commands
        .spawn((
            ChildOf(parent),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                width,
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id()
}

fn spawn_text(
    commands: &mut Commands,
    parent: Entity,
    text: String,
    font: TextFont,
    color: Color,
) -> Entity {
    commands
        .spawn((ChildOf(parent), Text::new(text), font, TextColor(color)))
        .id()
}

/// Le nom d'un objet, avec sa majuscule.
fn item_name(kit: &Kit, item: &str) -> String {
    let name = match kit.data.items.get(item) {
        Some(def) => kit.t.get(&def.name_key),
        None => kit.t.get(&format!("item.{item}")),
    };
    logic::capitalize(&name)
}

/// L'icône d'un objet ; à défaut (fichier illisible), les premières lettres de son nom.
fn spawn_icon(commands: &mut Commands, parent: Entity, kit: &Kit, item: &str, size: f32) {
    match kit.icons.get(item) {
        Some(image) => commands.spawn((
            ChildOf(parent),
            ImageNode::new(image),
            Node {
                width: px(size),
                height: px(size),
                ..default()
            },
            Pickable::IGNORE,
        )),
        None => {
            let letters: String = item_name(kit, item)
                .chars()
                .filter(|c| c.is_alphanumeric())
                .take(3)
                .collect();
            commands.spawn((
                ChildOf(parent),
                Text::new(letters.to_uppercase()),
                kit.fonts.mono((size / 4.0).max(12.0)),
                TextColor(theme::TEXT_DIM),
                Pickable::IGNORE,
            ))
        }
    };
}

/// Une case : icône, quantité en coin (au-delà d'un exemplaire).
fn spawn_cell(
    commands: &mut Commands,
    parent: Entity,
    kit: &Kit,
    at: CellRef,
    stack: Option<(&str, u32)>,
) {
    let filled = stack.is_some();
    let (fill, edge) = cell_colors(filled, false, false);
    let cell = commands
        .spawn((
            ChildOf(parent),
            GridCell { at, filled },
            Node {
                width: px(CELL),
                height: px(CELL),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
        ))
        .id();
    let Some((item, count)) = stack else {
        return;
    };
    spawn_icon(commands, cell, kit, item, ICON);
    if count > 1 {
        commands.spawn((
            ChildOf(cell),
            Text::new(kit.t.number(f64::from(count), 0)),
            kit.fonts.mono(12.0),
            TextColor(theme::TEXT),
            theme::label_shadow(),
            Node {
                position_type: PositionType::Absolute,
                right: px(4),
                bottom: px(1),
                ..default()
            },
            Pickable::IGNORE,
        ));
    }
}

fn spawn_grid(
    commands: &mut Commands,
    parent: Entity,
    kit: &Kit,
    columns: usize,
    cells: usize,
    stacks: &[Stack],
    at: fn(usize) -> CellRef,
) {
    let grid = commands
        .spawn((
            ChildOf(parent),
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::px(columns as u16, CELL),
                column_gap: px(GAP),
                row_gap: px(GAP),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    for i in 0..cells {
        let stack = stacks.get(i).map(|s| (s.item.as_str(), s.count));
        spawn_cell(commands, grid, kit, at(i), stack);
    }
}

fn spawn_inventory(commands: &mut Commands, body: Entity, kit: &Kit, b: &InventoryBody) {
    let t = kit.t;
    let main = commands
        .spawn((
            ChildOf(body),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                column_gap: px(20),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();

    // Équipement porté et compte.
    let left = spawn_column(commands, main, px(LEFT_WIDTH));
    spawn_section(commands, left, kit, &t.get("character.section.equipment"));
    for entry in &b.slots {
        let row = commands
            .spawn((
                ChildOf(left),
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(10),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .id();
        let item = entry.item.as_deref();
        spawn_cell(
            commands,
            row,
            kit,
            CellRef::Slot(entry.slot),
            item.map(|id| (id, 1)),
        );
        let labels = commands
            .spawn((
                ChildOf(row),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    flex_shrink: 1.0,
                    min_width: px(0),
                    ..default()
                },
            ))
            .id();
        let slot = t.get(&format!("item.slot.{}", entry.slot.key()));
        commands.spawn((
            ChildOf(labels),
            theme::heading(kit.fonts, &slot, 11.0, theme::TEXT_DIM),
        ));
        match item {
            Some(id) => spawn_text(
                commands,
                labels,
                item_name(kit, id),
                kit.fonts.body(15.0),
                theme::TEXT,
            ),
            None => spawn_text(
                commands,
                labels,
                t.get("character.slot.empty"),
                kit.fonts.body(14.0),
                theme::TEXT_DISABLED,
            ),
        };
    }
    commands.spawn((
        ChildOf(left),
        Node {
            height: px(6),
            ..default()
        },
    ));
    spawn_section(commands, left, kit, &t.get("character.section.money"));
    spawn_text(
        commands,
        left,
        b.credits.clone(),
        kit.fonts.mono(18.0),
        theme::TEXT,
    );
    spawn_text(
        commands,
        left,
        b.wuro.clone(),
        kit.fonts.mono(15.0),
        theme::TEXT_DIM,
    );

    // Les objets portés.
    let middle = spawn_column(commands, main, Val::Auto);
    spawn_section(commands, middle, kit, &t.get("character.section.items"));
    spawn_grid(
        commands,
        middle,
        kit,
        PACK_COLUMNS,
        b.pack_cells,
        &b.pack,
        CellRef::Pack,
    );

    // Le détail de l'objet survolé ou choisi (rempli par `render_focus`).
    let right = spawn_column(commands, main, px(DETAIL_WIDTH));
    spawn_section(commands, right, kit, &t.get("character.section.detail"));
    commands.spawn((
        ChildOf(right),
        DetailPane,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            width: percent(100),
            ..default()
        },
    ));

    // Réputations et emprunts.
    let bottom = commands
        .spawn((
            ChildOf(body),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                column_gap: px(24),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    let reps = commands
        .spawn((
            ChildOf(bottom),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                flex_grow: 1.6,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
        ))
        .id();
    spawn_section(commands, reps, kit, &t.get("character.section.reputation"));
    if b.reputations.is_empty() {
        spawn_text(
            commands,
            reps,
            t.get("character.rep.none"),
            kit.fonts.body(14.0),
            theme::TEXT_DIM,
        );
    }
    for rep in &b.reputations {
        spawn_reputation(commands, reps, kit, rep);
    }
    if b.more_reputations > 0 {
        spawn_text(
            commands,
            reps,
            t.format(
                "character.rep.more",
                &[("count", &b.more_reputations.to_string())],
            ),
            kit.fonts.body(13.0),
            theme::TEXT_DIM,
        );
    }
    let loans = commands
        .spawn((
            ChildOf(bottom),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
        ))
        .id();
    spawn_section(commands, loans, kit, &t.get("character.section.loans"));
    if b.loans.is_empty() {
        spawn_text(
            commands,
            loans,
            t.get("character.loan.none"),
            kit.fonts.body(14.0),
            theme::TEXT_DIM,
        );
    }
    for (index, loan) in b.loans.iter().enumerate() {
        spawn_loan(commands, loans, kit, loan, index > 0);
    }
}

/// Une organisation : nom (et grade), palier, valeur, jauge de −100 à +100.
fn spawn_reputation(commands: &mut Commands, parent: Entity, kit: &Kit, rep: &RepEntry) {
    let row = commands
        .spawn((
            ChildOf(parent),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                min_height: px(24),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    let names = commands
        .spawn((
            ChildOf(row),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
        ))
        .id();
    spawn_text(
        commands,
        names,
        rep.name.clone(),
        kit.fonts.body(15.0),
        theme::TEXT,
    );
    if let Some(note) = &rep.note {
        spawn_text(
            commands,
            names,
            note.clone(),
            kit.fonts.body(12.0),
            theme::TEXT_DIM,
        );
    }
    let tone = if rep.value >= 20 {
        theme::SCREEN
    } else if rep.value <= -20 {
        theme::DANGER
    } else {
        theme::TEXT
    };
    commands.spawn((
        ChildOf(row),
        Text::new(rep.tier.clone()),
        kit.fonts.body(14.0),
        TextColor(tone),
        Node {
            width: px(104),
            flex_shrink: 0.0,
            ..default()
        },
    ));
    commands.spawn((
        ChildOf(row),
        Text::new(format!("{:+}", rep.value)),
        kit.fonts.mono(14.0),
        TextColor(theme::TEXT),
        Node {
            width: px(40),
            flex_shrink: 0.0,
            ..default()
        },
    ));
    let gauge = commands
        .spawn((
            ChildOf(row),
            Node {
                width: px(GAUGE_WIDTH),
                height: px(6),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(BAR_BACK),
        ))
        .id();
    // La jauge part du milieu : à droite l'estime, à gauche l'hostilité.
    let half = rep.value.clamp(-100, 100) as f32 / 2.0;
    if rep.value != 0 {
        commands.spawn((
            ChildOf(gauge),
            Node {
                position_type: PositionType::Absolute,
                left: percent(50.0 + half.min(0.0)),
                width: percent(half.abs()),
                height: percent(100),
                ..default()
            },
            BackgroundColor(if rep.value > 0 {
                theme::SCREEN
            } else {
                theme::DANGER
            }),
        ));
    }
    commands.spawn((
        ChildOf(gauge),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            top: px(-2),
            width: px(1),
            height: px(10),
            ..default()
        },
        BackgroundColor(theme::STEEL),
    ));
}

/// Un emprunt : créancier (sur toute la largeur, il passe à la ligne s'il est long), puis des
/// lignes libellé : valeur. La valeur prend toute la largeur qui reste et ne se coupe jamais
/// (montants, date) ; le délai qui reste va dessous, en gris. Les échéances manquées en rouge.
/// `separated` : un emprunt qui en suit un autre s'en détache par un filet.
fn spawn_loan(
    commands: &mut Commands,
    parent: Entity,
    kit: &Kit,
    loan: &LoanEntry,
    separated: bool,
) {
    let t = kit.t;
    let block = commands
        .spawn((
            ChildOf(parent),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                width: percent(100),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    if separated {
        commands.spawn((
            ChildOf(block),
            Node {
                width: percent(100),
                height: px(1),
                margin: UiRect::vertical(px(4)),
                ..default()
            },
            BackgroundColor(theme::RULE),
        ));
    }
    spawn_text(
        commands,
        block,
        loan.lender.clone(),
        kit.fonts.body(15.0),
        theme::TEXT,
    );
    let pairs = [
        (
            t.get("character.loan.balance"),
            loan.balance.clone(),
            None,
            kit.fonts.mono(15.0),
            theme::ACCENT,
        ),
        (
            t.get("character.loan.installment"),
            loan.installment.clone(),
            None,
            kit.fonts.mono(15.0),
            theme::TEXT,
        ),
        (
            t.get("character.loan.due"),
            loan.due.clone(),
            loan.due_note.clone(),
            kit.fonts.body(14.0),
            theme::TEXT,
        ),
    ];
    for (label, value, note, font, color) in pairs {
        let row = commands
            .spawn((
                ChildOf(block),
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    column_gap: px(10),
                    width: percent(100),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .id();
        commands.spawn((
            ChildOf(row),
            Text::new(label),
            kit.fonts.body(14.0),
            TextColor(theme::TEXT_DIM),
            Node {
                width: px(130),
                flex_shrink: 0.0,
                ..default()
            },
        ));
        // Base nulle et croissance : la colonne des valeurs prend toute la largeur restante
        // (sans elle, le texte se réduisait à sa largeur minimale, un mot par ligne).
        let values = commands
            .spawn((
                ChildOf(row),
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    ..default()
                },
            ))
            .id();
        commands.spawn((
            ChildOf(values),
            Text::new(value),
            font,
            TextColor(color),
            TextLayout::no_wrap(),
        ));
        if let Some(note) = note {
            spawn_text(
                commands,
                values,
                note,
                kit.fonts.body(13.0),
                theme::TEXT_DIM,
            );
        }
    }
    if loan.missed > 0 {
        spawn_text(
            commands,
            block,
            t.format(
                "character.loan.missed",
                &[("count", &loan.missed.to_string())],
            ),
            kit.fonts.body(14.0),
            theme::DANGER,
        );
    }
}

fn spawn_skills(commands: &mut Commands, body: Entity, kit: &Kit, b: &SkillsBody) {
    let t = kit.t;
    spawn_text(
        commands,
        body,
        t.get("character.skills.intro"),
        kit.fonts.body(14.0),
        theme::TEXT_DIM,
    );
    let grid = commands
        .spawn((
            ChildOf(body),
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                column_gap: px(16),
                row_gap: px(8),
                width: percent(100),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    for skill in &b.skills {
        spawn_skill(commands, grid, kit, skill);
    }
    let traits = spawn_column(commands, body, percent(100));
    spawn_section(commands, traits, kit, &t.get("character.section.traits"));
    if b.traits.is_empty() {
        spawn_text(
            commands,
            traits,
            t.get("character.traits.none"),
            kit.fonts.body(15.0),
            theme::TEXT_DIM,
        );
    } else {
        spawn_text(
            commands,
            traits,
            b.traits.join(" · "),
            kit.fonts.body(15.0),
            theme::TEXT,
        );
    }
}

/// Une compétence : nom et niveau, barre d'expérience graduée en quarts, points et suite.
fn spawn_skill(commands: &mut Commands, grid: Entity, kit: &Kit, skill: &SkillEntry) {
    let t = kit.t;
    let card = commands
        .spawn((
            ChildOf(grid),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                padding: UiRect::axes(px(12), px(8)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(CARD_FILL),
            BorderColor::all(CELL_EDGE),
        ))
        .id();
    let row = |commands: &mut Commands| {
        commands
            .spawn((
                ChildOf(card),
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(10),
                    ..default()
                },
            ))
            .id()
    };
    let top = row(commands);
    commands.spawn((
        ChildOf(top),
        Text::new(skill.name.clone()),
        kit.fonts.body(16.0),
        TextColor(theme::TEXT),
        Node {
            flex_grow: 1.0,
            ..default()
        },
    ));
    spawn_text(
        commands,
        top,
        t.format(
            "character.skills.level",
            &[("level", &skill.level.to_string())],
        ),
        kit.fonts.mono(15.0),
        theme::ACCENT,
    );
    let bar = commands
        .spawn((
            ChildOf(card),
            Node {
                width: percent(100),
                height: px(8),
                ..default()
            },
            BackgroundColor(BAR_BACK),
        ))
        .id();
    commands.spawn((
        ChildOf(bar),
        Node {
            width: percent(f32::from(skill.permille) / 10.0),
            height: percent(100),
            ..default()
        },
        BackgroundColor(theme::SCREEN),
    ));
    for quarter in [25, 50, 75] {
        commands.spawn((
            ChildOf(bar),
            Node {
                position_type: PositionType::Absolute,
                left: percent(quarter),
                width: px(1),
                height: percent(100),
                ..default()
            },
            BackgroundColor(BAR_TICK),
        ));
    }
    let bottom = row(commands);
    commands.spawn((
        ChildOf(bottom),
        Text::new(t.format(
            "character.skills.points",
            &[
                ("xp", &t.number(skill.xp as f64, 0)),
                ("next", &t.number(skill.next as f64, 0)),
            ],
        )),
        kit.fonts.mono(13.0),
        TextColor(theme::TEXT_DIM),
        Node {
            flex_grow: 1.0,
            ..default()
        },
    ));
    spawn_text(
        commands,
        bottom,
        t.format(
            "character.skills.next",
            &[
                ("level", &(skill.level + 1).to_string()),
                ("left", &t.number(skill.left as f64, 0)),
            ],
        ),
        kit.fonts.body(13.0),
        theme::TEXT_DIM,
    );
}

fn spawn_container(commands: &mut Commands, body: Entity, kit: &Kit, b: &ContainerBody) {
    let t = kit.t;
    let main = commands
        .spawn((
            ChildOf(body),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                column_gap: px(28),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    let stash = spawn_column(commands, main, Val::Auto);
    spawn_section(commands, stash, kit, &t.get("character.container.stash"));
    spawn_grid(
        commands,
        stash,
        kit,
        BOX_COLUMNS,
        b.stash_cells,
        &b.stash,
        CellRef::Stash,
    );
    commands.spawn((
        ChildOf(main),
        Node {
            width: px(1),
            align_self: AlignSelf::Stretch,
            ..default()
        },
        BackgroundColor(theme::RULE),
    ));
    let pack = spawn_column(commands, main, Val::Auto);
    spawn_section(commands, pack, kit, &t.get("character.container.pack"));
    spawn_grid(
        commands,
        pack,
        kit,
        BOX_COLUMNS,
        b.pack_cells,
        &b.pack,
        CellRef::Pack,
    );
    let detail = spawn_column(commands, body, percent(100));
    spawn_section(commands, detail, kit, &t.get("character.section.detail"));
    commands.spawn((
        ChildOf(detail),
        DetailPane,
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FlexStart,
            column_gap: px(14),
            min_height: px(CELL + 8.0),
            ..default()
        },
    ));
}

/// Le panneau de détail : l'objet montré (survolé, sinon choisi), ou une invite.
pub(super) fn spawn_detail(
    commands: &mut Commands,
    pane: Entity,
    kit: &Kit,
    content: &Content,
    shown: Option<CellRef>,
) {
    let t = kit.t;
    let Some((at, item, count)) =
        shown.and_then(|at| content.item_at(at).map(|(item, count)| (at, item, count)))
    else {
        spawn_text(
            commands,
            pane,
            t.get("character.detail.none"),
            kit.fonts.body(14.0),
            theme::TEXT_DIM,
        );
        return;
    };
    let def = kit.data.items.get(item);
    let mut facts = Vec::new();
    if let Some(def) = def {
        facts.push(t.get(&format!("item.category.{}", def.category.key())));
        if let Some(slot) = def.slot {
            let slot = t.get(&format!("item.slot.{}", slot.key()));
            facts.push(t.format("character.detail.slot", &[("slot", &slot)]));
        }
        if def.max_stack > 1 {
            facts.push(t.format(
                "character.detail.stack",
                &[
                    ("count", &t.number(f64::from(count), 0)),
                    ("max", &t.number(f64::from(def.max_stack), 0)),
                ],
            ));
        }
    }
    let description = def.map(|def| t.get(&def.description_key));
    let vertical = matches!(content.body, Body::Inventory(_));
    let icon_box = if vertical { 96.0 } else { CELL };
    let head = commands
        .spawn((
            ChildOf(pane),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: if vertical {
                    AlignItems::Center
                } else {
                    AlignItems::FlexStart
                },
                column_gap: px(12),
                flex_grow: if vertical { 0.0 } else { 1.0 },
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    let frame = commands
        .spawn((
            ChildOf(head),
            Node {
                width: px(icon_box),
                height: px(icon_box),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(CELL_FILL),
            BorderColor::all(CELL_EDGE),
        ))
        .id();
    spawn_icon(commands, frame, kit, item, icon_box - 12.0);
    let words = commands
        .spawn((
            ChildOf(head),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
        ))
        .id();
    spawn_text(
        commands,
        words,
        item_name(kit, item),
        kit.fonts.title(if vertical { 18.0 } else { 17.0 }),
        theme::TEXT,
    );
    let facts = if vertical {
        facts
    } else {
        // En bandeau, les relevés tiennent sur une ligne.
        vec![facts.join(" · ")]
    };
    for fact in facts.into_iter().filter(|f| !f.is_empty()) {
        spawn_text(commands, words, fact, kit.fonts.body(13.0), theme::TEXT_DIM);
    }
    if matches!(at, CellRef::Slot(_)) {
        spawn_text(
            commands,
            words,
            t.get("character.detail.equipped"),
            kit.fonts.body(13.0),
            theme::ACCENT,
        );
    }
    // La description : sous l'en-tête (colonne), ou à côté du nom (bandeau).
    let text_parent = if vertical { pane } else { words };
    if let Some(description) = description {
        spawn_text(
            commands,
            text_parent,
            description,
            kit.fonts.body(14.0),
            theme::TEXT,
        );
    }
    if !vertical {
        return;
    }
    let action = match at {
        CellRef::Slot(slot) => Some((
            t.get("character.action.unequip"),
            ScreenButton::Unequip(slot),
        )),
        CellRef::Pack(_) if def.is_some_and(|def| def.slot.is_some()) => Some((
            t.get("character.action.equip"),
            ScreenButton::Equip(item.to_owned()),
        )),
        _ => None,
    };
    if let Some((label, button)) = action {
        spawn_button(commands, pane, kit, label, button);
    }
}
