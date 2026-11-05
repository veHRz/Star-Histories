//! PNJ de comptoir (M9, docs/m9-retours.md §3.4) : personnages anonymes et immobiles derrière les
//! comptoirs du spatioport, à qui l'on parle pour les services des bornes.
//!
//! Le lot L3 y placera les PNJ : un `SOCKET_<nom>_npc` du glTF (extra `usage: "npc"`, paramètres
//! `role` et `variant`) reçoit le modèle `characters/clerk_<variant>`, regard vers +Y Blender ;
//! son `INTERACT_<nom>` de genre `npc` couvre le PNJ et le comptoir et envoie
//! `items::TalkTo { role, variant }`, que la conversation (L4) traite. Contenu générique : aucun
//! PNJ nommé, aucun insigne (règle d'or n° 2). Pour l'instant, un plugin vide déjà branché.

use bevy::prelude::*;

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, _app: &mut App) {}
}
