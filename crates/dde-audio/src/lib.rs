//! DocDamage Engine - Audio System
//! 
//! Stem-based audio mixer with kira.

use kira::manager::{AudioManager, AudioManagerSettings};

/// Audio system
pub struct Audio {
    manager: AudioManager,
}

impl Audio {
    pub fn new() -> anyhow::Result<Self> {
        let manager = AudioManager::new(AudioManagerSettings::default())?;
        
        tracing::info!("Audio system initialized");
        
        Ok(Self { manager })
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new().expect("Failed to initialize audio")
    }
}
