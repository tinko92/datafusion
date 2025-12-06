# DataFusion Overflow-Safe Optimizer  
## Alignment  
  🎯 Core Requirements from Partner  

  1. Current State Analysis  

  - Completed: a < a + 1 → Optimized to Boolean(true)  
  - To be implemented: a + 1 < a → Should optimize to Boolean(false)  

  2. Core Concept: Overflow Risk  

  Mathematics vs. Computation:  
  - Mathematically: a < a + 1 is always true  
  - In computing: i32::MAX + 1 overflows to i32::MIN  

  Key Example:  
  -- When age = i32::MAX (2,147,483,647)  
  age < age + 1  
  -- age + 1 overflows to -2,147,483,648  
  -- Result: 2,147,483,647 < -2,147,483,648 = false  

  3. Specific Tasks  

  Task 1: Extend Simplification Rules  
  - Reverse comparison: a + b < a → b < 0  
  - Bilateral addition: a + b < a + c → b < c  
  - Multiplication merging: c1*a + c2*a → (c1 + c2)*a  

  Task 2: Overflow Check (Core)  
  fn may_overflow(expr) -> bool {  
      // Check data type and range  
      // Int32: [-2³¹, 2³¹-1]  
      // Safer after promotion to Int64  
      // Return: false(safe) or true(may overflow)  
  }  

  Task 3: Course Presentation  
  - Why optimizing trivial statements is needed  
  - Overflow counterexample demonstration  
  - Algorithm safety proof  

  🤔 Core Challenge  

  Safety vs. Performance Trade-off:  
  - Conservative strategy: Do not optimize when uncertain, ensure correctness  
  - Aggressive strategy: Optimize as much as possible, but may produce incorrect results  

  Your partner emphasizes: Optimize only when 100% certain no overflow will occur! This is why you need to implement the may_overflow function.  

## 📊 Implementation Status (vs Partner Requirements)  

### ✅ Completed (Meets Requirements)  
- **`a < a + 1 → Boolean(true)`**: Basic optimization rule (completed by partner)  
- **Overflow check function**: `may_overflow()` core safety mechanism  
- **Graded strategy**: Int8/16/32 optimized, Int64 handled conservatively  
- **Testing framework**: Comprehensive verification environment  
- **Reverse comparison**: `a + 1 < a → Boolean(false)`  

### ❌ To Implement (Partner Requirements)  
- **Task 1 Extensions**:  
  - `a + b < a → b < 0` ❌  
  - `a + b < a + c → b < c` ❌  
  - `c1*a + c2*a → (c1 + c2)*a` ❌  

### 🔧 Technical Challenges  
- **Type promotion impact**: DataFusion promotes all small integers to Int64 before optimization  
- **Pattern matching**: Expression recognition logic needs adjustment  
- **Safety boundary**: Int64 overflow risk still requires conservative handling  

## 🎯 Optimization Purpose  

Implement algebraic expression simplification in a database query optimizer while ensuring integer overflow safety:  

**Core Problem**: The mathematically always-true expression `a < a + 1` can become `false` in computing due to integer overflow.  

**Overflow Example**: When `a = i32::MAX (2,147,483,647)`, `a + 1` overflows to `-2,147,483,648`, causing `a < a + 1` to actually be `false`.  

**Optimization Goals**:  
- **Performance improvement**: Simplify mathematically always-true/false conditional expressions, reducing computational overhead.  
- **Safety guarantee**: Optimize only when 100% certain no overflow will occur.  
- **Correctness priority**: Prefer abandoning optimization over producing incorrect data results.  

**Implementation Scenarios**:  
- `a < a + 1` → `Boolean(true)` (when safe)  
- `a + 1 < a` → `Boolean(false)` (when safe)  

## 📁 Project Structure  

### Core Optimization Files  
- `datafusion/datafusion/optimizer/src/simplify_expressions/expr_simplifier.rs`  
  - **Purpose**: Main optimization logic implementation  
  - **New additions**: Overflow safety check helper functions and optimization rules  
  - **Location**: Lines 1970-2304  

### Overflow Check Functions  
- `is_simple_addition()` - Check if it's a simple addition expression  
- `expressions_equal_except_constant()` - Compare if expressions are equal  
- `extract_addition_constant()` - Extract constant value from addition  
- `may_overflow_with_positive_addition()` - Core overflow safety check function  

### Test Files  
- `datafusion/datafusion-examples/examples/overflow_optimization.rs`  
  - Basic overflow optimization tests  
- `datafusion/datafusion-examples/examples/int64_overflow_test.rs`  
  - Int64 boundary value tests  
- `datafusion/datafusion-examples/examples/comprehensive_overflow_test.rs`  
  - Comprehensive graded test suite  
- `datafusion/datafusion-examples/examples/performance_comparison.rs`  
  - Performance comparison tests  

## 🎯 Core Features  

### Graded Optimization Strategy  
- **Int8/Int16/Int32**: Assumed safe, optimization allowed  
- **Int64**: Handled conservatively, optimization prohibited  

### Optimization Rules  
- `a < a + 1` → `Boolean(true)` (when safe)  
- `a + 1 < a` → `Boolean(false)` (when safe)  

## 🚀 Running Tests  

```bash  
# Basic tests  
PROTOC=/usr/local/bin/protoc cargo run --example overflow_optimization  

# Comprehensive tests  
PROTOC=/usr/local/bin/protoc cargo run --example comprehensive_overflow_test  

# Performance tests  
PROTOC=/usr/local/bin/protoc cargo run --example performance_comparison  
```  

## 🔥 Original Type Check Mechanism (Core Technology)  

### 🎯 Problem Essence  
DataFusion promotes **all integer types to Int64** before expression optimization, making it impossible to distinguish original column types:  
- `age: Int32` → Becomes `age: Int64` during optimization  
- Cannot determine if `i32::MAX + 1` will overflow  

### 💡 Solution: Original Type Tracking System  

#### Data Flow Diagram  
```
Table Scan Phase → Type Coercion → Expression Simplification → Safe Optimization  
    ↓                   ↓                  ↓                 ↓  
Save Int32        Promote to Int64   Query Original Type   Decide Optimization  
```  

#### Core Implementation Flow  

1. **Type Saving Phase** (`TableScan`)  
   ```  
   File: datafusion/core/src/execution/session_state.rs:318  
   SessionState.original_type_tracker: OriginalTypeTracker  

   Function: Saves the original data type of all columns during table scan  
   ```  

2. **Type Passing Chain**  
   ```  
   SessionState → OptimizerConfig → SimplifyExpressions → SimplifyContext  

   File Locations:  
   - datafusion/optimizer/src/optimizer.rs:281 (OptimizerConfig trait)  
   - datafusion/expr/src/simplify.rs:60 (SimplifyContext struct)  
   - datafusion/common/src/type_tracker.rs:54 (OriginalTypeTracker)  
   ```  

3. **Type Query Phase** (`SimplifyInfo`)  
   ```  
   File: datafusion/expr/src/simplify.rs:31  

   trait SimplifyInfo {  
       fn is_originally_small_int(&self, expr: &Expr) -> Result<bool>  // Core method  
   }  
   ```  

4. **Safety Decision Phase** (`ExprSimplifier`)  
   ```  
   File: datafusion/optimizer/src/simplify_expressions/expr_simplifier.rs:1979  

   // Optimize only for original small integer types  
   if info.is_originally_small_int(&left)? {  
       return Transformed::yes(lit(true));  // Safe optimization  
   }  
   ```  

### 🛡️ Safety Grading Strategy  

| Original Type | Optimization Strategy | Safety Basis |  
|--------------|----------------------|--------------|  
| Int8/Int16/Int32 | ✅ Allow optimization | Small value range, addition less prone to overflow |  
| Int64 | ❌ Conservative handling | Large value range, real overflow risk |  

### 🎯 Key Check Function  

```rust  
// File: datafusion/common/src/type_tracker.rs:105  
pub fn was_promoted_from_small_int(&self, column: &Column, _current_type: &DataType) -> bool {  
    if let Some(original_type) = self.get_original_type(column) {  
        matches!(original_type, DataType::Int8 | DataType::Int16 | DataType::Int32)  
    } else {  
        false  
    }  
}  
```  

### 🏗️ Architectural Advantages  
- **Zero Intrusiveness**: Does not change DataFusion optimization order  
- **Type Safety**: Based on real original type information  
- **Backward Compatibility**: Does not affect existing optimization logic  
- **Extensibility**: Lays foundation for more complex type-dependent optimizations  

## 📊 Key Findings  

- DataFusion automatically promotes small integer types to Int64  
- Original type tracking solves information loss caused by type promotion  
- Int64 boundary values pose real overflow risks  
- Graded strategy balances safety and performance