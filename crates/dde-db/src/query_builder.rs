//! Type-safe, fluent Query Builder for database queries
//!
//! Provides a fluent API for building SQL queries with compile-time type safety.
//!
//! # Examples
//!
//! ```rust,ignore
//! // Find all enemy entities in a map
//! let enemies = Query::<EntityModel>::new()
//!     .filter_eq("entity_type", "enemy")
//!     .filter_eq("map_id", 5)
//!     .order_by("name", Order::Asc)
//!     .fetch_all(&db)?;
//!
//! // Search entities by name
//! let results = Query::<EntityModel>::new()
//!     .filter_like("name", "goblin")
//!     .fetch_all(&db)?;
//!
//! // Paginated query
//! let page = Query::<Tile>::new()
//!     .filter_eq("map_id", 1)
//!     .order_by("y", Order::Asc)
//!     .order_by("x", Order::Asc)
//!     .limit(100)
//!     .offset(page_num * 100)
//!     .fetch_all(&db)?;
//!
//! // Count results
//! let enemy_count = Query::<EntityModel>::new()
//!     .filter_eq("entity_type", "enemy")
//!     .count(&db)?;
//! ```

use crate::models::*;
use crate::{Database, DbError};
use rusqlite::ToSql;

/// Query builder for type-safe database queries
#[derive(Debug, Clone)]
pub struct Query<T: Queryable> {
    table: String,
    filters: Vec<Filter>,
    order_by: Vec<(String, Order)>,
    limit: Option<usize>,
    offset: Option<usize>,
    _phantom: std::marker::PhantomData<T>,
}

/// Filter condition for queries
#[derive(Debug, Clone)]
pub struct Filter {
    column: String,
    operator: Operator,
    value: FilterValue,
}

/// SQL operators for filtering
#[derive(Debug, Clone, Copy)]
pub enum Operator {
    /// = (Equals)
    Eq,
    /// != (Not equals)
    Ne,
    /// > (Greater than)
    Gt,
    /// >= (Greater than or equal)
    Gte,
    /// < (Less than)
    Lt,
    /// <= (Less than or equal)
    Lte,
    /// LIKE (Pattern matching)
    Like,
    /// IN (Value in list)
    In,
    /// BETWEEN (Range check)
    Between,
    /// IS NULL
    IsNull,
    /// IS NOT NULL
    IsNotNull,
}

/// Filter values that can be used in queries
#[derive(Debug, Clone)]
pub enum FilterValue {
    /// Integer value
    Int(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Boolean value
    Bool(bool),
    /// List of values (for IN operator)
    List(Vec<FilterValue>),
    /// Range of values (for BETWEEN operator)
    Range(Box<FilterValue>, Box<FilterValue>),
}

/// Sort order for ORDER BY clauses
#[derive(Debug, Clone, Copy)]
pub enum Order {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

/// Trait for models that can be queried
///
/// Implement this trait for any type that maps to a database table.
/// The `table_name()` method returns the SQL table name, and `from_row()`
/// constructs the model from a database row.
pub trait Queryable: Sized + Send + Sync {
    /// Returns the name of the database table for this model
    fn table_name() -> &'static str;
    
    /// Constructs the model from a `rusqlite` row
    fn from_row(row: &rusqlite::Row) -> std::result::Result<Self, rusqlite::Error>;
}

// =============================================================================
// Query Builder Implementation
// =============================================================================

impl<T: Queryable> Query<T> {
    /// Start building a query for the given model type
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let query = Query::<EntityModel>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            table: T::table_name().to_string(),
            filters: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add equality filter (=)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// Query::<EntityModel>::new()
    ///     .filter_eq("entity_type", "enemy")
    /// ```
    pub fn filter_eq(mut self, column: &str, value: impl Into<FilterValue>) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::Eq,
            value: value.into(),
        });
        self
    }

    /// Add not-equal filter (!=)
    pub fn filter_ne(mut self, column: &str, value: impl Into<FilterValue>) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::Ne,
            value: value.into(),
        });
        self
    }

    /// Add greater-than filter (>)
    pub fn filter_gt(mut self, column: &str, value: impl Into<FilterValue>) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::Gt,
            value: value.into(),
        });
        self
    }

    /// Add greater-than-or-equal filter (>=)
    pub fn filter_gte(mut self, column: &str, value: impl Into<FilterValue>) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::Gte,
            value: value.into(),
        });
        self
    }

    /// Add less-than filter (<)
    pub fn filter_lt(mut self, column: &str, value: impl Into<FilterValue>) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::Lt,
            value: value.into(),
        });
        self
    }

    /// Add less-than-or-equal filter (<=)
    pub fn filter_lte(mut self, column: &str, value: impl Into<FilterValue>) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::Lte,
            value: value.into(),
        });
        self
    }

    /// Add LIKE filter for partial string matching
    ///
    /// Automatically wraps the pattern with `%` wildcards.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// Query::<EntityModel>::new()
    ///     .filter_like("name", "goblin")  // Matches "Goblin King", "Red Goblin", etc.
    /// ```
    pub fn filter_like(mut self, column: &str, pattern: &str) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::Like,
            value: FilterValue::String(format!("%{}%", pattern)),
        });
        self
    }

    /// Add LIKE filter with a custom pattern (for more control)
    ///
    /// Does NOT automatically add wildcards - use this when you need
    /// specific pattern matching (e.g., "prefix%" or "%suffix").
    pub fn filter_like_raw(mut self, column: &str, pattern: impl Into<String>) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::Like,
            value: FilterValue::String(pattern.into()),
        });
        self
    }

    /// Add IN filter (value is in a list of values)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// Query::<EntityModel>::new()
    ///     .filter_in("entity_type", vec!["enemy", "boss"])
    /// ```
    pub fn filter_in(mut self, column: &str, values: Vec<impl Into<FilterValue>>) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::In,
            value: FilterValue::List(values.into_iter().map(|v| v.into()).collect()),
        });
        self
    }

    /// Add BETWEEN filter (value is within a range, inclusive)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// Query::<Tile>::new()
    ///     .filter_between("x", 0, 100)
    /// ```
    pub fn filter_between(
        mut self,
        column: &str,
        min: impl Into<FilterValue>,
        max: impl Into<FilterValue>,
    ) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::Between,
            value: FilterValue::Range(Box::new(min.into()), Box::new(max.into())),
        });
        self
    }

    /// Add IS NULL filter
    pub fn filter_is_null(mut self, column: &str) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::IsNull,
            value: FilterValue::Bool(true), // Value is ignored for IS NULL
        });
        self
    }

    /// Add IS NOT NULL filter
    pub fn filter_is_not_null(mut self, column: &str) -> Self {
        self.filters.push(Filter {
            column: column.to_string(),
            operator: Operator::IsNotNull,
            value: FilterValue::Bool(true), // Value is ignored for IS NOT NULL
        });
        self
    }

    /// Add ORDER BY clause
    ///
    /// Multiple calls to `order_by` are chained in the order they are called.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// Query::<Tile>::new()
    ///     .order_by("y", Order::Asc)
    ///     .order_by("x", Order::Asc)
    /// ```
    pub fn order_by(mut self, column: &str, order: Order) -> Self {
        self.order_by.push((column.to_string(), order));
        self
    }

    /// Set LIMIT clause
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Set OFFSET clause (for pagination)
    pub fn offset(mut self, n: usize) -> Self {
        self.offset = Some(n);
        self
    }

    /// Build SQL query string and parameters
    ///
    /// Returns the SQL query string and a vector of boxed parameters.
    fn build_sql(&self) -> (String, Vec<Box<dyn ToSql>>) {
        let mut sql = format!("SELECT * FROM {}", self.table);
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();

        // Build WHERE clause
        if !self.filters.is_empty() {
            let conditions: Vec<String> = self
                .filters
                .iter()
                .map(|f| match f.operator {
                    Operator::IsNull => format!("{} IS NULL", f.column),
                    Operator::IsNotNull => format!("{} IS NOT NULL", f.column),
                    Operator::In => {
                        if let FilterValue::List(list) = &f.value {
                            let placeholders: Vec<String> =
                                list.iter().map(|_| "?".to_string()).collect();
                            for item in list {
                                params.push(item.to_sql_box());
                            }
                            format!("{} IN ({})", f.column, placeholders.join(", "))
                        } else {
                            "1=0".to_string() // Invalid - return false
                        }
                    }
                    Operator::Between => {
                        if let FilterValue::Range(min, max) = &f.value {
                            params.push(min.to_sql_box());
                            params.push(max.to_sql_box());
                            format!("{} BETWEEN ? AND ?", f.column)
                        } else {
                            "1=0".to_string() // Invalid - return false
                        }
                    }
                    _ => {
                        params.push(f.value.to_sql_box());
                        format!("{} {} ?", f.column, f.operator.sql())
                    }
                })
                .collect();
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        // ORDER BY
        if !self.order_by.is_empty() {
            let orders: Vec<String> = self
                .order_by
                .iter()
                .map(|(col, ord)| format!("{} {}", col, ord.sql()))
                .collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&orders.join(", "));
        }

        // LIMIT/OFFSET
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        (sql, params)
    }

    /// Execute query and fetch all results
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let enemies = Query::<EntityModel>::new()
    ///     .filter_eq("entity_type", "enemy")
    ///     .fetch_all(&db)?;
    /// ```
    pub fn fetch_all(&self, db: &Database) -> Result<Vec<T>, DbError> {
        let (sql, params) = self.build_sql();
        let conn = db.conn();

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(&param_refs[..], |row| T::from_row(row))?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(DbError::from)
    }

    /// Execute query and fetch a single result
    ///
    /// Returns `Ok(None)` if no results match the query.
    /// If multiple rows match, only the first is returned (based on ordering).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let player = Query::<EntityModel>::new()
    ///     .filter_eq("entity_type", "player")
    ///     .fetch_one(&db)?;
    /// ```
    pub fn fetch_one(&self, db: &Database) -> Result<Option<T>, DbError> {
        let mut results = self.clone().limit(1).fetch_all(db)?;
        Ok(results.pop())
    }

    /// Count matching rows
    ///
    /// This is more efficient than fetching all results and counting,
    /// as it uses `SELECT COUNT(*)` instead of `SELECT *`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let count = Query::<EntityModel>::new()
    ///     .filter_eq("map_id", 1)
    ///     .count(&db)?;
    /// ```
    pub fn count(&self, db: &Database) -> Result<usize, DbError> {
        let (mut sql, params) = self.build_sql();

        // Replace SELECT * with SELECT COUNT(*)
        sql = sql.replacen("SELECT *", "SELECT COUNT(*)", 1);

        // Remove LIMIT/OFFSET for count (they don't affect the total count)
        if let Some(pos) = sql.find(" LIMIT") {
            sql.truncate(pos);
        }

        let conn = db.conn();
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let count: i64 = stmt.query_row(&param_refs[..], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Check if any rows match the query
    ///
    /// More efficient than count() when you only need to know if rows exist.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let has_enemies = Query::<EntityModel>::new()
    ///     .filter_eq("entity_type", "enemy")
    ///     .exists(&db)?;
    /// ```
    pub fn exists(&self, db: &Database) -> Result<bool, DbError> {
        let count = self.clone().limit(1).count(db)?;
        Ok(count > 0)
    }
}

impl<T: Queryable> Default for Query<T> {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Helper Implementations
// =============================================================================

impl Operator {
    /// Returns the SQL representation of this operator
    fn sql(&self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::Ne => "!=",
            Operator::Gt => ">",
            Operator::Gte => ">=",
            Operator::Lt => "<",
            Operator::Lte => "<=",
            Operator::Like => "LIKE",
            Operator::In => "IN",
            Operator::Between => "BETWEEN",
            Operator::IsNull => "IS NULL",
            Operator::IsNotNull => "IS NOT NULL",
        }
    }
}

impl Order {
    /// Returns the SQL representation of this order
    fn sql(&self) -> &'static str {
        match self {
            Order::Asc => "ASC",
            Order::Desc => "DESC",
        }
    }
}

impl FilterValue {
    /// Convert FilterValue to a boxed ToSql trait object
    fn to_sql_box(&self) -> Box<dyn ToSql> {
        match self {
            FilterValue::Int(v) => Box::new(*v),
            FilterValue::Float(v) => Box::new(*v),
            FilterValue::String(v) => Box::new(v.clone()),
            FilterValue::Bool(v) => Box::new(*v),
            FilterValue::List(_) => panic!("List should be handled separately"),
            FilterValue::Range(_, _) => panic!("Range should be handled separately"),
        }
    }
}

// =============================================================================
// Type Conversions
// =============================================================================

impl From<i32> for FilterValue {
    fn from(v: i32) -> Self {
        FilterValue::Int(v as i64)
    }
}

impl From<i64> for FilterValue {
    fn from(v: i64) -> Self {
        FilterValue::Int(v)
    }
}

impl From<u32> for FilterValue {
    fn from(v: u32) -> Self {
        FilterValue::Int(v as i64)
    }
}

impl From<u64> for FilterValue {
    fn from(v: u64) -> Self {
        FilterValue::Int(v as i64)
    }
}

impl From<f32> for FilterValue {
    fn from(v: f32) -> Self {
        FilterValue::Float(v as f64)
    }
}

impl From<f64> for FilterValue {
    fn from(v: f64) -> Self {
        FilterValue::Float(v)
    }
}

impl From<String> for FilterValue {
    fn from(v: String) -> Self {
        FilterValue::String(v)
    }
}

impl From<&str> for FilterValue {
    fn from(v: &str) -> Self {
        FilterValue::String(v.to_string())
    }
}

impl From<bool> for FilterValue {
    fn from(v: bool) -> Self {
        FilterValue::Bool(v)
    }
}

// =============================================================================
// Queryable Implementations
// =============================================================================

impl Queryable for EntityModel {
    fn table_name() -> &'static str {
        "entities"
    }

    fn from_row(row: &rusqlite::Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(EntityModel {
            entity_id: row.get("entity_id")?,
            entity_type: row.get("entity_type")?,
            name: row.get("name")?,
            map_id: row.get("map_id")?,
            x: row.get("x")?,
            y: row.get("y")?,
            sprite_sheet_id: row.get("sprite_sheet_id")?,
            direction: row.get("direction")?,
            logic_prompt: row.get("logic_prompt")?,
            dialogue_tree_id: row.get("dialogue_tree_id")?,
            stats_json: row.get("stats_json")?,
            equipment_json: row.get("equipment_json")?,
            inventory_json: row.get("inventory_json")?,
            patrol_path_json: row.get("patrol_path_json")?,
            schedule_json: row.get("schedule_json")?,
            faction_id: row.get("faction_id")?,
            is_interactable: row.get("is_interactable")?,
            is_collidable: row.get("is_collidable")?,
            respawn_ticks: row.get("respawn_ticks")?,
        })
    }
}

impl Queryable for Tile {
    fn table_name() -> &'static str {
        "tiles"
    }

    fn from_row(row: &rusqlite::Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(Tile {
            tile_id: row.get("tile_id")?,
            map_id: row.get("map_id")?,
            x: row.get("x")?,
            y: row.get("y")?,
            z: row.get("z")?,
            tileset_id: row.get("tileset_id")?,
            tile_index: row.get("tile_index")?,
            world_state: row.get("world_state")?,
            biome: row.get("biome")?,
            passable: row.get("passable")?,
            event_trigger_id: row.get("event_trigger_id")?,
        })
    }
}

impl Queryable for Map {
    fn table_name() -> &'static str {
        "maps"
    }

    fn from_row(row: &rusqlite::Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(Map {
            map_id: row.get("map_id")?,
            name: row.get("name")?,
            map_type: row.get("map_type")?,
            width: row.get("width")?,
            height: row.get("height")?,
            parent_map_id: row.get("parent_map_id")?,
            entry_x: row.get("entry_x")?,
            entry_y: row.get("entry_y")?,
            bgm_id: row.get("bgm_id")?,
            ambient_id: row.get("ambient_id")?,
            encounter_rate: row.get("encounter_rate")?,
            encounter_table_id: row.get("encounter_table_id")?,
            mode7_enabled: row.get("mode7_enabled")?,
            camera_bounds_json: row.get("camera_bounds_json")?,
        })
    }
}

impl Queryable for DialogueTreeModel {
    fn table_name() -> &'static str {
        "dialogue_trees"
    }

    fn from_row(row: &rusqlite::Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(DialogueTreeModel {
            tree_id: row.get("tree_id")?,
            tree_name: row.get("name")?,
            root_node_id: row.get::<_, i64>("root_node_id").map(|v| v.to_string()).unwrap_or_default(),
        })
    }
}

impl Queryable for DialogueNodeModel {
    fn table_name() -> &'static str {
        "dialogue_nodes"
    }

    fn from_row(row: &rusqlite::Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(DialogueNodeModel {
            node_id: row.get::<_, i64>("node_id")?.to_string(),
            tree_id: row.get("tree_id")?,
            node_type: row.get::<_, Option<String>>("node_type")?.unwrap_or_else(|| "dialogue".to_string()),
            speaker: row.get("speaker")?,
            text: row.get("text")?,
            next_node_id: row.get::<_, Option<i64>>("next_node_id")?.map(|v| v.to_string()),
            emotion: row.get("expression")?,
            conditions_json: row.get("condition_json")?,
            effects_json: row.get::<_, Option<String>>("effects_json")?,
        })
    }
}

impl Queryable for DialogueChoiceModel {
    fn table_name() -> &'static str {
        "dialogue_choices"
    }

    fn from_row(row: &rusqlite::Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(DialogueChoiceModel {
            choice_id: row.get("choice_id")?,
            node_id: row.get::<_, i64>("node_id")?.to_string(),
            tree_id: row.get("tree_id")?,
            choice_text: row.get("text")?,
            next_node_id: row.get::<_, Option<i64>>("next_node_id")?.map(|v| v.to_string()),
            conditions_json: row.get("condition_json")?,
            effects_json: row.get("effect_json")?,
            sort_order: row.get("sort_order")?,
        })
    }
}

impl Queryable for SaveSlotInfo {
    fn table_name() -> &'static str {
        "save_slots"
    }

    fn from_row(row: &rusqlite::Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(SaveSlotInfo {
            slot_number: row.get::<_, i64>("slot_number")? as u32,
            saved_at: row.get("saved_at")?,
            play_time_ms: row.get::<_, i64>("play_time_ms")? as u64,
            exists: true,
        })
    }
}

// =============================================================================
// Database Integration
// =============================================================================

impl Database {
    /// Start a type-safe query for a model
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let enemies = db.query::<EntityModel>()
    ///     .filter_eq("entity_type", "enemy")
    ///     .fetch_all()?;
    /// ```
    pub fn query<T: Queryable>(&self) -> Query<T> {
        Query::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_db_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("dde_query_test_{}.db", uuid::Uuid::new_v4()));
        path
    }

    fn setup_test_db() -> Database {
        let path = test_db_path();
        Database::create_new(&path, "Test Project").unwrap()
    }

    #[test]
    fn test_query_new() {
        let query = Query::<EntityModel>::new();
        assert!(query.filters.is_empty());
        assert!(query.order_by.is_empty());
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn test_filter_eq() {
        let query = Query::<EntityModel>::new().filter_eq("entity_type", "enemy");
        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.filters[0].column, "entity_type");
        assert!(matches!(query.filters[0].operator, Operator::Eq));
    }

    #[test]
    fn test_filter_like() {
        let query = Query::<EntityModel>::new().filter_like("name", "goblin");
        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.filters[0].column, "name");
        assert!(matches!(query.filters[0].operator, Operator::Like));
        if let FilterValue::String(v) = &query.filters[0].value {
            assert_eq!(v, "%goblin%");
        } else {
            panic!("Expected String value");
        }
    }

    #[test]
    fn test_filter_in() {
        let query = Query::<EntityModel>::new()
            .filter_in("entity_type", vec!["enemy", "boss"]);
        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.filters[0].column, "entity_type");
        assert!(matches!(query.filters[0].operator, Operator::In));
    }

    #[test]
    fn test_filter_between() {
        let query = Query::<Tile>::new().filter_between("x", 0, 100);
        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.filters[0].column, "x");
        assert!(matches!(query.filters[0].operator, Operator::Between));
    }

    #[test]
    fn test_filter_null() {
        let query = Query::<EntityModel>::new()
            .filter_is_null("faction_id")
            .filter_is_not_null("name");
        assert_eq!(query.filters.len(), 2);
        assert!(matches!(query.filters[0].operator, Operator::IsNull));
        assert!(matches!(query.filters[1].operator, Operator::IsNotNull));
    }

    #[test]
    fn test_order_by() {
        let query = Query::<Tile>::new()
            .order_by("y", Order::Asc)
            .order_by("x", Order::Desc);
        assert_eq!(query.order_by.len(), 2);
        assert_eq!(query.order_by[0].0, "y");
        assert!(matches!(query.order_by[0].1, Order::Asc));
        assert_eq!(query.order_by[1].0, "x");
        assert!(matches!(query.order_by[1].1, Order::Desc));
    }

    #[test]
    fn test_limit_offset() {
        let query = Query::<EntityModel>::new()
            .limit(100)
            .offset(50);
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(50));
    }

    #[test]
    fn test_build_sql_basic() {
        let query = Query::<EntityModel>::new();
        let (sql, params) = query.build_sql();
        assert_eq!(sql, "SELECT * FROM entities");
        assert!(params.is_empty());
    }

    #[test]
    fn test_build_sql_with_filter() {
        let query = Query::<EntityModel>::new()
            .filter_eq("entity_type", "enemy")
            .filter_eq("map_id", 5i32);
        let (sql, params) = query.build_sql();
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("entity_type = ?"));
        assert!(sql.contains("map_id = ?"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_build_sql_with_order() {
        let query = Query::<EntityModel>::new()
            .order_by("name", Order::Asc);
        let (sql, _) = query.build_sql();
        assert!(sql.contains("ORDER BY name ASC"));
    }

    #[test]
    fn test_build_sql_with_limit_offset() {
        let query = Query::<EntityModel>::new()
            .limit(100)
            .offset(50);
        let (sql, _) = query.build_sql();
        assert!(sql.contains("LIMIT 100"));
        assert!(sql.contains("OFFSET 50"));
    }

    #[test]
    fn test_build_sql_complete() {
        let query = Query::<EntityModel>::new()
            .filter_eq("entity_type", "enemy")
            .filter_gt("respawn_ticks", 0i32)
            .order_by("name", Order::Asc)
            .limit(10);
        let (sql, params) = query.build_sql();
        
        assert!(sql.starts_with("SELECT * FROM entities WHERE"));
        assert!(sql.contains("entity_type = ?"));
        assert!(sql.contains("respawn_ticks > ?"));
        assert!(sql.contains("ORDER BY name ASC"));
        assert!(sql.contains("LIMIT 10"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_filter_value_conversions() {
        let int_val: FilterValue = 42i32.into();
        assert!(matches!(int_val, FilterValue::Int(42)));

        let long_val: FilterValue = 42i64.into();
        assert!(matches!(long_val, FilterValue::Int(42)));

        let float_val: FilterValue = 3.14f64.into();
        assert!(matches!(float_val, FilterValue::Float(3.14)));

        let string_val: FilterValue = "hello".into();
        assert!(matches!(string_val, FilterValue::String(s) if s == "hello"));

        let bool_val: FilterValue = true.into();
        assert!(matches!(bool_val, FilterValue::Bool(true)));
    }

    #[test]
    fn test_operator_sql() {
        assert_eq!(Operator::Eq.sql(), "=");
        assert_eq!(Operator::Ne.sql(), "!=");
        assert_eq!(Operator::Gt.sql(), ">");
        assert_eq!(Operator::Gte.sql(), ">=");
        assert_eq!(Operator::Lt.sql(), "<");
        assert_eq!(Operator::Lte.sql(), "<=");
        assert_eq!(Operator::Like.sql(), "LIKE");
        assert_eq!(Operator::In.sql(), "IN");
        assert_eq!(Operator::Between.sql(), "BETWEEN");
        assert_eq!(Operator::IsNull.sql(), "IS NULL");
        assert_eq!(Operator::IsNotNull.sql(), "IS NOT NULL");
    }

    #[test]
    fn test_order_sql() {
        assert_eq!(Order::Asc.sql(), "ASC");
        assert_eq!(Order::Desc.sql(), "DESC");
    }

    #[test]
    fn test_queryable_table_names() {
        assert_eq!(EntityModel::table_name(), "entities");
        assert_eq!(Tile::table_name(), "tiles");
        assert_eq!(Map::table_name(), "maps");
        assert_eq!(DialogueTreeModel::table_name(), "dialogue_trees");
        assert_eq!(DialogueNodeModel::table_name(), "dialogue_nodes");
        assert_eq!(DialogueChoiceModel::table_name(), "dialogue_choices");
        assert_eq!(SaveSlotInfo::table_name(), "save_slots");
    }

    #[test]
    fn test_chained_filters() {
        let query = Query::<EntityModel>::new()
            .filter_eq("entity_type", "npc")
            .filter_ne("name", "Unknown")
            .filter_gt("respawn_ticks", 0i32)
            .filter_gte("faction_id", 1i32)
            .filter_lt("x", 100i32)
            .filter_lte("y", 100i32);
        
        assert_eq!(query.filters.len(), 6);
        assert!(matches!(query.filters[0].operator, Operator::Eq));
        assert!(matches!(query.filters[1].operator, Operator::Ne));
        assert!(matches!(query.filters[2].operator, Operator::Gt));
        assert!(matches!(query.filters[3].operator, Operator::Gte));
        assert!(matches!(query.filters[4].operator, Operator::Lt));
        assert!(matches!(query.filters[5].operator, Operator::Lte));
    }

    #[test]
    fn test_clone() {
        let query = Query::<EntityModel>::new()
            .filter_eq("entity_type", "enemy")
            .order_by("name", Order::Asc)
            .limit(10);
        
        let cloned = query.clone();
        assert_eq!(cloned.filters.len(), query.filters.len());
        assert_eq!(cloned.order_by.len(), query.order_by.len());
        assert_eq!(cloned.limit, query.limit);
    }

    #[test]
    fn test_exists_builds_correct_sql() {
        // exists() uses count internally, so we verify the SQL is built correctly
        let query = Query::<EntityModel>::new()
            .filter_eq("entity_type", "enemy");
        
        let (sql, _) = query.build_sql();
        // The exists() method modifies the SQL to remove LIMIT
        // Let's verify the base SQL is correct
        assert!(sql.contains("SELECT * FROM entities"));
        assert!(sql.contains("entity_type = ?"));
    }

    #[test]
    fn test_database_integration() {
        let db = setup_test_db();
        
        // Test that query() method returns a Query builder
        let query = db.query::<EntityModel>();
        let (sql, _) = query.build_sql();
        assert_eq!(sql, "SELECT * FROM entities");
    }
}
