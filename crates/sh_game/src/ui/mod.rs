//! Interface : thème (polices, cadres « futur usé »), caméra d'interface, menus, HUD.

use bevy::asset::AssetId;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::ui::IsDefaultUiCamera;

use crate::data::assets_dir;

pub mod character;
pub mod combat_hud;
pub mod flight_hud;
pub mod hud;
pub mod keys_panel;
pub mod menu;
pub mod options;
pub mod pause;
pub mod terminal;
pub mod theme;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            menu::MenuPlugin,
            hud::HudPlugin,
            flight_hud::FlightHudPlugin,
            combat_hud::CombatHudPlugin,
            terminal::TerminalPlugin,
            pause::PausePlugin,
            options::OptionsUiPlugin,
            // M9 : encart des touches (L2), écran du personnage (L5).
            keys_panel::KeysPanelPlugin,
            character::CharacterScreenPlugin,
        ))
        .add_systems(Startup, (setup_fonts, spawn_ui_camera))
        .add_systems(PreUpdate, learn_key_labels);
    }
}

/// Apprend la disposition réelle du clavier : chaque touche lettre pressée donne son vrai
/// caractère, qui devient l'étiquette affichée dans les aides (Q13).
fn learn_key_labels(
    mut reader: MessageReader<bevy::input::keyboard::KeyboardInput>,
    texts: Option<ResMut<crate::data::Texts>>,
) {
    use bevy::input::keyboard::Key;
    let Some(mut texts) = texts else {
        reader.clear();
        return;
    };
    for input in reader.read() {
        let Key::Character(c) = &input.logical_key else {
            continue;
        };
        let code = format!("{:?}", input.key_code);
        let letter_key = code.starts_with("Key");
        let alphabetic = c.chars().all(char::is_alphabetic);
        if c.chars().count() != 1 || !(letter_key || alphabetic) {
            continue;
        }
        let label = c.to_uppercase();
        // Lecture d'abord : ne marquer `Texts` modifié que si l'étiquette change. On compare
        // l'étiquette apprise elle-même, pas `key_label` : celle-ci passe par les remappages, et
        // une touche remappée paraîtrait changer à chaque appui (tout se redessinerait).
        if texts.keys.get(&code).map(String::as_str) != Some(label.as_str()) {
            texts.0.set_key(&code, &label);
        }
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
        mono: asset_server.load(theme::FONT_MONO),
        skin: theme::UiSkin::load(&asset_server),
    });
}
