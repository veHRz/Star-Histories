//! Textes visibles du joueur, par langue (`assets/data/lang/<code>.ron`).
//!
//! Décision de l'auteur (`docs/QUESTIONS.md` Q8 puis Q11) : le jeu est en français, la langue
//! de ses noms propres ; tous ses textes vivent ici pour pouvoir être traduits (`en.ron` est
//! tenu à jour en parallèle). Le code n'écrit jamais de phrase : il demande une clé
//! (`menu.new_game`) et remplit les paramètres nommés (`{name}`). Les nombres suivent la
//! langue ([`Lang::number`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Lang {
    /// Code de la langue (« fr »).
    pub code: String,
    pub strings: BTreeMap<String, String>,
    /// Étiquettes des touches physiques selon la disposition du clavier (`KeyQ` → « A » en
    /// AZERTY). Remplies par le jeu ([`Lang::set_key`]), jamais lues dans le fichier.
    #[serde(skip)]
    pub keys: BTreeMap<String, String>,
}

/// Les dispositions de clavier connues (`gameplay/keyboard.ron`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Keyboard {
    pub default_layout: String,
    pub layouts: BTreeMap<String, BTreeMap<String, String>>,
}

impl Lang {
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
