use std::sync::Arc;

use bevy::{color::palettes::css::*, prelude::*};
use bevy_persistent::Persistent;
use bevy_text_animation::*;

use super::{AppState, Settings};
use egg::egg_plugin;
use topdown::topdown_plugin;

mod egg;
mod topdown;

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

#[derive(Debug, Component)]
struct Player;

#[derive(Debug, Component)]
struct WorldCamera;

pub fn game_plugin(app: &mut App) {
    app.add_plugins((egg_plugin, topdown_plugin))
        .add_systems(OnEnter(AppState::Game), game_setup)
        .add_systems(PreUpdate, get_user_input)
        .add_systems(
            Update,
            (
                advance_text_interaction
                    .run_if(pressed_advance_key.and(any_with_component::<InteractionText>)),
                conclude_text_interaction
                    .run_if(in_state(InteractionState::Text).and(on_event::<InteractionAdvance>)),
                update_fade,
            ),
        )
        .add_event::<InteractionAdvance>()
        .init_resource::<UserInput>()
        .init_state::<GameState>()
        .init_state::<MovementState>()
        .init_state::<InteractionState>();
}

fn game_setup(mut commands: Commands) {
    commands.set_state(GameState::Egg);
}

#[derive(Debug, Resource)]
struct UserInput {
    raw_vector:           Vec2,
    last_valid_direction: Dir2,
}

impl Default for UserInput {
    fn default() -> Self {
        UserInput {
            raw_vector:           Vec2::ZERO,
            last_valid_direction: Dir2::EAST,
        }
    }
}

impl UserInput {
    pub fn moving(&self) -> bool {
        self.raw_vector != Vec2::ZERO
    }
}

fn get_user_input(
    key_input: Res<ButtonInput<KeyCode>>,
    settings: Res<Persistent<Settings>>,
    mut user_input: ResMut<UserInput>,
) {
    let mut raw_vector = Vec2::ZERO;
    if key_input.pressed(settings.up) {
        raw_vector += Vec2::Y;
    }
    if key_input.pressed(settings.down) {
        raw_vector -= Vec2::Y;
    }
    if key_input.pressed(settings.right) {
        raw_vector += Vec2::X;
    }
    if key_input.pressed(settings.left) {
        raw_vector -= Vec2::X;
    }
    user_input.raw_vector = raw_vector;

    if let Ok(new_direction) = Dir2::new(raw_vector) {
        user_input.last_valid_direction = new_direction;
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum InteractionState {
    #[default]
    None,
    Text,
    Monologue,
    Dialogue,
}

#[derive(Debug, Event)]
struct InteractionAdvance;

// Current entity with [EntityInteraction] in focus
#[derive(Debug, Component)]
struct InteractTarget(Option<Entity>);

#[derive(Clone, Copy, Component)]
enum EntityInteraction {
    Text(&'static str),
    Monologue,
    Dialouge,
    Special(Entity),
}

#[derive(Component)]
struct SpecialInteraction(SpecialInteractionFn);
type SpecialInteractionFn = Arc<dyn Fn(&mut Commands, Entity) + Send + Sync>;

impl core::fmt::Debug for SpecialInteraction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("SpecialInteraction").finish()
    }
}

impl SpecialInteraction {
    fn new(func: impl Fn(&mut Commands, Entity) + Send + Sync + 'static) -> Self {
        SpecialInteraction(Arc::new(func))
    }
}

fn play_interactions(
    In(input): In<Option<EntityInteraction>>,
    special_interactions: Query<&SpecialInteraction, With<EntityInteraction>>,
    mut commands: Commands,
) {
    let Some(interaction) = input else {
        debug!("Entity Interaction input invalid");
        return;
    };

    commands.set_state(MovementState::Disabled);
    match interaction {
        EntityInteraction::Text(text) => {
            commands.set_state(InteractionState::Text);
            commands
                .spawn(interaction_panel())
                .with_child(interaction_text(text));
        }
        EntityInteraction::Monologue => {
            commands.set_state(InteractionState::Monologue);
        }
        EntityInteraction::Dialouge => {
            commands.set_state(InteractionState::Dialogue);
            commands
                .spawn(interaction_panel())
                .with_child(interaction_text(""));
        }
        EntityInteraction::Special(entity) => {
            special_interactions
                .get(entity)
                .expect("Entity should have SpecialInteraction component")
                .0(&mut commands, entity);
        }
    }
}

fn advance_text_interaction(
    mut e_writer: EventWriter<InteractionAdvance>,
    interaction_text: Single<(&mut TextSimpleAnimator, &mut Text), With<InteractionText>>,
) {
    // Skip playing text animation
    let (mut animator, mut text) = interaction_text.into_inner();
    if animator.is_playing() {
        text.0 = animator.text.clone();
        animator.stop();
        return;
    }

    e_writer.write(InteractionAdvance);
}

fn conclude_text_interaction(
    mut commands: Commands,
    interaction_panel: Single<Entity, With<InteractionPanel>>,
) {
    commands.entity(interaction_panel.into_inner()).despawn();
    commands.set_state(InteractionState::None);
    commands.set_state(MovementState::Enabled);
}

#[derive(Debug, Component)]
struct InteractionPanel;

#[derive(Debug, Component)]
struct InteractionText;

fn interaction_panel() -> impl Bundle {
    (
        InteractionPanel,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(87.5),
            left: Val::Percent(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(12.5),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::linear_rgba(0.0, 0.0, 0.0, 0.75)),
    )
}

fn interaction_text(text: &'static str) -> impl Bundle {
    (
        InteractionText,
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(WHITE.into()),
        TextSimpleAnimator::new(text, 16.0),
    )
}

#[derive(Debug, Component)]
struct Fade;

#[derive(Clone, Component, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
struct Opacity(f32);

#[allow(clippy::type_complexity)]
fn update_fade(mut q_fade: Query<(&mut Sprite, &Opacity), (With<Fade>, Changed<Opacity>)>) {
    q_fade.iter_mut().for_each(|(mut sprite, Opacity(alpha))| {
        sprite.color = sprite.color.with_alpha(*alpha);
    });
}

fn pressed_interact_key(
    key_input: Res<ButtonInput<KeyCode>>,
    settings: Res<Persistent<Settings>>,
) -> bool {
    key_input.just_pressed(settings.interact)
}

fn pressed_advance_key(
    key_input: Res<ButtonInput<KeyCode>>,
    settings: Res<Persistent<Settings>>,
) -> bool {
    key_input.any_just_pressed([
        settings.jump,
        settings.interact,
        KeyCode::Escape,
        KeyCode::Enter,
        KeyCode::NumpadEnter,
    ])
}
