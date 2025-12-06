use arrow::datatypes::DataType;
use arrow::error::ArrowError;
use datafusion_common::{DataFusionError, Result, ScalarValue};
use datafusion_expr::{Expr, Operator};
use datafusion_expr::simplify::SimplifyInfo;
use datafusion_expr::interval_arithmetic::Interval as DFInterval;

pub fn is_integer_type(dt: &DataType) -> bool {
    matches!(
        dt,
        DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
    )
}

fn full_integer_range(dt: &DataType) -> Result<DFInterval> {
    let (lower, upper) = match dt {
        DataType::Int16 => (
            ScalarValue::Int16(Some(i16::MIN)),
            ScalarValue::Int16(Some(i16::MAX)),
        ),
        DataType::Int8 => (
            ScalarValue::Int8(Some(i8::MIN)),
            ScalarValue::Int8(Some(i8::MAX)),
        ),
        DataType::Int32 => (
            ScalarValue::Int32(Some(i32::MIN)),
            ScalarValue::Int32(Some(i32::MAX)),
        ),
        DataType::Int64 => (
            ScalarValue::Int64(Some(i64::MIN)),
            ScalarValue::Int64(Some(i64::MAX)),
        ),
        DataType::UInt8 => (
            ScalarValue::UInt8(Some(u8::MIN)),
            ScalarValue::UInt8(Some(u8::MAX)),
        ),
        DataType::UInt16 => (
            ScalarValue::UInt16(Some(u16::MIN)),
            ScalarValue::UInt16(Some(u16::MAX)),
        ),
        DataType::UInt32 => (
            ScalarValue::UInt32(Some(u32::MIN)),
            ScalarValue::UInt32(Some(u32::MAX)),
        ),
        DataType::UInt64 => (
            ScalarValue::UInt64(Some(u64::MIN)),
            ScalarValue::UInt64(Some(u64::MAX)),
        ),
        other => return DFInterval::make_unbounded(other),
    };

    DFInterval::try_new(lower, upper)
}

fn unbounded_for(dt: &DataType) -> Result<DFInterval> {
    DFInterval::make_unbounded(dt)
}


fn safe_interval_op<F>(dt: &DataType, op: F) -> Result<DFInterval>
where
    F: FnOnce() -> Result<DFInterval>,
{
    match op() {
        Ok(iv) => Ok(iv),
        Err(DataFusionError::ArrowError(e,_))
            if matches!(e.as_ref(), ArrowError::ArithmeticOverflow(_)) =>
        {
            unbounded_for(dt)
        }
        Err(e) => Err(e),
    }
}

fn cast_scalar_int_to_type(value: &ScalarValue, to: &DataType) -> Result<ScalarValue> {
    match (value, to) {
        (ScalarValue::Int16(v), DataType::Int64) => Ok(ScalarValue::Int64(v.map(|x| x as i64))),
        (ScalarValue::Int16(v), DataType::Int32) => Ok(ScalarValue::Int32(v.map(|x| x as i32))),
        (ScalarValue::Int16(v), DataType::Int16) => Ok(ScalarValue::Int16(*v)),
        
        (ScalarValue::Int32(v), DataType::Int16) => Ok(ScalarValue::Int16(v.map(|x| x as i16))),
        (ScalarValue::Int32(v), DataType::Int32) => Ok(ScalarValue::Int32(*v)),
        (ScalarValue::Int32(v), DataType::Int64) => Ok(ScalarValue::Int64(v.map(|x| x as i64))),

        (ScalarValue::Int64(v), DataType::Int16) => Ok(ScalarValue::Int16(v.map(|x| x as i16))),
        (ScalarValue::Int64(v), DataType::Int32) => Ok(ScalarValue::Int32(v.map(|x| x as i32))),
        (ScalarValue::Int64(v), DataType::Int64) => Ok(ScalarValue::Int64(*v)),

        _ => Err(DataFusionError::Internal(format!(
            "cast_scalar_int_to_type: unsupported cast from {:?} to {:?}",
            value.data_type(),
            to
        ))),
    }
}


pub fn integer_interval_for_expr<E: SimplifyInfo>(
    expr: &Expr,
    info: &E,
) -> Result<DFInterval> {
    let dt = info.get_data_type(expr)?;

    if !is_integer_type(&dt) {
        return unbounded_for(&dt);
    }

    match expr {
        Expr::Column(_) => full_integer_range(&dt),
        Expr::Literal(v, _) if is_integer_type(&v.data_type()) => {
            DFInterval::try_new(v.clone(), v.clone())
        }
        Expr::Negative(c) => {
            let iv = integer_interval_for_expr(c, info)?;
            safe_interval_op(&dt, || iv.arithmetic_negate())
        }
        Expr::Cast(c) => {
            let child_iv = integer_interval_for_expr(&c.expr, info)?;
            let to_type = &c.data_type;
            if child_iv.is_unbounded() {
                return unbounded_for(to_type);
            }
            let lower = cast_scalar_int_to_type(child_iv.lower(), to_type)?;
            let upper = cast_scalar_int_to_type(child_iv.upper(), to_type)?;

            DFInterval::try_new(lower, upper)
        }
        Expr::BinaryExpr(be) => {
            use Operator::*;
            match be.op {
                Plus | Minus | Multiply | Divide => {
                    let l = integer_interval_for_expr(&be.left, info)?;
                    let r = integer_interval_for_expr(&be.right, info)?;

                    if l.is_unbounded() || r.is_unbounded() {
                        return unbounded_for(&dt);
                    }

                    safe_interval_op(&dt, || match be.op {
                        Plus => l.add(&r),
                        Minus => l.sub(&r),
                        Multiply => l.mul(&r),
                        Divide => l.div(&r),
                        _ => unreachable!(),
                    })
                }
                _ => unbounded_for(&dt),
            }
        }
        _ => unbounded_for(&dt),
    }
}
