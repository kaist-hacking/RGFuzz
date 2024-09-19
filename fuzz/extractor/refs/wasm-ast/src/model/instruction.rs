//! WebAssembly instruction set.

use enum_iterator::{Sequence, cardinality};

use crate::{model::{
    DataIndex, ElementIndex, FloatType, FunctionIndex, GlobalIndex, IntegerType, LabelIndex,
    LocalIndex, NumberType, ReferenceType, TableIndex, TypeIndex, ValueType,
}, VectorShape, LaneIdx, FShape, IShape};

/// WebAssembly code consists of sequences of instructions.
/// Its computational model is based on a stack machine in that instructions manipulate values on
/// an implicit operand stack, consuming (popping) argument values and producing or returning
/// (pushing) result values.
/// In addition to dynamic operands from the stack,
/// some instructions also have static immediate arguments,
/// typically indices or type annotations, which are part of the instruction itself.
/// Some instructions are structured in that they bracket nested sequences of instructions.
/// The following sections group instructions into a number of different categories.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#instructions>
///
/// # Examples
/// See the specific instruction types for examples.
#[derive(Clone, Debug, PartialEq, Sequence)]
pub enum Instruction {
    Numeric(NumericInstruction),
    Vector(VectorInstruction),
    Reference(ReferenceInstruction),
    Parametric(ParametricInstruction),
    Variable(VariableInstruction),
    Table(TableInstruction),
    Memory(MemoryInstruction),
    Control(ControlInstruction),
}

/// Numeric instructions provide basic operations over numeric values of specific type.
/// These operations closely match respective operations available in hardware.
///
/// Some integer instructions come in two flavors,
/// where a signedness annotation sx distinguishes whether the operands are to be interpreted as
/// unsigned or signed integers. For the other integer instructions, the use of two’s complement
/// for the signed interpretation means that they behave the same regardless of signedness.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#numeric-instructions>
///
/// # Examples
/// ## Constant
/// ```rust
/// use wasm_ast::{NumericInstruction, Instruction};
///
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::I32Constant(42)),
///     42i32.into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::I64Constant(42i64)),
///     42i64.into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::F32Constant(0.1)),
///     0.1f32.into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::F64Constant(0.2)),
///     0.2f64.into()
/// );
/// ```
///
/// ## Integer
/// ```rust
/// use wasm_ast::{NumericInstruction, Instruction, IntegerType, SignExtension};
///
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::CountLeadingZeros(IntegerType::I32)),
///     NumericInstruction::CountLeadingZeros(IntegerType::I32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::CountTrailingZeros(IntegerType::I64)),
///     NumericInstruction::CountTrailingZeros(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::CountOnes(IntegerType::I64)),
///     NumericInstruction::CountOnes(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::DivideInteger(IntegerType::I64, SignExtension::Unsigned)),
///     NumericInstruction::DivideInteger(IntegerType::I64, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Remainder(IntegerType::I64, SignExtension::Unsigned)),
///     NumericInstruction::Remainder(IntegerType::I64, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::And(IntegerType::I64)),
///     NumericInstruction::And(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Or(IntegerType::I64)),
///     NumericInstruction::Or(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Xor(IntegerType::I64)),
///     NumericInstruction::Xor(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ShiftLeft(IntegerType::I64)),
///     NumericInstruction::ShiftLeft(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ShiftRight(IntegerType::I64, SignExtension::Unsigned)),
///     NumericInstruction::ShiftRight(IntegerType::I64, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::RotateLeft(IntegerType::I64)),
///     NumericInstruction::RotateLeft(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::RotateRight(IntegerType::I64)),
///     NumericInstruction::RotateRight(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::EqualToZero(IntegerType::I64)),
///     NumericInstruction::EqualToZero(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::LessThanInteger(IntegerType::I64, SignExtension::Unsigned)),
///     NumericInstruction::LessThanInteger(IntegerType::I64, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::GreaterThanInteger(IntegerType::I64, SignExtension::Unsigned)),
///     NumericInstruction::GreaterThanInteger(IntegerType::I64, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::LessThanOrEqualToInteger(IntegerType::I64, SignExtension::Unsigned)),
///     NumericInstruction::LessThanOrEqualToInteger(IntegerType::I64, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::GreaterThanOrEqualToInteger(IntegerType::I64, SignExtension::Unsigned)),
///     NumericInstruction::GreaterThanOrEqualToInteger(IntegerType::I64, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ExtendSigned8(IntegerType::I64)),
///     NumericInstruction::ExtendSigned8(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ExtendSigned16(IntegerType::I64)),
///     NumericInstruction::ExtendSigned16(IntegerType::I64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ExtendSigned32),
///     NumericInstruction::ExtendSigned32.into()
/// );
/// ```
///
/// ## Float
/// ```rust
/// use wasm_ast::{NumericInstruction, Instruction, FloatType};
///
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::AbsoluteValue(FloatType::F32)),
///     NumericInstruction::AbsoluteValue(FloatType::F32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Negate(FloatType::F64)),
///     NumericInstruction::Negate(FloatType::F64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::SquareRoot(FloatType::F64)),
///     NumericInstruction::SquareRoot(FloatType::F64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Ceiling(FloatType::F32)),
///     NumericInstruction::Ceiling(FloatType::F32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Floor(FloatType::F64)),
///     NumericInstruction::Floor(FloatType::F64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Truncate(FloatType::F64)),
///     NumericInstruction::Truncate(FloatType::F64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Nearest(FloatType::F64)),
///     NumericInstruction::Nearest(FloatType::F64).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::DivideFloat(FloatType::F32)),
///     NumericInstruction::DivideFloat(FloatType::F32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Minimum(FloatType::F32)),
///     NumericInstruction::Minimum(FloatType::F32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Maximum(FloatType::F32)),
///     NumericInstruction::Maximum(FloatType::F32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::CopySign(FloatType::F32)),
///     NumericInstruction::CopySign(FloatType::F32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::LessThanFloat(FloatType::F32)),
///     NumericInstruction::LessThanFloat(FloatType::F32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::GreaterThanFloat(FloatType::F32)),
///     NumericInstruction::GreaterThanFloat(FloatType::F32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::LessThanOrEqualToFloat(FloatType::F32)),
///     NumericInstruction::LessThanOrEqualToFloat(FloatType::F32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::GreaterThanOrEqualToFloat(FloatType::F32)),
///     NumericInstruction::GreaterThanOrEqualToFloat(FloatType::F32).into()
/// );
/// ```
///
/// ## Number
/// ```rust
/// use wasm_ast::{NumericInstruction, Instruction, NumberType};
///
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Add(NumberType::I32)),
///     NumericInstruction::Add(NumberType::I32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Subtract(NumberType::I32)),
///     NumericInstruction::Subtract(NumberType::I32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Multiply(NumberType::I32)),
///     NumericInstruction::Multiply(NumberType::I32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Equal(NumberType::I32)),
///     NumericInstruction::Equal(NumberType::I32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::NotEqual(NumberType::I32)),
///     NumericInstruction::NotEqual(NumberType::I32).into()
/// );
/// ```
///
/// ## Convert
/// ```rust
/// use wasm_ast::{NumericInstruction, Instruction, NumberType, SignExtension, IntegerType, FloatType};
///
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Wrap),
///     NumericInstruction::Wrap.into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ExtendWithSignExtension(SignExtension::Signed)),
///     NumericInstruction::ExtendWithSignExtension(SignExtension::Signed).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ConvertAndTruncate(IntegerType::I32, FloatType::F64, SignExtension::Unsigned)),
///     NumericInstruction::ConvertAndTruncate(IntegerType::I32, FloatType::F64, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ConvertAndTruncateWithSaturation(IntegerType::I32, FloatType::F64, SignExtension::Unsigned)),
///     NumericInstruction::ConvertAndTruncateWithSaturation(IntegerType::I32, FloatType::F64, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Demote),
///     NumericInstruction::Demote.into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Promote),
///     NumericInstruction::Promote.into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::Convert(FloatType::F64, IntegerType::I32, SignExtension::Unsigned)),
///     NumericInstruction::Convert(FloatType::F64, IntegerType::I32, SignExtension::Unsigned).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ReinterpretFloat(IntegerType::I32)),
///     NumericInstruction::ReinterpretFloat(IntegerType::I32).into()
/// );
/// assert_eq!(
///     Instruction::Numeric(NumericInstruction::ReinterpretInteger(FloatType::F64)),
///     NumericInstruction::ReinterpretInteger(FloatType::F64).into()
/// );
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Sequence)]
pub enum NumericInstruction {
    /// i32.const
    I32Constant(i32),
    /// i64.const
    I64Constant(i64),
    /// f32.const
    F32Constant(f32),
    /// f64.const
    F64Constant(f64),
    /// inn.clz
    CountLeadingZeros(IntegerType),
    /// inn.ctz
    CountTrailingZeros(IntegerType),
    /// inn.popcnt
    CountOnes(IntegerType),
    /// fnn.abs
    AbsoluteValue(FloatType),
    /// fnn.neg
    Negate(FloatType),
    /// fnn.sqrt
    SquareRoot(FloatType),
    /// fnn.ceil
    Ceiling(FloatType),
    /// fnn.floor
    Floor(FloatType),
    /// fnn.trunc
    Truncate(FloatType),
    /// fnn.nearest
    Nearest(FloatType),
    /// xnn.add
    Add(NumberType),
    /// xnn.sub
    Subtract(NumberType),
    /// xnn.mul
    Multiply(NumberType),
    /// inn.div_sx
    DivideInteger(IntegerType, SignExtension),
    /// fnn.div
    DivideFloat(FloatType),
    /// inn.rem_sx
    Remainder(IntegerType, SignExtension),
    /// inn.and
    And(IntegerType),
    /// inn.or
    Or(IntegerType),
    /// inn.xor
    Xor(IntegerType),
    /// inn.shl
    ShiftLeft(IntegerType),
    /// inn.shr_sx
    ShiftRight(IntegerType, SignExtension),
    /// inn.rotl
    RotateLeft(IntegerType),
    /// inn.rotr
    RotateRight(IntegerType),
    /// fnn.min
    Minimum(FloatType),
    /// fnn.max
    Maximum(FloatType),
    /// fnn.copysign
    CopySign(FloatType),
    /// inn.eqz
    EqualToZero(IntegerType),
    /// xnn.eq
    Equal(NumberType),
    /// xnn.ne
    NotEqual(NumberType),
    /// inn.lt_sx
    LessThanInteger(IntegerType, SignExtension),
    /// fnn.lt
    LessThanFloat(FloatType),
    /// inn.gt_sx
    GreaterThanInteger(IntegerType, SignExtension),
    /// fnn.gt
    GreaterThanFloat(FloatType),
    /// inn.le_sx
    LessThanOrEqualToInteger(IntegerType, SignExtension),
    /// fnn.le
    LessThanOrEqualToFloat(FloatType),
    /// inn.ge_sx
    GreaterThanOrEqualToInteger(IntegerType, SignExtension),
    /// fnn.ge
    GreaterThanOrEqualToFloat(FloatType),
    /// inn.extend8_s
    ExtendSigned8(IntegerType),
    /// inn.extend16_s
    ExtendSigned16(IntegerType),
    /// i64.extend32_s
    ExtendSigned32,
    /// i32.wrap_i64
    Wrap,
    /// i64.extend_i32_sx
    ExtendWithSignExtension(SignExtension),
    /// inn.trunc_fmm_sx
    ConvertAndTruncate(IntegerType, FloatType, SignExtension),
    /// inn.trunc_sat_fmm_sx
    ConvertAndTruncateWithSaturation(IntegerType, FloatType, SignExtension),
    /// f32.demote_f64
    Demote,
    /// f64.promote_f32
    Promote,
    /// fnn.convert_imm_sx
    Convert(FloatType, IntegerType, SignExtension),
    /// inn.reinterpret_fmm
    ReinterpretFloat(IntegerType),
    /// fnn.reinterpret_imm
    ReinterpretInteger(FloatType),
}

impl From<NumericInstruction> for Instruction {
    fn from(instruction: NumericInstruction) -> Self {
        Instruction::Numeric(instruction)
    }
}

impl From<i8> for Instruction {
    fn from(value: i8) -> Self {
        Self::Numeric(NumericInstruction::I32Constant(value as i32))
    }
}

impl From<i16> for Instruction {
    fn from(value: i16) -> Self {
        Self::Numeric(NumericInstruction::I32Constant(value as i32))
    }
}

impl From<i32> for Instruction {
    fn from(value: i32) -> Self {
        Self::Numeric(NumericInstruction::I32Constant(value))
    }
}

impl From<i64> for Instruction {
    fn from(value: i64) -> Self {
        Self::Numeric(NumericInstruction::I64Constant(value))
    }
}

impl From<u8> for Instruction {
    fn from(value: u8) -> Self {
        Self::Numeric(NumericInstruction::I32Constant(value as i32))
    }
}

impl From<u16> for Instruction {
    fn from(value: u16) -> Self {
        Self::Numeric(NumericInstruction::I32Constant(value as i32))
    }
}

impl From<u32> for Instruction {
    fn from(value: u32) -> Self {
        Self::Numeric(NumericInstruction::I64Constant(value as i64))
    }
}

impl From<f32> for Instruction {
    fn from(value: f32) -> Self {
        Self::Numeric(NumericInstruction::F32Constant(value))
    }
}

impl From<f64> for Instruction {
    fn from(value: f64) -> Self {
        Self::Numeric(NumericInstruction::F64Constant(value))
    }
}

/// Vector Instructions
#[derive(Copy, Clone, Debug, PartialEq, Sequence)]
pub enum VectorInstruction {
    /// v128.const
    V128Constant(i128),
    /// v128.not
    Not,
    /// v128.and
    And,
    /// v128.andnot
    AndNot,
    /// v128.or
    Or,
    /// v128.xor
    Xor,
    /// v128.bitselect
    BitSelect,
    /// v128.any_true
    AnyTrue,
    /// i8x16.shuffle laneidx16
    I8X16Shuffle([LaneIdx; 16]),
    /// i8x16.swizzle
    I8X16Swizzle,
    /// shape.splat
    Splat(VectorShape),
    /// i8x16.extract_lane_sx laneidx
    I8X16ExtractLane(SignExtension, LaneIdx),
    /// i16x8.extract_lane_sx laneidx
    I16X8ExtractLane(SignExtension, LaneIdx),
    /// i32x4.extract_lane laneidx
    I32X4ExtractLane(LaneIdx),
    /// i64x2.extract_lane laneidx
    I64X2ExtractLane(LaneIdx),
    /// fshape.extract_lane laneidx
    FExtractLane(FShape, LaneIdx),
    /// shape.replace_lane laneidx
    ReplaceLane(VectorShape, LaneIdx),
    /// i8x16.eq
    I8X16Eq,
    /// i8x16.ne
    I8X16Ne,
    /// i8x16.lt_sx
    I8X16Lt(SignExtension),
    /// i8x16.gt_sx
    I8X16Gt(SignExtension),
    /// i8x16.le_sx
    I8X16Le(SignExtension),
    /// i8x16.ge_sx
    I8X16Ge(SignExtension),
    /// i16x8.eq
    I16X8Eq,
    /// i16x8.ne
    I16X8Ne,
    /// i16x8.lt_sx
    I16X8Lt(SignExtension),
    /// i16x8.gt_sx
    I16X8Gt(SignExtension),
    /// i16x8.le_sx
    I16X8Le(SignExtension),
    /// i16x8.ge_sx
    I16X8Ge(SignExtension),
    /// i32x4.eq
    I32X4Eq,
    /// i32x4.ne
    I32X4Ne,
    /// i32x4.lt_sx
    I32X4Lt(SignExtension),
    /// i32x4.gt_sx
    I32X4Gt(SignExtension),
    /// i32x4.le_sx
    I32X4Le(SignExtension),
    /// i32x4.ge_sx
    I32X4Ge(SignExtension),
    /// i64x2.eq
    I64X2Eq,
    /// i64x2.ne
    I64X2Ne,
    /// i64x2.lt
    I64X2Lt,
    /// i64x2.gt
    I64X2Gt,
    /// i64x2.le
    I64X2Le,
    /// i64x2.ge
    I64X2Ge,
    /// fshape.eq
    FEq(FShape),
    /// fshape.ne
    FNe(FShape),
    /// fshape.lt
    FLt(FShape),
    /// fshape.gt
    FGt(FShape),
    /// fshape.le
    FLe(FShape),
    /// fshape.ge
    FGe(FShape),
    /// ishape.abs
    IAbs(IShape),
    /// ishape.neg
    INeg(IShape),
    /// i8x16.popcnt
    I8X16Popcnt,
    /// i16x8.q15mulr_sat_s
    I16X8Q15MulrSat,
    /// i32x4.dot_i16x8_s
    I32X4DotI16X8,
    /// fshape.abs
    FAbs(FShape),
    /// fshape.neg
    FNeg(FShape),
    /// fshape.sqrt
    Sqrt(FShape),
    /// fshape.ceil
    Ceil(FShape),
    /// fshape.floor
    Floor(FShape),
    /// fshape.trunc
    Trunc(FShape),
    /// fshape.nearest
    Nearest(FShape),
    /// ishape.all_true
    AllTrue(IShape),
    /// ishape.bitmask
    Bitmask(IShape),
    /// i8x16.narrow_i16x8_sx
    I8X16NarrowI16X8(SignExtension),
    /// i16x8.narrow_i32x4_sx
    I16X8NarrowI32X4(SignExtension),
    /// i16x8.extend_low_i8x16_sx
    I16X8ExtendLowI8X16(SignExtension),
    /// i16x8.extend_high_i8x16_sx
    I16X8ExtendHighI8X16(SignExtension),
    /// i32x4.extend_low_i16x8_sx
    I32X4ExtendLowI16X8(SignExtension),
    /// i32x4.extend_high_i16x8_sx
    I32X4ExtendHighI16X8(SignExtension),
    /// i64x2.extend_low_i32x4_sx
    I64X2ExtendLowI32X4(SignExtension),
    /// i64x2.extend_high_i32x4_sx
    I64X2ExtendHighI32X4(SignExtension),
    /// ishape.shl
    Shl(IShape),
    /// ishape.shr_sx
    Shr(IShape, SignExtension),
    /// ishape.add
    IAdd(IShape),
    /// ishape.sub
    ISub(IShape),
    /// i8x16.min_sx
    I8X16Min(SignExtension),
    /// i8x16.max_sx
    I8X16Max(SignExtension),
    /// i16x8.min_sx
    I16X8Min(SignExtension),
    /// i16x8.max_sx
    I16X8Max(SignExtension),
    /// i32x4.min_sx
    I32X4Min(SignExtension),
    /// i32x4.max_sx
    I32X4Max(SignExtension),
    /// i8x16.add_sat_sx
    I8X16AddSat(SignExtension),
    /// i8x16.sub_sat_sx
    I8X16SubSat(SignExtension),
    /// i16x8.add_sat_sx
    I16X8AddSat(SignExtension),
    /// i16x8.sub_sat_sx
    I16X8SubSat(SignExtension),
    /// i16x8.mul
    I16X8Mul,
    /// i32x4.mul
    I32X4Mul,
    /// i64x2.mul
    I64X2Mul,
    /// i8x16.avgr_u
    I8X16Avgr,
    /// i16X8.avgr_u
    I16X8Avgr,
    /// i16x8.extmul_low_i8x16_sx
    I16X8ExtmulLowI8X16(SignExtension),
    /// i16x8.extmul_high_i8x16_sx
    I16X8ExtmulHighI8X16(SignExtension),
    /// i32x4.extmul_low_i16x8_sx
    I32X4ExtmulLowI16X8(SignExtension),
    /// i32x4.extmul_high_i16x8_sx
    I32X4ExtmulHighI16X8(SignExtension),
    /// i64x2.extmul_low_i32x4_sx
    I64X2ExtmulLowI32X4(SignExtension),
    /// i64x2.extmul_high_i32x4_sx
    I64X2ExtmulHighI32X4(SignExtension),
    /// i16x8.extadd_pairwise_i8x16_sx
    I16X8ExtaddPairwiseI8X16(SignExtension),
    /// i32x4.extadd_pairwise_i16x8_sx
    I32X4ExtaddPairwiseI16X8(SignExtension),
    /// fshape.add
    FAdd(FShape),
    /// fshape.sub
    FSub(FShape),
    /// fshape.mul
    FMul(FShape),
    /// fshape.div
    FDiv(FShape),
    /// fshape.min
    FMin(FShape),
    /// fshape.max
    FMax(FShape),
    /// fshape.pmin
    Pmin(FShape),
    /// fshape.pmax
    Pmax(FShape),
    /// i32x4.trunc_sat_f32x4_sx
    I32X4TruncSatF32X4(SignExtension),
    /// i32x4.trunc_sat_f64x2_sx_zero
    I32X4TruncSatF64X2Zero(SignExtension),
    /// f32x4.convert_i32x4_sx
    F32X4ConvertI32X4(SignExtension),
    /// f32x4.demote_f64x2_zero
    F32X4DemoteF64X2Zero,
    /// f64x2.convert_low_i32x4_sx
    F64X2ConvertLowI32X4(SignExtension),
    /// f64x2.promote_low_f32x4
    F64X2PromoteLowF32X4,
}

/// Instructions in this group are concerned with accessing references.
/// These instruction produce a null value, check for a null value, or produce a reference to a given function, respectively.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#reference-instructions>
///
/// # Examples
/// ```rust
/// use wasm_ast::{ReferenceInstruction, Instruction, ReferenceType};
///
/// assert_eq!(
///     Instruction::Reference(ReferenceInstruction::Null(ReferenceType::External)),
///     ReferenceInstruction::Null(ReferenceType::External).into()
/// );
/// assert_eq!(
///     Instruction::Reference(ReferenceInstruction::IsNull),
///     ReferenceInstruction::IsNull.into()
/// );
/// assert_eq!(
///     Instruction::Reference(ReferenceInstruction::Function(3)),
///     ReferenceInstruction::Function(3).into()
/// );
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum ReferenceInstruction {
    /// ref.null
    /// Produce a null value.
    Null(ReferenceType),
    /// ref.is_null
    /// Check for a null value.
    IsNull,
    /// ref.func funcidx
    /// Produce a reference to a given function.
    Function(FunctionIndex),
}

impl From<ReferenceInstruction> for Instruction {
    fn from(instruction: ReferenceInstruction) -> Self {
        Self::Reference(instruction)
    }
}

/// Instructions in this group can operate on operands of any value type.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#parametric-instructions>
///
/// # Examples
/// ```rust
/// use wasm_ast::{ParametricInstruction, Instruction, ValueType};
///
/// assert_eq!(
///     Instruction::Parametric(ParametricInstruction::Drop),
///     ParametricInstruction::Drop.into()
/// );
/// assert_eq!(
///     Instruction::Parametric(ParametricInstruction::Select(Some(vec![ValueType::I32]))),
///     ParametricInstruction::Select(Some(vec![ValueType::I32])).into()
/// );
/// assert_eq!(
///     Instruction::Parametric(ParametricInstruction::Select(Some(vec![]))),
///     ParametricInstruction::Select(Some(vec![])).into()
/// );
/// assert_eq!(
///     Instruction::Parametric(ParametricInstruction::Select(None)),
///     ParametricInstruction::Select(None).into()
/// );
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParametricInstruction {
    /// The 𝖽𝗋𝗈𝗉 instruction simply throws away a single operand.
    Drop,
    /// The 𝗌𝖾𝗅𝖾𝖼𝗍 instruction selects one of its first two operands based on whether its third
    /// operand is zero or not. It may include a value type determining the type of these operands.
    /// If missing, the operands must be of numeric type.
    Select(Option<Vec<ValueType>>),
}

impl From<ParametricInstruction> for Instruction {
    fn from(instruction: ParametricInstruction) -> Self {
        Instruction::Parametric(instruction)
    }
}

impl Sequence for ParametricInstruction {
    const CARDINALITY: usize = 1 + cardinality::<ValueType>();

    fn next(&self) -> Option<Self> {
        match self {
            ParametricInstruction::Drop => {
                Some(ParametricInstruction::Select(None))
            },
            ParametricInstruction::Select(None) => {
                let valtype = enum_iterator::first::<ValueType>()?;
                Some(ParametricInstruction::Select(Some(vec![valtype])))
            }
            ParametricInstruction::Select(Some(valtypes)) => {
                let valtype = valtypes.first()?.clone();
                let last_valtype = enum_iterator::last::<ValueType>()?;
                if valtype == last_valtype {
                    None
                }
                else {
                    let new_opt_vec_valtype = Some(vec![enum_iterator::next::<ValueType>(&valtype)?]);
                    Some(ParametricInstruction::Select(new_opt_vec_valtype))
                }
            },
        }
    }

    fn previous(&self) -> Option<Self> {
        match self {
            ParametricInstruction::Drop => {
                None
            },
            ParametricInstruction::Select(None) => {
                Some(ParametricInstruction::Drop)
            },
            ParametricInstruction::Select(opt_vec_valtype) => {
                let valtype = opt_vec_valtype.clone()?.first()?.clone();
                let first_valtype = enum_iterator::first::<ValueType>()?;
                if valtype == first_valtype {
                    Some(ParametricInstruction::Select(None))
                }
                else {
                    let new_opt_vec_valtype = Some(vec![enum_iterator::previous::<ValueType>(&valtype)?]);
                    Some(ParametricInstruction::Select(new_opt_vec_valtype))
                }
            },
        }
    }

    fn first() -> Option<Self> {
        Some(ParametricInstruction::Drop)
    }

    fn last() -> Option<Self> {
        let last_valtype = enum_iterator::last::<ValueType>()?;
        let last_opt_vec_valtype = Some(vec![last_valtype]);
        Some(ParametricInstruction::Select(last_opt_vec_valtype))
    }
}

/// Variable instructions are concerned with access to local or global variables.
/// These instructions get or set the values of variables, respectively.
/// The 𝗅𝗈𝖼𝖺𝗅.𝗍𝖾𝖾 instruction is like 𝗅𝗈𝖼𝖺𝗅.𝗌𝖾𝗍 but also returns its argument.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#variable-instructions>
///
/// # Examples
/// ```rust
/// use wasm_ast::{VariableInstruction, Instruction, ValueType};
///
/// assert_eq!(
///     Instruction::Variable(VariableInstruction::LocalGet(0)),
///     VariableInstruction::LocalGet(0).into()
/// );
/// assert_eq!(
///     Instruction::Variable(VariableInstruction::LocalSet(1)),
///     VariableInstruction::LocalSet(1).into()
/// );
/// assert_eq!(
///     Instruction::Variable(VariableInstruction::LocalTee(1)),
///     VariableInstruction::LocalTee(1).into()
/// );
/// assert_eq!(
///     Instruction::Variable(VariableInstruction::GlobalGet(0)),
///     VariableInstruction::GlobalGet(0).into()
/// );
/// assert_eq!(
///     Instruction::Variable(VariableInstruction::GlobalSet(1)),
///     VariableInstruction::GlobalSet(1).into()
/// );
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum VariableInstruction {
    /// local.get localidx
    /// Get the value of a local variable.
    LocalGet(LocalIndex),
    /// local.set localidx
    /// Set the value of a local variable.
    LocalSet(LocalIndex),
    /// local.tee localidx
    /// The 𝗅𝗈𝖼𝖺𝗅.𝗍𝖾𝖾 instruction is like 𝗅𝗈𝖼𝖺𝗅.𝗌𝖾𝗍 but also returns its argument.
    LocalTee(LocalIndex),
    /// global.get globalidx
    /// Get the value of a global variable.
    GlobalGet(GlobalIndex),
    /// global.set globalidx
    /// Set the value of a global variable.
    GlobalSet(GlobalIndex),
}

impl From<VariableInstruction> for Instruction {
    fn from(instruction: VariableInstruction) -> Self {
        Instruction::Variable(instruction)
    }
}

/// Instructions in this group are concerned with tables table.
/// An additional instruction that accesses a table is the control instruction 𝖼𝖺𝗅𝗅_𝗂𝗇𝖽𝗂𝗋𝖾𝖼𝗍.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#table-instructions>
///
/// # Examples
/// ```rust
/// use wasm_ast::{TableInstruction, Instruction};
///
/// assert_eq!(
///     Instruction::Table(TableInstruction::Get(1)),
///     TableInstruction::Get(1).into()
/// );
/// assert_eq!(
///     Instruction::Table(TableInstruction::Set(1)),
///     TableInstruction::Set(1).into()
/// );
/// assert_eq!(
///     Instruction::Table(TableInstruction::Size(1)),
///     TableInstruction::Size(1).into()
/// );
/// assert_eq!(
///     Instruction::Table(TableInstruction::Grow(1)),
///     TableInstruction::Grow(1).into()
/// );
/// assert_eq!(
///     Instruction::Table(TableInstruction::Fill(1)),
///     TableInstruction::Fill(1).into()
/// );
/// assert_eq!(
///     Instruction::Table(TableInstruction::Copy(0, 1)),
///     TableInstruction::Copy(0, 1).into()
/// );
/// assert_eq!(
///     Instruction::Table(TableInstruction::Init(0, 0)),
///     TableInstruction::Init(0, 0).into()
/// );
/// assert_eq!(
///     Instruction::Table(TableInstruction::ElementDrop(0)),
///     TableInstruction::ElementDrop(0).into()
/// );
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum TableInstruction {
    /// The 𝗍𝖺𝖻𝗅𝖾.𝗀𝖾𝗍 instruction loads an element in a table.
    Get(TableIndex),
    /// The 𝗍𝖺𝖻𝗅𝖾.𝗌𝖾𝗍 instruction stores an element in a table.
    Set(TableIndex),
    /// The 𝗍𝖺𝖻𝗅𝖾.𝗌𝗂𝗓𝖾 instruction returns the current size of a table.
    Size(TableIndex),
    /// The 𝗍𝖺𝖻𝗅𝖾.𝗀𝗋𝗈𝗐 instruction grows table by a given delta and returns the previous size,
    /// or −1 if enough space cannot be allocated.
    /// It also takes an initialization value for the newly allocated entries.
    Grow(TableIndex),
    /// The 𝗍𝖺𝖻𝗅𝖾.𝖿𝗂𝗅𝗅 instruction sets all entries in a range to a given value.
    Fill(TableIndex),
    /// The 𝗍𝖺𝖻𝗅𝖾.𝖼𝗈𝗉𝗒 instruction copies elements from a source table region to a
    /// possibly overlapping destination region; the first index denotes the destination.
    Copy(TableIndex, TableIndex),
    /// The 𝗍𝖺𝖻𝗅𝖾.𝗂𝗇𝗂𝗍 instruction copies elements from a passive element segment into a table.
    Init(ElementIndex, TableIndex),
    /// The 𝖾𝗅𝖾𝗆.𝖽𝗋𝗈𝗉 instruction prevents further use of a passive element segment.
    /// This instruction is intended to be used as an optimization hint.
    /// After an element segment is dropped its elements can no longer be retrieved,
    /// so the memory used by this segment may be freed.
    ElementDrop(ElementIndex),
}

impl From<TableInstruction> for Instruction {
    fn from(instruction: TableInstruction) -> Self {
        Instruction::Table(instruction)
    }
}

/// Instructions in this group are concerned with linear memory.
/// Memory is accessed with 𝗅𝗈𝖺𝖽 and 𝗌𝗍𝗈𝗋𝖾 instructions for the different value types.
/// They all take a memory immediate memarg that contains an address offset and
/// the expected alignment (expressed as the exponent of a power of 2).
/// Integer loads and stores can optionally specify a storage size that is smaller than
/// the bit width of the respective value type.
/// In the case of loads, a sign extension mode sx is then required to select appropriate behavior.
///
/// The static address offset is added to the dynamic address operand,
/// yielding a 33 bit effective address that is the zero-based index at which the memory is accessed.
/// All values are read and written in little endian byte order.
/// A trap results if any of the accessed memory bytes lies outside the address range implied by
/// the memory’s current size.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#memory-instructions>
///
/// # Examples
/// ```rust
/// use wasm_ast::{MemoryInstruction, Instruction, NumberType, MemoryArgument, IntegerType, SignExtension};
///
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Load(NumberType::I32, MemoryArgument::default_offset(4))),
///     MemoryInstruction::Load(NumberType::I32, MemoryArgument::default_offset(4)).into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Load8(IntegerType::I32, SignExtension::Signed, MemoryArgument::default_offset(1))),
///     MemoryInstruction::Load8(IntegerType::I32, SignExtension::Signed, MemoryArgument::default_offset(1)).into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Load16(IntegerType::I64, SignExtension::Unsigned, MemoryArgument::default_offset(2))),
///     MemoryInstruction::Load16(IntegerType::I64, SignExtension::Unsigned, MemoryArgument::default_offset(2)).into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Load32(SignExtension::Signed, MemoryArgument::default_offset(4))),
///     MemoryInstruction::Load32(SignExtension::Signed, MemoryArgument::default_offset(4)).into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Store(NumberType::F64, MemoryArgument::default_offset(8))),
///     MemoryInstruction::Store(NumberType::F64, MemoryArgument::new(8, 0)).into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Store8(IntegerType::I32, MemoryArgument::default_offset(1))),
///     MemoryInstruction::Store8(IntegerType::I32, MemoryArgument::default_offset(1)).into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Store16(IntegerType::I64, MemoryArgument::default_offset(2))),
///     MemoryInstruction::Store16(IntegerType::I64, MemoryArgument::default_offset(2)).into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Store32(MemoryArgument::default_offset(4))),
///     MemoryInstruction::Store32(MemoryArgument::default_offset(4)).into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Size),
///     MemoryInstruction::Size.into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Grow),
///     MemoryInstruction::Grow.into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Fill),
///     MemoryInstruction::Fill.into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Copy),
///     MemoryInstruction::Copy.into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::Init(1)),
///     MemoryInstruction::Init(1).into()
/// );
/// assert_eq!(
///     Instruction::Memory(MemoryInstruction::DataDrop(0)),
///     MemoryInstruction::DataDrop(0).into()
/// );
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum MemoryInstruction {
    /// xnn.load memarg
    /// Load a number type from memory.
    Load(NumberType, MemoryArgument),
    /// xnn.store memarg
    /// Store a number type from memory.
    Store(NumberType, MemoryArgument),
    /// v128.load memarg
    V128Load(MemoryArgument),
    /// v128.store memarg
    V128Store(MemoryArgument),
    /// inn.load8_sx memarg
    /// Integer load that specifies a storage size that is smaller than
    /// the bit width of the respective value type.
    Load8(IntegerType, SignExtension, MemoryArgument),
    /// inn.load16_sx memarg
    Load16(IntegerType, SignExtension, MemoryArgument),
    /// i64.load32_sx memarg
    Load32(SignExtension, MemoryArgument),
    /// inn.store8 memarg
    /// Integer store that specifies a storage size that is smaller than
    /// the bit width of the respective value type.
    Store8(IntegerType, MemoryArgument),
    /// inn.store16 memarg
    Store16(IntegerType, MemoryArgument),
    /// i64.store32 memarg
    Store32(MemoryArgument),
    /// v128.load8x8_sx memarg
    V128Load8X8(SignExtension, MemoryArgument),
    /// v128.load16x4_sx memarg
    V128Load16X4(SignExtension, MemoryArgument),
    /// v128.load32x2_sx memarg
    V128Load32X2(SignExtension, MemoryArgument),
    /// v128.load32_zero memarg
    V128Load32Zero(MemoryArgument),
    /// v128.load64_zero memarg
    V128Load64Zero(MemoryArgument),
    /// v128.load8_splat memarg
    V128Load8Splat(MemoryArgument),
    /// v128.load16_splat memarg
    V128Load16Splat(MemoryArgument),
    /// v128.load32_splat memarg
    V128Load32Splat(MemoryArgument),
    /// v128.load64_splat memarg
    V128Load64Splat(MemoryArgument),
    /// v128.load8_lane memarg laneidx
    V128Load8Lane(MemoryArgument, LaneIdx),
    /// v128.load16_lane memarg laneidx
    V128Load16Lane(MemoryArgument, LaneIdx),
    /// v128.load32_lane memarg laneidx
    V128Load32Lane(MemoryArgument, LaneIdx),
    /// v128.load64_lane memarg laneidx
    V128Load64Lane(MemoryArgument, LaneIdx),
    /// v128.store8_lane memarg laneidx
    V128Store8Lane(MemoryArgument, LaneIdx),
    /// v128.store16_lane memarg laneidx
    V128Store16Lane(MemoryArgument, LaneIdx),
    /// v128.store32_lane memarg laneidx
    V128Store32Lane(MemoryArgument, LaneIdx),
    /// v128.store64_lane memarg laneidx
    V128Store64Lane(MemoryArgument, LaneIdx),
    /// The 𝗆𝖾𝗆𝗈𝗋𝗒.𝗌𝗂𝗓𝖾 instruction returns the current size of a memory.
    /// Operates in units of page size.
    Size,
    /// The 𝗆𝖾𝗆𝗈𝗋𝗒.𝗀𝗋𝗈𝗐 instruction grows memory by a given delta and returns the previous size,
    /// or −1 if enough memory cannot be allocated.
    Grow,
    /// The 𝗆𝖾𝗆𝗈𝗋𝗒.𝖿𝗂𝗅𝗅 instruction sets all values in a region to a given byte.
    Fill,
    /// The 𝗆𝖾𝗆𝗈𝗋𝗒.𝖼𝗈𝗉𝗒 instruction copies data from a source memory region to
    /// a possibly overlapping destination region.
    Copy,
    /// The 𝗆𝖾𝗆𝗈𝗋𝗒.𝗂𝗇𝗂𝗍 instruction copies data from a passive data segment into a memory.
    Init(DataIndex),
    /// he 𝖽𝖺𝗍𝖺.𝖽𝗋𝗈𝗉 instruction prevents further use of a passive data segment.
    /// This instruction is intended to be used as an optimization hint.
    /// After a data segment is dropped its data can no longer be retrieved,
    /// so the memory used by this segment may be freed.
    DataDrop(DataIndex),
}

impl From<MemoryInstruction> for Instruction {
    fn from(instruction: MemoryInstruction) -> Self {
        Instruction::Memory(instruction)
    }
}

/// Instructions in this group affect the flow of control.
/// The 𝖻𝗅𝗈𝖼𝗄, 𝗅𝗈𝗈𝗉 and 𝗂𝖿 instructions are structured instructions.
/// They bracket nested sequences of instructions, called blocks, terminated with, or separated by,
/// 𝖾𝗇𝖽 or 𝖾𝗅𝗌𝖾 pseudo-instructions. As the grammar prescribes, they must be well-nested.
///
/// A structured instruction can consume input and produce output on the operand stack according to
/// its annotated block type. It is given either as a type index that refers to a suitable function
/// type, or as an optional value type inline,
/// which is a shorthand for the function type []→[valtype?].
///
/// Each structured control instruction introduces an implicit label.
/// Labels are targets for branch instructions that reference them with label indices.
/// Unlike with other index spaces, indexing of labels is relative by nesting depth, that is,
/// label 0 refers to the innermost structured control instruction enclosing the referring branch
/// instruction, while increasing indices refer to those farther out.
/// Consequently, labels can only be referenced from within the associated structured control
/// instruction. This also implies that branches can only be directed outwards,
/// “breaking” from the block of the control construct they target.
/// The exact effect depends on that control construct.
/// In case of 𝖻𝗅𝗈𝖼𝗄 or 𝗂𝖿 it is a forward jump, resuming execution after the matching 𝖾𝗇𝖽.
/// In case of 𝗅𝗈𝗈𝗉 it is a backward jump to the beginning of the loop.
///
/// Taking a branch unwinds the operand stack up to the height where the targeted structured
/// control instruction was entered. However, branches may additionally consume operands themselves,
/// which they push back on the operand stack after unwinding.
/// Forward branches require operands according to the output of the targeted block’s type, i.e.,
/// represent the values produced by the terminated block.
/// Backward branches require operands according to the input of the targeted block’s type, i.e.,
/// represent the values consumed by the restarted block.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#control-instructions>
///
/// # Examples
/// ## Simple
/// ```rust
/// use wasm_ast::{ControlInstruction, Instruction};
///
/// assert_eq!(Instruction::Control(ControlInstruction::Nop), ControlInstruction::Nop.into());
/// assert_eq!(Instruction::Control(ControlInstruction::Unreachable), ControlInstruction::Unreachable.into());
/// assert_eq!(Instruction::Control(ControlInstruction::Branch(0)), ControlInstruction::Branch(0).into());
/// assert_eq!(Instruction::Control(ControlInstruction::BranchIf(1)), ControlInstruction::BranchIf(1).into());
/// assert_eq!(Instruction::Control(ControlInstruction::BranchTable(vec![0], 1)), ControlInstruction::BranchTable(vec![0], 1).into());
/// assert_eq!(Instruction::Control(ControlInstruction::Return), ControlInstruction::Return.into());
/// assert_eq!(Instruction::Control(ControlInstruction::Call(1)), ControlInstruction::Call(1).into());
/// assert_eq!(Instruction::Control(ControlInstruction::CallIndirect(0, 1)), ControlInstruction::CallIndirect(0, 1).into());
/// ```
///
/// ## Block
/// ```rust
/// use wasm_ast::{ControlInstruction, Instruction, Expression, BlockType, ValueType};
///
/// let expression = Expression::new(vec![ControlInstruction::Nop.into(), 0i32.into()]);
///
/// assert_eq!(
///     Instruction::Control(ControlInstruction::Block(BlockType::ValueType(ValueType::I32), expression.clone())),
///     ControlInstruction::Block(BlockType::ValueType(ValueType::I32), expression.clone()).into()
/// );
/// ```
///
/// ## Loop
/// ```rust
/// use wasm_ast::{ControlInstruction, Instruction, BlockType, Expression};
/// let expression = Expression::new(vec![ControlInstruction::Nop.into(), 0i32.into()]);
///
/// assert_eq!(
///     Instruction::Control(ControlInstruction::Loop(BlockType::Index(0), expression.clone())),
///     ControlInstruction::Loop(BlockType::Index(0), expression.clone()).into()
/// );
/// ```
///
/// ## If
/// ```rust
/// use wasm_ast::{ControlInstruction, Instruction, Expression, BlockType};
/// let expression = Expression::new(vec![ControlInstruction::Nop.into()]);
///
/// assert_eq!(
///     Instruction::Control(ControlInstruction::If(BlockType::None, expression.clone(), None)),
///     ControlInstruction::If(BlockType::None, expression.clone(), None).into()
/// );
///
/// assert_eq!(
///     Instruction::Control(ControlInstruction::If(BlockType::None, expression.clone(), Some(expression.clone()))),
///     ControlInstruction::If(BlockType::None, expression.clone(), Some(expression.clone())).into()
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Sequence)]
pub enum ControlInstruction {
    /// The 𝗇𝗈𝗉 instruction does nothing.
    Nop,
    /// The 𝗎𝗇𝗋𝖾𝖺𝖼𝗁𝖺𝖻𝗅𝖾 instruction causes an unconditional trap.
    Unreachable,
    /// A logical grouping used introduce a label around an expression.
    Block(BlockType, Expression),
    /// Executes the expression in a loop.
    Loop(BlockType, Expression),
    /// Conditionally executes a positive or (optional) negative branch based on a test value.
    If(BlockType, Expression, Option<Expression>),
    /// The 𝖻𝗋 instruction performs an unconditional branch.
    Branch(LabelIndex),
    /// The 𝖻𝗋_𝗂𝖿 instruction performs a conditional branch
    BranchIf(LabelIndex),
    /// The 𝖻𝗋_𝗍𝖺𝖻𝗅𝖾 instruction performs an indirect branch through an operand indexing into
    /// the label vector that is an immediate to the instruction,
    /// or to a default target if the operand is out of bounds.
    BranchTable(Vec<LabelIndex>, LabelIndex),
    /// The 𝗋𝖾𝗍𝗎𝗋𝗇 instruction is a shortcut for an unconditional branch to the outermost block,
    /// which implicitly is the body of the current function.
    Return,
    /// The 𝖼𝖺𝗅𝗅 instruction invokes another function, consuming the necessary arguments from
    /// the stack and returning the result values of the call.
    Call(FunctionIndex),
    /// The 𝖼𝖺𝗅𝗅_𝗂𝗇𝖽𝗂𝗋𝖾𝖼𝗍 instruction calls a function indirectly through an operand indexing into
    /// a table that is denoted by a table index and must have type 𝖿𝗎𝗇𝖼𝗋𝖾𝖿.
    /// Since it may contain functions of heterogeneous type,
    /// the callee is dynamically checked against the function type indexed by the instruction’s
    /// second immediate, and the call is aborted with a trap if it does not match.
    CallIndirect(TypeIndex, TableIndex),
}

impl From<ControlInstruction> for Instruction {
    fn from(instruction: ControlInstruction) -> Self {
        Instruction::Control(instruction)
    }
}

/// A structured instruction can consume input and produce output on the operand stack according to
/// its annotated block type.
/// It is given either as a type index that refers to a suitable function type,
/// or as an optional value type inline, which is a shorthand for the function type []→[valtype?].
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#control-instructions>
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum BlockType {
    None,
    Index(TypeIndex),
    ValueType(ValueType),
}

/// Argument to load and store instructions that contains an address offset and
/// the expected alignment (expressed as the exponent of a power of 2).
///
/// The static address offset is added to the dynamic address operand,
/// yielding a 33 bit effective address that is the zero-based index at which the memory is accessed.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#memory-instructions>
///
/// # Examples
/// ## With Offset & Alignment
/// ```rust
/// use wasm_ast::MemoryArgument;
///
/// let argument = MemoryArgument::new(4, 42);
///
/// assert_eq!(argument.offset(), 42);
/// assert_eq!(argument.align(), 4);
/// ```
///
/// ## With Offset Only
/// ```rust
/// use wasm_ast::MemoryArgument;
///
/// let argument = MemoryArgument::new(1, 42);
///
/// assert_eq!(argument.offset(), 42);
/// assert_eq!(argument.align(), 1);
/// ```
///
/// ## With Alignment Only
/// ```rust
/// use wasm_ast::MemoryArgument;
///
/// let argument = MemoryArgument::default_offset(4);
///
/// assert_eq!(argument.offset(), 0);
/// assert_eq!(argument.align(), 4);
/// ```
///
/// ## Default
/// ```rust
/// use wasm_ast::MemoryArgument;
///
/// let argument = MemoryArgument::default_offset(1);
///
/// assert_eq!(argument.offset(), 0);
/// assert_eq!(argument.align(), 1);
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub struct MemoryArgument {
    align: u32,
    offset: u32,
}

impl MemoryArgument {
    /// Creates a new memory argument with the given alignment and offset.
    pub fn new(align: u32, offset: u32) -> Self {
        MemoryArgument { align, offset }
    }

    /// Creates a new memory argument with the default offset and the given alignment.
    pub fn default_offset(align: u32) -> Self {
        MemoryArgument { offset: 0, align }
    }

    /// The static address offset of the memory instruction.
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// The memory alignment of the instruction expressed as the exponent of a power of 2.
    pub fn align(&self) -> u32 {
        self.align
    }
}

/// Some integer instructions come in two flavors, where a signedness annotation sx distinguishes
/// whether the operands are to be interpreted as unsigned or signed integers.
/// For the other integer instructions, the use of two’s complement for the signed interpretation
/// means that they behave the same regardless of signedness.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#numeric-instructions>
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum SignExtension {
    Signed,
    Unsigned,
}

/// Function bodies, initialization values for globals,
/// and offsets of element or data segments are given as expressions, which are sequences of instructions terminated by an 𝖾𝗇𝖽 marker.
/// In some places, validation restricts expressions to be constant,
/// which limits the set of allowable instructions.
///
/// See <https://webassembly.github.io/spec/core/syntax/instructions.html#expressions>
///
/// # Examples
/// ## Non-Empty
/// ```rust
/// use wasm_ast::{Expression, ControlInstruction, NumericInstruction, Instruction};
///
/// let expression = Expression::new(vec![0i32.into(), ControlInstruction::Nop.into()]);
///
/// assert_eq!(
///     expression,
///     Expression::new(vec![
///         Instruction::Numeric(NumericInstruction::I32Constant(0 as i32)),
///         Instruction::Control(ControlInstruction::Nop),
///     ])
/// );
/// assert_eq!(expression.instructions(), &[
///     Instruction::Numeric(NumericInstruction::I32Constant(0)),
///     Instruction::Control(ControlInstruction::Nop),
/// ]);
/// assert_eq!(expression.len(), 2);
/// assert!(!expression.is_empty());
/// assert_eq!(
///     expression,
///     vec![
///         Instruction::Numeric(NumericInstruction::I32Constant(0)),
///         Instruction::Control(ControlInstruction::Nop),
///     ].into()
/// );
/// ```
///
/// ## Empty
/// ```rust
/// use wasm_ast::Expression;
///
/// let expression = Expression::new(vec![]);
///
/// assert_eq!(expression, Expression::empty());
/// assert_eq!(expression, vec![].into());
/// assert_eq!(expression.instructions(), &[]);
/// assert_eq!(expression.len(), 0);
/// assert!(expression.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Expression {
    instructions: Vec<Instruction>,
}

impl Expression {
    /// Create a new expression from the given instructions.
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Expression { instructions }
    }

    /// Create a new empty expression.
    pub fn empty() -> Self {
        Expression {
            instructions: vec![],
        }
    }

    /// The instructions for this expression.
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    /// Returns true if this `Expression` has a length of zero, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Returns the length of this `Expression`, in number of instructions.
    pub fn len(&self) -> usize {
        self.instructions.len()
    }
}

impl From<Vec<Instruction>> for Expression {
    fn from(instructions: Vec<Instruction>) -> Self {
        Expression { instructions }
    }
}

impl Sequence for Expression {
    const CARDINALITY: usize = 1;

    fn next(&self) -> Option<Self> {
        None
    }

    fn previous(&self) -> Option<Self> {
        None
    }

    fn first() -> Option<Self> {
        Some(Expression::new(vec![ControlInstruction::Nop.into()].into()))
    }

    fn last() -> Option<Self> {
        Some(Expression::new(vec![ControlInstruction::Nop.into()].into()))
    }
}
