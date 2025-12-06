// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Original type tracking for safe overflow optimization

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::{Column, Result};
use arrow::datatypes::DataType;

/// Information about original types available during simplification
#[derive(Debug, Clone)]
pub struct OriginalTypeInfo {
    /// Original type before any coercion
    pub original_type: DataType,
    /// Current coerced type (after type coercion)
    pub current_type: DataType,
    /// Whether the type was coerced (promoted)
    pub was_coerced: bool,
}

impl OriginalTypeInfo {
    /// Create new original type info
    pub fn new(original_type: DataType, current_type: DataType) -> Self {
        Self {
            was_coerced: original_type != current_type,
            original_type,
            current_type,
        }
    }

    /// Check if this was originally a small integer type (Int8/16/32)
    pub fn is_originally_small_int(&self) -> bool {
        matches!(self.original_type, DataType::Int8 | DataType::Int16 | DataType::Int32)
    }
}

/// Tracks original column types before type coercion
#[derive(Debug, Clone)]
pub struct OriginalTypeTracker {
    /// Maps column references to their original data types
    /// Key: (table_name, column_name), Value: original DataType
    original_types: Arc<RwLock<HashMap<(Option<String>, String), DataType>>>,

    /// Maps expression IDs to their original types (for complex expressions)
    expression_types: Arc<RwLock<HashMap<String, DataType>>>,
}

impl OriginalTypeTracker {
    /// Create a new empty type tracker
    pub fn new() -> Self {
        Self {
            original_types: Arc::new(RwLock::new(HashMap::new())),
            expression_types: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record the original type for a column
    pub fn record_original_type(
        &self,
        table_name: &Option<String>,
        column_name: &str,
        data_type: DataType,
    ) -> Result<()> {
        let key = (table_name.clone(), column_name.to_string());
        let mut types = self.original_types.write().unwrap();
        types.insert(key, data_type);
        Ok(())
    }

    /// Get the original type for a column
    pub fn get_original_type(&self, column: &Column) -> Option<DataType> {
        let table_name = column.relation.as_ref().map(|r| r.to_string());
        let key = (table_name, column.name.clone());
        let types = self.original_types.read().unwrap();
        types.get(&key).cloned()
    }

    /// Get original type info with current type comparison
    pub fn get_original_type_info(&self, column: &Column, current_type: &DataType) -> Option<OriginalTypeInfo> {
        if let Some(original_type) = self.get_original_type(column) {
            Some(OriginalTypeInfo::new(original_type, current_type.clone()))
        } else {
            None
        }
    }

    /// Check if a column was originally a small integer type (Int8/16/32)
    /// If so, it's safe to optimize since we know its original bounds
    pub fn was_promoted_from_small_int(&self, column: &Column, _current_type: &DataType) -> bool {
        if let Some(original_type) = self.get_original_type(column) {
            matches!(original_type, DataType::Int8 | DataType::Int16 | DataType::Int32)
        } else {
            false
        }
    }

    /// Record the original type for an expression (by unique identifier)
    pub fn record_expression_type(&self, expr_id: &str, data_type: DataType) -> Result<()> {
        let mut expr_types = self.expression_types.write().unwrap();
        expr_types.insert(expr_id.to_string(), data_type);
        Ok(())
    }

    /// Get the original type for an expression
    pub fn get_expression_original_type(&self, expr_id: &str) -> Option<DataType> {
        let expr_types = self.expression_types.read().unwrap();
        expr_types.get(expr_id).cloned()
    }

    /// Clear all tracked types (useful for query boundaries)
    pub fn clear(&self) {
        self.original_types.write().unwrap().clear();
        self.expression_types.write().unwrap().clear();
    }

    /// Get the number of tracked columns
    pub fn len(&self) -> usize {
        self.original_types.read().unwrap().len()
    }

    /// Check if any types are tracked
    pub fn is_empty(&self) -> bool {
        self.original_types.read().unwrap().is_empty()
    }
}

impl Default for OriginalTypeTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract a unique key for a column reference
/// Format: "table_name.column_name" or just "column_name" if no table
pub fn column_to_key(column: &Column) -> String {
    match &column.relation {
        Some(relation) => format!("{}.{}", relation, column.name),
        None => column.name.clone(),
    }
}

/// Parse a column key back into table and column components
pub fn parse_column_key(key: &str) -> (Option<String>, String) {
    if let Some((table, column)) = key.rsplit_once('.') {
        (Some(table.to_string()), column.to_string())
    } else {
        (None, key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TableReference;

    #[test]
    fn test_original_type_tracker() {
        let tracker = OriginalTypeTracker::new();

        // Test recording and retrieving types
        let table_name = Some("test_table".to_string());
        tracker.record_original_type(&table_name, "id", DataType::Int32).unwrap();

        let column = Column {
            relation: Some(TableReference::Bare {
                table: "test_table".into(),
            }),
            name: "id".to_string(),
            spans: Default::default(),
        };

        assert_eq!(tracker.get_original_type(&column), Some(DataType::Int32));
    }

    #[test]
    fn test_was_promoted_from_small_int() {
        let tracker = OriginalTypeTracker::new();

        let table_name = Some("test_table".to_string());
        tracker.record_original_type(&table_name, "age", DataType::Int32).unwrap();

        let column = Column {
            relation: Some(TableReference::Bare {
                table: "test_table".into(),
            }),
            name: "age".to_string(),
            spans: Default::default(),
        };

        // Test promotion detection
        assert!(tracker.was_promoted_from_small_int(&column, &DataType::Int64));
        assert!(!tracker.was_promoted_from_small_int(&column, &DataType::Int32));
    }

    #[test]
    fn test_column_key_operations() {
        let column_with_table = Column {
            relation: Some(TableReference::Bare {
                table: "users".into(),
            }),
            name: "age".to_string(),
            spans: Default::default(),
        };

        let column_without_table = Column {
            relation: None,
            name: "value".to_string(),
            spans: Default::default(),
        };

        assert_eq!(column_to_key(&column_with_table), "users.age");
        assert_eq!(column_to_key(&column_without_table), "value");

        assert_eq!(parse_column_key("users.age"), (Some("users".to_string()), "age".to_string()));
        assert_eq!(parse_column_key("value"), (None, "value".to_string()));
    }

    #[test]
    fn test_original_type_info() {
        let info = OriginalTypeInfo::new(DataType::Int32, DataType::Int64);
        assert!(info.was_coerced);
        assert!(info.is_originally_small_int());

        let info = OriginalTypeInfo::new(DataType::Int64, DataType::Int64);
        assert!(!info.was_coerced);
        assert!(!info.is_originally_small_int());
    }
}