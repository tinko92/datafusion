// Test overflow optimization cases
use datafusion::prelude::*;
use datafusion::error::Result;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::array::{Int32Array, StringArray};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    // Create a test table with data that can cause overflow
    ctx.register_batch("test_table", create_test_batch()?)?;

    println!("=== Testing Overflow Optimizations ===\n");

    // Test case 1: a < a + 1 (should be optimized to true, but careful with overflow!)
    println!("🔍 Test 1: age < age + 1");
    println!("Mathematical expectation: This should always be true");
    println!("But with overflow: if age = i32::MAX, age + 1 = i32::MIN, so age < age + 1 becomes false!");
    println!();

    let df = ctx.sql("EXPLAIN VERBOSE SELECT * FROM test_table WHERE age < age + 1").await?;
    df.show().await?;
    println!();

    // Test case 2: a + 1 < a (should be optimized to false, but careful with overflow!)
    println!("🔍 Test 2: age + 1 < age");
    println!("Mathematical expectation: This should always be false");
    println!("But with overflow: if age = i32::MAX, age + 1 = i32::MIN, so age + 1 < age becomes true!");
    println!();

    let df = ctx.sql("EXPLAIN VERBOSE SELECT * FROM test_table WHERE age + 1 < age").await?;
    df.show().await?;
    println!();

    // Test case 3: Test with potential overflow values
    println!("🔍 Test 3: Testing with max int32 values");
    let df = ctx.sql("EXPLAIN VERBOSE SELECT * FROM test_table WHERE large_int < large_int + 1").await?;
    df.show().await?;
    println!();

    // Test case 4: Show the actual data to see what values we're working with
    println!("📊 Test 4: Show actual data values");
    let df = ctx.sql("SELECT age, large_int, age < age + 1 as age_plus1_check, age + 1 < age as plus1_age_check FROM test_table").await?;
    df.show().await?;

    Ok(())
}

fn create_test_batch() -> Result<RecordBatch> {
    let schema = Schema::new(vec![
        Field::new("age", DataType::Int32, false),
        Field::new("large_int", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
    ]);

    // Include values that might cause overflow
    let age = Int32Array::from(vec![
        25, 30, 45,
        i32::MAX,  // This will overflow when +1 is added: 2147483647 + 1 = -2147483648
        -10, 0
    ]);

    let large_int = Int32Array::from(vec![
        100, 200, 300,
        i32::MAX,  // Max int32 value
        i32::MIN,  // Min int32 value
        0
    ]);

    let name = StringArray::from(vec![
        "Alice", "Bob", "Charlie", "MaxValue", "MinValue", "Zero"
    ]);

    let batch = RecordBatch::try_new(Arc::new(schema), vec![
        Arc::new(age),
        Arc::new(large_int),
        Arc::new(name),
    ])?;

    Ok(batch)
}