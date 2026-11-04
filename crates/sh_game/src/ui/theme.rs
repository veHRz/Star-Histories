//! Identité de l'interface : polices, couleurs, cadres et gabarits communs.
//!
//! Direction « futur usé » (M8, `docs/refonte-visuelle.md` §2, lot U) : une interface d'appareil
//! utilitaire, pas un site web. Panneaux sombres à pans coupés, filets fins, rivets discrets,
//! coins marqués ; ambre (voyants) pour ce qui s'active, cyan (écrans) pour les relevés ; titres
//! en capitales espacées ; chiffres des relevés en police à chasse fixe. Les cadres sont de
//! petites images découpées en 9 tranches (`assets/ui/`, générées par `tools/ui/frames.py`) : un
//! seul quad par cadre, rien de plus coûteux qu'un fond uni sur le GPU intégré.
//!
//! Lisibilité d'abord : contrastes francs, tailles de texte jamais plus petites qu'avant, et en
//! vue à la première personne rien qui ne couvre le centre de l'écran.

use bevy::prelude::*;
use bevy::text::LetterSpacing;

/// Fond des écrans pleins (menus, chargement) : centre de la vignette.
pub const BACKDROP: Color = Color::srgb(0.040, 0.046, 0.056);
/// Bords de la vignette des écrans pleins.
pub const BACKDROP_EDGE: Color = Color::srgb(0.012, 0.014, 0.018);
/// Panneau uni (bandeaux et fiches des autres modules) : même teinte que le cadre `panel.png`.
pub const PANEL: Color = Color::srgba(0.050, 0.057, 0.068, 0.94);
/// Voile posé sur la vue 3D derrière un menu de pause ou un terminal.
pub const OVERLAY: Color = Color::srgba(0.010, 0.012, 0.016, 0.62);
pub const TEXT: Color = Color::srgb(0.91, 0.915, 0.92);
pub const TEXT_DIM: Color = Color::srgb(0.60, 0.63, 0.66);
pub const TEXT_DISABLED: Color = Color::srgb(0.34, 0.36, 0.39);
/// Ambre des voyants : ce qui s'active, ce qui demande l'attention.
pub const ACCENT: Color = Color::srgb(0.95, 0.66, 0.24);
/// Cyan des écrans : relevés d'instruments.
pub const SCREEN: Color = Color::srgb(0.45, 0.82, 0.90);
pub const SCREEN_DIM: Color = Color::srgba(0.45, 0.82, 0.90, 0.35);
/// Rouge d'alarme (coque au plus mal, impact).
pub const DANGER: Color = Color::srgb(0.92, 0.24, 0.18);
/// Acier des filets et des séparations.
pub const STEEL: Color = Color::srgba(0.42, 0.45, 0.49, 0.85);
pub const RULE: Color = Color::srgba(0.42, 0.45, 0.49, 0.45);
/// Boutons sans cadre image (petits boutons des options) : fond, survol, bordure.
pub const BUTTON: Color = Color::srgba(1.0, 1.0, 1.0, 0.04);
pub const BUTTON_HOVER: Color = Color::srgba(0.95, 0.66, 0.24, 0.22);
pub const BUTTON_BORDER: Color = Color::srgba(0.62, 0.66, 0.72, 0.28);

/// Chemins des polices (Inter, OFL ; DejaVu Sans Mono, licence Bitstream Vera et Arev ; voir
/// `assets/CREDITS.md`).
pub const FONT_REGULAR: &str = "fonts/Inter-Regular.ttf";
pub const FONT_TITLE: &str = "fonts/Inter-SemiBold.ttf";
pub const FONT_MONO: &str = "fonts/DejaVuSansMono.ttf";

/// Espacement des capitales des titres, en fraction de la taille du texte.
const HEADING_SPACING: f32 = 0.16;

/// Les cadres découpés en 9 tranches. Chaque image est dessinée au double de sa taille logique
/// (`tools/ui/frames.py`, `SCALE`) : nette à l'échelle 2, réduite de moitié à l'échelle 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    /// Grand panneau : coupes haut-gauche (ambre) et bas-droite, double filet, deux rivets.
    Panel,
    /// Bouton au repos : languette éteinte.
    Button,
    /// Bouton survolé : fond et filet ambre, languette allumée.
    ButtonHover,
    /// Relevé d'instrument (HUD) : verre sombre, équerres cyan aux quatre coins.
    Readout,
    /// Plaquette d'invite ou de message : fond discret, barre ambre à gauche.
    Plate,
}

/// Facteur de dessin des images de cadre (pixels d'image par pixel d'interface).
const FRAME_SCALE: f32 = 2.0;

impl Frame {
    fn path(self) -> &'static str {
        match self {
            Frame::Panel => "ui/panel.png",
            Frame::Button => "ui/button.png",
            Frame::ButtonHover => "ui/button_hover.png",
            Frame::Readout => "ui/readout.png",
            Frame::Plate => "ui/plate.png",
        }
    }

    /// Retrait intérieur minimal d'un nœud qui porte ce cadre (filets, coupes et voyants
    /// compris, plus un peu d'air), en pixels d'interface.
    pub fn inner(self) -> f32 {
        match self {
            Frame::Panel => 24.0,
            Frame::Button | Frame::ButtonHover => 12.0,
            Frame::Readout => 10.0,
            Frame::Plate => 8.0,
        }
    }

    /// Largeur des coins non étirés, en pixels d'interface (mêmes valeurs que `frames.py`).
    fn border(self) -> f32 {
        match self {
            Frame::Panel => 20.0,
            Frame::Button | Frame::ButtonHover => 10.0,
            Frame::Readout => 12.0,
            Frame::Plate => 6.0,
        }
    }
}

/// Les images des cadres, chargées une fois au démarrage.
#[derive(Clone, Debug)]
pub struct UiSkin {
    pub panel: Handle<Image>,
    pub button: Handle<Image>,
    pub button_hover: Handle<Image>,
    pub readout: Handle<Image>,
    pub plate: Handle<Image>,
}

impl UiSkin {
    pub fn load(asset_server: &AssetServer) -> Self {
        Self {
            panel: asset_server.load(Frame::Panel.path()),
            button: asset_server.load(Frame::Button.path()),
            button_hover: asset_server.load(Frame::ButtonHover.path()),
            readout: asset_server.load(Frame::Readout.path()),
            plate: asset_server.load(Frame::Plate.path()),
        }
    }

    fn handle(&self, frame: Frame) -> Handle<Image> {
        match frame {
            Frame::Panel => self.panel.clone(),
            Frame::Button => self.button.clone(),
            Frame::ButtonHover => self.button_hover.clone(),
            Frame::Readout => self.readout.clone(),
            Frame::Plate => self.plate.clone(),
        }
    }
}

/// Polices et cadres de l'interface. La police courante remplace la police par défaut de Bevy
/// (qui n'a pas les accents des noms canon) ; les polices de titre et à chasse fixe, et les
/// images des cadres, sont chargées à part.
#[derive(Resource, Clone)]
pub struct UiFonts {
    pub title: Handle<Font>,
    pub mono: Handle<Font>,
    pub skin: UiSkin,
}

impl UiFonts {
    pub fn title(&self, size: f32) -> TextFont {
        TextFont::from_font_size(FontSize::Px(size)).with_font(self.title.clone())
    }

    pub fn body(&self, size: f32) -> TextFont {
        TextFont::from_font_size(FontSize::Px(size))
    }

    /// Chasse fixe : relevés (vitesse, distances, munitions, argent), numéros de choix. Les
    /// chiffres gardent leur place quand la valeur change.
    pub fn mono(&self, size: f32) -> TextFont {
        TextFont::from_font_size(FontSize::Px(size)).with_font(self.mono.clone())
    }

    /// Police de titre et espacement des capitales (le texte se met en capitales par [`caps`]).
    pub fn heading(&self, size: f32) -> (TextFont, LetterSpacing) {
        (self.title(size), LetterSpacing::Px(size * HEADING_SPACING))
    }

    /// Un cadre découpé en 9 tranches, à poser sur un nœud (il est dessiné derrière ses enfants).
    /// Le nœud doit avoir un `padding` au moins égal à [`Frame::inner`] : le cadre couvre tout le
    /// nœud, les enfants restent à l'intérieur de ses filets.
    pub fn frame(&self, frame: Frame) -> ImageNode {
        ImageNode {
            image: self.skin.handle(frame),
            image_mode: NodeImageMode::Sliced(TextureSlicer {
                border: BorderRect::all(frame.border() * FRAME_SCALE),
                center_scale_mode: SliceScaleMode::Stretch,
                sides_scale_mode: SliceScaleMode::Stretch,
                // Coins à leur taille logique : l'image est dessinée au double.
                max_corner_scale: 1.0 / FRAME_SCALE,
            }),
            // Par défaut, une `ImageNode` se dessine dans la boîte de contenu, en retrait du
            // `padding` : le cadre collerait alors aux enfants. Il couvre tout le nœud.
            visual_box: VisualBox::BorderBox,
            ..default()
        }
    }
}

/// Un titre en capitales. `to_uppercase` garde les accents (« SYSTÈME »), comme l'exigent les
/// noms canon. À réserver aux libellés d'interface : un texte courant reste en casse normale.
pub fn caps(text: &str) -> String {
    text.to_uppercase()
}

/// Un titre d'écran ou de panneau : capitales espacées, police de titre.
pub fn heading(fonts: &UiFonts, text: &str, size: f32, color: Color) -> impl Bundle {
    (Text::new(caps(text)), fonts.heading(size), TextColor(color))
}

/// Une ligne de séparation : un court trait ambre, puis un filet d'acier.
pub fn rule(width: Val, margin: UiRect) -> impl Bundle {
    (
        Node {
            width,
            height: px(2),
            margin,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        },
        children![
            (
                Node {
                    width: px(36),
                    height: px(2),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(ACCENT),
            ),
            (
                Node {
                    flex_grow: 1.0,
                    height: px(1),
                    margin: UiRect::left(px(4)),
                    ..default()
                },
                BackgroundColor(RULE),
            ),
        ],
    )
}

/// Un filet fin d'une couleur donnée, sur toute la largeur du parent.
pub fn hairline(color: Color) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: px(1),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(color),
    )
}

/// Trois voyants décoratifs (ambre, cyan, éteint), en haut à droite d'un panneau : l'appareil
/// est sous tension. Purement visuels, sans texte.
pub fn status_lights() -> impl Bundle {
    let light = |color: Color| {
        (
            Node {
                width: px(6),
                height: px(6),
                ..default()
            },
            BackgroundColor(color),
        )
    };
    (
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(5),
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        },
        children![
            light(ACCENT),
            light(SCREEN),
            light(Color::srgba(0.42, 0.45, 0.49, 0.5)),
        ],
    )
}

/// Équerres aux quatre coins d'un parent positionné (écran plein, cadre) : le contour d'un
/// écran d'appareil. Nœuds absolus : ils ne dérangent pas la mise en page du parent.
pub fn spawn_corner_marks(
    parent: &mut ChildSpawnerCommands,
    inset: f32,
    length: f32,
    color: Color,
) {
    const THICK: f32 = 2.0;
    for (left, top) in [(true, true), (false, true), (true, false), (false, false)] {
        let mut horizontal = Node {
            position_type: PositionType::Absolute,
            width: px(length),
            height: px(THICK),
            ..default()
        };
        let mut vertical = Node {
            position_type: PositionType::Absolute,
            width: px(THICK),
            height: px(length),
            ..default()
        };
        for node in [&mut horizontal, &mut vertical] {
            if left {
                node.left = px(inset);
            } else {
                node.right = px(inset);
            }
            if top {
                node.top = px(inset);
            } else {
                node.bottom = px(inset);
            }
        }
        parent.spawn((horizontal, BackgroundColor(color)));
        parent.spawn((vertical, BackgroundColor(color)));
    }
}

/// Marque les boutons de menu (survol mis en évidence par `ui::menu`).
#[derive(Component, Default)]
pub struct MenuButton;

/// Un bouton de menu (menu principal, pause, chargement) portant son action `A`.
pub fn menu_button<A: Component>(
    fonts: &UiFonts,
    label: String,
    action: A,
    enabled: bool,
) -> impl Bundle {
    let text_color = if enabled { TEXT } else { TEXT_DISABLED };
    let mut frame = fonts.frame(Frame::Button);
    if !enabled {
        // Un bouton inactif s'efface avec son libellé.
        frame.color = Color::srgba(1.0, 1.0, 1.0, 0.45);
    }
    (
        MenuButton,
        bevy::ui_widgets::Button,
        bevy::ui_widgets::ActivateOnPress,
        action,
        Node {
            width: px(400),
            height: px(52),
            margin: UiRect::vertical(px(6)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(px(18)),
            ..default()
        },
        frame,
        children![(Text::new(label), fonts.body(22.0), TextColor(text_color))],
    )
}

/// Survol d'un bouton : un bouton à cadre change d'image, un bouton uni de fond. Partagé par
/// les menus et le terminal.
pub fn set_hovered(
    fonts: &UiFonts,
    hovered: bool,
    image: Option<Mut<ImageNode>>,
    background: Option<Mut<BackgroundColor>>,
) {
    if let Some(mut image) = image {
        let wanted = fonts.skin.handle(if hovered {
            Frame::ButtonHover
        } else {
            Frame::Button
        });
        if image.image != wanted {
            image.image = wanted;
        }
        return;
    }
    if let Some(mut background) = background {
        background.0 = if hovered { BUTTON_HOVER } else { BUTTON };
    }
}

/// Un écran plein, fond opaque en vignette, contenu centré en colonne.
pub fn fullscreen_backdrop() -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(BACKDROP_EDGE),
        BackgroundGradient::from(RadialGradient::new(
            UiPosition::CENTER,
            RadialGradientShape::FarthestCorner,
            vec![
                ColorStop::percent(BACKDROP, 0),
                ColorStop::percent(BACKDROP_EDGE, 100),
            ],
        )),
    )
}

/// La vignette des écrans pleins seule, d'une opacité donnée : une carte de chargement posée
/// sur le fondu au noir des transitions apparaît avec lui (`transition`).
pub fn backdrop_gradient(alpha: f32) -> BackgroundGradient {
    BackgroundGradient::from(RadialGradient::new(
        UiPosition::CENTER,
        RadialGradientShape::FarthestCorner,
        vec![
            ColorStop::percent(BACKDROP.with_alpha(alpha), 0),
            ColorStop::percent(BACKDROP_EDGE.with_alpha(alpha), 100),
        ],
    ))
}

/// Ombre portée courte d'un texte posé sur la vue 3D (date, étiquettes, repères) : lisible sur
/// un fond clair sans le flou ni le décalage de 4 px de l'ombre par défaut, qui dédoublait les
/// lettres.
pub fn label_shadow() -> TextShadow {
    TextShadow {
        offset: Vec2::splat(1.0),
        color: Color::srgba(0.0, 0.0, 0.0, 0.85),
    }
}
