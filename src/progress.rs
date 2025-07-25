use bevy::prelude::*;
use bevy_persistent::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    time::Duration,
};

use crate::{EnumSet, game::topdown::TopdownMapIndex};

pub fn progress_plugin(app: &mut App) {
    app.add_systems(Startup, initialize_saves)
        .register_type::<Progress>()
        .register_type::<ProgressFlag>()
        .register_type::<ProgressStorage>();
}

fn initialize_saves(mut commands: Commands) {
    let config_dir = dirs::config_dir()
        .map(|native_config_dir| native_config_dir.join("monkebucko"))
        .unwrap_or(std::path::Path::new("local").to_path_buf());

    commands.insert_resource(
        Persistent::<ProgressStorage>::builder()
            .name("saves")
            .format(StorageFormat::Ron)
            .path(config_dir.join("saves.ron"))
            .default(ProgressStorage::default())
            .build()
            .expect("failed to initialize saves"),
    )
}

pub fn save_progress_to_disk(
    save_slot: Res<SaveSlot>,
    mut time_played_start: ResMut<TimePlayedStart>,
    mut progress: ResMut<Progress>,
    mut storage: ResMut<Persistent<ProgressStorage>>,
) {
    use bevy::platform::time::Instant;
    let elapsed = time_played_start.elapsed();
    time_played_start.0 = Instant::now();

    progress.time_played += elapsed;

    storage
        .update(|saves| {
            *saves.get_slot_mut(*save_slot) = Some(progress.clone());
        })
        .expect("Failed to save");
}

pub fn has_progress_flag(flag: ProgressFlag) -> impl FnMut(Res<Progress>) -> bool + Clone {
    move |progress: Res<Progress>| progress.contains(&flag)
}

#[derive(Debug, Deref, Resource)]
pub struct TimePlayedStart(pub bevy::platform::time::Instant);

#[derive(Debug, Clone, Copy, Component, Resource, Reflect)]
#[reflect(Component, Resource)]
pub enum SaveSlot {
    SlotA,
    SlotB,
    SlotC,
}

impl std::fmt::Display for SaveSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Default, Deref, DerefMut, Resource, Reflect, Serialize, Deserialize)]
#[reflect(Resource, Serialize, Deserialize)]
pub struct ProgressStorage([Option<Progress>; 3]);

impl ProgressStorage {
    pub fn get_slot(&self, slot: SaveSlot) -> &Option<Progress> {
        // SAFETY: ['SaveSlot'] varient count MUST not exceed ['ProgressStorage'] array len (3)
        unsafe { self.0.get_unchecked(slot as usize) }
    }

    pub fn get_slot_mut(&mut self, slot: SaveSlot) -> &mut Option<Progress> {
        // SAFETY: ['SaveSlot'] varient count MUST not exceed ['ProgressStorage'] array len (3)
        unsafe { self.0.get_unchecked_mut(slot as usize) }
    }
}

#[derive(Debug, Clone, Deref, DerefMut, Resource, Reflect, Serialize, Deserialize)]
#[reflect(Resource, Serialize, Deserialize)]
pub struct Progress {
    pub time_played: Duration,

    #[deref]
    pub flags: EnumSet<ProgressFlag>,

    pub map:      TopdownMapIndex,
    pub position: Vec2,
}

impl Default for Progress {
    fn default() -> Self {
        const FIRST_SPAWN: Vec2 = vec2(832.0, 1024.0);
        Progress {
            time_played: Duration::default(),

            flags: EnumSet::default(),

            map:      TopdownMapIndex::default(),
            position: FIRST_SPAWN,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum ProgressFlag {
    #[default]
    None,
    CrackOpen,
    FirstLaunch,
}

impl Hash for ProgressFlag {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_usize(*self as usize);
    }
}

impl nohash_hasher::IsEnabled for ProgressFlag {}
