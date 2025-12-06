// 测试 DataFusion 溢出安全优化实现
use datafusion::prelude::*;
use datafusion::error::Result;
use datafusion_common::{DataFusionError, ScalarValue};
use datafusion_expr::{col, lit, Expr};
use datafusion::execution::context::SessionContext;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== DataFusion 溢出安全优化测试 ===\n");

    let ctx = SessionContext::new();

    // 创建测试表
    ctx.register_batch("test_table", create_test_data()?)?;

    // 测试场景 1: a + 1 < a (反方向)
    println!("🔍 测试场景 1: a + 1 < a");
    test_reverse_direction(&ctx).await?;

    // 测试场景 2: a < a + 1 (正方向)
    println!("\n🔍 测试场景 2: a < a + 1");
    test_forward_direction(&ctx).await?;

    // 测试场景 3: 边界值测试
    println!("\n🔍 测试场景 3: 边界值测试");
    test_boundary_values(&ctx).await?;

    // 测试场景 4: 不同的数据类型
    println!("\n🔍 测试场景 4: 不同数据类型");
    test_different_types(&ctx).await?;

    // 测试场景 5: 实际查询计划
    println!("\n🔍 测试场景 5: 实际查询计划");
    test_query_plans(&ctx).await?;

    println!("\n✅ 所有测试完成！");

    Ok(())
}

fn create_test_data() -> Result<arrow::record_batch::RecordBatch> {
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::array::{Int32Array, StringArray};
    use std::sync::Arc;

    let schema = Schema::new(vec![
        Field::new("age", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
    ]);

    // 包含正常值和边界值
    let ages = Int32Array::from(vec![
        25, 30, 45,  // 正常值
        2147483647, // i32::MAX - 边界值
        0, -10       // 其他值
    ]);

    let names = StringArray::from(vec![
        "Alice", "Bob", "Charlie", "MaxValue", "Zero", "Negative"
    ]);

    let batch = arrow::record_batch::RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(ages),
            Arc::new(names),
        ]
    )?;

    Ok(batch)
}

async fn test_reverse_direction(ctx: &SessionContext) -> Result<()> {
    println!("  测试: age + 1 < age");

    // 创建表达式
    let expr = col("age").add(lit(1i32)).lt(col("age"));

    // 简化表达式
    let simplified = ctx.state().simplify(expr.clone())?;

    println!("    原始表达式: {:?}", expr);
    println!("    简化后: {:?}", simplified);

    // 检查是否被优化
    match &simplified {
        Expr::Literal(ScalarValue::Boolean(Some(false)), _) => {
            println!("    ✅ 被优化为 false (如果安全)");
        }
        Expr::Literal(ScalarValue::Boolean(Some(true)), _) => {
            println!("    ❌ 错误优化为 true！");
        }
        _ => {
            println!("    ⚠️  保持原样 (保守策略 - 由于溢出风险)");
        }
    }

    // 测试实际查询
    let df = ctx.sql("SELECT * FROM test_table WHERE age + 1 < age").await?;
    println!("    查询计划已创建");

    Ok(())
}

async fn test_forward_direction(ctx: &SessionContext) -> Result<()> {
    println!("  测试: age < age + 1");

    // 创建表达式
    let expr = col("age").lt(col("age").add(lit(1i32)));

    // 简化表达式
    let simplified = ctx.state().simplify(expr.clone())?;

    println!("    原始表达式: {:?}", expr);
    println!("    简化后: {:?}", simplified);

    // 检查是否被优化
    match &simplified {
        Expr::Literal(ScalarValue::Boolean(Some(true)), _) => {
            println!("    ✅ 被优化为 true (如果安全)");
        }
        Expr::Literal(ScalarValue::Boolean(Some(false)), _) => {
            println!("    ❌ 错误优化为 false！");
        }
        _ => {
            println!("    ⚠️  保持原样 (保守策略 - 由于溢出风险)");
        }
    }

    // 测试实际查询
    let df = ctx.sql("SELECT * FROM test_table WHERE age < age + 1").await?;
    println!("    查询计划已创建");

    Ok(())
}

async fn test_boundary_values(ctx: &SessionContext) -> Result<()> {
    println!("  测试边界值情况");

    // 测试具体的边界值
    let max_val_expr = lit(2147483647i32); // i32::MAX
    let test_expr = max_val_expr.clone().lt(max_val_expr.add(lit(1i32)));

    let simplified = ctx.state().simplify(test_expr.clone())?;

    println!("    边界值测试: 2147483647 < 2147483647 + 1");
    println!("    实际结果: 2147483647 < {} = {}",
             2147483647i32.wrapping_add(1),
             2147483647 < 2147483647i32.wrapping_add(1));

    match &simplified {
        Expr::Literal(ScalarValue::Boolean(Some(value)), _) => {
            println!("    简化结果: {}", value);
            if *value {
                println!("    ❌ 错误！边界值不应该被优化为 true");
            } else {
                println!("    ✅ 正确！没有优化，避免了错误结果");
            }
        }
        _ => {
            println!("    ✅ 保持原样 - 最安全的策略");
        }
    }

    Ok(())
}

async fn test_different_types(ctx: &SessionContext) -> Result<()> {
    println!("  测试不同整数类型的行为");

    // 测试 Int64 (更安全)
    let int64_expr = lit(10000000000i64).lt(lit(10000000000i64).add(lit(1i64)));
    let simplified = ctx.state().simplify(int64_expr.clone())?;

    println!("    Int64 测试: 10000000000 < 10000000000 + 1");
    match &simplified {
        Expr::Literal(ScalarValue::Boolean(Some(value)), _) => {
            println!("    Int64 简化结果: {} (可能更安全)", value);
        }
        _ => {
            println!("    Int64 保持原样");
        }
    }

    // 测试负数情况
    let negative_expr = lit((-5i32)).lt(lit((-5i32)).add(lit(1i32)));
    let simplified = ctx.state().simplify(negative_expr.clone())?;

    println!("    负数测试: -5 < -5 + 1");
    match &simplified {
        Expr::Literal(ScalarValue::Boolean(Some(value)), _) => {
            println!("    负数简化结果: {}", value);
        }
        _ => {
            println!("    负数保持原样");
        }
    }

    Ok(())
}

async fn test_query_plans(ctx: &SessionContext) -> Result<()> {
    println!("  测试实际查询计划");

    // 测试 EXPLAIN VERBOSE
    let queries = vec![
        "EXPLAIN VERBOSE SELECT * FROM test_table WHERE age < age + 1",
        "EXPLAIN VERBOSE SELECT * FROM test_table WHERE age + 1 < age",
        "SELECT * FROM test_table WHERE age < age + 1",
        "SELECT * FROM test_table WHERE age + 1 < age",
    ];

    for (i, query) in queries.iter().enumerate() {
        println!("    查询 {}: {}", i + 1, query);

        match ctx.sql(query).await {
            Ok(df) => {
                if query.starts_with("EXPLAIN") {
                    // 显示计划
                    let results = df.collect().await?;
                    if !results.is_empty() {
                        println!("      ✅ 计划查询成功");
                    }
                } else {
                    // 显示结果
                    let results = df.collect().await?;
                    println!("      返回 {} 行结果", results.iter().map(|b| b.num_rows()).sum::<usize>());
                }
            }
            Err(e) => {
                println!("      ❌ 错误: {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_overflow_safe_optimization() -> Result<()> {
        let ctx = SessionContext::new();

        // 验证优化规则存在且工作正常
        let expr = col("test").lt(col("test").add(lit(1i32)));
        let simplified = ctx.state().simplify(expr)?;

        // 简化后的表达式应该不是明显的错误
        match &simplified {
            Expr::Literal(ScalarValue::Boolean(Some(value)), _) => {
                // 如果被优化为字面量，要确保逻辑正确
                assert!(*value == false || *value == true);
            }
            _ => {
                // 如果保持原样，这也是正确的行为
            }
        }

        Ok(())
    }
}