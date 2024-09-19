//! Parser of the WebAssembly binary format.

mod errors;
mod instructions;
mod module;
mod sections;
mod types;
mod values;

use crate::parser::sections::{
    parse_code_section, parse_custom_section, parse_data_count_section, parse_data_section,
    parse_element_section, parse_export_section, parse_function_section, parse_global_section,
    parse_import_section, parse_memory_section, parse_start_section, parse_table_section,
    parse_type_section,
};
use crate::{Expression, Function, Module, ModuleSection, ResultType, TypeIndex};
pub use errors::ParseError;
use nom::bytes::complete::tag;
use nom::combinator::all_consuming;
use nom::sequence::tuple;

/// A magic constant used to quickly identify WebAssembly binary file contents.
const PREAMBLE: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];

/// The version of the binary WebAssembly format emitted.
const VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

/// Parses the given bytes into a WebAssembly module.
/// The bytes are parsed using the WebAssembly binary format.
/// Requires that no trailing information is present after the last group of custom sections
/// (i.e. valid WebAssembly binary format passed in with trailing data will be treated as invalid).
///
/// Also, the function and code sections must have matching lengths.
///
/// See <https://webassembly.github.io/spec/core/binary/index.html>
///
/// # Examples
/// ## Empty
/// ```rust
/// use wasm_ast::parse_binary;
///
/// let module = parse_binary(b"\x00\x61\x73\x6D\x01\x00\x00\x00").unwrap();
///
/// assert_eq!(module.functions(), None);
/// assert_eq!(module.functions(), None);
/// assert_eq!(module.tables(), None);
/// assert_eq!(module.memories(), None);
/// assert_eq!(module.globals(), None);
/// assert_eq!(module.elements(), None);
/// assert_eq!(module.data(), None);
/// assert_eq!(module.start(), None);
/// assert_eq!(module.imports(), None);
/// assert_eq!(module.exports(), None);
/// assert_eq!(module.data_count(), None);
/// ```
pub fn parse_binary(input: &[u8]) -> Result<Module, ParseError> {
    let mut builder = Module::builder();

    let (input, _) = tuple((tag(PREAMBLE), tag(VERSION)))(input)?;

    let (input, custom_sections) =
        parse_custom_section(input).map_err(|_| ParseError::InvalidBinary)?;
    builder.set_custom_sections(ModuleSection::Custom, custom_sections);

    let (input, types) = parse_type_section(input)?;
    builder.set_function_types(types);

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Type, custom_sections);

    let (input, imports) = parse_import_section(input)?;
    builder.set_imports(imports);

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Import, custom_sections);

    let (input, signatures) = parse_function_section(input)?;

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Function, custom_sections);

    let (input, tables) = parse_table_section(input)?;
    builder.set_tables(tables);

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Table, custom_sections);

    let (input, memories) = parse_memory_section(input)?;
    builder.set_memories(memories);

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Memory, custom_sections);

    let (input, globals) = parse_global_section(input)?;
    builder.set_globals(globals);

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Global, custom_sections);

    let (input, exports) = parse_export_section(input)?;
    builder.set_exports(exports);

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Export, custom_sections);

    let (input, start) = parse_start_section(input)?;
    builder.set_start(start);

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Start, custom_sections);

    let (input, elements) = parse_element_section(input)?;
    builder.set_elements(elements);

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Element, custom_sections);

    let (input, data_count) = parse_data_count_section(input)?;
    builder.set_data_count(data_count);

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::DataCount, custom_sections);

    let (input, codes) = parse_code_section(input)?;

    validate_function_counts(codes.as_ref(), signatures.as_ref())?;

    builder.set_functions(zip_functions(signatures, codes));

    let (input, custom_sections) = parse_custom_section(input)?;
    builder.set_custom_sections(ModuleSection::Code, custom_sections);

    let (input, data) = parse_data_section(input)?;
    builder.set_data(data);

    let (_, custom_sections) = all_consuming(parse_custom_section)(input)?;
    builder.set_custom_sections(ModuleSection::Data, custom_sections);

    Ok(builder.build())
}

/// Zips code and function sections into a function syntax type.
fn zip_functions(
    signatures: Option<Vec<TypeIndex>>,
    codes: Option<Vec<(ResultType, Expression)>>,
) -> Option<Vec<Function>> {
    codes.zip(signatures).map(|(codes, signatures)| {
        codes
            .into_iter()
            .zip(signatures)
            .map(|((locals, body), kind)| Function::new(kind, locals, body))
            .collect()
    })
}

/// Validates the parsed function and code section lengths match.
fn validate_function_counts(
    codes: Option<&Vec<(ResultType, Expression)>>,
    signatures: Option<&Vec<TypeIndex>>,
) -> Result<(), ParseError> {
    if codes.is_none() && signatures.is_none() {
        return Ok(());
    }

    let code_count = codes.as_ref().map(|v| v.len());
    let signature_count = signatures.as_ref().map(|v| v.len());
    let lengths_match = code_count.zip(signature_count).filter(|(a, b)| a == b);

    lengths_match
        .map(|_| ())
        .ok_or(ParseError::MismatchedFunctionParts(
            code_count,
            signature_count,
        ))
}

/// Parses the given string into a WebAssembly module.
/// The string is first converted to WebAssembly binary, then parse.
/// Some information may be lost in the conversion from text to binary format.
///
/// # Examples
/// ## Empty
/// ```rust
/// use wasm_ast::parse_text;
///
/// let module = parse_text("(module)").unwrap();
///
/// assert_eq!(module.functions(), None);
/// assert_eq!(module.functions(), None);
/// assert_eq!(module.tables(), None);
/// assert_eq!(module.memories(), None);
/// assert_eq!(module.globals(), None);
/// assert_eq!(module.elements(), None);
/// assert_eq!(module.data(), None);
/// assert_eq!(module.start(), None);
/// assert_eq!(module.imports(), None);
/// assert_eq!(module.exports(), None);
/// assert_eq!(module.data_count(), None);
/// ```
#[cfg(feature = "text")]
pub fn parse_text(text: &str) -> Result<Module, ParseError> {
    let binary = wat::parse_str(text)?;

    parse_binary(binary.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Custom;

    #[test]
    fn validate_functions_no_code() {
        let result = validate_function_counts(None, Some(vec![]).as_ref());

        assert!(result.is_err());
    }

    #[test]
    fn validate_functions_no_signatures() {
        let result = validate_function_counts(Some(vec![]).as_ref(), None);

        assert!(result.is_err());
    }

    #[test]
    fn validate_functions_empty() {
        let result = validate_function_counts(None, None);

        assert!(result.is_ok());
    }

    #[test]
    fn validate_functions_match() {
        let result = validate_function_counts(
            Some(vec![(ResultType::empty(), Expression::empty())]).as_ref(),
            Some(vec![0]).as_ref(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn zip_functions_no_code() {
        let result = zip_functions(Some(vec![]), None);

        assert!(result.is_none());
    }

    #[test]
    fn zip_functions_no_signatures() {
        let result = zip_functions(None, Some(vec![]));

        assert!(result.is_none());
    }

    #[test]
    fn zip_functions_empty() {
        let result = zip_functions(None, None);

        assert!(result.is_none());
    }

    #[test]
    fn zip_functions_match() {
        let locals = ResultType::empty();
        let body = Expression::empty();
        let function = Function::new(0, locals.clone(), body.clone());

        let result = zip_functions(Some(vec![0]), Some(vec![(locals, body)]));

        assert_eq!(result, Some(vec![function]));
    }

    #[test]
    fn zip_functions_signature_longer() {
        let locals = ResultType::empty();
        let body = Expression::empty();
        let function = Function::new(0, locals.clone(), body.clone());

        let result = zip_functions(Some(vec![0, 1]), Some(vec![(locals, body)]));

        assert_eq!(result, Some(vec![function]));
    }

    #[test]
    fn zip_functions_code_longer() {
        let locals = ResultType::empty();
        let body = Expression::empty();
        let function = Function::new(0, locals.clone(), body.clone());

        let result = zip_functions(
            Some(vec![0]),
            Some(vec![(locals.clone(), body.clone()), (locals, body)]),
        );

        assert_eq!(result, Some(vec![function]));
    }

    #[test]
    fn empty_module() {
        let mut builder = Module::builder();

        builder.add_custom_section(
            ModuleSection::Custom,
            Custom::new("version".into(), Vec::from("0.1.0".as_bytes())),
        );

        let module = builder.build();
        let mut bytes: Vec<u8> = Vec::new();
        let prefix = b"\x00\x61\x73\x6D\x01\x00\x00\x00";
        let section = b"\x00";
        let name = "version".as_bytes();
        let version = "0.1.0".as_bytes();
        let size = name.len() + version.len() + 1;

        bytes.extend(prefix);
        bytes.extend(section);
        bytes.push(size as u8);
        bytes.push(name.len() as u8);
        bytes.extend(name);
        bytes.extend(version);

        let actual = parse_binary(bytes.as_slice()).unwrap();

        assert_eq!(actual, module);
    }
}
