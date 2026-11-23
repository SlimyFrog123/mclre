//! Audio engine with isolated thread and graceful degradation.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

use crossbeam_channel;
use kira::{
    AudioManager, AudioManagerSettings, Tween, backend::cpal::Error as CpalError,
    sound::static_sound::StaticSoundData,
};
use parking_lot::Mutex as ParkingMutex;

use crate::attenuation::AttenuationModel;
use crate::music::MusicScheduler;
use crate::registry::SoundEventRegistry;
use crate::sound_event::SoundCategory;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Failed to initialize audio backend: {0}")]
    BackendInit(String),
    #[error("Failed to load sound: {0}")]
    SoundLoad(String),
    #[error("Audio engine not initialized")]
    NotInitialized,
    #[error("Sound event not found: {0}")]
    SoundEventNotFound(String),
    #[error("Invalid listener index: {0}")]
    InvalidListenerIndex(usize),
}

#[derive(Debug, Clone)]
pub struct AudioEngineConfig {
    pub max_instances: usize,
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub enable_spatial: bool,
    pub num_listeners: usize,
}

impl Default for AudioEngineConfig {
    fn default() -> Self {
        Self {
            max_instances: 256,
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            enable_spatial: true,
            num_listeners: 1,
        }
    }
}

enum AudioCommand {
    PlaySound {
        event_id: String,
        position: Option<[f32; 3]>,
        listener_filter: ListenerFilter,
        volume: f32,
        pitch: f32,
    },
    SetMasterVolume {
        volume: f32,
    },
    SetCategoryVolume {
        category: SoundCategory,
        volume: f32,
    },
    UpdateMusic {
        context: crate::music::MusicContext,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenerFilter {
    All,
    Only(Vec<usize>),
    Except(Vec<usize>),
}
impl Default for ListenerFilter {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundHandle(pub u64);

#[derive(Clone)]
pub struct AudioEngineHandle {
    command_sender: crossbeam_channel::Sender<AudioCommand>,
    state: Arc<ParkingMutex<AudioEngineHandleState>>,
    next_handle_id: Arc<Mutex<u64>>,
}

struct AudioEngineHandleState {
    active_handles: Vec<SoundHandle>,
    next_handle_id: u64,
}

impl AudioEngineHandle {
    pub fn play_sound(
        &self,
        event_id: &str,
        position: Option<[f32; 3]>,
        listener_filter: ListenerFilter,
    ) -> Result<SoundHandle, AudioError> {
        let handle_id = {
            let mut s = self.state.lock();
            let id = s.next_handle_id;
            s.next_handle_id += 1;
            s.active_handles.push(SoundHandle(id));
            id
        };
        self.command_sender
            .send(AudioCommand::PlaySound {
                event_id: event_id.to_string(),
                position,
                listener_filter,
                volume: 1.0,
                pitch: 1.0,
            })
            .map_err(|_| AudioError::NotInitialized)?;
        Ok(SoundHandle(handle_id))
    }
    pub fn play_sound_with_params(
        &self,
        event_id: &str,
        position: Option<[f32; 3]>,
        listener_filter: ListenerFilter,
        volume: f32,
        pitch: f32,
    ) -> Result<SoundHandle, AudioError> {
        let handle_id = {
            let mut s = self.state.lock();
            let id = s.next_handle_id;
            s.next_handle_id += 1;
            s.active_handles.push(SoundHandle(id));
            id
        };
        self.command_sender
            .send(AudioCommand::PlaySound {
                event_id: event_id.to_string(),
                position,
                listener_filter,
                volume: volume.clamp(0.0, 1.0),
                pitch: pitch.clamp(0.5, 2.0),
            })
            .map_err(|_| AudioError::NotInitialized)?;
        Ok(SoundHandle(handle_id))
    }
    pub fn set_master_volume(&self, volume: f32) -> Result<(), AudioError> {
        self.command_sender
            .send(AudioCommand::SetMasterVolume {
                volume: volume.clamp(0.0, 1.0),
            })
            .map_err(|_| AudioError::NotInitialized)?;
        Ok(())
    }
    pub fn set_category_volume(
        &self,
        category: SoundCategory,
        volume: f32,
    ) -> Result<(), AudioError> {
        self.command_sender
            .send(AudioCommand::SetCategoryVolume {
                category,
                volume: volume.clamp(0.0, 1.0),
            })
            .map_err(|_| AudioError::NotInitialized)?;
        Ok(())
    }
    pub fn update_music(&self, context: crate::music::MusicContext) -> Result<(), AudioError> {
        self.command_sender
            .send(AudioCommand::UpdateMusic { context })
            .map_err(|_| AudioError::NotInitialized)?;
        Ok(())
    }
    pub fn shutdown(&self) {
        let _ = self.command_sender.send(AudioCommand::Shutdown);
    }
    pub fn active_sound_count(&self) -> usize {
        self.state.lock().active_handles.len()
    }
}

pub struct AudioEngine {
    handle: AudioEngineHandle,
    thread_handle: Option<thread::JoinHandle<()>>,
    initialized: AtomicBool,
}

impl AudioEngine {
    pub fn new(
        config: AudioEngineConfig,
        sound_registry: Arc<SoundEventRegistry>,
    ) -> Result<Self, AudioError> {
        let (tx, rx) = crossbeam_channel::bounded(256);
        let state = Arc::new(ParkingMutex::new(AudioEngineHandleState {
            active_handles: Vec::new(),
            next_handle_id: 1,
        }));
        let handle = AudioEngineHandle {
            command_sender: tx,
            state: state.clone(),
            next_handle_id: Arc::new(Mutex::new(1u64)),
        };
        let th = thread::Builder::new()
            .name("mclre-audio".to_string())
            .spawn(move || Self::audio_thread(rx, state, config, sound_registry))
            .map_err(|e| AudioError::BackendInit(format!("Failed to spawn audio thread: {}", e)))?;
        Ok(Self {
            handle,
            thread_handle: Some(th),
            initialized: AtomicBool::new(true),
        })
    }
    pub fn handle(&self) -> AudioEngineHandle {
        self.handle.clone()
    }
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.initialized.store(false, Ordering::Relaxed);
        let _ = self.handle.command_sender.send(AudioCommand::Shutdown);
        if let Some(t) = self.thread_handle.take() {
            let _ = t.join();
        }
    }
}

struct AudioEngineState {
    manager: AudioManager,
    sound_registry: Arc<SoundEventRegistry>,
    music_scheduler: MusicScheduler,
    attenuation_model: AttenuationModel,
    config: AudioEngineConfig,
}

impl AudioEngine {
    fn audio_thread(
        rx: crossbeam_channel::Receiver<AudioCommand>,
        hs: Arc<ParkingMutex<AudioEngineHandleState>>,
        cfg: AudioEngineConfig,
        reg: Arc<SoundEventRegistry>,
    ) {
        let mut es = match Self::init_audio_manager(&cfg) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Audio engine init failed: {}", e);
                Self::run_silent_mode(rx, hs);
                return;
            }
        };
        es.sound_registry = reg;
        es.config = cfg;
        tracing::info!("Audio engine started");
        'audio_loop: loop {
            while let Ok(c) = rx.try_recv() {
                match c {
                    AudioCommand::PlaySound {
                        event_id,
                        position,
                        listener_filter,
                        volume,
                        pitch,
                    } => Self::handle_play_sound(
                        &mut es,
                        event_id,
                        position,
                        listener_filter,
                        volume,
                        pitch,
                    ),
                    AudioCommand::SetMasterVolume { volume: _ } => {}
                    AudioCommand::SetCategoryVolume { category, volume } => {
                        Self::handle_set_cat_vol(&mut es, category, volume)
                    }
                    AudioCommand::UpdateMusic { context } => {
                        es.music_scheduler.update_context(context)
                    }
                    AudioCommand::Shutdown => break 'audio_loop,
                }
            }
            es.music_scheduler.tick(&mut es.manager);
            thread::sleep(Duration::from_millis(5));
        }
        tracing::info!("Audio engine shutting down");
    }

    fn init_audio_manager(c: &AudioEngineConfig) -> Result<AudioEngineState, AudioError> {
        let m = AudioManager::new(AudioManagerSettings::default())
            .map_err(|e: CpalError| AudioError::BackendInit(e.to_string()))?;
        // LCE legacy attenuation: ref_dist=16, max_dist=64, rolloff=1.0, min_vol=0.05
        Ok(AudioEngineState {
            manager: m,
            sound_registry: Arc::new(SoundEventRegistry::new()),
            music_scheduler: MusicScheduler::new(),
            attenuation_model: AttenuationModel::new(16.0, 64.0, 1.0, 0.05),
            config: c.clone(),
        })
    }

    fn run_silent_mode(
        rx: crossbeam_channel::Receiver<AudioCommand>,
        _hs: Arc<ParkingMutex<AudioEngineHandleState>>,
    ) {
        tracing::info!("Audio silent mode");
        loop {
            while rx.try_recv().is_ok() {}
            match rx.recv() {
                Ok(AudioCommand::Shutdown) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        tracing::info!("Silent mode shutting down");
    }

    fn handle_play_sound(
        st: &mut AudioEngineState,
        eid: String,
        _pos: Option<[f32; 3]>,
        _lf: ListenerFilter,
        vol: f32,
        pit: f32,
    ) {
        let ev = match st.sound_registry.get(&eid) {
            Some(e) => e,
            None => {
                tracing::warn!("Sound not found: {}", eid);
                return;
            }
        };
        let variant = if ev.variants.is_empty() {
            tracing::warn!("No variants: {}", eid);
            return;
        } else if ev.variants.len() == 1 {
            &ev.variants[0]
        } else {
            let total_weight: f32 = ev.variants.iter().map(|v| v.weight).sum();
            let mut r = fastrand::f32() * total_weight;
            for v in &ev.variants {
                r -= v.weight;
                if r <= 0.0 {
                    break;
                }
            }
            &ev.variants[0]
        };
        let sd = match StaticSoundData::from_file(&variant.path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Load failed: {}", e);
                return;
            }
        };
        let final_volume = vol * st.config.master_volume * st.config.sfx_volume;
        let mut handle = match st.manager.play(sd) {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("Play failed: {}", e);
                return;
            }
        };
        handle.set_volume(final_volume, Tween::default());
        handle.set_playback_rate(pit as f64, Tween::default());
    }

    fn handle_set_cat_vol(st: &mut AudioEngineState, cat: SoundCategory, vol: f32) {
        match cat {
            SoundCategory::Master => st.config.master_volume = vol,
            SoundCategory::Music => st.config.music_volume = vol,
            SoundCategory::Sfx => st.config.sfx_volume = vol,
            _ => {}
        }
    }
}
