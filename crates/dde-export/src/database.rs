//! MZ Database JSON Emitters
//!
//! Generates RPG Maker MZ compatible database files.

use crate::Result;
use serde::Serialize;

/// Actor definition
#[derive(Debug, Clone)]
pub struct ActorDefinition {
    pub id: i32,
    pub name: String,
    pub nickname: String,
    pub class_id: i32,
    pub initial_level: i32,
    pub max_level: i32,
    pub character_name: String,
    pub character_index: i32,
    pub face_name: String,
    pub face_index: i32,
    pub battler_name: String,
    pub description: String,
    pub note: String,
}

impl Default for ActorDefinition {
    fn default() -> Self {
        Self {
            id: 1,
            name: "Hero".to_string(),
            nickname: "".to_string(),
            class_id: 1,
            initial_level: 1,
            max_level: 99,
            character_name: "$Hero".to_string(),
            character_index: 0,
            face_name: "Hero".to_string(),
            face_index: 0,
            battler_name: "".to_string(),
            description: "The main hero.".to_string(),
            note: "".to_string(),
        }
    }
}

/// Class definition
#[derive(Debug, Clone)]
pub struct ClassDefinition {
    pub id: i32,
    pub name: String,
    pub note: String,
}

impl Default for ClassDefinition {
    fn default() -> Self {
        Self {
            id: 1,
            name: "Warrior".to_string(),
            note: "".to_string(),
        }
    }
}

/// Skill definition
#[derive(Debug, Clone, Default)]
pub struct SkillDefinition {
    pub id: i32,
    pub name: String,
    pub description: String,
}

/// Item definition
#[derive(Debug, Clone, Default)]
pub struct ItemDefinition {
    pub id: i32,
    pub name: String,
    pub description: String,
}

/// Enemy definition
#[derive(Debug, Clone, Default)]
pub struct EnemyDefinition {
    pub id: i32,
    pub name: String,
    pub battler_name: String,
    pub max_hp: i32,
}

/// Troop definition
#[derive(Debug, Clone, Default)]
pub struct TroopDefinition {
    pub id: i32,
    pub name: String,
}

/// State definition
#[derive(Debug, Clone, Default)]
pub struct StateDefinition {
    pub id: i32,
    pub name: String,
    pub icon_index: i32,
}

/// Animation definition
#[derive(Debug, Clone, Default)]
pub struct AnimationDefinition {
    pub id: i32,
    pub name: String,
}

/// Tileset definition
#[derive(Debug, Clone, Default)]
pub struct TilesetDefinition {
    pub id: i32,
    pub name: String,
    pub mode: i32,
}

/// System configuration
#[derive(Debug, Clone)]
pub struct SystemConfig {
    pub game_title: String,
    pub currency_unit: String,
    pub window_tone: [i32; 4],
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            game_title: "My RPG".to_string(),
            currency_unit: "G".to_string(),
            window_tone: [0, 0, 0, 0],
        }
    }
}

// MZ JSON serialization structs
#[derive(Serialize)]
struct MzActor {
    id: i32,
    name: String,
    nickname: String,
    #[serde(rename = "classId")]
    class_id: i32,
    #[serde(rename = "initialLevel")]
    initial_level: i32,
    #[serde(rename = "maxLevel")]
    max_level: i32,
    #[serde(rename = "characterName")]
    character_name: String,
    #[serde(rename = "characterIndex")]
    character_index: i32,
    #[serde(rename = "faceName")]
    face_name: String,
    #[serde(rename = "faceIndex")]
    face_index: i32,
    #[serde(rename = "battlerName")]
    battler_name: String,
    description: String,
    note: String,
}

impl From<&ActorDefinition> for MzActor {
    fn from(actor: &ActorDefinition) -> Self {
        Self {
            id: actor.id,
            name: actor.name.clone(),
            nickname: actor.nickname.clone(),
            class_id: actor.class_id,
            initial_level: actor.initial_level,
            max_level: actor.max_level,
            character_name: actor.character_name.clone(),
            character_index: actor.character_index,
            face_name: actor.face_name.clone(),
            face_index: actor.face_index,
            battler_name: actor.battler_name.clone(),
            description: actor.description.clone(),
            note: actor.note.clone(),
        }
    }
}

#[derive(Serialize)]
struct MzClass {
    id: i32,
    name: String,
    note: String,
}

impl From<&ClassDefinition> for MzClass {
    fn from(class: &ClassDefinition) -> Self {
        Self {
            id: class.id,
            name: class.name.clone(),
            note: class.note.clone(),
        }
    }
}

#[derive(Serialize)]
struct MzSystem {
    #[serde(rename = "gameTitle")]
    game_title: String,
    #[serde(rename = "currencyUnit")]
    currency_unit: String,
    #[serde(rename = "windowTone")]
    window_tone: [i32; 4],
}

impl From<&SystemConfig> for MzSystem {
    fn from(system: &SystemConfig) -> Self {
        Self {
            game_title: system.game_title.clone(),
            currency_unit: system.currency_unit.clone(),
            window_tone: system.window_tone,
        }
    }
}

/// Serialize actors to MZ JSON format
pub fn serialize_actors(actors: &[ActorDefinition]) -> Result<String> {
    let mz_actors: Vec<MzActor> = actors.iter().map(MzActor::from).collect();
    let mut output = vec![serde_json::Value::Null];
    for actor in mz_actors {
        output.push(serde_json::to_value(actor)?);
    }
    Ok(serde_json::to_string(&output)?)
}

/// Serialize classes to MZ JSON format
pub fn serialize_classes(classes: &[ClassDefinition]) -> Result<String> {
    let mz_classes: Vec<MzClass> = classes.iter().map(MzClass::from).collect();
    let mut output = vec![serde_json::Value::Null];
    for class in mz_classes {
        output.push(serde_json::to_value(class)?);
    }
    Ok(serde_json::to_string(&output)?)
}

/// Serialize skills to MZ JSON format
pub fn serialize_skills(skills: &[SkillDefinition]) -> Result<String> {
    let mut output = vec![serde_json::Value::Null];
    for skill in skills {
        output.push(serde_json::json!({
            "id": skill.id,
            "name": skill.name,
            "description": skill.description,
        }));
    }
    Ok(serde_json::to_string(&output)?)
}

/// Serialize items to MZ JSON format
pub fn serialize_items(items: &[ItemDefinition]) -> Result<String> {
    let mut output = vec![serde_json::Value::Null];
    for item in items {
        output.push(serde_json::json!({
            "id": item.id,
            "name": item.name,
            "description": item.description,
        }));
    }
    Ok(serde_json::to_string(&output)?)
}

/// Serialize enemies to MZ JSON format
pub fn serialize_enemies(enemies: &[EnemyDefinition]) -> Result<String> {
    let mut output = vec![serde_json::Value::Null];
    for enemy in enemies {
        output.push(serde_json::json!({
            "id": enemy.id,
            "name": enemy.name,
            "battlerName": enemy.battler_name,
            "maxHp": enemy.max_hp,
        }));
    }
    Ok(serde_json::to_string(&output)?)
}

/// Serialize troops to MZ JSON format
pub fn serialize_troops(troops: &[TroopDefinition]) -> Result<String> {
    let mut output = vec![serde_json::Value::Null];
    for troop in troops {
        output.push(serde_json::json!({
            "id": troop.id,
            "name": troop.name,
        }));
    }
    Ok(serde_json::to_string(&output)?)
}

/// Serialize states to MZ JSON format
pub fn serialize_states(states: &[StateDefinition]) -> Result<String> {
    let mut output = vec![serde_json::Value::Null];
    for state in states {
        output.push(serde_json::json!({
            "id": state.id,
            "name": state.name,
            "iconIndex": state.icon_index,
        }));
    }
    Ok(serde_json::to_string(&output)?)
}

/// Serialize animations to MZ JSON format
pub fn serialize_animations(animations: &[AnimationDefinition]) -> Result<String> {
    let mut output = vec![serde_json::Value::Null];
    for anim in animations {
        output.push(serde_json::json!({
            "id": anim.id,
            "name": anim.name,
        }));
    }
    Ok(serde_json::to_string(&output)?)
}

/// Serialize tilesets to MZ JSON format
pub fn serialize_tilesets(tilesets: &[TilesetDefinition]) -> Result<String> {
    let mut output = vec![serde_json::Value::Null];
    for tileset in tilesets {
        output.push(serde_json::json!({
            "id": tileset.id,
            "name": tileset.name,
            "mode": tileset.mode,
        }));
    }
    Ok(serde_json::to_string(&output)?)
}

/// Serialize system config to MZ JSON format
pub fn serialize_system(system: &SystemConfig) -> Result<String> {
    let mz_system = MzSystem::from(system);
    Ok(serde_json::to_string(&mz_system)?)
}
