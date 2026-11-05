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
}

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
            ..p.clone()
        });
    }
    out.portals.push(Portal {
        id: format!("{}__{}", ps.a, pd.a),
        a: ps.a.clone(),
        b: pd.a.clone(),
        kind: ps.kind,
        center: pd.center,
        normal: placement.rotate(ps.normal),
        width: ps.width,
        height: ps.height,
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
