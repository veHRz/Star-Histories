//! Post-traitement des modèles glTF du pipeline Blender (conventions d'AGENTS.md §9.3).
//!
//! Quand une instance de modèle est prête (`WorldInstanceReady`), ses nœuds sont relus :
//! - `*_col` : collision, jamais rendue : un pavé (le cas du kit) devient un `Collider::cuboid`,
//!   immédiat et bon marché ; tout autre maillage, son enveloppe convexe ;
//! - `LIGHT_*` : lumière ponctuelle réglée par les extras (`lumens`, `range`, `color`,
//!   `shadow_candidate`) ;
//! - `INTERACT_*` : zone d'interaction (`interact` = genre, plus ses paramètres) ;
//! - `SOCKET_*` : point d'attache (`usage`), par exemple l'œil d'un siège.
//!
//! Une cellule qui a une lightmap (et si le préréglage l'active) la reçoit sur tous ses
//! maillages qui ont un 2e jeu d'UV ; ses lumières n'y ajoutent plus que le spéculaire et
//! l'éclairage des objets mobiles (portes), sans carte d'ombre : les ombres sont précalculées.

use avian3d::prelude::*;
use bevy::gltf::GltfExtras;
use bevy::mesh::VertexAttributeValues;
use bevy::pbr::Lightmap;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use serde_json::Value;

use crate::graphics::GraphicsSettings;
use crate::sector::streaming::{CellInstance, CellReady, Quality};
use crate::sector::{ActiveSector, Layer};

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

/// Collision d'un maillage qui est un pavé aligné sur ses axes (tous les sommets sur les coins
/// de sa boîte englobante) : un cuboïde, sans calcul d'enveloppe à l'instanciation.
fn box_collider(mesh: &Mesh) -> Option<Collider> {
    let Some(VertexAttributeValues::Float32x3(points)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return None;
    };
    let (lo, hi) = points.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(lo, hi), p| (lo.min(Vec3::from(*p)), hi.max(Vec3::from(*p))),
    );
    const EPS: f32 = 1e-4;
    let on_corner =
        |p: &[f32; 3]| (0..3).all(|i| (p[i] - lo[i]).abs() < EPS || (p[i] - hi[i]).abs() < EPS);
    if points.is_empty() || !points.iter().all(on_corner) {
        return None;
    }
    let size = hi - lo;
    let center = (lo + hi) / 2.0;
    let cuboid = Collider::cuboid(size.x, size.y, size.z);
    Some(if center.length() < EPS {
        cuboid
    } else {
        Collider::compound(vec![(center, Quat::IDENTITY, cuboid)])
    })
}

/// Ressources du post-traitement (limite de paramètres d'un système) : assets et éclairage
/// précalculé.
#[derive(bevy::ecs::system::SystemParam)]
struct CellAssets<'w> {
    settings: Option<Res<'w, GraphicsSettings>>,
    sector: Option<Res<'w, ActiveSector>>,
    asset_server: Res<'w, AssetServer>,
    meshes: Res<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

impl CellAssets<'_> {
    /// La lightmap de la cellule et son exposition, si elle en a une et qu'elle est active.
    fn for_cell(&self, cell: &CellInstance) -> Option<(Handle<Image>, f32)> {
        if !self.settings.as_ref()?.preset.baked_lighting {
            return None;
        }
        let lightmap = self
            .sector
            .as_ref()?
            .sector
            .cell(&cell.cell)?
            .lightmap
            .as_ref()?;
        Some((
            self.asset_server.load(lightmap.image.clone()),
            lightmap.exposure,
        ))
    }
}

fn on_instance_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    instances: Query<(Option<&CellInstance>, Option<&ColliderLayer>)>,
    children: Query<&Children>,
    nodes: Query<(&Name, Option<&GltfExtras>)>,
    meshes: Query<(&Mesh3d, Option<&MeshMaterial3d<StandardMaterial>>)>,
    mut assets: CellAssets,
) {
    let root = event.entity;
    let Ok((cell, layer)) = instances.get(root) else {
        return;
    };
    let sector_quality = cell.is_some_and(|c| c.quality == Quality::Sector);
    let layer = layer.map_or(Layer::Static, |l| l.0);
    let lightmap = cell.and_then(|c| assets.for_cell(c));
    let mut stats = (0, 0, 0);

    if let Some((image, exposure)) = &lightmap {
        let mut lit = Vec::new();
        for node in children.iter_descendants(root) {
            let Ok((mesh, material)) = meshes.get(node) else {
                continue;
            };
            // Seuls les maillages dépliés pour la lightmap (pas les collisions) la reçoivent.
            let unwrapped = assets
                .meshes
                .get(&mesh.0)
                .is_some_and(|m| m.contains_attribute(Mesh::ATTRIBUTE_UV_1));
            if !unwrapped {
                continue;
            }
            commands.entity(node).insert(Lightmap {
                image: image.clone(),
                ..default()
            });
            lit.extend(material.map(|m| m.id()));
        }
        lit.sort_unstable();
        lit.dedup();
        // Les matériaux d'un modèle sont propres à son fichier, donc à la cellule.
        for id in lit {
            let stale = assets
                .materials
                .get(id)
                .is_some_and(|m| m.lightmap_exposure != *exposure);
            if stale && let Some(mut m) = assets.materials.get_mut(id) {
                m.lightmap_exposure = *exposure;
            }
        }
    }

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
                let layers = CollisionLayers::new(layer, [Layer::Static, Layer::Door]);
                let cuboid = meshes
                    .get(target)
                    .ok()
                    .and_then(|(mesh, _)| assets.meshes.get(&mesh.0))
                    .and_then(box_collider);
                let mut entity = commands.entity(target);
                match cuboid {
                    Some(collider) => entity.insert((collider, layers)),
                    None => entity.insert((ColliderConstructor::ConvexHullFromMesh, layers)),
                };
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
                // Son diffus est déjà dans la lightmap.
                affects_lightmapped_mesh_diffuse: lightmap.is_none(),
                ..default()
            });
            if lightmap.is_none() && number(&v, "shadow_candidate", 0.0) > 0.5 {
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
            "cellule {} ({:?}) prête : {} collisions, {} lumières, {} interactions, lightmap {}",
            cell.cell,
            cell.quality,
            stats.0,
            stats.1,
            stats.2,
            lightmap.is_some()
        );
    }
    commands.entity(root).insert(CellReady);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_pave_devient_un_cuboide() {
        let mesh = Mesh::from(Cuboid::new(1.0, 2.0, 3.0));
        let collider = box_collider(&mesh).expect("un pavé doit donner un cuboïde");
        let aabb = collider.aabb(Vec3::ZERO, Quat::IDENTITY, 0.0);
        assert!((aabb.size() - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-4);
    }

    #[test]
    fn une_sphere_garde_son_enveloppe() {
        assert!(box_collider(&Mesh::from(Sphere::new(1.0))).is_none());
    }
}
