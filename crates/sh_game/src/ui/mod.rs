//! Interface : thème, caméra d'interface, menus.

use bevy::asset::AssetId;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::ui::IsDefaultUiCamera;

use crate::data::assets_dir;

pub mod hud;
pub mod menu;
pub mod theme;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((menu::MenuPlugin, hud::HudPlugin))
            .add_systems(Startup, (setup_fonts, spawn_ui_camera));
    }
}

/// Couche de rendu réservée à l'interface : la caméra d'interface ne dessine aucune 3D.
pub const UI_LAYER: usize = 31;

/// La caméra qui dessine l'interface, au-dessus de la caméra du monde (`graphics`).
#[derive(Component)]
pub struct UiCamera;

fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Caméra d'interface"),
        UiCamera,
        IsDefaultUiCamera,
        Camera2d,
        Camera {
            order: 10,
            // Efface la fenêtre tant qu'aucune caméra du monde n'y dessine ; dès qu'une vue 3D
            // rend dans la fenêtre, `graphics` passe l'interface en mélange par-dessus.
            clear_color: ClearColorConfig::Default,
            ..default()
        },
        RenderLayers::layer(UI_LAYER),
    ));
}

fn setup_fonts(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    asset_server: Res<AssetServer>,
) {
    // Remplace la police par défaut (identifiant par défaut) par Inter : tout texte sans police
    // explicite, y compris celui des outils de Bevy, gagne les accents.
    let path = assets_dir().join(theme::FONT_REGULAR);
    match std::fs::read(&path) {
        Ok(bytes) => {
            if let Err(err) = fonts.insert(AssetId::default(), Font::from_bytes(bytes)) {
                error!("police par défaut non remplacée : {err}");
            }
        }
        Err(err) => error!("police {} illisible : {err}", path.display()),
    }
    commands.insert_resource(theme::UiFonts {
        title: asset_server.load(theme::FONT_TITLE),
    });
}
