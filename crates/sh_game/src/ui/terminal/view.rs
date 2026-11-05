//! L'affichage du terminal : une borne d'appareil « futur usé » (M9).
//!
//! Panneau à pans coupés (`theme::Frame::Panel`) ; en-tête avec le pictogramme de la borne
//! (dessiné en nœuds), son nom en capitales espacées, l'organisation qui la tient (cyan), les
//! voyants et la date ; bandeau de relevés à chasse fixe ; onglets ; corps en puits sombres
//! (liste à lignes alternées et détail aligné, ou panneaux, ou conversation) ; barre d'actions à
//! touches gravées, résultat de la dernière action coloré, consigne. Les couleurs disent l'état :
//! ambre pour ce qui demande l'attention, cyan pour les relevés sains, rouge d'alerte.
//!
//! L'écran n'est reconstruit que quand la page change (action, jour, fiche du capitaine) ou que
//! la sélection bouge : jamais à chaque frame. Le survol ne change que des couleurs.

use bevy::ecs::system::SystemParam;
use bevy::picking::events::{PointerOut, PointerOver};
use bevy::prelude::*;
use bevy::text::{Justify, LineBreak};
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};

use super::page::{
    Action, Body, Choice, Column, Ctx, Detail, Field, Gauge, Glyph, Page, Row, Slot, Tone,
    VISIBLE_ROWS, fraction_tone, page, scroll_for,
};
use super::{Screen, Terminal, TerminalChoice};
use crate::career::{Career, MissionLog};
use crate::clock::GameClock;
use crate::codex::Codex;
use crate::data::{Data, Texts};
use crate::interaction::Inventory;
use crate::living::LivingWorld;
use crate::state::AppState;
use crate::ui::theme::{self, Frame, UiFonts};
use crate::voyage::{Location, ShipFuel};

/// Fond des puits (liste, détail, relevés) : plus sombre que le panneau.
const WELL: Color = Color::srgba(0.012, 0.015, 0.020, 0.62);
const ROW_EVEN: Color = Color::srgba(1.0, 1.0, 1.0, 0.022);
const ROW_ODD: Color = Color::srgba(1.0, 1.0, 1.0, 0.055);
const ROW_HOVER: Color = Color::srgba(0.95, 0.66, 0.24, 0.10);
const ROW_SELECTED: Color = Color::srgba(0.95, 0.66, 0.24, 0.20);
/// Bande d'une ligne de détail sur deux.
const FIELD_BAND: Color = Color::srgba(1.0, 1.0, 1.0, 0.03);
const KEYCAP: Color = Color::srgba(1.0, 1.0, 1.0, 0.06);
const TAB_ACTIVE: Color = Color::srgba(0.95, 0.66, 0.24, 0.14);
const GLYPH_BACK: Color = Color::srgba(0.45, 0.82, 0.90, 0.06);
const GAUGE_BACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

/// Largeur du panneau : une conversation se lit mieux plus étroite qu'une liste à colonnes.
const WIDTH_LIST: f32 = 1180.0;
const WIDTH_DIALOGUE: f32 = 900.0;
const ROW_HEIGHT: f32 = 28.0;
const GAUGE_LABEL: f32 = 86.0;
const GAUGE_VALUE: f32 = 56.0;

#[derive(Component)]
pub(super) struct TerminalUi;

/// Un bouton d'action : l'indice de son action dans la page.
#[derive(Component, Clone, Copy)]
pub(super) struct ChoiceButton(usize);

/// Une ligne de liste : la cliquer la choisit.
#[derive(Component, Clone, Copy)]
pub(super) struct RowButton(usize);

/// Un onglet : le cliquer ouvre son écran.
#[derive(Component, Clone)]
pub(super) struct TabButton(Screen);

/// Une action grisée : elle ne s'allume pas au survol.
#[derive(Component)]
pub(super) struct DisabledChoice;

/// L'action en évidence (clavier : Entrée la fait) : elle reste allumée hors survol.
#[derive(Component)]
pub(super) struct Focused;

/// Couleurs de fond d'un élément sans cadre (ligne, onglet), au repos et survolé.
#[derive(Component, Clone, Copy)]
pub(super) struct HoverFill {
    base: Color,
    hover: Color,
}

fn tone_color(tone: Tone) -> Color {
    match tone {
        Tone::Normal => theme::TEXT,
        Tone::Dim => theme::TEXT_DIM,
        Tone::Accent => theme::ACCENT,
        Tone::Screen => theme::SCREEN,
        Tone::Danger => theme::DANGER,
        Tone::Disabled => theme::TEXT_DISABLED,
    }
}

/// Ce que lit une page, et si cela a changé depuis le dernier frame.
#[derive(SystemParam)]
pub(super) struct Sources<'w> {
    texts: Res<'w, Texts>,
    data: Res<'w, Data>,
    clock: Res<'w, GameClock>,
    location: Option<Res<'w, Location>>,
    living: Option<Res<'w, LivingWorld>>,
    log: Res<'w, MissionLog>,
    inventory: Res<'w, Inventory>,
    codex: Res<'w, Codex>,
    career: Res<'w, Career>,
    fuel: Option<Res<'w, ShipFuel>>,
}

impl Sources<'_> {
    fn changed(&self) -> bool {
        self.career.is_changed()
            || self.log.is_changed()
            || self.codex.is_changed()
            || self.inventory.is_changed()
            || self.texts.is_changed()
            || self.fuel.as_ref().is_some_and(|f| f.is_changed())
    }

    fn ctx(&self) -> Ctx<'_> {
        Ctx {
            texts: &self.texts,
            data: &self.data,
            cap: &self.career.captain,
            sim: self.living.as_ref().map(|w| &w.sim),
            here: self.location.as_ref().map(|l| &l.system),
            today: self.clock.date,
            log: &self.log,
            items: &self.inventory.items,
            codex: &self.codex,
            fuel: self.fuel.as_ref().map(|f| (f.units, f.capacity)),
            fuel_per_day: super::fuel::fuel_per_day(&self.data),
        }
    }
}

pub(super) fn render(
    mut commands: Commands,
    sources: Sources,
    fonts: Res<UiFonts>,
    terminal: Option<ResMut<Terminal>>,
    roots: Query<Entity, With<TerminalUi>>,
) {
    let Some(mut term) = terminal.filter(|t| !t.closing) else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    };
    let today = sources.clock.date;
    let mut redraw = roots.is_empty();
    if term.page.is_none() || sources.changed() || term.page_day != Some(today) {
        let fresh = page(&sources.ctx(), &term);
        term.page_day = Some(today);
        if term.page.as_ref() != Some(&fresh) {
            term.page = Some(fresh);
            redraw = true;
        }
    }
    let Some((preselect, count, rows)) = term
        .page
        .as_ref()
        .map(|p| (p.preselect(), p.cursor_len(), p.rows().len()))
    else {
        return;
    };
    // Un nouvel écran repart de sa ligne présélectionnée (ou de la première).
    if term.cursor_screen.as_ref() != Some(&term.screen) {
        term.cursor = preselect.unwrap_or(0);
        term.scroll = 0;
        term.cursor_screen = Some(term.screen.clone());
        redraw = true;
    }
    let cursor = term.cursor.min(count.saturating_sub(1));
    let scroll = scroll_for(cursor, term.scroll, rows);
    if term.cursor != cursor {
        term.cursor = cursor;
    }
    if term.scroll != scroll {
        term.scroll = scroll;
    }
    if !redraw && term.drawn == Some((cursor, scroll)) {
        return;
    }
    term.drawn = Some((cursor, scroll));
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some(page) = term.page.as_ref() else {
        return;
    };
    let numbered = page.numbered(cursor);
    let focus = if page.rows().is_empty() {
        numbered.get(cursor).copied()
    } else {
        page.primary(cursor)
    };
    let ui = Ui {
        fonts: &fonts,
        texts: &sources.texts,
        page,
        numbered: &numbered,
        focus,
        cursor,
        scroll,
    };
    ui.spawn(&mut commands);
}

/// Ce qu'il faut pour dessiner une page.
struct Ui<'a> {
    fonts: &'a UiFonts,
    texts: &'a Texts,
    page: &'a Page,
    numbered: &'a [usize],
    focus: Option<usize>,
    cursor: usize,
    scroll: usize,
}

impl Ui<'_> {
    fn spawn(&self, commands: &mut Commands) {
        let width = if matches!(self.page.body, Body::Dialogue { .. }) {
            WIDTH_DIALOGUE
        } else {
            WIDTH_LIST
        };
        let inner = Frame::Panel.inner();
        commands
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
                    padding: UiRect::top(percent(4)),
                    ..default()
                },
                // Voile : la borne et la pièce restent devinées autour du panneau.
                BackgroundColor(theme::OVERLAY.with_alpha(0.5)),
            ))
            .with_children(|root| {
                root.spawn((
                    Node {
                        width: px(width),
                        max_width: percent(96),
                        max_height: percent(78),
                        flex_direction: FlexDirection::Column,
                        // Retrait sur les quatre côtés : les filets, la coupe ambre (en haut à
                        // gauche) et la coupe du bas à droite restent dégagés.
                        padding: UiRect::new(
                            px(inner + 8.0),
                            px(inner + 4.0),
                            px(inner),
                            px(inner),
                        ),
                        row_gap: px(8),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    self.fonts.frame(Frame::Panel),
                ))
                .with_children(|panel| {
                    self.header(panel);
                    self.readouts(panel);
                    self.tabs(panel);
                    self.body(panel);
                    self.footer(panel);
                });
            });
    }

    // --- Petits éléments ----------------------------------------------------------------------

    /// Un libellé d'interface : capitales espacées, petite taille.
    fn label(&self, text: &str, size: f32, color: Color) -> impl Bundle {
        theme::heading(self.fonts, text, size, color)
    }

    fn value_font(&self, mono: bool, size: f32) -> TextFont {
        if mono {
            self.fonts.mono(size)
        } else {
            self.fonts.body(size)
        }
    }

    /// Une touche gravée (« 1 », « Échap »).
    fn keycap(&self, p: &mut ChildSpawnerCommands, key: &str, enabled: bool) {
        let (border, color) = if enabled {
            (theme::STEEL, theme::ACCENT)
        } else {
            (theme::RULE, theme::TEXT_DISABLED)
        };
        p.spawn((
            Node {
                min_width: px(24),
                padding: UiRect::new(px(6), px(6), px(1), px(2)),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(border),
            BackgroundColor(KEYCAP),
            children![(
                Text::new(key.to_owned()),
                self.fonts.mono(14.0),
                TextColor(color)
            )],
        ));
    }

    /// Le bouton d'une action, avec sa touche (numéro, ou Échap pour « Partir »).
    fn choice_button(&self, p: &mut ChildSpawnerCommands, index: usize) {
        let Some(c) = self.page.choices.get(index) else {
            return;
        };
        let key = if matches!(c.action, Action::Close) {
            Some(self.texts.key_label("Escape"))
        } else {
            self.numbered
                .iter()
                .position(|&i| i == index)
                .filter(|n| *n < 9)
                .map(|n| (n + 1).to_string())
        };
        let focused = self.focus == Some(index) && c.enabled;
        let mut frame = self.fonts.frame(if focused {
            Frame::ButtonHover
        } else {
            Frame::Button
        });
        if !c.enabled {
            // Une action impossible s'efface avec son libellé ; le détail dit pourquoi.
            frame.color = Color::srgba(1.0, 1.0, 1.0, 0.45);
        }
        let mut button = p.spawn((
            Button,
            ActivateOnPress,
            ChoiceButton(index),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                // À gauche : la languette du cadre.
                padding: UiRect::new(px(16), px(14), px(6), px(6)),
                flex_shrink: 0.0,
                ..default()
            },
            frame,
        ));
        button.with_children(|b| {
            if let Some(key) = &key {
                self.keycap(b, key, c.enabled);
            }
            b.spawn((
                Text::new(c.label.clone()),
                self.fonts.body(16.0),
                TextColor(if c.enabled {
                    theme::TEXT
                } else {
                    theme::TEXT_DISABLED
                }),
            ));
        });
        if !c.enabled {
            button.insert(DisabledChoice);
        }
        if focused {
            button.insert(Focused);
        }
    }

    /// Les boutons des actions retenues par `keep` (hors « Partir »), en colonne au pied d'un
    /// panneau.
    fn actions(&self, p: &mut ChildSpawnerCommands, keep: impl Fn(&Choice) -> bool) {
        let indices: Vec<usize> = self
            .page
            .choices
            .iter()
            .enumerate()
            .filter(|(_, c)| !matches!(c.action, Action::Close) && keep(c))
            .map(|(i, _)| i)
            .collect();
        if indices.is_empty() {
            return;
        }
        // L'espace libre pousse les actions au pied du panneau.
        p.spawn(Node {
            flex_grow: 1.0,
            min_height: px(4),
            ..default()
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|col| {
            for i in indices {
                self.choice_button(col, i);
            }
        });
    }

    /// Un puits : fond sombre, filet d'acier.
    fn well(grow: f32, padding: f32) -> impl Bundle {
        (
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: grow,
                flex_basis: px(0),
                min_width: px(0),
                padding: UiRect::all(px(padding)),
                border: UiRect::all(px(1)),
                row_gap: px(4),
                ..default()
            },
            BorderColor::all(theme::RULE),
            BackgroundColor(WELL),
        )
    }

    // --- En-tête, relevés, onglets ------------------------------------------------------------

    fn header(&self, p: &mut ChildSpawnerCommands) {
        let header = &self.page.header;
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(14),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|row| {
            glyph(row, header.glyph);
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: px(3),
                ..default()
            })
            .with_children(|col| {
                col.spawn(self.label(&header.title, 22.0, theme::ACCENT));
                if let Some(owner) = &header.owner {
                    col.spawn((
                        Text::new(owner.clone()),
                        self.fonts.body(15.0),
                        TextColor(theme::SCREEN),
                    ));
                }
            });
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: px(6),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|col| {
                col.spawn(theme::status_lights());
                col.spawn((
                    Text::new(header.date.clone()),
                    self.fonts.mono(14.0),
                    TextColor(theme::TEXT_DIM),
                ));
            });
        });
        p.spawn(theme::rule(
            percent(100),
            UiRect::new(px(0), px(0), px(2), px(2)),
        ));
    }

    fn readouts(&self, p: &mut ChildSpawnerCommands) {
        if self.page.readouts.is_empty() {
            return;
        }
        p.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                padding: UiRect::axes(px(2), px(6)),
                border: UiRect::all(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(theme::RULE),
            BackgroundColor(WELL),
        ))
        .with_children(|row| {
            for (i, field) in self.page.readouts.iter().enumerate() {
                let separator = if i == 0 { 0.0 } else { 1.0 };
                row.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(2),
                        padding: UiRect::axes(px(16), px(0)),
                        border: UiRect::left(px(separator)),
                        ..default()
                    },
                    BorderColor::all(theme::RULE),
                ))
                .with_children(|cell| {
                    cell.spawn(self.label(&field.label, 12.0, theme::TEXT_DIM));
                    cell.spawn((
                        Text::new(field.value.clone()),
                        self.value_font(field.mono, 16.0),
                        TextColor(tone_color(field.tone)),
                    ));
                });
            }
        });
    }

    fn tabs(&self, p: &mut ChildSpawnerCommands) {
        if self.page.tabs.is_empty() {
            return;
        }
        p.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                border: UiRect::bottom(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(theme::RULE),
        ))
        .with_children(|row| {
            for tab in &self.page.tabs {
                let (base, color, underline) = if tab.active {
                    (TAB_ACTIVE, theme::ACCENT, theme::ACCENT)
                } else {
                    (theme::BUTTON, theme::TEXT_DIM, Color::NONE)
                };
                let mut button = row.spawn((
                    Button,
                    ActivateOnPress,
                    TabButton(tab.screen.clone()),
                    Node {
                        padding: UiRect::axes(px(18), px(7)),
                        border: UiRect::bottom(px(2)),
                        ..default()
                    },
                    BorderColor::all(underline),
                    BackgroundColor(base),
                    children![self.label(&tab.label, 13.0, color)],
                ));
                if !tab.active {
                    button.insert(HoverFill {
                        base,
                        hover: theme::BUTTON_HOVER,
                    });
                }
            }
        });
    }

    // --- Corps --------------------------------------------------------------------------------

    fn body(&self, p: &mut ChildSpawnerCommands) {
        match &self.page.body {
            Body::Dialogue {
                speaker,
                line,
                extra,
            } => self.dialogue(p, speaker.as_deref(), line, extra.as_deref()),
            Body::List {
                columns,
                rows,
                empty,
                below,
                ..
            } => self.list(p, columns, rows, empty, below.as_ref()),
            Body::Panels(panels) => self.panels(p, panels),
        }
    }

    fn dialogue(
        &self,
        p: &mut ChildSpawnerCommands,
        speaker: Option<&str>,
        line: &str,
        extra: Option<&str>,
    ) {
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            flex_shrink: 1.0,
            min_height: px(0),
            overflow: Overflow::clip_y(),
            ..default()
        })
        .with_children(|col| {
            if let Some(speaker) = speaker {
                col.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Node {
                            width: px(3),
                            height: px(14),
                            ..default()
                        },
                        BackgroundColor(theme::ACCENT),
                    ));
                    row.spawn(self.label(speaker, 13.0, theme::ACCENT));
                });
            }
            // La réplique, sur une plaquette à barre ambre.
            col.spawn((
                Node {
                    padding: UiRect::new(px(18), px(14), px(10), px(12)),
                    flex_shrink: 0.0,
                    ..default()
                },
                self.fonts.frame(Frame::Plate),
                children![(
                    Text::new(line.to_owned()),
                    self.fonts.body(17.0),
                    TextColor(theme::TEXT)
                )],
            ));
            // Ce qu'il raconte (les nouvelles) : un relevé cyan.
            if let Some(extra) = extra {
                col.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        padding: UiRect::new(px(14), px(12), px(8), px(8)),
                        border: UiRect::left(px(2)),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BorderColor::all(theme::SCREEN),
                    BackgroundColor(WELL),
                ))
                .with_children(|news| {
                    for text in extra.lines() {
                        news.spawn((
                            Text::new(text.to_owned()),
                            self.fonts.body(15.0),
                            TextColor(theme::SCREEN),
                        ));
                    }
                });
            }
            col.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                margin: UiRect::top(px(4)),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|replies| {
                for (i, c) in self.page.choices.iter().enumerate() {
                    if matches!(c.slot, Slot::Panel(_)) && !matches!(c.action, Action::Close) {
                        self.choice_button(replies, i);
                    }
                }
            });
        });
    }

    fn list(
        &self,
        p: &mut ChildSpawnerCommands,
        columns: &[Column],
        rows: &[Row],
        empty: &str,
        below: Option<&Detail>,
    ) {
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(12),
            align_items: AlignItems::Stretch,
            flex_shrink: 1.0,
            min_height: px(0),
            overflow: Overflow::clip_y(),
            ..default()
        })
        .with_children(|row| {
            // La liste prend la plus grande part : ses colonnes portent des noms entiers
            // (« 240 pièces détachées »), le détail reste assez large pour ses valeurs.
            row.spawn(Self::well(1.6, 8.0)).with_children(|list| {
                // En-têtes de colonnes.
                list.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8),
                    padding: UiRect::new(px(14), px(10), px(2), px(4)),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|head| {
                    for column in columns {
                        head.spawn(cell_node(column)).with_children(|c| {
                            c.spawn(self.label(&column.label, 12.0, theme::TEXT_DIM));
                        });
                    }
                });
                list.spawn(theme::hairline(theme::RULE));
                if rows.is_empty() {
                    list.spawn((
                        Text::new(empty.to_owned()),
                        self.fonts.body(15.0),
                        TextColor(theme::TEXT_DIM),
                        Node {
                            margin: UiRect::all(px(12)),
                            ..default()
                        },
                    ));
                }
                if self.scroll > 0 {
                    self.more(list, '▲', self.scroll);
                }
                for (i, r) in rows.iter().enumerate().skip(self.scroll).take(VISIBLE_ROWS) {
                    self.row(list, columns, i, r);
                }
                let after = rows.len().saturating_sub(self.scroll + VISIBLE_ROWS);
                if after > 0 {
                    self.more(list, '▼', after);
                }
            });
            row.spawn(Self::well(1.0, 14.0)).with_children(|detail| {
                if let Some(r) = rows.get(self.cursor) {
                    self.detail(detail, &r.detail);
                    let cursor = self.cursor;
                    self.actions(detail, |c| c.slot == Slot::Row(cursor));
                } else {
                    detail.spawn((
                        Text::new(empty.to_owned()),
                        self.fonts.body(15.0),
                        TextColor(theme::TEXT_DIM),
                    ));
                }
            });
        });
        if let Some(below) = below {
            p.spawn(Self::well(0.0, 10.0))
                .insert(Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(px(14), px(8)),
                    border: UiRect::all(px(1)),
                    row_gap: px(3),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|b| self.detail(b, below));
        }
    }

    /// L'indication des lignes hors de vue (« ▲ 3 »).
    fn more(&self, p: &mut ChildSpawnerCommands, arrow: char, count: usize) {
        p.spawn((
            Text::new(format!("{arrow} {count}")),
            self.fonts.mono(13.0),
            TextColor(theme::SCREEN_DIM),
            Node {
                align_self: AlignSelf::FlexEnd,
                margin: UiRect::horizontal(px(10)),
                ..default()
            },
        ));
    }

    fn row(&self, p: &mut ChildSpawnerCommands, columns: &[Column], index: usize, row: &Row) {
        let selected = index == self.cursor;
        let base = if selected {
            ROW_SELECTED
        } else if index.is_multiple_of(2) {
            ROW_EVEN
        } else {
            ROW_ODD
        };
        let hover = if selected { ROW_SELECTED } else { ROW_HOVER };
        p.spawn((
            Button,
            ActivateOnPress,
            RowButton(index),
            HoverFill { base, hover },
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                height: px(ROW_HEIGHT),
                padding: UiRect::axes(px(14), px(0)).with_right(px(10)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(base),
        ))
        .with_children(|r| {
            // Le repère à gauche : ambre pour la ligne choisie, sinon la couleur de la ligne.
            let marker = match (selected, row.marker) {
                (true, _) => Some(theme::ACCENT),
                (false, Tone::Normal) => None,
                (false, tone) => Some(tone_color(tone)),
            };
            if let Some(color) = marker {
                r.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(4),
                        bottom: px(4),
                        width: px(3),
                        ..default()
                    },
                    BackgroundColor(color),
                ));
            }
            for (column, cell) in columns.iter().zip(&row.cells) {
                let color = if row.enabled {
                    tone_color(cell.tone)
                } else {
                    theme::TEXT_DISABLED
                };
                r.spawn(cell_node(column)).with_children(|c| {
                    c.spawn((
                        Text::new(cell.text.clone()),
                        self.value_font(cell.mono, 16.0),
                        TextColor(color),
                        TextLayout::no_wrap(),
                    ));
                });
            }
        });
    }

    fn panels(&self, p: &mut ChildSpawnerCommands, panels: &[Detail]) {
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(12),
            align_items: AlignItems::Stretch,
            flex_shrink: 1.0,
            min_height: px(0),
            overflow: Overflow::clip_y(),
            ..default()
        })
        .with_children(|row| {
            for (i, detail) in panels.iter().enumerate() {
                row.spawn(Self::well(1.0, 14.0)).with_children(|panel| {
                    self.detail(panel, detail);
                    self.actions(panel, |c| c.slot == Slot::Panel(i));
                });
            }
        });
    }

    fn detail(&self, p: &mut ChildSpawnerCommands, detail: &Detail) {
        if !detail.title.is_empty() {
            p.spawn((
                Text::new(detail.title.clone()),
                self.fonts.title(18.0),
                TextColor(theme::TEXT),
            ));
        }
        if let Some((subtitle, tone)) = &detail.subtitle {
            p.spawn(self.label(subtitle, 12.0, tone_color(*tone)));
        }
        if !detail.title.is_empty() || detail.subtitle.is_some() {
            p.spawn(theme::hairline(theme::RULE)).insert(Node {
                width: percent(100),
                height: px(1),
                margin: UiRect::vertical(px(4)),
                flex_shrink: 0.0,
                ..default()
            });
        }
        if detail.compact {
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|grid| {
                for (i, field) in detail.fields.iter().enumerate() {
                    // Bandes par paire : une ligne de la grille sur deux.
                    self.field(grid, field, (i / 2).is_multiple_of(2), Some(percent(50)));
                }
            });
        } else {
            for (i, field) in detail.fields.iter().enumerate() {
                self.field(p, field, i.is_multiple_of(2), None);
            }
        }
        self.gauges(p, detail);
        for (text, tone) in &detail.text {
            p.spawn((
                Text::new(text.clone()),
                self.fonts.body(14.0),
                TextColor(tone_color(*tone)),
                Node {
                    margin: UiRect::top(px(4)),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
        }
    }

    /// Une ligne « libellé … valeur ». Le libellé garde sa taille ; la valeur prend toute la
    /// largeur restante (base nulle, croissance 1) : sans cela, un texte qui peut rétrécir est
    /// mesuré à sa largeur minimale et se brise à chaque mot. Les chiffres, montants et dates ne
    /// passent jamais à la ligne ; un nom long y passe, en entier.
    fn field(&self, p: &mut ChildSpawnerCommands, field: &Field, band: bool, width: Option<Val>) {
        let linebreak = if field.wrap {
            LineBreak::WordBoundary
        } else {
            LineBreak::NoWrap
        };
        p.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                padding: UiRect::axes(px(8), px(3)),
                width: width.unwrap_or(Val::Auto),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(if band { FIELD_BAND } else { Color::NONE }),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(field.label.clone()),
                self.fonts.body(14.0),
                TextColor(theme::TEXT_DIM),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            row.spawn((
                Text::new(field.value.clone()),
                self.value_font(field.mono, 15.0),
                TextColor(tone_color(field.tone)),
                TextLayout::new(Justify::Right, linebreak),
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    ..default()
                },
            ));
        });
    }

    fn gauges(&self, p: &mut ChildSpawnerCommands, detail: &Detail) {
        if detail.gauges.is_empty() {
            return;
        }
        let columns = detail.gauge_columns.len().max(1) as f32;
        // Les en-têtes de colonnes, s'il y en a (une jauge seule, le réservoir, n'en a pas).
        if detail.gauge_columns.is_empty() {
            for gauge in &detail.gauges {
                self.gauge(p, gauge, columns, true);
            }
            return;
        }
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(10),
            padding: UiRect::axes(px(8), px(2)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|head| {
            head.spawn(Node {
                width: px(GAUGE_LABEL),
                flex_shrink: 0.0,
                ..default()
            });
            for column in &detail.gauge_columns {
                head.spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    ..default()
                })
                .with_children(|c| {
                    c.spawn(self.label(column, 12.0, theme::TEXT_DIM));
                });
            }
            head.spawn(Node {
                width: px(GAUGE_VALUE),
                flex_shrink: 0.0,
                ..default()
            });
        });
        for (i, gauge) in detail.gauges.iter().enumerate() {
            self.gauge(p, gauge, columns, i.is_multiple_of(2));
        }
    }

    /// Une ligne de jauge : libellé, une barre par colonne, valeur.
    fn gauge(&self, p: &mut ChildSpawnerCommands, gauge: &Gauge, columns: f32, band: bool) {
        p.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                padding: UiRect::axes(px(8), px(5)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(if band { FIELD_BAND } else { Color::NONE }),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(gauge.label.clone()),
                self.fonts.body(14.0),
                TextColor(theme::TEXT),
                Node {
                    width: px(GAUGE_LABEL),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            // Une jauge qui a moins de barres que de colonnes (le cœur) les couvre toutes.
            let grow = columns / gauge.parts.len().max(1) as f32;
            for &part in &gauge.parts {
                let part = part.clamp(0.0, 1.0);
                row.spawn((
                    Node {
                        flex_grow: grow,
                        flex_basis: px(0),
                        height: px(9),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BorderColor::all(theme::RULE),
                    BackgroundColor(GAUGE_BACK),
                    children![(
                        Node {
                            width: percent((part * 100.0) as f32),
                            height: percent(100),
                            ..default()
                        },
                        BackgroundColor(tone_color(fraction_tone(part))),
                    )],
                ));
            }
            row.spawn((
                Text::new(gauge.value.clone()),
                self.fonts.mono(14.0),
                TextColor(tone_color(gauge.tone)),
                TextLayout::justify(Justify::Right),
                Node {
                    width: px(GAUGE_VALUE),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
        });
    }

    // --- Pied ---------------------------------------------------------------------------------

    fn footer(&self, p: &mut ChildSpawnerCommands) {
        if let Some(note) = &self.page.note {
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Node {
                        width: px(8),
                        height: px(8),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(tone_color(note.tone)),
                ));
                row.spawn((
                    Text::new(note.text.clone()),
                    self.fonts.body(15.0),
                    TextColor(tone_color(note.tone)),
                ));
            });
        }
        p.spawn(theme::hairline(theme::RULE));
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: px(12),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: px(8),
                row_gap: px(6),
                ..default()
            })
            .with_children(|bar| {
                for (i, c) in self.page.choices.iter().enumerate() {
                    if c.slot == Slot::Bar && !matches!(c.action, Action::Close) {
                        self.choice_button(bar, i);
                    }
                }
            });
            if let Some(close) = self.page.close_index() {
                self.choice_button(row, close);
            }
        });
        p.spawn((
            Text::new(self.hint()),
            self.fonts.body(14.0),
            TextColor(theme::TEXT_DIM),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
        ));
    }

    /// La consigne : seulement les gestes qui servent sur cet écran.
    fn hint(&self) -> String {
        let t = self.texts;
        let page = self.page;
        let mut parts = Vec::new();
        if !page.rows().is_empty() {
            parts.push(t.get("terminal.hint.rows"));
        }
        if !self.numbered.is_empty() {
            parts.push(t.get(if matches!(page.body, Body::Dialogue { .. }) {
                "terminal.hint.reply"
            } else {
                "terminal.hint.digits"
            }));
        }
        if self.focus.is_some() {
            parts.push(t.get("terminal.hint.enter"));
        }
        if !page.tabs.is_empty() {
            parts.push(t.get("terminal.hint.tabs"));
        }
        let back = page.choices.iter().any(|c| {
            matches!(
                &c.action,
                Action::Goto(Screen::Dialogue { node, .. }) if node.is_empty()
            )
        });
        parts.push(t.get(if back {
            "terminal.hint.back"
        } else {
            "terminal.hint.escape"
        }));
        parts.join("   ·   ")
    }
}

/// La case d'une colonne : largeur fixe, ou la place restante ; coupée si le texte dépasse.
fn cell_node(column: &Column) -> Node {
    let fixed = column.width > 0.0;
    Node {
        width: if fixed { px(column.width) } else { Val::Auto },
        flex_grow: if fixed { 0.0 } else { 1.0 },
        flex_shrink: if fixed { 0.0 } else { 1.0 },
        flex_basis: if fixed { Val::Auto } else { px(0) },
        min_width: px(0),
        overflow: Overflow::clip(),
        justify_content: if column.right {
            JustifyContent::FlexEnd
        } else {
            JustifyContent::FlexStart
        },
        ..default()
    }
}

// --- Pictogrammes ---------------------------------------------------------------------------

/// Un rectangle plein, en coordonnées de la case du pictogramme (40 × 40).
fn rect(p: &mut ChildSpawnerCommands, (left, top, width, height): (f32, f32, f32, f32), c: Color) {
    shape(p, (left, top, width, height), c, BorderRadius::ZERO);
}

fn shape(
    p: &mut ChildSpawnerCommands,
    (left, top, width, height): (f32, f32, f32, f32),
    color: Color,
    border_radius: BorderRadius,
) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(width),
            height: px(height),
            border_radius,
            ..default()
        },
        BackgroundColor(color),
    ));
}

/// Un contour : seulement les bords `border`, de la couleur donnée.
fn outline(
    p: &mut ChildSpawnerCommands,
    (left, top, width, height): (f32, f32, f32, f32),
    border: UiRect,
    color: Color,
    border_radius: BorderRadius,
) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(width),
            height: px(height),
            border,
            border_radius,
            ..default()
        },
        BorderColor::all(color),
    ));
}

/// Le pictogramme de la borne : quelques formes ambre et cyan dans une case à filet d'acier.
/// Purement visuel, sans texte.
fn glyph(p: &mut ChildSpawnerCommands, glyph: Glyph) {
    let a = theme::ACCENT;
    let s = theme::SCREEN;
    let line = UiRect::all(px(2));
    p.spawn((
        Node {
            width: px(42),
            height: px(42),
            border: UiRect::all(px(1)),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(theme::STEEL),
        BackgroundColor(GLYPH_BACK),
    ))
    .with_children(|b| match glyph {
        // Un contrat : une feuille et ses lignes.
        Glyph::Contracts => {
            outline(b, (11.0, 7.0, 18.0, 26.0), line, a, BorderRadius::ZERO);
            rect(b, (15.0, 13.0, 10.0, 2.0), s);
            rect(b, (15.0, 18.0, 10.0, 2.0), s);
            rect(b, (15.0, 23.0, 6.0, 2.0), s);
        }
        // Le marché : des cours en barres.
        Glyph::Market => {
            rect(b, (9.0, 22.0, 5.0, 10.0), a);
            rect(b, (17.0, 15.0, 5.0, 17.0), a);
            rect(b, (25.0, 9.0, 5.0, 23.0), s);
            rect(b, (7.0, 33.0, 26.0, 2.0), theme::STEEL);
        }
        // La Banque : une pile de jetons.
        Glyph::Bank => {
            rect(b, (10.0, 27.0, 20.0, 4.0), a);
            rect(b, (12.0, 21.0, 20.0, 4.0), a);
            rect(b, (10.0, 15.0, 20.0, 4.0), a);
            rect(b, (12.0, 9.0, 20.0, 4.0), s);
        }
        // Le chantier : un engrenage.
        Glyph::Shipyard => {
            outline(
                b,
                (13.0, 13.0, 14.0, 14.0),
                UiRect::all(px(3)),
                a,
                BorderRadius::ZERO,
            );
            rect(b, (18.0, 7.0, 4.0, 6.0), a);
            rect(b, (18.0, 27.0, 4.0, 6.0), a);
            rect(b, (7.0, 18.0, 6.0, 4.0), a);
            rect(b, (27.0, 18.0, 6.0, 4.0), a);
            rect(b, (18.0, 18.0, 4.0, 4.0), s);
        }
        // La taverne : un verre.
        Glyph::Bar => {
            outline(
                b,
                (12.0, 8.0, 16.0, 14.0),
                UiRect::new(px(2), px(2), px(0), px(2)),
                a,
                BorderRadius::ZERO,
            );
            rect(b, (14.0, 13.0, 12.0, 7.0), s);
            rect(b, (19.0, 22.0, 2.0, 7.0), a);
            rect(b, (14.0, 29.0, 12.0, 2.0), a);
        }
        // Le journal : une liste à puces.
        Glyph::Log => {
            for (y, len) in [(11.0, 16.0), (18.0, 16.0), (25.0, 11.0)] {
                rect(b, (9.0, y, 3.0, 3.0), a);
                rect(b, (15.0, y + 0.5, len, 2.0), s);
            }
        }
        // La fiche : une silhouette.
        Glyph::Sheet => {
            shape(b, (15.0, 7.0, 10.0, 10.0), a, BorderRadius::MAX);
            shape(
                b,
                (11.0, 20.0, 18.0, 12.0),
                a,
                BorderRadius::px(6.0, 6.0, 0.0, 0.0),
            );
        }
        // Le codex : un livre ouvert.
        Glyph::Codex => {
            outline(b, (8.0, 10.0, 12.0, 20.0), line, a, BorderRadius::ZERO);
            outline(b, (20.0, 10.0, 12.0, 20.0), line, a, BorderRadius::ZERO);
            rect(b, (19.0, 10.0, 2.0, 20.0), s);
        }
        // Une conversation : une bulle.
        Glyph::Talk => {
            outline(
                b,
                (8.0, 9.0, 24.0, 16.0),
                line,
                a,
                BorderRadius::px(3.0, 3.0, 3.0, 3.0),
            );
            rect(b, (12.0, 25.0, 5.0, 5.0), a);
            rect(b, (13.0, 16.0, 14.0, 2.0), s);
        }
    });
}

// --- Souris ---------------------------------------------------------------------------------

pub(super) fn on_choice_button(
    event: On<Activate>,
    buttons: Query<&ChoiceButton>,
    mut choices: MessageWriter<TerminalChoice>,
) {
    if let Ok(ChoiceButton(i)) = buttons.get(event.entity) {
        choices.write(TerminalChoice(*i));
    }
}

/// Cliquer une ligne la choisit (son détail et ses actions s'affichent à droite).
pub(super) fn on_row_button(
    event: On<Activate>,
    rows: Query<&RowButton>,
    terminal: Option<ResMut<Terminal>>,
) {
    if let (Ok(RowButton(i)), Some(mut term)) = (rows.get(event.entity), terminal)
        && term.cursor != *i
    {
        term.cursor = *i;
    }
}

pub(super) fn on_tab_button(
    event: On<Activate>,
    tabs: Query<&TabButton>,
    terminal: Option<ResMut<Terminal>>,
) {
    if let (Ok(TabButton(screen)), Some(mut term)) = (tabs.get(event.entity), terminal)
        && term.screen != *screen
    {
        term.note = None;
        term.goto(screen.clone());
    }
}

pub(super) fn on_hover(
    event: On<PointerOver>,
    fonts: Res<UiFonts>,
    mut buttons: Query<
        (Option<&mut ImageNode>, Option<&mut BackgroundColor>),
        (With<ChoiceButton>, Without<DisabledChoice>),
    >,
    mut fills: Query<(&HoverFill, &mut BackgroundColor), Without<ChoiceButton>>,
) {
    if let Ok((image, background)) = buttons.get_mut(event.entity) {
        theme::set_hovered(&fonts, true, image, background);
    }
    if let Ok((fill, mut background)) = fills.get_mut(event.entity) {
        background.0 = fill.hover;
    }
}

pub(super) fn on_unhover(
    event: On<PointerOut>,
    fonts: Res<UiFonts>,
    mut buttons: Query<
        (Option<&mut ImageNode>, Option<&mut BackgroundColor>),
        (With<ChoiceButton>, Without<Focused>),
    >,
    mut fills: Query<(&HoverFill, &mut BackgroundColor), Without<ChoiceButton>>,
) {
    if let Ok((image, background)) = buttons.get_mut(event.entity) {
        theme::set_hovered(&fonts, false, image, background);
    }
    if let Ok((fill, mut background)) = fills.get_mut(event.entity) {
        background.0 = fill.base;
    }
}
