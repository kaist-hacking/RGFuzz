use crate::{VectorInstruction, VectorShape, FShape, IShape};
use crate::emitter::errors::EmitError;
use crate::emitter::types::{emit_reference_type, emit_value_type};
use crate::emitter::values::{
    emit_byte, emit_f32, emit_f64, emit_i32, emit_i64, emit_repeated, emit_u32, emit_vector,
};
use crate::model::{
    BlockType, ControlInstruction, Expression, FloatType, Instruction, IntegerType, MemoryArgument,
    MemoryInstruction, NumberType, NumericInstruction, ParametricInstruction, ReferenceInstruction,
    SignExtension, TableInstruction, VariableInstruction,
};
use std::io::Write;

use super::values::emit_i128;

/// Emit an expression to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#expressions
pub fn emit_expression<O: Write + ?Sized>(
    expression: &Expression,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;
    let expression = expression;

    for instruction in expression.instructions() {
        bytes += emit_instruction(instruction, output)?;
    }

    bytes += emit_byte(0x0Bu8, output)?;

    Ok(bytes)
}

/// Emit an instruction to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#
pub fn emit_instruction<O: Write + ?Sized>(
    instruction: &Instruction,
    output: &mut O,
) -> Result<usize, EmitError> {
    match instruction {
        Instruction::Numeric(instruction) => emit_numeric_instruction(instruction, output),
        Instruction::Reference(instruction) => emit_reference_instruction(instruction, output),
        Instruction::Parametric(instruction) => emit_parametric_instruction(instruction, output),
        Instruction::Variable(instruction) => emit_variable_instruction(instruction, output),
        Instruction::Table(instruction) => emit_table_instruction(instruction, output),
        Instruction::Memory(instruction) => emit_memory_instruction(instruction, output),
        Instruction::Control(instruction) => emit_control_instruction(instruction, output),
        Instruction::Vector(instruction) => emit_vector_instruction(instruction, output),
    }
}

/// Emit a numeric instruction to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#numeric-instructions
fn emit_numeric_instruction<O: Write + ?Sized>(
    instruction: &NumericInstruction,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    match instruction {
        // Constant Operations
        NumericInstruction::I32Constant(value) => {
            bytes += emit_byte(0x41u8, output)?;
            bytes += emit_i32(value, output)?;
        }
        NumericInstruction::I64Constant(value) => {
            bytes += emit_byte(0x42u8, output)?;
            bytes += emit_i64(value, output)?;
        }
        NumericInstruction::F32Constant(value) => {
            bytes += emit_byte(0x43u8, output)?;
            bytes += emit_f32(value, output)?;
        }
        NumericInstruction::F64Constant(value) => {
            bytes += emit_byte(0x44u8, output)?;
            bytes += emit_f64(value, output)?;
        }
        // i32 Test Operations
        NumericInstruction::EqualToZero(IntegerType::I32) => {
            bytes += emit_byte(0x45u8, output)?;
        }
        // i32 Relation Operations
        NumericInstruction::Equal(NumberType::I32) => {
            bytes += emit_byte(0x46u8, output)?;
        }
        NumericInstruction::NotEqual(NumberType::I32) => {
            bytes += emit_byte(0x47u8, output)?;
        }
        NumericInstruction::LessThanInteger(IntegerType::I32, SignExtension::Signed) => {
            bytes += emit_byte(0x48u8, output)?;
        }
        NumericInstruction::LessThanInteger(IntegerType::I32, SignExtension::Unsigned) => {
            bytes += emit_byte(0x49u8, output)?;
        }
        NumericInstruction::GreaterThanInteger(IntegerType::I32, SignExtension::Signed) => {
            bytes += emit_byte(0x4Au8, output)?;
        }
        NumericInstruction::GreaterThanInteger(IntegerType::I32, SignExtension::Unsigned) => {
            bytes += emit_byte(0x4Bu8, output)?;
        }
        NumericInstruction::LessThanOrEqualToInteger(IntegerType::I32, SignExtension::Signed) => {
            bytes += emit_byte(0x4Cu8, output)?;
        }
        NumericInstruction::LessThanOrEqualToInteger(IntegerType::I32, SignExtension::Unsigned) => {
            bytes += emit_byte(0x4Du8, output)?;
        }
        NumericInstruction::GreaterThanOrEqualToInteger(
            IntegerType::I32,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0x4Eu8, output)?;
        }
        NumericInstruction::GreaterThanOrEqualToInteger(
            IntegerType::I32,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0x4Fu8, output)?;
        }
        // i64 Test Operations
        NumericInstruction::EqualToZero(IntegerType::I64) => {
            bytes += emit_byte(0x50u8, output)?;
        }
        // i64 Relation Operations
        NumericInstruction::Equal(NumberType::I64) => {
            bytes += emit_byte(0x51u8, output)?;
        }
        NumericInstruction::NotEqual(NumberType::I64) => {
            bytes += emit_byte(0x52u8, output)?;
        }
        NumericInstruction::LessThanInteger(IntegerType::I64, SignExtension::Signed) => {
            bytes += emit_byte(0x53u8, output)?;
        }
        NumericInstruction::LessThanInteger(IntegerType::I64, SignExtension::Unsigned) => {
            bytes += emit_byte(0x54u8, output)?;
        }
        NumericInstruction::GreaterThanInteger(IntegerType::I64, SignExtension::Signed) => {
            bytes += emit_byte(0x55u8, output)?;
        }
        NumericInstruction::GreaterThanInteger(IntegerType::I64, SignExtension::Unsigned) => {
            bytes += emit_byte(0x56u8, output)?;
        }
        NumericInstruction::LessThanOrEqualToInteger(IntegerType::I64, SignExtension::Signed) => {
            bytes += emit_byte(0x57u8, output)?;
        }
        NumericInstruction::LessThanOrEqualToInteger(IntegerType::I64, SignExtension::Unsigned) => {
            bytes += emit_byte(0x58u8, output)?;
        }
        NumericInstruction::GreaterThanOrEqualToInteger(
            IntegerType::I64,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0x59u8, output)?;
        }
        NumericInstruction::GreaterThanOrEqualToInteger(
            IntegerType::I64,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0x5Au8, output)?;
        }
        // f32 Relation Operations
        NumericInstruction::Equal(NumberType::F32) => {
            bytes += emit_byte(0x5Bu8, output)?;
        }
        NumericInstruction::NotEqual(NumberType::F32) => {
            bytes += emit_byte(0x5Cu8, output)?;
        }
        NumericInstruction::LessThanFloat(FloatType::F32) => {
            bytes += emit_byte(0x5Du8, output)?;
        }
        NumericInstruction::GreaterThanFloat(FloatType::F32) => {
            bytes += emit_byte(0x5Eu8, output)?;
        }
        NumericInstruction::LessThanOrEqualToFloat(FloatType::F32) => {
            bytes += emit_byte(0x5Fu8, output)?;
        }
        NumericInstruction::GreaterThanOrEqualToFloat(FloatType::F32) => {
            bytes += emit_byte(0x60u8, output)?;
        }
        // f64 Relation Operations
        NumericInstruction::Equal(NumberType::F64) => {
            bytes += emit_byte(0x61u8, output)?;
        }
        NumericInstruction::NotEqual(NumberType::F64) => {
            bytes += emit_byte(0x62u8, output)?;
        }
        NumericInstruction::LessThanFloat(FloatType::F64) => {
            bytes += emit_byte(0x63u8, output)?;
        }
        NumericInstruction::GreaterThanFloat(FloatType::F64) => {
            bytes += emit_byte(0x64u8, output)?;
        }
        NumericInstruction::LessThanOrEqualToFloat(FloatType::F64) => {
            bytes += emit_byte(0x65u8, output)?;
        }
        NumericInstruction::GreaterThanOrEqualToFloat(FloatType::F64) => {
            bytes += emit_byte(0x66u8, output)?;
        }
        // i32 Unary Operations
        NumericInstruction::CountLeadingZeros(IntegerType::I32) => {
            bytes += emit_byte(0x67u8, output)?;
        }
        NumericInstruction::CountTrailingZeros(IntegerType::I32) => {
            bytes += emit_byte(0x68u8, output)?;
        }
        NumericInstruction::CountOnes(IntegerType::I32) => {
            bytes += emit_byte(0x69u8, output)?;
        }
        // i32 Binary Operations
        NumericInstruction::Add(NumberType::I32) => {
            bytes += emit_byte(0x6Au8, output)?;
        }
        NumericInstruction::Subtract(NumberType::I32) => {
            bytes += emit_byte(0x6Bu8, output)?;
        }
        NumericInstruction::Multiply(NumberType::I32) => {
            bytes += emit_byte(0x6Cu8, output)?;
        }
        NumericInstruction::DivideInteger(IntegerType::I32, SignExtension::Signed) => {
            bytes += emit_byte(0x6Du8, output)?;
        }
        NumericInstruction::DivideInteger(IntegerType::I32, SignExtension::Unsigned) => {
            bytes += emit_byte(0x6Eu8, output)?;
        }
        NumericInstruction::Remainder(IntegerType::I32, SignExtension::Signed) => {
            bytes += emit_byte(0x6Fu8, output)?;
        }
        NumericInstruction::Remainder(IntegerType::I32, SignExtension::Unsigned) => {
            bytes += emit_byte(0x70u8, output)?;
        }
        NumericInstruction::And(IntegerType::I32) => {
            bytes += emit_byte(0x71u8, output)?;
        }
        NumericInstruction::Or(IntegerType::I32) => {
            bytes += emit_byte(0x72u8, output)?;
        }
        NumericInstruction::Xor(IntegerType::I32) => {
            bytes += emit_byte(0x73u8, output)?;
        }
        NumericInstruction::ShiftLeft(IntegerType::I32) => {
            bytes += emit_byte(0x74u8, output)?;
        }
        NumericInstruction::ShiftRight(IntegerType::I32, SignExtension::Signed) => {
            bytes += emit_byte(0x75u8, output)?;
        }
        NumericInstruction::ShiftRight(IntegerType::I32, SignExtension::Unsigned) => {
            bytes += emit_byte(0x76u8, output)?;
        }
        NumericInstruction::RotateLeft(IntegerType::I32) => {
            bytes += emit_byte(0x77u8, output)?;
        }
        NumericInstruction::RotateRight(IntegerType::I32) => {
            bytes += emit_byte(0x78u8, output)?;
        }
        // i64 Unary Operations
        NumericInstruction::CountLeadingZeros(IntegerType::I64) => {
            bytes += emit_byte(0x79u8, output)?;
        }
        NumericInstruction::CountTrailingZeros(IntegerType::I64) => {
            bytes += emit_byte(0x7Au8, output)?;
        }
        NumericInstruction::CountOnes(IntegerType::I64) => {
            bytes += emit_byte(0x7Bu8, output)?;
        }
        // i64 Binary Operations
        NumericInstruction::Add(NumberType::I64) => {
            bytes += emit_byte(0x7Cu8, output)?;
        }
        NumericInstruction::Subtract(NumberType::I64) => {
            bytes += emit_byte(0x7Du8, output)?;
        }
        NumericInstruction::Multiply(NumberType::I64) => {
            bytes += emit_byte(0x7Eu8, output)?;
        }
        NumericInstruction::DivideInteger(IntegerType::I64, SignExtension::Signed) => {
            bytes += emit_byte(0x7Fu8, output)?;
        }
        NumericInstruction::DivideInteger(IntegerType::I64, SignExtension::Unsigned) => {
            bytes += emit_byte(0x80u8, output)?;
        }
        NumericInstruction::Remainder(IntegerType::I64, SignExtension::Signed) => {
            bytes += emit_byte(0x81u8, output)?;
        }
        NumericInstruction::Remainder(IntegerType::I64, SignExtension::Unsigned) => {
            bytes += emit_byte(0x82u8, output)?;
        }
        NumericInstruction::And(IntegerType::I64) => {
            bytes += emit_byte(0x83u8, output)?;
        }
        NumericInstruction::Or(IntegerType::I64) => {
            bytes += emit_byte(0x84u8, output)?;
        }
        NumericInstruction::Xor(IntegerType::I64) => {
            bytes += emit_byte(0x85u8, output)?;
        }
        NumericInstruction::ShiftLeft(IntegerType::I64) => {
            bytes += emit_byte(0x86u8, output)?;
        }
        NumericInstruction::ShiftRight(IntegerType::I64, SignExtension::Signed) => {
            bytes += emit_byte(0x87u8, output)?;
        }
        NumericInstruction::ShiftRight(IntegerType::I64, SignExtension::Unsigned) => {
            bytes += emit_byte(0x88u8, output)?;
        }
        NumericInstruction::RotateLeft(IntegerType::I64) => {
            bytes += emit_byte(0x89u8, output)?;
        }
        NumericInstruction::RotateRight(IntegerType::I64) => {
            bytes += emit_byte(0x8Au8, output)?;
        }
        // f32 Unary Operations
        NumericInstruction::AbsoluteValue(FloatType::F32) => {
            bytes += emit_byte(0x8Bu8, output)?;
        }
        NumericInstruction::Negate(FloatType::F32) => {
            bytes += emit_byte(0x8Cu8, output)?;
        }
        NumericInstruction::Ceiling(FloatType::F32) => {
            bytes += emit_byte(0x8Du8, output)?;
        }
        NumericInstruction::Floor(FloatType::F32) => {
            bytes += emit_byte(0x8Eu8, output)?;
        }
        NumericInstruction::Truncate(FloatType::F32) => {
            bytes += emit_byte(0x8Fu8, output)?;
        }
        NumericInstruction::Nearest(FloatType::F32) => {
            bytes += emit_byte(0x90u8, output)?;
        }
        NumericInstruction::SquareRoot(FloatType::F32) => {
            bytes += emit_byte(0x91u8, output)?;
        }
        // f32 Binary Operations
        NumericInstruction::Add(NumberType::F32) => {
            bytes += emit_byte(0x92u8, output)?;
        }
        NumericInstruction::Subtract(NumberType::F32) => {
            bytes += emit_byte(0x93u8, output)?;
        }
        NumericInstruction::Multiply(NumberType::F32) => {
            bytes += emit_byte(0x94u8, output)?;
        }
        NumericInstruction::DivideFloat(FloatType::F32) => {
            bytes += emit_byte(0x95u8, output)?;
        }
        NumericInstruction::Minimum(FloatType::F32) => {
            bytes += emit_byte(0x96u8, output)?;
        }
        NumericInstruction::Maximum(FloatType::F32) => {
            bytes += emit_byte(0x97u8, output)?;
        }
        NumericInstruction::CopySign(FloatType::F32) => {
            bytes += emit_byte(0x98u8, output)?;
        }
        // f64 Unary Operations
        NumericInstruction::AbsoluteValue(FloatType::F64) => {
            bytes += emit_byte(0x99u8, output)?;
        }
        NumericInstruction::Negate(FloatType::F64) => {
            bytes += emit_byte(0x9Au8, output)?;
        }
        NumericInstruction::Ceiling(FloatType::F64) => {
            bytes += emit_byte(0x9Bu8, output)?;
        }
        NumericInstruction::Floor(FloatType::F64) => {
            bytes += emit_byte(0x9Cu8, output)?;
        }
        NumericInstruction::Truncate(FloatType::F64) => {
            bytes += emit_byte(0x9Du8, output)?;
        }
        NumericInstruction::Nearest(FloatType::F64) => {
            bytes += emit_byte(0x9Eu8, output)?;
        }
        NumericInstruction::SquareRoot(FloatType::F64) => {
            bytes += emit_byte(0x9Fu8, output)?;
        }
        // f64 Binary Operations
        NumericInstruction::Add(NumberType::F64) => {
            bytes += emit_byte(0xA0u8, output)?;
        }
        NumericInstruction::Subtract(NumberType::F64) => {
            bytes += emit_byte(0xA1u8, output)?;
        }
        NumericInstruction::Multiply(NumberType::F64) => {
            bytes += emit_byte(0xA2u8, output)?;
        }
        NumericInstruction::DivideFloat(FloatType::F64) => {
            bytes += emit_byte(0xA3u8, output)?;
        }
        NumericInstruction::Minimum(FloatType::F64) => {
            bytes += emit_byte(0xA4u8, output)?;
        }
        NumericInstruction::Maximum(FloatType::F64) => {
            bytes += emit_byte(0xA5u8, output)?;
        }
        NumericInstruction::CopySign(FloatType::F64) => {
            bytes += emit_byte(0xA6u8, output)?;
        }
        // Convert Operations
        NumericInstruction::Wrap => {
            bytes += emit_byte(0xA7u8, output)?;
        }
        NumericInstruction::ConvertAndTruncate(
            IntegerType::I32,
            FloatType::F32,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0xA8u8, output)?;
        }
        NumericInstruction::ConvertAndTruncate(
            IntegerType::I32,
            FloatType::F32,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0xA9u8, output)?;
        }
        NumericInstruction::ConvertAndTruncate(
            IntegerType::I32,
            FloatType::F64,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0xAAu8, output)?;
        }
        NumericInstruction::ConvertAndTruncate(
            IntegerType::I32,
            FloatType::F64,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0xABu8, output)?;
        }
        NumericInstruction::ExtendWithSignExtension(SignExtension::Signed) => {
            bytes += emit_byte(0xACu8, output)?;
        }
        NumericInstruction::ExtendWithSignExtension(SignExtension::Unsigned) => {
            bytes += emit_byte(0xADu8, output)?;
        }
        NumericInstruction::ConvertAndTruncate(
            IntegerType::I64,
            FloatType::F32,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0xAEu8, output)?;
        }
        NumericInstruction::ConvertAndTruncate(
            IntegerType::I64,
            FloatType::F32,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0xAFu8, output)?;
        }
        NumericInstruction::ConvertAndTruncate(
            IntegerType::I64,
            FloatType::F64,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0xB0u8, output)?;
        }
        NumericInstruction::ConvertAndTruncate(
            IntegerType::I64,
            FloatType::F64,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0xB1u8, output)?;
        }
        NumericInstruction::Convert(FloatType::F32, IntegerType::I32, SignExtension::Signed) => {
            bytes += emit_byte(0xB2u8, output)?;
        }
        NumericInstruction::Convert(FloatType::F32, IntegerType::I32, SignExtension::Unsigned) => {
            bytes += emit_byte(0xB3u8, output)?;
        }
        NumericInstruction::Convert(FloatType::F32, IntegerType::I64, SignExtension::Signed) => {
            bytes += emit_byte(0xB4u8, output)?;
        }
        NumericInstruction::Convert(FloatType::F32, IntegerType::I64, SignExtension::Unsigned) => {
            bytes += emit_byte(0xB5u8, output)?;
        }
        NumericInstruction::Demote => {
            bytes += emit_byte(0xB6u8, output)?;
        }
        NumericInstruction::Convert(FloatType::F64, IntegerType::I32, SignExtension::Signed) => {
            bytes += emit_byte(0xB7u8, output)?;
        }
        NumericInstruction::Convert(FloatType::F64, IntegerType::I32, SignExtension::Unsigned) => {
            bytes += emit_byte(0xB8u8, output)?;
        }
        NumericInstruction::Convert(FloatType::F64, IntegerType::I64, SignExtension::Signed) => {
            bytes += emit_byte(0xB9u8, output)?;
        }
        NumericInstruction::Convert(FloatType::F64, IntegerType::I64, SignExtension::Unsigned) => {
            bytes += emit_byte(0xBAu8, output)?;
        }
        NumericInstruction::Promote => {
            bytes += emit_byte(0xBBu8, output)?;
        }
        NumericInstruction::ReinterpretFloat(IntegerType::I32) => {
            bytes += emit_byte(0xBCu8, output)?;
        }
        NumericInstruction::ReinterpretFloat(IntegerType::I64) => {
            bytes += emit_byte(0xBDu8, output)?;
        }
        NumericInstruction::ReinterpretInteger(FloatType::F32) => {
            bytes += emit_byte(0xBEu8, output)?;
        }
        NumericInstruction::ReinterpretInteger(FloatType::F64) => {
            bytes += emit_byte(0xBFu8, output)?;
        }
        NumericInstruction::ExtendSigned8(IntegerType::I32) => {
            bytes += emit_byte(0xC0u8, output)?;
        }
        NumericInstruction::ExtendSigned16(IntegerType::I32) => {
            bytes += emit_byte(0xC1u8, output)?;
        }
        NumericInstruction::ExtendSigned8(IntegerType::I64) => {
            bytes += emit_byte(0xC2u8, output)?;
        }
        NumericInstruction::ExtendSigned16(IntegerType::I64) => {
            bytes += emit_byte(0xC3u8, output)?;
        }
        NumericInstruction::ExtendSigned32 => {
            bytes += emit_byte(0xC4u8, output)?;
        }
        NumericInstruction::ConvertAndTruncateWithSaturation(
            IntegerType::I32,
            FloatType::F32,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(0u32, output)?;
        }
        NumericInstruction::ConvertAndTruncateWithSaturation(
            IntegerType::I32,
            FloatType::F32,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(1u32, output)?;
        }
        NumericInstruction::ConvertAndTruncateWithSaturation(
            IntegerType::I32,
            FloatType::F64,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(2u32, output)?;
        }
        NumericInstruction::ConvertAndTruncateWithSaturation(
            IntegerType::I32,
            FloatType::F64,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(3u32, output)?;
        }
        NumericInstruction::ConvertAndTruncateWithSaturation(
            IntegerType::I64,
            FloatType::F32,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(4u32, output)?;
        }
        NumericInstruction::ConvertAndTruncateWithSaturation(
            IntegerType::I64,
            FloatType::F32,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(5u32, output)?;
        }
        NumericInstruction::ConvertAndTruncateWithSaturation(
            IntegerType::I64,
            FloatType::F64,
            SignExtension::Signed,
        ) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(6u32, output)?;
        }
        NumericInstruction::ConvertAndTruncateWithSaturation(
            IntegerType::I64,
            FloatType::F64,
            SignExtension::Unsigned,
        ) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(7u32, output)?;
        }
    }

    Ok(bytes)
}

/// Emit a reference instruction to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#reference-instructions
pub fn emit_reference_instruction<O: Write + ?Sized>(
    instruction: &ReferenceInstruction,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    match instruction {
        ReferenceInstruction::Null(kind) => {
            bytes += emit_byte(0xD0u8, output)?;
            bytes += emit_reference_type(*kind, output)?;
        }
        ReferenceInstruction::IsNull => {
            bytes += emit_byte(0xD1u8, output)?;
        }
        ReferenceInstruction::Function(index) => {
            bytes += emit_byte(0xD2u8, output)?;
            bytes += emit_u32(index, output)?;
        }
    }

    Ok(bytes)
}

/// Emit a parametric instruction to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#parametric-instructions
pub fn emit_parametric_instruction<O: Write + ?Sized>(
    instruction: &ParametricInstruction,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    match instruction {
        ParametricInstruction::Drop => {
            bytes += emit_byte(0x1Au8, output)?;
        }
        ParametricInstruction::Select(Some(types)) => {
            bytes += emit_byte(0x1Cu8, output)?;
            bytes += emit_vector(types, output, emit_value_type)?;
        }
        ParametricInstruction::Select(None) => {
            bytes += emit_byte(0x1Bu8, output)?;
        }
    }

    Ok(bytes)
}

/// Emit a variable instruction to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#variable-instructions
fn emit_variable_instruction<O: Write + ?Sized>(
    instruction: &VariableInstruction,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    match instruction {
        VariableInstruction::LocalGet(index) => {
            bytes += emit_byte(0x20u8, output)?;
            bytes += emit_u32(index, output)?;
        }
        VariableInstruction::LocalSet(index) => {
            bytes += emit_byte(0x21u8, output)?;
            bytes += emit_u32(index, output)?;
        }
        VariableInstruction::LocalTee(index) => {
            bytes += emit_byte(0x22u8, output)?;
            bytes += emit_u32(index, output)?;
        }
        VariableInstruction::GlobalGet(index) => {
            bytes += emit_byte(0x23u8, output)?;
            bytes += emit_u32(index, output)?;
        }
        VariableInstruction::GlobalSet(index) => {
            bytes += emit_byte(0x24u8, output)?;
            bytes += emit_u32(index, output)?;
        }
    }

    Ok(bytes)
}

/// Emit a table instruction to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#table-instructions
fn emit_table_instruction<O: Write + ?Sized>(
    instruction: &TableInstruction,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    match instruction {
        TableInstruction::Get(index) => {
            bytes += emit_byte(0x25u8, output)?;
            bytes += emit_u32(index, output)?;
        }
        TableInstruction::Set(index) => {
            bytes += emit_byte(0x26u8, output)?;
            bytes += emit_u32(index, output)?;
        }
        TableInstruction::Init(element, table) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(12u32, output)?;
            bytes += emit_u32(element, output)?;
            bytes += emit_u32(table, output)?;
        }
        TableInstruction::ElementDrop(index) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(13u32, output)?;
            bytes += emit_u32(index, output)?;
        }
        TableInstruction::Copy(table_a, table_b) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(14u32, output)?;
            bytes += emit_u32(table_a, output)?;
            bytes += emit_u32(table_b, output)?;
        }
        TableInstruction::Grow(index) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(15u32, output)?;
            bytes += emit_u32(index, output)?;
        }
        TableInstruction::Size(index) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(16u32, output)?;
            bytes += emit_u32(index, output)?;
        }
        TableInstruction::Fill(index) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(17u32, output)?;
            bytes += emit_u32(index, output)?;
        }
    }

    Ok(bytes)
}

/// Emit a memory instruction to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#memory-instructions
pub fn emit_memory_instruction<O: Write + ?Sized>(
    instruction: &MemoryInstruction,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    match instruction {
        MemoryInstruction::Load(NumberType::I32, memory_argument) => {
            bytes += emit_byte(0x28u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load(NumberType::I64, memory_argument) => {
            bytes += emit_byte(0x29u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load(NumberType::F32, memory_argument) => {
            bytes += emit_byte(0x2Au8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load(NumberType::F64, memory_argument) => {
            bytes += emit_byte(0x2Bu8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load(memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(0u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load8(IntegerType::I32, SignExtension::Signed, memory_argument) => {
            bytes += emit_byte(0x2Cu8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load8(IntegerType::I32, SignExtension::Unsigned, memory_argument) => {
            bytes += emit_byte(0x2Du8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load16(IntegerType::I32, SignExtension::Signed, memory_argument) => {
            bytes += emit_byte(0x2Eu8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load16(IntegerType::I32, SignExtension::Unsigned, memory_argument) => {
            bytes += emit_byte(0x2Fu8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load8(IntegerType::I64, SignExtension::Signed, memory_argument) => {
            bytes += emit_byte(0x30u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load8(IntegerType::I64, SignExtension::Unsigned, memory_argument) => {
            bytes += emit_byte(0x31u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load16(IntegerType::I64, SignExtension::Signed, memory_argument) => {
            bytes += emit_byte(0x32u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load16(IntegerType::I64, SignExtension::Unsigned, memory_argument) => {
            bytes += emit_byte(0x33u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load32(SignExtension::Signed, memory_argument) => {
            bytes += emit_byte(0x34u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Load32(SignExtension::Unsigned, memory_argument) => {
            bytes += emit_byte(0x35u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Store(NumberType::I32, memory_argument) => {
            bytes += emit_byte(0x36u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Store(NumberType::I64, memory_argument) => {
            bytes += emit_byte(0x37u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Store(NumberType::F32, memory_argument) => {
            bytes += emit_byte(0x38u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Store(NumberType::F64, memory_argument) => {
            bytes += emit_byte(0x39u8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Store(memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(11u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Store8(IntegerType::I32, memory_argument) => {
            bytes += emit_byte(0x3Au8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Store16(IntegerType::I32, memory_argument) => {
            bytes += emit_byte(0x3Bu8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Store8(IntegerType::I64, memory_argument) => {
            bytes += emit_byte(0x3Cu8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Store16(IntegerType::I64, memory_argument) => {
            bytes += emit_byte(0x3Du8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::Store32(memory_argument) => {
            bytes += emit_byte(0x3Eu8, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load8X8(SignExtension::Signed, memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(1u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load8X8(SignExtension::Unsigned, memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(2u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load16X4(SignExtension::Signed, memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(3u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load16X4(SignExtension::Unsigned, memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(4u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load32X2(SignExtension::Signed, memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(5u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load32X2(SignExtension::Unsigned, memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(6u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load32Zero(memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(92u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load64Zero(memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(93u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load8Splat(memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(7u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load16Splat(memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(8u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load32Splat(memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(9u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load64Splat(memory_argument) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(10u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
        }
        MemoryInstruction::V128Load8Lane(memory_argument, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(84u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
            bytes += emit_byte(*laneidx, output)?;
        }
        MemoryInstruction::V128Load16Lane(memory_argument, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(85u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
            bytes += emit_byte(*laneidx, output)?;
        }
        MemoryInstruction::V128Load32Lane(memory_argument, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(86u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
            bytes += emit_byte(*laneidx, output)?;
        }
        MemoryInstruction::V128Load64Lane(memory_argument, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(87u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
            bytes += emit_byte(*laneidx, output)?;
        }
        MemoryInstruction::V128Store8Lane(memory_argument, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(88u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
            bytes += emit_byte(*laneidx, output)?;
        }
        MemoryInstruction::V128Store16Lane(memory_argument, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(89u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
            bytes += emit_byte(*laneidx, output)?;
        }
        MemoryInstruction::V128Store32Lane(memory_argument, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(90u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
            bytes += emit_byte(*laneidx, output)?;
        }
        MemoryInstruction::V128Store64Lane(memory_argument, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(91u32, output)?;
            bytes += emit_memory_argument(memory_argument, output)?;
            bytes += emit_byte(*laneidx, output)?;
        }
        MemoryInstruction::Size => {
            bytes += emit_byte(0x3Fu8, output)?;
            bytes += emit_byte(0x00u8, output)?;
        }
        MemoryInstruction::Grow => {
            bytes += emit_byte(0x40u8, output)?;
            bytes += emit_byte(0x00u8, output)?;
        }
        MemoryInstruction::Init(index) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(8u32, output)?;
            bytes += emit_u32(index, output)?;
            bytes += emit_byte(0x00u8, output)?;
        }
        MemoryInstruction::DataDrop(index) => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(9u32, output)?;
            bytes += emit_u32(index, output)?;
        }
        MemoryInstruction::Copy => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(10u32, output)?;
            bytes += emit_byte(0x00u8, output)?;
            bytes += emit_byte(0x00u8, output)?;
        }
        MemoryInstruction::Fill => {
            bytes += emit_byte(0xFCu8, output)?;
            bytes += emit_u32(11u32, output)?;
            bytes += emit_byte(0x00u8, output)?;
        }
    }

    Ok(bytes)
}

/// Emit a control instruction to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#control-instructions
fn emit_control_instruction<O: Write + ?Sized>(
    instruction: &ControlInstruction,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    match instruction {
        ControlInstruction::Unreachable => {
            bytes += emit_byte(0x00u8, output)?;
        }
        ControlInstruction::Nop => {
            bytes += emit_byte(0x01u8, output)?;
        }
        ControlInstruction::Block(kind, expression) => {
            bytes += emit_byte(0x02u8, output)?;
            bytes += emit_block_type(kind, output)?;
            bytes += emit_expression(expression, output)?;
        }
        ControlInstruction::Loop(kind, expression) => {
            bytes += emit_byte(0x03u8, output)?;
            bytes += emit_block_type(kind, output)?;
            bytes += emit_expression(expression, output)?;
        }
        ControlInstruction::If(kind, positive, negative) => {
            bytes += emit_byte(0x04u8, output)?;
            bytes += emit_block_type(kind, output)?;

            if let Some(negative) = negative {
                bytes += emit_repeated(positive.instructions(), output, emit_instruction)?;
                bytes += emit_byte(0x05u8, output)?;
                bytes += emit_expression(negative, output)?;
            } else {
                bytes += emit_expression(positive, output)?;
            }
        }
        ControlInstruction::Branch(index) => {
            bytes += emit_byte(0x0Cu8, output)?;
            bytes += emit_u32(index, output)?;
        }
        ControlInstruction::BranchIf(index) => {
            bytes += emit_byte(0x0Du8, output)?;
            bytes += emit_u32(index, output)?;
        }
        ControlInstruction::BranchTable(indices, index) => {
            bytes += emit_byte(0x0Eu8, output)?;
            bytes += emit_vector(indices, output, emit_u32)?;
            bytes += emit_u32(index, output)?;
        }
        ControlInstruction::Return => {
            bytes += emit_byte(0x0Fu8, output)?;
        }
        ControlInstruction::Call(index) => {
            bytes += emit_byte(0x10u8, output)?;
            bytes += emit_u32(index, output)?;
        }
        ControlInstruction::CallIndirect(table, kind) => {
            bytes += emit_byte(0x11u8, output)?;
            bytes += emit_u32(kind, output)?;
            bytes += emit_u32(table, output)?;
        }
    }

    Ok(bytes)
}

/// Emit a block type to the output.
///
/// See  https://webassembly.github.io/spec/core/binary/instructions.html#control-instructions
pub fn emit_block_type<O: Write + ?Sized>(
    kind: &BlockType,
    output: &mut O,
) -> Result<usize, EmitError> {
    match kind {
        BlockType::Index(index) => emit_i64(*index as i64, output),
        BlockType::ValueType(kind) => emit_value_type(kind, output),
        BlockType::None => emit_byte(0x40u8, output),
    }
}

/// Emit a memory argument to the output.
///
/// See https://webassembly.github.io/spec/core/binary/instructions.html#memory-instructions
pub fn emit_memory_argument<O: Write + ?Sized>(
    argument: &MemoryArgument,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    bytes += emit_u32(argument.align(), output)?;
    bytes += emit_u32(argument.offset(), output)?;

    Ok(bytes)
}

fn emit_vector_instruction<O: Write + ?Sized>(
    instruction: &VectorInstruction,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    match instruction {
        VectorInstruction::V128Constant(val) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(12u32, output)?;
            bytes += emit_i128(*val, output)?;
        },
        VectorInstruction::Not => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(77u32, output)?;
        },
        VectorInstruction::And => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(78u32, output)?;
        },
        VectorInstruction::AndNot => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(79u32, output)?;
        },
        VectorInstruction::Or => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(80u32, output)?;
        },
        VectorInstruction::Xor => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(81u32, output)?;
        },
        VectorInstruction::BitSelect => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(82u32, output)?;
        },
        VectorInstruction::AnyTrue => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(83u32, output)?;
        },
        VectorInstruction::I8X16Shuffle(laneidx16) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(13u32, output)?;
            for laneidx in laneidx16 {
                bytes += emit_byte(*laneidx, output)?;
            }
        },
        VectorInstruction::I8X16Swizzle => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(14u32, output)?;
        },
        VectorInstruction::Splat(VectorShape::I8X16) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(15u32, output)?;
        },
        VectorInstruction::Splat(VectorShape::I16X8) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(16u32, output)?;
        },
        VectorInstruction::Splat(VectorShape::I32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(17u32, output)?;
        },
        VectorInstruction::Splat(VectorShape::I64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(18u32, output)?;
        },
        VectorInstruction::Splat(VectorShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(19u32, output)?;
        },
        VectorInstruction::Splat(VectorShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(20u32, output)?;
        },
        VectorInstruction::I8X16ExtractLane(SignExtension::Signed, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(21u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::I8X16ExtractLane(SignExtension::Unsigned, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(22u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::ReplaceLane(VectorShape::I8X16, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(23u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::I16X8ExtractLane(SignExtension::Signed, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(24u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::I16X8ExtractLane(SignExtension::Unsigned, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(25u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::ReplaceLane(VectorShape::I16X8, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(26u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::I32X4ExtractLane(laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(27u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::ReplaceLane(VectorShape::I32X4, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(28u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::I64X2ExtractLane(laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(29u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::ReplaceLane(VectorShape::I64X2, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(30u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::FExtractLane(FShape::F32X4, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(31u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::ReplaceLane(VectorShape::F32X4, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(32u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::FExtractLane(FShape::F64X2, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(33u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::ReplaceLane(VectorShape::F64X2, laneidx) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(34u32, output)?;
            bytes += emit_byte(*laneidx, output)?;
        },
        VectorInstruction::I8X16Eq => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(35u32, output)?;
        },
        VectorInstruction::I8X16Ne => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(36u32, output)?;
        },
        VectorInstruction::I8X16Lt(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(37u32, output)?;
        },
        VectorInstruction::I8X16Lt(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(38u32, output)?;
        },
        VectorInstruction::I8X16Gt(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(39u32, output)?;
        },
        VectorInstruction::I8X16Gt(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(40u32, output)?;
        },
        VectorInstruction::I8X16Le(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(41u32, output)?;
        },
        VectorInstruction::I8X16Le(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(42u32, output)?;
        },
        VectorInstruction::I8X16Ge(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(43u32, output)?;
        },
        VectorInstruction::I8X16Ge(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(44u32, output)?;
        },
        VectorInstruction::I16X8Eq => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(45u32, output)?;
        },
        VectorInstruction::I16X8Ne => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(46u32, output)?;
        },
        VectorInstruction::I16X8Lt(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(47u32, output)?;
        },
        VectorInstruction::I16X8Lt(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(48u32, output)?;
        },
        VectorInstruction::I16X8Gt(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(49u32, output)?;
        },
        VectorInstruction::I16X8Gt(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(50u32, output)?;
        },
        VectorInstruction::I16X8Le(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(51u32, output)?;
        },
        VectorInstruction::I16X8Le(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(52u32, output)?;
        },
        VectorInstruction::I16X8Ge(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(53u32, output)?;
        },
        VectorInstruction::I16X8Ge(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(54u32, output)?;
        },
        VectorInstruction::I32X4Eq => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(55u32, output)?;
        },
        VectorInstruction::I32X4Ne => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(56u32, output)?;
        },
        VectorInstruction::I32X4Lt(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(57u32, output)?;
        },
        VectorInstruction::I32X4Lt(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(58u32, output)?;
        },
        VectorInstruction::I32X4Gt(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(59u32, output)?;
        },
        VectorInstruction::I32X4Gt(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(60u32, output)?;
        },
        VectorInstruction::I32X4Le(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(61u32, output)?;
        },
        VectorInstruction::I32X4Le(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(62u32, output)?;
        },
        VectorInstruction::I32X4Ge(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(63u32, output)?;
        },
        VectorInstruction::I32X4Ge(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(64u32, output)?;
        },
        VectorInstruction::I64X2Eq => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(214u32, output)?;
        },
        VectorInstruction::I64X2Ne => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(215u32, output)?;
        },
        VectorInstruction::I64X2Lt => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(216u32, output)?;
        },
        VectorInstruction::I64X2Gt => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(217u32, output)?;
        },
        VectorInstruction::I64X2Le => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(218u32, output)?;
        },
        VectorInstruction::I64X2Ge => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(219u32, output)?;
        },
        VectorInstruction::FEq(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(65u32, output)?;
        },
        VectorInstruction::FNe(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(66u32, output)?;
        },
        VectorInstruction::FLt(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(67u32, output)?;
        },
        VectorInstruction::FGt(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(68u32, output)?;
        },
        VectorInstruction::FLe(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(69u32, output)?;
        },
        VectorInstruction::FGe(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(70u32, output)?;
        },
        VectorInstruction::FEq(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(71u32, output)?;
        },
        VectorInstruction::FNe(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(72u32, output)?;
        },
        VectorInstruction::FLt(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(73u32, output)?;
        },
        VectorInstruction::FGt(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(74u32, output)?;
        },
        VectorInstruction::FLe(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(75u32, output)?;
        },
        VectorInstruction::FGe(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(76u32, output)?;
        },
        VectorInstruction::IAbs(IShape::I8X16) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(96u32, output)?;
        },
        VectorInstruction::INeg(IShape::I8X16) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(97u32, output)?;
        },
        VectorInstruction::I8X16Popcnt => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(98u32, output)?;
        },
        VectorInstruction::AllTrue(IShape::I8X16) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(99u32, output)?;
        },
        VectorInstruction::Bitmask(IShape::I8X16) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(100u32, output)?;
        },
        VectorInstruction::I8X16NarrowI16X8(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(101u32, output)?;
        },
        VectorInstruction::I8X16NarrowI16X8(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(102u32, output)?;
        },
        VectorInstruction::Shl(IShape::I8X16) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(107u32, output)?;
        },
        VectorInstruction::Shr(IShape::I8X16, SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(108u32, output)?;
        },
        VectorInstruction::Shr(IShape::I8X16, SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(109u32, output)?;
        },
        VectorInstruction::IAdd(IShape::I8X16) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(110u32, output)?;
        },
        VectorInstruction::I8X16AddSat(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(111u32, output)?;
        },
        VectorInstruction::I8X16AddSat(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(112u32, output)?;
        },
        VectorInstruction::ISub(IShape::I8X16) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(113u32, output)?;
        },
        VectorInstruction::I8X16SubSat(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(114u32, output)?;
        },
        VectorInstruction::I8X16SubSat(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(115u32, output)?;
        },
        VectorInstruction::I8X16Min(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(118u32, output)?;
        },
        VectorInstruction::I8X16Min(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(119u32, output)?;
        },
        VectorInstruction::I8X16Max(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(120u32, output)?;
        },
        VectorInstruction::I8X16Max(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(121u32, output)?;
        },
        VectorInstruction::I8X16Avgr => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(123u32, output)?;
        },
        VectorInstruction::I16X8ExtaddPairwiseI8X16(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(124u32, output)?;
        },
        VectorInstruction::I16X8ExtaddPairwiseI8X16(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(125u32, output)?;
        },
        VectorInstruction::IAbs(IShape::I16X8) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(128u32, output)?;
        },
        VectorInstruction::INeg(IShape::I16X8) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(129u32, output)?;
        },
        VectorInstruction::I16X8Q15MulrSat => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(130u32, output)?;
        },
        VectorInstruction::AllTrue(IShape::I16X8) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(131u32, output)?;
        },
        VectorInstruction::Bitmask(IShape::I16X8) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(132u32, output)?;
        },
        VectorInstruction::I16X8NarrowI32X4(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(133u32, output)?;
        },
        VectorInstruction::I16X8NarrowI32X4(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(134u32, output)?;
        },
        VectorInstruction::I16X8ExtendLowI8X16(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(135u32, output)?;
        },
        VectorInstruction::I16X8ExtendHighI8X16(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(136u32, output)?;
        },
        VectorInstruction::I16X8ExtendLowI8X16(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(137u32, output)?;
        },
        VectorInstruction::I16X8ExtendHighI8X16(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(138u32, output)?;
        },
        VectorInstruction::Shl(IShape::I16X8) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(139u32, output)?;
        },
        VectorInstruction::Shr(IShape::I16X8, SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(140u32, output)?;
        },
        VectorInstruction::Shr(IShape::I16X8, SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(141u32, output)?;
        },
        VectorInstruction::IAdd(IShape::I16X8) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(142u32, output)?;
        },
        VectorInstruction::I16X8AddSat(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(143u32, output)?;
        },
        VectorInstruction::I16X8AddSat(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(144u32, output)?;
        },
        VectorInstruction::ISub(IShape::I16X8) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(145u32, output)?;
        },
        VectorInstruction::I16X8SubSat(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(146u32, output)?;
        },
        VectorInstruction::I16X8SubSat(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(147u32, output)?;
        },
        VectorInstruction::I16X8Mul => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(149u32, output)?;
        },
        VectorInstruction::I16X8Min(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(150u32, output)?;
        },
        VectorInstruction::I16X8Min(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(151u32, output)?;
        },
        VectorInstruction::I16X8Max(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(152u32, output)?;
        },
        VectorInstruction::I16X8Max(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(153u32, output)?;
        },
        VectorInstruction::I16X8Avgr => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(155u32, output)?;
        },
        VectorInstruction::I16X8ExtmulLowI8X16(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(156u32, output)?;
        },
        VectorInstruction::I16X8ExtmulHighI8X16(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(157u32, output)?;
        },
        VectorInstruction::I16X8ExtmulLowI8X16(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(158u32, output)?;
        },
        VectorInstruction::I16X8ExtmulHighI8X16(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(159u32, output)?;
        },
        VectorInstruction::I32X4ExtaddPairwiseI16X8(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(126u32, output)?;
        },
        VectorInstruction::I32X4ExtaddPairwiseI16X8(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(127u32, output)?;
        },
        VectorInstruction::IAbs(IShape::I32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(160u32, output)?;
        },
        VectorInstruction::INeg(IShape::I32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(161u32, output)?;
        },
        VectorInstruction::AllTrue(IShape::I32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(163u32, output)?;
        },
        VectorInstruction::Bitmask(IShape::I32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(164u32, output)?;
        },
        VectorInstruction::I32X4ExtendLowI16X8(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(167u32, output)?;
        },
        VectorInstruction::I32X4ExtendHighI16X8(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(168u32, output)?;
        },
        VectorInstruction::I32X4ExtendLowI16X8(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(169u32, output)?;
        },
        VectorInstruction::I32X4ExtendHighI16X8(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(170u32, output)?;
        },
        VectorInstruction::Shl(IShape::I32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(171u32, output)?;
        },
        VectorInstruction::Shr(IShape::I32X4, SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(172u32, output)?;
        },
        VectorInstruction::Shr(IShape::I32X4, SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(173u32, output)?;
        },
        VectorInstruction::IAdd(IShape::I32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(174u32, output)?;
        },
        VectorInstruction::ISub(IShape::I32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(177u32, output)?;
        },
        VectorInstruction::I32X4Mul => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(181u32, output)?;
        },
        VectorInstruction::I32X4Min(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(182u32, output)?;
        },
        VectorInstruction::I32X4Min(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(183u32, output)?;
        },
        VectorInstruction::I32X4Max(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(184u32, output)?;
        },
        VectorInstruction::I32X4Max(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(185u32, output)?;
        },
        VectorInstruction::I32X4DotI16X8 => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(186u32, output)?;
        },
        VectorInstruction::I32X4ExtmulLowI16X8(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(188u32, output)?;
        },
        VectorInstruction::I32X4ExtmulHighI16X8(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(189u32, output)?;
        },
        VectorInstruction::I32X4ExtmulLowI16X8(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(190u32, output)?;
        },
        VectorInstruction::I32X4ExtmulHighI16X8(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(191u32, output)?;
        },
        VectorInstruction::IAbs(IShape::I64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(192u32, output)?;
        },
        VectorInstruction::INeg(IShape::I64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(193u32, output)?;
        },
        VectorInstruction::AllTrue(IShape::I64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(195u32, output)?;
        },
        VectorInstruction::Bitmask(IShape::I64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(196u32, output)?;
        },
        VectorInstruction::I64X2ExtendLowI32X4(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(199u32, output)?;
        },
        VectorInstruction::I64X2ExtendLowI32X4(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(200u32, output)?;
        },
        VectorInstruction::I64X2ExtendHighI32X4(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(201u32, output)?;
        },
        VectorInstruction::I64X2ExtendHighI32X4(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(202u32, output)?;
        },
        VectorInstruction::Shl(IShape::I64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(203u32, output)?;
        },
        VectorInstruction::Shr(IShape::I64X2, SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(204u32, output)?;
        },
        VectorInstruction::Shr(IShape::I64X2, SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(205u32, output)?;
        },
        VectorInstruction::IAdd(IShape::I64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(206u32, output)?;
        },
        VectorInstruction::ISub(IShape::I64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(209u32, output)?;
        },
        VectorInstruction::I64X2Mul => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(213u32, output)?;
        },
        VectorInstruction::I64X2ExtmulLowI32X4(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(220u32, output)?;
        },
        VectorInstruction::I64X2ExtmulHighI32X4(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(221u32, output)?;
        },
        VectorInstruction::I64X2ExtmulLowI32X4(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(222u32, output)?;
        },
        VectorInstruction::I64X2ExtmulHighI32X4(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(223u32, output)?;
        },
        VectorInstruction::Ceil(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(103u32, output)?;
        },
        VectorInstruction::Floor(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(104u32, output)?;
        },
        VectorInstruction::Trunc(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(105u32, output)?;
        },
        VectorInstruction::Nearest(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(106u32, output)?;
        },
        VectorInstruction::FAbs(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(224u32, output)?;
        },
        VectorInstruction::FNeg(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(225u32, output)?;
        },
        VectorInstruction::Sqrt(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(227u32, output)?;
        },
        VectorInstruction::FAdd(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(228u32, output)?;
        },
        VectorInstruction::FSub(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(229u32, output)?;
        },
        VectorInstruction::FMul(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(230u32, output)?;
        },
        VectorInstruction::FDiv(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(231u32, output)?;
        },
        VectorInstruction::FMin(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(232u32, output)?;
        },
        VectorInstruction::FMax(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(233u32, output)?;
        },
        VectorInstruction::Pmin(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(234u32, output)?;
        },
        VectorInstruction::Pmax(FShape::F32X4) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(235u32, output)?;
        },
        VectorInstruction::Ceil(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(116u32, output)?;
        },
        VectorInstruction::Floor(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(117u32, output)?;
        },
        VectorInstruction::Trunc(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(122u32, output)?;
        },
        VectorInstruction::Nearest(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(148u32, output)?;
        },
        VectorInstruction::FAbs(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(236u32, output)?;
        },
        VectorInstruction::FNeg(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(237u32, output)?;
        },
        VectorInstruction::Sqrt(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(239u32, output)?;
        },
        VectorInstruction::FAdd(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(240u32, output)?;
        },
        VectorInstruction::FSub(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(241u32, output)?;
        },
        VectorInstruction::FMul(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(242u32, output)?;
        },
        VectorInstruction::FDiv(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(243u32, output)?;
        },
        VectorInstruction::FMin(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(244u32, output)?;
        },
        VectorInstruction::FMax(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(245u32, output)?;
        },
        VectorInstruction::Pmin(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(246u32, output)?;
        },
        VectorInstruction::Pmax(FShape::F64X2) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(247u32, output)?;
        },
        VectorInstruction::I32X4TruncSatF32X4(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(248u32, output)?;
        },
        VectorInstruction::I32X4TruncSatF32X4(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(249u32, output)?;
        },
        VectorInstruction::F32X4ConvertI32X4(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(250u32, output)?;
        },
        VectorInstruction::F32X4ConvertI32X4(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(251u32, output)?;
        },
        VectorInstruction::I32X4TruncSatF64X2Zero(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(252u32, output)?;
        },
        VectorInstruction::I32X4TruncSatF64X2Zero(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(253u32, output)?;
        },
        VectorInstruction::F64X2ConvertLowI32X4(SignExtension::Signed) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(254u32, output)?;
        },
        VectorInstruction::F64X2ConvertLowI32X4(SignExtension::Unsigned) => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(255u32, output)?;
        },
        VectorInstruction::F32X4DemoteF64X2Zero => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(94u32, output)?;
        },
        VectorInstruction::F64X2PromoteLowF32X4 => {
            bytes += emit_byte(0xFDu8, output)?;
            bytes += emit_u32(95u32, output)?;
        },
    }

    Ok(bytes)
}