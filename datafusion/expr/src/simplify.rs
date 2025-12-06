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

//! Structs and traits to provide the information needed for expression simplification.

use arrow::datatypes::DataType;
use datafusion_common::{internal_datafusion_err, DFSchemaRef, Result, Column};
use datafusion_common::type_tracker::{OriginalTypeInfo, OriginalTypeTracker};

use crate::{execution_props::ExecutionProps, Expr, ExprSchemable};

/// Provides the information necessary to apply algebraic simplification to an
/// [Expr]. See [SimplifyContext] for one concrete implementation.
///
/// This trait exists so that other systems can plug schema
/// information in without having to create `DFSchema` objects. If you
/// have a [`DFSchemaRef`] you can use [`SimplifyContext`]
pub trait SimplifyInfo {
    /// Returns true if this Expr has boolean type
    fn is_boolean_type(&self, expr: &Expr) -> Result<bool>;

    /// Returns true of this expr is nullable (could possibly be NULL)
    fn nullable(&self, expr: &Expr) -> Result<bool>;

    /// Returns details needed for partial expression evaluation
    fn execution_props(&self) -> &ExecutionProps;

    /// Returns data type of this expr needed for determining optimized int type of a value
    fn get_data_type(&self, expr: &Expr) -> Result<DataType>;

    /// Get original type information for a column before coercion
    fn get_original_type_info(&self, column: &Column) -> Result<Option<OriginalTypeInfo>>;

    /// Check if an expression was originally a small integer type (Int8/16/32)
    fn is_originally_small_int(&self, expr: &Expr) -> Result<bool>;
}

/// Provides simplification information based on DFSchema and
/// [`ExecutionProps`]. This is the default implementation used by DataFusion
///
/// # Example
/// See the `simplify_demo` in the [`expr_api` example]
///
/// [`expr_api` example]: https://github.com/apache/datafusion/blob/main/datafusion-examples/examples/query_planning/expr_api.rs
#[derive(Debug, Clone)]
pub struct SimplifyContext<'a> {
    schema: Option<DFSchemaRef>,
    props: &'a ExecutionProps,
    original_type_tracker: Option<OriginalTypeTracker>,
}

impl<'a> SimplifyContext<'a> {
    /// Create a new SimplifyContext
    pub fn new(props: &'a ExecutionProps) -> Self {
        Self {
            schema: None,
            props,
            original_type_tracker: None,
        }
    }

    /// Register a [`DFSchemaRef`] with this context
    pub fn with_schema(mut self, schema: DFSchemaRef) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Set the original type tracker for overflow-safe optimization
    pub fn with_original_type_tracker(mut self, tracker: OriginalTypeTracker) -> Self {
        self.original_type_tracker = Some(tracker);
        self
    }
}

impl SimplifyInfo for SimplifyContext<'_> {
    /// Returns true if this Expr has boolean type
    fn is_boolean_type(&self, expr: &Expr) -> Result<bool> {
        if let Some(schema) = &self.schema {
            if let Ok(DataType::Boolean) = expr.get_type(schema) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Returns true if expr is nullable
    fn nullable(&self, expr: &Expr) -> Result<bool> {
        let schema = self.schema.as_ref().ok_or_else(|| {
            internal_datafusion_err!("attempt to get nullability without schema")
        })?;
        expr.nullable(schema.as_ref())
    }

    /// Returns data type of this expr needed for determining optimized int type of a value
    fn get_data_type(&self, expr: &Expr) -> Result<DataType> {
        let schema = self.schema.as_ref().ok_or_else(|| {
            internal_datafusion_err!("attempt to get data type without schema")
        })?;
        expr.get_type(schema)
    }

    fn execution_props(&self) -> &ExecutionProps {
        self.props
    }

    fn get_original_type_info(&self, column: &Column) -> Result<Option<OriginalTypeInfo>> {
        if let Some(tracker) = &self.original_type_tracker {
            // Get current type first
            let current_type = self.get_data_type(&Expr::Column(column.clone()))?;
            Ok(tracker.get_original_type_info(column, &current_type))
        } else {
            Ok(None)
        }
    }

    fn is_originally_small_int(&self, expr: &Expr) -> Result<bool> {
        match expr {
            Expr::Column(col) => {
                if let Some(tracker) = &self.original_type_tracker {
                    let current_type = self.get_data_type(expr)?;
                    Ok(tracker.was_promoted_from_small_int(col, &current_type))
                } else {
                    Ok(false)
                }
            }
            Expr::Cast(cast_expr) => {
                // Recursively check if the cast expression contains a small int column
                self.is_originally_small_int(&cast_expr.expr)
            }
            Expr::Alias(alias) => {
                // Check the aliased expression
                self.is_originally_small_int(&alias.expr)
            }
            _ => Ok(false),
        }
    }
}

/// Was the expression simplified?
#[derive(Debug)]
pub enum ExprSimplifyResult {
    /// The function call was simplified to an entirely new Expr
    Simplified(Expr),
    /// The function call could not be simplified, and the arguments
    /// are return unmodified.
    Original(Vec<Expr>),
}
