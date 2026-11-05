//! Post-traitement des modèles glTF du pipeline Blender (conventions d'AGENTS.md §9.3, et de
//! docs/m9-retours.md §3 pour M9).
//!
//! Quand une instance de modèle est prête (`WorldInstanceReady`), ses nœuds sont relus :
//! - `*_col` : collision, jamais rendue : un pavé (le cas du kit) devient un `Collider::cuboid`,
//!   immédiat et bon marché ; tout autre maillage, son enveloppe convexe ;
//! - `LIGHT_*` : lumière ponctuelle réglée par les extras (`lumens`, `range`, `color`,
//!   `shadow_candidate`) ;
//! - `INTERACT_*` : zone d'interaction, de genre `interact` avec ses paramètres : `console`
//!   (`label`), `seat` (`role`), `pickup` (`item`), `locker` (`container`), `npc` (`role`,
//!   `variant`), `ladder`. Son capteur est un pavé si le nœud porte `half_extents: [x, y, z]`
//!   (demi-tailles en mètres, repère local du nœud, Y-haut : il tourne avec l'objet), sinon une
//!   sphère de [`INTERACT_RADIUS`] ;
//! - `SOCKET_*` : point d'attache (`usage`), par exemple l'œil d'un siège (`seat_eye`) ou la place
//!   d'un PNJ de comptoir (`npc`, avec `role` et `variant` : [`crate::npc::CounterNpc`]) ;
//! - extra `detail` (0, 1, 2) sur n'importe quel nœud de décor (sauf `*_col`) : caché avec ses
//!   enfants si le préréglage ne montre pas ce niveau (`GraphicsPreset::shows_detail`) ; un
//!   changement de préréglage s'applique en direct (`graphics::apply_decor_detail`).
//!
//! **Objets posés** (`pickup`) : le modèle du catalogue `models/items/<item>.gltf` remplace les
//! enfants du nœud (l'apparence du kit, gardée tant que le modèle n'existe pas), posé
//! [`PICKUP_MARK_HEIGHT`] sous le nœud, avec son cap. Chaque point de ramassage d'une cellule a un
//! identifiant stable ([`pickup_id`] : `<secteur>/<cellule>/<nœud sans INTERACT_>`, le secteur
//! étant celui d'où vient la cellule, même assemblée à quai) ; un objet déjà pris
//! (`items::PickedUp`, sauvegardé) est retiré dès l'instanciation de sa cellule et ne revient
//! jamais.
//!
//! Une cellule qui a une lightmap (et si le préréglage l'active) la reçoit sur tous ses
//! maillages qui ont un 2e jeu d'UV ; ses lumières n'y ajoutent plus que le spéculaire et
//! l'éclairage des objets mobiles (portes), sans carte d'ombre : les ombres sont précalculées.

use std::collections::BTreeSet;

use avian3d::prelude::*;
use bevy::gltf::GltfExtras;
use bevy::mesh::VertexAttributeValues;
use bevy::pbr::Lightmap;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};
use serde_json::Value;

use crate::data::{Data, assets_dir};
use crate::graphics::GraphicsSettings;
use crate::items::{ItemCatalog, PickedUp};
use crate::npc::CounterNpc;
use crate::sector::streaming::{CellInstance, CellReady, Quality};
use crate::sector::{ActiveSector, Docked, Layer};

pub struct GltfSetupPlugin;

impl Plugin for GltfSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_instance_ready);
    }
}

/// Rayon du capteur d'un `INTERACT_*` sans `half_extents`.
pub const INTERACT_RADIUS: f32 = 0.35;

/// Hauteur d'un repère `INTERACT_*` de ramassage au-dessus de la surface où l'objet est posé
/// (convention du kit, `tools/blender/kit/ship_interior.py`, fonction `pickup`) : l'origine d'un
/// modèle d'objet est le centre de sa base.
pub const PICKUP_MARK_HEIGHT: f32 = 0.15;

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
    Console {
        label: String,
    },
    Seat {
        role: String,
    },
    Pickup {
        item: String,
    },
    /// Un casier : ouvre le conteneur de cet identifiant (`items::OpenContainer`).
    Locker {
        container: String,
    },
    /// Un PNJ de comptoir : la conversation de son rôle (`items::TalkTo`).
    Npc {
        role: String,
        variant: String,
    },
    Ladder,
}

/// Identifiant stable d'un point de ramassage (voir [`pickup_id`]), posé sur son nœud
/// `INTERACT_*` quand il vient d'une cellule de secteur.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct PickupId(pub String);

/// Le modèle d'objet posé à la place de l'apparence du kit, enfant du nœud de ramassage.
#[derive(Component)]
pub struct PickupModel;

/// Niveau de décor d'un nœud (extra `detail`, au moins 1) : sa visibilité suit le préréglage.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecorDetail(pub u8);

/// Point d'attache nommé d'un modèle.
#[derive(Component, Clone, Debug)]
pub struct Socket {
    pub name: String,
    pub usage: String,
}

/// L'identifiant d'un point de ramassage : le secteur d'origine de la cellule, la cellule et le
/// nom du nœud sans son préfixe `INTERACT_`. Stable d'une partie à l'autre tant que le secteur,
/// la cellule et le nœud gardent leur nom (un renommage doit migrer les sauvegardes).
pub fn pickup_id(sector: &str, cell: &str, node: &str) -> String {
    format!("{sector}/{cell}/{node}")
}

/// Le secteur d'où vient une cellule. À quai, le secteur chargé assemble le quartier et le
/// vaisseau amarré (`sh_data::dock`) sous l'identifiant du quartier : une cellule du vaisseau
/// garde celui du vaisseau, pour qu'un objet pris à quai le reste en vol, et inversement.
pub fn home_sector<'a>(
    active: &'a str,
    docked_ship: Option<&'a sh_data::Sector>,
    cell: &str,
) -> &'a str {
    match docked_ship {
        Some(ship) if ship.cell(cell).is_some() => ship.id.as_str(),
        _ => active,
    }
}

/// Les demi-tailles du volume d'interaction (`half_extents: [x, y, z]`), si l'extra est complet,
/// fini et strictement positif.
pub fn half_extents(v: &Value) -> Option<Vec3> {
    let values = v.get("half_extents")?.as_array()?;
    if values.len() != 3 {
        return None;
    }
    let get = |i: usize| {
        values
            .get(i)
            .and_then(Value::as_f64)
            .map(|n| n as f32)
            .filter(|n| n.is_finite() && *n > 0.0)
    };
    Some(Vec3::new(get(0)?, get(1)?, get(2)?))
}

/// Le capteur d'un `INTERACT_*` : le pavé de ses `half_extents`, sinon la sphère historique.
pub fn interact_collider(v: &Value) -> Collider {
    match half_extents(v) {
        Some(h) => Collider::cuboid(2.0 * h.x, 2.0 * h.y, 2.0 * h.z),
        None => Collider::sphere(INTERACT_RADIUS),
    }
}

/// Le niveau de décor d'un nœud (extra `detail`) : absent, négatif ou illisible = 0 (toujours
/// montré). Blender exporte un entier, mais une propriété flottante (`1.0`) compte aussi.
pub fn detail_level(v: &Value) -> u8 {
    let Some(detail) = v.get("detail") else {
        return 0;
    };
    detail
        .as_u64()
        .or_else(|| {
            detail
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.0)
                .map(|n| n.round() as u64)
        })
        .map_or(0, |n| u8::try_from(n).unwrap_or(u8::MAX))
}

/// La visibilité d'un nœud de décor optionnel, selon que le préréglage montre son niveau.
pub fn decor_visibility(shown: bool) -> Visibility {
    if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
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

/// Ressources du post-traitement (limite de paramètres d'un système) : assets, éclairage
/// précalculé, catalogue d'objets et objets déjà pris.
#[derive(bevy::ecs::system::SystemParam)]
struct CellAssets<'w> {
    settings: Option<Res<'w, GraphicsSettings>>,
    sector: Option<Res<'w, ActiveSector>>,
    docked: Option<Res<'w, Docked>>,
    data: Option<Res<'w, Data>>,
    catalog: Option<Res<'w, ItemCatalog>>,
    picked: Option<Res<'w, PickedUp>>,
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

    /// Le secteur d'origine de la cellule (préfixe des identifiants de ramassage).
    fn home_of(&self, cell: &CellInstance) -> Option<String> {
        let active = self.sector.as_ref()?;
        let ship = self
            .docked
            .as_ref()
            .zip(self.data.as_ref())
            .and_then(|(docked, data)| data.sector(&docked.ship));
        Some(home_sector(active.sector.id.as_str(), ship, &cell.cell).to_owned())
    }

    /// Le modèle de l'objet du catalogue, s'il est produit (les modèles arrivent au fil de
    /// l'eau : sans fichier, le nœud garde l'apparence du kit).
    fn item_model(&self, item: &str) -> Option<String> {
        let path = self.catalog.as_ref()?.get(item)?.model_path();
        assets_dir().join(&path).is_file().then_some(path)
    }

    /// Le préréglage courant montre-t-il ce niveau de décor ? (Sans réglage : tout.)
    fn shows_detail(&self, detail: u8) -> bool {
        self.settings
            .as_ref()
            .is_none_or(|s| s.preset.shows_detail(detail))
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
    let cell_lights = assets
        .settings
        .as_ref()
        .is_some_and(|s| s.preset.baked_cell_lights);
    let mut stats = (0, 0, 0);

    // Objets déjà pris : leur nœud est retiré, et aucune commande ne vise plus ses descendants
    // (une commande sur une entité détruite est une erreur).
    let home = cell.and_then(|c| assets.home_of(c));
    let mut dropped = Vec::new();
    let mut skip = BTreeSet::new();
    if let (Some(cell), Some(home), Some(picked)) = (cell, &home, &assets.picked) {
        for node in children.iter_descendants(root) {
            let Some(rest) = nodes
                .get(node)
                .ok()
                .and_then(|(name, _)| name.as_str().strip_prefix("INTERACT_"))
            else {
                continue;
            };
            if picked.contains(&pickup_id(home, &cell.cell, rest)) {
                dropped.push(node);
                skip.insert(node);
                skip.extend(children.iter_descendants(node));
            }
        }
    }

    if let Some((image, exposure)) = &lightmap {
        let mut lit = Vec::new();
        for node in children.iter_descendants(root) {
            if skip.contains(&node) {
                continue;
            }
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

    // Vitrages : sans reflets (Bas, Moyen), une simple teinte, bien moins chère qu'un matériau
    // éclairé mélangé sur une grande surface (le hublot du cockpit occupe un tiers de l'écran).
    let lit_glass = assets.settings.as_ref().is_some_and(|s| s.preset.lit_glass);
    if !lit_glass {
        let mut glass: Vec<_> = children
            .iter_descendants(root)
            .filter_map(|node| meshes.get(node).ok())
            .filter_map(|(_, material)| material.map(|m| m.id()))
            .collect();
        glass.sort_unstable();
        glass.dedup();
        for id in glass {
            let blended = assets
                .materials
                .get(id)
                .is_some_and(|m| matches!(m.alpha_mode, AlphaMode::Blend) && !m.unlit);
            if blended && let Some(mut m) = assets.materials.get_mut(id) {
                // Non éclairée, la couleur s'ajouterait telle quelle et voilerait l'espace :
                // presque noire, elle ne fait qu'assombrir et teinter ce qu'on voit au travers.
                let c = m.base_color.to_linear();
                m.base_color = Color::LinearRgba(LinearRgba::new(
                    c.red * 0.04,
                    c.green * 0.04,
                    c.blue * 0.04,
                    c.alpha,
                ));
                m.unlit = true;
            }
        }
    }

    for node in children.iter_descendants(root) {
        if skip.contains(&node) {
            continue;
        }
        let Ok((name, gltf_extras)) = nodes.get(node) else {
            continue;
        };
        let name = name.as_str();
        let v = extras(gltf_extras);
        // Décor optionnel : caché tout de suite si le préréglage ne le montre pas, puis suivi
        // par `graphics::apply_decor_detail`. Une collision reste cachée quoi qu'il arrive.
        let detail = detail_level(&v);
        if detail > 0 && !name.ends_with("_col") {
            commands.entity(node).insert((
                DecorDetail(detail),
                decor_visibility(assets.shows_detail(detail)),
            ));
        }
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
        } else if name.starts_with("LIGHT_")
            && !sector_quality
            && (lightmap.is_none() || cell_lights)
        {
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
                "locker" => Interactable::Locker {
                    container: text(&v, "container"),
                },
                "npc" => Interactable::Npc {
                    role: text(&v, "role"),
                    variant: text(&v, "variant"),
                },
                "ladder" => Interactable::Ladder,
                other => {
                    warn!("interaction inconnue « {other} » sur {rest}");
                    continue;
                }
            };
            if v.get("half_extents").is_some() && half_extents(&v).is_none() {
                warn!("{rest} : half_extents illisible, capteur sphérique");
            }
            if let Interactable::Pickup { item } = &interactable {
                if let (Some(cell), Some(home)) = (cell, &home) {
                    commands
                        .entity(node)
                        .insert(PickupId(pickup_id(home, &cell.cell, rest)));
                }
                if let Some(path) = assets.item_model(item) {
                    // Le modèle de l'objet remplace l'apparence du kit, posé sur la même surface.
                    for child in children.get(node).into_iter().flatten() {
                        commands.entity(*child).insert(Visibility::Hidden);
                    }
                    commands.spawn((
                        Name::new(format!("Objet posé {item}")),
                        PickupModel,
                        WorldAssetRoot(
                            assets
                                .asset_server
                                .load(GltfAssetLabel::Scene(0).from_asset(path)),
                        ),
                        Transform::from_xyz(0.0, -PICKUP_MARK_HEIGHT, 0.0),
                        ChildOf(node),
                    ));
                }
            }
            commands.entity(node).insert((
                interactable,
                interact_collider(&v),
                Sensor,
                CollisionLayers::new(Layer::Interact, LayerMask::NONE),
            ));
            stats.2 += 1;
        } else if let Some(rest) = name.strip_prefix("SOCKET_") {
            let usage = text(&v, "usage");
            let mut entity = commands.entity(node);
            if usage == "npc" {
                entity.insert(CounterNpc {
                    role: text(&v, "role"),
                    variant: text(&v, "variant"),
                });
            }
            entity.insert(Socket {
                name: rest.to_owned(),
                usage,
            });
        }
    }

    for node in dropped {
        commands.entity(node).despawn();
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
    use serde_json::json;

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

    #[test]
    fn identifiant_de_ramassage() {
        assert_eq!(
            pickup_id("player_ship", "quartiers", "quartiers_pickup_datapad"),
            "player_ship/quartiers/quartiers_pickup_datapad"
        );
        // À quai, une cellule du vaisseau garde le nom du vaisseau ; celles du quartier, le sien.
        let ship: sh_data::Sector = sh_data::ron_options()
            .from_str(
                r#"(id: "ship", kind: Ship, name_key: "n", door_model: "d",
                    cells: [(id: "quartiers", model: "q", min: (0, 0, 0), max: (1, 1, 1))],
                    portals: [], spawns: [])"#,
            )
            .expect("secteur de test illisible");
        assert_eq!(home_sector("port", Some(&ship), "quartiers"), "ship");
        assert_eq!(home_sector("port", Some(&ship), "hall"), "port");
        assert_eq!(home_sector("ship", None, "quartiers"), "ship");
    }

    #[test]
    fn volume_d_interaction() {
        let v = json!({ "interact": "seat", "half_extents": [0.3, 0.5, 0.25] });
        assert_eq!(half_extents(&v), Some(Vec3::new(0.3, 0.5, 0.25)));
        let aabb = interact_collider(&v).aabb(Vec3::ZERO, Quat::IDENTITY, 0.0);
        assert!((aabb.size() - Vec3::new(0.6, 1.0, 0.5)).length() < 1e-4);
        // Absent ou illisible : la sphère de 0,35 m.
        for v in [
            json!({ "interact": "pickup" }),
            json!({ "half_extents": [0.3, 0.5] }),
            json!({ "half_extents": [0.3, -0.5, 0.2] }),
            json!({ "half_extents": "large" }),
        ] {
            assert_eq!(half_extents(&v), None, "{v}");
            let aabb = interact_collider(&v).aabb(Vec3::ZERO, Quat::IDENTITY, 0.0);
            assert!((aabb.size() - Vec3::splat(2.0 * INTERACT_RADIUS)).length() < 1e-4);
        }
    }

    #[test]
    fn niveau_de_decor() {
        assert_eq!(detail_level(&Value::Null), 0);
        assert_eq!(detail_level(&json!({ "detail": 2 })), 2);
        assert_eq!(detail_level(&json!({ "detail": 1.0 })), 1);
        assert_eq!(detail_level(&json!({ "detail": -1 })), 0);
        assert_eq!(detail_level(&json!({ "detail": "haut" })), 0);
        assert_eq!(detail_level(&json!({ "detail": 900 })), u8::MAX);
        assert_eq!(decor_visibility(true), Visibility::Inherited);
        assert_eq!(decor_visibility(false), Visibility::Hidden);
    }
}
