// Build distribué : le jeu ne s'ouvre pas avec une console à côté (les journaux restent lisibles
// quand on le lance depuis un terminal qui redirige sa sortie).
#![cfg_attr(feature = "dist", windows_subsystem = "windows")]

use bevy::prelude::*;

fn main() -> AppExit {
    App::new().add_plugins(sh_game::StarHistoriesPlugin).run()
}
