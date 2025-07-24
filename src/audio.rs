use bevy::{audio::*, prelude::*};
use bevy_persistent::Persistent;

use crate::Settings;

pub fn audio_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            fade_in::<AudioSink>,
            fade_in::<SpatialAudioSink>,
            fade_out::<AudioSink>,
            fade_out::<SpatialAudioSink>,
        ),
    );
}

// fn set_added_audio_volume(
//     settings: Res<Persistent<Settings>>,
//     mut q_sinks: Query<&mut AudioSink, (Added<AudioSink>, Without<AudioFadeIn>)>,
// ) {
//     let volume_music = Volume::Linear(settings.music_vol);
//     q_sinks
//         .iter_mut()
//         .for_each(|mut sink| sink.set_volume(volume_music));
// }

pub trait AudioType {}

#[derive(Debug, Component)]
pub struct Sound;

impl AudioType for Sound {}

#[derive(Debug, Component)]
pub struct Music;

impl AudioType for Music {}

#[derive(Debug, Component)]
pub struct Ambience;

impl AudioType for Ambience {}

const FADE_TIME: f32 = 2.0;

#[derive(Debug, Component)]
pub struct AudioFadeIn;

#[derive(Debug, Component)]
pub struct AudioFadeOut;

use bevy::ecs::component::Mutable;

fn fade_in<S>(
    mut commands: Commands,
    mut q_audio_sink: Query<(Entity, &mut S), With<AudioFadeIn>>,
    settings: Res<Persistent<Settings>>,
    time: Res<Time>,
) where
    S: Component<Mutability = Mutable> + AudioSinkPlayback,
{
    q_audio_sink.iter_mut().for_each(|(entity, mut sink)| {
        let current_volume = sink.volume();
        let new_volume =
            current_volume + Volume::Linear((time.delta_secs() / FADE_TIME) * settings.music_vol);
        sink.set_volume(new_volume);
        if new_volume.to_linear() >= settings.music_vol {
            sink.set_volume(Volume::Linear(settings.music_vol));
            commands.entity(entity).remove::<AudioFadeIn>();
        }
    });
}

fn fade_out<S>(
    mut commands: Commands,
    mut q_audio_sink: Query<(Entity, &mut S), With<AudioFadeOut>>,
    settings: Res<Persistent<Settings>>,
    time: Res<Time>,
) where
    S: Component<Mutability = Mutable> + AudioSinkPlayback,
{
    q_audio_sink.iter_mut().for_each(|(entity, mut sink)| {
        let current_volume = sink.volume();
        let new_volume =
            current_volume - Volume::Linear((time.delta_secs() / FADE_TIME) * settings.music_vol);
        sink.set_volume(new_volume);
        if new_volume.to_linear() <= 0.0 {
            commands.entity(entity).despawn();
        }
    });
}

pub fn audio_fade_out<T: Component + AudioType>(
    mut commands: Commands,
    q_ambience: Query<Entity, (With<T>, Without<AudioFadeIn>)>,
) {
    q_ambience.iter().for_each(|entity| {
        commands.entity(entity).insert(AudioFadeOut);
    });
}

// pub fn fade_out_ambience(
//     mut commands: Commands,
//     q_ambience: Query<Entity, (With<Ambience>, Without<AudioFadeIn>)>,
// ) {
//     q_ambience.iter().for_each(|entity| {
//         commands.entity(entity).insert(AudioFadeOut);
//     });
// }

// pub fn fade_out_music(
//     mut commands: Commands,
//     q_ambience: Query<Entity, (With<Music>, Without<AudioFadeIn>)>,
// ) {
//     q_ambience.iter().for_each(|entity| {
//         commands.entity(entity).insert(AudioFadeOut);
//     });
// }

pub fn kill_all_sound(mut commands: Commands, q_sounds: Query<Entity, With<Sound>>) {
    q_sounds.iter().for_each(|entity| {
        commands.entity(entity).despawn();
    });
}
