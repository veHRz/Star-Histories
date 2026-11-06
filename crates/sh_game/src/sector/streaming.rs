//! Streaming des cellules : qualité Locale autour du joueur, Sectorielle là où l'on voit sans
//! modèle complet, rien ailleurs (AGENTS.md §6.2). Le plan de chaque frame est une fonction pure
//! ([`plan_cells`]), testée sans `World`.
//!
//! **Ce qui est chargé** (qualité Locale : modèle complet, collisions, interactions) :
//! - la cellule du joueur et celles à au plus [`LOCAL_DEPTH`] **portes** d'elle. Une
//!   **ouverture** ne compte pas (`Sector::door_depths`) : les cellules qu'elle relie se voient
//!   d'un bloc et se chargent d'un bloc (M11 : le hall, derrière l'ouverture du couloir du
//!   spatioport, est prêt dès le hangar ; on ne le voit plus se charger) ;
//! - **anticipation** : à moins de [`APPROACH_DISTANCE`] d'une porte, ou quand elle s'ouvre, les
//!   cellules autour de la salle d'en face se chargent déjà ; les deux côtés d'une porte qui
//!   s'ouvre sont toujours voulus (une porte de chargement n'attend jamais une cellule oubliée) ;
//! - une cellule qu'on voit (porte ouverte) n'est pas déchargée tant qu'on la voit ;
//! - **vaisseau amarré** : hors du bord, ses portes intérieures n'existent pas pour le streaming.
//!   Seul son sas, voisin du hangar, est chargé ; ses autres salles, cachées dans sa coque, ne
//!   sont pas instanciées du tout (leurs modèles restent préchargés jusqu'à [`PRELOAD_DEPTH`]).
//!   À bord, en vol comme à quai, tout le vaisseau suit les règles ordinaires.
//!
//! **Ce qui est affiché** ([`CellPlan::shown`]) : **occultation par les portails**. La cellule du
//! joueur et ce qu'on en voit de proche en proche (`Sector::seen_from`) : par les ouvertures, les
//! portes ouvertes, en mouvement ou qui vont s'ouvrir (la salle d'en face est là avant que le
//! panneau ne bouge), et les vitrages déclarés (`Cell::windows` : la fenêtre du chantier sur le
//! hangar ; la verrière du cockpit, qui montre le hangar du dedans et le cockpit du dehors). Une
//! cellule chargée hors de vue est **cachée** (`Visibility::Hidden` : rien n'est dessiné derrière
//! une cloison, ses lumières ne sont plus extraites, ses collisions et interactions restent ; elle
//! réapparaît dès qu'une porte s'ouvre), une cellule visible sans modèle complet prêt montre sa
//! **coque simplifiée** (qualité Sectorielle) ; au-delà, rien. La coque extérieure du vaisseau
//! amarré s'affiche avec son berceau ([`CellPlan::berth`]) : dans le sas, porte extérieure
//! fermée, ni le hangar ni la coque ne se dessinent. Une salle aux vitrages **non déclarés**
//! (données d'avant M11) garde par prudence la règle d'avant : ce qui l'entoure est vu, ses
//! voisines directes sont affichées (une fenêtre peut les montrer).
//!
//! **Sans à-coup** : une instance naît cachée et n'apparaît qu'une fois prête (post-traitement de
//! `gltf` fait) : la frame de l'instanciation et celle du premier affichage sont distinctes. Au
//! plus [`SPAWNS_PER_FRAME`] modèle complet instancié, [`UNLOADS_PER_FRAME`] déchargé,
//! [`LOD_SPAWNS_PER_FRAME`] coques simplifiées et [`REVEALS_PER_FRAME`] apparition d'une salle
//! qu'on ne voit peut-être pas (porte fermée) par frame ; ce qu'on voit à coup sûr apparaît sans
//! attendre. Une cellule affichée qu'on décharge passe d'abord la main à sa coque simplifiée,
//! prête et cachée jusque-là : jamais de trou, jamais deux modèles l'un dans l'autre.
//!
//! **Hystérésis** : on ne change de cellule courante qu'une fois franchement entré dans une autre,
//! et une cellule qui n'est plus voulue en Locale le reste [`UNLOAD_GRACE`] secondes.
//!
//! **Cellules imbriquées** (le vaisseau amarré dans le hangar) : la cellule courante est la plus
//! petite qui contient le joueur (`Sector::cell_at`) ; entrer dans une cellule emboîtée dans la
//! courante se fait sans attendre la marge d'hystérésis.
//!
//! **Arrivée** : l'écran de chargement (voile du HUD, fondu d'une transition) reste tant que la
//! cellule du joueur et la base Locale ([`CellPlan::local`]) ne sont pas toutes prêtes
//! ([`CellStates::arrival_ready`]), au plus [`ARRIVAL_MAX_WAIT`] secondes pour les voisines : on
//! n'arrive plus dans des pièces vides où seules les portes et les lampes sont là.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use sh_data::sector::is_external;
use sh_data::{Portal, PortalKind, Sector};

use crate::graphics::GraphicsSettings;
use crate::player::Player;
use crate::sector::doors::{Door, DoorState};
use crate::sector::{ActiveSector, ShipHull, placement_transform, v3};
use crate::state::ViewLevel;

pub struct StreamingPlugin;

impl Plugin for StreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CellStates>()
            .add_systems(OnExit(ViewLevel::Sector), reset_states)
            .add_systems(OnExit(ViewLevel::Global), reset_states)
            .add_systems(
                Update,
                // Condition sur chaque système d'un groupe chaîné (AGENTS.md §4).
                (
                    track_current_cell.run_if(resource_exists::<ActiveSector>),
                    stream_cells
                        .run_if(resource_exists::<ActiveSector>)
                        .in_set(StreamCells),
                )
                    .chain(),
            );
    }
}

/// Le plan du streaming et la visibilité des cellules. Les portes s'animent avant lui
/// (`doors::animate_doors`) : une porte qui bouge dans une frame a déjà sa salle d'en face
/// affichée dans cette même frame.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StreamCells;

fn reset_states(mut states: ResMut<CellStates>) {
    *states = CellStates::default();
}

/// Portes entre la cellule du joueur et les cellules voulues en qualité Locale (les ouvertures ne
/// comptent pas).
pub const LOCAL_DEPTH: usize = 1;
/// Portes jusqu'où les modèles complets se préchargent (poignées seulement).
pub const PRELOAD_DEPTH: usize = 2;
pub const UNLOAD_GRACE: f32 = 2.0;
pub const SPAWNS_PER_FRAME: usize = 1;
/// Modèles complets déchargés par frame (un grand arbre d'entités et ses collisions à retirer).
pub const UNLOADS_PER_FRAME: usize = 1;
/// Coques simplifiées instanciées par frame (quelques maillages chacune).
pub const LOD_SPAWNS_PER_FRAME: usize = 4;
/// Premières apparitions par frame des salles qu'on ne voit peut-être pas (voisines derrière une
/// porte fermée) : quand la cellule courante change, elles n'apparaissent pas toutes à la fois.
pub const REVEALS_PER_FRAME: usize = 1;
/// Distance horizontale (m) sous laquelle une porte annonce la salle d'en face.
pub const APPROACH_DISTANCE: f32 = 3.0;
/// Écart vertical (m) au-delà duquel une porte n'est pas « proche » (un autre niveau).
const APPROACH_HEIGHT: f32 = 2.5;
/// Marge de sortie de la cellule courante (hystérésis), en mètres.
const EXIT_MARGIN: f32 = 0.35;
/// Attente maximale des cellules voisines à l'arrivée dans un secteur, en secondes : au-delà,
/// l'écran de chargement se lève dès que la cellule du joueur est prête (un modèle manquant ne
/// bloque pas la partie).
pub const ARRIVAL_MAX_WAIT: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Local,
    Sector,
}

/// Une instance de cellule dans le monde.
#[derive(Component, Clone, Debug)]
pub struct CellInstance {
    pub cell: String,
    pub quality: Quality,
}

/// Posé par le post-traitement glTF quand l'instance est complète et prête.
#[derive(Component)]
pub struct CellReady;

#[derive(Default)]
pub struct CellSlot {
    pub full_handle: Option<Handle<WorldAsset>>,
    pub lod_handle: Option<Handle<WorldAsset>>,
    /// Lightmap de la cellule, chargée avec ses modèles (aucune cellule n'apparaît sans).
    pub lightmap_handle: Option<Handle<Image>>,
    pub full: Option<Entity>,
    pub lod: Option<Entity>,
    /// Secondes écoulées depuis que la cellule n'est plus voulue en Locale.
    pub unwanted_for: f32,
    /// Visibilité appliquée au modèle complet (il naît caché).
    pub full_visible: bool,
    /// Visibilité appliquée à la coque simplifiée (elle naît cachée).
    pub lod_visible: bool,
}

/// État du streaming : cellule courante et emplacements par cellule.
#[derive(Resource, Default)]
pub struct CellStates {
    pub current: Option<String>,
    pub slots: BTreeMap<String, CellSlot>,
    /// Transitions de cellule courante depuis l'entrée dans le secteur (mesures).
    pub transitions: u32,
    /// La base Locale autour de la cellule courante (elle comprise) : ce que l'arrivée attend.
    pub local: Vec<String>,
    /// Les cellules affichées ([`CellPlan::shown`]) ; les autres sont cachées ou absentes.
    pub shown: BTreeSet<String>,
    /// Secondes écoulées depuis l'entrée dans le secteur.
    pub waited: f32,
    /// Pour chaque cellule, celles qu'on voit avec elle par des ouvertures (elle comprise).
    groups: BTreeMap<String, Vec<String>>,
    /// Visibilité appliquée à la coque du vaisseau amarré.
    hull_visible: Option<bool>,
}

impl CellStates {
    /// La cellule est-elle prête en qualité Locale (collisions comprises) ?
    pub fn is_local_ready(&self, cell: &str, ready: &Query<(), With<CellReady>>) -> bool {
        self.slots
            .get(cell)
            .and_then(|s| s.full)
            .is_some_and(|e| ready.contains(e))
    }

    /// La cellule et celles qu'on voit avec elle par des ouvertures sont-elles toutes prêtes en
    /// qualité Locale ? C'est ce qu'une porte de chargement doit attendre : derrière elle, on voit
    /// tout le bloc (la porte du couloir du spatioport montre aussi le hall).
    pub fn is_side_ready(&self, cell: &str, ready: &Query<(), With<CellReady>>) -> bool {
        match self.groups.get(cell) {
            Some(group) => group.iter().all(|c| self.is_local_ready(c, ready)),
            None => self.is_local_ready(cell, ready),
        }
    }

    /// L'arrivée est-elle prête à être montrée ? La cellule du joueur est prête en qualité
    /// Locale, et la base Locale aussi (ou [`ARRIVAL_MAX_WAIT`] est écoulé). Lu par le voile de
    /// chargement du HUD et par la fin des transitions (`transition::Sequence::Arriving`).
    pub fn arrival_ready(&self, ready: &Query<(), With<CellReady>>) -> bool {
        let Some(current) = self.current.as_deref() else {
            return false;
        };
        self.is_local_ready(current, ready)
            && (self.waited >= ARRIVAL_MAX_WAIT
                || self.local.iter().all(|c| self.is_local_ready(c, ready)))
    }
}

/// Les portes vues du streaming (portails de genre `Door`, par identifiant).
#[derive(Clone, Debug, Default)]
pub struct DoorView {
    /// On voit au travers : ouvertes, en mouvement, ou sur le point de s'ouvrir.
    pub open: BTreeSet<String>,
    /// Elles s'ouvrent (attente de la cellule d'en face comprise).
    pub opening: BTreeSet<String>,
    /// À moins de [`APPROACH_DISTANCE`] du joueur.
    pub near: BTreeSet<String>,
}

impl DoorView {
    fn read(doors: &Query<&Door>, near: BTreeSet<String>) -> Self {
        let mut view = DoorView { near, ..default() };
        for door in doors {
            // Une porte qui attend de s'ouvrir compte déjà comme ouverte : la cellule d'en face
            // est affichée avant que le panneau ne bouge.
            if door.state != DoorState::Closed || door.openness > 0.0 {
                view.open.insert(door.portal.clone());
            }
            if matches!(door.state, DoorState::Unlocking | DoorState::Opening) {
                view.opening.insert(door.portal.clone());
            }
        }
        view
    }
}

/// Ce que le streaming veut pour une cellule courante, l'état des portes et les modèles déjà là.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CellPlan {
    /// La base Locale : la cellule courante (en tête) et celles à au plus [`LOCAL_DEPTH`] portes
    /// d'elle. L'arrivée les attend ([`CellStates::arrival_ready`]).
    pub local: Vec<String>,
    /// Toutes les cellules voulues en qualité Locale, par priorité d'instanciation : la base, ce
    /// qu'on voit et qui est déjà là, l'anticipation, les deux côtés d'une porte qui s'ouvre.
    pub wanted: Vec<String>,
    /// Les cellules affichées ; les autres sont cachées (modèle chargé) ou absentes.
    pub shown: BTreeSet<String>,
    /// Parmi elles, celles qu'on voit à coup sûr : elles apparaissent sans attendre leur tour.
    pub seen: BTreeSet<String>,
    /// Les cellules dont les modèles complets se préchargent (poignées, rien d'instancié).
    pub preload: BTreeSet<String>,
    /// La cellule qui abrite le vaisseau amarré (le hangar) : sa coque s'affiche avec elle.
    pub berth: Option<String>,
}

/// Le plan du streaming. `ship_cells` : les salles du vaisseau du joueur dans ce secteur (toutes
/// en vol, celles du vaisseau amarré à quai, aucune ailleurs) ; `loaded` : les cellules dont le
/// modèle complet est instancié.
pub fn plan_cells(
    sector: &Sector,
    current: &str,
    ship_cells: &[String],
    doors: &DoorView,
    loaded: &BTreeSet<String>,
) -> CellPlan {
    let is_ship = |c: &str| ship_cells.iter().any(|s| s == c);
    // Hors du vaisseau amarré, ses portes intérieures n'existent pas : seul son sas, voisin du
    // quai, se charge ; ses autres salles, cachées dans sa coque, ne sont pas instanciées.
    let outside = !is_ship(current);
    let pass = |p: &Portal| !(outside && is_ship(&p.a) && is_ship(&p.b));
    let is_door =
        |p: &Portal| p.kind == PortalKind::Door && !is_external(&p.a) && !is_external(&p.b);

    // Occultation par les portails : on voit, de proche en proche depuis la cellule courante, par
    // les ouvertures, les portes ouvertes (ou qui vont s'ouvrir) et les vitrages déclarés
    // (`Sector::seen_from` : la fenêtre du chantier, la verrière du cockpit sur le hangar et, vue
    // du hangar, le cockpit derrière elle). Le reste est caché.
    let open = |p: &Portal| doors.open.contains(&p.id);
    // Une salle aux vitrages non déclarés (données d'avant M11) peut avoir une verrière sur ce
    // qui l'entoure, et une fenêtre sur ses voisines : par prudence, l'un est vu, les autres
    // affichées (à leur tour), comme avant l'occultation.
    let undeclared = |c: &str| sector.cell(c).is_some_and(|c| c.windows.is_none());
    let mut sources = vec![current];
    if undeclared(current) {
        sources.extend(
            sector
                .containers(current)
                .into_iter()
                .map(|c| c.id.as_str()),
        );
    }
    let seen = sector.seen_from(&sources, open);
    let mut shown = seen.clone();
    for p in sector.portals.iter().filter(|p| is_door(p) && pass(p)) {
        for (here, there) in [(&p.a, &p.b), (&p.b, &p.a)] {
            if sources.contains(&here.as_str()) && undeclared(here) {
                shown.insert(there.as_str());
            }
        }
    }
    // Par un passage (sans vitrage) : ce qui, déjà là, reste chargé tant qu'on le voit.
    let reached = sector.reached_from(&[current], open);
    // Le berceau : la plus petite cellule hors du vaisseau qui entoure ses salles (le hangar).
    let berth = ship_cells
        .first()
        .filter(|_| ship_cells.len() < sector.cells.len())
        .and_then(|first| {
            sector
                .containers(first)
                .into_iter()
                .find(|c| !is_ship(&c.id))
        })
        .map(|c| c.id.as_str());

    // La base Locale : la cellule courante, ce qu'on voit, ce qu'on peut voir, puis par porte.
    let depths = sector.door_depths(&[current], pass);
    let depth = |c: &str| depths.get(c).copied().unwrap_or(usize::MAX);
    let mut local: Vec<&str> = depths
        .iter()
        .filter(|(_, d)| **d <= LOCAL_DEPTH)
        .map(|(c, _)| *c)
        .collect();
    local.sort_by_key(|c| {
        (
            *c != current,
            !seen.contains(c),
            !shown.contains(c),
            depth(c),
            *c,
        )
    });

    // Anticipation : une porte proche du bloc courant (ou qui s'ouvre) annonce la salle d'en face,
    // et ce qui l'entoure se charge avant qu'on y entre.
    let group: BTreeSet<&str> = depths
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(c, _)| *c)
        .collect();
    let mut centers: Vec<&str> = Vec::new();
    for p in sector.portals.iter().filter(|p| is_door(p) && pass(p)) {
        if !(doors.near.contains(&p.id) || doors.opening.contains(&p.id)) {
            continue;
        }
        for (here, there) in [(&p.a, &p.b), (&p.b, &p.a)] {
            if group.contains(here.as_str()) && !group.contains(there.as_str()) {
                centers.push(there.as_str());
            }
        }
    }
    let ring = sector.door_depths(&centers, pass);
    let mut ahead: Vec<&str> = ring
        .iter()
        .filter(|(_, d)| **d <= LOCAL_DEPTH)
        .map(|(c, _)| *c)
        .collect();
    ahead.sort_by_key(|c| (ring.get(c).copied().unwrap_or(usize::MAX), *c));
    // Une porte qui s'ouvre attend ses deux côtés : ils sont voulus, où qu'elle soit.
    let opening_sides = sector
        .portals
        .iter()
        .filter(|p| doors.opening.contains(&p.id))
        .flat_map(|p| [p.a.as_str(), p.b.as_str()])
        .filter(|c| sector.cell(c).is_some());
    // Ce qu'on voit par un passage et qui est déjà là ne se décharge pas sous nos yeux.
    let kept = reached.iter().copied().filter(|c| loaded.contains(*c));
    let mut wanted: Vec<&str> = Vec::new();
    for c in local
        .iter()
        .copied()
        .chain(kept)
        .chain(ahead)
        .chain(opening_sides)
    {
        if !wanted.contains(&c) {
            wanted.push(c);
        }
    }

    // Préchargement : toutes les portes comptent (le vaisseau amarré compris : retour rapide).
    let mut from = vec![current];
    from.extend(&centers);
    let preload = sector
        .door_depths(&from, |_| true)
        .into_iter()
        .filter(|(_, d)| *d <= PRELOAD_DEPTH)
        .map(|(c, _)| c)
        .chain(wanted.iter().copied())
        .map(str::to_owned)
        .collect();

    CellPlan {
        local: owned(&local),
        wanted: owned(&wanted),
        shown: owned(&shown),
        seen: owned(&seen),
        preload,
        berth: berth.map(str::to_owned),
    }
}

/// Des identifiants empruntés au secteur, copiés pour le plan.
fn owned<'a, 'b: 'a, C: FromIterator<String>>(cells: impl IntoIterator<Item = &'a &'b str>) -> C {
    cells.into_iter().map(|c| (*c).to_owned()).collect()
}

/// Les portes (portails de genre `Door`) à moins de [`APPROACH_DISTANCE`] du joueur, à
/// l'horizontale et au même niveau (`p` : ses pieds, repère du secteur).
pub fn near_doors(sector: &Sector, p: Vec3) -> BTreeSet<String> {
    sector
        .portals
        .iter()
        .filter(|portal| portal.kind == PortalKind::Door)
        .filter(|portal| {
            let c = v3(portal.center);
            Vec2::new(c.x - p.x, c.z - p.z).length() <= APPROACH_DISTANCE
                && (c.y - p.y).abs() <= APPROACH_HEIGHT
        })
        .map(|portal| portal.id.clone())
        .collect()
}

/// Les blocs d'ouvertures : pour chaque cellule, celles qu'on voit avec elle portes fermées.
fn opening_groups(sector: &Sector) -> BTreeMap<String, Vec<String>> {
    sector
        .cells
        .iter()
        .map(|c| {
            let group = sector.reached_from(&[c.id.as_str()], |_| false);
            (c.id.clone(), group.into_iter().map(str::to_owned).collect())
        })
        .collect()
}

/// Faut-il changer de cellule courante ? `best` : la plus petite cellule qui contient le point
/// (`Sector::cell_at`). On garde la courante tant qu'on est dans sa marge (hystérésis), sauf pour
/// entrer dans une cellule emboîtée en elle (le sas d'un vaisseau posé dans le hangar).
fn keeps_current(
    current: Option<&sh_data::sector::Cell>,
    best: Option<&sh_data::sector::Cell>,
    p: (f32, f32, f32),
) -> bool {
    let Some(current) = current else {
        return false;
    };
    match best {
        Some(best) if best.id == current.id => true,
        Some(best) if current.encloses(best) => false,
        _ => current.contains_with_margin(p, EXIT_MARGIN),
    }
}

fn track_current_cell(
    sector: Res<ActiveSector>,
    player: Query<&GlobalTransform, With<Player>>,
    mut states: ResMut<CellStates>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    let p = player.translation();
    let p = (p.x, p.y + 0.5, p.z);
    let s = &sector.sector;
    let best = s.cell_at(p);
    let current = states.current.as_deref().and_then(|id| s.cell(id));
    if keeps_current(current, best, p) {
        return;
    }
    if let Some(cell) = best
        && states.current.as_deref() != Some(&cell.id)
    {
        debug!("cellule courante : {}", cell.id);
        states.current = Some(cell.id.clone());
        states.transitions += 1;
    }
}

fn stream_cells(
    mut commands: Commands,
    time: Res<Time>,
    sector: Res<ActiveSector>,
    asset_server: Res<AssetServer>,
    mut states: ResMut<CellStates>,
    ready: Query<(), With<CellReady>>,
    settings: Option<Res<GraphicsSettings>>,
    player: Query<&GlobalTransform, With<Player>>,
    doors: Query<&Door>,
    hulls: Query<Entity, With<ShipHull>>,
) {
    let baked = settings.is_some_and(|s| s.preset.baked_lighting);
    let dt = time.delta_secs();
    let states = &mut *states;
    states.waited += dt;
    let Some(current) = states.current.clone() else {
        return;
    };
    let s = &sector.sector;
    if states.groups.is_empty() {
        states.groups = opening_groups(s);
    }
    let near = player
        .single()
        .ok()
        .map(|t| near_doors(s, t.translation()))
        .unwrap_or_default();
    let doors = DoorView::read(&doors, near);
    let loaded: BTreeSet<String> = states
        .slots
        .iter()
        .filter(|(_, slot)| slot.full.is_some())
        .map(|(id, _)| id.clone())
        .collect();
    let plan = plan_cells(s, &current, &sector.ship_cells, &doors, &loaded);
    if states.local != plan.local {
        states.local.clone_from(&plan.local);
    }
    if states.shown != plan.shown {
        states.shown.clone_from(&plan.shown);
    }

    // Poignées : modèles complets préchargés ; coques simplifiées et lightmaps de tout le secteur
    // (légères, prêtes pour ce qu'une porte ouverte ou une verrière peut montrer).
    for cell in &s.cells {
        let slot = states.slots.entry(cell.id.clone()).or_default();
        if plan.preload.contains(&cell.id) && slot.full_handle.is_none() {
            slot.full_handle =
                Some(asset_server.load(GltfAssetLabel::Scene(0).from_asset(cell.model.clone())));
        }
        if slot.lod_handle.is_none()
            && let Some(lod) = &cell.lod
        {
            slot.lod_handle =
                Some(asset_server.load(GltfAssetLabel::Scene(0).from_asset(lod.clone())));
        }
        if baked
            && slot.lightmap_handle.is_none()
            && let Some(lightmap) = &cell.lightmap
        {
            slot.lightmap_handle = Some(asset_server.load(lightmap.image.clone()));
        }
    }

    // Modèles complets, par priorité : la cellule du joueur d'abord, puis ce qu'on voit.
    let mut spawns = SPAWNS_PER_FRAME;
    for id in &plan.wanted {
        if spawns == 0 {
            break;
        }
        let (Some(cell), Some(slot)) = (s.cell(id), states.slots.get_mut(id.as_str())) else {
            continue;
        };
        if slot.full.is_some() || !lightmap_loaded(&asset_server, slot) {
            continue;
        }
        let Some(handle) = slot
            .full_handle
            .clone()
            .filter(|h| asset_server.is_loaded_with_dependencies(h))
        else {
            continue;
        };
        slot.full = Some(spawn_cell(
            &mut commands,
            &sector,
            cell,
            Quality::Local,
            handle,
        ));
        slot.full_visible = false;
        slot.unwanted_for = 0.0;
        spawns -= 1;
    }

    let mut unloads = UNLOADS_PER_FRAME;
    let mut lod_spawns = LOD_SPAWNS_PER_FRAME;
    let mut reveals = REVEALS_PER_FRAME;
    for cell in &s.cells {
        let id = cell.id.as_str();
        let Some(slot) = states.slots.get_mut(id) else {
            continue;
        };
        let wanted = plan.wanted.iter().any(|w| w == id);
        let shown = plan.shown.contains(id);
        let lod_ready = slot.lod.is_some_and(|e| ready.contains(e));
        if wanted {
            slot.unwanted_for = 0.0;
        } else if slot.full.is_some() {
            slot.unwanted_for += dt;
        }

        // Le modèle complet se montre une fois prêt, là où l'on peut le voir. Une salle qu'on ne
        // voit que par une fenêtre, peut-être, attend son tour.
        let mut full_ready = slot.full.is_some_and(|e| ready.contains(e));
        let mut full_visible = full_ready && shown;
        if full_visible && !slot.full_visible && !plan.seen.contains(id) {
            if reveals > 0 {
                reveals -= 1;
            } else {
                full_visible = false;
            }
        }

        // Déchargement, après le délai de grâce ; une cellule affichée attend que sa coque
        // simplifiée soit prête à prendre sa place (préparée plus bas, cachée jusque-là).
        if !wanted
            && slot.unwanted_for >= UNLOAD_GRACE
            && unloads > 0
            && (!shown || lod_ready || slot.lod_handle.is_none())
            && let Some(entity) = slot.full.take()
        {
            commands.entity(entity).despawn();
            slot.full_visible = false;
            full_ready = false;
            full_visible = false;
            unloads -= 1;
        }

        // La coque simplifiée : là où l'on voit sans modèle complet qui reste (absent, pas prêt,
        // ou sur le départ).
        if shown && !(full_ready && wanted) {
            if slot.lod.is_none()
                && lod_spawns > 0
                && lightmap_loaded(&asset_server, slot)
                && let Some(handle) = slot
                    .lod_handle
                    .clone()
                    .filter(|h| asset_server.is_loaded_with_dependencies(h))
            {
                slot.lod = Some(spawn_cell(
                    &mut commands,
                    &sector,
                    cell,
                    Quality::Sector,
                    handle,
                ));
                slot.lod_visible = false;
                lod_spawns -= 1;
            }
        } else if let Some(entity) = slot.lod.take() {
            commands.entity(entity).despawn();
            slot.lod_visible = false;
        }
        let lod_visible = shown && !full_visible && lod_ready && slot.lod.is_some();

        set_visible(
            &mut commands,
            slot.full,
            &mut slot.full_visible,
            full_visible,
        );
        set_visible(&mut commands, slot.lod, &mut slot.lod_visible, lod_visible);
    }

    // La coque du vaisseau amarré s'affiche avec son berceau : rien n'est dessiné derrière les
    // cloisons du hall quand la porte du hangar est fermée.
    let hull_visible = plan.berth.as_deref().is_none_or(|b| plan.shown.contains(b));
    if states.hull_visible != Some(hull_visible) && !hulls.is_empty() {
        for entity in &hulls {
            commands.entity(entity).try_insert(visibility(hull_visible));
        }
        states.hull_visible = Some(hull_visible);
    }
}

/// La lightmap de la cellule est-elle là (ou n'en a-t-elle pas) ? Aucune instance n'apparaît sans.
fn lightmap_loaded(asset_server: &AssetServer, slot: &CellSlot) -> bool {
    slot.lightmap_handle
        .as_ref()
        .is_none_or(|h| asset_server.is_loaded(h))
}

/// Instancie un modèle de cellule, **caché** : il n'apparaît qu'une fois prêt (`stream_cells`).
fn spawn_cell(
    commands: &mut Commands,
    sector: &ActiveSector,
    cell: &sh_data::Cell,
    quality: Quality,
    handle: Handle<WorldAsset>,
) -> Entity {
    let label = match quality {
        Quality::Local => "Locale",
        Quality::Sector => "Sectorielle",
    };
    commands
        .spawn((
            Name::new(format!("Cellule {} ({label})", cell.id)),
            CellInstance {
                cell: cell.id.clone(),
                quality,
            },
            WorldAssetRoot(handle),
            placement_transform(cell.placement),
            Visibility::Hidden,
            ChildOf(sector.root),
        ))
        .id()
}

fn visibility(visible: bool) -> Visibility {
    if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// Applique une visibilité seulement quand elle change (`applied` : la dernière appliquée).
fn set_visible(commands: &mut Commands, entity: Option<Entity>, applied: &mut bool, visible: bool) {
    if let Some(entity) = entity
        && *applied != visible
    {
        commands.entity(entity).try_insert(visibility(visible));
        *applied = visible;
    }
}

/// Cellule qui contient un point (repère du secteur), sans hystérésis : la plus petite, quand
/// des cellules s'emboîtent (`Sector::cell_at`).
pub fn cell_at(sector: &sh_data::Sector, p: Vec3) -> Option<&str> {
    sector.cell_at((p.x, p.y, p.z)).map(|c| c.id.as_str())
}

/// Centre d'une cellule.
pub fn cell_center(sector: &sh_data::Sector, id: &str) -> Option<Vec3> {
    sector.cell(id).map(|c| v3(c.center()))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sh_data::sector::OUTSIDE;

    use super::*;
    use crate::sector::{DISTRICT_PORT, SHIP_PORT};

    fn cell(id: &str, min: (f32, f32, f32), max: (f32, f32, f32)) -> sh_data::sector::Cell {
        sh_data::sector::Cell {
            id: id.into(),
            name_key: None,
            model: String::new(),
            lod: None,
            min,
            max,
            lightmap: None,
            placement: None,
            windows: None,
        }
    }

    #[test]
    fn hysteresis_et_cellules_emboitees() {
        let hangar = cell("hangar", (-10.0, 0.0, -10.0), (10.0, 8.0, 10.0));
        let sas = cell("sas", (0.0, 0.0, -1.0), (2.0, 2.5, 1.0));
        let couloir = cell("couloir", (2.0, 0.0, -1.0), (6.0, 2.5, 1.0));
        // Du hangar, entrer dans le sas qui y est posé : tout de suite.
        assert!(!keeps_current(Some(&hangar), Some(&sas), (0.1, 1.0, 0.0)));
        // Sortir du sas vers le hangar : seulement au-delà de la marge.
        assert!(keeps_current(Some(&sas), Some(&hangar), (-0.2, 1.0, 0.0)));
        assert!(!keeps_current(Some(&sas), Some(&hangar), (-0.5, 1.0, 0.0)));
        // Deux cellules voisines (non emboîtées) : la marge, dans les deux sens.
        assert!(keeps_current(Some(&sas), Some(&couloir), (2.2, 1.0, 0.0)));
        assert!(keeps_current(Some(&couloir), Some(&sas), (1.8, 1.0, 0.0)));
        assert!(!keeps_current(Some(&couloir), Some(&sas), (1.5, 1.0, 0.0)));
        // Rien de courant : on prend la meilleure.
        assert!(!keeps_current(None, Some(&sas), (1.0, 1.0, 0.0)));
        assert!(hangar.encloses(&sas) && !sas.encloses(&hangar) && !sas.encloses(&couloir));
    }

    fn read(file: &str) -> Sector {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/data/propositions/sectors")
            .join(file);
        sh_data::read_ron::<Sector>(&path).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Le vaisseau amarré au spatioport (secteur assemblé, `sh_data::dock`).
    fn dock(ship: &Sector, port: &Sector) -> Sector {
        sh_data::dock(ship, SHIP_PORT, port, DISTRICT_PORT).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Les secteurs du jeu (vitrages non déclarés, données d'avant M11) : le vaisseau seul (en
    /// vol), et amarré au spatioport, avec ses salles.
    fn sectors() -> (Sector, Sector, Vec<String>) {
        let ship = read("corvette.ron");
        let docked = dock(&ship, &read("spaceport.ron"));
        let cells = ship.cells.iter().map(|c| c.id.clone()).collect();
        (ship, docked, cells)
    }

    fn doors(open: &[&str], opening: &[&str], near: &[&str]) -> DoorView {
        let set = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect();
        DoorView {
            open: set(open),
            opening: set(opening),
            near: set(near),
        }
    }

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    fn has(cells: &[String], id: &str) -> bool {
        cells.iter().any(|c| c == id)
    }

    #[test]
    fn le_hall_est_charge_d_un_bloc_avec_le_couloir() {
        let (_, docked, ship) = sectors();
        let closed = doors(&[], &[], &[]);
        let none = BTreeSet::new();
        // Du hangar : le hall, derrière l'ouverture du couloir, est déjà en qualité Locale.
        let plan = plan_cells(&docked, "hangar", &ship, &closed, &none);
        assert_eq!(plan.local.first().map(String::as_str), Some("hangar"));
        let mut local = plan.local.clone();
        local.sort();
        assert_eq!(local, ["chantier", "couloir", "hall", "hangar", "sas"]);
        // Portes fermées : on voit le hangar ; ses voisines directes sont affichées (la fenêtre du
        // chantier), le hall reste caché derrière la porte du couloir.
        assert_eq!(plan.seen, set(&["hangar"]));
        assert_eq!(plan.shown, set(&["chantier", "couloir", "hangar", "sas"]));
        assert_eq!(plan.berth.as_deref(), Some("hangar"));
        // La porte du couloir s'ouvre : on voit le couloir et le hall d'un coup, tous deux voulus.
        let opening = doors(
            &["couloir__hangar"],
            &["couloir__hangar"],
            &["couloir__hangar"],
        );
        let plan = plan_cells(&docked, "hangar", &ship, &opening, &none);
        assert!(plan.seen.contains("couloir") && plan.seen.contains("hall"));
        // Et la salle d'en face annonce les siennes : bureau, taverne, marché, banque.
        for room in ["bureau", "taverne", "marche", "banque"] {
            assert!(has(&plan.wanted, room), "{room} : {:?}", plan.wanted);
            assert!(!has(&plan.local, room));
        }
        // Dans le couloir : le hall est vu par l'ouverture et chargé, avec les salles qu'il dessert.
        let plan = plan_cells(&docked, "couloir", &ship, &closed, &none);
        assert!(plan.seen.contains("hall"));
        for room in ["hall", "hangar", "bureau", "taverne", "marche", "banque"] {
            assert!(has(&plan.local, room), "{room} : {:?}", plan.local);
        }
        // Dans le hall : le hangar est chargé (une porte, par le couloir) mais caché.
        let plan = plan_cells(&docked, "hall", &ship, &closed, &none);
        assert!(has(&plan.local, "hangar") && !plan.shown.contains("hangar"));
        assert_eq!(
            plan.shown,
            set(&["banque", "bureau", "couloir", "hall", "marche", "taverne"])
        );
    }

    #[test]
    fn hors_du_bord_seul_le_sas_est_charge() {
        let (_, docked, ship) = sectors();
        let none = BTreeSet::new();
        let closed = doors(&[], &[], &[]);
        let others: Vec<&String> = ship.iter().filter(|c| *c != "sas").collect();
        for current in ["hangar", "couloir", "hall", "chantier", "marche"] {
            // Même à la porte du sas, les salles du vaisseau ne sont ni voulues ni affichées.
            let near_hatch = doors(&[], &[], &["sas__hangar"]);
            for view in [&closed, &near_hatch] {
                let plan = plan_cells(&docked, current, &ship, view, &none);
                for room in &others {
                    assert!(!has(&plan.wanted, room), "{current} : {room} voulu");
                    assert!(!plan.shown.contains(*room), "{current} : {room} affiché");
                }
            }
        }
        // Le sas, voisin du hangar, est chargé depuis le hangar, pas depuis le hall.
        assert!(has(
            &plan_cells(&docked, "hangar", &ship, &closed, &none).local,
            "sas"
        ));
        let hall = plan_cells(&docked, "hall", &ship, &closed, &none);
        assert!(!has(&hall.wanted, "sas") && !hall.shown.contains("sas"));
        // Les modèles du vaisseau restent préchargés depuis le hangar (retour rapide).
        let hangar = plan_cells(&docked, "hangar", &ship, &closed, &none);
        assert!(hangar.preload.contains("couloir_arriere"));
        // Les deux portes du sas ouvertes : on voit l'intérieur, il s'affiche (pas de trou).
        let open = doors(&["sas__hangar", "couloir_arriere__sas"], &[], &[]);
        let plan = plan_cells(&docked, "hangar", &ship, &open, &none);
        assert!(plan.shown.contains("couloir_arriere") && plan.shown.contains("couloir_avant"));
    }

    /// Les vitrages que déclareront les scripts Blender : la verrière du cockpit sur l'extérieur,
    /// la fenêtre du chantier sur le hangar, aucun ailleurs.
    fn with_windows(mut sector: Sector) -> Sector {
        for cell in &mut sector.cells {
            cell.windows = Some(match cell.id.as_str() {
                "cockpit" => vec![OUTSIDE.to_owned()],
                "chantier" => vec!["hangar".to_owned()],
                _ => Vec::new(),
            });
        }
        sector
    }

    #[test]
    fn occultation_par_les_portails() {
        let ship = with_windows(read("corvette.ron"));
        let docked = dock(&ship, &with_windows(read("spaceport.ron")));
        let cells: Vec<String> = ship.cells.iter().map(|c| c.id.clone()).collect();
        let none = BTreeSet::new();
        let closed = doors(&[], &[], &[]);
        // Dans le sas amarré, portes fermées : rien d'autre ne se dessine, ni le hangar, ni la
        // coque du vaisseau (son berceau est caché) ; le hangar et les couloirs restent chargés.
        let plan = plan_cells(&docked, "sas", &cells, &closed, &none);
        assert_eq!(plan.shown, set(&["sas"]));
        assert!(
            !plan
                .shown
                .contains(plan.berth.as_deref().unwrap_or_default())
        );
        for room in ["hangar", "couloir_arriere", "couloir_avant"] {
            assert!(has(&plan.wanted, room), "{room} : {:?}", plan.wanted);
        }
        // La porte extérieure commence à s'ouvrir (attente comprise) : le hangar est visible dès
        // cette frame, avec ce qu'on en voit par la fenêtre du chantier et la verrière.
        let hatch = doors(&["sas__hangar"], &["sas__hangar"], &["sas__hangar"]);
        let plan = plan_cells(&docked, "sas", &cells, &hatch, &none);
        assert_eq!(plan.shown, set(&["chantier", "cockpit", "hangar", "sas"]));
        // Au poste de pilotage : le hangar par la verrière, la coque avec lui.
        let plan = plan_cells(&docked, "cockpit", &cells, &closed, &none);
        assert_eq!(plan.shown, set(&["chantier", "cockpit", "hangar"]));
        assert_eq!(plan.berth.as_deref(), Some("hangar"));
        // Dans le hangar : la fenêtre du chantier, le cockpit derrière la verrière (coque
        // simplifiée : il n'est pas chargé) ; le hall, chargé, reste caché.
        let plan = plan_cells(&docked, "hangar", &cells, &closed, &none);
        assert_eq!(plan.shown, set(&["chantier", "cockpit", "hangar"]));
        assert!(!has(&plan.wanted, "cockpit") && has(&plan.wanted, "hall"));
        // Dans le hall : le couloir par l'ouverture ; le hangar et les salles derrière leurs
        // portes fermées sont cachés.
        let plan = plan_cells(&docked, "hall", &cells, &closed, &none);
        assert_eq!(plan.shown, set(&["couloir", "hall"]));
        assert_eq!(plan.shown, plan.seen);
        // En vol, dans un couloir : les deux couloirs, les salles chargées derrière leurs portes.
        let plan = plan_cells(&ship, "couloir_avant", &cells, &closed, &none);
        assert_eq!(plan.shown, set(&["couloir_arriere", "couloir_avant"]));
        assert_eq!(plan.local.len(), cells.len());
        // Au cockpit en vol, rien n'entoure le vaisseau dans le secteur : seul le cockpit.
        let plan = plan_cells(&ship, "cockpit", &cells, &closed, &none);
        assert_eq!(plan.shown, set(&["cockpit"]));
        assert!(plan.berth.is_none());
    }

    #[test]
    fn a_bord_a_quai() {
        let (_, docked, ship) = sectors();
        let none = BTreeSet::new();
        let closed = doors(&[], &[], &[]);
        // Dans le sas : le hangar (voisin et contenant), le couloir arrière et l'avant d'un bloc.
        let plan = plan_cells(&docked, "sas", &ship, &closed, &none);
        let mut local = plan.local.clone();
        local.sort();
        assert_eq!(local, ["couloir_arriere", "couloir_avant", "hangar", "sas"]);
        assert!(plan.seen.contains("hangar"));
        assert!(!has(&plan.wanted, "quartiers") && !plan.shown.contains("quartiers"));
        // À la porte intérieure du sas : tout le vaisseau s'annonce.
        let near = doors(&[], &[], &["couloir_arriere__sas"]);
        let plan = plan_cells(&docked, "sas", &ship, &near, &none);
        for room in &ship {
            assert!(has(&plan.wanted, room), "{room} : {:?}", plan.wanted);
        }
        // Au poste de pilotage : le hangar se voit par la verrière, en coque simplifiée, avec la
        // coque du vaisseau ; il n'est pas chargé.
        let plan = plan_cells(&docked, "cockpit", &ship, &closed, &none);
        assert!(plan.seen.contains("hangar") && !has(&plan.wanted, "hangar"));
        assert_eq!(plan.berth.as_deref(), Some("hangar"));
        let mut local = plan.local.clone();
        local.sort();
        assert_eq!(local, ["cockpit", "couloir_arriere", "couloir_avant"]);
    }

    #[test]
    fn en_vol() {
        let (ship_sector, _, ship) = sectors();
        let none = BTreeSet::new();
        let closed = doors(&[], &[], &[]);
        // Dans un couloir : les deux couloirs d'un bloc, et toutes les salles à une porte d'eux.
        let plan = plan_cells(&ship_sector, "couloir_avant", &ship, &closed, &none);
        assert_eq!(plan.local.len(), ship.len());
        assert_eq!(
            plan.shown,
            set(&[
                "carre",
                "chambre",
                "cockpit",
                "couloir_arriere",
                "couloir_avant",
                "quartiers"
            ])
        );
        assert_eq!(plan.seen, set(&["couloir_arriere", "couloir_avant"]));
        assert!(plan.berth.is_none());
        // Dans la chambre : sa porte mène au couloir avant, l'arrière vient avec lui.
        let plan = plan_cells(&ship_sector, "chambre", &ship, &closed, &none);
        let mut local = plan.local.clone();
        local.sort();
        assert_eq!(local, ["chambre", "couloir_arriere", "couloir_avant"]);
        assert_eq!(plan.local.first().map(String::as_str), Some("chambre"));
    }

    #[test]
    fn ce_qu_on_voit_ne_se_decharge_pas() {
        let (_, docked, ship) = sectors();
        // Du hangar, portes du couloir et du bureau ouvertes : le bureau se voit au fond du hall.
        let open = doors(&["couloir__hangar", "hall__bureau"], &[], &[]);
        let plan = plan_cells(&docked, "hangar", &ship, &open, &BTreeSet::new());
        assert!(plan.seen.contains("bureau") && !has(&plan.wanted, "bureau"));
        // Déjà là, il le reste tant qu'on le voit (sinon : sa coque simplifiée).
        let plan = plan_cells(&docked, "hangar", &ship, &open, &set(&["bureau"]));
        assert!(has(&plan.wanted, "bureau"));
        // Une porte qui s'ouvre loin du joueur : ses deux côtés sont voulus (elle n'attend pas
        // pour toujours une cellule que rien ne charge).
        let far = doors(&[], &["couloir_arriere__soute"], &[]);
        let plan = plan_cells(&docked, "hall", &ship, &far, &BTreeSet::new());
        assert!(has(&plan.wanted, "couloir_arriere") && has(&plan.wanted, "soute"));
    }

    #[test]
    fn portes_proches_et_blocs_d_ouvertures() {
        let (_, docked, _) = sectors();
        // Dans le couloir, contre la porte du hangar : l'ouverture du hall n'est pas une porte.
        let near = near_doors(&docked, Vec3::new(-5.0, 0.0, 0.0));
        assert_eq!(near, set(&["couloir__hangar"]));
        assert!(near_doors(&docked, Vec3::new(-8.0, 0.0, 0.0)).is_empty());
        let groups = opening_groups(&docked);
        let group = |id: &str| groups.get(id).cloned().unwrap_or_default();
        assert_eq!(group("couloir"), ["couloir", "hall"]);
        assert_eq!(group("hall"), ["couloir", "hall"]);
        assert_eq!(group("hangar"), ["hangar"]);
        assert_eq!(group("couloir_avant"), ["couloir_arriere", "couloir_avant"]);
    }
}
