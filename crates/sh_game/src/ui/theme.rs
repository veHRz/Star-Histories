//! Identité de l'interface : polices, couleurs, gabarits communs.
//!
//! Direction artistique validée (« réaliste stylisé, géométrie simple et lisible ») : une
//! interface sobre, sombre, à accent unique. Valeurs provisoires, à affiner avec l'auteur.

use bevy::prelude::*;

pub const BACKDROP: Color = Color::srgb(0.035, 0.04, 0.055);
pub const PANEL: Color = Color::srgba(0.06, 0.07, 0.09, 0.92);
pub const TEXT: Color = Color::srgb(0.90, 0.91, 0.93);
pub const TEXT_DIM: Color = Color::srgb(0.55, 0.58, 0.63);
pub const TEXT_DISABLED: Color = Color::srgb(0.33, 0.35, 0.38);
pub const ACCENT: Color = Color::srgb(0.85, 0.63, 0.30);
pub const BUTTON: Color = Color::srgba(1.0, 1.0, 1.0, 0.04);
pub const BUTTON_HOVER: Color = Color::srgba(0.85, 0.63, 0.30, 0.22);
pub const BUTTON_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.10);

/// Chemins des polices (Inter, OFL ; voir `assets/CREDITS.md`).
pub const FONT_REGULAR: &str = "fonts/Inter-Regular.ttf";
pub const FONT_TITLE: &str = "fonts/Inter-SemiBold.ttf";

/// Polices de l'interface. La police courante remplace la police par défaut de Bevy (qui n'a
/// pas les accents des noms canon) ; la police de titre est chargée à part.
#[derive(Resource, Clone)]
pub struct UiFonts {
    pub title: Handle<Font>,
}

impl UiFonts {
    pub fn title(&self, size: f32) -> TextFont {
        TextFont::from_font_size(FontSize::Px(size)).with_font(self.title.clone())
    }

    pub fn body(&self, size: f32) -> TextFont {
        TextFont::from_font_size(FontSize::Px(size))
    }
}

/// Un écran plein, fond opaque, contenu centré en colonne.
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
        BackgroundColor(BACKDROP),
    )
}
