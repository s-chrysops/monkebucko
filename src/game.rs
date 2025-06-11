use bevy::prelude::*;
use bevy_persistent::Persistent;

use super::{AppState, Settings};
use egg::egg_plugin;

mod egg;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Loading,
    Egg,
    _TopDown,
}

pub fn game_plugin(app: &mut App) {
    app.add_plugins(egg_plugin)
        .init_state::<GameState>()
        .add_systems(OnEnter(AppState::Game), game_setup)
        .add_systems(Update, entity_interactions.run_if(interacting));
}

pub fn game_setup(mut commands: Commands) {
    commands.set_state(GameState::Egg);
}

#[derive(Debug, Component)]
struct Player;

// Current entity with [EntityInteraction] in focus
#[derive(Debug, Component)]
struct InteractTarget(Option<Entity>);

#[derive(Debug, Component)]
enum EntityInteraction {
    Text(&'static str),
    Special,
}

const INTERACTION_RANGE: f32 = 1.0;
// const INTERACTION_NONE: usize = 0;
// const INTERACTION_CRACK: usize = 1;

fn over_interactables(
    over: Trigger<Pointer<Over>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    player: Single<&mut InteractTarget, With<Player>>,
) {
    // info!("Hovering");
    if let Ok(target_entity) = q_interactables.get(over.target()) {
        debug!("Over Target: {}", target_entity);
        *player.into_inner() = InteractTarget(Some(target_entity));
    }
}

fn out_interactables(
    out: Trigger<Pointer<Over>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    player: Single<&mut InteractTarget, With<Player>>,
) {
    // info!("Hovering");
    if let Ok(target_entity) = q_interactables.get(out.target()) {
        debug!("Out Target: {}", target_entity);
        *player.into_inner() = InteractTarget(None);
    }
}

fn interacting(key_input: Res<ButtonInput<KeyCode>>, settings: Res<Persistent<Settings>>) -> bool {
    key_input.just_pressed(settings.interact)
}

// Runs on Interact Key press
fn entity_interactions(
    player: Single<(&InteractTarget, &Transform), With<Player>>,
    q_interactables: Query<(&EntityInteraction, &Transform)>,
) {
    // Guards InteractTarget(None)
    let (InteractTarget(Some(target_entity)), player_transform) = player.into_inner() else {
        return;
    };

    // let Some(target_entity) = interact_target else { return };

    let (entity_interaction, target_transform) = q_interactables.get_inner(*target_entity).unwrap();

    if player_transform
        .translation
        .distance(target_transform.translation)
        > INTERACTION_RANGE
    {
        return;
    }

    match entity_interaction {
        EntityInteraction::Text(text) => info!("{}", text),
        EntityInteraction::Special => info!("Interaction Test Successful: bucko!"),
    };
}
