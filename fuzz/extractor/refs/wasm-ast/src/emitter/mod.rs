//! Emit WebAssembly binary format.

mod errors;
pub mod instruction;
mod module;
pub mod sections;
mod types;
mod values;

use crate::model::Module;
use sections::emit_module;
use std::fmt::Debug;
use std::io::Write;

/// Emits a binary representation of a WebAssembly Abstract Syntax Tree (AST) to a `Write` output.
///
/// See <https://webassembly.github.io/spec/core/binary/index.html>
///
/// # Examples
/// ## Empty
/// ```rust
/// use wasm_ast::emit_binary;
/// use wasm_ast::Module;
///
/// let mut buffer = Vec::new();
/// let binary = emit_binary(&Module::empty(), &mut buffer).unwrap();
///
/// assert_eq!(buffer, vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]);
/// ```
pub fn emit_binary<O: Write>(module: &Module, output: &mut O) -> Result<usize, errors::EmitError> {
    emit_module(module, output)
}

/// Counts the number of bytes written, but does else nothing with the bytes.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
struct CountingWrite {
    bytes: usize,
}

impl CountingWrite {
    /// Create a default instance of a counting write.
    pub fn new() -> Self {
        CountingWrite { bytes: 0 }
    }

    /// The number of bytes written so far.
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

impl Default for CountingWrite {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for CountingWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes += buf.len();

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.bytes += buf.len();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emitter::errors::EmitError;
    use crate::model::{
        ControlInstruction, Custom, Data, DataMode, Element, ElementInitializer, ElementMode,
        Export, ExportDescription, Expression, Function, FunctionType, Global, GlobalType, Import,
        ImportDescription, Instruction, Limit, Memory, MemoryType, Module, ModuleSection, Name,
        NumericInstruction, ReferenceType, ResultType, Start, Table, TableType, ValueType,
    };
    use crate::parser::parse_binary;
    use wasmtime::{Engine, Extern, Func, Instance, Store};

    fn validate(target: &Module) -> Result<(), EmitError> {
        let mut bytes = Vec::new();

        emit_binary(&target, &mut bytes)?;

        let parsed = parse_binary(bytes.as_slice())
            .map_err(|_| EmitError::IO(std::io::Error::from(std::io::ErrorKind::NotFound)))?;

        assert_eq!(target, &parsed);

        let engine = Engine::default();
        let module = wasmtime::Module::new(&engine, &bytes)
            .map_err(|_| EmitError::IO(std::io::Error::from(std::io::ErrorKind::NotFound)))?;
        let mut store = Store::new(&engine, 0);
        let mut imports: Vec<Extern> = Vec::new();

        if target.imports().is_some() {
            let start = Func::wrap(&mut store, || {});
            imports.push(start.into());
        }

        Instance::new(&mut store, &module, &imports)
            .map_err(|_| EmitError::IO(std::io::Error::from(std::io::ErrorKind::NotFound)))?;

        Ok(())
    }

    #[test]
    fn empty_module() {
        let mut buffer = Vec::new();
        let mut builder = Module::builder();

        builder.add_custom_section(
            ModuleSection::Custom,
            Custom::new("version".into(), Vec::from("0.1.0".as_bytes())),
        );

        let module = builder.build();
        let result = validate(&module);

        assert!(result.is_ok());

        emit_binary(&module, &mut buffer).unwrap();

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

        assert_eq!(&buffer, &bytes);
    }

    #[test]
    fn valid_empty_module() {
        let module = Module::empty();
        let result = validate(&module);

        assert!(result.is_ok());
    }

    #[test]
    fn valid_module() {
        let mut module = Module::builder();
        let function_type = FunctionType::new(
            ResultType::new(vec![ValueType::I64]),
            ResultType::new(vec![ValueType::F64]),
        );
        module.add_function_type(function_type).unwrap();

        let function = Function::new(
            0,
            ResultType::new(vec![ValueType::I32]),
            Expression::new(vec![Instruction::Numeric(NumericInstruction::F64Constant(
                0.0,
            ))]),
        );
        module.add_function(function).unwrap();

        let start_function_type =
            FunctionType::new(ResultType::new(vec![]), ResultType::new(vec![]));
        module.add_function_type(start_function_type).unwrap();

        let import = Import::new(
            Name::new("test".to_string()),
            Name::new("foobar".to_string()),
            ImportDescription::Function(1),
        );
        module.add_import(import).unwrap();

        let element = Element::new(
            ReferenceType::Function,
            ElementMode::Passive,
            vec![0].to_initializers(),
        );
        module.add_element(element).unwrap();

        let data = Data::new(DataMode::Passive, vec![42]);
        module.add_data(data).unwrap();

        let table = Table::new(TableType::new(ReferenceType::Function, Limit::new(1, None)));
        module.add_table(table).unwrap();

        let memory = Memory::new(MemoryType::new(Limit::new(1, None)));
        module.add_memory(memory).unwrap();

        let export = Export::new(
            Name::new("foobar".to_string()),
            ExportDescription::Function(0),
        );
        module.add_export(export);

        let start = Start::new(0);
        module.set_start(Some(start));

        let global = Global::new(
            GlobalType::immutable(ValueType::I64),
            Expression::new(vec![Instruction::Numeric(NumericInstruction::I64Constant(
                0,
            ))]),
        );
        module.add_global(global).unwrap();

        let result = validate(&module.build());

        assert!(result.is_ok());
    }

    #[test]
    fn valid_module_import() {
        let mut module = Module::builder();

        let start_function_type =
            FunctionType::new(ResultType::new(vec![]), ResultType::new(vec![]));
        module.add_function_type(start_function_type).unwrap();

        let import = Import::new(
            Name::new("test".to_string()),
            Name::new("foobar".to_string()),
            ImportDescription::Function(0),
        );
        module.add_import(import).unwrap();

        let result = validate(&module.build());

        assert!(result.is_ok());
    }

    #[test]
    fn valid_module_type_only() {
        let mut module = Module::builder();
        let function_type = FunctionType::new(
            ResultType::new(vec![ValueType::I64]),
            ResultType::new(vec![ValueType::F64]),
        );
        module.add_function_type(function_type).unwrap();

        let result = validate(&module.build());

        assert!(result.is_ok());
    }

    #[test]
    fn valid_module_function() {
        let mut module = Module::builder();
        let function_type = FunctionType::new(
            ResultType::new(vec![ValueType::I64]),
            ResultType::new(vec![ValueType::F64]),
        );
        module.add_function_type(function_type).unwrap();

        let function = Function::new(
            0,
            ResultType::new(vec![ValueType::I32]),
            Expression::new(vec![Instruction::Numeric(NumericInstruction::F64Constant(
                0.0,
            ))]),
        );
        module.add_function(function).unwrap();

        validate(&module.build()).unwrap();
    }

    #[test]
    fn valid_module_start() {
        let mut module = Module::builder();
        let function_type = FunctionType::new(ResultType::new(vec![]), ResultType::new(vec![]));
        module.add_function_type(function_type).unwrap();

        let function = Function::new(
            0,
            ResultType::new(vec![]),
            Expression::new(vec![Instruction::Control(ControlInstruction::Nop)]),
        );
        module.add_function(function).unwrap();

        let start = Start::new(0);
        module.set_start(Some(start));

        validate(&module.build()).unwrap();
    }

    #[test]
    fn valid_module_element() {
        let mut module = Module::builder();

        let function_type = FunctionType::new(
            ResultType::new(vec![ValueType::I64]),
            ResultType::new(vec![ValueType::F64]),
        );
        module.add_function_type(function_type).unwrap();

        let function = Function::new(
            0,
            ResultType::new(vec![ValueType::I32]),
            Expression::new(vec![Instruction::Numeric(NumericInstruction::F64Constant(
                0.0,
            ))]),
        );
        module.add_function(function).unwrap();

        let element = Element::new(
            ReferenceType::Function,
            ElementMode::Passive,
            vec![0].to_initializers(),
        );
        module.add_element(element).unwrap();

        let table = Table::new(TableType::new(ReferenceType::Function, Limit::new(0, None)));
        module.add_table(table).unwrap();

        validate(&module.build()).unwrap();
    }

    #[test]
    fn valid_module_table_only() {
        let mut module = Module::builder();

        let table = Table::new(TableType::new(ReferenceType::Function, Limit::new(0, None)));
        module.add_table(table).unwrap();

        validate(&module.build()).unwrap();
    }

    #[test]
    fn valid_module_data() {
        let mut module = Module::builder();

        let data = Data::new(DataMode::Passive, vec![1]);
        module.add_data(data).unwrap();

        let memory = Memory::new(MemoryType::new(Limit::new(0, None)));
        module.add_memory(memory).unwrap();

        validate(&module.build()).unwrap();
    }

    #[test]
    fn valid_module_memory_only() {
        let mut module = Module::builder();

        let memory = Memory::new(MemoryType::new(Limit::new(0, None)));
        module.add_memory(memory).unwrap();

        validate(&module.build()).unwrap();
    }

    #[test]
    fn valid_module_global_only() {
        let mut module = Module::builder();

        let global = Global::new(
            GlobalType::immutable(ValueType::I64),
            Expression::new(vec![Instruction::Numeric(NumericInstruction::I64Constant(
                0,
            ))]),
        );
        module.add_global(global).unwrap();

        validate(&module.build()).unwrap();
    }

    #[test]
    fn valid_module_import_only() {
        let mut module = Module::builder();

        let export = Export::new(
            Name::new("foobar".to_string()),
            ExportDescription::Global(0),
        );
        module.add_export(export);

        let global = Global::new(
            GlobalType::immutable(ValueType::I64),
            Expression::new(vec![Instruction::Numeric(NumericInstruction::I64Constant(
                0,
            ))]),
        );
        module.add_global(global).unwrap();

        validate(&module.build()).unwrap();
    }

    #[test]
    fn invalid_module() {
        let mut module = Module::builder();

        // function with no corresponding type.
        let function = Function::new(
            0,
            ResultType::new(vec![ValueType::I32]),
            Expression::new(vec![Instruction::Control(ControlInstruction::Nop)]),
        );
        module.add_function(function).unwrap();

        let result = validate(&module.build());

        assert!(result.is_err());
    }
}
