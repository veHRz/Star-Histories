//! Origine flottante (AGENTS.md §6.4) : positions absolues en `f64`, rendu en `f32` relatif.
//!
//! Chaque entité racine de l'espace porte une [`SpacePosition`] (absolue, en mètres, dans le
//! repère du système). Son `Transform` n'est que sa position **relative à l'origine flottante**,
//! qui suit le point marqué [`OriginFocus`] (le vaisseau du joueur, ou la caméra de test) :
//! quand il s'en éloigne de plus de `recenter_distance`, l'origine saute sur lui et tous les
//! `Transform` sont recalculés **depuis les `f64`**, sans accumulation d'erreur.
//!
//! Trois sortes d'entités, selon ce qui fait foi ([`PositionDriver`]) :
//! - `Space` (astres, stations, trafic sur rails) : la position absolue fait foi, le `Transform`
//!   en est déduit ;
//! - `Render` (ce qui bouge par la physique ou les commandes : vaisseaux, caméra) : le
//!   `Transform` fait foi, la position absolue en est relue à chaque frame ;
//! - `Frame` (le vaisseau du joueur, à bord) : **le vaisseau est le référentiel** (AGENTS.md
//!   §6.2). L'origine prend sa position et son orientation à chaque frame : l'intérieur reste
//!   immobile autour de `(0, 0, 0)`, c'est l'extérieur qui se déplace et tourne.
//!
//! Les systèmes tournent en `PostUpdate`, après la physique (avian, `FixedPostUpdate`) et avant
//! la propagation des transformations.

use bevy::math::{DQuat, DVec3};
use bevy::prelude::*;
use sh_core::SpacePos;
use sh_core::space::needs_recenter;

use crate::data::Data;

pub struct SpacePlugin;

impl Plugin for SpacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FloatingOrigin>()
            .configure_sets(PostUpdate, SpaceSystems.before(TransformSystems::Propagate))
            .add_systems(
                PostUpdate,
                (
                    apply_settings.run_if(resource_added::<Data>),
                    read_render_driven,
                    recenter,
                    place_space_driven,
                )
                    .chain()
                    .in_set(SpaceSystems),
            );
    }
}

/// Les systèmes de l'origine flottante.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpaceSystems;

/// Position absolue d'une entité racine de l'espace, en mètres, dans le repère du système.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(Transform, PositionDriver)]
pub struct SpacePosition(pub SpacePos);

/// Ce qui fait foi pour la position d'une entité.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PositionDriver {
    /// La [`SpacePosition`] fait foi (astres, stations) : le `Transform` en est déduit.
    #[default]
    Space,
    /// Le `Transform` fait foi (physique, commandes) : la [`SpacePosition`] en est relue.
    Render,
    /// Le référentiel (le vaisseau du joueur quand il est à bord) : sa position et son
    /// orientation absolues font foi et **deviennent l'origine** ; son `Transform` reste nul, ses
    /// enfants (l'intérieur, le joueur) ne bougent pas dans le rendu.
    Frame,
}

/// Orientation absolue d'une entité de l'espace (identité si absente).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct SpaceRotation(pub Quat);

/// Le point que suit l'origine flottante (au plus une entité).
#[derive(Component, Default)]
pub struct OriginFocus;

/// L'origine flottante : la position absolue du point `(0, 0, 0)` du rendu, et l'orientation
/// absolue des axes du rendu. L'orientation n'est autre que l'identité, sauf quand un
/// référentiel ([`PositionDriver::Frame`], le vaisseau du joueur) l'impose : tout l'extérieur
/// tourne alors autour de lui.
#[derive(Resource, Clone, Debug)]
pub struct FloatingOrigin {
    pub origin: SpacePos,
    pub orientation: DQuat,
    pub recenter_distance: f64,
    /// Recentrages depuis le démarrage (mesures, tests).
    pub shifts: u32,
}

impl Default for FloatingOrigin {
    fn default() -> Self {
        Self {
            origin: SpacePos::ZERO,
            orientation: DQuat::IDENTITY,
            recenter_distance: 2000.0,
            shifts: 0,
        }
    }
}

fn to_dvec(p: SpacePos) -> DVec3 {
    DVec3::new(p.x, p.y, p.z)
}

impl FloatingOrigin {
    /// Position de rendu d'un point absolu (soustraction et rotation en `f64`).
    pub fn to_render(&self, pos: SpacePos) -> Vec3 {
        (self.orientation.inverse() * to_dvec(pos - self.origin)).as_vec3()
    }

    /// Position absolue d'un point de rendu.
    pub fn to_space(&self, translation: Vec3) -> SpacePos {
        let d = self.orientation * translation.as_dvec3();
        self.origin + SpacePos::new(d.x, d.y, d.z)
    }

    /// Orientation de rendu d'une orientation absolue.
    pub fn render_rotation(&self, absolute: Quat) -> Quat {
        self.orientation.inverse().as_quat() * absolute
    }

    /// Direction de rendu d'une direction absolue.
    pub fn render_direction(&self, absolute: DVec3) -> Vec3 {
        (self.orientation.inverse() * absolute).as_vec3()
    }
}

fn apply_settings(data: Res<Data>, mut origin: ResMut<FloatingOrigin>) {
    origin.recenter_distance = data.space.recenter_distance;
}

/// Entités pilotées par leur `Transform` : relit leur position absolue. À l'apparition, c'est
/// la position absolue donnée par celui qui les crée qui fait foi.
fn read_render_driven(
    origin: Res<FloatingOrigin>,
    mut query: Query<(&mut Transform, &mut SpacePosition, &PositionDriver)>,
) {
    for (mut transform, mut pos, driver) in &mut query {
        if *driver != PositionDriver::Render {
            continue;
        }
        if pos.is_added() {
            transform.translation = origin.to_render(pos.0);
        } else {
            let space = origin.to_space(transform.translation);
            if space != pos.0 {
                pos.0 = space;
            }
        }
    }
}

/// Recentre l'origine : sur le référentiel s'il y en a un (à chaque frame, position et
/// orientation), sinon sur le point suivi quand il s'en est trop éloigné.
fn recenter(
    mut origin: ResMut<FloatingOrigin>,
    frame: Query<(&SpacePosition, Option<&SpaceRotation>, &PositionDriver)>,
    focus: Query<&SpacePosition, With<OriginFocus>>,
    mut render_driven: Query<(&mut Transform, &SpacePosition, &PositionDriver)>,
) {
    let anchor = frame
        .iter()
        .find(|(_, _, d)| **d == PositionDriver::Frame)
        .map(|(p, r, _)| (p.0, r.map_or(DQuat::IDENTITY, |r| r.0.as_dquat())));
    let (target, orientation) = match anchor {
        Some(anchor) => anchor,
        None => {
            let Ok(focus) = focus.single() else {
                return;
            };
            let local = focus.0.relative_to(origin.origin);
            if !needs_recenter(local, origin.recenter_distance) {
                return;
            }
            (focus.0, DQuat::IDENTITY)
        }
    };
    if target == origin.origin && orientation == origin.orientation {
        return;
    }
    let turn = (orientation.inverse() * origin.orientation).as_quat();
    origin.origin = target;
    origin.orientation = orientation;
    origin.shifts += 1;
    for (mut transform, pos, driver) in &mut render_driven {
        if *driver == PositionDriver::Render {
            // Recalcul depuis le f64 : aucun arrondi ne s'accumule d'un recentrage à l'autre.
            transform.translation = origin.to_render(pos.0);
            transform.rotation = turn * transform.rotation;
        }
    }
}

/// Entités pilotées par leur position absolue : place leur `Transform` (position et
/// orientation) ; le référentiel reste à l'origine, sans rotation.
fn place_space_driven(
    origin: Res<FloatingOrigin>,
    mut query: Query<(
        &mut Transform,
        Ref<SpacePosition>,
        Option<Ref<SpaceRotation>>,
        &PositionDriver,
    )>,
) {
    let moved = origin.is_changed();
    for (mut transform, pos, rotation, driver) in &mut query {
        match driver {
            PositionDriver::Space => {
                let turned = rotation.as_ref().is_some_and(|r| r.is_changed());
                if moved || pos.is_changed() || turned {
                    transform.translation = origin.to_render(pos.0);
                    transform.rotation =
                        origin.render_rotation(rotation.map_or(Quat::IDENTITY, |r| r.0));
                }
            }
            PositionDriver::Frame => {
                if transform.translation != Vec3::ZERO || transform.rotation != Quat::IDENTITY {
                    transform.translation = Vec3::ZERO;
                    transform.rotation = Quat::IDENTITY;
                }
            }
            PositionDriver::Render => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(recenter_distance: f64) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, SpacePlugin))
            .insert_resource(FloatingOrigin {
                recenter_distance,
                ..default()
            });
        app
    }

    fn space_of(app: &App, e: Entity) -> SpacePos {
        app.world()
            .get::<SpacePosition>(e)
            .map_or(SpacePos::ZERO, |p| p.0)
    }

    fn translation_of(app: &App, e: Entity) -> Vec3 {
        app.world()
            .get::<Transform>(e)
            .map_or(Vec3::NAN, |t| t.translation)
    }

    #[test]
    fn recentrer_conserve_les_positions_absolues() {
        let mut app = app(2000.0);
        let base = SpacePos::new(1.0e10, 0.0, 0.0);
        let world = app.world_mut();
        let focus = world
            .spawn((SpacePosition(base), PositionDriver::Render, OriginFocus))
            .id();
        let wingman = world
            .spawn((
                SpacePosition(base + SpacePos::new(1.0, 0.0, 0.0)),
                PositionDriver::Render,
            ))
            .id();
        let planet = world
            .spawn(SpacePosition(base + SpacePos::new(1.5e8, 0.0, 0.0)))
            .id();
        app.update();
        // Premier recentrage : l'origine saute sur le point suivi.
        assert!((app.world().resource::<FloatingOrigin>().origin.x - 1.0e10).abs() < 1e-6);

        // Les deux vaisseaux avancent ensemble de 2,5 km : au-delà du seuil.
        for e in [focus, wingman] {
            if let Some(mut t) = app.world_mut().get_mut::<Transform>(e) {
                t.translation.x += 2500.0;
            }
        }
        app.update();
        let origin = app.world().resource::<FloatingOrigin>().clone();
        assert_eq!(origin.shifts, 2);
        assert!(translation_of(&app, focus).length() < 1e-6);
        assert!((translation_of(&app, wingman).x - 1.0).abs() < 1e-4);
        assert!((space_of(&app, wingman).x - (1.0e10 + 2501.0)).abs() < 1e-3);
        // L'astre lointain est replacé depuis sa position f64.
        let planet_x = translation_of(&app, planet).x;
        assert!((f64::from(planet_x) - (1.5e8 - 2500.0)).abs() < 16.0);
    }

    /// Vole `steps` frames à 5,123 m par frame depuis `start` ; renvoie l'erreur finale (m) de
    /// la position absolue par rapport au calcul exact.
    fn fly_error(start: SpacePos, steps: u32) -> f64 {
        let mut app = app(2000.0);
        let focus = app
            .world_mut()
            .spawn((SpacePosition(start), PositionDriver::Render, OriginFocus))
            .id();
        app.update();
        let step = Vec3::new(5.123, 0.0, -1.7);
        for _ in 0..steps {
            if let Some(mut t) = app.world_mut().get_mut::<Transform>(focus) {
                t.translation += step;
            }
            app.update();
        }
        let exact = start
            + SpacePos::new(f64::from(step.x), f64::from(step.y), f64::from(step.z))
                * f64::from(steps);
        space_of(&app, focus).distance(exact)
    }

    #[test]
    fn la_precision_ne_depend_pas_de_la_distance_au_centre() {
        // 100 km de vol, près du centre puis à 10⁹ km : même erreur, et faible.
        let near = fly_error(SpacePos::ZERO, 20_000);
        let far = fly_error(SpacePos::new(1.0e12, 3.0e11, -7.0e11), 20_000);
        assert!(near < 0.5, "erreur près du centre : {near} m");
        assert!((far - near).abs() < 1e-2, "près {near} m, loin {far} m");
        // Sans origine flottante, à 10⁹ m un f32 n'avance même pas de 5 m par frame.
        let naive = 1.0e9_f32;
        assert_eq!(naive + 5.123, naive);
    }

    #[test]
    fn le_vaisseau_est_le_referentiel() {
        let mut app = app(2000.0);
        let ship_pos = SpacePos::new(3.0e8, 0.0, -4.0e8);
        // Le vaisseau a tourné de 90° vers la gauche : son avant (−Z) pointe vers −X.
        let turned = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let ship = app
            .world_mut()
            .spawn((
                SpacePosition(ship_pos),
                SpaceRotation(turned),
                PositionDriver::Frame,
            ))
            .id();
        // Une station à 1 km au nord absolu (−Z).
        let station = app
            .world_mut()
            .spawn(SpacePosition(ship_pos + SpacePos::new(0.0, 0.0, -1000.0)))
            .id();
        app.update();
        // Le vaisseau reste à l'origine, sans rotation ; la station apparaît à sa droite.
        assert_eq!(translation_of(&app, ship), Vec3::ZERO);
        let seen = translation_of(&app, station);
        assert!(
            (seen - Vec3::new(1000.0, 0.0, 0.0)).length() < 1e-3,
            "{seen}"
        );
        let rotation = app
            .world()
            .get::<Transform>(station)
            .map_or(Quat::NAN, |t| t.rotation);
        assert!(rotation.angle_between(turned.inverse()) < 1e-3);

        // Le vaisseau avance de 500 m vers son avant : la station se rapproche d'autant.
        let forward = turned * Vec3::NEG_Z * 500.0;
        if let Some(mut p) = app.world_mut().get_mut::<SpacePosition>(ship) {
            p.0 += SpacePos::new(f64::from(forward.x), 0.0, f64::from(forward.z));
        }
        app.update();
        let seen = translation_of(&app, station);
        assert!(
            (seen - Vec3::new(1000.0, 0.0, 500.0)).length() < 1e-3,
            "{seen}"
        );
    }
}
