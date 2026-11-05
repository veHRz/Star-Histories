//! Écran du personnage (M9, lot L5) : deux onglets, ouverts par `items::OpenCharacterScreen`
//! (touches physiques `KeyI` et `KeyO` par défaut, fiche du terminal, autopilote `SH_SCREEN`).
//!
//! - **Inventaire** : objets portés, à icônes (`ItemDef::icon_path`), équipement par emplacement
//!   (`items::Equipment`), récapitulatif des réputations et des emprunts ; à côté d'un casier
//!   ouvert (`items::OpenContainer`), le rangement entre le casier et soi (`items::Containers`).
//! - **Compétences** : niveau de chaque compétence et barre d'expérience vers le suivant.
//!
//! Textes dans `lang/<code>/character.ron`. Pour l'instant, un plugin vide déjà branché.

use bevy::prelude::*;

pub struct CharacterScreenPlugin;

impl Plugin for CharacterScreenPlugin {
    fn build(&self, _app: &mut App) {}
}
