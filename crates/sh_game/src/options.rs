//! Réglages du joueur (M7) : qualité graphique, plein écran, volumes, souris, touches. Ils vivent
//! dans `settings.ron` du dossier du joueur et s'appliquent aussitôt. Quand l'autopilote pilote,
//! le fichier n'est ni lu ni écrit : mesures et scénarios restent reproductibles.
//!
//! Remappage **par touche** : chaque touche d'origine reçoit une touche choisie, dans tous les
//! contextes où elle sert (E interagit à pied et fait rouler aux commandes). Les aides citent la
//! touche d'origine (`[KeyE]`) et affichent la touche choisie (`Lang::remaps`).

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use bevy_enhanced_input::prelude::Binding;
use serde::{Deserialize, Serialize};
use sh_data::PresetLevel;

use crate::data::{Data, Texts};
use crate::graphics::GraphicsSettings;

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(load()).add_systems(
            PostUpdate,
            (
                tag_default_keys,
                apply_keys,
                apply_window,
                apply_graphics,
                persist,
            )
                .chain(),
        );
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
    pub preset: PresetLevel,
    pub fullscreen: bool,
    /// Volumes de 0 à 1.
    pub master_volume: f32,
    pub music_volume: f32,
    pub effects_volume: f32,
    /// Multiplie la sensibilité de la souris (à pied et aux commandes).
    pub mouse_sensitivity: f32,
    pub invert_y: bool,
    /// Conseils de première fois (tutoriel léger).
    pub tips: bool,
    /// Touche d'origine → touche choisie (noms `KeyCode` : `KeyE` → `KeyF`).
    pub keys: BTreeMap<String, String>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            preset: PresetLevel::Medium,
            fullscreen: false,
            master_volume: 0.8,
            music_volume: 0.6,
            effects_volume: 0.8,
            mouse_sensitivity: 1.0,
            invert_y: false,
            tips: true,
            keys: BTreeMap::new(),
        }
    }
}

impl UserSettings {
    /// Échelle du mouvement de la souris : sensibilité, et axe vertical inversé au besoin.
    pub fn look_scale(&self) -> Vec2 {
        let s = self.mouse_sensitivity.clamp(0.1, 5.0);
        Vec2::new(s, if self.invert_y { -s } else { s })
    }

    /// La touche choisie à la place de la touche d'origine `default`.
    pub fn key_for(&self, default: KeyCode) -> KeyCode {
        self.keys
            .get(&key_name(default))
            .and_then(|name| parse_key(name))
            .unwrap_or(default)
    }

    /// Donne `key` à l'action de la touche d'origine `default`. Si une autre touche d'origine
    /// l'avait déjà, elles échangent leurs touches : aucune action ne reste sans touche.
    pub fn assign(&mut self, default: KeyCode, key: KeyCode) {
        let previous = self.key_for(default);
        if let Some(other) = REMAPPABLE
            .iter()
            .copied()
            .find(|d| *d != default && self.key_for(*d) == key)
        {
            self.set(other, previous);
        }
        self.set(default, key);
    }

    fn set(&mut self, default: KeyCode, key: KeyCode) {
        if key == default {
            self.keys.remove(&key_name(default));
        } else {
            self.keys.insert(key_name(default), key_name(key));
        }
    }
}

/// Les touches d'origine que le joueur peut remapper (Échap, Entrée et les flèches des menus
/// restent fixes).
pub const REMAPPABLE: [KeyCode; 22] = [
    KeyCode::KeyW,
    KeyCode::KeyS,
    KeyCode::KeyA,
    KeyCode::KeyD,
    KeyCode::Space,
    KeyCode::ControlLeft,
    KeyCode::ShiftLeft,
    KeyCode::KeyE,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyF,
    KeyCode::KeyZ,
    KeyCode::KeyG,
    KeyCode::KeyX,
    KeyCode::KeyM,
    KeyCode::KeyC,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::F5,
    KeyCode::F9,
];

/// Les touches qu'on peut choisir.
const CHOOSABLE: [KeyCode; 70] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
    KeyCode::Space,
    KeyCode::ShiftLeft,
    KeyCode::ShiftRight,
    KeyCode::ControlLeft,
    KeyCode::ControlRight,
    KeyCode::AltLeft,
    KeyCode::Tab,
    KeyCode::CapsLock,
    KeyCode::Backquote,
    KeyCode::Minus,
    KeyCode::Equal,
    KeyCode::BracketLeft,
    KeyCode::BracketRight,
    KeyCode::Semicolon,
    KeyCode::Quote,
    KeyCode::Comma,
    KeyCode::Period,
    KeyCode::Slash,
    KeyCode::Backslash,
    KeyCode::IntlBackslash,
    KeyCode::Insert,
    KeyCode::Delete,
];

/// Nom stable d'une touche (`KeyE`), celui des jetons de texte et du fichier de réglages.
pub fn key_name(key: KeyCode) -> String {
    format!("{key:?}")
}

pub fn parse_key(name: &str) -> Option<KeyCode> {
    CHOOSABLE.iter().copied().find(|k| key_name(*k) == name)
}

/// Une touche peut-elle être choisie ?
pub fn choosable(key: KeyCode) -> bool {
    CHOOSABLE.contains(&key)
}

fn settings_path() -> std::path::PathBuf {
    crate::save::user_dir().join("settings.ron")
}

fn load() -> UserSettings {
    if crate::dev::autopilot_active() {
        // `SH_KEYS=KeyE:KeyF,…` : un remappage imposé, pour le vérifier. L'autopilote est muet
        // (il tourne pendant que l'auteur travaille), sauf `SH_AUDIO=1`, et sans conseils
        // (captures propres), sauf `SH_TIPS=1`.
        let defaults = UserSettings::default();
        let mut settings = UserSettings {
            tips: std::env::var_os("SH_TIPS").is_some(),
            master_volume: if std::env::var_os("SH_AUDIO").is_some() {
                defaults.master_volume
            } else {
                0.0
            },
            ..defaults
        };
        for pair in std::env::var("SH_KEYS").unwrap_or_default().split(',') {
            if let Some((from, to)) = pair.split_once(':')
                && let (Some(from), Some(to)) = (parse_key(from.trim()), parse_key(to.trim()))
            {
                settings.assign(from, to);
            }
        }
        return settings;
    }
    let path = settings_path();
    if !path.exists() {
        return UserSettings::default();
    }
    match sh_data::read_ron(&path) {
        Ok(settings) => settings,
        Err(err) => {
            warn!("réglages illisibles, valeurs par défaut : {err}");
            UserSettings::default()
        }
    }
}

/// Écrit les réglages à chaque changement (pas au démarrage).
fn persist(settings: Res<UserSettings>) {
    if !settings.is_changed() || settings.is_added() || crate::dev::autopilot_active() {
        return;
    }
    if let Err(err) = sh_data::write_ron(&settings_path(), &*settings) {
        error!("réglages non enregistrés : {err}");
    }
}

/// La touche d'origine d'une liaison clavier, avant tout remappage.
#[derive(Component, Clone, Copy, Debug)]
pub struct DefaultKey(pub KeyCode);

fn tag_default_keys(
    mut commands: Commands,
    bindings: Query<(Entity, &Binding), (Added<Binding>, Without<DefaultKey>)>,
) {
    for (entity, binding) in &bindings {
        if let Binding::Keyboard { key, .. } = binding
            && REMAPPABLE.contains(key)
        {
            commands.entity(entity).try_insert(DefaultKey(*key));
        }
    }
}

/// Applique les touches choisies aux liaisons et aux étiquettes des aides.
fn apply_keys(
    mut commands: Commands,
    settings: Res<UserSettings>,
    texts: Option<ResMut<Texts>>,
    bindings: Query<(Entity, &Binding, &DefaultKey)>,
    added: Query<(), Added<DefaultKey>>,
) {
    if !settings.is_changed() && added.is_empty() {
        return;
    }
    for (entity, binding, default) in &bindings {
        let wanted = settings.key_for(default.0);
        if let Binding::Keyboard { key, mod_keys } = *binding
            && key != wanted
        {
            info!("touche : {:?} → {wanted:?}", default.0);
            commands.entity(entity).try_insert(Binding::Keyboard {
                key: wanted,
                mod_keys,
            });
        }
    }
    if let Some(mut texts) = texts {
        texts.0.remaps = REMAPPABLE
            .iter()
            .filter(|d| settings.key_for(**d) != **d)
            .map(|d| (key_name(*d), key_name(settings.key_for(*d))))
            .collect();
    }
}

fn apply_window(settings: Res<UserSettings>, mut window: Single<&mut Window, With<PrimaryWindow>>) {
    // `SH_WINDOW` décide pour l'autopilote.
    if !settings.is_changed() || std::env::var_os("SH_WINDOW").is_some() {
        return;
    }
    window.mode = if settings.fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
}

fn apply_graphics(
    settings: Res<UserSettings>,
    data: Option<Res<Data>>,
    graphics: Option<ResMut<GraphicsSettings>>,
) {
    // `SH_PRESET` décide pour l'autopilote.
    let (Some(data), Some(mut graphics)) = (data, graphics) else {
        return;
    };
    if !settings.is_changed() || std::env::var_os("SH_PRESET").is_some() {
        return;
    }
    if graphics.level != settings.preset {
        graphics.level = settings.preset;
        graphics.preset = data.graphics.get(settings.preset).clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_des_touches() {
        for key in REMAPPABLE {
            assert_eq!(parse_key(&key_name(key)), Some(key), "{key:?}");
        }
    }

    #[test]
    fn deux_actions_ne_partagent_jamais_une_touche() {
        let mut s = UserSettings::default();
        // E (interagir) prend F (se lever) : F reçoit E.
        s.assign(KeyCode::KeyE, KeyCode::KeyF);
        assert_eq!(s.key_for(KeyCode::KeyE), KeyCode::KeyF);
        assert_eq!(s.key_for(KeyCode::KeyF), KeyCode::KeyE);
        // Retour à l'origine : plus rien dans le fichier.
        s.assign(KeyCode::KeyE, KeyCode::KeyE);
        assert_eq!(s.key_for(KeyCode::KeyF), KeyCode::KeyF);
        assert!(s.keys.is_empty());
        // Une touche libre : pas d'échange.
        s.assign(KeyCode::KeyJ, KeyCode::KeyL);
        assert_eq!(s.keys.len(), 1);
    }
}
