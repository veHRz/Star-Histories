//! Le bandeau de combat (M6) : l'état du vaisseau par section, missiles et leurres, et l'alarme
//! (voile rouge) après un impact ou quand la coque est au plus mal. Visible seulement quand le
//! vaisseau du joueur est en combat.
//!
//! Habillage (M8) : un relevé d'instrument (équerres cyan, rouges quand la coque est au plus
//! mal), les valeurs en tableau à chasse fixe : sections et pourcentages alignés en colonnes.

use bevy::prelude::*;
use sh_data::SectionId;

use crate::combat::space::{AiPilot, Combatant, Side};
use crate::data::Texts;
use crate::state::ViewLevel;
use crate::ui::theme::{self, Frame, UiFonts};

pub struct CombatHudPlugin;

impl Plugin for CombatHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Global), spawn_combat_hud)
            .add_systems(
                Update,
                update_combat_hud.run_if(in_state(ViewLevel::Global)),
            );
    }
}

#[derive(Component)]
struct CombatPanel;

/// Le titre du relevé (« COQUE »).
#[derive(Component)]
struct CombatTitle;

/// Le tableau des valeurs.
#[derive(Component)]
struct CombatText;

/// Les commandes de combat, en petit sous le tableau.
#[derive(Component)]
struct CombatHint;

#[derive(Component)]
struct Alarm;

/// Intégrité sous laquelle la coque est « au plus mal » : battement d'alarme, relevé rouge.
const CRITICAL: f64 = 0.35;

/// Teinte du cadre quand la coque est au plus mal (multiplie le cyan des équerres).
const CRITICAL_TINT: Color = Color::srgb(1.0, 0.42, 0.36);

fn spawn_combat_hud(mut commands: Commands, fonts: Res<UiFonts>) {
    commands.spawn((
        Name::new("Alarme"),
        Alarm,
        DespawnOnExit(ViewLevel::Global),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            border: UiRect::all(px(10)),
            ..default()
        },
        BorderColor::all(theme::DANGER.with_alpha(0.0)),
        BackgroundColor(Color::srgba(0.6, 0.0, 0.0, 0.0)),
        Visibility::Hidden,
    ));
    commands.spawn((
        Name::new("Bandeau de combat"),
        CombatPanel,
        DespawnOnExit(ViewLevel::Global),
        Node {
            position_type: PositionType::Absolute,
            left: px(20),
            bottom: px(28),
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(px(18), px(18), px(10), px(12)),
            ..default()
        },
        fonts.frame(Frame::Readout),
        Visibility::Hidden,
        children![
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(24),
                    ..default()
                },
                children![
                    (
                        CombatTitle,
                        Text::new(""),
                        fonts.heading(14.0),
                        TextColor(theme::ACCENT),
                    ),
                    theme::status_lights(),
                ],
            ),
            (
                Node {
                    width: percent(100),
                    height: px(1),
                    margin: UiRect::vertical(px(6)),
                    ..default()
                },
                BackgroundColor(theme::SCREEN_DIM),
            ),
            (
                CombatText,
                Text::new(""),
                fonts.mono(15.0),
                TextColor(theme::TEXT),
            ),
            (
                CombatHint,
                Text::new(""),
                fonts.body(14.0),
                TextColor(theme::TEXT_DIM),
                Node {
                    margin: UiRect::top(px(8)),
                    ..default()
                },
            ),
        ],
    ));
}

/// Remplace le texte seulement s'il a changé : un texte réécrit est remis en page.
fn set_text(text: &mut Mut<Text>, wanted: &str) {
    if text.0 != wanted {
        text.0 = wanted.to_owned();
    }
}

#[allow(clippy::type_complexity)]
fn update_combat_hud(
    time: Res<Time>,
    texts: Res<Texts>,
    player: Query<&Combatant, Without<AiPilot>>,
    mut panel: Query<(&mut Visibility, &mut ImageNode), (With<CombatPanel>, Without<Alarm>)>,
    mut alarm: Query<
        (&mut Visibility, &mut BorderColor, &mut BackgroundColor),
        (With<Alarm>, Without<CombatPanel>),
    >,
    mut title: Query<(&mut Text, &mut TextColor), With<CombatTitle>>,
    mut body: Query<&mut Text, (With<CombatText>, Without<CombatTitle>, Without<CombatHint>)>,
    mut hint: Query<&mut Text, (With<CombatHint>, Without<CombatTitle>, Without<CombatText>)>,
) {
    let me = player.iter().find(|c| c.side == Side::Player);
    let shown = if me.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut v, _) in &mut panel {
        if *v != shown {
            *v = shown;
        }
    }
    let Some(me) = me else {
        for (mut v, ..) in &mut alarm {
            *v = Visibility::Hidden;
        }
        return;
    };
    let critical = me.hull.integrity() < CRITICAL;

    // Tableau à chasse fixe : noms de section complétés à la même largeur, pourcentages
    // alignés à droite sur trois chiffres.
    let pct = |x: f64, max: f64| format!("{:>3}", texts.number(100.0 * x / max.max(1e-9), 0));
    let sections: Vec<_> = SectionId::ALL
        .into_iter()
        .filter_map(|id| {
            me.hull
                .section(id)
                .map(|s| (texts.get(&format!("combat.section.{}", id.key())), s))
        })
        .collect();
    let width = sections
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for (name, s) in &sections {
        lines.push(texts.format(
            "combat.hud.section",
            &[
                ("section", &format!("{name:<width$}")),
                ("armor", &pct(s.armor, s.max_armor)),
                ("structure", &pct(s.structure, s.max_structure)),
            ],
        ));
    }
    lines.push(texts.format(
        "combat.hud.core",
        &[("core", &pct(me.hull.core, me.hull.max_core))],
    ));
    lines.push(texts.format(
        "combat.hud.ammo",
        &[
            (
                "missiles",
                &me.ammo(sh_data::WeaponKind::Missile).to_string(),
            ),
            ("decoys", &me.countermeasures.to_string()),
        ],
    ));
    let wanted = lines.join("\n");
    for mut t in &mut body {
        set_text(&mut t, &wanted);
    }
    let title_text = theme::caps(&texts.get("combat.hud.title"));
    let title_color = if critical {
        theme::DANGER
    } else {
        theme::ACCENT
    };
    for (mut t, mut color) in &mut title {
        set_text(&mut t, &title_text);
        if color.0 != title_color {
            color.0 = title_color;
        }
    }
    let hint_text = texts.get("combat.hud.hint");
    for mut t in &mut hint {
        set_text(&mut t, &hint_text);
    }
    let tint = if critical {
        CRITICAL_TINT
    } else {
        Color::WHITE
    };
    for (_, mut image) in &mut panel {
        if image.color != tint {
            image.color = tint;
        }
    }

    // L'alarme : un éclat rouge à l'impact, un battement lent quand la coque est au plus mal.
    let flash = (1.0 - me.since_hit as f32).max(0.0);
    let beat = if critical {
        0.5 + 0.5 * (time.elapsed_secs() * 4.0).sin()
    } else {
        0.0
    };
    let k = flash.max(beat * 0.6);
    for (mut v, mut border, mut bg) in &mut alarm {
        let wanted = if k > 0.01 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != wanted {
            *v = wanted;
        }
        *border = BorderColor::all(theme::DANGER.with_alpha(0.8 * k));
        bg.0 = Color::srgba(0.6, 0.0, 0.0, 0.12 * k);
    }
}
