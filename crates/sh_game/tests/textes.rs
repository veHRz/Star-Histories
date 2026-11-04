//! Chaque clé de texte citée dans le code de `sh_game` existe dans le fichier de langue :
//! un oubli se voit à `cargo t`, pas à l'écran.

use std::path::{Path, PathBuf};

fn sources(dossier: &Path) -> Vec<PathBuf> {
    let mut fichiers = Vec::new();
    for entree in std::fs::read_dir(dossier).into_iter().flatten().flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            fichiers.extend(sources(&chemin));
        } else if chemin.extension().is_some_and(|e| e == "rs") {
            fichiers.push(chemin);
        }
    }
    fichiers
}

/// Une clé de texte : segments ASCII minuscules séparés par des points (`menu.new_game`).
fn ressemble_a_une_cle(s: &str) -> bool {
    // Les noms de fichiers d'assets ne sont pas des clés.
    const FICHIERS: [&str; 6] = [".ron", ".ttf", ".png", ".gltf", ".glb", ".bin"];
    s.contains('.')
        && !FICHIERS.iter().any(|ext| s.ends_with(ext))
        && s.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
        })
}

#[test]
fn toutes_les_cles_existent() {
    let manifeste = Path::new(env!("CARGO_MANIFEST_DIR"));
    let racine = manifeste.ancestors().nth(2).unwrap_or(manifeste);
    let langue: sh_data::Lang =
        sh_data::read_ron(&racine.join("assets/data").join(sh_data::paths::LANG))
            .unwrap_or_else(|err| panic!("{err}"));

    let mut manquantes = Vec::new();
    let mut vues = 0;
    for fichier in sources(&manifeste.join("src")) {
        let code = std::fs::read_to_string(&fichier).unwrap_or_default();
        // Les littéraux de chaîne sont les segments impairs entre guillemets.
        for (i, litteral) in code.split('"').enumerate() {
            if i % 2 == 1 && ressemble_a_une_cle(litteral) {
                vues += 1;
                if !langue.has(litteral) {
                    manquantes.push(format!("{} : {litteral}", fichier.display()));
                }
            }
        }
    }
    assert!(
        vues > 10,
        "aucune clé trouvée : le test ne voit plus le code ({vues})"
    );
    assert!(
        manquantes.is_empty(),
        "clés absentes de fr.ron : {manquantes:?}"
    );
}
