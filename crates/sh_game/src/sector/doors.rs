//! Portes coulissantes des portails de genre `Door`.
//!
//! Une porte est aussi une **porte de chargement** (M11) : elle ne s'ouvre que lorsque ses deux
//! côtés sont prêts en qualité Locale, avec ce qu'on voit derrière eux par des ouvertures
//! (`CellStates::is_side_ready` : le hall derrière le couloir du spatioport). Prêts, elle s'ouvre
//! dans la frame même de l'appui ; sinon elle attend, au plus [`UNLOCK_MAX_WAIT`], puis s'ouvre
//! quand même sur la coque simplifiée d'en face (jamais bloquante : un modèle manquant ne ferme
//! pas une salle pour toujours). Elle s'anime avant le streaming (`streaming::StreamCells`) : dès
//! qu'elle quitte l'état fermé, la salle d'en face est affichée, avant que le panneau ne bouge.
//! Elle se referme seule quand plus personne n'est à proximité.
//!
//! **Modèle** (M10) : chaque porte prend le modèle de son portail s'il en a un (la porte
//! blindée du sas), sinon celui du secteur ; à quai, les portes du vaisseau gardent ainsi le
//! leur dans le secteur assemblé (`sh_data::dock`).
//!
//! **Porte sur le vide** (M10) : un portail dont un côté est un point d'attache « @… » encore
//! présent dans le secteur chargé mène à un secteur qui n'est pas là (le sas extérieur en vol :
//! rien n'est amarré). La porte est [`Door::sealed`] : elle reste fermée, sans état d'attente ni
//! son. Avant M10, elle attendait pour toujours une cellule « @dock » qui ne charge jamais.

use std::collections::BTreeMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

use crate::player::Player;
use crate::sector::gltf::ColliderLayer;
use crate::sector::streaming::{CellReady, CellStates, StreamCells};
use crate::sector::{ActiveSector, Layer, v3};

pub struct DoorsPlugin;

impl Plugin for DoorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_doors.run_if(resource_added::<ActiveSector>))
            .add_systems(
                Update,
                (
                    animate_doors
                        .run_if(resource_exists::<ActiveSector>)
                        .before(StreamCells),
                    auto_close.run_if(resource_exists::<ActiveSector>),
                ),
            );
    }
}

/// Durée d'ouverture ou de fermeture, en secondes.
const SLIDE_TIME: f32 = 0.55;
/// Attente maximale des deux côtés d'une porte de chargement, en secondes : au-delà, elle
/// s'ouvre sur la coque simplifiée d'en face.
pub const UNLOCK_MAX_WAIT: f32 = 2.0;
/// Délai avant fermeture automatique quand plus personne n'est près.
const AUTO_CLOSE_AFTER: f32 = 4.0;
/// Distance sous laquelle une porte reste ouverte (et ne se referme pas sur le joueur).
const KEEP_OPEN_DISTANCE: f32 = 2.2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DoorState {
    Closed,
    /// En attente que la cellule d'en face soit prête.
    Unlocking,
    Opening,
    Open,
    Closing,
}

#[derive(Component, Debug)]
pub struct Door {
    pub portal: String,
    pub a: String,
    pub b: String,
    pub state: DoorState,
    /// Ouverture de 0 (fermée) à 1 (ouverte).
    pub openness: f32,
    pub width: f32,
    /// Hauteur de l'ouverture (m) : le linteau, où se posent les noms de salle
    /// (`sector::door_signs`).
    pub height: f32,
    pub idle: f32,
    pub panel: Option<Entity>,
    /// Un côté mène à un point d'attache « @… » non résolu (le sas, vaisseau non amarré) : la
    /// porte reste fermée et inerte.
    pub sealed: bool,
}

impl Door {
    pub fn is_open(&self) -> bool {
        matches!(self.state, DoorState::Open | DoorState::Opening)
    }

    /// Le portail de cette porte mène-t-il au vide (un côté « @… » non résolu) ?
    pub fn leads_to_void(a: &str, b: &str) -> bool {
        sh_data::sector::is_external(a) || sh_data::sector::is_external(b)
    }

    pub fn request_open(&mut self) {
        if self.sealed {
            return;
        }
        if matches!(self.state, DoorState::Closed | DoorState::Closing) {
            self.state = DoorState::Unlocking;
            // L'attente de la cellule d'en face se compte à partir de l'appui.
            self.idle = 0.0;
        }
    }

    /// La porte est-elle en train de changer d'état (attente, ouverture, fermeture) ?
    pub fn is_moving(&self) -> bool {
        matches!(
            self.state,
            DoorState::Unlocking | DoorState::Opening | DoorState::Closing
        )
    }

    /// Un pas d'animation de `dt` secondes. `sides_ready` : ses deux côtés sont prêts en qualité
    /// Locale (bloc d'ouvertures compris). En attente, la porte s'ouvre dès qu'ils le sont, ou
    /// après [`UNLOCK_MAX_WAIT`], et commence à coulisser dans la même frame : une porte dont la
    /// salle d'en face est déjà là s'ouvre sans délai à l'appui.
    pub fn advance(&mut self, sides_ready: bool, dt: f32) {
        if self.state == DoorState::Unlocking {
            self.idle += dt;
            if !(sides_ready || self.idle >= UNLOCK_MAX_WAIT) {
                return;
            }
            self.state = DoorState::Opening;
        }
        let step = dt / SLIDE_TIME;
        match self.state {
            DoorState::Opening => {
                self.openness = (self.openness + step).min(1.0);
                if self.openness >= 1.0 {
                    self.state = DoorState::Open;
                    self.idle = 0.0;
                }
            }
            DoorState::Closing => {
                self.openness = (self.openness - step).max(0.0);
                if self.openness <= 0.0 {
                    self.state = DoorState::Closed;
                }
            }
            DoorState::Unlocking | DoorState::Open | DoorState::Closed => {}
        }
    }

    pub fn request_close(&mut self) {
        match self.state {
            // Encore fermée (elle attendait la cellule d'en face) : rien ne bouge, donc pas de
            // mouvement de fermeture (ni son).
            DoorState::Unlocking if self.openness <= 0.0 => self.state = DoorState::Closed,
            DoorState::Open | DoorState::Opening | DoorState::Unlocking => {
                self.state = DoorState::Closing;
            }
            DoorState::Closed | DoorState::Closing => {}
        }
    }

    pub fn toggle(&mut self) {
        if self.is_open() || self.state == DoorState::Unlocking {
            self.request_close();
        } else {
            self.request_open();
        }
    }
}

/// Le panneau mobile d'une porte.
#[derive(Component)]
struct DoorPanel;

fn spawn_doors(mut commands: Commands, sector: Res<ActiveSector>, asset_server: Res<AssetServer>) {
    let s = &sector.sector;
    // Une poignée par modèle (deux ou trois par secteur), partagée par ses portes.
    let mut models: BTreeMap<&str, Handle<WorldAsset>> = BTreeMap::new();
    for portal in s
        .portals
        .iter()
        .filter(|p| p.kind == sh_data::PortalKind::Door)
    {
        let base = v3(portal.center) - Vec3::Y * (portal.height / 2.0);
        let normal = v3(portal.normal).normalize_or(Vec3::Z);
        let path = s.door_model_of(portal);
        let model = models
            .entry(path)
            .or_insert_with(|| {
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(path.to_owned()))
            })
            .clone();
        let door = commands
            .spawn((
                Name::new(format!("Porte {}", portal.id)),
                Transform::from_translation(base).looking_to(normal, Vec3::Y),
                Visibility::default(),
                ChildOf(sector.root),
            ))
            .id();
        let panel = commands
            .spawn((
                Name::new("Panneau"),
                DoorPanel,
                WorldAssetRoot(model),
                ColliderLayer(Layer::Door),
                RigidBody::Kinematic,
                ChildOf(door),
            ))
            .id();
        commands.entity(door).insert(Door {
            portal: portal.id.clone(),
            a: portal.a.clone(),
            b: portal.b.clone(),
            state: DoorState::Closed,
            openness: 0.0,
            width: portal.width,
            height: portal.height,
            idle: 0.0,
            panel: Some(panel),
            sealed: Door::leads_to_void(&portal.a, &portal.b),
        });
    }
}

fn animate_doors(
    time: Res<Time>,
    states: Res<CellStates>,
    ready: Query<(), With<CellReady>>,
    mut doors: Query<&mut Door>,
    mut panels: Query<&mut Transform, With<DoorPanel>>,
) {
    let dt = time.delta_secs();
    for mut door in &mut doors {
        // Une porte immobile n'est pas modifiée : aucun changement signalé chaque frame (sons de
        // `audio`, qui suivent `Changed<Door>`).
        if door.is_moving() {
            // Porte de chargement : ses deux côtés, et ce qu'on voit derrière eux par des
            // ouvertures.
            let sides_ready = door.state == DoorState::Unlocking
                && states.is_side_ready(&door.a, &ready)
                && states.is_side_ready(&door.b, &ready);
            door.advance(sides_ready, dt);
        }
        if let Some(panel) = door.panel
            && let Ok(mut t) = panels.get_mut(panel)
        {
            // Le panneau coulisse dans la cloison ; courbe douce aux extrémités. Écrit seulement
            // s'il bouge (pas de propagation de transformée pour les portes au repos).
            let eased = door.openness * door.openness * (3.0 - 2.0 * door.openness);
            let x = eased * (door.width + 0.05);
            if t.translation.x != x {
                t.translation.x = x;
            }
        }
    }
}

fn auto_close(
    time: Res<Time>,
    player: Query<&GlobalTransform, With<Player>>,
    mut doors: Query<(&GlobalTransform, &mut Door)>,
) {
    let player = player.single().map(|p| p.translation()).ok();
    for (transform, mut door) in &mut doors {
        if door.state != DoorState::Open {
            continue;
        }
        let near = player.is_some_and(|p| p.distance(transform.translation()) < KEEP_OPEN_DISTANCE);
        if near {
            door.idle = 0.0;
        } else {
            door.idle += time.delta_secs();
            if door.idle >= AUTO_CLOSE_AFTER {
                door.request_close();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door(a: &str, b: &str) -> Door {
        Door {
            portal: format!("{a}__{b}"),
            a: a.into(),
            b: b.into(),
            state: DoorState::Closed,
            openness: 0.0,
            width: 1.4,
            height: 2.2,
            idle: 0.0,
            panel: None,
            sealed: Door::leads_to_void(a, b),
        }
    }

    #[test]
    fn une_porte_sur_le_vide_reste_fermee() {
        let mut hatch = door("sas", "@dock");
        assert!(hatch.sealed);
        hatch.toggle();
        assert_eq!(hatch.state, DoorState::Closed);
        hatch.request_open();
        assert_eq!(hatch.state, DoorState::Closed);
        hatch.toggle();
        assert_eq!(hatch.state, DoorState::Closed);
        assert!(!door("sas", "couloir").sealed);
    }

    #[test]
    fn porte_de_chargement() {
        let frame = 1.0 / 60.0;
        // Les deux côtés déjà prêts : elle coulisse dès la frame de l'appui.
        let mut d = door("couloir", "hangar");
        d.request_open();
        assert_eq!(d.state, DoorState::Unlocking);
        d.advance(true, frame);
        assert_eq!(d.state, DoorState::Opening);
        assert!(d.openness > 0.0);
        for _ in 0..60 {
            d.advance(false, frame);
        }
        assert!(d.state == DoorState::Open && d.openness >= 1.0);
        // La salle d'en face pas prête : elle attend, fermée, au plus UNLOCK_MAX_WAIT…
        let mut d = door("couloir", "hangar");
        d.idle = 3.0; // reste d'une ouverture précédente : l'attente part de l'appui
        d.request_open();
        d.advance(false, UNLOCK_MAX_WAIT - 0.1);
        assert!(d.state == DoorState::Unlocking && d.openness <= 0.0);
        assert!(d.is_moving());
        // … puis s'ouvre quand même (sur la coque simplifiée d'en face).
        d.advance(false, 0.2);
        assert!(d.state == DoorState::Opening && d.openness > 0.0);
        // La fermeture va jusqu'au bout, puis la porte ne bouge plus.
        d.request_close();
        assert_eq!(d.state, DoorState::Closing);
        for _ in 0..60 {
            d.advance(false, frame);
        }
        assert!(d.state == DoorState::Closed && d.openness <= 0.0 && !d.is_moving());
        // La porte du sas sur le vide reste inerte, sans état d'attente.
        let mut hatch = door("sas", "@dock");
        hatch.request_open();
        hatch.advance(true, UNLOCK_MAX_WAIT + 1.0);
        assert!(hatch.state == DoorState::Closed && !hatch.is_moving());
    }

    #[test]
    fn une_porte_en_attente_se_referme_sans_bouger() {
        let mut d = door("sas", "couloir");
        d.toggle();
        assert_eq!(d.state, DoorState::Unlocking);
        // Second appui avant que la cellule d'en face soit prête : elle n'a pas bougé.
        d.toggle();
        assert_eq!(d.state, DoorState::Closed);
        // Entrouverte, elle se referme pour de bon.
        d.state = DoorState::Opening;
        d.openness = 0.3;
        d.toggle();
        assert_eq!(d.state, DoorState::Closing);
    }
}
