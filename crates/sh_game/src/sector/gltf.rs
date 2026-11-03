//! Post-traitement des modèles glTF du pipeline Blender (conventions d'AGENTS.md §9.3).
//!
//! Quand une instance de modèle est prête (`WorldInstanceReady`), ses nœuds sont relus :
//! - `*_col` : collision (enveloppe convexe du maillage), jamais rendue ;
//! - `LIGHT_*` : lumière ponctuelle réglée par les extras (`lumens`, `range`, `color`,
//!   `shadow_candidate`) ;
//! - `INTERACT_*` : zone d'interaction (`interact` = genre, plus ses paramètres) ;
//! - `SOCKET_*` : point d'attache (`usage`), par exemple l'œil d'un siège.

use avian3d::prelude::*;
use bevy::gltf::GltfExtras;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use serde_json::Value;

use crate::sector::Layer;
use crate::sector::streaming::{CellInstance, CellReady, Quality};

pub struct GltfSetupPlugin;

impl Plugin for GltfSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_instance_ready);
    }
}

/// Couche de collision à donner aux nœuds `*_col` d'une instance (par défaut : statique).
#[derive(Component, Clone, Copy)]
pub struct ColliderLayer(pub Layer);

/// Lumière que le budget d'ombres peut ombrer (AGENTS.md §10 : ≤ 2 à la fois en Moyen).
#[derive(Component)]
pub struct ShadowCandidate;

/// Quelque chose avec quoi le joueur peut interagir.
#[derive(Component, Clone, Debug, PartialEq)]
pub enum Interactable {
    Door,
    Console { label: String },
    Seat { role: String },
    Pickup { item: String },
    Ladder,
}

/// Point d'attache nommé d'un modèle.
#[derive(Component, Clone, Debug)]
pub struct Socket {
    pub name: String,
    pub usage: String,
}

fn extras(value: Option<&GltfExtras>) -> Value {
    value
        .and_then(|e| serde_json::from_str(&e.value).ok())
        .unwrap_or(Value::Null)
}

fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn number(v: &Value, key: &str, default: f32) -> f32 {
    v.get(key)
        .and_then(Value::as_f64)
        .map_or(default, |n| n as f32)
}

fn on_instance_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    instances: Query<(Option<&CellInstance>, Option<&ColliderLayer>)>,
    children: Query<&Children>,
    nodes: Query<(&Name, Option<&GltfExtras>)>,
    meshes: Query<(), With<Mesh3d>>,
) {
    let root = event.entity;
    let Ok((cell, layer)) = instances.get(root) else {
        return;
    };
    let sector_quality = cell.is_some_and(|c| c.quality == Quality::Sector);
    let layer = layer.map_or(Layer::Static, |l| l.0);
    let mut stats = (0, 0, 0);

    for node in children.iter_descendants(root) {
        let Ok((name, gltf_extras)) = nodes.get(node) else {
            continue;
        };
        let name = name.as_str();
        if name.ends_with("_col") {
            commands.entity(node).insert(Visibility::Hidden);
            // Le maillage d'un nœud glTF est sur le nœud lui-même ou sur ses primitives.
            let targets = std::iter::once(node)
                .chain(children.get(node).into_iter().flatten().copied())
                .filter(|e| meshes.contains(*e));
            for target in targets {
                commands.entity(target).insert((
                    ColliderConstructor::ConvexHullFromMesh,
                    CollisionLayers::new(layer, [Layer::Static, Layer::Door]),
                ));
                stats.0 += 1;
            }
        } else if name.starts_with("LIGHT_") && !sector_quality {
            let v = extras(gltf_extras);
            let color = v
                .get("color")
                .and_then(Value::as_array)
                .map(|c| {
                    let get = |i: usize| c.get(i).and_then(Value::as_f64).unwrap_or(1.0) as f32;
                    Color::srgb(get(0), get(1), get(2))
                })
                .unwrap_or(Color::WHITE);
            let mut light = commands.entity(node);
            light.insert(PointLight {
                intensity: number(&v, "lumens", 20_000.0),
                range: number(&v, "range", 8.0),
                color,
                radius: 0.1,
                shadow_maps_enabled: false,
                ..default()
            });
            if number(&v, "shadow_candidate", 0.0) > 0.5 {
                light.insert(ShadowCandidate);
            }
            stats.1 += 1;
        } else if let Some(rest) = name.strip_prefix("INTERACT_") {
            let v = extras(gltf_extras);
            let interactable = match text(&v, "interact").as_str() {
                "console" => Interactable::Console {
                    label: text(&v, "label"),
                },
                "seat" => Interactable::Seat {
                    role: text(&v, "role"),
                },
                "pickup" => Interactable::Pickup {
                    item: text(&v, "item"),
                },
                "ladder" => Interactable::Ladder,
                other => {
                    warn!("interaction inconnue « {other} » sur {rest}");
                    continue;
                }
            };
            commands.entity(node).insert((
                interactable,
                Collider::sphere(0.35),
                Sensor,
                CollisionLayers::new(Layer::Interact, LayerMask::NONE),
            ));
            stats.2 += 1;
        } else if let Some(rest) = name.strip_prefix("SOCKET_") {
            let v = extras(gltf_extras);
            commands.entity(node).insert(Socket {
                name: rest.to_owned(),
                usage: text(&v, "usage"),
            });
        }
    }

    if let Some(cell) = cell {
        if cell.quality == Quality::Local {
            commands.entity(root).insert(RigidBody::Static);
        }
        debug!(
            "cellule {} ({:?}) prête : {} collisions, {} lumières, {} interactions",
            cell.cell, cell.quality, stats.0, stats.1, stats.2
        );
    }
    commands.entity(root).insert(CellReady);
}
