//! Audio engine: positional 3D sound, splitscreen listeners, sound events, music logic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod attenuation;
mod engine;
mod music;
mod registry;
mod sound_event;

pub use attenuation::AttenuationModel;
pub use engine::{
    AudioEngine, AudioEngineConfig, AudioEngineHandle, AudioError, ListenerFilter, SoundHandle,
};
pub use music::{MusicContext, MusicScheduler, MusicSchedulerConfig, MusicTrack};
pub use registry::{RegistryError, SoundEventId, SoundEventRegistry};
pub use sound_event::{SoundCategory, SoundEvent, SoundVariant};

/// Re-exports for convenience
pub mod prelude {
    pub use crate::attenuation::AttenuationModel;
    pub use crate::engine::{
        AudioEngine, AudioEngineConfig, AudioEngineHandle, AudioError, ListenerFilter, SoundHandle,
    };
    pub use crate::music::{MusicContext, MusicScheduler, MusicSchedulerConfig, MusicTrack};
    pub use crate::registry::{RegistryError, SoundEventId, SoundEventRegistry};
    pub use crate::sound_event::{SoundCategory, SoundEvent, SoundVariant};
}
