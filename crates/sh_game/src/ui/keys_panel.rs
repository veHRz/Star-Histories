//! Encart des touches (M9, lot L2) : à gauche de l'écran, la liste des actions possibles et de
//! leurs touches dans le contexte courant (à pied, aux commandes, carte), repliable par
//! [`ToggleKeyHelp`] (F1 par défaut). Composant réutilisable : chaque vue ou écran lui donne ses
//! lignes, les étiquettes des touches suivent la disposition du clavier et le remappage
//! (`Lang::key_label`). Le bandeau du haut garde le lieu et la salle.
//!
//! Pour l'instant : l'état affiché ou replié ([`KeyHelpVisible`], forcé par l'autopilote
//! `SH_KEYHELP=0|1`) et sa bascule ; l'encart lui-même viendra avec L2.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::input::ToggleKeyHelp;

pub struct KeysPanelPlugin;

impl Plugin for KeysPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyHelpVisible>()
            .add_observer(on_toggle_key_help);
    }
}

/// L'encart des touches est-il déplié ? Oui par défaut : un nouveau joueur voit les touches.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyHelpVisible(pub bool);

impl Default for KeyHelpVisible {
    fn default() -> Self {
        KeyHelpVisible(true)
    }
}

fn on_toggle_key_help(_: On<Start<ToggleKeyHelp>>, mut visible: ResMut<KeyHelpVisible>) {
    visible.0 = !visible.0;
}
