//! DocDamage Engine - Audio System
//!
//! Audio mixer with kira 0.9.
//! Features:
//! - BGM (Background Music)
//! - SFX (Sound Effects)
//! - Volume controls
//! - Battle SFX integration

use std::collections::HashMap;
use std::path::PathBuf;

use kira::manager::{AudioManager, AudioManagerSettings};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::tween::Tween;

/// Audio system
pub struct AudioSystem {
    /// Kira audio manager
    manager: AudioManager,
    /// Currently playing BGM
    current_bgm: Option<StreamingSoundHandle<kira::sound::FromFileError>>,
    /// Loaded SFX sounds
    sfx_cache: HashMap<String, StaticSoundData>,
    /// Active SFX handles
    active_sfx: Vec<StaticSoundHandle>,
    /// BGM volume (0.0 - 1.0)
    bgm_volume: f64,
    /// SFX volume (0.0 - 1.0)
    sfx_volume: f64,
    /// Audio asset path
    asset_path: PathBuf,
}

/// Audio error types
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Kira error: {0}")]
    Kira(#[from] kira::manager::backend::cpal::Error),

    #[error("Sound decode error: {0}")]
    Decode(#[from] kira::sound::FromFileError),

    #[error("Sound not found: {0}")]
    SoundNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AudioError>;

impl AudioSystem {
    /// Create new audio system
    pub fn new() -> Result<Self> {
        let manager = AudioManager::new(AudioManagerSettings::default())?;

        tracing::info!("Audio system initialized");

        Ok(Self {
            manager,
            current_bgm: None,
            sfx_cache: HashMap::new(),
            active_sfx: Vec::new(),
            bgm_volume: 0.8,
            sfx_volume: 1.0,
            asset_path: PathBuf::from("assets/audio"),
        })
    }

    /// Set audio asset path
    pub fn set_asset_path(&mut self, path: impl Into<PathBuf>) {
        self.asset_path = path.into();
    }

    // ==================== BGM (Background Music) ====================

    /// Play BGM
    pub fn play_bgm(&mut self, name: &str) -> Result<()> {
        let path = self.asset_path.join("bgm").join(format!("{}.ogg", name));

        if !path.exists() {
            tracing::warn!("BGM file not found: {:?}", path);
            return Ok(());
        }

        // Stop current BGM if any
        self.stop_bgm();

        // Load and play new BGM
        let sound_data = StreamingSoundData::from_file(&path)?.loop_region(..);

        let handle = self
            .manager
            .play(sound_data)
            .map_err(|_| AudioError::SoundNotFound(name.to_string()))?;

        self.current_bgm = Some(handle);
        tracing::info!("Playing BGM: {}", name);

        Ok(())
    }

    /// Stop BGM
    pub fn stop_bgm(&mut self) {
        if let Some(mut bgm) = self.current_bgm.take() {
            bgm.stop(Tween::default());
        }
    }

    /// Pause BGM
    pub fn pause_bgm(&mut self) {
        if let Some(ref mut bgm) = self.current_bgm {
            bgm.pause(Tween::default());
        }
    }

    /// Resume BGM
    pub fn resume_bgm(&mut self) {
        if let Some(ref mut bgm) = self.current_bgm {
            bgm.resume(Tween::default());
        }
    }

    /// Set BGM volume
    pub fn set_bgm_volume(&mut self, volume: f64) {
        self.bgm_volume = volume.clamp(0.0, 1.0);
        if let Some(ref mut bgm) = self.current_bgm {
            bgm.set_volume(self.bgm_volume, Tween::default());
        }
    }

    // ==================== SFX (Sound Effects) ====================

    /// Preload SFX into cache
    pub fn preload_sfx(&mut self, name: &str) -> Result<()> {
        if self.sfx_cache.contains_key(name) {
            return Ok(());
        }

        let path = self.asset_path.join("sfx").join(format!("{}.ogg", name));

        if !path.exists() {
            tracing::warn!("SFX file not found: {:?}", path);
            return Ok(());
        }

        let sound_data = StaticSoundData::from_file(&path)?;
        self.sfx_cache.insert(name.to_string(), sound_data);
        tracing::debug!("Preloaded SFX: {}", name);

        Ok(())
    }

    /// Play SFX
    pub fn play_sfx(&mut self, name: &str) -> Result<()> {
        // Try to get from cache or load on demand
        let sound_data = if let Some(data) = self.sfx_cache.get(name) {
            data.clone()
        } else {
            let path = self.asset_path.join("sfx").join(format!("{}.ogg", name));
            if !path.exists() {
                tracing::warn!("SFX file not found: {:?}", path);
                return Ok(());
            }
            let data = StaticSoundData::from_file(&path)?;
            self.sfx_cache.insert(name.to_string(), data.clone());
            data
        };

        let handle = self
            .manager
            .play(sound_data)
            .map_err(|_| AudioError::SoundNotFound(name.to_string()))?;

        self.active_sfx.push(handle);

        Ok(())
    }

    /// Set SFX volume
    pub fn set_sfx_volume(&mut self, volume: f64) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }

    /// Get BGM volume
    pub fn bgm_volume(&self) -> f64 {
        self.bgm_volume
    }

    /// Get SFX volume
    pub fn sfx_volume(&self) -> f64 {
        self.sfx_volume
    }

    // ==================== Battle SFX ====================

    /// Play battle hit SFX
    pub fn play_hit(&mut self) -> Result<()> {
        self.play_sfx("hit")
    }

    /// Play battle miss SFX
    pub fn play_miss(&mut self) -> Result<()> {
        self.play_sfx("miss")
    }

    /// Play critical hit SFX
    pub fn play_critical(&mut self) -> Result<()> {
        self.play_sfx("critical")
    }

    /// Play skill SFX by type
    pub fn play_skill_sfx(&mut self, skill_type: &str) -> Result<()> {
        match skill_type {
            "fire" => self.play_sfx("fire"),
            "ice" => self.play_sfx("ice"),
            "thunder" => self.play_sfx("thunder"),
            "heal" => self.play_sfx("heal"),
            "buff" => self.play_sfx("buff"),
            _ => self.play_sfx("skill"),
        }
    }

    /// Play battle start SFX
    pub fn play_battle_start(&mut self) -> Result<()> {
        self.play_sfx("battle_start")
    }

    /// Play victory SFX
    pub fn play_victory(&mut self) -> Result<()> {
        self.play_sfx("victory")
    }

    /// Play defeat SFX
    pub fn play_defeat(&mut self) -> Result<()> {
        self.play_sfx("defeat")
    }

    // ==================== Update ====================

    /// Update audio system (call each frame)
    pub fn update(&mut self) {
        // Clean up finished SFX
        self.active_sfx
            .retain(|sfx| !matches!(sfx.state(), kira::sound::PlaybackState::Stopped));
    }

    /// Preload common SFX
    pub fn preload_common_sfx(&mut self) -> Result<()> {
        let common_sfx = [
            "hit",
            "miss",
            "critical",
            "fire",
            "ice",
            "thunder",
            "heal",
            "buff",
            "skill",
            "battle_start",
            "victory",
            "defeat",
            "cursor",
            "select",
            "cancel",
            "step",
        ];

        for sfx in &common_sfx {
            let _ = self.preload_sfx(sfx);
        }

        Ok(())
    }
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new().expect("Failed to initialize audio")
    }
}

/// Audio events for integration with game systems
#[derive(Debug, Clone)]
pub enum AudioEvent {
    PlayBgm { name: String },
    StopBgm,
    PlaySfx { name: String },
    SetBgmVolume(f64),
    SetSfxVolume(f64),
}

/// Audio event bus for decoupled audio triggering
pub struct AudioEventBus {
    events: Vec<AudioEvent>,
}

impl AudioEventBus {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, event: AudioEvent) {
        self.events.push(event);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = AudioEvent> + '_ {
        self.events.drain(..)
    }
}

impl Default for AudioEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== AudioSystem Creation Tests ====================

    /// Test that AudioSystem::new() succeeds when audio hardware is available
    #[test]
    #[ignore = "Requires audio hardware - run manually with: cargo test -p dde-audio -- --ignored"]
    fn test_audio_system_creation_success() {
        let result = AudioSystem::new();
        assert!(
            result.is_ok(),
            "AudioSystem::new() should succeed when audio hardware is available"
        );

        let audio = result.unwrap();
        // Verify default values
        assert_eq!(audio.bgm_volume(), 0.8);
        assert_eq!(audio.sfx_volume(), 1.0);
    }

    /// Test that Default trait works (requires audio hardware)
    #[test]
    #[ignore = "Requires audio hardware - run manually with: cargo test -p dde-audio -- --ignored"]
    fn test_audio_system_default() {
        let audio: AudioSystem = Default::default();
        assert_eq!(audio.bgm_volume(), 0.8);
        assert_eq!(audio.sfx_volume(), 1.0);
    }

    // ==================== Volume Control Tests ====================

    /// Test BGM volume control (valid range)
    #[test]
    #[ignore = "Requires audio hardware"]
    fn test_bgm_volume_control() {
        let mut audio = AudioSystem::new().expect("Failed to create audio system");

        // Test setting volume to various values
        audio.set_bgm_volume(0.5);
        assert_eq!(audio.bgm_volume(), 0.5);

        audio.set_bgm_volume(0.0);
        assert_eq!(audio.bgm_volume(), 0.0);

        audio.set_bgm_volume(1.0);
        assert_eq!(audio.bgm_volume(), 1.0);

        audio.set_bgm_volume(0.75);
        assert_eq!(audio.bgm_volume(), 0.75);
    }

    /// Test SFX volume control (valid range)
    #[test]
    #[ignore = "Requires audio hardware"]
    fn test_sfx_volume_control() {
        let mut audio = AudioSystem::new().expect("Failed to create audio system");

        // Test setting volume to various values
        audio.set_sfx_volume(0.5);
        assert_eq!(audio.sfx_volume(), 0.5);

        audio.set_sfx_volume(0.0);
        assert_eq!(audio.sfx_volume(), 0.0);

        audio.set_sfx_volume(1.0);
        assert_eq!(audio.sfx_volume(), 1.0);

        audio.set_sfx_volume(0.25);
        assert_eq!(audio.sfx_volume(), 0.25);
    }

    /// Test volume clamping (values outside 0.0-1.0 range)
    #[test]
    #[ignore = "Requires audio hardware"]
    fn test_volume_clamping() {
        let mut audio = AudioSystem::new().expect("Failed to create audio system");

        // Test BGM volume clamping
        audio.set_bgm_volume(-0.5);
        assert_eq!(
            audio.bgm_volume(),
            0.0,
            "Negative volume should be clamped to 0.0"
        );

        audio.set_bgm_volume(1.5);
        assert_eq!(
            audio.bgm_volume(),
            1.0,
            "Volume > 1.0 should be clamped to 1.0"
        );

        // Test SFX volume clamping
        audio.set_sfx_volume(-1.0);
        assert_eq!(
            audio.sfx_volume(),
            0.0,
            "Negative volume should be clamped to 0.0"
        );

        audio.set_sfx_volume(2.0);
        assert_eq!(
            audio.sfx_volume(),
            1.0,
            "Volume > 1.0 should be clamped to 1.0"
        );
    }

    /// Test independent BGM and SFX volume controls
    #[test]
    #[ignore = "Requires audio hardware"]
    fn test_independent_volume_controls() {
        let mut audio = AudioSystem::new().expect("Failed to create audio system");

        // Set different volumes for BGM and SFX
        audio.set_bgm_volume(0.3);
        audio.set_sfx_volume(0.7);

        assert_eq!(audio.bgm_volume(), 0.3);
        assert_eq!(audio.sfx_volume(), 0.7);

        // Changing BGM should not affect SFX
        audio.set_bgm_volume(0.9);
        assert_eq!(audio.bgm_volume(), 0.9);
        assert_eq!(audio.sfx_volume(), 0.7);

        // Changing SFX should not affect BGM
        audio.set_sfx_volume(0.4);
        assert_eq!(audio.bgm_volume(), 0.9);
        assert_eq!(audio.sfx_volume(), 0.4);
    }

    // ==================== Mute Toggle Tests ====================

    /// Test muting by setting volume to 0
    #[test]
    #[ignore = "Requires audio hardware"]
    fn test_mute_via_zero_volume() {
        let mut audio = AudioSystem::new().expect("Failed to create audio system");

        // Set initial volume
        audio.set_bgm_volume(0.8);
        audio.set_sfx_volume(0.6);

        // Mute by setting volume to 0
        audio.set_bgm_volume(0.0);
        audio.set_sfx_volume(0.0);

        assert_eq!(audio.bgm_volume(), 0.0);
        assert_eq!(audio.sfx_volume(), 0.0);
    }

    /// Test mute/unmute toggle pattern
    #[test]
    #[ignore = "Requires audio hardware"]
    fn test_mute_unmute_toggle_pattern() {
        let mut audio = AudioSystem::new().expect("Failed to create audio system");

        // Initial volume
        audio.set_bgm_volume(0.8);
        audio.set_sfx_volume(1.0);

        // Store previous volume before muting
        let prev_bgm_vol = audio.bgm_volume();
        let prev_sfx_vol = audio.sfx_volume();

        // Mute
        audio.set_bgm_volume(0.0);
        audio.set_sfx_volume(0.0);
        assert!(audio.bgm_volume() == 0.0 && audio.sfx_volume() == 0.0);

        // Unmute (restore previous volume)
        audio.set_bgm_volume(prev_bgm_vol);
        audio.set_sfx_volume(prev_sfx_vol);
        assert_eq!(audio.bgm_volume(), 0.8);
        assert_eq!(audio.sfx_volume(), 1.0);
    }

    /// Test partial mute (only BGM or only SFX)
    #[test]
    #[ignore = "Requires audio hardware"]
    fn test_partial_mute() {
        let mut audio = AudioSystem::new().expect("Failed to create audio system");

        // Mute only BGM
        audio.set_bgm_volume(0.0);
        assert_eq!(audio.bgm_volume(), 0.0);
        assert_eq!(audio.sfx_volume(), 1.0); // SFX should remain at default

        // Restore and mute only SFX
        audio.set_bgm_volume(0.8);
        audio.set_sfx_volume(0.0);
        assert_eq!(audio.bgm_volume(), 0.8);
        assert_eq!(audio.sfx_volume(), 0.0);
    }

    // ==================== AudioEventBus Tests ====================

    /// Test AudioEventBus creation
    #[test]
    fn test_audio_event_bus_creation() {
        let bus = AudioEventBus::new();
        // Event bus should be empty initially
        let events: Vec<_> = bus.events.clone();
        assert!(events.is_empty());
    }

    /// Test pushing events to the bus
    #[test]
    fn test_audio_event_bus_push() {
        let mut bus = AudioEventBus::new();

        bus.push(AudioEvent::PlayBgm {
            name: "test".to_string(),
        });
        bus.push(AudioEvent::SetBgmVolume(0.5));
        bus.push(AudioEvent::PlaySfx {
            name: "hit".to_string(),
        });

        assert_eq!(bus.events.len(), 3);
    }

    /// Test draining events from the bus
    #[test]
    fn test_audio_event_bus_drain() {
        let mut bus = AudioEventBus::new();

        bus.push(AudioEvent::PlayBgm {
            name: "battle".to_string(),
        });
        bus.push(AudioEvent::StopBgm);
        bus.push(AudioEvent::SetSfxVolume(0.7));

        // Drain all events
        let drained: Vec<_> = bus.drain().collect();
        assert_eq!(drained.len(), 3);

        // Bus should be empty after drain
        assert!(bus.events.is_empty());

        // Verify event types
        matches!(drained[0], AudioEvent::PlayBgm { .. });
        matches!(drained[1], AudioEvent::StopBgm);
        matches!(drained[2], AudioEvent::SetSfxVolume(0.7));
    }

    /// Test AudioEventBus Default implementation
    #[test]
    fn test_audio_event_bus_default() {
        let bus: AudioEventBus = Default::default();
        assert!(bus.events.is_empty());
    }

    /// Test multiple drain cycles
    #[test]
    fn test_audio_event_bus_multiple_drains() {
        let mut bus = AudioEventBus::new();

        // First batch
        bus.push(AudioEvent::PlaySfx {
            name: "a".to_string(),
        });
        let first_drain: Vec<_> = bus.drain().collect();
        assert_eq!(first_drain.len(), 1);

        // Second batch (after first drain)
        bus.push(AudioEvent::PlaySfx {
            name: "b".to_string(),
        });
        bus.push(AudioEvent::PlaySfx {
            name: "c".to_string(),
        });
        let second_drain: Vec<_> = bus.drain().collect();
        assert_eq!(second_drain.len(), 2);

        // Third drain should be empty
        let third_drain: Vec<_> = bus.drain().collect();
        assert!(third_drain.is_empty());
    }

    // ==================== AudioEvent Tests ====================

    /// Test AudioEvent variants can be created and cloned
    #[test]
    fn test_audio_event_variants() {
        let play_bgm = AudioEvent::PlayBgm {
            name: "theme".to_string(),
        };
        let stop_bgm = AudioEvent::StopBgm;
        let play_sfx = AudioEvent::PlaySfx {
            name: "hit".to_string(),
        };
        let set_bgm_vol = AudioEvent::SetBgmVolume(0.5);
        let set_sfx_vol = AudioEvent::SetSfxVolume(0.8);

        // Test Clone
        let _ = play_bgm.clone();
        let _ = stop_bgm.clone();
        let _ = play_sfx.clone();
        let _ = set_bgm_vol.clone();
        let _ = set_sfx_vol.clone();

        // Test Debug
        let _ = format!("{:?}", play_bgm);
        let _ = format!("{:?}", stop_bgm);
        let _ = format!("{:?}", play_sfx);
        let _ = format!("{:?}", set_bgm_vol);
        let _ = format!("{:?}", set_sfx_vol);
    }
}
