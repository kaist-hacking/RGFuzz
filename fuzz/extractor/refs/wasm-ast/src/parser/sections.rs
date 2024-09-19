use crate::parser::module::{
    parse_code, parse_data, parse_element, parse_export, parse_global, parse_import, parse_memory,
    parse_start, parse_table,
};
use crate::parser::types::parse_function_type;
use crate::parser::values::{match_byte, parse_name, parse_u32, parse_vector};
use crate::{
    Custom, Data, Element, Export, Expression, FunctionType, Global, Import, Memory, ModuleSection,
    ResultType, Start, Table, TypeIndex,
};
use nom::bytes::complete::take;
use nom::combinator::{all_consuming, map, map_parser, opt, rest};
use nom::multi::fold_many1;
use nom::sequence::tuple;
use nom::{IResult, Parser};

/// Parses a WebAssembly custom section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#binary-customsec>
pub fn parse_custom_section(input: &[u8]) -> IResult<&[u8], Option<Vec<Custom>>> {
    opt(fold_many1(
        parse_section(ModuleSection::Custom, parse_custom_content),
        Vec::new,
        |mut accumulator, item| {
            accumulator.push(item);
            accumulator
        },
    ))(input)
}

/// Parses the custom content (name and bytes) of a custom section.
fn parse_custom_content(input: &[u8]) -> IResult<&[u8], Custom> {
    map(tuple((parse_name, rest)), |(name, contents)| {
        Custom::new(name, Vec::from(contents))
    })(input)
}

/// Parses a WebAssembly type section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#binary-typesec>
pub fn parse_type_section(input: &[u8]) -> IResult<&[u8], Option<Vec<FunctionType>>> {
    opt(parse_section(
        ModuleSection::Type,
        parse_vector(parse_function_type),
    ))(input)
}

/// Parses a WebAssembly import section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#binary-importsec>
pub fn parse_import_section(input: &[u8]) -> IResult<&[u8], Option<Vec<Import>>> {
    opt(parse_section(
        ModuleSection::Import,
        parse_vector(parse_import),
    ))(input)
}

/// Parses a WebAssembly function section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#function-section>
pub fn parse_function_section(input: &[u8]) -> IResult<&[u8], Option<Vec<TypeIndex>>> {
    opt(parse_section(
        ModuleSection::Function,
        parse_vector(parse_u32),
    ))(input)
}

/// Parses a WebAssembly table section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#table-section>
pub fn parse_table_section(input: &[u8]) -> IResult<&[u8], Option<Vec<Table>>> {
    opt(parse_section(
        ModuleSection::Table,
        parse_vector(parse_table),
    ))(input)
}

/// Parses a WebAssembly memory section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#memory-section>
pub fn parse_memory_section(input: &[u8]) -> IResult<&[u8], Option<Vec<Memory>>> {
    opt(parse_section(
        ModuleSection::Memory,
        parse_vector(parse_memory),
    ))(input)
}

/// Parses a WebAssembly global section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#global-section>
pub fn parse_global_section(input: &[u8]) -> IResult<&[u8], Option<Vec<Global>>> {
    opt(parse_section(
        ModuleSection::Global,
        parse_vector(parse_global),
    ))(input)
}

/// Parses a WebAssembly data count section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#data-count-section>
pub fn parse_data_count_section(input: &[u8]) -> IResult<&[u8], Option<u32>> {
    opt(parse_section(ModuleSection::DataCount, parse_u32))(input)
}

/// Parses a WebAssembly data section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#data-section>
pub fn parse_data_section(input: &[u8]) -> IResult<&[u8], Option<Vec<Data>>> {
    opt(parse_section(ModuleSection::Data, parse_vector(parse_data)))(input)
}

/// Parses a WebAssembly start section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#start-section>
pub fn parse_start_section(input: &[u8]) -> IResult<&[u8], Option<Start>> {
    opt(parse_section(ModuleSection::Start, parse_start))(input)
}

/// Parses a WebAssembly export section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#export-section>
pub fn parse_export_section(input: &[u8]) -> IResult<&[u8], Option<Vec<Export>>> {
    opt(parse_section(
        ModuleSection::Export,
        parse_vector(parse_export),
    ))(input)
}

/// Parses a WebAssembly element section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#element-section>
pub fn parse_element_section(input: &[u8]) -> IResult<&[u8], Option<Vec<Element>>> {
    opt(parse_section(
        ModuleSection::Element,
        parse_vector(parse_element),
    ))(input)
}

/// Type alias for a code section entry.
type Code = Vec<(ResultType, Expression)>;

/// Parses a WebAssembly code section.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#code-section>
pub fn parse_code_section(input: &[u8]) -> IResult<&[u8], Option<Code>> {
    opt(parse_section(ModuleSection::Code, parse_vector(parse_code)))(input)
}

/// Parses a section with the given identifier.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#sections>
fn parse_section<'input, O, P>(
    section: ModuleSection,
    parser: P,
) -> impl FnMut(&'input [u8]) -> IResult<&'input [u8], O>
where
    P: Parser<&'input [u8], O, nom::error::Error<&'input [u8]>>,
{
    map_parser(parse_section_raw(section), all_consuming(parser))
}

/// Parses the raw bytes of a section with the given identifier.
/// Validates the section identified and length.
///
/// See <https://webassembly.github.io/spec/core/binary/modules.html#sections>
fn parse_section_raw(section: ModuleSection) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8]> {
    move |input| {
        let (input, _) = match_byte(section as u8)(input)?;
        let (input, length) = parse_u32(input)?;

        take(length)(input)
    }
}
