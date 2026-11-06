//! Secteurs jouables (AGENTS.md §6.2) : un secteur est un graphe de **cellules** (salles,
//! couloirs, îlots de rues) reliées par des **portails** (portes, sas, ouvertures).
//!
//! Les descripteurs (`assets/data/**/sectors/*.ron`) sont générés par les scripts Blender qui
//! produisent la géométrie (`tools/blender/`) : la géométrie et sa description ne divergent
//! jamais. Coordonnées dans le repère du jeu (Y-haut, −Z = avant), en mètres, relatives à
//! l'origine du secteur.
//!
//! **Portails externes** : un côté de portail dont le nom commence par `@` (`@dock`, `@ship`)
//! désigne un point d'attache vers un autre secteur. Le sas du vaisseau du joueur a le sien,
//! verrouillé tant que le vaisseau vole ; [`dock`] assemble à l'amarrage le vaisseau et le
//! quartier qui l'accueille en un seul secteur.
//!
//! **Cellules imbriquées** (M9) : à quai, le vaisseau repose dans le hangar, et les boîtes de
//! ses cellules sont à l'intérieur de celle du hangar. Une position appartient à la cellule la
//! plus petite qui la contient ([`Sector::cell_at`]) : dans le sas, on est dans le sas, pas dans
//! le hangar qui l'entoure ; le hangar est son **contenant** ([`Sector::containers`]).
//!
//! **Profondeur de streaming** (M11) : la distance d'une cellule à une autre, pour le streaming,
//! est le nombre de **portes** à franchir ([`Sector::door_depths`]) ; une ouverture ne compte
//! pas, les cellules qu'elle relie se voient d'un bloc. [`Sector::seen_from`] suit les ouvertures,
//! les portes ouvertes et les **vitrages** déclarés ([`Cell::windows`] : la fenêtre du chantier
//! sur le hangar, la verrière du cockpit sur l'extérieur) : ce qu'on peut voir d'un endroit. Le
//! jeu en tire ce qu'il charge et ce qu'il affiche (`sh_game::sector::streaming`).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use sh_core::Id;

use crate::common::Status;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sector {
    pub id: Id<Sector>,
    pub kind: SectorKind,
    /// Clé de langue du nom affiché.
    pub name_key: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Modèle des panneaux de porte posés à chaque portail de genre `Door`.
    pub door_model: String,
    pub cells: Vec<Cell>,
    pub portals: Vec<Portal>,
    pub spawns: Vec<Spawn>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectorKind {
    /// Un petit vaisseau entier, ou la section d'un grand.
    Ship,
    /// Une section de station.
    Station,
    /// Un quartier d'une planète.
    District,
}

/// Une cellule : son modèle complet (qualité Locale), son modèle simplifié (qualité
/// Sectorielle), sa boîte englobante (cloisons comprises) et son éclairage précalculé.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub id: String,
    /// Clé de langue du nom de la salle, selon son usage (`room.cockpit`, fichier
    /// `lang/<code>/rooms.ron`), affiché dans le bandeau ; la validation exige qu'elle existe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_key: Option<String>,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lod: Option<String>,
    pub min: (f32, f32, f32),
    pub max: (f32, f32, f32),
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lightmap: Option<CellLightmap>,
    /// Placement du modèle dans le secteur (cellules d'un secteur amarré, [`dock`]) ; les
    /// boîtes sont déjà exprimées dans le repère du secteur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<Placement>,
    /// Ce qu'on voit par les vitrages de la salle (M11) : des cellules du secteur (la fenêtre du
    /// chantier sur le hangar) et [`OUTSIDE`] pour l'extérieur (la verrière du cockpit). Une
    /// fenêtre se voit des deux côtés ([`Sector::window_neighbors`]). `None` : non déclaré (données
    /// d'avant M11) ; le streaming suppose alors qu'une voisine directe peut se voir par une
    /// fenêtre et que l'extérieur se voit. `Some([])` : aucun vitrage, les portes fermées
    /// occultent tout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<Vec<String>>,
}

/// Ce que montre une fenêtre sur l'extérieur ([`Cell::windows`]) : ce qui entoure le vaisseau ou le
/// bâtiment (le hangar, à quai) ; et, vue de dehors, la salle derrière elle.
pub const OUTSIDE: &str = "@outside";

/// Translation et rotation (quarts de tour autour de la verticale, sens direct vu d'en haut).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub translation: (f32, f32, f32),
    pub quarter_turns: u8,
}

impl Placement {
    /// Rotation seule d'un vecteur.
    pub fn rotate(&self, v: (f32, f32, f32)) -> (f32, f32, f32) {
        let (x, y, z) = v;
        match self.quarter_turns % 4 {
            0 => (x, y, z),
            1 => (z, y, -x),
            2 => (-x, y, -z),
            _ => (-z, y, x),
        }
    }

    /// Point transformé.
    pub fn apply(&self, p: (f32, f32, f32)) -> (f32, f32, f32) {
        let (x, y, z) = self.rotate(p);
        let t = self.translation;
        (x + t.0, y + t.1, z + t.2)
    }
}

/// Un côté de portail hors du secteur (point d'attache vers un autre secteur).
pub fn is_external(cell: &str) -> bool {
    cell.starts_with('@')
}

/// Éclairage diffus précalculé d'une cellule (`tools/blender/lightmap.py`), commun à ses deux
/// modèles : ils partagent le même 2e jeu d'UV.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CellLightmap {
    /// Image PNG sRGB, normalisée.
    pub image: String,
    /// Facteur qui rend à l'image sa luminance en nits (`StandardMaterial::lightmap_exposure`).
    pub exposure: f32,
}

impl Cell {
    pub fn contains(&self, p: (f32, f32, f32)) -> bool {
        self.contains_with_margin(p, 0.0)
    }

    /// Point dans la boîte agrandie de `margin` (négative : boîte réduite).
    pub fn contains_with_margin(&self, p: (f32, f32, f32), margin: f32) -> bool {
        (self.min.0 - margin..=self.max.0 + margin).contains(&p.0)
            && (self.min.1 - margin..=self.max.1 + margin).contains(&p.1)
            && (self.min.2 - margin..=self.max.2 + margin).contains(&p.2)
    }

    /// Volume de la boîte (m³) : départage des cellules imbriquées.
    pub fn volume(&self) -> f32 {
        (self.max.0 - self.min.0) * (self.max.1 - self.min.1) * (self.max.2 - self.min.2)
    }

    /// La boîte de `inner` est-elle dans celle-ci, et plus petite ? Le hangar autour des salles du
    /// vaisseau qui y est amarré.
    pub fn encloses(&self, inner: &Cell) -> bool {
        let inside = |p: (f32, f32, f32)| {
            p.0 >= self.min.0
                && p.1 >= self.min.1
                && p.2 >= self.min.2
                && p.0 <= self.max.0
                && p.1 <= self.max.1
                && p.2 <= self.max.2
        };
        inside(inner.min) && inside(inner.max) && inner.volume() < self.volume()
    }

    /// La salle a-t-elle un vitrage sur l'extérieur ([`OUTSIDE`]) ? `None` : non déclaré.
    pub fn sees_outside(&self) -> Option<bool> {
        self.windows
            .as_ref()
            .map(|w| w.iter().any(|c| c == OUTSIDE))
    }

    pub fn center(&self) -> (f32, f32, f32) {
        (
            (self.min.0 + self.max.0) / 2.0,
            (self.min.1 + self.max.1) / 2.0,
            (self.min.2 + self.max.2) / 2.0,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortalKind {
    /// Porte coulissante : se ferme, occulte, sert de porte de chargement.
    Door,
    /// Ouverture permanente (couloir continu, arche).
    Opening,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Portal {
    pub id: String,
    pub a: String,
    pub b: String,
    pub kind: PortalKind,
    /// Centre de l'ouverture.
    pub center: (f32, f32, f32),
    /// Normale du plan de l'ouverture, de `a` vers `b`.
    pub normal: (f32, f32, f32),
    pub width: f32,
    pub height: f32,
    /// Modèle de la porte de ce portail (genre `Door`), s'il diffère de celui du secteur
    /// (`Sector::door_model`) : la porte blindée du sas, ou les portes d'un vaisseau amarré dans
    /// un quartier qui a les siennes ([`dock`]). `None` : le modèle du secteur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door_model: Option<String>,
}

impl Portal {
    /// La cellule de l'autre côté, si `cell` est l'un des deux côtés.
    pub fn other(&self, cell: &str) -> Option<&str> {
        if self.a == cell {
            Some(&self.b)
        } else if self.b == cell {
            Some(&self.a)
        } else {
            None
        }
    }
}

/// Point d'apparition du joueur ou d'un PNJ.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spawn {
    pub id: String,
    pub cell: String,
    pub pos: (f32, f32, f32),
    /// Cap en degrés autour de l'axe vertical : 0 = vers l'avant (−Z), positif vers la gauche.
    pub heading_deg: f32,
}

impl Sector {
    pub fn cell(&self, id: &str) -> Option<&Cell> {
        self.cells.iter().find(|c| c.id == id)
    }

    /// Le modèle de la porte d'un portail : le sien s'il en a un, sinon celui du secteur.
    pub fn door_model_of<'a>(&'a self, portal: &'a Portal) -> &'a str {
        portal.door_model.as_deref().unwrap_or(&self.door_model)
    }

    /// La cellule qui contient ce point : la plus petite, quand des boîtes s'emboîtent (le
    /// vaisseau amarré dans son hangar) ; à égalité, la première.
    pub fn cell_at(&self, p: (f32, f32, f32)) -> Option<&Cell> {
        self.cells
            .iter()
            .filter(|c| c.contains(p))
            .min_by(|a, b| a.volume().total_cmp(&b.volume()))
    }

    /// Cellules voisines (reliées par un portail), points d'attache externes exclus.
    pub fn neighbors<'a>(&'a self, cell: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.portals
            .iter()
            .filter_map(move |p| p.other(cell))
            .filter(|c| !is_external(c))
    }

    /// Le portail externe de ce nom (`@dock`), orienté de la cellule vers l'extérieur.
    pub fn external_port(&self, name: &str) -> Option<Portal> {
        self.portals.iter().find_map(|p| {
            if p.b == name {
                Some(p.clone())
            } else if p.a == name {
                let n = p.normal;
                Some(Portal {
                    a: p.b.clone(),
                    b: p.a.clone(),
                    normal: (-n.0, -n.1, -n.2),
                    ..p.clone()
                })
            } else {
                None
            }
        })
    }

    /// Cellules à `depth` portails au plus de `cell` (elle comprise), triées par identifiant.
    pub fn within(&self, cell: &str, depth: usize) -> Vec<String> {
        let mut found = vec![cell.to_owned()];
        let mut frontier = vec![cell.to_owned()];
        for _ in 0..depth {
            let mut next = Vec::new();
            for c in &frontier {
                for n in self.neighbors(c) {
                    if !found.iter().any(|f| f == n) {
                        found.push(n.to_owned());
                        next.push(n.to_owned());
                    }
                }
            }
            frontier = next;
        }
        found.sort();
        found
    }

    /// Profondeur de streaming des cellules atteignables depuis `from` (profondeur 0) : le nombre
    /// de **portes** à franchir pour y aller. Une ouverture (`PortalKind::Opening`) ne compte pas :
    /// les cellules qu'elle relie se voient l'une de l'autre, elles se chargent d'un bloc (le
    /// couloir du spatioport et le hall). `pass` écarte des portails, comme s'ils n'existaient pas
    /// (les portes intérieures d'un vaisseau amarré, vu du quai) ; un point d'attache externe ne
    /// mène nulle part. Les identifiants de `from` qui ne sont pas des cellules sont ignorés.
    pub fn door_depths<'a>(
        &'a self,
        from: &[&str],
        pass: impl Fn(&Portal) -> bool,
    ) -> BTreeMap<&'a str, usize> {
        let mut depths: BTreeMap<&'a str, usize> = BTreeMap::new();
        let mut queue = VecDeque::new();
        for id in from {
            if let Some(cell) = self.cell(id)
                && depths.insert(cell.id.as_str(), 0).is_none()
            {
                queue.push_back(cell.id.as_str());
            }
        }
        // Parcours « 0-1 » : un pas gratuit (ouverture) passe en tête de file, une porte en queue ;
        // une cellule revue plus près est reprise.
        while let Some(cell) = queue.pop_front() {
            let Some(&depth) = depths.get(cell) else {
                continue;
            };
            for portal in self.portals.iter().filter(|p| pass(p)) {
                let Some(next) = portal.other(cell) else {
                    continue;
                };
                if is_external(next) {
                    continue;
                }
                let step = usize::from(portal.kind == PortalKind::Door);
                let reached = depth + step;
                if depths.get(next).is_none_or(|&d| reached < d) {
                    depths.insert(next, reached);
                    if step == 0 {
                        queue.push_front(next);
                    } else {
                        queue.push_back(next);
                    }
                }
            }
        }
        depths
    }

    /// Les cellules où l'on passe depuis `from` (elles comprises) sans franchir de porte fermée :
    /// par les ouvertures et les portes que `open` dit ouvertes.
    pub fn reached_from<'a>(
        &'a self,
        from: &[&str],
        open: impl Fn(&Portal) -> bool,
    ) -> BTreeSet<&'a str> {
        self.door_depths(from, |p| p.kind == PortalKind::Opening || open(p))
            .into_keys()
            .collect()
    }

    /// Les cellules qu'on voit depuis `from` (elles comprises), de proche en proche : par les
    /// ouvertures, les portes que `open` dit ouvertes et les vitrages déclarés
    /// ([`Sector::window_neighbors`]) ; par un vitrage sur l'extérieur ([`OUTSIDE`]), la cellule
    /// qui entoure (le hangar vu de la verrière du vaisseau amarré) et, en sens inverse, la salle
    /// derrière la verrière, vue de dehors. Une fenêtre se voit des deux côtés, même si un seul
    /// d'entre eux la déclare ; une salle sans aucun vitrage déclaré, d'un côté ni de l'autre, ne
    /// voit et ne se voit que par ses portails.
    pub fn seen_from<'a>(
        &'a self,
        from: &[&str],
        open: impl Fn(&Portal) -> bool,
    ) -> BTreeSet<&'a str> {
        let mut seen: BTreeSet<&'a str> = BTreeSet::new();
        let mut queue: Vec<&'a str> = from
            .iter()
            .filter_map(|id| self.cell(id))
            .map(|c| c.id.as_str())
            .collect();
        while let Some(id) = queue.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(cell) = self.cell(id) else {
                continue;
            };
            for portal in &self.portals {
                if (portal.kind == PortalKind::Opening || open(portal))
                    && let Some(next) = portal.other(id)
                    && self.cell(next).is_some()
                {
                    queue.push(next);
                }
            }
            queue.extend(self.window_neighbors(id));
            // Par une verrière : ce qui entoure ; vue de dehors : ce qui est derrière une verrière.
            if cell.sees_outside() == Some(true) {
                queue.extend(self.containers(id).into_iter().map(|c| c.id.as_str()));
            }
            queue.extend(
                self.cells
                    .iter()
                    .filter(|inner| inner.sees_outside() == Some(true) && cell.encloses(inner))
                    .map(|inner| inner.id.as_str()),
            );
        }
        seen
    }

    /// Les cellules qu'on voit par les vitrages de `cell` : celles qu'elle déclare, et celles qui
    /// la déclarent (une fenêtre se voit des deux côtés, même déclarée d'un seul : le chantier
    /// déclare sa fenêtre, le hangar la voit aussi), triées, sans doublon.
    pub fn window_neighbors(&self, cell: &str) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .cell(cell)
            .and_then(|c| c.windows.as_ref())
            .into_iter()
            .flatten()
            .map(String::as_str)
            .filter(|c| *c != OUTSIDE && *c != cell && self.cell(c).is_some())
            .collect();
        for other in &self.cells {
            let sees_us = other
                .windows
                .as_ref()
                .is_some_and(|w| w.iter().any(|c| c == cell));
            if sees_us && other.id != cell {
                out.push(&other.id);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Les cellules qui entourent `cell` (sa boîte est dans la leur : le hangar autour des salles
    /// du vaisseau amarré), de la plus petite à la plus grande.
    pub fn containers(&self, cell: &str) -> Vec<&Cell> {
        let Some(inner) = self.cell(cell) else {
            return Vec::new();
        };
        let mut out: Vec<&Cell> = self.cells.iter().filter(|c| c.encloses(inner)).collect();
        out.sort_by(|a, b| a.volume().total_cmp(&b.volume()));
        out
    }

    /// Erreurs de cohérence internes du secteur.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut ids: Vec<&str> = self.cells.iter().map(|c| c.id.as_str()).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            out.push("cellule en double".to_owned());
        }
        for cell in &self.cells {
            if !sh_core::id::is_valid_id(&cell.id) {
                out.push(format!("identifiant de cellule invalide : {}", cell.id));
            }
            let (lo, hi) = (cell.min, cell.max);
            if !(lo.0 < hi.0 && lo.1 < hi.1 && lo.2 < hi.2) {
                out.push(format!("cellule {} : boîte vide ou inversée", cell.id));
            }
            if let Some(lightmap) = &cell.lightmap
                && !(lightmap.exposure.is_finite() && lightmap.exposure > 0.0)
            {
                out.push(format!(
                    "cellule {} : exposition de lightmap invalide",
                    cell.id
                ));
            }
            // Un vitrage donne sur l'extérieur ou sur une autre cellule du secteur.
            for seen in cell.windows.iter().flatten() {
                if seen != OUTSIDE && (*seen == cell.id || self.cell(seen).is_none()) {
                    out.push(format!("cellule {} : vitrage sur {seen} inconnu", cell.id));
                }
            }
        }
        for p in &self.portals {
            for side in [&p.a, &p.b].into_iter().filter(|s| !is_external(s)) {
                if self.cell(side).is_none() {
                    out.push(format!("portail {} : cellule inconnue {side}", p.id));
                }
            }
            let n = p.normal;
            let len = (n.0 * n.0 + n.1 * n.1 + n.2 * n.2).sqrt();
            if (len - 1.0).abs() > 0.01 {
                out.push(format!("portail {} : normale non unitaire", p.id));
            }
            if is_external(&p.a) && is_external(&p.b) {
                out.push(format!("portail {} : deux côtés externes", p.id));
            }
            // Le centre d'un portail est à la frontière de ses deux cellules.
            for side in [&p.a, &p.b] {
                if let Some(c) = self.cell(side)
                    && !c.contains_with_margin(p.center, 0.05)
                {
                    out.push(format!(
                        "portail {} : centre hors de la cellule {side}",
                        p.id
                    ));
                }
            }
        }
        for s in &self.spawns {
            match self.cell(&s.cell) {
                Some(c) if c.contains(s.pos) => {}
                Some(_) => out.push(format!("apparition {} hors de sa cellule", s.id)),
                None => out.push(format!("apparition {} : cellule inconnue {}", s.id, s.cell)),
            }
        }
        // Toutes les cellules sont atteignables depuis la première.
        if let Some(first) = self.cells.first() {
            let reachable = self.within(&first.id, self.cells.len());
            for c in &self.cells {
                if !reachable.contains(&c.id) {
                    out.push(format!("cellule {} inatteignable", c.id));
                }
            }
        }
        out
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DockError {
    #[error("{0} : pas de point d'attache {1}")]
    NoPort(String, String),
    #[error("cellule {0} présente dans les deux secteurs")]
    DuplicateCell(String),
}

/// Assemble un vaisseau amarré et le quartier qui l'accueille en un seul secteur.
///
/// Le point d'attache `ship_port` du vaisseau (son sas, vers l'extérieur) est posé sur le point
/// d'attache `district_port` du quartier (sa passerelle, vers le vaisseau) : le vaisseau est
/// tourné d'un nombre entier de quarts de tour (ses boîtes restent alignées sur les axes) puis
/// translaté. Les deux points d'attache deviennent un seul portail, la porte du sas.
///
/// Le secteur obtenu garde le modèle de porte du quartier : les portes du vaisseau, et celle du
/// sas, reçoivent donc chacune le modèle qu'elles avaient dans le vaisseau (le leur, ou à défaut
/// celui du vaisseau), sans quoi elles prendraient celui du quartier (M10 : portes du vaisseau
/// jaunes au spatioport).
pub fn dock(
    ship: &Sector,
    ship_port: &str,
    district: &Sector,
    district_port: &str,
) -> Result<Sector, DockError> {
    let ps = ship
        .external_port(ship_port)
        .ok_or_else(|| DockError::NoPort(ship.id.to_string(), ship_port.to_owned()))?;
    let pd = district
        .external_port(district_port)
        .ok_or_else(|| DockError::NoPort(district.id.to_string(), district_port.to_owned()))?;
    // Quart de tour qui retourne la normale du sas contre celle de la passerelle.
    let turns = (0..4u8)
        .min_by(|&a, &b| {
            let gap = |k: u8| {
                let n = Placement {
                    translation: (0.0, 0.0, 0.0),
                    quarter_turns: k,
                }
                .rotate(ps.normal);
                let d = (n.0 + pd.normal.0, n.1 + pd.normal.1, n.2 + pd.normal.2);
                d.0 * d.0 + d.1 * d.1 + d.2 * d.2
            };
            gap(a).total_cmp(&gap(b))
        })
        .unwrap_or(0);
    let rotation = Placement {
        translation: (0.0, 0.0, 0.0),
        quarter_turns: turns,
    };
    let c = rotation.rotate(ps.center);
    let placement = Placement {
        translation: (pd.center.0 - c.0, pd.center.1 - c.1, pd.center.2 - c.2),
        quarter_turns: turns,
    };

    let mut out = district.clone();
    out.portals
        .retain(|p| !is_external(&p.a) && !is_external(&p.b));
    for cell in &ship.cells {
        if out.cell(&cell.id).is_some() {
            return Err(DockError::DuplicateCell(cell.id.clone()));
        }
        let (a, b) = (placement.apply(cell.min), placement.apply(cell.max));
        out.cells.push(Cell {
            min: (a.0.min(b.0), a.1.min(b.1), a.2.min(b.2)),
            max: (a.0.max(b.0), a.1.max(b.1), a.2.max(b.2)),
            placement: Some(placement),
            ..cell.clone()
        });
    }
    for p in ship
        .portals
        .iter()
        .filter(|p| !is_external(&p.a) && !is_external(&p.b))
    {
        out.portals.push(Portal {
            center: placement.apply(p.center),
            normal: placement.rotate(p.normal),
            door_model: Some(ship.door_model_of(p).to_owned()),
            ..p.clone()
        });
    }
    // La porte du sas est celle du vaisseau (sa porte extérieure), pas celle de la passerelle.
    out.portals.push(Portal {
        id: format!("{}__{}", ps.a, pd.a),
        a: ps.a.clone(),
        b: pd.a.clone(),
        kind: ps.kind,
        center: pd.center,
        normal: placement.rotate(ps.normal),
        width: ps.width,
        height: ps.height,
        door_model: Some(ship.door_model_of(&ps).to_owned()),
    });
    for spawn in &ship.spawns {
        out.spawns.push(Spawn {
            pos: placement.apply(spawn.pos),
            heading_deg: spawn.heading_deg + 90.0 * f32::from(turns),
            ..spawn.clone()
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(id: &str, min: (f32, f32, f32), max: (f32, f32, f32)) -> Cell {
        Cell {
            id: id.into(),
            name_key: None,
            model: format!("{id}.glb"),
            lod: None,
            min,
            max,
            lightmap: None,
            placement: None,
            windows: None,
        }
    }

    fn portal(a: &str, b: &str, center: (f32, f32, f32), normal: (f32, f32, f32)) -> Portal {
        Portal {
            id: format!("{a}__{b}"),
            a: a.into(),
            b: b.into(),
            kind: PortalKind::Door,
            center,
            normal,
            width: 1.4,
            height: 2.2,
            door_model: None,
        }
    }

    fn sector(id: &str, cells: Vec<Cell>, portals: Vec<Portal>) -> Sector {
        Sector {
            id: Id::new(id).unwrap_or_else(|e| panic!("{e}")),
            kind: SectorKind::Ship,
            name_key: String::new(),
            status: Status::Canon,
            source: None,
            door_model: "door.glb".into(),
            cells,
            portals,
            spawns: vec![Spawn {
                id: "sas".into(),
                cell: "sas".into(),
                pos: (0.0, 0.0, 0.0),
                heading_deg: 0.0,
            }],
        }
    }

    #[test]
    fn le_vaisseau_s_amarre_a_la_passerelle() {
        // Le sas (2 × 2 m) s'ouvre vers −X ; la passerelle s'étend vers −Z et attend le
        // vaisseau à son extrémité +Z.
        let ship = sector(
            "ship",
            vec![cell("sas", (0.0, 0.0, -1.0), (2.0, 2.5, 1.0))],
            vec![portal("sas", "@dock", (0.0, 1.1, 0.0), (-1.0, 0.0, 0.0))],
        );
        let district = sector(
            "port",
            vec![cell("passerelle", (-1.0, 0.0, -10.0), (1.0, 2.5, 0.0))],
            vec![portal(
                "passerelle",
                "@ship",
                (0.0, 1.1, 0.0),
                (0.0, 0.0, 1.0),
            )],
        );
        let docked = dock(&ship, "@dock", &district, "@ship").unwrap_or_else(|e| panic!("{e}"));
        assert!(docked.problems().is_empty(), "{:?}", docked.problems());
        // Le sas est passé derrière la passerelle (+Z), tourné de trois quarts de tour.
        let sas = docked.cell("sas").unwrap_or_else(|| panic!("sas absent"));
        assert_eq!(sas.placement.map(|p| p.quarter_turns), Some(3));
        assert!(sas.min.2 >= -1e-4 && sas.max.2 <= 2.0 + 1e-4, "{sas:?}");
        let door = docked
            .portals
            .iter()
            .find(|p| p.id == "sas__passerelle")
            .unwrap_or_else(|| panic!("porte du sas absente"));
        assert_eq!(door.normal, (0.0, 0.0, -1.0));
        assert_eq!(docked.within("passerelle", 1), vec!["passerelle", "sas"]);
    }

    #[test]
    fn a_quai_les_portes_du_vaisseau_gardent_leur_modele() {
        // Un vaisseau à deux salles (porte intérieure, sas blindé) et un quartier à deux salles,
        // chacun avec son modèle de porte.
        let mut hatch = portal("sas", "@dock", (0.0, 1.1, 0.0), (-1.0, 0.0, 0.0));
        hatch.door_model = Some("airlock.glb".into());
        let mut ship = sector(
            "ship",
            vec![
                cell("sas", (0.0, 0.0, -1.0), (2.0, 2.5, 1.0)),
                cell("couloir", (2.0, 0.0, -1.0), (6.0, 2.5, 1.0)),
            ],
            vec![
                portal("sas", "couloir", (2.0, 1.1, 0.0), (1.0, 0.0, 0.0)),
                hatch,
            ],
        );
        ship.door_model = "ship_door.glb".into();
        let mut district = sector(
            "port",
            vec![
                cell("hall", (-1.0, 0.0, -20.0), (1.0, 2.5, -10.0)),
                cell("passerelle", (-1.0, 0.0, -10.0), (1.0, 2.5, 0.0)),
            ],
            vec![
                portal("hall", "passerelle", (0.0, 1.1, -10.0), (0.0, 0.0, 1.0)),
                portal("passerelle", "@ship", (0.0, 1.1, 0.0), (0.0, 0.0, 1.0)),
            ],
        );
        district.door_model = "station_door.glb".into();
        let docked = dock(&ship, "@dock", &district, "@ship").unwrap_or_else(|e| panic!("{e}"));
        let model = |id: &str| {
            let p = docked
                .portals
                .iter()
                .find(|p| p.id == id)
                .unwrap_or_else(|| panic!("portail {id} absent"));
            docked.door_model_of(p).to_owned()
        };
        assert_eq!(docked.door_model, "station_door.glb");
        assert_eq!(model("hall__passerelle"), "station_door.glb");
        assert_eq!(model("sas__couloir"), "ship_door.glb");
        assert_eq!(model("sas__passerelle"), "airlock.glb");
        // Un sas sans modèle propre prend celui du vaisseau, pas celui du quartier.
        for p in &mut ship.portals {
            p.door_model = None;
        }
        let docked = dock(&ship, "@dock", &district, "@ship").unwrap_or_else(|e| panic!("{e}"));
        let sas = docked
            .portals
            .iter()
            .find(|p| p.id == "sas__passerelle")
            .unwrap_or_else(|| panic!("porte du sas absente"));
        assert_eq!(docked.door_model_of(sas), "ship_door.glb");
    }

    #[test]
    fn la_plus_petite_cellule_l_emporte() {
        // Un sas posé dans un hangar : ses boîtes sont dans celle du hangar.
        let mut port = sector(
            "port",
            vec![
                cell("hangar", (-10.0, 0.0, -10.0), (10.0, 8.0, 10.0)),
                cell("sas", (0.0, 0.0, -1.0), (2.0, 2.5, 1.0)),
            ],
            vec![portal("hangar", "sas", (0.0, 1.1, 0.0), (1.0, 0.0, 0.0))],
        );
        port.spawns.clear();
        let at = |p| port.cell_at(p).map(|c| c.id.as_str());
        assert_eq!(at((1.0, 1.0, 0.0)), Some("sas"));
        assert_eq!(at((-5.0, 1.0, 0.0)), Some("hangar"));
        assert_eq!(at((50.0, 1.0, 0.0)), None);
    }

    /// Les secteurs du jeu (générés par les scripts Blender), et le vaisseau amarré au spatioport.
    fn real(file: &str) -> Sector {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/data/propositions/sectors")
            .join(file);
        crate::read_ron(&path).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn une_ouverture_ne_compte_pas_comme_une_porte() {
        let port = real("spaceport.ron");
        let all = |_: &Portal| true;
        let depths = port.door_depths(&["hangar"], all);
        // Le hall est derrière le couloir, par une ouverture : à une porte du hangar, comme lui.
        assert_eq!(depths.get("couloir"), Some(&1));
        assert_eq!(depths.get("hall"), Some(&1));
        assert_eq!(depths.get("bureau"), Some(&2));
        // Depuis le hall, le couloir est à zéro porte, le hangar à une.
        let depths = port.door_depths(&["hall"], all);
        assert_eq!(depths.get("couloir"), Some(&0));
        assert_eq!(depths.get("hangar"), Some(&1));
        // Un portail écarté n'existe pas ; une source inconnue est ignorée.
        let depths = port.door_depths(&["hangar", "nulle_part"], |p| p.id != "couloir__hangar");
        assert!(!depths.contains_key("couloir") && !depths.contains_key("nulle_part"));
        // Le vaisseau seul : les deux couloirs d'un bloc, toutes les salles à une porte d'eux.
        let ship = real("corvette.ron");
        let depths = ship.door_depths(&["couloir_avant"], all);
        assert_eq!(depths.get("couloir_arriere"), Some(&0));
        assert!(
            ship.cells
                .iter()
                .all(|c| depths.get(c.id.as_str()).is_some_and(|&d| d <= 1))
        );
    }

    #[test]
    fn ce_qu_on_voit_suit_les_ouvertures_et_les_portes_ouvertes() {
        let port = real("spaceport.ron");
        let closed = |_: &Portal| false;
        let set = |ids: &[&'static str]| ids.iter().copied().collect::<BTreeSet<&str>>();
        assert_eq!(port.seen_from(&["hangar"], closed), set(&["hangar"]));
        assert_eq!(port.seen_from(&["hall"], closed), set(&["couloir", "hall"]));
        let open = |p: &Portal| p.id == "couloir__hangar" || p.id == "hall__bureau";
        assert_eq!(
            port.seen_from(&["hangar"], open),
            set(&["bureau", "couloir", "hall", "hangar"])
        );
    }

    #[test]
    fn le_hangar_entoure_le_vaisseau_amarre() {
        let docked = dock(
            &real("corvette.ron"),
            "@dock",
            &real("spaceport.ron"),
            "@ship",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let ids = |cells: Vec<&Cell>| cells.iter().map(|c| c.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(docked.containers("sas")), vec!["hangar"]);
        assert_eq!(ids(docked.containers("cockpit")), vec!["hangar"]);
        assert!(docked.containers("hangar").is_empty());
        assert!(docked.containers("hall").is_empty());
        let (hangar, sas) = (docked.cell("hangar"), docked.cell("sas"));
        assert!(
            hangar
                .zip(sas)
                .is_some_and(|(h, s)| h.encloses(s) && !s.encloses(h))
        );
    }

    /// Déclare les vitrages d'une cellule (ce qu'écriront les scripts Blender).
    fn declare(sector: &mut Sector, cell: &str, windows: &[&str]) {
        for c in &mut sector.cells {
            if c.id == cell {
                c.windows = Some(windows.iter().map(|w| (*w).to_owned()).collect());
            }
        }
    }

    #[test]
    fn on_voit_par_les_vitrages_declares() {
        let mut ship = real("corvette.ron");
        let mut port = real("spaceport.ron");
        declare(&mut ship, "cockpit", &[OUTSIDE]);
        declare(&mut ship, "sas", &[]);
        declare(&mut port, "chantier", &["hangar"]);
        let docked = dock(&ship, "@dock", &port, "@ship").unwrap_or_else(|e| panic!("{e}"));
        assert!(
            !docked.problems().iter().any(|p| p.contains("vitrage")),
            "{:?}",
            docked.problems()
        );
        let closed = |_: &Portal| false;
        let set = |ids: &[&'static str]| ids.iter().copied().collect::<BTreeSet<&str>>();
        // Une fenêtre se voit des deux côtés, même si le hangar ne déclare rien lui-même.
        assert_eq!(docked.window_neighbors("hangar"), vec!["chantier"]);
        assert_eq!(docked.window_neighbors("chantier"), vec!["hangar"]);
        assert!(docked.window_neighbors("hall").is_empty());
        // Du hangar : la fenêtre du chantier, et le cockpit par la verrière.
        assert_eq!(
            docked.seen_from(&["hangar"], closed),
            set(&["chantier", "cockpit", "hangar"])
        );
        // Du cockpit : le hangar par la verrière (et, par lui, le chantier).
        assert_eq!(
            docked.seen_from(&["cockpit"], closed),
            set(&["chantier", "cockpit", "hangar"])
        );
        // Du sas, portes fermées : rien d'autre ; porte extérieure ouverte : le hangar et ce
        // qu'on en voit.
        assert_eq!(docked.seen_from(&["sas"], closed), set(&["sas"]));
        assert_eq!(
            docked.seen_from(&["sas"], |p| p.id == "sas__hangar"),
            set(&["chantier", "cockpit", "hangar", "sas"])
        );
        // Un vitrage sur une cellule inconnue est une erreur.
        declare(&mut port, "hall", &["nulle_part"]);
        assert_eq!(port.problems().len(), 1, "{:?}", port.problems());
    }

    #[test]
    fn vitrages_declares_partout() {
        // Ce que déclarera le lot D : la fenêtre des deux côtés, la verrière, rien ailleurs.
        let mut ship = real("corvette.ron");
        let mut port = real("spaceport.ron");
        for c in ship.cells.iter_mut().chain(port.cells.iter_mut()) {
            c.windows = Some(match c.id.as_str() {
                "cockpit" => vec![OUTSIDE.to_owned()],
                "chantier" => vec!["hangar".to_owned()],
                "hangar" => vec!["chantier".to_owned()],
                _ => Vec::new(),
            });
        }
        let docked = dock(&ship, "@dock", &port, "@ship").unwrap_or_else(|e| panic!("{e}"));
        assert!(docked.problems().is_empty(), "{:?}", docked.problems());
        let closed = |_: &Portal| false;
        let set = |ids: &[&'static str]| ids.iter().copied().collect::<BTreeSet<&str>>();
        // Déclarée des deux côtés, la fenêtre n'est comptée qu'une fois.
        assert_eq!(docked.window_neighbors("hangar"), vec!["chantier"]);
        assert_eq!(docked.window_neighbors("chantier"), vec!["hangar"]);
        let hangar = set(&["chantier", "cockpit", "hangar"]);
        assert_eq!(docked.seen_from(&["hangar"], closed), hangar);
        assert_eq!(docked.seen_from(&["chantier"], closed), hangar);
        assert_eq!(docked.seen_from(&["cockpit"], closed), hangar);
        assert_eq!(docked.seen_from(&["sas"], closed), set(&["sas"]));
        assert_eq!(
            docked.seen_from(&["hall"], closed),
            set(&["couloir", "hall"])
        );
        assert_eq!(
            docked.seen_from(&["couloir_avant"], closed),
            set(&["couloir_arriere", "couloir_avant"])
        );
        // En vol, la verrière ne montre aucune cellule.
        assert_eq!(ship.seen_from(&["cockpit"], closed), set(&["cockpit"]));
    }

    #[test]
    fn un_portail_externe_n_est_pas_un_voisin() {
        let ship = sector(
            "ship",
            vec![cell("sas", (0.0, 0.0, -1.0), (2.0, 2.5, 1.0))],
            vec![portal("sas", "@dock", (0.0, 1.1, 0.0), (-1.0, 0.0, 0.0))],
        );
        assert!(ship.problems().is_empty(), "{:?}", ship.problems());
        assert_eq!(ship.neighbors("sas").count(), 0);
    }
}
