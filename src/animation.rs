use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Event, Deref, PartialEq)]
pub struct SpriteAnimationFinished(Entity);

#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct SpriteAnimation {
    first_index: usize,
    last_index:  usize,

    frame_timer: Timer,
    delay:       f32,

    looping: bool,
    playing: bool,
    started: bool,
}

impl SpriteAnimation {
    pub fn new(first: usize, last: usize, fps: u8) -> Self {
        SpriteAnimation {
            first_index: first,
            last_index:  last,
            frame_timer: Timer::from_seconds((fps as f32).recip(), TimerMode::Once),
            delay:       0.0, // because I need it and don't want another to add another Timer :p
            looping:     false,
            playing:     true,
            started:     false,
        }
    }

    // A little hack to change sprite without the Sprite component
    // Will set the frame on the NEXT render update
    pub fn set_frame(index: usize) -> Self {
        SpriteAnimation {
            first_index: index,
            last_index:  index,
            // Sadly this might cause inconsistencies with systems running
            // at over 1000 frames per second. Apologies to those users.
            frame_timer: Timer::from_seconds(0.001, TimerMode::Once),
            delay:       0.0,
            looping:     false,
            playing:     true,
            started:     false,
        }
    }

    pub fn with_delay(mut self, secs: f32) -> Self {
        self.delay = secs;
        self
    }

    pub fn looping(mut self) -> Self {
        self.looping = true;
        self
    }

    pub fn paused(mut self) -> Self {
        self.playing = false;
        self
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn change_fps(&mut self, fps: u8) {
        let elapsed = self.frame_timer.elapsed();
        self.frame_timer = Timer::from_seconds((fps as f32).recip(), TimerMode::Once);
        self.frame_timer.set_elapsed(elapsed);
    }
}

pub fn sprite_animations_plugin(app: &mut App) {
    app.add_systems(Update, (set_first_frame, play_animations).chain())
        .register_type::<SpriteAnimation>()
        .add_event::<SpriteAnimationFinished>();
}

fn set_first_frame(mut query: Query<(&mut SpriteAnimation, &mut Sprite)>) {
    query.iter_mut().for_each(|(mut animation, mut sprite)| {
        if !animation.started {
            sprite
                .texture_atlas
                .as_mut()
                .expect("Animated Sprite with no Texture Atlas")
                .index = animation.first_index;
            animation.started = true;
        }
    });
}

fn play_animations(
    mut commands: Commands,
    mut e_writer: EventWriter<SpriteAnimationFinished>,
    mut query: Query<(Entity, &mut SpriteAnimation, &mut Sprite)>,
    time: Res<Time>,
) {
    query
        .iter_mut()
        .filter(|(_entity, animation, _sprite)| animation.playing)
        .for_each(|(entity, mut animation, mut sprite)| {
            if animation.delay > 0.0 {
                animation.delay -= time.delta_secs();
            }

            // delay check will short-circuit the if statement and not tick the frame_timer... I hope
            if animation.delay <= 0.0 && animation.frame_timer.tick(time.delta()).just_finished() {
                let atlas = sprite
                    .texture_atlas
                    .as_mut()
                    .expect("Animated Sprite with no Texture Atlas");

                // let previous_index = atlas.index;
                if atlas.index == animation.last_index {
                    if animation.looping {
                        atlas.index = animation.first_index;
                        animation.frame_timer.reset();
                    }
                    let event = SpriteAnimationFinished(entity);
                    e_writer.write(event);
                    commands.trigger_targets(event, entity);
                } else if atlas.index < animation.last_index {
                    atlas.index += 1;
                    animation.frame_timer.reset();
                }
                // info!(
                //     "Playing sprite index {} - {}",
                //     animation.first_index, animation.last_index
                // );
                // info!(
                //     "Previous index: {} Current index: {}",
                //     previous_index, atlas.index
                // );
            }
        });
}
