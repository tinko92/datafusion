// Debug test to understand expression structure
use datafusion::prelude::*;
use datafusion::error::Result;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::array::{Int32Array, StringArray};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    println!("=== Debug Expression Structure ===\n");

    // Create test table
    let batch = create_test_batch()?;
    ctx.register_batch("test_table", batch)?;

    // Test 1: See the plan before and after type coercion
    println!("🔍 Test 1: Full plan analysis");
    let df = ctx.sql("EXPLAIN VERBOSE SELECT * FROM test_table WHERE age < age + 1").await?;
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

            println!("Stage: {}", plan_type);
            println!("Plan:  {}\n", plan);
        }
    }

    // Test 2: Try with explicit cast to see what happens
    println!("🔍 Test 2: Explicit cast");
    let df = ctx.sql("EXPLAIN VERBOSE SELECT * FROM test_table WHERE CAST(age AS Int64) < CAST(age AS Int64) + 1").await?;
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
                println!("After simplify_expressions: {}", plan);
            }
        }
    }

    Ok(())
}

fn create_test_batch() -> Result<RecordBatch> {
    let schema = Schema::new(vec![
        Field::new("age", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
    ]);

    let age = Int32Array::from(vec![25, 30, 100]);
    let name = StringArray::from(vec!["Alice", "Bob", "Charlie"]);

    Ok(RecordBatch::try_new(Arc::new(schema), vec![
        Arc::new(age),
        Arc::new(name),
    ])?)
}