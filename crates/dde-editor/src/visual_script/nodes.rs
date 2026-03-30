//! Visual Scripting Node Definitions
//!
//! Defines all available node types, pins, and their properties for the blueprint system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Unique identifier for nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Generate a new unique node ID
    pub fn new() -> Self {
        Self(rand::random())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for pins
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PinId(pub u64);

impl PinId {
    /// Generate a new unique pin ID
    pub fn new() -> Self {
        Self(rand::random())
    }
}

impl Default for PinId {
    fn default() -> Self {
        Self::new()
    }
}

/// All available node types for visual scripting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    // ==================== Event Nodes ====================
    /// Triggered when player interacts with an entity
    OnInteract,
    /// Triggered when entity enters a region
    OnEnterRegion { region_id: u32 },
    /// Triggered when an item is used
    OnItemUse { item_id: u32 },
    /// Triggered when battle starts
    OnBattleStart { encounter_id: u32 },
    /// Triggered on game tick
    OnTick,
    /// Triggered when player steps on tile
    OnStep { x: i32, y: i32 },

    // ==================== Condition Nodes ====================
    /// Check if player has an item
    HasItem { item_id: u32, quantity: u32 },
    /// Check stat against a value
    StatCheck {
        stat: StatType,
        operator: CompareOp,
        value: i32,
    },
    /// Check quest stage
    QuestStage { quest_id: u32, stage: u32 },
    /// Check time of day
    TimeOfDay { min_hour: u8, max_hour: u8 },
    /// Random chance check
    RandomChance { percent: u8 },
    /// Check game flag
    GameFlag { flag_key: String, expected: bool },
    /// Compare two values
    Compare {
        left: ValueSource,
        operator: CompareOp,
        right: ValueSource,
    },

    // ==================== Action Nodes ====================
    /// Move entity to position
    MoveEntity { x: i32, y: i32, relative: bool },
    /// Play animation
    PlayAnimation { anim_id: u32, target: AnimationTarget },
    /// Start a battle
    StartBattle { encounter_id: u32, transition: String },
    /// Show dialogue
    ShowDialogue { text: String, speaker: String, portrait: Option<u32> },
    /// Modify a variable
    ModifyVariable {
        name: String,
        operation: MathOp,
        value: i32,
    },
    /// Give item to player
    GiveItem { item_id: u32, quantity: u32 },
    /// Remove item from player
    RemoveItem { item_id: u32, quantity: u32 },
    /// Teleport player
    Teleport { map_id: u32, x: i32, y: i32 },
    /// Play sound effect
    PlaySfx { sound_id: String },
    /// Change background music
    ChangeBgm { bgm_id: String, fade_ms: u32 },
    /// Spawn entity
    SpawnEntity {
        template_id: u32,
        x: i32,
        y: i32,
    },
    /// Despawn entity
    DespawnEntity { entity_ref: EntityRef },
    /// Set game flag
    SetGameFlag { flag_key: String, value: bool },
    /// Start quest
    StartQuest { quest_id: u32 },
    /// Update quest objective
    UpdateQuest {
        quest_id: u32,
        objective_id: u32,
        progress: u32,
    },
    /// Complete quest
    CompleteQuest { quest_id: u32 },
    /// Show notification
    ShowNotification { text: String, duration_secs: f32 },
    /// Apply damage/healing
    ModifyHealth { target: EntityRef, amount: i32 },
    /// Grant experience
    GrantExp { target: EntityRef, amount: u32 },

    // ==================== Flow Control Nodes ====================
    /// Branch based on condition
    Branch,
    /// Loop a specific number of times
    Loop { count: u32 },
    /// Loop while condition is true
    WhileLoop,
    /// For each item in collection
    ForEach { collection: CollectionType },
    /// Delay execution
    Delay { seconds: f32 },
    /// Execute branches in parallel
    Parallel,
    /// Execute nodes in sequence
    Sequence,
    /// Wait for all parallel branches
    Join,
    /// Break out of loop
    Break,
    /// Continue to next iteration
    Continue,

    // ==================== Variable Nodes ====================
    /// Get variable value
    GetVariable { name: String },
    /// Set variable value
    SetVariable { name: String },
    /// Literal boolean value
    BoolLiteral { value: bool },
    /// Literal number value
    NumberLiteral { value: f64 },
    /// Literal string value
    StringLiteral { value: String },

    // ==================== Math Nodes ====================
    /// Add two numbers
    Add,
    /// Subtract two numbers
    Subtract,
    /// Multiply two numbers
    Multiply,
    /// Divide two numbers
    Divide,
    /// Modulo operation
    Modulo,
    /// Clamp value between min and max
    Clamp { min: f64, max: f64 },
    /// Random number in range
    RandomRange { min: f64, max: f64 },

    // ==================== Entity Nodes ====================
    /// Get player entity
    GetPlayer,
    /// Get entity position
    GetPosition { entity: EntityRef },
    /// Get entity stat
    GetStat { entity: EntityRef, stat: StatType },
    /// Set entity stat
    SetStat { entity: EntityRef, stat: StatType },
    /// Find nearest entity
    FindNearest { entity_type: String, radius: f32 },
    /// Get all entities in region
    GetEntitiesInRegion { region_id: u32 },
}

/// Stat types for checks and modifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatType {
    Health,
    MaxHealth,
    Mana,
    MaxMana,
    Strength,
    Defense,
    Speed,
    Level,
    Exp,
    Gold,
}

impl StatType {
    /// Get display name for the stat
    pub fn display_name(&self) -> &'static str {
        match self {
            StatType::Health => "Health",
            StatType::MaxHealth => "Max Health",
            StatType::Mana => "Mana",
            StatType::MaxMana => "Max Mana",
            StatType::Strength => "Strength",
            StatType::Defense => "Defense",
            StatType::Speed => "Speed",
            StatType::Level => "Level",
            StatType::Exp => "Experience",
            StatType::Gold => "Gold",
        }
    }
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompareOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

impl CompareOp {
    /// Get display symbol for the operator
    pub fn symbol(&self) -> &'static str {
        match self {
            CompareOp::Equal => "==",
            CompareOp::NotEqual => "!=",
            CompareOp::LessThan => "<",
            CompareOp::LessThanOrEqual => "<=",
            CompareOp::GreaterThan => ">",
            CompareOp::GreaterThanOrEqual => ">=",
        }
    }
}

/// Math operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MathOp {
    Set,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

impl MathOp {
    /// Get display symbol for the operation
    pub fn symbol(&self) -> &'static str {
        match self {
            MathOp::Set => "=",
            MathOp::Add => "+=",
            MathOp::Subtract => "-=",
            MathOp::Multiply => "*=",
            MathOp::Divide => "/=",
            MathOp::Modulo => "%=",
        }
    }
}

/// Source of a value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueSource {
    Literal(f64),
    Variable(String),
    Stat { entity: EntityRef, stat: StatType },
}

/// Entity reference types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EntityRef {
    SelfEntity,
    Player,
    Target,
    ById(u64),
}

/// Animation target types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AnimationTarget {
    SelfEntity,
    Player,
    Target,
}

/// Collection types for iteration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollectionType {
    Inventory,
    Party,
    EntitiesInRegion { region_id: u32 },
    Custom(String),
}

/// Pin data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PinType {
    /// Execution flow control
    Execution,
    /// Boolean value
    Boolean,
    /// Numeric value (integer or float)
    Number,
    /// String value
    String,
    /// Entity reference
    Entity,
    /// Item reference
    Item,
    /// Vector/Position
    Vector,
    /// Any type (for dynamic pins)
    Any,
}

impl PinType {
    /// Get the color for this pin type (for UI rendering)
    pub fn color(&self) -> egui::Color32 {
        match self {
            PinType::Execution => egui::Color32::WHITE,
            PinType::Boolean => egui::Color32::from_rgb(200, 50, 50),
            PinType::Number => egui::Color32::from_rgb(50, 150, 200),
            PinType::String => egui::Color32::from_rgb(200, 150, 50),
            PinType::Entity => egui::Color32::from_rgb(150, 200, 50),
            PinType::Item => egui::Color32::from_rgb(200, 100, 200),
            PinType::Vector => egui::Color32::from_rgb(100, 200, 200),
            PinType::Any => egui::Color32::GRAY,
        }
    }

    /// Check if this pin type can connect to another
    pub fn can_connect_to(&self, other: &PinType) -> bool {
        match (self, other) {
            (a, b) if a == b => true,
            (PinType::Any, _) | (_, PinType::Any) => true,
            (PinType::Number, PinType::Boolean) | (PinType::Boolean, PinType::Number) => true,
            _ => false,
        }
    }
}

/// A pin on a node (input or output)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    pub id: PinId,
    pub pin_type: PinType,
    pub name: String,
    pub optional: bool,
    pub default_value: Option<PinValue>,
}

impl Pin {
    /// Create a new pin
    pub fn new(name: impl Into<String>, pin_type: PinType) -> Self {
        Self {
            id: PinId::new(),
            pin_type,
            name: name.into(),
            optional: false,
            default_value: None,
        }
    }

    /// Make this pin optional
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Set a default value
    pub fn with_default(mut self, value: PinValue) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Create an execution input pin
    pub fn exec_in(name: impl Into<String>) -> Self {
        Self::new(name, PinType::Execution)
    }

    /// Create an execution output pin
    pub fn exec_out(name: impl Into<String>) -> Self {
        Self::new(name, PinType::Execution)
    }

    /// Create a data input pin
    pub fn data_in(name: impl Into<String>, pin_type: PinType) -> Self {
        Self::new(name, pin_type)
    }

    /// Create a data output pin
    pub fn data_out(name: impl Into<String>, pin_type: PinType) -> Self {
        Self::new(name, pin_type)
    }
}

/// Values that can be stored in pins
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PinValue {
    None,
    Bool(bool),
    Number(f64),
    String(String),
    Entity(u64),
    Item(u32),
    Vector([f32; 2]),
}

/// A node in the visual script graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub node_type: NodeType,
    pub position: [f32; 2],
    pub inputs: Vec<Pin>,
    pub outputs: Vec<Pin>,
    pub properties: HashMap<String, NodeProperty>,
    pub comment: Option<String>,
}

impl Node {
    /// Create a new node
    pub fn new(node_type: NodeType, position: [f32; 2]) -> Self {
        let mut node = Self {
            id: NodeId::new(),
            node_type: node_type.clone(),
            position,
            inputs: Vec::new(),
            outputs: Vec::new(),
            properties: HashMap::new(),
            comment: None,
        };
        node.setup_default_pins();
        node
    }

    /// Setup default pins based on node type
    fn setup_default_pins(&mut self) {
        match &self.node_type {
            // Event nodes - no execution input, single output
            NodeType::OnInteract
            | NodeType::OnEnterRegion { .. }
            | NodeType::OnItemUse { .. }
            | NodeType::OnBattleStart { .. }
            | NodeType::OnTick
            | NodeType::OnStep { .. } => {
                self.outputs.push(Pin::exec_out("Then"));
            }

            // Condition nodes - execution in/out, data outputs
            NodeType::HasItem { .. }
            | NodeType::StatCheck { .. }
            | NodeType::QuestStage { .. }
            | NodeType::TimeOfDay { .. }
            | NodeType::RandomChance { .. }
            | NodeType::GameFlag { .. }
            | NodeType::Compare { .. } => {
                self.inputs.push(Pin::exec_in("In"));
                self.outputs.push(Pin::exec_out("True"));
                self.outputs.push(Pin::exec_out("False"));
            }

            // Action nodes - single execution in/out
            NodeType::MoveEntity { .. }
            | NodeType::PlayAnimation { .. }
            | NodeType::StartBattle { .. }
            | NodeType::ShowDialogue { .. }
            | NodeType::ModifyVariable { .. }
            | NodeType::GiveItem { .. }
            | NodeType::RemoveItem { .. }
            | NodeType::Teleport { .. }
            | NodeType::PlaySfx { .. }
            | NodeType::ChangeBgm { .. }
            | NodeType::SpawnEntity { .. }
            | NodeType::DespawnEntity { .. }
            | NodeType::SetGameFlag { .. }
            | NodeType::StartQuest { .. }
            | NodeType::UpdateQuest { .. }
            | NodeType::CompleteQuest { .. }
            | NodeType::ShowNotification { .. }
            | NodeType::ModifyHealth { .. }
            | NodeType::GrantExp { .. }
            | NodeType::Delay { .. } => {
                self.inputs.push(Pin::exec_in("In"));
                self.outputs.push(Pin::exec_out("Then"));
            }

            // Flow control
            NodeType::Branch => {
                self.inputs.push(Pin::data_in("Condition", PinType::Boolean));
                self.inputs.push(Pin::exec_in("In"));
                self.outputs.push(Pin::exec_out("True"));
                self.outputs.push(Pin::exec_out("False"));
            }
            NodeType::Loop { .. } => {
                self.inputs.push(Pin::exec_in("In"));
                self.outputs.push(Pin::exec_out("Body"));
                self.outputs.push(Pin::exec_out("Completed"));
            }
            NodeType::WhileLoop => {
                self.inputs.push(Pin::data_in("Condition", PinType::Boolean));
                self.inputs.push(Pin::exec_in("In"));
                self.outputs.push(Pin::exec_out("Body"));
                self.outputs.push(Pin::exec_out("Completed"));
            }
            NodeType::ForEach { .. } => {
                self.inputs.push(Pin::exec_in("In"));
                self.outputs.push(Pin::exec_out("Body"));
                self.outputs.push(Pin::data_out("Element", PinType::Any));
                self.outputs.push(Pin::exec_out("Completed"));
            }
            NodeType::Sequence | NodeType::Parallel => {
                self.inputs.push(Pin::exec_in("In"));
                // Dynamic number of outputs
                self.outputs.push(Pin::exec_out("0"));
                self.outputs.push(Pin::exec_out("1"));
            }
            NodeType::Join => {
                self.inputs.push(Pin::exec_in("0"));
                self.inputs.push(Pin::exec_in("1"));
                self.outputs.push(Pin::exec_out("Then"));
            }
            NodeType::Break | NodeType::Continue => {
                self.inputs.push(Pin::exec_in("In"));
            }

            // Variable nodes
            NodeType::GetVariable { name } => {
                self.properties
                    .insert("variable_name".to_string(), NodeProperty::String(name.clone()));
                self.outputs.push(Pin::data_out("Value", PinType::Any));
            }
            NodeType::SetVariable { name } => {
                self.properties
                    .insert("variable_name".to_string(), NodeProperty::String(name.clone()));
                self.inputs.push(Pin::exec_in("In"));
                self.inputs.push(Pin::data_in("Value", PinType::Any).optional());
                self.outputs.push(Pin::exec_out("Then"));
            }
            NodeType::BoolLiteral { value } => {
                self.properties.insert(
                    "value".to_string(),
                    NodeProperty::Bool(*value),
                );
                self.outputs.push(Pin::data_out("Value", PinType::Boolean));
            }
            NodeType::NumberLiteral { value } => {
                self.properties.insert(
                    "value".to_string(),
                    NodeProperty::Number(*value),
                );
                self.outputs.push(Pin::data_out("Value", PinType::Number));
            }
            NodeType::StringLiteral { value } => {
                self.properties.insert(
                    "value".to_string(),
                    NodeProperty::String(value.clone()),
                );
                self.outputs.push(Pin::data_out("Value", PinType::String));
            }

            // Math nodes
            NodeType::Add | NodeType::Subtract | NodeType::Multiply | NodeType::Divide | NodeType::Modulo => {
                self.inputs.push(Pin::data_in("A", PinType::Number));
                self.inputs.push(Pin::data_in("B", PinType::Number));
                self.outputs.push(Pin::data_out("Result", PinType::Number));
            }
            NodeType::Clamp { .. } => {
                self.inputs.push(Pin::data_in("Value", PinType::Number));
                self.outputs.push(Pin::data_out("Result", PinType::Number));
            }
            NodeType::RandomRange { .. } => {
                self.outputs.push(Pin::data_out("Result", PinType::Number));
            }

            // Entity nodes
            NodeType::GetPlayer => {
                self.outputs.push(Pin::data_out("Player", PinType::Entity));
            }
            NodeType::GetPosition { .. } => {
                self.inputs.push(Pin::data_in("Entity", PinType::Entity));
                self.outputs.push(Pin::data_out("Position", PinType::Vector));
            }
            NodeType::GetStat { stat, .. } => {
                self.inputs.push(Pin::data_in("Entity", PinType::Entity));
                self.properties.insert(
                    "stat".to_string(),
                    NodeProperty::String(format!("{:?}", stat)),
                );
                self.outputs.push(Pin::data_out("Value", PinType::Number));
            }
            NodeType::SetStat { stat, .. } => {
                self.inputs.push(Pin::exec_in("In"));
                self.inputs.push(Pin::data_in("Entity", PinType::Entity));
                self.inputs.push(Pin::data_in("Value", PinType::Number));
                self.properties.insert(
                    "stat".to_string(),
                    NodeProperty::String(format!("{:?}", stat)),
                );
                self.outputs.push(Pin::exec_out("Then"));
            }
            NodeType::FindNearest { .. } => {
                self.inputs.push(Pin::data_in("Origin", PinType::Vector).optional());
                self.outputs.push(Pin::data_out("Entity", PinType::Entity));
                self.outputs.push(Pin::data_out("Found", PinType::Boolean));
            }
            NodeType::GetEntitiesInRegion { .. } => {
                self.outputs.push(Pin::data_out("Entities", PinType::Any));
            }
        }
    }

    /// Get the display name for this node type
    pub fn display_name(&self) -> String {
        match &self.node_type {
            NodeType::OnInteract => "On Interact".to_string(),
            NodeType::OnEnterRegion { region_id } => format!("On Enter Region {}", region_id),
            NodeType::OnItemUse { item_id } => format!("On Item Use {}", item_id),
            NodeType::OnBattleStart { encounter_id } => format!("On Battle {}", encounter_id),
            NodeType::OnTick => "On Tick".to_string(),
            NodeType::OnStep { x, y } => format!("On Step ({}, {})", x, y),
            NodeType::HasItem { item_id, quantity } => format!("Has Item {} x{}", item_id, quantity),
            NodeType::StatCheck { stat, operator, value } => {
                format!("{} {} {}", stat.display_name(), operator.symbol(), value)
            }
            NodeType::QuestStage { quest_id, stage } => format!("Quest {} at Stage {}", quest_id, stage),
            NodeType::TimeOfDay { min_hour, max_hour } => format!("Time {}:00-{}:00", min_hour, max_hour),
            NodeType::RandomChance { percent } => format!("{}% Chance", percent),
            NodeType::GameFlag { flag_key, expected } => format!("Flag '{}' is {}", flag_key, expected),
            NodeType::Compare { .. } => "Compare".to_string(),
            NodeType::MoveEntity { x, y, relative } => {
                if *relative {
                    format!("Move by ({}, {})", x, y)
                } else {
                    format!("Move to ({}, {})", x, y)
                }
            }
            NodeType::PlayAnimation { anim_id, .. } => format!("Play Animation {}", anim_id),
            NodeType::StartBattle { encounter_id, .. } => format!("Start Battle {}", encounter_id),
            NodeType::ShowDialogue { speaker, .. } => format!("Dialogue: {}", speaker),
            NodeType::ModifyVariable { name, operation, value } => {
                format!("{} {} {}", name, operation.symbol(), value)
            }
            NodeType::GiveItem { item_id, quantity } => format!("Give Item {} x{}", item_id, quantity),
            NodeType::RemoveItem { item_id, quantity } => format!("Remove Item {} x{}", item_id, quantity),
            NodeType::Teleport { map_id, x, y } => format!("Teleport to {}:{},{}", map_id, x, y),
            NodeType::PlaySfx { sound_id } => format!("Play SFX: {}", sound_id),
            NodeType::ChangeBgm { bgm_id, .. } => format!("Change BGM: {}", bgm_id),
            NodeType::SpawnEntity { template_id, x, y } => format!("Spawn {} at {},{})", template_id, x, y),
            NodeType::DespawnEntity { .. } => "Despawn Entity".to_string(),
            NodeType::SetGameFlag { flag_key, value } => format!("Set Flag '{}' = {}", flag_key, value),
            NodeType::StartQuest { quest_id } => format!("Start Quest {}", quest_id),
            NodeType::UpdateQuest { quest_id, .. } => format!("Update Quest {}", quest_id),
            NodeType::CompleteQuest { quest_id } => format!("Complete Quest {}", quest_id),
            NodeType::ShowNotification { .. } => "Show Notification".to_string(),
            NodeType::ModifyHealth { amount, .. } => {
                if *amount >= 0 {
                    format!("Heal {} HP", amount)
                } else {
                    format!("Damage {} HP", -amount)
                }
            }
            NodeType::GrantExp { amount, .. } => format!("Grant {} EXP", amount),
            NodeType::Branch => "Branch".to_string(),
            NodeType::Loop { count } => format!("Loop {} times", count),
            NodeType::WhileLoop => "While Loop".to_string(),
            NodeType::ForEach { .. } => "For Each".to_string(),
            NodeType::Delay { seconds } => format!("Delay {:.1}s", seconds),
            NodeType::Parallel => "Parallel".to_string(),
            NodeType::Sequence => "Sequence".to_string(),
            NodeType::Join => "Join".to_string(),
            NodeType::Break => "Break".to_string(),
            NodeType::Continue => "Continue".to_string(),
            NodeType::GetVariable { name } => format!("Get '{}'", name),
            NodeType::SetVariable { name } => format!("Set '{}'", name),
            NodeType::BoolLiteral { value } => format!("{}", value),
            NodeType::NumberLiteral { value } => format!("{}", value),
            NodeType::StringLiteral { value } => format!("'{}'", value),
            NodeType::Add => "Add".to_string(),
            NodeType::Subtract => "Subtract".to_string(),
            NodeType::Multiply => "Multiply".to_string(),
            NodeType::Divide => "Divide".to_string(),
            NodeType::Modulo => "Modulo".to_string(),
            NodeType::Clamp { min, max } => format!("Clamp [{}, {}]", min, max),
            NodeType::RandomRange { min, max } => format!("Random [{}, {}]", min, max),
            NodeType::GetPlayer => "Get Player".to_string(),
            NodeType::GetPosition { .. } => "Get Position".to_string(),
            NodeType::GetStat { stat, .. } => format!("Get {}", stat.display_name()),
            NodeType::SetStat { stat, .. } => format!("Set {}", stat.display_name()),
            NodeType::FindNearest { entity_type, .. } => format!("Find Nearest {}", entity_type),
            NodeType::GetEntitiesInRegion { region_id } => format!("Entities in Region {}", region_id),
        }
    }

    /// Get the category color for this node
    pub fn category_color(&self) -> egui::Color32 {
        match &self.node_type {
            // Events - yellow
            NodeType::OnInteract
            | NodeType::OnEnterRegion { .. }
            | NodeType::OnItemUse { .. }
            | NodeType::OnBattleStart { .. }
            | NodeType::OnTick
            | NodeType::OnStep { .. } => egui::Color32::from_rgb(220, 180, 50),

            // Conditions - purple
            NodeType::HasItem { .. }
            | NodeType::StatCheck { .. }
            | NodeType::QuestStage { .. }
            | NodeType::TimeOfDay { .. }
            | NodeType::RandomChance { .. }
            | NodeType::GameFlag { .. }
            | NodeType::Compare { .. } => egui::Color32::from_rgb(150, 80, 180),

            // Actions - blue
            NodeType::MoveEntity { .. }
            | NodeType::PlayAnimation { .. }
            | NodeType::StartBattle { .. }
            | NodeType::ShowDialogue { .. }
            | NodeType::ModifyVariable { .. }
            | NodeType::GiveItem { .. }
            | NodeType::RemoveItem { .. }
            | NodeType::Teleport { .. }
            | NodeType::PlaySfx { .. }
            | NodeType::ChangeBgm { .. }
            | NodeType::SpawnEntity { .. }
            | NodeType::DespawnEntity { .. }
            | NodeType::SetGameFlag { .. }
            | NodeType::StartQuest { .. }
            | NodeType::UpdateQuest { .. }
            | NodeType::CompleteQuest { .. }
            | NodeType::ShowNotification { .. }
            | NodeType::ModifyHealth { .. }
            | NodeType::GrantExp { .. } => egui::Color32::from_rgb(60, 130, 220),

            // Flow control - green
            NodeType::Branch
            | NodeType::Loop { .. }
            | NodeType::WhileLoop
            | NodeType::ForEach { .. }
            | NodeType::Delay { .. }
            | NodeType::Parallel
            | NodeType::Sequence
            | NodeType::Join
            | NodeType::Break
            | NodeType::Continue => egui::Color32::from_rgb(60, 180, 100),

            // Variables - teal
            NodeType::GetVariable { .. }
            | NodeType::SetVariable { .. }
            | NodeType::BoolLiteral { .. }
            | NodeType::NumberLiteral { .. }
            | NodeType::StringLiteral { .. } => egui::Color32::from_rgb(50, 180, 180),

            // Math - orange
            NodeType::Add
            | NodeType::Subtract
            | NodeType::Multiply
            | NodeType::Divide
            | NodeType::Modulo
            | NodeType::Clamp { .. }
            | NodeType::RandomRange { .. } => egui::Color32::from_rgb(220, 140, 50),

            // Entity - red
            NodeType::GetPlayer
            | NodeType::GetPosition { .. }
            | NodeType::GetStat { .. }
            | NodeType::SetStat { .. }
            | NodeType::FindNearest { .. }
            | NodeType::GetEntitiesInRegion { .. } => egui::Color32::from_rgb(200, 80, 80),
        }
    }

    /// Move the node by a delta
    pub fn move_by(&mut self, delta: [f32; 2]) {
        self.position[0] += delta[0];
        self.position[1] += delta[1];
    }

    /// Get a pin by ID
    pub fn get_pin(&self, pin_id: PinId) -> Option<&Pin> {
        self.inputs
            .iter()
            .chain(self.outputs.iter())
            .find(|p| p.id == pin_id)
    }

    /// Get a mutable pin by ID
    pub fn get_pin_mut(&mut self, pin_id: PinId) -> Option<&mut Pin> {
        self.inputs
            .iter_mut()
            .chain(self.outputs.iter_mut())
            .find(|p| p.id == pin_id)
    }

    /// Check if this is an event node (has no execution input)
    pub fn is_event_node(&self) -> bool {
        !self.inputs.iter().any(|p| p.pin_type == PinType::Execution)
    }

    /// Get all execution input pins
    pub fn exec_inputs(&self) -> impl Iterator<Item = &Pin> {
        self.inputs.iter().filter(|p| p.pin_type == PinType::Execution)
    }

    /// Get all execution output pins
    pub fn exec_outputs(&self) -> impl Iterator<Item = &Pin> {
        self.outputs.iter().filter(|p| p.pin_type == PinType::Execution)
    }

    /// Get all data input pins
    pub fn data_inputs(&self) -> impl Iterator<Item = &Pin> {
        self.inputs.iter().filter(|p| p.pin_type != PinType::Execution)
    }

    /// Get all data output pins
    pub fn data_outputs(&self) -> impl Iterator<Item = &Pin> {
        self.outputs.iter().filter(|p| p.pin_type != PinType::Execution)
    }
}

/// Property values for nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeProperty {
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<String>),
    Enum { value: String, options: Vec<String> },
}

/// Category information for node palette
#[derive(Debug, Clone)]
pub struct NodeCategory {
    pub name: &'static str,
    pub color: egui::Color32,
    pub node_types: Vec<NodeTypeTemplate>,
}

/// Template for creating nodes of a specific type
pub struct NodeTypeTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub factory: Arc<dyn Fn() -> NodeType + Send + Sync>,
}

impl std::fmt::Debug for NodeTypeTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTypeTemplate")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("factory", &"<fn>")
            .finish()
    }
}

impl Clone for NodeTypeTemplate {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            description: self.description,
            factory: Arc::clone(&self.factory),
        }
    }
}

impl NodeTypeTemplate {
    /// Create a new node type template
    pub fn new(
        name: &'static str,
        description: &'static str,
        factory: impl Fn() -> NodeType + Send + Sync + 'static,
    ) -> Self {
        Self {
            name,
            description,
            factory: Arc::new(factory),
        }
    }

    /// Create a node from this template
    pub fn create_node(&self, position: [f32; 2]) -> Node {
        Node::new((self.factory)(), position)
    }
}

/// Get all available node categories for the palette
pub fn get_node_categories() -> Vec<NodeCategory> {
    vec![
        NodeCategory {
            name: "Events",
            color: egui::Color32::from_rgb(220, 180, 50),
            node_types: vec![
                NodeTypeTemplate::new("On Interact", "Triggered when player interacts with this entity", || NodeType::OnInteract),
                NodeTypeTemplate::new("On Enter Region", "Triggered when entity enters a region", || NodeType::OnEnterRegion { region_id: 0 }),
                NodeTypeTemplate::new("On Item Use", "Triggered when an item is used", || NodeType::OnItemUse { item_id: 0 }),
                NodeTypeTemplate::new("On Battle Start", "Triggered when battle starts", || NodeType::OnBattleStart { encounter_id: 0 }),
                NodeTypeTemplate::new("On Tick", "Triggered every game tick", || NodeType::OnTick),
                NodeTypeTemplate::new("On Step", "Triggered when player steps on tile", || NodeType::OnStep { x: 0, y: 0 }),
            ],
        },
        NodeCategory {
            name: "Conditions",
            color: egui::Color32::from_rgb(150, 80, 180),
            node_types: vec![
                NodeTypeTemplate::new("Has Item", "Check if player has item", || NodeType::HasItem { item_id: 0, quantity: 1 }),
                NodeTypeTemplate::new("Stat Check", "Check stat value", || NodeType::StatCheck { stat: StatType::Health, operator: CompareOp::GreaterThan, value: 0 }),
                NodeTypeTemplate::new("Quest Stage", "Check quest progress", || NodeType::QuestStage { quest_id: 0, stage: 0 }),
                NodeTypeTemplate::new("Time of Day", "Check current time", || NodeType::TimeOfDay { min_hour: 6, max_hour: 18 }),
                NodeTypeTemplate::new("Random Chance", "Random probability check", || NodeType::RandomChance { percent: 50 }),
                NodeTypeTemplate::new("Game Flag", "Check game flag value", || NodeType::GameFlag { flag_key: String::new(), expected: true }),
                NodeTypeTemplate::new("Compare", "Compare two values", || NodeType::Compare { left: ValueSource::Literal(0.0), operator: CompareOp::Equal, right: ValueSource::Literal(0.0) }),
            ],
        },
        NodeCategory {
            name: "Actions",
            color: egui::Color32::from_rgb(60, 130, 220),
            node_types: vec![
                NodeTypeTemplate::new("Move Entity", "Move entity to position", || NodeType::MoveEntity { x: 0, y: 0, relative: false }),
                NodeTypeTemplate::new("Play Animation", "Play animation on entity", || NodeType::PlayAnimation { anim_id: 0, target: AnimationTarget::SelfEntity }),
                NodeTypeTemplate::new("Start Battle", "Start a battle encounter", || NodeType::StartBattle { encounter_id: 0, transition: String::from("swirl") }),
                NodeTypeTemplate::new("Show Dialogue", "Show dialogue text", || NodeType::ShowDialogue { text: String::new(), speaker: String::new(), portrait: None }),
                NodeTypeTemplate::new("Modify Variable", "Modify a game variable", || NodeType::ModifyVariable { name: String::new(), operation: MathOp::Set, value: 0 }),
                NodeTypeTemplate::new("Give Item", "Give item to player", || NodeType::GiveItem { item_id: 0, quantity: 1 }),
                NodeTypeTemplate::new("Remove Item", "Remove item from player", || NodeType::RemoveItem { item_id: 0, quantity: 1 }),
                NodeTypeTemplate::new("Teleport", "Teleport player to location", || NodeType::Teleport { map_id: 0, x: 0, y: 0 }),
                NodeTypeTemplate::new("Play SFX", "Play sound effect", || NodeType::PlaySfx { sound_id: String::new() }),
                NodeTypeTemplate::new("Change BGM", "Change background music", || NodeType::ChangeBgm { bgm_id: String::new(), fade_ms: 1000 }),
                NodeTypeTemplate::new("Spawn Entity", "Spawn an entity", || NodeType::SpawnEntity { template_id: 0, x: 0, y: 0 }),
                NodeTypeTemplate::new("Despawn Entity", "Remove an entity", || NodeType::DespawnEntity { entity_ref: EntityRef::Target }),
                NodeTypeTemplate::new("Set Game Flag", "Set a game flag", || NodeType::SetGameFlag { flag_key: String::new(), value: true }),
                NodeTypeTemplate::new("Start Quest", "Start a quest", || NodeType::StartQuest { quest_id: 0 }),
                NodeTypeTemplate::new("Update Quest", "Update quest progress", || NodeType::UpdateQuest { quest_id: 0, objective_id: 0, progress: 1 }),
                NodeTypeTemplate::new("Complete Quest", "Complete a quest", || NodeType::CompleteQuest { quest_id: 0 }),
                NodeTypeTemplate::new("Show Notification", "Show UI notification", || NodeType::ShowNotification { text: String::new(), duration_secs: 3.0 }),
                NodeTypeTemplate::new("Modify Health", "Apply damage or healing", || NodeType::ModifyHealth { target: EntityRef::Target, amount: 0 }),
                NodeTypeTemplate::new("Grant EXP", "Give experience points", || NodeType::GrantExp { target: EntityRef::Target, amount: 0 }),
            ],
        },
        NodeCategory {
            name: "Flow Control",
            color: egui::Color32::from_rgb(60, 180, 100),
            node_types: vec![
                NodeTypeTemplate::new("Branch", "Branch based on condition", || NodeType::Branch),
                NodeTypeTemplate::new("Loop", "Loop N times", || NodeType::Loop { count: 3 }),
                NodeTypeTemplate::new("While Loop", "Loop while condition", || NodeType::WhileLoop),
                NodeTypeTemplate::new("For Each", "Iterate collection", || NodeType::ForEach { collection: CollectionType::Party }),
                NodeTypeTemplate::new("Delay", "Wait for duration", || NodeType::Delay { seconds: 1.0 }),
                NodeTypeTemplate::new("Parallel", "Execute in parallel", || NodeType::Parallel),
                NodeTypeTemplate::new("Sequence", "Execute in sequence", || NodeType::Sequence),
                NodeTypeTemplate::new("Join", "Wait for parallel branches", || NodeType::Join),
                NodeTypeTemplate::new("Break", "Exit loop", || NodeType::Break),
                NodeTypeTemplate::new("Continue", "Skip to next iteration", || NodeType::Continue),
            ],
        },
        NodeCategory {
            name: "Variables",
            color: egui::Color32::from_rgb(50, 180, 180),
            node_types: vec![
                NodeTypeTemplate::new("Get Variable", "Read variable value", || NodeType::GetVariable { name: String::new() }),
                NodeTypeTemplate::new("Set Variable", "Write variable value", || NodeType::SetVariable { name: String::new() }),
                NodeTypeTemplate::new("Bool Literal", "True/False constant", || NodeType::BoolLiteral { value: false }),
                NodeTypeTemplate::new("Number Literal", "Number constant", || NodeType::NumberLiteral { value: 0.0 }),
                NodeTypeTemplate::new("String Literal", "Text constant", || NodeType::StringLiteral { value: String::new() }),
            ],
        },
        NodeCategory {
            name: "Math",
            color: egui::Color32::from_rgb(220, 140, 50),
            node_types: vec![
                NodeTypeTemplate::new("Add", "A + B", || NodeType::Add),
                NodeTypeTemplate::new("Subtract", "A - B", || NodeType::Subtract),
                NodeTypeTemplate::new("Multiply", "A * B", || NodeType::Multiply),
                NodeTypeTemplate::new("Divide", "A / B", || NodeType::Divide),
                NodeTypeTemplate::new("Modulo", "A % B", || NodeType::Modulo),
                NodeTypeTemplate::new("Clamp", "Clamp to range", || NodeType::Clamp { min: 0.0, max: 1.0 }),
                NodeTypeTemplate::new("Random Range", "Random number", || NodeType::RandomRange { min: 0.0, max: 1.0 }),
            ],
        },
        NodeCategory {
            name: "Entity",
            color: egui::Color32::from_rgb(200, 80, 80),
            node_types: vec![
                NodeTypeTemplate::new("Get Player", "Get player entity", || NodeType::GetPlayer),
                NodeTypeTemplate::new("Get Position", "Get entity position", || NodeType::GetPosition { entity: EntityRef::SelfEntity }),
                NodeTypeTemplate::new("Get Stat", "Read entity stat", || NodeType::GetStat { entity: EntityRef::SelfEntity, stat: StatType::Health }),
                NodeTypeTemplate::new("Set Stat", "Write entity stat", || NodeType::SetStat { entity: EntityRef::SelfEntity, stat: StatType::Health }),
                NodeTypeTemplate::new("Find Nearest", "Find closest entity", || NodeType::FindNearest { entity_type: String::new(), radius: 10.0 }),
                NodeTypeTemplate::new("Get Entities In Region", "All entities in region", || NodeType::GetEntitiesInRegion { region_id: 0 }),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new(NodeType::OnInteract, [100.0, 200.0]);
        assert_eq!(node.position, [100.0, 200.0]);
        assert!(!node.outputs.is_empty());
    }

    #[test]
    fn test_pin_types() {
        assert!(PinType::Number.can_connect_to(&PinType::Number));
        assert!(PinType::Any.can_connect_to(&PinType::String));
        assert!(PinType::Boolean.can_connect_to(&PinType::Number));
        assert!(!PinType::Execution.can_connect_to(&PinType::Number));
    }

    #[test]
    fn test_node_categories() {
        let categories = get_node_categories();
        assert!(!categories.is_empty());
        assert!(categories.iter().any(|c| c.name == "Events"));
        assert!(categories.iter().any(|c| c.name == "Actions"));
    }

    #[test]
    fn test_event_node_detection() {
        let event_node = Node::new(NodeType::OnInteract, [0.0, 0.0]);
        assert!(event_node.is_event_node());

        let action_node = Node::new(NodeType::MoveEntity { x: 0, y: 0, relative: false }, [0.0, 0.0]);
        assert!(!action_node.is_event_node());
    }
}
