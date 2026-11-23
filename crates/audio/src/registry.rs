//! Sound event registry and sounds.json parsing.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::sound_event::{SoundCategory, SoundEvent, SoundVariant};

/// Errors that can occur when loading sound events
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Failed to read sounds.json: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse sounds.json: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Sound event not found: {0}")]
    NotFound(String),
    #[error("Duplicate sound event: {0}")]
    Duplicate(String),
}

/// Registry of sound events loaded from sounds.json
#[derive(Debug, Default)]
pub struct SoundEventRegistry {
    events: HashMap<String, Arc<SoundEvent>>,
}

impl SoundEventRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
        }
    }

    /// Load sound events from a sounds.json file
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), RegistryError> {
        let content = std::fs::read_to_string(path)?;
        self.load_from_json(&content)
    }

    /// Load sound events from a JSON string
    pub fn load_from_json(&mut self, json: &str) -> Result<(), RegistryError> {
        let raw_events: HashMap<String, RawSoundEvent> = serde_json::from_str(json)?;

        for (id, raw) in raw_events {
            if self.events.contains_key(&id) {
                return Err(RegistryError::Duplicate(id));
            }

            let event = SoundEvent {
                id: id.clone(),
                category: raw.category.unwrap_or(SoundCategory::Sfx),
                variants: raw
                    .sounds
                    .into_iter()
                    .map(|s| SoundVariant {
                        path: s.name,
                        weight: s.weight.unwrap_or(1.0),
                        pitch: s.pitch.unwrap_or(1.0),
                        volume: s.volume.unwrap_or(1.0),
                        stream: s.stream.unwrap_or(false),
                        attenuation_distance: s.attenuation_distance,
                        min_volume: s.min_volume.unwrap_or(0.0),
                    })
                    .collect(),
                preload: raw.preload.unwrap_or(false),
            };

            self.events.insert(id, Arc::new(event));
        }

        Ok(())
    }

    /// Get a sound event by ID
    pub fn get(&self, id: &str) -> Option<Arc<SoundEvent>> {
        self.events.get(id).cloned()
    }

    /// Get a sound event by ID (required, panics if not found)
    pub fn get_required(&self, id: &str) -> Arc<SoundEvent> {
        self.get(id)
            .expect(&format!("Sound event not found: {}", id))
    }

    /// Check if a sound event exists
    pub fn contains(&self, id: &str) -> bool {
        self.events.contains_key(id)
    }

    /// Iterate over all sound events
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<SoundEvent>)> {
        self.events.iter()
    }

    /// Get the number of registered sound events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Register a sound event programmatically
    pub fn register(&mut self, event: SoundEvent) -> Result<(), RegistryError> {
        let id = event.id.clone();
        if self.events.contains_key(&id) {
            return Err(RegistryError::Duplicate(id));
        }
        self.events.insert(id, Arc::new(event));
        Ok(())
    }
}

/// Raw sound event from sounds.json
#[derive(Debug, Deserialize)]
struct RawSoundEvent {
    category: Option<SoundCategory>,
    sounds: Vec<RawSoundVariant>,
    preload: Option<bool>,
}

/// Raw sound variant from sounds.json
#[derive(Debug, Deserialize)]
struct RawSoundVariant {
    name: String,
    weight: Option<f32>,
    pitch: Option<f32>,
    volume: Option<f32>,
    stream: Option<bool>,
    attenuation_distance: Option<f32>,
    min_volume: Option<f32>,
}

/// Identifier for a sound event in the registry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SoundEventId(pub u32);

impl SoundEventId {
    /// Create a new sound event ID from a hash of the string
    pub fn from_string(s: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        Self(hasher.finish() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SOUNDS_JSON: &str = r#"{
        "minecraft:block.stone.break": {
            "category": "block",
            "sounds": [
                { "name": "sounds/block/stone_break1.ogg", "weight": 1.0 },
                { "name": "sounds/block/stone_break2.ogg", "weight": 1.0 },
                { "name": "sounds/block/stone_break3.ogg", "weight": 1.0 }
            ]
        },
        "minecraft:entity.zombie.ambient": {
            "category": "entity",
            "sounds": [
                { "name": "sounds/entity/zombie/ambient1.ogg", "weight": 1.0, "pitch": 0.9 },
                { "name": "sounds/entity/zombie/ambient2.ogg", "weight": 1.0, "pitch": 1.1 }
            ]
        }
    }"#;

    #[test]
    fn test_load_sounds_json() {
        let mut registry = SoundEventRegistry::new();
        registry.load_from_json(TEST_SOUNDS_JSON).unwrap();

        assert_eq!(registry.len(), 2);
        assert!(registry.contains("minecraft:block.stone.break"));
        assert!(registry.contains("minecraft:entity.zombie.ambient"));

        let stone_break = registry.get("minecraft:block.stone.break").unwrap();
        assert_eq!(stone_break.variants.len(), 3);
        assert_eq!(
            stone_break.category,
            crate::sound_event::SoundCategory::Block
        );

        let zombie = registry.get("minecraft:entity.zombie.ambient").unwrap();
        assert_eq!(zombie.variants.len(), 2);
        assert_eq!(zombie.category, crate::sound_event::SoundCategory::Entity);
        assert_eq!(zombie.variants[0].pitch, 0.9);
        assert_eq!(zombie.variants[1].pitch, 1.1);
    }

    #[test]
    fn test_duplicate_detection() {
        let mut registry = SoundEventRegistry::new();
        registry.load_from_json(TEST_SOUNDS_JSON).unwrap();

        let duplicate_json = r#"{
            "minecraft:block.stone.break": {
                "sounds": [{ "name": "sounds/block/stone_break4.ogg" }]
            }
        }"#;

        let result = registry.load_from_json(duplicate_json);
        assert!(result.is_err());
    }
}
