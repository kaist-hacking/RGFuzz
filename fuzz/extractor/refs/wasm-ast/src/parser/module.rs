use crate::parser::instructions::parse_expression;
use crate::parser::types::{
    parse_global_type, parse_memory_type, parse_reference_type, parse_table_type, parse_value_type,
};
use crate::parser::values::{match_byte, parse_byte_vector, parse_name, parse_u32, parse_vector};
use crate::{
    Data, Element, ElementInitializer, Export, ExportDescription, Expression, Global, Import,
    ImportDescription, Memory, ReferenceType, ResultType, Start, Table,
};
use nom::branch::alt;
use nom::bytes::complete::take;
use nom::combinator::{all_consuming, map};
use nom::multi::fold_many_m_n;
use nom::sequence::{preceded, tuple};
use nom::IResult;

/// Parses a WebAssembly import component from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#binary-importsec>
pub fn parse_import(input: &[u8]) -> IResult<&[u8], Import> {
    map(
        tuple((parse_name, parse_name, parse_import_description)),
        |(module, import, description)| Import::new(module, import, description),
    )(input)
}

/// Parses an import description.
fn parse_import_description(input: &[u8]) -> IResult<&[u8], ImportDescription> {
    alt((
        map(
            preceded(match_byte(0x00), parse_u32),
            ImportDescription::Function,
        ),
        map(
            preceded(match_byte(0x01), parse_table_type),
            ImportDescription::Table,
        ),
        map(
            preceded(match_byte(0x02), parse_memory_type),
            ImportDescription::Memory,
        ),
        map(
            preceded(match_byte(0x03), parse_global_type),
            ImportDescription::Global,
        ),
    ))(input)
}

/// Parses a WebAssembly table component from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#memory-section>
pub fn parse_table(input: &[u8]) -> IResult<&[u8], Table> {
    map(parse_table_type, Table::from)(input)
}

/// Parses a WebAssembly memory component from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#memory-section>
pub fn parse_memory(input: &[u8]) -> IResult<&[u8], Memory> {
    map(parse_memory_type, Memory::from)(input)
}

/// Parses a WebAssembly global component from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#global-section>
pub fn parse_global(input: &[u8]) -> IResult<&[u8], Global> {
    map(
        tuple((parse_global_type, parse_expression)),
        |(kind, initializer)| Global::new(kind, initializer),
    )(input)
}

/// Parses a WebAssembly data component from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#data-section>
pub fn parse_data(input: &[u8]) -> IResult<&[u8], Data> {
    alt((
        map(
            preceded(
                match_byte(0x00),
                tuple((parse_expression, parse_byte_vector)),
            ),
            |(offset, bytes)| Data::active(0, offset, bytes.into()),
        ),
        map(preceded(match_byte(0x01), parse_byte_vector), |bytes| {
            Data::passive(bytes.into())
        }),
        map(
            preceded(
                match_byte(0x02),
                tuple((parse_u32, parse_expression, parse_byte_vector)),
            ),
            |(memory, offset, bytes)| Data::active(memory, offset, bytes.into()),
        ),
    ))(input)
}

/// Parses a WebAssembly start component from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#start-section>
pub fn parse_start(input: &[u8]) -> IResult<&[u8], Start> {
    map(parse_u32, Start::new)(input)
}

/// Parses a WebAssembly export component from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#export-section>
pub fn parse_export(input: &[u8]) -> IResult<&[u8], Export> {
    map(
        tuple((parse_name, parse_export_description)),
        |(export, description)| Export::new(export, description),
    )(input)
}

/// Parses an export description.
fn parse_export_description(input: &[u8]) -> IResult<&[u8], ExportDescription> {
    alt((
        map(
            preceded(match_byte(0x00), parse_u32),
            ExportDescription::Function,
        ),
        map(
            preceded(match_byte(0x01), parse_u32),
            ExportDescription::Table,
        ),
        map(
            preceded(match_byte(0x02), parse_u32),
            ExportDescription::Memory,
        ),
        map(
            preceded(match_byte(0x03), parse_u32),
            ExportDescription::Global,
        ),
    ))(input)
}

/// Parses a WebAssembly element component from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#element-section>
pub fn parse_element(input: &[u8]) -> IResult<&[u8], Element> {
    alt((
        map(
            preceded(
                match_byte(0x00),
                tuple((parse_expression, parse_vector(parse_u32))),
            ),
            |(offset, functions)| {
                Element::active(
                    0,
                    offset,
                    ReferenceType::Function,
                    functions.to_initializers(),
                )
            },
        ),
        map(
            preceded(
                match_byte(0x01),
                preceded(match_byte(0x00), parse_vector(parse_u32)),
            ),
            |functions| Element::passive(ReferenceType::Function, functions.to_initializers()),
        ),
        map(
            preceded(
                match_byte(0x02),
                tuple((
                    parse_u32,
                    parse_expression,
                    preceded(match_byte(0x00), parse_vector(parse_u32)),
                )),
            ),
            |(table, offset, functions)| {
                Element::active(
                    table,
                    offset,
                    ReferenceType::Function,
                    functions.to_initializers(),
                )
            },
        ),
        map(
            preceded(
                match_byte(0x03),
                preceded(match_byte(0x00), parse_vector(parse_u32)),
            ),
            |functions| Element::declarative(ReferenceType::Function, functions.to_initializers()),
        ),
        map(
            preceded(
                match_byte(0x04),
                tuple((parse_expression, parse_vector(parse_expression))),
            ),
            |(offset, initializers)| {
                Element::active(0, offset, ReferenceType::Function, initializers)
            },
        ),
        map(
            preceded(
                match_byte(0x05),
                tuple((parse_reference_type, parse_vector(parse_expression))),
            ),
            |(kind, initializers)| Element::passive(kind, initializers),
        ),
        map(
            preceded(
                match_byte(0x06),
                tuple((
                    parse_u32,
                    parse_expression,
                    parse_reference_type,
                    parse_vector(parse_expression),
                )),
            ),
            |(table, offset, kind, initializers)| {
                Element::active(table, offset, kind, initializers)
            },
        ),
        map(
            preceded(
                match_byte(0x07),
                tuple((parse_reference_type, parse_vector(parse_expression))),
            ),
            |(kind, initializers)| Element::declarative(kind, initializers),
        ),
    ))(input)
}

/// Parses a WebAssembly code portion of a function component from the input.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#code-section>
pub fn parse_code(input: &[u8]) -> IResult<&[u8], (ResultType, Expression)> {
    let (input, size) = parse_u32(input)?;
    let (remaining, input) = take(size as usize)(input)?;
    let (_, code) = all_consuming(tuple((parse_locals, parse_expression)))(input)?;

    Ok((remaining, code))
}

/// Parses the value types of locals in a function.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#code-section>
pub fn parse_locals(input: &[u8]) -> IResult<&[u8], ResultType> {
    let (input, length) = parse_u32(input)?;
    let length = length as usize;
    let (remaining, value_types) = fold_many_m_n(
        length,
        length,
        tuple((parse_u32, parse_value_type)),
        Vec::new,
        |mut accumulator, (count, kind)| {
            accumulator.reserve(count as usize);
            accumulator.extend((0..count).map(|_| kind));
            accumulator
        },
    )(input)?;

    Ok((remaining, value_types.into()))
}
