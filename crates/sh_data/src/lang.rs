//! Textes visibles du joueur, par langue (`assets/data/lang/`).
//!
//! Décision de l'auteur (`docs/QUESTIONS.md` Q8 puis Q11) : le jeu est en français, la langue
//! de ses noms propres ; tous ses textes vivent ici pour pouvoir être traduits (`en` est
//! tenu à jour en parallèle). Le code n'écrit jamais de phrase : il demande une clé
//! (`menu.new_game`) et remplit les paramètres nommés (`{name}`). Les nombres suivent la
//! langue ([`Lang::number`]).
//!
//! **Fichiers d'une langue** ([`load_lang`], M9) : la langue `<code>` est la fusion de
//! - son fichier commun `lang/<code>.ron`, au format complet `(code: "fr", strings: { "clé":
//!   "texte", … })` : menus, nombres, noms tirés des données, tout ce qui n'a pas de domaine ;
//! - ses fichiers de domaine `lang/<code>/*.ron`, lus par ordre de nom, au format allégé : un
//!   simple dictionnaire `{ "clé": "texte", … }` (options, HUD, terminal, carte, monde, salles…).
//!
//! Un domaine a son fichier pour que deux lots de travail n'écrivent jamais dans le même ; par
//! convention, `fr` et `en` rangent une clé dans le fichier de même nom. Une clé définie deux
//! fois, dans deux fichiers ou dans le même, est gardée à sa première définition et notée
//! ([`Lang::duplicates`]) : la validation refuse alors les données en citant les deux fichiers.
//! `en` a exactement les mêmes clés que `fr`, tous fichiers confondus (test de `sh_data`).

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{DataError, read_ron};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Lang {
    /// Code de la langue (« fr »).
    pub code: String,
    pub strings: BTreeMap<String, String>,
    /// Fichier d'origine de chaque clé, relatif au dossier de données (`lang/fr/hud.ron`) :
    /// pour situer un problème. Rempli par [`load_lang`].
    #[serde(skip)]
    pub origins: BTreeMap<String, String>,
    /// Clés définies plus d'une fois : la première définition l'emporte, la validation refuse.
    #[serde(skip)]
    pub duplicates: Vec<DuplicateKey>,
    /// Étiquettes des touches physiques selon la disposition du clavier (`KeyQ` → « A » en
    /// AZERTY). Remplies par le jeu ([`Lang::set_key`]), jamais lues dans le fichier.
    #[serde(skip)]
    pub keys: BTreeMap<String, String>,
    /// Touches remappées par le joueur (options) : `KeyE` → `KeyF`. Les jetons citent la touche
    /// d'origine, l'étiquette affichée est celle de la touche choisie.
    #[serde(skip)]
    pub remaps: BTreeMap<String, String>,
}

/// Une clé de texte définie deux fois (fichiers relatifs au dossier de données ; les deux sont
/// égaux si le doublon est dans un même fichier).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateKey {
    pub key: String,
    pub first: String,
    pub second: String,
}

/// Les dispositions de clavier connues (`gameplay/keyboard.ron`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Keyboard {
    pub default_layout: String,
    pub layouts: BTreeMap<String, BTreeMap<String, String>>,
}

/// Les entrées d'un dictionnaire de textes, dans l'ordre du fichier et doublons compris : un
/// `BTreeMap` garderait la dernière définition sans rien dire.
struct Entries(Vec<(String, String)>);

impl<'de> Deserialize<'de> for Entries {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;

        impl<'de> serde::de::Visitor<'de> for EntriesVisitor {
            type Value = Entries;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("un dictionnaire { \"clé\": \"texte\" }")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Entries, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, String>()? {
                    entries.push(entry);
                }
                Ok(Entries(entries))
            }
        }

        deserializer.deserialize_map(EntriesVisitor)
    }
}

/// Le fichier commun d'une langue (`lang/<code>.ron`).
#[derive(Deserialize)]
struct LangFile {
    code: String,
    strings: Entries,
}

/// Charge la langue `code` : son fichier commun puis ses fichiers de domaine (voir l'en-tête
/// du module). Un dossier de domaine absent n'est pas une erreur.
pub fn load_lang(data_dir: &Path, code: &str) -> Result<Lang, DataError> {
    let common = format!("{}/{code}.ron", crate::paths::LANG_DIR);
    let file: LangFile = read_ron(&data_dir.join(&common))?;
    let mut lang = Lang {
        code: file.code,
        ..Lang::default()
    };
    lang.absorb(&common, file.strings.0);
    let dir = format!("{}/{code}", crate::paths::LANG_DIR);
    for (rel, path) in domain_files(&data_dir.join(&dir), &dir) {
        let entries: Entries = read_ron(&path)?;
        lang.absorb(&rel, entries.0);
    }
    Ok(lang)
}

/// Les `*.ron` d'un dossier de domaine, triés par nom, avec leur chemin relatif `rel/nom.ron`.
fn domain_files(dir: &Path, rel: &str) -> Vec<(String, PathBuf)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "ron"))
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_owned();
            Some((format!("{rel}/{name}"), path))
        })
        .collect();
    files.sort();
    files
}

impl Lang {
    /// Ajoute les entrées d'un fichier ; une clé déjà connue est notée en double, pas remplacée.
    fn absorb(&mut self, file: &str, entries: Vec<(String, String)>) {
        for (key, text) in entries {
            if let Some(first) = self.origins.get(&key) {
                self.duplicates.push(DuplicateKey {
                    first: first.clone(),
                    second: file.to_owned(),
                    key,
                });
                continue;
            }
            self.origins.insert(key.clone(), file.to_owned());
            self.strings.insert(key, text);
        }
    }

    /// Le texte d'une clé. Une clé absente renvoie la clé elle-même entre crochets, pour
    /// qu'un oubli se voie à l'écran au lieu de planter (le test de cohérence l'attrape avant).
    /// Les jetons de touche (`[KeyE]`) y sont remplacés par l'étiquette de la touche
    /// ([`Lang::key_label`]).
    pub fn get(&self, key: &str) -> String {
        match self.strings.get(key) {
            Some(text) if text.contains('[') => self.with_keys(text),
            Some(text) => text.clone(),
            None => format!("[{key}]"),
        }
    }

    /// L'étiquette d'une touche physique (nom `KeyCode`) : celle de la disposition, sinon le
    /// nom de la langue (`key.Space`), sinon la lettre ou le chiffre de sa position.
    pub fn key_label(&self, code: &str) -> String {
        let code = self.remaps.get(code).map_or(code, String::as_str);
        if let Some(label) = self.keys.get(code) {
            return label.clone();
        }
        if let Some(name) = self.strings.get(&format!("key.{code}")) {
            return name.clone();
        }
        code.strip_prefix("Key")
            .or_else(|| code.strip_prefix("Digit"))
            .unwrap_or(code)
            .to_owned()
    }

    /// Fixe l'étiquette d'une touche ; `true` si elle a changé.
    pub fn set_key(&mut self, code: &str, label: &str) -> bool {
        if self.keys.get(code).map(String::as_str) == Some(label) {
            return false;
        }
        self.keys.insert(code.to_owned(), label.to_owned());
        true
    }

    /// Remplace les jetons `[NomDeTouche]` d'un texte (nom `KeyCode` : `[KeyE]`, `[Space]`).
    fn with_keys(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find('[') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            match after.find(']') {
                Some(end) if is_key_name(&after[..end]) => {
                    out.push_str(&self.key_label(&after[..end]));
                    rest = &after[end + 1..];
                }
                _ => {
                    out.push('[');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        out
    }

    pub fn has(&self, key: &str) -> bool {
        self.strings.contains_key(key)
    }

    /// Un nombre à `decimals` décimales, avec les séparateurs de la langue (clés
    /// `number.group` et `number.decimal` : « 12 500,5 » en français).
    pub fn number(&self, value: f64, decimals: usize) -> String {
        let group = self.strings.get("number.group").map_or(",", String::as_str);
        let point = self
            .strings
            .get("number.decimal")
            .map_or(".", String::as_str);
        let text = format!("{:.*}", decimals, value.abs());
        let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
        let mut out = String::new();
        // « -0 » n'existe pas : le signe ne compte que si le nombre arrondi n'est pas nul.
        if value < 0.0 && text.chars().any(|c| c.is_ascii_digit() && c != '0') {
            out.push('-');
        }
        for (i, c) in int.chars().enumerate() {
            if i > 0 && (int.len() - i).is_multiple_of(3) {
                out.push_str(group);
            }
            out.push(c);
        }
        if !frac.is_empty() {
            out.push_str(point);
            out.push_str(frac);
        }
        out
    }

    /// Le texte d'une clé, paramètres nommés remplacés (`{name}` → valeur).
    pub fn format(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut text = self.get(key);
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }
}

/// Un nom de touche `KeyCode` : une majuscule, puis des lettres et des chiffres ASCII.
fn is_key_name(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cles_et_parametres() {
        let lang = Lang {
            code: "en".into(),
            strings: BTreeMap::from([("date".into(), "Day {day}, {year}".into())]),
            ..Lang::default()
        };
        assert_eq!(
            lang.format("date", &[("day", "12"), ("year", "425at")]),
            "Day 12, 425at"
        );
        assert_eq!(lang.get("absente"), "[absente]");
        assert!(lang.has("date"));
    }

    #[test]
    fn touches_selon_la_disposition() {
        let mut lang = Lang {
            code: "fr".into(),
            strings: BTreeMap::from([
                (
                    "aide".into(),
                    "[KeyW][KeyA][KeyS][KeyD], [Space] : bouger [sic]".into(),
                ),
                ("key.Space".into(), "Espace".into()),
            ]),
            ..Lang::default()
        };
        assert_eq!(lang.get("aide"), "WASD, Espace : bouger [sic]");
        assert!(lang.set_key("KeyW", "Z"));
        assert!(lang.set_key("KeyA", "Q"));
        assert!(!lang.set_key("KeyA", "Q"));
        assert_eq!(lang.get("aide"), "ZQSD, Espace : bouger [sic]");
        assert_eq!(lang.key_label("Digit1"), "1");
        // Remappée par le joueur : le jeton d'origine affiche la touche choisie.
        lang.remaps.insert("KeyW".into(), "KeyA".into());
        assert_eq!(lang.get("aide"), "QQSD, Espace : bouger [sic]");
    }

    /// Le fichier commun et les fichiers de domaine se fusionnent ; un doublon est noté avec
    /// ses deux fichiers, et la première définition reste.
    #[test]
    fn fusion_des_fichiers_de_domaine() {
        let dir = std::env::temp_dir().join(format!("sh_lang_{}", std::process::id()));
        let write = |rel: &str, text: &str| {
            let path = dir.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{e}"));
            }
            fs::write(&path, text).unwrap_or_else(|e| panic!("{e}"));
        };
        write(
            "lang/xx.ron",
            r#"(code: "xx", strings: { "menu.a": "A", "menu.b": "B" })"#,
        );
        write("lang/xx/hud.ron", r#"{ "hud.c": "C", "menu.a": "autre", }"#);
        write(
            "lang/xx/world.ron",
            r#"{ "world.d": "D", "world.d": "encore" }"#,
        );
        write("lang/xx/notes.txt", "pas une langue");
        let lang = load_lang(&dir, "xx").unwrap_or_else(|e| panic!("{e}"));
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(lang.code, "xx");
        assert_eq!(lang.strings.len(), 4);
        assert_eq!(lang.get("menu.a"), "A");
        assert_eq!(lang.get("hud.c"), "C");
        assert_eq!(lang.get("world.d"), "D");
        assert_eq!(
            lang.origins.get("hud.c").map(String::as_str),
            Some("lang/xx/hud.ron")
        );
        assert_eq!(
            lang.duplicates,
            vec![
                DuplicateKey {
                    key: "menu.a".into(),
                    first: "lang/xx.ron".into(),
                    second: "lang/xx/hud.ron".into(),
                },
                DuplicateKey {
                    key: "world.d".into(),
                    first: "lang/xx/world.ron".into(),
                    second: "lang/xx/world.ron".into(),
                },
            ]
        );
    }

    #[test]
    fn nombres_a_la_francaise() {
        let fr = Lang {
            code: "fr".into(),
            strings: BTreeMap::from([
                ("number.group".into(), "\u{202f}".into()),
                ("number.decimal".into(), ",".into()),
            ]),
            ..Lang::default()
        };
        assert_eq!(fr.number(1_234_567.891, 2), "1\u{202f}234\u{202f}567,89");
        assert_eq!(fr.number(-2.5, 1), "-2,5");
        assert_eq!(fr.number(-0.2, 0), "0");
        assert_eq!(fr.number(999.0, 0), "999");
        assert_eq!(Lang::default().number(12_500.0, 0), "12,500");
    }
}
