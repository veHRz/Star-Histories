//! Logique pure de l'écran du personnage, testée sans `World` : cases d'inventaire découpées
//! selon la pile maximale de chaque objet, progression d'une compétence vers le niveau suivant,
//! rangement entre un casier et soi (ce qu'un rangement spécialisé refuse ne bouge pas),
//! équipement porté, déplacement au clavier dans une grille.

use std::collections::BTreeMap;

use sh_data::{EquipSlot, ItemCategory, ItemsFile};

use crate::items::Containers;

/// Une case d'inventaire : un objet et sa quantité, jamais plus que sa pile maximale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stack {
    pub item: String,
    pub count: u32,
}

/// Les cases d'un ensemble d'objets (inventaire, casier) : une quantité qui dépasse la pile
/// maximale occupe plusieurs cases. Ordre stable : par catégorie, puis dans l'ordre du
/// catalogue ; un objet inconnu du catalogue (vieille sauvegarde) vient en dernier, en une case.
pub fn stacks(items: &BTreeMap<String, u32>, catalog: &ItemsFile) -> Vec<Stack> {
    let mut held: Vec<(&String, u32)> = items
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(id, count)| (id, *count))
        .collect();
    held.sort_by_key(|(id, _)| sort_key(catalog, id));
    let mut out = Vec::new();
    for (id, count) in held {
        let max = catalog.get(id).map_or(count, |def| def.max_stack.max(1));
        let mut left = count;
        while left > 0 {
            let n = left.min(max);
            out.push(Stack {
                item: id.clone(),
                count: n,
            });
            left -= n;
        }
    }
    out
}

fn sort_key(catalog: &ItemsFile, id: &str) -> (usize, usize, String) {
    match catalog
        .items
        .iter()
        .enumerate()
        .find(|(_, def)| def.id == id)
    {
        Some((index, def)) => (
            ItemCategory::ALL
                .iter()
                .position(|c| *c == def.category)
                .unwrap_or(usize::MAX),
            index,
            id.to_owned(),
        ),
        None => (usize::MAX, usize::MAX, id.to_owned()),
    }
}

/// Nombre de cases dessinées pour `filled` cases pleines : des rangées entières, au moins
/// `min_rows` (une grille vide reste une grille : on voit la place qui reste).
pub fn grid_cells(filled: usize, columns: usize, min_rows: usize) -> usize {
    let columns = columns.max(1);
    filled.div_ceil(columns).max(min_rows) * columns
}

/// Où en est une compétence : son niveau, son expérience et les seuils qui l'encadrent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Progress {
    pub level: u32,
    pub xp: f64,
    /// Expérience du niveau atteint (`xp_per_level × n²`).
    pub floor: f64,
    /// Expérience du niveau suivant (`xp_per_level × (n + 1)²`).
    pub next: f64,
}

impl Progress {
    /// Part du chemin parcourue entre les deux seuils, de 0 à 1 (la barre d'expérience).
    pub fn fraction(&self) -> f64 {
        let span = self.next - self.floor;
        if span > 0.0 {
            ((self.xp - self.floor) / span).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Expérience qui manque pour le niveau suivant.
    pub fn remaining(&self) -> f64 {
        (self.next - self.xp).max(0.0)
    }
}

/// La progression d'une compétence. Même formule que `Captain::skill_level` (P23) : le niveau
/// n demande `xp_per_level × n²` points.
pub fn progress(xp: f64, xp_per_level: f64) -> Progress {
    let xp = if xp.is_finite() { xp.max(0.0) } else { 0.0 };
    if xp_per_level.is_nan() || xp_per_level <= 0.0 {
        return Progress {
            level: 0,
            xp,
            floor: 0.0,
            next: 0.0,
        };
    }
    let level = (xp / xp_per_level).sqrt().floor() as u32;
    let n = f64::from(level);
    Progress {
        level,
        xp,
        floor: xp_per_level * n * n,
        next: xp_per_level * (n + 1.0) * (n + 1.0),
    }
}

/// Retire au plus `count` exemplaires d'un objet ; renvoie le nombre retiré. Un objet épuisé
/// disparaît de la liste.
pub fn take(items: &mut BTreeMap<String, u32>, item: &str, count: u32) -> u32 {
    let Some(held) = items.get_mut(item) else {
        return 0;
    };
    let taken = count.min(*held);
    *held -= taken;
    if *held == 0 {
        items.remove(item);
    }
    taken
}

/// Ajoute `count` exemplaires d'un objet.
pub fn put(items: &mut BTreeMap<String, u32>, item: &str, count: u32) {
    if count == 0 {
        return;
    }
    let held = items.entry(item.to_owned()).or_default();
    *held = held.saturating_add(count);
}

/// Un conteneur d'un vaisseau : le vaisseau propriétaire (`items::PLAYER_SHIP`) et
/// l'identifiant du conteneur.
#[derive(Clone, Copy, Debug)]
pub struct Place<'a> {
    pub ship: &'a str,
    pub container: &'a str,
}

/// Range dans un conteneur au plus `count` exemplaires portés ; renvoie le nombre rangé, ou
/// `None` si le conteneur refuse l'objet (un rangement spécialisé : le frigo ne prend que la
/// nourriture, `ItemsFile::accepts`). Rien ne bouge alors.
pub fn store(
    items: &mut BTreeMap<String, u32>,
    containers: &mut Containers,
    catalog: &ItemsFile,
    at: Place,
    item: &str,
    count: u32,
) -> Option<u32> {
    if !catalog.accepts(at.container, item) {
        return None;
    }
    let moved = take(items, item, count);
    containers.add(at.ship, at.container, item, moved);
    Some(moved)
}

/// Prend dans un conteneur au plus `count` exemplaires ; renvoie le nombre pris.
pub fn retrieve(
    items: &mut BTreeMap<String, u32>,
    containers: &mut Containers,
    at: Place,
    item: &str,
    count: u32,
) -> u32 {
    let moved = containers.remove(at.ship, at.container, item, count);
    put(items, item, moved);
    moved
}

/// Équipe un objet porté dans son emplacement ; l'objet qui l'occupait revient dans
/// l'inventaire. Renvoie l'emplacement, ou `None` si l'objet ne se porte pas ou n'est pas là.
pub fn equip(
    items: &mut BTreeMap<String, u32>,
    equipment: &mut BTreeMap<EquipSlot, String>,
    catalog: &ItemsFile,
    item: &str,
) -> Option<EquipSlot> {
    let slot = catalog.get(item)?.slot?;
    if take(items, item, 1) == 0 {
        return None;
    }
    if let Some(previous) = equipment.insert(slot, item.to_owned()) {
        put(items, &previous, 1);
    }
    Some(slot)
}

/// Retire l'objet d'un emplacement et le remet dans l'inventaire ; renvoie l'objet.
pub fn unequip(
    items: &mut BTreeMap<String, u32>,
    equipment: &mut BTreeMap<EquipSlot, String>,
    slot: EquipSlot,
) -> Option<String> {
    let item = equipment.remove(&slot)?;
    put(items, &item, 1);
    Some(item)
}

/// Un pas au clavier dans une grille.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// La case d'arrivée.
    To(usize),
    /// Le pas sort par la gauche (ou la droite) de la grille, à cette rangée : l'appelant passe
    /// à la zone voisine (équipement, casier) ou reste sur place.
    OutLeft(usize),
    OutRight(usize),
}

/// Un pas (`dx`, `dy` : −1, 0 ou 1 ; `dy` positif vers le bas) depuis la case `at` d'une grille de
/// `len` cases sur `columns` colonnes. Le pas horizontal passe avant le vertical.
pub fn step(at: usize, len: usize, columns: usize, dx: i32, dy: i32) -> Step {
    let columns = columns.max(1);
    let len = len.max(1);
    let at = at.min(len - 1);
    let (row, col) = (at / columns, at % columns);
    if dx < 0 {
        return if col == 0 {
            Step::OutLeft(row)
        } else {
            Step::To(at - 1)
        };
    }
    if dx > 0 {
        return if col + 1 == columns || at + 1 >= len {
            Step::OutRight(row)
        } else {
            Step::To(at + 1)
        };
    }
    let rows = len.div_ceil(columns);
    let row = match dy.signum() {
        -1 => row.saturating_sub(1),
        1 => (row + 1).min(rows - 1),
        _ => row,
    };
    Step::To((row * columns + col).min(len - 1))
}

/// Le texte avec une majuscule initiale (les noms d'objets sont en minuscules dans la langue :
/// ils se glissent dans une phrase).
pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Une liste `identifiant:nombre` séparée par des virgules, pour l'autopilote : `SH_GIVE`
/// (`ration:3,spare_parts:25,medkit`, 999 au plus) et `SH_XP` (`piloting:250`). Nombre absent :
/// 1 ; nombre illisible ou nul : l'entrée est ignorée.
pub fn parse_counts(spec: &str, max: u32) -> Vec<(String, u32)> {
    spec.split(',')
        .filter_map(|part| {
            let part = part.trim();
            let (id, count) = match part.split_once(':') {
                Some((id, count)) => (id.trim(), count.trim().parse::<u32>().ok()?),
                None => (part, 1),
            };
            (!id.is_empty() && count > 0).then(|| (id.to_owned(), count.min(max)))
        })
        .collect()
}

/// Une liste `conteneur:nom` séparée par des virgules, pour l'autopilote (`SH_LOCKER_NAME`) :
/// une entrée sans `:`, sans conteneur ou sans nom est ignorée.
pub fn parse_names(spec: &str) -> Vec<(String, String)> {
    spec.split(',')
        .filter_map(|part| {
            let (id, name) = part.split_once(':')?;
            let (id, name) = (id.trim(), name.trim());
            (!id.is_empty() && !name.is_empty()).then(|| (id.to_owned(), name.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::PLAYER_SHIP;
    use sh_data::ItemDef;
    use sh_data::items::ItemSubcategory;

    fn def(id: &str, category: ItemCategory, slot: Option<EquipSlot>, max_stack: u32) -> ItemDef {
        ItemDef {
            id: id.to_owned(),
            name_key: format!("item.{id}"),
            description_key: format!("item.{id}.description"),
            category,
            subcategory: match category {
                ItemCategory::Consumable => Some(ItemSubcategory::Food),
                _ => None,
            },
            slot,
            max_stack,
            firearm: None,
        }
    }

    fn catalog() -> ItemsFile {
        ItemsFile {
            items: vec![
                def("datapad", ItemCategory::Tool, None, 1),
                def("spare_parts", ItemCategory::Material, None, 20),
                def(
                    "mag_pistol",
                    ItemCategory::Equipment,
                    Some(EquipSlot::Weapon),
                    1,
                ),
                def(
                    "flight_suit",
                    ItemCategory::Equipment,
                    Some(EquipSlot::Outfit),
                    1,
                ),
                def("ration", ItemCategory::Consumable, None, 10),
            ],
            ..ItemsFile::default()
        }
    }

    fn map(entries: &[(&str, u32)]) -> BTreeMap<String, u32> {
        entries
            .iter()
            .map(|(id, n)| ((*id).to_owned(), *n))
            .collect()
    }

    #[test]
    fn piles_decoupees_et_ordonnees() {
        let items = map(&[
            ("ration", 23),
            ("spare_parts", 5),
            ("mag_pistol", 2),
            ("relique", 4),
            ("datapad", 0),
        ]);
        let cells: Vec<(String, u32)> = stacks(&items, &catalog())
            .into_iter()
            .map(|s| (s.item, s.count))
            .collect();
        let expected: Vec<(String, u32)> = [
            // Équipement d'abord, une arme par case (pile de 1).
            ("mag_pistol", 1),
            ("mag_pistol", 1),
            // Consommables : 23 rations en piles de 10.
            ("ration", 10),
            ("ration", 10),
            ("ration", 3),
            ("spare_parts", 5),
            // Inconnu du catalogue : une seule case, en dernier.
            ("relique", 4),
        ]
        .iter()
        .map(|(id, n)| ((*id).to_owned(), *n))
        .collect();
        assert_eq!(cells, expected);
    }

    #[test]
    fn grille_en_rangees_entieres() {
        assert_eq!(grid_cells(0, 6, 4), 24);
        assert_eq!(grid_cells(24, 6, 4), 24);
        assert_eq!(grid_cells(25, 6, 4), 30);
        assert_eq!(grid_cells(3, 0, 1), 3);
    }

    #[test]
    fn progression_entre_deux_niveaux() {
        let start = progress(0.0, 100.0);
        assert_eq!((start.level, start.floor, start.next), (0, 0.0, 100.0));
        assert_eq!(start.fraction(), 0.0);
        assert_eq!(start.remaining(), 100.0);

        let half = progress(50.0, 100.0);
        assert_eq!(half.level, 0);
        assert!((half.fraction() - 0.5).abs() < 1e-9);

        // Niveau 1 à 100 points, niveau 2 à 400 : 250 points, c'est la moitié du chemin.
        let one = progress(250.0, 100.0);
        assert_eq!((one.level, one.floor, one.next), (1, 100.0, 400.0));
        assert!((one.fraction() - 0.5).abs() < 1e-9);
        assert_eq!(one.remaining(), 150.0);

        // Pile sur un seuil : le niveau est atteint, la barre repart de zéro.
        let exact = progress(400.0, 100.0);
        assert_eq!((exact.level, exact.floor, exact.next), (2, 400.0, 900.0));
        assert_eq!(exact.fraction(), 0.0);

        // Données absurdes : rien ne plante.
        assert_eq!(progress(-5.0, 100.0).level, 0);
        assert_eq!(progress(f64::NAN, 100.0).xp, 0.0);
        assert_eq!(progress(500.0, 0.0).fraction(), 0.0);
    }

    /// La barre suit exactement le niveau de la fiche (`xp_per_level × n²`).
    #[test]
    fn progression_coherente_avec_le_niveau() {
        for xp in [0.0, 99.9, 100.0, 399.0, 400.0, 2_500.0, 9_999.0, 10_000.0] {
            let p = progress(xp, 100.0);
            assert_eq!(p.level, (xp / 100.0_f64).sqrt().floor() as u32, "{xp}");
            assert!(p.floor <= xp && xp < p.next, "{xp} : {p:?}");
        }
    }

    #[test]
    fn prendre_et_poser() {
        let mut items = map(&[("ration", 3)]);
        assert_eq!(take(&mut items, "ration", 2), 2);
        assert_eq!(take(&mut items, "ration", 5), 1);
        assert!(items.is_empty());
        assert_eq!(take(&mut items, "ration", 1), 0);
        put(&mut items, "ration", 0);
        assert!(items.is_empty());
        put(&mut items, "ration", u32::MAX);
        put(&mut items, "ration", 4);
        assert_eq!(items.get("ration"), Some(&u32::MAX));
    }

    fn at(container: &str) -> Place<'_> {
        Place {
            ship: PLAYER_SHIP,
            container,
        }
    }

    #[test]
    fn ranger_et_reprendre_dans_un_casier() {
        let catalog = catalog();
        let mut items = map(&[("spare_parts", 25)]);
        let mut containers = Containers::default();
        containers.add(PLAYER_SHIP, "casier", "ration", 3);
        // Une case de pièces (pile de 20) part au casier ; il en reste 5 sur soi.
        assert_eq!(
            store(
                &mut items,
                &mut containers,
                &catalog,
                at("casier"),
                "spare_parts",
                20
            ),
            Some(20)
        );
        assert_eq!(items.get("spare_parts"), Some(&5));
        assert_eq!(
            containers
                .contents("casier")
                .and_then(|c| c.get("spare_parts")),
            Some(&20)
        );
        // On ne range pas ce qu'on n'a pas.
        assert_eq!(
            store(
                &mut items,
                &mut containers,
                &catalog,
                at("casier"),
                "medkit",
                1
            ),
            Some(0)
        );
        // Reprendre : au plus ce que contient le casier.
        assert_eq!(
            retrieve(&mut items, &mut containers, at("casier"), "ration", 10),
            3
        );
        assert_eq!(items.get("ration"), Some(&3));
        assert_eq!(
            containers.contents("casier").and_then(|c| c.get("ration")),
            None
        );
        assert_eq!(
            retrieve(&mut items, &mut containers, at("ailleurs"), "ration", 1),
            0
        );
    }

    /// Le frigo ne prend que la nourriture : le reste ne bouge pas.
    #[test]
    fn le_frigo_refuse_ce_qui_ne_se_mange_pas() {
        let mut catalog = catalog();
        catalog.storage.insert(
            "frigo".to_owned(),
            sh_data::items::StorageDef {
                accepts: Some(ItemSubcategory::Food),
                kind_key: Some("container.fridge".to_owned()),
            },
        );
        let mut items = map(&[("ration", 4), ("spare_parts", 5)]);
        let mut containers = Containers::default();
        assert_eq!(
            store(
                &mut items,
                &mut containers,
                &catalog,
                at("frigo"),
                "spare_parts",
                5
            ),
            None
        );
        assert_eq!(items.get("spare_parts"), Some(&5));
        assert_eq!(containers.contents("frigo"), None);
        assert_eq!(
            store(
                &mut items,
                &mut containers,
                &catalog,
                at("frigo"),
                "ration",
                4
            ),
            Some(4)
        );
        assert_eq!(containers.stock(PLAYER_SHIP, "frigo", &catalog), 4);
        // Le frigo d'un autre vaisseau du même modèle n'en sait rien.
        assert_eq!(containers.stock("autre", "frigo", &catalog), 0);
    }

    #[test]
    fn equiper_et_retirer() {
        let catalog = catalog();
        let mut items = map(&[("flight_suit", 1), ("mag_pistol", 1), ("ration", 2)]);
        let mut equipment = BTreeMap::from([(EquipSlot::Weapon, "mag_pistol".to_owned())]);
        // Une ration ne se porte pas.
        assert_eq!(equip(&mut items, &mut equipment, &catalog, "ration"), None);
        assert_eq!(
            equip(&mut items, &mut equipment, &catalog, "flight_suit"),
            Some(EquipSlot::Outfit)
        );
        assert_eq!(items.get("flight_suit"), None);
        // Le second pistolet prend la place du premier, qui revient dans l'inventaire.
        assert_eq!(
            equip(&mut items, &mut equipment, &catalog, "mag_pistol"),
            Some(EquipSlot::Weapon)
        );
        assert_eq!(items.get("mag_pistol"), Some(&1));
        // Pas d'objet : rien ne change.
        assert_eq!(
            equip(&mut items, &mut equipment, &catalog, "flight_suit"),
            None
        );
        assert_eq!(
            unequip(&mut items, &mut equipment, EquipSlot::Outfit),
            Some("flight_suit".to_owned())
        );
        assert_eq!(items.get("flight_suit"), Some(&1));
        assert_eq!(unequip(&mut items, &mut equipment, EquipSlot::Tool), None);
        assert_eq!(equipment.len(), 1);
    }

    #[test]
    fn pas_dans_la_grille() {
        // Grille de 3 colonnes, 7 cases : rangées 0-2, 3-5, 6.
        assert_eq!(step(4, 7, 3, 1, 0), Step::To(5));
        assert_eq!(step(5, 7, 3, 1, 0), Step::OutRight(1));
        assert_eq!(step(3, 7, 3, -1, 0), Step::OutLeft(1));
        assert_eq!(step(4, 7, 3, 0, 1), Step::To(6));
        assert_eq!(step(1, 7, 3, 0, -1), Step::To(1));
        assert_eq!(step(6, 7, 3, 1, 0), Step::OutRight(2));
        assert_eq!(step(99, 7, 3, 0, 0), Step::To(6));
        assert_eq!(step(0, 0, 0, 0, 1), Step::To(0));
    }

    #[test]
    fn majuscule_initiale() {
        assert_eq!(capitalize("épave"), "Épave");
        assert_eq!(capitalize("tablette"), "Tablette");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn noms_donnes_par_l_autopilote() {
        assert_eq!(
            parse_names("corvette_chambre_casier_1: Mes affaires ,sans_nom:, :x,frigo"),
            vec![(
                "corvette_chambre_casier_1".to_owned(),
                "Mes affaires".to_owned()
            )]
        );
    }

    #[test]
    fn objets_donnes_par_l_autopilote() {
        assert_eq!(
            parse_counts(" ration:3, medkit ,spare_parts:5000,bad:x,:2,zero:0,", 999),
            vec![
                ("ration".to_owned(), 3),
                ("medkit".to_owned(), 1),
                ("spare_parts".to_owned(), 999),
            ]
        );
    }
}
