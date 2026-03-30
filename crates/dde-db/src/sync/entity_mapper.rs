//! Entity Mapper
//!
//! Maps between ECS entity IDs and database entity IDs.

use std::collections::HashMap;

/// Bidirectional mapping between ECS and database entity IDs
pub struct EntityMapper {
    /// ECS ID -> Database ID
    ecs_to_db: HashMap<u64, u64>,
    /// Database ID -> ECS ID
    db_to_ecs: HashMap<u64, u64>,
    /// Next database ID to assign (for new entities)
    next_db_id: u64,
}

impl EntityMapper {
    /// Create a new entity mapper
    pub fn new() -> Self {
        Self {
            ecs_to_db: HashMap::new(),
            db_to_ecs: HashMap::new(),
            next_db_id: 1,
        }
    }

    /// Create a new entity mapper with a starting ID
    pub fn with_start_id(start_id: u64) -> Self {
        Self {
            ecs_to_db: HashMap::new(),
            db_to_ecs: HashMap::new(),
            next_db_id: start_id,
        }
    }

    /// Register a mapping between ECS ID and database ID
    pub fn register(&mut self, ecs_id: u64, db_id: u64) {
        // Remove any existing mappings
        self.unregister_ecs(ecs_id);
        self.unregister_db(db_id);

        self.ecs_to_db.insert(ecs_id, db_id);
        self.db_to_ecs.insert(db_id, ecs_id);

        // Update next_db_id if necessary
        if db_id >= self.next_db_id {
            self.next_db_id = db_id + 1;
        }
    }

    /// Get database ID from ECS ID
    pub fn to_db(&self, ecs_id: u64) -> Option<u64> {
        self.ecs_to_db.get(&ecs_id).copied()
    }

    /// Get ECS ID from database ID
    pub fn to_ecs(&self, db_id: u64) -> Option<u64> {
        self.db_to_ecs.get(&db_id).copied()
    }

    /// Check if ECS ID is mapped
    pub fn has_ecs(&self, ecs_id: u64) -> bool {
        self.ecs_to_db.contains_key(&ecs_id)
    }

    /// Check if database ID is mapped
    pub fn has_db(&self, db_id: u64) -> bool {
        self.db_to_ecs.contains_key(&db_id)
    }

    /// Unregister by ECS ID
    pub fn unregister_ecs(&mut self, ecs_id: u64) -> Option<u64> {
        if let Some(db_id) = self.ecs_to_db.remove(&ecs_id) {
            self.db_to_ecs.remove(&db_id);
            Some(db_id)
        } else {
            None
        }
    }

    /// Unregister by database ID
    pub fn unregister_db(&mut self, db_id: u64) -> Option<u64> {
        if let Some(ecs_id) = self.db_to_ecs.remove(&db_id) {
            self.ecs_to_db.remove(&ecs_id);
            Some(ecs_id)
        } else {
            None
        }
    }

    /// Create a new database ID (for entities that don't exist in DB yet)
    pub fn allocate_db_id(&mut self) -> u64 {
        let id = self.next_db_id;
        self.next_db_id += 1;
        id
    }

    /// Get or create a database ID for an ECS entity
    pub fn get_or_allocate_db_id(&mut self, ecs_id: u64) -> u64 {
        if let Some(db_id) = self.to_db(ecs_id) {
            db_id
        } else {
            let db_id = self.allocate_db_id();
            self.register(ecs_id, db_id);
            db_id
        }
    }

    /// Get all ECS IDs
    pub fn all_ecs_ids(&self) -> Vec<u64> {
        self.ecs_to_db.keys().copied().collect()
    }

    /// Get all database IDs
    pub fn all_db_ids(&self) -> Vec<u64> {
        self.db_to_ecs.keys().copied().collect()
    }

    /// Get mapping count
    pub fn len(&self) -> usize {
        self.ecs_to_db.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.ecs_to_db.is_empty()
    }

    /// Clear all mappings
    pub fn clear(&mut self) {
        self.ecs_to_db.clear();
        self.db_to_ecs.clear();
        self.next_db_id = 1;
    }

    /// Get the next database ID that will be allocated
    pub fn next_db_id(&self) -> u64 {
        self.next_db_id
    }

    /// Load mappings from a list of (ecs_id, db_id) pairs
    pub fn load_mappings(&mut self, mappings: Vec<(u64, u64)>) {
        self.clear();
        for (ecs_id, db_id) in mappings {
            self.register(ecs_id, db_id);
        }
    }

    /// Export mappings as a list of (ecs_id, db_id) pairs
    pub fn export_mappings(&self) -> Vec<(u64, u64)> {
        self.ecs_to_db.iter().map(|(&e, &d)| (e, d)).collect()
    }

    /// Create a mapping from ECS entities to new database IDs
    pub fn map_ecs_entities(&mut self, ecs_ids: &[u64]) -> Vec<(u64, u64)> {
        ecs_ids
            .iter()
            .map(|&ecs_id| {
                let db_id = self.get_or_allocate_db_id(ecs_id);
                (ecs_id, db_id)
            })
            .collect()
    }

    /// Validate that mappings are consistent (bidirectional)
    pub fn validate(&self) -> Result<(), String> {
        // Check that all ECS->DB mappings have corresponding DB->ECS mappings
        for (&ecs_id, &db_id) in &self.ecs_to_db {
            match self.db_to_ecs.get(&db_id) {
                Some(&mapped_ecs) if mapped_ecs == ecs_id => {}
                Some(&mapped_ecs) => {
                    return Err(format!(
                        "Inconsistent mapping: ECS {} -> DB {}, but DB {} -> ECS {}",
                        ecs_id, db_id, db_id, mapped_ecs
                    ));
                }
                None => {
                    return Err(format!(
                        "Inconsistent mapping: ECS {} -> DB {}, but no reverse mapping",
                        ecs_id, db_id
                    ));
                }
            }
        }

        // Check that all DB->ECS mappings have corresponding ECS->DB mappings
        for (&db_id, &ecs_id) in &self.db_to_ecs {
            match self.ecs_to_db.get(&ecs_id) {
                Some(&mapped_db) if mapped_db == db_id => {}
                Some(&mapped_db) => {
                    return Err(format!(
                        "Inconsistent mapping: DB {} -> ECS {}, but ECS {} -> DB {}",
                        db_id, ecs_id, ecs_id, mapped_db
                    ));
                }
                None => {
                    return Err(format!(
                        "Inconsistent mapping: DB {} -> ECS {}, but no reverse mapping",
                        db_id, ecs_id
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get statistics about the mapping
    pub fn statistics(&self) -> EntityMapperStats {
        EntityMapperStats {
            total_mappings: self.len(),
            next_db_id: self.next_db_id,
        }
    }
}

impl Default for EntityMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about entity mappings
#[derive(Debug, Clone, Copy)]
pub struct EntityMapperStats {
    /// Total number of mappings
    pub total_mappings: usize,
    /// Next database ID to be allocated
    pub next_db_id: u64,
}

/// Serializable entity ID mapping
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityIdMapping {
    /// ECS entity ID
    pub ecs_id: u64,
    /// Database entity ID
    pub db_id: u64,
    /// Optional entity name for debugging
    pub name: Option<String>,
}

impl EntityIdMapping {
    /// Create a new mapping
    pub fn new(ecs_id: u64, db_id: u64) -> Self {
        Self {
            ecs_id,
            db_id,
            name: None,
        }
    }

    /// Create a new mapping with name
    pub fn with_name(ecs_id: u64, db_id: u64, name: impl Into<String>) -> Self {
        Self {
            ecs_id,
            db_id,
            name: Some(name.into()),
        }
    }
}

/// Builder for loading/saving entity mappings
pub struct EntityMappingBuilder {
    mappings: Vec<EntityIdMapping>,
}

impl EntityMappingBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    /// Add a mapping
    pub fn add(&mut self, ecs_id: u64, db_id: u64) -> &mut Self {
        self.mappings.push(EntityIdMapping::new(ecs_id, db_id));
        self
    }

    /// Add a mapping with name
    pub fn add_named(&mut self, ecs_id: u64, db_id: u64, name: impl Into<String>) -> &mut Self {
        self.mappings
            .push(EntityIdMapping::with_name(ecs_id, db_id, name));
        self
    }

    /// Build into EntityMapper
    pub fn build(self) -> EntityMapper {
        let mut mapper = EntityMapper::new();
        for mapping in self.mappings {
            mapper.register(mapping.ecs_id, mapping.db_id);
        }
        mapper
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.mappings)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mappings: Vec<EntityIdMapping> = serde_json::from_str(json)?;
        Ok(Self { mappings })
    }

    /// Get mappings count
    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }
}

impl Default for EntityMappingBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_mapper_creation() {
        let mapper = EntityMapper::new();
        assert!(mapper.is_empty());
        assert_eq!(mapper.len(), 0);
    }

    #[test]
    fn test_register_and_lookup() {
        let mut mapper = EntityMapper::new();
        
        mapper.register(100, 1);
        mapper.register(200, 2);
        
        assert_eq!(mapper.to_db(100), Some(1));
        assert_eq!(mapper.to_db(200), Some(2));
        assert_eq!(mapper.to_ecs(1), Some(100));
        assert_eq!(mapper.to_ecs(2), Some(200));
        
        assert!(mapper.has_ecs(100));
        assert!(mapper.has_db(1));
        assert!(!mapper.has_ecs(999));
        assert!(!mapper.has_db(999));
    }

    #[test]
    fn test_unregister() {
        let mut mapper = EntityMapper::new();
        
        mapper.register(100, 1);
        assert_eq!(mapper.len(), 1);
        
        let unmapped = mapper.unregister_ecs(100);
        assert_eq!(unmapped, Some(1));
        assert_eq!(mapper.len(), 0);
        assert!(!mapper.has_ecs(100));
        assert!(!mapper.has_db(1));
        
        // Re-register and unregister by DB
        mapper.register(100, 1);
        let unmapped = mapper.unregister_db(1);
        assert_eq!(unmapped, Some(100));
        assert_eq!(mapper.len(), 0);
    }

    #[test]
    fn test_allocate_id() {
        let mut mapper = EntityMapper::new();
        
        let id1 = mapper.allocate_db_id();
        assert_eq!(id1, 1);
        
        let id2 = mapper.allocate_db_id();
        assert_eq!(id2, 2);
        
        // Register with higher ID
        mapper.register(100, 10);
        let id3 = mapper.allocate_db_id();
        assert_eq!(id3, 11);
    }

    #[test]
    fn test_get_or_allocate() {
        let mut mapper = EntityMapper::new();
        
        // First time allocates new
        let db_id1 = mapper.get_or_allocate_db_id(100);
        assert_eq!(db_id1, 1);
        
        // Second time returns existing
        let db_id2 = mapper.get_or_allocate_db_id(100);
        assert_eq!(db_id2, 1);
        
        // Different ECS gets new ID
        let db_id3 = mapper.get_or_allocate_db_id(200);
        assert_eq!(db_id3, 2);
    }

    #[test]
    fn test_re_register() {
        let mut mapper = EntityMapper::new();
        
        mapper.register(100, 1);
        mapper.register(100, 2); // Re-register with different DB ID
        
        assert_eq!(mapper.to_db(100), Some(2));
        assert!(!mapper.has_db(1)); // Old mapping removed
    }

    #[test]
    fn test_export_import_mappings() {
        let mut mapper = EntityMapper::new();
        mapper.register(100, 1);
        mapper.register(200, 2);
        mapper.register(300, 3);
        
        let exported = mapper.export_mappings();
        assert_eq!(exported.len(), 3);
        
        let mut new_mapper = EntityMapper::new();
        new_mapper.load_mappings(exported);
        
        assert_eq!(new_mapper.to_db(100), Some(1));
        assert_eq!(new_mapper.to_db(200), Some(2));
        assert_eq!(new_mapper.to_db(300), Some(3));
    }

    #[test]
    fn test_validate() {
        let mut mapper = EntityMapper::new();
        
        // Valid mappings
        mapper.register(100, 1);
        mapper.register(200, 2);
        assert!(mapper.validate().is_ok());
        
        // Manually create inconsistency (shouldn't happen in normal use)
        mapper.db_to_ecs.insert(1, 999);
        assert!(mapper.validate().is_err());
    }

    #[test]
    fn test_mapping_builder() {
        let mut builder = EntityMappingBuilder::new();
        builder.add(100, 1).add_named(200, 2, "Player");
        
        assert_eq!(builder.len(), 2);
        
        let mapper = builder.build();
        assert_eq!(mapper.to_db(100), Some(1));
        assert_eq!(mapper.to_db(200), Some(2));
    }

    #[test]
    fn test_mapping_builder_json() {
        let mut builder = EntityMappingBuilder::new();
        builder.add(100, 1).add_named(200, 2, "Player");
        
        let json = builder.to_json().unwrap();
        let loaded = EntityMappingBuilder::from_json(&json).unwrap();
        
        assert_eq!(loaded.len(), 2);
        
        let mapper = loaded.build();
        assert_eq!(mapper.to_db(100), Some(1));
        assert_eq!(mapper.to_db(200), Some(2));
    }

    #[test]
    fn test_map_ecs_entities() {
        let mut mapper = EntityMapper::new();
        
        let mappings = mapper.map_ecs_entities(&[100, 200, 300]);
        assert_eq!(mappings.len(), 3);
        
        // Should have sequential IDs
        assert_eq!(mappings[0], (100, 1));
        assert_eq!(mappings[1], (200, 2));
        assert_eq!(mappings[2], (300, 3));
    }

    #[test]
    fn test_clear() {
        let mut mapper = EntityMapper::new();
        
        mapper.register(100, 1);
        mapper.register(200, 2);
        
        assert_eq!(mapper.len(), 2);
        
        mapper.clear();
        
        assert_eq!(mapper.len(), 0);
        assert!(mapper.is_empty());
        assert_eq!(mapper.next_db_id(), 1); // Reset
    }

    #[test]
    fn test_statistics() {
        let mut mapper = EntityMapper::new();
        
        mapper.register(100, 1);
        mapper.register(200, 5);
        
        let stats = mapper.statistics();
        assert_eq!(stats.total_mappings, 2);
        assert_eq!(stats.next_db_id, 6); // 5 + 1
    }
}
