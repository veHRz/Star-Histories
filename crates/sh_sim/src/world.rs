//! L'état de la galaxie et son pas journalier (AGENTS.md §7).
//!
//! Déterministe : un seul générateur ChaCha (sous-flux par domaine), des `BTreeMap` partout,
//! aucun ordre dépendant d'une table de hachage. Même graine ⇒ même histoire.
//!
//! Ordre d'un jour : flottes (arrivées, réparations, patrouilles, errances) → économie (stocks,
//! pénuries) → convois et raids → escarmouches → stabilité → projets → calendrier.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use sh_core::{Id, MapPos, SimRng, StellarDate, Year};
use sh_data::sim::{Economy, StabilityRule};
use sh_data::{CalendarKind, Faction, Fleet, GameData, Place, Rails, SimSettings};

use crate::events::{EventKind, SimEvent};
use crate::travel::detour_length;

/// Les constantes tirées des données, partagées par toutes les copies du monde.
#[derive(Debug)]
struct Context {
    settings: SimSettings,
    rails: Rails,
    days_per_unit: f64,
    positions: BTreeMap<Id<Place>, MapPos>,
    obstacles: Vec<(MapPos, f64)>,
    public_systems: Vec<Id<Place>>,
    start: StellarDate,
    /// Nom et secret de chaque flotte des données (pillards sans lieu simulé compris).
    fleet_names: BTreeMap<Id<Fleet>, (String, bool)>,
}

#[derive(Clone, Debug)]
pub struct SystemState {
    pub stock: BTreeMap<String, f64>,
    pub economy: Economy,
    pub stability: Option<f64>,
    pub shortages: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mission {
    Station,
    Patrol { route: Vec<Id<Place>>, next: usize },
    Repair { per_day: f64 },
    Wander,
}

#[derive(Clone, Debug)]
pub struct FleetState {
    pub name: String,
    pub faction: Option<Id<Faction>>,
    pub location: Id<Place>,
    /// En route : destination et date d'arrivée.
    pub transit: Option<(Id<Place>, StellarDate)>,
    /// État de 0 (épave) à 1 (opérationnelle).
    pub condition: f64,
    pub mission: Mission,
    pub secret: bool,
    dwell: u32,
}

#[derive(Clone, Debug)]
pub struct ProjectState {
    pub name: String,
    pub progress: f64,
    pub secret: bool,
}

#[derive(Clone, Debug)]
struct Delivery {
    to: Id<Place>,
    commodity: String,
    amount: f64,
    date: StellarDate,
}

/// L'état global de la simulation.
#[derive(Clone, Debug)]
pub struct SimWorld {
    date: StellarDate,
    rng: SimRng,
    pub systems: BTreeMap<Id<Place>, SystemState>,
    pub fleets: BTreeMap<Id<Fleet>, FleetState>,
    pub projects: BTreeMap<String, ProjectState>,
    deliveries: Vec<Delivery>,
    history: Vec<SimEvent>,
    ctx: Option<Arc<Context>>,
}

/// Jours de stock visés pour une marchandise consommée.
const TARGET_DAYS: f64 = 30.0;
/// Jours de stock au départ de la partie.
const START_DAYS: f64 = 60.0;

impl SimWorld {
    /// Un monde vide au premier jour de `start`, tiré de la graine `seed`.
    pub fn new(seed: u64, start: StellarDate) -> Self {
        Self {
            date: start,
            rng: SimRng::new(seed),
            systems: BTreeMap::new(),
            fleets: BTreeMap::new(),
            projects: BTreeMap::new(),
            deliveries: Vec::new(),
            history: Vec::new(),
            ctx: None,
        }
    }

    /// Un monde vide au début de la partie : premier jour de 425at.
    pub fn at_present(seed: u64) -> Self {
        Self::new(seed, StellarDate::start_of(Year::PRESENT))
    }

    /// La galaxie en 425at depuis les données : canon (flottes, positions, §6.1) et réglages
    /// proposés (`sim.ron`). `seed` remplace la graine des données si elle est donnée.
    pub fn from_data(data: &GameData, seed: Option<u64>) -> Self {
        let g = &data.galaxy;
        let s = &data.sim;
        let mut positions = BTreeMap::new();
        for sys in &g.systems {
            positions.insert(sys.id.clone(), sys.center);
        }
        for b in &g.bodies {
            if let Some(p) = b.pos {
                positions.insert(b.id.clone(), p);
            }
        }
        for st in &g.stations {
            positions.insert(st.id.clone(), st.pos);
        }
        let start = StellarDate::start_of(Year::PRESENT);
        let ctx = Context {
            settings: s.clone(),
            rails: data.rails.clone(),
            days_per_unit: data.travel.hyperspace_days_per_unit,
            positions,
            obstacles: g
                .phenomena
                .iter()
                .filter(|p| p.blocks_hyperspace)
                .filter_map(|p| Some((p.pos?, p.radius? * data.travel.nebula_margin)))
                .collect(),
            public_systems: g
                .systems
                .iter()
                .filter(|s| !s.secret)
                .map(|s| s.id.clone())
                .collect(),
            start,
            fleet_names: g
                .fleets
                .iter()
                .map(|f| (f.id.clone(), (f.name.clone(), f.secret)))
                .collect(),
        };
        let mut world = Self::new(seed.unwrap_or(s.seed), start);

        for (id, economy) in &s.economies {
            // Stock de départ : deux mois de la consommation (ou de la production), de quoi
            // attendre les premiers convois, qui traversent parfois des semaines d'hyperespace.
            let mut stock: BTreeMap<String, f64> = BTreeMap::new();
            for (c, q) in economy.consumes.iter().chain(&economy.produces) {
                let s = stock.entry(c.clone()).or_default();
                *s = s.max(q * START_DAYS);
            }
            world.systems.insert(
                id.clone(),
                SystemState {
                    stock,
                    economy: economy.clone(),
                    stability: s.stability.get(id).map(|r| r.initial),
                    shortages: BTreeSet::new(),
                },
            );
        }
        for (id, rule) in &s.stability {
            world
                .systems
                .entry(id.clone())
                .or_insert_with(|| SystemState {
                    stock: BTreeMap::new(),
                    economy: Economy::default(),
                    stability: Some(rule.initial),
                    shortages: BTreeSet::new(),
                });
        }

        for f in &g.fleets {
            // Seules les flottes placées en un lieu connu sont simulées (pas les « mobiles »
            // sans lieu, ni les flottes détruites).
            // Position canon, sinon base (les meutes de la Ligue : « Rade-Noire / No Man's Land »).
            let known = |l: &sh_data::Location| {
                l.place()
                    .filter(|p| ctx.positions.contains_key(*p))
                    .cloned()
            };
            let Some(location) = known(&f.position).or_else(|| known(&f.base)) else {
                continue;
            };
            let wanderer = s.wanderers.contains(&f.id);
            let (location, mission) = if wanderer {
                (
                    f.base.place().cloned().unwrap_or_else(|| location.clone()),
                    Mission::Wander,
                )
            } else if let Some(repair) = s.repairs.get(&f.id) {
                (
                    location.clone(),
                    Mission::Repair {
                        per_day: repair.per_day,
                    },
                )
            } else if let Some(route) = s.patrols.get(&f.id) {
                (
                    location.clone(),
                    Mission::Patrol {
                        route: route.clone(),
                        next: 0,
                    },
                )
            } else {
                (location.clone(), Mission::Station)
            };
            world.fleets.insert(
                f.id.clone(),
                FleetState {
                    name: f.name.clone(),
                    faction: f.faction.clone(),
                    location,
                    transit: None,
                    condition: s.repairs.get(&f.id).map_or(1.0, |r| r.condition),
                    mission,
                    secret: f.secret,
                    dwell: 0,
                },
            );
        }
        // Les flottes « mobiles » dont la base est un lieu (Flotte Beta) partent de leur base.
        for f in g.fleets.iter().filter(|f| s.wanderers.contains(&f.id)) {
            if world.fleets.contains_key(&f.id) {
                continue;
            }
            if let Some(base) = f.base.place().filter(|p| ctx.positions.contains_key(*p)) {
                world.fleets.insert(
                    f.id.clone(),
                    FleetState {
                        name: f.name.clone(),
                        faction: f.faction.clone(),
                        location: base.clone(),
                        transit: None,
                        condition: 1.0,
                        mission: Mission::Wander,
                        secret: f.secret,
                        dwell: 0,
                    },
                );
            }
        }
        for p in &s.projects {
            world.projects.insert(
                p.id.clone(),
                ProjectState {
                    name: p.name.clone(),
                    progress: p.progress,
                    secret: p.secret,
                },
            );
        }
        // Les routes tournent depuis longtemps : les convois déjà en route arrivent dans les
        // premiers jours (Ostrag est à deux mois de Primaterre).
        for convoy in &s.convoys {
            let days = Self::travel_days(&ctx, &convoy.from, &convoy.to);
            let every = i64::from(convoy.every_days.max(1));
            let mut k = every;
            while k <= days {
                world.deliveries.push(Delivery {
                    to: convoy.to.clone(),
                    commodity: convoy.commodity.clone(),
                    amount: convoy.amount,
                    date: start.add_days(k),
                });
                k += every;
            }
        }
        world.ctx = Some(Arc::new(ctx));
        world
    }

    pub fn date(&self) -> StellarDate {
        self.date
    }

    pub fn seed(&self) -> u64 {
        self.rng.seed()
    }

    /// Tout ce qui s'est passé depuis le début, dans l'ordre.
    pub fn history(&self) -> &[SimEvent] {
        &self.history
    }

    /// Les événements publics seulement (nouvelles, rumeurs, connaissance du joueur).
    pub fn public_events(&self) -> impl Iterator<Item = &SimEvent> {
        self.history.iter().filter(|e| e.public)
    }

    /// Prix local d'une marchandise (crédits par unité), selon le stock au regard de la
    /// consommation (ou de la production) du système.
    pub fn price(&self, place: &Id<Place>, commodity: &str) -> Option<f64> {
        let ctx = self.ctx.as_ref()?;
        let base = *ctx.settings.base_prices.get(commodity)?;
        let sys = self.systems.get(place)?;
        let daily = sys
            .economy
            .consumes
            .get(commodity)
            .or_else(|| sys.economy.produces.get(commodity))
            .copied()
            .unwrap_or(0.0);
        if daily <= 0.0 {
            return Some(base);
        }
        let stock = sys.stock.get(commodity).copied().unwrap_or(0.0);
        let ratio = (daily * TARGET_DAYS / stock.max(1.0)).sqrt();
        Some(base * ratio.clamp(0.4, 4.0))
    }

    fn emit(&mut self, kind: EventKind, public: bool) {
        self.history.push(SimEvent {
            date: self.date,
            kind,
            public,
        });
    }

    /// Avance d'un jour (le pas de la simulation macro).
    pub fn step_day(&mut self) {
        self.date = self.date.add_days(1);
        let Some(ctx) = self.ctx.clone() else {
            return;
        };
        let day = self.date.day_index();
        // Un sous-flux par domaine et par jour : ajouter un tirage dans un domaine ne décale
        // ni les autres domaines, ni les autres jours.
        let stream = |name: &str| {
            self.rng.fork(
                sh_core::rng::fnv1a(name.as_bytes())
                    ^ day.unsigned_abs().wrapping_mul(0x9e37_79b9_7f4a_7c15),
            )
        };
        let (mut fleets, mut convoys, mut skirmishes, mut stability) = (
            stream("flottes"),
            stream("convois"),
            stream("escarmouches"),
            stream("stabilite"),
        );
        self.step_fleets(&ctx, &mut fleets);
        self.step_economy(&ctx);
        self.step_convoys(&ctx, &mut convoys);
        self.step_skirmishes(&ctx, &mut skirmishes);
        self.step_stability(&ctx, &mut stability);
        self.step_projects(&ctx);
        self.step_calendar(&ctx);
    }

    /// Avance de `days` jours.
    pub fn advance_days(&mut self, days: u32) {
        for _ in 0..days {
            self.step_day();
        }
    }

    /// Avance jusqu'à `date` (sans effet si elle est passée).
    pub fn advance_to(&mut self, date: StellarDate) {
        while self.date < date {
            self.step_day();
        }
    }

    fn travel_days(ctx: &Context, a: &Id<Place>, b: &Id<Place>) -> i64 {
        match (ctx.positions.get(a), ctx.positions.get(b)) {
            (Some(pa), Some(pb)) => {
                let (length, _) = detour_length(*pa, *pb, &ctx.obstacles);
                (length * ctx.days_per_unit).ceil().max(1.0) as i64
            }
            _ => 1,
        }
    }

    /// Une flotte tenue par un rail (Flotte d'Attaque, Alpha…) ne bouge pas.
    fn pinned(ctx: &Context, fleet: &Id<Fleet>, date: StellarDate) -> bool {
        ctx.rails
            .fleets_stay
            .iter()
            .any(|r| &r.fleet == fleet && r.until.is_none_or(|u| date < u))
    }

    fn step_fleets(&mut self, ctx: &Context, rng: &mut SimRng) {
        let ids: Vec<Id<Fleet>> = self.fleets.keys().cloned().collect();
        for id in ids {
            let pinned = Self::pinned(ctx, &id, self.date);
            let Some(fleet) = self.fleets.get_mut(&id) else {
                continue;
            };
            let mut events = Vec::new();
            if let Some((to, arrive)) = fleet.transit.clone()
                && self.date >= arrive
            {
                fleet.location = to.clone();
                fleet.transit = None;
                fleet.dwell = 0;
                events.push((
                    EventKind::FleetArrived {
                        fleet: id.clone(),
                        at: to,
                    },
                    !fleet.secret,
                ));
            }
            match fleet.mission.clone() {
                Mission::Repair { per_day } => {
                    fleet.condition = (fleet.condition + per_day).min(1.0);
                    if fleet.condition >= 1.0 {
                        fleet.mission = Mission::Station;
                        events.push((
                            EventKind::FleetRepaired { fleet: id.clone() },
                            !fleet.secret,
                        ));
                    }
                }
                Mission::Patrol { route, next } if !pinned && fleet.transit.is_none() => {
                    fleet.dwell += 1;
                    if fleet.dwell >= ctx.settings.patrol_dwell_days && !route.is_empty() {
                        let to = route[next % route.len()].clone();
                        let next = (next + 1) % route.len();
                        if to != fleet.location {
                            let days = Self::travel_days(ctx, &fleet.location, &to);
                            fleet.transit = Some((to, self.date.add_days(days)));
                        }
                        fleet.mission = Mission::Patrol { route, next };
                        fleet.dwell = 0;
                    }
                }
                Mission::Wander if !pinned && fleet.transit.is_none() => {
                    fleet.dwell += 1;
                    let dwell = ctx.settings.patrol_dwell_days + rng.range_u32(0, 10);
                    if fleet.dwell >= dwell
                        && let Some(to) = rng.pick(&ctx.public_systems).cloned()
                        && to != fleet.location
                    {
                        let days = Self::travel_days(ctx, &fleet.location, &to);
                        fleet.transit = Some((to, self.date.add_days(days)));
                        fleet.dwell = 0;
                    }
                }
                _ => {}
            }
            for (kind, public) in events {
                self.emit(kind, public);
            }
        }
    }

    fn step_economy(&mut self, _ctx: &Context) {
        let date = self.date;
        let (arrived, pending): (Vec<_>, Vec<_>) =
            self.deliveries.drain(..).partition(|d| d.date <= date);
        self.deliveries = pending;
        for d in arrived {
            if let Some(sys) = self.systems.get_mut(&d.to) {
                *sys.stock.entry(d.commodity).or_default() += d.amount;
            }
        }
        let mut events = Vec::new();
        for (id, sys) in &mut self.systems {
            for (c, q) in &sys.economy.produces {
                let cap =
                    4.0 * TARGET_DAYS * q.max(sys.economy.consumes.get(c).copied().unwrap_or(0.0));
                let stock = sys.stock.entry(c.clone()).or_default();
                *stock = (*stock + q).min(cap);
            }
            for (c, q) in &sys.economy.consumes {
                let stock = sys.stock.entry(c.clone()).or_default();
                *stock = (*stock - q).max(0.0);
                let days = *stock / q.max(1e-9);
                if days < 3.0 && sys.shortages.insert(c.clone()) {
                    events.push(EventKind::Shortage {
                        at: id.clone(),
                        commodity: c.clone(),
                    });
                } else if days > 10.0 && sys.shortages.remove(c) {
                    events.push(EventKind::Resupplied {
                        at: id.clone(),
                        commodity: c.clone(),
                    });
                }
            }
        }
        for kind in events {
            self.emit(kind, true);
        }
    }

    /// Distance d'un point à un segment (unités de carte).
    fn distance_to_route(p: MapPos, a: MapPos, b: MapPos) -> f64 {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 {
            (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (p.x - (a.x + dx * t)).hypot(p.y - (a.y + dy * t))
    }

    fn step_convoys(&mut self, ctx: &Context, rng: &mut SimRng) {
        let day = self.date.day_index();
        for convoy in &ctx.settings.convoys {
            let offset =
                i64::from(sh_core::rng::fnv1a(convoy.id.as_bytes()) as u32 % convoy.every_days);
            if (day + offset) % i64::from(convoy.every_days) != 0 {
                continue;
            }
            let (Some(a), Some(b)) = (
                ctx.positions.get(&convoy.from),
                ctx.positions.get(&convoy.to),
            ) else {
                continue;
            };
            let mut amount = convoy.amount;
            if let Some(src) = self.systems.get_mut(&convoy.from) {
                let stock = src.stock.entry(convoy.commodity.clone()).or_default();
                amount = amount.min(*stock);
                *stock -= amount;
            }
            // Pillards à portée de la route, qui chassent cette faction et ne l'épargnent pas.
            for r in &ctx.settings.raiders {
                // Des pillards sans lieu simulé (la Flotte Nordique, « aucune base ») chassent
                // quand même sur leurs terrains, au complet.
                let (name, secret) = ctx.fleet_names.get(&r.fleet).cloned().unwrap_or_default();
                let (condition, travelling) = self
                    .fleets
                    .get(&r.fleet)
                    .map_or((1.0, false), |f| (f.condition, f.transit.is_some()));
                let spared = ctx
                    .rails
                    .spares
                    .iter()
                    .any(|s| s.attacker == r.faction && s.spared == convoy.faction);
                let in_range = r.grounds.iter().any(|g| {
                    ctx.positions
                        .get(g)
                        .is_some_and(|p| Self::distance_to_route(*p, *a, *b) <= r.range)
                });
                if spared
                    || !r.targets.contains(&convoy.faction)
                    || condition < 0.3
                    || travelling
                    || !in_range
                    || !rng.chance(r.chance)
                {
                    continue;
                }
                let raiders = r.public_alias.clone().unwrap_or(name);
                let public = !secret || r.public_alias.is_some();
                let escort = convoy.escort.as_ref().filter(|e| {
                    self.fleets.get(*e).is_some_and(|f| {
                        f.transit.is_none()
                            && (f.location == convoy.from || f.location == convoy.to)
                    })
                });
                let success = rng.chance(if escort.is_some() {
                    r.escorted_success
                } else {
                    0.85
                });
                match (success, escort) {
                    (false, Some(escort)) => {
                        let kind = EventKind::ConvoyDefended {
                            convoy: convoy.id.clone(),
                            route: (convoy.from.clone(), convoy.to.clone()),
                            owner: convoy.faction.clone(),
                            raiders,
                            escort: escort.clone(),
                        };
                        self.emit(kind, public);
                    }
                    (false, None) => {}
                    (true, _) => {
                        let lost = amount * (0.15 + 0.3 * rng.unit_f64());
                        amount -= lost;
                        let kind = EventKind::ConvoyRaided {
                            convoy: convoy.id.clone(),
                            route: (convoy.from.clone(), convoy.to.clone()),
                            owner: convoy.faction.clone(),
                            raiders,
                            lost,
                        };
                        self.emit(kind, public);
                    }
                }
                break;
            }
            let days = Self::travel_days(ctx, &convoy.from, &convoy.to);
            self.deliveries.push(Delivery {
                to: convoy.to.clone(),
                commodity: convoy.commodity.clone(),
                amount,
                date: self.date.add_days(days),
            });
        }
    }

    fn faction_strength(&self, faction: &Id<Faction>) -> f64 {
        self.fleets
            .values()
            .filter(|f| f.faction.as_ref() == Some(faction))
            .map(|f| f.condition)
            .sum::<f64>()
            .max(0.1)
    }

    fn step_skirmishes(&mut self, ctx: &Context, rng: &mut SimRng) {
        for s in &ctx.settings.skirmishes {
            if !rng.chance(s.chance_per_day) {
                continue;
            }
            let Some(at) = rng.pick(&s.places).cloned() else {
                continue;
            };
            let (sa, sb) = (self.faction_strength(&s.a), self.faction_strength(&s.b));
            let a_wins = rng.unit_f64() < sa / (sa + sb);
            let (winner, loser) = if a_wins { (&s.a, &s.b) } else { (&s.b, &s.a) };
            for f in self
                .fleets
                .values_mut()
                .filter(|f| f.faction.as_ref() == Some(loser))
            {
                f.condition = (f.condition - 0.02).max(0.35);
            }
            for f in self
                .fleets
                .values_mut()
                .filter(|f| f.faction.as_ref() == Some(winner))
            {
                f.condition = (f.condition + 0.01).min(1.0);
            }
            let kind = EventKind::Skirmish {
                winner: winner.clone(),
                loser: loser.clone(),
                at,
            };
            self.emit(kind, true);
        }
    }

    fn step_stability(&mut self, ctx: &Context, rng: &mut SimRng) {
        let mut revolts = Vec::new();
        for (id, rule) in &ctx.settings.stability {
            let Some(sys) = self.systems.get_mut(id) else {
                continue;
            };
            let Some(s) = sys.stability.as_mut() else {
                continue;
            };
            let hungry = sys.shortages.contains("food");
            let StabilityRule {
                revolt_below,
                recovery_per_day,
                pressure_per_day,
                after_repression,
                ..
            } = *rule;
            let pressure = pressure_per_day * if hungry { 3.0 } else { 1.0 };
            *s =
                (*s + recovery_per_day - pressure + 0.002 * (rng.unit_f64() - 0.5)).clamp(0.0, 1.0);
            if *s < revolt_below {
                *s = after_repression;
                revolts.push(id.clone());
            }
        }
        for at in revolts {
            self.emit(EventKind::Revolt { at }, true);
        }
    }

    fn step_projects(&mut self, ctx: &Context) {
        let years = f64::from(ctx.start.days_until(self.date) as i32) / 365.0;
        let mut events = Vec::new();
        for def in &ctx.settings.projects {
            let Some(p) = self.projects.get_mut(&def.id) else {
                continue;
            };
            if p.progress >= 1.0 {
                continue;
            }
            let before = p.progress;
            let rate = def.per_day * (1.0 + def.acceleration_per_year * years);
            let mut after = (before + rate).min(1.0);
            // Rail : pas d'achèvement avant la date du canon.
            if let Some(rail) = ctx
                .rails
                .projects_not_before
                .iter()
                .find(|r| r.project == def.id)
                && self.date < rail.not_before
            {
                after = after.min(0.995);
            }
            p.progress = after;
            let (b10, a10) = (
                (before * 10.0).floor() as u32,
                (after * 10.0).floor() as u32,
            );
            if a10 > b10 {
                events.push((
                    EventKind::ProjectMilestone {
                        project: def.id.clone(),
                        name: def.name.clone(),
                        percent: a10 * 10,
                    },
                    !def.secret,
                ));
            }
        }
        for (kind, public) in events {
            self.emit(kind, public);
        }
    }

    fn step_calendar(&mut self, ctx: &Context) {
        for c in &ctx.settings.calendar {
            if c.date == self.date {
                let kind = match c.kind {
                    CalendarKind::Arena => EventKind::Arena {
                        at: c.place.clone(),
                    },
                    CalendarKind::UglCampaign => EventKind::UglCampaign {
                        at: c.place.clone(),
                    },
                };
                self.emit(kind, true);
            }
        }
    }
}
