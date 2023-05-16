use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::sync::{Arc, Mutex};
use wasmer::wasmparser::{BlockType as WpTypeOrFuncType, Operator};
use wasmer::{
    AsStoreMut, ExportIndex, FunctionMiddleware, GlobalInit, GlobalType, Instance,
    LocalFunctionIndex, MiddlewareError, MiddlewareReaderState, ModuleMiddleware, Mutability, Type,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

#[derive(Clone)]
struct CostGlobalIndexes(GlobalIndex, GlobalIndex);

impl CostGlobalIndexes {
    /// The global index in the current module for remaining points.
    fn remaining_points(&self) -> GlobalIndex {
        self.0
    }

    /// The global index in the current module for a boolean indicating whether points are exhausted
    /// or not.
    /// This boolean is represented as a i32 global:
    ///   * 0: there are remaining points
    ///   * 1: points have been exhausted
    fn points_exhausted(&self) -> GlobalIndex {
        self.1
    }
}

impl fmt::Debug for CostGlobalIndexes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CostGlobalIndexes")
            .field("remaining_points", &self.remaining_points())
            .field("points_exhausted", &self.points_exhausted())
            .finish()
    }
}

pub struct Cost {
    /// Limit of points.
    budget: u64,

    /// The global indexes for Cost points.
    global_indexes: Mutex<Option<CostGlobalIndexes>>,

    /// Accumulated counts of each operator.
    pub operation_counts: Arc<Mutex<HashMap<String, u64>>>,
}

/// The function-level Cost middleware.
pub struct FunctionCost {
    /// The global indexes for Cost points.
    global_indexes: CostGlobalIndexes,

    /// Accumulated cost of the current basic block.
    accumulated_cost: u64,

    /// Accumulated counts of each operator.
    operation_counts: Arc<Mutex<HashMap<String, u64>>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CostPoints {
    /// The given number of Cost points is left for the execution.
    /// If the value is 0, all points are consumed but the execution
    /// was not terminated.
    Remaining(u64),

    /// The execution was terminated because the Cost points were
    /// exhausted.  You can recover from this state by setting the
    /// points via [`set_remaining_points`] and restart the execution.
    Exhausted,
}

impl Cost {
    /// Creates a `Cost` middleware.
    pub fn new(budget: u64) -> Self {
        Self {
            budget,
            global_indexes: Mutex::new(None),
            operation_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl fmt::Debug for Cost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cost")
            .field("budget", &self.budget)
            .field("cost_function", &"<function>")
            .field("global_indexes", &self.global_indexes)
            .finish()
    }
}

impl ModuleMiddleware for Cost {
    /// Generates a `FunctionMiddleware` for a given function.
    fn generate_function_middleware(&self, _: LocalFunctionIndex) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionCost {
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
            accumulated_cost: 0,
            operation_counts: self.operation_counts.clone(),
        })
    }

    /// Transforms a `ModuleInfo` struct in-place. This is called before application on functions begins.
    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        if global_indexes.is_some() {
            panic!("Cost::transform_module_info: Attempting to use a `Cost` middleware from multiple modules.");
        }

        // Append a global for remaining points and initialize it.
        let remaining_points_global_index = module_info
            .globals
            .push(GlobalType::new(Type::I64, Mutability::Var));

        module_info
            .global_initializers
            .push(GlobalInit::I64Const(self.budget as i64));

        module_info.exports.insert(
            "compilet_cost_remaining_points".to_string(),
            ExportIndex::Global(remaining_points_global_index),
        );

        // Append a global for the exhausted points boolean and initialize it.
        let points_exhausted_global_index = module_info
            .globals
            .push(GlobalType::new(Type::I32, Mutability::Var));

        module_info
            .global_initializers
            .push(GlobalInit::I32Const(0));

        module_info.exports.insert(
            "compilet_cost_points_exhausted".to_string(),
            ExportIndex::Global(points_exhausted_global_index),
        );

        *global_indexes = Some(CostGlobalIndexes(
            remaining_points_global_index,
            points_exhausted_global_index,
        ))
    }
}

impl fmt::Debug for FunctionCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionCost")
            .field("cost_function", &"<function>")
            .field("global_indexes", &self.global_indexes)
            .finish()
    }
}

impl FunctionMiddleware for FunctionCost {
    fn feed<'a>(
        &mut self,
        operator: Operator<'a>,
        state: &mut MiddlewareReaderState<'a>,
    ) -> Result<(), MiddlewareError> {
        // Get the cost of the current operator, and add it to the accumulator.
        // This needs to be done before the Cost logic, to prevent operators like `Call` from escaping Cost in some
        // corner cases.
        // Reference: https://nemequ.github.io/waspr/instructions
        // Reference: https://github.com/WebAssembly/binaryen/blob/main/src/ir/cost.h
        self.accumulated_cost += match operator {
            Operator::LocalGet { .. } => 0,
            Operator::LocalSet { .. } | Operator::LocalTee { .. } => 1,
            Operator::GlobalGet { .. } => 1,
            Operator::GlobalSet { .. } => 2,
            Operator::F32Load { .. }
            | Operator::F64Load { .. }
            | Operator::I32Load { .. }
            | Operator::I64Load { .. }
            | Operator::I32Load8S { .. }
            | Operator::I32Load8U { .. }
            | Operator::I32Load16S { .. }
            | Operator::I32Load16U { .. }
            | Operator::I64Load8S { .. }
            | Operator::I64Load8U { .. }
            | Operator::I64Load16S { .. }
            | Operator::I64Load16U { .. }
            | Operator::I64Load32S { .. }
            | Operator::I64Load32U { .. } => 1,
            Operator::I32AtomicLoad { .. }
            | Operator::I32AtomicLoad8U { .. }
            | Operator::I32AtomicLoad16U { .. }
            | Operator::I64AtomicLoad { .. }
            | Operator::I64AtomicLoad8U { .. }
            | Operator::I64AtomicLoad16U { .. }
            | Operator::I64AtomicLoad32U { .. } => 10 + 1,
            Operator::F32Store { .. }
            | Operator::F64Store { .. }
            | Operator::I32Store { .. }
            | Operator::I64Store { .. }
            | Operator::I32Store8 { .. }
            | Operator::I32Store16 { .. }
            | Operator::I64Store8 { .. }
            | Operator::I64Store16 { .. }
            | Operator::I64Store32 { .. } => 2,
            Operator::I32AtomicStore { .. }
            | Operator::I32AtomicStore8 { .. }
            | Operator::I32AtomicStore16 { .. }
            | Operator::I64AtomicStore { .. }
            | Operator::I64AtomicStore8 { .. }
            | Operator::I64AtomicStore16 { .. }
            | Operator::I64AtomicStore32 { .. } => 10 + 2,
            Operator::F32Const { .. }
            | Operator::F64Const { .. }
            | Operator::I32Const { .. }
            | Operator::I64Const { .. } => 1,
            Operator::F32ConvertI32S
            | Operator::F32ConvertI32U
            | Operator::F32ConvertI64S
            | Operator::F32ConvertI64U
            | Operator::F64ConvertI32S
            | Operator::F64ConvertI32U
            | Operator::F64ConvertI64S
            | Operator::F64ConvertI64U
            | Operator::I32ReinterpretF32
            | Operator::I64ReinterpretF64
            | Operator::F32ReinterpretI32
            | Operator::F64ReinterpretI64
            | Operator::I32WrapI64
            | Operator::I32Extend8S
            | Operator::I32Extend16S
            | Operator::I64Extend8S
            | Operator::I64Extend16S
            | Operator::I64Extend32S
            | Operator::I64ExtendI32U
            | Operator::I64ExtendI32S
            | Operator::F32Trunc
            | Operator::F64Trunc
            | Operator::I32TruncF32S
            | Operator::I32TruncF32U
            | Operator::I32TruncF64S
            | Operator::I32TruncF64U
            | Operator::I32TruncSatF32S
            | Operator::I32TruncSatF32U
            | Operator::I32TruncSatF64S
            | Operator::I32TruncSatF64U
            | Operator::I64TruncF32S
            | Operator::I64TruncF32U
            | Operator::I64TruncF64S
            | Operator::I64TruncF64U
            | Operator::I64TruncSatF32S
            | Operator::I64TruncSatF32U
            | Operator::I64TruncSatF64S
            | Operator::I64TruncSatF64U
            | Operator::F32DemoteF64
            | Operator::F64PromoteF32
            | Operator::I32Popcnt
            | Operator::I64Popcnt
            | Operator::I32Clz
            | Operator::I32Ctz
            | Operator::I64Clz
            | Operator::I64Ctz
            | Operator::F32Neg
            | Operator::F64Neg
            | Operator::F32Abs
            | Operator::F64Abs
            | Operator::F32Ceil
            | Operator::F64Ceil
            | Operator::F32Floor
            | Operator::F64Floor
            | Operator::F32Nearest
            | Operator::F64Nearest
            | Operator::I32Eqz
            | Operator::I64Eqz => 1,
            Operator::F32Sqrt | Operator::F64Sqrt => 2,
            Operator::F32x4Splat
            | Operator::F64x2Splat
            | Operator::I16x8Splat
            | Operator::I32x4Splat
            | Operator::I64x2Splat
            | Operator::I8x16Splat
            | Operator::V128Not
            | Operator::V128AnyTrue
            | Operator::F32x4Abs
            | Operator::F32x4Neg
            | Operator::F32x4Sqrt
            | Operator::F32x4Ceil
            | Operator::F32x4Floor
            | Operator::F32x4Trunc
            | Operator::F32x4Nearest
            | Operator::F64x2Abs
            | Operator::F64x2Neg
            | Operator::F64x2Sqrt
            | Operator::F64x2Ceil
            | Operator::F64x2Floor
            | Operator::F64x2Trunc
            | Operator::F64x2Nearest
            | Operator::I8x16Abs
            | Operator::I8x16Neg
            | Operator::I8x16AllTrue
            | Operator::I8x16Bitmask
            | Operator::I8x16Popcnt
            | Operator::I16x8Abs
            | Operator::I16x8Neg
            | Operator::I16x8AllTrue
            | Operator::I16x8Bitmask
            | Operator::I32x4Abs
            | Operator::I32x4Neg
            | Operator::I32x4AllTrue
            | Operator::I32x4Bitmask
            | Operator::I64x2Abs
            | Operator::I64x2Neg
            | Operator::I64x2AllTrue
            | Operator::I64x2Bitmask
            | Operator::F32x4ConvertI32x4S
            | Operator::F32x4ConvertI32x4U
            | Operator::I32x4TruncSatF32x4S
            | Operator::I32x4TruncSatF32x4U
            | Operator::F64x2ConvertLowI32x4S
            | Operator::F64x2ConvertLowI32x4U
            | Operator::I32x4TruncSatF64x2SZero
            | Operator::I32x4TruncSatF64x2UZero
            | Operator::I16x8ExtAddPairwiseI8x16S
            | Operator::I16x8ExtAddPairwiseI8x16U
            | Operator::I32x4ExtAddPairwiseI16x8S
            | Operator::I32x4ExtAddPairwiseI16x8U
            | Operator::I16x8ExtendHighI8x16S
            | Operator::I16x8ExtendLowI8x16S
            | Operator::I16x8ExtendHighI8x16U
            | Operator::I16x8ExtendLowI8x16U
            | Operator::I32x4ExtendHighI16x8S
            | Operator::I32x4ExtendLowI16x8S
            | Operator::I32x4ExtendHighI16x8U
            | Operator::I32x4ExtendLowI16x8U
            | Operator::I64x2ExtendHighI32x4S
            | Operator::I64x2ExtendLowI32x4S
            | Operator::I64x2ExtendHighI32x4U
            | Operator::I64x2ExtendLowI32x4U
            | Operator::F32x4DemoteF64x2Zero
            | Operator::F64x2PromoteLowF32x4
            | Operator::I32x4RelaxedTruncSatF32x4S
            | Operator::I32x4RelaxedTruncSatF32x4U
            | Operator::I32x4RelaxedTruncSatF64x2SZero
            | Operator::I32x4RelaxedTruncSatF64x2UZero => 1,
            Operator::I32Add
            | Operator::I32Sub
            | Operator::I64Add
            | Operator::I64Sub
            | Operator::F32Add
            | Operator::F32Sub
            | Operator::F64Add
            | Operator::F64Sub => 1,
            Operator::I32Mul | Operator::I64Mul | Operator::F32Mul | Operator::F64Mul => 2,
            Operator::I32DivS
            | Operator::I32DivU
            | Operator::I32RemS
            | Operator::I32RemU
            | Operator::I64DivS
            | Operator::I64DivU
            | Operator::I64RemS
            | Operator::I64RemU
            | Operator::F32Div
            | Operator::F64Div => 3,
            Operator::I32And
            | Operator::I32Or
            | Operator::I32Xor
            | Operator::I32Shl
            | Operator::I32ShrS
            | Operator::I32ShrU
            | Operator::I32Rotl
            | Operator::I32Rotr
            | Operator::I64And
            | Operator::I64Or
            | Operator::I64Xor
            | Operator::I64Shl
            | Operator::I64ShrS
            | Operator::I64ShrU
            | Operator::I64Rotl
            | Operator::I64Rotr => 1,
            Operator::F32Copysign | Operator::F64Copysign => 1,
            Operator::F32Min | Operator::F32Max | Operator::F64Min | Operator::F64Max => 1,
            Operator::I32Eq
            | Operator::I32Ne
            | Operator::I32LtS
            | Operator::I32LtU
            | Operator::I32LeS
            | Operator::I32LeU
            | Operator::I32GtS
            | Operator::I32GtU
            | Operator::I32GeS
            | Operator::I32GeU
            | Operator::I64Eq
            | Operator::I64Ne
            | Operator::I64LtS
            | Operator::I64LtU
            | Operator::I64LeS
            | Operator::I64LeU
            | Operator::I64GtS
            | Operator::I64GtU
            | Operator::I64GeS
            | Operator::I64GeU
            | Operator::F32Eq
            | Operator::F32Ne
            | Operator::F32Lt
            | Operator::F32Le
            | Operator::F32Gt
            | Operator::F32Ge
            | Operator::F64Eq
            | Operator::F64Ne
            | Operator::F64Lt
            | Operator::F64Le
            | Operator::F64Gt
            | Operator::F64Ge => 1,
            Operator::Block { .. }
            | Operator::Loop { .. }
            | Operator::If { .. }
            | Operator::Else
            | Operator::End
            | Operator::Br { .. }
            | Operator::BrIf { .. }
            | Operator::BrTable { .. }
            | Operator::Select => 1,
            Operator::MemoryGrow { .. } | Operator::MemorySize { .. } => 1,
            Operator::MemoryInit { .. }
            | Operator::MemoryCopy { .. }
            | Operator::MemoryFill { .. } => 6,
            Operator::Return
            | Operator::Unreachable
            | Operator::Nop
            | Operator::Drop
            | Operator::Try { .. } => 0,
            Operator::Call { .. } => 4,
            Operator::CallIndirect { .. } => 6,
            Operator::DataDrop { .. } => 5,
            Operator::Throw { .. } => 100,
            _ => {
                eprintln!("Penalty Instruction [{:?}]", &operator);
                1000
            }
        };

        // Add 1 to the count of the current operator, do static analysis
        let x = format!("{:?}", operator);
        let name = x.split_whitespace().next().unwrap();
        let mut operation_counts = self.operation_counts.lock().unwrap();
        operation_counts
            .entry(name.to_string())
            .and_modify(|counter| *counter += 1)
            .or_insert(1);

        // Possible sources and targets of a branch. Finalize the cost of the previous basic block and perform necessary checks.
        match operator {
            Operator::Loop { .. } // loop headers are branch targets
            | Operator::End // block ends are branch targets
            | Operator::Else // "else" is the "end" of an if branch
            | Operator::Br { .. } // branch source
            | Operator::BrTable { .. } // branch source
            | Operator::BrIf { .. } // branch source
            | Operator::Call { .. } // function call - branch source
            | Operator::CallIndirect { .. } // function call - branch source
            | Operator::Return // end of function - branch source
            => {
                if self.accumulated_cost > 0 {
                    state.extend(&[
                        // if unsigned(globals[remaining_points_index]) < unsigned(self.accumulated_cost) { throw(); }
                        Operator::GlobalGet { global_index: self.global_indexes.remaining_points().as_u32() },
                        Operator::I64Const { value: self.accumulated_cost as i64 },
                        Operator::I64LtU,
                        Operator::If { blockty: WpTypeOrFuncType::Empty },
                        Operator::I32Const { value: 1 },
                        Operator::GlobalSet { global_index: self.global_indexes.points_exhausted().as_u32() },
                        Operator::Unreachable,
                        Operator::End,

                        // globals[remaining_points_index] -= self.accumulated_cost;
                        Operator::GlobalGet { global_index: self.global_indexes.remaining_points().as_u32() },
                        Operator::I64Const { value: self.accumulated_cost as i64 },
                        Operator::I64Sub,
                        Operator::GlobalSet { global_index: self.global_indexes.remaining_points().as_u32() },
                    ]);

                    self.accumulated_cost = 0;
                }
            }
            _ => {}
        }
        state.push_operator(operator);

        Ok(())
    }
}

pub fn get_remaining_points(ctx: &mut impl AsStoreMut, instance: &Instance) -> CostPoints {
    let exhausted: i32 = instance
        .exports
        .get_global("compilet_cost_points_exhausted")
        .expect("Can't get `compilet_cost_points_exhausted` from Instance")
        .get(ctx)
        .try_into()
        .expect("`compilet_cost_points_exhausted` from Instance has wrong type");

    if exhausted > 0 {
        return CostPoints::Exhausted;
    }

    let points = instance
        .exports
        .get_global("compilet_cost_remaining_points")
        .expect("Can't get `compilet_cost_remaining_points` from Instance")
        .get(ctx)
        .try_into()
        .expect("`compilet_cost_remaining_points` from Instance has wrong type");

    CostPoints::Remaining(points)
}
