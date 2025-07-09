use std::hash::{Hash, Hasher};

use bevy::{platform::collections::HashMap, prelude::*};
use nohash_hasher::IsEnabled;
use serde::{Deserialize, Serialize};

use crate::{BuildBuckoNoHashHasher, EnumMap};

#[derive(
    Debug, Default, Clone, Copy, Component, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[reflect(Default, Component, Serialize, Deserialize)]
pub enum MonologueId {
    #[default]
    None,
    Test,
}

impl Hash for MonologueId {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_usize(*self as usize);
    }
}

impl IsEnabled for MonologueId {}

#[derive(Debug, Reflect, Serialize, Deserialize)]
pub struct Monologue {
    loop_index: usize,
    text:       String,
}

impl Monologue {
    fn new(loop_index: usize, text: String) -> Self {
        Monologue { loop_index, text }
    }

    fn get(&self, line: usize) -> Option<&str> {
        self.text.lines().nth(line)
    }

    fn len(&self) -> usize {
        self.text.lines().count()
    }
}

#[derive(Debug, Resource, Reflect, Serialize, Deserialize)]
#[reflect(Resource)]
pub struct MonologueServer {
    storage:  EnumMap<MonologueId, Monologue>,
    progress: EnumMap<MonologueId, usize>,
}

impl MonologueServer {
    pub fn next_line(&mut self, id: &MonologueId) -> &str {
        let line_index = self.progress.entry(*id).or_default();

        let monologue = self
            .storage
            .get(id)
            .expect("All MonologueId should be in storage");

        let line = monologue
            .get(*line_index)
            .expect("line index should never exceed line count");

        *line_index += 1;

        if *line_index == monologue.len() {
            *line_index = monologue.loop_index
        }

        line
    }
}

impl FromWorld for MonologueServer {
    fn from_world(_world: &mut World) -> Self {
        let mut storage: EnumMap<MonologueId, Monologue> =
            HashMap::with_hasher(BuildBuckoNoHashHasher::default());

        let progress: EnumMap<MonologueId, usize> =
            HashMap::with_hasher(BuildBuckoNoHashHasher::default());

        storage.insert(MonologueId::None, Monologue::new(0, String::new()));

        storage.insert(
            MonologueId::Test,
            Monologue::new(2, {
                let mut text = "First line\n".to_string();
                text.push_str("Second line\n");
                text.push_str("Third Line\n");
                text.push_str("Last line, go back to third");
                text
            }),
        );

        MonologueServer { storage, progress }
    }
}
