//! Le bandeau de combat (M6) : l'état du vaisseau par section, missiles et leurres, et l'alarme
//! (voile rouge) après un impact ou quand la coque est au plus mal. Visible seulement quand le
//! vaisseau du joueur est en combat.

use bevy::prelude::*;
use sh_data::SectionId;

use crate::combat::space::{AiPilot, Combatant, Side};
use crate::data::Texts;
use crate::state::ViewLevel;
use crate::ui::theme::{self, UiFonts};

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

#[derive(Component)]
struct CombatText;

#[derive(Component)]
struct Alarm;

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
        BorderColor::all(Color::srgba(0.9, 0.05, 0.02, 0.0)),
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
            padding: UiRect::axes(px(14), px(10)),
            ..default()
        },
        BackgroundColor(theme::PANEL),
        Visibility::Hidden,
        children![(
            CombatText,
            Text::new(""),
            fonts.body(14.0),
            TextColor(theme::TEXT),
        )],
    ));
}

fn update_combat_hud(
    time: Res<Time>,
    texts: Res<Texts>,
    player: Query<&Combatant, Without<AiPilot>>,
    mut panel: Query<&mut Visibility, (With<CombatPanel>, Without<Alarm>)>,
    mut alarm: Query<
        (&mut Visibility, &mut BorderColor, &mut BackgroundColor),
        (With<Alarm>, Without<CombatPanel>),
    >,
    mut text: Query<&mut Text, With<CombatText>>,
) {
    let me = player.iter().find(|c| c.side == Side::Player);
    let shown = if me.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut panel {
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
    let pct = |x: f64, max: f64| texts.number(100.0 * x / max.max(1e-9), 0);
    let mut lines = vec![texts.get("combat.hud.title")];
    for id in SectionId::ALL {
        if let Some(s) = me.hull.section(id) {
            lines.push(texts.format(
                "combat.hud.section",
                &[
                    (
                        "section",
                        &texts.get(&format!("combat.section.{}", id.key())),
                    ),
                    ("armor", &pct(s.armor, s.max_armor)),
                    ("structure", &pct(s.structure, s.max_structure)),
                ],
            ));
        }
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
    lines.push(texts.get("combat.hud.hint"));
    let wanted = lines.join("\n");
    for mut t in &mut text {
        if t.0 != wanted {
            t.0.clone_from(&wanted);
        }
    }
    // L'alarme : un éclat rouge à l'impact, un battement lent quand la coque est au plus mal.
    let flash = (1.0 - me.since_hit as f32).max(0.0);
    let critical = me.hull.integrity() < 0.35;
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
        *border = BorderColor::all(Color::srgba(0.9, 0.05, 0.02, 0.8 * k));
        bg.0 = Color::srgba(0.6, 0.0, 0.0, 0.12 * k);
    }
}
