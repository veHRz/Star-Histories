//! Démarrage et chargement : `Boot` prépare l'interface, `Loading` lit et valide les données
//! sur un fil de calcul, sans bloquer l'affichage de l'écran de chargement (titre en capitales,
//! jauge à curseur ambre qui va et vient : la durée n'est pas mesurable).

use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::window::PrimaryWindow;
use sh_data::GameData;

use crate::data::{Data, Texts, data_dir};
use crate::materials::TextureLibrary;
use crate::state::{AppState, CurrentPlace, EnterGame, ViewLevel};
use crate::ui::theme::{self, UiFonts};

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StartRequest>()
            .add_systems(OnEnter(AppState::Boot), boot)
            .add_systems(
                OnEnter(AppState::Loading),
                (start_loading, spawn_loading_screen),
            )
            .add_systems(
                Update,
                (animate_loading_dots, finish_loading).run_if(in_state(AppState::Loading)),
            );
    }
}

/// Où aller une fois les données chargées : le menu principal, ou directement une nouvelle
/// partie ou une vue (autopilote `SH_START`).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub enum StartRequest {
    #[default]
    Menu,
    /// Comme le bouton « Nouvelle partie » (le lieu dépend des données).
    NewGame,
    /// Une sauvegarde du dossier du joueur, par son nom de fichier.
    Load(String),
    Direct(EnterGame),
}

#[derive(Resource)]
struct LoadTask(Task<Result<GameData, String>>);

fn boot(
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    let texts = Texts::load();
    window.title = texts.get("app.title");
    commands.insert_resource(texts);
    next.set(AppState::Loading);
}

fn start_loading(mut commands: Commands) {
    let dir = data_dir();
    info!("chargement des données depuis {}", dir.display());
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let data = GameData::load(&dir).map_err(|err| err.to_string())?;
        data.validate().map_err(|report| report.to_string())?;
        Ok(data)
    });
    commands.insert_resource(LoadTask(task));
}

/// Les points de suspension animés, dans un segment à chasse fixe : trois caractères toujours
/// (points puis espaces), le libellé centré ne bouge pas.
#[derive(Component)]
struct LoadingDots;

/// Le curseur qui parcourt la jauge (chargement sans progression mesurable : il va et vient).
#[derive(Component)]
struct LoadingSweep;

/// Jauge de chargement : largeur de la piste et du curseur, en pixels d'interface.
const TRACK_WIDTH: f32 = 360.0;
const SWEEP_WIDTH: f32 = 72.0;

fn spawn_loading_screen(mut commands: Commands, texts: Res<Texts>, fonts: Res<UiFonts>) {
    let root = commands
        .spawn((
            DespawnOnExit(AppState::Loading),
            theme::fullscreen_backdrop(),
        ))
        .id();
    commands.entity(root).with_children(|screen| {
        theme::spawn_corner_marks(screen, 28.0, 44.0, theme::SCREEN_DIM);
        screen.spawn(theme::heading(
            &fonts,
            &texts.get("app.title"),
            48.0,
            theme::TEXT,
        ));
        screen.spawn(theme::rule(
            px(TRACK_WIDTH + 80.0),
            UiRect::new(px(0), px(0), px(14), px(28)),
        ));
        screen.spawn((
            Text::new(theme::caps(&texts.get("loading.title"))),
            fonts.heading(20.0),
            TextColor(theme::TEXT_DIM),
            children![(
                TextSpan::new("   "),
                fonts.mono(20.0),
                TextColor(theme::TEXT_DIM),
                LoadingDots,
            )],
        ));
        // La piste et son curseur ambre : le curseur glisse par `UiTransform` (pas de
        // recalcul de la mise en page à chaque image).
        screen.spawn((
            Node {
                width: px(TRACK_WIDTH),
                height: px(4),
                margin: UiRect::top(px(14)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::RULE),
            children![(
                LoadingSweep,
                Node {
                    width: px(SWEEP_WIDTH),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(theme::ACCENT),
                UiTransform::default(),
            )],
        ));
    });
}

fn animate_loading_dots(
    time: Res<Time>,
    mut dots: Query<&mut TextSpan, With<LoadingDots>>,
    mut sweep: Query<&mut UiTransform, With<LoadingSweep>>,
) {
    let count = (time.elapsed_secs() * 3.0) as usize % 4;
    let wanted = format!("{:<3}", ".".repeat(count));
    for mut text in &mut dots {
        // Seulement quand le nombre de points change : un texte réécrit est remis en page.
        if text.0 != wanted {
            text.0 = wanted.clone();
        }
    }
    // Aller-retour doux sur la piste.
    let phase = 0.5 - 0.5 * (time.elapsed_secs() * 2.2).cos();
    let x = phase * (TRACK_WIDTH - SWEEP_WIDTH);
    for mut transform in &mut sweep {
        transform.translation = Val2::px(x, 0.0);
    }
}

fn finish_loading(
    mut commands: Commands,
    task: Option<ResMut<LoadTask>>,
    textures: Res<TextureLibrary>,
    start: Res<StartRequest>,
    mut app_state: ResMut<NextState<AppState>>,
    mut view: ResMut<NextState<ViewLevel>>,
    mut place: ResMut<CurrentPlace>,
    mut exit: MessageWriter<AppExit>,
) {
    // Les textures partagées doivent être prêtes (mipmaps comprises) avant le premier modèle :
    // les poser plus tard réenverrait tout au GPU en pleine partie.
    let Some(mut task) = task.filter(|_| textures.ready()) else {
        return;
    };
    let Some(result) = check_ready(&mut task.0) else {
        return;
    };
    commands.remove_resource::<LoadTask>();

    let data = match result {
        Ok(data) => data,
        Err(message) => {
            // Des données incohérentes rendent le jeu imprévisible : on refuse de démarrer.
            error!("données invalides, arrêt :\n{message}");
            exit.write(AppExit::error());
            return;
        }
    };
    let g = &data.galaxy;
    info!(
        "données chargées : {} systèmes, {} astres, {} stations, {} phénomènes, {} hubs de Portails, {} flottes, {} factions",
        g.systems.len(),
        g.bodies.len(),
        g.stations.len(),
        g.phenomena.len(),
        g.portal_hubs.len(),
        g.fleets.len(),
        data.factions.factions.len(),
    );

    let request = match &*start {
        StartRequest::Menu => None,
        StartRequest::NewGame => Some(EnterGame::new_game(&data)),
        StartRequest::Load(name) => {
            // Le chargement passe par le menu principal, comme depuis l'interface.
            let path = crate::save::saves_dir().join(name);
            match crate::save::read_save(&path) {
                Ok(save) => {
                    commands.insert_resource(crate::save::PendingLoad(Box::new(save)));
                }
                Err(err) => {
                    error!("SH_START : sauvegarde {} illisible : {err}", path.display());
                    exit.write(AppExit::error());
                    return;
                }
            }
            None
        }
        StartRequest::Direct(request) => Some(request.clone()),
    };
    match request {
        Some(request) => {
            if let Some(id) = &request.place
                && id != "test"
                && !place_exists(&data, id)
            {
                error!("SH_START : lieu inconnu « {id} »");
                exit.write(AppExit::error());
                return;
            }
            info!("démarrage direct : {:?}", request);
            request.apply(&mut app_state, &mut view, &mut place);
        }
        None => app_state.set(AppState::MainMenu),
    }
    commands.insert_resource(Data(Arc::new(data)));
}

fn place_exists(data: &GameData, id: &str) -> bool {
    data.sector(id).is_some()
        || sh_core::Id::new(id).is_ok_and(|id| data.galaxy.place_name(&id).is_some())
}
