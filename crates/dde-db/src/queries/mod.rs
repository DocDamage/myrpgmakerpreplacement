//! Database queries

use crate::models::{DialogueChoiceModel, DialogueNodeModel, DialogueTreeModel, EntityModel, Tile};
use crate::Database;
use crate::Result;

/// Result type for loading a complete dialogue tree with nodes and choices
type DialogueTreeResult = Result<
    Option<(
        DialogueTreeModel,
        Vec<DialogueNodeModel>,
        Vec<DialogueChoiceModel>,
    )>,
>;

/// Queries for tiles
pub struct TileQueries;

impl TileQueries {
    /// Get tiles in a rectangular region
    pub fn get_tiles_in_region(
        db: &Database,
        map_id: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<Vec<Tile>> {
        let mut stmt = db.conn().prepare(
            "SELECT tile_id, map_id, x, y, z, tileset_id, tile_index, world_state, biome, passable, event_trigger_id 
             FROM tiles 
             WHERE map_id = ?1 AND x >= ?2 AND x < ?3 AND y >= ?4 AND y < ?5"
        )?;

        let x_end = x + width;
        let y_end = y + height;

        let tiles = stmt
            .query_map(
                [map_id, x as u32, x_end as u32, y as u32, y_end as u32],
                |row| {
                    Ok(Tile {
                        tile_id: row.get(0)?,
                        map_id: row.get(1)?,
                        x: row.get(2)?,
                        y: row.get(3)?,
                        z: row.get(4)?,
                        tileset_id: row.get(5)?,
                        tile_index: row.get(6)?,
                        world_state: row.get(7)?,
                        biome: row.get(8)?,
                        passable: row.get(9)?,
                        event_trigger_id: row.get(10)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tiles)
    }

    /// Get tile at position
    pub fn get_tile_at(db: &Database, map_id: u32, x: i32, y: i32, z: i32) -> Result<Option<Tile>> {
        let tile: Option<Tile> = db.conn().query_row(
            "SELECT tile_id, map_id, x, y, z, tileset_id, tile_index, world_state, biome, passable, event_trigger_id 
             FROM tiles 
             WHERE map_id = ?1 AND x = ?2 AND y = ?3 AND z = ?4",
            [map_id, x as u32, y as u32, z as u32],
            |row| {
                Ok(Tile {
                    tile_id: row.get(0)?,
                    map_id: row.get(1)?,
                    x: row.get(2)?,
                    y: row.get(3)?,
                    z: row.get(4)?,
                    tileset_id: row.get(5)?,
                    tile_index: row.get(6)?,
                    world_state: row.get(7)?,
                    biome: row.get(8)?,
                    passable: row.get(9)?,
                    event_trigger_id: row.get(10)?,
                })
            }
        ).ok();

        Ok(tile)
    }
}

/// Queries for entities
pub struct EntityQueries;

impl EntityQueries {
    /// Get entities on a map
    pub fn get_entities_on_map(db: &Database, map_id: u32) -> Result<Vec<EntityModel>> {
        let mut stmt = db.conn().prepare(
            "SELECT entity_id, entity_type, name, map_id, x, y, sprite_sheet_id, direction, 
                    logic_prompt, dialogue_tree_id, stats_json, equipment_json, inventory_json, 
                    patrol_path_json, schedule_json, faction_id, is_interactable, is_collidable, respawn_ticks 
             FROM entities 
             WHERE map_id = ?1"
        )?;

        let entities = stmt
            .query_map([map_id], |row| {
                Ok(EntityModel {
                    entity_id: row.get(0)?,
                    entity_type: row.get(1)?,
                    name: row.get(2)?,
                    map_id: row.get(3)?,
                    x: row.get(4)?,
                    y: row.get(5)?,
                    sprite_sheet_id: row.get(6)?,
                    direction: row.get(7)?,
                    logic_prompt: row.get(8)?,
                    dialogue_tree_id: row.get(9)?,
                    stats_json: row.get(10)?,
                    equipment_json: row.get(11)?,
                    inventory_json: row.get(12)?,
                    patrol_path_json: row.get(13)?,
                    schedule_json: row.get(14)?,
                    faction_id: row.get(15)?,
                    is_interactable: row.get(16)?,
                    is_collidable: row.get(17)?,
                    respawn_ticks: row.get(18)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entities)
    }

    /// Get entity by ID
    pub fn get_entity(db: &Database, entity_id: u64) -> Result<Option<EntityModel>> {
        let entity: Option<EntityModel> = db.conn().query_row(
            "SELECT entity_id, entity_type, name, map_id, x, y, sprite_sheet_id, direction, 
                    logic_prompt, dialogue_tree_id, stats_json, equipment_json, inventory_json, 
                    patrol_path_json, schedule_json, faction_id, is_interactable, is_collidable, respawn_ticks 
             FROM entities 
             WHERE entity_id = ?1",
            [entity_id],
            |row| {
                Ok(EntityModel {
                    entity_id: row.get(0)?,
                    entity_type: row.get(1)?,
                    name: row.get(2)?,
                    map_id: row.get(3)?,
                    x: row.get(4)?,
                    y: row.get(5)?,
                    sprite_sheet_id: row.get(6)?,
                    direction: row.get(7)?,
                    logic_prompt: row.get(8)?,
                    dialogue_tree_id: row.get(9)?,
                    stats_json: row.get(10)?,
                    equipment_json: row.get(11)?,
                    inventory_json: row.get(12)?,
                    patrol_path_json: row.get(13)?,
                    schedule_json: row.get(14)?,
                    faction_id: row.get(15)?,
                    is_interactable: row.get(16)?,
                    is_collidable: row.get(17)?,
                    respawn_ticks: row.get(18)?,
                })
            }
        ).ok();

        Ok(entity)
    }
}

/// Queries for dialogue trees
pub struct DialogueQueries;

impl DialogueQueries {
    /// Load a complete dialogue tree with all nodes and choices
    pub fn load_dialogue_tree(db: &Database, tree_id: u32) -> DialogueTreeResult {
        // Load tree metadata
        let tree: Option<DialogueTreeModel> = db
            .conn()
            .query_row(
                "SELECT tree_id, tree_name, root_node_id FROM dialogue_trees WHERE tree_id = ?1",
                [tree_id],
                |row| {
                    Ok(DialogueTreeModel {
                        tree_id: row.get(0)?,
                        tree_name: row.get(1)?,
                        root_node_id: row.get(2)?,
                    })
                },
            )
            .ok();

        let tree = match tree {
            Some(t) => t,
            None => return Ok(None),
        };

        // Load nodes
        let mut node_stmt = db.conn().prepare(
            "SELECT node_id, tree_id, node_type, speaker, text, next_node_id, emotion, conditions_json, effects_json 
             FROM dialogue_nodes WHERE tree_id = ?1"
        )?;

        let nodes: Vec<DialogueNodeModel> = node_stmt
            .query_map([tree_id], |row| {
                Ok(DialogueNodeModel {
                    node_id: row.get(0)?,
                    tree_id: row.get(1)?,
                    node_type: row.get(2)?,
                    speaker: row.get(3)?,
                    text: row.get(4)?,
                    next_node_id: row.get(5)?,
                    emotion: row.get(6)?,
                    conditions_json: row.get(7)?,
                    effects_json: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Load choices
        let mut choice_stmt = db.conn().prepare(
            "SELECT choice_id, node_id, tree_id, choice_text, next_node_id, conditions_json, effects_json, sort_order 
             FROM dialogue_choices WHERE tree_id = ?1 ORDER BY sort_order"
        )?;

        let choices: Vec<DialogueChoiceModel> = choice_stmt
            .query_map([tree_id], |row| {
                Ok(DialogueChoiceModel {
                    choice_id: row.get(0)?,
                    node_id: row.get(1)?,
                    tree_id: row.get(2)?,
                    choice_text: row.get(3)?,
                    next_node_id: row.get(4)?,
                    conditions_json: row.get(5)?,
                    effects_json: row.get(6)?,
                    sort_order: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(Some((tree, nodes, choices)))
    }

    /// Get all dialogue tree IDs
    pub fn list_dialogue_trees(db: &Database) -> Result<Vec<(u32, String)>> {
        let mut stmt = db
            .conn()
            .prepare("SELECT tree_id, tree_name FROM dialogue_trees ORDER BY tree_name")?;

        let trees = stmt
            .query_map([], |row| {
                Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(trees)
    }

    /// Save a dialogue tree
    pub fn save_dialogue_tree(
        db: &mut Database,
        tree: &DialogueTreeModel,
        nodes: &[DialogueNodeModel],
        choices: &[DialogueChoiceModel],
    ) -> Result<()> {
        let tx = db.transaction()?;

        // Insert or replace tree
        tx.execute(
            "INSERT OR REPLACE INTO dialogue_trees (tree_id, tree_name, root_node_id) VALUES (?1, ?2, ?3)",
            (&tree.tree_id, &tree.tree_name, &tree.root_node_id),
        )?;

        // Delete existing nodes and choices
        tx.execute(
            "DELETE FROM dialogue_nodes WHERE tree_id = ?1",
            [&tree.tree_id],
        )?;
        tx.execute(
            "DELETE FROM dialogue_choices WHERE tree_id = ?1",
            [&tree.tree_id],
        )?;

        // Insert nodes
        for node in nodes {
            tx.execute(
                "INSERT INTO dialogue_nodes (node_id, tree_id, node_type, speaker, text, next_node_id, emotion, conditions_json, effects_json) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                (
                    &node.node_id,
                    &node.tree_id,
                    &node.node_type,
                    &node.speaker,
                    &node.text,
                    &node.next_node_id,
                    &node.emotion,
                    &node.conditions_json,
                    &node.effects_json,
                ),
            )?;
        }

        // Insert choices
        for choice in choices {
            tx.execute(
                "INSERT INTO dialogue_choices (choice_id, node_id, tree_id, choice_text, next_node_id, conditions_json, effects_json, sort_order) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    &choice.choice_id,
                    &choice.node_id,
                    &choice.tree_id,
                    &choice.choice_text,
                    &choice.next_node_id,
                    &choice.conditions_json,
                    &choice.effects_json,
                    &choice.sort_order,
                ),
            )?;
        }

        tx.commit()?;
        Ok(())
    }
}
