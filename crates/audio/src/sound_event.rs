//! Sound event definitions and categories.

use std::sync::Arc;

use fastrand;
use serde::{Deserialize, Serialize};

/// Sound category for volume control and mixing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundCategory {
    /// Master volume (affects all sounds)
    Master,
    /// Music tracks
    Music,
    /// Sound effects (block breaks, footsteps, etc.)
    Sfx,
    /// Block-related sounds
    Block,
    /// Entity sounds (mobs, players)
    Entity,
    /// Environmental sounds (weather, fire, etc.)
    Environment,
    /// UI sounds (button clicks, menu navigation)
    Ui,
    /// Player-specific sounds (hurt, death, eating)
    Player,
    /// Record/music disc sounds
    Record,
}

impl Default for SoundCategory {
    fn default() -> Self {
        Self::Sfx
    }
}

/// A variant of a sound event (different file, pitch, volume, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundVariant {
    /// Path to the sound file (relative to assets/sounds/)
    pub path: String,
    /// Weight for random selection (higher = more likely)
    pub weight: f32,
    /// Pitch multiplier (1.0 = normal)
    pub pitch: f32,
    /// Volume multiplier (1.0 = normal)
    pub volume: f32,
    /// Whether to stream the sound (for long sounds like music)
    pub stream: bool,
    /// Override attenuation distance (None = use default)
    pub attenuation_distance: Option<f32>,
    /// Minimum volume at max distance
    pub min_volume: f32,
}

impl Default for SoundVariant {
    fn default() -> Self {
        Self {
            path: String::new(),
            weight: 1.0,
            pitch: 1.0,
            volume: 1.0,
            stream: false,
            attenuation_distance: None,
            min_volume: 0.0,
        }
    }
}

/// A sound event definition (matches Java Edition sounds.json format)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEvent {
    /// Unique identifier (namespace:path)
    pub id: String,
    /// Category for volume control
    pub category: SoundCategory,
    /// Variants of this sound (randomly selected when played)
    pub variants: Vec<SoundVariant>,
    /// Whether to preload this sound on startup
    pub preload: bool,
}

impl SoundEvent {
    /// Create a new sound event with a single variant
    pub fn new(id: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            category: SoundCategory::Sfx,
            variants: vec![SoundVariant {
                path: path.into(),
                ..Default::default()
            }],
            preload: false,
        }
    }

    /// Add a variant to this sound event
    pub fn with_variant(mut self, variant: SoundVariant) -> Self {
        self.variants.push(variant);
        self
    }

    /// Set the category
    pub fn with_category(mut self, category: SoundCategory) -> Self {
        self.category = category;
        self
    }

    /// Set preload flag
    pub fn with_preload(mut self, preload: bool) -> Self {
        self.preload = preload;
        self
    }

    /// Get a random variant based on weights
    pub fn random_variant(&self) -> Option<&SoundVariant> {
        if self.variants.is_empty() {
            return None;
        }

        if self.variants.len() == 1 {
            return Some(&self.variants[0]);
        }

        let total_weight: f32 = self.variants.iter().map(|v| v.weight).sum();
        if total_weight <= 0.0 {
            return Some(&self.variants[0]);
        }

        let mut rand = fastrand::f32() * total_weight;
        for variant in &self.variants {
            rand -= variant.weight;
            if rand <= 0.0 {
                return Some(variant);
            }
        }

        Some(&self.variants[0])
    }

    /// Get the variant with highest weight (for deterministic selection)
    pub fn primary_variant(&self) -> Option<&SoundVariant> {
        self.variants
            .iter()
            .max_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_event_creation() {
        let event = SoundEvent::new(
            "minecraft:block.stone.break",
            "sounds/block/stone_break.ogg",
        );
        assert_eq!(event.id, "minecraft:block.stone.break");
        assert_eq!(event.variants.len(), 1);
        assert_eq!(event.variants[0].path, "sounds/block/stone_break.ogg");
    }

    #[test]
    fn test_sound_event_with_variants() {
        let event = SoundEvent::new(
            "minecraft:entity.zombie.ambient",
            "sounds/entity/zombie/ambient1.ogg",
        )
        .with_variant(SoundVariant {
            path: "sounds/entity/zombie/ambient2.ogg".to_string(),
            weight: 1.0,
            pitch: 1.1,
            ..Default::default()
        })
        .with_category(SoundCategory::Entity);

        assert_eq!(event.variants.len(), 2);
        assert_eq!(event.category, SoundCategory::Entity);
    }

    #[test]
    fn test_random_variant_selection() {
        let event = SoundEvent::new("test", "a.ogg")
            .with_variant(SoundVariant {
                path: "b.ogg".to_string(),
                weight: 1.0,
                ..Default::default()
            })
            .with_variant(SoundVariant {
                path: "c.ogg".to_string(),
                weight: 1.0,
                ..Default::default()
            });

        // Should always return one of the variants
        for _ in 0..100 {
            let variant = event.random_variant().unwrap();
            assert!(["a.ogg", "b.ogg", "c.ogg"].contains(&variant.path.as_str()));
        }
    }

    #[test]
    fn test_weighted_selection() {
        let event = SoundEvent::new("test", "a.ogg")
            .with_variant(SoundVariant {
                path: "b.ogg".to_string(),
                weight: 100.0,
                ..Default::default()
            })
            .with_variant(SoundVariant {
                path: "c.ogg".to_string(),
                weight: 1.0,
                ..Default::default()
            });

        // b.ogg should be selected ~99% of the time
        let mut b_count = 0;
        for _ in 0..1000 {
            if event.random_variant().unwrap().path == "b.ogg" {
                b_count += 1;
            }
        }
        assert!(b_count > 900); // Should be ~990
    }
}
