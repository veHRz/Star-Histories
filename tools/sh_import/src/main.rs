//! `cargo run -p sh_import` : régénère `assets/data/canon/galaxy.ron` depuis la carte canon.
//!
//! Options :
//! - `--check` : n'écrit rien, échoue si le fichier versionné n'est pas à jour ;
//! - `--input <json>` / `--output <ron>` : autres chemins (relatifs à la racine du dépôt).

use std::path::PathBuf;
use std::process::ExitCode;

use sh_import::{DEFAULT_INPUT, DEFAULT_OUTPUT, import, workspace_root};

fn main() -> ExitCode {
    let mut check = false;
    let root = workspace_root();
    let mut input = root.join(DEFAULT_INPUT);
    let mut output = root.join(DEFAULT_OUTPUT);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--input" | "--output" => {
                let Some(value) = args.next() else {
                    eprintln!("sh_import : {arg} attend un chemin");
                    return ExitCode::FAILURE;
                };
                let path = root.join(PathBuf::from(value));
                if arg == "--input" {
                    input = path;
                } else {
                    output = path;
                }
            }
            other => {
                eprintln!("sh_import : option inconnue « {other} » (--check, --input, --output)");
                return ExitCode::FAILURE;
            }
        }
    }

    let json = match std::fs::read_to_string(&input) {
        Ok(json) => json,
        Err(err) => {
            eprintln!(
                "sh_import : lecture de {} impossible : {err}",
                input.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let ron = match import(&json) {
        Ok(ron) => ron,
        Err(err) => {
            eprintln!("sh_import : {err}");
            return ExitCode::FAILURE;
        }
    };

    if check {
        let current = std::fs::read_to_string(&output).unwrap_or_default();
        if current == ron {
            println!("sh_import : {} est à jour", output.display());
            return ExitCode::SUCCESS;
        }
        eprintln!(
            "sh_import : {} n'est pas à jour, relancer `cargo run -p sh_import`",
            output.display()
        );
        return ExitCode::FAILURE;
    }

    if let Some(dir) = output.parent()
        && let Err(err) = std::fs::create_dir_all(dir)
    {
        eprintln!(
            "sh_import : création de {} impossible : {err}",
            dir.display()
        );
        return ExitCode::FAILURE;
    }
    if let Err(err) = std::fs::write(&output, &ron) {
        eprintln!(
            "sh_import : écriture de {} impossible : {err}",
            output.display()
        );
        return ExitCode::FAILURE;
    }
    println!(
        "sh_import : {} écrit ({} octets)",
        output.display(),
        ron.len()
    );
    ExitCode::SUCCESS
}
