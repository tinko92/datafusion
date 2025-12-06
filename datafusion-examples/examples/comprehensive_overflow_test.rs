// Comprehensive Overflow Optimization Test Suite
// Tests the hierarchical optimization strategy for different integer types
use datafusion::prelude::*;
use datafusion::error::Result;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::array::{Int8Array, Int16Array, Int32Array, Int64Array, StringArray};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    println!("=== Comprehensive Overflow Optimization Test Suite ===\n");
    println!("Testing hierarchical optimization strategy:");
    println!("- Int8/Int16/Int32: Should be optimized (assume safe promotion to Int64)");
    println!("- Int64: Should NOT be optimized (conservative approach)\n");

    // Create test tables for different integer types
    ctx.register_batch("int8_table", create_int8_batch()?)?;
    ctx.register_batch("int16_table", create_int16_batch()?)?;
    ctx.register_batch("int32_table", create_int32_batch()?)?;
    ctx.register_batch("int64_table", create_int64_batch()?)?;

    // Test 1: Int8 - Should be optimized now!
    println!("🔍 Test 1: Int8 - small_int < small_int + 1");
    println!("Expected: Should be optimized to Boolean(true)");
    test_optimization(&ctx, "int8_table", "small_int", "Int8").await?;
    println!();

    // Test 2: Int16 - Should be optimized now!
    println!("🔍 Test 2: Int16 - medium_int < medium_int + 1");
    println!("Expected: Should be optimized to Boolean(true)");
    test_optimization(&ctx, "int16_table", "medium_int", "Int16").await?;
    println!();

    // Test 3: Int32 - Should be optimized now!
    println!("🔍 Test 3: Int32 - age < age + 1");
    println!("Expected: Should be optimized to Boolean(true)");
    test_optimization(&ctx, "int32_table", "age", "Int32").await?;
    println!();

    // Test 4: Int64 - Should NOT be optimized (conservative)
    println!("🔍 Test 4: Int64 - big_int < big_int + 1");
    println!("Expected: Should NOT be optimized (remain as filter)");
    test_optimization(&ctx, "int64_table", "big_int", "Int64").await?;
    println!();

    // Test 5: Edge case - Int32 with MAX value
    println!("🔍 Test 5: Int32 MAX - int32_max < int32_max + 1");
    println!("Expected: Should be optimized (Int32 MAX is safe in Int64 context)");
    test_specific_case(&ctx, "int32_table", "int32_max", "Int32_MAX").await?;
    println!();

    // Test 6: Edge case - Int64 with MAX value
    println!("🔍 Test 6: Int64 MAX - int64_max < int64_max + 1");
    println!("Expected: Should NOT be optimized (safety first for Int64)");
    test_specific_case(&ctx, "int64_table", "int64_max", "Int64_MAX").await?;
    println!();

    // Test 7: Reverse direction tests
    println!("🔍 Test 7: Reverse direction - x + 1 < x");
    println!("Expected: Should be optimized to Boolean(false) for small types, NOT for Int64");
    test_reverse_optimization(&ctx).await?;
    println!();

    // Test 8: Show actual execution results
    println!("📊 Test 8: Actual execution results comparison");
    show_execution_results(&ctx).await?;

    Ok(())
}

async fn test_optimization(
    ctx: &SessionContext,
    table_name: &str,
    column_name: &str,
    data_type: &str,
) -> Result<()> {
    let sql = format!(
        "EXPLAIN VERBOSE SELECT * FROM {} WHERE {} < {} + 1",
        table_name, column_name, column_name
    );

    println!("SQL: {}", sql);

    let df = ctx.sql(&sql).await?;
    let results = df.collect().await?;

    // Find the simplify_expressions row
    let mut optimized = false;
    for batch in results {
        for row in 0..batch.num_rows() {
            let plan_type = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(row);

            let plan = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(row);

            if plan_type.contains("simplify_expressions") {
                println!("After simplify_expressions: {}", plan);
                if plan.contains("Boolean(true)") {
                    optimized = true;
                    println!("✅ {} OPTIMIZED to Boolean(true)", data_type);
                } else if plan.contains("<") {
                    println!("❌ {} NOT optimized (kept as filter)", data_type);
                }
                break;
            }
        }
    }

    if !optimized && (data_type == "Int8" || data_type == "Int16" || data_type == "Int32") {
        println!("⚠️  WARNING: {} was expected to be optimized but wasn't!", data_type);
    }

    Ok(())
}

async fn test_specific_case(
    ctx: &SessionContext,
    table_name: &str,
    column_name: &str,
    case_name: &str,
) -> Result<()> {
    let _sql = format!(
        "EXPLAIN VERBOSE * FROM {} WHERE {} < {} + 1 AND {} = ?",
        table_name, column_name, column_name, column_name
    );

    println!("Testing edge case: {}", case_name);

    // For specific values, we'll use a different approach
    let filter_sql = format!(
        "SELECT COUNT(*) as count FROM {} WHERE {} < {} + 1",
        table_name, column_name, column_name
    );

    let df = ctx.sql(&filter_sql).await?;
    let results = df.collect().await?;

    if let Some(batch) = results.first() {
        let count = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(0);

        println!("Rows returned: {}", count);

        if count == 0 && (table_name == "int32_table") {
            println!("✅ Optimized: Empty result suggests Boolean(false) was applied");
        } else if count > 0 && (table_name == "int64_table") {
            println!("✅ Conservative optimization: All rows returned, no optimization applied");
        }
    }

    Ok(())
}

async fn test_reverse_optimization(ctx: &SessionContext) -> Result<()> {
    let test_cases = vec![
        ("int8_table", "small_int", "Int8"),
        ("int32_table", "age", "Int32"),
        ("int64_table", "big_int", "Int64"),
    ];

    for (table, column, data_type) in test_cases {
        let sql = format!(
            "EXPLAIN VERBOSE SELECT * FROM {} WHERE {} + 1 < {}",
            table, column, column
        );

        println!("Testing reverse for {}: {}", data_type, sql);

        let df = ctx.sql(&sql).await?;
        let results = df.collect().await?;

        for batch in results {
            for row in 0..batch.num_rows() {
                let plan_type = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .value(row);

                let plan = batch
                    .column(1)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .value(row);

                if plan_type.contains("simplify_expressions") {
                    if plan.contains("Boolean(false)") {
                        println!("✅ {}: x + 1 < x optimized to Boolean(false)", data_type);
                    } else if plan.contains("Boolean(true)") {
                        println!("🔥 {}: x + 1 < x optimized to Boolean(true) [WRONG!]", data_type);
                    } else {
                        println!("❌ {}: x + 1 < x NOT optimized", data_type);
                    }
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn show_execution_results(ctx: &SessionContext) -> Result<()> {
    println!("Comparing actual execution results:");

    // Int32 results
    let df = ctx.sql("SELECT age, age < age + 1 as result FROM int32_table").await?;
    df.show().await?;
    println!();

    // Int64 results
    let df = ctx.sql("SELECT big_int, big_int < big_int + 1 as result FROM int64_table").await?;
    df.show().await?;

    Ok(())
}

// Create test data for different integer types
fn create_int8_batch() -> Result<RecordBatch> {
    let schema = Schema::new(vec![
        Field::new("small_int", DataType::Int8, false),
        Field::new("name", DataType::Utf8, false),
    ]);

    let small_int = Int8Array::from(vec![
        10i8, 50i8, i8::MAX, // MAX = 127
        -5i8, 0i8
    ]);

    let name = StringArray::from(vec![
        "Normal", "Medium", "Int8_MAX", "Minus", "Zero"
    ]);

    Ok(RecordBatch::try_new(Arc::new(schema), vec![
        Arc::new(small_int),
        Arc::new(name),
    ])?)
}

fn create_int16_batch() -> Result<RecordBatch> {
    let schema = Schema::new(vec![
        Field::new("medium_int", DataType::Int16, false),
        Field::new("name", DataType::Utf8, false),
    ]);

    let medium_int = Int16Array::from(vec![
        1000i16, 2000i16, i16::MAX, // MAX = 32767
        -100i16, 0i16
    ]);

    let name = StringArray::from(vec![
        "Low", "High", "Int16_MAX", "Negative", "Zero"
    ]);

    Ok(RecordBatch::try_new(Arc::new(schema), vec![
        Arc::new(medium_int),
        Arc::new(name),
    ])?)
}

fn create_int32_batch() -> Result<RecordBatch> {
    let schema = Schema::new(vec![
        Field::new("age", DataType::Int32, false),
        Field::new("int32_max", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
    ]);

    let age = Int32Array::from(vec![
        25i32, 30i32, 100i32, -10i32, 0i32
    ]);

    let int32_max = Int32Array::from(vec![
        i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX
    ]);

    let name = StringArray::from(vec![
        "Adult", "Adult2", "Senior", "Young", "Zero"
    ]);

    Ok(RecordBatch::try_new(Arc::new(schema), vec![
        Arc::new(age),
        Arc::new(int32_max),
        Arc::new(name),
    ])?)
}

fn create_int64_batch() -> Result<RecordBatch> {
    let schema = Schema::new(vec![
        Field::new("big_int", DataType::Int64, false),
        Field::new("int64_max", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
    ]);

    let big_int = Int64Array::from(vec![
        1000i64, 1000000i64, i64::MAX - 1, -1000i64, 0i64
    ]);

    let int64_max = Int64Array::from(vec![
        i64::MAX, i64::MAX, i64::MAX, i64::MAX, i64::MAX
    ]);

    let name = StringArray::from(vec![
        "Thousand", "Million", "NearMax", "Negative", "Zero"
    ]);

    Ok(RecordBatch::try_new(Arc::new(schema), vec![
        Arc::new(big_int),
        Arc::new(int64_max),
        Arc::new(name),
    ])?)
}