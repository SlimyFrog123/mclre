//! Background music scheduler matching LCE track selection & cooldown timings.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use kira::{AudioManager, Tween, sound::static_sound::StaticSoundData};

use thiserror::Error;

/// Errors that can occur in the music scheduler
#[derive(Debug, Error)]
pub enum MusicError {
    #[error("Failed to load music track: {0}")]
    LoadFailed(String),
    #[error("No music tracks available for context: {0:?}")]
    NoTracksAvailable(MusicContext),
    #[error("Music scheduler not initialized")]
    NotInitialized,
}

/// Music context for track selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MusicContext {
    /// Main menu / title screen
    Menu,
    /// Creative mode
    Creative,
    /// Survival mode (Overworld)
    Survival,
    /// Nether dimension
    Nether,
    /// The End dimension
    End,
    /// Underwater
    Underwater,
    /// Cave/underground
    Cave,
    /// Night time (Overworld)
    Night,
    /// Combat
    Combat,
    /// Credits
    Credits,
}

impl Default for MusicContext {
    fn default() -> Self {
        Self::Menu
    }
}

/// A music track entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MusicTrack {
    /// Sound event ID for this track
    pub event_id: String,
    /// Contexts where this track can play
    pub contexts: Vec<MusicContext>,
    /// Minimum delay before this track can play again (seconds)
    pub min_delay: f32,
    /// Maximum delay before this track can play again (seconds)
    pub max_delay: f32,
    /// Weight for random selection
    pub weight: f32,
    /// Whether this track is a "rare" track (plays less often)
    pub rare: bool,
}

impl Default for MusicTrack {
    fn default() -> Self {
        Self {
            event_id: String::new(),
            contexts: vec![MusicContext::Survival],
            min_delay: 120.0,
            max_delay: 300.0,
            weight: 1.0,
            rare: false,
        }
    }
}

/// Configuration for the music scheduler
#[derive(Debug, Clone)]
pub struct MusicSchedulerConfig {
    /// Minimum time between music tracks (seconds)
    pub min_track_gap: f32,
    /// Volume for music tracks (0.0 to 1.0)
    pub music_volume: f32,
    /// Fade in duration (seconds)
    pub fade_in: f32,
    /// Fade out duration (seconds)
    pub fade_out: f32,
}

impl Default for MusicSchedulerConfig {
    fn default() -> Self {
        Self {
            min_track_gap: 0.0,
            music_volume: 1.0,
            fade_in: 2.0,
            fade_out: 2.0,
        }
    }
}

/// Music scheduler state
pub struct MusicScheduler {
    tracks: HashMap<MusicContext, Vec<MusicTrack>>,
    current_track: Option<PlayingTrack>,
    last_track_end: Option<Instant>,
    next_track_time: Option<Instant>,
    config: MusicSchedulerConfig,
    current_context: MusicContext,
    force_next_track: Option<MusicContext>,
}

/// A currently playing track
struct PlayingTrack {
    handle: kira::sound::static_sound::StaticSoundHandle,
    track: MusicTrack,
    start_time: Instant,
    fade_state: FadeState,
}

enum FadeState {
    FadingIn { start: Instant, duration: f32 },
    Playing,
    FadingOut { start: Instant, duration: f32 },
    Stopped,
}

impl MusicScheduler {
    /// Create a new music scheduler
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
            current_track: None,
            last_track_end: None,
            next_track_time: None,
            config: MusicSchedulerConfig::default(),
            current_context: MusicContext::Menu,
            force_next_track: None,
        }
    }

    /// Create a new music scheduler with custom config
    pub fn with_config(config: MusicSchedulerConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Register a music track for a context
    pub fn register_track(&mut self, track: MusicTrack) {
        for context in &track.contexts {
            self.tracks.entry(*context).or_default().push(track.clone());
        }
    }

    /// Register multiple tracks at once
    pub fn register_tracks(&mut self, tracks: Vec<MusicTrack>) {
        for track in tracks {
            self.register_track(track);
        }
    }

    /// Update the current music context (dimension, biome, etc.)
    pub fn update_context(&mut self, context: MusicContext) {
        if self.current_context != context {
            self.current_context = context;
            // Force a track change when context changes
            self.force_next_track = Some(context);
            self.stop_current_track();
        }
    }

    /// Force a specific track to play next
    pub fn force_track(&mut self, context: MusicContext) {
        self.force_next_track = Some(context);
        self.stop_current_track();
    }

    /// Tick the music scheduler (call every frame/update)
    pub fn tick(&mut self, manager: &mut AudioManager) {
        let now = Instant::now();

        // Handle fade in/out
        if let Some(track) = &mut self.current_track {
            match &mut track.fade_state {
                FadeState::FadingIn { start, duration } => {
                    let elapsed = start.elapsed().as_secs_f32();
                    let progress = (elapsed / *duration).clamp(0.0, 1.0);
                    // TODO: Apply volume fade to track handle
                    if progress >= 1.0 {
                        track.fade_state = FadeState::Playing;
                    }
                }
                FadeState::FadingOut { start, duration } => {
                    let elapsed = start.elapsed().as_secs_f32();
                    let progress = (elapsed / *duration).clamp(0.0, 1.0);
                    // TODO: Apply volume fade to track handle
                    if progress >= 1.0 {
                        track.fade_state = FadeState::Stopped;
                    }
                }
                FadeState::Stopped => {
                    self.current_track = None;
                    self.last_track_end = Some(now);
                    self.schedule_next_track(now);
                }
                _ => {}
            }
        }

        // Check if it's time to start a new track
        if self.current_track.is_none() {
            if let Some(next_time) = self.next_track_time {
                if now >= next_time {
                    self.start_next_track(manager);
                }
            } else {
                self.schedule_next_track(now);
            }
        }
    }

    /// Schedule the next track to play
    fn schedule_next_track(&mut self, now: Instant) {
        // Check if we have a forced context
        let context = self.force_next_track.take().unwrap_or(self.current_context);

        // Get available tracks for this context
        let available_tracks = self.tracks.get(&context);
        if let Some(tracks) = available_tracks {
            if tracks.is_empty() {
                // No tracks for this context, try default
                self.schedule_next_track_for_context(now, MusicContext::Menu);
                return;
            }

            // Pick a random track weighted by weight
            let track = self.pick_random_track(tracks);
            let min_delay = track.min_delay;
            let max_delay = track.max_delay;
            let delay = fastrand::f32() * (max_delay - min_delay) + min_delay;

            self.next_track_time = Some(now + Duration::from_secs_f32(delay));
        } else {
            // No tracks for current context, try menu
            self.schedule_next_track_for_context(now, MusicContext::Menu);
        }
    }

    fn schedule_next_track_for_context(&mut self, now: Instant, context: MusicContext) {
        if let Some(tracks) = self.tracks.get(&context) {
            if !tracks.is_empty() {
                let track = self.pick_random_track(tracks);
                let min_delay = track.min_delay;
                let max_delay = track.max_delay;
                let delay = fastrand::f32() * (max_delay - min_delay) + min_delay;
                self.next_track_time = Some(now + Duration::from_secs_f32(delay));
            }
        }
    }

    /// Start the next scheduled track
    fn start_next_track(&mut self, manager: &mut AudioManager) {
        let context = self.force_next_track.take().unwrap_or(self.current_context);

        if let Some(tracks) = self.tracks.get(&context) {
            if tracks.is_empty() {
                return;
            }

            let track = self.pick_random_track(tracks).clone();

            // Load and play the track
            match StaticSoundData::from_file(&track.event_id) {
                Ok(sound_data) => {
                    let mut handle = match manager.play(sound_data) {
                        Ok(h) => h,
                        Err(e) => {
                            tracing::warn!("Failed to play music track: {}", e);
                            return;
                        }
                    };
                    handle.set_volume(0.0, Tween::default()); // Start at 0 for fade in
                    // Note: loop_region is set on sound data, not handle

                    self.current_track = Some(PlayingTrack {
                        handle,
                        track,
                        start_time: Instant::now(),
                        fade_state: FadeState::FadingIn {
                            start: Instant::now(),
                            duration: self.config.fade_in,
                        },
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to load music track {}: {}", track.event_id, e);
                }
            }
        }
    }

    /// Stop the currently playing track with fade out
    fn stop_current_track(&mut self) {
        if let Some(track) = &mut self.current_track {
            track.fade_state = FadeState::FadingOut {
                start: Instant::now(),
                duration: self.config.fade_out,
            };
        }
    }

    /// Pick a random track weighted by weight
    fn pick_random_track<'a>(&self, tracks: &'a [MusicTrack]) -> &'a MusicTrack {
        if tracks.len() == 1 {
            return &tracks[0];
        }

        let total_weight: f32 = tracks.iter().map(|t| t.weight).sum();
        if total_weight <= 0.0 {
            return &tracks[0];
        }

        let mut rand = fastrand::f32() * total_weight;
        for track in tracks {
            rand -= track.weight;
            if rand <= 0.0 {
                return track;
            }
        }

        &tracks[0]
    }

    /// Get the current music context
    pub fn current_context(&self) -> MusicContext {
        self.current_context
    }

    /// Check if music is currently playing
    pub fn is_playing(&self) -> bool {
        matches!(
            self.current_track.as_ref().map(|t| &t.fade_state),
            Some(FadeState::Playing) | Some(FadeState::FadingIn { .. })
        )
    }

    /// Load default LCE music tracks
    pub fn load_lce_tracks(&mut self) {
        // LCE music tracks (from reference)
        let lce_tracks = vec![
            // Menu music
            MusicTrack {
                event_id: "minecraft:music.menu".to_string(),
                contexts: vec![MusicContext::Menu],
                min_delay: 0.0,
                max_delay: 0.0,
                weight: 1.0,
                rare: false,
            },
            // Creative mode music (C418 - Sweden, etc.)
            MusicTrack {
                event_id: "minecraft:music.creative".to_string(),
                contexts: vec![MusicContext::Creative],
                min_delay: 120.0,
                max_delay: 300.0,
                weight: 1.0,
                rare: false,
            },
            // Survival Overworld music
            MusicTrack {
                event_id: "minecraft:music.overworld".to_string(),
                contexts: vec![MusicContext::Survival],
                min_delay: 180.0,
                max_delay: 600.0,
                weight: 1.0,
                rare: false,
            },
            // Nether music
            MusicTrack {
                event_id: "minecraft:music.nether".to_string(),
                contexts: vec![MusicContext::Nether],
                min_delay: 120.0,
                max_delay: 300.0,
                weight: 1.0,
                rare: false,
            },
            // End music
            MusicTrack {
                event_id: "minecraft:music.end".to_string(),
                contexts: vec![MusicContext::End],
                min_delay: 180.0,
                max_delay: 600.0,
                weight: 1.0,
                rare: false,
            },
        ];

        self.register_tracks(lce_tracks);
    }
}

impl Default for MusicScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_music_scheduler_creation() {
        let scheduler = MusicScheduler::new();
        assert_eq!(scheduler.current_context(), MusicContext::Menu);
        assert!(!scheduler.is_playing());
    }

    #[test]
    fn test_music_track_registration() {
        let mut scheduler = MusicScheduler::new();
        scheduler.register_track(MusicTrack {
            event_id: "test_track".to_string(),
            contexts: vec![MusicContext::Survival],
            ..Default::default()
        });

        assert_eq!(
            scheduler.tracks.get(&MusicContext::Survival).unwrap().len(),
            1
        );
    }

    #[test]
    fn test_context_change_forces_track_change() {
        let mut scheduler = MusicScheduler::new();
        scheduler.register_track(MusicTrack {
            event_id: "track1".to_string(),
            contexts: vec![MusicContext::Survival],
            ..Default::default()
        });
        scheduler.register_track(MusicTrack {
            event_id: "track2".to_string(),
            contexts: vec![MusicContext::Nether],
            ..Default::default()
        });

        scheduler.update_context(MusicContext::Survival);
        assert_eq!(scheduler.current_context(), MusicContext::Survival);

        scheduler.update_context(MusicContext::Nether);
        assert_eq!(scheduler.current_context(), MusicContext::Nether);
        // Force next track should be set
        assert!(scheduler.force_next_track.is_some());
    }
}
