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
        .init_state::<GameState>()
        .init_state::<MovementState>()
        .add_systems(OnEnter(AppState::Game), game_setup)
        .add_systems(Update, update_fade)
        .add_systems(
            Update,
            on_text_interaction.run_if(any_with_component::<InteractionText>),
        );
}

fn game_setup(mut commands: Commands) {
    commands.set_state(GameState::Egg);
}

// Current entity with [EntityInteraction] in focus
#[derive(Debug, Component)]
struct InteractTarget(Option<Entity>);

type SpecialInteractionFn = Arc<dyn Fn(&mut Commands, Entity) + Send + Sync>;

#[derive(Clone, Component)]
enum EntityInteraction {
    Text(&'static str),
    Special(SpecialInteractionFn),
}

impl EntityInteraction {
    pub fn special(
        func: impl Fn(&mut Commands, Entity) + Send + Sync + 'static,
    ) -> EntityInteraction {
        EntityInteraction::Special(Arc::new(func))
    }
}

impl core::fmt::Debug for EntityInteraction {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            EntityInteraction::Text(text) => write!(f, "Text(\"{}\")", text),
            EntityInteraction::Special(_) => write!(f, "Special(...)"),
        }
    }
}
fn pressing_interact_key(
    key_input: Res<ButtonInput<KeyCode>>,
    settings: Res<Persistent<Settings>>,
) -> bool {
    key_input.just_pressed(settings.interact)
}

fn play_interactions(
    In(input): In<Option<(EntityInteraction, Entity)>>,
    mut commands: Commands,
) {
    let Some((interaction, entity)) = input else {
        return;
    };

    commands.set_state(MovementState::Disabled);
    match interaction {
        EntityInteraction::Text(text) => {
            commands
                .spawn(interaction_panel())
                .with_child(interaction_text(text));
        }
        EntityInteraction::Special(func) => func(&mut commands, entity),
    }
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
            top: Val::Percent(70.0),
            left: Val::Percent(10.0),
            width: Val::Percent(80.0),
            height: Val::Percent(20.0),
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
