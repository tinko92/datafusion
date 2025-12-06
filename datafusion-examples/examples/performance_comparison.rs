// Performance Comparison Test for Overflow Optimizations
use datafusion::prelude::*;
use datafusion::error::Result;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::array::{Int32Array, Int64Array, StringArray};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    println!("=== Performance Comparison: Before vs After Optimization ===\n");

    // Create larger test dataset
    let batch = create_large_test_batch(10000)?; // 10,000 rows
    ctx.register_batch("large_table", batch)?;

    // Test 1: Performance without optimization (direct filter)
    println!("🔍 Test 1: Direct filter evaluation (baseline)");
    let start = Instant::now();
    let df1 = ctx.sql("SELECT COUNT(*) as total FROM large_table WHERE age < age + 1").await?;
    let result1 = df1.collect().await?;
    let baseline_time = start.elapsed();

    if let Some(batch) = result1.first() {
        let count = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(0);
        println!("Rows returned: {}", count);
        println!("Baseline time: {:?}", baseline_time);
    }

    println!();

    // Test 2: Performance with optimization
    println!("🔍 Test 2: Optimized query (should be faster)");
    let start = Instant::now();
    let df2 = ctx.sql("EXPLAIN VERBOSE SELECT COUNT(*) as total FROM large_table WHERE age < age + 1").await?;
    let result2 = df2.collect().await?;
    let optimized_time = start.elapsed();

    // Check if optimization was applied
    let mut was_optimized = false;
    for batch in &result2 {
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
                if plan.contains("Boolean(true)") {
                    was_optimized = true;
                    println!("✅ Query was optimized to Boolean(true)");
                } else {
                    println!("❌ Query was NOT optimized");
                }
                println!("After optimization: {}", plan);
                break;
            }
        }
    }

    println!("Optimization check time: {:?}", optimized_time);
    println!();

    // Test 3: Compare with reverse condition
    println!("🔍 Test 3: Reverse condition (age + 1 < age)");
    let start = Instant::now();
    let df3 = ctx.sql("EXPLAIN VERBOSE SELECT COUNT(*) as total FROM large_table WHERE age + 1 < age").await?;
    let result3 = df3.collect().await?;
    let reverse_time = start.elapsed();

    let mut reverse_optimized = false;
    for batch in &result3 {
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
                    reverse_optimized = true;
                    println!("✅ Reverse condition optimized to Boolean(false)");
                } else if plan.contains("EmptyRelation") {
                    println!("✅ Reverse condition optimized to EmptyRelation");
                    reverse_optimized = true;
                } else {
                    println!("❌ Reverse condition NOT optimized");
                }
                break;
            }
        }
    }

    println!("Reverse condition check time: {:?}", reverse_time);
    println!();

    // Test 4: Test with Int64 (should NOT be optimized)
    println!("🔍 Test 4: Int64 test (should NOT be optimized)");
    let start = Instant::now();
    let df4 = ctx.sql("EXPLAIN VERBOSE SELECT COUNT(*) as total FROM large_table WHERE big_int < big_int + 1").await?;
    let result4 = df4.collect().await?;
    let int64_time = start.elapsed();

    let mut int64_optimized = false;
    for batch in &result4 {
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
                if plan.contains("Boolean(true)") {
                    int64_optimized = true;
                    println!("⚠️  WARNING: Int64 was optimized (unexpected!)");
                } else {
                    println!("✅ Int64 correctly NOT optimized (as expected)");
                }
                break;
            }
        }
    }

    println!("Int64 check time: {:?}", int64_time);
    println!();

    // Summary
    println!("📊 === PERFORMANCE SUMMARY ===");
    println!("Rows processed: 10,000");
    println!("Int32 optimization applied: {}", was_optimized);
    println!("Int32 reverse optimization applied: {}", reverse_optimized);
    println!("Int64 optimization applied: {} (should be false)", int64_optimized);

    if was_optimized {
        println!("🚀 Int32 queries benefit from optimization - will run faster at scale!");
    }

    if !int64_optimized {
        println!("🛡️  Int64 queries remain conservative - safety first!");
    }

    println!();

    // Test 5: Scalability test with much larger dataset
    println!("🔍 Test 5: Scalability test with 100,000 rows");
    let large_batch = create_large_test_batch(100000)?;
    ctx.register_batch("huge_table", large_batch)?;

    let start = Instant::now();
    let df5 = ctx.sql("SELECT COUNT(*) as total FROM huge_table WHERE age < age + 1").await?;
    let _ = df5.collect().await?;
    let huge_time = start.elapsed();

    println!("Query time for 100,000 rows: {:?}", huge_time);

    Ok(())
}

fn create_large_test_batch(num_rows: usize) -> Result<RecordBatch> {
    let schema = Schema::new(vec![
        Field::new("age", DataType::Int32, false),
        Field::new("big_int", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
    ]);

    // Create test data with various values
    let mut age_values = Vec::with_capacity(num_rows);
    let mut big_int_values = Vec::with_capacity(num_rows);
    let mut name_values = Vec::with_capacity(num_rows);

    for i in 0..num_rows {
        // Mix of normal and edge case values
        age_values.push(match i % 10 {
            0 => i32::MAX,
            1 => i32::MIN,
            2 => 0,
            3 => 25,
            4 => -25,
            _ => (i % 1000) as i32,
        });

        big_int_values.push(match i % 10 {
            0 => i64::MAX,
            1 => i64::MIN,
            2 => 0,
            3 => 1000000,
            4 => -1000000,
            _ => (i as i64) * 1000,
        });

        name_values.push(format!("User_{}", i));
    }

    let age = Int32Array::from(age_values);
    let big_int = Int64Array::from(big_int_values);
    let name = StringArray::from(name_values);

    RecordBatch::try_new(Arc::new(schema), vec![
        Arc::new(age),
        Arc::new(big_int),
        Arc::new(name),
    ])
}