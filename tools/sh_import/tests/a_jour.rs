//! L'import est rejouable à l'identique : le fichier versionné est exactement ce que produit
//! l'outil à partir de la carte canon actuelle.

use sh_import::{DEFAULT_INPUT, DEFAULT_OUTPUT, import, workspace_root};

#[test]
fn galaxy_ron_est_a_jour() {
    let root = workspace_root();
    let json = std::fs::read_to_string(root.join(DEFAULT_INPUT)).expect("carte canon lisible");
    let attendu = import(&json).expect("import sans erreur");
    let actuel = std::fs::read_to_string(root.join(DEFAULT_OUTPUT)).unwrap_or_default();
    assert!(
        actuel == attendu,
        "{DEFAULT_OUTPUT} n'est pas à jour : relancer `cargo run -p sh_import`"
    );
    // Deux imports successifs donnent le même octet pour octet.
    assert_eq!(import(&json).expect("second import"), attendu);
}
