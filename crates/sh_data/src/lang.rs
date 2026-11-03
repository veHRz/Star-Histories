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
    /// Code de la langue (« en »).
    pub code: String,
    pub strings: BTreeMap<String, String>,
}

impl Lang {
    /// Le texte d'une clé. Une clé absente renvoie la clé elle-même entre crochets, pour
    /// qu'un oubli se voie à l'écran au lieu de planter (le test de cohérence l'attrape avant).
    pub fn get(&self, key: &str) -> String {
        self.strings
            .get(key)
            .cloned()
            .unwrap_or_else(|| format!("[{key}]"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cles_et_parametres() {
        let lang = Lang {
            code: "en".into(),
            strings: BTreeMap::from([("date".into(), "Day {day}, {year}".into())]),
        };
        assert_eq!(
            lang.format("date", &[("day", "12"), ("year", "425at")]),
            "Day 12, 425at"
        );
        assert_eq!(lang.get("absente"), "[absente]");
        assert!(lang.has("date"));
    }

    #[test]
    fn nombres_a_la_francaise() {
        let fr = Lang {
            code: "fr".into(),
            strings: BTreeMap::from([
                ("number.group".into(), "\u{202f}".into()),
                ("number.decimal".into(), ",".into()),
            ]),
        };
        assert_eq!(fr.number(1_234_567.891, 2), "1\u{202f}234\u{202f}567,89");
        assert_eq!(fr.number(-2.5, 1), "-2,5");
        assert_eq!(fr.number(-0.2, 0), "0");
        assert_eq!(fr.number(999.0, 0), "999");
        assert_eq!(Lang::default().number(12_500.0, 0), "12,500");
    }
}
