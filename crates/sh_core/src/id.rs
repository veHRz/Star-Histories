//! Identifiants typés.
//!
//! Un identifiant est une chaîne ASCII stable en `snake_case` (`concordia`, `vorrh_ii`,
//! `sys_hexis`), jamais affichée au joueur. Le paramètre de type empêche de confondre un
//! identifiant de faction avec un identifiant de lieu : `Id<Faction>` et `Id<Place>` sont
//! deux types distincts, sans coût à l'exécution.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Identifiant d'une entité de type `T` (le type n'est qu'un marqueur).
pub struct Id<T> {
    name: Arc<str>,
    _kind: PhantomData<fn() -> T>,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "identifiant invalide : « {0} » (attendu : ASCII minuscule, chiffres et « _ », commençant par une lettre)"
)]
pub struct IdError(pub String);

/// Vrai si `s` respecte la convention des identifiants (AGENTS.md §8.1).
pub fn is_valid_id(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !s.ends_with('_')
        && !s.contains("__")
}

impl<T> Id<T> {
    /// Crée un identifiant après vérification de la convention.
    pub fn new(name: &str) -> Result<Self, IdError> {
        if is_valid_id(name) {
            Ok(Self {
                name: Arc::from(name),
                _kind: PhantomData,
            })
        } else {
            Err(IdError(name.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// Change le marqueur de type (ex. un identifiant d'astre vu comme un identifiant de lieu).
    pub fn cast<U>(&self) -> Id<U> {
        Id {
            name: Arc::clone(&self.name),
            _kind: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            name: Arc::clone(&self.name),
            _kind: PhantomData,
        }
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.name)
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.name)
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Id::new(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Faction;
    struct Place;

    #[test]
    fn convention_des_identifiants() {
        for ok in ["concordia", "vorrh_ii", "sys_hexis", "p7", "nsu_liberte"] {
            assert!(is_valid_id(ok), "{ok}");
        }
        for faux in [
            "",
            "Concordia",
            "vorrh-ii",
            "_x",
            "x_",
            "a__b",
            "7k",
            "hélion",
            "a b",
        ] {
            assert!(!is_valid_id(faux), "{faux}");
        }
    }

    #[test]
    fn egalite_et_ordre() {
        let a: Id<Faction> = Id::new("empire").unwrap();
        let b: Id<Faction> = Id::new("ugl").unwrap();
        assert!(a < b);
        assert_eq!(a.clone(), a);
        let lieu: Id<Place> = a.cast();
        assert_eq!(lieu.as_str(), "empire");
    }

    #[test]
    fn serde_valide_la_convention() {
        let id: Id<Faction> = ron::from_str("\"hexa_corp\"").unwrap();
        assert_eq!(id.to_string(), "hexa_corp");
        assert!(ron::from_str::<Id<Faction>>("\"Hexa-Corp\"").is_err());
    }
}
