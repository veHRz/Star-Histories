//! PNJ de comptoir (M9, docs/m9-retours.md §3.4) : personnages anonymes et immobiles derrière les
//! comptoirs du spatioport, à qui l'on parle pour les services des bornes.
//!
//! Un `SOCKET_<nom>_npc` du glTF (extra `usage: "npc"`, paramètres `role` et `variant`) devient
//! une place de PNJ ([`CounterNpc`], posée par `sector::gltf`) ; ce module y fait apparaître le
//! modèle `models/characters/clerk_<variant>.gltf` ([`clerk_model_path`]), enfant du point
//! d'attache : origine au centre de la base, regard selon le socket (+Y Blender, −Z dans le jeu).
//! Un modèle absent (les variantes arrivent au fil de l'eau) donne un avertissement, une fois par
//! fichier, et la place reste vide. Le PNJ ne bouge pas et n'a pas de collision : le comptoir
//! arrête le joueur. Son `INTERACT_<nom>` de genre `npc` couvre le PNJ et le comptoir ; viser et
//! appuyer sur la touche d'interaction envoie `items::TalkTo { role, variant }`
//! (`interaction`), que la conversation traite. Contenu générique : aucun PNJ nommé, aucun
//! insigne (règle d'or n° 2).

use std::collections::BTreeSet;

use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;

use crate::data::assets_dir;
use crate::sector::ActiveSector;

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_clerks.run_if(resource_exists::<ActiveSector>));
    }
}

/// La place d'un PNJ de comptoir : son rôle (`contracts`, `bank`, `market`, `shipyard`, `bar`…)
/// et la variante de son modèle (`a`, `b`, `c`).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct CounterNpc {
    pub role: String,
    pub variant: String,
}

/// Le modèle d'un PNJ de comptoir, enfant de sa place.
#[derive(Component)]
pub struct ClerkModel;

/// Le modèle d'une variante de PNJ de comptoir, relatif à `assets/`.
pub fn clerk_model_path(variant: &str) -> String {
    format!("models/characters/clerk_{variant}.gltf")
}

/// Fait apparaître le PNJ des places nouvellement instanciées (une cellule qui se charge).
fn spawn_clerks(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    spots: Query<(Entity, &CounterNpc), Added<CounterNpc>>,
    mut missing: Local<BTreeSet<String>>,
) {
    for (spot, npc) in &spots {
        let path = clerk_model_path(&npc.variant);
        if !assets_dir().join(&path).is_file() {
            if missing.insert(path.clone()) {
                warn!(
                    "PNJ de comptoir « {} » : modèle {path} absent, place laissée vide",
                    npc.role
                );
            }
            continue;
        }
        commands.spawn((
            Name::new(format!("PNJ de comptoir {} ({})", npc.role, npc.variant)),
            ClerkModel,
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))),
            Transform::default(),
            ChildOf(spot),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemin_du_modele() {
        assert_eq!(clerk_model_path("b"), "models/characters/clerk_b.gltf");
    }
}
