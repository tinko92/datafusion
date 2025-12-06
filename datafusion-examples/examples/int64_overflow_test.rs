// Test Int64 overflow edge cases
use datafusion::prelude::*;
use datafusion::error::Result;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::array::{Int64Array, StringArray};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    // Create test data with Int64 boundary values
    ctx.register_batch("int64_test", create_int64_batch()?)?;

    println!("=== Testing Int64 Overflow Edge Cases ===\n");

    // Test case 1: Int64 MAX + 1 (will definitely overflow)
    println!("🔍 Test 1: Int64::MAX < Int64::MAX + 1");
    println!("Int64::MAX = {}", i64::MAX);
    println!("Int64::MAX + 1 = {} (OVERFLOW!)", i64::MAX.wrapping_add(1));
    println!();

    let df = ctx.sql("EXPLAIN VERBOSE SELECT * FROM int64_test WHERE big_int < big_int + 1").await?;
    df.show().await?;
    println!();

    // Test case 2: Show actual computation results
    println!("📊 Test 2: Actual computation results");
    let df = ctx.sql("SELECT
        big_int,
        big_int + 1 as big_int_plus1,
        big_int < big_int + 1 as should_be_true,
        big_int + 1 < big_int as should_be_false
    FROM int64_test").await?;
    df.show().await?;

    Ok(())
}

fn create_int64_batch() -> Result<RecordBatch> {
    let schema = Schema::new(vec![
        Field::new("big_int", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
    ]);

    // Test with Int64 boundary values
    let big_int = Int64Array::from(vec![
        100i64,                    // Normal value
        i64::MAX - 1,              // Near max
        i64::MAX,                  // Max int64 value - DANGEROUS!
        -1i64,                     // Negative value
        i64::MIN,                  // Min int64 value
        0i64,                      // Zero
    ]);

    let name = StringArray::from(vec![
        "Normal", "NearMax", "Int64_MAX", "MinusOne", "Int64_MIN", "Zero"
    ]);

    let batch = RecordBatch::try_new(Arc::new(schema), vec![
        Arc::new(big_int),
        Arc::new(name),
    ])?;

    Ok(batch)
}