//! Plugins Bevy de Star Histories.
//!
//! [`StarHistoriesPlugin`] agrège un plugin par domaine. L'ordre entre domaines est explicite :
//! les états et le chargement d'abord, puis l'interface, les entrées, les vues et le rendu.
//! La logique pure vit dans `sh_core`, `sh_data` et `sh_sim` ; ce crate ne fait que l'envelopper.

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};

pub mod data;
pub mod dev;
#[cfg(feature = "dev")]
mod dev_tools;
pub mod graphics;
pub mod input;
pub mod loading;
pub mod state;
pub mod ui;
pub mod views;

pub struct StarHistoriesPlugin;

impl Plugin for StarHistoriesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Star Histories".into(),
                resolution: WindowResolution::new(1600, 900),
                // Pas de synchronisation verticale forcée : les mesures de l'autopilote doivent
                // refléter le vrai coût d'une frame. La vsync deviendra une option (M7).
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            state::StatePlugin,
            loading::LoadingPlugin,
            ui::UiPlugin,
            input::InputPlugin,
            views::ViewsPlugin,
            graphics::GraphicsPlugin,
            dev::AutopilotPlugin,
        ));
        #[cfg(feature = "dev")]
        app.add_plugins(dev_tools::DevToolsPlugin);
    }
}
