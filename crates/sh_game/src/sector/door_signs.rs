//! Noms des salles aux portes (M10, retour 8 de l'auteur) : au-dessus du linteau de chaque
//! porte, une plaque par face donne le nom de la salle **de l'autre côté** (« Cockpit » vu du
//! couloir, « Couloir » vu du cockpit).
//!
//! Les plaques sont des [`FootLabel`] (`views`), enfants de la porte : elles se lisent à pied, de
//! face, à quelques mètres, et seulement depuis la cellule de leur face (une cloison ne cache pas
//! l'interface : la cellule courante fait office d'occultation, et suit le streaming). Le nom
//! vient du descripteur (`name_key` de la cellule, `lang/<code>/rooms.ron`) : il est connu même
//! quand la salle d'en face n'est pas chargée.
//!
//! Rien sur une porte qui donne sur le vide (un côté « @… » non résolu : le sas en vol), ni sur
//! la face d'une salle sans nom.

use bevy::prelude::*;

use crate::data::Texts;
use crate::sector::ActiveSector;
use crate::sector::doors::Door;
use crate::views::FootLabel;

pub struct DoorSignsPlugin;

impl Plugin for DoorSignsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                add_signs.run_if(resource_exists::<ActiveSector>),
                retranslate_signs.run_if(resource_changed::<Texts>),
            ),
        );
    }
}

/// Hauteur de l'étiquette au-dessus du haut de l'ouverture (m) : sur le linteau.
const SIGN_RISE: f32 = 0.18;
/// Décalage de l'étiquette devant le plan de la porte, de chaque côté (m).
const SIGN_OFFSET: f32 = 0.2;

/// Une plaque de nom de salle : la clé de langue de la salle qu'elle annonce.
#[derive(Component)]
struct DoorSign {
    name_key: String,
}

/// Les deux faces d'une porte : (cellule d'où l'on lit, cellule annoncée, côté dans le repère de
/// la porte). La porte regarde de `a` vers `b` (`looking_to(normale)`) : son avant local (−Z)
/// pointe vers `b`, donc la face lue depuis `a` est du côté +Z.
fn faces(door: &Door) -> [(&str, &str, Vec3); 2] {
    [
        (door.a.as_str(), door.b.as_str(), Vec3::Z),
        (door.b.as_str(), door.a.as_str(), Vec3::NEG_Z),
    ]
}

/// Pose les plaques d'une porte nouvellement créée (`doors::spawn_doors`).
fn add_signs(
    mut commands: Commands,
    sector: Res<ActiveSector>,
    texts: Res<Texts>,
    doors: Query<(Entity, &Door), Added<Door>>,
) {
    let s = &sector.sector;
    for (entity, door) in &doors {
        if door.sealed {
            continue;
        }
        for (from, to, side) in faces(door) {
            let Some(name_key) = s.cell(to).and_then(|c| c.name_key.clone()) else {
                continue;
            };
            commands.spawn((
                Name::new(format!("Nom de salle {to} (depuis {from})")),
                FootLabel::new(texts.get(&name_key))
                    .facing(side)
                    .in_cell(from),
                DoorSign { name_key },
                // Repère de la porte : origine au pied de l'ouverture, au centre.
                Transform::from_translation(
                    Vec3::Y * (door.height + SIGN_RISE) + side * SIGN_OFFSET,
                ),
                ChildOf(entity),
            ));
        }
    }
}

/// La langue a changé : les plaques se retraduisent (rien à faire le reste du temps).
fn retranslate_signs(texts: Res<Texts>, mut signs: Query<(&DoorSign, &mut FootLabel)>) {
    for (sign, mut label) in &mut signs {
        let text = texts.get(&sign.name_key);
        if label.text != text {
            label.text = text;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sector::doors::DoorState;

    #[test]
    fn chaque_face_annonce_la_salle_d_en_face() {
        let door = Door {
            portal: "couloir__cockpit".into(),
            a: "couloir".into(),
            b: "cockpit".into(),
            state: DoorState::Closed,
            openness: 0.0,
            width: 1.4,
            height: 2.2,
            idle: 0.0,
            panel: None,
            sealed: false,
        };
        let [from_a, from_b] = faces(&door);
        assert_eq!((from_a.0, from_a.1), ("couloir", "cockpit"));
        assert_eq!((from_b.0, from_b.1), ("cockpit", "couloir"));
        // La porte regarde vers `b` (−Z local) : la face lue depuis `a` est à l'opposé.
        let forward = Transform::default().looking_to(Vec3::X, Vec3::Y);
        let a_side = forward.rotation * from_a.2;
        assert!(a_side.dot(Vec3::X) < -0.99, "{a_side:?}");
    }
}
