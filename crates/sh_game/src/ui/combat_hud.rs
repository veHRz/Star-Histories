//! Le schéma du vaisseau (M9, lot L2) et l'alarme de combat (M6).
//!
//! **Schéma** : toujours visible à bord en vue Globale (pas seulement en combat), en bas à
//! droite, loin du bandeau de pilotage (bas centre) et des messages. Une silhouette vue de
//! dessus, nez en haut ; autour, quatre arcs de plaques, un par section (proue, poupe, bâbord,
//! tribord : P26). **Pas de bouclier, jamais** (AGENTS.md §13, Q16) :
//! - les arcs montrent le **blindage** : ils raccourcissent vers leur milieu quand il s'use et
//!   passent du cyan à l'ambre puis au rouge ; une plaque perdue laisse son emplacement rouge
//!   sombre ; le pourcentage est écrit à côté ;
//! - les régions de la silhouette prennent la couleur de la **structure** de leur section (rouge :
//!   section au plus mal ou ouverte) ;
//! - le losange central, celle du **cœur** ; dessous, le cœur, les missiles et les leurres.
//!
//! Hors combat, l'état vient de la fiche du capitaine (les dégâts restent jusqu'au chantier) ;
//! en combat, de la coque qui se bat. Le schéma n'est réécrit que quand l'état affiché change
//! (pourcentages entiers) : en vol tranquille, il ne coûte rien.
//!
//! **Alarme** : un voile rouge à l'impact, un battement lent quand la coque est au plus mal ; le
//! titre et les équerres du schéma rougissent alors.
//!
//! Images blanches teintées par le jeu : `assets/ui/hud/`, générées par `tools/ui/hud.py` ; les
//! rectangles `SCHEMATIC_*` ci-dessous sont ceux que ce script imprime.

use bevy::prelude::*;
use sh_data::{SectionId, WeaponKind};

use crate::career::Career;
use crate::combat::space::{AiPilot, Combatant, Side};
use crate::data::{Data, Texts};
use crate::state::ViewLevel;
use crate::ui::theme::{self, Frame, UiFonts};

pub struct CombatHudPlugin;

impl Plugin for CombatHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ViewLevel::Global), spawn_ship_hud)
            .add_systems(Update, update_ship_hud.run_if(in_state(ViewLevel::Global)));
    }
}

/// Zone du dessin, en pixels d'interface ; les images y sont dessinées au double.
const SCHEMATIC_AREA: Vec2 = Vec2::new(184.0, 168.0);
const IMAGE_SCALE: f32 = 2.0;
/// Rectangles (x, y, largeur, hauteur) des calques dans la zone (`tools/ui/hud.py`). La poupe
/// et le tribord sont les reflets de la proue et du bâbord.
const SCHEMATIC_SHIP: [f32; 4] = [62.0, 22.0, 60.0, 124.0];
const SCHEMATIC_BOW_ARC: [f32; 4] = [55.0, 7.0, 74.0, 24.0];
const SCHEMATIC_PORT_ARC: [f32; 4] = [31.0, 44.0, 18.0, 80.0];
const SCHEMATIC_CORE: [f32; 4] = [85.0, 81.0, 14.0, 14.0];

/// Distance au bas de l'écran au-dessus de laquelle le coin bas droit est libre en vue Globale
/// (le schéma fait ≈ 250 px de haut, posé à 28 px du bas) : la santé et les munitions à pied
/// (`combat::foot`) s'y placent.
pub const SCHEMATIC_CLEARANCE: f32 = 292.0;

/// Intégrité sous laquelle la coque est « au plus mal » : battement d'alarme, schéma rouge.
const CRITICAL: f64 = 0.35;
/// Seuils des couleurs d'état (pourcentages) : cyan au-dessus du premier, ambre, puis rouge.
const GOOD: u8 = 60;
const WORN: u8 = 30;

/// Teinte du cadre quand la coque est au plus mal (multiplie le cyan des équerres).
const CRITICAL_TINT: Color = Color::srgb(1.0, 0.42, 0.36);
/// Emplacement d'une plaque (arc de fond) ; rouge sombre quand le blindage de la section est
/// perdu.
const ARC_SLOT: Color = Color::srgba(0.42, 0.45, 0.49, 0.30);
const ARC_LOST: Color = Color::srgba(0.92, 0.24, 0.18, 0.40);

#[derive(Component)]
struct ShipPanel;

#[derive(Component)]
struct Alarm;

/// Un calque image du schéma.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Layer {
    /// L'emplacement des plaques d'une section.
    ArcSlot(SectionId),
    /// Les plaques qui restent.
    ArcFill(SectionId),
    /// La région de la silhouette.
    Region(SectionId),
    Core,
}

/// Un texte du schéma.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Readout {
    Title,
    Armor(SectionId),
    Core,
    Missiles,
    Decoys,
}

/// Ce que montre le schéma, en pourcentages entiers : il n'est réécrit que quand cela change.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ShipStatus {
    /// Par section, dans l'ordre de [`SectionId::ALL`] ; `None` : la coque n'a pas la section.
    armor: [Option<u8>; 4],
    structure: [Option<u8>; 4],
    core: u8,
    missiles: u32,
    decoys: u32,
    critical: bool,
}

/// Où dessiner l'arc d'une section, et son pourcentage.
struct ArcLayout {
    rect: [f32; 4],
    image: &'static str,
    flip_x: bool,
    flip_y: bool,
    /// L'arc raccourcit en hauteur (bâbord, tribord), sinon en largeur.
    vertical: bool,
    /// Le pourcentage : gauche, haut, largeur, alignement.
    number: (f32, f32, f32, Justify),
}

fn arc_layout(id: SectionId) -> ArcLayout {
    let [bx, by, bw, bh] = SCHEMATIC_BOW_ARC;
    let [px_, py, pw, ph] = SCHEMATIC_PORT_ARC;
    let (w, h) = (SCHEMATIC_AREA.x, SCHEMATIC_AREA.y);
    // Chiffres : au bout droit des arcs de proue et de poupe, à l'extérieur des arcs latéraux.
    let mid = h / 2.0 - 7.0;
    match id {
        SectionId::Bow => ArcLayout {
            rect: [bx, by, bw, bh],
            image: "ui/hud/arc_bow.png",
            flip_x: false,
            flip_y: false,
            vertical: false,
            number: (bx + bw + 2.0, by - 1.0, 30.0, Justify::Left),
        },
        SectionId::Stern => ArcLayout {
            rect: [bx, h - by - bh, bw, bh],
            image: "ui/hud/arc_bow.png",
            flip_x: false,
            flip_y: true,
            vertical: false,
            number: (bx + bw + 2.0, h - by - 13.0, 30.0, Justify::Left),
        },
        SectionId::Port => ArcLayout {
            rect: [px_, py, pw, ph],
            image: "ui/hud/arc_port.png",
            flip_x: false,
            flip_y: false,
            vertical: true,
            number: (0.0, mid, px_ - 3.0, Justify::Right),
        },
        SectionId::Starboard => ArcLayout {
            rect: [w - px_ - pw, py, pw, ph],
            image: "ui/hud/arc_port.png",
            flip_x: true,
            flip_y: false,
            vertical: true,
            number: (w - px_ + 3.0, mid, px_ - 3.0, Justify::Left),
        },
    }
}

fn region_image(id: SectionId) -> &'static str {
    match id {
        SectionId::Bow => "ui/hud/hull_bow.png",
        SectionId::Stern => "ui/hud/hull_stern.png",
        SectionId::Port => "ui/hud/hull_port.png",
        SectionId::Starboard => "ui/hud/hull_starboard.png",
    }
}

/// Un calque image posé dans la zone du dessin.
fn layer(image: Handle<Image>, rect: [f32; 4], flip: (bool, bool), color: Color) -> impl Bundle {
    (
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(rect[0]),
            top: px(rect[1]),
            width: px(rect[2]),
            height: px(rect[3]),
            ..default()
        },
        ImageNode {
            image,
            color,
            flip_x: flip.0,
            flip_y: flip.1,
            // Étirée à la taille du nœud : un arc raccourci (rectangle de l'image et nœud réduits
            // ensemble) garde sa forme ; `Auto` arrondirait le rectangle et décalerait l'image.
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
    )
}

fn spawn_ship_hud(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Name::new("Alarme"),
        Alarm,
        DespawnOnExit(ViewLevel::Global),
        Pickable::IGNORE,
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
    let rule = || {
        (
            Pickable::IGNORE,
            Node {
                width: percent(100),
                height: px(1),
                margin: UiRect::vertical(px(6)),
                ..default()
            },
            BackgroundColor(theme::SCREEN_DIM),
        )
    };
    commands
        .spawn((
            Name::new("Schéma du vaisseau"),
            ShipPanel,
            DespawnOnExit(ViewLevel::Global),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                right: px(20),
                bottom: px(28),
                width: px(SCHEMATIC_AREA.x + 44.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::new(px(14), px(14), px(10), px(11)),
                ..default()
            },
            fonts.frame(Frame::Readout),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                Pickable::IGNORE,
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                children![
                    (
                        Readout::Title,
                        theme::heading(&fonts, &texts.get("combat.hud.title"), 13.0, theme::ACCENT),
                        Pickable::IGNORE,
                    ),
                    theme::status_lights(),
                ],
            ));
            panel.spawn(rule());
            panel
                .spawn((
                    Pickable::IGNORE,
                    Node {
                        width: px(SCHEMATIC_AREA.x),
                        height: px(SCHEMATIC_AREA.y),
                        ..default()
                    },
                ))
                .with_children(|area| {
                    for id in SectionId::ALL {
                        let arc = arc_layout(id);
                        let image: Handle<Image> = asset_server.load(arc.image);
                        let flip = (arc.flip_x, arc.flip_y);
                        area.spawn((
                            Layer::ArcSlot(id),
                            layer(image.clone(), arc.rect, flip, ARC_SLOT),
                        ));
                        area.spawn((
                            Layer::ArcFill(id),
                            layer(image, arc.rect, flip, theme::SCREEN),
                        ));
                    }
                    for id in SectionId::ALL {
                        area.spawn((
                            Layer::Region(id),
                            layer(
                                asset_server.load(region_image(id)),
                                SCHEMATIC_SHIP,
                                (false, false),
                                theme::SCREEN,
                            ),
                        ));
                    }
                    area.spawn((
                        Layer::Core,
                        layer(
                            asset_server.load("ui/hud/core.png"),
                            SCHEMATIC_CORE,
                            (false, false),
                            theme::SCREEN,
                        ),
                    ));
                    for id in SectionId::ALL {
                        let (left, top, width, justify) = arc_layout(id).number;
                        area.spawn((
                            Readout::Armor(id),
                            Pickable::IGNORE,
                            Text::new(""),
                            fonts.mono(12.0),
                            TextColor(theme::SCREEN),
                            TextLayout::justify(justify),
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(left),
                                top: px(top),
                                width: px(width),
                                ..default()
                            },
                        ));
                    }
                });
            panel.spawn(rule());
            panel
                .spawn((
                    Pickable::IGNORE,
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Baseline,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                ))
                .with_children(|footer| {
                    for (label, readout) in [
                        ("hud.ship.core", Readout::Core),
                        ("hud.ship.missiles", Readout::Missiles),
                        ("hud.ship.decoys", Readout::Decoys),
                    ] {
                        footer.spawn((
                            Pickable::IGNORE,
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Baseline,
                                column_gap: px(4),
                                ..default()
                            },
                            children![
                                (
                                    Text::new(texts.get(label)),
                                    fonts.body(12.0),
                                    TextColor(theme::TEXT_DIM),
                                    Pickable::IGNORE,
                                ),
                                (
                                    readout,
                                    Text::new(""),
                                    fonts.mono(13.0),
                                    TextColor(theme::TEXT),
                                    Pickable::IGNORE,
                                ),
                            ],
                        ));
                    }
                });
        });
}

/// Un pourcentage entier ; ce qui reste se voit : jamais 0 tant qu'il reste quelque chose.
fn share(value: f64, max: f64) -> u8 {
    if value <= 0.0 {
        return 0;
    }
    (100.0 * value / max.max(1e-9)).round().clamp(1.0, 100.0) as u8
}

fn state_color(pct: u8) -> Color {
    if pct >= GOOD {
        theme::SCREEN
    } else if pct >= WORN {
        theme::ACCENT
    } else {
        theme::DANGER
    }
}

/// L'état du vaisseau du joueur : la coque qui se bat, sinon celle de la fiche du capitaine
/// (`None` : intacte), d'après sa définition de combat. `None` sans vaisseau de départ.
fn ship_status(
    data: &Data,
    career: Option<&Career>,
    fighting: Option<&Combatant>,
) -> Option<ShipStatus> {
    let def = data.start().and_then(|s| data.combat.ship(&s.ship))?;
    let hull = match fighting {
        Some(c) => Some(&c.hull),
        None => career.and_then(|c| c.captain.hull.as_ref()),
    };
    let mut armor = [None; 4];
    let mut structure = [None; 4];
    for (i, id) in SectionId::ALL.into_iter().enumerate() {
        match hull {
            Some(hull) => {
                if let Some(s) = hull.section(id) {
                    armor[i] = Some(share(s.armor, s.max_armor));
                    structure[i] = Some(share(s.structure, s.max_structure));
                }
            }
            // Coque intacte : chaque section que la définition prévoit, pleine.
            None if def.sections.iter().any(|s| s.id == id) => {
                armor[i] = Some(100);
                structure[i] = Some(100);
            }
            None => {}
        }
    }
    let (missiles, decoys) = match fighting {
        Some(c) => (c.ammo(WeaponKind::Missile), c.countermeasures),
        // Hors combat : la dotation (rechargée à chaque combat, `Combatant::new`).
        None => (
            def.weapons
                .iter()
                .filter(|m| {
                    data.combat
                        .ship_weapons
                        .get(&m.weapon)
                        .is_some_and(|w| w.kind == WeaponKind::Missile)
                })
                .map(|m| m.ammo)
                .sum(),
            def.countermeasures,
        ),
    };
    Some(ShipStatus {
        armor,
        structure,
        core: hull.map_or(100, |h| share(h.core, h.max_core)),
        missiles,
        decoys,
        critical: hull.is_some_and(|h| h.integrity() < CRITICAL),
    })
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_ship_hud(
    time: Res<Time>,
    data: Res<Data>,
    texts: Res<Texts>,
    career: Option<Res<Career>>,
    fighters: Query<&Combatant, Without<AiPilot>>,
    mut panel: Query<
        (&mut Visibility, &mut ImageNode),
        (With<ShipPanel>, Without<Layer>, Without<Alarm>),
    >,
    mut layers: Query<(&Layer, &mut ImageNode, &mut Node, &mut Visibility)>,
    mut readouts: Query<(&Readout, &mut Text, &mut TextColor)>,
    mut alarm: Query<
        (&mut Visibility, &mut BorderColor, &mut BackgroundColor),
        (With<Alarm>, Without<ShipPanel>, Without<Layer>),
    >,
    mut last: Local<Option<ShipStatus>>,
) {
    let fighting = fighters.iter().find(|c| c.side == Side::Player);
    let status = ship_status(&data, career.as_deref(), fighting);

    // L'alarme (en combat seulement) : un éclat rouge à l'impact, un battement lent quand la
    // coque est au plus mal.
    let k = match fighting {
        Some(me) => {
            let flash = (1.0 - me.since_hit as f32).max(0.0);
            let beat = if me.hull.integrity() < CRITICAL {
                0.5 + 0.5 * (time.elapsed_secs() * 4.0).sin()
            } else {
                0.0
            };
            flash.max(beat * 0.6)
        }
        None => 0.0,
    };
    for (mut v, mut border, mut bg) in &mut alarm {
        let wanted = if k > 0.01 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != wanted {
            *v = wanted;
        }
        if k > 0.01 {
            *border = BorderColor::all(theme::DANGER.with_alpha(0.8 * k));
            bg.0 = Color::srgba(0.6, 0.0, 0.0, 0.12 * k);
        }
    }

    // Le schéma : réécrit seulement quand l'état affiché change (ou à l'arrivée d'un nouveau
    // schéma, encore vierge).
    let fresh = panel.iter().any(|(v, _)| *v == Visibility::Hidden) && status.is_some();
    if *last == status && !fresh {
        return;
    }
    let shown = if status.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut v, mut frame) in &mut panel {
        if *v != shown {
            *v = shown;
        }
        let tint = if status.as_ref().is_some_and(|s| s.critical) {
            CRITICAL_TINT
        } else {
            Color::WHITE
        };
        if frame.color != tint {
            frame.color = tint;
        }
    }
    if let Some(status) = &status {
        apply_layers(status, &mut layers);
        apply_readouts(status, &texts, &mut readouts);
    }
    *last = status;
}

/// Les calques : arcs raccourcis vers leur milieu (rectangle de l'image et nœud réduits
/// ensemble, les plaques gardent leur forme), couleurs d'état.
fn apply_layers(
    status: &ShipStatus,
    layers: &mut Query<(&Layer, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let index = |id: SectionId| SectionId::ALL.iter().position(|s| *s == id).unwrap_or(0);
    for (layer, mut image, mut node, mut visibility) in layers.iter_mut() {
        let (shown, color) = match *layer {
            Layer::ArcSlot(id) => match status.armor[index(id)] {
                Some(0) => (true, ARC_LOST),
                Some(_) => (true, ARC_SLOT),
                None => (false, ARC_SLOT),
            },
            Layer::ArcFill(id) => match status.armor[index(id)] {
                Some(pct) if pct > 0 => {
                    let arc = arc_layout(id);
                    let r = f32::from(pct) / 100.0;
                    let [x, y, w, h] = arc.rect;
                    let (iw, ih) = (w * IMAGE_SCALE, h * IMAGE_SCALE);
                    let (left, top, width, height, rect) = if arc.vertical {
                        let cut = (1.0 - r) / 2.0;
                        (
                            x,
                            y + cut * h,
                            w,
                            r * h,
                            Rect::new(0.0, cut * ih, iw, (1.0 - cut) * ih),
                        )
                    } else {
                        let cut = (1.0 - r) / 2.0;
                        (
                            x + cut * w,
                            y,
                            r * w,
                            h,
                            Rect::new(cut * iw, 0.0, (1.0 - cut) * iw, ih),
                        )
                    };
                    node.left = px(left);
                    node.top = px(top);
                    node.width = px(width);
                    node.height = px(height);
                    image.rect = Some(rect);
                    (true, state_color(pct))
                }
                _ => (false, theme::SCREEN),
            },
            Layer::Region(id) => match status.structure[index(id)] {
                Some(pct) => (true, state_color(pct).with_alpha(0.9)),
                None => (false, theme::SCREEN),
            },
            Layer::Core => (true, state_color(status.core)),
        };
        let wanted = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
        if image.color != color {
            image.color = color;
        }
    }
}

fn apply_readouts(
    status: &ShipStatus,
    texts: &Texts,
    readouts: &mut Query<(&Readout, &mut Text, &mut TextColor)>,
) {
    let index = |id: SectionId| SectionId::ALL.iter().position(|s| *s == id).unwrap_or(0);
    for (readout, mut text, mut color) in readouts.iter_mut() {
        let (wanted, wanted_color) = match *readout {
            Readout::Title => (
                text.0.clone(),
                if status.critical {
                    theme::DANGER
                } else {
                    theme::ACCENT
                },
            ),
            Readout::Armor(id) => match status.armor[index(id)] {
                Some(pct) => (pct.to_string(), state_color(pct)),
                None => (String::new(), theme::TEXT_DIM),
            },
            Readout::Core => (
                texts.format("hud.ship.percent", &[("value", &status.core.to_string())]),
                state_color(status.core),
            ),
            Readout::Missiles => (
                status.missiles.to_string(),
                if status.missiles == 0 {
                    theme::TEXT_DIM
                } else {
                    theme::TEXT
                },
            ),
            Readout::Decoys => (
                status.decoys.to_string(),
                if status.decoys == 0 {
                    theme::TEXT_DIM
                } else {
                    theme::TEXT
                },
            ),
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
        if color.0 != wanted_color {
            color.0 = wanted_color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pourcentages_visibles() {
        assert_eq!(share(0.0, 30.0), 0);
        assert_eq!(share(0.01, 30.0), 1);
        assert_eq!(share(30.0, 30.0), 100);
        assert_eq!(share(15.0, 30.0), 50);
    }

    #[test]
    fn couleurs_d_etat() {
        assert_eq!(state_color(100), theme::SCREEN);
        assert_eq!(state_color(45), theme::ACCENT);
        assert_eq!(state_color(10), theme::DANGER);
    }
}
