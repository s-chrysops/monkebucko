use std::time::Duration;

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Event, Deref, PartialEq)]
pub struct SpriteAnimationFinished(Entity);

#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct SpriteAnimation {
    first_index: usize,
    last_index:  usize,
    frame_timer: Timer,
    looping:     bool,
}

impl SpriteAnimation {
    pub fn new(first: usize, last: usize, fps: u8) -> Self {
        SpriteAnimation {
            first_index: first,
            last_index:  last,
            frame_timer: Self::timer_from_fps(fps),
            looping:     false,
        }
    }

    pub fn _looping(mut self) -> Self {
        self.looping = true;
        self
    }

    // A little hack to change sprite without the Sprite component
    // Will set the frame on the NEXT render update
    pub fn set_frame(index: usize) -> Self {
        SpriteAnimation {
            first_index: index,
            last_index:  index,
            frame_timer: Self::timer_from_fps(240),
            looping:     true,
        }
    }

    fn timer_from_fps(fps: u8) -> Timer {
        Timer::new(Duration::from_secs_f32(1.0 / (fps as f32)), TimerMode::Once)
    }
}

pub fn sprite_animations_plugin(app: &mut App) {
    app.add_systems(Update, play_animations)
        .register_type::<SpriteAnimation>()
        .add_event::<SpriteAnimationFinished>();
}

fn play_animations(
    mut commands: Commands,
    mut e_writer: EventWriter<SpriteAnimationFinished>,
    mut query: Query<(Entity, &mut SpriteAnimation, &mut Sprite)>,
    time: Res<Time>,
) {
    query
        .iter_mut()
        .for_each(|(entity, mut animation, mut sprite)| {
            animation.frame_timer.tick(time.delta());
            if animation.frame_timer.just_finished() {
                let atlas = sprite
                    .texture_atlas
                    .as_mut()
                    .expect("Animated Sprite with no Texture Atlas");

                let event = SpriteAnimationFinished(entity);

                if animation.first_index == animation.last_index {
                    atlas.index = animation.first_index;
                    e_writer.write(event);
                    commands.trigger_targets(event, entity);
                } else if atlas.index < animation.last_index {
                    atlas.index += 1;
                    animation.frame_timer.reset();
                } else if animation.looping {
                    atlas.index = animation.first_index;
                    animation.frame_timer.reset();
                } else {
                    e_writer.write(event);
                    commands.trigger_targets(event, entity);
                }
            }
        });
}
