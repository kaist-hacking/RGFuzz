use crate::emitter::errors::EmitError;
use crate::emitter::values::{emit_byte, emit_u32, emit_vector};
use crate::model::{
    FunctionType, GlobalType, Limit, MemoryType, Mutability, ReferenceType, ResultType, TableType,
    ValueType,
};
use std::borrow::Borrow;
use std::io::Write;

pub fn emit_reference_type<O: Write + ?Sized>(
    kind: ReferenceType,
    output: &mut O,
) -> Result<usize, EmitError> {
    emit_value_type(ValueType::from(kind), output)
}

pub fn emit_value_type<T: Borrow<ValueType>, O: Write + ?Sized>(
    kind: T,
    output: &mut O,
) -> Result<usize, EmitError> {
    let value: u8 = match *kind.borrow() {
        ValueType::I32 => 0x7F,
        ValueType::I64 => 0x7E,
        ValueType::F32 => 0x7D,
        ValueType::F64 => 0x7C,
        ValueType::V128 => 0x7B,
        ValueType::FunctionReference => 0x70,
        ValueType::ExternalReference => 0x6F,
    };

    emit_byte(value, output)
}

pub fn emit_result_type<O: Write + ?Sized>(
    kind: &ResultType,
    output: &mut O,
) -> Result<usize, EmitError> {
    emit_vector(kind.kinds(), output, emit_value_type)
}

pub fn emit_function_type<O: Write + ?Sized>(
    kind: &FunctionType,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    bytes += emit_byte(0x60u8, output)?;
    bytes += emit_result_type(kind.parameters(), output)?;
    bytes += emit_result_type(kind.results(), output)?;

    Ok(bytes)
}

pub fn emit_limit<O: Write + ?Sized>(limits: &Limit, output: &mut O) -> Result<usize, EmitError> {
    let mut bytes = 0;

    match limits.max() {
        Some(max) => {
            bytes += emit_byte(0x01u8, output)?;
            bytes += emit_u32(limits.min(), output)?;
            bytes += emit_u32(max, output)?;
        }
        None => {
            bytes += emit_byte(0x00u8, output)?;
            bytes += emit_u32(limits.min(), output)?;
        }
    };

    Ok(bytes)
}

pub fn emit_memory_type<O: Write + ?Sized>(
    kind: &MemoryType,
    output: &mut O,
) -> Result<usize, EmitError> {
    emit_limit(kind.limits(), output)
}

pub fn emit_table_type<O: Write + ?Sized>(
    kind: &TableType,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    bytes += emit_reference_type(kind.kind(), output)?;
    bytes += emit_limit(kind.limits(), output)?;

    Ok(bytes)
}

pub fn emit_global_type<O: Write + ?Sized>(
    kind: &GlobalType,
    output: &mut O,
) -> Result<usize, EmitError> {
    let mut bytes = 0;

    bytes += emit_value_type(kind.kind(), output)?;

    let mutability: u8 = match kind.mutability() {
        Mutability::Immutable => 0x00,
        Mutability::Mutable => 0x01,
    };

    bytes += emit_byte(mutability, output)?;

    Ok(bytes)
}
