//! Réglages du joueur (M7, refaits en M9) : affichage, son, souris, touches, aides. Ils vivent
//! dans `settings.ron` du dossier du joueur et s'appliquent aussitôt. Quand l'autopilote pilote,
//! le fichier n'est ni lu ni écrit : mesures et scénarios restent reproductibles.
//!
//! **Touches, par action** (M9) : chaque action de [`KEY_ACTIONS`] a deux cases, principale et
//! secondaire ([`KeySlots`]) ; le fichier ne garde que celles qui diffèrent de l'origine.
//! Assigner une touche ne change aucune autre action. Si la touche sert déjà à une action active
//! en même temps (un contexte d'entrée en commun : [`UserSettings::conflicts`]), l'écran des
//! options le dit et le joueur corrige. À pied et aux commandes, jamais actifs ensemble, peuvent
//! partager une touche (Z avance à pied et pousse en vol).
//!
//! Pourquoi : le remappage de M7 était **par touche** et échangeait deux touches en silence, et
//! l'écran affichait la touche choisie à travers `Lang::key_label`, qui applique déjà le
//! remappage. Chaque échange s'affichait donc à l'envers (E → F montrait E et F inchangés, une
//! touche libre prise ensuite montrait celle d'une autre ligne) : l'écran semblait échanger des
//! touches au hasard. Les cases passent désormais par [`physical_label`].
//!
//! Les aides citent la touche d'origine (`[KeyE]`) et affichent la touche choisie
//! (`Lang::remaps`, [`UserSettings::token_remaps`]) ; quand les actions d'une même touche
//! d'origine ont divergé (Interagir à pied, Roulis aux commandes), celle du contexte courant
//! l'emporte.
//!
//! Les anciens fichiers (M7, `keys: { "KeyE": "KeyF" }`) sont relus : une touche d'origine
//! remappée l'est pour toutes ses actions, comme avant ([`UserSettings::migrate`]).
//!
//! Sensibilités (M9) : un modificateur `Scale` sur l'action [`Look`] (à pied) et un autre sur
//! [`Steer`] (aux commandes : base de `gameplay/flight.ron`, puis le réglage du joueur). Les
//! systèmes de jeu ne lisent plus que l'inversion de l'axe ([`UserSettings::look_scale`]).
//!
//! Limite d'images, synchronisation verticale et compteur d'images (M11) : réglés ici, appliqués
//! par `frame_pace` (qui laisse `SH_FPS_LIMIT` et `SH_VSYNC` décider pour l'autopilote).
//! Sans fichier de réglages (première partie), le préréglage suit le GPU : Bas sur un GPU
//! intégré, Moyen ailleurs ([`default_preset_for_gpu`]).
//!
//! La résolution dynamique de la vague 3 de M11 a été retirée (décision de l'auteur : la qualité
//! Haute devenait floue). Un fichier qui contient encore `dynamic_resolution` se relit : serde
//! ignore les champs inconnus (pas de `deny_unknown_fields`, voir `ancien_champ_ignore`).

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use bevy_enhanced_input::prelude::{
    Action, ActionOf, Binding, BindingOf, Negate, Scale, SwizzleAxis,
};
use serde::{Deserialize, Serialize};
use sh_data::{Lang, PresetLevel};

use crate::data::{Data, Texts};
use crate::graphics::GraphicsSettings;
use crate::input::{
    CTX_FOOT, CTX_MAP, CTX_PILOTING, KEY_ACTIONS, KeyAction, Look, MapControls, OnFoot, Piloting,
    Steer, context_of, key_action_index,
};
use crate::state::ControlMode;
use crate::ui::keys_panel::KeyHelpVisible;

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
    fn build(&self, app: &mut App) {
        let (settings, first_launch) = load();
        if first_launch {
            app.insert_resource(FirstLaunch);
        }
        app.insert_resource(settings)
            .insert_resource(SteerBase(read_steer_base()))
            // Au premier frame, l'adaptateur graphique est connu (`RenderPlugin::finish`) ; le
            // préréglage n'est lu qu'ensuite (chargement des textures, `graphics`).
            .add_systems(Startup, preset_for_gpu)
            .add_systems(
                PostUpdate,
                (
                    tag_bindings,
                    apply_keys,
                    apply_token_remaps,
                    apply_mouse,
                    apply_layout,
                    sync_key_help,
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
    /// Sensibilité de la souris à pied (multiplicateur du regard).
    pub mouse_sensitivity: f32,
    /// Sensibilité de la direction aux commandes (M9) : multiplicateur posé sur la base de
    /// `gameplay/flight.ron` ([`SteerBase`]).
    pub piloting_sensitivity: f32,
    /// Axe vertical inversé, à pied et aux commandes.
    pub invert_y: bool,
    /// Conseils de première fois (tutoriel léger).
    pub tips: bool,
    /// Tutoriel guidé des premières minutes (M8) ; activé par défaut.
    pub tutorial: bool,
    /// Disposition du clavier des étiquettes (`gameplay/keyboard.ron`) ; vide ou inconnue :
    /// celle des données.
    pub keyboard_layout: String,
    /// Encart des touches déplié (`ui::keys_panel`), suivi de la touche qui le replie.
    pub key_help: bool,
    /// Touches choisies, par action (`foot_interact`) ; une action absente garde sa touche
    /// d'origine.
    pub bindings: BTreeMap<String, KeySlots>,
    /// Ancien format (M7) : touche d'origine → touche choisie. Relu et converti au chargement
    /// ([`UserSettings::migrate`]), jamais réécrit.
    #[serde(rename = "keys", skip_serializing_if = "BTreeMap::is_empty")]
    pub legacy_keys: BTreeMap<String, String>,
    /// Limite d'images par seconde (M11), appliquée par `frame_pace` ; 0 : illimitée, choisie
    /// par le joueur et gardée telle quelle. Les choix de l'option : `frame_pace::FPS_LIMITS`.
    /// Un fichier plus ancien n'a pas ces trois champs : ils prennent leur valeur par défaut, ici
    /// [`DEFAULT_FPS_LIMIT`].
    #[serde(default = "default_fps_limit")]
    pub fps_limit: u32,
    /// Synchronisation verticale (M11).
    pub vsync: bool,
    /// Compteur d'images par seconde dans le coin de l'écran (M11).
    pub show_fps: bool,
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
            piloting_sensitivity: 1.0,
            invert_y: false,
            tips: true,
            tutorial: true,
            keyboard_layout: String::new(),
            key_help: true,
            bindings: BTreeMap::new(),
            legacy_keys: BTreeMap::new(),
            fps_limit: DEFAULT_FPS_LIMIT,
            vsync: false,
            show_fps: false,
        }
    }
}

/// Le préréglage d'une première partie, selon le type de l'adaptateur graphique (nom de la
/// variante `wgpu::DeviceType`, que Bevy ne réexporte pas) : **Bas** sur un GPU intégré, la
/// cible de la machine de référence (AGENTS.md §10, Q15) ; Moyen ailleurs.
pub fn default_preset_for_gpu(device_type: &str) -> PresetLevel {
    if device_type == "IntegratedGpu" {
        PresetLevel::Low
    } else {
        PresetLevel::Medium
    }
}

/// Aucun fichier de réglages au lancement : le préréglage suivra le GPU ([`preset_for_gpu`]).
#[derive(Resource)]
struct FirstLaunch;

/// Première partie : le préréglage suit le GPU. Le fichier n'est pas écrit pour autant (il ne
/// l'est qu'au premier réglage changé) : tant que le joueur n'a rien choisi, chaque lancement
/// refait ce choix. Un fichier existant garde le sien ; l'autopilote n'est jamais concerné.
fn preset_for_gpu(
    mut commands: Commands,
    first: Option<Res<FirstLaunch>>,
    adapter: Option<Res<RenderAdapterInfo>>,
    mut settings: ResMut<UserSettings>,
) {
    if first.is_none() {
        return;
    }
    commands.remove_resource::<FirstLaunch>();
    let Some(adapter) = adapter else {
        return;
    };
    let kind = format!("{:?}", adapter.device_type);
    let preset = default_preset_for_gpu(&kind);
    info!(
        "premier lancement : préréglage {preset:?} pour {} ({kind})",
        adapter.name
    );
    if settings.preset != preset {
        settings.preset = preset;
    }
}

/// Limite d'images du joueur par défaut (M11, décision réversible de l'orchestrateur) : la
/// cible de la machine de référence. Sans limite, ses images alternaient 2,5 ms et 20,8 ms
/// (écart-type 7,3 ms) : les « chutes une fois sur deux » de l'auteur ; à 60 i/s, 16,67 ms
/// réguliers (écart-type 0,09 ms). L'autopilote garde « illimitée » (`autopilot_settings`).
pub const DEFAULT_FPS_LIMIT: u32 = 60;

fn default_fps_limit() -> u32 {
    DEFAULT_FPS_LIMIT
}

/// Une des deux cases d'une action.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Slot {
    #[default]
    Primary,
    Secondary,
}

impl Slot {
    pub const BOTH: [Slot; 2] = [Slot::Primary, Slot::Secondary];

    pub fn other(self) -> Slot {
        match self {
            Slot::Primary => Slot::Secondary,
            Slot::Secondary => Slot::Primary,
        }
    }
}

/// Les touches d'une action : principale et secondaire, chacune vide ou non. Écrites par leur
/// nom (`(primary: "KeyF", secondary: "")`, chaîne vide : case vide).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "SlotsFile", into = "SlotsFile")]
pub struct KeySlots {
    pub primary: Option<KeyCode>,
    pub secondary: Option<KeyCode>,
}

impl KeySlots {
    /// Les touches d'origine d'une action : la sienne en principale, rien en secondaire.
    pub fn default_for(action: &KeyAction) -> Self {
        Self {
            primary: Some(action.default),
            secondary: None,
        }
    }

    pub fn get(self, slot: Slot) -> Option<KeyCode> {
        match slot {
            Slot::Primary => self.primary,
            Slot::Secondary => self.secondary,
        }
    }

    fn set(&mut self, slot: Slot, key: Option<KeyCode>) {
        match slot {
            Slot::Primary => self.primary = key,
            Slot::Secondary => self.secondary = key,
        }
    }

    /// Les touches assignées, la principale d'abord.
    pub fn keys(self) -> impl Iterator<Item = KeyCode> {
        self.primary.into_iter().chain(self.secondary)
    }

    pub fn contains(self, key: KeyCode) -> bool {
        self.primary == Some(key) || self.secondary == Some(key)
    }
}

/// Forme écrite d'un [`KeySlots`].
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct SlotsFile {
    primary: String,
    secondary: String,
}

impl From<SlotsFile> for KeySlots {
    fn from(file: SlotsFile) -> Self {
        Self {
            primary: parse_key(&file.primary),
            secondary: parse_key(&file.secondary),
        }
    }
}

impl From<KeySlots> for SlotsFile {
    fn from(slots: KeySlots) -> Self {
        Self {
            primary: slots.primary.map(key_name).unwrap_or_default(),
            secondary: slots.secondary.map(key_name).unwrap_or_default(),
        }
    }
}

/// Le nom d'une case vide dans `Lang::remaps` : une aide qui cite une action sans touche
/// affiche le texte de `key.None` (« — »).
const NO_KEY: &str = "None";

impl UserSettings {
    /// Échelle du mouvement de la souris pour les systèmes de jeu : l'axe vertical inversé au
    /// besoin. Les sensibilités sont des modificateurs des actions ([`Self::foot_scale`],
    /// [`Self::steer_scale`]) : à pied et aux commandes, chacune la sienne.
    pub fn look_scale(&self) -> Vec2 {
        Vec2::new(1.0, if self.invert_y { -1.0 } else { 1.0 })
    }

    /// Multiplicateur du regard à pied.
    pub fn foot_scale(&self) -> f32 {
        self.mouse_sensitivity.clamp(0.1, 5.0)
    }

    /// Multiplicateur de la direction aux commandes : la base des données, puis le réglage.
    pub fn steer_scale(&self, base: f32) -> f32 {
        base * self.piloting_sensitivity.clamp(0.1, 5.0)
    }

    /// Les touches d'une action.
    pub fn slots(&self, action: &KeyAction) -> KeySlots {
        self.bindings
            .get(action.id)
            .copied()
            .unwrap_or_else(|| KeySlots::default_for(action))
    }

    /// Met `key` (ou rien) dans une case de l'action. Aucune autre action ne change : un
    /// conflit se signale ([`Self::conflicts`]), il ne se corrige pas en silence. La même touche
    /// dans l'autre case de la même action n'y servirait à rien : cette case se vide.
    pub fn set_slot(&mut self, action: &KeyAction, slot: Slot, key: Option<KeyCode>) {
        let mut slots = self.slots(action);
        if key.is_some() && slots.get(slot.other()) == key {
            slots.set(slot.other(), None);
        }
        slots.set(slot, key);
        if slots == KeySlots::default_for(action) {
            self.bindings.remove(action.id);
        } else {
            self.bindings.insert(action.id.to_owned(), slots);
        }
    }

    /// Toutes les touches d'origine.
    pub fn reset_keys(&mut self) {
        self.bindings.clear();
        self.legacy_keys.clear();
    }

    /// Les autres actions actives en même temps que `action` (un contexte d'entrée en commun)
    /// qui ont aussi la touche `key`, dans l'ordre du menu.
    pub fn conflicts(&self, action: &KeyAction, key: KeyCode) -> Vec<&'static KeyAction> {
        KEY_ACTIONS
            .iter()
            .filter(|other| {
                other.id != action.id
                    && other.contexts & action.contexts != 0
                    && self.slots(other).contains(key)
            })
            .collect()
    }

    /// La touche de cette case sert-elle aussi à une action active en même temps ?
    pub fn in_conflict(&self, action: &KeyAction, slot: Slot) -> bool {
        self.slots(action)
            .get(slot)
            .is_some_and(|key| !self.conflicts(action, key).is_empty())
    }

    /// Remplacements des jetons de touche des textes (`Lang::remaps`) : touche d'origine →
    /// touche affichée (la principale, sinon la secondaire, sinon « — »). Quand les actions
    /// d'une même touche d'origine n'ont plus la même touche, celle du contexte `context`
    /// l'emporte (une aide parle du contexte où elle s'affiche).
    pub fn token_remaps(&self, context: u8) -> BTreeMap<String, String> {
        let shown = |a: &KeyAction| self.slots(a).keys().next();
        let mut remaps = BTreeMap::new();
        for (index, action) in KEY_ACTIONS.iter().enumerate() {
            // Chaque touche d'origine une fois, à sa première action.
            if KEY_ACTIONS[..index]
                .iter()
                .any(|a| a.default == action.default)
            {
                continue;
            }
            let first = shown(action);
            let mut same = KEY_ACTIONS.iter().filter(|a| a.default == action.default);
            let chosen = if same.clone().all(|a| shown(a) == first) {
                first
            } else {
                same.find(|a| a.contexts & context != 0)
                    .map_or(first, shown)
            };
            if chosen != Some(action.default) {
                remaps.insert(
                    key_name(action.default),
                    chosen.map_or_else(|| NO_KEY.to_owned(), key_name),
                );
            }
        }
        remaps
    }

    /// Applique un remappage écrit (`SH_KEYS`) : des paires `de:vers` séparées par des
    /// virgules. `de` : une action (`foot_interact`) ou une touche d'origine (`KeyE` : toutes
    /// ses actions, comme le remappage par touche de M7) ; `vers` : `principale[+secondaire]`,
    /// `-` pour une case vide (`foot_jump:KeyB+KeyN`, `foot_crouch:-`).
    pub fn apply_key_spec(&mut self, spec: &str) {
        let parse = |name: &str| -> Option<Option<KeyCode>> {
            match name.trim() {
                "-" => Some(None),
                name => parse_key(name).map(Some),
            }
        };
        for pair in spec.split(',').filter(|p| !p.trim().is_empty()) {
            let Some((from, to)) = pair.split_once(':') else {
                warn!("SH_KEYS : « {pair} » ignoré (de:vers)");
                continue;
            };
            let (primary, secondary) = match to.split_once('+') {
                Some((primary, secondary)) => (primary, Some(secondary)),
                None => (to, None),
            };
            // `secondary` : `None` si elle n'est pas donnée (sa case ne change pas).
            let (Some(primary), Some(secondary)) = (
                parse(primary),
                secondary.map_or(Some(None), |name| parse(name).map(Some)),
            ) else {
                warn!("SH_KEYS : touche inconnue dans « {pair} »");
                continue;
            };
            let from = from.trim();
            let targets: Vec<&KeyAction> = match (action_by_id(from), parse_key(from)) {
                (Some(action), _) => vec![action],
                (None, Some(key)) => KEY_ACTIONS.iter().filter(|a| a.default == key).collect(),
                (None, None) => {
                    warn!("SH_KEYS : action ou touche inconnue « {from} »");
                    continue;
                }
            };
            for action in targets {
                self.set_slot(action, Slot::Primary, primary);
                if let Some(secondary) = secondary {
                    self.set_slot(action, Slot::Secondary, secondary);
                }
            }
        }
    }

    /// Convertit l'ancien remappage par touche (M7) : chaque action de la touche d'origine
    /// reçoit la touche choisie en principale (une action déjà réglée au nouveau format garde
    /// son réglage).
    pub fn migrate(&mut self) {
        for (from, to) in std::mem::take(&mut self.legacy_keys) {
            let (Some(from), Some(to)) = (parse_key(&from), parse_key(&to)) else {
                continue;
            };
            for action in KEY_ACTIONS.iter().filter(|a| a.default == from) {
                if !self.bindings.contains_key(action.id) {
                    self.set_slot(action, Slot::Primary, Some(to));
                }
            }
        }
    }
}

/// L'action remappable d'identifiant `id`.
pub fn action_by_id(id: &str) -> Option<&'static KeyAction> {
    KEY_ACTIONS.iter().find(|a| a.id == id)
}

/// Les touches qu'on peut choisir. Échap (annuler), Retour arrière (vider une case), Entrée et
/// les flèches (naviguer) restent aux menus.
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

/// L'étiquette d'une touche physique, **sans** remappage : celle de la disposition, sinon le
/// nom de la langue (`key.Space`), sinon la lettre ou le chiffre de sa position.
/// `Lang::key_label` remplace d'abord une touche d'origine par la touche choisie (pour les
/// jetons des aides) : une case du menu des touches, qui montre la touche elle-même, ne doit pas
/// y passer (c'était l'affichage « à l'envers » de M7).
pub fn physical_label(lang: &Lang, key: KeyCode) -> String {
    let code = key_name(key);
    if let Some(label) = lang.keys.get(&code) {
        return label.clone();
    }
    if let Some(name) = lang.strings.get(&format!("key.{code}")) {
        return name.clone();
    }
    code.strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))
        .unwrap_or(&code)
        .to_owned()
}

fn settings_path() -> std::path::PathBuf {
    crate::save::user_dir().join("settings.ron")
}

/// Les réglages de l'autopilote, qui ne lit pas ceux du joueur ; `env` lit une variable
/// d'environnement. `SH_KEYS=KeyE:KeyF,foot_jump:KeyB+KeyN,…` : un remappage imposé, pour le
/// vérifier. L'autopilote est muet (il tourne pendant que l'auteur travaille), sauf `SH_AUDIO=1`,
/// et sans conseils ni tutoriel (captures propres), sauf `SH_TIPS=1` et `SH_TUTORIAL=1`. Il n'a
/// **pas de limite d'images** : ses mesures voient le vrai coût d'une image et restent
/// comparables d'un jalon à l'autre (`SH_FPS_LIMIT` en pose une, `frame_pace`).
fn autopilot_settings(env: impl Fn(&str) -> Option<String>) -> UserSettings {
    let defaults = UserSettings::default();
    let mut settings = UserSettings {
        tips: env("SH_TIPS").is_some(),
        tutorial: env("SH_TUTORIAL").is_some_and(|v| v.trim() != "0"),
        master_volume: if env("SH_AUDIO").is_some() {
            defaults.master_volume
        } else {
            0.0
        },
        fps_limit: 0,
        ..defaults
    };
    settings.apply_key_spec(&env("SH_KEYS").unwrap_or_default());
    settings
}

/// Les réglages du lancement, et s'il n'y avait pas encore de fichier (premier lancement).
fn load() -> (UserSettings, bool) {
    if crate::dev::autopilot_active() {
        return (autopilot_settings(|name| std::env::var(name).ok()), false);
    }
    let path = settings_path();
    if !path.exists() {
        return (UserSettings::default(), true);
    }
    match sh_data::read_ron::<UserSettings>(&path) {
        Ok(mut settings) => {
            settings.migrate();
            (settings, false)
        }
        Err(err) => {
            warn!("réglages illisibles, valeurs par défaut : {err}");
            (UserSettings::default(), false)
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

// --- Sensibilité de pilotage (lue à part) et disposition du clavier ------------------------

/// Sensibilité de base de la direction à la souris aux commandes (`gameplay/flight.ron`,
/// `steer_sensitivity`), sous le réglage du joueur.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct SteerBase(pub f32);

/// Le seul champ de `flight.ron` lu ici ; les maniements par vaisseau passent par `sh_data`,
/// qui ignore ce champ (à y déplacer avec sa validation, voir le rapport du lot L1 de M9).
#[derive(Deserialize)]
struct FlightTuning {
    #[serde(default = "unit_scale")]
    steer_sensitivity: f32,
}

fn unit_scale() -> f32 {
    1.0
}

fn read_flight_tuning(data_dir: &std::path::Path) -> Result<f32, String> {
    let tuning: FlightTuning =
        sh_data::read_ron(&data_dir.join(sh_data::paths::FLIGHT)).map_err(|err| err.to_string())?;
    let value = tuning.steer_sensitivity;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(format!("steer_sensitivity doit être positive ({value})"))
    }
}

fn read_steer_base() -> f32 {
    read_flight_tuning(&crate::data::data_dir()).unwrap_or_else(|err| {
        warn!("sensibilité de pilotage de flight.ron illisible, 1 par défaut : {err}");
        1.0
    })
}

/// La disposition de clavier retenue (`gameplay/keyboard.ron`, choix de l'onglet Général) :
/// celle du joueur si elle existe, sinon celle des données.
pub fn current_layout<'a>(keyboard: &'a sh_data::Keyboard, settings: &'a UserSettings) -> &'a str {
    if keyboard.layouts.contains_key(&settings.keyboard_layout) {
        &settings.keyboard_layout
    } else {
        &keyboard.default_layout
    }
}

// --- Liaisons --------------------------------------------------------------------------------

/// Une liaison clavier remappable : l'action de [`KEY_ACTIONS`] (index) et la case qu'elle porte.
/// La principale est la liaison déclarée dans `input`, la secondaire sa jumelle, ajoutée ici.
#[derive(Component, Clone, Copy, Debug)]
pub struct KeyBinding {
    pub action: usize,
    pub slot: Slot,
}

/// Liaison déjà examinée par [`tag_bindings`] (remappable ou non).
#[derive(Component)]
struct BindingSeen;

/// Reconnaît les liaisons clavier remappables (contexte de leur action et touche d'origine)
/// et ajoute à chacune une jumelle pour la seconde touche, avec les mêmes modificateurs (les
/// préréglages de directions orientent chaque touche par `Negate` et `SwizzleAxis`).
#[allow(clippy::type_complexity)]
fn tag_bindings(
    mut commands: Commands,
    bindings: Query<
        (
            Entity,
            &Binding,
            Option<&BindingOf>,
            Option<&Negate>,
            Option<&SwizzleAxis>,
        ),
        Without<BindingSeen>,
    >,
    actions: Query<(
        Has<ActionOf<OnFoot>>,
        Has<ActionOf<Piloting>>,
        Has<ActionOf<MapControls>>,
        Has<ActionOf<crate::input::MenuControls>>,
    )>,
) {
    for (entity, binding, of, negate, swizzle) in &bindings {
        // La scène pose ses relations en plusieurs fois : sans elles, on réessaie au frame suivant.
        let Some(of) = of else {
            continue;
        };
        let Ok((foot, piloting, map, menu)) = actions.get(**of) else {
            continue;
        };
        let context = match (foot, piloting, map, menu) {
            (true, ..) => CTX_FOOT,
            (_, true, ..) => CTX_PILOTING,
            (_, _, true, _) => CTX_MAP,
            (.., true) => 0,
            _ => continue,
        };
        commands.entity(entity).try_insert(BindingSeen);
        let Binding::Keyboard { key, .. } = *binding else {
            continue;
        };
        let Some(action) = key_action_index(context, key) else {
            continue;
        };
        commands.entity(entity).try_insert(KeyBinding {
            action,
            slot: Slot::Primary,
        });
        let mut twin = commands.spawn((
            Name::new("Liaison secondaire"),
            BindingSeen,
            KeyBinding {
                action,
                slot: Slot::Secondary,
            },
            Binding::None,
            BindingOf(**of),
        ));
        if let Some(negate) = negate {
            twin.insert(*negate);
        }
        if let Some(swizzle) = swizzle {
            twin.insert(*swizzle);
        }
    }
}

/// Réinsère le `Binding` choisi (composant immuable) sur chaque liaison remappable : la
/// touche de sa case, ou `Binding::None` pour une case vide.
fn apply_keys(
    mut commands: Commands,
    settings: Res<UserSettings>,
    bindings: Query<(Entity, &Binding, &KeyBinding)>,
    added: Query<(), Added<KeyBinding>>,
) {
    if !settings.is_changed() && added.is_empty() {
        return;
    }
    for (entity, binding, tag) in &bindings {
        let Some(action) = KEY_ACTIONS.get(tag.action) else {
            continue;
        };
        let wanted = match settings.slots(action).get(tag.slot) {
            Some(key) => Binding::Keyboard {
                key,
                mod_keys: binding.mod_keys(),
            },
            None => Binding::None,
        };
        if *binding != wanted {
            commands.entity(entity).try_insert(wanted);
        }
    }
}

/// Les étiquettes des jetons de touche des aides (`Lang::remaps`) suivent les touches choisies
/// et le contexte courant ; dans les menus, le dernier contexte de jeu (à pied au départ).
fn apply_token_remaps(
    settings: Res<UserSettings>,
    mode: Res<State<ControlMode>>,
    texts: Option<ResMut<Texts>>,
    mut last: Local<Option<u8>>,
) {
    let Some(mut texts) = texts else {
        return;
    };
    let context = context_of(*mode.get()).or(*last).unwrap_or(CTX_FOOT);
    if !settings.is_changed() && !texts.is_added() && *last == Some(context) {
        return;
    }
    *last = Some(context);
    let remaps = settings.token_remaps(context);
    // Lecture d'abord : ne marquer `Texts` modifié (tout se redessine) que s'il change.
    if texts.remaps != remaps {
        texts.0.remaps = remaps;
    }
}

/// Les sensibilités : un `Scale` sur le regard à pied et sur la direction aux commandes.
#[allow(clippy::type_complexity)]
fn apply_mouse(
    mut commands: Commands,
    settings: Res<UserSettings>,
    base: Res<SteerBase>,
    mut look: Query<(Entity, Option<&mut Scale>), (With<Action<Look>>, Without<Action<Steer>>)>,
    mut steer: Query<(Entity, Option<&mut Scale>), (With<Action<Steer>>, Without<Action<Look>>)>,
) {
    set_scale(&mut commands, look.iter_mut(), settings.foot_scale());
    set_scale(
        &mut commands,
        steer.iter_mut(),
        settings.steer_scale(base.0),
    );
}

fn set_scale<'a>(
    commands: &mut Commands,
    actions: impl Iterator<Item = (Entity, Option<Mut<'a, Scale>>)>,
    value: f32,
) {
    let factor = Vec3::splat(value);
    for (entity, scale) in actions {
        match scale {
            Some(mut scale) => {
                if scale.factor != factor {
                    scale.factor = factor;
                }
            }
            None => {
                commands.entity(entity).try_insert(Scale::new(factor));
            }
        }
    }
}

/// La disposition choisie donne les étiquettes des touches (les appuis les corrigent ensuite,
/// `ui::learn_key_labels`).
fn apply_layout(
    settings: Res<UserSettings>,
    data: Option<Res<Data>>,
    texts: Option<ResMut<Texts>>,
    mut applied: Local<Option<String>>,
) {
    let (Some(data), Some(mut texts)) = (data, texts) else {
        return;
    };
    let keyboard = &data.keyboard;
    let wanted = current_layout(keyboard, &settings);
    if applied.as_deref() == Some(wanted) {
        return;
    }
    // Au démarrage, `Texts::load` a déjà posé la disposition des données : si c'est elle, on
    // garde aussi ce que les premiers appuis ont pu apprendre.
    let first = applied.is_none();
    *applied = Some(wanted.to_owned());
    if first && wanted == keyboard.default_layout {
        return;
    }
    let Some(labels) = keyboard.layouts.get(wanted) else {
        return;
    };
    texts.0.keys = labels.clone();
}

/// Le réglage de l'encart des touches et son état en jeu (la touche qui le replie) se suivent :
/// le joueur le retrouve comme il l'a laissé. Au départ de l'autopilote, l'encart décide
/// (`SH_KEYHELP`).
fn sync_key_help(mut settings: ResMut<UserSettings>, visible: Option<ResMut<KeyHelpVisible>>) {
    let Some(mut visible) = visible else {
        return;
    };
    let autopilot_start = settings.is_added() && crate::dev::autopilot_active();
    let settings_win = settings.is_changed() && !autopilot_start;
    if settings_win && visible.0 != settings.key_help {
        visible.0 = settings.key_help;
    } else if !settings_win
        && (visible.is_changed() || autopilot_start)
        && settings.key_help != visible.0
    {
        settings.key_help = visible.0;
    }
}

fn apply_window(settings: Res<UserSettings>, mut window: Single<&mut Window, With<PrimaryWindow>>) {
    // `SH_WINDOW` décide pour l'autopilote.
    if !settings.is_changed() || std::env::var_os("SH_WINDOW").is_some() {
        return;
    }
    // Seulement s'il change : tout réglage (même la touche qui replie l'encart, en jeu)
    // repasse ici, et réécrire le mode de la fenêtre n'est pas gratuit.
    let wanted = if settings.fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
    if window.mode != wanted {
        window.mode = wanted;
    }
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
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::input::KeyCategory;

    fn action(id: &str) -> &'static KeyAction {
        action_by_id(id).unwrap_or_else(|| panic!("action inconnue : {id}"))
    }

    fn data_dir() -> PathBuf {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest
            .ancestors()
            .nth(2)
            .unwrap_or(manifest)
            .join("assets/data")
    }

    fn ids(actions: &[&KeyAction]) -> Vec<&'static str> {
        actions.iter().map(|a| a.id).collect()
    }

    #[test]
    fn noms_des_touches() {
        for key in CHOOSABLE {
            assert_eq!(parse_key(&key_name(key)), Some(key), "{key:?}");
        }
        for action in &KEY_ACTIONS {
            assert!(choosable(action.default), "{}", action.id);
        }
        assert_eq!(parse_key(""), None);
    }

    /// La table des actions suit les liaisons de `input` : identifiants uniques, une seule
    /// action par touche d'origine dans chaque contexte (c'est ainsi qu'une liaison retrouve son
    /// action), contextes cohérents avec la page.
    #[test]
    fn table_des_actions() {
        for (i, a) in KEY_ACTIONS.iter().enumerate() {
            assert!(
                KEY_ACTIONS[..i].iter().all(|b| b.id != a.id),
                "identifiant en double : {}",
                a.id
            );
            assert!(
                !a.id.contains('.'),
                "{} : pas de point (clé de texte)",
                a.id
            );
            assert_ne!(a.contexts, 0, "{} : aucun contexte", a.id);
            let expected = match a.category {
                KeyCategory::Interface => None,
                KeyCategory::OnFoot => Some(CTX_FOOT),
                KeyCategory::Piloting => Some(CTX_PILOTING),
                KeyCategory::Map => Some(CTX_MAP),
            };
            if let Some(expected) = expected {
                assert_eq!(a.contexts, expected, "{}", a.id);
            }
        }
        for context in [CTX_FOOT, CTX_PILOTING, CTX_MAP] {
            for (i, a) in KEY_ACTIONS.iter().enumerate() {
                if a.contexts & context != 0 {
                    assert_eq!(key_action_index(context, a.default), Some(i), "{}", a.id);
                }
            }
        }
        // Aucune action active en même temps qu'une autre n'a sa touche d'origine.
        let defaults = UserSettings::default();
        for a in &KEY_ACTIONS {
            assert!(!defaults.in_conflict(a, Slot::Primary), "{}", a.id);
        }
    }

    #[test]
    fn chaque_action_a_son_libelle() {
        let layouts: sh_data::Keyboard =
            sh_data::read_ron(&data_dir().join(sh_data::paths::KEYBOARD))
                .unwrap_or_else(|e| panic!("{e}"));
        for code in [sh_data::paths::LANG_CODE, sh_data::paths::LANG_EN_CODE] {
            let lang = sh_data::load_lang(&data_dir(), code).unwrap_or_else(|e| panic!("{e}"));
            let mut keys: Vec<String> = KEY_ACTIONS
                .iter()
                .map(|a| format!("options.action.{}", a.id))
                .collect();
            keys.extend(
                KeyCategory::ALL
                    .iter()
                    .map(|c| format!("options.keys.page.{}", c.name())),
            );
            keys.extend(
                layouts
                    .layouts
                    .keys()
                    .map(|id| format!("options.layout.{id}")),
            );
            keys.push(format!("key.{NO_KEY}"));
            for key in keys {
                assert!(lang.has(&key), "{code} : clé absente {key}");
            }
        }
    }

    /// Plus d'échange : la touche est assignée, rien d'autre ne bouge.
    #[test]
    fn assigner_ne_change_que_la_case_choisie() {
        let mut s = UserSettings::default();
        s.set_slot(action("foot_interact"), Slot::Primary, Some(KeyCode::KeyF));
        assert_eq!(
            s.slots(action("foot_interact")),
            KeySlots {
                primary: Some(KeyCode::KeyF),
                secondary: None
            }
        );
        // Se lever (aux commandes) garde F, Roulis à droite garde E : contextes jamais actifs
        // ensemble, aucun conflit.
        assert_eq!(
            s.slots(action("piloting_leave_seat")).primary,
            Some(KeyCode::KeyF)
        );
        assert_eq!(
            s.slots(action("piloting_roll_right")).primary,
            Some(KeyCode::KeyE)
        );
        assert!(
            s.conflicts(action("foot_interact"), KeyCode::KeyF)
                .is_empty()
        );
        assert_eq!(s.bindings.len(), 1);
        // Retour à l'origine : plus rien dans le fichier.
        s.set_slot(action("foot_interact"), Slot::Primary, Some(KeyCode::KeyE));
        assert!(s.bindings.is_empty());
    }

    /// Un conflit : même catégorie, ou catégories actives en même temps (Interface et À pied).
    #[test]
    fn conflits_par_contexte() {
        let mut s = UserSettings::default();
        s.set_slot(action("foot_interact"), Slot::Primary, Some(KeyCode::KeyR));
        assert_eq!(
            ids(&s.conflicts(action("foot_interact"), KeyCode::KeyR)),
            ["foot_reload"]
        );
        assert!(s.in_conflict(action("foot_reload"), Slot::Primary));

        let mut s = UserSettings::default();
        s.set_slot(
            action("interface_journal"),
            Slot::Secondary,
            Some(KeyCode::KeyR),
        );
        // Le journal ne s'ouvre pas sur la carte : le zoom de la carte n'est pas en conflit.
        assert_eq!(
            ids(&s.conflicts(action("interface_journal"), KeyCode::KeyR)),
            ["foot_reload"]
        );
        assert!(s.in_conflict(action("interface_journal"), Slot::Secondary));
        assert!(!s.in_conflict(action("interface_journal"), Slot::Primary));
        assert!(!s.in_conflict(action("map_zoom_in"), Slot::Primary));

        let mut s = UserSettings::default();
        s.set_slot(action("interface_map"), Slot::Primary, Some(KeyCode::KeyR));
        assert_eq!(
            ids(&s.conflicts(action("interface_map"), KeyCode::KeyR)),
            ["foot_reload", "map_zoom_in"]
        );

        // À pied et aux commandes ne sont jamais actifs ensemble.
        let mut s = UserSettings::default();
        s.set_slot(action("piloting_dock"), Slot::Primary, Some(KeyCode::KeyR));
        assert!(
            s.conflicts(action("piloting_dock"), KeyCode::KeyR)
                .is_empty()
        );
        s.set_slot(action("piloting_dock"), Slot::Primary, Some(KeyCode::KeyE));
        assert_eq!(
            ids(&s.conflicts(action("piloting_dock"), KeyCode::KeyE)),
            ["piloting_roll_right"]
        );
    }

    #[test]
    fn seconde_touche_et_cases_vides() {
        let mut s = UserSettings::default();
        let jump = action("foot_jump");
        s.set_slot(jump, Slot::Secondary, Some(KeyCode::KeyB));
        assert_eq!(
            s.slots(jump),
            KeySlots {
                primary: Some(KeyCode::Space),
                secondary: Some(KeyCode::KeyB)
            }
        );
        assert_eq!(
            s.slots(jump).keys().collect::<Vec<_>>(),
            [KeyCode::Space, KeyCode::KeyB]
        );
        // Vider la principale : la secondaire reste.
        s.set_slot(jump, Slot::Primary, None);
        assert_eq!(s.slots(jump).keys().collect::<Vec<_>>(), [KeyCode::KeyB]);
        // La touche de l'autre case de la même action : cette case se vide.
        s.set_slot(jump, Slot::Primary, Some(KeyCode::KeyB));
        assert_eq!(
            s.slots(jump),
            KeySlots {
                primary: Some(KeyCode::KeyB),
                secondary: None
            }
        );
        s.reset_keys();
        assert!(s.bindings.is_empty());
        assert_eq!(s.slots(jump), KeySlots::default_for(jump));
    }

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sh_options_{}_{name}.ron", std::process::id()))
    }

    /// Les deux touches s'écrivent par leur nom et se relisent à l'identique.
    #[test]
    fn deux_touches_aller_retour() {
        let mut s = UserSettings::default();
        s.set_slot(action("foot_jump"), Slot::Secondary, Some(KeyCode::KeyB));
        s.set_slot(action("foot_crouch"), Slot::Primary, None);
        s.set_slot(action("piloting_dock"), Slot::Primary, Some(KeyCode::KeyT));
        s.piloting_sensitivity = 1.5;
        s.fps_limit = 144;
        s.vsync = true;
        s.show_fps = true;
        let path = temp_file("aller_retour");
        sh_data::write_ron(&path, &s).unwrap_or_else(|e| panic!("{e}"));
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let back: UserSettings = sh_data::read_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        let _ = std::fs::remove_file(&path);
        assert_eq!(back, s);
        assert!(text.contains("\"KeyB\""), "{text}");
        // L'ancien champ n'est pas réécrit.
        assert!(!text.contains("keys:"), "{text}");
    }

    /// 60 i/s par défaut ; « Illimitée » (0) choisie par le joueur s'écrit et se relit telle
    /// quelle, sans retomber sur le défaut.
    #[test]
    fn limite_d_images_par_defaut_et_illimitee_gardee() {
        assert_eq!(DEFAULT_FPS_LIMIT, 60);
        assert_eq!(UserSettings::default().fps_limit, DEFAULT_FPS_LIMIT);
        let s = UserSettings {
            fps_limit: 0,
            ..UserSettings::default()
        };
        let path = temp_file("illimitee");
        sh_data::write_ron(&path, &s).unwrap_or_else(|e| panic!("{e}"));
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let back: UserSettings = sh_data::read_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        let _ = std::fs::remove_file(&path);
        assert!(text.contains("fps_limit: 0"), "{text}");
        assert_eq!(back.fps_limit, 0);
        // Un fichier qui ne cite que ce champ : le reste prend ses défauts, le 0 reste.
        let path = temp_file("illimitee_seule");
        std::fs::write(&path, "(fps_limit: 0)").unwrap_or_else(|e| panic!("{e}"));
        let back: UserSettings = sh_data::read_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        let _ = std::fs::remove_file(&path);
        assert_eq!(back.fps_limit, 0);
        assert_eq!(back.preset, UserSettings::default().preset);
    }

    /// L'autopilote n'a pas de limite d'images (mesures comparables), est muet et sans
    /// conseils ; ses variables le règlent.
    #[test]
    fn reglages_de_l_autopilote() {
        let s = autopilot_settings(|_| None);
        assert_eq!(s.fps_limit, 0);
        assert_eq!(s.master_volume, 0.0);
        assert!(!s.tips && !s.tutorial);
        let s = autopilot_settings(|name| match name {
            "SH_AUDIO" | "SH_TIPS" => Some("1".into()),
            "SH_TUTORIAL" => Some("0".into()),
            "SH_KEYS" => Some("foot_interact:KeyF".into()),
            _ => None,
        });
        assert_eq!(s.fps_limit, 0);
        assert_eq!(s.master_volume, UserSettings::default().master_volume);
        assert!(s.tips && !s.tutorial);
        assert_eq!(
            s.slots(action("foot_interact")).primary,
            Some(KeyCode::KeyF)
        );
    }

    /// Première partie : Bas sur un GPU intégré, Moyen ailleurs (et sur un type inconnu).
    #[test]
    fn prereglage_selon_le_gpu() {
        assert_eq!(default_preset_for_gpu("IntegratedGpu"), PresetLevel::Low);
        for other in ["DiscreteGpu", "VirtualGpu", "Cpu", "Other", ""] {
            assert_eq!(
                default_preset_for_gpu(other),
                PresetLevel::Medium,
                "{other}"
            );
        }
        // Le défaut de la structure (fichier illisible, autopilote) reste Moyen.
        assert_eq!(UserSettings::default().preset, PresetLevel::Medium);
    }

    /// Le fichier de l'auteur, écrit pendant la vague 3 de M11, contient encore
    /// `dynamic_resolution` (option retirée) : il se relit sans erreur, et aucun autre réglage
    /// n'est perdu.
    #[test]
    fn ancien_champ_ignore() {
        let path = temp_file("dynres");
        std::fs::write(
            &path,
            r#"(
    preset: High,
    fullscreen: true,
    master_volume: 0.4,
    music_volume: 0.6,
    effects_volume: 0.8,
    mouse_sensitivity: 1.2,
    piloting_sensitivity: 0.9,
    invert_y: false,
    tips: false,
    tutorial: false,
    keyboard_layout: "qwerty",
    key_help: false,
    bindings: {
        "foot_interact": (primary: "KeyF", secondary: ""),
    },
    fps_limit: 75,
    vsync: true,
    show_fps: true,
    dynamic_resolution: true,
)"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let back: UserSettings = sh_data::read_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        let _ = std::fs::remove_file(&path);
        assert_eq!(back.preset, PresetLevel::High);
        assert!(back.fullscreen && back.vsync && back.show_fps);
        assert!(!back.tips && !back.tutorial && !back.key_help);
        assert_eq!(back.master_volume, 0.4);
        assert_eq!(back.piloting_sensitivity, 0.9);
        assert_eq!(back.keyboard_layout, "qwerty");
        assert_eq!(back.fps_limit, 75);
        assert_eq!(
            back.slots(action("foot_interact")).primary,
            Some(KeyCode::KeyF)
        );
        // Réécrit, il n'a plus le champ retiré.
        let path = temp_file("dynres_reecrit");
        sh_data::write_ron(&path, &back).unwrap_or_else(|e| panic!("{e}"));
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        assert!(!text.contains("dynamic_resolution"), "{text}");
    }

    /// Un fichier de M7 (une touche par touche d'origine) se relit sans erreur.
    #[test]
    fn ancien_fichier_relu() {
        let path = temp_file("ancien");
        std::fs::write(
            &path,
            r#"(
    preset: Low,
    fullscreen: true,
    master_volume: 0.5,
    music_volume: 0.6,
    effects_volume: 0.8,
    mouse_sensitivity: 1.3,
    invert_y: true,
    tips: false,
    tutorial: false,
    keys: {
        "KeyE": "KeyF",
        "KeyF": "KeyE",
    },
)"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut s: UserSettings = sh_data::read_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        let _ = std::fs::remove_file(&path);
        s.migrate();
        assert_eq!(s.preset, PresetLevel::Low);
        assert!(s.invert_y && s.fullscreen);
        assert_eq!(s.piloting_sensitivity, 1.0);
        // Réglages de M11 absents du fichier : la limite par défaut (60), ni vsync ni compteur.
        assert_eq!(
            (s.fps_limit, s.vsync, s.show_fps),
            (DEFAULT_FPS_LIMIT, false, false)
        );
        assert!(s.legacy_keys.is_empty());
        // L'ancienne touche E (interagir, roulis à droite) est sur F partout, et F sur E.
        for (id, key) in [
            ("foot_interact", KeyCode::KeyF),
            ("piloting_roll_right", KeyCode::KeyF),
            ("piloting_leave_seat", KeyCode::KeyE),
            ("map_zoom_out", KeyCode::KeyE),
        ] {
            assert_eq!(s.slots(action(id)).primary, Some(key), "{id}");
        }
        assert_eq!(s.slots(action("foot_jump")).primary, Some(KeyCode::Space));
    }

    #[test]
    fn jetons_selon_le_contexte() {
        let mut s = UserSettings::default();
        assert!(s.token_remaps(CTX_FOOT).is_empty());
        // Interagir passe sur F, le roulis reste sur E : [KeyE] dépend du contexte.
        s.set_slot(action("foot_interact"), Slot::Primary, Some(KeyCode::KeyF));
        assert_eq!(
            s.token_remaps(CTX_FOOT).get("KeyE").map(String::as_str),
            Some("KeyF")
        );
        assert_eq!(s.token_remaps(CTX_PILOTING).get("KeyE"), None);
        // Toutes les actions de W sur I : partout.
        for id in ["foot_forward", "piloting_forward", "map_north"] {
            s.set_slot(action(id), Slot::Primary, Some(KeyCode::KeyI));
        }
        for context in [CTX_FOOT, CTX_PILOTING, CTX_MAP] {
            assert_eq!(
                s.token_remaps(context).get("KeyW").map(String::as_str),
                Some("KeyI")
            );
        }
        // Une action commune vidée, sa secondaire sert ; sans touche du tout : « — ».
        s.set_slot(
            action("interface_journal"),
            Slot::Secondary,
            Some(KeyCode::KeyL),
        );
        s.set_slot(action("interface_journal"), Slot::Primary, None);
        assert_eq!(
            s.token_remaps(CTX_FOOT).get("KeyJ").map(String::as_str),
            Some("KeyL")
        );
        s.set_slot(action("interface_journal"), Slot::Secondary, None);
        assert_eq!(
            s.token_remaps(CTX_FOOT).get("KeyJ").map(String::as_str),
            Some(NO_KEY)
        );
    }

    /// `SH_KEYS` : par touche d'origine (forme de M7), par action, deux touches, case vide.
    #[test]
    fn remappage_impose() {
        let mut s = UserSettings::default();
        s.apply_key_spec("KeyE:KeyF, foot_jump:KeyB+KeyN, foot_crouch:-, inconnu:KeyA, KeyQ:Bof");
        assert_eq!(
            s.slots(action("foot_interact")).primary,
            Some(KeyCode::KeyF)
        );
        assert_eq!(
            s.slots(action("piloting_roll_right")).primary,
            Some(KeyCode::KeyF)
        );
        assert_eq!(
            s.slots(action("foot_jump")),
            KeySlots {
                primary: Some(KeyCode::KeyB),
                secondary: Some(KeyCode::KeyN)
            }
        );
        assert_eq!(s.slots(action("foot_crouch")).keys().count(), 0);
        assert_eq!(
            s.slots(action("piloting_roll_left")).primary,
            Some(KeyCode::KeyQ)
        );
        s.apply_key_spec("");
        assert_eq!(s.bindings.len(), 4);
    }

    #[test]
    fn sensibilites() {
        let s = UserSettings {
            invert_y: true,
            mouse_sensitivity: 2.0,
            piloting_sensitivity: 0.5,
            ..UserSettings::default()
        };
        assert_eq!(s.look_scale(), Vec2::new(1.0, -1.0));
        assert_eq!(s.foot_scale(), 2.0);
        assert_eq!(s.steer_scale(3.0), 1.5);
    }

    /// Demande de l'auteur du 05/11/2026 : la direction à la souris trois fois plus sensible.
    #[test]
    fn sensibilite_du_pilotage_triplee() {
        let base = read_flight_tuning(&data_dir()).unwrap_or_else(|e| panic!("{e}"));
        assert!((base - 3.0).abs() < 1e-6, "{base}");
    }
}
