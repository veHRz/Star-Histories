//! Textes visibles du joueur, par langue (`assets/data/lang/<code>.ron`).
//!
//! Décision de l'auteur (`docs/QUESTIONS.md` Q8) : le jeu est en anglais pour commencer, et
//! tous ses textes vivent ici pour pouvoir être traduits. Le code n'écrit jamais de phrase :
//! il demande une clé (`menu.new_game`) et remplit les paramètres nommés (`{name}`).

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
}
