use wasm_ast::{
    ControlInstruction, Custom, Data, Export, Expression, Function, FunctionType, Import, Limit,
    Memory, Module, ModuleSection, ResultType, Start, ValueType,
};

fn main() {
    let message = "Hello, World!";
    let function_types = vec![
        FunctionType::side_effect(vec![ValueType::I32, ValueType::I32].into()),
        FunctionType::runnable(),
    ];
    let imports = vec![Import::function("console".into(), "log".into(), 0)];
    let start = Start::new(1);
    let functions = vec![Function::new(
        1,
        ResultType::empty(),
        vec![
            0i32.into(),
            (message.len() as u32).into(),
            ControlInstruction::Call(0).into(),
        ]
        .into(),
    )];
    let memories = vec![Memory::new(Limit::bounded(1, 4).into())];
    let exports = vec![Export::memory("memory".into(), 0)];
    let data = vec![Data::active(0, Expression::empty(), Vec::from(message))];
    let header_custom = vec![Custom::new("version".into(), Vec::from("1.0.0"))];
    let footer_custom = vec![Custom::new("footer".into(), Vec::from("foot"))];

    let mut builder = Module::builder();
    builder.set_function_types(Some(function_types.clone()));
    builder.set_functions(Some(functions.clone()));
    builder.set_memories(Some(memories.clone()));
    builder.set_data(Some(data.clone()));
    builder.set_start(Some(start));
    builder.set_imports(Some(imports.clone()));
    builder.set_exports(Some(exports.clone()));
    builder.set_custom_sections(ModuleSection::Custom, Some(header_custom.clone()));
    builder.set_custom_sections(ModuleSection::Data, Some(footer_custom.clone()));
    builder.set_data_count(Some(1));

    let module = builder.build();

    assert_eq!(module.function_types(), Some(function_types.as_slice()));
    assert_eq!(module.functions(), Some(functions.as_slice()));
    assert_eq!(module.tables(), None);
    assert_eq!(module.memories(), Some(memories.as_slice()));
    assert_eq!(module.globals(), None);
    assert_eq!(module.elements(), None);
    assert_eq!(module.data(), Some(data.as_slice()));
    assert_eq!(module.start(), Some(&start));
    assert_eq!(module.imports(), Some(imports.as_slice()));
    assert_eq!(module.exports(), Some(exports.as_slice()));
    assert_eq!(
        module.custom_sections_at(ModuleSection::Custom),
        Some(header_custom.as_slice())
    );
    assert_eq!(
        module.custom_sections_at(ModuleSection::Export),
        Some(footer_custom.as_slice())
    );
    assert_eq!(module.data_count(), Some(1));
}
