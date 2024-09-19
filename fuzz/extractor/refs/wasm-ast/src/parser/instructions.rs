use crate::parser::types::{parse_reference_type, parse_value_type};
use crate::parser::values::{match_byte, parse_s32, parse_s33, parse_s64, parse_u32, parse_vector};
use crate::{
    BlockType, ControlInstruction, Expression, FloatType, Instruction, IntegerType, MemoryArgument,
    MemoryInstruction, NumberType, NumericInstruction, ParametricInstruction, ReferenceInstruction,
    SignExtension, TableInstruction, VariableInstruction,
};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::fold_many0;
use nom::number::complete::{le_f32, le_f64};
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::IResult;

/// Marks the end of an expression.
const EXPRESSION_END: u8 = 0x0B;

/// Parses a WebAssembly expression from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#expressions>
pub fn parse_expression(input: &[u8]) -> IResult<&[u8], Expression> {
    parse_expression_with_terminal(EXPRESSION_END)(input)
}

/// Parses a WebAssembly expression with a given terminating opcode from the input.
fn parse_expression_with_terminal<'input>(
    terminal: u8,
) -> impl FnMut(&'input [u8]) -> IResult<&'input [u8], Expression> {
    map(
        terminated(
            fold_many0(parse_instruction, Vec::new, |mut accumulator, item| {
                accumulator.push(item);
                accumulator
            }),
            match_byte(terminal),
        ),
        Expression::new,
    )
}

/// Parses a WebAssembly instruction from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html>
pub fn parse_instruction(input: &[u8]) -> IResult<&[u8], Instruction> {
    alt((
        map(parse_control_instruction, Instruction::from),
        map(parse_reference_instruction, Instruction::from),
        map(parse_parametric_instruction, Instruction::from),
        map(parse_variable_instruction, Instruction::from),
        map(parse_table_instruction, Instruction::from),
        map(parse_memory_instruction, Instruction::from),
        map(parse_numeric_instruction, Instruction::from),
    ))(input)
}

/// Parses a WebAssembly control instruction from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#control-instructions>
pub fn parse_control_instruction(input: &[u8]) -> IResult<&[u8], ControlInstruction> {
    alt((
        map(match_byte(0x00), |_| ControlInstruction::Unreachable),
        map(match_byte(0x01), |_| ControlInstruction::Nop),
        map(
            preceded(
                match_byte(0x02),
                tuple((parse_block_type, parse_expression)),
            ),
            |(kind, expression)| ControlInstruction::Block(kind, expression),
        ),
        map(
            preceded(
                match_byte(0x03),
                tuple((parse_block_type, parse_expression)),
            ),
            |(kind, expression)| ControlInstruction::Loop(kind, expression),
        ),
        map(
            preceded(
                match_byte(0x04),
                tuple((parse_block_type, parse_expression)),
            ),
            |(kind, expression)| ControlInstruction::If(kind, expression, None),
        ),
        map(
            preceded(
                match_byte(0x04),
                tuple((
                    parse_block_type,
                    parse_expression_with_terminal(0x05),
                    parse_expression,
                )),
            ),
            |(kind, true_expression, false_expression)| {
                ControlInstruction::If(kind, true_expression, Some(false_expression))
            },
        ),
        map(
            preceded(match_byte(0x0C), parse_u32),
            ControlInstruction::Branch,
        ),
        map(
            preceded(match_byte(0x0D), parse_u32),
            ControlInstruction::BranchIf,
        ),
        map(
            preceded(
                match_byte(0x0E),
                tuple((parse_vector(parse_u32), parse_u32)),
            ),
            |(head, last)| ControlInstruction::BranchTable(head, last),
        ),
        map(match_byte(0x0F), |_| ControlInstruction::Return),
        map(
            preceded(match_byte(0x10), parse_u32),
            ControlInstruction::Call,
        ),
        map(
            preceded(match_byte(0x11), tuple((parse_u32, parse_u32))),
            |(type_index, table_index)| ControlInstruction::CallIndirect(type_index, table_index),
        ),
    ))(input)
}

/// Parses a WebAssembly control instruction's block type from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#control-instructions>
pub fn parse_block_type(input: &[u8]) -> IResult<&[u8], BlockType> {
    alt((
        map(match_byte(0x40), |_| BlockType::None),
        map(parse_value_type, BlockType::ValueType),
        map(parse_s33, |index| BlockType::Index(index as u32)),
    ))(input)
}

/// Parses a WebAssembly reference instruction from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#reference-instructions>
pub fn parse_reference_instruction(input: &[u8]) -> IResult<&[u8], ReferenceInstruction> {
    alt((
        map(
            preceded(match_byte(0xD0), parse_reference_type),
            ReferenceInstruction::Null,
        ),
        map(match_byte(0xD1), |_| ReferenceInstruction::IsNull),
        map(
            preceded(match_byte(0xD2), parse_u32),
            ReferenceInstruction::Function,
        ),
    ))(input)
}

/// Parses a WebAssembly parametric instruction from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#parametric-instructions>
pub fn parse_parametric_instruction(input: &[u8]) -> IResult<&[u8], ParametricInstruction> {
    alt((
        map(match_byte(0x1A), |_| ParametricInstruction::Drop),
        map(match_byte(0x1B), |_| ParametricInstruction::Select(None)),
        map(
            preceded(match_byte(0x1C), parse_vector(parse_value_type)),
            |value_types| ParametricInstruction::Select(Some(value_types)),
        ),
    ))(input)
}

/// Parses a WebAssembly variable instruction from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#variable-instructions>
pub fn parse_variable_instruction(input: &[u8]) -> IResult<&[u8], VariableInstruction> {
    alt((
        map(
            preceded(match_byte(0x20), parse_u32),
            VariableInstruction::LocalGet,
        ),
        map(
            preceded(match_byte(0x21), parse_u32),
            VariableInstruction::LocalSet,
        ),
        map(
            preceded(match_byte(0x22), parse_u32),
            VariableInstruction::LocalTee,
        ),
        map(
            preceded(match_byte(0x23), parse_u32),
            VariableInstruction::GlobalGet,
        ),
        map(
            preceded(match_byte(0x24), parse_u32),
            VariableInstruction::GlobalSet,
        ),
    ))(input)
}

/// Parses a WebAssembly table instruction from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#table-instructions>
pub fn parse_table_instruction(input: &[u8]) -> IResult<&[u8], TableInstruction> {
    alt((
        map(preceded(match_byte(0x25), parse_u32), TableInstruction::Get),
        map(preceded(match_byte(0x26), parse_u32), TableInstruction::Set),
        map(
            preceded(tag([0xFC, 12u8]), tuple((parse_u32, parse_u32))),
            |(element, table)| TableInstruction::Init(element, table),
        ),
        map(
            preceded(tag([0xFC, 13u8]), parse_u32),
            TableInstruction::ElementDrop,
        ),
        map(
            preceded(tag([0xFC, 14u8]), tuple((parse_u32, parse_u32))),
            |(table_a, table_b)| TableInstruction::Copy(table_a, table_b),
        ),
        map(
            preceded(tag([0xFC, 15u8]), parse_u32),
            TableInstruction::Grow,
        ),
        map(
            preceded(tag([0xFC, 16u8]), parse_u32),
            TableInstruction::Size,
        ),
        map(
            preceded(tag([0xFC, 17u8]), parse_u32),
            TableInstruction::Fill,
        ),
    ))(input)
}

/// Parses a WebAssembly memory instruction from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#memory-instructions>
pub fn parse_memory_instruction(input: &[u8]) -> IResult<&[u8], MemoryInstruction> {
    alt((
        alt((
            map(
                preceded(match_byte(0x28), parse_memory_argument),
                |memarg| MemoryInstruction::Load(NumberType::I32, memarg),
            ),
            map(
                preceded(match_byte(0x29), parse_memory_argument),
                |memarg| MemoryInstruction::Load(NumberType::I64, memarg),
            ),
            map(
                preceded(match_byte(0x2A), parse_memory_argument),
                |memarg| MemoryInstruction::Load(NumberType::F32, memarg),
            ),
            map(
                preceded(match_byte(0x2B), parse_memory_argument),
                |memarg| MemoryInstruction::Load(NumberType::F64, memarg),
            ),
            map(
                preceded(match_byte(0x2C), parse_memory_argument),
                |memarg| MemoryInstruction::Load8(IntegerType::I32, SignExtension::Signed, memarg),
            ),
            map(
                preceded(match_byte(0x2D), parse_memory_argument),
                |memarg| {
                    MemoryInstruction::Load8(IntegerType::I32, SignExtension::Unsigned, memarg)
                },
            ),
            map(
                preceded(match_byte(0x2E), parse_memory_argument),
                |memarg| MemoryInstruction::Load16(IntegerType::I32, SignExtension::Signed, memarg),
            ),
            map(
                preceded(match_byte(0x2F), parse_memory_argument),
                |memarg| {
                    MemoryInstruction::Load16(IntegerType::I32, SignExtension::Unsigned, memarg)
                },
            ),
            map(
                preceded(match_byte(0x30), parse_memory_argument),
                |memarg| MemoryInstruction::Load8(IntegerType::I64, SignExtension::Signed, memarg),
            ),
            map(
                preceded(match_byte(0x31), parse_memory_argument),
                |memarg| {
                    MemoryInstruction::Load8(IntegerType::I64, SignExtension::Unsigned, memarg)
                },
            ),
            map(
                preceded(match_byte(0x32), parse_memory_argument),
                |memarg| MemoryInstruction::Load16(IntegerType::I64, SignExtension::Signed, memarg),
            ),
            map(
                preceded(match_byte(0x33), parse_memory_argument),
                |memarg| {
                    MemoryInstruction::Load16(IntegerType::I64, SignExtension::Unsigned, memarg)
                },
            ),
            map(
                preceded(match_byte(0x34), parse_memory_argument),
                |memarg| MemoryInstruction::Load32(SignExtension::Signed, memarg),
            ),
            map(
                preceded(match_byte(0x35), parse_memory_argument),
                |memarg| MemoryInstruction::Load32(SignExtension::Unsigned, memarg),
            ),
        )),
        alt((
            map(
                preceded(match_byte(0x36), parse_memory_argument),
                |memarg| MemoryInstruction::Store(NumberType::I32, memarg),
            ),
            map(
                preceded(match_byte(0x37), parse_memory_argument),
                |memarg| MemoryInstruction::Store(NumberType::I64, memarg),
            ),
            map(
                preceded(match_byte(0x38), parse_memory_argument),
                |memarg| MemoryInstruction::Store(NumberType::F32, memarg),
            ),
            map(
                preceded(match_byte(0x39), parse_memory_argument),
                |memarg| MemoryInstruction::Store(NumberType::F64, memarg),
            ),
            map(
                preceded(match_byte(0x3A), parse_memory_argument),
                |memarg| MemoryInstruction::Store8(IntegerType::I32, memarg),
            ),
            map(
                preceded(match_byte(0x3B), parse_memory_argument),
                |memarg| MemoryInstruction::Store16(IntegerType::I32, memarg),
            ),
            map(
                preceded(match_byte(0x3C), parse_memory_argument),
                |memarg| MemoryInstruction::Store8(IntegerType::I64, memarg),
            ),
            map(
                preceded(match_byte(0x3D), parse_memory_argument),
                |memarg| MemoryInstruction::Store16(IntegerType::I64, memarg),
            ),
            map(
                preceded(match_byte(0x3E), parse_memory_argument),
                MemoryInstruction::Store32,
            ),
        )),
        map(tag([0x3F, 0x00]), |_| MemoryInstruction::Size),
        map(tag([0x40, 0x00]), |_| MemoryInstruction::Grow),
        map(
            delimited(tag([0xFC, 8u8]), parse_u32, match_byte(0x00)),
            MemoryInstruction::Init,
        ),
        map(
            preceded(tag([0xFC, 9u8]), parse_u32),
            MemoryInstruction::DataDrop,
        ),
        map(tag([0xFC, 10u8, 0x00, 0x00]), |_| MemoryInstruction::Copy),
        map(tag([0xFC, 11u8, 0x00]), |_| MemoryInstruction::Fill),
    ))(input)
}

/// Parses a WebAssembly memory instruction memarg from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#memory-instructions>
pub fn parse_memory_argument(input: &[u8]) -> IResult<&[u8], MemoryArgument> {
    map(tuple((parse_u32, parse_u32)), |(align, offset)| {
        MemoryArgument::new(align, offset)
    })(input)
}

/// Parses a WebAssembly numeric instruction from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/instructions.html#numeric-instructions>
pub fn parse_numeric_instruction(input: &[u8]) -> IResult<&[u8], NumericInstruction> {
    alt((
        alt((
            map(
                preceded(match_byte(0x41), parse_s32),
                NumericInstruction::I32Constant,
            ),
            map(
                preceded(match_byte(0x42), parse_s64),
                NumericInstruction::I64Constant,
            ),
            map(
                preceded(match_byte(0x43), le_f32),
                NumericInstruction::F32Constant,
            ),
            map(
                preceded(match_byte(0x44), le_f64),
                NumericInstruction::F64Constant,
            ),
        )),
        alt((
            map(match_byte(0x45), |_| {
                NumericInstruction::EqualToZero(IntegerType::I32)
            }),
            map(match_byte(0x46), |_| {
                NumericInstruction::Equal(NumberType::I32)
            }),
            map(match_byte(0x47), |_| {
                NumericInstruction::NotEqual(NumberType::I32)
            }),
            map(match_byte(0x48), |_| {
                NumericInstruction::LessThanInteger(IntegerType::I32, SignExtension::Signed)
            }),
            map(match_byte(0x49), |_| {
                NumericInstruction::LessThanInteger(IntegerType::I32, SignExtension::Unsigned)
            }),
            map(match_byte(0x4A), |_| {
                NumericInstruction::GreaterThanInteger(IntegerType::I32, SignExtension::Signed)
            }),
            map(match_byte(0x4B), |_| {
                NumericInstruction::GreaterThanInteger(IntegerType::I32, SignExtension::Unsigned)
            }),
            map(match_byte(0x4C), |_| {
                NumericInstruction::LessThanOrEqualToInteger(
                    IntegerType::I32,
                    SignExtension::Signed,
                )
            }),
            map(match_byte(0x4D), |_| {
                NumericInstruction::LessThanOrEqualToInteger(
                    IntegerType::I32,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0x4E), |_| {
                NumericInstruction::GreaterThanOrEqualToInteger(
                    IntegerType::I32,
                    SignExtension::Signed,
                )
            }),
            map(match_byte(0x4F), |_| {
                NumericInstruction::GreaterThanOrEqualToInteger(
                    IntegerType::I32,
                    SignExtension::Unsigned,
                )
            }),
        )),
        alt((
            map(match_byte(0x50), |_| {
                NumericInstruction::EqualToZero(IntegerType::I64)
            }),
            map(match_byte(0x51), |_| {
                NumericInstruction::Equal(NumberType::I64)
            }),
            map(match_byte(0x52), |_| {
                NumericInstruction::NotEqual(NumberType::I64)
            }),
            map(match_byte(0x53), |_| {
                NumericInstruction::LessThanInteger(IntegerType::I64, SignExtension::Signed)
            }),
            map(match_byte(0x54), |_| {
                NumericInstruction::LessThanInteger(IntegerType::I64, SignExtension::Unsigned)
            }),
            map(match_byte(0x55), |_| {
                NumericInstruction::GreaterThanInteger(IntegerType::I64, SignExtension::Signed)
            }),
            map(match_byte(0x56), |_| {
                NumericInstruction::GreaterThanInteger(IntegerType::I64, SignExtension::Unsigned)
            }),
            map(match_byte(0x57), |_| {
                NumericInstruction::LessThanOrEqualToInteger(
                    IntegerType::I64,
                    SignExtension::Signed,
                )
            }),
            map(match_byte(0x58), |_| {
                NumericInstruction::LessThanOrEqualToInteger(
                    IntegerType::I64,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0x59), |_| {
                NumericInstruction::GreaterThanOrEqualToInteger(
                    IntegerType::I64,
                    SignExtension::Signed,
                )
            }),
            map(match_byte(0x5A), |_| {
                NumericInstruction::GreaterThanOrEqualToInteger(
                    IntegerType::I64,
                    SignExtension::Unsigned,
                )
            }),
        )),
        alt((
            map(match_byte(0x5B), |_| {
                NumericInstruction::Equal(NumberType::F32)
            }),
            map(match_byte(0x5C), |_| {
                NumericInstruction::NotEqual(NumberType::F32)
            }),
            map(match_byte(0x5D), |_| {
                NumericInstruction::LessThanFloat(FloatType::F32)
            }),
            map(match_byte(0x5E), |_| {
                NumericInstruction::GreaterThanFloat(FloatType::F32)
            }),
            map(match_byte(0x5F), |_| {
                NumericInstruction::LessThanOrEqualToFloat(FloatType::F32)
            }),
            map(match_byte(0x60), |_| {
                NumericInstruction::GreaterThanOrEqualToFloat(FloatType::F32)
            }),
        )),
        alt((
            map(match_byte(0x61), |_| {
                NumericInstruction::Equal(NumberType::F64)
            }),
            map(match_byte(0x62), |_| {
                NumericInstruction::NotEqual(NumberType::F64)
            }),
            map(match_byte(0x63), |_| {
                NumericInstruction::LessThanFloat(FloatType::F64)
            }),
            map(match_byte(0x64), |_| {
                NumericInstruction::GreaterThanFloat(FloatType::F64)
            }),
            map(match_byte(0x65), |_| {
                NumericInstruction::LessThanOrEqualToFloat(FloatType::F64)
            }),
            map(match_byte(0x66), |_| {
                NumericInstruction::GreaterThanOrEqualToFloat(FloatType::F64)
            }),
        )),
        alt((
            map(match_byte(0x67), |_| {
                NumericInstruction::CountLeadingZeros(IntegerType::I32)
            }),
            map(match_byte(0x68), |_| {
                NumericInstruction::CountTrailingZeros(IntegerType::I32)
            }),
            map(match_byte(0x69), |_| {
                NumericInstruction::CountOnes(IntegerType::I32)
            }),
            map(match_byte(0x6A), |_| {
                NumericInstruction::Add(NumberType::I32)
            }),
            map(match_byte(0x6B), |_| {
                NumericInstruction::Subtract(NumberType::I32)
            }),
            map(match_byte(0x6C), |_| {
                NumericInstruction::Multiply(NumberType::I32)
            }),
            map(match_byte(0x6D), |_| {
                NumericInstruction::DivideInteger(IntegerType::I32, SignExtension::Signed)
            }),
            map(match_byte(0x6E), |_| {
                NumericInstruction::DivideInteger(IntegerType::I32, SignExtension::Unsigned)
            }),
            map(match_byte(0x6F), |_| {
                NumericInstruction::Remainder(IntegerType::I32, SignExtension::Signed)
            }),
            map(match_byte(0x70), |_| {
                NumericInstruction::Remainder(IntegerType::I32, SignExtension::Unsigned)
            }),
            map(match_byte(0x71), |_| {
                NumericInstruction::And(IntegerType::I32)
            }),
            map(match_byte(0x72), |_| {
                NumericInstruction::Or(IntegerType::I32)
            }),
            map(match_byte(0x73), |_| {
                NumericInstruction::Xor(IntegerType::I32)
            }),
            map(match_byte(0x74), |_| {
                NumericInstruction::ShiftLeft(IntegerType::I32)
            }),
            map(match_byte(0x75), |_| {
                NumericInstruction::ShiftRight(IntegerType::I32, SignExtension::Signed)
            }),
            map(match_byte(0x76), |_| {
                NumericInstruction::ShiftRight(IntegerType::I32, SignExtension::Unsigned)
            }),
            map(match_byte(0x77), |_| {
                NumericInstruction::RotateLeft(IntegerType::I32)
            }),
            map(match_byte(0x78), |_| {
                NumericInstruction::RotateRight(IntegerType::I32)
            }),
        )),
        alt((
            map(match_byte(0x79), |_| {
                NumericInstruction::CountLeadingZeros(IntegerType::I64)
            }),
            map(match_byte(0x7A), |_| {
                NumericInstruction::CountTrailingZeros(IntegerType::I64)
            }),
            map(match_byte(0x7B), |_| {
                NumericInstruction::CountOnes(IntegerType::I64)
            }),
            map(match_byte(0x7C), |_| {
                NumericInstruction::Add(NumberType::I64)
            }),
            map(match_byte(0x7D), |_| {
                NumericInstruction::Subtract(NumberType::I64)
            }),
            map(match_byte(0x7E), |_| {
                NumericInstruction::Multiply(NumberType::I64)
            }),
            map(match_byte(0x7F), |_| {
                NumericInstruction::DivideInteger(IntegerType::I64, SignExtension::Signed)
            }),
            map(match_byte(0x80), |_| {
                NumericInstruction::DivideInteger(IntegerType::I64, SignExtension::Unsigned)
            }),
            map(match_byte(0x81), |_| {
                NumericInstruction::Remainder(IntegerType::I64, SignExtension::Signed)
            }),
            map(match_byte(0x82), |_| {
                NumericInstruction::Remainder(IntegerType::I64, SignExtension::Unsigned)
            }),
            map(match_byte(0x83), |_| {
                NumericInstruction::And(IntegerType::I64)
            }),
            map(match_byte(0x84), |_| {
                NumericInstruction::Or(IntegerType::I64)
            }),
            map(match_byte(0x85), |_| {
                NumericInstruction::Xor(IntegerType::I64)
            }),
            map(match_byte(0x86), |_| {
                NumericInstruction::ShiftLeft(IntegerType::I64)
            }),
            map(match_byte(0x87), |_| {
                NumericInstruction::ShiftRight(IntegerType::I64, SignExtension::Signed)
            }),
            map(match_byte(0x88), |_| {
                NumericInstruction::ShiftRight(IntegerType::I64, SignExtension::Unsigned)
            }),
            map(match_byte(0x89), |_| {
                NumericInstruction::RotateLeft(IntegerType::I64)
            }),
            map(match_byte(0x8A), |_| {
                NumericInstruction::RotateRight(IntegerType::I64)
            }),
        )),
        alt((
            map(match_byte(0x8B), |_| {
                NumericInstruction::AbsoluteValue(FloatType::F32)
            }),
            map(match_byte(0x8C), |_| {
                NumericInstruction::Negate(FloatType::F32)
            }),
            map(match_byte(0x8D), |_| {
                NumericInstruction::Ceiling(FloatType::F32)
            }),
            map(match_byte(0x8E), |_| {
                NumericInstruction::Floor(FloatType::F32)
            }),
            map(match_byte(0x8F), |_| {
                NumericInstruction::Truncate(FloatType::F32)
            }),
            map(match_byte(0x90), |_| {
                NumericInstruction::Nearest(FloatType::F32)
            }),
            map(match_byte(0x91), |_| {
                NumericInstruction::SquareRoot(FloatType::F32)
            }),
            map(match_byte(0x92), |_| {
                NumericInstruction::Add(NumberType::F32)
            }),
            map(match_byte(0x93), |_| {
                NumericInstruction::Subtract(NumberType::F32)
            }),
            map(match_byte(0x94), |_| {
                NumericInstruction::Multiply(NumberType::F32)
            }),
            map(match_byte(0x95), |_| {
                NumericInstruction::DivideFloat(FloatType::F32)
            }),
            map(match_byte(0x96), |_| {
                NumericInstruction::Minimum(FloatType::F32)
            }),
            map(match_byte(0x97), |_| {
                NumericInstruction::Maximum(FloatType::F32)
            }),
            map(match_byte(0x98), |_| {
                NumericInstruction::CopySign(FloatType::F32)
            }),
        )),
        alt((
            map(match_byte(0x99), |_| {
                NumericInstruction::AbsoluteValue(FloatType::F64)
            }),
            map(match_byte(0x9A), |_| {
                NumericInstruction::Negate(FloatType::F64)
            }),
            map(match_byte(0x9B), |_| {
                NumericInstruction::Ceiling(FloatType::F64)
            }),
            map(match_byte(0x9C), |_| {
                NumericInstruction::Floor(FloatType::F64)
            }),
            map(match_byte(0x9D), |_| {
                NumericInstruction::Truncate(FloatType::F64)
            }),
            map(match_byte(0x9E), |_| {
                NumericInstruction::Nearest(FloatType::F64)
            }),
            map(match_byte(0x9F), |_| {
                NumericInstruction::SquareRoot(FloatType::F64)
            }),
            map(match_byte(0xA0), |_| {
                NumericInstruction::Add(NumberType::F64)
            }),
            map(match_byte(0xA1), |_| {
                NumericInstruction::Subtract(NumberType::F64)
            }),
            map(match_byte(0xA2), |_| {
                NumericInstruction::Multiply(NumberType::F64)
            }),
            map(match_byte(0xA3), |_| {
                NumericInstruction::DivideFloat(FloatType::F64)
            }),
            map(match_byte(0xA4), |_| {
                NumericInstruction::Minimum(FloatType::F64)
            }),
            map(match_byte(0xA5), |_| {
                NumericInstruction::Maximum(FloatType::F64)
            }),
            map(match_byte(0xA6), |_| {
                NumericInstruction::CopySign(FloatType::F64)
            }),
        )),
        alt((
            map(match_byte(0xA7), |_| NumericInstruction::Wrap),
            map(match_byte(0xA8), |_| {
                NumericInstruction::ConvertAndTruncate(
                    IntegerType::I32,
                    FloatType::F32,
                    SignExtension::Signed,
                )
            }),
            map(match_byte(0xA9), |_| {
                NumericInstruction::ConvertAndTruncate(
                    IntegerType::I32,
                    FloatType::F32,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0xAA), |_| {
                NumericInstruction::ConvertAndTruncate(
                    IntegerType::I32,
                    FloatType::F64,
                    SignExtension::Signed,
                )
            }),
            map(match_byte(0xAB), |_| {
                NumericInstruction::ConvertAndTruncate(
                    IntegerType::I32,
                    FloatType::F64,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0xAC), |_| {
                NumericInstruction::ExtendWithSignExtension(SignExtension::Signed)
            }),
            map(match_byte(0xAD), |_| {
                NumericInstruction::ExtendWithSignExtension(SignExtension::Unsigned)
            }),
        )),
        alt((
            map(match_byte(0xAE), |_| {
                NumericInstruction::ConvertAndTruncate(
                    IntegerType::I64,
                    FloatType::F32,
                    SignExtension::Signed,
                )
            }),
            map(match_byte(0xAF), |_| {
                NumericInstruction::ConvertAndTruncate(
                    IntegerType::I64,
                    FloatType::F32,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0xB0), |_| {
                NumericInstruction::ConvertAndTruncate(
                    IntegerType::I64,
                    FloatType::F64,
                    SignExtension::Signed,
                )
            }),
            map(match_byte(0xB1), |_| {
                NumericInstruction::ConvertAndTruncate(
                    IntegerType::I64,
                    FloatType::F64,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0xB2), |_| {
                NumericInstruction::Convert(FloatType::F32, IntegerType::I32, SignExtension::Signed)
            }),
            map(match_byte(0xB3), |_| {
                NumericInstruction::Convert(
                    FloatType::F32,
                    IntegerType::I32,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0xB4), |_| {
                NumericInstruction::Convert(FloatType::F32, IntegerType::I64, SignExtension::Signed)
            }),
            map(match_byte(0xB5), |_| {
                NumericInstruction::Convert(
                    FloatType::F32,
                    IntegerType::I64,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0xB6), |_| NumericInstruction::Demote),
            map(match_byte(0xB7), |_| {
                NumericInstruction::Convert(FloatType::F64, IntegerType::I32, SignExtension::Signed)
            }),
            map(match_byte(0xB8), |_| {
                NumericInstruction::Convert(
                    FloatType::F64,
                    IntegerType::I32,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0xB9), |_| {
                NumericInstruction::Convert(FloatType::F64, IntegerType::I64, SignExtension::Signed)
            }),
            map(match_byte(0xBA), |_| {
                NumericInstruction::Convert(
                    FloatType::F64,
                    IntegerType::I64,
                    SignExtension::Unsigned,
                )
            }),
            map(match_byte(0xBB), |_| NumericInstruction::Promote),
        )),
        alt((
            map(match_byte(0xBC), |_| {
                NumericInstruction::ReinterpretFloat(IntegerType::I32)
            }),
            map(match_byte(0xBD), |_| {
                NumericInstruction::ReinterpretFloat(IntegerType::I64)
            }),
            map(match_byte(0xBE), |_| {
                NumericInstruction::ReinterpretInteger(FloatType::F32)
            }),
            map(match_byte(0xBF), |_| {
                NumericInstruction::ReinterpretInteger(FloatType::F64)
            }),
        )),
        alt((
            map(match_byte(0xC0), |_| {
                NumericInstruction::ExtendSigned8(IntegerType::I32)
            }),
            map(match_byte(0xC1), |_| {
                NumericInstruction::ExtendSigned16(IntegerType::I32)
            }),
            map(match_byte(0xC2), |_| {
                NumericInstruction::ExtendSigned8(IntegerType::I64)
            }),
            map(match_byte(0xC3), |_| {
                NumericInstruction::ExtendSigned16(IntegerType::I64)
            }),
            map(match_byte(0xC4), |_| NumericInstruction::ExtendSigned32),
        )),
        alt((
            map(tag([0xFC, 0u8]), |_| {
                NumericInstruction::ConvertAndTruncateWithSaturation(
                    IntegerType::I32,
                    FloatType::F32,
                    SignExtension::Signed,
                )
            }),
            map(tag([0xFC, 1u8]), |_| {
                NumericInstruction::ConvertAndTruncateWithSaturation(
                    IntegerType::I32,
                    FloatType::F32,
                    SignExtension::Unsigned,
                )
            }),
            map(tag([0xFC, 2u8]), |_| {
                NumericInstruction::ConvertAndTruncateWithSaturation(
                    IntegerType::I32,
                    FloatType::F64,
                    SignExtension::Signed,
                )
            }),
            map(tag([0xFC, 3u8]), |_| {
                NumericInstruction::ConvertAndTruncateWithSaturation(
                    IntegerType::I32,
                    FloatType::F64,
                    SignExtension::Unsigned,
                )
            }),
            map(tag([0xFC, 4u8]), |_| {
                NumericInstruction::ConvertAndTruncateWithSaturation(
                    IntegerType::I64,
                    FloatType::F32,
                    SignExtension::Signed,
                )
            }),
            map(tag([0xFC, 5u8]), |_| {
                NumericInstruction::ConvertAndTruncateWithSaturation(
                    IntegerType::I64,
                    FloatType::F32,
                    SignExtension::Unsigned,
                )
            }),
            map(tag([0xFC, 6u8]), |_| {
                NumericInstruction::ConvertAndTruncateWithSaturation(
                    IntegerType::I64,
                    FloatType::F64,
                    SignExtension::Signed,
                )
            }),
            map(tag([0xFC, 7u8]), |_| {
                NumericInstruction::ConvertAndTruncateWithSaturation(
                    IntegerType::I64,
                    FloatType::F64,
                    SignExtension::Unsigned,
                )
            }),
        )),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_expression() {
        let extra = 3;
        let input = vec![EXPRESSION_END, extra];

        let (remaining, actual) = parse_expression(input.as_slice()).unwrap();

        assert_eq!(actual, Expression::empty());
        assert_eq!(remaining, &[extra]);
    }

    #[test]
    fn parse_expression_with_instruction() {
        let extra = 3;
        let byte = 0x01;
        let input = vec![byte, EXPRESSION_END, extra];

        let (remaining, actual) = parse_expression(input.as_slice()).unwrap();
        let expected = Expression::new(vec![ControlInstruction::Nop.into()]);

        assert_eq!(actual, expected);
        assert_eq!(remaining, &[extra]);
    }

    #[test]
    fn parse_invalid_expression() {
        let input = vec![3];
        let result = parse_expression(input.as_slice());

        assert!(result.is_err());
    }
}
