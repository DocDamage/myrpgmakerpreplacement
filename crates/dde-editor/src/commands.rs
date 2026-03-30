//! Command Pattern for Undo/Redo System
//!
//! Provides a generic command pattern for undoable operations beyond just tile painting.
//! Supports entity operations, map modifications, and composite commands for grouping.
//!
//! # Example Usage
//! ```
//! use dde_editor::commands::{CommandStack, CommandContext};
//!
//! let mut stack = CommandStack::new(100);
//! let mut ctx = CommandContext::default();
//!
//! // Execute commands, undo, redo...
//! // See individual command types for usage examples
//! ```

use std::collections::VecDeque;
use std::marker::PhantomData;

use crate::tilemap::{LayerType, Tile, TileMap};
use dde_core::{Entity, World};

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Errors that can occur during command execution
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum CommandError {
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Command undo failed: {0}")]
    UndoFailed(String),

    #[error("Invalid context: {0}")]
    InvalidContext(&'static str),

    #[error("Entity not found: {0:?}")]
    EntityNotFound(Entity),

    #[error("Component not found on entity: {0:?}")]
    ComponentNotFound(Entity),

    #[error("Map operation failed: {0}")]
    MapError(String),

    #[error("Index out of bounds: {index} >= {len}")]
    IndexOutOfBounds { index: usize, len: usize },
}

// =============================================================================
// COMMAND CONTEXT
// =============================================================================

/// Context passed to commands during execution
///
/// Contains references to the various subsystems that commands may need to modify.
/// All fields are optional to allow commands to specify their requirements.
#[derive(Default)]
pub struct CommandContext<'a> {
    /// ECS World for entity operations
    pub world: Option<&'a mut World>,
    /// Database for persistent storage operations
    pub db: Option<&'a mut dde_db::Database>,
    /// Tilemap for map editing operations
    pub tilemap: Option<&'a mut TileMap>,
    /// Current selection state
    pub selection: Option<&'a mut Selection>,
}

impl<'a> CommandContext<'a> {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with only a world reference
    pub fn with_world(world: &'a mut World) -> Self {
        Self {
            world: Some(world),
            db: None,
            tilemap: None,
            selection: None,
        }
    }

    /// Create a context with only a tilemap reference
    pub fn with_tilemap(tilemap: &'a mut TileMap) -> Self {
        Self {
            world: None,
            db: None,
            tilemap: Some(tilemap),
            selection: None,
        }
    }

    /// Create a full context with all subsystems
    pub fn new_full(
        world: &'a mut World,
        db: &'a mut dde_db::Database,
        tilemap: &'a mut TileMap,
        selection: &'a mut Selection,
    ) -> Self {
        Self {
            world: Some(world),
            db: Some(db),
            tilemap: Some(tilemap),
            selection: Some(selection),
        }
    }

    /// Require world reference, return error if not present
    pub fn world(&mut self) -> Result<&mut World, CommandError> {
        self.world
            .as_deref_mut()
            .ok_or(CommandError::InvalidContext(
                "World not available in context",
            ))
    }

    /// Require tilemap reference, return error if not present
    pub fn tilemap(&mut self) -> Result<&mut TileMap, CommandError> {
        self.tilemap
            .as_deref_mut()
            .ok_or(CommandError::InvalidContext(
                "Tilemap not available in context",
            ))
    }

    /// Require database reference, return error if not present
    pub fn db(&mut self) -> Result<&mut dde_db::Database, CommandError> {
        self.db.as_deref_mut().ok_or(CommandError::InvalidContext(
            "Database not available in context",
        ))
    }

    /// Require selection reference, return error if not present
    pub fn selection(&mut self) -> Result<&mut Selection, CommandError> {
        self.selection
            .as_deref_mut()
            .ok_or(CommandError::InvalidContext(
                "Selection not available in context",
            ))
    }
}

// =============================================================================
// SELECTION
// =============================================================================

/// Selection state for the editor
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Currently selected entities
    pub selected_entities: Vec<Entity>,
    /// Currently selected tile positions (x, y, layer)
    pub selected_tiles: Vec<(u32, u32, LayerType)>,
    /// Selection bounds (for area selection)
    pub bounds: Option<SelectionBounds>,
}

/// Bounds of a rectangular selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Selection {
    /// Create a new empty selection
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all selections
    pub fn clear(&mut self) {
        self.selected_entities.clear();
        self.selected_tiles.clear();
        self.bounds = None;
    }

    /// Select a single entity
    pub fn select_entity(&mut self, entity: Entity) {
        self.selected_entities.clear();
        self.selected_entities.push(entity);
    }

    /// Add entity to selection
    pub fn add_entity(&mut self, entity: Entity) {
        if !self.selected_entities.contains(&entity) {
            self.selected_entities.push(entity);
        }
    }

    /// Remove entity from selection
    pub fn remove_entity(&mut self, entity: Entity) {
        self.selected_entities.retain(|&e| e != entity);
    }

    /// Check if entity is selected
    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }
}

// =============================================================================
// COMMAND TRAIT
// =============================================================================

/// A command that can be executed and undone
///
/// Implement this trait for any operation that should support undo/redo.
/// Commands should store all data needed to undo their operation.
///
/// # Type Parameters
/// - Must be `Send + Sync` for thread safety
/// - Should store owned data for undo state
pub trait Command: Send + Sync {
    /// Get the display name of this command for UI
    fn name(&self) -> &str;

    /// Execute the command
    ///
    /// # Arguments
    /// * `ctx` - The command context containing references to editor state
    ///
    /// # Returns
    /// * `Ok(())` if execution succeeded
    /// * `Err(CommandError)` if execution failed
    fn execute(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError>;

    /// Undo the command
    ///
    /// # Arguments
    /// * `ctx` - The command context containing references to editor state
    ///
    /// # Returns
    /// * `Ok(())` if undo succeeded
    /// * `Err(CommandError)` if undo failed
    fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError>;

    /// Redo the command (default: calls execute)
    ///
    /// Override this if redo needs different behavior than execute
    /// (e.g., for commands that need to preserve entity IDs)
    ///
    /// # Arguments
    /// * `ctx` - The command context containing references to editor state
    ///
    /// # Returns
    /// * `Ok(())` if redo succeeded
    /// * `Err(CommandError)` if redo failed
    fn redo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        self.execute(ctx)
    }

    /// Check if this command can be merged with another
    ///
    /// Used for coalescing rapid-fire commands (e.g., drag painting)
    ///
    /// # Arguments
    /// * `other` - The other command to check for merge compatibility
    ///
    /// # Returns
    /// * `Some(Box<dyn Command>)` if commands can be merged (returns the merged command)
    /// * `None` if commands cannot be merged
    fn can_merge_with(&self, _other: &dyn Command) -> Option<Box<dyn Command>> {
        None
    }

    /// Clone this command into a boxed trait object
    fn clone_box(&self) -> Box<dyn Command>;
}

impl Clone for Box<dyn Command> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// =============================================================================
// COMMAND STACK
// =============================================================================

/// Stack-based command history for undo/redo
///
/// Maintains a list of executed commands with a current index pointer.
/// Commands are stored in a VecDeque with a maximum size limit.
/// When the limit is exceeded, oldest commands are removed.
///
/// # Type Parameters
/// * `max_size` - Maximum number of commands to retain in history
pub struct CommandStack {
    commands: VecDeque<Box<dyn Command>>,
    current_index: usize,
    max_size: usize,
}

impl Default for CommandStack {
    fn default() -> Self {
        Self::new(100)
    }
}

impl CommandStack {
    /// Create a new command stack with the specified maximum size
    ///
    /// # Arguments
    /// * `max_size` - Maximum number of commands to retain
    ///
    /// # Example
    /// ```
    /// use dde_editor::commands::CommandStack;
    ///
    /// let stack = CommandStack::new(100);
    /// ```
    pub fn new(max_size: usize) -> Self {
        Self {
            commands: VecDeque::with_capacity(max_size),
            current_index: 0,
            max_size: max_size.max(1),
        }
    }

    /// Execute a new command and add it to the stack
    ///
    /// This executes the command and then adds it to the history.
    /// Any redo history is cleared when a new command is executed.
    ///
    /// # Arguments
    /// * `cmd` - The command to execute
    /// * `ctx` - The command context
    ///
    /// # Returns
    /// * `Ok(())` if execution succeeded
    /// * `Err(CommandError)` if execution failed
    pub fn execute(
        &mut self,
        mut cmd: Box<dyn Command>,
        ctx: &mut CommandContext,
    ) -> Result<(), CommandError> {
        // Execute the command first
        cmd.execute(ctx)?;

        // Remove any redo history if we're not at the end
        if self.current_index < self.commands.len() {
            self.commands.truncate(self.current_index);
        }

        // Check if we can merge with the last command
        if let Some(last) = self.commands.back() {
            if let Some(merged) = last.can_merge_with(&*cmd) {
                self.commands.pop_back();
                self.commands.push_back(merged);
                return Ok(());
            }
        }

        // Add the new command
        self.commands.push_back(cmd);
        self.current_index += 1;

        // Enforce max size
        while self.commands.len() > self.max_size {
            self.commands.pop_front();
            self.current_index = self.current_index.saturating_sub(1);
        }

        Ok(())
    }

    /// Undo the last executed command
    ///
    /// # Arguments
    /// * `ctx` - The command context
    ///
    /// # Returns
    /// * `Ok(())` if undo succeeded
    /// * `Err(CommandError::IndexOutOfBounds)` if no command to undo
    pub fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        if self.current_index == 0 {
            return Err(CommandError::IndexOutOfBounds { index: 0, len: 0 });
        }

        self.current_index -= 1;
        let cmd = &mut self.commands[self.current_index];
        cmd.undo(ctx)?;

        Ok(())
    }

    /// Redo the next undone command
    ///
    /// # Arguments
    /// * `ctx` - The command context
    ///
    /// # Returns
    /// * `Ok(())` if redo succeeded
    /// * `Err(CommandError::IndexOutOfBounds)` if no command to redo
    pub fn redo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        if self.current_index >= self.commands.len() {
            return Err(CommandError::IndexOutOfBounds {
                index: self.current_index,
                len: self.commands.len(),
            });
        }

        let cmd = &mut self.commands[self.current_index];
        cmd.redo(ctx)?;
        self.current_index += 1;

        Ok(())
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.current_index < self.commands.len()
    }

    /// Get the number of commands in history
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get the current index position
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Get list of recent commands for UI display
    ///
    /// Returns a vector of (index, command_name) tuples.
    /// The current index is marked implicitly by position.
    ///
    /// # Example
    /// ```
    /// use dde_editor::commands::CommandStack;
    ///
    /// let stack = CommandStack::new(100);
    /// let history = stack.history();
    /// for (idx, name) in history {
    ///     println!("{}: {}", idx, name);
    /// }
    /// ```
    pub fn history(&self) -> Vec<(usize, String)> {
        self.commands
            .iter()
            .enumerate()
            .map(|(idx, cmd)| (idx, cmd.name().to_string()))
            .collect()
    }

    /// Get command history with current position marker
    ///
    /// Returns the command name and whether it's in the undo or redo zone
    pub fn history_with_state(&self) -> Vec<(usize, String, CommandState)> {
        self.commands
            .iter()
            .enumerate()
            .map(|(idx, cmd)| {
                let state = if idx < self.current_index {
                    CommandState::Undoable
                } else {
                    CommandState::Redoable
                };
                (idx, cmd.name().to_string(), state)
            })
            .collect()
    }

    /// Clear all commands from the stack
    pub fn clear(&mut self) {
        self.commands.clear();
        self.current_index = 0;
    }

    /// Jump to a specific point in history
    ///
    /// This undoes or redoes commands as needed to reach the target index.
    ///
    /// # Arguments
    /// * `index` - The target index to jump to
    /// * `ctx` - The command context
    ///
    /// # Returns
    /// * `Ok(())` if jump succeeded
    /// * `Err(CommandError::IndexOutOfBounds)` if index is out of range
    pub fn jump_to(&mut self, index: usize, ctx: &mut CommandContext) -> Result<(), CommandError> {
        if index > self.commands.len() {
            return Err(CommandError::IndexOutOfBounds {
                index,
                len: self.commands.len(),
            });
        }

        // Undo commands if moving backwards
        while self.current_index > index {
            self.undo(ctx)?;
        }

        // Redo commands if moving forwards
        while self.current_index < index {
            self.redo(ctx)?;
        }

        Ok(())
    }

    /// Get the last command name (if any)
    pub fn last_command_name(&self) -> Option<&str> {
        if self.current_index > 0 {
            self.commands.get(self.current_index - 1).map(|c| c.name())
        } else {
            None
        }
    }

    /// Get the next redo command name (if any)
    pub fn next_redo_name(&self) -> Option<&str> {
        self.commands.get(self.current_index).map(|c| c.name())
    }
}

/// State of a command in the history
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandState {
    /// Command can be undone
    Undoable,
    /// Command can be redone
    Redoable,
}

// =============================================================================
// ENTITY COMMANDS
// =============================================================================

/// Data required to recreate an entity
#[derive(Clone)]
pub struct EntityData {
    /// Components to spawn with
    pub components: Vec<Box<dyn ComponentClone>>,
    /// Initial position
    pub position: (i32, i32),
}

impl std::fmt::Debug for EntityData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityData")
            .field("component_count", &self.components.len())
            .field("position", &self.position)
            .finish()
    }
}

/// Trait for cloning components (simplified for example)
pub trait ComponentClone: Send + Sync {
    fn clone_box(&self) -> Box<dyn ComponentClone>;
}

impl Clone for Box<dyn ComponentClone> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Command to spawn a new entity
#[derive(Debug, Clone)]
pub struct SpawnEntityCommand {
    pub name: String,
    pub entity_data: EntityData,
    pub spawned_id: Option<Entity>,
}

impl SpawnEntityCommand {
    pub fn new(name: impl Into<String>, entity_data: EntityData) -> Self {
        Self {
            name: name.into(),
            entity_data,
            spawned_id: None,
        }
    }
}

impl Command for SpawnEntityCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let world = ctx.world()?;

        // Spawn the entity
        let entity = world.spawn(());
        self.spawned_id = Some(entity);

        // Add components would go here in a real implementation
        // For now, we just track the entity ID

        Ok(())
    }

    fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        if let Some(entity) = self.spawned_id {
            let world = ctx.world()?;
            world
                .despawn(entity)
                .map_err(|_| CommandError::EntityNotFound(entity))?;
            Ok(())
        } else {
            Err(CommandError::UndoFailed(
                "Entity was never spawned".to_string(),
            ))
        }
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }
}

/// Command to delete an entity
#[derive(Debug, Clone)]
pub struct DeleteEntityCommand {
    pub name: String,
    pub entity_id: Entity,
    pub deleted_data: Option<EntityData>,
}

impl DeleteEntityCommand {
    pub fn new(entity_id: Entity) -> Self {
        Self {
            name: format!("Delete Entity {:?}", entity_id),
            entity_id,
            deleted_data: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl Command for DeleteEntityCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let world = ctx.world()?;

        // In a real implementation, we'd serialize all components here
        // For now, we just store placeholder data
        self.deleted_data = Some(EntityData {
            components: Vec::new(),
            position: (0, 0),
        });

        world
            .despawn(self.entity_id)
            .map_err(|_| CommandError::EntityNotFound(self.entity_id))?;

        Ok(())
    }

    fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let world = ctx.world()?;

        // Respawn the entity with original data
        let entity = world.spawn(());
        self.entity_id = entity;

        // Restore components would go here

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }
}

/// Command to move an entity
#[derive(Debug, Clone)]
pub struct MoveEntityCommand {
    pub name: String,
    pub entity_id: Entity,
    pub old_pos: (i32, i32),
    pub new_pos: (i32, i32),
}

impl MoveEntityCommand {
    pub fn new(entity_id: Entity, old_pos: (i32, i32), new_pos: (i32, i32)) -> Self {
        Self {
            name: format!("Move Entity to ({}, {})", new_pos.0, new_pos.1),
            entity_id,
            old_pos,
            new_pos,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl Command for MoveEntityCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let _world = ctx.world()?;

        // In hecs, use query_one_mut to get mutable access to components
        // For this example, we'll use a placeholder implementation
        // In a real implementation:
        // if let Ok(pos) = _world.query_one_mut::<&mut dde_core::components::Position>(self.entity_id) {
        //     pos.x = self.new_pos.0;
        //     pos.y = self.new_pos.1;
        // }

        Ok(())
    }

    fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let _world = ctx.world()?;

        // In hecs, use query_one_mut to get mutable access to components
        // For this example, we'll use a placeholder implementation

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }
}

/// Command to modify a component on an entity
///
/// T is the component type. In hecs, any type that is Send + Sync + 'static can be a component.
#[derive(Debug, Clone)]
pub struct ModifyComponentCommand<T: Clone + Send + Sync + 'static> {
    pub name: String,
    pub entity_id: Entity,
    pub old_value: Option<T>,
    pub new_value: T,
    _phantom: PhantomData<T>,
}

impl<T: Clone + Send + Sync + 'static> ModifyComponentCommand<T> {
    pub fn new(entity_id: Entity, old_value: Option<T>, new_value: T) -> Self {
        Self {
            name: format!("Modify Component on {:?}", entity_id),
            entity_id,
            old_value,
            new_value,
            _phantom: PhantomData,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl<T: Clone + Send + Sync + 'static> Command for ModifyComponentCommand<T> {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let _world = ctx.world()?;

        // In a real implementation, we'd use hecs's component methods
        // For hecs: world.insert(self.entity_id, (self.new_value.clone(),))

        Ok(())
    }

    fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let _world = ctx.world()?;

        // Restore old value
        if let Some(ref _old) = self.old_value {
            // Insert component with old value
            // For hecs: world.insert(self.entity_id, (old.clone(),))
        }

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }
}

// =============================================================================
// MAP COMMANDS
// =============================================================================

/// Data for a single tile
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileData {
    pub x: u32,
    pub y: u32,
    pub tile: Tile,
}

/// Command to resize the map
#[derive(Debug, Clone)]
pub struct ResizeMapCommand {
    pub name: String,
    pub old_size: (u32, u32),
    pub new_size: (u32, u32),
    pub preserved_data: Vec<(LayerType, Vec<TileData>)>,
}

impl ResizeMapCommand {
    pub fn new(old_size: (u32, u32), new_size: (u32, u32)) -> Self {
        Self {
            name: format!("Resize Map to {}x{}", new_size.0, new_size.1),
            old_size,
            new_size,
            preserved_data: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl Command for ResizeMapCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let tilemap = ctx.tilemap()?;

        // Store current data for all layers
        self.preserved_data.clear();

        for layer_type in LayerType::all() {
            if let Some(layer) = tilemap.get_layer(layer_type) {
                let tiles: Vec<TileData> = (0..layer.height)
                    .flat_map(|y| (0..layer.width).map(move |x| (x, y)))
                    .filter_map(|(x, y)| layer.get_tile(x, y).map(|t| TileData { x, y, tile: *t }))
                    .collect();
                self.preserved_data.push((layer_type, tiles));
            }
        }

        // Resize the map
        tilemap.resize(self.new_size.0, self.new_size.1);

        Ok(())
    }

    fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let tilemap = ctx.tilemap()?;

        // Restore old size
        tilemap.resize(self.old_size.0, self.old_size.1);

        // Restore preserved data
        for (layer_type, tiles) in &self.preserved_data {
            if let Some(layer) = tilemap.get_layer_mut(*layer_type) {
                for tile_data in tiles {
                    if tile_data.x < self.old_size.0 && tile_data.y < self.old_size.1 {
                        layer.set_tile(tile_data.x, tile_data.y, tile_data.tile);
                    }
                }
            }
        }

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }
}

/// Map property types
#[derive(Debug, Clone, PartialEq)]
pub enum MapProperty {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// Command to set a map property
#[derive(Debug, Clone)]
pub struct SetMapPropertyCommand {
    pub name: String,
    pub property_name: String,
    pub old_value: Option<MapProperty>,
    pub new_value: MapProperty,
}

impl SetMapPropertyCommand {
    pub fn new(property_name: impl Into<String>, new_value: MapProperty) -> Self {
        let name = property_name.into();
        Self {
            name: format!("Set Property '{}'", name),
            property_name: name,
            old_value: None,
            new_value,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl Command for SetMapPropertyCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let tilemap = ctx.tilemap()?;

        // Store old value
        self.old_value = tilemap
            .properties
            .get(&self.property_name)
            .map(|s| MapProperty::String(s.clone()));

        // Set new value
        let value_str = match &self.new_value {
            MapProperty::String(s) => s.clone(),
            MapProperty::Int(i) => i.to_string(),
            MapProperty::Float(f) => f.to_string(),
            MapProperty::Bool(b) => b.to_string(),
        };

        tilemap
            .properties
            .insert(self.property_name.clone(), value_str);

        Ok(())
    }

    fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let tilemap = ctx.tilemap()?;

        match &self.old_value {
            Some(MapProperty::String(s)) => {
                tilemap
                    .properties
                    .insert(self.property_name.clone(), s.clone());
            }
            Some(_) => {
                // Convert other types back to strings
                let s = match self.old_value.as_ref().unwrap() {
                    MapProperty::String(s) => s.clone(),
                    MapProperty::Int(i) => i.to_string(),
                    MapProperty::Float(f) => f.to_string(),
                    MapProperty::Bool(b) => b.to_string(),
                };
                tilemap.properties.insert(self.property_name.clone(), s);
            }
            None => {
                tilemap.properties.remove(&self.property_name);
            }
        }

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }
}

/// Command to paint tiles (integrates with existing tilemap system)
#[derive(Debug, Clone)]
pub struct PaintTilesCommand {
    pub name: String,
    pub layer: LayerType,
    pub changes: Vec<TileChange>,
}

/// A single tile change for painting
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileChange {
    pub x: u32,
    pub y: u32,
    pub old_tile: Tile,
    pub new_tile: Tile,
}

impl PaintTilesCommand {
    pub fn new(layer: LayerType, changes: Vec<TileChange>) -> Self {
        Self {
            name: format!("Paint {} Tiles", changes.len()),
            layer,
            changes,
        }
    }

    pub fn single(x: u32, y: u32, layer: LayerType, old_tile: Tile, new_tile: Tile) -> Self {
        Self {
            name: "Paint Tile".to_string(),
            layer,
            changes: vec![TileChange {
                x,
                y,
                old_tile,
                new_tile,
            }],
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl Command for PaintTilesCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let tilemap = ctx.tilemap()?;

        for change in &self.changes {
            if let Some(layer) = tilemap.get_layer_mut(self.layer) {
                layer.set_tile(change.x, change.y, change.new_tile);
            }
        }

        Ok(())
    }

    fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        let tilemap = ctx.tilemap()?;

        for change in &self.changes {
            if let Some(layer) = tilemap.get_layer_mut(self.layer) {
                layer.set_tile(change.x, change.y, change.old_tile);
            }
        }

        Ok(())
    }

    fn can_merge_with(&self, _other: &dyn Command) -> Option<Box<dyn Command>> {
        // Note: In a real implementation, we'd use downcasting
        // This is a simplified version
        None
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }
}

// =============================================================================
// COMPOSITE COMMAND
// =============================================================================

/// Composite command for grouping multiple commands
///
/// All commands in the composite are executed/undone as a single unit.
/// This is useful for operations that modify multiple things at once.
///
/// # Example
/// ```
/// use dde_editor::commands::{CompositeCommand, PaintTilesCommand, TileChange};
/// use dde_editor::tilemap::LayerType;
///
/// let mut composite = CompositeCommand::new("Bulk Edit");
/// // Add individual commands to the composite
/// let changes = vec![TileChange {
///     x: 0, y: 0,
///     old_tile: Default::default(),
///     new_tile: Default::default(),
/// }];
/// composite.add_command(Box::new(PaintTilesCommand::new(LayerType::Terrain, changes)));
/// ```
#[derive(Clone)]
pub struct CompositeCommand {
    pub name: String,
    pub commands: Vec<Box<dyn Command>>,
}

impl std::fmt::Debug for CompositeCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeCommand")
            .field("name", &self.name)
            .field("command_count", &self.commands.len())
            .finish()
    }
}

impl CompositeCommand {
    /// Create a new empty composite command
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            commands: Vec::new(),
        }
    }

    /// Add a command to the composite
    pub fn add_command(&mut self, cmd: Box<dyn Command>) {
        self.commands.push(cmd);
    }

    /// Create a composite with a single command
    pub fn from_command(name: impl Into<String>, cmd: Box<dyn Command>) -> Self {
        let mut composite = Self::new(name);
        composite.add_command(cmd);
        composite
    }

    /// Get the number of sub-commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the composite is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Command for CompositeCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        for cmd in &mut self.commands {
            cmd.execute(ctx)?;
        }
        Ok(())
    }

    fn undo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        // Undo in reverse order
        for cmd in self.commands.iter_mut().rev() {
            cmd.undo(ctx)?;
        }
        Ok(())
    }

    fn redo(&mut self, ctx: &mut CommandContext) -> Result<(), CommandError> {
        // Redo in original order
        for cmd in &mut self.commands {
            cmd.redo(ctx)?;
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }
}

// =============================================================================
// EDITOR STATE INTEGRATION
// =============================================================================

/// Extension trait for editor types to support commands
pub trait CommandEditor {
    /// Execute a command
    fn execute_command(&mut self, cmd: Box<dyn Command>) -> Result<(), CommandError>;

    /// Undo last command
    fn undo(&mut self);

    /// Redo last undone command
    fn redo(&mut self);

    /// Check if undo is available
    fn can_undo(&self) -> bool;

    /// Check if redo is available
    fn can_redo(&self) -> bool;

    /// Get command history
    fn command_history(&self) -> Vec<(usize, String)>;
}

// =============================================================================
// UI INTEGRATION
// =============================================================================

/// Draw a history panel showing command stack state
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `command_stack` - The command stack to display
pub fn draw_history_panel(ui: &mut egui::Ui, command_stack: &CommandStack) {
    ui.label(egui::RichText::new("History").strong());
    ui.separator();

    let history = command_stack.history_with_state();

    if history.is_empty() {
        ui.label("No history");
        return;
    }

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .show(ui, |ui| {
            for (idx, name, state) in history {
                let text = format!("{}: {}", idx + 1, name);
                let label = match state {
                    CommandState::Undoable => {
                        egui::RichText::new(text).color(ui.visuals().text_color())
                    }
                    CommandState::Redoable => {
                        egui::RichText::new(text).color(ui.visuals().weak_text_color())
                    }
                };
                ui.label(label);
            }
        });

    ui.separator();

    // Show current position
    let current = command_stack.current_index();
    let total = command_stack.len();
    ui.label(format!("Position: {}/{}", current, total));

    // Show undo/redo availability
    ui.horizontal(|ui| {
        if command_stack.can_undo() {
            ui.label("✓ Undo available");
        } else {
            ui.label("✗ Cannot undo");
        }
    });
    ui.horizontal(|ui| {
        if command_stack.can_redo() {
            ui.label("✓ Redo available");
        } else {
            ui.label("✗ Cannot redo");
        }
    });
}

/// Draw command buttons for undo/redo
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `command_stack` - The command stack
/// * `ctx` - The command context for executing undo/redo
pub fn draw_command_buttons(
    ui: &mut egui::Ui,
    command_stack: &mut CommandStack,
    ctx: &mut CommandContext,
) {
    ui.horizontal(|ui| {
        let can_undo = command_stack.can_undo();
        let can_redo = command_stack.can_redo();

        if ui
            .add_enabled(can_undo, egui::Button::new("↶ Undo"))
            .clicked()
        {
            if let Err(e) = command_stack.undo(ctx) {
                tracing::error!("Undo failed: {:?}", e);
            }
        }

        if ui
            .add_enabled(can_redo, egui::Button::new("↷ Redo"))
            .clicked()
        {
            if let Err(e) = command_stack.redo(ctx) {
                tracing::error!("Redo failed: {:?}", e);
            }
        }
    });
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Mock command for testing
    #[derive(Debug, Clone)]
    struct MockCommand {
        name: String,
        executed: bool,
        undone: bool,
    }

    impl MockCommand {
        fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                executed: false,
                undone: false,
            }
        }
    }

    impl Command for MockCommand {
        fn name(&self) -> &str {
            &self.name
        }

        fn execute(&mut self, _ctx: &mut CommandContext) -> Result<(), CommandError> {
            self.executed = true;
            self.undone = false;
            Ok(())
        }

        fn undo(&mut self, _ctx: &mut CommandContext) -> Result<(), CommandError> {
            self.undone = true;
            self.executed = false;
            Ok(())
        }

        fn clone_box(&self) -> Box<dyn Command> {
            Box::new(self.clone())
        }
    }

    // Mock command that fails
    #[derive(Debug, Clone)]
    struct FailingCommand {
        fail_on: FailOn,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum FailOn {
        Execute,
        Undo,
        #[allow(dead_code)]
        Never,
    }

    impl Command for FailingCommand {
        fn name(&self) -> &str {
            "Failing Command"
        }

        fn execute(&mut self, _ctx: &mut CommandContext) -> Result<(), CommandError> {
            if self.fail_on == FailOn::Execute {
                Err(CommandError::ExecutionFailed("Execute failed".to_string()))
            } else {
                Ok(())
            }
        }

        fn undo(&mut self, _ctx: &mut CommandContext) -> Result<(), CommandError> {
            if self.fail_on == FailOn::Undo {
                Err(CommandError::UndoFailed("Undo failed".to_string()))
            } else {
                Ok(())
            }
        }

        fn clone_box(&self) -> Box<dyn Command> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_command_execution_and_undo() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        // Execute a command
        let cmd = Box::new(MockCommand::new("Test Command"));
        stack.execute(cmd, &mut ctx).unwrap();

        assert_eq!(stack.len(), 1);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        // Undo the command
        stack.undo(&mut ctx).unwrap();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
    }

    #[test]
    fn test_redo_functionality() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        // Execute and undo a command
        let cmd = Box::new(MockCommand::new("Test Command"));
        stack.execute(cmd, &mut ctx).unwrap();
        stack.undo(&mut ctx).unwrap();

        // Redo the command
        stack.redo(&mut ctx).unwrap();
        assert_eq!(stack.current_index(), 1);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_stack_limits() {
        let mut stack = CommandStack::new(3);
        let mut ctx = CommandContext::default();

        // Execute more commands than the limit
        for i in 0..5 {
            let cmd = Box::new(MockCommand::new(format!("Command {}", i)));
            stack.execute(cmd, &mut ctx).unwrap();
        }

        // Should only retain the last 3
        assert_eq!(stack.len(), 3);
        let history = stack.history();
        assert_eq!(history[0].1, "Command 2");
        assert_eq!(history[1].1, "Command 3");
        assert_eq!(history[2].1, "Command 4");
    }

    #[test]
    fn test_clear_history() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        stack
            .execute(Box::new(MockCommand::new("Cmd 1")), &mut ctx)
            .unwrap();
        stack
            .execute(Box::new(MockCommand::new("Cmd 2")), &mut ctx)
            .unwrap();

        stack.clear();

        assert_eq!(stack.len(), 0);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_new_command_clears_redo_history() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        stack
            .execute(Box::new(MockCommand::new("Cmd 1")), &mut ctx)
            .unwrap();
        stack
            .execute(Box::new(MockCommand::new("Cmd 2")), &mut ctx)
            .unwrap();
        stack.undo(&mut ctx).unwrap();

        assert!(stack.can_redo());

        // Execute new command - should clear redo history
        stack
            .execute(Box::new(MockCommand::new("Cmd 3")), &mut ctx)
            .unwrap();

        assert!(!stack.can_redo());
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_execution_failure() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        let cmd = Box::new(FailingCommand {
            fail_on: FailOn::Execute,
        });
        let result = stack.execute(cmd, &mut ctx);

        assert!(result.is_err());
        assert_eq!(stack.len(), 0); // Command should not be added on failure
    }

    #[test]
    fn test_undo_failure() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        let cmd = Box::new(FailingCommand {
            fail_on: FailOn::Undo,
        });
        stack.execute(cmd, &mut ctx).unwrap();

        let result = stack.undo(&mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_jump_to_index() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        stack
            .execute(Box::new(MockCommand::new("Cmd 1")), &mut ctx)
            .unwrap();
        stack
            .execute(Box::new(MockCommand::new("Cmd 2")), &mut ctx)
            .unwrap();
        stack
            .execute(Box::new(MockCommand::new("Cmd 3")), &mut ctx)
            .unwrap();

        // Jump back to index 1
        stack.jump_to(1, &mut ctx).unwrap();
        assert_eq!(stack.current_index(), 1);

        // Jump forward to index 3
        stack.jump_to(3, &mut ctx).unwrap();
        assert_eq!(stack.current_index(), 3);
    }

    #[test]
    fn test_jump_to_out_of_bounds() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        stack
            .execute(Box::new(MockCommand::new("Cmd 1")), &mut ctx)
            .unwrap();

        let result = stack.jump_to(5, &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_composite_command() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        let mut composite = CompositeCommand::new("Bulk Operation");
        composite.add_command(Box::new(MockCommand::new("Sub Cmd 1")));
        composite.add_command(Box::new(MockCommand::new("Sub Cmd 2")));
        composite.add_command(Box::new(MockCommand::new("Sub Cmd 3")));

        stack.execute(Box::new(composite), &mut ctx).unwrap();
        assert_eq!(stack.len(), 1);

        // Undo should undo all sub-commands
        stack.undo(&mut ctx).unwrap();
        assert!(!stack.can_undo());
    }

    #[test]
    #[allow(static_mut_refs)]
    fn test_composite_undo_order() {
        let mut ctx = CommandContext::default();

        // Track execution order
        static mut EXECUTE_ORDER: Vec<&str> = Vec::new();
        static mut UNDO_ORDER: Vec<&str> = Vec::new();

        #[derive(Debug, Clone)]
        struct TrackingCommand(&'static str);

        impl Command for TrackingCommand {
            fn name(&self) -> &str {
                self.0
            }

            fn execute(&mut self, _ctx: &mut CommandContext) -> Result<(), CommandError> {
                unsafe {
                    EXECUTE_ORDER.push(self.0);
                }
                Ok(())
            }

            fn undo(&mut self, _ctx: &mut CommandContext) -> Result<(), CommandError> {
                unsafe {
                    UNDO_ORDER.push(self.0);
                }
                Ok(())
            }

            fn clone_box(&self) -> Box<dyn Command> {
                Box::new(self.clone())
            }
        }

        let mut composite = CompositeCommand::new("Test");
        composite.add_command(Box::new(TrackingCommand("A")));
        composite.add_command(Box::new(TrackingCommand("B")));
        composite.add_command(Box::new(TrackingCommand("C")));

        unsafe {
            EXECUTE_ORDER.clear();
            UNDO_ORDER.clear();
        }

        composite.execute(&mut ctx).unwrap();
        unsafe {
            assert_eq!(EXECUTE_ORDER, vec!["A", "B", "C"]);
        }

        composite.undo(&mut ctx).unwrap();
        unsafe {
            // Undo should happen in reverse order
            assert_eq!(UNDO_ORDER, vec!["C", "B", "A"]);
        }
    }

    #[test]
    fn test_paint_tiles_command() {
        let mut tilemap = TileMap::new("test", "Test", 10, 10, 32);

        let old_tile = Tile::empty();
        let new_tile = Tile::new(1, 5);

        let mut cmd = PaintTilesCommand::single(5, 5, LayerType::Terrain, old_tile, new_tile);

        // Execute
        {
            let mut ctx = CommandContext::with_tilemap(&mut tilemap);
            cmd.execute(&mut ctx).unwrap();
        }
        let tile = tilemap.get_tile_at(5, 5, LayerType::Terrain).unwrap();
        assert_eq!(tile.tile_index, 5);

        // Undo
        {
            let mut ctx = CommandContext::with_tilemap(&mut tilemap);
            cmd.undo(&mut ctx).unwrap();
        }
        let tile = tilemap.get_tile_at(5, 5, LayerType::Terrain).unwrap();
        assert!(tile.is_empty());
    }

    #[test]
    fn test_resize_map_command() {
        let mut tilemap = TileMap::new("test", "Test", 10, 10, 32);

        // Set a tile to verify preservation
        {
            let mut ctx = CommandContext::with_tilemap(&mut tilemap);
            if let Some(layer) = ctx
                .tilemap
                .as_mut()
                .unwrap()
                .get_layer_mut(LayerType::Terrain)
            {
                layer.set_tile(5, 5, Tile::new(1, 5));
            }
        }

        let mut cmd = ResizeMapCommand::new((10, 10), (20, 20));

        // Execute resize
        {
            let mut ctx = CommandContext::with_tilemap(&mut tilemap);
            cmd.execute(&mut ctx).unwrap();
        }
        assert_eq!(tilemap.width, 20);
        assert_eq!(tilemap.height, 20);

        // Undo resize
        {
            let mut ctx = CommandContext::with_tilemap(&mut tilemap);
            cmd.undo(&mut ctx).unwrap();
        }
        assert_eq!(tilemap.width, 10);
        assert_eq!(tilemap.height, 10);

        // Verify tile was restored
        let tile = tilemap.get_tile_at(5, 5, LayerType::Terrain).unwrap();
        assert_eq!(tile.tile_index, 5);
    }

    #[test]
    fn test_set_map_property_command() {
        let mut tilemap = TileMap::new("test", "Test", 10, 10, 32);

        let mut cmd =
            SetMapPropertyCommand::new("weather", MapProperty::String("rainy".to_string()));

        // Execute
        {
            let mut ctx = CommandContext::with_tilemap(&mut tilemap);
            cmd.execute(&mut ctx).unwrap();
        }
        assert_eq!(
            tilemap.properties.get("weather"),
            Some(&"rainy".to_string())
        );

        // Undo
        {
            let mut ctx = CommandContext::with_tilemap(&mut tilemap);
            cmd.undo(&mut ctx).unwrap();
        }
        assert!(!tilemap.properties.contains_key("weather"));
    }

    #[test]
    fn test_command_context_validation() {
        let mut ctx = CommandContext::default();

        // Should fail - no world
        let result = ctx.world();
        assert!(result.is_err());

        // Should fail - no tilemap
        let result = ctx.tilemap();
        assert!(result.is_err());
    }

    #[test]
    fn test_selection_operations() {
        let mut selection = Selection::new();

        // Create a dummy entity using spawn
        let mut world = World::new();
        let entity = world.spawn(());

        selection.select_entity(entity);
        assert!(selection.is_selected(entity));

        selection.clear();
        assert!(!selection.is_selected(entity));
    }

    #[test]
    fn test_command_history() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        stack
            .execute(Box::new(MockCommand::new("Cmd A")), &mut ctx)
            .unwrap();
        stack
            .execute(Box::new(MockCommand::new("Cmd B")), &mut ctx)
            .unwrap();
        stack.undo(&mut ctx).unwrap();

        let history = stack.history_with_state();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].2, CommandState::Undoable);
        assert_eq!(history[1].2, CommandState::Redoable);
    }

    #[test]
    fn test_last_command_name() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        assert!(stack.last_command_name().is_none());

        stack
            .execute(Box::new(MockCommand::new("Last Command")), &mut ctx)
            .unwrap();
        assert_eq!(stack.last_command_name(), Some("Last Command"));
    }

    #[test]
    fn test_next_redo_name() {
        let mut stack = CommandStack::new(10);
        let mut ctx = CommandContext::default();

        stack
            .execute(Box::new(MockCommand::new("Redoable Command")), &mut ctx)
            .unwrap();
        stack.undo(&mut ctx).unwrap();

        assert_eq!(stack.next_redo_name(), Some("Redoable Command"));
    }
}
