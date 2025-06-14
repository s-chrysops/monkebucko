use bevy::{color::palettes::css::*, prelude::*};
use bevy_persistent::Persistent;
use bevy_text_animation::*;

use super::{AppState, Settings};
use egg::egg_plugin;

mod egg;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Loading,
    Egg,
    TopDown,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum MovementState {
    #[default]
    Enabled,
    Disabled,
}

pub fn game_plugin(app: &mut App) {
    app.add_plugins(egg_plugin)
        .init_state::<GameState>()
        .init_state::<MovementState>()
        .init_state::<InteractionState>()
        .add_systems(OnEnter(AppState::Game), game_setup)
        .add_systems(
            Update,
            play_interactions
                .run_if(pressing_interact_key)
                .run_if(in_state(MovementState::Enabled)),
        )
        .add_systems(
            Update,
            on_text_interaction.run_if(in_state(InteractionState::OnTextInteraction)),
        );
}

fn game_setup(mut commands: Commands) {
    commands.set_state(GameState::Egg);
}

#[derive(Debug, Component)]
struct Player;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum InteractionState {
    #[default]
    None,
    OnTextInteraction,
    Special,
}

// Current entity with [EntityInteraction] in focus
#[derive(Debug, Component)]
struct InteractTarget(Option<Entity>);

#[derive(Clone, Copy, Debug, Component)]
enum EntityInteraction {
    Text(&'static str),
    Special,
}

#[derive(Debug, Component)]
struct InteractionPanel;

#[derive(Debug, Component)]
struct InteractionText;

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
        // info!("Over Target: {}", target_entity);
        *player.into_inner() = InteractTarget(Some(target_entity));
    }
    // let depth = over.event().event.hit.depth;
    // info!(depth);
}

fn out_interactables(
    out: Trigger<Pointer<Out>>,
    q_interactables: Query<Entity, With<EntityInteraction>>,
    player: Single<&mut InteractTarget, With<Player>>,
) {
    // info!("Not Hovering");
    if let Ok(_target_entity) = q_interactables.get(out.target()) {
        // info!("Out Target: {}", target_entity);
        *player.into_inner() = InteractTarget(None);
    }
}

fn pressing_interact_key(
    key_input: Res<ButtonInput<KeyCode>>,
    settings: Res<Persistent<Settings>>,
) -> bool {
    key_input.just_pressed(settings.interact)
}

// Runs on Interact Key press
fn play_interactions(
    mut commands: Commands,
    player: Single<(&InteractTarget, &Transform), With<Player>>,
    q_interactables: Query<(&EntityInteraction, &Transform)>,
    // mut ev_writer: EventWriter<EntityInteraction>,
) {
    // info!("Interacting");
    // Guards InteractTarget(None)
    let (InteractTarget(Some(target_entity)), player_transform) = player.into_inner() else {
        return;
    };

    // info!("Target Entity: {}", target_entity);
    // let Some(target_entity) = interact_target else { return };

    let (entity_interaction, target_transform) = q_interactables.get_inner(*target_entity).unwrap();
    // info!("Entity Interaction: {:?}", entity_interaction);

    if player_transform
        .translation
        .distance(target_transform.translation)
        > INTERACTION_RANGE
    {
        return;
    }

    // ev_writer.write(*entity_interaction);

    match entity_interaction {
        EntityInteraction::Text(text) => {
            commands.set_state(MovementState::Disabled);
            commands.set_state(InteractionState::OnTextInteraction);
            commands.spawn(interaction_panel(text));
        }
        EntityInteraction::Special => {
            commands.set_state(MovementState::Disabled);
            commands.set_state(InteractionState::Special)
        }
    }
}

fn interaction_panel(text: &'static str) -> impl Bundle {
    (
        InteractionPanel,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(70.0),
            left: Val::Percent(10.0),
            width: Val::Percent(80.0),
            height: Val::Percent(20.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::linear_rgba(0.0, 0.0, 0.0, 0.75)),
        children![(
            InteractionText,
            Text::new(""),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(WHITE.into()),
            TextSimpleAnimator::new(text, 16.0),
        )],
    )
}

fn on_text_interaction(
    mut commands: Commands,
    key_input: Res<ButtonInput<KeyCode>>,
    settings: Res<Persistent<Settings>>,
    interaction_panel: Single<Entity, With<InteractionPanel>>,
    interaction_text: Single<(&mut TextSimpleAnimator, &mut Text), With<InteractionText>>,
) {
    if !key_input.any_just_pressed([settings.jump, settings.interact, KeyCode::Escape]) {
        return;
    }

    // Skip text animation
    let (mut animator, mut text) = interaction_text.into_inner();
    if animator.is_playing() {
        text.0 = animator.text.clone();
        animator.state = TextAnimationState::Stopped;
        return;
    }

    commands.entity(interaction_panel.into_inner()).despawn();
    commands.set_state(MovementState::Enabled);
    commands.set_state(InteractionState::None);
}

// fn play_interactions(
//     mut commands: Commands,
//     mut interaction_events: EventReader<EntityInteraction>,
// ) {
//     let Some(interaction) = interaction_events.read().last() else {
//         return;
//     };

//     commands.set_state(MovementState::Disabled);

//     match interaction {
//         EntityInteraction::Text(text) => {
//             let interaction_panel = commands
//                 .spawn((
//                     Node {
//                         width: Val::Percent(100.0),
//                         height: Val::Percent(50.0),
//                         justify_content: JustifyContent::Center,
//                         align_items: AlignItems::Center,
//                         ..default()
//                     },
//                     BackgroundColor(Color::linear_rgba(0.0, 0.0, 0.0, 0.75)),
//                 ))
//                 .id();

//             let interaction_text = commands
//                 .spawn((
//                     Text::new(""),
//                     TextFont {
//                         font_size: 32.0,
//                         ..default()
//                     },
//                     TextColor(WHITE.into()),
//                 ))
//                 .insert(TextSimpleAnimator::new(text, 8.0))
//                 .id();

//             commands
//                 .entity(interaction_panel)
//                 .add_child(interaction_text);
//             info!("test");
//         }
//         EntityInteraction::Special => info!("Interaction Test Successful: bucko!"),
//     }
// }
