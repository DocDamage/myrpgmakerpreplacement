//! Deterministic Replay System
//!
//! Records and replays gameplay deterministically using the existing RngPool.
//! Replays capture initial world state, RNG seed, and all player inputs per tick.

use crate::resources::RngPool;
use crate::serialization::WorldSnapshot;
use crate::{Entity, World};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use thiserror::Error;

/// Current replay format version
pub const REPLAY_VERSION: &str = "1.0.0";

/// File extension for replay files
pub const REPLAY_FILE_EXTENSION: &str = ".ddr";

/// A replay recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay {
    /// Initial world seed
    pub seed: u64,
    /// Initial game state snapshot
    pub initial_state: WorldSnapshot,
    /// Recorded inputs per tick
    pub inputs: Vec<TickInputs>,
    /// Total duration in ticks
    pub total_ticks: u64,
    /// Metadata
    pub metadata: ReplayMetadata,
}

impl Replay {
    /// Create a new replay with the given seed and initial state
    pub fn new(seed: u64, initial_state: WorldSnapshot) -> Self {
        Self {
            seed,
            initial_state,
            inputs: Vec::new(),
            total_ticks: 0,
            metadata: ReplayMetadata::default(),
        }
    }

    /// Serialize to compact binary format (using JSON for now, bincode optional)
    pub fn to_bytes(&self) -> Result<Vec<u8>, ReplayError> {
        serde_json::to_vec(self).map_err(ReplayError::Serialization)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplayError> {
        serde_json::from_slice(bytes).map_err(ReplayError::Serialization)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, ReplayError> {
        serde_json::to_string_pretty(self).map_err(ReplayError::Serialization)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, ReplayError> {
        serde_json::from_str(json).map_err(ReplayError::Serialization)
    }

    /// Get file extension
    pub fn file_extension() -> &'static str {
        REPLAY_FILE_EXTENSION
    }

    /// Get duration in seconds (based on 20 ticks per second)
    pub fn duration_secs(&self) -> f64 {
        self.total_ticks as f64 * 0.05
    }

    /// Add metadata to the replay
    pub fn with_metadata(mut self, metadata: ReplayMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Get inputs for a specific tick
    pub fn get_inputs_for_tick(&self, tick: u64) -> Option<&[PlayerInput]> {
        self.inputs
            .iter()
            .find(|ti| ti.tick == tick)
            .map(|ti| ti.inputs.as_slice())
    }
}

/// Inputs recorded for a single tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickInputs {
    pub tick: u64,
    pub inputs: Vec<PlayerInput>,
}

impl TickInputs {
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            inputs: Vec::new(),
        }
    }

    pub fn with_input(mut self, input: PlayerInput) -> Self {
        self.inputs.push(input);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }
}

/// Player input types that can be recorded and replayed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlayerInput {
    /// Move in a direction
    Move { direction: crate::Direction4 },
    /// Interact with target entity (stored as bits for serialization)
    Interact { target: Option<u64> },
    /// Use a skill (target stored as bits for serialization)
    UseSkill { skill_id: u32, target: Option<u64> },
    /// Use an item (target stored as bits for serialization)
    UseItem { item_id: u32, target: Option<u64> },
    /// Open menu
    MenuOpen,
    /// Close menu
    MenuClose,
    /// Select menu option
    MenuSelect { option: u32 },
    /// Make a dialogue choice
    DialogueChoice { choice_index: u32 },
    /// Battle action input
    BattleAction { action: BattleInput },
    /// Confirm/action button
    Confirm,
    /// Cancel/back button
    Cancel,
    /// Run/sprint toggle
    RunToggle,
}

/// Battle-specific input types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BattleInput {
    /// Basic attack (target stored as bits for serialization)
    Attack { target: u64 },
    /// Use skill (target stored as bits for serialization)
    Skill { skill_id: u32, target: Option<u64> },
    /// Use item (target stored as bits for serialization)
    Item { item_id: u32, target: Option<u64> },
    /// Defend/guard
    Defend,
    /// Attempt to flee
    Flee,
}

impl BattleInput {
    /// Create an Attack input from an Entity
    pub fn attack(target: Entity) -> Self {
        Self::Attack {
            target: target.to_bits().get(),
        }
    }

    /// Create a Skill input from an Entity
    pub fn skill(skill_id: u32, target: Option<Entity>) -> Self {
        Self::Skill {
            skill_id,
            target: target.map(|e| e.to_bits().get()),
        }
    }

    /// Create an Item input from an Entity
    pub fn item(item_id: u32, target: Option<Entity>) -> Self {
        Self::Item {
            item_id,
            target: target.map(|e| e.to_bits().get()),
        }
    }

    /// Get target entity bits from battle input, if any
    pub fn target_bits(&self) -> Option<u64> {
        match self {
            Self::Attack { target } => Some(*target),
            Self::Skill { target, .. } => *target,
            Self::Item { target, .. } => *target,
            _ => None,
        }
    }

    /// Get target entity from battle input, if any
    pub fn target_entity(&self) -> Option<Entity> {
        self.target_bits().and_then(Entity::from_bits)
    }
}

impl PlayerInput {
    /// Create an Interact input from an Entity
    pub fn interact(target: Option<Entity>) -> Self {
        Self::Interact {
            target: target.map(|e| e.to_bits().get()),
        }
    }

    /// Create a UseSkill input from an Entity
    pub fn use_skill(skill_id: u32, target: Option<Entity>) -> Self {
        Self::UseSkill {
            skill_id,
            target: target.map(|e| e.to_bits().get()),
        }
    }

    /// Create a UseItem input from an Entity
    pub fn use_item(item_id: u32, target: Option<Entity>) -> Self {
        Self::UseItem {
            item_id,
            target: target.map(|e| e.to_bits().get()),
        }
    }

    /// Get target entity bits from input, if any
    pub fn target_bits(&self) -> Option<u64> {
        match self {
            Self::Interact { target } => *target,
            Self::UseSkill { target, .. } => *target,
            Self::UseItem { target, .. } => *target,
            Self::BattleAction { action } => action.target_bits(),
            _ => None,
        }
    }

    /// Get target entity from input, if any
    pub fn target_entity(&self) -> Option<Entity> {
        self.target_bits().and_then(Entity::from_bits)
    }
}

/// Replay metadata for display and organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayMetadata {
    /// Replay format version
    pub version: String,
    /// Creation timestamp (Unix milliseconds)
    pub created_at: i64,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Player name
    pub player_name: String,
    /// Map name where recording started
    pub map_name: String,
    /// Optional description
    pub description: Option<String>,
    /// Engine version
    pub engine_version: String,
}

impl Default for ReplayMetadata {
    fn default() -> Self {
        Self {
            version: REPLAY_VERSION.to_string(),
            created_at: chrono::Utc::now().timestamp_millis(),
            duration_secs: 0.0,
            player_name: "Player".to_string(),
            map_name: "Unknown".to_string(),
            description: None,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl ReplayMetadata {
    /// Create new metadata with the given player and map names
    pub fn new(player_name: impl Into<String>, map_name: impl Into<String>) -> Self {
        Self {
            player_name: player_name.into(),
            map_name: map_name.into(),
            ..Default::default()
        }
    }

    /// Add a description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Format creation time as human-readable string
    pub fn formatted_time(&self) -> String {
        chrono::DateTime::from_timestamp_millis(self.created_at)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Format duration as MM:SS
    pub fn formatted_duration(&self) -> String {
        let mins = (self.duration_secs as u64) / 60;
        let secs = (self.duration_secs as u64) % 60;
        format!("{:02}:{:02}", mins, secs)
    }
}

/// Replay recording session
pub struct ReplayRecorder {
    replay: Replay,
    current_tick_inputs: Vec<PlayerInput>,
    current_tick: u64,
    #[allow(dead_code)]
    rng_pool: RngPool,
    is_recording: bool,
}

impl ReplayRecorder {
    /// Start recording from current state
    pub fn new(seed: u64, initial_state: WorldSnapshot) -> Self {
        Self {
            replay: Replay::new(seed, initial_state),
            current_tick_inputs: Vec::new(),
            current_tick: 0,
            rng_pool: RngPool::from_seed(seed),
            is_recording: true,
        }
    }

    /// Record an input for current tick
    pub fn record_input(&mut self, input: PlayerInput) {
        if self.is_recording {
            self.current_tick_inputs.push(input);
        }
    }

    /// Record multiple inputs for current tick
    pub fn record_inputs(&mut self, inputs: impl IntoIterator<Item = PlayerInput>) {
        if self.is_recording {
            self.current_tick_inputs.extend(inputs);
        }
    }

    /// Advance to next tick, storing accumulated inputs
    pub fn advance_tick(&mut self) {
        if !self.is_recording {
            return;
        }

        // Store inputs for the tick that just completed
        let tick_inputs = TickInputs {
            tick: self.current_tick,
            inputs: std::mem::take(&mut self.current_tick_inputs),
        };

        // Only store ticks that have inputs (optimization)
        if !tick_inputs.is_empty() {
            self.replay.inputs.push(tick_inputs);
        }

        self.current_tick += 1;
        self.replay.total_ticks = self.current_tick;
    }

    /// Stop recording and return the replay
    pub fn finish(mut self) -> Replay {
        self.is_recording = false;

        // Store any remaining inputs for the current tick
        if !self.current_tick_inputs.is_empty() {
            let tick_inputs = TickInputs {
                tick: self.current_tick,
                inputs: std::mem::take(&mut self.current_tick_inputs),
            };
            self.replay.inputs.push(tick_inputs);
        }

        // Update metadata
        self.replay.metadata.duration_secs = self.replay.duration_secs();

        self.replay
    }

    /// Get current tick count
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Pause recording (inputs will be ignored but tick counter continues)
    pub fn pause(&mut self) {
        self.is_recording = false;
    }

    /// Resume recording
    pub fn resume(&mut self) {
        self.is_recording = true;
    }

    /// Save replay to file
    pub fn save_to_file(&self, path: &Path) -> Result<(), ReplayError> {
        if self.is_recording {
            return Err(ReplayError::StillRecording);
        }

        let bytes = self.replay.to_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Get reference to the replay being recorded
    pub fn replay(&self) -> &Replay {
        &self.replay
    }

    /// Get mutable reference to metadata
    pub fn metadata_mut(&mut self) -> &mut ReplayMetadata {
        &mut self.replay.metadata
    }
}

/// Replay playback session
pub struct ReplayPlayer {
    replay: Replay,
    current_tick: u64,
    rng_pool: RngPool,
    is_playing: bool,
    playback_speed: f32,
    input_queue: VecDeque<TickInputs>,
    last_inputs: Vec<PlayerInput>,
}

impl ReplayPlayer {
    /// Create a new replay player from a replay
    pub fn new(replay: Replay) -> Self {
        let rng_pool = RngPool::from_seed(replay.seed);
        let input_queue: VecDeque<TickInputs> = replay.inputs.clone().into();

        Self {
            replay,
            current_tick: 0,
            rng_pool,
            is_playing: false,
            playback_speed: 1.0,
            input_queue,
            last_inputs: Vec::new(),
        }
    }

    /// Load replay from file
    pub fn from_file(path: &Path) -> Result<Self, ReplayError> {
        let bytes = std::fs::read(path)?;
        let replay = Replay::from_bytes(&bytes)?;
        Ok(Self::new(replay))
    }

    /// Start playback
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Stop and reset to beginning
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_tick = 0;
        self.rng_pool = RngPool::from_seed(self.replay.seed);
        self.input_queue = self.replay.inputs.clone().into();
        self.last_inputs.clear();
    }

    /// Seek to specific tick
    /// Note: This resets and replays from start for determinism
    pub fn seek_to(&mut self, tick: u64) -> Result<(), ReplayError> {
        if tick > self.replay.total_ticks {
            return Err(ReplayError::TickOutOfRange {
                tick,
                max: self.replay.total_ticks,
            });
        }

        // Reset to beginning
        self.stop();

        // Advance to target tick without processing
        // In a real implementation, you'd want to use checkpoints for efficiency
        while self.current_tick < tick {
            self.next_tick_internal();
        }

        Ok(())
    }

    /// Step forward one tick (when paused)
    pub fn step_forward(&mut self) -> Option<Vec<PlayerInput>> {
        if self.is_playing {
            return None;
        }
        self.next_tick_internal()
    }

    /// Step backward one tick (requires checkpoints - simplified version)
    /// Note: This resets and replays from start to target tick
    pub fn step_backward(&mut self) -> Option<Vec<PlayerInput>> {
        if self.is_playing || self.current_tick == 0 {
            return None;
        }

        let target_tick = self.current_tick.saturating_sub(1);
        self.seek_to(target_tick).ok()?;
        Some(self.last_inputs.clone())
    }

    /// Set playback speed (0.5 = half, 2.0 = double)
    pub fn set_speed(&mut self, speed: f32) {
        self.playback_speed = speed.clamp(0.1, 10.0);
    }

    /// Get current playback speed
    pub fn playback_speed(&self) -> f32 {
        self.playback_speed
    }

    /// Get inputs for current tick and advance
    pub fn next_tick(&mut self) -> Option<Vec<PlayerInput>> {
        if !self.is_playing {
            return None;
        }
        self.next_tick_internal()
    }

    /// Internal tick advancement
    fn next_tick_internal(&mut self) -> Option<Vec<PlayerInput>> {
        if self.current_tick >= self.replay.total_ticks {
            self.is_playing = false;
            return None;
        }

        // Get inputs for this tick
        let inputs = if let Some(tick_inputs) = self.input_queue.front() {
            if tick_inputs.tick == self.current_tick {
                let inputs = self.input_queue.pop_front().unwrap().inputs;
                self.last_inputs = inputs.clone();
                inputs
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        self.current_tick += 1;
        Some(inputs)
    }

    /// Check if replay is finished
    pub fn is_finished(&self) -> bool {
        self.current_tick >= self.replay.total_ticks
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Get current progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.replay.total_ticks == 0 {
            0.0
        } else {
            self.current_tick as f32 / self.replay.total_ticks as f32
        }
    }

    /// Get current tick
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get total ticks
    pub fn total_ticks(&self) -> u64 {
        self.replay.total_ticks
    }

    /// Get reference to replay
    pub fn replay(&self) -> &Replay {
        &self.replay
    }

    /// Get mutable RNG pool for simulation
    pub fn rng_pool(&mut self) -> &mut RngPool {
        &mut self.rng_pool
    }
}

/// Replay state for UI and game loop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayState {
    /// Currently recording
    Recording,
    /// Playing back a replay
    Playing,
    /// Paused (either recording or playback)
    Paused,
    /// Replay finished
    Finished,
    /// No active replay operation
    Inactive,
}

/// Checkpoint for rewind capability
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub tick: u64,
    pub world_hash: u64,
    pub snapshot: WorldSnapshot,
    pub rng_state: u64, // Seed + tick offset for RngPool reconstruction
}

/// Checkpoint system for efficient seeking and rewind
pub struct CheckpointSystem {
    checkpoints: Vec<Checkpoint>,
    interval_ticks: u64,
    max_checkpoints: usize,
}

impl CheckpointSystem {
    /// Create a new checkpoint system
    pub fn new(interval_ticks: u64, max_checkpoints: usize) -> Self {
        Self {
            checkpoints: Vec::new(),
            interval_ticks: interval_ticks.max(1),
            max_checkpoints,
        }
    }

    /// Create checkpoint every N ticks
    pub fn maybe_create_checkpoint(&mut self, tick: u64, world: &World, seed: u64) {
        if tick % self.interval_ticks == 0 {
            let snapshot = crate::serialization::WorldSerializer::serialize(world, seed, tick);
            let hash = compute_world_hash(world);

            let checkpoint = Checkpoint {
                tick,
                world_hash: hash,
                snapshot,
                rng_state: seed.wrapping_add(tick),
            };

            self.checkpoints.push(checkpoint);

            // Remove old checkpoints if we exceed max
            if self.checkpoints.len() > self.max_checkpoints {
                // Keep first checkpoint (tick 0) and remove oldest after that
                if self.checkpoints.len() > 1 {
                    self.checkpoints.remove(1);
                }
            }
        }
    }

    /// Find nearest checkpoint before or at the given tick
    pub fn find_nearest(&self, tick: u64) -> Option<&Checkpoint> {
        self.checkpoints
            .iter()
            .filter(|c| c.tick <= tick)
            .max_by_key(|c| c.tick)
    }

    /// Get all checkpoints
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    /// Clear all checkpoints
    pub fn clear(&mut self) {
        self.checkpoints.clear();
    }

    /// Get checkpoint count
    pub fn len(&self) -> usize {
        self.checkpoints.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.checkpoints.is_empty()
    }
}

impl Default for CheckpointSystem {
    fn default() -> Self {
        Self::new(600, 100) // Every 30 seconds at 20 TPS, max 100 checkpoints
    }
}

/// Compute a hash of the world state for determinism verification
fn compute_world_hash(world: &World) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // Hash entity count
    world.iter().count().hash(&mut hasher);

    // Hash component data for each entity
    // This is a simplified version - a real implementation would hash all relevant components
    for (entity, ()) in world.query::<()>().iter() {
        entity.to_bits().hash(&mut hasher);
    }

    hasher.finish()
}

/// Replay-related errors
#[derive(Debug, Error)]
pub enum ReplayError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cannot save while still recording")]
    StillRecording,

    #[error("Tick {tick} out of range (max: {max})")]
    TickOutOfRange { tick: u64, max: u64 },

    #[error("Replay verification failed: {0}")]
    VerificationFailed(String),

    #[error("Invalid replay format: {0}")]
    InvalidFormat(String),

    #[error("Replay version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },
}

/// Result type for replay operations
pub type ReplayResult<T> = Result<T, ReplayError>;

/// Verify that a replay produces the same results
pub fn verify_replay(
    replay: &Replay,
    world: &mut World,
    mut simulator: impl FnMut(&mut World, &mut RngPool, Vec<PlayerInput>),
    mut verifier: impl FnMut(&World) -> VerificationData,
) -> ReplayResult<()> {
    // Clear and reset world to initial state
    world.clear();
    crate::serialization::WorldSerializer::deserialize(world, &replay.initial_state);

    // Recreate RNG with same seed
    let mut rng_pool = RngPool::from_seed(replay.seed);

    // Track verification data per tick
    let mut expected_verifications: Vec<(u64, VerificationData)> = Vec::new();

    // Replay all inputs and collect verification data
    for tick_input in &replay.inputs {
        // Run simulation for this tick
        simulator(world, &mut rng_pool, tick_input.inputs.clone());

        // Collect verification data
        let data = verifier(world);
        expected_verifications.push((tick_input.tick, data));
    }

    // Now run again and verify
    world.clear();
    crate::serialization::WorldSerializer::deserialize(world, &replay.initial_state);
    let mut rng_pool = RngPool::from_seed(replay.seed);

    for (tick, expected_data) in expected_verifications {
        let tick_inputs = replay
            .get_inputs_for_tick(tick)
            .map(|inputs| inputs.to_vec())
            .unwrap_or_default();

        simulator(world, &mut rng_pool, tick_inputs);

        let actual_data = verifier(world);

        if expected_data.world_hash != actual_data.world_hash {
            return Err(ReplayError::VerificationFailed(format!(
                "Desync at tick {}: hash mismatch (expected {}, got {})",
                tick, expected_data.world_hash, actual_data.world_hash
            )));
        }
    }

    Ok(())
}

/// Data for verification comparison
#[derive(Debug, Clone)]
pub struct VerificationData {
    pub world_hash: u64,
    pub entity_count: usize,
    pub tick: u64,
}

impl VerificationData {
    pub fn new(world: &World, tick: u64) -> Self {
        Self {
            world_hash: compute_world_hash(world),
            entity_count: world.iter().count(),
            tick,
        }
    }
}

/// Game loop integration for replay functionality
pub struct ReplayGameLoop {
    replay_recorder: Option<ReplayRecorder>,
    replay_player: Option<ReplayPlayer>,
    state: ReplayState,
    checkpoint_system: Option<CheckpointSystem>,
}

impl Default for ReplayGameLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayGameLoop {
    /// Create a new replay-aware game loop helper
    pub fn new() -> Self {
        Self {
            replay_recorder: None,
            replay_player: None,
            state: ReplayState::Inactive,
            checkpoint_system: None,
        }
    }

    /// Start recording
    pub fn start_recording(&mut self, seed: u64, world: &World) {
        let snapshot = crate::serialization::WorldSerializer::serialize(world, seed, 0);
        self.replay_recorder = Some(ReplayRecorder::new(seed, snapshot));
        self.replay_player = None;
        self.state = ReplayState::Recording;
        self.checkpoint_system = Some(CheckpointSystem::default());
    }

    /// Stop recording and return the replay
    pub fn stop_recording(&mut self) -> Option<Replay> {
        self.state = ReplayState::Inactive;
        self.checkpoint_system = None;
        self.replay_recorder.take().map(|r| r.finish())
    }

    /// Start playing replay
    pub fn start_replay(&mut self, replay: Replay) {
        self.replay_recorder = None;
        let mut player = ReplayPlayer::new(replay);
        player.play();
        self.replay_player = Some(player);
        self.state = ReplayState::Playing;
    }

    /// Stop replay playback
    pub fn stop_replay(&mut self) {
        self.replay_player = None;
        self.state = ReplayState::Inactive;
    }

    /// Pause current operation
    pub fn pause(&mut self) {
        match self.state {
            ReplayState::Recording => {
                if let Some(recorder) = &mut self.replay_recorder {
                    recorder.pause();
                }
                self.state = ReplayState::Paused;
            }
            ReplayState::Playing => {
                if let Some(player) = &mut self.replay_player {
                    player.pause();
                }
                self.state = ReplayState::Paused;
            }
            _ => {}
        }
    }

    /// Resume current operation
    pub fn resume(&mut self) {
        if self.state == ReplayState::Paused {
            if self.replay_player.is_some() {
                if let Some(player) = &mut self.replay_player {
                    player.play();
                }
                self.state = ReplayState::Playing;
            } else if self.replay_recorder.is_some() {
                if let Some(recorder) = &mut self.replay_recorder {
                    recorder.resume();
                }
                self.state = ReplayState::Recording;
            }
        }
    }

    /// Get inputs for current frame
    /// Returns replay inputs if playing, None if recording or inactive
    pub fn get_inputs(&mut self) -> Option<Vec<PlayerInput>> {
        if let Some(player) = &mut self.replay_player {
            player.next_tick()
        } else {
            None
        }
    }

    /// Record inputs for current frame (call this with player inputs)
    pub fn record_inputs(&mut self, inputs: Vec<PlayerInput>) {
        if let Some(recorder) = &mut self.replay_recorder {
            for input in inputs {
                recorder.record_input(input);
            }
            recorder.advance_tick();
        }

        // Update checkpoint system if active
        // Note: Actual checkpoint creation requires world reference, use create_checkpoint() instead
        let _ = &self.checkpoint_system;
        let _ = &self.replay_recorder;
    }

    /// Create a checkpoint (call this from your game loop with world reference)
    pub fn create_checkpoint(&mut self, tick: u64, world: &World, seed: u64) {
        if let Some(checkpoints) = &mut self.checkpoint_system {
            checkpoints.maybe_create_checkpoint(tick, world, seed);
        }
    }

    /// Get current replay state
    pub fn state(&self) -> ReplayState {
        self.state
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        matches!(self.state, ReplayState::Recording)
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        matches!(self.state, ReplayState::Playing)
    }

    /// Get mutable recorder reference
    pub fn recorder_mut(&mut self) -> Option<&mut ReplayRecorder> {
        self.replay_recorder.as_mut()
    }

    /// Get mutable player reference
    pub fn player_mut(&mut self) -> Option<&mut ReplayPlayer> {
        self.replay_player.as_mut()
    }

    /// Get recorder reference
    pub fn recorder(&self) -> Option<&ReplayRecorder> {
        self.replay_recorder.as_ref()
    }

    /// Get player reference
    pub fn player(&self) -> Option<&ReplayPlayer> {
        self.replay_player.as_ref()
    }

    /// Get progress (0.0 to 1.0) if playing
    pub fn progress(&self) -> Option<f32> {
        self.replay_player.as_ref().map(|p| p.progress())
    }

    /// Seek to specific tick (if playing)
    pub fn seek_to(&mut self, tick: u64) -> ReplayResult<()> {
        if let Some(player) = &mut self.replay_player {
            player.seek_to(tick)?;
            Ok(())
        } else {
            Err(ReplayError::InvalidFormat(
                "Cannot seek - no active replay".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{WorldSerializer, WorldSnapshot};
    use crate::World;

    fn create_test_world() -> (World, u64) {
        let world = World::new();
        let seed = 12345u64;
        (world, seed)
    }

    #[test]
    fn test_replay_creation() {
        let (_, seed) = create_test_world();
        let snapshot = WorldSnapshot::new(seed);
        let replay = Replay::new(seed, snapshot);

        assert_eq!(replay.seed, seed);
        assert!(replay.inputs.is_empty());
        assert_eq!(replay.total_ticks, 0);
    }

    #[test]
    fn test_replay_recorder() {
        let (world, seed) = create_test_world();
        let snapshot = WorldSerializer::serialize(&world, seed, 0);
        let mut recorder = ReplayRecorder::new(seed, snapshot);

        // Record some inputs
        recorder.record_input(PlayerInput::Move {
            direction: crate::Direction4::Up,
        });
        recorder.advance_tick();

        recorder.record_input(PlayerInput::Move {
            direction: crate::Direction4::Right,
        });
        recorder.record_input(PlayerInput::Confirm);
        recorder.advance_tick();

        assert_eq!(recorder.current_tick(), 2);

        let replay = recorder.finish();
        assert_eq!(replay.inputs.len(), 2);
        assert_eq!(replay.total_ticks, 2);
    }

    #[test]
    fn test_replay_player() {
        let (world, seed) = create_test_world();
        let snapshot = WorldSerializer::serialize(&world, seed, 0);
        let mut recorder = ReplayRecorder::new(seed, snapshot);

        // Record 5 ticks of inputs
        for i in 0..5 {
            if i % 2 == 0 {
                recorder.record_input(PlayerInput::Move {
                    direction: crate::Direction4::Up,
                });
            }
            recorder.advance_tick();
        }

        let replay = recorder.finish();
        assert_eq!(replay.total_ticks, 5);

        // Playback
        let mut player = ReplayPlayer::new(replay);
        assert!(!player.is_playing());
        assert_eq!(player.current_tick(), 0);

        player.play();
        assert!(player.is_playing());

        // Step through all ticks
        let mut tick_count = 0;
        while let Some(inputs) = player.next_tick() {
            tick_count += 1;
            // Odd ticks should have no inputs
            if tick_count % 2 == 1 && tick_count <= 5 {
                assert!(!inputs.is_empty());
            }
        }

        assert_eq!(tick_count, 5);
        assert!(player.is_finished());
        assert_eq!(player.progress(), 1.0);
    }

    #[test]
    fn test_replay_serialization_roundtrip() {
        let (world, seed) = create_test_world();
        let snapshot = WorldSerializer::serialize(&world, seed, 0);
        let mut recorder = ReplayRecorder::new(seed, snapshot);

        recorder.record_input(PlayerInput::Move {
            direction: crate::Direction4::Down,
        });
        recorder.advance_tick();

        let replay = recorder.finish();

        // Serialize
        let bytes = replay.to_bytes().unwrap();
        let json = replay.to_json().unwrap();

        // Deserialize
        let replay_from_bytes = Replay::from_bytes(&bytes).unwrap();
        let replay_from_json = Replay::from_json(&json).unwrap();

        assert_eq!(replay_from_bytes.seed, replay.seed);
        assert_eq!(replay_from_bytes.total_ticks, replay.total_ticks);
        assert_eq!(replay_from_json.seed, replay.seed);
        assert_eq!(replay_from_json.total_ticks, replay.total_ticks);
    }

    #[test]
    fn test_checkpoint_system() {
        let mut checkpoints = CheckpointSystem::new(100, 10);
        let (world, seed) = create_test_world();

        // Create checkpoints at various ticks
        for tick in [0, 100, 200, 300, 400] {
            checkpoints.maybe_create_checkpoint(tick, &world, seed);
        }

        assert_eq!(checkpoints.len(), 5);

        // Find nearest checkpoint
        let cp = checkpoints.find_nearest(250).unwrap();
        assert_eq!(cp.tick, 200);

        let cp = checkpoints.find_nearest(500).unwrap();
        assert_eq!(cp.tick, 400);

        let cp = checkpoints.find_nearest(50).unwrap();
        assert_eq!(cp.tick, 0);
    }

    #[test]
    fn test_replay_game_loop() {
        let (world, seed) = create_test_world();
        let mut loop_helper = ReplayGameLoop::new();

        // Start recording
        loop_helper.start_recording(seed, &world);
        assert!(loop_helper.is_recording());
        assert_eq!(loop_helper.state(), ReplayState::Recording);

        // Record some inputs
        let inputs = vec![
            PlayerInput::Move {
                direction: crate::Direction4::Up,
            },
            PlayerInput::Confirm,
        ];
        loop_helper.record_inputs(inputs);

        // Stop recording
        let replay = loop_helper.stop_recording().unwrap();
        assert!(!loop_helper.is_recording());

        // Start playback
        loop_helper.start_replay(replay);
        assert!(loop_helper.is_playing());

        // Get inputs
        let replay_inputs = loop_helper.get_inputs();
        assert!(replay_inputs.is_some());
        assert_eq!(replay_inputs.unwrap().len(), 2);
    }

    #[test]
    fn test_replay_metadata() {
        let metadata =
            ReplayMetadata::new("TestPlayer", "TestMap").with_description("A test replay");

        assert_eq!(metadata.player_name, "TestPlayer");
        assert_eq!(metadata.map_name, "TestMap");
        assert_eq!(metadata.description, Some("A test replay".to_string()));

        // Check formatted duration
        let metadata_with_duration = ReplayMetadata {
            duration_secs: 125.0,
            ..metadata.clone()
        };
        assert_eq!(metadata_with_duration.formatted_duration(), "02:05");
    }

    #[test]
    fn test_player_seek() {
        let (world, seed) = create_test_world();
        let snapshot = WorldSerializer::serialize(&world, seed, 0);
        let mut recorder = ReplayRecorder::new(seed, snapshot);

        // Record 10 ticks
        for i in 0..10 {
            if i % 3 == 0 {
                recorder.record_input(PlayerInput::Confirm);
            }
            recorder.advance_tick();
        }

        let replay = recorder.finish();
        let mut player = ReplayPlayer::new(replay);

        // Seek to tick 5
        player.seek_to(5).unwrap();
        assert_eq!(player.current_tick(), 5);

        // Seek to invalid tick should fail
        assert!(player.seek_to(100).is_err());
    }

    #[test]
    fn test_replay_player_controls() {
        let (world, seed) = create_test_world();
        let snapshot = WorldSerializer::serialize(&world, seed, 0);
        let mut recorder = ReplayRecorder::new(seed, snapshot);

        for _ in 0..5 {
            recorder.advance_tick();
        }

        let replay = recorder.finish();
        let mut player = ReplayPlayer::new(replay);

        // Test speed
        player.set_speed(2.0);
        assert_eq!(player.playback_speed(), 2.0);

        // Speed should be clamped
        player.set_speed(100.0);
        assert_eq!(player.playback_speed(), 10.0);

        // Test stop and reset
        player.play();
        player.next_tick(); // Move to tick 1
        assert_eq!(player.current_tick(), 1);

        player.stop();
        assert!(!player.is_playing());
        assert_eq!(player.current_tick(), 0);
    }

    #[test]
    fn test_player_input_variants() {
        let inputs = vec![
            PlayerInput::Move {
                direction: crate::Direction4::Up,
            },
            PlayerInput::Interact { target: None },
            PlayerInput::UseSkill {
                skill_id: 1,
                target: None,
            },
            PlayerInput::UseItem {
                item_id: 42,
                target: None,
            },
            PlayerInput::MenuOpen,
            PlayerInput::MenuClose,
            PlayerInput::MenuSelect { option: 3 },
            PlayerInput::DialogueChoice { choice_index: 1 },
            PlayerInput::BattleAction {
                action: BattleInput::Defend,
            },
            PlayerInput::Confirm,
            PlayerInput::Cancel,
            PlayerInput::RunToggle,
        ];

        // Verify all variants serialize/deserialize correctly
        for input in inputs {
            let json = serde_json::to_string(&input).unwrap();
            let deserialized: PlayerInput = serde_json::from_str(&json).unwrap();
            assert_eq!(input, deserialized);
        }
    }
}
