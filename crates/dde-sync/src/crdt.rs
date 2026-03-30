//! Conflict-free Replicated Data Types for SQLite synchronization
//!
//! This module implements simple CRDT types:
//! - LWWRegister: Last-Write-Wins Register
//! - LWWMap: Map of LWW registers keyed by entity

#[cfg(test)]
use dde_core::Entity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// Vector clock for tracking causality
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VClock {
    pub timestamp: u64,
    pub node_id: u32,
}

impl VClock {
    pub fn new(timestamp: u64, node_id: u32) -> Self {
        Self { timestamp, node_id }
    }

    pub fn now(node_id: u32) -> Self {
        Self {
            timestamp: current_timestamp(),
            node_id,
        }
    }
}

/// Last-Write-Wins Register
/// A simple CRDT where the value with the highest timestamp wins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LWWRegister<T> {
    value: Option<T>,
    timestamp: u64,
    node_id: u32,
}

impl<T: Clone> LWWRegister<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Some(value),
            timestamp: current_timestamp(),
            node_id: 0,
        }
    }

    pub fn with_timestamp(value: T, timestamp: u64, node_id: u32) -> Self {
        Self {
            value: Some(value),
            timestamp,
            node_id,
        }
    }

    pub fn empty() -> Self {
        Self {
            value: None,
            timestamp: 0,
            node_id: 0,
        }
    }

    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Update the value with a new timestamp
    pub fn update(&mut self, value: T, timestamp: u64, node_id: u32) {
        if timestamp > self.timestamp || (timestamp == self.timestamp && node_id > self.node_id) {
            self.value = Some(value);
            self.timestamp = timestamp;
            self.node_id = node_id;
        }
    }

    /// Merge another register into this one
    pub fn merge(&mut self, other: &Self) {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.node_id > self.node_id)
        {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.node_id = other.node_id;
        }
    }
}

impl<T: Clone> Default for LWWRegister<T> {
    fn default() -> Self {
        Self::empty()
    }
}

/// LWW Map - a map of LWW registers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LWWMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    entries: HashMap<K, LWWRegister<V>>,
}

impl<K: Eq + Hash + Clone, V: Clone> LWWMap<K, V> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key).and_then(|reg| reg.value())
    }

    pub fn get_register(&self, key: &K) -> Option<&LWWRegister<V>> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: K, value: V, timestamp: u64, node_id: u32) {
        self.entries
            .entry(key)
            .or_insert_with(LWWRegister::empty)
            .update(value, timestamp, node_id);
    }

    pub fn remove(&mut self, key: &K, timestamp: u64, _node_id: u32) {
        // In LWW, removal is just setting a tombstone with a value of None
        // For simplicity, we just remove the entry
        if let Some(reg) = self.entries.get(key) {
            if timestamp > reg.timestamp() {
                self.entries.remove(key);
            }
        }
    }

    pub fn apply(&mut self, key: K, register: LWWRegister<V>) {
        self.entries
            .entry(key)
            .and_modify(|e| e.merge(&register))
            .or_insert(register);
    }

    pub fn merge(&mut self, other: &Self) {
        for (key, reg) in &other.entries {
            self.apply(key.clone(), reg.clone());
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &LWWRegister<V>)> {
        self.entries.iter()
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Default for LWWMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// CRDT for entity component data - stores ComponentData directly
pub type EntityCrdt = LWWMap<dde_core::Entity, crate::protocol::ComponentData>;

/// CRDT for tile map data - stores tile_id directly
pub type TileMapCrdt = LWWMap<(u32, i32, i32, i32), u32>;

/// SQLite operation derived from CRDT changes
#[derive(Debug, Clone)]
pub enum SqliteOp {
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<serde_json::Value>,
    },
    Update {
        table: String,
        columns: Vec<String>,
        values: Vec<serde_json::Value>,
        where_clause: String,
        where_params: Vec<serde_json::Value>,
    },
    Delete {
        table: String,
        where_clause: String,
        where_params: Vec<serde_json::Value>,
    },
}

impl SqliteOp {
    /// Convert to SQL string
    pub fn to_sql(&self) -> String {
        match self {
            SqliteOp::Insert { table, columns, values } => {
                let placeholders: Vec<String> = (1..=values.len())
                    .map(|i| format!("?{i}"))
                    .collect();
                format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    table,
                    columns.join(", "),
                    placeholders.join(", ")
                )
            }
            SqliteOp::Update { table, columns, where_clause, .. } => {
                let set_clause: Vec<String> = columns
                    .iter()
                    .enumerate()
                    .map(|(i, col)| format!("{} = ?{}", col, i + 1))
                    .collect();
                format!(
                    "UPDATE {} SET {} WHERE {}",
                    table,
                    set_clause.join(", "),
                    where_clause
                )
            }
            SqliteOp::Delete { table, where_clause, .. } => {
                format!("DELETE FROM {} WHERE {}", table, where_clause)
            }
        }
    }
}

/// Merges remote changes into local state
pub fn merge_changes(local: &mut EntityCrdt, remote: &EntityCrdt) {
    local.merge(remote);
}

/// Generates SQLite operations from CRDT state
pub fn crdt_to_sqlite_ops(crdt: &EntityCrdt) -> Vec<SqliteOp> {
    let mut ops = Vec::new();

    // Iterate through the CRDT and generate SQL operations
    for (entity, register) in crdt.iter() {
        if let Some(component_data) = register.value() {
            ops.push(SqliteOp::Insert {
                table: "entity_components".to_string(),
                columns: vec![
                    "entity_id".to_string(),
                    "component_type".to_string(),
                    "data".to_string(),
                ],
                values: vec![
                    serde_json::json!(entity.to_bits().get()),
                    serde_json::json!(component_data.component_type.clone()),
                    component_data.data.clone(),
                ],
            });
        }
    }

    ops
}

/// CRDT Document for tracking all project state
#[derive(Debug, Clone)]
pub struct ProjectCrdt {
    pub entities: EntityCrdt,
    pub tile_maps: TileMapCrdt,
    pub version: u64,
    pub node_id: u32,
}

impl ProjectCrdt {
    pub fn new(node_id: u32) -> Self {
        Self {
            entities: EntityCrdt::default(),
            tile_maps: TileMapCrdt::default(),
            version: 0,
            node_id,
        }
    }

    /// Apply a component update to the CRDT
    pub fn update_component(&mut self, entity: dde_core::Entity, component: crate::protocol::ComponentData) {
        let timestamp = current_timestamp();
        self.entities.insert(entity, component, timestamp, self.node_id);
        self.version += 1;
    }

    /// Apply a tile update to the CRDT
    pub fn update_tile(&mut self, map_id: u32, x: i32, y: i32, z: i32, tile_id: u32) {
        let key = (map_id, x, y, z);
        let timestamp = current_timestamp();
        self.tile_maps.insert(key, tile_id, timestamp, self.node_id);
        self.version += 1;
    }

    /// Merge another CRDT into this one
    pub fn merge(&mut self, other: &Self) {
        merge_changes(&mut self.entities, &other.entities);
        self.tile_maps.merge(&other.tile_maps);
        self.version = self.version.max(other.version);
    }
}

impl Default for ProjectCrdt {
    fn default() -> Self {
        Self::new(0)
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ComponentData;

    #[test]
    fn test_lww_register() {
        let mut reg = LWWRegister::new("hello");
        assert_eq!(reg.value(), Some(&"hello"));

        // Update with higher timestamp
        reg.update("world", reg.timestamp() + 1, 0);
        assert_eq!(reg.value(), Some(&"world"));

        // Update with lower timestamp should not change
        reg.update("ignored", reg.timestamp() - 1, 0);
        assert_eq!(reg.value(), Some(&"world"));
    }

    #[test]
    fn test_lww_map() {
        let mut map = LWWMap::new();
        map.insert("key1", "value1", 1, 0);
        assert_eq!(map.get(&"key1"), Some(&"value1"));

        map.insert("key1", "value2", 2, 0);
        assert_eq!(map.get(&"key1"), Some(&"value2"));
    }

    #[test]
    fn test_lww_map_merge() {
        let mut map1 = LWWMap::new();
        map1.insert("key1", "value1", 1, 0);

        let mut map2 = LWWMap::new();
        map2.insert("key2", "value2", 1, 0);

        map1.merge(&map2);
        assert_eq!(map1.get(&"key1"), Some(&"value1"));
        assert_eq!(map1.get(&"key2"), Some(&"value2"));
    }

    #[test]
    fn test_project_crdt_new() {
        let crdt = ProjectCrdt::new(1);
        assert_eq!(crdt.version, 0);
        assert_eq!(crdt.node_id, 1);
    }

    #[test]
    fn test_update_component() {
        let mut crdt = ProjectCrdt::new(0);
        let entity = dde_core::Entity::DANGLING;
        let component = ComponentData {
            component_type: "transform".to_string(),
            data: serde_json::json!({"x": 10, "y": 20}),
        };

        crdt.update_component(entity, component);
        assert_eq!(crdt.version, 1);
        assert!(crdt.entities.contains_key(&entity));
    }
}
