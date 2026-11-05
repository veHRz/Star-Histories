//! Encart des touches (M9, lot L2) : à gauche de l'écran, sous le bandeau du lieu, les actions
//! possibles et leurs touches, repliable par [`ToggleKeyHelp`] (F1 par défaut). Replié, il
//! garde son titre et la touche qui le rouvre. [`KeyHelpVisible`] dit s'il est déplié (oui par
//! défaut ; l'autopilote le force par `SH_KEYHELP=0|1`).
//!
//! - **Encart automatique** : en jeu, il liste les actions des contextes d'entrée **actifs** (à
//!   pied, aux commandes, carte ; rien dans un menu) avec leurs touches **réelles**, lues dans
//!   les entités de liaison (`Binding`, reliées à leur action par `BindingOf`) : un remappage,
//!   une seconde touche, la disposition du clavier s'y voient d'eux-mêmes. Les lignes et leurs
//!   libellés (clés `keys.action.*`) sont dans les tables ci-dessous ; une ligne sans touche ne
//!   s'affiche pas. Petit par demande de l'auteur : les actions voisines partagent une ligne,
//!   une dizaine de lignes au plus ; les actions d'interface (carte, écrans, sauvegarde, pause)
//!   viennent sous un filet, en plus discret. Une seconde touche (liaison jumelle) s'affiche à
//!   côté de la première ; une case vide (`Binding::None`) et la manette sont ignorées.
//! - **Encart d'un écran** (réutilisable) : un écran pose [`KeyHelp`] sur une de ses entités ;
//!   tant qu'elle existe, l'encart montre ces lignes à la place, au-dessus des voiles des menus
//!   (la plus récente l'emporte) :
//!
//! ```ignore
//! commands.spawn((
//!     MyScreenRoot,
//!     KeyHelp::new([
//!         KeyHint::action::<Select>(texts.get("keys.action.select")),
//!         KeyHint::key(KeyCode::Escape, texts.get("keys.action.back")),
//!     ]),
//! ));
//! ```
//!
//! Une liaison porte déjà la touche choisie par le joueur : on l'étiquette telle quelle (selon la
//! disposition du clavier), sans repasser par le remappage des jetons de texte
//! (`Lang::key_label`, qui part de la touche d'origine).
//!
//! Écrans (M10) : le terminal, la conversation, la pause et les options posent leur liste ici au
//! lieu d'écrire leurs touches en bas de l'écran. Une ligne peut montrer une plage de touches
//! (« 1–9 », [`KeySource::Range`]), une combinaison (« Maj+Tab », [`KeySource::Chord`]), un clic
//! ou la molette. L'encart existe aussi au menu principal (options), et la touche qui le replie
//! y marche comme en jeu : le contexte Menu n'a pas cette action, [`toggle_in_menus`] lit ses
//! liaisons. Un écran large réserve à gauche la place de l'encart ([`SCREEN_RESERVE`]).
//!
//! Coût : l'encart ne se reconstruit que si son contenu change (contexte, touches, langue, repli,
//! liste d'un écran) ; le reste du temps, il ne fait rien.

use std::any::type_name;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use sh_data::Lang;

use crate::data::Texts;
use crate::input::{
    Boost, Countermeasure, Crouch, Dock, FireGuns, FireMissile, FireWeapon, Interact, Jump,
    LeaveSeat, Look, MapControls, MenuControls, Move, OnFoot, OpenCodex, OpenInventory,
    OpenJournal, OpenSheet, OpenSkills, Pan, Pause, Piloting, QuickLoad, QuickSave, Reload, Roll,
    Select, SkipTutorial, Sprint, Steer, Thrust, ToggleFlightAssist, ToggleKeyHelp, ToggleMap,
    TravelHyperspace, TravelPortal, Zoom,
};
use crate::options::UserSettings;
use crate::state::{AppState, ControlMode};
use crate::tutorial::Tutorial;
use crate::ui::character::naming::NamingScreen;
use crate::ui::options::{OptionsInput, OptionsScreen};
use crate::ui::theme::{self, Frame, UiFonts};
use crate::views::BANNER_BOTTOM;

pub struct KeysPanelPlugin;

impl Plugin for KeysPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyHelpVisible>()
            .add_observer(on_toggle_key_help)
            .add_systems(OnEnter(AppState::InGame), spawn_panel::<true>)
            .add_systems(OnEnter(AppState::MainMenu), spawn_panel::<false>)
            .add_systems(
                Update,
                (
                    toggle_in_menus
                        .run_if(panel_exists)
                        .run_if(in_state(ControlMode::Menu))
                        .before(OptionsInput),
                    refresh_panel.run_if(panel_exists),
                ),
            );
    }
}

/// L'encart existe en partie, et au menu principal pour les options qui s'y ouvrent.
fn panel_exists(state: Option<Res<State<AppState>>>) -> bool {
    state.is_some_and(|s| matches!(s.get(), AppState::InGame | AppState::MainMenu))
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

/// Dans un menu (terminal, pause, options) : le contexte Menu n'a pas l'action
/// [`ToggleKeyHelp`] (ses touches ne se remappent pas), mais l'encart de l'écran doit se replier
/// avec la même touche qu'en jeu. On lit donc les liaisons clavier de l'action (remappage et
/// seconde touche compris). Pas pendant que les options attendent une touche : elle va à la case ;
/// ni pendant la saisie d'un nom (casier) : une lettre liée au repli s'écrit dans le champ.
fn toggle_in_menus(
    keys: Res<ButtonInput<KeyCode>>,
    options: Option<Res<OptionsScreen>>,
    naming: Option<Res<NamingScreen>>,
    actions: Query<(&Name, &Bindings), With<ActionSettings>>,
    bindings: Query<&Binding>,
    mut visible: ResMut<KeyHelpVisible>,
) {
    if keys.get_just_pressed().next().is_none()
        || options.is_some_and(|o| o.is_waiting())
        || naming.is_some()
    {
        return;
    }
    let name = type_name::<ToggleKeyHelp>();
    let pressed = actions
        .iter()
        .filter(|(n, _)| n.as_str() == name)
        .flat_map(|(_, list)| list.iter())
        .filter_map(|b| bindings.get(b).ok())
        .any(|binding| match *binding {
            Binding::Keyboard { key, mod_keys } => {
                keys.just_pressed(key) && mods_held(&keys, mod_keys)
            }
            _ => false,
        });
    if pressed {
        visible.0 = !visible.0;
    }
}

/// Les modificateurs d'une combinaison sont-ils tenus ?
fn mods_held(keys: &ButtonInput<KeyCode>, mods: ModKeys) -> bool {
    [
        (
            ModKeys::CONTROL,
            [KeyCode::ControlLeft, KeyCode::ControlRight],
        ),
        (ModKeys::SHIFT, [KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        (ModKeys::ALT, [KeyCode::AltLeft, KeyCode::AltRight]),
        (ModKeys::SUPER, [KeyCode::SuperLeft, KeyCode::SuperRight]),
    ]
    .into_iter()
    .all(|(flag, pair)| !mods.contains(flag) || keys.any_pressed(pair))
}

// --- API réutilisable ----------------------------------------------------------------------------

/// Une touche montrée dans l'encart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeySource {
    /// Toutes les touches liées à une action (nom de type, voir [`KeyHint::action`]), dans tous
    /// les contextes : remappage et seconde touche compris.
    Action(&'static str),
    /// Une touche fixe (Échap, Entrée des menus…), étiquetée selon la disposition du clavier.
    Key(KeyCode),
    Mouse(MouseButton),
    /// Le déplacement de la souris.
    MouseMotion,
    /// La molette.
    Wheel,
    /// Une plage de touches fixes, en une seule touche dessinée (« 1–9 »).
    Range(KeyCode, KeyCode),
    /// Une combinaison fixe : un modificateur tenu, puis une touche (« Maj+Tab »).
    Chord(KeyCode, KeyCode),
}

/// Une ligne de l'encart : des touches, un libellé déjà traduit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyHint {
    pub keys: Vec<KeySource>,
    pub label: String,
}

impl KeyHint {
    /// Les touches réelles de l'action `A` (lues dans ses liaisons).
    pub fn action<A: InputAction>(label: impl Into<String>) -> Self {
        Self {
            keys: vec![KeySource::Action(type_name::<A>())],
            label: label.into(),
        }
    }

    pub fn key(key: KeyCode, label: impl Into<String>) -> Self {
        Self {
            keys: vec![KeySource::Key(key)],
            label: label.into(),
        }
    }

    pub fn mouse(button: MouseButton, label: impl Into<String>) -> Self {
        Self {
            keys: vec![KeySource::Mouse(button)],
            label: label.into(),
        }
    }

    /// Plusieurs touches de toutes sortes sur une ligne, dans l'ordre donné.
    pub fn keys(keys: impl IntoIterator<Item = KeySource>, label: impl Into<String>) -> Self {
        Self {
            keys: keys.into_iter().collect(),
            label: label.into(),
        }
    }

    /// Une plage de touches fixes (« 1–9 »).
    pub fn range(first: KeyCode, last: KeyCode, label: impl Into<String>) -> Self {
        Self::keys([KeySource::Range(first, last)], label)
    }

    /// Ajoute sur la même ligne les touches d'une autre action.
    pub fn and_action<A: InputAction>(mut self) -> Self {
        self.keys.push(KeySource::Action(type_name::<A>()));
        self
    }

    /// Ajoute sur la même ligne une touche fixe.
    pub fn and_key(mut self, key: KeyCode) -> Self {
        self.keys.push(KeySource::Key(key));
        self
    }

    /// Ajoute sur la même ligne un bouton de la souris (« Clic G »).
    pub fn and_mouse(mut self, button: MouseButton) -> Self {
        self.keys.push(KeySource::Mouse(button));
        self
    }

    /// Ajoute sur la même ligne la molette.
    pub fn and_wheel(mut self) -> Self {
        self.keys.push(KeySource::Wheel);
        self
    }
}

/// Posé sur une entité (la racine d'un écran, le plus souvent) : tant qu'elle existe, l'encart
/// montre ces lignes au lieu de celles des contextes actifs, au-dessus des voiles des menus. Le
/// titre par défaut est « Touches » ; le repli (F1) s'applique aussi.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyHelp {
    pub title: Option<String>,
    pub hints: Vec<KeyHint>,
}

impl KeyHelp {
    pub fn new(hints: impl IntoIterator<Item = KeyHint>) -> Self {
        Self {
            title: None,
            hints: hints.into_iter().collect(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

// --- Tables de l'encart automatique ------------------------------------------------------------

/// Le nom d'une action : celui que porte son entité (`Name`, posé par `Action<A>`).
type ActionName = fn() -> &'static str;

/// Quelles liaisons d'une action montrer, et dans quel ordre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    /// Toutes, dans l'ordre des liaisons (boutons).
    All,
    /// Déplacement à plat (sortie 2D) : avant, gauche, arrière, droite (« ZQSD »).
    Cardinal,
    /// Poussée dans le plan (sortie 3D, −Z = avant) : avant, gauche, arrière, droite.
    Plane,
    /// Poussée verticale (sortie 3D) : haut, bas.
    Vertical,
    /// Un axe, négatif puis positif (roulis : gauche, droite).
    Axis,
    /// Un axe, positif puis négatif (zoom : rapprocher, éloigner).
    AxisReversed,
    /// Deviné d'après les sens des liaisons (listes d'écran).
    Auto,
}

/// Le sens d'une liaison : l'axe où son modificateur la range (`SwizzleAxis`) et son signe
/// (`Negate`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dir {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Part {
    fn order(self) -> Option<&'static [Dir]> {
        match self {
            Part::All | Part::Auto => None,
            Part::Cardinal => Some(&[Dir::PosY, Dir::NegX, Dir::NegY, Dir::PosX]),
            Part::Plane => Some(&[Dir::NegZ, Dir::NegX, Dir::PosZ, Dir::PosX]),
            Part::Vertical => Some(&[Dir::PosY, Dir::NegY]),
            Part::Axis => Some(&[Dir::NegX, Dir::PosX]),
            Part::AxisReversed => Some(&[Dir::PosX, Dir::NegX]),
        }
    }

    /// La souris et la molette (sans sens) se montrent-elles sur cette ligne ? Pas sur une
    /// moitié de poussée : elles iraient deux fois.
    fn takes_pointer(self) -> bool {
        !matches!(self, Part::Plane | Part::Vertical)
    }
}

/// Une ligne n'apparaît que…
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum When {
    Always,
    /// … pendant le tutoriel guidé.
    Tutorial,
}

/// Une ligne de l'encart automatique : les touches d'une ou plusieurs actions du contexte, un
/// libellé (clé de langue). Plusieurs actions voisines partagent une ligne (« Clic G Clic D :
/// Canons · missile ») : l'encart reste petit (une dizaine de lignes au plus).
struct Row {
    keys: &'static [(ActionName, Part)],
    label: &'static str,
    when: When,
    /// Action d'interface (carte, écrans, sauvegarde, pause) : sous un filet, en plus discret,
    /// après les actions propres au contexte.
    interface: bool,
}

impl Row {
    const fn new(keys: &'static [(ActionName, Part)], label: &'static str) -> Self {
        Self {
            keys,
            label,
            when: When::Always,
            interface: false,
        }
    }

    const fn interface(self) -> Self {
        Self {
            interface: true,
            ..self
        }
    }

    const fn during_tutorial(self) -> Self {
        Self {
            when: When::Tutorial,
            ..self
        }
    }
}

const MAP_PAUSE: Row = Row::new(
    &[
        (type_name::<ToggleMap>, Part::All),
        (type_name::<Pause>, Part::All),
    ],
    "keys.action.map_pause",
)
.interface();
const RECORDS: Row = Row::new(
    &[
        (type_name::<OpenSheet>, Part::All),
        (type_name::<OpenJournal>, Part::All),
        (type_name::<OpenCodex>, Part::All),
    ],
    "keys.action.records",
)
.interface();
const CHARACTER: Row = Row::new(
    &[
        (type_name::<OpenInventory>, Part::All),
        (type_name::<OpenSkills>, Part::All),
    ],
    "keys.action.character",
)
.interface();
const QUICK: Row = Row::new(
    &[
        (type_name::<QuickSave>, Part::All),
        (type_name::<QuickLoad>, Part::All),
    ],
    "keys.action.quicksave",
)
.interface();
const SKIP: Row = Row::new(
    &[(type_name::<SkipTutorial>, Part::All)],
    "keys.action.skip_tutorial",
)
.interface()
.during_tutorial();
const PAUSE: Row = Row::new(&[(type_name::<Pause>, Part::All)], "keys.action.pause").interface();

/// À pied : 6 lignes du contexte, 4 d'interface (+ 1 pendant le tutoriel).
const ON_FOOT_ROWS: &[Row] = &[
    Row::new(&[(type_name::<Move>, Part::Cardinal)], "keys.action.move"),
    Row::new(&[(type_name::<Look>, Part::All)], "keys.action.look"),
    Row::new(
        &[
            (type_name::<Sprint>, Part::All),
            (type_name::<Jump>, Part::All),
        ],
        "keys.action.sprint_jump",
    ),
    Row::new(&[(type_name::<Crouch>, Part::All)], "keys.action.crouch"),
    Row::new(
        &[(type_name::<Interact>, Part::All)],
        "keys.action.interact",
    ),
    Row::new(
        &[
            (type_name::<FireWeapon>, Part::All),
            (type_name::<Reload>, Part::All),
        ],
        "keys.action.fire_reload",
    ),
    MAP_PAUSE,
    RECORDS,
    CHARACTER,
    QUICK,
    SKIP,
];

/// Aux commandes : 6 lignes du contexte, 4 d'interface (+ 1 pendant le tutoriel).
const PILOTING_ROWS: &[Row] = &[
    Row::new(
        &[
            (type_name::<Thrust>, Part::Plane),
            (type_name::<Thrust>, Part::Vertical),
        ],
        "keys.action.thrust",
    ),
    Row::new(
        &[
            (type_name::<Steer>, Part::All),
            (type_name::<Roll>, Part::Axis),
        ],
        "keys.action.steer_roll",
    ),
    Row::new(
        &[
            (type_name::<Boost>, Part::All),
            (type_name::<ToggleFlightAssist>, Part::All),
        ],
        "keys.action.cruise_assist",
    ),
    Row::new(
        &[
            (type_name::<Dock>, Part::All),
            (type_name::<LeaveSeat>, Part::All),
        ],
        "keys.action.dock_leave",
    ),
    Row::new(
        &[
            (type_name::<FireGuns>, Part::All),
            (type_name::<FireMissile>, Part::All),
        ],
        "keys.action.guns_missile",
    ),
    Row::new(
        &[(type_name::<Countermeasure>, Part::All)],
        "keys.action.decoys",
    ),
    MAP_PAUSE,
    RECORDS,
    CHARACTER,
    QUICK,
    SKIP,
];

/// Sur la carte : 6 lignes du contexte, 1 d'interface (+ 1 pendant le tutoriel).
const MAP_ROWS: &[Row] = &[
    Row::new(&[(type_name::<Pan>, Part::Cardinal)], "keys.action.pan"),
    Row::new(
        &[(type_name::<Zoom>, Part::AxisReversed)],
        "keys.action.zoom",
    ),
    Row::new(&[(type_name::<Select>, Part::All)], "keys.action.select"),
    Row::new(
        &[(type_name::<TravelPortal>, Part::All)],
        "keys.action.travel_portal",
    ),
    Row::new(
        &[(type_name::<TravelHyperspace>, Part::All)],
        "keys.action.travel_hyperspace",
    ),
    Row::new(
        &[(type_name::<ToggleMap>, Part::All)],
        "keys.action.map_back",
    ),
    PAUSE,
    SKIP,
];

// --- Lecture des liaisons ----------------------------------------------------------------------

/// Les contextes d'entrée, leurs actions et leurs liaisons.
#[derive(SystemParam)]
struct InputView<'w, 's> {
    on_foot: Query<'w, 's, (&'static ContextActivity<OnFoot>, &'static Actions<OnFoot>)>,
    piloting: Query<
        'w,
        's,
        (
            &'static ContextActivity<Piloting>,
            &'static Actions<Piloting>,
        ),
    >,
    map: Query<
        'w,
        's,
        (
            &'static ContextActivity<MapControls>,
            &'static Actions<MapControls>,
        ),
    >,
    /// Toute entité d'action porte `ActionSettings` (composant requis par `Action<A>`).
    actions:
        Query<'w, 's, (Entity, &'static Name, Option<&'static Bindings>), With<ActionSettings>>,
    bindings: Query<
        'w,
        's,
        (
            &'static Binding,
            Option<&'static Negate>,
            Option<&'static SwizzleAxis>,
        ),
    >,
}

/// Les actions des contextes actifs d'un type.
fn active_actions<C: Component>(
    contexts: &Query<(&ContextActivity<C>, &Actions<C>)>,
) -> Option<Vec<Entity>> {
    let mut found: Option<Vec<Entity>> = None;
    for (activity, actions) in contexts {
        if **activity {
            found.get_or_insert_with(Vec::new).extend(actions.iter());
        }
    }
    found
}

impl InputView<'_, '_> {
    /// Les tables des contextes actifs, avec leurs actions (un menu n'en a pas : rien à lister).
    fn active_tables(&self) -> Vec<(&'static [Row], Vec<Entity>)> {
        [
            (ON_FOOT_ROWS, active_actions(&self.on_foot)),
            (PILOTING_ROWS, active_actions(&self.piloting)),
            (MAP_ROWS, active_actions(&self.map)),
        ]
        .into_iter()
        .filter_map(|(rows, actions)| actions.map(|a| (rows, a)))
        .collect()
    }

    fn named(&self, action: Entity, name: &str) -> bool {
        self.actions
            .get(action)
            .is_ok_and(|(_, n, _)| n.as_str() == name)
    }

    /// Les étiquettes des liaisons d'une action, rangées selon `part`, ajoutées à `out` (sans
    /// doublon).
    fn action_caps(&self, lang: &Lang, action: Entity, part: Part, out: &mut Vec<String>) {
        let Ok((_, _, Some(bindings))) = self.actions.get(action) else {
            return;
        };
        let bound: Vec<Bound> = bindings
            .iter()
            .filter_map(|b| self.bindings.get(b).ok())
            .filter_map(|(binding, negate, swizzle)| bound(lang, binding, negate, swizzle))
            .collect();
        let part = if part == Part::Auto {
            guess_part(&bound)
        } else {
            part
        };
        if part.takes_pointer() {
            for b in bound.iter().filter(|b| b.pointer) {
                push_unique(out, b.label.clone());
            }
        }
        let keys = bound.into_iter().filter(|b| !b.pointer);
        let Some(order) = part.order() else {
            for b in keys {
                push_unique(out, b.label);
            }
            return;
        };
        // Jeux de touches : un sens déjà pris ouvre un nouveau jeu (ZQSD, puis les flèches).
        let mut sets: Vec<Vec<Option<String>>> = Vec::new();
        for b in keys {
            let Some(slot) = order.iter().position(|d| *d == b.dir) else {
                continue;
            };
            match sets.iter_mut().find(|s| s[slot].is_none()) {
                Some(set) => set[slot] = Some(b.label),
                None => {
                    let mut set = vec![None; order.len()];
                    set[slot] = Some(b.label);
                    sets.push(set);
                }
            }
        }
        for set in sets {
            let labels: Vec<String> = set.into_iter().flatten().collect();
            // Quatre sens d'un caractère chacun : une seule touche dessinée (« ZQSD »).
            if order.len() == 4
                && labels.len() == 4
                && labels.iter().all(|l| l.chars().count() == 1)
            {
                push_unique(out, labels.concat());
            } else {
                for label in labels {
                    push_unique(out, label);
                }
            }
        }
    }

    /// Les étiquettes d'une source de touche d'une liste d'écran.
    fn source_caps(&self, lang: &Lang, source: &KeySource, out: &mut Vec<String>) {
        match source {
            KeySource::Action(name) => {
                for (entity, n, _) in &self.actions {
                    if n.as_str() == *name {
                        self.action_caps(lang, entity, Part::Auto, out);
                    }
                }
            }
            KeySource::Key(key) => push_unique(out, key_label(lang, *key)),
            KeySource::Mouse(button) => push_unique(out, mouse_label(lang, *button)),
            KeySource::MouseMotion => push_unique(out, lang.get("keys.cap.mouse")),
            KeySource::Wheel => push_unique(out, lang.get("keys.cap.wheel")),
            KeySource::Range(first, last) => push_unique(out, range_label(lang, *first, *last)),
            KeySource::Chord(modifier, key) => {
                push_unique(out, chord_label(lang, *modifier, *key));
            }
        }
    }
}

/// Une combinaison fixe en une étiquette (« Maj+Tab »).
fn chord_label(lang: &Lang, modifier: KeyCode, key: KeyCode) -> String {
    lang.format(
        "keys.cap.combo",
        &[
            ("mod", key_label(lang, modifier).as_str()),
            ("key", key_label(lang, key).as_str()),
        ],
    )
}

/// Une plage de touches en une étiquette (« 1–9 ») : touches fixes, étiquetées selon la
/// disposition du clavier, sans remappage.
fn range_label(lang: &Lang, first: KeyCode, last: KeyCode) -> String {
    lang.format(
        "keys.cap.range",
        &[
            ("first", key_label(lang, first).as_str()),
            ("last", key_label(lang, last).as_str()),
        ],
    )
}

/// Une liaison telle que l'encart la montre.
struct Bound {
    label: String,
    /// Souris ou molette : sans sens.
    pointer: bool,
    dir: Dir,
}

/// L'étiquette et le sens d'une liaison ; `None` pour ce que l'encart ne montre pas (manette,
/// entrées personnalisées).
fn bound(
    lang: &Lang,
    binding: &Binding,
    negate: Option<&Negate>,
    swizzle: Option<&SwizzleAxis>,
) -> Option<Bound> {
    let (label, pointer) = match *binding {
        Binding::Keyboard { key, mod_keys } => {
            (with_mods(lang, mod_keys, key_label(lang, key)), false)
        }
        Binding::MouseButton { button, mod_keys } => {
            (with_mods(lang, mod_keys, mouse_label(lang, button)), false)
        }
        Binding::MouseMotion { .. } => (lang.get("keys.cap.mouse"), true),
        Binding::MouseWheel { .. } => (lang.get("keys.cap.wheel"), true),
        _ => return None,
    };
    Some(Bound {
        label,
        pointer,
        dir: direction(negate, swizzle),
    })
}

/// Le sens d'un bouton : l'axe où le modificateur range sa valeur, puis son signe (mêmes règles
/// que `SwizzleAxis` et `Negate` pour une entrée à une dimension).
fn direction(negate: Option<&Negate>, swizzle: Option<&SwizzleAxis>) -> Dir {
    let axis = match swizzle {
        Some(SwizzleAxis::YXZ | SwizzleAxis::ZXY) => 1,
        Some(SwizzleAxis::ZYX | SwizzleAxis::YZX | SwizzleAxis::YYX | SwizzleAxis::ZZX) => 2,
        _ => 0,
    };
    let negative = negate.is_some_and(|n| [n.x, n.y, n.z][axis]);
    match (axis, negative) {
        (0, false) => Dir::PosX,
        (0, true) => Dir::NegX,
        (1, false) => Dir::PosY,
        (1, true) => Dir::NegY,
        (_, false) => Dir::PosZ,
        (_, true) => Dir::NegZ,
    }
}

/// Liste d'écran : la façon de ranger les touches d'une action, d'après leurs sens.
fn guess_part(bound: &[Bound]) -> Part {
    let dirs: Vec<Dir> = bound.iter().filter(|b| !b.pointer).map(|b| b.dir).collect();
    let has = |d: Dir| dirs.contains(&d);
    let depth = has(Dir::PosZ) || has(Dir::NegZ);
    if !depth && (has(Dir::PosY) || has(Dir::NegY)) {
        Part::Cardinal
    } else if !depth && has(Dir::NegX) {
        Part::Axis
    } else {
        Part::All
    }
}

/// L'étiquette d'une touche physique, telle qu'elle est : disposition du clavier (`Lang::keys`),
/// sinon nom de la langue (`key.Space`, options), sinon nom de l'encart (`keys.cap.ArrowUp`),
/// sinon la lettre ou le chiffre de sa position. Pas de remappage : la liaison porte déjà la
/// touche choisie.
fn key_label(lang: &Lang, key: KeyCode) -> String {
    let code = format!("{key:?}");
    if let Some(label) = lang.keys.get(&code) {
        return label.clone();
    }
    for prefix in ["key.", "keys.cap."] {
        if let Some(name) = lang.strings.get(&format!("{prefix}{code}")) {
            return name.clone();
        }
    }
    code.strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))
        .unwrap_or(&code)
        .to_owned()
}

fn mouse_label(lang: &Lang, button: MouseButton) -> String {
    lang.get(match button {
        MouseButton::Left => "keys.cap.mouse_left",
        MouseButton::Right => "keys.cap.mouse_right",
        MouseButton::Middle => "keys.cap.mouse_middle",
        _ => "keys.cap.mouse_other",
    })
}

/// Une combinaison (Ctrl + touche) : les modificateurs devant.
fn with_mods(lang: &Lang, mods: ModKeys, label: String) -> String {
    let mut out = label;
    for (flag, key) in [
        (ModKeys::SUPER, KeyCode::SuperLeft),
        (ModKeys::ALT, KeyCode::AltLeft),
        (ModKeys::SHIFT, KeyCode::ShiftLeft),
        (ModKeys::CONTROL, KeyCode::ControlLeft),
    ] {
        if mods.contains(flag) {
            out = lang.format(
                "keys.cap.combo",
                &[
                    ("mod", key_label(lang, key).as_str()),
                    ("key", out.as_str()),
                ],
            );
        }
    }
    out
}

fn push_unique(out: &mut Vec<String>, label: String) {
    if !out.contains(&label) {
        out.push(label);
    }
}

// --- L'encart ----------------------------------------------------------------------------------

/// Place de l'encart : sous le bandeau du lieu, à gauche ; petit (la demande de l'auteur), il
/// laisse libres le réticule, les conseils (centrés en haut) et l'essentiel de la vue. Sa largeur
/// est celle de sa plus longue ligne (≈ 260 px avec les touches par défaut : libellés courts,
/// jamais de retour à la ligne) ; une seconde touche ou un nom de touche long l'élargit plutôt
/// que de déborder du cadre.
const PANEL_TOP: f32 = BANNER_BOTTOM + 12.0;
/// Opacité de la plaquette (fond et barre ambre). Son fond peint n'est opaque qu'aux deux tiers
/// (`tools/ui/frames.py`) : une ligne se lisait mal devant un bandeau lumineux (M9). L'encart en
/// pose donc une seconde dessous ([`build_panel`]) : ≈ 0,88 d'opacité, même découpe, même barre.
const PANEL_ALPHA: f32 = 1.0;
/// Taille des libellés.
const LABEL_SIZE: f32 = 14.0;
/// Au-dessus du HUD ; une liste d'écran passe au-dessus des voiles des menus et du terminal (20),
/// des messages (25) et des options (30), sous l'écran de mort (50).
const PANEL_Z: i32 = 4;
const PANEL_Z_SCREEN: i32 = 32;

/// La place que l'encart prend à gauche d'un écran, marge comprise : un écran large (les listes
/// du terminal) se décale d'autant quand la fenêtre est trop étroite pour qu'il tienne au centre
/// sans passer dessous (≈ 1 600 px de large). Les libellés des écrans sont courts.
pub const SCREEN_RESERVE: f32 = 310.0;

#[derive(Component)]
struct KeysPanel;

/// Ce que montre l'encart : il n'est reconstruit que quand cela change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PanelContent {
    /// Affiché du tout (rien dans un menu sans liste d'écran).
    shown: bool,
    /// Liste d'un écran : au-dessus des voiles.
    screen: bool,
    expanded: bool,
    title: String,
    /// Les touches qui le replient ou le déplient.
    toggle: Vec<String>,
    /// Les lignes (vides quand il est replié).
    rows: Vec<PanelRow>,
}

/// Une ligne affichée : les étiquettes des touches, le libellé, et si c'est une action
/// d'interface (sous un filet, plus discrète).
#[derive(Clone, Debug, PartialEq, Eq)]
struct PanelRow {
    caps: Vec<String>,
    label: String,
    interface: bool,
}

#[derive(Default)]
struct PanelState {
    /// L'encart pour lequel `built` vaut (un nouvel encart, nouvelle partie, repart de rien).
    panel: Option<Entity>,
    built: Option<PanelContent>,
    tutorial: bool,
    /// Les entités porteuses de [`KeyHelp`], de la plus ancienne à la plus récente.
    screens: Vec<Entity>,
}

/// Ce qui peut changer le contenu de l'encart.
#[derive(SystemParam)]
struct PanelChanges<'w, 's> {
    mode: Res<'w, State<ControlMode>>,
    bindings: Query<'w, 's, (), Changed<Binding>>,
    removed_bindings: RemovedComponents<'w, 's, Binding>,
    contexts: Query<
        'w,
        's,
        (),
        Or<(
            Changed<ContextActivity<OnFoot>>,
            Changed<ContextActivity<Piloting>>,
            Changed<ContextActivity<MapControls>>,
            Changed<ContextActivity<MenuControls>>,
        )>,
    >,
    added_screens: Query<'w, 's, Entity, Added<KeyHelp>>,
    changed_screens: Query<'w, 's, (), Changed<KeyHelp>>,
    removed_screens: RemovedComponents<'w, 's, KeyHelp>,
}

/// L'encart d'une partie (`IN_GAME`) ou du menu principal : chacun part avec son état.
fn spawn_panel<const IN_GAME: bool>(mut commands: Commands, fonts: Res<UiFonts>) {
    let mut frame = fonts.frame(Frame::Plate);
    frame.color = Color::WHITE.with_alpha(PANEL_ALPHA);
    let scope = if IN_GAME {
        DespawnOnExit(AppState::InGame)
    } else {
        DespawnOnExit(AppState::MainMenu)
    };
    commands.spawn((
        Name::new("Encart des touches"),
        KeysPanel,
        scope,
        // Rien n'y retient un clic : la carte galactique reste cliquable dessous.
        Pickable::IGNORE,
        GlobalZIndex(PANEL_Z),
        Node {
            position_type: PositionType::Absolute,
            left: px(20),
            top: px(PANEL_TOP),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            // Large à gauche : la barre ambre de la plaquette ; de l'air à droite, que le texte
            // de l'en-tête ne touche pas le bord.
            padding: UiRect::new(px(14), px(16), px(6), px(7)),
            ..default()
        },
        frame,
        Visibility::Hidden,
    ));
}

#[allow(clippy::too_many_arguments)]
fn refresh_panel(
    mut commands: Commands,
    texts: Res<Texts>,
    fonts: Res<UiFonts>,
    visible: Res<KeyHelpVisible>,
    settings: Res<UserSettings>,
    tutorial: Option<Res<Tutorial>>,
    view: InputView,
    mut changes: PanelChanges,
    screens: Query<&KeyHelp>,
    mut panel: Query<(Entity, &mut Visibility, &mut GlobalZIndex), With<KeysPanel>>,
    mut state: Local<PanelState>,
) {
    // Toujours lus : des événements de retrait non lus resteraient en attente.
    let removed_bindings = changes.removed_bindings.read().count() > 0;
    let mut screens_changed = false;
    for entity in changes.removed_screens.read() {
        state.screens.retain(|e| *e != entity);
        screens_changed = true;
    }
    for entity in &changes.added_screens {
        state.screens.retain(|e| *e != entity);
        state.screens.push(entity);
        screens_changed = true;
    }
    let Ok((entity, mut visibility, mut z)) = panel.single_mut() else {
        return;
    };
    if state.panel != Some(entity) {
        state.panel = Some(entity);
        state.built = None;
    }
    let tutorial_active = crate::tutorial::is_active(&settings, tutorial.as_deref());
    let dirty = state.built.is_none()
        || removed_bindings
        || screens_changed
        || !changes.bindings.is_empty()
        || !changes.contexts.is_empty()
        || !changes.changed_screens.is_empty()
        || changes.mode.is_changed()
        || texts.is_changed()
        || visible.is_changed()
        || tutorial_active != state.tutorial;
    if !dirty {
        return;
    }
    state.tutorial = tutorial_active;
    let screen = state
        .screens
        .iter()
        .rev()
        .find_map(|e| screens.get(*e).ok());
    let content = panel_content(&view, &texts, visible.0, tutorial_active, screen);
    if state.built.as_ref() == Some(&content) {
        return;
    }
    let wanted = if content.shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *visibility != wanted {
        *visibility = wanted;
    }
    let wanted_z = if content.screen {
        PANEL_Z_SCREEN
    } else {
        PANEL_Z
    };
    if z.0 != wanted_z {
        z.0 = wanted_z;
    }
    build_panel(&mut commands, entity, &fonts, &texts, &content);
    state.built = Some(content);
}

/// Le contenu de l'encart : la liste d'un écran s'il y en a une, sinon les lignes des contextes
/// actifs dont une action a une touche.
fn panel_content(
    view: &InputView,
    texts: &Texts,
    expanded: bool,
    tutorial: bool,
    screen: Option<&KeyHelp>,
) -> PanelContent {
    let lang: &Lang = texts;
    let mut toggle = Vec::new();
    view.source_caps(
        lang,
        &KeySource::Action(type_name::<ToggleKeyHelp>()),
        &mut toggle,
    );
    let mut rows: Vec<PanelRow> = Vec::new();
    let (shown, title) = match screen {
        Some(help) => {
            for hint in &help.hints {
                let mut caps = Vec::new();
                for source in &hint.keys {
                    view.source_caps(lang, source, &mut caps);
                }
                rows.push(PanelRow {
                    caps,
                    label: hint.label.clone(),
                    interface: false,
                });
            }
            let title = help
                .title
                .clone()
                .unwrap_or_else(|| texts.get("keys.panel.title"));
            (true, title)
        }
        None => {
            for (table, actions) in view.active_tables() {
                for row in table {
                    if row.when == When::Tutorial && !tutorial {
                        continue;
                    }
                    let mut caps = Vec::new();
                    for (name, part) in row.keys {
                        let name = name();
                        for action in actions.iter().filter(|a| view.named(**a, name)) {
                            view.action_caps(lang, *action, *part, &mut caps);
                        }
                    }
                    // Une action sans touche (tout remappé ailleurs, manette seule) : pas de ligne.
                    if caps.is_empty() {
                        continue;
                    }
                    let entry = PanelRow {
                        caps,
                        label: texts.get(row.label),
                        interface: row.interface,
                    };
                    if !rows.contains(&entry) {
                        rows.push(entry);
                    }
                }
            }
            // Les actions du contexte d'abord, l'interface ensuite (tri stable).
            rows.sort_by_key(|r| r.interface);
            (!rows.is_empty(), texts.get("keys.panel.title"))
        }
    };
    if !expanded {
        rows.clear();
    }
    PanelContent {
        shown,
        screen: screen.is_some(),
        expanded,
        title,
        toggle,
        rows,
    }
}

/// Reconstruit les enfants de l'encart : l'en-tête (titre, touche de repli), puis, déplié, une
/// ligne par entrée (les touches, puis le libellé, sans retour à la ligne) ; les actions
/// d'interface viennent sous un filet, en plus discret.
fn build_panel(
    commands: &mut Commands,
    panel: Entity,
    fonts: &UiFonts,
    texts: &Texts,
    content: &PanelContent,
) {
    commands.entity(panel).despawn_children();
    commands.entity(panel).with_children(|ui| {
        // La seconde plaquette, sous le contenu, à la taille de l'encart (padding compris) : elle
        // ne prend aucune place dans la mise en page.
        let mut backing = fonts.frame(Frame::Plate);
        backing.color = Color::WHITE.with_alpha(PANEL_ALPHA);
        ui.spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
            backing,
        ));
        ui.spawn((
            Pickable::IGNORE,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
        ))
        .with_children(|header| {
            let title_color = if content.expanded {
                theme::ACCENT
            } else {
                theme::TEXT_DIM
            };
            header.spawn((
                theme::heading(fonts, &content.title, 11.0, title_color),
                TextLayout::no_wrap(),
                Pickable::IGNORE,
            ));
            header.spawn((
                Pickable::IGNORE,
                Node {
                    flex_grow: 1.0,
                    min_width: px(10),
                    ..default()
                },
            ));
            for cap in &content.toggle {
                header.spawn(theme::keycap(fonts, cap));
            }
            let action = if content.expanded {
                "keys.panel.hide"
            } else {
                "keys.panel.show"
            };
            header.spawn((
                Text::new(texts.get(action)),
                fonts.body(11.0),
                TextColor(theme::TEXT_DIM),
                TextLayout::no_wrap(),
                Pickable::IGNORE,
            ));
        });
        if !content.expanded || content.rows.is_empty() {
            return;
        }
        ui.spawn((theme::hairline(theme::RULE), Pickable::IGNORE));
        ui.spawn((
            Pickable::IGNORE,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                ..default()
            },
        ))
        .with_children(|list| {
            let mut previous_context = false;
            for row in &content.rows {
                if row.interface && previous_context {
                    list.spawn((
                        Pickable::IGNORE,
                        Node {
                            width: percent(100),
                            height: px(1),
                            margin: UiRect::vertical(px(2)),
                            ..default()
                        },
                        BackgroundColor(theme::RULE.with_alpha(0.25)),
                    ));
                }
                previous_context = !row.interface;
                let color = if row.interface {
                    theme::TEXT_DIM
                } else {
                    theme::TEXT
                };
                list.spawn((
                    Pickable::IGNORE,
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        ..default()
                    },
                ))
                .with_children(|line| {
                    line.spawn((
                        Pickable::IGNORE,
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(2),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ))
                    .with_children(|keys| {
                        for cap in &row.caps {
                            keys.spawn(theme::keycap(fonts, cap));
                        }
                    });
                    line.spawn((
                        Text::new(row.label.clone()),
                        fonts.body(LABEL_SIZE),
                        TextColor(color),
                        // Jamais sur deux lignes : la touche resterait seule en haut.
                        TextLayout::no_wrap(),
                        Pickable::IGNORE,
                    ));
                });
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sens_des_liaisons() {
        // Les préréglages de `bevy_enhanced_input` : ZQSD (cardinal) et la poussée (spatial).
        assert_eq!(direction(None, None), Dir::PosX);
        assert_eq!(direction(Some(&Negate::all()), None), Dir::NegX);
        assert_eq!(direction(None, Some(&SwizzleAxis::YXZ)), Dir::PosY);
        assert_eq!(
            direction(Some(&Negate::all()), Some(&SwizzleAxis::YXZ)),
            Dir::NegY
        );
        assert_eq!(
            direction(Some(&Negate::all()), Some(&SwizzleAxis::ZYX)),
            Dir::NegZ
        );
    }

    #[test]
    fn etiquettes_physiques() {
        let mut lang = Lang::default();
        lang.strings.insert("key.Space".into(), "Espace".into());
        lang.strings.insert("keys.cap.ArrowUp".into(), "↑".into());
        lang.set_key("KeyQ", "A");
        // Un remappage des jetons de texte ne touche pas l'étiquette d'une liaison.
        lang.remaps.insert("KeyQ".into(), "KeyE".into());
        assert_eq!(key_label(&lang, KeyCode::KeyQ), "A");
        assert_eq!(key_label(&lang, KeyCode::Space), "Espace");
        assert_eq!(key_label(&lang, KeyCode::ArrowUp), "↑");
        assert_eq!(key_label(&lang, KeyCode::Digit1), "1");
        assert_eq!(key_label(&lang, KeyCode::F1), "F1");
    }

    #[test]
    fn plages_et_combinaisons() {
        let mut lang = Lang::default();
        lang.strings
            .insert("keys.cap.range".into(), "{first}–{last}".into());
        lang.strings
            .insert("keys.cap.combo".into(), "{mod}+{key}".into());
        lang.strings.insert("key.ShiftLeft".into(), "Maj".into());
        // Un remappage des jetons de texte ne touche pas les touches fixes des écrans.
        lang.remaps.insert("Digit1".into(), "KeyE".into());
        assert_eq!(range_label(&lang, KeyCode::Digit1, KeyCode::Digit9), "1–9");
        assert_eq!(
            chord_label(&lang, KeyCode::ShiftLeft, KeyCode::Tab),
            "Maj+Tab"
        );
        let hint = KeyHint::range(KeyCode::Digit1, KeyCode::Digit9, "Répondre")
            .and_mouse(MouseButton::Left)
            .and_wheel();
        assert_eq!(
            hint.keys,
            vec![
                KeySource::Range(KeyCode::Digit1, KeyCode::Digit9),
                KeySource::Mouse(MouseButton::Left),
                KeySource::Wheel
            ]
        );
    }

    #[test]
    fn liaisons_sans_touche_ignorees() {
        // Une case vide du menu des touches devient `Binding::None` ; la manette ne s'affiche pas.
        let lang = Lang::default();
        assert!(bound(&lang, &Binding::None, None, None).is_none());
        assert!(
            bound(
                &lang,
                &Binding::GamepadButton(GamepadButton::South),
                None,
                None
            )
            .is_none()
        );
        let key = Binding::Keyboard {
            key: KeyCode::KeyE,
            mod_keys: ModKeys::empty(),
        };
        assert_eq!(
            bound(&lang, &key, None, None).map(|b| b.label),
            Some("E".into())
        );
    }

    #[test]
    fn rangement_devine() {
        let b = |dir| Bound {
            label: String::new(),
            pointer: false,
            dir,
        };
        assert_eq!(guess_part(&[b(Dir::PosX)]), Part::All);
        assert_eq!(guess_part(&[b(Dir::PosX), b(Dir::NegX)]), Part::Axis);
        assert_eq!(
            guess_part(&[b(Dir::PosX), b(Dir::NegX), b(Dir::PosY), b(Dir::NegY)]),
            Part::Cardinal
        );
        assert_eq!(guess_part(&[b(Dir::NegZ), b(Dir::PosY)]), Part::All);
    }
}
