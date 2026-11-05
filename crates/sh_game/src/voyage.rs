//! Voyages interstellaires (M3) : où est le vaisseau, la carte, l'itinéraire, le départ.
//!
//! - Aux commandes en vol, `M` ouvre la carte galactique ; `M` la referme et le vaisseau reprend
//!   sa route là où il était ([`ResumeFlight`]).
//! - Sur la carte, un clic choisit une destination : un système, ou depuis M9 un monde isolé
//!   (Q18 : par l'hyperespace, arrivée en orbite). Les itinéraires par Portail et par
//!   hyperespace sont chiffrés ([`RoutePlan`], `sh_sim::travel`) ; `1` ou `2` part.
//! - Le départ est une transition ([`crate::transition::Sequence::Travel`]) : la date avance de
//!   la durée du voyage, le carburant baisse, et l'on arrive en vue Globale à destination.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use sh_core::{Id, SpacePos};
use sh_data::Place;
use sh_data::travel::{Endpoint, endpoint};
use sh_sim::travel::{TravelPlan, hyperspace_blocked, hyperspace_plan, portal_plan};

use crate::data::Data;
use crate::input::{Select, ToggleMap, TravelHyperspace, TravelPortal};
use crate::sector::{Aboard, ActiveSector, PendingAboard};
use crate::space::{SpacePosition, SpaceRotation};
use crate::state::{AppState, ControlMode, CurrentPlace, ViewLevel};
use crate::transition::Sequence;

pub struct VoyagePlugin;

impl Plugin for VoyagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredEntry>()
            .add_systems(OnEnter(AppState::InGame), init_voyage)
            .add_observer(on_toggle_map)
            .add_observer(on_select)
            .add_observer(on_travel_portal)
            .add_observer(on_travel_hyperspace);
    }
}

/// Le lieu où se trouve le vaisseau du joueur : un système, ou depuis M9 un **monde isolé**
/// (astre hors système, en orbite). Le champ garde son nom historique : c'est l'identifiant
/// d'une extrémité de voyage (`sh_data::travel::endpoint`), que lisent la carte, les voyages et
/// la sauvegarde ; [`Location::endpoint`] le résout.
#[derive(Resource, Clone, Debug)]
pub struct Location {
    pub system: Id<Place>,
}

impl Location {
    /// Le système ou le monde isolé où l'on est (`None` : lieu inconnu ou devenu secret).
    pub fn endpoint<'a>(&self, data: &'a Data) -> Option<Endpoint<'a>> {
        endpoint(&data.galaxy, &self.system)
    }
}

/// Le carburant lourd du vaisseau du joueur.
#[derive(Resource, Clone, Debug)]
pub struct ShipFuel {
    pub units: f64,
    pub capacity: f64,
}

/// La pose du vaisseau quand on a ouvert la carte : il la reprend en refermant la carte.
#[derive(Resource, Clone, Debug)]
pub struct ResumeFlight {
    pub system: Id<Place>,
    pub pos: SpacePos,
    pub rot: Quat,
}

/// L'entrée de la carte sous le curseur (posée par la vue Galaxie).
#[derive(Resource, Default, Clone, Debug)]
pub struct HoveredEntry(pub Option<sh_data::MapEntry>);

/// La destination choisie et ses itinéraires.
#[derive(Resource, Clone, Debug)]
pub struct RoutePlan {
    pub dest: Id<Place>,
    pub portal: Option<TravelPlan>,
    pub hyperspace: Option<TravelPlan>,
    /// La destination est au cœur d'une nébuleuse qui bloque l'hyperespace (Rade-Noire).
    pub hyperspace_blocked: bool,
}

fn init_voyage(
    mut commands: Commands,
    data: Res<Data>,
    location: Option<Res<Location>>,
    fuel: Option<Res<ShipFuel>>,
) {
    let Some(start) = data.start() else {
        return;
    };
    if location.is_none() {
        commands.insert_resource(Location {
            system: start.system.clone(),
        });
    }
    if fuel.is_none()
        && let Some(ship) = data.travel.ships.get(&start.ship)
    {
        commands.insert_resource(ShipFuel {
            units: ship.fuel_capacity,
            capacity: ship.fuel_capacity,
        });
    }
}

/// À la reprise après la carte ou un voyage : aux commandes.
pub fn helm_aboard(data: &Data) -> Option<Aboard> {
    let start = data.start()?;
    Some(Aboard {
        spawn: start.aboard_spawn.clone(),
        seat_role: Some("pilot".into()),
    })
}

fn on_toggle_map(_: On<Start<ToggleMap>>, mut commands: Commands) {
    commands.queue(toggle_map);
}

/// Ouvre la carte (aux commandes, en vol) ou la referme (retour en vol).
pub fn toggle_map(world: &mut World) {
    if world.contains_resource::<Sequence>() {
        return;
    }
    let view = world.get_resource::<State<ViewLevel>>().map(|s| *s.get());
    let mode = world.get_resource::<State<ControlMode>>().map(|s| *s.get());
    let aboard = world.resource::<Data>().clone();
    match view {
        Some(ViewLevel::Global) if mode == Some(ControlMode::Piloting) => {
            let root = world.get_resource::<ActiveSector>().map(|s| s.root);
            let location = world.get_resource::<Location>().map(|l| l.system.clone());
            if let (Some(root), Some(system)) = (root, location) {
                let pos = world.get::<SpacePosition>(root).map(|p| p.0);
                let rot = world.get::<SpaceRotation>(root).map(|r| r.0);
                if let (Some(pos), Some(rot)) = (pos, rot) {
                    world.insert_resource(ResumeFlight { system, pos, rot });
                }
            }
            world
                .resource_mut::<NextState<ViewLevel>>()
                .set(ViewLevel::Galaxy);
        }
        Some(ViewLevel::Galaxy) => {
            let Some(system) = world.get_resource::<Location>().map(|l| l.system.clone()) else {
                return;
            };
            if let Some(helm) = helm_aboard(&aboard) {
                world.insert_resource(PendingAboard(helm));
            }
            world.resource_mut::<CurrentPlace>().0 = Some(system.to_string());
            world.remove_resource::<RoutePlan>();
            world
                .resource_mut::<NextState<ViewLevel>>()
                .set(ViewLevel::Global);
        }
        _ => {}
    }
}

fn on_select(
    _: On<Start<Select>>,
    data: Res<Data>,
    hovered: Res<HoveredEntry>,
    location: Option<Res<Location>>,
    view: Res<State<ViewLevel>>,
    mut commands: Commands,
) {
    if *view.get() != ViewLevel::Galaxy {
        return;
    }
    let (Some(entry), Some(location)) = (&hovered.0, location) else {
        return;
    };
    // Un système ou un monde isolé public ; ni station, ni phénomène, ni le lieu où l'on est.
    if !entry.destination
        || endpoint(&data.galaxy, &entry.id).is_none()
        || entry.id == location.system
    {
        return;
    }
    commands.insert_resource(plan_route(&data, &location.system, &entry.id));
}

/// Les itinéraires d'un lieu à un autre (systèmes ou mondes isolés), pour le vaisseau du joueur.
pub fn plan_route(data: &Data, from: &Id<Place>, to: &Id<Place>) -> RoutePlan {
    let ship = data
        .start()
        .and_then(|s| data.travel.ships.get(&s.ship))
        .cloned()
        .unwrap_or(sh_data::ShipLogistics {
            fuel_capacity: 0.0,
            fuel_per_day: 0.0,
        });
    RoutePlan {
        dest: to.clone(),
        portal: portal_plan(
            &data.galaxy,
            &data.portal_links,
            &data.travel,
            &ship,
            from,
            to,
        ),
        hyperspace: hyperspace_plan(&data.galaxy, &data.travel, &ship, from, to),
        hyperspace_blocked: hyperspace_blocked(&data.galaxy, &data.travel, to),
    }
}

fn on_travel_portal(_: On<Start<TravelPortal>>, mut commands: Commands) {
    commands.queue(|world: &mut World| depart(world, sh_sim::travel::TravelMode::Portal));
}

fn on_travel_hyperspace(_: On<Start<TravelHyperspace>>, mut commands: Commands) {
    commands.queue(|world: &mut World| depart(world, sh_sim::travel::TravelMode::Hyperspace));
}

/// Part vers la destination choisie, par ce mode, si l'itinéraire existe et que le carburant
/// suffit. Appelée par les touches ou par l'autopilote.
pub fn depart(world: &mut World, mode: sh_sim::travel::TravelMode) {
    if world.contains_resource::<Sequence>() {
        return;
    }
    let Some(route) = world.get_resource::<RoutePlan>().cloned() else {
        return;
    };
    let plan = match mode {
        sh_sim::travel::TravelMode::Portal => route.portal,
        sh_sim::travel::TravelMode::Hyperspace => route.hyperspace,
    };
    let Some(plan) = plan else {
        return;
    };
    let fuel = world.get_resource::<ShipFuel>().map_or(0.0, |f| f.units);
    if !plan.feasible || plan.fuel > fuel {
        return;
    }
    crate::transition::start_travel(world, route.dest, plan);
}
