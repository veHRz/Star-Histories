//! Autopilote de développement, piloté par variables d'environnement.
//!
//! Permet à un agent IA (ou à un script) de lancer le jeu, de vérifier qu'il tourne
//! sans panique et d'en récupérer une capture d'écran, sans intervention humaine :
//!
//! - `SH_EXIT_AFTER_FRAMES=N` : quitte proprement après N frames.
//! - `SH_SCREENSHOT=chemin.png` : enregistre une capture puis quitte.
//! - `SH_SCREENSHOT_FRAME=N` : frame de la capture (défaut : 90, le temps que la scène se charge).
//!
//! Sans aucune de ces variables, le plugin ne fait rien.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Capturing, Screenshot, save_to_disk};

/// Frames d'attente maximale après la demande de capture avant d'abandonner.
const SCREENSHOT_TIMEOUT_FRAMES: u32 = 600;

pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        let exit_after = env_u32("SH_EXIT_AFTER_FRAMES");
        let screenshot = std::env::var_os("SH_SCREENSHOT").map(|path| PendingScreenshot {
            path: PathBuf::from(path),
            at_frame: env_u32("SH_SCREENSHOT_FRAME").unwrap_or(90),
            requested: false,
        });
        if exit_after.is_none() && screenshot.is_none() {
            return;
        }
        if let Some(shot) = &screenshot {
            // Une ancienne capture ferait croire que la nouvelle est déjà écrite.
            let _ = std::fs::remove_file(&shot.path);
            if let Some(dir) = shot.path.parent().filter(|dir| !dir.as_os_str().is_empty())
                && let Err(err) = std::fs::create_dir_all(dir)
            {
                error!("autopilote : impossible de créer {} : {err}", dir.display());
            }
        }
        app.insert_resource(Autopilot {
            frame: 0,
            exit_after,
            screenshot,
        })
        .add_systems(Last, run_autopilot);
    }
}

#[derive(Resource)]
struct Autopilot {
    frame: u32,
    exit_after: Option<u32>,
    screenshot: Option<PendingScreenshot>,
}

struct PendingScreenshot {
    path: PathBuf,
    at_frame: u32,
    requested: bool,
}

fn run_autopilot(
    mut commands: Commands,
    mut autopilot: ResMut<Autopilot>,
    capturing: Query<(), With<Capturing>>,
    mut exit: MessageWriter<AppExit>,
) {
    autopilot.frame += 1;
    let frame = autopilot.frame;

    if let Some(shot) = autopilot.screenshot.as_mut() {
        if !shot.requested && frame >= shot.at_frame {
            info!("autopilote : capture d'écran vers {}", shot.path.display());
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(shot.path.clone()));
            shot.requested = true;
        } else if shot.requested {
            let written = shot.path.exists() && capturing.is_empty();
            if written && frame >= shot.at_frame + 5 {
                info!("autopilote : capture écrite, fermeture");
                exit.write(AppExit::Success);
                return;
            }
            if frame >= shot.at_frame + SCREENSHOT_TIMEOUT_FRAMES {
                error!("autopilote : la capture n'a pas été écrite à temps");
                exit.write(AppExit::error());
                return;
            }
        }
    }

    if autopilot.exit_after.is_some_and(|n| frame >= n) {
        info!("autopilote : {frame} frames atteintes, fermeture");
        exit.write(AppExit::Success);
    }
}

fn env_u32(name: &str) -> Option<u32> {
    std::env::var(name).ok()?.trim().parse().ok()
}
