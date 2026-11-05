//! Cache des images générées de la vue Globale (ciel, textures de planète).
//!
//! Les générer prend quelques centaines de millisecondes : faites au changement de vue, elles
//! figeraient l'écran. Elles sont donc gardées d'une vue à l'autre et **préparées en tâche de
//! fond** quand le vaisseau est à quai (le système du port est celui où l'on repartira) : au
//! départ, tout est prêt. À défaut, la vue Globale les calcule sur place (premier démarrage).
//!
//! Un astre à apparence peinte (M9, [`crate::views::planet`]) n'a rien à calculer : sa carte de
//! couleur, chargée dès l'écran de chargement, est fournie d'avance au cache ([`Procedural::provide`]).

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, futures::check_ready};
use sh_data::{Galaxy, PlacedKind, StarSystem, system_layout};

use crate::data::Data;
use crate::sector::Docked;
use crate::views::planet::{Look, PlanetLookPlugin, planet_texture};
use crate::views::sky::{SkyNebula, SkySpec, cube_image, render_sky};
use crate::voyage::RoutePlan;

pub struct ProceduralPlugin;

impl Plugin for ProceduralPlugin {
    fn build(&self, app: &mut App) {
        // Les astres peints passent par ce cache : leur greffon vit avec lui.
        app.add_plugins(PlanetLookPlugin);
        app.init_resource::<Procedural>().add_systems(
            Update,
            (
                prewarm_departure.run_if(resource_exists::<Docked>),
                prewarm_route.run_if(resource_exists::<RoutePlan>),
                collect_ready,
            )
                .chain()
                .run_if(resource_exists::<Data>),
        );
    }
}

/// Côté d'une face du ciel, et hauteur des textures de planète (la largeur est double).
pub const SKY_SIZE: usize = 1024;
pub const PLANET_TEXTURE: usize = 512;

/// Les images prêtes, et celles en cours de calcul, par clé (`sky:<système>`, `planet:<astre>`).
#[derive(Resource, Default)]
pub struct Procedural {
    ready: BTreeMap<String, Handle<Image>>,
    pending: BTreeMap<String, Task<Image>>,
}

impl Procedural {
    /// L'image de cette clé : prête, attendue si elle est en cours, sinon calculée sur place.
    pub fn get(
        &mut self,
        key: &str,
        images: &mut Assets<Image>,
        make: impl FnOnce() -> Image,
    ) -> Handle<Image> {
        if let Some(handle) = self.ready.get(key) {
            return handle.clone();
        }
        let image = match self.pending.remove(key) {
            Some(task) => block_on(task),
            None => {
                let start = std::time::Instant::now();
                let image = make();
                debug!(
                    "{key} généré sur place en {:.0} ms",
                    start.elapsed().as_secs_f64() * 1e3
                );
                image
            }
        };
        let handle = images.add(image);
        self.ready.insert(key.to_owned(), handle.clone());
        handle
    }

    /// Fournit d'avance l'image d'une clé (texture peinte d'un astre) : la vue Globale la
    /// prendra telle quelle, sans rien calculer.
    pub fn provide(&mut self, key: String, handle: Handle<Image>) {
        self.pending.remove(&key);
        self.ready.insert(key, handle);
    }

    /// Oublie l'image d'une clé : elle sera recalculée à la prochaine demande.
    pub fn forget(&mut self, key: &str) {
        self.ready.remove(key);
    }

    /// Lance le calcul en tâche de fond, s'il n'est ni prêt ni en cours.
    pub fn prewarm(&mut self, key: String, make: impl FnOnce() -> Image + Send + 'static) {
        if self.ready.contains_key(&key) || self.pending.contains_key(&key) {
            return;
        }
        let task = AsyncComputeTaskPool::get().spawn(async move { make() });
        self.pending.insert(key, task);
    }
}

pub fn sky_key(system: Option<&StarSystem>) -> String {
    format!("sky:{}", system.map_or("-", |s| s.id.as_str()))
}

pub fn planet_key(id: &str) -> String {
    format!("planet:{id}")
}

/// Le ciel vu depuis un système (ou depuis le centre de la carte, faute de système).
pub fn make_sky(galaxy: &Galaxy, system: Option<&StarSystem>) -> Image {
    let frame = &galaxy.frame;
    let observer = system.map_or([frame.center.0, frame.center.1 + 100.0], |s| {
        [s.center.x, s.center.y]
    });
    let nebulae = galaxy
        .phenomena
        .iter()
        .filter(|p| !p.secret && p.kind_note.to_lowercase().contains("nébuleuse"))
        .filter_map(|p| {
            Some(SkyNebula {
                center: [p.pos?.x, p.pos?.y],
                radius: p.radius?,
            })
        })
        .collect();
    let spec = SkySpec {
        observer,
        galaxy_center: [frame.center.0, frame.center.1],
        galaxy_radii: [frame.radius_x, frame.radius_y],
        nebulae,
        seed: system.map_or(0, |s| sh_core::rng::fnv1a(s.id.as_str().as_bytes())),
    };
    cube_image(&render_sky(&spec, SKY_SIZE))
}

/// Apparence procédurale d'un astre (Hexis : monde industriel, si son apparence peinte manque).
pub fn look_of(kind: PlacedKind, id: &str) -> Look {
    match (kind, id) {
        (_, "hexis") => Look::Industrial,
        (PlacedKind::GasGiant, _) => Look::GasGiant,
        _ => Look::Rocky,
    }
}

pub fn make_planet(kind: PlacedKind, id: &str) -> Image {
    planet_texture(
        look_of(kind, id),
        sh_core::rng::fnv1a(id.as_bytes()),
        PLANET_TEXTURE,
    )
}

/// Prépare en tâche de fond le ciel et les astres d'un système.
fn prewarm_system(data: &Data, cache: &mut Procedural, system: &StarSystem) {
    let shared = data.clone();
    let id = system.id.clone();
    cache.prewarm(sky_key(Some(system)), move || {
        make_sky(&shared.galaxy, shared.galaxy.system(&id))
    });
    for p in system_layout(&data.galaxy, system, &data.space.scale) {
        let Some(id) = p.id.map(|i| i.to_string()) else {
            continue;
        };
        if matches!(
            p.kind,
            PlacedKind::Planet | PlacedKind::GasGiant | PlacedKind::Moon
        ) {
            let kind = p.kind;
            cache.prewarm(planet_key(&id), move || make_planet(kind, &id));
        }
    }
}

/// À quai : préparer le système où l'on repartira.
fn prewarm_departure(data: Res<Data>, docked: Res<Docked>, mut cache: ResMut<Procedural>) {
    let system = data
        .scenes
        .iter()
        .find(|s| s.docks.iter().any(|d| d.sector == docked.district))
        .and_then(|scene| data.galaxy.system(&scene.system));
    if let Some(system) = system {
        prewarm_system(&data, &mut cache, system);
    }
}

/// Une destination est choisie sur la carte : préparer son système.
fn prewarm_route(data: Res<Data>, route: Res<RoutePlan>, mut cache: ResMut<Procedural>) {
    if let Some(system) = data.galaxy.system(&route.dest).filter(|s| !s.secret) {
        prewarm_system(&data, &mut cache, system);
    }
}

fn collect_ready(mut cache: ResMut<Procedural>, mut images: ResMut<Assets<Image>>) {
    let done: Vec<(String, Image)> = cache
        .pending
        .iter_mut()
        .filter_map(|(key, task)| check_ready(task).map(|image| (key.clone(), image)))
        .collect();
    for (key, image) in done {
        cache.pending.remove(&key);
        let handle = images.add(image);
        info!("{key} prêt (préparé en tâche de fond)");
        cache.ready.insert(key, handle);
    }
}
