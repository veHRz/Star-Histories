//! Outils de développement (feature `dev` uniquement, jamais dans un build distribué).
//!
//! - **F1** : inspecteur du monde (entités, ressources, assets) ;
//! - **F2** : caméra libre sur la caméra du monde (ZQSD/WASD, clic droit pour regarder) ;
//! - **F3** : affichage FPS et temps de frame (masqué par défaut pendant une capture de
//!   l'autopilote, pour des images propres ; `SH_SHOW_FPS=1` le force).

use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig};
use bevy::input::common_conditions::{input_just_pressed, input_toggle_active};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::graphics::WorldCamera;
use crate::state::ControlMode;

pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        // Capture ou scénario (qui prend ses propres captures) : images sans compteur.
        let capturing = std::env::var_os("SH_SCREENSHOT").is_some()
            || std::env::var_os("SH_SCENARIO").is_some();
        let show_fps = std::env::var("SH_SHOW_FPS").is_ok_and(|v| v == "1") || !capturing;
        app.add_plugins((
            EguiPlugin::default(),
            WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::F1)),
            FreeCameraPlugin,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextFont::from_font_size(FontSize::Px(14.0)),
                    enabled: show_fps,
                    frame_time_graph_config: FrameTimeGraphConfig {
                        enabled: show_fps,
                        // Budgets d'AGENTS.md §10 : 30 i/s minimum, 60 i/s visés.
                        min_fps: 30.0,
                        target_fps: 60.0,
                    },
                    ..default()
                },
            },
        ))
        .add_systems(
            Update,
            (
                toggle_free_camera.run_if(input_just_pressed(KeyCode::F2)),
                toggle_fps.run_if(input_just_pressed(KeyCode::F3)),
            ),
        );
    }
}

/// Mode de commande à rétablir en quittant la caméra libre.
#[derive(Component)]
struct FreeCameraWas(ControlMode);

fn toggle_free_camera(
    mut commands: Commands,
    cameras: Query<(Entity, Option<&FreeCameraWas>), With<WorldCamera>>,
    mode: Res<State<ControlMode>>,
    mut next_mode: ResMut<NextState<ControlMode>>,
) {
    for (entity, was) in &cameras {
        match was {
            Some(FreeCameraWas(previous)) => {
                commands
                    .entity(entity)
                    .remove::<(FreeCamera, FreeCameraWas)>();
                next_mode.set(*previous);
            }
            None => {
                // Les commandes de jeu sont coupées : la caméra libre lit le clavier elle-même.
                commands
                    .entity(entity)
                    .insert((FreeCamera::default(), FreeCameraWas(*mode.get())));
                next_mode.set(ControlMode::Menu);
            }
        }
    }
}

fn toggle_fps(mut config: ResMut<FpsOverlayConfig>) {
    config.enabled = !config.enabled;
    config.frame_time_graph_config.enabled = config.enabled;
}
