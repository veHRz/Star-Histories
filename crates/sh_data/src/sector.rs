//! Secteurs jouables (AGENTS.md §6.2) : un secteur est un graphe de **cellules** (salles,
//! couloirs, îlots de rues) reliées par des **portails** (portes, sas, ouvertures).
//!
//! Les descripteurs (`assets/data/**/sectors/*.ron`) sont générés par les scripts Blender qui
//! produisent la géométrie (`tools/blender/`) : la géométrie et sa description ne divergent
//! jamais. Coordonnées dans le repère du jeu (Y-haut, −Z = avant), en mètres, relatives à
//! l'origine du secteur.

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
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lod: Option<String>,
    pub min: (f32, f32, f32),
    pub max: (f32, f32, f32),
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lightmap: Option<CellLightmap>,
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

    /// Cellules voisines (reliées par un portail).
    pub fn neighbors<'a>(&'a self, cell: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.portals.iter().filter_map(move |p| p.other(cell))
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
            for side in [&p.a, &p.b] {
                if self.cell(side).is_none() {
                    out.push(format!("portail {} : cellule inconnue {side}", p.id));
                }
            }
            let n = p.normal;
            let len = (n.0 * n.0 + n.1 * n.1 + n.2 * n.2).sqrt();
            if (len - 1.0).abs() > 0.01 {
                out.push(format!("portail {} : normale non unitaire", p.id));
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
