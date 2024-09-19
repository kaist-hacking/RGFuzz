use crate::emitter::errors::EmitError;
use crate::emitter::module::{
    emit_custom_content, emit_data, emit_element, emit_export, emit_function, emit_global,
    emit_import, emit_memory, emit_start, emit_table,
};
use crate::emitter::types::emit_function_type;
use crate::emitter::values::{
    emit_byte, emit_bytes, emit_repeated, emit_u32, emit_usize, emit_vector,
};
use crate::emitter::CountingWrite;
use crate::model::{Custom, Function, Module, ModuleSection, TypeIndex};
use std::io::Write;

/// A magic constant used to quickly identify WebAssembly binary file contents.
const PREAMBLE: [u8; 4] = [0x00u8, 0x61u8, 0x73u8, 0x6Du8];

/// The version of the binary WebAssembly format emitted.
const VERSION: [u8; 4] = [0x01u8, 0x00u8, 0x00u8, 0x00u8];

/// Emit a module to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html
pub fn emit_module<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    let mut bytes = 0;

    bytes += emit_bytes(&PREAMBLE, output, false)?;
    bytes += emit_bytes(&VERSION, output, false)?;
    bytes += emit_custom_sections(module, ModuleSection::Custom, output)?;
    bytes += emit_type_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Type, output)?;
    bytes += emit_import_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Import, output)?;
    bytes += emit_function_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Function, output)?;
    bytes += emit_table_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Table, output)?;
    bytes += emit_memory_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Memory, output)?;
    bytes += emit_global_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Global, output)?;
    bytes += emit_export_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Export, output)?;
    bytes += emit_start_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Start, output)?;
    bytes += emit_element_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Element, output)?;
    bytes += emit_data_count_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::DataCount, output)?;
    bytes += emit_code_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Code, output)?;
    bytes += emit_data_section(module, output)?;
    bytes += emit_custom_sections(module, ModuleSection::Data, output)?;

    Ok(bytes)
}

/// Emits the custom section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#custom-section
pub fn emit_custom_sections<O: Write>(
    module: &Module,
    insertion_point: ModuleSection,
    output: &mut O,
) -> Result<usize, EmitError> {
    match module.custom_sections_at(insertion_point) {
        None => Ok(0),
        Some(sections) => emit_repeated(sections, output, emit_custom_section),
    }
}

/// Emits the custom section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#custom-section
pub fn emit_custom_section<O: Write>(custom: &Custom, output: &mut O) -> Result<usize, EmitError> {
    emit_section(ModuleSection::Custom, output, |o| {
        emit_custom_content(custom, o)
    })
}

/// Emits the type section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#type-section
pub fn emit_type_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.function_types() {
        None => Ok(0),
        Some(types) => emit_section(ModuleSection::Type, output, |o| {
            emit_vector(types, o, emit_function_type)
        }),
    }
}

/// Emits the import section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#import-section
pub fn emit_import_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.imports() {
        None => Ok(0),
        Some(imports) => emit_section(ModuleSection::Import, output, |o| {
            emit_vector(imports, o, emit_import)
        }),
    }
}

/// Emits the function section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#function-section
pub fn emit_function_section<O: Write>(
    module: &Module,
    output: &mut O,
) -> Result<usize, EmitError> {
    match module.functions() {
        None => Ok(0),
        Some(functions) => {
            let types: Vec<TypeIndex> = functions.iter().map(Function::kind).collect();

            emit_section(ModuleSection::Function, output, move |o| {
                emit_vector(types.as_slice(), o, emit_u32)
            })
        }
    }
}

/// Emits the table section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#table-section
pub fn emit_table_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.tables() {
        Some(tables) => emit_section(ModuleSection::Table, output, |o| {
            emit_vector(tables, o, emit_table)
        }),
        None => Ok(0),
    }
}

/// Emits the memory section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#memory-section
pub fn emit_memory_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.memories() {
        Some(memories) => emit_section(ModuleSection::Memory, output, |o| {
            emit_vector(memories, o, emit_memory)
        }),
        None => Ok(0),
    }
}

/// Emits the global section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#global-section
pub fn emit_global_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.globals() {
        Some(globals) => emit_section(ModuleSection::Global, output, |o| {
            emit_vector(globals, o, emit_global)
        }),
        None => Ok(0),
    }
}

/// Emits the export section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#export-section
pub fn emit_export_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.exports() {
        Some(exports) => emit_section(ModuleSection::Export, output, |o| {
            emit_vector(exports, o, emit_export)
        }),
        None => Ok(0),
    }
}
/// Emits the start section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#start-section
pub fn emit_start_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.start() {
        Some(start) => emit_section(ModuleSection::Start, output, |o| emit_start(start, o)),
        None => Ok(0),
    }
}
/// Emits the elements section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#elements-section
pub fn emit_element_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.elements() {
        None => Ok(0),
        Some(elements) => emit_section(ModuleSection::Element, output, |o| {
            emit_vector(elements, o, emit_element)
        }),
    }
}

/// Emits the data count section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#data-count-section
pub fn emit_data_count_section<O: Write>(
    module: &Module,
    output: &mut O,
) -> Result<usize, EmitError> {
    match module.data_count() {
        None => Ok(0),
        Some(count) => emit_section(ModuleSection::DataCount, output, |o| emit_u32(count, o)),
    }
}

/// Emits the code section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#code-section
pub fn emit_code_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.functions() {
        None => Ok(0),
        Some(functions) => emit_section(ModuleSection::Code, output, |o| {
            emit_vector(functions, o, emit_function)
        }),
    }
}

/// Emits the data section to the output.
///
/// See https://webassembly.github.io/spec/core/binary/modules.html#data-section
pub fn emit_data_section<O: Write>(module: &Module, output: &mut O) -> Result<usize, EmitError> {
    match module.data() {
        None => Ok(0),
        Some(data) => emit_section(ModuleSection::Data, output, |o| {
            emit_vector(data, o, emit_data)
        }),
    }
}

/// Emits a module section to the given output.
/// Sections need to be prefixed by their length.
/// Since we do not know the length of the emitted contents ahead of time,
/// a buffer is used to hold the emitted values and copy the buffer contents to the output.
/// The buffer is reset before emitting content and after it is copied.
pub fn emit_section<E, O>(
    section: ModuleSection,
    output: &mut O,
    emit: E,
) -> Result<usize, EmitError>
where
    O: Write,
    E: Fn(&mut dyn Write) -> Result<usize, EmitError>,
{
    let mut bytes = 0;
    let mut counter = CountingWrite::new();

    emit(&mut counter)?;

    bytes += emit_byte(section as u8, output)?;
    bytes += emit_usize(counter.bytes(), output)?;
    bytes += emit(output)?;

    Ok(bytes)
}
