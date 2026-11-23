//! Attenuation models for 3D positional audio.

use crate::engine::ListenerFilter;

/// Attenuation model for 3D positional audio
#[derive(Debug, Clone)]
pub struct AttenuationModel {
    /// Reference distance (below this, no attenuation)
    pub reference_distance: f32,
    /// Maximum distance (beyond this, volume is min_volume)
    pub max_distance: f32,
    /// Rolloff factor (how quickly volume decreases)
    pub rolloff_factor: f32,
    /// Minimum volume at max distance
    pub min_volume: f32,
}

impl Default for AttenuationModel {
    fn default() -> Self {
        // LCE-style attenuation (similar to Java Edition)
        Self {
            reference_distance: 16.0, // 16 blocks
            max_distance: 64.0,       // 64 blocks
            rolloff_factor: 1.0,      // Linear rolloff
            min_volume: 0.0,
        }
    }
}

impl AttenuationModel {
    /// Create a new attenuation model with custom parameters
    pub fn new(
        reference_distance: f32,
        max_distance: f32,
        rolloff_factor: f32,
        min_volume: f32,
    ) -> Self {
        Self {
            reference_distance,
            max_distance,
            rolloff_factor,
            min_volume,
        }
    }

    /// Create a new attenuation model with default LCE parameters
    pub fn lce_legacy() -> Self {
        Self {
            reference_distance: 16.0,
            max_distance: 64.0,
            rolloff_factor: 1.0, // Linear
            min_volume: 0.05,    // Slight minimum for LCE
        }
    }

    /// Calculate attenuation for a sound at a given position relative to listeners
    pub fn calculate(
        &self,
        sound_pos: [f32; 3],
        listeners: &[[f32; 3]],
        listener_filter: &ListenerFilter,
    ) -> f32 {
        if listeners.is_empty() {
            return 0.0;
        }

        // Find the closest listener that should hear this sound
        let mut max_volume = 0.0;

        for (i, listener_pos) in listeners.iter().enumerate() {
            // Check if this listener should hear the sound
            if !self.listener_should_hear(i, listener_filter) {
                continue;
            }

            let dx = sound_pos[0] - listener_pos[0];
            let dy = sound_pos[1] - listener_pos[1];
            let dz = sound_pos[2] - listener_pos[2];
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            let volume = self.calculate_attenuation(distance);
            if volume > max_volume {
                max_volume = volume;
            }
        }

        max_volume
    }

    /// Calculate attenuation for a specific distance
    pub fn calculate_attenuation(&self, distance: f32) -> f32 {
        if distance <= self.reference_distance {
            return 1.0;
        }

        if distance >= self.max_distance {
            return self.min_volume;
        }

        // Linear rolloff (LCE/Java Edition style)
        let factor =
            (distance - self.reference_distance) / (self.max_distance - self.reference_distance);
        let volume = 1.0 - (factor * self.rolloff_factor);

        volume.clamp(self.min_volume, 1.0)
    }

    /// Check if a listener should hear the sound based on filter
    fn listener_should_hear(&self, listener_index: usize, filter: &ListenerFilter) -> bool {
        match filter {
            ListenerFilter::All => true,
            ListenerFilter::Only(indices) => indices.contains(&listener_index),
            ListenerFilter::Except(indices) => !indices.contains(&listener_index),
        }
    }

    /// Create attenuation model for modern Java Edition (reference)
    pub fn java_edition() -> Self {
        Self {
            reference_distance: 16.0,
            max_distance: 64.0,
            rolloff_factor: 1.0,
            min_volume: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attenuation_reference_distance() {
        let model = AttenuationModel::default();
        assert_eq!(model.calculate_attenuation(0.0), 1.0);
        assert_eq!(model.calculate_attenuation(16.0), 1.0);
    }

    #[test]
    fn test_attenuation_max_distance() {
        let model = AttenuationModel::default();
        assert_eq!(model.calculate_attenuation(64.0), 0.0);
        assert_eq!(model.calculate_attenuation(100.0), 0.0);
    }

    #[test]
    fn test_attenuation_linear_rolloff() {
        let model = AttenuationModel::default();
        // At halfway between reference and max (40 blocks)
        let vol = model.calculate_attenuation(40.0);
        // factor = (40-16)/(64-16) = 24/48 = 0.5
        // volume = 1.0 - 0.5 * 1.0 = 0.5
        assert!((vol - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_lce_legacy_attenuation() {
        let model = AttenuationModel::lce_legacy();
        assert_eq!(model.min_volume, 0.05);
        assert_eq!(model.calculate_attenuation(64.0), 0.05);
    }
}
